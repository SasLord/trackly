//! `AttachmentService` - read side of the attachment storage (Phase 41.3).
//!
//! `authorize` is the single access check (D-04): list, file, thumb, upload,
//! delete, restore, purge and history all go through it. Knowing an attachment
//! id or a content hash grants nothing; the file path is built only from the
//! hash stored in the DB row.

use std::path::PathBuf;
use std::sync::Arc;

use rusqlite::OptionalExtension;
use trackly_core::auth::{Identity, Role};
use trackly_core::domain::attachments::{ParentKind, DEFAULT_MAX_BYTES, SETTING_MAX_BYTES};
use trackly_core::error::AppError;
use trackly_core::primitives::clock::Clock;
use trackly_infra::db::{pools::ReaderPool, writer_worker::WriterHandle};
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::repos::attachments_sqlite::{AttachmentRow, SqliteAttachmentRepository};
use trackly_infra::repos::audit_log_sqlite::SqliteAuditLogRepository;

use super::attachment_store::AttachmentStore;
use crate::dto::attachment::{AttachmentDto, AttachmentListDto};
use crate::dto::printer::WsEvent;

/// MIME types the browser can show inline.
const INLINE_MIMES: [&str; 5] = [
    "image/jpeg",
    "image/png",
    "image/webp",
    "image/gif",
    "application/pdf",
];
/// MIME types that get a thumbnail.
const PHOTO_MIMES: [&str; 4] = ["image/jpeg", "image/png", "image/webp", "image/gif"];

/// Parent object reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParentRef {
    pub kind: ParentKind,
    pub id: i64,
}

/// What the caller wants to do. `DeleteOwn` carries the uploader of the row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Read,
    Attach,
    DeleteOwn { uploaded_by: Option<i64> },
    DeleteAny,
    ManageDeleted,
}

/// Caller permissions on a parent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Perms {
    pub can_attach: bool,
    pub can_delete_any: bool,
    pub can_manage_deleted: bool,
    pub readonly: bool,
}

/// Which representation to open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadKind {
    File,
    Thumb,
}

/// Resolved file ready to stream; the path never comes from user input.
#[derive(Debug, Clone)]
pub struct OpenedAttachment {
    pub path: PathBuf,
    pub mime: String,
    pub original_name: String,
    pub size: i64,
    pub inline: bool,
}

/// Employee access to device/printer/cartridge attachments.
///
/// Deny-by-default in 41.3 (D-27): opened by phase 41.6 together with the
/// Devices section rule for Employee.
fn employee_can_read_device(_caller: &Identity, _kind: ParentKind, _parent_id: i64) -> bool {
    false
}

fn internal(e: tokio::task::JoinError) -> AppError {
    AppError::Internal {
        source_chain: format!("spawn_blocking: {e}"),
    }
}

/// Application service for attachments. `Arc` fields keep `Clone` O(1).
#[derive(Clone)]
pub struct AttachmentService {
    pub writer: Arc<WriterHandle>,
    pub readers: Arc<ReaderPool>,
    // Used by the write paths (plans 07+).
    #[allow(dead_code)]
    pub(crate) clock: Arc<dyn Clock + Send + Sync>,
    pub(crate) store: Arc<AttachmentStore>,
    pub(crate) repo: Arc<SqliteAttachmentRepository>,
    #[allow(dead_code)]
    pub(crate) audit_repo: Arc<SqliteAuditLogRepository>,
    /// Shared WS sender; `None` in test fixtures, wired by `AppCtx::build`.
    pub(crate) ws_tx: Option<Arc<tokio::sync::broadcast::Sender<WsEvent>>>,
}

impl AttachmentService {
    pub fn new(
        writer: Arc<WriterHandle>,
        readers: Arc<ReaderPool>,
        clock: Arc<dyn Clock + Send + Sync>,
        store: Arc<AttachmentStore>,
    ) -> Self {
        Self {
            writer,
            readers,
            clock,
            store,
            repo: Arc::new(SqliteAttachmentRepository),
            audit_repo: Arc::new(SqliteAuditLogRepository),
            ws_tx: None,
        }
    }

    /// Convenience constructor over a blob root directory (used by fixtures
    /// that cannot name the `pub(crate)` store type).
    pub fn with_root(
        writer: Arc<WriterHandle>,
        readers: Arc<ReaderPool>,
        clock: Arc<dyn Clock + Send + Sync>,
        root: PathBuf,
    ) -> Self {
        Self::new(writer, readers, clock, Arc::new(AttachmentStore::new(root)))
    }

    /// Builder: wire the shared WS broadcast sender.
    #[must_use]
    pub fn with_ws_tx(mut self, ws_tx: Arc<tokio::sync::broadcast::Sender<WsEvent>>) -> Self {
        self.ws_tx = Some(ws_tx);
        self
    }

    pub fn store(&self) -> &Arc<AttachmentStore> {
        &self.store
    }

    /// Single access check (D-04). Returns the caller's permissions on the
    /// parent or an error: `NotFound` for a missing/soft-deleted parent
    /// (D-19/D-26), `Forbidden` for a denied caller.
    pub async fn authorize(
        &self,
        parent: ParentRef,
        caller: &Identity,
        intent: Intent,
    ) -> Result<Perms, AppError> {
        let readers = self.readers.clone();
        let owner = tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            load_parent(&conn, parent)
        })
        .await
        .map_err(internal)??;
        decide(parent, owner, caller, intent)
    }

    /// Parent's attachments with the caller's permissions.
    pub async fn list(
        &self,
        parent: ParentRef,
        caller: &Identity,
    ) -> Result<AttachmentListDto, AppError> {
        let perms = self.authorize(parent, caller, Intent::Read).await?;
        let include_deleted = perms.can_manage_deleted;
        let readers = self.readers.clone();
        let repo = self.repo.clone();
        let rows = tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            repo.list_by_parent_on(&conn, parent.kind.as_str(), parent.id, include_deleted)
        })
        .await
        .map_err(internal)??;
        let max = self.max_bytes().await?;
        let mut items = Vec::new();
        let mut deleted = Vec::new();
        for r in &rows {
            let dto = self.to_dto(r);
            if r.deleted_at_utc.is_some() {
                deleted.push(dto);
            } else {
                items.push(dto);
            }
        }
        Ok(AttachmentListDto {
            items,
            deleted,
            max_size_bytes: max,
            can_attach: perms.can_attach,
            can_delete_any: perms.can_delete_any,
            can_manage_deleted: perms.can_manage_deleted,
            readonly: perms.readonly,
        })
    }

    pub(crate) fn to_dto(&self, r: &AttachmentRow) -> AttachmentDto {
        let photo = PHOTO_MIMES.contains(&r.mime.as_str());
        let has_thumbnail = photo
            && self
                .store
                .thumb_path(&r.content_hash)
                .map(|p| p.is_file())
                .unwrap_or(false);
        AttachmentDto {
            id: r.id,
            original_name: r.original_name.clone(),
            mime: r.mime.clone(),
            size_bytes: r.size_bytes,
            has_thumbnail,
            viewable: INLINE_MIMES.contains(&r.mime.as_str()),
            missing: !self.store.blob_exists(&r.content_hash),
            uploaded_by_name: r.uploaded_by_name.clone(),
            uploaded_by_id: r.uploaded_by_user_id,
            created_at_utc: r.created_at_utc,
            deleted_by_name: r.deleted_by_name.clone(),
            deleted_at_utc: r.deleted_at_utc,
        }
    }

    /// Size limit from `app_settings`, falling back to the default.
    pub async fn max_bytes(&self) -> Result<i64, AppError> {
        let readers = self.readers.clone();
        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            let v: Option<String> = conn
                .query_row(
                    "SELECT value FROM app_settings WHERE key = ?1",
                    rusqlite::params![SETTING_MAX_BYTES],
                    |r| r.get(0),
                )
                .optional()
                .map_err(map_rusqlite)?;
            Ok(v.and_then(|s| s.trim().parse::<i64>().ok())
                .filter(|n| *n > 0)
                .unwrap_or(DEFAULT_MAX_BYTES as i64))
        })
        .await
        .map_err(internal)?
    }

    /// Resolves the file of attachment `id` for streaming after `authorize`.
    pub async fn open_for_read(
        &self,
        id: i64,
        caller: &Identity,
        kind: ReadKind,
    ) -> Result<OpenedAttachment, AppError> {
        let readers = self.readers.clone();
        let repo = self.repo.clone();
        let row = tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            repo.get_by_id_on(&conn, id)
        })
        .await
        .map_err(internal)??
        .ok_or(AppError::NotFound {
            entity: "attachment",
            id,
        })?;
        let pkind = ParentKind::parse(&row.parent_kind).ok_or(AppError::NotFound {
            entity: "attachment",
            id,
        })?;
        let parent = ParentRef {
            kind: pkind,
            id: row.parent_id,
        };
        let perms = self.authorize(parent, caller, Intent::Read).await?;
        if row.deleted_at_utc.is_some() && !perms.can_manage_deleted {
            return Err(AppError::NotFound {
                entity: "attachment",
                id,
            });
        }
        let not_file = AppError::NotFound {
            entity: "attachment_file",
            id,
        };
        match kind {
            ReadKind::File => {
                let path = self.store.blob_path(&row.content_hash)?;
                if !path.is_file() {
                    return Err(not_file);
                }
                Ok(OpenedAttachment {
                    path,
                    mime: row.mime.clone(),
                    original_name: row.original_name.clone(),
                    size: row.size_bytes,
                    inline: INLINE_MIMES.contains(&row.mime.as_str()),
                })
            }
            ReadKind::Thumb => {
                if !PHOTO_MIMES.contains(&row.mime.as_str()) {
                    return Err(not_file);
                }
                let path = self.store.thumb_path(&row.content_hash)?;
                let size = std::fs::metadata(&path)
                    .ok()
                    .filter(|m| m.is_file())
                    .map(|m| m.len() as i64)
                    .ok_or(not_file)?;
                Ok(OpenedAttachment {
                    path,
                    mime: "image/jpeg".to_string(),
                    original_name: row.original_name.clone(),
                    size,
                    inline: true,
                })
            }
        }
    }

    /// Author id of a request parent (for `AttachmentsChanged`), `None` for
    /// device/cartridge parents.
    pub async fn requested_by_for_event(&self, parent: ParentRef) -> Option<i64> {
        let readers = self.readers.clone();
        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            load_parent(&conn, parent).unwrap_or_default()
        })
        .await
        .ok()
        .flatten()
    }
}

/// Live parent row: returns the request author (`None` for other kinds).
/// Missing or soft-deleted parent is `NotFound`.
fn load_parent(conn: &rusqlite::Connection, parent: ParentRef) -> Result<Option<i64>, AppError> {
    let (entity, sql): (&'static str, &str) = match parent.kind {
        ParentKind::Request => (
            "request",
            "SELECT requested_by_user_id FROM requests WHERE id = ?1 AND deleted_at_utc IS NULL",
        ),
        ParentKind::Device => (
            "device",
            "SELECT NULL FROM devices WHERE id = ?1 AND deleted_at_utc IS NULL",
        ),
        ParentKind::Cartridge => (
            "cartridge",
            "SELECT NULL FROM cartridges WHERE id = ?1 AND deleted_at_utc IS NULL",
        ),
    };
    let found: Option<Option<i64>> = conn
        .query_row(sql, rusqlite::params![parent.id], |r| r.get(0))
        .optional()
        .map_err(map_rusqlite)?;
    found.ok_or(AppError::NotFound {
        entity,
        id: parent.id,
    })
}

/// Pure decision step of `authorize`.
fn decide(
    parent: ParentRef,
    request_author: Option<i64>,
    caller: &Identity,
    intent: Intent,
) -> Result<Perms, AppError> {
    let staff = matches!(caller.role, Role::Admin | Role::Manager);
    let admin = matches!(caller.role, Role::Admin);
    let readable = if staff {
        true
    } else {
        match parent.kind {
            // Author only, in any request status (D-02).
            ParentKind::Request => request_author.is_some() && caller.user_id == request_author,
            ParentKind::Device => employee_can_read_device(caller, parent.kind, parent.id),
            // Cartridges are never visible to Employee.
            ParentKind::Cartridge => false,
        }
    };
    if !readable {
        return Err(AppError::Forbidden);
    }
    // Employee-author may attach to own request only (D-02/D-03).
    let can_attach = staff || parent.kind == ParentKind::Request;
    let perms = Perms {
        can_attach,
        can_delete_any: staff,
        can_manage_deleted: admin,
        readonly: !can_attach,
    };
    let allowed = match intent {
        Intent::Read => true,
        Intent::Attach => perms.can_attach,
        Intent::DeleteAny => perms.can_delete_any,
        Intent::ManageDeleted => perms.can_manage_deleted,
        Intent::DeleteOwn { uploaded_by } => uploaded_by.is_some() && uploaded_by == caller.user_id,
    };
    if allowed {
        Ok(perms)
    } else {
        Err(AppError::Forbidden)
    }
}

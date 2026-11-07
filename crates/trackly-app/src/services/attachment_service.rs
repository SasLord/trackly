//! `AttachmentService` - read side of the attachment storage (Phase 41.3).
//!
//! `authorize` is the single access check (D-04): list, file, thumb, upload,
//! delete, restore, purge and history all go through it. Knowing an attachment
//! id or a content hash grants nothing; the file path is built only from the
//! hash stored in the DB row.

use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Arc;

use rusqlite::OptionalExtension;
use trackly_core::auth::{Identity, Role};
use trackly_core::domain::attachments::{
    sanitize_file_name, ParentKind, RejectCode, DEFAULT_MAX_BYTES, MAX_BYTES_CEILING,
    SETTING_MAX_BYTES,
};
use trackly_core::error::AppError;
use trackly_core::primitives::clock::Clock;
use trackly_infra::db::{pools::ReaderPool, writer_worker::WriterHandle};
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::repos::attachments_sqlite::{
    AttachmentNew, AttachmentRow, SqliteAttachmentRepository,
};
use trackly_infra::repos::audit_log_sqlite::{AuditEntry, SqliteAuditLogRepository};

use super::attachment_media;
use super::attachment_store::AttachmentStore;
use crate::dto::attachment::{
    AttachmentDto, AttachmentHistoryEventDto, AttachmentListDto, AttachmentUploadDto,
};
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

/// Process-wide cap for CPU-heavy media work (sniff, HEIC, thumbnail).
static MEDIA_PERMITS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(2);

/// Bytes of the head read for signature sniffing.
const HEAD_BYTES: usize = 32 * 1024;

/// Failure while receiving the body into staging.
#[derive(Debug)]
pub enum StageError {
    /// Per-file reject (size limit); never aborts other files.
    Reject(RejectCode),
    /// Disk failure on the staging file.
    Internal(AppError),
}

/// Upload body being received into a temp file under `<root>/.tmp`.
pub struct StagedUpload {
    file: tempfile::NamedTempFile,
    count: u64,
}

impl StagedUpload {
    /// Appends a chunk; fails with `TooLarge` as soon as the running size
    /// exceeds `min(limit, MAX_BYTES_CEILING)` (D-07, T-41.3-24).
    pub fn write_chunk(&mut self, chunk: &[u8], limit: i64) -> Result<(), StageError> {
        let limit = (limit.max(0) as u64).min(MAX_BYTES_CEILING);
        self.count += chunk.len() as u64;
        if self.count > limit {
            return Err(StageError::Reject(RejectCode::TooLarge));
        }
        self.file.write_all(chunk).map_err(|e| {
            StageError::Internal(AppError::Validation {
                field: "attachments_path".to_string(),
                message: format!("Не удалось записать вложение: {e}"),
            })
        })
    }

    /// Bytes received so far.
    pub fn len(&self) -> u64 {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}

/// Result of the blocking media stage.
enum Prepared {
    Rejected(RejectCode),
    Ready {
        temp: tempfile::NamedTempFile,
        hash: String,
        mime: &'static str,
        size: i64,
        name: String,
    },
}

fn audit_entity(kind: ParentKind) -> &'static str {
    match kind {
        ParentKind::Request => "request",
        ParentKind::Device => "device",
        ParentKind::Cartridge => "cartridge",
    }
}

/// Replaces the extension of `name` with `jpg` (HEIC converted to JPEG).
fn with_jpg_ext(name: &str) -> String {
    let stem = match name.rsplit_once('.') {
        Some((stem, _)) if !stem.is_empty() => stem,
        _ => name,
    };
    format!("{stem}.jpg")
}

/// Application service for attachments. `Arc` fields keep `Clone` O(1).
#[derive(Clone)]
pub struct AttachmentService {
    pub writer: Arc<WriterHandle>,
    pub readers: Arc<ReaderPool>,
    pub(crate) clock: Arc<dyn Clock + Send + Sync>,
    pub(crate) store: Arc<AttachmentStore>,
    pub(crate) repo: Arc<SqliteAttachmentRepository>,
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

    /// Access check before the first body byte is read (D-04).
    pub async fn authorize_attach(
        &self,
        caller: &Identity,
        parent: ParentRef,
    ) -> Result<Perms, AppError> {
        self.authorize(parent, caller, Intent::Attach).await
    }

    /// Effective upload limit: the setting, never above the hard ceiling.
    pub async fn effective_limit(&self) -> Result<i64, AppError> {
        Ok(self.max_bytes().await?.min(MAX_BYTES_CEILING as i64))
    }

    /// Opens a staging temp file for one incoming file.
    pub fn begin_staging(&self) -> Result<StagedUpload, AppError> {
        let file = self
            .store
            .begin_staging()
            .map_err(|e| AppError::Validation {
                field: "attachments_path".to_string(),
                message: format!("Не удалось начать загрузку вложения: {e}"),
            })?;
        Ok(StagedUpload { file, count: 0 })
    }

    /// Validates, stores and registers one staged file. Rejects come back as
    /// `Ok(AttachmentUploadDto { reject_code })`; only `Forbidden`, `NotFound`
    /// and `Internal` are errors. On any reject the staging temp is dropped.
    pub async fn ingest(
        &self,
        caller: &Identity,
        parent: ParentRef,
        raw_name: &str,
        staged: StagedUpload,
    ) -> Result<AttachmentUploadDto, AppError> {
        self.authorize_attach(caller, parent).await?;
        let reject = |c: RejectCode| AttachmentUploadDto {
            attachment: None,
            reject_code: Some(c.as_code().to_string()),
        };
        if staged.count == 0 {
            return Ok(reject(RejectCode::Empty));
        }
        let decoded = percent_encoding::percent_decode_str(raw_name)
            .decode_utf8_lossy()
            .into_owned();
        let name = sanitize_file_name(&decoded);

        let _permit = MEDIA_PERMITS
            .acquire()
            .await
            .map_err(|e| AppError::Internal {
                source_chain: format!("media semaphore: {e}"),
            })?;
        let store = self.store.clone();
        let prepared = tokio::task::spawn_blocking(move || prepare(&store, staged, name))
            .await
            .map_err(internal)??;
        drop(_permit);

        let (temp, hash, mime, size, name) = match prepared {
            Prepared::Rejected(code) => return Ok(reject(code)),
            Prepared::Ready {
                temp,
                hash,
                mime,
                size,
                name,
            } => (temp, hash, mime, size, name),
        };

        let repo = self.repo.clone();
        let audit_repo = self.audit_repo.clone();
        let store = self.store.clone();
        let now = self.clock.unix_seconds();
        let user_id = caller.user_id;
        let kind_str = parent.kind.as_str();
        let entity = audit_entity(parent.kind);
        let parent_id = parent.id;
        let row = self
            .writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;
                let id = repo.insert_on(
                    &tx,
                    &AttachmentNew {
                        parent_kind: kind_str,
                        parent_id,
                        original_name: &name,
                        mime,
                        size_bytes: size,
                        content_hash: &hash,
                        uploaded_by_user_id: user_id,
                        created_at_utc: now,
                    },
                )?;
                audit_repo.insert(
                    &tx,
                    AuditEntry {
                        entity_type: entity,
                        entity_id: parent_id,
                        action: "custom:attachment_added",
                        user_id,
                        before_json: None,
                        after_json: None,
                        payload_json: Some(
                            serde_json::json!({ "attachment_id": id, "name": name }).to_string(),
                        ),
                        created_at_utc: now,
                    },
                )?;
                // D-18: moves only when the blob is absent. If the commit below
                // fails after the rename, the orphan blob is left for the
                // integrity scan (plan 08).
                store.commit_staged(temp, &hash)?;
                let row = repo.get_by_id_on(&tx, id)?.ok_or(AppError::NotFound {
                    entity: "attachment",
                    id,
                })?;
                tx.commit().map_err(map_rusqlite)?;
                Ok(row)
            })
            .await?;

        // D-15/D-25: only after the writer returned Ok.
        self.notify_changed(parent).await;
        Ok(AttachmentUploadDto {
            attachment: Some(self.to_dto(&row)),
            reject_code: None,
        })
    }

    /// Sends `AttachmentsChanged` (D-15/D-25). Called only AFTER the writer
    /// await of a mutation that changed something; the gate test
    /// `attachments_changed_gate` enforces the ordering.
    async fn notify_changed(&self, parent: ParentRef) {
        let requested_by = self.requested_by_for_event(parent).await;
        if let Some(tx) = &self.ws_tx {
            let _ = tx.send(WsEvent::AttachmentsChanged {
                parent_kind: parent.kind.as_str().to_string(),
                parent_id: parent.id,
                requested_by_user_id: requested_by,
            });
        }
    }

    async fn load_row(&self, id: i64) -> Result<(AttachmentRow, ParentRef), AppError> {
        let readers = self.readers.clone();
        let repo = self.repo.clone();
        let not_found = AppError::NotFound {
            entity: "attachment",
            id,
        };
        let row = tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            repo.get_by_id_on(&conn, id)
        })
        .await
        .map_err(internal)??
        .ok_or(not_found)?;
        let kind = ParentKind::parse(&row.parent_kind).ok_or(AppError::NotFound {
            entity: "attachment",
            id,
        })?;
        let parent = ParentRef {
            kind,
            id: row.parent_id,
        };
        Ok((row, parent))
    }

    /// Soft delete (D-16): the uploader (own file, parent still readable) or
    /// Admin/Manager (any). The file stays on disk (D-18). Idempotent.
    pub async fn delete(&self, caller: &Identity, id: i64) -> Result<(), AppError> {
        let (row, parent) = self.load_row(id).await?;
        let perms = self.authorize(parent, caller, Intent::Read).await?;
        let intent = if perms.can_delete_any {
            Intent::DeleteAny
        } else {
            Intent::DeleteOwn {
                uploaded_by: row.uploaded_by_user_id,
            }
        };
        self.authorize(parent, caller, intent).await?;
        let repo = self.repo.clone();
        let audit_repo = self.audit_repo.clone();
        let now = self.clock.unix_seconds();
        let user_id = caller.user_id;
        let entity = audit_entity(parent.kind);
        let name = row.original_name.clone();
        let changed = self
            .writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;
                let changed = repo.soft_delete_on(&tx, id, user_id, now)?;
                if changed {
                    insert_audit(
                        &audit_repo,
                        &tx,
                        entity,
                        parent.id,
                        "custom:attachment_deleted",
                        user_id,
                        id,
                        &name,
                        now,
                    )?;
                }
                tx.commit().map_err(map_rusqlite)?;
                Ok(changed)
            })
            .await?;
        if changed {
            self.notify_changed(parent).await;
        }
        Ok(())
    }

    /// Restore a soft-deleted attachment (D-17): Admin only, parent must be live.
    pub async fn restore(&self, caller: &Identity, id: i64) -> Result<(), AppError> {
        let (row, parent) = self.load_row(id).await?;
        self.authorize(parent, caller, Intent::ManageDeleted)
            .await?;
        let repo = self.repo.clone();
        let audit_repo = self.audit_repo.clone();
        let now = self.clock.unix_seconds();
        let user_id = caller.user_id;
        let entity = audit_entity(parent.kind);
        let name = row.original_name.clone();
        let changed = self
            .writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;
                let changed = repo.restore_on(&tx, id)?;
                if changed {
                    insert_audit(
                        &audit_repo,
                        &tx,
                        entity,
                        parent.id,
                        "custom:attachment_restored",
                        user_id,
                        id,
                        &name,
                        now,
                    )?;
                }
                tx.commit().map_err(map_rusqlite)?;
                Ok(changed)
            })
            .await?;
        if changed {
            self.notify_changed(parent).await;
        }
        Ok(())
    }

    /// Purge a soft-deleted attachment (D-17/D-18): Admin only. The blob and
    /// thumbnail go away only when no other row, including soft-deleted ones,
    /// references the hash; count and removal run inside the single writer
    /// closure, so a concurrent upload of the same bytes is serialized with it.
    pub async fn purge(&self, caller: &Identity, id: i64) -> Result<(), AppError> {
        let (_row, parent) = self.load_row(id).await?;
        self.authorize(parent, caller, Intent::ManageDeleted)
            .await?;
        let repo = self.repo.clone();
        let audit_repo = self.audit_repo.clone();
        let store = self.store.clone();
        let now = self.clock.unix_seconds();
        let user_id = caller.user_id;
        let entity = audit_entity(parent.kind);
        self.writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;
                let row = repo.get_by_id_on(&tx, id)?.ok_or(AppError::NotFound {
                    entity: "attachment",
                    id,
                })?;
                if row.deleted_at_utc.is_none() {
                    return Err(AppError::Validation {
                        field: "attachment".to_string(),
                        message: "Сначала удалите вложение, затем очистите его".to_string(),
                    });
                }
                repo.delete_row_on(&tx, id)?;
                let last = repo.count_by_hash_on(&tx, &row.content_hash, None)? == 0;
                insert_audit(
                    &audit_repo,
                    &tx,
                    entity,
                    parent.id,
                    "custom:attachment_purged",
                    user_id,
                    id,
                    &row.original_name,
                    now,
                )?;
                tx.commit().map_err(map_rusqlite)?;
                if last {
                    // Row gone and committed; an I/O failure here leaves an
                    // orphan blob for the integrity scan, never a dangling row.
                    if let Err(e) = store.remove_blob(&row.content_hash) {
                        tracing::warn!("attachment purge: blob removal failed: {e}");
                    }
                }
                Ok(())
            })
            .await?;
        self.notify_changed(parent).await;
        Ok(())
    }

    /// Attachment events of a parent (D-14) with the current state of each
    /// attachment: `active`, `deleted` or `purged`.
    pub async fn history(
        &self,
        caller: &Identity,
        parent: ParentRef,
    ) -> Result<Vec<AttachmentHistoryEventDto>, AppError> {
        self.authorize(parent, caller, Intent::Read).await?;
        let readers = self.readers.clone();
        let entity = audit_entity(parent.kind);
        let parent_id = parent.id;
        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            let mut stmt = conn
                .prepare(
                    "SELECT l.action, l.payload_json, l.created_at_utc, u.full_name \
                     FROM audit_log l LEFT JOIN users u ON u.id = l.user_id \
                     WHERE l.entity_type = ?1 AND l.entity_id = ?2 \
                       AND substr(l.action, 1, 18) = 'custom:attachment_' \
                     ORDER BY l.created_at_utc, l.id",
                )
                .map_err(map_rusqlite)?;
            let rows = stmt
                .query_map(rusqlite::params![entity, parent_id], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, Option<String>>(1)?,
                        r.get::<_, i64>(2)?,
                        r.get::<_, Option<String>>(3)?,
                    ))
                })
                .map_err(map_rusqlite)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(map_rusqlite)?;
            let mut out = Vec::new();
            for (action, payload, at, user_name) in rows {
                let v: serde_json::Value = payload
                    .and_then(|p| serde_json::from_str(&p).ok())
                    .unwrap_or(serde_json::Value::Null);
                let Some(att_id) = v.get("attachment_id").and_then(|x| x.as_i64()) else {
                    continue;
                };
                let name = v
                    .get("name")
                    .and_then(|x| x.as_str())
                    .unwrap_or_default()
                    .to_string();
                let deleted: Option<Option<i64>> = conn
                    .query_row(
                        "SELECT deleted_at_utc FROM attachments WHERE id = ?1",
                        rusqlite::params![att_id],
                        |r| r.get(0),
                    )
                    .optional()
                    .map_err(map_rusqlite)?;
                let state = match deleted {
                    None => "purged",
                    Some(Some(_)) => "deleted",
                    Some(None) => "active",
                };
                out.push(AttachmentHistoryEventDto {
                    attachment_id: att_id,
                    name,
                    action: action
                        .strip_prefix("custom:attachment_")
                        .unwrap_or(&action)
                        .to_string(),
                    state: state.to_string(),
                    user_name,
                    created_at_utc: at,
                });
            }
            Ok(out)
        })
        .await
        .map_err(internal)?
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

/// Audit row of an attachment mutation; payload keeps the name snapshot.
#[allow(clippy::too_many_arguments)]
fn insert_audit(
    audit_repo: &SqliteAuditLogRepository,
    tx: &rusqlite::Transaction<'_>,
    entity: &'static str,
    parent_id: i64,
    action: &'static str,
    user_id: Option<i64>,
    attachment_id: i64,
    name: &str,
    now: i64,
) -> Result<(), AppError> {
    audit_repo.insert(
        tx,
        AuditEntry {
            entity_type: entity,
            entity_id: parent_id,
            action,
            user_id,
            before_json: None,
            after_json: None,
            payload_json: Some(
                serde_json::json!({ "attachment_id": attachment_id, "name": name }).to_string(),
            ),
            created_at_utc: now,
        },
    )
}

/// Blocking stage: sniff, optional HEIC conversion, hash, thumbnail.
fn prepare(
    store: &AttachmentStore,
    staged: StagedUpload,
    name: String,
) -> Result<Prepared, AppError> {
    let io = |e: std::io::Error| AppError::Internal {
        source_chain: format!("attachment staging io: {e}"),
    };
    let temp = staged.file;
    let mut head = Vec::with_capacity(HEAD_BYTES);
    std::fs::File::open(temp.path())
        .map_err(io)?
        .take(HEAD_BYTES as u64)
        .read_to_end(&mut head)
        .map_err(io)?;
    let full = if head.starts_with(&[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]) {
        Some(std::fs::read(temp.path()).map_err(io)?)
    } else {
        None
    };
    let sniffed = match attachment_media::sniff(&head, full.as_deref(), &name) {
        Ok(s) => s,
        Err(attachment_media::MediaReject::Empty) => {
            return Ok(Prepared::Rejected(RejectCode::Empty))
        }
        Err(attachment_media::MediaReject::TypeNotAllowed) => {
            return Ok(Prepared::Rejected(RejectCode::TypeNotAllowed))
        }
    };
    let (temp, mime, name) = if sniffed.is_heic {
        let bytes = std::fs::read(temp.path()).map_err(io)?;
        let Ok(jpeg) = attachment_media::heic_to_jpeg(&bytes) else {
            return Ok(Prepared::Rejected(RejectCode::HeicFailed));
        };
        drop(bytes);
        let mut out = store.begin_staging().map_err(io)?;
        out.write_all(&jpeg).map_err(io)?;
        (out, "image/jpeg", with_jpg_ext(&name))
    } else {
        (temp, sniffed.mime, name)
    };
    let hash = attachment_media::sha256_file(temp.path()).map_err(io)?;
    let size = std::fs::metadata(temp.path()).map_err(io)?.len() as i64;
    if PHOTO_MIMES.contains(&mime) {
        // A failed thumbnail never fails the upload (D-11). A concurrent purge
        // may remove it before the row exists: the UI then shows the type icon.
        if let Ok(dest) = store.thumb_path(&hash) {
            if !dest.is_file() {
                if let Some(dir) = dest.parent() {
                    let _ = std::fs::create_dir_all(dir);
                }
                if let Err(e) = attachment_media::make_thumbnail(temp.path(), &dest) {
                    tracing::debug!("attachment thumbnail skipped: {e}");
                }
            }
        }
    }
    Ok(Prepared::Ready {
        temp,
        hash,
        mime,
        size,
        name,
    })
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

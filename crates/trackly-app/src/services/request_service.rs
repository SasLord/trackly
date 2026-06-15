//! `RequestService` — application service for request lifecycle.
//!
//! Single-writer discipline: every mutation goes through
//! `WriterHandle::execute(closure)` with a `BEGIN IMMEDIATE` transaction.
//!
//! WS push (D-Notify-01):
//!   - After `create()` → broadcasts `WsEvent::NewRequest`.
//!   - After `transition()` → broadcasts `WsEvent::RequestStatusChanged`.
//!
//! Status transitions are validated via `RequestTransitionOp::validate_from_status()`
//! (domain rule D-Req-Lifecycle-01).

use std::sync::Arc;

use trackly_core::auth::{authorize, Action, Identity};
use trackly_core::domain::printers::RequestTransitionOp;
use trackly_core::domain::requests::{Pagination, RequestFilter, RequestNew};
use trackly_core::error::AppError;
use trackly_core::ports::requests::RequestRepository;
use trackly_core::primitives::clock::Clock;
use trackly_infra::db::{pools::ReaderPool, writer_worker::WriterHandle};
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::repos::audit_log_sqlite::{AuditEntry, SqliteAuditLogRepository};
use trackly_infra::repos::requests_sqlite::SqliteRequestRepository;

use crate::dto::printer::WsEvent;
use crate::dto::cartridge::AuditEntryDto;
use crate::dto::request::{
    RequestCountsDto, RequestCreateDto, RequestDto, RequestListResponse, RequestTransitionPayload,
};

/// Application service for request lifecycle. `Arc`-fields keep `Clone` O(1).
#[derive(Clone)]
pub struct RequestService {
    pub writer: Arc<WriterHandle>,
    pub readers: Arc<ReaderPool>,
    pub(crate) clock: Arc<dyn Clock + Send + Sync>,
    pub(crate) request_repo: Arc<SqliteRequestRepository>,
    pub(crate) audit_repo: Arc<SqliteAuditLogRepository>,
    /// WS broadcast sender (D-Notify-01).
    pub(crate) ws_tx: Arc<tokio::sync::broadcast::Sender<WsEvent>>,
}

impl RequestService {
    pub fn new(
        writer: Arc<WriterHandle>,
        readers: Arc<ReaderPool>,
        clock: Arc<dyn Clock + Send + Sync>,
        ws_tx: Arc<tokio::sync::broadcast::Sender<WsEvent>>,
    ) -> Self {
        Self {
            writer,
            readers,
            clock,
            request_repo: Arc::new(SqliteRequestRepository),
            audit_repo: Arc::new(SqliteAuditLogRepository),
            ws_tx,
        }
    }

    // -----------------------------------------------------------------------
    // Read paths
    // -----------------------------------------------------------------------

    /// Get a single request by ID.
    pub async fn get(&self, id: i64) -> Result<RequestDto, AppError> {
        let readers = self.readers.clone();
        let repo = self.request_repo.clone();
        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            let row = repo.get(&conn, id)?;
            Ok(RequestDto::from(row))
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking: {e}"),
        })?
    }

    /// List requests (paginated), optionally filtered.
    pub async fn list(
        &self,
        filter: RequestFilter,
        page: Pagination,
    ) -> Result<RequestListResponse, AppError> {
        let readers = self.readers.clone();
        let repo = self.request_repo.clone();
        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            let (rows, total) = repo.list(&conn, &filter, &page)?;
            let items = rows.into_iter().map(RequestDto::from).collect();
            Ok(RequestListResponse {
                items,
                total: total as i64,
            })
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking: {e}"),
        })?
    }

    /// Get aggregate counts for the status switch-bar.
    pub async fn counts(&self) -> Result<RequestCountsDto, AppError> {
        let readers = self.readers.clone();
        let repo = self.request_repo.clone();
        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            let c = repo.counts(&conn)?;
            Ok(RequestCountsDto {
                all: c.all,
                open: c.open,
                in_progress: c.in_progress,
                completed: c.completed,
                rejected: c.rejected,
            })
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking: {e}"),
        })?
    }

    /// Request audit history (REQ-07).
    pub async fn get_history(&self, request_id: i64) -> Result<Vec<AuditEntryDto>, AppError> {
        let readers = self.readers.clone();
        let repo = self.request_repo.clone();
        tokio::task::spawn_blocking(move || -> Result<Vec<AuditEntryDto>, AppError> {
            let conn = readers.acquire();
            let rows = repo.get_history(&conn, request_id)?;
            Ok(rows
                .into_iter()
                .map(|r| AuditEntryDto {
                    id: r.id,
                    action: r.action,
                    payload_json: r.payload_json,
                    before_json: r.before_json,
                    after_json: r.after_json,
                    created_at_utc: r.created_at_utc,
                })
                .collect())
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking: {e}"),
        })?
    }

    // -----------------------------------------------------------------------
    // Write paths
    // -----------------------------------------------------------------------

    /// Create a new request. All roles can create (Action::CreateRequest).
    ///
    /// After successful write, broadcasts `WsEvent::NewRequest` to WS clients.
    pub async fn create(
        &self,
        payload: RequestCreateDto,
        caller: &Identity,
    ) -> Result<RequestDto, AppError> {
        authorize(caller, &Action::CreateRequest)?;
        let now = self.clock.unix_seconds();
        let user_id = caller.user_id;
        let request_repo = self.request_repo.clone();
        let audit_repo = self.audit_repo.clone();

        let new = RequestNew {
            request_type: payload.request_type.clone(),
            requested_by_user_id: user_id.unwrap_or(1), // trusted_admin fallback
            printer_device_id: payload.printer_device_id.map(|id| id as i64),
            cartridge_model_id: payload.cartridge_model_id.map(|id| id as i64),
            category_id: payload.category_id.map(|id| id as i64),
            description: payload.description.clone(),
        };

        let request_id = self
            .writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;
                let id = request_repo.insert_in_tx(&tx, &new, now)?;
                audit_repo.insert(
                    &tx,
                    AuditEntry {
                        entity_type: "request",
                        entity_id: id,
                        action: "create",
                        user_id,
                        before_json: None,
                        after_json: None,
                        payload_json: None,
                        created_at_utc: now,
                    },
                )?;
                tx.commit().map_err(map_rusqlite)?;
                Ok(id)
            })
            .await?;

        let dto = self.get(request_id).await?;

        // WS push after successful create (D-Notify-01).
        let _ = self.ws_tx.send(WsEvent::NewRequest {
            request_id,
            request_type: dto.request_type.clone(),
            requester_name: dto.requester_name.clone().unwrap_or_default(),
        });

        Ok(dto)
    }

    /// Apply a lifecycle transition to a request.
    ///
    /// - Admin/Manager can Accept, Reject, Complete.
    /// - Domain rule enforced: `validate_from_status()`.
    /// - After successful write, broadcasts `WsEvent::RequestStatusChanged`.
    pub async fn transition(
        &self,
        payload: RequestTransitionPayload,
        caller: &Identity,
    ) -> Result<RequestDto, AppError> {
        authorize(caller, &Action::TransitionRequests)?;
        let now = self.clock.unix_seconds();
        let user_id = caller.user_id;
        let request_repo = self.request_repo.clone();
        let audit_repo = self.audit_repo.clone();

        let (request_id, version, op, assigned_to, linked_cartridge_id) = match &payload {
            RequestTransitionPayload::Accept {
                request_id,
                version,
                assigned_to_user_id,
            } => (
                *request_id,
                *version,
                RequestTransitionOp::Accept,
                assigned_to_user_id.map(|id| id as i64),
                None,
            ),
            RequestTransitionPayload::Reject {
                request_id,
                version,
                notes,
            } => (
                *request_id,
                *version,
                RequestTransitionOp::Reject {
                    notes: notes.clone(),
                },
                None,
                None,
            ),
            RequestTransitionPayload::Complete {
                request_id,
                version,
                notes,
                linked_cartridge_id,
            } => (
                *request_id,
                *version,
                RequestTransitionOp::Complete {
                    notes: notes.clone(),
                    linked_cartridge_id: linked_cartridge_id.map(|id| id as i64),
                },
                None,
                linked_cartridge_id.map(|id| id as i64),
            ),
        };

        let new_status = op.target_status().to_string();

        self.writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;
                request_repo.transition_in_tx(
                    &tx,
                    request_id,
                    version,
                    &op,
                    assigned_to,
                    linked_cartridge_id,
                    now,
                )?;
                audit_repo.insert(
                    &tx,
                    AuditEntry {
                        entity_type: "request",
                        entity_id: request_id,
                        action: op.audit_action(),
                        user_id,
                        before_json: None,
                        after_json: None,
                        payload_json: None,
                        created_at_utc: now,
                    },
                )?;
                tx.commit().map_err(map_rusqlite)?;
                Ok(())
            })
            .await?;

        let dto = self.get(request_id).await?;

        // WS push after successful transition (D-Notify-01).
        // NOTE: RequestStatusChanged — NOT RequestUpdated (06-CONTEXT sync).
        let _ = self.ws_tx.send(WsEvent::RequestStatusChanged {
            request_id,
            new_status,
        });

        Ok(dto)
    }
}

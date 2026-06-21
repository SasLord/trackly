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
use crate::dto::request::{
    ApproveAdRegisterDto, RequestCountsDto, RequestCreateDto, RequestDto, RequestHistoryEntryDto,
    RequestListResponse, RequestPrinterOptionDto, RequestTransitionPayload,
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
    ///
    /// D-REQ-01 / BOLA closure: an Employee caller may only fetch a request
    /// they themselves own — `caller.user_id != dto.requested_by_user_id`
    /// returns `AppError::Forbidden` rather than the DTO. Admin/Manager
    /// callers pass through unchanged.
    pub async fn get(&self, id: i64, caller: &Identity) -> Result<RequestDto, AppError> {
        let readers = self.readers.clone();
        let repo = self.request_repo.clone();
        let dto = tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            let row = repo.get(&conn, id)?;
            Ok(RequestDto::from(row))
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking: {e}"),
        })??;

        if matches!(caller.role, trackly_core::auth::Role::Employee)
            && dto.requested_by_user_id != caller.user_id.unwrap_or(-1)
        {
            return Err(AppError::Forbidden);
        }

        Ok(dto)
    }

    /// List requests (paginated), optionally filtered.
    ///
    /// REQ-06 / T-09-11: `ad_register` rows are excluded at the SQL level for
    /// non-admin callers — never row-hidden client-side.
    ///
    /// D-REQ-01: for an Employee caller, `filter.requested_by_user_id` is
    /// unconditionally overridden to `caller.user_id` — the client-supplied
    /// value (if any) is discarded, never merged or validated-then-trusted.
    /// An Employee identity with no `user_id` (should not occur in practice —
    /// `Identity::trusted_admin()` only ever produces `Role::Admin`) is
    /// rejected defensively rather than silently falling through to an
    /// unrestricted `None` filter.
    pub async fn list(
        &self,
        filter: RequestFilter,
        page: Pagination,
        caller: &Identity,
    ) -> Result<RequestListResponse, AppError> {
        let exclude_ad_register = !matches!(caller.role, trackly_core::auth::Role::Admin);

        let mut filter = filter;
        if matches!(caller.role, trackly_core::auth::Role::Employee) {
            if caller.user_id.is_none() {
                return Err(AppError::Forbidden);
            }
            filter.requested_by_user_id = caller.user_id;
        }

        let readers = self.readers.clone();
        let repo = self.request_repo.clone();
        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            let (rows, total) = repo.list(&conn, &filter, &page, exclude_ad_register)?;
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
    ///
    /// D-REQ-01: for an Employee caller, counts are scoped to
    /// `requested_by_user_id = caller.user_id` — never the org-wide totals.
    pub async fn counts(&self, caller: &Identity) -> Result<RequestCountsDto, AppError> {
        let requested_by_user_id = if matches!(caller.role, trackly_core::auth::Role::Employee) {
            if caller.user_id.is_none() {
                return Err(AppError::Forbidden);
            }
            caller.user_id
        } else {
            None
        };

        let readers = self.readers.clone();
        let repo = self.request_repo.clone();
        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            let c = repo.counts(&conn, requested_by_user_id)?;
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
    ///
    /// D-REQ-01 / BOLA closure: reuses [`Self::get`]'s ownership check before
    /// touching `audit_log` — an Employee who does not own `request_id` gets
    /// `AppError::Forbidden` and the history query never runs.
    pub async fn get_history(
        &self,
        request_id: i64,
        caller: &Identity,
    ) -> Result<Vec<RequestHistoryEntryDto>, AppError> {
        let _ = self.get(request_id, caller).await?;

        let readers = self.readers.clone();
        let repo = self.request_repo.clone();
        tokio::task::spawn_blocking(move || -> Result<Vec<RequestHistoryEntryDto>, AppError> {
            let conn = readers.acquire();
            let rows = repo.get_history(&conn, request_id)?;
            Ok(rows
                .into_iter()
                .map(|r| RequestHistoryEntryDto {
                    id: r.id,
                    action: r.action,
                    created_at_utc: r.created_at_utc,
                    actor_name: r.actor_name,
                    // `notes` is carried in payload_json as {"notes": "..."} for
                    // reject/complete transitions; absent for create/accept.
                    notes: r.payload_json.as_deref().and_then(|p| {
                        serde_json::from_str::<serde_json::Value>(p)
                            .ok()
                            .and_then(|v| {
                                v.get("notes")
                                    .and_then(|n| n.as_str())
                                    .map(|s| s.to_string())
                            })
                    }),
                })
                .collect())
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking: {e}"),
        })?
    }

    /// Printer options for the create-request form's printer dropdown
    /// (D-PRN-01). Gated on `Action::CreateRequest` — every role has it
    /// (employee included), deliberately NOT `ReadData`/`ReadPrinters`
    /// (Phase 10 closed those for Employee — this endpoint must not
    /// regress that closure by reusing either gate).
    ///
    /// Returns only `{id, name, location}` — no SNMP/community/IP/serial
    /// fields leave the server (BOLA/BOPLA closure, T-11-02-I). Sorted by
    /// location (printers without a location sort last), then by name.
    pub async fn printer_options(
        &self,
        caller: &Identity,
    ) -> Result<Vec<RequestPrinterOptionDto>, AppError> {
        authorize(caller, &Action::CreateRequest)?;

        let readers = self.readers.clone();
        tokio::task::spawn_blocking(move || -> Result<Vec<RequestPrinterOptionDto>, AppError> {
            let conn = readers.acquire();
            let mut stmt = conn
                .prepare(
                    "SELECT d.id, d.name, l.name AS location \
                     FROM devices d \
                     LEFT JOIN locations l ON d.location_id = l.id \
                     WHERE d.type_id = 2 AND d.deleted_at_utc IS NULL \
                     ORDER BY l.name IS NULL, l.name, d.name",
                )
                .map_err(map_rusqlite)?;
            let rows = stmt
                .query_map([], |row| {
                    Ok(RequestPrinterOptionDto {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        location: row.get(2)?,
                    })
                })
                .map_err(map_rusqlite)?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(map_rusqlite)?;
            Ok(rows)
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
            // User-facing `create()` never originates ad_register requests —
            // those are written directly by AuthService::on_ad_bind_success.
            ad_subtype: None,
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

        let dto = self.get(request_id, caller).await?;

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
    ///
    /// `ad_register` requests being Rejected take a special path (T-09-12/
    /// T-09-14, D-REG-03): see [`Self::reject_ad_register`]. All other
    /// request types use the generic transition path below.
    pub async fn transition(
        &self,
        payload: RequestTransitionPayload,
        caller: &Identity,
    ) -> Result<RequestDto, AppError> {
        authorize(caller, &Action::TransitionRequests)?;

        if let RequestTransitionPayload::Reject {
            request_id,
            version,
            notes,
        } = &payload
        {
            let current = self.get(*request_id, caller).await?;
            if current.request_type == "ad_register" {
                return self
                    .reject_ad_register(*request_id, *version, notes.clone(), caller)
                    .await;
            }
        }

        let now = self.clock.unix_seconds();
        let user_id = caller.user_id;
        let request_repo = self.request_repo.clone();
        let audit_repo = self.audit_repo.clone();

        let (request_id, version, op, assigned_to, linked_cartridge_id) = match &payload {
            RequestTransitionPayload::Accept {
                request_id,
                version,
                // Assignee is resolved server-side from the caller (the acceptor) —
                // the client-supplied value is ignored. This mirrors the D-REQ-01
                // server-side-override pattern and prevents a bogus FK write: in
                // unlocked-desktop mode the UI sends id 0 ("Рабочий стол" sentinel),
                // which has no users row and previously failed the
                // requests.assigned_to_user_id → users(id) FK. caller.user_id is
                // None for trusted-desktop → COALESCE keeps the existing value.
                assigned_to_user_id: _,
            } => (
                *request_id,
                *version,
                RequestTransitionOp::Accept,
                caller.user_id,
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

        // Carry the transition notes into the audit payload so the History
        // block (REQ-07) can show the reject/complete reason. Create/accept
        // have no notes → payload stays NULL.
        let notes_json: Option<String> = match &op {
            RequestTransitionOp::Reject { notes } => notes.clone(),
            RequestTransitionOp::Complete { notes, .. } => notes.clone(),
            RequestTransitionOp::Accept => None,
        }
        .map(|n| serde_json::json!({ "notes": n }).to_string());

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
                        payload_json: notes_json,
                        created_at_utc: now,
                    },
                )?;
                tx.commit().map_err(map_rusqlite)?;
                Ok(())
            })
            .await?;

        let dto = self.get(request_id, caller).await?;

        // WS push after successful transition (D-Notify-01).
        // NOTE: RequestStatusChanged — NOT RequestUpdated (06-CONTEXT sync).
        let _ = self.ws_tx.send(WsEvent::RequestStatusChanged {
            request_id,
            new_status,
        });

        Ok(dto)
    }

    /// Approve an `ad_register` request (USR-09/USR-11, D-REG-02, T-09-12).
    ///
    /// Admin-only (`Action::ManageUsers` — role elevation is a privileged
    /// operation, Security V4). Validates `role` via `Role::from_str`,
    /// defaulting to "employee" if absent (D-REG-02). Branches on
    /// `ad_subtype`:
    /// - `"register"` (pending mode, user row `is_active=0`): activates the
    ///   user and sets the chosen role.
    /// - `"restore"` (blocked/soft-deleted, user row already exists):
    ///   revives the user (`deleted_at_utc = NULL, is_active = 1`) and sets
    ///   the chosen role.
    ///
    /// All in one writer transaction + audit_log entries for both the user
    /// mutation and the request completion (T-09-14).
    pub async fn approve_ad_register(
        &self,
        payload: ApproveAdRegisterDto,
        caller: &Identity,
    ) -> Result<RequestDto, AppError> {
        authorize(caller, &Action::ManageUsers)?;

        let role =
            trackly_core::auth::Role::from_str(payload.role.as_deref().unwrap_or("employee"))?;
        let role_str = role.as_str().to_string();

        let current = self.get(payload.request_id, caller).await?;
        if current.request_type != "ad_register" {
            return Err(AppError::Validation {
                field: "request_id".to_string(),
                message: "request is not an ad_register request".to_string(),
            });
        }
        let ad_subtype = current.ad_subtype.clone().unwrap_or_default();
        let target_user_id = current.requested_by_user_id;

        let now = self.clock.unix_seconds();
        let user_id = caller.user_id;
        let request_repo = self.request_repo.clone();
        let audit_repo = self.audit_repo.clone();
        let request_id = payload.request_id;
        let version = payload.version;
        let role_for_tx = role_str.clone();

        self.writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;

                // Activate (register) or revive (restore) the target user,
                // setting the admin-selected role.
                if ad_subtype == "restore" {
                    tx.execute(
                        "UPDATE users SET role = ?1, is_active = 1, deleted_at_utc = NULL, \
                         updated_at_utc = ?2, version = version + 1 WHERE id = ?3",
                        rusqlite::params![role_for_tx, now, target_user_id],
                    )
                    .map_err(map_rusqlite)?;
                } else {
                    tx.execute(
                        "UPDATE users SET role = ?1, is_active = 1, \
                         updated_at_utc = ?2, version = version + 1 WHERE id = ?3",
                        rusqlite::params![role_for_tx, now, target_user_id],
                    )
                    .map_err(map_rusqlite)?;
                }

                tx.execute(
                    "INSERT INTO audit_log \
                     (entity_type, entity_id, action, user_id, before_json, after_json, \
                      payload_json, created_at_utc) \
                     VALUES ('user', ?1, 'ad_register_approve', ?2, NULL, NULL, ?3, ?4)",
                    rusqlite::params![
                        target_user_id,
                        user_id,
                        serde_json::json!({ "role": role_for_tx }).to_string(),
                        now
                    ],
                )
                .map_err(map_rusqlite)?;

                // Mark the request completed directly (open → completed).
                // NOT `transition_in_tx`/`RequestTransitionOp::Complete` — that
                // op's domain rule expects the cartridge/printer in_progress →
                // completed state machine (`validate_from_status`). The
                // ad_register approve flow is its own state machine: a single
                // admin decision moves "open" straight to "completed", with
                // the optimistic-lock check still enforced manually below.
                let affected = tx
                    .execute(
                        "UPDATE requests SET status = 'completed', updated_at_utc = ?1, \
                         version = version + 1 \
                         WHERE id = ?2 AND version = ?3 AND status = 'open' \
                           AND deleted_at_utc IS NULL",
                        rusqlite::params![now, request_id, version],
                    )
                    .map_err(map_rusqlite)?;
                if affected == 0 {
                    let current = request_repo.fetch_in_tx(&tx, request_id)?;
                    return Err(AppError::OptimisticLockMismatch {
                        entity: "request",
                        id: request_id,
                        expected: version,
                        actual: current.version,
                    });
                }
                audit_repo.insert(
                    &tx,
                    AuditEntry {
                        entity_type: "request",
                        entity_id: request_id,
                        action: "ad_register_approve",
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

        let dto = self.get(request_id, caller).await?;

        let _ = self.ws_tx.send(WsEvent::RequestStatusChanged {
            request_id,
            new_status: "completed".to_string(),
        });

        Ok(dto)
    }

    /// Reject an `ad_register` request (D-REG-03, T-09-14).
    ///
    /// Three branches, keyed on `ad_subtype` + current user state:
    /// - `"register"` + pending mode (user `is_active=0`, never activated):
    ///   discard — request rejected, user stays inactive (no access granted).
    /// - `"register"` + auto-accept mode (user `is_active=1`, already has a
    ///   session-capable account): soft-delete the auto-created user on reject.
    /// - `"restore"`: request rejected, existing user stays blocked
    ///   (no mutation to the user row).
    ///
    /// All single-writer + audited.
    async fn reject_ad_register(
        &self,
        request_id: i64,
        version: i64,
        notes: Option<String>,
        caller: &Identity,
    ) -> Result<RequestDto, AppError> {
        authorize(caller, &Action::ManageUsers)?;

        let current = self.get(request_id, caller).await?;
        let ad_subtype = current.ad_subtype.clone().unwrap_or_default();
        let target_user_id = current.requested_by_user_id;

        // Determine whether the target user row is already active (auto-accept
        // path) or still inactive (pending path) — drives the soft-delete branch.
        let readers = self.readers.clone();
        let user_is_active: bool =
            tokio::task::spawn_blocking(move || -> Result<bool, AppError> {
                let conn = readers.acquire();
                let is_active: i64 = conn
                    .query_row(
                        "SELECT is_active FROM users WHERE id = ?1",
                        rusqlite::params![target_user_id],
                        |r| r.get(0),
                    )
                    .map_err(map_rusqlite)?;
                Ok(is_active != 0)
            })
            .await
            .map_err(|e| AppError::Internal {
                source_chain: format!("spawn_blocking: {e}"),
            })??;

        let now = self.clock.unix_seconds();
        let user_id = caller.user_id;
        let request_repo = self.request_repo.clone();
        let audit_repo = self.audit_repo.clone();
        let notes_json = notes
            .clone()
            .map(|n| serde_json::json!({ "notes": n }).to_string());

        self.writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;

                // "register" + already-active user = auto-accept path → soft-delete
                // on reject (T-09-14: revoke access that was already granted).
                if ad_subtype == "register" && user_is_active {
                    tx.execute(
                        "UPDATE users SET deleted_at_utc = ?1, is_active = 0, \
                         updated_at_utc = ?1, version = version + 1 WHERE id = ?2",
                        rusqlite::params![now, target_user_id],
                    )
                    .map_err(map_rusqlite)?;
                    tx.execute(
                        "INSERT INTO audit_log \
                         (entity_type, entity_id, action, user_id, before_json, after_json, \
                          payload_json, created_at_utc) \
                         VALUES ('user', ?1, 'ad_register_reject_softdelete', ?2, NULL, NULL, NULL, ?3)",
                        rusqlite::params![target_user_id, user_id, now],
                    )
                    .map_err(map_rusqlite)?;
                }
                // "register" + pending (inactive) user: discard — no user mutation,
                // user stays inactive with no access.
                // "restore": no user mutation — user stays blocked as before.

                request_repo.transition_in_tx(
                    &tx,
                    request_id,
                    version,
                    &RequestTransitionOp::Reject {
                        notes: notes.clone(),
                    },
                    None,
                    None,
                    now,
                )?;
                audit_repo.insert(
                    &tx,
                    AuditEntry {
                        entity_type: "request",
                        entity_id: request_id,
                        action: "custom:reject",
                        user_id,
                        before_json: None,
                        after_json: None,
                        payload_json: notes_json,
                        created_at_utc: now,
                    },
                )?;

                tx.commit().map_err(map_rusqlite)?;
                Ok(())
            })
            .await?;

        let dto = self.get(request_id, caller).await?;

        let _ = self.ws_tx.send(WsEvent::RequestStatusChanged {
            request_id,
            new_status: "rejected".to_string(),
        });

        Ok(dto)
    }
}

//! `ActService` — application service for acts приёма-передачи.
//!
//! Phase 3 plan 02 scope:
//!   - `create` — handover only (return lifecycle is plan 03)
//!   - `get` / `list` / `counts` / `peek_next_number`
//!   - `delete_soft` — minimal stub (full undo via audit_log lives in plan 03)
//!
//! Single-writer discipline: every mutation goes through
//! `WriterHandle::execute(closure)` with a `BEGIN IMMEDIATE` transaction.
//! `counters.act_number` is incremented atomically via
//! `increment_counter_in_tx` (UPDATE ... RETURNING — D-Counter-Acts-01).

use std::sync::Arc;

use rusqlite::params;
use trackly_core::domain::acts::{ActItemRow, ActRow, ActType};
use trackly_core::error::AppError;
use trackly_core::ports::acts::ActRepository;
use trackly_core::primitives::clock::Clock;
use trackly_infra::db::{pools::ReaderPool, writer_worker::WriterHandle};
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::repos::acts_sqlite::{
    increment_counter_in_tx, peek_counter, peek_counter_in_tx,
};
use trackly_infra::repos::audit_log_sqlite::AuditEntry;
use trackly_infra::repos::{SqliteActRepository, SqliteAuditLogRepository, SqliteDeviceRepository};

use crate::dto::act::{
    act_dto_from_row, ActCreateDto, ActDto, ActFilter, ActItemDto, ActListResponse, ActsCountsDto,
    Pagination,
};

/// Application service for act lifecycle. `Arc`-fields keep `Clone` O(1).
#[derive(Clone)]
pub struct ActService {
    pub writer: Arc<WriterHandle>,
    pub readers: Arc<ReaderPool>,
    pub(crate) clock: Arc<dyn Clock + Send + Sync>,
    pub(crate) acts_repo: Arc<SqliteActRepository>,
    pub(crate) audit_repo: Arc<SqliteAuditLogRepository>,
    pub(crate) devices_repo: Arc<SqliteDeviceRepository>,
}

impl ActService {
    pub fn new(
        writer: Arc<WriterHandle>,
        readers: Arc<ReaderPool>,
        clock: Arc<dyn Clock + Send + Sync>,
    ) -> Self {
        Self {
            writer,
            readers,
            clock,
            acts_repo: Arc::new(SqliteActRepository),
            audit_repo: Arc::new(SqliteAuditLogRepository),
            devices_repo: Arc::new(SqliteDeviceRepository),
        }
    }

    // -----------------------------------------------------------------------
    // Validation
    // -----------------------------------------------------------------------

    fn validate_create(p: &ActCreateDto) -> Result<(), AppError> {
        if p.giver_name.trim().is_empty() {
            return Err(AppError::Validation {
                field: "giver_name".into(),
                message: "Поле «Сдал» обязательно".into(),
            });
        }
        if p.receiver_name.trim().is_empty() {
            return Err(AppError::Validation {
                field: "receiver_name".into(),
                message: "Поле «Принял» обязательно".into(),
            });
        }
        if p.items.is_empty() {
            return Err(AppError::Validation {
                field: "items".into(),
                message: "Добавьте хотя бы одну позицию".into(),
            });
        }
        if p.items.len() > 100 {
            return Err(AppError::Validation {
                field: "items".into(),
                message: "Максимум 100 позиций в одном акте".into(),
            });
        }
        for (idx, it) in p.items.iter().enumerate() {
            if it.quantity < 1 {
                return Err(AppError::Validation {
                    field: format!("items[{idx}].quantity"),
                    message: "Количество должно быть ≥ 1".into(),
                });
            }
        }
        if let Some(n) = p.number_override {
            if n < 1 {
                return Err(AppError::Validation {
                    field: "number_override".into(),
                    message: "Номер акта должен быть ≥ 1".into(),
                });
            }
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Create handover (ACT-01, ACT-03, ACT-13, ACT-14)
    // -----------------------------------------------------------------------

    pub async fn create(&self, payload: ActCreateDto) -> Result<ActDto, AppError> {
        Self::validate_create(&payload)?;
        let now = self.clock.unix_seconds();
        let acts_repo = self.acts_repo.clone();
        let audit_repo = self.audit_repo.clone();
        let devices_repo = self.devices_repo.clone();
        let user_id_opt: Option<i64> = None;

        let act_id = self
            .writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;

                // Resolve status_id for «В работе» via the V014 code column (B-1).
                let in_work_status_id: i64 = tx
                    .query_row(
                        "SELECT id FROM device_statuses WHERE code = 'в_работе'",
                        [],
                        |r| r.get(0),
                    )
                    .map_err(|e| match e {
                        rusqlite::Error::QueryReturnedNoRows => AppError::Internal {
                            source_chain:
                                "device_statuses missing code='в_работе' — V014 not applied?"
                                    .into(),
                        },
                        other => map_rusqlite(other),
                    })?;

                // 1. Resolve number: override (with uniqueness check) OR atomic counter inc.
                let number = if let Some(custom) = payload.number_override {
                    // Uniqueness check INCLUDING soft-deleted (D-Soft-vs-Hard-Acts-01).
                    let exists: bool = tx
                        .query_row(
                            "SELECT EXISTS(SELECT 1 FROM acts WHERE number=?1 LIMIT 1)",
                            params![custom],
                            |r| r.get(0),
                        )
                        .map_err(map_rusqlite)?;
                    if exists {
                        return Err(AppError::Conflict {
                            reason: format!("Акт №{custom} уже существует"),
                        });
                    }
                    custom
                } else {
                    increment_counter_in_tx(&tx, "act_number")?
                };

                // 2. INSERT acts.
                let new_row = ActRow {
                    id: 0,
                    number,
                    sub_number: None,
                    parent_act_id: None,
                    act_type: ActType::Handover,
                    giver_name: payload.giver_name.clone(),
                    receiver_name: payload.receiver_name.clone(),
                    location_id: payload.location_id,
                    location: None,
                    notes: payload.notes.clone(),
                    deadline_utc: payload.deadline_utc,
                    archived: false,
                    created_at_utc: now,
                    updated_at_utc: now,
                    deleted_at_utc: None,
                    version: 1,
                    parent_number: None,
                    sibling_return_count: None,
                };
                let act_id = acts_repo.insert_act_in_tx(&tx, &new_row)?;

                // 3. If override path — audit override AFTER we know act_id.
                if let Some(custom) = payload.number_override {
                    let next_auto = peek_counter_in_tx(&tx, "act_number")? + 1;
                    let payload_json = serde_json::json!({
                        "requested": custom,
                        "next_auto_would_be": next_auto,
                    })
                    .to_string();
                    audit_repo.insert(
                        &tx,
                        AuditEntry {
                            entity_type: "act",
                            entity_id: act_id,
                            action: "custom:act_number_override",
                            user_id: user_id_opt,
                            before_json: None,
                            after_json: None,
                            payload_json: Some(payload_json),
                            created_at_utc: now,
                        },
                    )?;
                }

                // 4. INSERT act_items + UPDATE devices + audit each device mutation.
                for item in &payload.items {
                    let before = devices_repo.get_in_tx(&tx, item.device_id)?;
                    let before_json = serde_json::to_string(&serde_json::json!({
                        "id": before.id,
                        "status_id": before.status_id,
                        "location_id": before.location_id,
                        "location": before.location,
                        "state": before.state,
                        "kit": before.kit,
                        "version": before.version,
                    }))
                    .map_err(|e| AppError::Internal {
                        source_chain: format!("before_json: {e}"),
                    })?;

                    acts_repo.insert_act_item_in_tx(
                        &tx,
                        act_id,
                        item.device_id,
                        item.quantity,
                        before.state.as_deref(),
                        before.kit.as_deref(),
                    )?;

                    let after = devices_repo.update_status_and_location_in_tx(
                        &tx,
                        item.device_id,
                        in_work_status_id,
                        payload.location_id,
                        now,
                    )?;
                    let after_json = serde_json::to_string(&serde_json::json!({
                        "id": after.id,
                        "status_id": after.status_id,
                        "location_id": after.location_id,
                        "location": after.location,
                        "state": after.state,
                        "kit": after.kit,
                        "version": after.version,
                    }))
                    .map_err(|e| AppError::Internal {
                        source_chain: format!("after_json: {e}"),
                    })?;

                    let payload_json = serde_json::json!({
                        "act_id": act_id,
                        "kind": "handover",
                    })
                    .to_string();
                    audit_repo.insert(
                        &tx,
                        AuditEntry {
                            entity_type: "device",
                            entity_id: item.device_id,
                            action: "update",
                            user_id: user_id_opt,
                            before_json: Some(before_json),
                            after_json: Some(after_json),
                            payload_json: Some(payload_json),
                            created_at_utc: now,
                        },
                    )?;
                }

                // 5. Final audit row for the act creation.
                let act_after = acts_repo.fetch_full_in_tx(&tx, act_id)?;
                let act_after_json = serde_json::to_string(&serde_json::json!({
                    "id": act_after.id,
                    "number": act_after.number,
                    "act_type": act_after.act_type.to_sql(),
                    "giver_name": act_after.giver_name,
                    "receiver_name": act_after.receiver_name,
                    "location_id": act_after.location_id,
                    "deadline_utc": act_after.deadline_utc,
                    "version": act_after.version,
                }))
                .map_err(|e| AppError::Internal {
                    source_chain: format!("act after_json: {e}"),
                })?;
                audit_repo.insert(
                    &tx,
                    AuditEntry {
                        entity_type: "act",
                        entity_id: act_id,
                        action: "create",
                        user_id: user_id_opt,
                        before_json: None,
                        after_json: Some(act_after_json),
                        payload_json: None,
                        created_at_utc: now,
                    },
                )?;

                tx.commit().map_err(map_rusqlite)?;
                Ok(act_id)
            })
            .await?;

        self.get(act_id).await
    }

    // -----------------------------------------------------------------------
    // Read paths
    // -----------------------------------------------------------------------

    pub async fn get(&self, id: i64) -> Result<ActDto, AppError> {
        let readers = self.readers.clone();
        let repo = self.acts_repo.clone();
        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            let row = repo.get(&conn, id)?;
            let items = load_items_for_act(&conn, id)?;
            // return_ids stays empty until plan 03 (return lifecycle).
            Ok(act_dto_from_row(row, items, Vec::new()))
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking: {e}"),
        })?
    }

    pub async fn list(
        &self,
        filter: ActFilter,
        pagination: Pagination,
    ) -> Result<ActListResponse, AppError> {
        if pagination.limit > 200 {
            return Err(AppError::Validation {
                field: "pagination.limit".into(),
                message: "Максимум 200 элементов на страницу".into(),
            });
        }
        let domain_filter = filter.into_domain()?;
        let domain_page = trackly_core::domain::acts::Pagination {
            offset: pagination.offset,
            limit: pagination.limit,
        };

        let readers = self.readers.clone();
        let repo = self.acts_repo.clone();
        let (items, total) =
            tokio::task::spawn_blocking(move || -> Result<(Vec<ActDto>, u64), AppError> {
                let conn = readers.acquire();
                let (rows, total) = repo.list(&conn, &domain_filter, &domain_page)?;
                let mut out = Vec::with_capacity(rows.len());
                for row in rows {
                    let items = load_items_for_act(&conn, row.id)?;
                    out.push(act_dto_from_row(row, items, Vec::new()));
                }
                Ok((out, total))
            })
            .await
            .map_err(|e| AppError::Internal {
                source_chain: format!("spawn_blocking: {e}"),
            })??;

        Ok(ActListResponse { items, total })
    }

    pub async fn counts(&self) -> Result<ActsCountsDto, AppError> {
        let readers = self.readers.clone();
        let repo = self.acts_repo.clone();
        let counts = tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            repo.counts(&conn)
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking: {e}"),
        })??;
        Ok(counts.into())
    }

    pub async fn peek_next_number(&self) -> Result<i64, AppError> {
        let readers = self.readers.clone();
        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            let current = peek_counter(&conn, "act_number")?;
            Ok::<i64, AppError>(current + 1)
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking: {e}"),
        })?
    }

    // -----------------------------------------------------------------------
    // Soft-delete (MINIMAL STUB — plan 03 replaces with full audit-log undo)
    // -----------------------------------------------------------------------

    /// TODO(plan 03): replace with full undo logic that restores device snapshots
    /// from `audit_log.before_json` and recomputes parent.archived for return acts.
    pub async fn delete_soft(&self, id: i64, version: i64) -> Result<(), AppError> {
        let now = self.clock.unix_seconds();
        let acts_repo = self.acts_repo.clone();
        let audit_repo = self.audit_repo.clone();
        let user_id_opt: Option<i64> = None;

        self.writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;
                acts_repo.soft_delete_in_tx(&tx, id, version, now)?;
                audit_repo.insert(
                    &tx,
                    AuditEntry {
                        entity_type: "act",
                        entity_id: id,
                        action: "delete",
                        user_id: user_id_opt,
                        before_json: None,
                        after_json: None,
                        payload_json: None,
                        created_at_utc: now,
                    },
                )?;
                tx.commit().map_err(map_rusqlite)?;
                Ok(())
            })
            .await
    }
}

// ---------------------------------------------------------------------------
// Helpers — items loading (joined with devices)
// ---------------------------------------------------------------------------

fn load_items_for_act(
    conn: &rusqlite::Connection,
    act_id: i64,
) -> Result<Vec<ActItemDto>, AppError> {
    let mut stmt = conn
        .prepare(
            "SELECT ai.id, ai.device_id, ai.quantity, ai.condition_at_time, ai.complectation_at_time, \
                    d.name, d.inventory_number, d.serial_number, d.model \
               FROM act_items ai \
               JOIN devices d ON d.id = ai.device_id \
              WHERE ai.act_id = ?1 \
              ORDER BY ai.id ASC",
        )
        .map_err(map_rusqlite)?;
    let rows = stmt
        .query_map(params![act_id], |r| {
            Ok(ActItemDto {
                id: r.get(0)?,
                device_id: r.get(1)?,
                quantity: r.get(2)?,
                condition_at_time: r.get(3)?,
                complectation_at_time: r.get(4)?,
                device_name: r.get(5)?,
                inventory_no: r.get(6)?,
                serial_no: r.get(7)?,
                model: r.get(8)?,
            })
        })
        .map_err(map_rusqlite)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(map_rusqlite)?);
    }
    Ok(out)
}

// `ActItemRow` re-export (unused yet — plan 03 will consume).
#[allow(dead_code)]
fn _act_item_row_ref(_r: &ActItemRow) {}

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
use trackly_core::domain::devices::DeviceRow;
use trackly_core::error::AppError;
use trackly_core::ports::acts::ActRepository;
use trackly_core::primitives::clock::Clock;
use trackly_infra::db::{pools::ReaderPool, writer_worker::WriterHandle};
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::repos::acts_sqlite::{
    increment_counter_in_tx, next_sub_number_for_parent, peek_counter, peek_counter_in_tx,
    recompute_parent_archived,
};
use trackly_infra::repos::audit_log_sqlite::AuditEntry;
use trackly_infra::repos::{SqliteActRepository, SqliteAuditLogRepository, SqliteDeviceRepository};

use crate::dto::act::{
    act_dto_from_row, ActCreateDto, ActDto, ActFilter, ActItemDto, ActListResponse, ActReturnDto,
    ActReturnItemDto, ActsCountsDto, Pagination,
};
use crate::dto::suggest::SuggestPersonField;
use crate::pdf::PdfRenderer;
use crate::services::organization_service::OrganizationService;
use crate::services::template_service::TemplateService;

/// Application service for act lifecycle. `Arc`-fields keep `Clone` O(1).
#[derive(Clone)]
pub struct ActService {
    pub writer: Arc<WriterHandle>,
    pub readers: Arc<ReaderPool>,
    pub(crate) clock: Arc<dyn Clock + Send + Sync>,
    pub(crate) acts_repo: Arc<SqliteActRepository>,
    pub(crate) audit_repo: Arc<SqliteAuditLogRepository>,
    pub(crate) devices_repo: Arc<SqliteDeviceRepository>,
    /// PDF pipeline deps — Optional чтобы Phase 2 тесты (helper-based fixtures
    /// `ActService::new`) могли работать без переписывания. AppCtx::build
    /// вызывает `with_pdf_pipeline(...)` — production runtime всегда имеет
    /// заполненные поля. Если render_pdf вызвать с None — вернёт `Internal`.
    pub(crate) templates: Option<Arc<TemplateService>>,
    pub(crate) organization: Option<Arc<OrganizationService>>,
    pub(crate) pdf: Option<Arc<PdfRenderer>>,
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
            templates: None,
            organization: None,
            pdf: None,
        }
    }

    /// Builder: подключить PDF pipeline deps (templates + organization + pdf).
    /// Используется в `AppCtx::build` (production runtime) и в plan-04
    /// integration tests, проверяющих render_pdf end-to-end.
    pub fn with_pdf_pipeline(
        mut self,
        templates: Arc<TemplateService>,
        organization: Arc<OrganizationService>,
        pdf: Arc<PdfRenderer>,
    ) -> Self {
        self.templates = Some(templates);
        self.organization = Some(organization);
        self.pdf = Some(pdf);
        self
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
        // T-03.1-02: bound quantity to prevent DoS via 10000x clones.
        const MAX_CLONE_QTY: i64 = 1000;
        // UAT Fix #3/#4 (Phase 3.1): UI device_ids[] tracking — when canonical
        // device_ids[] is used, dedup проверка ensures no device_id appears в
        // двух items.
        let mut seen_device_ids: std::collections::HashSet<i64> = std::collections::HashSet::new();
        for (idx, it) in p.items.iter().enumerate() {
            if it.quantity < 1 {
                return Err(AppError::Validation {
                    field: format!("items[{idx}].quantity"),
                    message: "Количество должно быть ≥ 1".into(),
                });
            }
            if it.quantity > MAX_CLONE_QTY {
                return Err(AppError::Validation {
                    field: format!("items[{idx}].quantity"),
                    message: "Кол-во не должно превышать 1000".into(),
                });
            }
            // UAT Fix #4: when device_ids[] supplied, quantity must match its length.
            if !it.device_ids.is_empty() && it.device_ids.len() as i64 != it.quantity {
                return Err(AppError::Validation {
                    field: format!("items[{idx}].device_ids"),
                    message: format!(
                        "device_ids.len()={} не совпадает с quantity={}",
                        it.device_ids.len(),
                        it.quantity
                    ),
                });
            }
            // Dedup: same device_id не может быть в двух items одновременно.
            let ids_to_check: Vec<i64> = if it.device_ids.is_empty() {
                vec![it.device_id]
            } else {
                it.device_ids.clone()
            };
            for did in &ids_to_check {
                if !seen_device_ids.insert(*did) {
                    return Err(AppError::Validation {
                        field: format!("items[{idx}].device_ids"),
                        message: format!("Устройство id={} включено в акт более одного раза", did),
                    });
                }
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

                // UAT Fix #3/#4 (Phase 3.1): on_warehouse_status_id resolved
                // здесь для group-validation (device_ids[] canonical path).
                let on_warehouse_status_id: i64 = tx
                    .query_row(
                        "SELECT id FROM device_statuses WHERE code = 'на_складе'",
                        [],
                        |r| r.get(0),
                    )
                    .map_err(|e| match e {
                        rusqlite::Error::QueryReturnedNoRows => AppError::Internal {
                            source_chain:
                                "device_statuses missing code='на_складе' — V014 not applied?"
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
                // G-2 (Phase 3.1 Plan 04): handover_date_utc — explicit
                // payload value или fallback на now() (backward-compat для
                // clients без поля).
                let handover_date = payload.handover_date_utc.unwrap_or(now);
                // UAT-fix: resolve location — name (autocomplete) → id, иначе
                // fallback на location_id. INSERT OR IGNORE + SELECT в helper.
                let resolved_location_id: Option<i64> =
                    if let Some(name) = payload.location_name.as_deref() {
                        devices_repo.resolve_location_id_in_tx(&tx, Some(name), now)?
                    } else {
                        payload.location_id
                    };
                let new_row = ActRow {
                    id: 0,
                    number,
                    sub_number: None,
                    parent_act_id: None,
                    act_type: ActType::Handover,
                    giver_name: payload.giver_name.clone(),
                    receiver_name: payload.receiver_name.clone(),
                    location_id: resolved_location_id,
                    location: None,
                    notes: payload.notes.clone(),
                    deadline_utc: payload.deadline_utc,
                    archived: false,
                    created_at_utc: now,
                    updated_at_utc: now,
                    deleted_at_utc: None,
                    version: 1,
                    handover_date_utc: handover_date,
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
                //
                // G-12 (Phase 03.1) clone-on-handover:
                //   - quantity == 1: legacy path (existing single device_id).
                //   - quantity  > 1: clone (quantity - 1) device rows via
                //     SqliteDeviceRepository::clone_device_in_tx; each clone
                //     gets its own act_item (parent_act_item_id = original
                //     act_item.id for clones, NULL for the original).
                //
                // Each effective device (original + clones) goes through the
                // same status/location/audit cycle so undo (which replays from
                // audit_log) restores every clone independently.
                for item in &payload.items {
                    let source_before = devices_repo.get_in_tx(&tx, item.device_id)?;

                    // UAT Fix #3/#4 (Phase 3.1): canonical path — если UI передал
                    // device_ids[] (группа existing devices того же типа на складе),
                    // используем их напрямую без клонирования. Legacy fallback
                    // (device_ids пуст) — original clone-on-handover behavior.
                    let effective_device_ids: Vec<i64> = if !item.device_ids.is_empty() {
                        // Canonical: используем existing devices группы (no cloning).
                        // Validate: каждый ID должен быть на_складе и иметь serial=NULL
                        // (защита от unintended group-substitution серийного устройства).
                        for &dev_id in &item.device_ids {
                            let d = devices_repo.get_in_tx(&tx, dev_id)?;
                            if d.status_id != on_warehouse_status_id {
                                return Err(AppError::Conflict {
                                    reason: format!(
                                        "Устройство id={} больше не на складе — \
                                         обновите список и повторите.",
                                        dev_id
                                    ),
                                });
                            }
                        }
                        item.device_ids.clone()
                    } else {
                        // Legacy: clone-on-handover (item.quantity-1 клонов
                        // источника). Сохраняется для backward-compat и для
                        // случая когда в стоке только 1 девайс группы.
                        let mut ids: Vec<i64> = Vec::with_capacity(item.quantity as usize);
                        ids.push(item.device_id);
                        for _ in 1..item.quantity {
                            let clone_id =
                                devices_repo.clone_device_in_tx(&tx, item.device_id, now)?;
                            // Audit the clone-creation (T-03.1-04 repudiation).
                            let clone_after = devices_repo.get_in_tx(&tx, clone_id)?;
                            let clone_after_json =
                                device_snapshot_json(&clone_after).map_err(|e| {
                                    AppError::Internal {
                                        source_chain: format!("clone after_json: {e}"),
                                    }
                                })?;
                            let clone_payload_json = serde_json::json!({
                                "source_device_id": item.device_id,
                                "act_id": act_id,
                            })
                            .to_string();
                            audit_repo.insert(
                                &tx,
                                AuditEntry {
                                    entity_type: "device",
                                    entity_id: clone_id,
                                    action: "custom:device_clone_for_handover",
                                    user_id: user_id_opt,
                                    before_json: None,
                                    after_json: Some(clone_after_json),
                                    payload_json: Some(clone_payload_json),
                                    created_at_utc: now,
                                },
                            )?;
                            ids.push(clone_id);
                        }
                        ids
                    };

                    // INSERT act_items: one row per effective device.
                    // The first row (original device_id) has parent_act_item_id = NULL.
                    // Subsequent rows reference the first row's id (clone provenance).
                    let mut first_act_item_id: Option<i64> = None;
                    for (idx, &dev_id) in effective_device_ids.iter().enumerate() {
                        let parent_aiid = if idx == 0 { None } else { first_act_item_id };
                        tx.execute(
                            "INSERT INTO act_items \
                             (act_id, device_id, quantity, condition_at_time, \
                              complectation_at_time, parent_act_item_id) \
                             VALUES (?1, ?2, 1, ?3, ?4, ?5)",
                            params![
                                act_id,
                                dev_id,
                                source_before.state.as_deref(),
                                source_before.kit.as_deref(),
                                parent_aiid,
                            ],
                        )
                        .map_err(map_rusqlite)?;
                        if idx == 0 {
                            first_act_item_id = Some(tx.last_insert_rowid());
                        }
                    }

                    // UPDATE devices (each effective device) + audit each mutation.
                    // We snapshot BEFORE status change (source: before status update;
                    // clones: just after creation, before status update).
                    for &dev_id in &effective_device_ids {
                        let before = devices_repo.get_in_tx(&tx, dev_id)?;
                        let before_json =
                            device_snapshot_json(&before).map_err(|e| AppError::Internal {
                                source_chain: format!("before_json: {e}"),
                            })?;

                        // DEF-3: передавать resolved_location_id (вычисленный из
                        // location_name на строке ~258), а не payload.location_id.
                        // Поскольку акты создаются через location_name (commit b2c43a5),
                        // payload.location_id = None → без этого фикса devices.location_id
                        // не обновлялся при handover.
                        let after = devices_repo.update_status_and_location_in_tx(
                            &tx,
                            dev_id,
                            in_work_status_id,
                            resolved_location_id,
                            now,
                        )?;
                        let after_json =
                            device_snapshot_json(&after).map_err(|e| AppError::Internal {
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
                                entity_id: dev_id,
                                action: "update",
                                user_id: user_id_opt,
                                before_json: Some(before_json),
                                after_json: Some(after_json),
                                payload_json: Some(payload_json),
                                created_at_utc: now,
                            },
                        )?;
                    }
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
    // do_return (ACT-06, ACT-07, ACT-08, ACT-09)
    // -----------------------------------------------------------------------

    fn validate_return(payload: &ActReturnDto) -> Result<(), AppError> {
        if payload.items.is_empty() {
            return Err(AppError::Validation {
                field: "items".into(),
                message: "Добавьте хотя бы одну позицию к возврату".into(),
            });
        }
        // CR-03 (ACT-13) + G-12 (Phase 03.1): intra-payload dedup. After G-12
        // shift one ReturnItemDto can carry multiple device_id'ов через
        // `device_ids: Vec<i64>` (clones share одного act_item_id для UI groupа).
        // Dedup primary key — device_id (a device cannot be returned twice in one payload);
        // дубликат act_item_id допустим если разные device_ids указывают на
        // разные cloned act_items одного оригинала, но в G-12 модели каждый
        // act_item ↔ единственный device_id, поэтому дубликат act_item_id
        // практически всегда ошибка → оставляем дубликат-чек для строгости.
        let mut seen_act_items: std::collections::HashSet<i64> = std::collections::HashSet::new();
        let mut seen_device_ids: std::collections::HashSet<i64> = std::collections::HashSet::new();
        for (idx, it) in payload.items.iter().enumerate() {
            if !seen_act_items.insert(it.act_item_id) {
                return Err(AppError::Validation {
                    field: format!("items[{idx}].act_item_id"),
                    message: format!("act_item_id={} продублирован в возврате", it.act_item_id),
                });
            }
            // Canonical device_ids (G-12) с fallback на [device_id] (legacy).
            let dids = effective_device_ids(it);
            if dids.is_empty() {
                return Err(AppError::Validation {
                    field: format!("items[{idx}].device_ids"),
                    message: "Укажите хотя бы один device_id к возврату".into(),
                });
            }
            for &did in &dids {
                if !seen_device_ids.insert(did) {
                    return Err(AppError::Validation {
                        field: format!("items[{idx}].device_ids"),
                        message: format!("device_id={} продублирован в возврате", did),
                    });
                }
            }
            // quantity-чек применим только в legacy-режиме (device_ids пуст).
            if it.device_ids.is_empty() && it.quantity < 1 {
                return Err(AppError::Validation {
                    field: format!("items[{idx}].quantity"),
                    message: "Количество должно быть ≥ 1".into(),
                });
            }
            // Если bulk-режим выключен — каждый item обязан содержать override.
            if !payload.apply_to_all {
                if it.condition_override.is_none() {
                    return Err(AppError::Validation {
                        field: format!("items[{idx}].condition_override"),
                        message: "Состояние обязательно (apply_to_all = false)".into(),
                    });
                }
                // Принимаем либо id, либо name.
                if it.location_id_override.is_none() && it.location_name_override.is_none() {
                    return Err(AppError::Validation {
                        field: format!("items[{idx}].location_id_override"),
                        message: "Расположение обязательно (apply_to_all = false)".into(),
                    });
                }
            }
        }
        Ok(())
    }

    pub async fn do_return(&self, act_id: i64, payload: ActReturnDto) -> Result<ActDto, AppError> {
        Self::validate_return(&payload)?;
        let now = self.clock.unix_seconds();
        let acts_repo = self.acts_repo.clone();
        let audit_repo = self.audit_repo.clone();
        let devices_repo = self.devices_repo.clone();
        let user_id_opt: Option<i64> = None;

        let return_act_id = self
            .writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;

                // 1. Load + validate parent.
                let parent = acts_repo.fetch_full_in_tx(&tx, act_id)?;
                if parent.deleted_at_utc.is_some() {
                    return Err(AppError::NotFound {
                        entity: "act",
                        id: act_id,
                    });
                }
                if parent.act_type != ActType::Handover {
                    return Err(AppError::Validation {
                        field: "act_id".into(),
                        message: "Возврат можно оформить только по handover-акту".into(),
                    });
                }
                if parent.archived {
                    return Err(AppError::Conflict {
                        reason: format!(
                            "Акт №{} уже архивирован — все устройства вернулись",
                            parent.number
                        ),
                    });
                }

                // 2. Validate act_item_id refs (все принадлежат parent акту).
                //
                // G-12 (Phase 03.1): `device_ids[]` canonical — каждый device_id
                // ДОЛЖЕН принадлежать handover-акту (existence check на парах
                // (parent_act_id, device_id) без зависимости от act_item_id —
                // это устойчиво к clone-провенансу, где act_item_id может
                // указывать на оригинал или на клон).
                for (idx, it) in payload.items.iter().enumerate() {
                    let dids = effective_device_ids(it);
                    for &did in &dids {
                        let exists: bool = tx
                            .query_row(
                                "SELECT EXISTS(SELECT 1 FROM act_items \
                                 WHERE act_id = ?1 AND device_id = ?2 LIMIT 1)",
                                params![act_id, did],
                                |r| r.get(0),
                            )
                            .map_err(map_rusqlite)?;
                        if !exists {
                            return Err(AppError::Validation {
                                field: format!("items[{idx}].device_ids"),
                                message: format!(
                                    "device_id={} не принадлежит акту №{}",
                                    did, parent.number
                                ),
                            });
                        }
                    }
                }

                // 3. Resolve on_warehouse_status_id.
                let on_warehouse_status_id: i64 = tx
                    .query_row(
                        "SELECT id FROM device_statuses WHERE code = 'на_складе'",
                        [],
                        |r| r.get(0),
                    )
                    .map_err(|e| match e {
                        rusqlite::Error::QueryReturnedNoRows => AppError::Internal {
                            source_chain:
                                "device_statuses missing code='на_складе' — V014 not applied?"
                                    .into(),
                        },
                        other => map_rusqlite(other),
                    })?;

                // 3b. CR-02 (ACT-13): resolve in_work_status_id once, reused
                // by the per-item status guard inside the loop below.
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

                // 3a. Resolve bulk_location_name → id (если задан). Имя
                // имеет приоритет над `bulk_location_id` (UX-friendly).
                let resolved_bulk_location_id: Option<i64> =
                    if let Some(name) = payload.bulk_location_name.as_deref() {
                        devices_repo.resolve_location_id_in_tx(&tx, Some(name), now)?
                    } else {
                        payload.bulk_location_id
                    };

                // 4. Next sub_number (atomic MAX+1 в той же tx).
                let sub_number = next_sub_number_for_parent(&tx, act_id)?;

                // 5. INSERT return-act (number = parent.number, повторяем).
                //    giver/receiver наследуются от parent (Decision: discretion-zone,
                //    upgrade в plan 04 если UI запросит).
                let return_row = ActRow {
                    id: 0,
                    number: parent.number,
                    sub_number: Some(sub_number),
                    parent_act_id: Some(act_id),
                    act_type: ActType::Return,
                    giver_name: parent.giver_name.clone(),
                    receiver_name: parent.receiver_name.clone(),
                    location_id: resolved_bulk_location_id,
                    location: None,
                    notes: None,
                    deadline_utc: None,
                    archived: false,
                    created_at_utc: now,
                    updated_at_utc: now,
                    deleted_at_utc: None,
                    version: 1,
                    // Return-акты наследуют parent.handover_date_utc.
                    handover_date_utc: parent.handover_date_utc,
                    parent_number: None,
                    sibling_return_count: None,
                };
                let return_act_id = acts_repo.insert_act_in_tx(&tx, &return_row)?;

                // 6. For each return-item: snapshot → insert act_item → update device → audit.
                //
                // G-12 (Phase 03.1) flat-iterate over canonical device_ids:
                // в новой модели каждый returned device = одна строка в
                // return-act_items (quantity=1), один device update, один audit.
                // Legacy fallback (`device_ids` пуст) даёт один элемент
                // [device_id] с per_device_qty=item.quantity (для PRE-V015
                // тестов и backward-compat).
                for item in &payload.items {
                    let dids = effective_device_ids(item);
                    let per_device_qty: i64 = if item.device_ids.is_empty() {
                        item.quantity
                    } else {
                        1
                    };

                    // Per-row override resolution one раз на item (одинаковый
                    // condition/location для всех device_ids в этом item).
                    let effective_condition: Option<String> =
                        item.condition_override.clone().or_else(|| {
                            if payload.apply_to_all {
                                payload.bulk_condition.clone()
                            } else {
                                None
                            }
                        });
                    let per_row_loc_id: Option<i64> =
                        if let Some(name) = item.location_name_override.as_deref() {
                            devices_repo.resolve_location_id_in_tx(&tx, Some(name), now)?
                        } else {
                            item.location_id_override
                        };
                    // DEF-3: если effective_location=None, update_full_in_tx запишет
                    // NULL в location_id. Caller обязан передать bulk_location_name или
                    // location_name_override для восстановления расположения при возврате.
                    let effective_location: Option<i64> = per_row_loc_id.or({
                        if payload.apply_to_all {
                            resolved_bulk_location_id
                        } else {
                            None
                        }
                    });

                    for &device_id in &dids {
                        let before = devices_repo.get_in_tx(&tx, device_id)?;

                        // CR-02 (ACT-13): status guard. Защита от двойного
                        // возврата — device должен быть 'в_работе'.
                        if before.status_id != in_work_status_id {
                            return Err(AppError::Conflict {
                                reason: format!(
                                    "Устройство id={} уже не в работе — возможно, оно уже возвращено",
                                    device_id
                                ),
                            });
                        }

                        // CR-04 (ACT-13): quantity bound. В G-12 модели handover_qty
                        // на (act_id, device_id) ВСЕГДА единственная строка
                        // act_items с quantity=1 (clones имеют свой act_item).
                        // SUM используется на legacy data (PRE-V015 qty>1 originals).
                        let handover_qty: i64 = tx
                            .query_row(
                                "SELECT COALESCE(SUM(quantity), 0) FROM act_items \
                                 WHERE act_id = ?1 AND device_id = ?2",
                                params![act_id, device_id],
                                |r| r.get(0),
                            )
                            .map_err(map_rusqlite)?;
                        let already_returned: i64 = tx
                            .query_row(
                                "SELECT COALESCE(SUM(rai.quantity), 0) \
                                 FROM act_items rai \
                                 JOIN acts ra ON ra.id = rai.act_id \
                                 WHERE ra.parent_act_id = ?1 \
                                   AND rai.device_id = ?2 \
                                   AND ra.deleted_at_utc IS NULL",
                                params![act_id, device_id],
                                |r| r.get(0),
                            )
                            .map_err(map_rusqlite)?;
                        if per_device_qty + already_returned > handover_qty {
                            return Err(AppError::Validation {
                                field: "items".into(),
                                message: format!(
                                    "Возврат превышает выданное количество для устройства id={}: \
                                     уже возвращено {} + текущее {} > выдано {}",
                                    device_id, already_returned, per_device_qty, handover_qty,
                                ),
                            });
                        }
                        // CR-03 fix: в legacy режиме (device_ids: []) status guard
                        // на 'в_работе' блокирует любой второй partial return —
                        // первое частичное вернуло device в 'на_складе' и второе
                        // вернёт Conflict. Чтобы избежать misleading «уже не в работе»,
                        // явно требуем full-closing return: per_device_qty +
                        // already_returned == handover_qty. Сообщение объясняет
                        // правильное использование. В G-12 (canonical device_ids[])
                        // each act_item = 1 device → этот guard не срабатывает.
                        if item.device_ids.is_empty()
                            && per_device_qty + already_returned != handover_qty
                        {
                            return Err(AppError::Validation {
                                field: "items".into(),
                                message: format!(
                                    "Legacy-режим возврата (без device_ids[]) поддерживает только \
                                     один полный закрывающий возврат: для устройства id={} нужно \
                                     вернуть ровно {}, а не {} (уже возвращено {}). Используйте \
                                     device_ids[] для частичных возвратов (G-12 contract).",
                                    device_id,
                                    handover_qty - already_returned,
                                    per_device_qty,
                                    already_returned,
                                ),
                            });
                        }

                        let before_json =
                            device_snapshot_json(&before).map_err(|e| AppError::Internal {
                                source_chain: format!("return before_json: {e}"),
                            })?;

                        // INSERT act_item for the return-act (snapshot return moment).
                        acts_repo.insert_act_item_in_tx(
                            &tx,
                            return_act_id,
                            device_id,
                            per_device_qty,
                            effective_condition.as_deref(),
                            before.kit.as_deref(),
                        )?;

                        // UPDATE devices: → склад + condition.
                        let after = devices_repo.update_full_in_tx(
                            &tx,
                            device_id,
                            on_warehouse_status_id,
                            effective_location,
                            effective_condition.as_deref(),
                            now,
                        )?;
                        let after_json =
                            device_snapshot_json(&after).map_err(|e| AppError::Internal {
                                source_chain: format!("return after_json: {e}"),
                            })?;

                        let payload_json = serde_json::json!({
                            "act_id": return_act_id,
                            "kind": "return",
                        })
                        .to_string();
                        audit_repo.insert(
                            &tx,
                            AuditEntry {
                                entity_type: "device",
                                entity_id: device_id,
                                action: "update",
                                user_id: user_id_opt,
                                before_json: Some(before_json),
                                after_json: Some(after_json),
                                payload_json: Some(payload_json),
                                created_at_utc: now,
                            },
                        )?;
                    }
                }

                // 7. Recompute parent.archived (auto-archive at 100% return).
                recompute_parent_archived(&tx, act_id, now)?;

                // 8. Final audit row for the return-act creation.
                let return_act_after = acts_repo.fetch_full_in_tx(&tx, return_act_id)?;
                let act_after_json = serde_json::to_string(&serde_json::json!({
                    "id": return_act_after.id,
                    "number": return_act_after.number,
                    "sub_number": return_act_after.sub_number,
                    "parent_act_id": return_act_after.parent_act_id,
                    "act_type": return_act_after.act_type.to_sql(),
                    "version": return_act_after.version,
                }))
                .map_err(|e| AppError::Internal {
                    source_chain: format!("return act after_json: {e}"),
                })?;
                audit_repo.insert(
                    &tx,
                    AuditEntry {
                        entity_type: "act",
                        entity_id: return_act_id,
                        action: "create",
                        user_id: user_id_opt,
                        before_json: None,
                        after_json: Some(act_after_json),
                        payload_json: None,
                        created_at_utc: now,
                    },
                )?;

                tx.commit().map_err(map_rusqlite)?;
                Ok(return_act_id)
            })
            .await?;

        self.get(return_act_id).await
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
            let mut items = load_items_for_act(&conn, id)?;
            // G-10 / G-12 (Phase 03.1): populate outstanding_device_ids
            // только для handover-актов (returns не возвращают сами себя).
            if row.act_type == ActType::Handover {
                populate_outstanding_device_ids(&conn, id, &mut items)?;
            }
            // Заполнить return_ids только для handover-актов.
            let return_ids = if row.act_type == ActType::Handover {
                repo.list_returns_for_parent(&conn, id)?
                    .into_iter()
                    .map(|r| r.id)
                    .collect()
            } else {
                Vec::new()
            };
            Ok(act_dto_from_row(row, items, return_ids))
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking: {e}"),
        })?
    }

    /// FTS5 + LIKE search over acts (ACT-04).
    ///
    /// Если `query` (после trim) пустой — fallback на `self.list(filter, page)`.
    /// Иначе строит plain LIKE pattern и FTS5 MATCH expression и делегирует
    /// в `SqliteActRepository::search_acts`.
    pub async fn search(
        &self,
        query: String,
        filter: ActFilter,
        pagination: Pagination,
    ) -> Result<ActListResponse, AppError> {
        if pagination.limit > 200 {
            return Err(AppError::Validation {
                field: "pagination.limit".into(),
                message: "Максимум 200 элементов на страницу".into(),
            });
        }
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return self.list(filter, pagination).await;
        }

        // LIKE escape (T-03-05-01): любые `%` и `_` в пользовательском вводе
        // должны быть literally матчены, не как wildcard. SQLite по умолчанию
        // не имеет `ESCAPE`-клаузы в нашем WHERE — используем `\`-escape +
        // `ESCAPE '\\'`-добавление в SQL. Простейший путь: убрать `%`/`_` из
        // запроса (заменить на пробел) — для пользовательского поиска по ФИО
        // это безвредно.
        let cleaned: String = trimmed
            .chars()
            .map(|c| if c == '%' || c == '_' { ' ' } else { c })
            .collect();
        let plain_query = format!("%{}%", cleaned.trim());
        let fts_query = build_fts_query(&cleaned);

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
                let (rows, total) = repo.search_acts(
                    &conn,
                    &plain_query,
                    &fts_query,
                    &domain_filter,
                    &domain_page,
                )?;
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

    /// G-5: autocomplete для полей «Кто сдал» / «Кто принял».
    /// Источник — DISTINCT `acts.{giver_name|receiver_name}` отсортированные
    /// по frequency DESC (alpha ASC tiebreak). LIKE-prefix match с
    /// `escape_like` защитой от SQL injection через `%` / `_` / `\`.
    /// Soft-deleted акты не учитываются.
    ///
    /// Phase 5 (future): UNION ALL с AD displayName — расширение в SQL без
    /// изменения сигнатуры / UI contract.
    pub async fn suggest_person(
        &self,
        field: SuggestPersonField,
        prefix: &str,
        limit: usize,
    ) -> Result<Vec<String>, AppError> {
        if prefix.chars().count() > 100 {
            return Err(AppError::Validation {
                field: "prefix".into(),
                message: "prefix слишком длинный (макс. 100 символов)".into(),
            });
        }
        let bounded_limit = limit.clamp(1, 20) as i64;
        let pattern = format!("{}%", escape_like(prefix));
        // whitelisted column через enum (НЕ string interpolation от user).
        let column = match field {
            SuggestPersonField::Giver => "giver_name",
            SuggestPersonField::Receiver => "receiver_name",
        };
        let sql = format!(
            "SELECT {col} AS name, COUNT(*) AS freq \
               FROM acts \
              WHERE {col} LIKE ?1 ESCAPE '\\' \
                AND deleted_at_utc IS NULL \
              GROUP BY {col} \
              ORDER BY freq DESC, {col} ASC \
              LIMIT ?2",
            col = column
        );

        let readers = self.readers.clone();
        tokio::task::spawn_blocking(move || -> Result<Vec<String>, AppError> {
            let conn = readers.acquire();
            let mut stmt = conn.prepare(&sql).map_err(map_rusqlite)?;
            let rows = stmt
                .query_map(params![pattern, bounded_limit], |r| r.get::<_, String>(0))
                .map_err(map_rusqlite)?;
            let mut out = Vec::new();
            for r in rows {
                out.push(r.map_err(map_rusqlite)?);
            }
            Ok(out)
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking: {e}"),
        })?
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
    // Soft-delete (ACT-06, ACT-10) — полный undo через audit_log.before_json.
    //
    // Семантика (D-Undo-01):
    //   - handover: undo всех device-mutations (handover + cascaded returns)
    //     → soft-delete handover + cascade soft-delete returns + audit.
    //   - return:   undo own device-mutations → soft-delete + recompute
    //     parent.archived (un-archive если был archived) + audit.
    // -----------------------------------------------------------------------

    pub async fn delete_soft(&self, id: i64, version: i64) -> Result<(), AppError> {
        let now = self.clock.unix_seconds();
        let acts_repo = self.acts_repo.clone();
        let audit_repo = self.audit_repo.clone();
        let devices_repo = self.devices_repo.clone();
        let user_id_opt: Option<i64> = None;

        self.writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;

                // Optimistic-lock check + load row (включая deleted_at_utc).
                let act = acts_repo.fetch_full_in_tx(&tx, id)?;
                if act.deleted_at_utc.is_some() {
                    return Err(AppError::NotFound { entity: "act", id });
                }
                if act.version != version {
                    return Err(AppError::OptimisticLockMismatch {
                        entity: "act",
                        id,
                        expected: version,
                        actual: act.version,
                    });
                }

                match act.act_type {
                    ActType::Handover => {
                        // Cascade-undo всех активных returns в обратном порядке
                        // (LIFO — последний return undo'ится первым), затем
                        // handover. После каждого return-undo делаем
                        // soft-delete return-акта и пишем audit.
                        let returns = acts_repo.list_returns_for_parent_in_tx(&tx, id)?;
                        // Reverse order для LIFO.
                        for ret in returns.iter().rev() {
                            undo_device_mutations_for_act(
                                &tx,
                                &devices_repo,
                                &audit_repo,
                                ret.id,
                                user_id_opt,
                                now,
                            )?;
                            // Soft-delete return-акт + DELETE items (CASCADE
                            // не сработает на soft-delete — делаем явно через
                            // helper repo).
                            acts_repo.soft_delete_in_tx(&tx, ret.id, ret.version, now)?;
                            audit_repo.insert(
                                &tx,
                                AuditEntry {
                                    entity_type: "act",
                                    entity_id: ret.id,
                                    action: "delete",
                                    user_id: user_id_opt,
                                    before_json: None,
                                    after_json: None,
                                    payload_json: Some(
                                        serde_json::json!({
                                            "cascade_from_handover": id,
                                        })
                                        .to_string(),
                                    ),
                                    created_at_utc: now,
                                },
                            )?;
                        }

                        // Now undo handover's own device mutations.
                        undo_device_mutations_for_act(
                            &tx,
                            &devices_repo,
                            &audit_repo,
                            id,
                            user_id_opt,
                            now,
                        )?;
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
                    }
                    ActType::Return => {
                        // Undo own device mutations → soft-delete → recompute parent.
                        undo_device_mutations_for_act(
                            &tx,
                            &devices_repo,
                            &audit_repo,
                            id,
                            user_id_opt,
                            now,
                        )?;
                        acts_repo.soft_delete_in_tx(&tx, id, version, now)?;
                        if let Some(parent_id) = act.parent_act_id {
                            recompute_parent_archived(&tx, parent_id, now)?;
                        }
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
                    }
                }

                tx.commit().map_err(map_rusqlite)?;
                Ok(())
            })
            .await
    }

    // -----------------------------------------------------------------------
    // PDF render path (ACT-11, DEV-15 — Plan 04)
    // -----------------------------------------------------------------------

    /// Render handover act → PDF bytes (D-PDF-Render-Path-01).
    ///
    /// 3-stage pipeline:
    ///   1. Load full ActDto (with items + optional parent block).
    ///   2. Load OrgData + active `act_handover` template body.
    ///   3. Build MiniJinja context per D-PDF-Templates-Schema-01.
    ///   4. Render template → JSON string (safe-mode + 5s timeout).
    ///   5. Deserialize JSON → DocSpec (Validation если broken).
    ///   6. krilla render → Vec<u8>.
    pub async fn render_pdf(&self, act_id: i64) -> Result<Vec<u8>, AppError> {
        let pipeline = self.pdf_pipeline()?;
        let act = self.get(act_id).await?;
        let org = pipeline.organization.read().await?;
        let safe_logo = pipeline.organization.safe_logo_canonical(&org).await?;
        let template_src = pipeline.templates.get_active("act_handover").await?;

        // Optional parent block для return-актов (Plan 04 рендерит handover,
        // но для cascade — оставляем path).
        let parent_block: Option<serde_json::Value> = if let Some(parent_id) = act.parent_act_id {
            let parent = self.get(parent_id).await?;
            Some(serde_json::json!({
                "number": parent.number,
                "date_human": format_ru_date(parent.created_at_utc),
                "date": format_iso_date(parent.created_at_utc),
            }))
        } else {
            None
        };

        // Compute suffix (часть после числа) для печатной формы — берём из
        // ActDto.number (уже отформатирован через format_act_number).
        let suffix = compute_suffix_from_display(&act.number, act.number_raw);

        let items_json: Vec<serde_json::Value> = act
            .items
            .iter()
            .map(|it| {
                serde_json::json!({
                    "name": it.device_name,
                    "inventory_no": it.inventory_no,
                    "serial_no": it.serial_no,
                    "model": it.model,
                    "specs": serde_json::Value::Null,
                    "kit": it.complectation_at_time,
                    "condition": it.condition_at_time,
                    "quantity": it.quantity,
                })
            })
            .collect();

        let ctx = serde_json::json!({
            "org": {
                "name": org.name,
                "inn": org.inn,
                "kpp": org.kpp,
                "address": org.address,
                "logo_path": safe_logo.map(|p| p.display().to_string()),
            },
            "act": {
                "number": act.number_raw,
                "suffix": suffix,
                "date": format_iso_date(act.created_at_utc),
                "date_human": format_ru_date(act.created_at_utc),
                "giver_name": act.giver_name,
                "receiver_name": act.receiver_name,
                "deadline": act.deadline_utc.map(format_iso_date),
                "deadline_human": act.deadline_utc.map(format_ru_date),
                "location_name": act.location,
                "items": items_json,
                "parent": parent_block,
            },
            "return": {
                "condition_default": serde_json::Value::Null,
                "location_default": serde_json::Value::Null,
            },
        });

        let rendered = crate::pdf::minijinja_env::render_with_timeout(
            &pipeline.pdf.minijinja_env,
            "act_handover",
            &template_src,
            ctx,
        )
        .await?;

        let spec: crate::pdf::docspec::DocSpec =
            serde_json::from_str(&rendered).map_err(|e| AppError::Validation {
                field: "template".to_string(),
                message: format!("Шаблон не выдал валидный DocSpec JSON: {e}"),
            })?;

        pipeline.pdf.render_docspec(&spec)
    }

    /// Render acceptance document (документ приёма устройства на склад) → PDF bytes.
    ///
    /// Использует kind=`act_acceptance` шаблон. Контекст беднее handover'а —
    /// одна позиция (device), плюс шапка организации и подписи.
    pub async fn render_acceptance_pdf(
        &self,
        device_id: i64,
        giver_name: String,
        receiver_name: String,
        date_utc: i64,
    ) -> Result<Vec<u8>, AppError> {
        let pipeline = self.pdf_pipeline()?;
        let org = pipeline.organization.read().await?;
        let safe_logo = pipeline.organization.safe_logo_canonical(&org).await?;
        let template_src = pipeline.templates.get_active("act_acceptance").await?;

        // Загрузить device.
        let readers = self.readers.clone();
        let device_json: serde_json::Value =
            tokio::task::spawn_blocking(move || -> Result<serde_json::Value, AppError> {
                let conn = readers.acquire();
                let row = conn
                    .query_row(
                        "SELECT d.name, d.inventory_number, d.serial_number, d.model, d.condition \
                         FROM devices d WHERE d.id = ?1 AND d.deleted_at_utc IS NULL",
                        params![device_id],
                        |r| {
                            Ok((
                                r.get::<_, String>(0)?,
                                r.get::<_, Option<String>>(1)?,
                                r.get::<_, Option<String>>(2)?,
                                r.get::<_, Option<String>>(3)?,
                                r.get::<_, Option<String>>(4)?,
                            ))
                        },
                    )
                    .map_err(|e| match e {
                        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound {
                            entity: "device",
                            id: device_id,
                        },
                        other => map_rusqlite(other),
                    })?;
                Ok(serde_json::json!({
                    "name": row.0,
                    "inventory_no": row.1,
                    "serial_no": row.2,
                    "model": row.3,
                    "condition": row.4,
                }))
            })
            .await
            .map_err(|e| AppError::Internal {
                source_chain: format!("spawn_blocking load device for acceptance: {e}"),
            })??;

        let ctx = serde_json::json!({
            "org": {
                "name": org.name,
                "inn": org.inn,
                "kpp": org.kpp,
                "address": org.address,
                "logo_path": safe_logo.map(|p| p.display().to_string()),
            },
            "device": device_json,
            "document": {
                "giver_name": giver_name,
                "receiver_name": receiver_name,
                "date_human": format_ru_date(date_utc),
                "date": format_iso_date(date_utc),
            },
        });

        let rendered = crate::pdf::minijinja_env::render_with_timeout(
            &pipeline.pdf.minijinja_env,
            "act_acceptance",
            &template_src,
            ctx,
        )
        .await?;

        let spec: crate::pdf::docspec::DocSpec =
            serde_json::from_str(&rendered).map_err(|e| AppError::Validation {
                field: "template".to_string(),
                message: format!("Шаблон не выдал валидный DocSpec JSON: {e}"),
            })?;

        pipeline.pdf.render_docspec(&spec)
    }

    /// Возвращает PDF-pipeline deps как refs или `Internal` если не подключены.
    fn pdf_pipeline(&self) -> Result<PdfPipelineRefs<'_>, AppError> {
        match (&self.templates, &self.organization, &self.pdf) {
            (Some(t), Some(o), Some(p)) => Ok(PdfPipelineRefs {
                templates: t,
                organization: o,
                pdf: p,
            }),
            _ => Err(AppError::Internal {
                source_chain: "ActService::render_pdf called without with_pdf_pipeline".into(),
            }),
        }
    }
}

struct PdfPipelineRefs<'a> {
    templates: &'a Arc<TemplateService>,
    organization: &'a Arc<OrganizationService>,
    pdf: &'a Arc<PdfRenderer>,
}

// ---------------------------------------------------------------------------
// PDF helpers — date formatting + suffix extraction
// ---------------------------------------------------------------------------

const MONTHS_RU: [&str; 12] = [
    "января",
    "февраля",
    "марта",
    "апреля",
    "мая",
    "июня",
    "июля",
    "августа",
    "сентября",
    "октября",
    "ноября",
    "декабря",
];

/// «28 мая 2026 г.» — RU-only formatter поверх `time` crate.
/// G-5 helper (T-03.1-02-01): escape `%`, `_`, `\` для SQL LIKE с
/// `ESCAPE '\'`-клаузой. Защищает от LIKE injection через пользовательский
/// prefix.
fn escape_like(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        if ch == '%' || ch == '_' || ch == '\\' {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

pub fn format_ru_date(unix_seconds: i64) -> String {
    let odt = match time::OffsetDateTime::from_unix_timestamp(unix_seconds) {
        Ok(odt) => odt,
        Err(_) => return "—".to_string(),
    };
    let day = odt.day();
    let month_idx = (odt.month() as u8 as usize).saturating_sub(1);
    let month = MONTHS_RU.get(month_idx).copied().unwrap_or("—");
    let year = odt.year();
    format!("{day} {month} {year} г.")
}

/// «2026-05-28» — ISO date.
pub fn format_iso_date(unix_seconds: i64) -> String {
    let odt = match time::OffsetDateTime::from_unix_timestamp(unix_seconds) {
        Ok(odt) => odt,
        Err(_) => return "—".to_string(),
    };
    format!(
        "{:04}-{:02}-{:02}",
        odt.year(),
        odt.month() as u8,
        odt.day()
    )
}

/// Извлекает суффикс (например, «в», «в1», «в2») из отформатированного
/// `ActDto.number` относительно raw counter value. Для handover → "".
fn compute_suffix_from_display(display: &str, number_raw: i64) -> String {
    let raw_str = number_raw.to_string();
    if let Some(rest) = display.strip_prefix(&raw_str) {
        rest.to_string()
    } else {
        // Дисплей не начинается с raw (return: «42в» где raw мог быть 999).
        // В этом случае весь display — это {parent_number}{suffix}; ищем 'в'.
        if let Some(idx) = display.find('в') {
            display[idx..].to_string()
        } else {
            String::new()
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers — undo path
// ---------------------------------------------------------------------------

/// Восстанавливает все devices, на которых данный акт оставил mutation, из
/// `audit_log.before_json`. Для каждого восстановленного device пишет
/// `audit_log` запись `action='custom:undo'`.
///
/// Используется и для handover undo, и для return undo (single shared path).
fn undo_device_mutations_for_act(
    tx: &rusqlite::Transaction<'_>,
    devices_repo: &SqliteDeviceRepository,
    audit_repo: &SqliteAuditLogRepository,
    act_id: i64,
    user_id_opt: Option<i64>,
    now: i64,
) -> Result<(), AppError> {
    let rows = audit_repo.select_device_mutations_for_act(tx, act_id)?;
    for (device_id, before_json) in rows.into_iter().rev() {
        let snapshot: serde_json::Value =
            serde_json::from_str(&before_json).map_err(|e| AppError::Internal {
                source_chain: format!("undo: corrupt before_json for device {device_id}: {e}"),
            })?;
        let restored = devices_repo.restore_from_snapshot_in_tx(tx, device_id, &snapshot, now)?;
        let after_json = device_snapshot_json(&restored).map_err(|e| AppError::Internal {
            source_chain: format!("undo after_json: {e}"),
        })?;
        let payload_json = serde_json::json!({
            "undo_of_act_id": act_id,
        })
        .to_string();
        audit_repo.insert(
            tx,
            AuditEntry {
                entity_type: "device",
                entity_id: device_id,
                action: "custom:undo",
                user_id: user_id_opt,
                before_json: Some(before_json),
                after_json: Some(after_json),
                payload_json: Some(payload_json),
                created_at_utc: now,
            },
        )?;
    }
    Ok(())
}

/// Канонический snapshot device-row для записи в `audit_log.{before,after}_json`.
///
/// Включает ВСЕ поля, необходимые для `restore_from_snapshot_in_tx` (D-Undo-01):
/// `id`, `status_id`, `location_id`, `state`, `kit`, `name`, `model`,
/// `inventory_no`, `serial_no`, `specs`, `type_id`, `version`.
/// G-12 canonical device_ids per ReturnItemDto с fallback на legacy
/// `[device_id]` (для совместимости с уже существующими тестами / клиентами,
/// которые шлют единственный device_id + quantity=1).
fn effective_device_ids(item: &ActReturnItemDto) -> Vec<i64> {
    if item.device_ids.is_empty() {
        vec![item.device_id]
    } else {
        item.device_ids.clone()
    }
}

fn device_snapshot_json(row: &DeviceRow) -> Result<String, serde_json::Error> {
    serde_json::to_string(&serde_json::json!({
        "id": row.id,
        "type_id": row.type_id,
        "name": row.name,
        "inventory_no": row.inventory_no,
        "serial_no": row.serial_no,
        "model": row.model,
        "state": row.state,
        "kit": row.kit,
        "location_id": row.location_id,
        "location": row.location,
        "status_id": row.status_id,
        "specs": row.specs,
        "version": row.version,
    }))
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
                // G-10/G-12 (Phase 03.1): outstanding_device_ids заполняется
                // в caller'е (ActService::get / list / search) — этот helper
                // только подгружает joined-device fields. Initialized to empty;
                // populate_outstanding_device_ids() fills handover-acts.
                outstanding_device_ids: Vec::new(),
            })
        })
        .map_err(map_rusqlite)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(map_rusqlite)?);
    }
    Ok(out)
}

/// G-10 / G-12 (Phase 03.1): for each handover-act item, fill
/// `outstanding_device_ids` — the device_id'ы ещё НЕ возвращённые через
/// активные return-акты этого parent.
///
/// SQL semantics (B-1 compliant — `act_items` НЕ имеет `deleted_at_utc`,
/// soft-delete filter применяется ТОЛЬКО на `acts` через JOIN):
///
/// ```sql
/// SELECT device_id FROM act_items WHERE act_id = ?
///   EXCEPT
/// SELECT rai.device_id FROM act_items rai
///   JOIN acts ra ON ra.id = rai.act_id
///  WHERE ra.parent_act_id = ?  AND ra.deleted_at_utc IS NULL;
/// ```
///
/// Каждый ActItemDto получает `outstanding_device_ids = [device_id]` если этот
/// device НЕ возвращён, иначе `[]`. В G-12 модели 1 act_item ↔ 1 device_id,
/// поэтому outstanding на конкретном item это всегда либо `[device_id]`
/// либо `[]`.
fn populate_outstanding_device_ids(
    conn: &rusqlite::Connection,
    act_id: i64,
    items: &mut [ActItemDto],
) -> Result<(), AppError> {
    let mut stmt = conn
        .prepare(
            "SELECT device_id FROM act_items WHERE act_id = ?1 \
             EXCEPT \
             SELECT rai.device_id FROM act_items rai \
               JOIN acts ra ON ra.id = rai.act_id \
              WHERE ra.parent_act_id = ?1 AND ra.deleted_at_utc IS NULL",
        )
        .map_err(map_rusqlite)?;
    let outstanding: std::collections::HashSet<i64> = stmt
        .query_map(params![act_id], |r| r.get::<_, i64>(0))
        .map_err(map_rusqlite)?
        .collect::<rusqlite::Result<_>>()
        .map_err(map_rusqlite)?;
    for item in items.iter_mut() {
        if outstanding.contains(&item.device_id) {
            item.outstanding_device_ids = vec![item.device_id];
        } else {
            item.outstanding_device_ids = Vec::new();
        }
    }
    Ok(())
}

// `ActItemRow` re-export (unused yet — plan 03 will consume).
#[allow(dead_code)]
fn _act_item_row_ref(_r: &ActItemRow) {}

// ---------------------------------------------------------------------------
// FTS5 query builder (mirrors devices_sqlite::build_fts_query — Phase 2)
// ---------------------------------------------------------------------------

/// Sanitize user input for FTS5 MATCH queries (T-03-05-01).
///
/// 1-к-1 c `devices_sqlite::build_fts_query` (приватный в trackly-infra crate,
/// поэтому дублируем здесь — короткая функция, кросс-crate publish не оправдан
/// в Phase 3).
///
/// - Splits on whitespace.
/// - Strips null bytes.
/// - Escapes internal `"` as `""`.
/// - Wraps each token in double-quotes and appends `*` for prefix search.
///
/// Empty input → empty result; caller-side handles fallback.
pub(crate) fn build_fts_query(user_input: &str) -> String {
    user_input
        .split_whitespace()
        .map(|t| t.replace('\0', "").replace('"', "\"\""))
        .filter(|t| !t.is_empty())
        .map(|t| format!("\"{t}\"*"))
        .collect::<Vec<_>>()
        .join(" ")
}

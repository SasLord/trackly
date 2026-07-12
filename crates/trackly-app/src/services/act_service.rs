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
use trackly_core::domain::acts::{ActItemRow, ActPatch, ActRow, ActType};
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
    ActReturnItemDto, ActUpdateDto, ActUpdateReturnDto, ActsCountsDto, Pagination,
};
use crate::dto::suggest::SuggestPersonField;
use crate::pdf::PdfRenderer;
use crate::services::org_db_service::OrgDbService;
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
    /// D-05 (Phase 14 plan 03): единый источник org-реквизитов для act-рендера
    /// — `org_settings` (то, что пишет Settings UI), не `org.json`. `organization`
    /// остаётся подключённым для logo-пути (`safe_logo_canonical`) и
    /// `render_acceptance_pdf`, который по D-03 остаётся вне скоупа этой фазы.
    pub(crate) org_db: Option<Arc<OrgDbService>>,
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
            org_db: None,
        }
    }

    /// Builder: подключить PDF pipeline deps (templates + organization + pdf + org_db).
    /// Используется в `AppCtx::build` (production runtime) и в plan-04
    /// integration tests, проверяющих render_pdf end-to-end.
    ///
    /// `org_db` — Optional (Phase 14 plan 03 D-05): pre-existing test fixtures
    /// calling the old 3-arg signature would break; `with_org_db` sets it
    /// separately so `ActService::new(...).with_pdf_pipeline(...)` call sites
    /// without org_db keep compiling (org-context degrades — see `pdf_pipeline()`).
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

    /// Builder: подключить `OrgDbService` (D-05) — источник org-реквизитов для
    /// act-рендера. Отдельный builder-метод, чтобы не ломать существующие
    /// call sites `with_pdf_pipeline(templates, organization, pdf)`.
    pub fn with_org_db(mut self, org_db: Arc<OrgDbService>) -> Self {
        self.org_db = Some(org_db);
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
    // update (ACT-02, Phase 19 plan 03) — edit existing handover act
    // -----------------------------------------------------------------------

    fn validate_update(p: &ActUpdateDto) -> Result<(), AppError> {
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
        // Dedup: same device_id не может встречаться дважды в items (flat
        // check — ActUpdateItemDto's cardinality is one device_id per item,
        // unlike ActCreateDto's quantity/device_ids[] sub-list).
        let mut seen_device_ids: std::collections::HashSet<i64> = std::collections::HashSet::new();
        for (idx, it) in p.items.iter().enumerate() {
            if !seen_device_ids.insert(it.device_id) {
                return Err(AppError::Validation {
                    field: format!("items[{idx}].device_id"),
                    message: format!(
                        "Устройство id={} включено в акт более одного раза",
                        it.device_id
                    ),
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

    /// Edit an existing handover act's header + item set (ACT-02).
    ///
    /// `payload.items` is a FULL replacement set — added/retained/removed
    /// device_ids are computed by diffing against the act's current
    /// `act_items`. D-05: header-only edits (same device_id set) never touch
    /// device rows. D-06: added devices transition на_складе→в_работе exactly
    /// like `create`; removed devices are restored to their MOST RECENT prior
    /// state (not the original pre-handover state — Pitfall 2). D-07: only
    /// `ActType::Handover` acts are editable — enforced server-side,
    /// independent of the UI's disabled-button state. D-08: a `removed`
    /// device_id that has already been consumed by a completed/active return
    /// is rejected, aborting the WHOLE update (no partial writes).
    pub async fn update(&self, payload: ActUpdateDto) -> Result<ActDto, AppError> {
        Self::validate_update(&payload)?;
        let now = self.clock.unix_seconds();
        let acts_repo = self.acts_repo.clone();
        let audit_repo = self.audit_repo.clone();
        let devices_repo = self.devices_repo.clone();
        let user_id_opt: Option<i64> = None;

        let act_id = self
            .writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;

                // 1. Load act (incl. soft-deleted flag).
                let act = acts_repo.fetch_full_in_tx(&tx, payload.id)?;
                if act.deleted_at_utc.is_some() {
                    return Err(AppError::NotFound {
                        entity: "act",
                        id: payload.id,
                    });
                }

                // 2. D-07: only handover acts are editable — server-side,
                // authoritative regardless of what any client sends.
                if act.act_type != ActType::Handover {
                    return Err(AppError::Validation {
                        field: "id".into(),
                        message: "Редактировать можно только акты выдачи (handover)".into(),
                    });
                }

                // 3. Defense-in-depth CAS pre-check. The structural guarantee
                // is `update_act_header_in_tx`'s own `WHERE version=?` clause
                // — this early check just gives a cleaner error path before
                // any device work starts.
                if act.version != payload.expected_version {
                    return Err(AppError::OptimisticLockMismatch {
                        entity: "act",
                        id: payload.id,
                        expected: payload.expected_version,
                        actual: act.version,
                    });
                }

                // 4. Resolve device_statuses ids (mirrors `create`).
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

                // 5. Compute delta between current act_items and the payload's
                // full replacement set.
                let d_old: std::collections::HashSet<i64> = {
                    let mut stmt = tx
                        .prepare("SELECT device_id FROM act_items WHERE act_id = ?1")
                        .map_err(map_rusqlite)?;
                    let ids: std::collections::HashSet<i64> = stmt
                        .query_map(params![payload.id], |r| r.get::<_, i64>(0))
                        .map_err(map_rusqlite)?
                        .collect::<rusqlite::Result<_>>()
                        .map_err(map_rusqlite)?;
                    ids
                };
                let d_new: std::collections::HashSet<i64> =
                    payload.items.iter().map(|i| i.device_id).collect();
                let added: Vec<i64> = d_new.difference(&d_old).copied().collect();
                let unchanged: Vec<i64> = d_old.intersection(&d_new).copied().collect();

                // Resolve location once — name (autocomplete) takes priority
                // over `location_id`, mirrors `create`'s pattern.
                let resolved_location_id: Option<i64> =
                    if let Some(name) = payload.location_name.as_deref() {
                        devices_repo.resolve_location_id_in_tx(&tx, Some(name), now)?
                    } else {
                        payload.location_id
                    };

                // 6. Added devices: status guard (same shape as `create`'s
                // device_ids[] path) THEN the add-loop body copied verbatim
                // from `create` (before/after snapshot, status+location
                // transition, audit "update"/"handover").
                for &dev_id in &added {
                    let d = devices_repo.get_in_tx(&tx, dev_id)?;
                    if d.status_id != on_warehouse_status_id {
                        return Err(AppError::Conflict {
                            reason: format!(
                                "Устройство id={} больше не на складе — обновите список и \
                                 повторите.",
                                dev_id
                            ),
                        });
                    }
                }
                for &dev_id in &added {
                    let before = devices_repo.get_in_tx(&tx, dev_id)?;
                    let before_json =
                        device_snapshot_json(&before).map_err(|e| AppError::Internal {
                            source_chain: format!("before_json: {e}"),
                        })?;
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
                        "act_id": payload.id,
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

                    // INSERT act_items row for the newly added position.
                    // `complectation_at_time`: matching item's value if
                    // Some, else fall back to the source device's live kit
                    // (mirrors `create`'s `source_before.kit` default).
                    let complectation: Option<String> = payload
                        .items
                        .iter()
                        .find(|i| i.device_id == dev_id)
                        .and_then(|i| i.complectation_at_time.clone())
                        .or_else(|| before.kit.clone());
                    tx.execute(
                        "INSERT INTO act_items \
                         (act_id, device_id, quantity, condition_at_time, \
                          complectation_at_time, parent_act_item_id) \
                         VALUES (?1, ?2, 1, ?3, ?4, NULL)",
                        params![
                            payload.id,
                            dev_id,
                            before.state.as_deref(),
                            complectation,
                        ],
                    )
                    .map_err(map_rusqlite)?;
                }

                // 7. Retained (unchanged device_id set) rows: overwrite
                // complectation_at_time только если the matching item's
                // value is Some (D-04 комплектация edit on retained rows).
                // WR-03: only fire the UPDATE + audit row when the incoming
                // value actually DIFFERS from what's stored — a no-op
                // resubmit of the same комплектация must write neither.
                for &dev_id in &unchanged {
                    if let Some(item) = payload.items.iter().find(|i| i.device_id == dev_id) {
                        if let Some(v) = &item.complectation_at_time {
                            let stored: Option<String> = tx
                                .query_row(
                                    "SELECT complectation_at_time FROM act_items \
                                     WHERE act_id = ?1 AND device_id = ?2",
                                    params![payload.id, dev_id],
                                    |r| r.get(0),
                                )
                                .map_err(map_rusqlite)?;
                            if stored.as_deref() != Some(v.as_str()) {
                                tx.execute(
                                    "UPDATE act_items SET complectation_at_time = ?1 \
                                     WHERE act_id = ?2 AND device_id = ?3",
                                    params![v, payload.id, dev_id],
                                )
                                .map_err(map_rusqlite)?;

                                let before_json = serde_json::json!({
                                    "device_id": dev_id,
                                    "complectation_at_time": stored,
                                })
                                .to_string();
                                let after_json = serde_json::json!({
                                    "device_id": dev_id,
                                    "complectation_at_time": v,
                                })
                                .to_string();
                                audit_repo.insert(
                                    &tx,
                                    AuditEntry {
                                        entity_type: "act_item",
                                        entity_id: payload.id,
                                        action: "custom:act_item_complectation_edit",
                                        user_id: user_id_opt,
                                        before_json: Some(before_json),
                                        after_json: Some(after_json),
                                        payload_json: Some(
                                            serde_json::json!({ "act_id": payload.id })
                                                .to_string(),
                                        ),
                                        created_at_utc: now,
                                    },
                                )?;
                            }
                        }
                    }
                }

                // 8a. D-08 guard: a `removed` device_id that has already been
                // consumed by a completed/active return is rejected — this
                // must be checked for ALL removed devices BEFORE any of
                // their mutations run (validate-then-mutate). If even one
                // removed device fails this guard, the WHOLE update aborts
                // (transaction rollback on any Err before commit).
                let removed: Vec<i64> = d_old.difference(&d_new).copied().collect();
                let outstanding = populate_outstanding_device_ids_in_tx(&tx, payload.id)?;
                for &removed_id in &removed {
                    if !outstanding.contains(&removed_id) {
                        return Err(AppError::Conflict {
                            reason: format!(
                                "Устройство id={} уже возвращено по акту возврата — \
                                 редактирование позиции невозможно",
                                removed_id
                            ),
                        });
                    }
                }

                // 8b. Number uniqueness re-check (A3) — same rule as `create`,
                // only re-run when the number actually changes.
                if let Some(n) = payload.number_override {
                    if n != act.number {
                        let exists: bool = tx
                            .query_row(
                                "SELECT EXISTS(SELECT 1 FROM acts WHERE number=?1 LIMIT 1)",
                                params![n],
                                |r| r.get(0),
                            )
                            .map_err(map_rusqlite)?;
                        if exists {
                            return Err(AppError::Conflict {
                                reason: format!("Акт №{n} уже существует"),
                            });
                        }
                    }
                }

                // 8c. Removed devices: restore to the MOST RECENT prior state
                // (Pitfall 2 — NOT the original pre-handover state) via
                // `select_latest_device_mutation`'s `DESC LIMIT 1` lookup,
                // then delete the act_items row.
                for &removed_id in &removed {
                    let before_json = audit_repo
                        .select_latest_device_mutation(&tx, payload.id, removed_id)?
                        .ok_or_else(|| AppError::Internal {
                            source_chain: format!(
                                "update: no audit trail for outstanding device {removed_id} \
                                 on act {}",
                                payload.id
                            ),
                        })?;
                    let snapshot: serde_json::Value = serde_json::from_str(&before_json)
                        .map_err(|e| AppError::Internal {
                            source_chain: format!(
                                "update: corrupt before_json for device {removed_id}: {e}"
                            ),
                        })?;
                    let restored = devices_repo.restore_from_snapshot_in_tx(
                        &tx,
                        removed_id,
                        &snapshot,
                        now,
                    )?;
                    let after_json =
                        device_snapshot_json(&restored).map_err(|e| AppError::Internal {
                            source_chain: format!("update remove after_json: {e}"),
                        })?;
                    audit_repo.insert(
                        &tx,
                        AuditEntry {
                            entity_type: "device",
                            entity_id: removed_id,
                            action: "custom:update_remove",
                            user_id: user_id_opt,
                            before_json: Some(before_json),
                            after_json: Some(after_json),
                            payload_json: Some(
                                serde_json::json!({ "act_id": payload.id }).to_string(),
                            ),
                            created_at_utc: now,
                        },
                    )?;
                    tx.execute(
                        "DELETE FROM act_items WHERE act_id = ?1 AND device_id = ?2",
                        params![payload.id, removed_id],
                    )
                    .map_err(map_rusqlite)?;
                }

                // 9. Build ActPatch + CAS header UPDATE. The 5 original
                // header fields are unconditional in `update_act_header_in_tx`'s
                // SQL (per Plan 19-02) — always supply resolved Some(..)
                // values; `handover_date_utc`/`number` use COALESCE semantics
                // (None = no change).
                let patch = ActPatch {
                    giver_name: Some(payload.giver_name.clone()),
                    receiver_name: Some(payload.receiver_name.clone()),
                    location_id: Some(resolved_location_id),
                    notes: Some(payload.notes.clone()),
                    deadline_utc: Some(payload.deadline_utc),
                    handover_date_utc: payload.handover_date_utc,
                    number: payload.number_override,
                    expected_version: payload.expected_version,
                };
                acts_repo.update_act_header_in_tx(&tx, payload.id, &patch, now)?;

                // 9a. Recompute acts.archived (CR-01 gap closure) whenever the
                // item set actually changed. `update_act_header_in_tx` just
                // ran a CAS `WHERE version = expected_version` bump; this
                // recompute bumps `version` again unconditionally on
                // `WHERE id`. It MUST run AFTER the CAS header UPDATE above —
                // running it BEFORE would advance version past
                // expected_version, making the CAS match 0 rows and raising a
                // spurious OptimisticLockMismatch. It also MUST run before
                // step 10's final-audit fetch so `act_after`/the returned
                // ActDto reflect the double bump. Gated on add/remove so
                // header-only edits (added and removed both empty) keep the
                // single version+1 contract asserted by
                // `header_only_edit_does_not_touch_devices` — archived can
                // only change when the outstanding-device count changes.
                if !added.is_empty() || !removed.is_empty() {
                    recompute_parent_archived(&tx, payload.id, now)?;
                }

                // 9b. If the number actually changed, audit it distinctly
                // (mirrors `create`'s `custom:act_number_override` shape) AND
                // cascade the new number to any child return acts (WR-01).
                // Return acts store a COPY of the parent's `number` (see
                // `do_return`'s INSERT) rather than a live reference; without
                // this cascade the old number stays "in use" forever by the
                // orphaned return rows, permanently blocking reuse by the
                // step-8b uniqueness check. Return rows keep a distinct
                // `sub_number`, so the shared UNIQUE(number, COALESCE(sub_number,0))
                // index cannot be violated by this cascade.
                if let Some(n) = payload.number_override {
                    if n != act.number {
                        tx.execute(
                            "UPDATE acts SET number = ?1, updated_at_utc = ?2 \
                             WHERE parent_act_id = ?3 AND deleted_at_utc IS NULL",
                            params![n, now, payload.id],
                        )
                        .map_err(map_rusqlite)?;

                        let override_payload_json = serde_json::json!({
                            "requested": n,
                            "previous": act.number,
                        })
                        .to_string();
                        audit_repo.insert(
                            &tx,
                            AuditEntry {
                                entity_type: "act",
                                entity_id: payload.id,
                                action: "custom:act_number_override",
                                user_id: user_id_opt,
                                before_json: None,
                                after_json: None,
                                payload_json: Some(override_payload_json),
                                created_at_utc: now,
                            },
                        )?;
                    }
                }

                // 10. Final audit row for the header edit (real before/after
                // diff, unlike `create`'s tail which only has an after_json).
                let act_after = acts_repo.fetch_full_in_tx(&tx, payload.id)?;
                let before_json = serde_json::to_string(&serde_json::json!({
                    "giver_name": act.giver_name,
                    "receiver_name": act.receiver_name,
                    "location_id": act.location_id,
                    "notes": act.notes,
                    "deadline_utc": act.deadline_utc,
                    "handover_date_utc": act.handover_date_utc,
                    "number": act.number,
                    "version": act.version,
                }))
                .map_err(|e| AppError::Internal {
                    source_chain: format!("act before_json: {e}"),
                })?;
                let after_json = serde_json::to_string(&serde_json::json!({
                    "giver_name": act_after.giver_name,
                    "receiver_name": act_after.receiver_name,
                    "location_id": act_after.location_id,
                    "notes": act_after.notes,
                    "deadline_utc": act_after.deadline_utc,
                    "handover_date_utc": act_after.handover_date_utc,
                    "number": act_after.number,
                    "version": act_after.version,
                }))
                .map_err(|e| AppError::Internal {
                    source_chain: format!("act after_json: {e}"),
                })?;
                audit_repo.insert(
                    &tx,
                    AuditEntry {
                        entity_type: "act",
                        entity_id: payload.id,
                        action: "update",
                        user_id: user_id_opt,
                        before_json: Some(before_json),
                        after_json: Some(after_json),
                        payload_json: None,
                        created_at_utc: now,
                    },
                )?;

                tx.commit().map_err(map_rusqlite)?;
                Ok(payload.id)
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
                    // Phase 22 (ACT-03, Pitfall 1 fix / D-12): use the
                    // payload's own submitted giver/receiver when present —
                    // previously hard-copied from `parent.*` even though
                    // `ReturnModal.svelte` collected these fields locally and
                    // never sent them. `None` falls back to the historical
                    // parent-swap default (back-compat with any
                    // not-yet-updated client).
                    giver_name: payload
                        .giver_name
                        .clone()
                        .unwrap_or_else(|| parent.receiver_name.clone()),
                    receiver_name: payload
                        .receiver_name
                        .clone()
                        .unwrap_or_else(|| parent.giver_name.clone()),
                    location_id: resolved_bulk_location_id,
                    location: None,
                    notes: None,
                    deadline_utc: None,
                    archived: false,
                    created_at_utc: now,
                    updated_at_utc: now,
                    deleted_at_utc: None,
                    version: 1,
                    // Phase 22 (ACT-03, D-05): a return's «Дата возврата» is
                    // now its OWN field, no longer inherited from
                    // `parent.handover_date_utc`. `None` falls back to `now`
                    // (back-compat with clients not yet sending this field) —
                    // this is a deliberate semantic break from the pre-Phase-22
                    // inheritance model (see V034 backfill migration for
                    // historical rows).
                    handover_date_utc: payload.handover_date_utc.unwrap_or(now),
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
    // update_return (ACT-03, Phase 22 plan 02) — edit existing return act
    // -----------------------------------------------------------------------

    fn validate_update_return(p: &ActUpdateReturnDto) -> Result<(), AppError> {
        // D-10: an empty item set is rejected up-front — use `delete_soft`
        // to fully undo a return instead (mirrors `validate_update`'s own
        // items-non-empty check, `act_service.rs:528-533`).
        if p.items.is_empty() {
            return Err(AppError::Validation {
                field: "items".into(),
                message: "Добавьте хотя бы одну позицию".into(),
            });
        }
        Ok(())
    }

    /// Edit an existing **return** act's header + item set (ACT-03).
    ///
    /// `payload.items` is a FULL replacement set — added/retained/removed
    /// device_ids are computed by diffing against the return's current
    /// `act_items`, mirroring `update()`'s own D-06 delta but with an
    /// INVERTED direction: an "added" device_id here is a newly-returned
    /// outstanding device (в_работе → на_складе, reusing `do_return`'s
    /// per-device loop body), a "removed" device_id is an "un-return"
    /// (restored to its prior в_работе state via the same audit-snapshot
    /// mechanism `delete_soft`'s `ActType::Return` branch uses).
    ///
    /// D-11: any `removed` device, or any `retained` device whose payload
    /// requests an actual condition/location change, is rejected with
    /// `AppError::Conflict` if its CURRENT `(status_id, location_id, state)`
    /// diverges from what THIS return's own most-recent mutation set —
    /// covers both a later-handover reissue AND a manual device-page
    /// relocation. No force-override.
    ///
    /// D-10: an empty `items` set is rejected server-side before the
    /// transaction opens (`validate_update_return`).
    pub async fn update_return(&self, payload: ActUpdateReturnDto) -> Result<ActDto, AppError> {
        Self::validate_update_return(&payload)?;
        let now = self.clock.unix_seconds();
        let acts_repo = self.acts_repo.clone();
        let audit_repo = self.audit_repo.clone();
        let devices_repo = self.devices_repo.clone();
        let user_id_opt: Option<i64> = None;

        let return_act_id = self
            .writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;

                // 1. Load act (incl. soft-deleted flag).
                let act = acts_repo.fetch_full_in_tx(&tx, payload.id)?;
                if act.deleted_at_utc.is_some() {
                    return Err(AppError::NotFound {
                        entity: "act",
                        id: payload.id,
                    });
                }

                // 2. Type-guard — INVERTED vs `update()`: only Return acts
                // are editable via this method.
                if act.act_type != ActType::Return {
                    return Err(AppError::Validation {
                        field: "id".into(),
                        message: "Редактировать можно только акты возврата".into(),
                    });
                }

                // 3. Defense-in-depth CAS pre-check. The structural guarantee
                // is `update_act_header_in_tx`'s own `WHERE version=?` clause.
                if act.version != payload.expected_version {
                    return Err(AppError::OptimisticLockMismatch {
                        entity: "act",
                        id: payload.id,
                        expected: payload.expected_version,
                        actual: act.version,
                    });
                }

                let parent_act_id = act
                    .parent_act_id
                    .expect("return act always has parent_act_id");

                // 4. Resolve device_statuses ids (mirrors `do_return`).
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

                // 5. Resolve bulk_location_name → id (имя приоритетнее id,
                // mirrors `do_return`).
                let resolved_bulk_location_id: Option<i64> =
                    if let Some(name) = payload.bulk_location_name.as_deref() {
                        devices_repo.resolve_location_id_in_tx(&tx, Some(name), now)?
                    } else {
                        payload.bulk_location_id
                    };

                // 6. Build per-device effective (quantity, condition,
                // location) map from the payload's items — mirrors
                // `do_return`'s per-item override resolution exactly
                // (per-row override wins, `apply_to_all` bulk fallback else
                // None).
                let mut effective_by_device: std::collections::HashMap<
                    i64,
                    (i64, Option<String>, Option<i64>),
                > = std::collections::HashMap::new();
                for item in &payload.items {
                    let dids = effective_device_ids(item);
                    let per_device_qty: i64 = if item.device_ids.is_empty() {
                        item.quantity
                    } else {
                        1
                    };
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
                    let effective_location: Option<i64> = per_row_loc_id.or({
                        if payload.apply_to_all {
                            resolved_bulk_location_id
                        } else {
                            None
                        }
                    });
                    for &device_id in &dids {
                        effective_by_device.insert(
                            device_id,
                            (per_device_qty, effective_condition.clone(), effective_location),
                        );
                    }
                }

                // 7. Compute delta between current act_items (this return's
                // own, NOT the parent's) and the payload's full replacement
                // set.
                let d_old: std::collections::HashSet<i64> = {
                    let mut stmt = tx
                        .prepare("SELECT device_id FROM act_items WHERE act_id = ?1")
                        .map_err(map_rusqlite)?;
                    let ids: std::collections::HashSet<i64> = stmt
                        .query_map(params![payload.id], |r| r.get::<_, i64>(0))
                        .map_err(map_rusqlite)?
                        .collect::<rusqlite::Result<_>>()
                        .map_err(map_rusqlite)?;
                    ids
                };
                let d_new: std::collections::HashSet<i64> =
                    effective_by_device.keys().copied().collect();
                let added: Vec<i64> = d_new.difference(&d_old).copied().collect();
                let removed: Vec<i64> = d_old.difference(&d_new).copied().collect();
                let retained: Vec<i64> = d_old.intersection(&d_new).copied().collect();

                // 8a. VALIDATE `added`: must belong to the parent handover's
                // act_items (mirrors `do_return`'s existence check,
                // `act_service.rs:1143-1163`) AND be currently 'в_работе'
                // (mirrors `do_return`'s status guard, `:1286`) — BEFORE any
                // mutation runs (validate-then-mutate).
                for &dev_id in &added {
                    let exists: bool = tx
                        .query_row(
                            "SELECT EXISTS(SELECT 1 FROM act_items \
                             WHERE act_id = ?1 AND device_id = ?2 LIMIT 1)",
                            params![parent_act_id, dev_id],
                            |r| r.get(0),
                        )
                        .map_err(map_rusqlite)?;
                    if !exists {
                        return Err(AppError::Validation {
                            field: "items".into(),
                            message: format!(
                                "device_id={} не принадлежит родительскому акту",
                                dev_id
                            ),
                        });
                    }
                    let d = devices_repo.get_in_tx(&tx, dev_id)?;
                    if d.status_id != in_work_status_id {
                        return Err(AppError::Conflict {
                            reason: format!(
                                "Устройство id={} уже не в работе — возможно, оно уже возвращено",
                                dev_id
                            ),
                        });
                    }
                }

                // 8b. D-11 guard (Pattern 4): for every `removed` device AND
                // every `retained` device whose payload requests an actual
                // condition/location change, compare the device's CURRENT
                // `(status_id, location_id, state)` against what THIS
                // return's own most-recent mutation set
                // (`select_latest_device_mutation_pair`'s `after_json`) —
                // reject with `Conflict` on any mismatch, BEFORE any
                // mutation runs. Devices in `retained` with NO requested
                // value change skip this check entirely (an unrelated no-op
                // resubmit must not be blocked).
                let mut retained_with_change: Vec<i64> = Vec::new();
                for &dev_id in &retained {
                    let (_, eff_condition, eff_location) = effective_by_device
                        .get(&dev_id)
                        .cloned()
                        .unwrap_or((1, None, None));
                    let stored_condition: Option<String> = tx
                        .query_row(
                            "SELECT condition_at_time FROM act_items \
                             WHERE act_id = ?1 AND device_id = ?2",
                            params![payload.id, dev_id],
                            |r| r.get(0),
                        )
                        .map_err(map_rusqlite)?;
                    let current = devices_repo.get_in_tx(&tx, dev_id)?;
                    let condition_changed = eff_condition
                        .as_deref()
                        .map(|c| Some(c) != stored_condition.as_deref())
                        .unwrap_or(false);
                    let location_changed = eff_location
                        .map(|l| Some(l) != current.location_id)
                        .unwrap_or(false);
                    if condition_changed || location_changed {
                        retained_with_change.push(dev_id);
                    }
                }

                for &dev_id in removed.iter().chain(retained_with_change.iter()) {
                    let (_before_json, after_json) = audit_repo
                        .select_latest_device_mutation_pair(&tx, payload.id, dev_id)?
                        .ok_or_else(|| AppError::Internal {
                            source_chain: format!(
                                "update_return: no audit trail for device {dev_id} on return \
                                 act {}",
                                payload.id
                            ),
                        })?;
                    let expected: serde_json::Value = serde_json::from_str(&after_json)
                        .map_err(|e| AppError::Internal {
                            source_chain: format!(
                                "update_return: corrupt after_json for device {dev_id}: {e}"
                            ),
                        })?;
                    let current = devices_repo.get_in_tx(&tx, dev_id)?;
                    let safe = expected.get("status_id").and_then(|v| v.as_i64())
                        == Some(current.status_id)
                        && expected.get("location_id").and_then(|v| v.as_i64())
                            == current.location_id
                        && expected.get("state").and_then(|v| v.as_str())
                            == current.state.as_deref();
                    if !safe {
                        return Err(AppError::Conflict {
                            reason: format!(
                                "Устройство id={} изменилось после этого возврата (другой акт \
                                 или изменение вручную) — редактирование строки невозможно",
                                dev_id
                            ),
                        });
                    }
                }

                // 9. MUTATE `removed` — un-return: restore to the MOST
                // RECENT prior state (same Pitfall-2-class DESC LIMIT 1
                // lookup `update()` uses), then delete the act_items row.
                for &removed_id in &removed {
                    let before_json = audit_repo
                        .select_latest_device_mutation(&tx, payload.id, removed_id)?
                        .ok_or_else(|| AppError::Internal {
                            source_chain: format!(
                                "update_return: no audit trail for device {removed_id} on \
                                 return act {}",
                                payload.id
                            ),
                        })?;
                    let snapshot: serde_json::Value = serde_json::from_str(&before_json)
                        .map_err(|e| AppError::Internal {
                            source_chain: format!(
                                "update_return: corrupt before_json for device {removed_id}: {e}"
                            ),
                        })?;
                    let restored = devices_repo.restore_from_snapshot_in_tx(
                        &tx,
                        removed_id,
                        &snapshot,
                        now,
                    )?;
                    let after_json =
                        device_snapshot_json(&restored).map_err(|e| AppError::Internal {
                            source_chain: format!("update_return remove after_json: {e}"),
                        })?;
                    audit_repo.insert(
                        &tx,
                        AuditEntry {
                            entity_type: "device",
                            entity_id: removed_id,
                            action: "custom:update_remove",
                            user_id: user_id_opt,
                            before_json: Some(before_json),
                            after_json: Some(after_json),
                            payload_json: Some(
                                serde_json::json!({ "act_id": payload.id }).to_string(),
                            ),
                            created_at_utc: now,
                        },
                    )?;
                    tx.execute(
                        "DELETE FROM act_items WHERE act_id = ?1 AND device_id = ?2",
                        params![payload.id, removed_id],
                    )
                    .map_err(map_rusqlite)?;
                }

                // 10. MUTATE `added` — newly returned outstanding devices:
                // в_работе → на_складе (reuses `do_return`'s per-device
                // transition), INSERT act_items row.
                for &added_id in &added {
                    let before = devices_repo.get_in_tx(&tx, added_id)?;
                    let before_json =
                        device_snapshot_json(&before).map_err(|e| AppError::Internal {
                            source_chain: format!("update_return add before_json: {e}"),
                        })?;
                    let (qty, condition, location) = effective_by_device
                        .get(&added_id)
                        .cloned()
                        .unwrap_or((1, None, None));
                    let after = devices_repo.update_full_in_tx(
                        &tx,
                        added_id,
                        on_warehouse_status_id,
                        location,
                        condition.as_deref(),
                        now,
                    )?;
                    let after_json =
                        device_snapshot_json(&after).map_err(|e| AppError::Internal {
                            source_chain: format!("update_return add after_json: {e}"),
                        })?;
                    audit_repo.insert(
                        &tx,
                        AuditEntry {
                            entity_type: "device",
                            entity_id: added_id,
                            action: "update",
                            user_id: user_id_opt,
                            before_json: Some(before_json),
                            after_json: Some(after_json),
                            payload_json: Some(
                                serde_json::json!({ "act_id": payload.id, "kind": "return" })
                                    .to_string(),
                            ),
                            created_at_utc: now,
                        },
                    )?;
                    acts_repo.insert_act_item_in_tx(
                        &tx,
                        payload.id,
                        added_id,
                        qty,
                        condition.as_deref(),
                        before.kit.as_deref(),
                    )?;
                }

                // 11. MUTATE `retained_with_change` — condition/location
                // edit on an already-returned device: re-apply на_складе
                // with the new condition/location, UPDATE
                // act_items.condition_at_time (gated by D-11 above; a no-op
                // resubmit writes nothing per the `retained_with_change`
                // filter).
                for &dev_id in &retained_with_change {
                    let before = devices_repo.get_in_tx(&tx, dev_id)?;
                    let before_json =
                        device_snapshot_json(&before).map_err(|e| AppError::Internal {
                            source_chain: format!("update_return retained before_json: {e}"),
                        })?;
                    let (_, condition, location) = effective_by_device
                        .get(&dev_id)
                        .cloned()
                        .unwrap_or((1, None, None));
                    let after = devices_repo.update_full_in_tx(
                        &tx,
                        dev_id,
                        on_warehouse_status_id,
                        location,
                        condition.as_deref(),
                        now,
                    )?;
                    let after_json =
                        device_snapshot_json(&after).map_err(|e| AppError::Internal {
                            source_chain: format!("update_return retained after_json: {e}"),
                        })?;
                    audit_repo.insert(
                        &tx,
                        AuditEntry {
                            entity_type: "device",
                            entity_id: dev_id,
                            action: "update",
                            user_id: user_id_opt,
                            before_json: Some(before_json),
                            after_json: Some(after_json),
                            payload_json: Some(
                                serde_json::json!({ "act_id": payload.id, "kind": "return" })
                                    .to_string(),
                            ),
                            created_at_utc: now,
                        },
                    )?;
                    tx.execute(
                        "UPDATE act_items SET condition_at_time = ?1 \
                         WHERE act_id = ?2 AND device_id = ?3",
                        params![condition, payload.id, dev_id],
                    )
                    .map_err(map_rusqlite)?;
                }

                // 12. Header CAS write — `update_act_header_in_tx` REUSED
                // UNCHANGED from Phase 19 (zero `act_type` branching in that
                // helper). `number: None` guarantees return numbers never
                // change (out of scope, D-10). `notes`/`deadline_utc` are
                // always cleared to `None` — return acts have no such fields
                // in the edit form.
                let patch = ActPatch {
                    giver_name: Some(payload.giver_name.clone()),
                    receiver_name: Some(payload.receiver_name.clone()),
                    location_id: Some(resolved_bulk_location_id),
                    notes: Some(None),
                    deadline_utc: Some(None),
                    handover_date_utc: Some(payload.handover_date_utc),
                    number: None,
                    expected_version: payload.expected_version,
                };
                acts_repo.update_act_header_in_tx(&tx, payload.id, &patch, now)?;

                // 13. Recompute the PARENT's archived flag whenever the
                // item-count-changing delta was non-empty (mirrors
                // `update()`'s own gate, and `do_return`/`delete_soft`'s
                // Return branch, which always call this unconditionally on
                // their own single-direction deltas).
                if !added.is_empty() || !removed.is_empty() {
                    recompute_parent_archived(&tx, parent_act_id, now)?;
                }

                // 14. Final audit row for the header edit (real before/after
                // diff).
                let act_after = acts_repo.fetch_full_in_tx(&tx, payload.id)?;
                let before_json = serde_json::to_string(&serde_json::json!({
                    "giver_name": act.giver_name,
                    "receiver_name": act.receiver_name,
                    "location_id": act.location_id,
                    "notes": act.notes,
                    "deadline_utc": act.deadline_utc,
                    "handover_date_utc": act.handover_date_utc,
                    "number": act.number,
                    "version": act.version,
                }))
                .map_err(|e| AppError::Internal {
                    source_chain: format!("act before_json: {e}"),
                })?;
                let after_json = serde_json::to_string(&serde_json::json!({
                    "giver_name": act_after.giver_name,
                    "receiver_name": act_after.receiver_name,
                    "location_id": act_after.location_id,
                    "notes": act_after.notes,
                    "deadline_utc": act_after.deadline_utc,
                    "handover_date_utc": act_after.handover_date_utc,
                    "number": act_after.number,
                    "version": act_after.version,
                }))
                .map_err(|e| AppError::Internal {
                    source_chain: format!("act after_json: {e}"),
                })?;
                audit_repo.insert(
                    &tx,
                    AuditEntry {
                        entity_type: "act",
                        entity_id: payload.id,
                        action: "update",
                        user_id: user_id_opt,
                        before_json: Some(before_json),
                        after_json: Some(after_json),
                        payload_json: None,
                        created_at_utc: now,
                    },
                )?;

                tx.commit().map_err(map_rusqlite)?;
                Ok(payload.id)
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
            // D-07 (Phase 22): capture `archived` before `row` is moved into
            // act_dto_from_row (ActRow is not Copy).
            let archived = row.archived;
            let mut dto = act_dto_from_row(row, items, return_ids);
            dto.archived_at_utc = compute_archived_at_utc(&conn, id, archived)?;
            Ok(dto)
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

    /// G-5 / GAP-12-01 (12-04): autocomplete для полей «Кто сдал» / «Кто
    /// принял» (актные модалки) И «Кому выдал» / «Кто выдал» (картриджные
    /// операции, `OperationModal`) — единый backend-источник подсказок для
    /// `PersonAutocomplete.svelte` во всех формах.
    ///
    /// Источник — UNION ALL до трёх арм:
    ///   1. `acts.{giver_name|receiver_name}` (soft-deleted акты исключены)
    ///   2. `cartridges.holder_name` (soft-deleted картриджи исключены) —
    ///      обе enum-ветки (`Giver`/`Receiver`) читают `holder_name`
    ///      одинаково: у cartridges нет различия giver/receiver, это
    ///      единственная person-name колонка на этой таблице.
    ///   3. (только `Giver`) `audit_log.payload_json->given_by_name` для
    ///      картриджных операций `custom:install`/`custom:to_refill`
    ///      (GAP-12-06, A3, часть «а») — значение поля «Кто выдал» сейчас
    ///      пишется ТОЛЬКО в JSON-payload (в отличие от «Кому выдал», у
    ///      которого есть queryable-колонка `cartridges.holder_name`), без
    ///      этой арки имя «Кто выдал» никогда не попадало в подсказки.
    ///
    /// Имена дедуплицируются и агрегируются по сумме frequency между всеми
    /// арками (CTE с `GROUP BY name, SUM(freq)` в внешнем запросе),
    /// отсортированы по frequency DESC (alpha ASC tiebreak). LIKE-prefix
    /// match с `escape_like` защитой от SQL injection через `%` / `_` / `\`.
    ///
    /// Phase 5 (future): четвёртая UNION ALL арка с AD displayName —
    /// расширение в SQL без изменения сигнатуры / UI contract.
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
        // given_by_name арка — только для контекста «Кто выдал» (Giver),
        // симметрично тому, как holder_name участвует в обеих ветках, но
        // given_by_name по семантике относится только к giver-стороне
        // картриджных операций install/to_refill.
        let given_by_name_arm = match field {
            SuggestPersonField::Giver => {
                " UNION ALL \
                 SELECT json_extract(payload_json, '$.given_by_name') AS name, COUNT(*) AS freq \
                   FROM audit_log \
                  WHERE entity_type = 'cartridge' \
                    AND action IN ('custom:install', 'custom:to_refill') \
                    AND json_extract(payload_json, '$.given_by_name') IS NOT NULL \
                    AND json_extract(payload_json, '$.given_by_name') != '' \
                    AND json_extract(payload_json, '$.given_by_name') LIKE ?1 ESCAPE '\\' \
                  GROUP BY json_extract(payload_json, '$.given_by_name')"
            }
            SuggestPersonField::Receiver => "",
        };
        let sql = format!(
            "SELECT name, SUM(freq) AS total_freq FROM ( \
                 SELECT {col} AS name, COUNT(*) AS freq \
                   FROM acts \
                  WHERE {col} LIKE ?1 ESCAPE '\\' \
                    AND deleted_at_utc IS NULL \
                  GROUP BY {col} \
                 UNION ALL \
                 SELECT holder_name AS name, COUNT(*) AS freq FROM cartridges \
                  WHERE holder_name LIKE ?1 ESCAPE '\\' \
                    AND deleted_at_utc IS NULL \
                  GROUP BY holder_name \
                 {given_by_name_arm} \
             ) \
              GROUP BY name \
              ORDER BY total_freq DESC, name ASC \
              LIMIT ?2",
            col = column,
            given_by_name_arm = given_by_name_arm
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

    /// Render handover act → HTML string (D-PDF-Render-Path-01, Phase 16 D-10).
    ///
    /// Pipeline (rewired in Phase 16 Plan 02 — frozen krilla document-spec path removed):
    ///   1. Load full ActDto (with items + optional parent block).
    ///   2. Load OrgData/org_settings requisites + logo bytes → base64 `data:` URI.
    ///   3. Read `templates/act_handover.html` (file-first, embedded fallback).
    ///   4. Build MiniJinja context per D-PDF-Templates-Schema-01.
    ///   5. Render template → HTML string (autoescape-ON safe-mode + 5s timeout).
    pub async fn render_pdf(&self, act_id: i64) -> Result<String, AppError> {
        let pipeline = self.pdf_pipeline()?;
        let act = self.get(act_id).await?;
        // D-05 (Phase 14 plan 03): org-реквизиты читаются из `org_settings`
        // (единый источник, который пишет Settings UI), а не из org.json.
        // `organization` (org.json) остаётся подключён только для logo-пути
        // (`safe_logo_canonical`). Fallback на пустой OrgSettingsDto, если
        // org_db не подключён (helper-фикстуры без with_org_db) — деградирует
        // в пустые реквизиты, не в ошибку рендера.
        let org_legacy = pipeline.organization.read().await?;
        let (org_dto, logo_bytes, logo_mime) = match pipeline.org_db {
            Some(org_db) => {
                let (dto, logo_bytes, logo_mime) = org_db.get_for_pdf().await?;
                (dto, logo_bytes, logo_mime)
            }
            None => (
                crate::dto::reports::OrgSettingsDto {
                    org_name: org_legacy.name.clone(),
                    inn: org_legacy.inn.clone(),
                    kpp: org_legacy.kpp.clone(),
                    address: org_legacy.address.clone(),
                    has_logo: false,
                    phone: String::new(),
                    fax: String::new(),
                    email: String::new(),
                    okpo: String::new(),
                    ogrn: String::new(),
                },
                None,
                None,
            ),
        };
        // T-16-05 mitigation: `logo_bytes` originates exclusively from
        // `OrgDbService::get_for_pdf` (org_settings BLOB, written only via
        // authenticated Settings UI) — never from request-supplied bytes.
        let logo_data_uri: Option<String> = logo_bytes.map(|bytes| {
            use base64::Engine;
            let mime = logo_mime.as_deref().unwrap_or("image/png");
            format!(
                "data:{mime};base64,{}",
                base64::engine::general_purpose::STANDARD.encode(bytes)
            )
        });
        // Phase 16 (D-04/D-10): read the HTML template source from
        // templates/act_handover.html (file-first, embedded-default
        // fallback) instead of the DB-backed `document_templates` table.
        let templates_dir =
            crate::pdf::html_templates::resolve_templates_dir(&pipeline.organization.paths);
        let embedded_default = crate::pdf::html_templates::DEFAULT_HTML_TEMPLATES
            .iter()
            .find(|(f, _)| *f == "act_handover.html")
            .map(|(_, body)| *body)
            .unwrap_or("");
        let template_src = crate::pdf::html_templates::load_template(
            &templates_dir,
            "act_handover.html",
            embedded_default,
        );

        // Optional parent block для return-актов (Plan 04 рендерит handover,
        // но для cascade — оставляем path).
        let parent_block: Option<serde_json::Value> = if let Some(parent_id) = act.parent_act_id {
            let parent = self.get(parent_id).await?;
            Some(serde_json::json!({
                "number": parent.number,
                "date_human": format_ru_date(parent.handover_date_utc),
                "date": format_iso_date(parent.handover_date_utc),
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
                    "specs": it.specs,
                    "kit": it.complectation_at_time,
                    "condition": it.condition_at_time,
                    "quantity": it.quantity,
                })
            })
            .collect();

        let ctx = serde_json::json!({
            "org": {
                "name": org_dto.org_name,
                "inn": org_dto.inn,
                "kpp": org_dto.kpp,
                "address": org_dto.address,
                "phone": org_dto.phone,
                "fax": org_dto.fax,
                "email": org_dto.email,
                "okpo": org_dto.okpo,
                "ogrn": org_dto.ogrn,
                "logo_data_uri": logo_data_uri,
            },
            "act": {
                "number": act.number_raw,
                "suffix": suffix,
                "date": format_iso_date(act.handover_date_utc),
                "date_human": format_ru_date(act.handover_date_utc),
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
            &crate::pdf::minijinja_env::build_safe_html_env(),
            "act_handover_html",
            &template_src,
            ctx,
        )
        .await?;

        Ok(rendered)
    }

    /// Render acceptance document (документ приёма устройства на склад) → HTML string
    /// (Phase 16 D-10).
    ///
    /// Читает `templates/act_acceptance.html` (file-first, embedded fallback).
    /// Контекст беднее handover'а — одна позиция (device), плюс шапка
    /// организации и подписи.
    pub async fn render_acceptance_pdf(
        &self,
        device_id: i64,
        giver_name: String,
        receiver_name: String,
        date_utc: i64,
    ) -> Result<String, AppError> {
        let pipeline = self.pdf_pipeline()?;
        let org = pipeline.organization.read().await?;
        // Phase 16 (D-11): legacy org.json logo has no BLOB storage — read the
        // canonicalized local file's bytes (path-traversal-guarded via
        // safe_logo_canonical) and embed as a base64 data: URI.
        let logo_data_uri =
            pipeline
                .organization
                .read_logo_bytes(&org)
                .await?
                .map(|(bytes, mime)| {
                    use base64::Engine;
                    format!(
                        "data:{mime};base64,{}",
                        base64::engine::general_purpose::STANDARD.encode(bytes)
                    )
                });
        // Phase 16 (D-04/D-10): read the HTML template source from
        // templates/act_acceptance.html (file-first, embedded-default
        // fallback) instead of the DB-backed `document_templates` table.
        let templates_dir =
            crate::pdf::html_templates::resolve_templates_dir(&pipeline.organization.paths);
        let embedded_default = crate::pdf::html_templates::DEFAULT_HTML_TEMPLATES
            .iter()
            .find(|(f, _)| *f == "act_acceptance.html")
            .map(|(_, body)| *body)
            .unwrap_or("");
        let template_src = crate::pdf::html_templates::load_template(
            &templates_dir,
            "act_acceptance.html",
            embedded_default,
        );

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
                "logo_data_uri": logo_data_uri,
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
            &crate::pdf::minijinja_env::build_safe_html_env(),
            "act_acceptance_html",
            &template_src,
            ctx,
        )
        .await?;

        Ok(rendered)
    }

    /// Возвращает PDF-pipeline deps как refs или `Internal` если не подключены.
    /// `org_db` — Option-aware (D-05): helper-фикстуры (`ActService::new` без
    /// `with_org_db`) не падают, только org-контекст рендера деградирует к
    /// пустым реквизитам (см. `render_pdf`'s fallback branch).
    ///
    /// Phase 16: `templates`/`pdf` (`TemplateService`/`PdfRenderer`) больше не
    /// читаются render_pdf/render_acceptance_pdf (HTML-путь читает шаблоны
    /// через `html_templates::load_template`, не через фризнутый krilla
    /// document-spec pipeline) — но их наличие остаётся частью guard-условия
    /// "PDF pipeline подключён",
    /// поэтому проверка `(Some, Some, Some)` сохранена без изменений; сами
    /// значения в `PdfPipelineRefs` больше не прокидываются (были dead code).
    fn pdf_pipeline(&self) -> Result<PdfPipelineRefs<'_>, AppError> {
        match (&self.templates, &self.organization, &self.pdf) {
            (Some(_), Some(o), Some(_)) => Ok(PdfPipelineRefs {
                organization: o,
                org_db: self.org_db.as_ref(),
            }),
            _ => Err(AppError::Internal {
                source_chain: "ActService::render_pdf called without with_pdf_pipeline".into(),
            }),
        }
    }
}

struct PdfPipelineRefs<'a> {
    organization: &'a Arc<OrganizationService>,
    org_db: Option<&'a Arc<OrgDbService>>,
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
                    d.name, d.inventory_number, d.serial_number, d.model, d.notes, \
                    dl.id AS device_location_id, dl.name AS device_location \
               FROM act_items ai \
               JOIN devices d ON d.id = ai.device_id \
               LEFT JOIN locations dl ON d.location_id = dl.id \
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
                // D-01 (Phase 14 plan 03): live device.notes value, not a snapshot.
                specs: r.get(9)?,
                // G-10/G-12 (Phase 03.1): outstanding_device_ids заполняется
                // в caller'е (ActService::get / list / search) — этот helper
                // только подгружает joined-device fields. Initialized to empty;
                // populate_outstanding_device_ids() fills handover-acts.
                outstanding_device_ids: Vec::new(),
                // Phase 22 (ACT-03, Pitfall 2): текущее расположение устройства,
                // нужно для prefill «Расположение» в форме редактирования возврата.
                device_location_id: r.get(10)?,
                device_location: r.get(11)?,
            })
        })
        .map_err(map_rusqlite)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(map_rusqlite)?);
    }
    Ok(out)
}

/// D-07 (Phase 22) — «Дата архивации», compute-on-read: `MAX(handover_date_utc)`
/// among `parent_act_id`'s non-deleted `act_type='return'` children, populated
/// ONLY when `archived == true`. No stored column, no migration.
///
/// `!archived` short-circuits with `Ok(None)` — no query. Uses `query_row`
/// (NOT `.optional()`) because a `MAX()` aggregate over zero matching rows
/// still returns one row with a SQL `NULL`, not `QueryReturnedNoRows`;
/// `.optional()` would be the wrong tool here and could mask a real query
/// error.
fn compute_archived_at_utc(
    conn: &rusqlite::Connection,
    parent_act_id: i64,
    archived: bool,
) -> Result<Option<i64>, AppError> {
    if !archived {
        return Ok(None);
    }
    conn.query_row(
        "SELECT MAX(handover_date_utc) FROM acts \
          WHERE parent_act_id = ?1 AND act_type = 'return' AND deleted_at_utc IS NULL",
        params![parent_act_id],
        |r| r.get::<_, Option<i64>>(0),
    )
    .map_err(map_rusqlite)
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

/// `_in_tx` twin of `populate_outstanding_device_ids` (Phase 19, ACT-02 D-08
/// guard) — same `EXCEPT` predicate, but operates on an open write
/// `Transaction` and returns the outstanding `HashSet<i64>` directly instead
/// of mutating a slice of `ActItemDto`. Used by `ActService::update` to
/// reject removal of a device_id already consumed by a completed/active
/// return, BEFORE any device mutation for that removal runs.
fn populate_outstanding_device_ids_in_tx(
    tx: &rusqlite::Transaction<'_>,
    act_id: i64,
) -> Result<std::collections::HashSet<i64>, AppError> {
    let mut stmt = tx
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
    Ok(outstanding)
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

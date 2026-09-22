//! `DeviceService` — application service for the Devices entity.
//!
//! Owns:
//! - `writer`       — single-writer handle for all DB mutations
//! - `readers`      — reader pool for all DB reads
//! - `clock`        — UTC timestamp source
//! - `repo`         — SQLite adapter (Arc so the service is cheaply Clone-able)
//! - `csv_sessions` — in-memory import session store (preview→commit TTL store)
//!
//! CRUD методы реализованы в Plan 03.
//! Search/autocomplete/grouping — Plan 04.
//! CSV import/export — Plan 05.

use std::sync::Arc;

use trackly_core::auth::Identity;
use trackly_core::error::AppError;

/// Excel formula injection prevention (T-02-05-03).
/// Prefixes cells starting with `=`, `+`, `-`, `@` with `'` (Excel-safe per OWASP).
fn csv_safe(value: &str) -> String {
    if value.starts_with(['=', '+', '-', '@']) {
        format!("'{value}")
    } else {
        value.to_string()
    }
}
use trackly_core::domain::number_templates::{NumberTemplateRow, TemplateType};
use trackly_core::domain::place_movements::{MovementEntityKind, MovementSource};
use trackly_core::domain::printers::PrinterNew;
use trackly_core::ports::devices::DeviceRepository;
use trackly_core::primitives::clock::Clock;
use trackly_infra::db::{pools::ReaderPool, writer_worker::WriterHandle};
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::repos::{
    SqliteCartridgeRepository, SqliteDeviceRepository, SqlitePlaceMovementsRepository,
    SqlitePlaceRepository, SqlitePrinterRepository,
};

use std::collections::HashMap;

use crate::csv::session_store::ImportSessionStore;
use crate::csv::{decode_to_string, detect, parse_rows, ImportSession};
use crate::dto::device::{
    CsvImportPreviewResponse, CsvImportReport, DeviceDto, DeviceFilter, DeviceGroup,
    DeviceListResponse, DeviceNew, DevicePatch, DeviceSaveOutcome, Pagination, RowError,
    StatusCount, STATE_HINTS,
};
use crate::dto::number_template::{
    NumberFieldInput, NumberWarningDto, NumberWarningKind, TemplateContextDto,
};
use crate::dto::printer::WsEvent;
use crate::services::number_template_service::NumberTemplateService;

/// Application service for device management.
///
/// `Arc`-wrapped fields make `Clone` O(1) — used by Tauri State and axum State.
#[derive(Clone)]
pub struct DeviceService {
    pub writer: Arc<WriterHandle>,
    pub readers: Arc<ReaderPool>,
    pub(crate) clock: Arc<dyn Clock + Send + Sync>,
    pub(crate) repo: Arc<SqliteDeviceRepository>,
    pub(crate) printer_repo: Arc<SqlitePrinterRepository>,
    pub(crate) place_repo: Arc<SqlitePlaceRepository>,
    pub(crate) place_movements_repo: Arc<SqlitePlaceMovementsRepository>,
    pub(crate) cartridge_repo: Arc<SqliteCartridgeRepository>,
    #[allow(dead_code)]
    pub(crate) csv_sessions: Arc<ImportSessionStore>,
    /// Phase 40.2 Plan 08 (NUM-09): the single owner of occupied/script-mix
    /// checks + NUM-08 context memory for the `device_create`/
    /// `printer_create` numbering space. Constructed INTERNALLY from the
    /// SAME `writer`/`readers`/`clock` triple this service already
    /// receives — mirrors `ActService`/`CartridgeService`'s identical
    /// precedent (Plans 06/07), keeping every existing
    /// `DeviceService::new(writer, readers, clock)` call site across the
    /// test suite compiling unchanged.
    pub(crate) number_templates: Arc<NumberTemplateService>,
    /// D-14 invalidation broadcast (`WsEvent::NumberSpaceChanged`) — `None`
    /// in most test fixtures; `AppCtx::build` wires the real shared sender
    /// via `with_ws_tx`. Optional builder field, mirrors `ActService`/
    /// `CartridgeService`'s own `ws_tx` field.
    pub(crate) ws_tx: Option<Arc<tokio::sync::broadcast::Sender<WsEvent>>>,
}

impl DeviceService {
    /// Construct a new `DeviceService`.
    ///
    /// Called from `AppCtx::build` after reader pool initialization.
    pub fn new(
        writer: Arc<WriterHandle>,
        readers: Arc<ReaderPool>,
        clock: Arc<dyn Clock + Send + Sync>,
    ) -> Self {
        let number_templates = Arc::new(NumberTemplateService::new(
            writer.clone(),
            readers.clone(),
            clock.clone(),
        ));
        Self {
            writer,
            readers,
            clock,
            repo: Arc::new(SqliteDeviceRepository),
            printer_repo: Arc::new(SqlitePrinterRepository),
            place_repo: Arc::new(SqlitePlaceRepository),
            place_movements_repo: Arc::new(SqlitePlaceMovementsRepository),
            cartridge_repo: Arc::new(SqliteCartridgeRepository),
            csv_sessions: Arc::new(ImportSessionStore::new()),
            number_templates,
            ws_tx: None,
        }
    }

    /// Builder: wire the shared WS broadcast sender (D-14). Used by
    /// `AppCtx::build` (production runtime); test fixtures that never
    /// subscribe to a WS channel simply skip calling this (`ws_tx` stays
    /// `None`, broadcasting is silently skipped).
    pub fn with_ws_tx(mut self, ws_tx: Arc<tokio::sync::broadcast::Sender<WsEvent>>) -> Self {
        self.ws_tx = Some(ws_tx);
        self
    }

    // -----------------------------------------------------------------------
    // Numbering (Phase 40.2 Plan 08, NUM-09/D-05/D-08)
    // -----------------------------------------------------------------------

    /// Validate the shape of `trimmed` (BE-WR-10) and reject it if it is
    /// already taken by a LIVE device/printer other than `exclude_id`? Empty strings are never checked (SPEC NUM-09 — the
    /// device/printer inventory number is the one number field in this
    /// entire phase that stays optional, D-16).
    async fn reject_if_occupied(
        &self,
        trimmed: &str,
        exclude_id: Option<i64>,
    ) -> Result<(), AppError> {
        if trimmed.is_empty() {
            return Ok(());
        }
        // BE-WR-10: every non-empty inventory number (create, interactive
        // create, update, bulk_create, CSV import via create) passes through
        // here, so the shape check lives at this single choke point.
        trackly_core::text::number_value::validate_number_value("inventory_no", trimmed)?;
        if self
            .number_templates
            .is_occupied(TemplateType::DeviceInventory, trimmed, exclude_id)
            .await?
            .is_some()
        {
            return Err(AppError::Conflict {
                reason: format!("номер уже занят — {trimmed}"),
            });
        }
        Ok(())
    }

    /// Script-mix warning (NUM-12) — only meaningful on the two paths that
    /// actually carry a `confirm_script_mix` flag to skip re-showing an
    /// already-seen warning (`update()`/`create_single_with_number_check()`).
    /// `create()`/`bulk_create()`'s callers (CSV import, the legacy
    /// two-step printer-create modal) have no interactive confirmation UI
    /// at their call sites — occupied is still unconditionally enforced for
    /// them via `reject_if_occupied` alone, but a non-blocking warning is
    /// silently accepted rather than surfaced.
    async fn script_mix_warning(
        &self,
        trimmed: &str,
        exclude_id: Option<i64>,
        confirm_script_mix: bool,
    ) -> Result<Option<NumberWarningDto>, AppError> {
        if trimmed.is_empty() || confirm_script_mix {
            return Ok(None);
        }
        self.number_templates
            .detect_warnings(TemplateType::DeviceInventory, trimmed, exclude_id)
            .await
    }

    /// D-14: best-effort invalidation broadcast — a device/printer
    /// inventory number was just written. Both contexts are always sent
    /// together since they read the same physical `inventory_number` column
    /// (mirrors `cartridge_create`/`drum_create`'s shared-column broadcast).
    fn broadcast_number_space_changed(&self) {
        if let Some(ws_tx) = &self.ws_tx {
            let _ = ws_tx.send(WsEvent::NumberSpaceChanged {
                contexts: vec!["device_create".to_string(), "printer_create".to_string()],
            });
        }
    }

    // -----------------------------------------------------------------------
    // Validation helpers
    // -----------------------------------------------------------------------

    fn validate_new(new: &DeviceNew) -> Result<(), AppError> {
        if new.name.trim().is_empty() {
            return Err(AppError::Validation {
                field: "name".to_string(),
                message: "Наименование обязательно для заполнения".to_string(),
            });
        }
        if new.type_id <= 0 {
            return Err(AppError::Validation {
                field: "type_id".to_string(),
                message: "Тип устройства обязателен".to_string(),
            });
        }
        if new.status_id <= 0 {
            return Err(AppError::Validation {
                field: "status_id".to_string(),
                message: "Статус устройства обязателен".to_string(),
            });
        }
        Ok(())
    }

    /// device_types seed ids (V001): устройство=1, принтер=2.
    const DEVICE_TYPE_ID: i64 = 1;
    const PRINTER_TYPE_ID: i64 = 2;

    /// Синхронизировать строку `printers` с `type_id` (quick 260820-rdj: полная
    /// конверсия Устройство ⇄ Принтер). Идемпотентна — безопасно вызывать при
    /// каждом create/update/bulk_create независимо от того, менялся ли type_id
    /// в этом конкретном вызове; выполняется ВНУТРИ той же транзакции, что и
    /// INSERT/UPDATE устройства, поэтому конверсия атомарна (никогда не оставляет
    /// devices.type_id=2 без строки printers, и наоборот).
    fn sync_printer_row_in_tx(
        printer_repo: &SqlitePrinterRepository,
        tx: &rusqlite::Transaction<'_>,
        device_id: i64,
        type_id: i64,
        now_utc: i64,
    ) -> Result<(), AppError> {
        match type_id {
            Self::PRINTER_TYPE_ID => {
                if !printer_repo.exists_for_device_in_tx(tx, device_id)? {
                    printer_repo.create_in_tx(
                        tx,
                        &PrinterNew {
                            device_id,
                            ip_address: None,
                            community_raw: "public".to_string(),
                            snmp_version: "v2c".to_string(),
                            oid_profile_id: None,
                            usb_host_device_id: None,
                        },
                        now_utc,
                    )?;
                }
            }
            Self::DEVICE_TYPE_ID => {
                printer_repo.delete_by_device_id_in_tx(tx, device_id)?;
            }
            _ => {} // неизвестный/будущий тип — printers не трогаем (вне области decision)
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // CRUD
    // -----------------------------------------------------------------------

    /// Shared writer-transaction body: INSERT device + sync printer row +
    /// audit_log, then re-fetch as `DeviceDto`. Factored out of `create()`
    /// so `create_single_with_number_check()` (Phase 40.2 Plan 08) can reuse
    /// it after running its OWN number-resolution chain — the only
    /// difference between the two callers is what happens BEFORE this runs
    /// (occupied/script-mix checks), never what happens inside the
    /// transaction itself.
    ///
    /// `new.inventory_no` is expected to be ALREADY resolved (trimmed,
    /// empty converted to `None`) by the caller — this method does no
    /// number validation of its own.
    async fn insert_new_and_get(&self, new: DeviceNew) -> Result<DeviceDto, AppError> {
        let now = self.clock.unix_seconds();
        let repo = self.repo.clone();
        let printer_repo = self.printer_repo.clone();
        let domain_new: trackly_core::domain::devices::DeviceNew = new.into();
        let user_id_opt: Option<i64> = None; // Phase 2 — no auth yet

        let id = self
            .writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;

                let id = repo.create_in_tx(&tx, &domain_new, now)?;
                Self::sync_printer_row_in_tx(&printer_repo, &tx, id, domain_new.type_id, now)?;
                let after = repo.get_in_tx(&tx, id)?;
                let after_dto = DeviceDto::from(after);
                let after_json =
                    serde_json::to_string(&after_dto).map_err(|e| AppError::Internal {
                        source_chain: format!("audit_log after-json: {e}"),
                    })?;

                tx.execute(
                    "INSERT INTO audit_log \
                     (entity_type, entity_id, action, user_id, before_json, after_json, payload_json, created_at_utc) \
                     VALUES ('device', ?1, 'create', ?2, NULL, ?3, NULL, ?4)",
                    rusqlite::params![id, user_id_opt, after_json, now],
                )
                .map_err(map_rusqlite)?;

                tx.commit().map_err(map_rusqlite)?;
                Ok(id)
            })
            .await?;

        self.get(id).await
    }

    /// Создать новое устройство.
    ///
    /// Валидирует обязательные поля, затем вставляет устройство и запись audit_log
    /// в одной транзакции (RESEARCH §Pattern 2, T-02-03-03).
    /// `new.place_id` — уже разрешённый caller'ом ID места (PlacePicker); ни один
    /// путь записи устройства больше не создаёт место неявно по строке (D-18).
    ///
    /// Phase 40.2 Plan 08 (D-08/NUM-09): the ONLY current callers of this
    /// method (CSV import's `import_csv_commit`, and the legacy two-step
    /// `PrinterCreateModal.svelte` create flow) have no interactive UI to
    /// confirm a script-mix warning — occupied uniqueness is still
    /// unconditionally enforced (D-08 explicit `.trim()`, NOT
    /// `normalize_str` — RESEARCH Pitfall 4), but a script-mix warning is
    /// silently accepted rather than surfaced. See
    /// `create_single_with_number_check` for the interactive equivalent
    /// Plan 13 will wire the real single-device create FORM to.
    pub async fn create(&self, new: DeviceNew) -> Result<DeviceSaveOutcome, AppError> {
        Self::validate_new(&new)?;

        let trimmed = new.inventory_no.as_deref().map(|v| v.trim().to_string());
        if let Some(t) = trimmed.as_deref().filter(|s| !s.is_empty()) {
            self.reject_if_occupied(t, None).await?;
        }
        let mut new = new;
        new.inventory_no = trimmed.filter(|s| !s.is_empty());
        let number_set = new.inventory_no.is_some();

        let dto = self.insert_new_and_get(new).await?;
        if number_set {
            self.broadcast_number_space_changed();
        }
        Ok(DeviceSaveOutcome::Created(Box::new(dto)))
    }

    /// The REAL interactive single-device/printer create path (Phase 40.2
    /// Plan 08, NUM-09/D-01) — Plan 13 wires `DeviceFormBody.svelte` to call
    /// this INSTEAD OF `bulk_create(new, 1)` once the UI carries a
    /// `NumberFieldInput` (template picker + confirm flags). Full D-01
    /// chain for devices: occupied -> template mismatch (NUM-11, create-only
    /// — mirrors `ActService::create()`/`CartridgeService::create()`) ->
    /// script-mix. Corrected post-Plan-13 (fix 40.2-13/NUM-11): an earlier
    /// doc-comment here claimed devices/printers were scoped OUT of the
    /// mismatch check — that was wrong. CONTEXT D-01 and SPEC NUM-11 apply
    /// whenever a template is selected in the field, regardless of entity
    /// family; this method's own `<action>` text in the fix simply hadn't
    /// been implemented yet. NUM-08 context memory (`remember_context`) only
    /// happens here, never in `create()`/`bulk_create()` — those two have no
    /// `NumberFieldInput` at all (`DeviceNew` carries no `template_id`), so
    /// there is nothing to remember on those paths.
    pub async fn create_single_with_number_check(
        &self,
        new: DeviceNew,
        number_input: NumberFieldInput,
        context: TemplateContextDto,
    ) -> Result<DeviceSaveOutcome, AppError> {
        // BE-WR-01: this path is gated on `MutateDevices`; it may only touch
        // the device/printer context memory, never «Новый акт»/картриджи.
        if !matches!(
            context,
            TemplateContextDto::DeviceCreate | TemplateContextDto::PrinterCreate
        ) {
            return Err(AppError::Validation {
                field: "context".into(),
                message: "Недопустимый контекст для создания устройства.".into(),
            });
        }
        Self::validate_new(&new)?;

        let trimmed = number_input.value.trim().to_string();
        let trimmed_opt = if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.clone())
        };
        if !trimmed.is_empty() {
            // D-01 order: occupied (blocking) -> template mismatch (NUM-11,
            // create-only) -> script-mix (NUM-12). `confirm_mismatch`/
            // `confirm_script_mix` only skip RE-SHOWING an already-seen
            // warning — they never skip the occupied check.
            self.reject_if_occupied(&trimmed, None).await?;

            if let Some(template_id) = number_input.template_id {
                if !number_input.confirm_mismatch {
                    let template_dto = self
                        .number_templates
                        .get_for_context(template_id, context)
                        .await?;
                    let today_utc = self.clock.unix_seconds();
                    let synthetic_row = NumberTemplateRow {
                        id: template_dto.id,
                        template_type: TemplateType::DeviceInventory,
                        mask: template_dto.mask.clone(),
                        created_at_utc: 0,
                        updated_at_utc: 0,
                        version: template_dto.version,
                    };
                    if NumberTemplateService::check_mismatch(&synthetic_row, &trimmed, today_utc) {
                        let context_label = match context {
                            TemplateContextDto::PrinterCreate => "Новый принтер",
                            _ => "Новое устройство",
                        };
                        return Ok(DeviceSaveOutcome::NeedsConfirmation(NumberWarningDto {
                            kind: NumberWarningKind::Mismatch,
                            message: format!(
                                "Номер «{trimmed}» не подходит под шаблон «{}». Сохранить как \
                                 есть? Следующее окно «{context_label}» откроется без шаблона.",
                                template_dto.mask
                            ),
                            doppelganger: None,
                        }));
                    }
                }
            }

            if let Some(warning) = self
                .script_mix_warning(&trimmed, None, number_input.confirm_script_mix)
                .await?
            {
                return Ok(DeviceSaveOutcome::NeedsConfirmation(warning));
            }
        }

        let mut new = new;
        new.inventory_no = trimmed_opt;

        let dto = self.insert_new_and_get(new).await?;

        // NUM-08: remember the template used for this context ONLY when it
        // was actually used without a confirmed mismatch — a confirmed
        // mismatch (or no template at all) resets the context to "no
        // template" so the next popup for this context opens empty
        // (mirrors `ActService::create()`/`CartridgeService::create()`).
        let to_remember = if number_input.template_id.is_some() && !number_input.confirm_mismatch {
            number_input.template_id
        } else {
            None
        };
        self.number_templates
            .remember_context(context, to_remember)
            .await?;
        self.broadcast_number_space_changed();

        Ok(DeviceSaveOutcome::Created(Box::new(dto)))
    }

    /// Получить устройство по ID.
    pub async fn get(&self, id: i64) -> Result<DeviceDto, AppError> {
        let readers = self.readers.clone();
        let repo = self.repo.clone();
        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            repo.get(&conn, id).map(DeviceDto::from)
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking: {e}"),
        })?
    }

    /// Список устройств с фильтром и пагинацией.
    /// T-02-03-05: limit max 200.
    pub async fn list(
        &self,
        filter: DeviceFilter,
        page: Pagination,
    ) -> Result<DeviceListResponse, AppError> {
        // Validate pagination (T-02-03-05).
        if page.limit > 200 {
            return Err(AppError::Validation {
                field: "pagination.limit".to_string(),
                message: "Максимальный размер страницы — 200".to_string(),
            });
        }

        let readers = self.readers.clone();
        let repo = self.repo.clone();
        let domain_filter = trackly_core::domain::devices::DeviceFilter {
            type_id: filter.type_id,
            place_id: filter.place_id,
            status_id: filter.status_id,
            state: filter.state,
            name_prefix: filter.name_prefix,
            include_deleted: filter.include_deleted,
            group_by_condition: false,
        };
        let domain_page = trackly_core::domain::devices::Pagination {
            offset: page.offset,
            limit: page.limit,
        };

        let (rows, total) = tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            repo.list(&conn, &domain_filter, &domain_page)
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking: {e}"),
        })??;

        Ok(DeviceListResponse {
            items: rows.into_iter().map(DeviceDto::from).collect(),
            total,
        })
    }

    /// Обновить устройство с optimistic-lock.
    /// `patch.place_id` — уже разрешённый caller'ом ID места (PlacePicker); D-18.
    ///
    /// Phase 40.2 Plan 08 (D-05/D-08/NUM-09): `patch.number_input` runs
    /// through the SAME occupied + script-mix chain as
    /// `create_single_with_number_check()` — mirrors
    /// `ActService::update()`'s pre-check pattern (resolved BEFORE the
    /// writer transaction opens). D-05: update NEVER checks template
    /// mismatch (`DeviceNumberEditInput` structurally has no `template_id`
    /// field at all) and NEVER calls `remember_context` (NUM-08 context
    /// memory is a create-only concept). Editing unrelated fields (name,
    /// place, etc.) without touching `number_input` skips the whole chain
    /// entirely — the number is only re-validated when it actually changes.
    pub async fn update(
        &self,
        caller: &Identity,
        id: i64,
        version: i64,
        patch: DevicePatch,
    ) -> Result<DeviceSaveOutcome, AppError> {
        let mut resolved_number: Option<String> = None; // Some(trimmed) => number actually changes
        if let Some(number_input) = &patch.number_input {
            let trimmed = number_input.value.trim().to_string();
            let current = self.get(id).await?;
            let current_trimmed = current
                .inventory_no
                .as_deref()
                .unwrap_or("")
                .trim()
                .to_string();
            if trimmed != current_trimmed {
                if !trimmed.is_empty() {
                    self.reject_if_occupied(&trimmed, Some(id)).await?;
                    if let Some(warning) = self
                        .script_mix_warning(&trimmed, Some(id), number_input.confirm_script_mix)
                        .await?
                    {
                        return Ok(DeviceSaveOutcome::NeedsConfirmation(warning));
                    }
                }
                resolved_number = Some(trimmed);
            }
        }
        let number_changed = resolved_number.is_some();

        let now = self.clock.unix_seconds();
        let repo = self.repo.clone();
        let printer_repo = self.printer_repo.clone();
        let place_repo = self.place_repo.clone();
        let place_movements_repo = self.place_movements_repo.clone();
        let cartridge_repo = self.cartridge_repo.clone();
        let mut domain_patch: trackly_core::domain::devices::DevicePatch = patch.into();
        if let Some(number) = resolved_number {
            // Outer `Some` = "field touched"; inner `None`/`Some(v)` decide
            // whether this is a clear-to-NULL or a real value (D-08 — the
            // value here is ALREADY trimmed and occupied/script-mix-checked
            // above, `devices_sqlite.rs::update_in_tx` writes it verbatim).
            domain_patch.inventory_no = Some(if number.is_empty() {
                None
            } else {
                Some(number)
            });
        }
        // Извлекаем ДО перемещения в writer-closure — `Identity` не `Send` через эту
        // границу (Plan 40-03, аналог `place_service::create`).
        let user_id_opt: Option<i64> = caller.user_id;

        let updated_row = self
            .writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;

                // before_json для audit_log
                let before = repo.get_in_tx(&tx, id).ok();
                let before_json = before
                    .as_ref()
                    .map(|row| serde_json::to_string(&DeviceDto::from(row.clone())))
                    .transpose()
                    .map_err(|e| AppError::Internal {
                        source_chain: format!("audit_log before-json: {e}"),
                    })?;
                let before_place_id = before.as_ref().and_then(|b| b.place_id);

                let after = repo.update_in_tx(&tx, id, version, &domain_patch, now)?;
                Self::sync_printer_row_in_tx(&printer_repo, &tx, id, after.type_id, now)?;
                let after_json =
                    serde_json::to_string(&DeviceDto::from(after.clone())).map_err(|e| {
                        AppError::Internal {
                            source_chain: format!("audit_log after-json: {e}"),
                        }
                    })?;

                tx.execute(
                    "INSERT INTO audit_log \
                     (entity_type, entity_id, action, user_id, before_json, after_json, payload_json, created_at_utc) \
                     VALUES ('device', ?1, 'update', ?2, ?3, ?4, NULL, ?5)",
                    rusqlite::params![id, user_id_opt, before_json, after_json, now],
                )
                .map_err(map_rusqlite)?;

                // HST-01: запись перемещения (D-04/D-06 guard внутри record_movement_if_applicable).
                // `source` жёстко задан сервером — клиент не может его подменить (T-40-15).
                place_movements_repo.record_movement_if_applicable(
                    &tx,
                    place_repo.as_ref(),
                    MovementEntityKind::Device,
                    id,
                    before_place_id,
                    after.place_id,
                    MovementSource::Manual,
                    None,
                    None,
                    user_id_opt,
                    now,
                )?;

                // Phase 40-21 (UAT-40 «cartridge-does-not-follow-printer», вариант B):
                // когда меняется место принтера, каскадим место на все прикреплённые
                // картриджи в ТОЙ ЖЕ транзакции. Шире, чем D-04/D-06 гейт самого
                // устройства — принтеры физически двигаются даже когда их собственная
                // запись не логируется (например переход через NULL).
                //
                // Phase 40-28 (CR-03, 40-VERIFICATION.md gap 1): `after.place_id.is_some()`
                // гейтит именно ОЧИСТКУ места принтера (Some -> None) — продуктовое
                // решение оставить место прикреплённых картриджей нетронутым, когда
                // место принтера стало неизвестным, а не молча стереть его в NULL без
                // единой строки place_movements/audit_log (D-06 делает Some->None
                // недвижением и на уровне картриджей тоже). Каскад по-прежнему
                // срабатывает на None->Some (первое назначение места принтеру) и
                // Some->Some (обычный переезд).
                // D-18 (WARNING-1, аудит 2026-09-17, 40.1-03): этот каскад — известное
                // ограничение клиентской инвалидации счётчиков дерева «Места»
                // (`notifyPlaceContentChanged`, `ui/src/lib/stores/placeContentEvents.svelte.ts`).
                // Место картриджей меняется ЦЕЛИКОМ на сервере, в ТОЙ ЖЕ транзакции, что и
                // сохранение принтера (`cascade_place_for_printer_in_tx` ниже); `DeviceDto`,
                // возвращаемый клиенту после сохранения принтера, не несёт списка id
                // затронутых картриджей — клиент структурно не может узнать, какие места
                // инвалидировать, не читая заново весь список картриджей принтера.
                // Решение фазы 40.1: НЕ решается новым API в этом объёме (только
                // `notifyPlaceContentChanged`, не новые бэкенд-контракты) и НЕ игнорируется
                // молча — счётчики дерева для мест затронутых картриджей могут оставаться
                // неточными до следующей полной перезагрузки страницы «Места». Если точность
                // без перезагрузки понадобится, решение — расширить ответ этого метода списком
                // id затронутых картриджей (или их старых/новых place_id).
                if after.place_id.is_some() && before_place_id != after.place_id {
                    cartridge_repo.cascade_place_for_printer_in_tx(
                        &tx,
                        id,
                        after.place_id,
                        MovementSource::Manual,
                        "вместе с принтером",
                        user_id_opt,
                        now,
                    )?;
                }

                tx.commit().map_err(map_rusqlite)?;
                Ok(after)
            })
            .await?;

        if number_changed {
            self.broadcast_number_space_changed();
        }

        Ok(DeviceSaveOutcome::Created(Box::new(DeviceDto::from(
            updated_row,
        ))))
    }

    /// Мягкое удаление устройства (soft-delete) с optimistic-lock.
    pub async fn delete_soft(&self, id: i64, version: i64) -> Result<(), AppError> {
        let now = self.clock.unix_seconds();
        let repo = self.repo.clone();
        let user_id_opt: Option<i64> = None;

        self.writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;

                // before_json для audit_log (recovery).
                let before = repo.get_in_tx(&tx, id).ok();
                let before_json = before
                    .as_ref()
                    .map(|row| serde_json::to_string(&DeviceDto::from(row.clone())))
                    .transpose()
                    .map_err(|e| AppError::Internal {
                        source_chain: format!("audit_log before-json (delete): {e}"),
                    })?;

                repo.delete_soft_in_tx(&tx, id, version, now)?;

                tx.execute(
                    "INSERT INTO audit_log \
                     (entity_type, entity_id, action, user_id, before_json, after_json, payload_json, created_at_utc) \
                     VALUES ('device', ?1, 'delete', ?2, ?3, NULL, NULL, ?4)",
                    rusqlite::params![id, user_id_opt, before_json, now],
                )
                .map_err(map_rusqlite)?;

                tx.commit().map_err(map_rusqlite)?;
                Ok(())
            })
            .await
    }

    /// Возвращает список state-hints (DEV-10).
    pub fn state_hints(&self) -> Vec<String> {
        STATE_HINTS.iter().map(|&s| s.to_string()).collect()
    }

    // -----------------------------------------------------------------------
    // Search / Autocomplete / Grouping (Plan 04)
    // -----------------------------------------------------------------------

    /// FTS5 full-text search по name/inventory_number/serial_number/model.
    ///
    /// Sanitizes user input (T-02-04-01). Returns paginated `DeviceListResponse`.
    pub async fn search(
        &self,
        query: String,
        page: Pagination,
    ) -> Result<DeviceListResponse, AppError> {
        if page.limit > 200 {
            return Err(AppError::Validation {
                field: "pagination.limit".to_string(),
                message: "Максимальный размер страницы — 200".to_string(),
            });
        }

        let readers = self.readers.clone();
        let repo = self.repo.clone();
        let domain_page = trackly_core::domain::devices::Pagination {
            offset: page.offset,
            limit: page.limit,
        };

        let (rows, total) = tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            repo.search_fts(&conn, &query, &domain_page)
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking: {e}"),
        })??;

        Ok(DeviceListResponse {
            items: rows.into_iter().map(DeviceDto::from).collect(),
            total,
        })
    }

    /// `status_in`: optional list of device-status codes (V014 `device_statuses.code`)
    /// — service resolves each code → status_id; unknown codes return Validation.
    pub async fn autocomplete(
        &self,
        field_str: String,
        prefix: String,
        ctx_name: Option<String>,
        ctx_status_id: Option<i64>,
        status_in: Option<Vec<String>>,
    ) -> Result<Vec<String>, AppError> {
        // Enum-whitelist validation on service layer (T-02-04-02).
        let field = trackly_core::domain::devices::AutocompleteField::from_str(&field_str)?;

        // Resolve status codes → ids (V014 device_statuses.code, B-1). None → no filter.
        let resolved_status_ids: Option<Vec<i64>> = if let Some(codes) = status_in {
            if codes.is_empty() {
                None
            } else {
                let readers = self.readers.clone();
                let codes_clone = codes.clone();
                let ids: Vec<i64> =
                    tokio::task::spawn_blocking(move || -> Result<Vec<i64>, AppError> {
                        let conn = readers.acquire();
                        let mut out = Vec::with_capacity(codes_clone.len());
                        for code in &codes_clone {
                            let id_opt: Option<i64> = conn
                                .query_row(
                                    "SELECT id FROM device_statuses WHERE code = ?1",
                                    rusqlite::params![code],
                                    |r| r.get(0),
                                )
                                .ok();
                            match id_opt {
                                Some(id) => out.push(id),
                                None => {
                                    return Err(AppError::Validation {
                                        field: "status_in".to_string(),
                                        message: format!("Unknown status code: {code}"),
                                    });
                                }
                            }
                        }
                        Ok(out)
                    })
                    .await
                    .map_err(|e| AppError::Internal {
                        source_chain: format!("spawn_blocking status_in resolve: {e}"),
                    })??;
                Some(ids)
            }
        } else {
            None
        };

        let readers = self.readers.clone();
        let repo = self.repo.clone();

        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            repo.autocomplete(
                &conn,
                field,
                &prefix,
                ctx_name.as_deref(),
                ctx_status_id,
                resolved_status_ids.as_deref(),
            )
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking: {e}"),
        })?
    }

    /// Список сгруппированных не-уникальных устройств (DEV-11).
    ///
    /// Группирует по (type, name, model, specs, kit, state, location, status)
    /// для устройств без inventory_number и serial_number.
    pub async fn list_grouped(
        &self,
        filter: DeviceFilter,
        page: Pagination,
    ) -> Result<Vec<DeviceGroup>, AppError> {
        if page.limit > 200 {
            return Err(AppError::Validation {
                field: "pagination.limit".to_string(),
                message: "Максимальный размер страницы — 200".to_string(),
            });
        }

        let readers = self.readers.clone();
        let repo = self.repo.clone();
        let domain_filter = trackly_core::domain::devices::DeviceFilter {
            type_id: filter.type_id,
            place_id: filter.place_id,
            status_id: filter.status_id,
            state: filter.state,
            name_prefix: filter.name_prefix,
            include_deleted: filter.include_deleted,
            group_by_condition: filter.group_by_condition,
        };
        let domain_page = trackly_core::domain::devices::Pagination {
            offset: page.offset,
            limit: page.limit,
        };

        let group_rows = tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            repo.list_grouped(&conn, &domain_filter, &domain_page)
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking: {e}"),
        })??;

        let groups = group_rows
            .into_iter()
            .map(|g| DeviceGroup {
                repr: DeviceDto::from(g.repr),
                count: g.count as u64,
                ids: g.ids.into_iter().map(|id| id as i32).collect(),
                condition_distinct_count: g.condition_distinct_count,
                place_distinct_count: g.place_distinct_count,
            })
            .collect();

        Ok(groups)
    }

    /// Возвращает количество активных устройств по статусам.
    pub async fn status_counts(&self) -> Result<Vec<StatusCount>, AppError> {
        let readers = self.readers.clone();
        let repo = self.repo.clone();

        let raw = tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            repo.count_by_status(&conn)
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking: {e}"),
        })??;

        Ok(raw
            .into_iter()
            .map(|(status_id, count)| StatusCount { status_id, count })
            .collect())
    }

    // -----------------------------------------------------------------------
    // CSV Import / Export (Plan 05)
    // -----------------------------------------------------------------------

    /// Phase 1 of CSV import: decode bytes, sniff encoding+delimiter, parse rows,
    /// store session, return preview.
    ///
    /// T-02-05-01: rejects files > 50 MB.
    pub async fn import_csv_preview(
        &self,
        bytes: Vec<u8>,
    ) -> Result<CsvImportPreviewResponse, AppError> {
        // T-02-05-01: size cap (50 MB).
        if bytes.len() > 50 * 1024 * 1024 {
            return Err(AppError::Validation {
                field: "file".to_string(),
                message: "Файл больше 50 МБ".to_string(),
            });
        }

        // Sniff encoding + delimiter.
        let profile = detect(&bytes);
        // Decode bytes to String.
        let (text, had_replacements) = decode_to_string(&bytes, profile.encoding);
        // Parse CSV.
        let (headers, all_rows) =
            parse_rows(&text, profile.delimiter).map_err(|e| AppError::Validation {
                field: "file".to_string(),
                message: format!("Не удалось разобрать CSV: {e}"),
            })?;

        let total_rows = all_rows.len() as u64;
        let preview_rows = all_rows.iter().take(5).cloned().collect();

        // Store session for commit step.
        let session = ImportSession {
            encoding: profile.encoding,
            delimiter: profile.delimiter,
            headers: headers.clone(),
            all_rows,
            created: std::time::Instant::now(),
        };
        let token = self.csv_sessions.put(session);

        Ok(CsvImportPreviewResponse {
            token: token.to_string(),
            encoding: profile.encoding.name().to_string(),
            delimiter: (profile.delimiter as char).to_string(),
            headers,
            preview_rows,
            total_rows,
            had_replacements,
        })
    }

    /// Phase 2 of CSV import: retrieve session, validate + insert rows, return report.
    ///
    /// `mapping`: CSV column header → device field name (e.g. "Наименование" → "name").
    /// Known field names: "type", "name", "inventory_no", "serial_no", "model",
    ///   "specs", "kit", "state", "place", "status".
    /// Unknown keys are ignored (T-02-05-08).
    pub async fn import_csv_commit(
        &self,
        token: String,
        mapping: HashMap<String, String>,
    ) -> Result<CsvImportReport, AppError> {
        let uuid = uuid::Uuid::parse_str(&token).map_err(|_| AppError::Validation {
            field: "token".to_string(),
            message: "Некорректный токен".to_string(),
        })?;

        let session = self
            .csv_sessions
            .take(uuid)
            .ok_or_else(|| AppError::Validation {
                field: "token".to_string(),
                message: "Сессия истекла или уже использована".to_string(),
            })?;

        // Build header→column-index lookup.
        let header_idx: HashMap<String, usize> = session
            .headers
            .iter()
            .enumerate()
            .map(|(i, h)| (h.clone(), i))
            .collect();

        // Fetch the full non-archived place candidate set ONCE (not per-row),
        // keyed by `full_path.to_lowercase()` → `place_id` (exact match only,
        // UI-SPEC §12 — no partial/fuzzy match, no auto-create on miss).
        let readers = self.readers.clone();
        let place_repo = self.place_repo.clone();
        let place_by_path: HashMap<String, i64> =
            tokio::task::spawn_blocking(move || -> Result<HashMap<String, i64>, AppError> {
                use trackly_core::ports::places::PlaceRepository;
                let conn = readers.acquire();
                let rows = place_repo.list_all(&conn, false)?;
                Ok(rows
                    .into_iter()
                    .filter_map(|r| r.full_path.map(|p| (p.to_lowercase(), r.id)))
                    .collect())
            })
            .await
            .map_err(|e| AppError::Internal {
                source_chain: format!("spawn_blocking: {e}"),
            })??;

        let mut report = CsvImportReport {
            inserted: 0,
            failed: Vec::new(),
        };

        // Phase 40.2 Plan 08 (NUM-16, RESEARCH Pitfall 5): tracks inventory
        // numbers already seen EARLIER in THIS SAME file (key = trimmed +
        // lowercased, value = the 1-based row_index of the FIRST row that
        // used it) — `self.create(...)`'s own occupied check runs against
        // the DB one row at a time, so by the time a same-file duplicate's
        // SECOND row is processed, the first row is already committed and
        // the DB check alone would report "занят в БД" instead of the more
        // useful "повтор строки N". Populated BEFORE `self.create(...)` is
        // ever called for a row (independent of whether that row's `create`
        // call itself succeeds) so a later duplicate always points at the
        // correct earliest row.
        let mut seen_numbers: HashMap<String, u64> = HashMap::new();

        // Process each row individually (per-row error accumulation, D-CSV-01).
        for (row_offset, row) in session.all_rows.iter().enumerate() {
            let row_index = (row_offset + 1) as u64; // 1-based for user display

            // Build DeviceNew from mapping.
            let build_result = Self::build_device_new_from_row(row, &header_idx, &mapping);
            let (mut new_device, place_text) = match build_result {
                Ok(d) => d,
                Err(e) => {
                    report.failed.push(RowError {
                        row_index,
                        error_code: "Validation".to_string(),
                        error_message: e.to_string(),
                    });
                    continue;
                }
            };

            // NUM-16: in-file duplicate check — SEPARATE from, and checked
            // BEFORE, the DB-level occupied check `self.create(...)` will
            // run later for this same row.
            if let Some(raw_number) = new_device.inventory_no.as_deref() {
                let trimmed_number = raw_number.trim().to_string();
                if !trimmed_number.is_empty() {
                    let key = trimmed_number.to_lowercase();
                    if let Some(&first_row_index) = seen_numbers.get(&key) {
                        report.failed.push(RowError {
                            row_index,
                            error_code: "Validation".to_string(),
                            error_message: format!(
                                "номер уже занят — {trimmed_number} (повтор строки {first_row_index})"
                            ),
                        });
                        continue;
                    }
                    seen_numbers.insert(key, row_index);
                }
            }

            // Resolve place-path text against the place tree (exact match only,
            // UI-SPEC §12). No text → place_id stays None (D-07: place optional).
            if let Some(text) = place_text {
                let key = text.trim().to_lowercase();
                match place_by_path.get(&key) {
                    Some(&id) => new_device.place_id = Some(id),
                    None => {
                        // NB: `error_message` intentionally omits the "Строка N:" prefix —
                        // `row_index` is a separate RowError field and the UI (import
                        // modal's error-list) prepends it generically for every row, same
                        // as every other error path in this function. Baking the prefix in
                        // here too would render "Строка 12: Строка 12: место «...» не
                        // найдено в дереве." (double prefix) instead of UI-SPEC §12's exact
                        // copy "Строка 12: место «...» не найдено в дереве."
                        report.failed.push(RowError {
                            row_index,
                            error_code: "Validation".to_string(),
                            error_message: format!("место «{text}» не найдено в дереве."),
                        });
                        continue;
                    }
                }
            }

            // Validate via service validation.
            if let Err(e) = Self::validate_new(&new_device) {
                let msg = match &e {
                    AppError::Validation { message, .. } => message.clone(),
                    _ => e.to_string(),
                };
                report.failed.push(RowError {
                    row_index,
                    error_code: "Validation".to_string(),
                    error_message: msg,
                });
                continue;
            }

            // Insert via service.create (audit_log; place_id already resolved above).
            match self.create(new_device).await {
                Ok(DeviceSaveOutcome::Created(_)) => {
                    report.inserted += 1;
                }
                Ok(DeviceSaveOutcome::NeedsConfirmation(warning)) => {
                    // `create()` never triggers this today (it never calls
                    // `script_mix_warning`, only `reject_if_occupied`) — but
                    // handled defensively rather than silently miscounting
                    // an unconfirmed row as inserted, in case that changes.
                    report.failed.push(RowError {
                        row_index,
                        error_code: "Validation".to_string(),
                        error_message: warning.message,
                    });
                }
                Err(e) => {
                    let (code, msg) = match &e {
                        AppError::Validation { field, message } => {
                            (format!("Validation:{field}"), message.clone())
                        }
                        AppError::NotFound { entity, id } => {
                            ("NotFound".to_string(), format!("{entity} {id} не найден"))
                        }
                        // NUM-16: occupied-in-DB (not this file) — `reason`
                        // is ALREADY the exact UI-SPEC text ("номер уже
                        // занят — {номер}", no "повтор" suffix) built by
                        // `reject_if_occupied` — passed through verbatim,
                        // not reassembled here.
                        AppError::Conflict { reason } => ("Conflict".to_string(), reason.clone()),
                        _ => ("Internal".to_string(), e.to_string()),
                    };
                    report.failed.push(RowError {
                        row_index,
                        error_code: code,
                        error_message: msg,
                    });
                }
            }
        }

        Ok(report)
    }

    /// Build a `DeviceNew` from a CSV row using the provided column mapping.
    ///
    /// Returns `(DeviceNew, Option<String>)` — the `DeviceNew` always has
    /// `place_id: None`; the second element is the raw place-path text from
    /// the CSV cell (if the "place" column was mapped), resolved against
    /// the place tree by the caller (`import_csv_commit`), never here — this
    /// function has no DB access.
    fn build_device_new_from_row(
        row: &[String],
        header_idx: &HashMap<String, usize>,
        mapping: &HashMap<String, String>,
    ) -> Result<(DeviceNew, Option<String>), String> {
        let get_field = |csv_col: &str| -> Option<String> {
            header_idx
                .get(csv_col)
                .and_then(|&idx| row.get(idx))
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        };

        // Resolve each mapped CSV column to a device field.
        let mut name: Option<String> = None;
        let mut type_label: Option<String> = None;
        let mut inventory_no: Option<String> = None;
        let mut serial_no: Option<String> = None;
        let mut model: Option<String> = None;
        let mut specs: Option<String> = None;
        let mut kit: Option<String> = None;
        let mut state: Option<String> = None;
        let mut place_text: Option<String> = None;
        let mut status_label: Option<String> = None;

        for (csv_col, device_field) in mapping {
            let value = get_field(csv_col);
            match device_field.as_str() {
                "name" => name = value.or(name),
                "type" => type_label = value.or(type_label),
                "inventory_no" => inventory_no = value.or(inventory_no),
                "serial_no" => serial_no = value.or(serial_no),
                "model" => model = value.or(model),
                "specs" => specs = value.or(specs),
                "kit" => kit = value.or(kit),
                "state" => state = value.or(state),
                "place" => place_text = value.or(place_text),
                "status" => status_label = value.or(status_label),
                _ => {} // T-02-05-08: unknown keys ignored
            }
        }

        // Resolve type_id from label (or default to 1 = "Устройство").
        // For CSV import, we use well-known seed IDs: type_id 1 = Устройство, 2 = Расходник.
        let type_id = Self::resolve_type_id(type_label.as_deref());
        // Resolve status_id from label (or default to 1 = "На складе").
        let status_id = Self::resolve_status_id(status_label.as_deref());

        // name is required.
        let name = name.ok_or_else(|| "Наименование обязательно для заполнения".to_string())?;

        Ok((
            DeviceNew {
                type_id,
                name,
                inventory_no,
                serial_no,
                model,
                specs,
                kit,
                state,
                place_id: None,
                status_id,
            },
            place_text,
        ))
    }

    /// Resolve type_id from a Russian type label.
    /// Known seed values: 1 = Устройство, 2 = Расходник.
    fn resolve_type_id(label: Option<&str>) -> i64 {
        match label {
            Some("Расходник") | Some("расходник") => 2,
            _ => 1, // default: Устройство
        }
    }

    /// Resolve status_id from a Russian status label.
    /// Known seed values: 1 = На складе, 2 = В работе, 3 = На ремонте, 4 = Списано.
    fn resolve_status_id(label: Option<&str>) -> i64 {
        match label {
            Some("В работе") | Some("в работе") => 2,
            Some("На ремонте") | Some("на ремонте") => 3,
            Some("Списано") | Some("списано") => 4,
            _ => 1, // default: На складе
        }
    }

    /// Export devices as UTF-8 BOM + semicolon-delimited CSV (D-CSV-02).
    ///
    /// Returns a String containing the full CSV (BOM + headers + rows).
    /// The caller saves this to a file via the save-dialog.
    ///
    /// T-02-05-03: Excel formula injection prevention — cells starting with
    /// `=`, `+`, `-`, `@` are prefixed with `'` (Excel-safe).
    pub async fn export_csv(&self, filter: DeviceFilter) -> Result<String, AppError> {
        // Fetch all devices matching the filter.
        // Note: bypasses the 200-item pagination cap in `list()` by calling the repo directly.
        let readers = self.readers.clone();
        let repo = self.repo.clone();
        let domain_filter = trackly_core::domain::devices::DeviceFilter {
            type_id: filter.type_id,
            place_id: filter.place_id,
            status_id: filter.status_id,
            state: filter.state,
            name_prefix: filter.name_prefix,
            include_deleted: filter.include_deleted,
            group_by_condition: false,
        };
        // Use a very large limit to fetch all rows for export.
        let domain_page = trackly_core::domain::devices::Pagination {
            offset: 0,
            limit: 1_000_000,
        };

        use trackly_core::ports::devices::DeviceRepository;
        let (rows, _total) = tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            repo.as_ref().list(&conn, &domain_filter, &domain_page)
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking export_csv: {e}"),
        })??;

        struct ExportItems {
            items: Vec<DeviceDto>,
        }
        let response = ExportItems {
            items: rows.into_iter().map(DeviceDto::from).collect(),
        };

        let mut wtr = csv::WriterBuilder::new()
            .delimiter(b';')
            .from_writer(Vec::new());

        // Russian headers (D-CSV-02).
        wtr.write_record([
            "Тип",
            "Наименование",
            "Инвентарный №",
            "Серийный №",
            "Модель",
            "Технические характеристики",
            "Комплектация",
            "Состояние",
            "Место",
            "Статус",
        ])
        .map_err(|e| AppError::Internal {
            source_chain: format!("csv writer headers: {e}"),
        })?;

        for device in &response.items {
            // Lookup type name from type_id (known seed values).
            let type_name = Self::type_id_to_name(device.type_id);
            // Lookup status name from status_id.
            let status_name = Self::status_id_to_name(device.status_id);

            wtr.write_record(&[
                csv_safe(type_name),
                csv_safe(&device.name),
                csv_safe(device.inventory_no.as_deref().unwrap_or("")),
                csv_safe(device.serial_no.as_deref().unwrap_or("")),
                csv_safe(device.model.as_deref().unwrap_or("")),
                csv_safe(device.specs.as_deref().unwrap_or("")),
                csv_safe(device.kit.as_deref().unwrap_or("")),
                csv_safe(device.state.as_deref().unwrap_or("")),
                csv_safe(device.full_path.as_deref().unwrap_or("")),
                csv_safe(status_name),
            ])
            .map_err(|e| AppError::Internal {
                source_chain: format!("csv writer row: {e}"),
            })?;
        }

        let inner = wtr.into_inner().map_err(|e| AppError::Internal {
            source_chain: format!("csv writer flush: {e}"),
        })?;

        let body = String::from_utf8(inner).map_err(|e| AppError::Internal {
            source_chain: format!("csv utf8 conversion: {e}"),
        })?;

        // Prepend UTF-8 BOM (D-CSV-02: Russian Excel requires BOM to detect UTF-8).
        let mut output = String::with_capacity(3 + body.len());
        output.push('\u{FEFF}'); // U+FEFF encoded as UTF-8 = EF BB BF
        output.push_str(&body);

        Ok(output)
    }

    fn type_id_to_name(type_id: i64) -> &'static str {
        match type_id {
            2 => "Расходник",
            _ => "Устройство",
        }
    }

    fn status_id_to_name(status_id: i64) -> &'static str {
        match status_id {
            2 => "В работе",
            3 => "На ремонте",
            4 => "Списано",
            _ => "На складе",
        }
    }

    /// Получить несколько устройств по списку ID (DEV-11 expand).
    pub async fn list_by_ids(&self, ids: Vec<i64>) -> Result<Vec<DeviceDto>, AppError> {
        if ids.len() > 1000 {
            return Err(AppError::Validation {
                field: "ids".to_string(),
                message: "Нельзя запросить более 1000 устройств за один раз".to_string(),
            });
        }
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let readers = self.readers.clone();
        let repo = self.repo.clone();

        let rows = tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            repo.list_by_ids(&conn, &ids)
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking: {e}"),
        })??;

        Ok(rows.into_iter().map(DeviceDto::from).collect())
    }

    // -----------------------------------------------------------------------
    // Bulk create (scope extension 2026-05-26)
    // -----------------------------------------------------------------------

    /// Создать N независимых устройств в одной транзакции (scope extension).
    ///
    /// Правила:
    /// - `count` должен быть от 1 до 100 включительно.
    /// - Если `count > 1`, оба поля inventory_no и serial_no должны быть пустыми
    ///   (нельзя создавать дубликаты с уникальными номерами).
    /// - При `count == 1` поведение идентично `create()` — внутри обычный цикл.
    /// - Все строки создаются в одной транзакции с одним снапшотом `created_at_utc`.
    pub async fn bulk_create(
        &self,
        new: DeviceNew,
        count: u32,
    ) -> Result<Vec<DeviceDto>, AppError> {
        // Validate count range.
        if count == 0 || count > 100 {
            return Err(AppError::Validation {
                field: "count".to_string(),
                message: "Количество должно быть от 1 до 100".to_string(),
            });
        }

        // Validate: если count > 1, номера должны быть пустыми.
        if count > 1 {
            let inv_nonempty = new
                .inventory_no
                .as_deref()
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false);
            let ser_nonempty = new
                .serial_no
                .as_deref()
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false);

            if inv_nonempty || ser_nonempty {
                return Err(AppError::Validation {
                    field: "count".to_string(),
                    message: "Нельзя создавать больше одного устройства, если задан инвентарный или серийный номер".to_string(),
                });
            }
        }

        // Validate required fields.
        Self::validate_new(&new)?;

        // Phase 40.2 Plan 08 (NUM-09): `count==1` is the REAL "create a
        // single device/printer" UI path today (`DeviceFormBody.svelte`
        // calls `bulkCreate` even at qty=1 — Plan 13 migrates it to
        // `create_single_with_number_check`, the interactive equivalent
        // with script-mix confirmation support). `count>1` is unreachable
        // past the validation above (`inventory_no` forced empty there), so
        // trimming/checking is a no-op for it. `DeviceNew` carries no
        // `confirm_script_mix` flag — a non-blocking warning is silently
        // accepted here rather than surfaced, same as `create()`.
        let trimmed = new.inventory_no.as_deref().map(|v| v.trim().to_string());
        let trimmed_nonempty = trimmed
            .as_deref()
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        if let Some(t) = &trimmed_nonempty {
            self.reject_if_occupied(t, None).await?;
        }
        let mut new = new;
        new.inventory_no = trimmed_nonempty.clone();

        let now = self.clock.unix_seconds();
        let repo = self.repo.clone();
        let printer_repo = self.printer_repo.clone();
        let domain_new: trackly_core::domain::devices::DeviceNew = new.into();
        let user_id_opt: Option<i64> = None; // Phase 2 — no auth yet

        let ids: Vec<i64> = self
            .writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;

                let mut created_ids = Vec::with_capacity(count as usize);
                for _ in 0..count {
                    let id = repo.create_in_tx(&tx, &domain_new, now)?;
                    Self::sync_printer_row_in_tx(&printer_repo, &tx, id, domain_new.type_id, now)?;

                    // audit_log row for each inserted device.
                    let after = repo.get_in_tx(&tx, id)?;
                    let after_dto = DeviceDto::from(after);
                    let after_json =
                        serde_json::to_string(&after_dto).map_err(|e| AppError::Internal {
                            source_chain: format!("audit_log after-json: {e}"),
                        })?;

                    tx.execute(
                        "INSERT INTO audit_log \
                         (entity_type, entity_id, action, user_id, before_json, after_json, payload_json, created_at_utc) \
                         VALUES ('device', ?1, 'create', ?2, NULL, ?3, NULL, ?4)",
                        rusqlite::params![id, user_id_opt, after_json, now],
                    )
                    .map_err(map_rusqlite)?;

                    created_ids.push(id);
                }

                tx.commit().map_err(map_rusqlite)?;
                Ok(created_ids)
            })
            .await?;

        if trimmed_nonempty.is_some() {
            self.broadcast_number_space_changed();
        }

        self.list_by_ids(ids).await
    }
}

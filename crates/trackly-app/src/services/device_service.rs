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
use trackly_core::domain::printers::PrinterNew;
use trackly_core::ports::devices::DeviceRepository;
use trackly_core::primitives::clock::Clock;
use trackly_infra::db::{pools::ReaderPool, writer_worker::WriterHandle};
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::repos::{SqliteDeviceRepository, SqlitePlaceRepository, SqlitePrinterRepository};

use std::collections::HashMap;

use crate::csv::session_store::ImportSessionStore;
use crate::csv::{decode_to_string, detect, parse_rows, ImportSession};
use crate::dto::device::{
    CsvImportPreviewResponse, CsvImportReport, DeviceDto, DeviceFilter, DeviceGroup,
    DeviceListResponse, DeviceNew, DevicePatch, Pagination, RowError, StatusCount, STATE_HINTS,
};

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
    #[allow(dead_code)]
    pub(crate) csv_sessions: Arc<ImportSessionStore>,
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
        Self {
            writer,
            readers,
            clock,
            repo: Arc::new(SqliteDeviceRepository),
            printer_repo: Arc::new(SqlitePrinterRepository),
            place_repo: Arc::new(SqlitePlaceRepository),
            csv_sessions: Arc::new(ImportSessionStore::new()),
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

    /// Создать новое устройство.
    ///
    /// Валидирует обязательные поля, затем вставляет устройство и запись audit_log
    /// в одной транзакции (RESEARCH §Pattern 2, T-02-03-03).
    /// `new.place_id` — уже разрешённый caller'ом ID места (PlacePicker); ни один
    /// путь записи устройства больше не создаёт место неявно по строке (D-18).
    pub async fn create(&self, new: DeviceNew) -> Result<DeviceDto, AppError> {
        Self::validate_new(&new)?;

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
    pub async fn update(
        &self,
        id: i64,
        version: i64,
        patch: DevicePatch,
    ) -> Result<DeviceDto, AppError> {
        let now = self.clock.unix_seconds();
        let repo = self.repo.clone();
        let printer_repo = self.printer_repo.clone();
        let domain_patch: trackly_core::domain::devices::DevicePatch = patch.into();
        let user_id_opt: Option<i64> = None;

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

                tx.commit().map_err(map_rusqlite)?;
                Ok(after)
            })
            .await?;

        Ok(DeviceDto::from(updated_row))
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
    ///   "specs", "kit", "state", "location", "status".
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

            // Resolve place-path text against the place tree (exact match only,
            // UI-SPEC §12). No text → place_id stays None (D-07: place optional).
            if let Some(text) = place_text {
                let key = text.trim().to_lowercase();
                match place_by_path.get(&key) {
                    Some(&id) => new_device.place_id = Some(id),
                    None => {
                        report.failed.push(RowError {
                            row_index,
                            error_code: "Validation".to_string(),
                            error_message: format!("Строка {row_index}: место «{text}» не найдено в дереве."),
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
                Ok(_) => {
                    report.inserted += 1;
                }
                Err(e) => {
                    let (code, msg) = match &e {
                        AppError::Validation { field, message } => {
                            (format!("Validation:{field}"), message.clone())
                        }
                        AppError::NotFound { entity, id } => {
                            ("NotFound".to_string(), format!("{entity} {id} не найден"))
                        }
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
    /// the CSV cell (if the "location" column was mapped), resolved against
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
                "location" => place_text = value.or(place_text),
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

        self.list_by_ids(ids).await
    }
}

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
use trackly_core::ports::devices::DeviceRepository;
use trackly_core::primitives::clock::Clock;
use trackly_infra::db::{pools::ReaderPool, writer_worker::WriterHandle};
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::repos::SqliteDeviceRepository;

use crate::csv::session_store::ImportSessionStore;
use crate::dto::device::{
    DeviceDto, DeviceFilter, DeviceGroup, DeviceListResponse, DeviceNew, DevicePatch, Pagination,
    StatusCount, STATE_HINTS,
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

    // -----------------------------------------------------------------------
    // CRUD
    // -----------------------------------------------------------------------

    /// Создать новое устройство.
    ///
    /// Валидирует обязательные поля, затем вставляет устройство и запись audit_log
    /// в одной транзакции (RESEARCH §Pattern 2, T-02-03-03).
    pub async fn create(&self, new: DeviceNew) -> Result<DeviceDto, AppError> {
        Self::validate_new(&new)?;

        let now = self.clock.unix_seconds();
        let repo = self.repo.clone();
        let domain_new: trackly_core::domain::devices::DeviceNew = new.into();
        let user_id_opt: Option<i64> = None; // Phase 2 — no auth yet

        let id = self
            .writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;

                let id = repo.create_in_tx(&tx, &domain_new, now)?;
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
            location_id: filter.location_id,
            status_id: filter.status_id,
            state: filter.state,
            name_prefix: filter.name_prefix,
            include_deleted: filter.include_deleted,
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
    pub async fn update(
        &self,
        id: i64,
        version: i64,
        patch: DevicePatch,
    ) -> Result<DeviceDto, AppError> {
        let now = self.clock.unix_seconds();
        let repo = self.repo.clone();
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

    /// Per-field autocomplete с опциональным контекстным фильтром.
    ///
    /// Validates `field_str` against `AutocompleteField` whitelist (T-02-04-02).
    /// Returns up to 30 DISTINCT values, sorted ASC.
    pub async fn autocomplete(
        &self,
        field_str: String,
        prefix: String,
        ctx_name: Option<String>,
    ) -> Result<Vec<String>, AppError> {
        // Enum-whitelist validation on service layer (T-02-04-02).
        let field = trackly_core::domain::devices::AutocompleteField::from_str(&field_str)?;

        let readers = self.readers.clone();
        let repo = self.repo.clone();

        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            repo.autocomplete(&conn, field, &prefix, ctx_name.as_deref())
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
            location_id: filter.location_id,
            status_id: filter.status_id,
            state: filter.state,
            name_prefix: filter.name_prefix,
            include_deleted: filter.include_deleted,
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
        let domain_new: trackly_core::domain::devices::DeviceNew = new.into();
        let user_id_opt: Option<i64> = None; // Phase 2 — no auth yet

        let ids: Vec<i64> = self
            .writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;

                let mut created_ids = Vec::with_capacity(count as usize);
                for _ in 0..count {
                    let id = repo.create_in_tx(&tx, &domain_new, now)?;

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

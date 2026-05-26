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
    DeviceDto, DeviceFilter, DeviceListResponse, DeviceNew, DevicePatch, Pagination, STATE_HINTS,
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
}

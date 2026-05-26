//! SQLite adapter for `DeviceRepository`.
//!
//! `SqliteDeviceRepository` implements `trackly_core::ports::devices::DeviceRepository`
//! using `rusqlite::Connection` as the `Conn` associated type.
//!
//! Column-name mapping (Path B, PATTERNS.md):
//! - DTO `inventory_no`  ↔  DB `inventory_number`
//! - DTO `serial_no`     ↔  DB `serial_number`
//! - DTO `state`         ↔  DB `condition`
//! - DTO `kit`           ↔  DB `complectation`
//! - DTO `specs`         ↔  DB `notes`
//!
//! Все SQL параметризованы через `rusqlite::params![...]` (T-02-03-01).

use rusqlite::{Connection, OptionalExtension};
use trackly_core::domain::devices::{
    DeviceFilter, DeviceGroupRow, DeviceNew, DevicePatch, DeviceRow, Pagination,
};
use trackly_core::error::AppError;
use trackly_core::ports::devices::DeviceRepository;

use crate::error_conversions::map_rusqlite;

/// SQLite-backed device repository adapter.
#[derive(Debug, Default, Clone)]
pub struct SqliteDeviceRepository;

/// SELECT с полным набором колонок в том порядке, который ожидает `from_row`.
const SELECT_DEVICES: &str = "
    SELECT id, type_id, name, inventory_number, serial_number, model,
           condition, complectation, location_id, status_id, notes,
           version, created_at_utc, updated_at_utc, deleted_at_utc
    FROM devices
";

/// Маппинг строки результата → `DeviceRow`.
/// Порядок колонок должен совпадать с `SELECT_DEVICES`.
fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<DeviceRow> {
    Ok(DeviceRow {
        id: row.get(0)?,
        type_id: row.get(1)?,
        name: row.get(2)?,
        inventory_no: row.get(3)?,   // inventory_number → inventory_no
        serial_no: row.get(4)?,      // serial_number → serial_no
        model: row.get(5)?,
        state: row.get(6)?,          // condition → state
        kit: row.get(7)?,            // complectation → kit
        location_id: row.get(8)?,
        status_id: row.get(9)?,
        specs: row.get(10)?,         // notes → specs
        version: row.get(11)?,
        created_at_utc: row.get(12)?,
        updated_at_utc: row.get(13)?,
        deleted_at_utc: row.get(14)?,
    })
}

/// Нормализует пустые строки в NULL (Pitfall #12: пустая строка ≠ отсутствие).
fn normalize_str(s: Option<&str>) -> Option<&str> {
    s.and_then(|v| if v.trim().is_empty() { None } else { Some(v) })
}

/// Вспомогательные методы для использования внутри rusqlite-транзакций.
/// `DeviceService` использует эти методы внутри `writer.execute` closures.
impl SqliteDeviceRepository {
    /// INSERT в пределах транзакции. Возвращает новый `id`.
    pub fn create_in_tx(
        &self,
        tx: &rusqlite::Transaction<'_>,
        new: &DeviceNew,
        now_utc: i64,
    ) -> Result<i64, AppError> {
        let inventory_number =
            normalize_str(new.inventory_no.as_deref());
        let serial_number = normalize_str(new.serial_no.as_deref());

        tx.execute(
            "INSERT INTO devices \
             (type_id, name, inventory_number, serial_number, model, \
              condition, complectation, location_id, status_id, notes, \
              version, created_at_utc, updated_at_utc) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 1, ?11, ?11)",
            rusqlite::params![
                new.type_id,
                new.name,
                inventory_number,
                serial_number,
                new.model,
                new.state,
                new.kit,
                new.location_id,
                new.status_id,
                new.specs,
                now_utc,
            ],
        )
        .map_err(map_rusqlite)?;

        Ok(tx.last_insert_rowid())
    }

    /// GET в пределах транзакции. Возвращает `DeviceRow` включая soft-deleted.
    pub fn get_in_tx(
        &self,
        tx: &rusqlite::Transaction<'_>,
        id: i64,
    ) -> Result<DeviceRow, AppError> {
        tx.query_row(
            &format!("{SELECT_DEVICES} WHERE id = ?1"),
            rusqlite::params![id],
            from_row,
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                AppError::NotFound { entity: "device", id }
            }
            other => map_rusqlite(other),
        })
    }

    /// UPDATE в пределах транзакции с optimistic-lock.
    pub fn update_in_tx(
        &self,
        tx: &rusqlite::Transaction<'_>,
        id: i64,
        version: i64,
        patch: &DevicePatch,
        now_utc: i64,
    ) -> Result<DeviceRow, AppError> {
        let affected = tx
            .execute(
                "UPDATE devices SET
                   name             = COALESCE(?1, name),
                   type_id          = COALESCE(?2, type_id),
                   inventory_number = COALESCE(?3, inventory_number),
                   serial_number    = COALESCE(?4, serial_number),
                   model            = COALESCE(?5, model),
                   condition        = COALESCE(?6, condition),
                   complectation    = COALESCE(?7, complectation),
                   location_id      = COALESCE(?8, location_id),
                   status_id        = COALESCE(?9, status_id),
                   notes            = COALESCE(?10, notes),
                   version          = version + 1,
                   updated_at_utc   = ?11
                 WHERE id = ?12 AND version = ?13 AND deleted_at_utc IS NULL",
                rusqlite::params![
                    patch.name.as_deref(),
                    patch.type_id,
                    patch.inventory_no.as_deref(),
                    patch.serial_no.as_deref(),
                    patch.model.as_deref(),
                    patch.state.as_deref(),
                    patch.kit.as_deref(),
                    patch.location_id,
                    patch.status_id,
                    patch.specs.as_deref(),
                    now_utc,
                    id,
                    version,
                ],
            )
            .map_err(map_rusqlite)?;

        if affected == 0 {
            let actual: Option<i64> = tx
                .query_row(
                    "SELECT version FROM devices WHERE id = ?1",
                    rusqlite::params![id],
                    |r| r.get(0),
                )
                .optional()
                .map_err(map_rusqlite)?;

            return match actual {
                None => Err(AppError::NotFound { entity: "device", id }),
                Some(actual) => Err(AppError::OptimisticLockMismatch {
                    entity: "device",
                    id,
                    expected: version,
                    actual,
                }),
            };
        }

        self.get_in_tx(tx, id)
    }

    /// Soft-delete в пределах транзакции.
    pub fn delete_soft_in_tx(
        &self,
        tx: &rusqlite::Transaction<'_>,
        id: i64,
        version: i64,
        now_utc: i64,
    ) -> Result<(), AppError> {
        let affected = tx
            .execute(
                "UPDATE devices SET deleted_at_utc = ?1, version = version + 1, \
                 updated_at_utc = ?1 \
                 WHERE id = ?2 AND version = ?3 AND deleted_at_utc IS NULL",
                rusqlite::params![now_utc, id, version],
            )
            .map_err(map_rusqlite)?;

        if affected == 0 {
            let actual: Option<i64> = tx
                .query_row(
                    "SELECT version FROM devices WHERE id = ?1",
                    rusqlite::params![id],
                    |r| r.get(0),
                )
                .optional()
                .map_err(map_rusqlite)?;

            return match actual {
                None => Err(AppError::NotFound { entity: "device", id }),
                Some(actual) => Err(AppError::OptimisticLockMismatch {
                    entity: "device",
                    id,
                    expected: version,
                    actual,
                }),
            };
        }

        Ok(())
    }
}

impl DeviceRepository for SqliteDeviceRepository {
    type Conn = Connection;

    fn create(
        &self,
        conn: &mut Self::Conn,
        new: &DeviceNew,
        now_utc: i64,
    ) -> Result<i64, AppError> {
        let inventory_number = normalize_str(new.inventory_no.as_deref());
        let serial_number = normalize_str(new.serial_no.as_deref());

        conn.execute(
            "INSERT INTO devices \
             (type_id, name, inventory_number, serial_number, model, \
              condition, complectation, location_id, status_id, notes, \
              version, created_at_utc, updated_at_utc) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 1, ?11, ?11)",
            rusqlite::params![
                new.type_id,
                new.name,
                inventory_number,
                serial_number,
                new.model,
                new.state,
                new.kit,
                new.location_id,
                new.status_id,
                new.specs,
                now_utc,
            ],
        )
        .map_err(map_rusqlite)?;

        Ok(conn.last_insert_rowid())
    }

    fn get(&self, conn: &Self::Conn, id: i64) -> Result<DeviceRow, AppError> {
        conn.query_row(
            &format!("{SELECT_DEVICES} WHERE id = ?1 AND deleted_at_utc IS NULL"),
            rusqlite::params![id],
            from_row,
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                AppError::NotFound { entity: "device", id }
            }
            other => map_rusqlite(other),
        })
    }

    fn list(
        &self,
        conn: &Self::Conn,
        filter: &DeviceFilter,
        page: &Pagination,
    ) -> Result<(Vec<DeviceRow>, u64), AppError> {
        let include_deleted = filter.include_deleted;
        let status_id = filter.status_id;
        let type_id = filter.type_id;
        // T-02-03-05: max limit 200.
        let limit = (page.limit.min(200)) as i64;
        let offset = page.offset as i64;

        let total: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM devices WHERE
                   (?1 = 1 OR deleted_at_utc IS NULL) AND
                   (?2 IS NULL OR status_id = ?2) AND
                   (?3 IS NULL OR type_id = ?3)",
                rusqlite::params![include_deleted as i64, status_id, type_id],
                |r| r.get(0),
            )
            .map_err(map_rusqlite)?;

        let mut stmt = conn
            .prepare(&format!(
                "{SELECT_DEVICES} WHERE
                   (?1 = 1 OR deleted_at_utc IS NULL) AND
                   (?2 IS NULL OR status_id = ?2) AND
                   (?3 IS NULL OR type_id = ?3)
                 ORDER BY name
                 LIMIT ?4 OFFSET ?5"
            ))
            .map_err(map_rusqlite)?;

        let rows = stmt
            .query_map(
                rusqlite::params![include_deleted as i64, status_id, type_id, limit, offset],
                from_row,
            )
            .map_err(map_rusqlite)?;

        let mut devices = Vec::new();
        for row in rows {
            devices.push(row.map_err(map_rusqlite)?);
        }

        Ok((devices, total as u64))
    }

    fn update(
        &self,
        conn: &mut Self::Conn,
        id: i64,
        version: i64,
        patch: &DevicePatch,
        now_utc: i64,
    ) -> Result<DeviceRow, AppError> {
        let affected = conn
            .execute(
                "UPDATE devices SET
                   name             = COALESCE(?1, name),
                   type_id          = COALESCE(?2, type_id),
                   inventory_number = COALESCE(?3, inventory_number),
                   serial_number    = COALESCE(?4, serial_number),
                   model            = COALESCE(?5, model),
                   condition        = COALESCE(?6, condition),
                   complectation    = COALESCE(?7, complectation),
                   location_id      = COALESCE(?8, location_id),
                   status_id        = COALESCE(?9, status_id),
                   notes            = COALESCE(?10, notes),
                   version          = version + 1,
                   updated_at_utc   = ?11
                 WHERE id = ?12 AND version = ?13 AND deleted_at_utc IS NULL",
                rusqlite::params![
                    patch.name.as_deref(),
                    patch.type_id,
                    patch.inventory_no.as_deref(),
                    patch.serial_no.as_deref(),
                    patch.model.as_deref(),
                    patch.state.as_deref(),
                    patch.kit.as_deref(),
                    patch.location_id,
                    patch.status_id,
                    patch.specs.as_deref(),
                    now_utc,
                    id,
                    version,
                ],
            )
            .map_err(map_rusqlite)?;

        if affected == 0 {
            let actual: Option<i64> = conn
                .query_row(
                    "SELECT version FROM devices WHERE id = ?1",
                    rusqlite::params![id],
                    |r| r.get(0),
                )
                .optional()
                .map_err(map_rusqlite)?;

            return match actual {
                None => Err(AppError::NotFound { entity: "device", id }),
                Some(actual) => Err(AppError::OptimisticLockMismatch {
                    entity: "device",
                    id,
                    expected: version,
                    actual,
                }),
            };
        }

        self.get(conn, id)
    }

    fn delete_soft(
        &self,
        conn: &mut Self::Conn,
        id: i64,
        version: i64,
        now_utc: i64,
    ) -> Result<(), AppError> {
        let affected = conn
            .execute(
                "UPDATE devices SET deleted_at_utc = ?1, version = version + 1, \
                 updated_at_utc = ?1 \
                 WHERE id = ?2 AND version = ?3 AND deleted_at_utc IS NULL",
                rusqlite::params![now_utc, id, version],
            )
            .map_err(map_rusqlite)?;

        if affected == 0 {
            let actual: Option<i64> = conn
                .query_row(
                    "SELECT version FROM devices WHERE id = ?1",
                    rusqlite::params![id],
                    |r| r.get(0),
                )
                .optional()
                .map_err(map_rusqlite)?;

            return match actual {
                None => Err(AppError::NotFound { entity: "device", id }),
                Some(actual) => Err(AppError::OptimisticLockMismatch {
                    entity: "device",
                    id,
                    expected: version,
                    actual,
                }),
            };
        }
        Ok(())
    }

    fn search_fts(
        &self,
        _conn: &Self::Conn,
        _fts_query: &str,
        _page: &Pagination,
    ) -> Result<Vec<DeviceRow>, AppError> {
        todo!("Plan 04: search/autocomplete/grouping implementation")
    }

    fn autocomplete(
        &self,
        _conn: &Self::Conn,
        _field: &str,
        _prefix: &str,
        _ctx_name: Option<&str>,
    ) -> Result<Vec<String>, AppError> {
        todo!("Plan 04: search/autocomplete/grouping implementation")
    }

    fn list_grouped(
        &self,
        _conn: &Self::Conn,
        _filter: &DeviceFilter,
        _page: &Pagination,
    ) -> Result<Vec<DeviceGroupRow>, AppError> {
        todo!("Plan 04: search/autocomplete/grouping implementation")
    }
}

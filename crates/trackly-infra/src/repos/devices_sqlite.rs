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
    AutocompleteField, DeviceFilter, DeviceGroupRow, DeviceNew, DevicePatch, DeviceRow, Pagination,
};
use trackly_core::error::AppError;
use trackly_core::ports::devices::DeviceRepository;

use crate::error_conversions::map_rusqlite;

/// SQLite-backed device repository adapter.
#[derive(Debug, Default, Clone)]
pub struct SqliteDeviceRepository;

/// SELECT с полным набором колонок в том порядке, который ожидает `from_row`.
/// LEFT JOIN locations добавляет `l.name` как последний столбец (индекс 15).
const SELECT_DEVICES: &str = "
    SELECT d.id, d.type_id, d.name, d.inventory_number, d.serial_number, d.model,
           d.condition, d.complectation, d.location_id, d.status_id, d.notes,
           d.version, d.created_at_utc, d.updated_at_utc, d.deleted_at_utc,
           l.name AS location_name
    FROM devices d
    LEFT JOIN locations l ON d.location_id = l.id
";

/// Маппинг строки результата → `DeviceRow`.
/// Порядок колонок должен совпадать с `SELECT_DEVICES`.
fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<DeviceRow> {
    Ok(DeviceRow {
        id: row.get(0)?,
        type_id: row.get(1)?,
        name: row.get(2)?,
        inventory_no: row.get(3)?, // inventory_number → inventory_no
        serial_no: row.get(4)?,    // serial_number → serial_no
        model: row.get(5)?,
        state: row.get(6)?, // condition → state
        kit: row.get(7)?,   // complectation → kit
        location_id: row.get(8)?,
        status_id: row.get(9)?,
        specs: row.get(10)?, // notes → specs
        version: row.get(11)?,
        created_at_utc: row.get(12)?,
        updated_at_utc: row.get(13)?,
        deleted_at_utc: row.get(14)?,
        location: row.get(15)?, // l.name from LEFT JOIN
    })
}

/// Нормализует пустые строки в NULL (Pitfall #12: пустая строка ≠ отсутствие).
fn normalize_str(s: Option<&str>) -> Option<&str> {
    s.and_then(|v| if v.trim().is_empty() { None } else { Some(v) })
}

/// Sanitize user input for FTS5 MATCH queries (T-02-04-01).
///
/// - Splits on whitespace
/// - Escapes internal `"` as `""`
/// - Strips null bytes
/// - Wraps each token in double-quotes and appends `*` for prefix search
///
/// Example: `"AND OR"` → `"\"AND\"*" "\"OR\"*"` (FTS5 treats quoted tokens as literals)
fn build_fts_query(user_input: &str) -> String {
    user_input
        .split_whitespace()
        .map(|t| t.replace('\0', "").replace('"', "\"\""))
        .filter(|t| !t.is_empty())
        .map(|t| format!("\"{t}\"*"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Вспомогательные методы для использования внутри rusqlite-транзакций.
/// `DeviceService` использует эти методы внутри `writer.execute` closures.
impl SqliteDeviceRepository {
    /// Разрешает строковое название расположения в `location_id`.
    ///
    /// Если строка непустая:
    ///   - Создаёт запись в `locations` если не существует (INSERT OR IGNORE).
    ///   - Возвращает id существующей или только что созданной записи.
    ///
    /// Если строка пустая / None — возвращает None.
    pub fn resolve_location_id_in_tx(
        &self,
        tx: &rusqlite::Transaction<'_>,
        location: Option<&str>,
        now_utc: i64,
    ) -> Result<Option<i64>, AppError> {
        let name = match normalize_str(location) {
            Some(n) => n,
            None => return Ok(None),
        };

        tx.execute(
            "INSERT OR IGNORE INTO locations (name, created_at_utc, updated_at_utc) \
             VALUES (?1, ?2, ?2)",
            rusqlite::params![name, now_utc],
        )
        .map_err(map_rusqlite)?;

        let id: i64 = tx
            .query_row(
                "SELECT id FROM locations WHERE name = ?1",
                rusqlite::params![name],
                |r| r.get(0),
            )
            .map_err(map_rusqlite)?;

        Ok(Some(id))
    }

    /// INSERT в пределах транзакции. Возвращает новый `id`.
    pub fn create_in_tx(
        &self,
        tx: &rusqlite::Transaction<'_>,
        new: &DeviceNew,
        now_utc: i64,
    ) -> Result<i64, AppError> {
        let inventory_number = normalize_str(new.inventory_no.as_deref());
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
            &format!("{SELECT_DEVICES} WHERE d.id = ?1"),
            rusqlite::params![id],
            from_row,
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound {
                entity: "device",
                id,
            },
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
                None => Err(AppError::NotFound {
                    entity: "device",
                    id,
                }),
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

    /// UPDATE device.status_id + device.location_id внутри транзакции.
    ///
    /// Используется в `ActService::create` (handover): после INSERT акта
    /// все позиции переводятся в статус «В работе» с новым `location_id`.
    /// Возвращает свежий `DeviceRow` (после update) для записи в `audit_log.after_json`.
    ///
    /// NB: `version` инкрементируется, `updated_at_utc` обновляется. FK на
    /// `device_statuses(status_id)` гарантирует целостность.
    pub fn update_status_and_location_in_tx(
        &self,
        tx: &rusqlite::Transaction<'_>,
        device_id: i64,
        status_id: i64,
        location_id: Option<i64>,
        now_utc: i64,
    ) -> Result<DeviceRow, AppError> {
        let affected = tx
            .execute(
                "UPDATE devices SET status_id = ?1, location_id = ?2, \
                 version = version + 1, updated_at_utc = ?3 \
                 WHERE id = ?4 AND deleted_at_utc IS NULL",
                rusqlite::params![status_id, location_id, now_utc, device_id],
            )
            .map_err(map_rusqlite)?;
        if affected == 0 {
            return Err(AppError::NotFound {
                entity: "device",
                id: device_id,
            });
        }
        self.get_in_tx(tx, device_id)
    }

    /// UPDATE status_id + location_id + condition внутри транзакции — used by
    /// `do_return` (plan 03). В отличие от `update_status_and_location_in_tx`
    /// (plan 02 handover path), здесь также может меняться `condition` поле.
    ///
    /// `condition: Option<&str>` — если `None`, поле НЕ меняется (COALESCE);
    /// если `Some` — записывается ровно это значение (включая пустую строку
    /// — нормализуй на уровне сервиса при необходимости).
    pub fn update_full_in_tx(
        &self,
        tx: &rusqlite::Transaction<'_>,
        device_id: i64,
        status_id: i64,
        location_id: Option<i64>,
        condition: Option<&str>,
        now_utc: i64,
    ) -> Result<DeviceRow, AppError> {
        let affected = tx
            .execute(
                "UPDATE devices SET \
                   status_id      = ?1, \
                   location_id    = ?2, \
                   condition      = COALESCE(?3, condition), \
                   version        = version + 1, \
                   updated_at_utc = ?4 \
                 WHERE id = ?5 AND deleted_at_utc IS NULL",
                rusqlite::params![status_id, location_id, condition, now_utc, device_id],
            )
            .map_err(map_rusqlite)?;
        if affected == 0 {
            return Err(AppError::NotFound {
                entity: "device",
                id: device_id,
            });
        }
        self.get_in_tx(tx, device_id)
    }

    /// Восстанавливает device-row из snapshot, ранее записанного в
    /// `audit_log.before_json` (D-Undo-01).
    ///
    /// snapshot — это `serde_json::Value`, который содержит как минимум
    /// поля `status_id`, `location_id`, `state` (= condition), `kit`
    /// (= complectation), `version`. Дополнительные поля (`name`, `model`,
    /// `inventory_no`, `serial_no`, `specs`, `type_id`) применяются, если
    /// присутствуют, иначе COALESCE сохраняет текущее значение в БД.
    ///
    /// Semantics:
    ///   - `version` инкрементируется на 1 (новая ревизия), чтобы не
    ///     ломать optimistic-lock в дальнейшем.
    ///   - `updated_at_utc` ставится в `now_utc` (это новая мутация, не
    ///     перепись истории).
    ///
    /// Если row отсутствует — `AppError::NotFound`.
    pub fn restore_from_snapshot_in_tx(
        &self,
        tx: &rusqlite::Transaction<'_>,
        device_id: i64,
        snapshot: &serde_json::Value,
        now_utc: i64,
    ) -> Result<DeviceRow, AppError> {
        // Extract scalars. Поля `status_id` обязательны; остальные опциональны.
        let status_id = snapshot
            .get("status_id")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| AppError::Internal {
                source_chain: format!("undo: snapshot for device {device_id} lacks status_id"),
            })?;
        let location_id: Option<i64> = snapshot.get("location_id").and_then(|v| v.as_i64());
        let state: Option<&str> = snapshot.get("state").and_then(|v| v.as_str());
        let kit: Option<&str> = snapshot.get("kit").and_then(|v| v.as_str());
        // Optional «full» fields (когда snapshot писался полностью).
        let name: Option<&str> = snapshot.get("name").and_then(|v| v.as_str());
        let model: Option<&str> = snapshot.get("model").and_then(|v| v.as_str());
        let inventory_no: Option<&str> = snapshot.get("inventory_no").and_then(|v| v.as_str());
        let serial_no: Option<&str> = snapshot.get("serial_no").and_then(|v| v.as_str());
        let specs: Option<&str> = snapshot.get("specs").and_then(|v| v.as_str());
        let type_id: Option<i64> = snapshot.get("type_id").and_then(|v| v.as_i64());

        let affected = tx
            .execute(
                "UPDATE devices SET \
                   type_id          = COALESCE(?1, type_id), \
                   name             = COALESCE(?2, name), \
                   inventory_number = COALESCE(?3, inventory_number), \
                   serial_number    = COALESCE(?4, serial_number), \
                   model            = COALESCE(?5, model), \
                   condition        = COALESCE(?6, condition), \
                   complectation    = COALESCE(?7, complectation), \
                   notes            = COALESCE(?8, notes), \
                   status_id        = ?9, \
                   location_id      = ?10, \
                   version          = version + 1, \
                   updated_at_utc   = ?11 \
                 WHERE id = ?12 AND deleted_at_utc IS NULL",
                rusqlite::params![
                    type_id,
                    name,
                    inventory_no,
                    serial_no,
                    model,
                    state,
                    kit,
                    specs,
                    status_id,
                    location_id,
                    now_utc,
                    device_id,
                ],
            )
            .map_err(map_rusqlite)?;
        if affected == 0 {
            return Err(AppError::NotFound {
                entity: "device",
                id: device_id,
            });
        }
        self.get_in_tx(tx, device_id)
    }

    /// Phase 03.1 (G-12 clone-on-handover): клонирует device-row внутри
    /// writer-tx. Возвращает id вновь созданной row.
    ///
    /// Semantics (G-12 + W-5):
    /// - Все scalar-поля копируются из source EXCEPT:
    ///   - `inventory_number := NULL` (G-12 decision (b) — клоны анонимны)
    ///   - `serial_number    := NULL` (W-5 — физический serial уникален)
    /// - `version := 1`, `created_at_utc/updated_at_utc := now_utc`,
    ///   `deleted_at_utc := NULL`.
    /// - `status_id` сохраняется (caller сразу переведёт в 'в_работе' через
    ///   update_status_and_location_in_tx — типичный pattern в ActService::create).
    ///
    /// Errors: AppError::NotFound если source отсутствует / soft-deleted.
    pub fn clone_device_in_tx(
        &self,
        tx: &rusqlite::Transaction<'_>,
        source_id: i64,
        now_utc: i64,
    ) -> Result<i64, AppError> {
        let affected = tx
            .execute(
                "INSERT INTO devices ( \
                   type_id, name, inventory_number, serial_number, model, \
                   condition, complectation, notes, \
                   location_id, status_id, \
                   version, created_at_utc, updated_at_utc, deleted_at_utc \
                 ) \
                 SELECT \
                   d.type_id, d.name, NULL, NULL, d.model, \
                   d.condition, d.complectation, d.notes, \
                   d.location_id, d.status_id, \
                   1, ?1, ?1, NULL \
                 FROM devices d \
                 WHERE d.id = ?2 AND d.deleted_at_utc IS NULL",
                rusqlite::params![now_utc, source_id],
            )
            .map_err(map_rusqlite)?;
        if affected == 0 {
            return Err(AppError::NotFound {
                entity: "device",
                id: source_id,
            });
        }
        Ok(tx.last_insert_rowid())
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
                None => Err(AppError::NotFound {
                    entity: "device",
                    id,
                }),
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
            &format!("{SELECT_DEVICES} WHERE d.id = ?1 AND d.deleted_at_utc IS NULL"),
            rusqlite::params![id],
            from_row,
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound {
                entity: "device",
                id,
            },
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
                "SELECT COUNT(*) FROM devices d WHERE
                   (?1 = 1 OR d.deleted_at_utc IS NULL) AND
                   (?2 IS NULL OR d.status_id = ?2) AND
                   (?3 IS NULL OR d.type_id = ?3)",
                rusqlite::params![include_deleted as i64, status_id, type_id],
                |r| r.get(0),
            )
            .map_err(map_rusqlite)?;

        let mut stmt = conn
            .prepare(&format!(
                "{SELECT_DEVICES} WHERE
                   (?1 = 1 OR d.deleted_at_utc IS NULL) AND
                   (?2 IS NULL OR d.status_id = ?2) AND
                   (?3 IS NULL OR d.type_id = ?3)
                 ORDER BY d.name
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
                None => Err(AppError::NotFound {
                    entity: "device",
                    id,
                }),
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
                None => Err(AppError::NotFound {
                    entity: "device",
                    id,
                }),
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
        conn: &Self::Conn,
        fts_query: &str,
        page: &Pagination,
    ) -> Result<(Vec<DeviceRow>, u64), AppError> {
        // Build sanitized FTS5 MATCH query (T-02-04-01).
        let match_expr = build_fts_query(fts_query);

        // Empty query after sanitization → return empty result set.
        if match_expr.is_empty() {
            return Ok((Vec::new(), 0));
        }

        let limit = page.limit.min(200) as i64;
        let offset = page.offset as i64;

        // Total count for pagination UI.
        let total: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM devices d
                 JOIN devices_fts ON d.id = devices_fts.rowid
                 WHERE devices_fts MATCH ?1
                   AND d.deleted_at_utc IS NULL",
                rusqlite::params![match_expr],
                |r| r.get(0),
            )
            .map_err(map_rusqlite)?;

        let mut stmt = conn
            .prepare(&format!(
                "SELECT d.id, d.type_id, d.name, d.inventory_number, d.serial_number,
                        d.model, d.condition, d.complectation, d.location_id, d.status_id,
                        d.notes, d.version, d.created_at_utc, d.updated_at_utc, d.deleted_at_utc,
                        l.name AS location_name
                 FROM devices d
                 LEFT JOIN locations l ON d.location_id = l.id
                 JOIN devices_fts ON d.id = devices_fts.rowid
                 WHERE devices_fts MATCH ?1
                   AND d.deleted_at_utc IS NULL
                 ORDER BY rank
                 LIMIT {limit} OFFSET {offset}"
            ))
            .map_err(map_rusqlite)?;

        let rows = stmt
            .query_map(rusqlite::params![match_expr], from_row)
            .map_err(map_rusqlite)?;

        let mut devices = Vec::new();
        for row in rows {
            devices.push(row.map_err(map_rusqlite)?);
        }

        Ok((devices, total as u64))
    }

    fn autocomplete(
        &self,
        conn: &Self::Conn,
        field: AutocompleteField,
        prefix: &str,
        ctx_name: Option<&str>,
        ctx_status_id: Option<i64>,
        status_in: Option<&[i64]>,
    ) -> Result<Vec<String>, AppError> {
        let like_pattern = format!("{prefix}%");
        let mut results = Vec::new();

        // Build optional status-IN fragment + params. When `status_in` is set,
        // we inline a parameterised `status_id IN (?, ?, ...)` clause. The
        // parameter indices are appended AFTER the explicit ones used by the
        // base SQL — for `Location` queries we re-use `d.status_id`, for
        // direct device-field queries we use `status_id`.
        let status_in_filter_devices: Option<String> = status_in.and_then(|ids| {
            if ids.is_empty() {
                None
            } else {
                let placeholders: Vec<String> = ids.iter().map(|_| "?".to_string()).collect();
                Some(format!("status_id IN ({})", placeholders.join(",")))
            }
        });
        let status_in_filter_location: Option<String> = status_in.and_then(|ids| {
            if ids.is_empty() {
                None
            } else {
                let placeholders: Vec<String> = ids.iter().map(|_| "?".to_string()).collect();
                Some(format!("d.status_id IN ({})", placeholders.join(",")))
            }
        });

        // Location is special: queries `locations` table via JOIN with context filtering.
        if field.is_location() {
            let mut clauses = vec![
                "l.deleted_at_utc IS NULL".to_string(),
                "l.name LIKE ?1".to_string(),
                "d.deleted_at_utc IS NULL".to_string(),
            ];
            if ctx_name.is_some() {
                clauses.push("d.name = ?2".to_string());
            }
            if ctx_status_id.is_some() {
                let idx = if ctx_name.is_some() { 3 } else { 2 };
                clauses.push(format!("d.status_id = ?{idx}"));
            }
            if let Some(ref f) = status_in_filter_location {
                clauses.push(f.clone());
            }
            let sql = format!(
                "SELECT DISTINCT l.name
                 FROM locations l
                 JOIN devices d ON d.location_id = l.id
                 WHERE {conds}
                 ORDER BY l.name
                 LIMIT 30",
                conds = clauses.join("\n                 AND "),
            );

            use rusqlite::types::ToSql;
            let like_box: Box<dyn ToSql> = Box::new(like_pattern.clone());
            let mut owned_params: Vec<Box<dyn ToSql>> = vec![like_box];
            if let Some(name) = ctx_name {
                owned_params.push(Box::new(name.to_string()));
            }
            if let Some(sid) = ctx_status_id {
                owned_params.push(Box::new(sid));
            }
            if let Some(ids) = status_in {
                for id in ids {
                    owned_params.push(Box::new(*id));
                }
            }

            let mut stmt = conn.prepare(&sql).map_err(map_rusqlite)?;
            let param_refs: Vec<&dyn ToSql> = owned_params.iter().map(|b| b.as_ref()).collect();
            let rows = stmt
                .query_map(param_refs.as_slice(), |r| r.get::<_, String>(0))
                .map_err(map_rusqlite)?;
            for row in rows {
                results.push(row.map_err(map_rusqlite)?);
            }
            return Ok(results);
        }

        // All other fields: query `devices` table directly.
        // Column name comes ONLY from the whitelisted enum — never from user input (T-02-04-02).
        let col = field.sql_column();

        let mut clauses = vec![
            format!("{col} IS NOT NULL"),
            format!("{col} != ''"),
            format!("{col} LIKE ?1"),
        ];
        if ctx_name.is_some() {
            clauses.push("name = ?2".to_string());
        }
        if ctx_status_id.is_some() {
            let idx = if ctx_name.is_some() { 3 } else { 2 };
            clauses.push(format!("status_id = ?{idx}"));
        }
        if let Some(ref f) = status_in_filter_devices {
            clauses.push(f.clone());
        }
        let sql = format!(
            "SELECT DISTINCT {col} FROM devices
             WHERE deleted_at_utc IS NULL
               AND {conds}
             ORDER BY {col}
             LIMIT 30",
            conds = clauses.join("\n               AND "),
        );

        use rusqlite::types::ToSql;
        let mut owned_params: Vec<Box<dyn ToSql>> = vec![Box::new(like_pattern.clone())];
        if let Some(name) = ctx_name {
            owned_params.push(Box::new(name.to_string()));
        }
        if let Some(sid) = ctx_status_id {
            owned_params.push(Box::new(sid));
        }
        if let Some(ids) = status_in {
            for id in ids {
                owned_params.push(Box::new(*id));
            }
        }

        let mut stmt = conn.prepare(&sql).map_err(map_rusqlite)?;
        let param_refs: Vec<&dyn ToSql> = owned_params.iter().map(|b| b.as_ref()).collect();
        let rows = stmt
            .query_map(param_refs.as_slice(), |r| r.get::<_, String>(0))
            .map_err(map_rusqlite)?;
        for row in rows {
            results.push(row.map_err(map_rusqlite)?);
        }

        Ok(results)
    }

    fn list_grouped(
        &self,
        conn: &Self::Conn,
        filter: &DeviceFilter,
        page: &Pagination,
    ) -> Result<Vec<DeviceGroupRow>, AppError> {
        let status_id = filter.status_id;
        let limit = page.limit.min(200) as i64;
        let offset = page.offset as i64;

        // Group devices by (type_id, name) only.
        //
        // Round 8 fix: the previous key included model/notes/complectation/condition/
        // location_id/status_id, which was too strict — two monitors with the same
        // Наименование but different locations or statuses would NOT collapse.
        //
        // The user's mental model is «same Наименование → same group», so we relax
        // the key to just (type_id, name). Other attribute columns become MAX(...)
        // aggregates for the representative row — for groups of truly identical
        // devices those aggregates equal the unique value; for heterogeneous groups
        // (different locations, statuses, etc.) they show one representative value,
        // which is acceptable because the user can expand the group to see all members.
        //
        // Representative row: MIN(id) for deterministic ordering.
        // GROUP_CONCAT(id) parsed to extract all IDs (T-02-04-06).
        // list_grouped uses a manual query (not SELECT_DEVICES) because it aggregates.
        let mut stmt = conn
            .prepare(
                "SELECT
                   MIN(d.id)                       AS repr_id,
                   COUNT(*)                        AS cnt,
                   GROUP_CONCAT(d.id)              AS id_list,
                   d.type_id, d.name,
                   MAX(d.model)                    AS model,
                   MAX(d.notes)                    AS notes,
                   MAX(d.complectation)            AS complectation,
                   MAX(d.condition)                AS condition,
                   MAX(d.location_id)              AS location_id,
                   MAX(d.status_id)                AS status_id,
                   MAX(d.version)                  AS version,
                   MAX(d.created_at_utc)           AS created_at_utc,
                   MAX(d.updated_at_utc)           AS updated_at_utc,
                   l.name                          AS location_name,
                   MAX(d.inventory_number)         AS inv_no,
                   MAX(d.serial_number)            AS serial_no
                 FROM devices d
                 LEFT JOIN locations l ON l.id = (
                   SELECT MAX(d2.location_id)
                   FROM devices d2
                   WHERE d2.type_id = d.type_id
                     AND d2.name = d.name
                     AND d2.deleted_at_utc IS NULL
                     AND (?1 IS NULL OR d2.status_id = ?1)
                 )
                 WHERE d.deleted_at_utc IS NULL
                   AND (?1 IS NULL OR d.status_id = ?1)
                 GROUP BY d.type_id, d.name
                 ORDER BY d.name
                 LIMIT ?2 OFFSET ?3",
            )
            .map_err(map_rusqlite)?;

        let rows = stmt
            .query_map(rusqlite::params![status_id, limit, offset], |row| {
                let repr_id: i64 = row.get(0)?;
                let count: i64 = row.get(1)?;
                let id_list: String = row.get(2)?;
                let type_id: i64 = row.get(3)?;
                let name: String = row.get(4)?;
                let model: Option<String> = row.get(5)?;
                let specs: Option<String> = row.get(6)?; // notes
                let kit: Option<String> = row.get(7)?; // complectation
                let state: Option<String> = row.get(8)?; // condition
                let location_id: Option<i64> = row.get(9)?;
                let status_id: i64 = row.get(10)?;
                let version: i64 = row.get(11)?;
                let created_at_utc: i64 = row.get(12)?;
                let updated_at_utc: i64 = row.get(13)?;
                let location_name: Option<String> = row.get(14)?;
                // MAX aggregates: for count==1 these equal the device's actual values;
                // for count>1 the UI hides inv/serial columns via colspan, so the value
                // is present but not displayed (no regression for multi-device groups).
                let inv_no: Option<String> = row.get(15)?;
                let serial_no: Option<String> = row.get(16)?;

                Ok((
                    repr_id,
                    count,
                    id_list,
                    type_id,
                    name,
                    model,
                    specs,
                    kit,
                    state,
                    location_id,
                    status_id,
                    version,
                    created_at_utc,
                    updated_at_utc,
                    location_name,
                    inv_no,
                    serial_no,
                ))
            })
            .map_err(map_rusqlite)?;

        let mut groups = Vec::new();
        for row_result in rows {
            let (
                repr_id,
                count,
                id_list,
                type_id,
                name,
                model,
                specs,
                kit,
                state,
                location_id,
                status_id,
                version,
                created_at_utc,
                updated_at_utc,
                location_name,
                inv_no,
                serial_no,
            ) = row_result.map_err(map_rusqlite)?;

            // Parse GROUP_CONCAT result (T-02-04-06: parse failure → AppError::Internal).
            let ids: Result<Vec<i64>, _> = id_list
                .split(',')
                .map(|s| s.trim().parse::<i64>())
                .collect();
            let ids = ids.map_err(|_e| AppError::Internal {
                source_chain: format!("GROUP_CONCAT parsing failed for group id_list: {id_list}"),
            })?;

            let repr = DeviceRow {
                id: repr_id,
                type_id,
                name,
                inventory_no: inv_no,
                serial_no,
                model,
                specs,
                kit,
                state,
                location_id,
                location: location_name,
                status_id,
                version,
                created_at_utc,
                updated_at_utc,
                deleted_at_utc: None,
            };

            groups.push(DeviceGroupRow { repr, ids, count });
        }

        Ok(groups)
    }

    fn count_by_status(&self, conn: &Self::Conn) -> Result<Vec<(i64, u64)>, AppError> {
        let mut stmt = conn
            .prepare(
                "SELECT status_id, COUNT(*) AS cnt
                 FROM devices
                 WHERE deleted_at_utc IS NULL
                 GROUP BY status_id",
            )
            .map_err(map_rusqlite)?;

        let rows = stmt
            .query_map([], |row| {
                let status_id: i64 = row.get(0)?;
                let count: i64 = row.get(1)?;
                Ok((status_id, count))
            })
            .map_err(map_rusqlite)?;

        let mut result = Vec::new();
        for row in rows {
            let (status_id, count) = row.map_err(map_rusqlite)?;
            result.push((status_id, count as u64));
        }

        Ok(result)
    }

    fn list_by_ids(&self, conn: &Self::Conn, ids: &[i64]) -> Result<Vec<DeviceRow>, AppError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        if ids.len() > 1000 {
            return Err(AppError::Validation {
                field: "ids".to_string(),
                message: "Нельзя запросить более 1000 устройств за один раз".to_string(),
            });
        }

        // Build parameterized IN clause (safe for bounded ids.len()).
        let placeholders = ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("?{}", i + 1))
            .collect::<Vec<_>>()
            .join(", ");

        let sql = format!(
            "{SELECT_DEVICES} WHERE d.id IN ({placeholders}) AND d.deleted_at_utc IS NULL ORDER BY d.id"
        );

        let mut stmt = conn.prepare(&sql).map_err(map_rusqlite)?;

        // Build params dynamically.
        use rusqlite::types::ToSql;
        let params: Vec<&dyn ToSql> = ids.iter().map(|id| id as &dyn ToSql).collect();

        let rows = stmt
            .query_map(params.as_slice(), from_row)
            .map_err(map_rusqlite)?;

        let mut devices = Vec::new();
        for row in rows {
            devices.push(row.map_err(map_rusqlite)?);
        }

        Ok(devices)
    }
}

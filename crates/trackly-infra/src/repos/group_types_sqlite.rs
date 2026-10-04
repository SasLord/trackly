//! SQLite-адаптер `GroupTypeRepository` (Phase 41).
//!
//! Единственное место SQL по `group_types` и `group_type_properties`.
//! Все запросы параметризованы через `rusqlite::params![...]` (T-41-04-01) —
//! ввод вызывающего никогда не конкатенируется в текст SQL.
//!
//! Смены `code`/`behavior` у репозитория нет вовсе; прямой `UPDATE` этих
//! колонок отклоняет триггер `trg_group_types_immutable` (V045), а
//! `map_rusqlite` отображает его ABORT в `AppError::Conflict`.

use rusqlite::{Connection, OptionalExtension};
use trackly_core::domain::groups::{
    GroupTypeNew, GroupTypePatch, GroupTypeRow, PropertyDataType, PropertyNew, PropertyPatch,
    PropertyRow,
};
use trackly_core::error::AppError;
use trackly_core::ports::group_types::GroupTypeRepository;

use crate::error_conversions::map_rusqlite;

/// Свойство по умолчанию встроенного типа (D-31). Без serde: структура живёт
/// только между сервисом и репозиторием.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DefaultProperty {
    pub name: &'static str,
    pub data_type: PropertyDataType,
    pub show_on_map: bool,
}

/// SQLite-реализация репозитория типов групп (zero-sized, как `SqlitePlaceRepository`).
#[derive(Debug, Default, Clone)]
pub struct SqliteGroupTypeRepository;

/// SELECT типа в порядке колонок, который ждёт `type_from_row`.
const SELECT_TYPES: &str = "
    SELECT id, code, name, behavior, is_builtin, sort_order, quick_action_enabled,
           quick_action_label, version, created_at_utc, updated_at_utc
    FROM group_types
";

fn type_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<GroupTypeRow> {
    let is_builtin: i64 = row.get(4)?;
    let quick_action_enabled: i64 = row.get(6)?;
    Ok(GroupTypeRow {
        id: row.get(0)?,
        code: row.get(1)?,
        name: row.get(2)?,
        behavior: row.get(3)?,
        is_builtin: is_builtin != 0,
        sort_order: row.get(5)?,
        quick_action_enabled: quick_action_enabled != 0,
        quick_action_label: row.get(7)?,
        version: row.get(8)?,
        created_at_utc: row.get(9)?,
        updated_at_utc: row.get(10)?,
    })
}

fn get_type_impl(conn: &Connection, id: i64) -> Result<GroupTypeRow, AppError> {
    conn.query_row(
        &format!("{SELECT_TYPES} WHERE id = ?1 AND deleted_at_utc IS NULL"),
        rusqlite::params![id],
        type_from_row,
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound {
            entity: "group_type",
            id,
        },
        other => map_rusqlite(other),
    })
}

/// Нулевое число затронутых строк CAS -> `NotFound` или `OptimisticLockMismatch`.
fn resolve_type_cas_failure(conn: &Connection, id: i64, expected: i64) -> AppError {
    let actual: Option<i64> = conn
        .query_row(
            "SELECT version FROM group_types WHERE id = ?1 AND deleted_at_utc IS NULL",
            rusqlite::params![id],
            |r| r.get(0),
        )
        .optional()
        .unwrap_or(None);
    match actual {
        None => AppError::NotFound {
            entity: "group_type",
            id,
        },
        Some(actual) => AppError::OptimisticLockMismatch {
            entity: "group_type",
            id,
            expected,
            actual,
        },
    }
}

/// SELECT свойства в порядке колонок, который ждёт `property_from_row`.
const SELECT_PROPERTIES: &str = "
    SELECT id, type_id, name, data_type, sort_order, is_required, show_on_map,
           archived_at_utc, version, created_at_utc, updated_at_utc
    FROM group_type_properties
";

fn property_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<PropertyRow> {
    let is_required: i64 = row.get(5)?;
    let show_on_map: i64 = row.get(6)?;
    Ok(PropertyRow {
        id: row.get(0)?,
        type_id: row.get(1)?,
        name: row.get(2)?,
        data_type: row.get(3)?,
        sort_order: row.get(4)?,
        is_required: is_required != 0,
        show_on_map: show_on_map != 0,
        archived_at_utc: row.get(7)?,
        version: row.get(8)?,
        created_at_utc: row.get(9)?,
        updated_at_utc: row.get(10)?,
    })
}

fn get_property_impl(conn: &Connection, id: i64) -> Result<PropertyRow, AppError> {
    conn.query_row(
        &format!("{SELECT_PROPERTIES} WHERE id = ?1 AND deleted_at_utc IS NULL"),
        rusqlite::params![id],
        property_from_row,
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound {
            entity: "group_type_property",
            id,
        },
        other => map_rusqlite(other),
    })
}

/// Нулевое число затронутых строк CAS свойства -> `NotFound` или `OptimisticLockMismatch`.
fn resolve_property_cas_failure(conn: &Connection, id: i64, expected: i64) -> AppError {
    let actual: Option<i64> = conn
        .query_row(
            "SELECT version FROM group_type_properties WHERE id = ?1 AND deleted_at_utc IS NULL",
            rusqlite::params![id],
            |r| r.get(0),
        )
        .optional()
        .unwrap_or(None);
    match actual {
        None => AppError::NotFound {
            entity: "group_type_property",
            id,
        },
        Some(actual) => AppError::OptimisticLockMismatch {
            entity: "group_type_property",
            id,
            expected,
            actual,
        },
    }
}

impl SqliteGroupTypeRepository {
    /// Идемпотентный засев встроенных типов в переданной транзакции (GRP-01).
    ///
    /// Вставляет только отсутствующие по `code` и возвращает число вставленных.
    /// Конфликт по `code` намеренно ничего не делает (не перезаписывает строку):
    /// переименование типа администратором переживает повторный засев,
    /// `id` не меняется.
    pub fn seed_builtin_types_in_tx(
        &self,
        tx: &rusqlite::Transaction<'_>,
        types: &[GroupTypeNew],
        now_utc: i64,
    ) -> Result<usize, AppError> {
        let mut inserted = 0usize;
        for t in types {
            inserted += tx
                .execute(
                    "INSERT INTO group_types
                       (code, name, behavior, is_builtin, sort_order, quick_action_enabled,
                        quick_action_label, created_at_utc, updated_at_utc, version)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8, 1)
                     ON CONFLICT(code) DO NOTHING",
                    rusqlite::params![
                        t.code,
                        t.name,
                        t.behavior.as_str(),
                        t.is_builtin as i64,
                        t.sort_order,
                        t.quick_action_enabled as i64,
                        t.quick_action_label,
                        now_utc,
                    ],
                )
                .map_err(map_rusqlite)?;
        }
        Ok(inserted)
    }
}

impl SqliteGroupTypeRepository {
    /// Одноразовый засев свойств по умолчанию встроенного типа (D-31).
    ///
    /// Маркер `default_props_seeded` проверяется и выставляется одним `UPDATE`;
    /// свойства вставляются только если затронута ровно одна строка, в той же
    /// транзакции. Возвращает `true`, если засев выполнен сейчас. Скрытое,
    /// удалённое или переименованное администратором свойство после этого не
    /// воскресает и не дублируется: повторный вызов ничего не вставляет.
    pub fn seed_default_properties_in_tx(
        &self,
        tx: &rusqlite::Transaction<'_>,
        type_code: &str,
        defaults: &[DefaultProperty],
        now_utc: i64,
    ) -> Result<bool, AppError> {
        let affected = tx
            .execute(
                "UPDATE group_types SET default_props_seeded = 1, updated_at_utc = ?2
                 WHERE code = ?1 AND is_builtin = 1 AND default_props_seeded = 0",
                rusqlite::params![type_code, now_utc],
            )
            .map_err(map_rusqlite)?;
        if affected != 1 {
            return Ok(false);
        }
        let type_id: i64 = tx
            .query_row(
                "SELECT id FROM group_types WHERE code = ?1",
                rusqlite::params![type_code],
                |r| r.get(0),
            )
            .map_err(map_rusqlite)?;
        for (idx, d) in defaults.iter().enumerate() {
            tx.execute(
                "INSERT INTO group_type_properties
                   (type_id, name, data_type, sort_order, is_required, show_on_map,
                    created_at_utc, updated_at_utc, version)
                 VALUES (?1, ?2, ?3, ?4, 0, ?5, ?6, ?6, 1)",
                rusqlite::params![
                    type_id,
                    d.name,
                    d.data_type.as_str(),
                    idx as i64,
                    d.show_on_map as i64,
                    now_utc,
                ],
            )
            .map_err(map_rusqlite)?;
        }
        Ok(true)
    }
}

impl GroupTypeRepository for SqliteGroupTypeRepository {
    type Conn = Connection;

    fn list_types(&self, conn: &Connection) -> Result<Vec<GroupTypeRow>, AppError> {
        let mut stmt = conn
            .prepare(&format!(
                "{SELECT_TYPES} WHERE deleted_at_utc IS NULL ORDER BY sort_order, id"
            ))
            .map_err(map_rusqlite)?;
        let rows = stmt
            .query_map([], type_from_row)
            .map_err(map_rusqlite)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(map_rusqlite)?;
        Ok(rows)
    }

    fn get_type(&self, conn: &Connection, id: i64) -> Result<GroupTypeRow, AppError> {
        get_type_impl(conn, id)
    }

    fn get_type_by_code(
        &self,
        conn: &Connection,
        code: &str,
    ) -> Result<Option<GroupTypeRow>, AppError> {
        conn.query_row(
            &format!("{SELECT_TYPES} WHERE code = ?1 AND deleted_at_utc IS NULL"),
            rusqlite::params![code],
            type_from_row,
        )
        .optional()
        .map_err(map_rusqlite)
    }

    fn create_type(
        &self,
        conn: &mut Connection,
        new: &GroupTypeNew,
        now_utc: i64,
    ) -> Result<i64, AppError> {
        conn.execute(
            "INSERT INTO group_types
               (code, name, behavior, is_builtin, sort_order, quick_action_enabled,
                quick_action_label, created_at_utc, updated_at_utc, version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8, 1)",
            rusqlite::params![
                new.code,
                new.name,
                new.behavior.as_str(),
                new.is_builtin as i64,
                new.sort_order,
                new.quick_action_enabled as i64,
                new.quick_action_label,
                now_utc,
            ],
        )
        .map_err(map_rusqlite)?;
        Ok(conn.last_insert_rowid())
    }

    fn update_type(
        &self,
        conn: &mut Connection,
        id: i64,
        version: i64,
        patch: &GroupTypePatch,
        now_utc: i64,
    ) -> Result<GroupTypeRow, AppError> {
        // quick_action_label: Option<Option<String>> — CASE-флаг «поле передано»,
        // а не COALESCE (иначе явный сброс в NULL неотличим от «не передано»).
        let label_present = patch.quick_action_label.is_some() as i64;
        let label_value: Option<&str> =
            patch.quick_action_label.as_ref().and_then(|v| v.as_deref());
        let affected = conn
            .execute(
                "UPDATE group_types SET
                   name                 = COALESCE(?1, name),
                   sort_order           = COALESCE(?2, sort_order),
                   quick_action_enabled = COALESCE(?3, quick_action_enabled),
                   quick_action_label   = CASE WHEN ?4 = 1 THEN ?5 ELSE quick_action_label END,
                   version              = version + 1,
                   updated_at_utc       = ?6
                 WHERE id = ?7 AND version = ?8 AND deleted_at_utc IS NULL",
                rusqlite::params![
                    patch.name,
                    patch.sort_order,
                    patch.quick_action_enabled.map(|b| b as i64),
                    label_present,
                    label_value,
                    now_utc,
                    id,
                    version,
                ],
            )
            .map_err(map_rusqlite)?;
        if affected == 0 {
            return Err(resolve_type_cas_failure(conn, id, version));
        }
        get_type_impl(conn, id)
    }

    fn delete_type(&self, conn: &mut Connection, id: i64) -> Result<(), AppError> {
        let affected = conn
            .execute(
                "DELETE FROM group_types WHERE id = ?1",
                rusqlite::params![id],
            )
            .map_err(map_rusqlite)?;
        if affected == 0 {
            return Err(AppError::NotFound {
                entity: "group_type",
                id,
            });
        }
        Ok(())
    }

    fn count_groups_of_type(&self, conn: &Connection, type_id: i64) -> Result<i64, AppError> {
        conn.query_row(
            "SELECT COUNT(*) FROM groups WHERE type_id = ?1 AND deleted_at_utc IS NULL",
            rusqlite::params![type_id],
            |r| r.get(0),
        )
        .map_err(map_rusqlite)
    }

    fn list_properties(
        &self,
        conn: &Connection,
        type_id: i64,
        include_archived: bool,
    ) -> Result<Vec<PropertyRow>, AppError> {
        let mut stmt = conn
            .prepare(&format!(
                "{SELECT_PROPERTIES}
                 WHERE type_id = ?1 AND deleted_at_utc IS NULL
                   AND (?2 = 1 OR archived_at_utc IS NULL)
                 ORDER BY sort_order, id"
            ))
            .map_err(map_rusqlite)?;
        let rows = stmt
            .query_map(
                rusqlite::params![type_id, include_archived as i64],
                property_from_row,
            )
            .map_err(map_rusqlite)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(map_rusqlite)?;
        Ok(rows)
    }

    fn get_property(&self, conn: &Connection, id: i64) -> Result<PropertyRow, AppError> {
        get_property_impl(conn, id)
    }

    fn create_property(
        &self,
        conn: &mut Connection,
        new: &PropertyNew,
        now_utc: i64,
    ) -> Result<i64, AppError> {
        // sort_order = MAX + 1 среди ВСЕХ свойств типа (включая скрытые): скрытое
        // свойство хранит своё место, новое не должно с ним столкнуться.
        conn.execute(
            "INSERT INTO group_type_properties
               (type_id, name, data_type, sort_order, is_required, show_on_map,
                created_at_utc, updated_at_utc, version)
             SELECT ?1, ?2, ?3, COALESCE(MAX(sort_order) + 1, 0), ?4, ?5, ?6, ?6, 1
             FROM group_type_properties WHERE type_id = ?1",
            rusqlite::params![
                new.type_id,
                new.name,
                new.data_type.as_str(),
                new.is_required as i64,
                new.show_on_map as i64,
                now_utc,
            ],
        )
        .map_err(map_rusqlite)?;
        Ok(conn.last_insert_rowid())
    }

    fn update_property(
        &self,
        conn: &mut Connection,
        id: i64,
        version: i64,
        patch: &PropertyPatch,
        now_utc: i64,
    ) -> Result<PropertyRow, AppError> {
        let affected = conn
            .execute(
                "UPDATE group_type_properties SET
                   name           = COALESCE(?1, name),
                   data_type      = COALESCE(?2, data_type),
                   is_required    = COALESCE(?3, is_required),
                   show_on_map    = COALESCE(?4, show_on_map),
                   version        = version + 1,
                   updated_at_utc = ?5
                 WHERE id = ?6 AND version = ?7 AND deleted_at_utc IS NULL",
                rusqlite::params![
                    patch.name,
                    patch.data_type.map(|d| d.as_str()),
                    patch.is_required.map(|b| b as i64),
                    patch.show_on_map.map(|b| b as i64),
                    now_utc,
                    id,
                    version,
                ],
            )
            .map_err(map_rusqlite)?;
        if affected == 0 {
            return Err(resolve_property_cas_failure(conn, id, version));
        }
        get_property_impl(conn, id)
    }

    fn archive_property(
        &self,
        conn: &mut Connection,
        id: i64,
        now_utc: i64,
    ) -> Result<(), AppError> {
        let affected = conn
            .execute(
                "UPDATE group_type_properties SET
                   archived_at_utc = ?1, version = version + 1, updated_at_utc = ?1
                 WHERE id = ?2 AND archived_at_utc IS NULL AND deleted_at_utc IS NULL",
                rusqlite::params![now_utc, id],
            )
            .map_err(map_rusqlite)?;
        if affected == 0 {
            // Уже скрыто — не ошибка; нет строки — NotFound.
            get_property_impl(conn, id)?;
        }
        Ok(())
    }

    fn unarchive_property(
        &self,
        conn: &mut Connection,
        id: i64,
        now_utc: i64,
    ) -> Result<(), AppError> {
        let affected = conn
            .execute(
                "UPDATE group_type_properties SET
                   archived_at_utc = NULL, version = version + 1, updated_at_utc = ?1
                 WHERE id = ?2 AND archived_at_utc IS NOT NULL AND deleted_at_utc IS NULL",
                rusqlite::params![now_utc, id],
            )
            .map_err(map_rusqlite)?;
        if affected == 0 {
            // Уже живое — не ошибка; нет строки — NotFound.
            get_property_impl(conn, id)?;
        }
        Ok(())
    }

    fn delete_property_hard(&self, conn: &mut Connection, id: i64) -> Result<(), AppError> {
        let affected = conn
            .execute(
                "DELETE FROM group_type_properties WHERE id = ?1",
                rusqlite::params![id],
            )
            .map_err(map_rusqlite)?;
        if affected == 0 {
            return Err(AppError::NotFound {
                entity: "group_type_property",
                id,
            });
        }
        Ok(())
    }

    fn filled_group_count(&self, conn: &Connection, property_id: i64) -> Result<i64, AppError> {
        conn.query_row(
            "SELECT COUNT(DISTINCT group_id) FROM group_property_values
             WHERE property_id = ?1
               AND (value_ref IS NOT NULL OR (value_text IS NOT NULL AND value_text <> ''))",
            rusqlite::params![property_id],
            |r| r.get(0),
        )
        .map_err(map_rusqlite)
    }

    fn groups_missing_required(
        &self,
        conn: &Connection,
        type_id: i64,
        property_id: i64,
    ) -> Result<Vec<(i64, String)>, AppError> {
        // Нарушитель = нет строки значения (отсутствие строки = «пусто»), а не пустая строка.
        let mut stmt = conn
            .prepare(
                "SELECT g.id, g.name FROM groups g
                 WHERE g.type_id = ?1 AND g.deleted_at_utc IS NULL
                   AND NOT EXISTS (
                     SELECT 1 FROM group_property_values v
                     WHERE v.group_id = g.id AND v.property_id = ?2
                       AND (v.value_ref IS NOT NULL
                            OR (v.value_text IS NOT NULL AND v.value_text <> ''))
                   )
                 ORDER BY g.seq, g.id",
            )
            .map_err(map_rusqlite)?;
        let rows = stmt
            .query_map(rusqlite::params![type_id, property_id], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
            })
            .map_err(map_rusqlite)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(map_rusqlite)?;
        Ok(rows)
    }

    fn reorder_properties(
        &self,
        conn: &mut Connection,
        type_id: i64,
        ordered_ids: &[i64],
        now_utc: i64,
    ) -> Result<(), AppError> {
        let tx = conn.transaction().map_err(map_rusqlite)?;
        let mut live: Vec<i64> = {
            let mut stmt = tx
                .prepare(
                    "SELECT id FROM group_type_properties
                     WHERE type_id = ?1 AND archived_at_utc IS NULL AND deleted_at_utc IS NULL",
                )
                .map_err(map_rusqlite)?;
            let ids = stmt
                .query_map(rusqlite::params![type_id], |r| r.get::<_, i64>(0))
                .map_err(map_rusqlite)?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(map_rusqlite)?;
            ids
        };
        let mut requested = ordered_ids.to_vec();
        live.sort_unstable();
        requested.sort_unstable();
        // Сверка множеств: чужой id, дубликат или пропуск — отказ до любой записи.
        if live != requested {
            return Err(AppError::Validation {
                field: "ordered_ids".to_string(),
                message: "Список свойств для сортировки не совпадает с текущим набором \
                          свойств типа. Обновите страницу и повторите."
                    .to_string(),
            });
        }
        for (pos, id) in ordered_ids.iter().enumerate() {
            tx.execute(
                "UPDATE group_type_properties SET sort_order = ?1, updated_at_utc = ?2
                 WHERE id = ?3 AND type_id = ?4",
                rusqlite::params![pos as i64, now_utc, id, type_id],
            )
            .map_err(map_rusqlite)?;
        }
        tx.commit().map_err(map_rusqlite)?;
        Ok(())
    }
}

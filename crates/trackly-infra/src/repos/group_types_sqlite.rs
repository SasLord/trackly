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
use trackly_core::domain::groups::{GroupTypeNew, GroupTypePatch, GroupTypeRow};
use trackly_core::error::AppError;
use trackly_core::ports::group_types::GroupTypeRepository;

use crate::error_conversions::map_rusqlite;

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
}

//! SQLite adapter for `PlaceRepository`.
//!
//! `SqlitePlaceRepository` implements `trackly_core::ports::places::PlaceRepository`
//! using `rusqlite::Connection` as the `Conn` associated type.
//!
//! This is the single place in the codebase where every recursive-CTE query for the
//! places tree lives — no other file re-derives tree-traversal SQL (39-RESEARCH.md):
//!   - Pattern 2 (descendant-subtree CTE) powers `subtree_stats`, `list_subtree_contents`,
//!     and `delete_hard`'s pre-flight conflict check.
//!   - Pattern 3 (ancestor-chain CTE) powers `move_node`'s cycle check.
//!   - `list_storage_place_ids` is a third, distinct CTE shape: an ancestor WALK from
//!     every node (not a single-root descendant walk) — D-11.4's "is_storage inherits
//!     from any ancestor" semantics.
//!
//! Все SQL параметризованы через `rusqlite::params![...]` (T-39-04-01) — никакой
//! конкатенации caller-supplied значений в текст запроса.

use rusqlite::{Connection, OptionalExtension};
use trackly_core::domain::places::{PlaceContentRow, PlaceKind, PlaceNew, PlaceRow, SubtreeStats};
use trackly_core::error::AppError;
use trackly_core::ports::places::PlaceRepository;

use crate::error_conversions::map_rusqlite;

/// SQLite-backed place repository adapter (zero-sized, mirrors `SqliteDeviceRepository`).
#[derive(Debug, Default, Clone)]
pub struct SqlitePlaceRepository;

/// SELECT с полным набором колонок в порядке, который ожидает `from_row`.
/// LEFT JOIN place_full_paths добавляет `pfp.full_path` как последний
/// "человекочитаемый" столбец (индекс 9) — мирорит `SELECT_DEVICES`'s
/// `LEFT JOIN locations` shape, заменяя таблицу на всегда-живое view.
const SELECT_PLACES: &str = "
    SELECT p.id, p.parent_id, p.kind, p.name, p.level, p.is_storage, p.sort_order,
           p.archived_at_utc, p.notes, pfp.full_path,
           p.created_at_utc, p.updated_at_utc, p.deleted_at_utc, p.version
    FROM places p
    LEFT JOIN place_full_paths pfp ON pfp.place_id = p.id
";

/// Маппинг строки результата `SELECT_PLACES` → `PlaceRow`.
/// Порядок колонок должен совпадать с `SELECT_PLACES`.
fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<PlaceRow> {
    let kind_sql: String = row.get(2)?;
    let kind = PlaceKind::from_str(&kind_sql).map_err(|_| {
        // CHECK constraint on places.kind guarantees one of the six tokens —
        // if we see another value, the schema has been tampered with
        // (mirrors acts_sqlite.rs's ActType mapping convention).
        rusqlite::Error::FromSqlConversionFailure(
            2,
            rusqlite::types::Type::Text,
            format!("invalid places.kind in DB: {kind_sql}").into(),
        )
    })?;
    let is_storage: i64 = row.get(5)?;
    Ok(PlaceRow {
        id: row.get(0)?,
        parent_id: row.get(1)?,
        kind,
        name: row.get(3)?,
        level: row.get(4)?,
        is_storage: is_storage != 0,
        sort_order: row.get(6)?,
        archived_at_utc: row.get(7)?,
        notes: row.get(8)?,
        full_path: row.get(9)?,
        created_at_utc: row.get(10)?,
        updated_at_utc: row.get(11)?,
        deleted_at_utc: row.get(12)?,
        version: row.get(13)?,
    })
}

/// GET (single row, excluding soft-deleted). Works against either a plain
/// `Connection` or a `Transaction` (deref-coerces to `&Connection`).
fn get_impl(conn: &Connection, id: i64) -> Result<PlaceRow, AppError> {
    conn.query_row(
        &format!("{SELECT_PLACES} WHERE p.id = ?1 AND p.deleted_at_utc IS NULL"),
        rusqlite::params![id],
        from_row,
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound { entity: "place", id },
        other => map_rusqlite(other),
    })
}

/// Resolves a zero-rows-affected optimistic-lock CAS write into either
/// `AppError::NotFound` (row doesn't exist / already soft-deleted) or
/// `AppError::OptimisticLockMismatch` (row exists, `version` differs) —
/// mirrors the established `devices_sqlite.rs`/`acts_sqlite.rs` convention
/// of distinguishing the two cases rather than collapsing both into one
/// generic conflict.
fn resolve_cas_failure(conn: &Connection, id: i64, expected: i64) -> AppError {
    let actual: Option<i64> = conn
        .query_row(
            "SELECT version FROM places WHERE id = ?1 AND deleted_at_utc IS NULL",
            rusqlite::params![id],
            |r| r.get(0),
        )
        .optional()
        .unwrap_or(None);
    match actual {
        None => AppError::NotFound { entity: "place", id },
        Some(actual) => AppError::OptimisticLockMismatch {
            entity: "place",
            id,
            expected,
            actual,
        },
    }
}

// Task 2 (this plan's second commit) adds: `subtree_stats_impl`,
// `list_subtree_contents_impl`, `list_storage_place_ids_impl`, `full_path_impl` —
// the Pattern 2 (descendant-subtree) and D-11.4 (ancestor-walk) CTE queries that
// `delete_hard`/`subtree_stats`/`list_subtree_contents`/`list_storage_place_ids`/
// `full_path` depend on. Stubbed with `unimplemented!()` below as an intra-plan
// checkpoint (acceptance criteria explicitly allow this before Task 2 lands).

impl PlaceRepository for SqlitePlaceRepository {
    type Conn = Connection;

    fn create(&self, conn: &mut Self::Conn, new: &PlaceNew, now_utc: i64) -> Result<i64, AppError> {
        conn.execute(
            "INSERT INTO places \
             (parent_id, kind, name, level, is_storage, sort_order, notes, \
              version, created_at_utc, updated_at_utc) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, ?8, ?8)",
            rusqlite::params![
                new.parent_id,
                new.kind.as_str(),
                new.name,
                new.level,
                new.is_storage as i64,
                new.sort_order,
                new.notes,
                now_utc,
            ],
        )
        .map_err(map_rusqlite)?;

        Ok(conn.last_insert_rowid())
    }

    fn get(&self, conn: &Self::Conn, id: i64) -> Result<PlaceRow, AppError> {
        get_impl(conn, id)
    }

    fn list_children(&self, conn: &Self::Conn, parent_id: Option<i64>) -> Result<Vec<PlaceRow>, AppError> {
        // `p.parent_id IS ?1` handles both `Some(id)` (behaves like `=`) and
        // `None` (matches NULL rows, i.e. root nodes) with a single query —
        // no branching SQL needed. Natural sibling sort (D-05, Pattern 4)
        // is intentionally NOT applied here; the caller (place_service.rs,
        // Plan 05) sorts in Rust via `domain::places::sibling_cmp`.
        let sql = format!("{SELECT_PLACES} WHERE p.parent_id IS ?1 AND p.deleted_at_utc IS NULL");
        let mut stmt = conn.prepare(&sql).map_err(map_rusqlite)?;
        let rows = stmt
            .query_map(rusqlite::params![parent_id], from_row)
            .map_err(map_rusqlite)?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(map_rusqlite)
    }

    fn list_all(&self, conn: &Self::Conn, include_archived: bool) -> Result<Vec<PlaceRow>, AppError> {
        let sql = if include_archived {
            format!("{SELECT_PLACES} WHERE p.deleted_at_utc IS NULL")
        } else {
            format!("{SELECT_PLACES} WHERE p.deleted_at_utc IS NULL AND p.archived_at_utc IS NULL")
        };
        let mut stmt = conn.prepare(&sql).map_err(map_rusqlite)?;
        let rows = stmt.query_map([], from_row).map_err(map_rusqlite)?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(map_rusqlite)
    }

    fn rename(
        &self,
        conn: &mut Self::Conn,
        id: i64,
        name: &str,
        version: i64,
        now_utc: i64,
    ) -> Result<PlaceRow, AppError> {
        let affected = conn
            .execute(
                "UPDATE places SET name = ?1, updated_at_utc = ?2, version = version + 1 \
                 WHERE id = ?3 AND version = ?4 AND deleted_at_utc IS NULL",
                rusqlite::params![name, now_utc, id, version],
            )
            .map_err(map_rusqlite)?;

        if affected == 0 {
            return Err(resolve_cas_failure(conn, id, version));
        }

        get_impl(conn, id)
    }

    fn move_node(
        &self,
        conn: &mut Self::Conn,
        id: i64,
        new_parent_id: Option<i64>,
        version: i64,
        now_utc: i64,
    ) -> Result<PlaceRow, AppError> {
        let tx = conn.transaction().map_err(map_rusqlite)?;

        // Pattern 3 (39-RESEARCH.md): cycle check runs FIRST, inside the same
        // transaction as the UPDATE below. Moving to root (`None`) can never
        // create a cycle, so the check only runs for `Some(new_parent_id)`.
        if let Some(np) = new_parent_id {
            let is_cycle: i64 = tx
                .query_row(
                    "WITH RECURSIVE ancestors(id) AS (
                        SELECT parent_id FROM places WHERE id = ?1
                        UNION ALL
                        SELECT p.parent_id FROM places p
                        JOIN ancestors a ON p.id = a.id
                        WHERE p.parent_id IS NOT NULL
                     )
                     SELECT EXISTS(
                       SELECT 1 WHERE ?1 = ?2
                       UNION ALL
                       SELECT 1 FROM ancestors WHERE id = ?2
                     )",
                    rusqlite::params![np, id],
                    |r| r.get(0),
                )
                .map_err(map_rusqlite)?;

            if is_cycle != 0 {
                return Err(AppError::Validation {
                    field: "parent_id".to_string(),
                    message: "Нельзя переместить место внутрь самого себя или своего вложенного места."
                        .to_string(),
                });
            }
        }

        let affected = tx
            .execute(
                "UPDATE places SET parent_id = ?1, updated_at_utc = ?2, version = version + 1 \
                 WHERE id = ?3 AND version = ?4 AND deleted_at_utc IS NULL",
                rusqlite::params![new_parent_id, now_utc, id, version],
            )
            .map_err(map_rusqlite)?;

        if affected == 0 {
            return Err(resolve_cas_failure(&tx, id, version));
        }

        let row = get_impl(&tx, id)?;
        tx.commit().map_err(map_rusqlite)?;
        Ok(row)
    }

    fn archive(&self, conn: &mut Self::Conn, id: i64, version: i64, now_utc: i64) -> Result<(), AppError> {
        let affected = conn
            .execute(
                "UPDATE places SET archived_at_utc = ?1, updated_at_utc = ?1, version = version + 1 \
                 WHERE id = ?2 AND version = ?3 AND deleted_at_utc IS NULL",
                rusqlite::params![now_utc, id, version],
            )
            .map_err(map_rusqlite)?;

        if affected == 0 {
            return Err(resolve_cas_failure(conn, id, version));
        }
        Ok(())
    }

    fn unarchive(&self, conn: &mut Self::Conn, id: i64, version: i64, now_utc: i64) -> Result<(), AppError> {
        let affected = conn
            .execute(
                "UPDATE places SET archived_at_utc = NULL, updated_at_utc = ?1, version = version + 1 \
                 WHERE id = ?2 AND version = ?3 AND deleted_at_utc IS NULL",
                rusqlite::params![now_utc, id, version],
            )
            .map_err(map_rusqlite)?;

        if affected == 0 {
            return Err(resolve_cas_failure(conn, id, version));
        }
        Ok(())
    }

    fn delete_hard(&self, _conn: &mut Self::Conn, _id: i64, _version: i64) -> Result<(), AppError> {
        // Task 2: Pattern 2 subtree-stats pre-flight conflict check (D-14).
        unimplemented!("delete_hard: Task 2 of this plan")
    }

    fn subtree_stats(&self, _conn: &Self::Conn, _root_id: i64) -> Result<SubtreeStats, AppError> {
        // Task 2: Pattern 2 descendant-subtree CTE (D-25/D-21/PLC-06).
        unimplemented!("subtree_stats: Task 2 of this plan")
    }

    fn list_subtree_contents(
        &self,
        _conn: &Self::Conn,
        _root_id: i64,
        _nested: bool,
    ) -> Result<Vec<PlaceContentRow>, AppError> {
        // Task 2: PLC-06 content-of-place UNION query.
        unimplemented!("list_subtree_contents: Task 2 of this plan")
    }

    fn list_storage_place_ids(&self, _conn: &Self::Conn) -> Result<Vec<i64>, AppError> {
        // Task 2: D-11.4 ancestor-walk CTE.
        unimplemented!("list_storage_place_ids: Task 2 of this plan")
    }

    fn full_path(&self, _conn: &Self::Conn, _id: i64) -> Result<String, AppError> {
        // Task 2: place_full_paths lookup.
        unimplemented!("full_path: Task 2 of this plan")
    }
}

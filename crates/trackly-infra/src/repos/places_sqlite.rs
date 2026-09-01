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
use trackly_core::domain::places::{
    shorten_place_path, PathDisplayVariant, PlaceContentRow, PlaceKind, PlaceNew, PlaceRow,
    SubtreeStats,
};
use trackly_core::error::AppError;
use trackly_core::ports::places::PlaceRepository;

use crate::error_conversions::map_rusqlite;
use crate::repos::place_path_settings::read_path_display_separators;

/// SQLite-backed place repository adapter (zero-sized, mirrors `SqliteDeviceRepository`).
#[derive(Debug, Default, Clone)]
pub struct SqlitePlaceRepository;

/// SELECT с полным набором колонок в порядке, который ожидает `from_row`.
/// LEFT JOIN place_full_paths добавляет `pfp.full_path` как последний
/// "человекочитаемый" столбец (индекс 9) — мирорит `SELECT_DEVICES`'s
/// `LEFT JOIN locations` shape, заменяя таблицу на всегда-живое view.
const SELECT_PLACES: &str = "
    SELECT p.id, p.parent_id, p.kind, p.name, p.level, p.is_storage, p.sort_order,
           p.archived_at_utc, p.notes, pfp.full_path, p.path_variant_override,
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
    let path_variant_override_sql: Option<String> = row.get(10)?;
    // В отличие от `places.kind` выше, этот столбец добавлен ALTER'ом в V039 БЕЗ
    // CHECK constraint — схема его форму не гарантирует, поэтому вероятность
    // мусора здесь выше. Читатель мягкий, писатель строгий: нераспознанный токен
    // деградирует до `None` («как у родителя») + `tracing::warn!`, а валидация
    // остаётся на пути записи (`PathDisplayVariant::from_str` в `place_service`).
    // Иначе одна испорченная строка роняла бы `list_all`/`list_children`/`get`
    // целиком, то есть весь раздел «Места» (IN-01). Единообразно с четырьмя
    // другими потребителями того же токена (`devices_sqlite`, `cartridges_sqlite`,
    // `report_service`, `act_service`), которые уже деградируют мягко.
    let path_variant_override =
        path_variant_override_sql.and_then(|v| match PathDisplayVariant::from_str(&v) {
            Ok(variant) => Some(variant),
            Err(_) => {
                let place_id: i64 = row.get(0).unwrap_or(-1);
                tracing::warn!(
                    place_id,
                    token = %v,
                    "unrecognized places.path_variant_override token — degrading to NULL \
                     (\"as parent\"); the column has no CHECK constraint, so this value was \
                     written outside the app"
                );
                None
            }
        });
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
        path_variant_override,
        created_at_utc: row.get(11)?,
        updated_at_utc: row.get(12)?,
        deleted_at_utc: row.get(13)?,
        version: row.get(14)?,
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
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound {
            entity: "place",
            id,
        },
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
        None => AppError::NotFound {
            entity: "place",
            id,
        },
        Some(actual) => AppError::OptimisticLockMismatch {
            entity: "place",
            id,
            expected,
            actual,
        },
    }
}

/// Pattern 2 (39-RESEARCH.md): subtree counts under `root_id`, inclusive of the
/// root itself. Shared verbatim by `subtree_stats` (D-25 tree counters / D-21
/// consequences preview) and `delete_hard`'s pre-flight conflict check (D-14) —
/// one source of truth, not two.
fn subtree_stats_impl(conn: &Connection, root_id: i64) -> Result<SubtreeStats, AppError> {
    conn.query_row(
        "WITH RECURSIVE subtree(id) AS (
            SELECT id FROM places WHERE id = ?1 AND deleted_at_utc IS NULL
            UNION ALL
            SELECT p.id FROM places p
            JOIN subtree s ON p.parent_id = s.id
            WHERE p.deleted_at_utc IS NULL
         )
         SELECT
           (SELECT COUNT(*) FROM places WHERE parent_id = ?1 AND deleted_at_utc IS NULL) AS direct_children,
           (SELECT COUNT(*) FROM places WHERE id IN (SELECT id FROM subtree) AND id != ?1 AND deleted_at_utc IS NULL) AS nested_places,
           (SELECT COUNT(*) FROM devices WHERE place_id IN (SELECT id FROM subtree) AND deleted_at_utc IS NULL) AS device_count,
           (SELECT COUNT(*) FROM cartridges WHERE place_id IN (SELECT id FROM subtree) AND deleted_at_utc IS NULL) AS cartridge_count,
           -- CR-01 (phase 39 review): acts referencing this subtree through ANY of
           -- the three D-16 frozen-snapshot columns (`acts.place_id`,
           -- `acts.bulk_place_id`, `act_items.place_id_override`) — DISTINCT on
           -- `a.id` so an act referencing the subtree through two/three columns at
           -- once still counts once. `act_items` has no `deleted_at_utc` of its own
           -- (D-Schema-03, junction table); soft-delete is checked on the parent
           -- act only, matching how the row would actually disappear from the UI.
           (SELECT COUNT(DISTINCT a.id) FROM acts a
              LEFT JOIN act_items ai ON ai.act_id = a.id
            WHERE (a.place_id IN (SELECT id FROM subtree)
                OR a.bulk_place_id IN (SELECT id FROM subtree)
                OR ai.place_id_override IN (SELECT id FROM subtree))
              AND a.deleted_at_utc IS NULL) AS referencing_act_count",
        rusqlite::params![root_id],
        |row| {
            Ok(SubtreeStats {
                direct_children: row.get(0)?,
                nested_places: row.get(1)?,
                device_count: row.get(2)?,
                cartridge_count: row.get(3)?,
                referencing_act_count: row.get(4)?,
            })
        },
    )
    .map_err(map_rusqlite)
}

/// Pattern 2's "content of place" leg (PLC-06 / D-23): devices, printers
/// (devices whose `type_id` is the seeded "Принтер" type — printers have no
/// `place_id` of their own, they resolve through `devices.place_id`, per
/// V020__printers.sql), and cartridges — UNIONed into one `PlaceContentRow`
/// shape. `nested: true` (default, D-24) includes the whole subtree via the
/// Pattern 2 descendant CTE; `nested: false` restricts to `place_id = root_id`
/// exactly ("Только здесь").
fn list_subtree_contents_impl(
    conn: &Connection,
    root_id: i64,
    nested: bool,
) -> Result<Vec<PlaceContentRow>, AppError> {
    let (sep_ends, sep_last_two) = read_path_display_separators(conn);
    let cte = if nested {
        "WITH RECURSIVE subtree(id) AS (
            SELECT id FROM places WHERE id = ?1 AND deleted_at_utc IS NULL
            UNION ALL
            SELECT p.id FROM places p
            JOIN subtree s ON p.parent_id = s.id
            WHERE p.deleted_at_utc IS NULL
         )
         "
    } else {
        ""
    };
    let place_filter = if nested {
        "IN (SELECT id FROM subtree)"
    } else {
        "= ?1"
    };

    let sql = format!(
        "{cte}SELECT 'device' AS kind, d.id, d.name, d.inventory_number,
                pfp.full_path, ds.name AS status_name, pev.effective_variant
         FROM devices d
         JOIN place_full_paths pfp ON pfp.place_id = d.place_id
         LEFT JOIN device_statuses ds ON ds.id = d.status_id
         LEFT JOIN place_effective_variant pev ON pev.place_id = d.place_id
         WHERE d.deleted_at_utc IS NULL
           AND d.type_id != (SELECT id FROM device_types WHERE name = 'Принтер')
           AND d.place_id {place_filter}
         UNION ALL
         SELECT 'printer' AS kind, d.id, d.name, d.inventory_number,
                pfp.full_path, ds.name AS status_name, pev.effective_variant
         FROM devices d
         JOIN place_full_paths pfp ON pfp.place_id = d.place_id
         LEFT JOIN device_statuses ds ON ds.id = d.status_id
         LEFT JOIN place_effective_variant pev ON pev.place_id = d.place_id
         WHERE d.deleted_at_utc IS NULL
           AND d.type_id = (SELECT id FROM device_types WHERE name = 'Принтер')
           AND d.place_id {place_filter}
         UNION ALL
         SELECT 'cartridge' AS kind, c.id, (m.brand || ' ' || m.model) AS name, c.code,
                pfp.full_path, cs.name AS status_name, pev.effective_variant
         FROM cartridges c
         JOIN place_full_paths pfp ON pfp.place_id = c.place_id
         JOIN cartridge_models m ON m.id = c.model_id
         LEFT JOIN cartridge_statuses cs ON cs.id = c.status_id
         LEFT JOIN place_effective_variant pev ON pev.place_id = c.place_id
         WHERE c.deleted_at_utc IS NULL
           AND c.place_id {place_filter}"
    );

    let mut stmt = conn.prepare(&sql).map_err(map_rusqlite)?;
    let rows = stmt
        .query_map(rusqlite::params![root_id], |row| {
            let full_path: String = row.get(4)?;
            let effective_variant: Option<String> = row.get(6)?;
            let place_path_short = effective_variant
                .and_then(|token| PathDisplayVariant::from_str(&token).ok())
                .map(|variant| shorten_place_path(&full_path, variant, &sep_ends, &sep_last_two))
                .unwrap_or_else(|| full_path.clone());
            Ok(PlaceContentRow {
                kind: row.get(0)?,
                id: row.get(1)?,
                name: row.get(2)?,
                inventory_or_code: row.get(3)?,
                full_path,
                place_path_short,
                status_name: row.get(5)?,
            })
        })
        .map_err(map_rusqlite)?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(map_rusqlite)
}

/// D-11.4: a place counts as a storage place if it itself OR any ancestor has
/// `is_storage = 1`. Distinct CTE shape from `subtree_stats_impl`/
/// `list_subtree_contents_impl` (which walk DOWN from one root via
/// `parent_id`) — this one walks UP from every node's own `parent_id` chain.
/// Sole source of data for all three D-11 is_storage effects (D-10: is_storage
/// never determines an item's own status).
fn list_storage_place_ids_impl(conn: &Connection) -> Result<Vec<i64>, AppError> {
    let mut stmt = conn
        .prepare(
            "WITH RECURSIVE anc(orig_id, id) AS (
                SELECT id, id FROM places WHERE deleted_at_utc IS NULL
                UNION ALL
                SELECT a.orig_id, p.parent_id
                FROM places p
                JOIN anc a ON p.id = a.id
                WHERE p.parent_id IS NOT NULL AND p.deleted_at_utc IS NULL
             )
             SELECT DISTINCT anc.orig_id
             FROM anc
             JOIN places p2 ON p2.id = anc.id
             WHERE p2.is_storage = 1 AND p2.deleted_at_utc IS NULL",
        )
        .map_err(map_rusqlite)?;
    let rows = stmt.query_map([], |row| row.get(0)).map_err(map_rusqlite)?;
    rows.collect::<rusqlite::Result<Vec<i64>>>()
        .map_err(map_rusqlite)
}

/// Resolve the root-to-leaf, `' / '`-joined full path via `place_full_paths`
/// (always live, never cached — the view recomputes on every query).
fn full_path_impl(conn: &Connection, id: i64) -> Result<String, AppError> {
    conn.query_row(
        "SELECT full_path FROM place_full_paths WHERE place_id = ?1",
        rusqlite::params![id],
        |row| row.get(0),
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound {
            entity: "place",
            id,
        },
        other => map_rusqlite(other),
    })
}

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

    fn list_children(
        &self,
        conn: &Self::Conn,
        parent_id: Option<i64>,
    ) -> Result<Vec<PlaceRow>, AppError> {
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
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(map_rusqlite)
    }

    fn list_all(
        &self,
        conn: &Self::Conn,
        include_archived: bool,
    ) -> Result<Vec<PlaceRow>, AppError> {
        let sql = if include_archived {
            format!("{SELECT_PLACES} WHERE p.deleted_at_utc IS NULL")
        } else {
            format!("{SELECT_PLACES} WHERE p.deleted_at_utc IS NULL AND p.archived_at_utc IS NULL")
        };
        let mut stmt = conn.prepare(&sql).map_err(map_rusqlite)?;
        let rows = stmt.query_map([], from_row).map_err(map_rusqlite)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(map_rusqlite)
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

    fn set_path_variant(
        &self,
        conn: &mut Self::Conn,
        id: i64,
        path_variant_override: Option<&str>,
        version: i64,
        now_utc: i64,
    ) -> Result<PlaceRow, AppError> {
        let affected = conn
            .execute(
                "UPDATE places SET path_variant_override = ?1, updated_at_utc = ?2, \
                 version = version + 1 \
                 WHERE id = ?3 AND version = ?4 AND deleted_at_utc IS NULL",
                rusqlite::params![path_variant_override, now_utc, id, version],
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
                    message:
                        "Нельзя переместить место внутрь самого себя или своего вложенного места."
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

    fn archive(
        &self,
        conn: &mut Self::Conn,
        id: i64,
        version: i64,
        now_utc: i64,
    ) -> Result<(), AppError> {
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

    fn unarchive(
        &self,
        conn: &mut Self::Conn,
        id: i64,
        version: i64,
        now_utc: i64,
    ) -> Result<(), AppError> {
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

    fn delete_hard(&self, conn: &mut Self::Conn, id: i64, version: i64) -> Result<(), AppError> {
        let tx = conn.transaction().map_err(map_rusqlite)?;

        // D-14 (literal, not negotiable): no cascade, no auto-reparenting.
        // Pattern 2 subtree-stats runs first; any non-zero count blocks the
        // delete with exact counts (not a generic refusal). `ON DELETE
        // RESTRICT` FKs (Plan 01) are defense-in-depth behind this check
        // (T-39-04-02).
        let stats = subtree_stats_impl(&tx, id)?;
        // `nested_places` уже считает ВСЕХ потомков, включая прямых детей:
        // прибавлять к нему `direct_children` — значит посчитать их дважды.
        // Сервисный слой (D-14, `build_delete_blocked_message`) тоже берёт `nested_places`.
        // CR-01: `referencing_act_count` тоже блокирует удаление — D-16 замораживает
        // ссылку акта на место даже после того, как все устройства уехали, так что
        // место с нулевыми остальными счётчиками всё ещё может быть undeletable.
        let total = stats.nested_places
            + stats.device_count
            + stats.cartridge_count
            + stats.referencing_act_count;
        if total > 0 {
            return Err(AppError::Conflict {
                reason: format!(
                    "Нельзя удалить место: содержит {} вложенных мест, {} устройств, {} картриджей, {} актов.",
                    stats.nested_places, stats.device_count, stats.cartridge_count, stats.referencing_act_count,
                ),
            });
        }

        let affected = tx
            .execute(
                "DELETE FROM places WHERE id = ?1 AND version = ?2",
                rusqlite::params![id, version],
            )
            .map_err(map_rusqlite)?;

        if affected == 0 {
            return Err(resolve_cas_failure(&tx, id, version));
        }

        tx.commit().map_err(map_rusqlite)?;
        Ok(())
    }

    fn subtree_stats(&self, conn: &Self::Conn, root_id: i64) -> Result<SubtreeStats, AppError> {
        subtree_stats_impl(conn, root_id)
    }

    fn list_subtree_contents(
        &self,
        conn: &Self::Conn,
        root_id: i64,
        nested: bool,
    ) -> Result<Vec<PlaceContentRow>, AppError> {
        list_subtree_contents_impl(conn, root_id, nested)
    }

    fn list_storage_place_ids(&self, conn: &Self::Conn) -> Result<Vec<i64>, AppError> {
        list_storage_place_ids_impl(conn)
    }

    fn full_path(&self, conn: &Self::Conn, id: i64) -> Result<String, AppError> {
        full_path_impl(conn, id)
    }
}

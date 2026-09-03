//! Single owner of `resolve_movement_act_number` — the display-number
//! resolution formula for a movement's linked act (Phase 40 gap-closure
//! CR-01/WR-10 round, Plan 40-29).
//!
//! Extracted out of `place_movement_service.rs::get_timeline` (its original,
//! only owner) so that `report_service.rs::query_movements_inner` can call
//! the exact same logic instead of re-deriving it from a raw `a.number`
//! column select — the WR-03/WR-08 duplication anti-pattern this phase has
//! been fighting throughout. Both the timeline and the movements report now
//! show the SAME canonical number ("20в" for a solo return act, never the
//! bare parent "20") for the same underlying movement.

use rusqlite::{params, OptionalExtension};

use trackly_core::domain::acts::ActType;

use crate::dto::act::format_act_number;

/// Soft-degraded act_number resolution (never `.expect()`/`?` — an act row
/// being gone must not crash the whole timeline/report read).
///
/// CR-02: `acts.number` is `INTEGER NOT NULL` (V004__acts.sql) — read as
/// `i64` and format afterward. Reading it as `String` failed with
/// `InvalidColumnType` on every single row (never just a missing act), and
/// `.ok()` silently swallowed that real error, so this was permanently
/// `None`. The soft-degrade below is kept for a genuinely missing act row
/// (`.optional()`/`.ok()`), not to mask a type mismatch.
///
/// Routes the raw columns through the SAME query shape as
/// `SqliteActRepository::SELECT_ACTS` (acts_sqlite.rs) — `act_type`,
/// `sub_number`, the parent's `number` via a self-join, and a correlated
/// `sibling_return_count` subquery — then hands them to `format_act_number`,
/// the single owner of the display rule (D-Numbering-01).
pub fn resolve_movement_act_number(
    conn: &rusqlite::Connection,
    act_id: Option<i64>,
) -> Option<String> {
    act_id.and_then(|act_id| {
        conn.query_row(
            "SELECT a.number, a.sub_number, a.act_type, p.number AS parent_number, \
                    (SELECT COUNT(*) FROM acts r \
                        WHERE r.parent_act_id = COALESCE(a.parent_act_id, a.id) \
                          AND r.deleted_at_utc IS NULL) AS sibling_return_count \
               FROM acts a \
               LEFT JOIN acts p ON p.id = a.parent_act_id \
              WHERE a.id = ?1",
            params![act_id],
            |r| {
                let number: i64 = r.get(0)?;
                let sub_number: Option<i64> = r.get(1)?;
                let act_type_sql: String = r.get(2)?;
                let parent_number: Option<i64> = r.get(3)?;
                let sibling_return_count: Option<i64> = r.get(4)?;
                // Same soft-degrade contract as `acts_sqlite.rs::from_row`:
                // an unexpected value is an `Err` here, absorbed into `None`
                // by `.optional().ok()` below — never `?`/`.expect()`.
                let act_type = match act_type_sql.as_str() {
                    "handover" => ActType::Handover,
                    "return" => ActType::Return,
                    other => {
                        return Err(rusqlite::Error::FromSqlConversionFailure(
                            2,
                            rusqlite::types::Type::Text,
                            format!("invalid act_type in DB: {other}").into(),
                        ));
                    }
                };
                Ok(format_act_number(
                    act_type,
                    number,
                    sub_number,
                    parent_number,
                    sibling_return_count,
                ))
            },
        )
        .optional()
        .ok()
        .flatten()
    })
}

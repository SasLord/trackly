//! SQLite adapter for the `place_movements` table (Phase 40, migration V040).
//!
//! **This is the single write-side entry point for Phase 40 (D-01).** Every write site
//! that changes an entity's `place_id` — device manual move, cartridge manual move,
//! cartridge transition (main + nested auto-return), act create/update/do_return/
//! update_return, and D-28's bulk move — MUST call [`SqlitePlaceMovementsRepository::
//! record_movement_if_applicable`]. No write site is allowed to construct its own
//! `INSERT INTO place_movements` or re-derive the D-04/D-06 skip guard: that duplication
//! is exactly the "5 copies, 1 forgot to degrade softly" shape that caused IN-01 in
//! Phase 39.2.
//!
//! Mirrors [`crate::repos::audit_log_sqlite::SqliteAuditLogRepository`]'s zero-field
//! unit-struct shape and its `*_in_tx(&self, tx: &Transaction<'_>, ...)` convention:
//! every write method here operates on the caller's already-open transaction and never
//! opens its own (D-01) — the movement row must land in the same transaction as the
//! mutation that caused it.

use rusqlite::{params, Connection, Transaction};
use trackly_core::domain::place_movements::{
    is_reportable_place_change, MovementEntityKind, MovementSource,
};
use trackly_core::error::AppError;
use trackly_core::ports::places::PlaceRepository;

use crate::error_conversions::map_rusqlite;

/// SQLite-backed place_movements repository adapter (zero-sized).
#[derive(Debug, Default, Clone)]
pub struct SqlitePlaceMovementsRepository;

/// A single `place_movements` row, ready to be inserted.
///
/// `'a` ties the `note` borrow to the caller's frame; `from_place_path`/`to_place_path`/
/// `actor_name_snapshot` are owned because they are resolved (JOIN-free, at write time —
/// D-09/D-10) by the caller (`record_movement_if_applicable`) just before this struct is
/// built.
#[derive(Debug, Clone)]
pub struct NewMovement<'a> {
    pub entity_type: &'static str,
    pub entity_id: i64,
    pub from_place_id: i64,
    pub from_place_path: String,
    pub to_place_id: i64,
    pub to_place_path: String,
    pub source: &'static str,
    pub note: Option<&'a str>,
    pub act_id: Option<i64>,
    pub user_id: Option<i64>,
    pub actor_name_snapshot: Option<String>,
    pub created_at_utc: i64,
}

/// A single `place_movements` row, as read back for history display (HST-02/HST-04).
#[derive(Debug, Clone)]
pub struct MovementRow {
    pub id: i64,
    pub entity_type: String,
    pub entity_id: i64,
    pub from_place_id: i64,
    pub from_place_path: String,
    pub to_place_id: i64,
    pub to_place_path: String,
    pub source: String,
    pub note: Option<String>,
    pub act_id: Option<i64>,
    pub user_id: Option<i64>,
    pub actor_name_snapshot: Option<String>,
    pub created_at_utc: i64,
}

impl SqlitePlaceMovementsRepository {
    /// Insert a single `place_movements` row inside an open transaction.
    ///
    /// No guard logic here — the D-04/D-06 skip decision lives one level up, in
    /// `record_movement_if_applicable`. This is a plain, unconditional INSERT.
    pub fn insert_in_tx(
        &self,
        tx: &Transaction<'_>,
        movement: NewMovement<'_>,
    ) -> Result<(), AppError> {
        tx.execute(
            "INSERT INTO place_movements \
             (entity_type, entity_id, from_place_id, from_place_path, to_place_id, to_place_path, \
              source, note, act_id, user_id, actor_name_snapshot, created_at_utc) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                movement.entity_type,
                movement.entity_id,
                movement.from_place_id,
                movement.from_place_path,
                movement.to_place_id,
                movement.to_place_path,
                movement.source,
                movement.note,
                movement.act_id,
                movement.user_id,
                movement.actor_name_snapshot,
                movement.created_at_utc,
            ],
        )
        .map_err(map_rusqlite)?;
        Ok(())
    }

    /// The single shared write-side helper (D-01). Every one of the seven write sites
    /// calls this — never `insert_in_tx` directly — so the D-04/D-06 guard and the
    /// actor/path snapshot logic exist in exactly one place.
    ///
    /// 1. `is_reportable_place_change` decides FIRST, before any snapshot work, whether
    ///    this change is worth recording at all (D-04: no-op edit; D-06: first
    ///    assignment `NULL -> place` or clearing `place -> NULL` — neither is a "move").
    /// 2. If reportable, resolves both place-path snapshots via
    ///    `PlaceRepository::full_path` (D-10) — never a later JOIN.
    /// 3. Resolves the actor ФИО snapshot (D-09) via a direct `users` lookup that
    ///    soft-degrades to `None` on any failure (missing row, DB hiccup) — mirrors the
    ///    Pitfall 6 / IN-01 discipline: a missing `users` row must never crash the
    ///    mutation that triggered this movement recording.
    #[allow(clippy::too_many_arguments)]
    pub fn record_movement_if_applicable(
        &self,
        tx: &Transaction<'_>,
        places_repo: &dyn PlaceRepository<Conn = Connection>,
        entity_type: MovementEntityKind,
        entity_id: i64,
        before_place_id: Option<i64>,
        after_place_id: Option<i64>,
        source: MovementSource,
        note: Option<&str>,
        act_id: Option<i64>,
        user_id: Option<i64>,
        now_utc: i64,
    ) -> Result<(), AppError> {
        if !is_reportable_place_change(before_place_id, after_place_id) {
            return Ok(());
        }

        // Safe: is_reportable_place_change already proved both sides are Some.
        let from_place_id = before_place_id.expect("guarded Some(before_place_id)");
        let to_place_id = after_place_id.expect("guarded Some(after_place_id)");

        let from_place_path = places_repo.full_path(tx, from_place_id)?;
        let to_place_path = places_repo.full_path(tx, to_place_id)?;

        // Soft-degrade (Pitfall 6 / IN-01): a missing/unreadable `users` row must never
        // fail the mutation that triggered this movement — `.ok()`, never `?`.
        let actor_name_snapshot = user_id.and_then(|uid| {
            tx.query_row(
                "SELECT full_name FROM users WHERE id = ?1",
                params![uid],
                |r| r.get::<_, String>(0),
            )
            .ok()
        });

        self.insert_in_tx(
            tx,
            NewMovement {
                entity_type: entity_type.as_str(),
                entity_id,
                from_place_id,
                from_place_path,
                to_place_id,
                to_place_path,
                source: source.as_str(),
                note,
                act_id,
                user_id,
                actor_name_snapshot,
                created_at_utc: now_utc,
            },
        )
    }

    /// D-03: undo scoping. Deletes all `place_movements` rows tied to a given act — a
    /// plain `DELETE`, no compensating record. This is the ONLY place the
    /// `DELETE FROM place_movements WHERE act_id = ?` SQL may live; plan 40-20's undo
    /// path calls this rather than hand-rolling the statement in `act_service.rs`.
    ///
    /// Returns the number of rows deleted.
    pub fn delete_by_act_id_in_tx(
        &self,
        tx: &Transaction<'_>,
        act_id: i64,
    ) -> Result<usize, AppError> {
        tx.execute(
            "DELETE FROM place_movements WHERE act_id = ?1",
            params![act_id],
        )
        .map_err(map_rusqlite)
    }

    /// HST-02/HST-17 timeline read: all movements for one `(entity_type, entity_id)`
    /// pair, newest-first (D-20), unpaginated (no `LIMIT` — D-20/RESEARCH's scale note:
    /// movements per item are "единицы за годы" at this org's scale).
    pub fn get_history(
        &self,
        conn: &Connection,
        entity_type: &str,
        entity_id: i64,
    ) -> Result<Vec<MovementRow>, AppError> {
        let mut stmt = conn
            .prepare(
                "SELECT id, entity_type, entity_id, from_place_id, from_place_path, to_place_id, \
                        to_place_path, source, note, act_id, user_id, actor_name_snapshot, created_at_utc \
                   FROM place_movements \
                  WHERE entity_type = ?1 AND entity_id = ?2 \
                  ORDER BY created_at_utc DESC, id DESC",
            )
            .map_err(map_rusqlite)?;

        let rows = stmt
            .query_map(params![entity_type, entity_id], |r| {
                Ok(MovementRow {
                    id: r.get(0)?,
                    entity_type: r.get(1)?,
                    entity_id: r.get(2)?,
                    from_place_id: r.get(3)?,
                    from_place_path: r.get(4)?,
                    to_place_id: r.get(5)?,
                    to_place_path: r.get(6)?,
                    source: r.get(7)?,
                    note: r.get(8)?,
                    act_id: r.get(9)?,
                    user_id: r.get(10)?,
                    actor_name_snapshot: r.get(11)?,
                    created_at_utc: r.get(12)?,
                })
            })
            .map_err(map_rusqlite)?;

        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(map_rusqlite)?);
        }
        Ok(out)
    }
}

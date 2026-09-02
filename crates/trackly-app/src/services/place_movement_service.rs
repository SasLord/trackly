//! `PlaceMovementService` — read-side application service for the movement-history
//! timeline (Phase 40 Plan 10, HST-02).
//!
//! Read-only: routed entirely through the reader pool via
//! `tokio::task::spawn_blocking` (mirrors `CartridgeService::get_history`'s shape),
//! never touches the writer. `get_timeline` is the ONE method on this service —
//! gated by `authorize(caller, &Action::ReadPlaces)` (D-12, Admin|Manager) as its
//! first line, matching the movements report's gate (Plan 40-11/40-12) and
//! `PlaceService`'s own read methods.
//!
//! D-21: a printer's timeline is not a special case here — printers are stored with
//! `entity_type = "device"` (`MovementEntityKind` has no `Printer` variant), so the
//! CALLER (UI, Plan 40-15/17) is responsible for passing `entity_type = "device"` with
//! the printer's underlying device id. This service has no branch for "printer" at all.

use std::sync::Arc;

use rusqlite::{params, OptionalExtension};

use trackly_core::auth::{authorize, Action, Identity};
use trackly_core::error::AppError;
use trackly_infra::db::pools::ReaderPool;
use trackly_infra::repos::SqlitePlaceMovementsRepository;

use crate::dto::place_movements::MovementEntryDto;
use crate::services::place_path_display::compute_place_path_short;

/// Application service for the HST-02 timeline read side.
pub struct PlaceMovementService {
    pub readers: Arc<ReaderPool>,
    pub(crate) repo: Arc<SqlitePlaceMovementsRepository>,
}

impl PlaceMovementService {
    pub fn new(readers: Arc<ReaderPool>) -> Self {
        Self {
            readers,
            repo: Arc::new(SqlitePlaceMovementsRepository),
        }
    }

    /// Returns the full, unpaginated, newest-first movement timeline for one
    /// `(entity_type, entity_id)` pair (D-20's `get_history` ordering, Plan 40-05).
    ///
    /// Gated by `Action::ReadPlaces` (D-12, Admin|Manager) — checked FIRST, before any
    /// DB query runs (T-40-21: BOLA mitigation — an Employee is denied the entire
    /// surface regardless of `entity_id`, there is no per-item ownership check to
    /// bypass).
    pub async fn get_timeline(
        &self,
        caller: &Identity,
        entity_type: &str,
        entity_id: i64,
    ) -> Result<Vec<MovementEntryDto>, AppError> {
        authorize(caller, &Action::ReadPlaces)?;

        let readers = self.readers.clone();
        let repo = self.repo.clone();
        let entity_type = entity_type.to_string();

        tokio::task::spawn_blocking(move || -> Result<Vec<MovementEntryDto>, AppError> {
            let conn = readers.acquire();
            let rows = repo.get_history(&conn, &entity_type, entity_id)?;

            let mut out = Vec::with_capacity(rows.len());
            for row in rows {
                // D-11 actor-display precedence: ФИО snapshot → current login
                // fallback (edge case, should not normally happen post-40-05) →
                // "система" for user_id IS NULL.
                let actor_display = match (row.user_id, row.actor_name_snapshot.as_deref()) {
                    (Some(_), Some(name)) if !name.is_empty() => name.to_string(),
                    (Some(uid), _) => conn
                        .query_row("SELECT login FROM users WHERE id = ?1", params![uid], |r| {
                            r.get::<_, String>(0)
                        })
                        .optional()
                        .ok()
                        .flatten()
                        .unwrap_or_else(|| "система".to_string()),
                    (None, _) => "система".to_string(),
                };

                // Soft-degraded act_number resolution (never `.expect()`/`?` — an
                // act row being gone must not crash the whole timeline read).
                let act_number: Option<String> = row.act_id.and_then(|act_id| {
                    conn.query_row(
                        "SELECT number FROM acts WHERE id = ?1",
                        params![act_id],
                        |r| r.get::<_, String>(0),
                    )
                    .optional()
                    .ok()
                    .flatten()
                });

                // Shortening formula: single owner is
                // `place_path_display::compute_place_path_short` (Plan 40-02) — no
                // JS mirror, no re-derived copy (WR-03/WR-08 anti-pattern).
                let from_place_path_short = compute_place_path_short(
                    &readers,
                    Some(row.from_place_id),
                    Some(row.from_place_path.clone()),
                );
                let to_place_path_short = compute_place_path_short(
                    &readers,
                    Some(row.to_place_id),
                    Some(row.to_place_path.clone()),
                );

                out.push(MovementEntryDto {
                    id: row.id,
                    entity_type: row.entity_type,
                    entity_id: row.entity_id,
                    from_place_id: row.from_place_id,
                    from_place_path: row.from_place_path,
                    from_place_path_short,
                    to_place_id: row.to_place_id,
                    to_place_path: row.to_place_path,
                    to_place_path_short,
                    actor_display,
                    // Raw token passed through by design (Pitfall 6 / IN-01):
                    // an unrecognized `source` never crashes this read — the UI
                    // composes the final phrase, `MovementSource::from_str_lenient`
                    // is available for any Rust-side caller needing a typed match.
                    source: row.source,
                    note: row.note,
                    act_id: row.act_id,
                    act_number,
                    created_at_utc: row.created_at_utc,
                });
            }

            Ok(out)
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking: {e}"),
        })?
    }
}

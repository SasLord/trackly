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
use crate::services::act_number_display::resolve_movement_act_number;
use crate::services::place_path_display::compute_place_path_short_with_conn;

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

            let out: Vec<MovementEntryDto> = rows
                .into_iter()
                .map(|row| movement_row_to_dto(&conn, row))
                .collect();

            Ok(out)
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking: {e}"),
        })?
    }
}

/// ФИО-снимок → login текущего пользователя → «система» (D-11). Единый владелец
/// приоритета автора для перемещений и событий истории (фаза 41.10).
pub(crate) fn resolve_actor_display(
    conn: &rusqlite::Connection,
    user_id: Option<i64>,
    snapshot: Option<&str>,
) -> String {
    match (user_id, snapshot) {
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
    }
}

/// Строка журнала → `MovementEntryDto`. Единый владелец формул номера акта и
/// сокращённых путей (общий для `get_timeline` и `HistoryService`).
pub(crate) fn movement_row_to_dto(
    conn: &rusqlite::Connection,
    row: trackly_infra::repos::place_movements_sqlite::MovementRow,
) -> MovementEntryDto {
    let actor_display =
        resolve_actor_display(conn, row.user_id, row.actor_name_snapshot.as_deref());
    let act_number: Option<String> = resolve_movement_act_number(conn, row.act_id);
    let from_place_path_short = compute_place_path_short_with_conn(
        conn,
        Some(row.from_place_id),
        Some(row.from_place_path.clone()),
    );
    let to_place_path_short = compute_place_path_short_with_conn(
        conn,
        Some(row.to_place_id),
        Some(row.to_place_path.clone()),
    );
    MovementEntryDto {
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
        source: row.source,
        note: row.note,
        act_id: row.act_id,
        act_number,
        batch_id: row.batch_id,
        group_id: row.group_id,
        group_label: row.entity_label,
        request_id: row.request_id,
        created_at_utc: row.created_at_utc,
    }
}

//! Place-movement timeline Tauri command — Plan 40-10 (HST-02).
//!
//! Паттерн: `build_*` helper + thin tauri-command/specta wrapper (mirrors
//! `tauri_cmds/places.rs` verbatim). Both transports (Tauri invoke + axum HTTP, see
//! `http/place_movements.rs`) delegate to the SAME `build_place_movements_get_timeline`
//! function — no business logic duplication across transports (T-40-22 mitigation).
//!
//! `PlaceMovementService::get_timeline` already calls `authorize(caller,
//! &Action::ReadPlaces)` internally as its first line — the `authorize()` call here is
//! a second, deliberate defense-in-depth gate at the transport boundary, matching every
//! other `build_*` helper file's convention in this codebase (`build_places_*`,
//! `build_devices_*`, etc.), none of which assume their underlying service self-gates.
//!
//! specta attribute ПОСЛЕ tauri-command attribute — требование tauri-specta v2 rc.21.

use crate::context::AppCtx;
use crate::dto::place_movements::MovementEntryDto;
use crate::tauri_cmds::users::resolve_tauri_identity;
use trackly_core::auth::{authorize, Action, Identity};
use trackly_core::error::AppError;

/// Чтение: требует `caller` с правом `ReadPlaces` (D-12, Admin|Manager).
pub async fn build_place_movements_get_timeline(
    ctx: &AppCtx,
    caller: &Identity,
    entity_type: String,
    entity_id: i64,
) -> Result<Vec<MovementEntryDto>, AppError> {
    authorize(caller, &Action::ReadPlaces)?;
    ctx.place_movements
        .get_timeline(caller, &entity_type, entity_id)
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn place_movements_get_timeline(
    state: tauri::State<'_, AppCtx>,
    entity_type: String,
    entity_id: i32,
) -> Result<Vec<MovementEntryDto>, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_place_movements_get_timeline(state.inner(), &caller, entity_type, entity_id as i64).await
}

//! Place Tauri commands — Plan 12: exposes the completed `PlaceService`
//! (Plans 05 + 08) over the Tauri transport.
//!
//! Паттерн: `build_*` helper + thin tauri-command/specta wrapper (identical to
//! `tauri_cmds/devices.rs` — mirrored verbatim). Оба транспорта (Tauri invoke +
//! axum HTTP, see `http/places.rs`) делегируют одним и тем же `build_places_*`
//! функциям — no business logic duplication across transports.
//!
//! specta attribute ПОСЛЕ tauri-command attribute — требование tauri-specta v2 rc.21.
//!
//! D-20: every mutation helper gates `Action::MutatePlaces` (Admin-only — the
//! ONE Action in this project where Manager cannot mutate, unlike every other
//! Mutate* Action). Every read helper gates `Action::ReadPlaces` (Admin|Manager).
//! `PlaceService`'s own methods (Plans 05/08) already call `authorize()`
//! internally as their first line — the `authorize()` calls here are a second,
//! deliberate defense-in-depth gate at the transport boundary, matching this
//! plan's explicit acceptance criteria and every other `build_*` helper file's
//! convention in this codebase (`build_devices_*`, `build_cartridges_*`, etc.),
//! none of which assume their underlying service self-gates.

use crate::context::AppCtx;
use crate::dto::place::{PlaceContentDto, PlaceDto, PlaceNewDto, PlacePathDto, SubtreeStatsDto};
use crate::tauri_cmds::users::resolve_tauri_identity;
use trackly_core::auth::{authorize, Action, Identity};
use trackly_core::error::AppError;

// ---------------------------------------------------------------------------
// build_* helpers — используются и Tauri, и axum транспортами
// ---------------------------------------------------------------------------

/// Мутация: требует `caller` с правом `MutatePlaces` (D-20, Admin-only).
pub async fn build_places_create(
    ctx: &AppCtx,
    caller: &Identity,
    new: PlaceNewDto,
) -> Result<PlaceDto, AppError> {
    authorize(caller, &Action::MutatePlaces)?;
    let domain_new = new.into_domain()?;
    ctx.places.create(caller, domain_new).await
}

/// Мутация: требует `caller` с правом `MutatePlaces` (D-20, Admin-only).
pub async fn build_places_rename(
    ctx: &AppCtx,
    caller: &Identity,
    id: i64,
    name: String,
    version: i64,
) -> Result<PlaceDto, AppError> {
    authorize(caller, &Action::MutatePlaces)?;
    ctx.places.rename(caller, id, name, version).await
}

/// Мутация: требует `caller` с правом `MutatePlaces` (D-12/D-20, Admin-only —
/// НЕ `Action::ManageSettings`, права те же, что у `places_rename`).
pub async fn build_places_set_path_variant(
    ctx: &AppCtx,
    caller: &Identity,
    id: i64,
    path_variant_override: Option<String>,
    version: i64,
) -> Result<PlaceDto, AppError> {
    authorize(caller, &Action::MutatePlaces)?;
    ctx.places
        .set_path_variant(caller, id, path_variant_override, version)
        .await
}

/// Мутация: требует `caller` с правом `MutatePlaces` (D-20, Admin-only).
pub async fn build_places_move(
    ctx: &AppCtx,
    caller: &Identity,
    id: i64,
    new_parent_id: Option<i64>,
    version: i64,
) -> Result<PlaceDto, AppError> {
    authorize(caller, &Action::MutatePlaces)?;
    ctx.places
        .move_node(caller, id, new_parent_id, version)
        .await
}

/// Мутация: требует `caller` с правом `MutatePlaces` (D-20, Admin-only).
pub async fn build_places_archive(
    ctx: &AppCtx,
    caller: &Identity,
    id: i64,
    version: i64,
) -> Result<(), AppError> {
    authorize(caller, &Action::MutatePlaces)?;
    ctx.places.archive(caller, id, version).await
}

/// Мутация: требует `caller` с правом `MutatePlaces` (D-20, Admin-only).
pub async fn build_places_unarchive(
    ctx: &AppCtx,
    caller: &Identity,
    id: i64,
    version: i64,
) -> Result<(), AppError> {
    authorize(caller, &Action::MutatePlaces)?;
    ctx.places.unarchive(caller, id, version).await
}

/// Мутация: требует `caller` с правом `MutatePlaces` (D-20, Admin-only).
pub async fn build_places_delete(
    ctx: &AppCtx,
    caller: &Identity,
    id: i64,
    version: i64,
) -> Result<(), AppError> {
    authorize(caller, &Action::MutatePlaces)?;
    ctx.places.delete_hard(caller, id, version).await
}

/// Чтение: требует `caller` с правом `ReadPlaces` (D-20, Admin|Manager).
pub async fn build_places_get(
    ctx: &AppCtx,
    caller: &Identity,
    id: i64,
) -> Result<PlaceDto, AppError> {
    authorize(caller, &Action::ReadPlaces)?;
    let row = ctx.places.get(caller, id).await?;
    Ok(PlaceDto::from(row))
}

/// Чтение: требует `caller` с правом `ReadPlaces` (D-20, Admin|Manager).
pub async fn build_places_list_children(
    ctx: &AppCtx,
    caller: &Identity,
    parent_id: Option<i64>,
) -> Result<Vec<PlaceDto>, AppError> {
    authorize(caller, &Action::ReadPlaces)?;
    let rows = ctx.places.list_children(caller, parent_id).await?;
    Ok(rows.into_iter().map(PlaceDto::from).collect())
}

/// Чтение: требует `caller` с правом `ReadPlaces` (D-20, Admin|Manager).
pub async fn build_places_list_all(
    ctx: &AppCtx,
    caller: &Identity,
    include_archived: bool,
) -> Result<Vec<PlaceDto>, AppError> {
    authorize(caller, &Action::ReadPlaces)?;
    let rows = ctx.places.list_all(caller, include_archived).await?;
    Ok(rows.into_iter().map(PlaceDto::from).collect())
}

/// Чтение: требует `caller` с правом `ReadPlaces` (D-20, Admin|Manager).
pub async fn build_places_subtree_stats(
    ctx: &AppCtx,
    caller: &Identity,
    root_id: i64,
) -> Result<SubtreeStatsDto, AppError> {
    authorize(caller, &Action::ReadPlaces)?;
    let stats = ctx.places.subtree_stats(caller, root_id).await?;
    Ok(SubtreeStatsDto::from(stats))
}

/// Чтение: требует `caller` с правом `ReadPlaces` (D-20, Admin|Manager).
/// Wraps `PlaceService::list_subtree_contents` (PLC-06 / D-23 — "content of
/// place" listing; `nested: true` default per D-24 includes the whole subtree).
pub async fn build_places_contents(
    ctx: &AppCtx,
    caller: &Identity,
    root_id: i64,
    nested: bool,
) -> Result<Vec<PlaceContentDto>, AppError> {
    authorize(caller, &Action::ReadPlaces)?;
    let rows = ctx
        .places
        .list_subtree_contents(caller, root_id, nested)
        .await?;
    Ok(rows.into_iter().map(PlaceContentDto::from).collect())
}

/// Чтение: требует `caller` с правом `ReadPlaces` (D-20, Admin|Manager).
/// Cyrillic-safe full-path substring search (PLC-03/PLC-05) — see
/// `PlaceService::search`'s own doc-comment for the no-SQL-LIKE rationale.
pub async fn build_places_search(
    ctx: &AppCtx,
    caller: &Identity,
    query: String,
) -> Result<Vec<PlacePathDto>, AppError> {
    authorize(caller, &Action::ReadPlaces)?;
    ctx.places.search(caller, query).await
}

// ---------------------------------------------------------------------------
// Tauri commands — thin wrappers
// ---------------------------------------------------------------------------

#[tauri::command]
#[specta::specta]
pub async fn places_create(
    state: tauri::State<'_, AppCtx>,
    place: PlaceNewDto,
) -> Result<PlaceDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_places_create(state.inner(), &caller, place).await
}

#[tauri::command]
#[specta::specta]
pub async fn places_rename(
    state: tauri::State<'_, AppCtx>,
    id: i32,
    name: String,
    version: i32,
) -> Result<PlaceDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_places_rename(state.inner(), &caller, id as i64, name, version as i64).await
}

#[tauri::command]
#[specta::specta]
pub async fn places_set_path_variant(
    state: tauri::State<'_, AppCtx>,
    id: i32,
    path_variant_override: Option<String>,
    version: i32,
) -> Result<PlaceDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_places_set_path_variant(
        state.inner(),
        &caller,
        id as i64,
        path_variant_override,
        version as i64,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn places_move(
    state: tauri::State<'_, AppCtx>,
    id: i32,
    new_parent_id: Option<i32>,
    version: i32,
) -> Result<PlaceDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_places_move(
        state.inner(),
        &caller,
        id as i64,
        new_parent_id.map(|v| v as i64),
        version as i64,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn places_archive(
    state: tauri::State<'_, AppCtx>,
    id: i32,
    version: i32,
) -> Result<(), AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_places_archive(state.inner(), &caller, id as i64, version as i64).await
}

#[tauri::command]
#[specta::specta]
pub async fn places_unarchive(
    state: tauri::State<'_, AppCtx>,
    id: i32,
    version: i32,
) -> Result<(), AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_places_unarchive(state.inner(), &caller, id as i64, version as i64).await
}

#[tauri::command]
#[specta::specta]
pub async fn places_delete(
    state: tauri::State<'_, AppCtx>,
    id: i32,
    version: i32,
) -> Result<(), AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_places_delete(state.inner(), &caller, id as i64, version as i64).await
}

#[tauri::command]
#[specta::specta]
pub async fn places_get(state: tauri::State<'_, AppCtx>, id: i32) -> Result<PlaceDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_places_get(state.inner(), &caller, id as i64).await
}

#[tauri::command]
#[specta::specta]
pub async fn places_list_children(
    state: tauri::State<'_, AppCtx>,
    parent_id: Option<i32>,
) -> Result<Vec<PlaceDto>, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_places_list_children(state.inner(), &caller, parent_id.map(|v| v as i64)).await
}

#[tauri::command]
#[specta::specta]
pub async fn places_list_all(
    state: tauri::State<'_, AppCtx>,
    include_archived: bool,
) -> Result<Vec<PlaceDto>, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_places_list_all(state.inner(), &caller, include_archived).await
}

#[tauri::command]
#[specta::specta]
pub async fn places_subtree_stats(
    state: tauri::State<'_, AppCtx>,
    root_id: i32,
) -> Result<SubtreeStatsDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_places_subtree_stats(state.inner(), &caller, root_id as i64).await
}

#[tauri::command]
#[specta::specta]
pub async fn places_contents(
    state: tauri::State<'_, AppCtx>,
    root_id: i32,
    nested: bool,
) -> Result<Vec<PlaceContentDto>, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_places_contents(state.inner(), &caller, root_id as i64, nested).await
}

#[tauri::command]
#[specta::specta]
pub async fn places_search(
    state: tauri::State<'_, AppCtx>,
    query: String,
) -> Result<Vec<PlacePathDto>, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_places_search(state.inner(), &caller, query).await
}

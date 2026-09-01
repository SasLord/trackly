//! Cartridges Tauri commands — Phase 4 Plan 03.
//!
//! Pattern (S-1): `build_*` helper + thin `#[tauri::command] #[specta::specta]`
//! wrapper. Both transports (Tauri invoke + axum POST) delegate to the same
//! helper.
//!
//! The `#[specta::specta]` attribute MUST appear AFTER `#[tauri::command]`
//! — required by tauri-specta v2 rc.21.
//!
//! Phase 5 Plan 04: мутации (create, update, delete, transition, model_create,
//! model_update, model_delete) требуют `caller: &Identity` с правом `MutateCartridges`.
//! Tauri wrappers resolve identity через `resolve_tauri_identity` (D-Desktop-01/02).

use crate::context::AppCtx;
use crate::dto::cartridge::{
    AuditEntryDto, CartridgeCountsDto, CartridgeCreateDto, CartridgeDto, CartridgeFilter,
    CartridgeListResponse, CartridgeModelCreateDto, CartridgeModelDto, CartridgeModelPatchDto,
    CartridgeTransitionPayload, LowStockItemDto, Pagination,
};
use crate::tauri_cmds::users::resolve_tauri_identity;
use trackly_core::auth::{authorize, Action, Identity};
use trackly_core::error::AppError;

// ---------------------------------------------------------------------------
// build_* helpers (shared with axum handlers)
// ---------------------------------------------------------------------------

pub async fn build_cartridges_list(
    ctx: &AppCtx,
    caller: &Identity,
    filter: CartridgeFilter,
    pagination: Pagination,
) -> Result<CartridgeListResponse, AppError> {
    authorize(caller, &Action::ReadData)?;
    ctx.cartridges.list(filter, pagination).await
}

pub async fn build_cartridges_get(
    ctx: &AppCtx,
    caller: &Identity,
    id: i64,
) -> Result<CartridgeDto, AppError> {
    authorize(caller, &Action::ReadData)?;
    ctx.cartridges.get(id).await
}

/// Мутация: требует `caller` с правом `MutateCartridges` (Admin | Manager).
pub async fn build_cartridges_create(
    ctx: &AppCtx,
    caller: &Identity,
    payload: CartridgeCreateDto,
) -> Result<CartridgeDto, AppError> {
    authorize(caller, &Action::MutateCartridges)?;
    ctx.cartridges.create(payload).await
}

/// Мутация: требует `caller` с правом `MutateCartridges`.
pub async fn build_cartridges_update(
    ctx: &AppCtx,
    caller: &Identity,
    id: i64,
    version: i64,
    place_id: Option<i64>,
    notes: Option<String>,
) -> Result<CartridgeDto, AppError> {
    authorize(caller, &Action::MutateCartridges)?;
    ctx.cartridges
        .update(caller, id, version, place_id, notes)
        .await
}

/// Мутация: требует `caller` с правом `MutateCartridges`.
pub async fn build_cartridges_delete(
    ctx: &AppCtx,
    caller: &Identity,
    id: i64,
    version: i64,
) -> Result<(), AppError> {
    authorize(caller, &Action::MutateCartridges)?;
    ctx.cartridges.delete(id, version).await
}

/// Мутация: требует `caller` с правом `MutateCartridges`.
pub async fn build_cartridges_transition(
    ctx: &AppCtx,
    caller: &Identity,
    payload: CartridgeTransitionPayload,
) -> Result<CartridgeDto, AppError> {
    authorize(caller, &Action::MutateCartridges)?;
    ctx.cartridges.transition(caller, payload).await
}

pub async fn build_cartridges_search(
    ctx: &AppCtx,
    caller: &Identity,
    query: String,
    filter: CartridgeFilter,
) -> Result<CartridgeListResponse, AppError> {
    authorize(caller, &Action::ReadData)?;
    ctx.cartridges.search(query, filter).await
}

pub async fn build_cartridges_status_counts(
    ctx: &AppCtx,
    caller: &Identity,
) -> Result<CartridgeCountsDto, AppError> {
    authorize(caller, &Action::ReadData)?;
    ctx.cartridges.status_counts().await
}

pub async fn build_cartridges_get_history(
    ctx: &AppCtx,
    caller: &Identity,
    id: i64,
) -> Result<Vec<AuditEntryDto>, AppError> {
    authorize(caller, &Action::ReadData)?;
    ctx.cartridges.get_history(id).await
}

pub async fn build_cartridges_low_stock(
    ctx: &AppCtx,
    caller: &Identity,
) -> Result<Vec<LowStockItemDto>, AppError> {
    authorize(caller, &Action::ReadData)?;
    ctx.cartridges.low_stock().await
}

pub async fn build_cartridge_models_list(
    ctx: &AppCtx,
    caller: &Identity,
) -> Result<Vec<CartridgeModelDto>, AppError> {
    authorize(caller, &Action::ReadData)?;
    ctx.cartridges.model_list().await
}

pub async fn build_cartridge_models_get(
    ctx: &AppCtx,
    caller: &Identity,
    id: i64,
) -> Result<CartridgeModelDto, AppError> {
    authorize(caller, &Action::ReadData)?;
    ctx.cartridges.model_get(id).await
}

/// Мутация: требует `caller` с правом `MutateCartridges`.
pub async fn build_cartridge_models_create(
    ctx: &AppCtx,
    caller: &Identity,
    payload: CartridgeModelCreateDto,
) -> Result<CartridgeModelDto, AppError> {
    authorize(caller, &Action::MutateCartridges)?;
    ctx.cartridges.model_create(payload).await
}

/// Мутация: требует `caller` с правом `MutateCartridges`.
pub async fn build_cartridge_models_update(
    ctx: &AppCtx,
    caller: &Identity,
    payload: CartridgeModelPatchDto,
) -> Result<CartridgeModelDto, AppError> {
    authorize(caller, &Action::MutateCartridges)?;
    ctx.cartridges.model_update(payload).await
}

/// Мутация: требует `caller` с правом `MutateCartridges`.
pub async fn build_cartridge_models_delete(
    ctx: &AppCtx,
    caller: &Identity,
    id: i64,
    version: i64,
) -> Result<(), AppError> {
    authorize(caller, &Action::MutateCartridges)?;
    ctx.cartridges.model_delete(id, version).await
}

pub async fn build_cartridges_suggest_brand(
    ctx: &AppCtx,
    caller: &Identity,
    prefix: String,
) -> Result<Vec<String>, AppError> {
    authorize(caller, &Action::ReadData)?;
    ctx.cartridges.suggest_brand(prefix).await
}

pub async fn build_cartridges_suggest_model(
    ctx: &AppCtx,
    caller: &Identity,
    brand: String,
    prefix: String,
) -> Result<Vec<String>, AppError> {
    authorize(caller, &Action::ReadData)?;
    ctx.cartridges.suggest_model(brand, prefix).await
}

pub async fn build_cartridges_suggest_compat_printer(
    ctx: &AppCtx,
    caller: &Identity,
    prefix: String,
) -> Result<Vec<String>, AppError> {
    authorize(caller, &Action::ReadData)?;
    ctx.cartridges.suggest_compat_printer(prefix).await
}

/// D-11.4 storage-place ids (B4) — reachable by whatever role can already
/// perform cartridge transitions, same as `full_path` already being visible
/// on every cartridge read without a separate places-authorization check
/// (Task 3's rationale — `storage_place_ids` is not `ReadPlaces`-gated).
pub async fn build_cartridge_storage_place_ids(
    ctx: &AppCtx,
    caller: &Identity,
) -> Result<Vec<i64>, AppError> {
    authorize(caller, &Action::ReadData)?;
    ctx.cartridges.storage_place_ids().await
}

// ---------------------------------------------------------------------------
// Thin Tauri wrappers
// ---------------------------------------------------------------------------

#[tauri::command]
#[specta::specta]
pub async fn cartridges_list(
    state: tauri::State<'_, AppCtx>,
    filter: CartridgeFilter,
    pagination: Pagination,
) -> Result<CartridgeListResponse, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_cartridges_list(state.inner(), &caller, filter, pagination).await
}

#[tauri::command]
#[specta::specta]
pub async fn cartridges_get(
    state: tauri::State<'_, AppCtx>,
    id: i32,
) -> Result<CartridgeDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_cartridges_get(state.inner(), &caller, id as i64).await
}

#[tauri::command]
#[specta::specta]
pub async fn cartridges_create(
    state: tauri::State<'_, AppCtx>,
    payload: CartridgeCreateDto,
) -> Result<CartridgeDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_cartridges_create(state.inner(), &caller, payload).await
}

#[tauri::command]
#[specta::specta]
pub async fn cartridges_update(
    state: tauri::State<'_, AppCtx>,
    id: i32,
    version: i32,
    place_id: Option<i32>,
    notes: Option<String>,
) -> Result<CartridgeDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_cartridges_update(
        state.inner(),
        &caller,
        id as i64,
        version as i64,
        place_id.map(|v| v as i64),
        notes,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn cartridges_delete(
    state: tauri::State<'_, AppCtx>,
    id: i32,
    version: i32,
) -> Result<(), AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_cartridges_delete(state.inner(), &caller, id as i64, version as i64).await
}

#[tauri::command]
#[specta::specta]
pub async fn cartridges_transition(
    state: tauri::State<'_, AppCtx>,
    payload: CartridgeTransitionPayload,
) -> Result<CartridgeDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_cartridges_transition(state.inner(), &caller, payload).await
}

#[tauri::command]
#[specta::specta]
pub async fn cartridges_search(
    state: tauri::State<'_, AppCtx>,
    query: String,
    filter: CartridgeFilter,
) -> Result<CartridgeListResponse, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_cartridges_search(state.inner(), &caller, query, filter).await
}

#[tauri::command]
#[specta::specta]
pub async fn cartridges_status_counts(
    state: tauri::State<'_, AppCtx>,
) -> Result<CartridgeCountsDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_cartridges_status_counts(state.inner(), &caller).await
}

#[tauri::command]
#[specta::specta]
pub async fn cartridges_get_history(
    state: tauri::State<'_, AppCtx>,
    id: i32,
) -> Result<Vec<AuditEntryDto>, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_cartridges_get_history(state.inner(), &caller, id as i64).await
}

#[tauri::command]
#[specta::specta]
pub async fn cartridges_low_stock(
    state: tauri::State<'_, AppCtx>,
) -> Result<Vec<LowStockItemDto>, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_cartridges_low_stock(state.inner(), &caller).await
}

#[tauri::command]
#[specta::specta]
pub async fn cartridge_models_list(
    state: tauri::State<'_, AppCtx>,
) -> Result<Vec<CartridgeModelDto>, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_cartridge_models_list(state.inner(), &caller).await
}

#[tauri::command]
#[specta::specta]
pub async fn cartridge_models_get(
    state: tauri::State<'_, AppCtx>,
    id: i32,
) -> Result<CartridgeModelDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_cartridge_models_get(state.inner(), &caller, id as i64).await
}

#[tauri::command]
#[specta::specta]
pub async fn cartridge_models_create(
    state: tauri::State<'_, AppCtx>,
    payload: CartridgeModelCreateDto,
) -> Result<CartridgeModelDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_cartridge_models_create(state.inner(), &caller, payload).await
}

#[tauri::command]
#[specta::specta]
pub async fn cartridge_models_update(
    state: tauri::State<'_, AppCtx>,
    payload: CartridgeModelPatchDto,
) -> Result<CartridgeModelDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_cartridge_models_update(state.inner(), &caller, payload).await
}

#[tauri::command]
#[specta::specta]
pub async fn cartridge_models_delete(
    state: tauri::State<'_, AppCtx>,
    id: i32,
    version: i32,
) -> Result<(), AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_cartridge_models_delete(state.inner(), &caller, id as i64, version as i64).await
}

#[tauri::command]
#[specta::specta]
pub async fn cartridges_suggest_brand(
    state: tauri::State<'_, AppCtx>,
    prefix: String,
) -> Result<Vec<String>, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_cartridges_suggest_brand(state.inner(), &caller, prefix).await
}

#[tauri::command]
#[specta::specta]
pub async fn cartridges_suggest_model(
    state: tauri::State<'_, AppCtx>,
    brand: String,
    prefix: String,
) -> Result<Vec<String>, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_cartridges_suggest_model(state.inner(), &caller, brand, prefix).await
}

#[tauri::command]
#[specta::specta]
pub async fn cartridges_suggest_compat_printer(
    state: tauri::State<'_, AppCtx>,
    prefix: String,
) -> Result<Vec<String>, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_cartridges_suggest_compat_printer(state.inner(), &caller, prefix).await
}

#[tauri::command]
#[specta::specta]
pub async fn cartridge_storage_place_ids(
    state: tauri::State<'_, AppCtx>,
) -> Result<Vec<i32>, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    let ids = build_cartridge_storage_place_ids(state.inner(), &caller).await?;
    Ok(ids.into_iter().map(|v| v as i32).collect())
}

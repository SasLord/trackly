//! Acts Tauri commands — Plan 02 vertical slice.
//!
//! Pattern (S-1): `build_*` helper + thin `#[tauri::command] #[specta::specta]`
//! wrapper. Both transports (Tauri invoke + axum POST) delegate to the same
//! helper.
//!
//! The `#[specta::specta]` attribute MUST appear AFTER `#[tauri::command]`
//! — required by tauri-specta v2 rc.21.
//!
//! Phase 5 Plan 04: мутации (create, return, delete, render_pdf, render_acceptance_pdf)
//! требуют `caller: &Identity` с правом `MutateActs`. Tauri wrappers resolve identity
//! через `resolve_tauri_identity` (D-Desktop-01/02).

use crate::context::AppCtx;
use crate::dto::act::{
    ActCreateDto, ActDto, ActFilter, ActListResponse, ActReturnDto, ActsCountsDto, Pagination,
};
use crate::dto::suggest::SuggestPersonField;
use crate::tauri_cmds::users::resolve_tauri_identity;
use trackly_core::auth::{authorize, Action, Identity};
use trackly_core::error::AppError;

// ---------------------------------------------------------------------------
// build_* helpers (shared with axum handlers)
// ---------------------------------------------------------------------------

pub async fn build_acts_list(
    ctx: &AppCtx,
    caller: &Identity,
    filter: ActFilter,
    pagination: Pagination,
) -> Result<ActListResponse, AppError> {
    authorize(caller, &Action::ReadData)?;
    ctx.acts.list(filter, pagination).await
}

pub async fn build_acts_search(
    ctx: &AppCtx,
    caller: &Identity,
    query: String,
    filter: ActFilter,
    pagination: Pagination,
) -> Result<ActListResponse, AppError> {
    authorize(caller, &Action::ReadData)?;
    ctx.acts.search(query, filter, pagination).await
}

pub async fn build_acts_get(ctx: &AppCtx, caller: &Identity, id: i64) -> Result<ActDto, AppError> {
    authorize(caller, &Action::ReadData)?;
    ctx.acts.get(id).await
}

/// Мутация: требует `caller` с правом `MutateActs` (Admin | Manager).
pub async fn build_acts_create(
    ctx: &AppCtx,
    caller: &Identity,
    payload: ActCreateDto,
) -> Result<ActDto, AppError> {
    authorize(caller, &Action::MutateActs)?;
    ctx.acts.create(payload).await
}

/// Мутация: требует `caller` с правом `MutateActs`.
pub async fn build_acts_return(
    ctx: &AppCtx,
    caller: &Identity,
    act_id: i64,
    payload: ActReturnDto,
) -> Result<ActDto, AppError> {
    authorize(caller, &Action::MutateActs)?;
    ctx.acts.do_return(act_id, payload).await
}

/// Мутация: требует `caller` с правом `MutateActs`.
pub async fn build_acts_delete(
    ctx: &AppCtx,
    caller: &Identity,
    id: i64,
    version: i64,
) -> Result<(), AppError> {
    authorize(caller, &Action::MutateActs)?;
    ctx.acts.delete_soft(id, version).await
}

pub async fn build_acts_counts(ctx: &AppCtx, caller: &Identity) -> Result<ActsCountsDto, AppError> {
    authorize(caller, &Action::ReadData)?;
    ctx.acts.counts().await
}

pub async fn build_acts_peek_next_number(ctx: &AppCtx, caller: &Identity) -> Result<i64, AppError> {
    authorize(caller, &Action::ReadData)?;
    ctx.acts.peek_next_number().await
}

/// Мутация (PDF generation tied to act): требует `caller` с правом `MutateActs`.
///
/// Phase 16 (D-09/D-10): возвращает HTML-строку, не PDF bytes —
/// `ActService::render_pdf` возвращает `Result<String, AppError>` (Plan 16-02);
/// печать выполняется через диалог браузера (`srcdoc` iframe + `print()`),
/// system-viewer-опенер (`acts_open_pdf_in_system`) удалён (Plan 16-03).
pub async fn build_acts_render_pdf(
    ctx: &AppCtx,
    caller: &Identity,
    act_id: i64,
) -> Result<String, AppError> {
    authorize(caller, &Action::MutateActs)?;
    ctx.acts.render_pdf(act_id).await
}

pub async fn build_acts_suggest_person(
    ctx: &AppCtx,
    caller: &Identity,
    field: SuggestPersonField,
    prefix: String,
) -> Result<Vec<String>, AppError> {
    authorize(caller, &Action::ReadData)?;
    ctx.acts.suggest_person(field, &prefix, 20).await
}

/// Мутация (acceptance PDF): требует `caller` с правом `MutateActs`.
///
/// Phase 16 (D-10): возвращает HTML-строку — см. комментарий у `build_acts_render_pdf`.
pub async fn build_devices_render_acceptance_pdf(
    ctx: &AppCtx,
    caller: &Identity,
    device_id: i64,
    giver_name: String,
    receiver_name: String,
    date_utc: i64,
) -> Result<String, AppError> {
    authorize(caller, &Action::MutateActs)?;
    ctx.acts
        .render_acceptance_pdf(device_id, giver_name, receiver_name, date_utc)
        .await
}

// ---------------------------------------------------------------------------
// Thin Tauri wrappers
// ---------------------------------------------------------------------------

#[tauri::command]
#[specta::specta]
pub async fn acts_list(
    state: tauri::State<'_, AppCtx>,
    filter: ActFilter,
    pagination: Pagination,
) -> Result<ActListResponse, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_acts_list(state.inner(), &caller, filter, pagination).await
}

#[tauri::command]
#[specta::specta]
pub async fn acts_search(
    state: tauri::State<'_, AppCtx>,
    query: String,
    filter: ActFilter,
    pagination: Pagination,
) -> Result<ActListResponse, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_acts_search(state.inner(), &caller, query, filter, pagination).await
}

#[tauri::command]
#[specta::specta]
pub async fn acts_get(state: tauri::State<'_, AppCtx>, id: i32) -> Result<ActDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_acts_get(state.inner(), &caller, id as i64).await
}

#[tauri::command]
#[specta::specta]
pub async fn acts_create(
    state: tauri::State<'_, AppCtx>,
    payload: ActCreateDto,
) -> Result<ActDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_acts_create(state.inner(), &caller, payload).await
}

#[tauri::command]
#[specta::specta]
pub async fn acts_return(
    state: tauri::State<'_, AppCtx>,
    act_id: i32,
    payload: ActReturnDto,
) -> Result<ActDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_acts_return(state.inner(), &caller, act_id as i64, payload).await
}

#[tauri::command]
#[specta::specta]
pub async fn acts_delete(
    state: tauri::State<'_, AppCtx>,
    id: i32,
    version: i32,
) -> Result<(), AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_acts_delete(state.inner(), &caller, id as i64, version as i64).await
}

#[tauri::command]
#[specta::specta]
pub async fn acts_counts(state: tauri::State<'_, AppCtx>) -> Result<ActsCountsDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_acts_counts(state.inner(), &caller).await
}

#[tauri::command]
#[specta::specta]
pub async fn acts_peek_next_number(state: tauri::State<'_, AppCtx>) -> Result<i32, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    let next = build_acts_peek_next_number(state.inner(), &caller).await?;
    Ok(next as i32)
}

#[tauri::command]
#[specta::specta]
pub async fn acts_render_pdf(
    state: tauri::State<'_, AppCtx>,
    act_id: i32,
) -> Result<String, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_acts_render_pdf(state.inner(), &caller, act_id as i64).await
}

#[tauri::command]
#[specta::specta]
pub async fn acts_suggest_person(
    state: tauri::State<'_, AppCtx>,
    field: SuggestPersonField,
    prefix: String,
) -> Result<Vec<String>, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_acts_suggest_person(state.inner(), &caller, field, prefix).await
}

#[tauri::command]
#[specta::specta]
pub async fn devices_render_acceptance_pdf(
    state: tauri::State<'_, AppCtx>,
    device_id: i32,
    giver_name: String,
    receiver_name: String,
    date_utc: i32,
) -> Result<String, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_devices_render_acceptance_pdf(
        state.inner(),
        &caller,
        device_id as i64,
        giver_name,
        receiver_name,
        date_utc as i64,
    )
    .await
}

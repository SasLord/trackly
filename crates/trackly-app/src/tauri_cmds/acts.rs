//! Acts Tauri commands — Plan 02 vertical slice.
//!
//! Pattern (S-1): `build_*` helper + thin `#[tauri::command] #[specta::specta]`
//! wrapper. Both transports (Tauri invoke + axum POST) delegate to the same
//! helper.
//!
//! The `#[specta::specta]` attribute MUST appear AFTER `#[tauri::command]`
//! — required by tauri-specta v2 rc.21.

use crate::context::AppCtx;
use crate::dto::act::{
    ActCreateDto, ActDto, ActFilter, ActListResponse, ActReturnDto, ActsCountsDto, Pagination,
};
use trackly_core::error::AppError;

// ---------------------------------------------------------------------------
// build_* helpers (shared with axum handlers)
// ---------------------------------------------------------------------------

pub async fn build_acts_list(
    ctx: &AppCtx,
    filter: ActFilter,
    pagination: Pagination,
) -> Result<ActListResponse, AppError> {
    ctx.acts.list(filter, pagination).await
}

pub async fn build_acts_get(ctx: &AppCtx, id: i64) -> Result<ActDto, AppError> {
    ctx.acts.get(id).await
}

pub async fn build_acts_create(ctx: &AppCtx, payload: ActCreateDto) -> Result<ActDto, AppError> {
    ctx.acts.create(payload).await
}

pub async fn build_acts_return(
    ctx: &AppCtx,
    act_id: i64,
    payload: ActReturnDto,
) -> Result<ActDto, AppError> {
    ctx.acts.do_return(act_id, payload).await
}

pub async fn build_acts_delete(ctx: &AppCtx, id: i64, version: i64) -> Result<(), AppError> {
    ctx.acts.delete_soft(id, version).await
}

pub async fn build_acts_counts(ctx: &AppCtx) -> Result<ActsCountsDto, AppError> {
    ctx.acts.counts().await
}

pub async fn build_acts_peek_next_number(ctx: &AppCtx) -> Result<i64, AppError> {
    ctx.acts.peek_next_number().await
}

pub async fn build_acts_render_pdf(ctx: &AppCtx, act_id: i64) -> Result<Vec<u8>, AppError> {
    ctx.acts.render_pdf(act_id).await
}

pub async fn build_devices_render_acceptance_pdf(
    ctx: &AppCtx,
    device_id: i64,
    giver_name: String,
    receiver_name: String,
    date_utc: i64,
) -> Result<Vec<u8>, AppError> {
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
    build_acts_list(state.inner(), filter, pagination).await
}

#[tauri::command]
#[specta::specta]
pub async fn acts_get(state: tauri::State<'_, AppCtx>, id: i32) -> Result<ActDto, AppError> {
    build_acts_get(state.inner(), id as i64).await
}

#[tauri::command]
#[specta::specta]
pub async fn acts_create(
    state: tauri::State<'_, AppCtx>,
    payload: ActCreateDto,
) -> Result<ActDto, AppError> {
    build_acts_create(state.inner(), payload).await
}

#[tauri::command]
#[specta::specta]
pub async fn acts_return(
    state: tauri::State<'_, AppCtx>,
    act_id: i32,
    payload: ActReturnDto,
) -> Result<ActDto, AppError> {
    build_acts_return(state.inner(), act_id as i64, payload).await
}

#[tauri::command]
#[specta::specta]
pub async fn acts_delete(
    state: tauri::State<'_, AppCtx>,
    id: i32,
    version: i32,
) -> Result<(), AppError> {
    build_acts_delete(state.inner(), id as i64, version as i64).await
}

#[tauri::command]
#[specta::specta]
pub async fn acts_counts(state: tauri::State<'_, AppCtx>) -> Result<ActsCountsDto, AppError> {
    build_acts_counts(state.inner()).await
}

#[tauri::command]
#[specta::specta]
pub async fn acts_peek_next_number(state: tauri::State<'_, AppCtx>) -> Result<i32, AppError> {
    let next = build_acts_peek_next_number(state.inner()).await?;
    Ok(next as i32)
}

#[tauri::command]
#[specta::specta]
pub async fn acts_render_pdf(
    state: tauri::State<'_, AppCtx>,
    act_id: i32,
) -> Result<Vec<u8>, AppError> {
    build_acts_render_pdf(state.inner(), act_id as i64).await
}

#[tauri::command]
#[specta::specta]
pub async fn devices_render_acceptance_pdf(
    state: tauri::State<'_, AppCtx>,
    device_id: i32,
    giver_name: String,
    receiver_name: String,
    date_utc: i32,
) -> Result<Vec<u8>, AppError> {
    build_devices_render_acceptance_pdf(
        state.inner(),
        device_id as i64,
        giver_name,
        receiver_name,
        date_utc as i64,
    )
    .await
}

//! Device Tauri commands — Plan 03 CRUD.
//!
//! Паттерн: `build_*` helper + thin `#[tauri::command] #[specta::specta]` wrapper.
//! Оба транспорта делегируют одному и тому же `build_*` функции.
//!
//! `#[specta::specta]` ПОСЛЕ `#[tauri::command]` — требование tauri-specta v2 rc.21.

use crate::context::AppCtx;
use crate::dto::device::{
    DeviceDto, DeviceFilter, DeviceListResponse, DeviceNew, DevicePatch, Pagination,
};
use trackly_core::error::AppError;

// ---------------------------------------------------------------------------
// build_* helpers — используются и Tauri, и axum транспортами
// ---------------------------------------------------------------------------

pub async fn build_devices_list(
    ctx: &AppCtx,
    filter: DeviceFilter,
    pagination: Pagination,
) -> Result<DeviceListResponse, AppError> {
    ctx.devices.list(filter, pagination).await
}

pub async fn build_devices_get(ctx: &AppCtx, id: i64) -> Result<DeviceDto, AppError> {
    ctx.devices.get(id).await
}

pub async fn build_devices_create(ctx: &AppCtx, new: DeviceNew) -> Result<DeviceDto, AppError> {
    ctx.devices.create(new).await
}

pub async fn build_devices_update(
    ctx: &AppCtx,
    id: i64,
    version: i64,
    patch: DevicePatch,
) -> Result<DeviceDto, AppError> {
    ctx.devices.update(id, version, patch).await
}

pub async fn build_devices_delete(ctx: &AppCtx, id: i64, version: i64) -> Result<(), AppError> {
    ctx.devices.delete_soft(id, version).await
}

pub async fn build_devices_state_hints(ctx: &AppCtx) -> Result<Vec<String>, AppError> {
    Ok(ctx.devices.state_hints())
}

// ---------------------------------------------------------------------------
// Tauri commands — thin wrappers
// ---------------------------------------------------------------------------

#[tauri::command]
#[specta::specta]
pub async fn devices_list(
    state: tauri::State<'_, AppCtx>,
    filter: DeviceFilter,
    pagination: Pagination,
) -> Result<DeviceListResponse, AppError> {
    build_devices_list(state.inner(), filter, pagination).await
}

#[tauri::command]
#[specta::specta]
pub async fn devices_get(state: tauri::State<'_, AppCtx>, id: i32) -> Result<DeviceDto, AppError> {
    build_devices_get(state.inner(), id as i64).await
}

#[tauri::command]
#[specta::specta]
pub async fn devices_create(
    state: tauri::State<'_, AppCtx>,
    new: DeviceNew,
) -> Result<DeviceDto, AppError> {
    build_devices_create(state.inner(), new).await
}

#[tauri::command]
#[specta::specta]
pub async fn devices_update(
    state: tauri::State<'_, AppCtx>,
    id: i32,
    version: i32,
    patch: DevicePatch,
) -> Result<DeviceDto, AppError> {
    build_devices_update(state.inner(), id as i64, version as i64, patch).await
}

#[tauri::command]
#[specta::specta]
pub async fn devices_delete(
    state: tauri::State<'_, AppCtx>,
    id: i32,
    version: i32,
) -> Result<(), AppError> {
    build_devices_delete(state.inner(), id as i64, version as i64).await
}

#[tauri::command]
#[specta::specta]
pub async fn devices_state_hints(
    state: tauri::State<'_, AppCtx>,
) -> Result<Vec<String>, AppError> {
    build_devices_state_hints(state.inner()).await
}

//! Device Tauri commands — Plan 03 CRUD + Plan 04 Search/Autocomplete/Grouping.
//!
//! Паттерн: `build_*` helper + thin `#[tauri::command] #[specta::specta]` wrapper.
//! Оба транспорта делегируют одному и тому же `build_*` функции.
//!
//! `#[specta::specta]` ПОСЛЕ `#[tauri::command]` — требование tauri-specta v2 rc.21.

use crate::context::AppCtx;
use std::collections::HashMap;

use crate::dto::device::{
    CsvImportPreviewResponse, CsvImportReport, DeviceDto, DeviceFilter, DeviceGroup,
    DeviceListResponse, DeviceNew, DevicePatch, Pagination, StatusCount,
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

pub async fn build_devices_search(
    ctx: &AppCtx,
    query: String,
    pagination: Pagination,
) -> Result<DeviceListResponse, AppError> {
    ctx.devices.search(query, pagination).await
}

pub async fn build_locations_autocomplete(
    ctx: &AppCtx,
    prefix: String,
) -> Result<Vec<String>, AppError> {
    ctx.devices.locations_autocomplete(prefix).await
}

pub async fn build_devices_autocomplete(
    ctx: &AppCtx,
    field: String,
    prefix: String,
    ctx_name: Option<String>,
    ctx_status_id: Option<i64>,
    status_in: Option<Vec<String>>,
) -> Result<Vec<String>, AppError> {
    ctx.devices
        .autocomplete(field, prefix, ctx_name, ctx_status_id, status_in)
        .await
}

pub async fn build_devices_list_grouped(
    ctx: &AppCtx,
    filter: DeviceFilter,
    pagination: Pagination,
) -> Result<Vec<DeviceGroup>, AppError> {
    ctx.devices.list_grouped(filter, pagination).await
}

pub async fn build_devices_status_counts(ctx: &AppCtx) -> Result<Vec<StatusCount>, AppError> {
    ctx.devices.status_counts().await
}

pub async fn build_devices_list_by_ids(
    ctx: &AppCtx,
    ids: Vec<i64>,
) -> Result<Vec<DeviceDto>, AppError> {
    ctx.devices.list_by_ids(ids).await
}

pub async fn build_devices_bulk_create(
    ctx: &AppCtx,
    device: DeviceNew,
    count: u32,
) -> Result<Vec<DeviceDto>, AppError> {
    ctx.devices.bulk_create(device, count).await
}

// ---------------------------------------------------------------------------
// Tauri commands — thin wrappers (Plan 03)
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
    device: DeviceNew,
) -> Result<DeviceDto, AppError> {
    build_devices_create(state.inner(), device).await
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
pub async fn devices_state_hints(state: tauri::State<'_, AppCtx>) -> Result<Vec<String>, AppError> {
    build_devices_state_hints(state.inner()).await
}

// ---------------------------------------------------------------------------
// Tauri commands — Plan 04: Search / Autocomplete / Grouping
// ---------------------------------------------------------------------------

#[tauri::command]
#[specta::specta]
pub async fn devices_search(
    state: tauri::State<'_, AppCtx>,
    query: String,
    pagination: Pagination,
) -> Result<DeviceListResponse, AppError> {
    build_devices_search(state.inner(), query, pagination).await
}

#[tauri::command]
#[specta::specta]
pub async fn locations_autocomplete(
    state: tauri::State<'_, AppCtx>,
    prefix: String,
) -> Result<Vec<String>, AppError> {
    build_locations_autocomplete(state.inner(), prefix).await
}

#[tauri::command]
#[specta::specta]
pub async fn devices_autocomplete(
    state: tauri::State<'_, AppCtx>,
    field: String,
    prefix: String,
    ctx_name: Option<String>,
    ctx_status_id: Option<i32>,
    status_in: Option<Vec<String>>,
) -> Result<Vec<String>, AppError> {
    build_devices_autocomplete(
        state.inner(),
        field,
        prefix,
        ctx_name,
        ctx_status_id.map(|id| id as i64),
        status_in,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn devices_list_grouped(
    state: tauri::State<'_, AppCtx>,
    filter: DeviceFilter,
    pagination: Pagination,
) -> Result<Vec<DeviceGroup>, AppError> {
    build_devices_list_grouped(state.inner(), filter, pagination).await
}

#[tauri::command]
#[specta::specta]
pub async fn devices_status_counts(
    state: tauri::State<'_, AppCtx>,
) -> Result<Vec<StatusCount>, AppError> {
    build_devices_status_counts(state.inner()).await
}

#[tauri::command]
#[specta::specta]
pub async fn devices_list_by_ids(
    state: tauri::State<'_, AppCtx>,
    ids: Vec<i32>,
) -> Result<Vec<DeviceDto>, AppError> {
    build_devices_list_by_ids(state.inner(), ids.into_iter().map(|id| id as i64).collect()).await
}

// ---------------------------------------------------------------------------
// Tauri commands — scope extension: bulk create
// ---------------------------------------------------------------------------

#[tauri::command]
#[specta::specta]
pub async fn devices_bulk_create(
    state: tauri::State<'_, AppCtx>,
    device: DeviceNew,
    count: u32,
) -> Result<Vec<DeviceDto>, AppError> {
    build_devices_bulk_create(state.inner(), device, count).await
}

// ---------------------------------------------------------------------------
// build_* helpers — CSV import / export (Plan 05)
// ---------------------------------------------------------------------------

pub async fn build_devices_import_csv_preview(
    ctx: &AppCtx,
    bytes: Vec<u8>,
) -> Result<CsvImportPreviewResponse, AppError> {
    ctx.devices.import_csv_preview(bytes).await
}

pub async fn build_devices_import_csv_commit(
    ctx: &AppCtx,
    token: String,
    mapping: HashMap<String, String>,
) -> Result<CsvImportReport, AppError> {
    ctx.devices.import_csv_commit(token, mapping).await
}

pub async fn build_devices_export_csv(
    ctx: &AppCtx,
    filter: DeviceFilter,
) -> Result<String, AppError> {
    ctx.devices.export_csv(filter).await
}

// ---------------------------------------------------------------------------
// Tauri commands — CSV import / export (Plan 05)
// ---------------------------------------------------------------------------

#[tauri::command]
#[specta::specta]
pub async fn devices_import_csv_preview(
    state: tauri::State<'_, AppCtx>,
    bytes: Vec<u8>,
) -> Result<CsvImportPreviewResponse, AppError> {
    build_devices_import_csv_preview(state.inner(), bytes).await
}

#[tauri::command]
#[specta::specta]
pub async fn devices_import_csv_commit(
    state: tauri::State<'_, AppCtx>,
    token: String,
    mapping: HashMap<String, String>,
) -> Result<CsvImportReport, AppError> {
    build_devices_import_csv_commit(state.inner(), token, mapping).await
}

#[tauri::command]
#[specta::specta]
pub async fn devices_export_csv(
    state: tauri::State<'_, AppCtx>,
    filter: DeviceFilter,
) -> Result<String, AppError> {
    build_devices_export_csv(state.inner(), filter).await
}

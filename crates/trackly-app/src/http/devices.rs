//! Device axum HTTP routes — Plan 03 CRUD + Plan 04 Search/Autocomplete/Grouping.
//!
//! Все routes — POST /api/v1/devices_* (аналогично Tauri command names).
//! Handlers — thin adapters, делегируют `build_*` helpers из tauri_cmds.
//!
//! Паттерн из PATTERNS.md §Pattern 1.

use axum::{extract::State, routing::post, Json, Router};

use crate::context::AppCtx;
use std::collections::HashMap;

use crate::dto::device::{
    CsvImportPreviewResponse, CsvImportReport, DeviceDto, DeviceFilter, DeviceGroup,
    DeviceListResponse, DeviceNew, DevicePatch, Pagination, StatusCount,
};
use crate::error_axum::AppErrorResponse;
use crate::tauri_cmds::devices::{
    build_devices_autocomplete, build_devices_bulk_create, build_devices_create,
    build_devices_delete, build_devices_export_csv, build_devices_get,
    build_devices_import_csv_commit, build_devices_import_csv_preview, build_devices_list,
    build_devices_list_by_ids, build_devices_list_grouped, build_devices_search,
    build_devices_state_hints, build_devices_status_counts, build_devices_update,
    build_locations_autocomplete,
};

// ---------------------------------------------------------------------------
// Payload structs для HTTP routes
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize)]
pub struct ListPayload {
    pub filter: DeviceFilter,
    pub pagination: Pagination,
}

#[derive(serde::Deserialize)]
pub struct GetPayload {
    pub id: i64,
}

#[derive(serde::Deserialize)]
pub struct CreatePayload {
    pub device: DeviceNew,
}

#[derive(serde::Deserialize)]
pub struct UpdatePayload {
    pub id: i64,
    pub version: i64,
    pub patch: DevicePatch,
}

#[derive(serde::Deserialize)]
pub struct DeletePayload {
    pub id: i64,
    pub version: i64,
}

#[derive(serde::Deserialize)]
pub struct SearchPayload {
    pub query: String,
    pub pagination: Pagination,
}

#[derive(serde::Deserialize)]
pub struct AutocompletePayload {
    pub field: String,
    pub prefix: String,
    pub ctx_name: Option<String>,
    pub ctx_status_id: Option<i64>,
    pub status_in: Option<Vec<String>>,
}

#[derive(serde::Deserialize)]
pub struct ListGroupedPayload {
    pub filter: DeviceFilter,
    pub pagination: Pagination,
}

#[derive(serde::Deserialize)]
pub struct ListByIdsPayload {
    pub ids: Vec<i64>,
}

#[derive(serde::Deserialize)]
pub struct BulkCreatePayload {
    pub device: DeviceNew,
    pub count: u32,
}

// CSV import / export payloads (Plan 05)

#[derive(serde::Deserialize)]
pub struct ImportCsvPreviewPayload {
    pub bytes: Vec<u8>,
}

#[derive(serde::Deserialize)]
pub struct ImportCsvCommitPayload {
    pub token: String,
    pub mapping: HashMap<String, String>,
}

#[derive(serde::Deserialize)]
pub struct ExportCsvPayload {
    pub filter: DeviceFilter,
}

#[derive(serde::Deserialize)]
pub struct LocationsAutocompletePayload {
    pub prefix: String,
}

// ---------------------------------------------------------------------------
// Handlers (Plan 03)
// ---------------------------------------------------------------------------

pub async fn handler_list(
    State(ctx): State<AppCtx>,
    Json(payload): Json<ListPayload>,
) -> Result<Json<DeviceListResponse>, AppErrorResponse> {
    Ok(Json(
        build_devices_list(&ctx, payload.filter, payload.pagination)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_get(
    State(ctx): State<AppCtx>,
    Json(payload): Json<GetPayload>,
) -> Result<Json<DeviceDto>, AppErrorResponse> {
    Ok(Json(
        build_devices_get(&ctx, payload.id)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_create(
    State(ctx): State<AppCtx>,
    Json(payload): Json<CreatePayload>,
) -> Result<Json<DeviceDto>, AppErrorResponse> {
    Ok(Json(
        build_devices_create(&ctx, payload.device)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_update(
    State(ctx): State<AppCtx>,
    Json(payload): Json<UpdatePayload>,
) -> Result<Json<DeviceDto>, AppErrorResponse> {
    Ok(Json(
        build_devices_update(&ctx, payload.id, payload.version, payload.patch)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_delete(
    State(ctx): State<AppCtx>,
    Json(payload): Json<DeletePayload>,
) -> Result<Json<()>, AppErrorResponse> {
    build_devices_delete(&ctx, payload.id, payload.version)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(()))
}

pub async fn handler_state_hints(
    State(ctx): State<AppCtx>,
) -> Result<Json<Vec<String>>, AppErrorResponse> {
    Ok(Json(
        build_devices_state_hints(&ctx)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

// ---------------------------------------------------------------------------
// Handlers (Plan 04)
// ---------------------------------------------------------------------------

pub async fn handler_search(
    State(ctx): State<AppCtx>,
    Json(payload): Json<SearchPayload>,
) -> Result<Json<DeviceListResponse>, AppErrorResponse> {
    Ok(Json(
        build_devices_search(&ctx, payload.query, payload.pagination)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_autocomplete(
    State(ctx): State<AppCtx>,
    Json(payload): Json<AutocompletePayload>,
) -> Result<Json<Vec<String>>, AppErrorResponse> {
    Ok(Json(
        build_devices_autocomplete(
            &ctx,
            payload.field,
            payload.prefix,
            payload.ctx_name,
            payload.ctx_status_id,
            payload.status_in,
        )
        .await
        .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_list_grouped(
    State(ctx): State<AppCtx>,
    Json(payload): Json<ListGroupedPayload>,
) -> Result<Json<Vec<DeviceGroup>>, AppErrorResponse> {
    Ok(Json(
        build_devices_list_grouped(&ctx, payload.filter, payload.pagination)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_status_counts(
    State(ctx): State<AppCtx>,
) -> Result<Json<Vec<StatusCount>>, AppErrorResponse> {
    Ok(Json(
        build_devices_status_counts(&ctx)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_list_by_ids(
    State(ctx): State<AppCtx>,
    Json(payload): Json<ListByIdsPayload>,
) -> Result<Json<Vec<DeviceDto>>, AppErrorResponse> {
    Ok(Json(
        build_devices_list_by_ids(&ctx, payload.ids)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_bulk_create(
    State(ctx): State<AppCtx>,
    Json(payload): Json<BulkCreatePayload>,
) -> Result<Json<Vec<DeviceDto>>, AppErrorResponse> {
    Ok(Json(
        build_devices_bulk_create(&ctx, payload.device, payload.count)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

// ---------------------------------------------------------------------------
// Handlers (Plan 05) — CSV import / export
// ---------------------------------------------------------------------------

pub async fn handler_import_csv_preview(
    State(ctx): State<AppCtx>,
    Json(payload): Json<ImportCsvPreviewPayload>,
) -> Result<Json<CsvImportPreviewResponse>, AppErrorResponse> {
    Ok(Json(
        build_devices_import_csv_preview(&ctx, payload.bytes)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_import_csv_commit(
    State(ctx): State<AppCtx>,
    Json(payload): Json<ImportCsvCommitPayload>,
) -> Result<Json<CsvImportReport>, AppErrorResponse> {
    Ok(Json(
        build_devices_import_csv_commit(&ctx, payload.token, payload.mapping)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_export_csv(
    State(ctx): State<AppCtx>,
    Json(payload): Json<ExportCsvPayload>,
) -> Result<Json<String>, AppErrorResponse> {
    Ok(Json(
        build_devices_export_csv(&ctx, payload.filter)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

// ---------------------------------------------------------------------------
// Handlers (Plan 03.3) — locations autocomplete for browser mode
// ---------------------------------------------------------------------------

pub async fn handler_locations_autocomplete(
    State(ctx): State<AppCtx>,
    Json(payload): Json<LocationsAutocompletePayload>,
) -> Result<Json<Vec<String>>, AppErrorResponse> {
    Ok(Json(
        build_locations_autocomplete(&ctx, payload.prefix)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router() -> Router<AppCtx> {
    Router::new()
        // Plan 03 CRUD
        .route("/api/v1/devices_list", post(handler_list))
        .route("/api/v1/devices_get", post(handler_get))
        .route("/api/v1/devices_create", post(handler_create))
        .route("/api/v1/devices_update", post(handler_update))
        .route("/api/v1/devices_delete", post(handler_delete))
        .route("/api/v1/devices_state_hints", post(handler_state_hints))
        // Plan 04 Search/Autocomplete/Grouping
        .route("/api/v1/devices_search", post(handler_search))
        .route("/api/v1/devices_autocomplete", post(handler_autocomplete))
        .route("/api/v1/devices_list_grouped", post(handler_list_grouped))
        .route("/api/v1/devices_status_counts", post(handler_status_counts))
        .route("/api/v1/devices_list_by_ids", post(handler_list_by_ids))
        // Plan 03.3: locations autocomplete for browser mode (ITEM-4)
        .route(
            "/api/v1/locations_autocomplete",
            post(handler_locations_autocomplete),
        )
        // Scope extension: bulk create
        .route("/api/v1/devices_bulk_create", post(handler_bulk_create))
        // Plan 05: CSV import / export
        .route(
            "/api/v1/devices_import_csv_preview",
            post(handler_import_csv_preview),
        )
        .route(
            "/api/v1/devices_import_csv_commit",
            post(handler_import_csv_commit),
        )
        .route("/api/v1/devices_export_csv", post(handler_export_csv))
}

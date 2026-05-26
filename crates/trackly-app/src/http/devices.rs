//! Device axum HTTP routes — Plan 03.
//!
//! Все routes — POST /api/v1/devices_* (аналогично Tauri command names).
//! Handlers — thin adapters, делегируют `build_*` helpers из tauri_cmds.
//!
//! Паттерн из PATTERNS.md §Pattern 1.

use axum::{extract::State, routing::post, Json, Router};

use crate::context::AppCtx;
use crate::dto::device::{
    DeviceDto, DeviceFilter, DeviceListResponse, DeviceNew, DevicePatch, Pagination,
};
use crate::error_axum::AppErrorResponse;
use crate::tauri_cmds::devices::{
    build_devices_create, build_devices_delete, build_devices_get, build_devices_list,
    build_devices_state_hints, build_devices_update,
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

// ---------------------------------------------------------------------------
// Handlers
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
// Router
// ---------------------------------------------------------------------------

pub fn router() -> Router<AppCtx> {
    Router::new()
        .route("/api/v1/devices_list", post(handler_list))
        .route("/api/v1/devices_get", post(handler_get))
        .route("/api/v1/devices_create", post(handler_create))
        .route("/api/v1/devices_update", post(handler_update))
        .route("/api/v1/devices_delete", post(handler_delete))
        .route("/api/v1/devices_state_hints", post(handler_state_hints))
}

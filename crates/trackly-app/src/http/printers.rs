//! Printers axum HTTP routes — Phase 6 Plan 03.
//!
//! Mirrors `tauri_cmds::printers` via POST endpoints.
//!
//! Pattern (S-2): каждый handler делегирует соответствующему `build_*` helper
//! из tauri_cmds/printers.rs — один DTO, два транспорта.

use axum::{extract::State, routing::post, Json, Router};
use tower_sessions::Session;

use crate::context::AppCtx;
use crate::dto::printer::{
    DiscoveredPrinterDto, Pagination, PrinterCompatibleModelsDto, PrinterCreateDto, PrinterDto,
    PrinterFilter, PrinterListResponse,
};
use crate::error_axum::AppErrorResponse;
use crate::http::auth::session_identity;
use crate::tauri_cmds::printers::{
    build_printers_acknowledge_alert, build_printers_admit, build_printers_create,
    build_printers_discover, build_printers_get, build_printers_get_compatible_models,
    build_printers_list, build_printers_refresh, build_printers_set_compatible_models,
};

// ---------------------------------------------------------------------------
// Payload wrappers (camelCase для совместимости с браузером)
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListPayload {
    pub filter: PrinterFilter,
    pub pagination: Pagination,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPayload {
    pub id: i32,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePayload {
    pub payload: PrinterCreateDto,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverPayload {
    pub ip_start: String,
    pub ip_end: String,
    pub community: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshPayload {
    pub id: i32,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcknowledgeAlertPayload {
    pub printer_id: i32,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdmitPayload {
    pub selected_ips: Vec<String>,
    pub community: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetCompatibleModelsPayload {
    pub device_id: i32,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetCompatibleModelsPayload {
    pub device_id: i32,
    pub model_ids: Vec<i32>,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

pub async fn handler_list(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<ListPayload>,
) -> Result<Json<PrinterListResponse>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_printers_list(&ctx, &identity, p.filter, p.pagination)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_get(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<GetPayload>,
) -> Result<Json<PrinterDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_printers_get(&ctx, &identity, p.id as i64)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_create(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<CreatePayload>,
) -> Result<Json<PrinterDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_printers_create(&ctx, &identity, p.payload)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_discover(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<DiscoverPayload>,
) -> Result<Json<Vec<DiscoveredPrinterDto>>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_printers_discover(&ctx, &identity, p.ip_start, p.ip_end, p.community)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_refresh(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<RefreshPayload>,
) -> Result<Json<PrinterDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_printers_refresh(&ctx, &identity, p.id as i64)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_acknowledge_alert(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<AcknowledgeAlertPayload>,
) -> Result<Json<()>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    build_printers_acknowledge_alert(&ctx, &identity, p.printer_id as i64)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(()))
}

pub async fn handler_admit(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<AdmitPayload>,
) -> Result<Json<Vec<PrinterDto>>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_printers_admit(&ctx, &identity, p.selected_ips, p.community)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_get_compatible_models(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<GetCompatibleModelsPayload>,
) -> Result<Json<PrinterCompatibleModelsDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_printers_get_compatible_models(&ctx, &identity, p.device_id as i64)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_set_compatible_models(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<SetCompatibleModelsPayload>,
) -> Result<Json<PrinterCompatibleModelsDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_printers_set_compatible_models(
            &ctx,
            &identity,
            p.device_id as i64,
            p.model_ids.into_iter().map(|id| id as i64).collect(),
        )
        .await
        .map_err(AppErrorResponse::from)?,
    ))
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router() -> Router<AppCtx> {
    Router::new()
        .route("/api/v1/printers_list", post(handler_list))
        .route("/api/v1/printers_get", post(handler_get))
        .route("/api/v1/printers_create", post(handler_create))
        .route("/api/v1/printers_discover", post(handler_discover))
        .route("/api/v1/printers_admit", post(handler_admit))
        .route("/api/v1/printers_refresh", post(handler_refresh))
        .route(
            "/api/v1/printers_acknowledge_alert",
            post(handler_acknowledge_alert),
        )
        .route(
            "/api/v1/printers_get_compatible_models",
            post(handler_get_compatible_models),
        )
        .route(
            "/api/v1/printers_set_compatible_models",
            post(handler_set_compatible_models),
        )
}

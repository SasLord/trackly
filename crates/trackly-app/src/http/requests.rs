//! Requests axum HTTP routes — Phase 6 Plan 03.
//!
//! Mirrors `tauri_cmds::requests` via POST endpoints.
//!
//! Pattern (S-2): каждый handler делегирует соответствующему `build_*` helper
//! из tauri_cmds/requests.rs — один DTO, два транспорта.
//!
//! WS push: НЕ выполняется в этих handler'ах. Единственный владелец
//! broadcast — слой `RequestService` (create/transition/approve_ad_register
//! сами шлют WsEvent через свой `ws_tx`, который является тем же
//! `Arc<broadcast::Sender>`, что и `ctx.ws_broadcast`). Повторная отправка
//! здесь приводила к двойной доставке события каждому подписчику (CR-01,
//! симптом «WS toast spam») — удалена.

use axum::{extract::State, routing::post, Json, Router};
use tower_sessions::Session;

use crate::context::AppCtx;
use crate::dto::request::{
    ApproveAdRegisterDto, Pagination, RequestCategoryDto, RequestCountsDto, RequestCreateDto,
    RequestDto, RequestFilter, RequestHistoryEntryDto, RequestListResponse,
    RequestPrinterOptionDto, RequestTransitionPayload,
};
use crate::error_axum::AppErrorResponse;
use crate::http::auth::session_identity;
use crate::tauri_cmds::requests::{
    build_request_printer_options, build_requests_approve_ad_register, build_requests_counts,
    build_requests_create, build_requests_get, build_requests_get_history, build_requests_list,
    build_requests_list_categories, build_requests_transition,
};

// ---------------------------------------------------------------------------
// Payload wrappers
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListPayload {
    pub filter: RequestFilter,
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
    pub dto: RequestCreateDto,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransitionPayload {
    pub payload: RequestTransitionPayload,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApproveAdRegisterPayload {
    pub payload: ApproveAdRegisterDto,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

pub async fn handler_list(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<ListPayload>,
) -> Result<Json<RequestListResponse>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_requests_list(&ctx, &identity, p.filter, p.pagination)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_get(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<GetPayload>,
) -> Result<Json<RequestDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_requests_get(&ctx, &identity, p.id as i64)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_create(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<CreatePayload>,
) -> Result<Json<RequestDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    let result = build_requests_create(&ctx, &identity, p.dto)
        .await
        .map_err(AppErrorResponse::from)?;
    // WS push is owned by RequestService::create (the single broadcast owner) —
    // do NOT re-broadcast here. `ctx.ws_broadcast` and `RequestService.ws_tx`
    // are the SAME Arc<broadcast::Sender>, so a second send is a literal
    // double-fire to every subscriber (CR-01 / "WS toast spam" fix).
    Ok(Json(result))
}

pub async fn handler_transition(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<TransitionPayload>,
) -> Result<Json<RequestDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    let result = build_requests_transition(&ctx, &identity, p.payload)
        .await
        .map_err(AppErrorResponse::from)?;
    // WS push is owned by RequestService::transition (the single broadcast
    // owner) — do NOT re-broadcast here (CR-01 double-fire fix).
    Ok(Json(result))
}

pub async fn handler_approve_ad_register(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<ApproveAdRegisterPayload>,
) -> Result<Json<RequestDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    let result = build_requests_approve_ad_register(&ctx, &identity, p.payload)
        .await
        .map_err(AppErrorResponse::from)?;
    // WS push is owned by RequestService::approve_ad_register (the single
    // broadcast owner) — do NOT re-broadcast here (CR-01 double-fire fix).
    Ok(Json(result))
}

pub async fn handler_counts(
    State(ctx): State<AppCtx>,
    session: Session,
) -> Result<Json<RequestCountsDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_requests_counts(&ctx, &identity)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_list_categories(
    State(ctx): State<AppCtx>,
    session: Session,
) -> Result<Json<Vec<RequestCategoryDto>>, AppErrorResponse> {
    let _identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_requests_list_categories(&ctx)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_get_history(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<GetPayload>,
) -> Result<Json<Vec<RequestHistoryEntryDto>>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_requests_get_history(&ctx, &identity, p.id as i64)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

/// Printer options for the create-request form (D-PRN-01). Read-only —
/// no `ws_broadcast` push (this is a read, not a mutation).
pub async fn handler_request_printer_options(
    State(ctx): State<AppCtx>,
    session: Session,
) -> Result<Json<Vec<RequestPrinterOptionDto>>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_request_printer_options(&ctx, &identity)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router() -> Router<AppCtx> {
    Router::new()
        .route("/api/v1/requests_list", post(handler_list))
        .route("/api/v1/requests_get", post(handler_get))
        .route("/api/v1/requests_create", post(handler_create))
        .route("/api/v1/requests_transition", post(handler_transition))
        .route(
            "/api/v1/requests_approve_ad_register",
            post(handler_approve_ad_register),
        )
        .route("/api/v1/requests_counts", post(handler_counts))
        .route(
            "/api/v1/requests_list_categories",
            post(handler_list_categories),
        )
        .route("/api/v1/requests_get_history", post(handler_get_history))
        .route(
            "/api/v1/request_printer_options",
            post(handler_request_printer_options),
        )
}

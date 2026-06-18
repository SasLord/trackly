//! Requests axum HTTP routes — Phase 6 Plan 03.
//!
//! Mirrors `tauri_cmds::requests` via POST endpoints.
//!
//! Pattern (S-2): каждый handler делегирует соответствующему `build_*` helper
//! из tauri_cmds/requests.rs — один DTO, два транспорта.
//!
//! WS push: handler_create и handler_transition отправляют WsEvent через
//! ctx.ws_broadcast после успешной мутации (D-Notify-01).

use axum::{extract::State, routing::post, Json, Router};
use tower_sessions::Session;

use crate::context::AppCtx;
use crate::dto::printer::WsEvent;
use crate::dto::request::{
    Pagination, RequestCountsDto, RequestCreateDto, RequestDto, RequestFilter,
    RequestHistoryEntryDto, RequestListResponse, RequestTransitionPayload,
};
use crate::error_axum::AppErrorResponse;
use crate::http::auth::session_identity;
use crate::tauri_cmds::requests::{
    build_requests_counts, build_requests_create, build_requests_get, build_requests_get_history,
    build_requests_list, build_requests_list_categories, build_requests_transition,
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

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

pub async fn handler_list(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<ListPayload>,
) -> Result<Json<RequestListResponse>, AppErrorResponse> {
    let _identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_requests_list(&ctx, p.filter, p.pagination)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_get(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<GetPayload>,
) -> Result<Json<RequestDto>, AppErrorResponse> {
    let _identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_requests_get(&ctx, p.id as i64)
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
    // WS push after create (D-Notify-01) — broadcast already done in RequestService::create,
    // but we re-broadcast from HTTP transport as well for completeness.
    ctx.ws_broadcast
        .send(WsEvent::NewRequest {
            request_id: result.id,
            request_type: result.request_type.clone(),
            requester_name: result.requester_name.clone().unwrap_or_default(),
        })
        .ok();
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
    // WS push after transition (D-Notify-01) — broadcast already done in service layer.
    ctx.ws_broadcast
        .send(WsEvent::RequestStatusChanged {
            request_id: result.id,
            new_status: result.status.clone(),
        })
        .ok();
    Ok(Json(result))
}

pub async fn handler_counts(
    State(ctx): State<AppCtx>,
    session: Session,
) -> Result<Json<RequestCountsDto>, AppErrorResponse> {
    let _identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_requests_counts(&ctx)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_list_categories(
    State(ctx): State<AppCtx>,
    session: Session,
) -> Result<Json<Vec<String>>, AppErrorResponse> {
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
    let _identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_requests_get_history(&ctx, p.id as i64)
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
        .route("/api/v1/requests_counts", post(handler_counts))
        .route(
            "/api/v1/requests_list_categories",
            post(handler_list_categories),
        )
        .route("/api/v1/requests_get_history", post(handler_get_history))
}

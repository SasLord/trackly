//! Acts axum HTTP routes — Plan 02 vertical slice.
//!
//! Mirrors `tauri_cmds::acts` via POST endpoints. The router is BUILT in
//! Plan 02 but NOT bound to a TCP listener — server-mode wiring is Phase 5.

use axum::http::{header, StatusCode};
use axum::response::IntoResponse;
use axum::{extract::State, routing::post, Json, Router};

use crate::context::AppCtx;
use crate::dto::act::{
    ActCreateDto, ActDto, ActFilter, ActListResponse, ActReturnDto, ActsCountsDto, Pagination,
};
use crate::error_axum::AppErrorResponse;
use crate::tauri_cmds::acts::{
    build_acts_counts, build_acts_create, build_acts_delete, build_acts_get, build_acts_list,
    build_acts_peek_next_number, build_acts_render_pdf, build_acts_return,
    build_devices_render_acceptance_pdf,
};

#[derive(serde::Deserialize)]
pub struct ListPayload {
    pub filter: ActFilter,
    pub pagination: Pagination,
}

#[derive(serde::Deserialize)]
pub struct GetPayload {
    pub id: i64,
}

#[derive(serde::Deserialize)]
pub struct CreatePayload {
    pub payload: ActCreateDto,
}

#[derive(serde::Deserialize)]
pub struct DeletePayload {
    pub id: i64,
    pub version: i64,
}

#[derive(serde::Deserialize)]
pub struct ReturnPayload {
    pub act_id: i64,
    pub payload: ActReturnDto,
}

#[derive(serde::Deserialize)]
pub struct RenderPdfPayload {
    pub act_id: i64,
}

#[derive(serde::Deserialize)]
pub struct RenderAcceptancePdfPayload {
    pub device_id: i64,
    pub giver_name: String,
    pub receiver_name: String,
    pub date_utc: i64,
}

// Handlers ------------------------------------------------------------------

pub async fn handler_list(
    State(ctx): State<AppCtx>,
    Json(p): Json<ListPayload>,
) -> Result<Json<ActListResponse>, AppErrorResponse> {
    Ok(Json(
        build_acts_list(&ctx, p.filter, p.pagination)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_get(
    State(ctx): State<AppCtx>,
    Json(p): Json<GetPayload>,
) -> Result<Json<ActDto>, AppErrorResponse> {
    Ok(Json(
        build_acts_get(&ctx, p.id)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_create(
    State(ctx): State<AppCtx>,
    Json(p): Json<CreatePayload>,
) -> Result<Json<ActDto>, AppErrorResponse> {
    Ok(Json(
        build_acts_create(&ctx, p.payload)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_return(
    State(ctx): State<AppCtx>,
    Json(p): Json<ReturnPayload>,
) -> Result<Json<ActDto>, AppErrorResponse> {
    Ok(Json(
        build_acts_return(&ctx, p.act_id, p.payload)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_delete(
    State(ctx): State<AppCtx>,
    Json(p): Json<DeletePayload>,
) -> Result<Json<()>, AppErrorResponse> {
    build_acts_delete(&ctx, p.id, p.version)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(()))
}

pub async fn handler_counts(
    State(ctx): State<AppCtx>,
) -> Result<Json<ActsCountsDto>, AppErrorResponse> {
    Ok(Json(
        build_acts_counts(&ctx)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_peek_next_number(
    State(ctx): State<AppCtx>,
) -> Result<Json<i64>, AppErrorResponse> {
    Ok(Json(
        build_acts_peek_next_number(&ctx)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_render_pdf(
    State(ctx): State<AppCtx>,
    Json(p): Json<RenderPdfPayload>,
) -> Result<impl IntoResponse, AppErrorResponse> {
    let bytes = build_acts_render_pdf(&ctx, p.act_id)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/pdf")],
        bytes,
    ))
}

pub async fn handler_render_acceptance_pdf(
    State(ctx): State<AppCtx>,
    Json(p): Json<RenderAcceptancePdfPayload>,
) -> Result<impl IntoResponse, AppErrorResponse> {
    let bytes = build_devices_render_acceptance_pdf(
        &ctx,
        p.device_id,
        p.giver_name,
        p.receiver_name,
        p.date_utc,
    )
    .await
    .map_err(AppErrorResponse::from)?;
    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/pdf")],
        bytes,
    ))
}

pub fn router() -> Router<AppCtx> {
    Router::new()
        .route("/api/v1/acts_list", post(handler_list))
        .route("/api/v1/acts_get", post(handler_get))
        .route("/api/v1/acts_create", post(handler_create))
        .route("/api/v1/acts_return", post(handler_return))
        .route("/api/v1/acts_delete", post(handler_delete))
        .route("/api/v1/acts_counts", post(handler_counts))
        .route(
            "/api/v1/acts_peek_next_number",
            post(handler_peek_next_number),
        )
        .route("/api/v1/acts_render_pdf", post(handler_render_pdf))
        .route(
            "/api/v1/devices_render_acceptance_pdf",
            post(handler_render_acceptance_pdf),
        )
}

//! Acts axum HTTP routes — Plan 02 vertical slice.
//!
//! Mirrors `tauri_cmds::acts` via POST endpoints. The router is BUILT in
//! Plan 02 but NOT bound to a TCP listener — server-mode wiring is Phase 5.
//!
//! Phase 5 Plan 04: mutation handlers protected by `authorize(&identity, &Action::MutateActs)`.
//! Read handlers require only a valid session.

use axum::http::{header, StatusCode};
use axum::response::IntoResponse;
use axum::{extract::State, routing::post, Json, Router};
use tower_sessions::Session;

use crate::context::AppCtx;
use crate::dto::act::{
    ActCreateDto, ActDto, ActFilter, ActListResponse, ActReturnDto, ActUpdateDto, ActsCountsDto,
    Pagination,
};
use crate::dto::suggest::SuggestPersonField;
use crate::error_axum::AppErrorResponse;
use crate::http::auth::session_identity;
use crate::tauri_cmds::acts::{
    build_acts_counts, build_acts_create, build_acts_delete, build_acts_get, build_acts_list,
    build_acts_peek_next_number, build_acts_render_pdf, build_acts_return, build_acts_search,
    build_acts_suggest_person, build_acts_update, build_devices_render_acceptance_pdf,
};

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListPayload {
    pub filter: ActFilter,
    pub pagination: Pagination,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchPayload {
    pub query: String,
    pub filter: ActFilter,
    pub pagination: Pagination,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPayload {
    pub id: i64,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePayload {
    pub payload: ActCreateDto,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletePayload {
    pub id: i64,
    pub version: i64,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePayload {
    pub payload: ActUpdateDto,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReturnPayload {
    pub act_id: i64,
    pub payload: ActReturnDto,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderPdfPayload {
    pub act_id: i64,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestPersonPayload {
    pub field: SuggestPersonField,
    pub prefix: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderAcceptancePdfPayload {
    pub device_id: i64,
    pub giver_name: String,
    pub receiver_name: String,
    pub date_utc: i64,
}

// Handlers ------------------------------------------------------------------

pub async fn handler_list(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<ListPayload>,
) -> Result<Json<ActListResponse>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_acts_list(&ctx, &identity, p.filter, p.pagination)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_search(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<SearchPayload>,
) -> Result<Json<ActListResponse>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_acts_search(&ctx, &identity, p.query, p.filter, p.pagination)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_get(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<GetPayload>,
) -> Result<Json<ActDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_acts_get(&ctx, &identity, p.id)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_create(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<CreatePayload>,
) -> Result<Json<ActDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_acts_create(&ctx, &identity, p.payload)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_return(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<ReturnPayload>,
) -> Result<Json<ActDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_acts_return(&ctx, &identity, p.act_id, p.payload)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_delete(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<DeletePayload>,
) -> Result<Json<()>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    build_acts_delete(&ctx, &identity, p.id, p.version)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(()))
}

pub async fn handler_update(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<UpdatePayload>,
) -> Result<Json<ActDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_acts_update(&ctx, &identity, p.payload)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_counts(
    State(ctx): State<AppCtx>,
    session: Session,
) -> Result<Json<ActsCountsDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_acts_counts(&ctx, &identity)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_peek_next_number(
    State(ctx): State<AppCtx>,
    session: Session,
) -> Result<Json<i64>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_acts_peek_next_number(&ctx, &identity)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

// Phase 16 (D-09/D-10): both handlers return the HTML string produced by
// ActService::render_pdf/render_acceptance_pdf as `text/html; charset=utf-8`.
// Printing/saving happens via the browser's print dialog (srcdoc iframe +
// print()) on both desktop and LAN — no server-side canonical PDF anymore.
pub async fn handler_render_pdf(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<RenderPdfPayload>,
) -> Result<impl IntoResponse, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    let html = build_acts_render_pdf(&ctx, &identity, p.act_id)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        html,
    ))
}

pub async fn handler_render_acceptance_pdf(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<RenderAcceptancePdfPayload>,
) -> Result<impl IntoResponse, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    let html = build_devices_render_acceptance_pdf(
        &ctx,
        &identity,
        p.device_id,
        p.giver_name,
        p.receiver_name,
        p.date_utc,
    )
    .await
    .map_err(AppErrorResponse::from)?;
    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        html,
    ))
}

pub async fn handler_suggest_person(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<SuggestPersonPayload>,
) -> Result<Json<Vec<String>>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_acts_suggest_person(&ctx, &identity, p.field, p.prefix)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub fn router() -> Router<AppCtx> {
    Router::new()
        .route("/api/v1/acts_list", post(handler_list))
        .route("/api/v1/acts_search", post(handler_search))
        .route("/api/v1/acts_get", post(handler_get))
        .route("/api/v1/acts_create", post(handler_create))
        .route("/api/v1/acts_return", post(handler_return))
        .route("/api/v1/acts_delete", post(handler_delete))
        .route("/api/v1/acts_update", post(handler_update))
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
        .route("/api/v1/acts_suggest_person", post(handler_suggest_person))
}

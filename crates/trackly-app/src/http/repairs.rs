//! HTTP-маршруты ремонтных заявок (Phase 41.4, plan 14): зеркало `tauri_cmds::repairs`.
//!
//! Каждый обработчик берёт сессию, зовёт тот же `build_repairs_*`, что и
//! Tauri-команда. `repairs_render_act` отдаёт `text/html` как `acts_render_pdf`.

use axum::http::{header, StatusCode};
use axum::response::IntoResponse;
use axum::{extract::State, routing::post, Json, Router};
use tower_sessions::Session;

use crate::context::AppCtx;
use crate::dto::repair::{
    OpenRepairRefDto, RepairAssignDeviceDto, RepairAssignResultDto, RepairCancelDto,
    RepairCompleteDto, RepairCreateDto, RepairCreateResultDto, RepairDeviceOptionDto,
    RepairLastPlacesDto, RepairMergeDto, RepairMergeResultDto, RepairMoveChoiceDto,
    RepairReceiveDto, RepairRejectDto, RepairSendDto, RepairStateDto, RepairTransitionResultDto,
};
use crate::error_axum::AppErrorResponse;
use crate::http::auth::session_identity;
use crate::tauri_cmds::repairs as r;

pub async fn handler_device_options(
    State(ctx): State<AppCtx>,
    session: Session,
) -> Result<Json<Vec<RepairDeviceOptionDto>>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        r::build_repairs_device_options(&ctx, &identity)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePayload {
    pub dto: RepairCreateDto,
}

pub async fn handler_create(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<CreatePayload>,
) -> Result<Json<RepairCreateResultDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        r::build_repairs_create(&ctx, &identity, p.dto)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetStatePayload {
    pub request_id: i64,
}

pub async fn handler_get_state(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<GetStatePayload>,
) -> Result<Json<RepairStateDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        r::build_repairs_get_state(&ctx, &identity, p.request_id)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_last_places(
    State(ctx): State<AppCtx>,
    session: Session,
) -> Result<Json<RepairLastPlacesDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        r::build_repairs_last_places(&ctx, &identity)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenForDevicesPayload {
    pub device_ids: Vec<i64>,
}

pub async fn handler_open_for_devices(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<OpenForDevicesPayload>,
) -> Result<Json<Vec<OpenRepairRefDto>>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        r::build_repairs_open_for_devices(&ctx, &identity, p.device_ids)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptPayload {
    pub request_id: i64,
    pub version: i64,
    pub choice: RepairMoveChoiceDto,
}

pub async fn handler_accept(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<AcceptPayload>,
) -> Result<Json<RepairTransitionResultDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        r::build_repairs_accept(&ctx, &identity, p.request_id, p.version, p.choice)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveToStoragePayload {
    pub request_id: i64,
    pub version: i64,
    pub place_id: i64,
}

pub async fn handler_move_to_storage(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<MoveToStoragePayload>,
) -> Result<Json<RepairTransitionResultDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        r::build_repairs_move_to_storage(&ctx, &identity, p.request_id, p.version, p.place_id)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendPayload {
    pub dto: RepairSendDto,
}

pub async fn handler_send(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<SendPayload>,
) -> Result<Json<RepairTransitionResultDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        r::build_repairs_send(&ctx, &identity, p.dto)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceivePayload {
    pub dto: RepairReceiveDto,
}

pub async fn handler_receive(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<ReceivePayload>,
) -> Result<Json<RepairTransitionResultDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        r::build_repairs_receive(&ctx, &identity, p.dto)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletePayload {
    pub dto: RepairCompleteDto,
}

pub async fn handler_complete(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<CompletePayload>,
) -> Result<Json<RepairTransitionResultDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        r::build_repairs_complete(&ctx, &identity, p.dto)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelRepairPayload {
    pub dto: RepairCancelDto,
}

pub async fn handler_cancel_repair(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<CancelRepairPayload>,
) -> Result<Json<RepairTransitionResultDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        r::build_repairs_cancel_repair(&ctx, &identity, p.dto)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RejectPayload {
    pub dto: RepairRejectDto,
}

pub async fn handler_reject(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<RejectPayload>,
) -> Result<Json<RepairTransitionResultDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        r::build_repairs_reject(&ctx, &identity, p.dto)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelByAuthorPayload {
    pub dto: RepairCancelDto,
}

pub async fn handler_cancel_by_author(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<CancelByAuthorPayload>,
) -> Result<Json<RepairTransitionResultDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        r::build_repairs_cancel_by_author(&ctx, &identity, p.dto)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignDevicePayload {
    pub dto: RepairAssignDeviceDto,
}

pub async fn handler_assign_device(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<AssignDevicePayload>,
) -> Result<Json<RepairAssignResultDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        r::build_repairs_assign_device(&ctx, &identity, p.dto)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MergePayload {
    pub dto: RepairMergeDto,
}

pub async fn handler_merge_into_open(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<MergePayload>,
) -> Result<Json<RepairMergeResultDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        r::build_repairs_merge_into_open(&ctx, &identity, p.dto)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkScansPayload {
    pub visit_id: i64,
    pub attachment_ids: Vec<i64>,
}

pub async fn handler_link_scans(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<LinkScansPayload>,
) -> Result<Json<RepairStateDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        r::build_repairs_link_scans(&ctx, &identity, p.visit_id, p.attachment_ids)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderActPayload {
    pub visit_id: i64,
}

pub async fn handler_render_act(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<RenderActPayload>,
) -> Result<impl IntoResponse, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    let html = r::build_repairs_render_act(&ctx, &identity, p.visit_id)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        html,
    ))
}

pub fn router() -> Router<AppCtx> {
    Router::new()
        .route(
            "/api/v1/repairs_device_options",
            post(handler_device_options),
        )
        .route("/api/v1/repairs_create", post(handler_create))
        .route("/api/v1/repairs_get_state", post(handler_get_state))
        .route("/api/v1/repairs_last_places", post(handler_last_places))
        .route(
            "/api/v1/repairs_open_for_devices",
            post(handler_open_for_devices),
        )
        .route("/api/v1/repairs_accept", post(handler_accept))
        .route(
            "/api/v1/repairs_move_to_storage",
            post(handler_move_to_storage),
        )
        .route("/api/v1/repairs_send", post(handler_send))
        .route("/api/v1/repairs_receive", post(handler_receive))
        .route("/api/v1/repairs_complete", post(handler_complete))
        .route("/api/v1/repairs_cancel_repair", post(handler_cancel_repair))
        .route("/api/v1/repairs_reject", post(handler_reject))
        .route(
            "/api/v1/repairs_cancel_by_author",
            post(handler_cancel_by_author),
        )
        .route("/api/v1/repairs_assign_device", post(handler_assign_device))
        .route(
            "/api/v1/repairs_merge_into_open",
            post(handler_merge_into_open),
        )
        .route("/api/v1/repairs_link_scans", post(handler_link_scans))
        .route("/api/v1/repairs_render_act", post(handler_render_act))
}

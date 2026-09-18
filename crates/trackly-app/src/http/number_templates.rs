//! Numbering-template axum HTTP routes — Phase 40.2, Plan 05.
//!
//! Mirrors `tauri_cmds::number_templates` via POST endpoints — same
//! `build_*` helpers, same two gate categories (see that module's doc
//! comment for the Group A/B split rationale).

use axum::{extract::State, routing::post, Json, Router};
use tower_sessions::Session;

use crate::context::AppCtx;
use crate::dto::number_template::{
    NextNumberDto, NumberTemplateDto, OccupyingRecordDto, TemplateContextDto, TemplateTypeDto,
};
use crate::error_axum::AppErrorResponse;
use crate::http::auth::session_identity;
use crate::tauri_cmds::number_templates::{
    build_number_template_contexts_get, build_number_template_contexts_set,
    build_number_templates_create, build_number_templates_delete,
    build_number_templates_is_occupied, build_number_templates_list,
    build_number_templates_list_by_context, build_number_templates_peek_next,
    build_number_templates_preview_mask, build_number_templates_update_mask,
};

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePayload {
    pub template_type: TemplateTypeDto,
    pub mask: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMaskPayload {
    pub id: i64,
    pub mask: String,
    pub version: i64,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletePayload {
    pub id: i64,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListPayload {
    pub template_type: Option<TemplateTypeDto>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewMaskPayload {
    pub template_type: TemplateTypeDto,
    pub mask: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListByContextPayload {
    pub context: TemplateContextDto,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeekNextPayload {
    pub template_id: i64,
    pub context: TemplateContextDto,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextGetPayload {
    pub context: TemplateContextDto,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextSetPayload {
    pub context: TemplateContextDto,
    pub template_id: Option<i64>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IsOccupiedPayload {
    pub context: TemplateContextDto,
    pub candidate: String,
    pub exclude_id: Option<i64>,
}

// Handlers — Group A: CRUD, Admin-only (Action::ManageSettings) ------------

pub async fn handler_create(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<CreatePayload>,
) -> Result<Json<NumberTemplateDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_number_templates_create(&ctx, &identity, p.template_type, p.mask)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_update_mask(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<UpdateMaskPayload>,
) -> Result<Json<NumberTemplateDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_number_templates_update_mask(&ctx, &identity, p.id, p.mask, p.version)
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
    build_number_templates_delete(&ctx, &identity, p.id)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(()))
}

pub async fn handler_list(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<ListPayload>,
) -> Result<Json<Vec<NumberTemplateDto>>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_number_templates_list(&ctx, &identity, p.template_type)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_preview_mask(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<PreviewMaskPayload>,
) -> Result<Json<NextNumberDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_number_templates_preview_mask(&ctx, &identity, p.template_type, p.mask)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

// Handlers — Group B: usage, gate derived from `context` -------------------

pub async fn handler_list_by_context(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<ListByContextPayload>,
) -> Result<Json<Vec<NumberTemplateDto>>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_number_templates_list_by_context(&ctx, &identity, p.context)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_peek_next(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<PeekNextPayload>,
) -> Result<Json<NextNumberDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_number_templates_peek_next(&ctx, &identity, p.template_id, p.context)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_contexts_get(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<ContextGetPayload>,
) -> Result<Json<Option<i64>>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_number_template_contexts_get(&ctx, &identity, p.context)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_contexts_set(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<ContextSetPayload>,
) -> Result<Json<()>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    build_number_template_contexts_set(&ctx, &identity, p.context, p.template_id)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(()))
}

pub async fn handler_is_occupied(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<IsOccupiedPayload>,
) -> Result<Json<Option<OccupyingRecordDto>>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_number_templates_is_occupied(&ctx, &identity, p.context, p.candidate, p.exclude_id)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub fn router() -> Router<AppCtx> {
    Router::new()
        .route("/api/v1/number_templates_create", post(handler_create))
        .route(
            "/api/v1/number_templates_update_mask",
            post(handler_update_mask),
        )
        .route("/api/v1/number_templates_delete", post(handler_delete))
        .route("/api/v1/number_templates_list", post(handler_list))
        .route(
            "/api/v1/number_templates_preview_mask",
            post(handler_preview_mask),
        )
        .route(
            "/api/v1/number_templates_list_by_context",
            post(handler_list_by_context),
        )
        .route(
            "/api/v1/number_templates_peek_next",
            post(handler_peek_next),
        )
        .route(
            "/api/v1/number_template_contexts_get",
            post(handler_contexts_get),
        )
        .route(
            "/api/v1/number_template_contexts_set",
            post(handler_contexts_set),
        )
        .route(
            "/api/v1/number_templates_is_occupied",
            post(handler_is_occupied),
        )
}

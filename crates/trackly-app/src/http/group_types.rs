//! Group type axum HTTP routes — Phase 41 Plan 09.
//!
//! Все routes — POST-эндпоинты вида /api/v1/<имя_команды> (имя маршрута == имя
//! Tauri-команды). Handlers — thin adapters: резолвят session identity и
//! вызывают ТЕ ЖЕ `build_group_types_*` / `build_group_type_properties_*` из
//! `tauri_cmds::group_types`. Гейты (`ManageGroupTypes` Admin-only на мутациях,
//! `ReadGroups` Admin|Manager на чтениях) живут внутри `build_*` — здесь они не
//! дублируются.

use axum::{extract::State, routing::post, Json, Router};
use tower_sessions::Session;

use crate::context::AppCtx;
use crate::dto::group_types::{
    GroupRefDto, GroupTypeCreateDto, GroupTypeDto, GroupTypePropertyDto, GroupTypeUpdateDto,
    PropertyCreateDto, PropertyDeleteOutcomeDto, PropertyUpdateDto,
};
use crate::error_axum::AppErrorResponse;
use crate::http::auth::session_identity;
use crate::tauri_cmds::group_types::{
    build_group_type_properties_create, build_group_type_properties_delete,
    build_group_type_properties_empty_groups, build_group_type_properties_reorder,
    build_group_type_properties_unarchive, build_group_type_properties_update,
    build_group_types_create, build_group_types_delete, build_group_types_list,
    build_group_types_update,
};

// ---------------------------------------------------------------------------
// Payload structs
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListPayload {
    pub include_archived: bool,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePayload {
    pub dto: GroupTypeCreateDto,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePayload {
    pub id: i64,
    pub version: i64,
    pub dto: GroupTypeUpdateDto,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletePayload {
    pub id: i64,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PropertyCreatePayload {
    pub dto: PropertyCreateDto,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PropertyUpdatePayload {
    pub id: i64,
    pub version: i64,
    pub dto: PropertyUpdateDto,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PropertyDeletePayload {
    pub id: i64,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PropertyUnarchivePayload {
    pub id: i64,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PropertyReorderPayload {
    pub type_id: i32,
    pub ordered_ids: Vec<i32>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PropertyEmptyGroupsPayload {
    pub property_id: i64,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

pub async fn handler_list(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(payload): Json<ListPayload>,
) -> Result<Json<Vec<GroupTypeDto>>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_group_types_list(&ctx, &identity, payload.include_archived)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_create(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(payload): Json<CreatePayload>,
) -> Result<Json<GroupTypeDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_group_types_create(&ctx, &identity, payload.dto)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_update(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(payload): Json<UpdatePayload>,
) -> Result<Json<GroupTypeDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_group_types_update(&ctx, &identity, payload.id, payload.version, payload.dto)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_delete(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(payload): Json<DeletePayload>,
) -> Result<Json<()>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_group_types_delete(&ctx, &identity, payload.id)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_property_create(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(payload): Json<PropertyCreatePayload>,
) -> Result<Json<GroupTypePropertyDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_group_type_properties_create(&ctx, &identity, payload.dto)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_property_update(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(payload): Json<PropertyUpdatePayload>,
) -> Result<Json<GroupTypePropertyDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_group_type_properties_update(
            &ctx,
            &identity,
            payload.id,
            payload.version,
            payload.dto,
        )
        .await
        .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_property_delete(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(payload): Json<PropertyDeletePayload>,
) -> Result<Json<PropertyDeleteOutcomeDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_group_type_properties_delete(&ctx, &identity, payload.id)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_property_unarchive(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(payload): Json<PropertyUnarchivePayload>,
) -> Result<Json<GroupTypePropertyDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_group_type_properties_unarchive(&ctx, &identity, payload.id)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_property_reorder(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(payload): Json<PropertyReorderPayload>,
) -> Result<Json<Vec<GroupTypePropertyDto>>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_group_type_properties_reorder(&ctx, &identity, payload.type_id, payload.ordered_ids)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_property_empty_groups(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(payload): Json<PropertyEmptyGroupsPayload>,
) -> Result<Json<Vec<GroupRefDto>>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_group_type_properties_empty_groups(&ctx, &identity, payload.property_id)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub fn router() -> Router<AppCtx> {
    Router::new()
        .route("/api/v1/group_types_list", post(handler_list))
        .route("/api/v1/group_types_create", post(handler_create))
        .route("/api/v1/group_types_update", post(handler_update))
        .route("/api/v1/group_types_delete", post(handler_delete))
        .route(
            "/api/v1/group_type_properties_create",
            post(handler_property_create),
        )
        .route(
            "/api/v1/group_type_properties_update",
            post(handler_property_update),
        )
        .route(
            "/api/v1/group_type_properties_delete",
            post(handler_property_delete),
        )
        .route(
            "/api/v1/group_type_properties_unarchive",
            post(handler_property_unarchive),
        )
        .route(
            "/api/v1/group_type_properties_reorder",
            post(handler_property_reorder),
        )
        .route(
            "/api/v1/group_type_properties_empty_groups",
            post(handler_property_empty_groups),
        )
}

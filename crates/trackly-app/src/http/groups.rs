//! Group axum HTTP routes — Phase 41 Plan 13.
//!
//! Все routes — POST-эндпоинты вида /api/v1/<имя_команды> (имя маршрута == имя
//! Tauri-команды). Handlers — thin adapters: резолвят session identity и
//! вызывают ТЕ ЖЕ `build_groups_*` из `tauri_cmds::groups`. Гейты
//! (`ReadGroups` на чтениях, `MutateGroups` на мутациях) живут внутри
//! `build_*` — здесь они не дублируются.

use axum::{extract::State, routing::post, Json, Router};
use tower_sessions::Session;

use crate::context::AppCtx;
use crate::dto::groups::{
    DeviceMembershipDto, GroupAddDevicesDto, GroupAddDevicesResultDto, GroupCardDto,
    GroupCompositionDto, GroupCreateDto, GroupDeleteResultDto, GroupDto, GroupMoveDto,
    GroupMoveResultDto, GroupRemoveDevicesDto, GroupSearchHitDto, GroupSetParentDto,
    GroupSetValuesDto, UserOptionDto,
};
use crate::error_axum::AppErrorResponse;
use crate::http::auth::session_identity;
use crate::tauri_cmds::groups::{
    build_groups_add_devices, build_groups_card, build_groups_composition, build_groups_create,
    build_groups_delete, build_groups_for_devices, build_groups_get, build_groups_list,
    build_groups_move, build_groups_remove_devices, build_groups_search, build_groups_set_parent,
    build_groups_set_values, build_groups_update, build_groups_user_options,
};

// ---------------------------------------------------------------------------
// Payload structs
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdPayload {
    pub id: i64,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompositionPayload {
    pub group_id: i64,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchPayload {
    pub query: String,
    pub exclude_group_id: Option<i64>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForDevicesPayload {
    pub device_ids: Vec<i64>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserOptionsPayload {
    pub query: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePayload {
    pub dto: GroupCreateDto,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePayload {
    pub id: i64,
    pub version: i64,
    pub name: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetParentPayload {
    pub dto: GroupSetParentDto,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddDevicesPayload {
    pub dto: GroupAddDevicesDto,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveDevicesPayload {
    pub dto: GroupRemoveDevicesDto,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MovePayload {
    pub dto: GroupMoveDto,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetValuesPayload {
    pub dto: GroupSetValuesDto,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

pub async fn handler_list(
    State(ctx): State<AppCtx>,
    session: Session,
) -> Result<Json<Vec<GroupDto>>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_groups_list(&ctx, &identity)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_get(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(payload): Json<IdPayload>,
) -> Result<Json<GroupDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_groups_get(&ctx, &identity, payload.id)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_card(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(payload): Json<IdPayload>,
) -> Result<Json<GroupCardDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_groups_card(&ctx, &identity, payload.id)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_composition(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(payload): Json<CompositionPayload>,
) -> Result<Json<GroupCompositionDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_groups_composition(&ctx, &identity, payload.group_id)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_search(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(payload): Json<SearchPayload>,
) -> Result<Json<Vec<GroupSearchHitDto>>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_groups_search(&ctx, &identity, payload.query, payload.exclude_group_id)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_for_devices(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(payload): Json<ForDevicesPayload>,
) -> Result<Json<Vec<DeviceMembershipDto>>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_groups_for_devices(&ctx, &identity, payload.device_ids)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_user_options(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(payload): Json<UserOptionsPayload>,
) -> Result<Json<Vec<UserOptionDto>>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_groups_user_options(&ctx, &identity, payload.query)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_create(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(payload): Json<CreatePayload>,
) -> Result<Json<GroupDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_groups_create(&ctx, &identity, payload.dto)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_update(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(payload): Json<UpdatePayload>,
) -> Result<Json<GroupDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_groups_update(&ctx, &identity, payload.id, payload.version, payload.name)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_delete(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(payload): Json<IdPayload>,
) -> Result<Json<GroupDeleteResultDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_groups_delete(&ctx, &identity, payload.id)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_set_parent(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(payload): Json<SetParentPayload>,
) -> Result<Json<GroupMoveResultDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_groups_set_parent(&ctx, &identity, payload.dto)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_add_devices(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(payload): Json<AddDevicesPayload>,
) -> Result<Json<GroupAddDevicesResultDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_groups_add_devices(&ctx, &identity, payload.dto)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_remove_devices(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(payload): Json<RemoveDevicesPayload>,
) -> Result<Json<i32>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_groups_remove_devices(&ctx, &identity, payload.dto)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_move(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(payload): Json<MovePayload>,
) -> Result<Json<GroupMoveResultDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_groups_move(&ctx, &identity, payload.dto)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_set_values(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(payload): Json<SetValuesPayload>,
) -> Result<Json<GroupCardDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_groups_set_values(&ctx, &identity, payload.dto)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub fn router() -> Router<AppCtx> {
    Router::new()
        .route("/api/v1/groups_list", post(handler_list))
        .route("/api/v1/groups_get", post(handler_get))
        .route("/api/v1/groups_card", post(handler_card))
        .route("/api/v1/groups_composition", post(handler_composition))
        .route("/api/v1/groups_search", post(handler_search))
        .route("/api/v1/groups_for_devices", post(handler_for_devices))
        .route("/api/v1/groups_user_options", post(handler_user_options))
        .route("/api/v1/groups_create", post(handler_create))
        .route("/api/v1/groups_update", post(handler_update))
        .route("/api/v1/groups_delete", post(handler_delete))
        .route("/api/v1/groups_set_parent", post(handler_set_parent))
        .route("/api/v1/groups_add_devices", post(handler_add_devices))
        .route(
            "/api/v1/groups_remove_devices",
            post(handler_remove_devices),
        )
        .route("/api/v1/groups_move", post(handler_move))
        .route("/api/v1/groups_set_values", post(handler_set_values))
}

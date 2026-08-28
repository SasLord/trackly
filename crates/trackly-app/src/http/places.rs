//! Place axum HTTP routes — Plan 12.
//!
//! Все routes — POST-эндпоинты вида places_<action> под /api/v1 (аналогично Tauri command names).
//! Handlers — thin adapters, делегируют `build_places_*` helpers из
//! `tauri_cmds::places` (mirrors `http/devices.rs`'s exact structure —
//! PATTERNS.md §Pattern 1: "один DTO, два транспорта").
//!
//! D-20: mutation handlers reach `authorize(&Action::MutatePlaces)` (Admin-only)
//! and read handlers reach `authorize(&Action::ReadPlaces)` (Admin|Manager)
//! INSIDE `build_places_*` — this file never re-implements the gate, it only
//! resolves the session identity and delegates.

use axum::{extract::State, routing::post, Json, Router};
use tower_sessions::Session;

use crate::context::AppCtx;
use crate::dto::place::{PlaceContentDto, PlaceDto, PlaceNewDto, PlacePathDto, SubtreeStatsDto};
use crate::error_axum::AppErrorResponse;
use crate::http::auth::session_identity;
use crate::tauri_cmds::places::{
    build_places_archive, build_places_contents, build_places_create, build_places_delete,
    build_places_get, build_places_list_all, build_places_list_children, build_places_move,
    build_places_rename, build_places_search, build_places_set_path_variant,
    build_places_subtree_stats, build_places_unarchive,
};

// ---------------------------------------------------------------------------
// Payload structs для HTTP routes
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePayload {
    pub place: PlaceNewDto,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenamePayload {
    pub id: i64,
    pub name: String,
    pub version: i64,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetPathVariantPayload {
    pub id: i64,
    pub path_variant_override: Option<String>,
    pub version: i64,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MovePayload {
    pub id: i64,
    pub new_parent_id: Option<i64>,
    pub version: i64,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchivePayload {
    pub id: i64,
    pub version: i64,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnarchivePayload {
    pub id: i64,
    pub version: i64,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletePayload {
    pub id: i64,
    pub version: i64,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPayload {
    pub id: i64,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListChildrenPayload {
    pub parent_id: Option<i64>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListAllPayload {
    pub include_archived: bool,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubtreeStatsPayload {
    pub root_id: i64,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentsPayload {
    pub root_id: i64,
    pub nested: bool,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchPayload {
    pub query: String,
}

// ---------------------------------------------------------------------------
// Handlers — mutations (Action::MutatePlaces, D-20 Admin-only)
// ---------------------------------------------------------------------------

pub async fn handler_create(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(payload): Json<CreatePayload>,
) -> Result<Json<PlaceDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_places_create(&ctx, &identity, payload.place)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_rename(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(payload): Json<RenamePayload>,
) -> Result<Json<PlaceDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_places_rename(&ctx, &identity, payload.id, payload.name, payload.version)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_set_path_variant(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(payload): Json<SetPathVariantPayload>,
) -> Result<Json<PlaceDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_places_set_path_variant(
            &ctx,
            &identity,
            payload.id,
            payload.path_variant_override,
            payload.version,
        )
        .await
        .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_move(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(payload): Json<MovePayload>,
) -> Result<Json<PlaceDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_places_move(
            &ctx,
            &identity,
            payload.id,
            payload.new_parent_id,
            payload.version,
        )
        .await
        .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_archive(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(payload): Json<ArchivePayload>,
) -> Result<Json<()>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    build_places_archive(&ctx, &identity, payload.id, payload.version)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(()))
}

pub async fn handler_unarchive(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(payload): Json<UnarchivePayload>,
) -> Result<Json<()>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    build_places_unarchive(&ctx, &identity, payload.id, payload.version)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(()))
}

pub async fn handler_delete(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(payload): Json<DeletePayload>,
) -> Result<Json<()>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    build_places_delete(&ctx, &identity, payload.id, payload.version)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(()))
}

// ---------------------------------------------------------------------------
// Handlers — reads (Action::ReadPlaces, D-20 Admin|Manager)
// ---------------------------------------------------------------------------

pub async fn handler_get(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(payload): Json<GetPayload>,
) -> Result<Json<PlaceDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_places_get(&ctx, &identity, payload.id)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_list_children(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(payload): Json<ListChildrenPayload>,
) -> Result<Json<Vec<PlaceDto>>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_places_list_children(&ctx, &identity, payload.parent_id)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_list_all(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(payload): Json<ListAllPayload>,
) -> Result<Json<Vec<PlaceDto>>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_places_list_all(&ctx, &identity, payload.include_archived)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_subtree_stats(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(payload): Json<SubtreeStatsPayload>,
) -> Result<Json<SubtreeStatsDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_places_subtree_stats(&ctx, &identity, payload.root_id)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_contents(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(payload): Json<ContentsPayload>,
) -> Result<Json<Vec<PlaceContentDto>>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_places_contents(&ctx, &identity, payload.root_id, payload.nested)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_search(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(payload): Json<SearchPayload>,
) -> Result<Json<Vec<PlacePathDto>>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_places_search(&ctx, &identity, payload.query)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router() -> Router<AppCtx> {
    Router::new()
        .route("/api/v1/places_create", post(handler_create))
        .route("/api/v1/places_rename", post(handler_rename))
        .route(
            "/api/v1/places_set_path_variant",
            post(handler_set_path_variant),
        )
        .route("/api/v1/places_move", post(handler_move))
        .route("/api/v1/places_archive", post(handler_archive))
        .route("/api/v1/places_unarchive", post(handler_unarchive))
        .route("/api/v1/places_delete", post(handler_delete))
        .route("/api/v1/places_get", post(handler_get))
        .route("/api/v1/places_list_children", post(handler_list_children))
        .route("/api/v1/places_list_all", post(handler_list_all))
        .route("/api/v1/places_subtree_stats", post(handler_subtree_stats))
        .route("/api/v1/places_contents", post(handler_contents))
        .route("/api/v1/places_search", post(handler_search))
}

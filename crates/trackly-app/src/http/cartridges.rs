//! Cartridges axum HTTP routes — Phase 4 Plan 03.
//!
//! Mirrors `tauri_cmds::cartridges` via POST endpoints. The router is BUILT in
//! Plan 03 but NOT bound to a TCP listener — server-mode wiring is Phase 5.
//!
//! Phase 5 Plan 04: mutation handlers protected by `authorize(&identity, &Action::MutateCartridges)`.
//! Read handlers require only a valid session.

use axum::{extract::State, routing::post, Json, Router};
use tower_sessions::Session;

use crate::context::AppCtx;
use crate::dto::cartridge::{
    AuditEntryDto, CartridgeCountsDto, CartridgeCreateDto, CartridgeDto, CartridgeFilter,
    CartridgeListResponse, CartridgeModelCreateDto, CartridgeModelDto, CartridgeModelPatchDto,
    CartridgeTransitionPayload, LowStockItemDto, Pagination,
};
use crate::error_axum::AppErrorResponse;
use crate::http::auth::session_identity;
use crate::tauri_cmds::cartridges::{
    build_cartridge_models_create, build_cartridge_models_delete, build_cartridge_models_get,
    build_cartridge_models_list, build_cartridge_models_update, build_cartridges_create,
    build_cartridges_delete, build_cartridges_get, build_cartridges_get_history,
    build_cartridges_list, build_cartridges_low_stock, build_cartridges_search,
    build_cartridges_status_counts, build_cartridges_suggest_brand,
    build_cartridges_suggest_compat_printer, build_cartridges_suggest_location,
    build_cartridges_suggest_model, build_cartridges_transition, build_cartridges_update,
};

// ---------------------------------------------------------------------------
// Payload wrappers
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListPayload {
    pub filter: CartridgeFilter,
    pub pagination: Pagination,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPayload {
    /// i32 matches #[specta(type = i32)] in CartridgeDto — transport parity with Tauri (WR-05).
    pub id: i32,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePayload {
    pub payload: CartridgeCreateDto,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePayload {
    /// i32 matches #[specta(type = i32)] in CartridgeDto — transport parity with Tauri (WR-05).
    pub id: i32,
    pub version: i32,
    pub location: Option<String>,
    pub notes: Option<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletePayload {
    /// i32 matches #[specta(type = i32)] in CartridgeDto — transport parity with Tauri (WR-05).
    pub id: i32,
    pub version: i32,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransitionPayload {
    pub payload: CartridgeTransitionPayload,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchPayload {
    pub query: String,
    pub filter: CartridgeFilter,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelCreatePayload {
    pub payload: CartridgeModelCreateDto,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelUpdatePayload {
    pub payload: CartridgeModelPatchDto,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestBrandPayload {
    pub prefix: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestModelPayload {
    pub brand: String,
    pub prefix: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestCompatPayload {
    pub field: String,
    pub prefix: String,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

pub async fn handler_list(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<ListPayload>,
) -> Result<Json<CartridgeListResponse>, AppErrorResponse> {
    let _identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_cartridges_list(&ctx, p.filter, p.pagination)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_get(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<GetPayload>,
) -> Result<Json<CartridgeDto>, AppErrorResponse> {
    let _identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_cartridges_get(&ctx, p.id as i64)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_create(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<CreatePayload>,
) -> Result<Json<CartridgeDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_cartridges_create(&ctx, &identity, p.payload)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_update(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<UpdatePayload>,
) -> Result<Json<CartridgeDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_cartridges_update(
            &ctx,
            &identity,
            p.id as i64,
            p.version as i64,
            p.location,
            p.notes,
        )
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
    build_cartridges_delete(&ctx, &identity, p.id as i64, p.version as i64)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(()))
}

pub async fn handler_transition(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<TransitionPayload>,
) -> Result<Json<CartridgeDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_cartridges_transition(&ctx, &identity, p.payload)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_search(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<SearchPayload>,
) -> Result<Json<CartridgeListResponse>, AppErrorResponse> {
    let _identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_cartridges_search(&ctx, p.query, p.filter)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_status_counts(
    State(ctx): State<AppCtx>,
    session: Session,
) -> Result<Json<CartridgeCountsDto>, AppErrorResponse> {
    let _identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_cartridges_status_counts(&ctx)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_get_history(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<GetPayload>,
) -> Result<Json<Vec<AuditEntryDto>>, AppErrorResponse> {
    let _identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_cartridges_get_history(&ctx, p.id as i64)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_low_stock(
    State(ctx): State<AppCtx>,
    session: Session,
) -> Result<Json<Vec<LowStockItemDto>>, AppErrorResponse> {
    let _identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_cartridges_low_stock(&ctx)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_models_list(
    State(ctx): State<AppCtx>,
    session: Session,
) -> Result<Json<Vec<CartridgeModelDto>>, AppErrorResponse> {
    let _identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_cartridge_models_list(&ctx)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_models_get(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<GetPayload>,
) -> Result<Json<CartridgeModelDto>, AppErrorResponse> {
    let _identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_cartridge_models_get(&ctx, p.id as i64)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_models_create(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<ModelCreatePayload>,
) -> Result<Json<CartridgeModelDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_cartridge_models_create(&ctx, &identity, p.payload)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_models_update(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<ModelUpdatePayload>,
) -> Result<Json<CartridgeModelDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_cartridge_models_update(&ctx, &identity, p.payload)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_models_delete(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<DeletePayload>,
) -> Result<Json<()>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    build_cartridge_models_delete(&ctx, &identity, p.id as i64, p.version as i64)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(()))
}

pub async fn handler_suggest_brand(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<SuggestBrandPayload>,
) -> Result<Json<Vec<String>>, AppErrorResponse> {
    let _identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_cartridges_suggest_brand(&ctx, p.prefix)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_suggest_model(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<SuggestModelPayload>,
) -> Result<Json<Vec<String>>, AppErrorResponse> {
    let _identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_cartridges_suggest_model(&ctx, p.brand, p.prefix)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_suggest_compat_printer(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<SuggestCompatPayload>,
) -> Result<Json<Vec<String>>, AppErrorResponse> {
    let _identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_cartridges_suggest_compat_printer(&ctx, p.field, p.prefix)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_suggest_location(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<SuggestBrandPayload>,
) -> Result<Json<Vec<String>>, AppErrorResponse> {
    let _identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_cartridges_suggest_location(&ctx, p.prefix)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

// ---------------------------------------------------------------------------
// Router (built but NOT bound — Phase 5 wires it to the listener)
// ---------------------------------------------------------------------------

pub fn router() -> Router<AppCtx> {
    Router::new()
        .route("/api/v1/cartridges_list", post(handler_list))
        .route("/api/v1/cartridges_get", post(handler_get))
        .route("/api/v1/cartridges_create", post(handler_create))
        .route("/api/v1/cartridges_update", post(handler_update))
        .route("/api/v1/cartridges_delete", post(handler_delete))
        .route("/api/v1/cartridges_transition", post(handler_transition))
        .route("/api/v1/cartridges_search", post(handler_search))
        .route(
            "/api/v1/cartridges_status_counts",
            post(handler_status_counts),
        )
        .route("/api/v1/cartridges_get_history", post(handler_get_history))
        .route("/api/v1/cartridges_low_stock", post(handler_low_stock))
        .route("/api/v1/cartridge_models_list", post(handler_models_list))
        .route("/api/v1/cartridge_models_get", post(handler_models_get))
        .route(
            "/api/v1/cartridge_models_create",
            post(handler_models_create),
        )
        .route(
            "/api/v1/cartridge_models_update",
            post(handler_models_update),
        )
        .route(
            "/api/v1/cartridge_models_delete",
            post(handler_models_delete),
        )
        .route(
            "/api/v1/cartridges_suggest_brand",
            post(handler_suggest_brand),
        )
        .route(
            "/api/v1/cartridges_suggest_model",
            post(handler_suggest_model),
        )
        .route(
            "/api/v1/cartridges_suggest_compat_printer",
            post(handler_suggest_compat_printer),
        )
        .route(
            "/api/v1/cartridges_suggest_location",
            post(handler_suggest_location),
        )
}

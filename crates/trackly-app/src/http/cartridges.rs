//! Cartridges axum HTTP routes — Phase 4 Plan 03.
//!
//! Mirrors `tauri_cmds::cartridges` via POST endpoints. The router is BUILT in
//! Plan 03 but NOT bound to a TCP listener — server-mode wiring is Phase 5.

use axum::{extract::State, routing::post, Json, Router};

use crate::context::AppCtx;
use crate::dto::cartridge::{
    AuditEntryDto, CartridgeCountsDto, CartridgeCreateDto, CartridgeDto, CartridgeFilter,
    CartridgeListResponse, CartridgeModelCreateDto, CartridgeModelDto, CartridgeModelPatchDto,
    CartridgeTransitionPayload, LowStockItemDto, Pagination,
};
use crate::error_axum::AppErrorResponse;
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
pub struct ListPayload {
    pub filter: CartridgeFilter,
    pub pagination: Pagination,
}

#[derive(serde::Deserialize)]
pub struct GetPayload {
    pub id: i64,
}

#[derive(serde::Deserialize)]
pub struct CreatePayload {
    pub payload: CartridgeCreateDto,
}

#[derive(serde::Deserialize)]
pub struct UpdatePayload {
    pub id: i64,
    pub version: i64,
    pub location: Option<String>,
    pub notes: Option<String>,
}

#[derive(serde::Deserialize)]
pub struct DeletePayload {
    pub id: i64,
    pub version: i64,
}

#[derive(serde::Deserialize)]
pub struct TransitionPayload {
    pub payload: CartridgeTransitionPayload,
}

#[derive(serde::Deserialize)]
pub struct SearchPayload {
    pub query: String,
    pub filter: CartridgeFilter,
}

#[derive(serde::Deserialize)]
pub struct ModelCreatePayload {
    pub payload: CartridgeModelCreateDto,
}

#[derive(serde::Deserialize)]
pub struct ModelUpdatePayload {
    pub payload: CartridgeModelPatchDto,
}

#[derive(serde::Deserialize)]
pub struct SuggestBrandPayload {
    pub prefix: String,
}

#[derive(serde::Deserialize)]
pub struct SuggestModelPayload {
    pub brand: String,
    pub prefix: String,
}

#[derive(serde::Deserialize)]
pub struct SuggestCompatPayload {
    pub field: String,
    pub prefix: String,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

pub async fn handler_list(
    State(ctx): State<AppCtx>,
    Json(p): Json<ListPayload>,
) -> Result<Json<CartridgeListResponse>, AppErrorResponse> {
    Ok(Json(
        build_cartridges_list(&ctx, p.filter, p.pagination)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_get(
    State(ctx): State<AppCtx>,
    Json(p): Json<GetPayload>,
) -> Result<Json<CartridgeDto>, AppErrorResponse> {
    Ok(Json(
        build_cartridges_get(&ctx, p.id)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_create(
    State(ctx): State<AppCtx>,
    Json(p): Json<CreatePayload>,
) -> Result<Json<CartridgeDto>, AppErrorResponse> {
    Ok(Json(
        build_cartridges_create(&ctx, p.payload)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_update(
    State(ctx): State<AppCtx>,
    Json(p): Json<UpdatePayload>,
) -> Result<Json<CartridgeDto>, AppErrorResponse> {
    Ok(Json(
        build_cartridges_update(&ctx, p.id, p.version, p.location, p.notes)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_delete(
    State(ctx): State<AppCtx>,
    Json(p): Json<DeletePayload>,
) -> Result<Json<()>, AppErrorResponse> {
    build_cartridges_delete(&ctx, p.id, p.version)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(()))
}

pub async fn handler_transition(
    State(ctx): State<AppCtx>,
    Json(p): Json<TransitionPayload>,
) -> Result<Json<CartridgeDto>, AppErrorResponse> {
    Ok(Json(
        build_cartridges_transition(&ctx, p.payload)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_search(
    State(ctx): State<AppCtx>,
    Json(p): Json<SearchPayload>,
) -> Result<Json<CartridgeListResponse>, AppErrorResponse> {
    Ok(Json(
        build_cartridges_search(&ctx, p.query, p.filter)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_status_counts(
    State(ctx): State<AppCtx>,
) -> Result<Json<CartridgeCountsDto>, AppErrorResponse> {
    Ok(Json(
        build_cartridges_status_counts(&ctx)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_get_history(
    State(ctx): State<AppCtx>,
    Json(p): Json<GetPayload>,
) -> Result<Json<Vec<AuditEntryDto>>, AppErrorResponse> {
    Ok(Json(
        build_cartridges_get_history(&ctx, p.id)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_low_stock(
    State(ctx): State<AppCtx>,
) -> Result<Json<Vec<LowStockItemDto>>, AppErrorResponse> {
    Ok(Json(
        build_cartridges_low_stock(&ctx)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_models_list(
    State(ctx): State<AppCtx>,
) -> Result<Json<Vec<CartridgeModelDto>>, AppErrorResponse> {
    Ok(Json(
        build_cartridge_models_list(&ctx)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_models_get(
    State(ctx): State<AppCtx>,
    Json(p): Json<GetPayload>,
) -> Result<Json<CartridgeModelDto>, AppErrorResponse> {
    Ok(Json(
        build_cartridge_models_get(&ctx, p.id)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_models_create(
    State(ctx): State<AppCtx>,
    Json(p): Json<ModelCreatePayload>,
) -> Result<Json<CartridgeModelDto>, AppErrorResponse> {
    Ok(Json(
        build_cartridge_models_create(&ctx, p.payload)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_models_update(
    State(ctx): State<AppCtx>,
    Json(p): Json<ModelUpdatePayload>,
) -> Result<Json<CartridgeModelDto>, AppErrorResponse> {
    Ok(Json(
        build_cartridge_models_update(&ctx, p.payload)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_models_delete(
    State(ctx): State<AppCtx>,
    Json(p): Json<DeletePayload>,
) -> Result<Json<()>, AppErrorResponse> {
    build_cartridge_models_delete(&ctx, p.id, p.version)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(()))
}

pub async fn handler_suggest_brand(
    State(ctx): State<AppCtx>,
    Json(p): Json<SuggestBrandPayload>,
) -> Result<Json<Vec<String>>, AppErrorResponse> {
    Ok(Json(
        build_cartridges_suggest_brand(&ctx, p.prefix)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_suggest_model(
    State(ctx): State<AppCtx>,
    Json(p): Json<SuggestModelPayload>,
) -> Result<Json<Vec<String>>, AppErrorResponse> {
    Ok(Json(
        build_cartridges_suggest_model(&ctx, p.brand, p.prefix)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_suggest_compat_printer(
    State(ctx): State<AppCtx>,
    Json(p): Json<SuggestCompatPayload>,
) -> Result<Json<Vec<String>>, AppErrorResponse> {
    Ok(Json(
        build_cartridges_suggest_compat_printer(&ctx, p.field, p.prefix)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_suggest_location(
    State(ctx): State<AppCtx>,
    Json(p): Json<SuggestBrandPayload>,
) -> Result<Json<Vec<String>>, AppErrorResponse> {
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
        .route(
            "/api/v1/cartridges_get_history",
            post(handler_get_history),
        )
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

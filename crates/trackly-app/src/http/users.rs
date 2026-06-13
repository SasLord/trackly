//! Users HTTP routes — Plan 03.
//!
//! POST /api/v1/users_* CRUD + change_password + reset_password.
//! Все маршруты защищены session middleware (из protected router'а в build_router).
//!
//! Паттерн аналогичен http/devices.rs.

use axum::{extract::State, routing::post, Json, Router};
use serde::Deserialize;
use tower_sessions::Session;

use trackly_core::error::AppError;

use crate::context::AppCtx;
use crate::dto::auth::{
    ChangePasswordRequest, UserDto, UserFilter, UserListResponse, UserNew, UserPatch,
};
use crate::dto::device::Pagination;
use crate::error_axum::AppErrorResponse;
use crate::http::auth::session_identity;

// ---------------------------------------------------------------------------
// Payload structs
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct ListPayload {
    pub filter: UserFilter,
    pub pagination: Pagination,
}

#[derive(Debug, Deserialize)]
pub struct CreatePayload {
    // Renamed from `new` to `user_new` to match Tauri command parameter name
    // (TypeScript reserves `new` as a keyword — same payload shape on both transports).
    pub user_new: UserNew,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePayload {
    pub id: i64,
    pub version: i64,
    pub patch: UserPatch,
}

#[derive(Debug, Deserialize)]
pub struct DeletePayload {
    pub id: i64,
    pub version: i64,
}

#[derive(Debug, Deserialize)]
pub struct ChangePasswordPayload {
    pub user_id: i64,
    pub req: ChangePasswordRequest,
}

#[derive(Debug, Deserialize)]
pub struct ResetPasswordPayload {
    pub user_id: i64,
    pub new_password: String,
}

// ---------------------------------------------------------------------------
// build_* helpers
// ---------------------------------------------------------------------------

pub async fn build_users_list(
    ctx: &AppCtx,
    _session: &Session,
    filter: UserFilter,
    pagination: Pagination,
) -> Result<UserListResponse, AppError> {
    ctx.auth.list_users(filter, pagination).await
}

pub async fn build_users_create(
    ctx: &AppCtx,
    session: &Session,
    new: UserNew,
) -> Result<UserDto, AppError> {
    let caller = session_identity(session).await?;
    ctx.auth.create_user(new, &caller).await
}

pub async fn build_users_update(
    ctx: &AppCtx,
    session: &Session,
    id: i64,
    version: i64,
    patch: UserPatch,
) -> Result<UserDto, AppError> {
    let caller = session_identity(session).await?;
    ctx.auth.update_user(id, version, patch, &caller).await
}

pub async fn build_users_delete(
    ctx: &AppCtx,
    session: &Session,
    id: i64,
    version: i64,
) -> Result<(), AppError> {
    let caller = session_identity(session).await?;
    ctx.auth.delete_user(id, version, &caller).await
}

pub async fn build_users_change_password(
    ctx: &AppCtx,
    user_id: i64,
    req: ChangePasswordRequest,
) -> Result<(), AppError> {
    ctx.auth.change_password(user_id, req).await
}

pub async fn build_users_reset_password(
    ctx: &AppCtx,
    session: &Session,
    user_id: i64,
    new_password: String,
) -> Result<(), AppError> {
    let caller = session_identity(session).await?;
    ctx.auth.reset_password(user_id, new_password, &caller).await
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

pub async fn handler_list(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(payload): Json<ListPayload>,
) -> Result<Json<UserListResponse>, AppErrorResponse> {
    Ok(Json(
        build_users_list(&ctx, &session, payload.filter, payload.pagination)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_create(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(payload): Json<CreatePayload>,
) -> Result<Json<UserDto>, AppErrorResponse> {
    Ok(Json(
        build_users_create(&ctx, &session, payload.user_new)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_update(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(payload): Json<UpdatePayload>,
) -> Result<Json<UserDto>, AppErrorResponse> {
    Ok(Json(
        build_users_update(&ctx, &session, payload.id, payload.version, payload.patch)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_delete(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(payload): Json<DeletePayload>,
) -> Result<Json<()>, AppErrorResponse> {
    build_users_delete(&ctx, &session, payload.id, payload.version)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(()))
}

pub async fn handler_change_password(
    State(ctx): State<AppCtx>,
    Json(payload): Json<ChangePasswordPayload>,
) -> Result<Json<()>, AppErrorResponse> {
    build_users_change_password(&ctx, payload.user_id, payload.req)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(()))
}

pub async fn handler_reset_password(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(payload): Json<ResetPasswordPayload>,
) -> Result<Json<()>, AppErrorResponse> {
    build_users_reset_password(&ctx, &session, payload.user_id, payload.new_password)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(()))
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router() -> Router<AppCtx> {
    Router::new()
        .route("/api/v1/users_list", post(handler_list))
        .route("/api/v1/users_create", post(handler_create))
        .route("/api/v1/users_update", post(handler_update))
        .route("/api/v1/users_delete", post(handler_delete))
        .route("/api/v1/users_change_password", post(handler_change_password))
        .route("/api/v1/users_reset_password", post(handler_reset_password))
}

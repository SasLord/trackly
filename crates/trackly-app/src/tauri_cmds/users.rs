//! Users Tauri commands — Plan 03.
//!
//! Реализует D-Desktop-01/02: identity resolution зависит от desktop_lock_enabled.
//!
//! `resolve_tauri_identity()` определяет caller:
//! - lock=OFF → `Identity::trusted_admin()` (D-Desktop-01)
//! - lock=ON  → `ctx.auth.desktop_identity()` (D-Desktop-02: verified desktop identity)
//!
//! **ВАЖНО:** НЕ использовать hardcoded `Identity::trusted_admin()` напрямую в
//! обработчиках — нарушение T-05-DL. Всегда через `resolve_tauri_identity()`.
//!
//! Паттерн: `build_*` helper + тонкий `#[tauri::command] #[specta::specta]` wrapper.

use crate::context::AppCtx;
use crate::dto::auth::{
    ChangePasswordRequest, UserDto, UserFilter, UserListResponse, UserNew, UserPatch,
};
use crate::dto::device::Pagination;
use trackly_core::auth::Identity;
use trackly_core::error::AppError;

// ---------------------------------------------------------------------------
// Identity helper (D-Desktop-01/02)
// ---------------------------------------------------------------------------

/// Определить Identity для Tauri-команд.
///
/// - desktop_lock_enabled=false → trusted_admin() (D-Desktop-01 unlocked mode)
/// - desktop_lock_enabled=true  → desktop_identity() (D-Desktop-02 locked mode)
///
/// Это единственное место, где desktop_lock_enabled проверяется для users CRUD.
/// НЕ инлайнить в handlers — нарушение T-05-DL.
pub async fn resolve_tauri_identity(ctx: &AppCtx) -> Result<Identity, AppError> {
    let lock = ctx.auth.get_desktop_lock_enabled().await?;
    if lock {
        Ok(ctx.auth.desktop_identity().await)
    } else {
        Ok(Identity::trusted_admin())
    }
}

// ---------------------------------------------------------------------------
// build_* helpers
// ---------------------------------------------------------------------------

pub async fn build_users_list_tauri(
    ctx: &AppCtx,
    filter: UserFilter,
    pagination: Pagination,
) -> Result<UserListResponse, AppError> {
    // CR-03: authorize the listing — resolve the desktop identity.
    let caller = resolve_tauri_identity(ctx).await?;
    ctx.auth.list_users(filter, pagination, &caller).await
}

pub async fn build_users_create_tauri(
    ctx: &AppCtx,
    new: UserNew,
) -> Result<UserDto, AppError> {
    let caller = resolve_tauri_identity(ctx).await?;
    ctx.auth.create_user(new, &caller).await
}

pub async fn build_users_update_tauri(
    ctx: &AppCtx,
    id: i64,
    version: i64,
    patch: UserPatch,
) -> Result<UserDto, AppError> {
    let caller = resolve_tauri_identity(ctx).await?;
    ctx.auth.update_user(id, version, patch, &caller).await
}

pub async fn build_users_delete_tauri(
    ctx: &AppCtx,
    id: i64,
    version: i64,
) -> Result<(), AppError> {
    let caller = resolve_tauri_identity(ctx).await?;
    ctx.auth.delete_user(id, version, &caller).await
}

pub async fn build_users_change_password_tauri(
    ctx: &AppCtx,
    req: ChangePasswordRequest,
) -> Result<(), AppError> {
    // CR-02: never trust a caller-supplied user_id — derive the subject from
    // the resolved desktop identity. In unlocked mode (or when admin count is
    // ambiguous) `resolve_tauri_identity` yields trusted_admin (user_id = None),
    // which has no concrete account whose own password could be changed.
    let caller = resolve_tauri_identity(ctx).await?;
    let user_id = caller.user_id.ok_or(AppError::Unauthorized)?;
    ctx.auth.change_password(user_id, req).await
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

#[tauri::command]
#[specta::specta]
pub async fn users_list(
    state: tauri::State<'_, AppCtx>,
    filter: UserFilter,
    pagination: Pagination,
) -> Result<UserListResponse, AppError> {
    build_users_list_tauri(state.inner(), filter, pagination).await
}

#[tauri::command]
#[specta::specta]
pub async fn users_create(
    state: tauri::State<'_, AppCtx>,
    user_new: UserNew,
) -> Result<UserDto, AppError> {
    build_users_create_tauri(state.inner(), user_new).await
}

#[tauri::command]
#[specta::specta]
pub async fn users_update(
    state: tauri::State<'_, AppCtx>,
    id: i32,
    version: i32,
    patch: UserPatch,
) -> Result<UserDto, AppError> {
    build_users_update_tauri(state.inner(), id as i64, version as i64, patch).await
}

#[tauri::command]
#[specta::specta]
pub async fn users_delete(
    state: tauri::State<'_, AppCtx>,
    id: i32,
    version: i32,
) -> Result<(), AppError> {
    build_users_delete_tauri(state.inner(), id as i64, version as i64).await
}

#[tauri::command]
#[specta::specta]
pub async fn users_change_password(
    state: tauri::State<'_, AppCtx>,
    req: ChangePasswordRequest,
) -> Result<(), AppError> {
    // CR-02: user_id intentionally not accepted from the caller.
    build_users_change_password_tauri(state.inner(), req).await
}

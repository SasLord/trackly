//! Requests Tauri commands — Phase 6 Plan 03.
//!
//! Pattern (S-1): `build_*` helper + thin `#[tauri::command] #[specta::specta]`
//! wrapper. Both transports (Tauri invoke + axum POST) delegate to the same helper.
//!
//! `#[specta::specta]` MUST appear AFTER `#[tauri::command]`.
//!
//! Desktop push of `WsEvent::RequestStatusChanged` is handled by the global
//! `ws_broadcast` → `trackly-event` bridge wired in `main.rs`'s `.setup(...)`
//! (gap-closure: previously `requests_transition`/`requests_approve_ad_register`
//! each called `app.emit(...)` directly here — that is now redundant since
//! `RequestService::transition`/`approve_ad_register` already push the same
//! `WsEvent` onto `ctx.ws_broadcast`, which the bridge forwards to the
//! desktop webview. Removing the direct emits avoids double-firing the event
//! on desktop).

use crate::context::AppCtx;
use crate::dto::request::{
    ApproveAdRegisterDto, Pagination, RequestCategoryDto, RequestCountsDto, RequestCreateDto,
    RequestDto, RequestFilter, RequestHistoryEntryDto, RequestListResponse,
    RequestTransitionPayload,
};
use crate::tauri_cmds::users::resolve_tauri_identity;
use trackly_core::auth::{authorize, Action, Identity};
use trackly_core::error::AppError;

// ---------------------------------------------------------------------------
// build_* helpers (shared with axum handlers)
// ---------------------------------------------------------------------------

/// Список заявок. REQ-06/T-09-11: `ad_register` заявки видны только Admin —
/// фильтрация на уровне SQL внутри `RequestService::list`.
pub async fn build_requests_list(
    ctx: &AppCtx,
    caller: &Identity,
    filter: RequestFilter,
    pagination: Pagination,
) -> Result<RequestListResponse, AppError> {
    ctx.requests
        .list(filter.into(), pagination.into(), caller)
        .await
}

pub async fn build_requests_get(
    ctx: &AppCtx,
    caller: &Identity,
    id: i64,
) -> Result<RequestDto, AppError> {
    ctx.requests.get(id, caller).await
}

/// Создание заявки — разрешено всем авторизованным (Action::CreateRequest).
pub async fn build_requests_create(
    ctx: &AppCtx,
    caller: &Identity,
    dto: RequestCreateDto,
) -> Result<RequestDto, AppError> {
    authorize(caller, &Action::CreateRequest)?;
    ctx.requests.create(dto, caller).await
}

/// Переход статуса заявки — только Admin | Manager (Action::TransitionRequests).
pub async fn build_requests_transition(
    ctx: &AppCtx,
    caller: &Identity,
    payload: RequestTransitionPayload,
) -> Result<RequestDto, AppError> {
    authorize(caller, &Action::TransitionRequests)?;
    ctx.requests.transition(payload, caller).await
}

/// Approve an `ad_register` request with an admin-selected role — Admin only
/// (Action::ManageUsers, T-09-12 role-elevation gate).
pub async fn build_requests_approve_ad_register(
    ctx: &AppCtx,
    caller: &Identity,
    payload: ApproveAdRegisterDto,
) -> Result<RequestDto, AppError> {
    ctx.requests.approve_ad_register(payload, caller).await
}

/// Счётчики по статусам (для switch-bar).
pub async fn build_requests_counts(
    ctx: &AppCtx,
    caller: &Identity,
) -> Result<RequestCountsDto, AppError> {
    ctx.requests.counts(caller).await
}

/// История заявки из audit_log (REQ-07).
pub async fn build_requests_get_history(
    ctx: &AppCtx,
    caller: &Identity,
    id: i64,
) -> Result<Vec<RequestHistoryEntryDto>, AppError> {
    ctx.requests.get_history(id, caller).await
}

/// Список категорий заявок (request_categories) — `{ id, name }` (D-CAT-01).
///
/// Returns id alongside name so the create-request form can send a correct
/// `category_id` instead of relying on a hardcoded client-side array.
pub async fn build_requests_list_categories(
    ctx: &AppCtx,
) -> Result<Vec<RequestCategoryDto>, AppError> {
    let readers = ctx.readers.clone();
    tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        let mut stmt = conn
            .prepare("SELECT id, name FROM request_categories ORDER BY name")
            .map_err(|e| trackly_core::error::AppError::Internal {
                source_chain: format!("prepare: {e}"),
            })?;
        let categories = stmt
            .query_map([], |row| {
                Ok(RequestCategoryDto {
                    id: row.get(0)?,
                    name: row.get(1)?,
                })
            })
            .map_err(|e| trackly_core::error::AppError::Internal {
                source_chain: format!("query: {e}"),
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(categories)
    })
    .await
    .map_err(|e| AppError::Internal {
        source_chain: format!("spawn_blocking: {e}"),
    })?
}

// ---------------------------------------------------------------------------
// Thin Tauri wrappers
// ---------------------------------------------------------------------------

#[tauri::command]
#[specta::specta]
pub async fn requests_list(
    state: tauri::State<'_, AppCtx>,
    filter: RequestFilter,
    pagination: Pagination,
) -> Result<RequestListResponse, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_requests_list(state.inner(), &caller, filter, pagination).await
}

#[tauri::command]
#[specta::specta]
pub async fn requests_get(
    state: tauri::State<'_, AppCtx>,
    id: i32,
) -> Result<RequestDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_requests_get(state.inner(), &caller, id as i64).await
}

#[tauri::command]
#[specta::specta]
pub async fn requests_create(
    state: tauri::State<'_, AppCtx>,
    dto: RequestCreateDto,
) -> Result<RequestDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_requests_create(state.inner(), &caller, dto).await
}

/// Desktop push: `RequestService::transition` pushes `WsEvent::RequestStatusChanged`
/// onto `ctx.ws_broadcast`; the global bridge in `main.rs` forwards it to the
/// desktop webview as `trackly-event` (gap-closure — see module doc comment).
#[tauri::command]
#[specta::specta]
pub async fn requests_transition(
    state: tauri::State<'_, AppCtx>,
    payload: RequestTransitionPayload,
) -> Result<RequestDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_requests_transition(state.inner(), &caller, payload).await
}

/// Approve an `ad_register` request — Admin only. Desktop push on success via
/// the `ws_broadcast` bridge (see module doc comment).
#[tauri::command]
#[specta::specta]
pub async fn requests_approve_ad_register(
    state: tauri::State<'_, AppCtx>,
    payload: ApproveAdRegisterDto,
) -> Result<RequestDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_requests_approve_ad_register(state.inner(), &caller, payload).await
}

#[tauri::command]
#[specta::specta]
pub async fn requests_counts(
    state: tauri::State<'_, AppCtx>,
) -> Result<RequestCountsDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_requests_counts(state.inner(), &caller).await
}

#[tauri::command]
#[specta::specta]
pub async fn requests_list_categories(
    state: tauri::State<'_, AppCtx>,
) -> Result<Vec<RequestCategoryDto>, AppError> {
    build_requests_list_categories(state.inner()).await
}

#[tauri::command]
#[specta::specta]
pub async fn requests_get_history(
    state: tauri::State<'_, AppCtx>,
    id: i32,
) -> Result<Vec<RequestHistoryEntryDto>, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_requests_get_history(state.inner(), &caller, id as i64).await
}

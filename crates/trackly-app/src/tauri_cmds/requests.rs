//! Requests Tauri commands — Phase 6 Plan 03.
//!
//! Pattern (S-1): `build_*` helper + thin `#[tauri::command] #[specta::specta]`
//! wrapper. Both transports (Tauri invoke + axum POST) delegate to the same helper.
//!
//! `#[specta::specta]` MUST appear AFTER `#[tauri::command]`.
//!
//! `requests_transition` использует `tauri::AppHandle` для desktop push
//! WsEvent::RequestStatusChanged через `app.emit("trackly-event", ...)` (D-Notify-01).

use crate::context::AppCtx;
use crate::dto::printer::WsEvent;
// tauri::Emitter trait is needed for app.emit() in Tauri 2.x.
use tauri::Emitter;
use crate::dto::request::{
    Pagination, RequestCountsDto, RequestCreateDto, RequestDto, RequestFilter, RequestListResponse,
    RequestTransitionPayload,
};
use crate::tauri_cmds::users::resolve_tauri_identity;
use trackly_core::auth::{authorize, Action, Identity};
use trackly_core::error::AppError;

// ---------------------------------------------------------------------------
// build_* helpers (shared with axum handlers)
// ---------------------------------------------------------------------------

pub async fn build_requests_list(
    ctx: &AppCtx,
    filter: RequestFilter,
    pagination: Pagination,
) -> Result<RequestListResponse, AppError> {
    ctx.requests.list(filter.into(), pagination.into()).await
}

pub async fn build_requests_get(ctx: &AppCtx, id: i64) -> Result<RequestDto, AppError> {
    ctx.requests.get(id).await
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

/// Счётчики по статусам (для switch-bar).
pub async fn build_requests_counts(ctx: &AppCtx) -> Result<RequestCountsDto, AppError> {
    ctx.requests.counts().await
}

/// Список категорий заявок (request_categories).
pub async fn build_requests_list_categories(ctx: &AppCtx) -> Result<Vec<String>, AppError> {
    // In Phase 6 v1: return hardcoded list (V024 migration seeds request_categories).
    // Phase 7 will wire to a real categories service.
    let readers = ctx.readers.clone();
    tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        let mut stmt = conn
            .prepare("SELECT name FROM request_categories ORDER BY name")
            .map_err(|e| trackly_core::error::AppError::Internal {
                source_chain: format!("prepare: {e}"),
            })?;
        let names = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| trackly_core::error::AppError::Internal {
                source_chain: format!("query: {e}"),
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(names)
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
    build_requests_list(state.inner(), filter, pagination).await
}

#[tauri::command]
#[specta::specta]
pub async fn requests_get(
    state: tauri::State<'_, AppCtx>,
    id: i32,
) -> Result<RequestDto, AppError> {
    build_requests_get(state.inner(), id as i64).await
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

/// Desktop push: после перехода отправляет `trackly-event` через AppHandle.
#[tauri::command]
#[specta::specta]
pub async fn requests_transition(
    state: tauri::State<'_, AppCtx>,
    app: tauri::AppHandle,
    payload: RequestTransitionPayload,
) -> Result<RequestDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    let result = build_requests_transition(state.inner(), &caller, payload).await?;
    // Desktop push (no WS server needed — Tauri emits to all webview windows).
    app.emit(
        "trackly-event",
        &WsEvent::RequestStatusChanged {
            request_id: result.id,
            new_status: result.status.clone(),
        },
    )
    .ok();
    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub async fn requests_counts(
    state: tauri::State<'_, AppCtx>,
) -> Result<RequestCountsDto, AppError> {
    build_requests_counts(state.inner()).await
}

#[tauri::command]
#[specta::specta]
pub async fn requests_list_categories(
    state: tauri::State<'_, AppCtx>,
) -> Result<Vec<String>, AppError> {
    build_requests_list_categories(state.inner()).await
}

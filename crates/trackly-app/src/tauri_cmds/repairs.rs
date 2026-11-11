//! Команды ремонтных заявок (Phase 41.4, plan 14).
//!
//! Схема S-1: `build_repairs_*` — чистая функция, общая для Tauri-команды и
//! axum-обработчика (`http::repairs`). Права проверяются дважды: здесь и внутри
//! `RepairService` (build не должен быть единственной защитой).

use crate::context::AppCtx;
use crate::dto::repair::{
    OpenRepairRefDto, RepairAssignDeviceDto, RepairAssignResultDto, RepairCancelDto,
    RepairCompleteDto, RepairCreateDto, RepairCreateResultDto, RepairDeviceHistoryDto,
    RepairDeviceOptionDto, RepairLastPlacesDto, RepairMergeDto, RepairMergeResultDto,
    RepairMoveChoiceDto, RepairReceiveDto, RepairRejectDto, RepairSendDto, RepairStateDto,
    RepairTransitionResultDto,
};
use crate::tauri_cmds::users::resolve_tauri_identity;
use trackly_core::auth::{authorize, Action, Identity};
use trackly_core::error::AppError;

// ---------------------------------------------------------------------------
// build_* helpers (shared with axum handlers)
// ---------------------------------------------------------------------------

pub async fn build_repairs_device_options(
    ctx: &AppCtx,
    caller: &Identity,
) -> Result<Vec<RepairDeviceOptionDto>, AppError> {
    authorize(caller, &Action::CreateRequest)?;
    ctx.repairs.device_options(caller).await
}

pub async fn build_repairs_create(
    ctx: &AppCtx,
    caller: &Identity,
    dto: RepairCreateDto,
) -> Result<RepairCreateResultDto, AppError> {
    authorize(caller, &Action::CreateRequest)?;
    ctx.repairs.create(dto, caller).await
}

pub async fn build_repairs_get_state(
    ctx: &AppCtx,
    caller: &Identity,
    request_id: i64,
) -> Result<RepairStateDto, AppError> {
    authorize(caller, &Action::CreateRequest)?;
    ctx.repairs.get_state(request_id, caller).await
}

pub async fn build_repairs_device_history(
    ctx: &AppCtx,
    caller: &Identity,
    device_id: i64,
) -> Result<RepairDeviceHistoryDto, AppError> {
    authorize(caller, &Action::TransitionRequests)?;
    ctx.repairs.device_history(device_id, caller).await
}

pub async fn build_repairs_last_places(
    ctx: &AppCtx,
    caller: &Identity,
) -> Result<RepairLastPlacesDto, AppError> {
    authorize(caller, &Action::TransitionRequests)?;
    ctx.repairs.last_places(caller).await
}

pub async fn build_repairs_open_for_devices(
    ctx: &AppCtx,
    caller: &Identity,
    device_ids: Vec<i64>,
) -> Result<Vec<OpenRepairRefDto>, AppError> {
    authorize(caller, &Action::TransitionRequests)?;
    ctx.repairs
        .open_requests_for_devices(device_ids, caller)
        .await
}

pub async fn build_repairs_accept(
    ctx: &AppCtx,
    caller: &Identity,
    request_id: i64,
    version: i64,
    choice: RepairMoveChoiceDto,
) -> Result<RepairTransitionResultDto, AppError> {
    authorize(caller, &Action::TransitionRequests)?;
    ctx.repairs
        .accept(request_id, version, choice, caller)
        .await
}

pub async fn build_repairs_move_to_storage(
    ctx: &AppCtx,
    caller: &Identity,
    request_id: i64,
    version: i64,
    place_id: i64,
) -> Result<RepairTransitionResultDto, AppError> {
    authorize(caller, &Action::TransitionRequests)?;
    ctx.repairs
        .move_to_storage(request_id, version, place_id, caller)
        .await
}

pub async fn build_repairs_send(
    ctx: &AppCtx,
    caller: &Identity,
    dto: RepairSendDto,
) -> Result<RepairTransitionResultDto, AppError> {
    authorize(caller, &Action::TransitionRequests)?;
    ctx.repairs.send(dto, caller).await
}

pub async fn build_repairs_receive(
    ctx: &AppCtx,
    caller: &Identity,
    dto: RepairReceiveDto,
) -> Result<RepairTransitionResultDto, AppError> {
    authorize(caller, &Action::TransitionRequests)?;
    ctx.repairs.receive(dto, caller).await
}

pub async fn build_repairs_complete(
    ctx: &AppCtx,
    caller: &Identity,
    dto: RepairCompleteDto,
) -> Result<RepairTransitionResultDto, AppError> {
    authorize(caller, &Action::TransitionRequests)?;
    ctx.repairs.complete(dto, caller).await
}

pub async fn build_repairs_cancel_repair(
    ctx: &AppCtx,
    caller: &Identity,
    dto: RepairCancelDto,
) -> Result<RepairTransitionResultDto, AppError> {
    authorize(caller, &Action::TransitionRequests)?;
    ctx.repairs.cancel_repair(dto, caller).await
}

pub async fn build_repairs_reject(
    ctx: &AppCtx,
    caller: &Identity,
    dto: RepairRejectDto,
) -> Result<RepairTransitionResultDto, AppError> {
    authorize(caller, &Action::TransitionRequests)?;
    ctx.repairs.reject(dto, caller).await
}

pub async fn build_repairs_cancel_by_author(
    ctx: &AppCtx,
    caller: &Identity,
    dto: RepairCancelDto,
) -> Result<RepairTransitionResultDto, AppError> {
    authorize(caller, &Action::CancelOwnRequest)?;
    ctx.repairs.cancel_by_author(dto, caller).await
}

pub async fn build_repairs_assign_device(
    ctx: &AppCtx,
    caller: &Identity,
    dto: RepairAssignDeviceDto,
) -> Result<RepairAssignResultDto, AppError> {
    authorize(caller, &Action::TransitionRequests)?;
    ctx.repairs.assign_device(dto, caller).await
}

pub async fn build_repairs_merge_into_open(
    ctx: &AppCtx,
    caller: &Identity,
    dto: RepairMergeDto,
) -> Result<RepairMergeResultDto, AppError> {
    authorize(caller, &Action::TransitionRequests)?;
    ctx.repairs.merge_into_open(dto, caller).await
}

pub async fn build_repairs_link_scans(
    ctx: &AppCtx,
    caller: &Identity,
    visit_id: i64,
    attachment_ids: Vec<i64>,
) -> Result<RepairStateDto, AppError> {
    authorize(caller, &Action::TransitionRequests)?;
    ctx.repairs
        .link_scans(visit_id, attachment_ids, caller)
        .await
}

/// HTML акта для браузерной печати; в журнал «Акты» не пишет.
pub async fn build_repairs_render_act(
    ctx: &AppCtx,
    caller: &Identity,
    visit_id: i64,
) -> Result<String, AppError> {
    authorize(caller, &Action::TransitionRequests)?;
    ctx.repair_acts.render_html(visit_id, caller).await
}

// ---------------------------------------------------------------------------
// Tauri-обёртки (числа i32 — ограничение specta)
// ---------------------------------------------------------------------------

#[tauri::command]
#[specta::specta]
pub async fn repairs_device_options(
    state: tauri::State<'_, AppCtx>,
) -> Result<Vec<RepairDeviceOptionDto>, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_repairs_device_options(state.inner(), &caller).await
}

#[tauri::command]
#[specta::specta]
pub async fn repairs_create(
    state: tauri::State<'_, AppCtx>,
    dto: RepairCreateDto,
) -> Result<RepairCreateResultDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_repairs_create(state.inner(), &caller, dto).await
}

#[tauri::command]
#[specta::specta]
pub async fn repairs_get_state(
    state: tauri::State<'_, AppCtx>,
    request_id: i32,
) -> Result<RepairStateDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_repairs_get_state(state.inner(), &caller, request_id as i64).await
}

#[tauri::command]
#[specta::specta]
pub async fn repairs_last_places(
    state: tauri::State<'_, AppCtx>,
) -> Result<RepairLastPlacesDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_repairs_last_places(state.inner(), &caller).await
}

#[tauri::command]
#[specta::specta]
pub async fn repairs_open_for_devices(
    state: tauri::State<'_, AppCtx>,
    device_ids: Vec<i32>,
) -> Result<Vec<OpenRepairRefDto>, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_repairs_open_for_devices(
        state.inner(),
        &caller,
        device_ids.into_iter().map(i64::from).collect(),
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn repairs_accept(
    state: tauri::State<'_, AppCtx>,
    request_id: i32,
    version: i32,
    choice: RepairMoveChoiceDto,
) -> Result<RepairTransitionResultDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_repairs_accept(
        state.inner(),
        &caller,
        request_id as i64,
        version as i64,
        choice,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn repairs_move_to_storage(
    state: tauri::State<'_, AppCtx>,
    request_id: i32,
    version: i32,
    place_id: i32,
) -> Result<RepairTransitionResultDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_repairs_move_to_storage(
        state.inner(),
        &caller,
        request_id as i64,
        version as i64,
        place_id as i64,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn repairs_send(
    state: tauri::State<'_, AppCtx>,
    dto: RepairSendDto,
) -> Result<RepairTransitionResultDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_repairs_send(state.inner(), &caller, dto).await
}

#[tauri::command]
#[specta::specta]
pub async fn repairs_receive(
    state: tauri::State<'_, AppCtx>,
    dto: RepairReceiveDto,
) -> Result<RepairTransitionResultDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_repairs_receive(state.inner(), &caller, dto).await
}

#[tauri::command]
#[specta::specta]
pub async fn repairs_complete(
    state: tauri::State<'_, AppCtx>,
    dto: RepairCompleteDto,
) -> Result<RepairTransitionResultDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_repairs_complete(state.inner(), &caller, dto).await
}

#[tauri::command]
#[specta::specta]
pub async fn repairs_cancel_repair(
    state: tauri::State<'_, AppCtx>,
    dto: RepairCancelDto,
) -> Result<RepairTransitionResultDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_repairs_cancel_repair(state.inner(), &caller, dto).await
}

#[tauri::command]
#[specta::specta]
pub async fn repairs_reject(
    state: tauri::State<'_, AppCtx>,
    dto: RepairRejectDto,
) -> Result<RepairTransitionResultDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_repairs_reject(state.inner(), &caller, dto).await
}

#[tauri::command]
#[specta::specta]
pub async fn repairs_cancel_by_author(
    state: tauri::State<'_, AppCtx>,
    dto: RepairCancelDto,
) -> Result<RepairTransitionResultDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_repairs_cancel_by_author(state.inner(), &caller, dto).await
}

#[tauri::command]
#[specta::specta]
pub async fn repairs_assign_device(
    state: tauri::State<'_, AppCtx>,
    dto: RepairAssignDeviceDto,
) -> Result<RepairAssignResultDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_repairs_assign_device(state.inner(), &caller, dto).await
}

#[tauri::command]
#[specta::specta]
pub async fn repairs_device_history(
    state: tauri::State<'_, AppCtx>,
    device_id: i32,
) -> Result<RepairDeviceHistoryDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_repairs_device_history(state.inner(), &caller, device_id as i64).await
}

#[tauri::command]
#[specta::specta]
pub async fn repairs_merge_into_open(
    state: tauri::State<'_, AppCtx>,
    dto: RepairMergeDto,
) -> Result<RepairMergeResultDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_repairs_merge_into_open(state.inner(), &caller, dto).await
}

#[tauri::command]
#[specta::specta]
pub async fn repairs_link_scans(
    state: tauri::State<'_, AppCtx>,
    visit_id: i32,
    attachment_ids: Vec<i32>,
) -> Result<RepairStateDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_repairs_link_scans(
        state.inner(),
        &caller,
        visit_id as i64,
        attachment_ids.into_iter().map(i64::from).collect(),
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn repairs_render_act(
    state: tauri::State<'_, AppCtx>,
    visit_id: i32,
) -> Result<String, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_repairs_render_act(state.inner(), &caller, visit_id as i64).await
}

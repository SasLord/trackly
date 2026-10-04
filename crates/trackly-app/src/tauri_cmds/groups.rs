//! Group Tauri commands — Phase 41 Plan 13: exposes `GroupService`
//! (Plans 08, 10, 11, 12) over the Tauri transport.
//!
//! Паттерн: `build_*` helper + thin tauri-command/specta wrapper (как
//! `tauri_cmds/group_types.rs`). Оба транспорта (Tauri invoke + axum HTTP, см.
//! `http/groups.rs`) делегируют одним и тем же `build_groups_*` — бизнес-правил
//! в адаптерах нет.
//!
//! specta attribute ПОСЛЕ tauri-command attribute — требование tauri-specta v2 rc.21.
//!
//! GRP-09: каждое чтение гейтится `Action::ReadGroups`, каждая мутация —
//! `Action::MutateGroups` (Admin|Manager; employee отрезан). `GroupService` сам
//! вызывает `authorize()` первой строкой; здесь второй, осознанный гейт на
//! границе транспорта (защита в глубину).
//!
//! D-01: `groups_add_devices` — единственный обработчик добавления в состав для
//! обоих путей UI. D-17/D-18: `groups_move` — единственный обработчик «Перенести».

use crate::context::AppCtx;
use crate::dto::groups::{
    DeviceMembershipDto, GroupAddDevicesDto, GroupAddDevicesResultDto, GroupCardDto,
    GroupCompositionDto, GroupCreateDto, GroupDeleteResultDto, GroupDto, GroupMoveDto,
    GroupMoveResultDto, GroupRemoveDevicesDto, GroupSearchHitDto, GroupSetParentDto,
    GroupSetValuesDto, UserOptionDto,
};
use crate::tauri_cmds::users::resolve_tauri_identity;
use trackly_core::auth::{authorize, Action, Identity};
use trackly_core::error::AppError;

// ---------------------------------------------------------------------------
// build_* helpers — используются и Tauri, и axum транспортами
// ---------------------------------------------------------------------------

/// Чтение: `ReadGroups` (Admin|Manager).
pub async fn build_groups_list(ctx: &AppCtx, caller: &Identity) -> Result<Vec<GroupDto>, AppError> {
    authorize(caller, &Action::ReadGroups)?;
    ctx.groups.list_groups(caller).await
}

/// Чтение: `ReadGroups`.
pub async fn build_groups_get(
    ctx: &AppCtx,
    caller: &Identity,
    id: i64,
) -> Result<GroupDto, AppError> {
    authorize(caller, &Action::ReadGroups)?;
    ctx.groups.get_group(caller, id).await
}

/// Чтение: `ReadGroups` — карточка со свойствами и принтерами.
pub async fn build_groups_card(
    ctx: &AppCtx,
    caller: &Identity,
    id: i64,
) -> Result<GroupCardDto, AppError> {
    authorize(caller, &Action::ReadGroups)?;
    ctx.groups.card(caller, id).await
}

/// Чтение: `ReadGroups` — состав группы.
pub async fn build_groups_composition(
    ctx: &AppCtx,
    caller: &Identity,
    group_id: i64,
) -> Result<GroupCompositionDto, AppError> {
    authorize(caller, &Action::ReadGroups)?;
    ctx.groups.composition(caller, group_id).await
}

/// Чтение: `ReadGroups` — поиск группы для вложения/добавления.
pub async fn build_groups_search(
    ctx: &AppCtx,
    caller: &Identity,
    query: String,
    exclude_group_id: Option<i64>,
) -> Result<Vec<GroupSearchHitDto>, AppError> {
    authorize(caller, &Action::ReadGroups)?;
    ctx.groups.search(caller, query, exclude_group_id).await
}

/// Чтение: `ReadGroups` — членство набора устройств (пачкой).
pub async fn build_groups_for_devices(
    ctx: &AppCtx,
    caller: &Identity,
    device_ids: Vec<i64>,
) -> Result<Vec<DeviceMembershipDto>, AppError> {
    authorize(caller, &Action::ReadGroups)?;
    ctx.groups.for_devices(caller, device_ids).await
}

/// Чтение: `ReadGroups` (в отличие от `users_list`) — только id/full_name/login.
pub async fn build_groups_user_options(
    ctx: &AppCtx,
    caller: &Identity,
    query: String,
) -> Result<Vec<UserOptionDto>, AppError> {
    authorize(caller, &Action::ReadGroups)?;
    ctx.groups.user_options(caller, query).await
}

/// Мутация: `MutateGroups` (Admin|Manager).
pub async fn build_groups_create(
    ctx: &AppCtx,
    caller: &Identity,
    dto: GroupCreateDto,
) -> Result<GroupDto, AppError> {
    authorize(caller, &Action::MutateGroups)?;
    ctx.groups.create_group(caller, dto).await
}

/// Мутация: `MutateGroups` — переименование с CAS по версии.
pub async fn build_groups_update(
    ctx: &AppCtx,
    caller: &Identity,
    id: i64,
    version: i64,
    name: String,
) -> Result<GroupDto, AppError> {
    authorize(caller, &Action::MutateGroups)?;
    ctx.groups.update_group(caller, id, version, name).await
}

/// Мутация: `MutateGroups`.
pub async fn build_groups_delete(
    ctx: &AppCtx,
    caller: &Identity,
    id: i64,
) -> Result<GroupDeleteResultDto, AppError> {
    authorize(caller, &Action::MutateGroups)?;
    ctx.groups.delete_group(caller, id).await
}

/// Мутация: `MutateGroups` — вложение группы в группу (D-04).
pub async fn build_groups_set_parent(
    ctx: &AppCtx,
    caller: &Identity,
    dto: GroupSetParentDto,
) -> Result<GroupMoveResultDto, AppError> {
    authorize(caller, &Action::MutateGroups)?;
    ctx.groups
        .set_parent(caller, dto.id, dto.version, dto.parent_group_id)
        .await
}

/// Мутация: `MutateGroups` — единый обработчик добавления в состав (D-01).
pub async fn build_groups_add_devices(
    ctx: &AppCtx,
    caller: &Identity,
    dto: GroupAddDevicesDto,
) -> Result<GroupAddDevicesResultDto, AppError> {
    authorize(caller, &Action::MutateGroups)?;
    ctx.groups
        .add_devices(caller, dto.group_id, dto.device_ids)
        .await
}

/// Мутация: `MutateGroups` — вывод устройств из группы (D-02); число выведенных.
pub async fn build_groups_remove_devices(
    ctx: &AppCtx,
    caller: &Identity,
    dto: GroupRemoveDevicesDto,
) -> Result<i32, AppError> {
    authorize(caller, &Action::MutateGroups)?;
    ctx.groups
        .remove_devices(caller, dto.group_id, dto.device_ids)
        .await
}

/// Мутация: `MutateGroups` — единый обработчик «Перенести» (D-17/D-18).
pub async fn build_groups_move(
    ctx: &AppCtx,
    caller: &Identity,
    dto: GroupMoveDto,
) -> Result<GroupMoveResultDto, AppError> {
    authorize(caller, &Action::MutateGroups)?;
    ctx.groups
        .move_group(caller, dto.id, dto.version, dto.target_place_id)
        .await
}

/// Мутация: `MutateGroups` — запись значений свойств, серверная нормализация.
pub async fn build_groups_set_values(
    ctx: &AppCtx,
    caller: &Identity,
    dto: GroupSetValuesDto,
) -> Result<GroupCardDto, AppError> {
    authorize(caller, &Action::MutateGroups)?;
    ctx.groups
        .set_values(caller, dto.id, dto.version, dto.values)
        .await
}

// ---------------------------------------------------------------------------
// Tauri commands — thin wrappers
// ---------------------------------------------------------------------------

#[tauri::command]
#[specta::specta]
pub async fn groups_list(state: tauri::State<'_, AppCtx>) -> Result<Vec<GroupDto>, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_groups_list(state.inner(), &caller).await
}

#[tauri::command]
#[specta::specta]
pub async fn groups_get(state: tauri::State<'_, AppCtx>, id: i32) -> Result<GroupDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_groups_get(state.inner(), &caller, id as i64).await
}

#[tauri::command]
#[specta::specta]
pub async fn groups_card(
    state: tauri::State<'_, AppCtx>,
    id: i32,
) -> Result<GroupCardDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_groups_card(state.inner(), &caller, id as i64).await
}

#[tauri::command]
#[specta::specta]
pub async fn groups_composition(
    state: tauri::State<'_, AppCtx>,
    group_id: i32,
) -> Result<GroupCompositionDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_groups_composition(state.inner(), &caller, group_id as i64).await
}

#[tauri::command]
#[specta::specta]
pub async fn groups_search(
    state: tauri::State<'_, AppCtx>,
    query: String,
    exclude_group_id: Option<i32>,
) -> Result<Vec<GroupSearchHitDto>, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_groups_search(
        state.inner(),
        &caller,
        query,
        exclude_group_id.map(i64::from),
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn groups_for_devices(
    state: tauri::State<'_, AppCtx>,
    device_ids: Vec<i32>,
) -> Result<Vec<DeviceMembershipDto>, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_groups_for_devices(
        state.inner(),
        &caller,
        device_ids.into_iter().map(i64::from).collect(),
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn groups_user_options(
    state: tauri::State<'_, AppCtx>,
    query: String,
) -> Result<Vec<UserOptionDto>, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_groups_user_options(state.inner(), &caller, query).await
}

#[tauri::command]
#[specta::specta]
pub async fn groups_create(
    state: tauri::State<'_, AppCtx>,
    dto: GroupCreateDto,
) -> Result<GroupDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_groups_create(state.inner(), &caller, dto).await
}

#[tauri::command]
#[specta::specta]
pub async fn groups_update(
    state: tauri::State<'_, AppCtx>,
    id: i32,
    version: i32,
    name: String,
) -> Result<GroupDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_groups_update(state.inner(), &caller, id as i64, version as i64, name).await
}

#[tauri::command]
#[specta::specta]
pub async fn groups_delete(
    state: tauri::State<'_, AppCtx>,
    id: i32,
) -> Result<GroupDeleteResultDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_groups_delete(state.inner(), &caller, id as i64).await
}

#[tauri::command]
#[specta::specta]
pub async fn groups_set_parent(
    state: tauri::State<'_, AppCtx>,
    dto: GroupSetParentDto,
) -> Result<GroupMoveResultDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_groups_set_parent(state.inner(), &caller, dto).await
}

#[tauri::command]
#[specta::specta]
pub async fn groups_add_devices(
    state: tauri::State<'_, AppCtx>,
    dto: GroupAddDevicesDto,
) -> Result<GroupAddDevicesResultDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_groups_add_devices(state.inner(), &caller, dto).await
}

#[tauri::command]
#[specta::specta]
pub async fn groups_remove_devices(
    state: tauri::State<'_, AppCtx>,
    dto: GroupRemoveDevicesDto,
) -> Result<i32, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_groups_remove_devices(state.inner(), &caller, dto).await
}

#[tauri::command]
#[specta::specta]
pub async fn groups_move(
    state: tauri::State<'_, AppCtx>,
    dto: GroupMoveDto,
) -> Result<GroupMoveResultDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_groups_move(state.inner(), &caller, dto).await
}

#[tauri::command]
#[specta::specta]
pub async fn groups_set_values(
    state: tauri::State<'_, AppCtx>,
    dto: GroupSetValuesDto,
) -> Result<GroupCardDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_groups_set_values(state.inner(), &caller, dto).await
}

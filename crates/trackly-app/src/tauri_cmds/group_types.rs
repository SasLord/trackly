//! Group type Tauri commands — Phase 41 Plan 09: exposes `GroupTypeService`
//! (Plan 07) over the Tauri transport.
//!
//! Паттерн: `build_*` helper + thin tauri-command/specta wrapper (как
//! `tauri_cmds/places.rs`). Оба транспорта (Tauri invoke + axum HTTP, см.
//! `http/group_types.rs`) делегируют одним и тем же `build_group_types_*` /
//! `build_group_type_properties_*` — бизнес-правил в адаптерах нет.
//!
//! specta attribute ПОСЛЕ tauri-command attribute — требование tauri-specta v2 rc.21.
//!
//! GRP-09: каждая мутация типа/свойства гейтится `Action::ManageGroupTypes`
//! (только Admin), каждое чтение — `Action::ReadGroups` (Admin|Manager).
//! `GroupTypeService` сам вызывает `authorize()` первой строкой; здесь второй,
//! осознанный гейт на границе транспорта (защита в глубину).

use crate::context::AppCtx;
use crate::dto::group_types::{
    GroupRefDto, GroupTypeCreateDto, GroupTypeDto, GroupTypePropertyDto, GroupTypeUpdateDto,
    PropertyCreateDto, PropertyDeleteOutcomeDto, PropertyUpdateDto,
};
use crate::tauri_cmds::users::resolve_tauri_identity;
use trackly_core::auth::{authorize, Action, Identity};
use trackly_core::error::AppError;

// ---------------------------------------------------------------------------
// build_* helpers — используются и Tauri, и axum транспортами
// ---------------------------------------------------------------------------

/// Чтение: `ReadGroups` (Admin|Manager).
pub async fn build_group_types_list(
    ctx: &AppCtx,
    caller: &Identity,
    include_archived: bool,
) -> Result<Vec<GroupTypeDto>, AppError> {
    authorize(caller, &Action::ReadGroups)?;
    ctx.group_types.list_types(caller, include_archived).await
}

/// Мутация: `ManageGroupTypes` (Admin-only).
pub async fn build_group_types_create(
    ctx: &AppCtx,
    caller: &Identity,
    dto: GroupTypeCreateDto,
) -> Result<GroupTypeDto, AppError> {
    authorize(caller, &Action::ManageGroupTypes)?;
    ctx.group_types.create_type(caller, dto).await
}

/// Мутация: `ManageGroupTypes` (Admin-only). `code`/`behavior` в DTO сервис
/// отклоняет при любом отличии от текущих (SPEC 1).
pub async fn build_group_types_update(
    ctx: &AppCtx,
    caller: &Identity,
    id: i64,
    version: i64,
    dto: GroupTypeUpdateDto,
) -> Result<GroupTypeDto, AppError> {
    authorize(caller, &Action::ManageGroupTypes)?;
    ctx.group_types.update_type(caller, id, version, dto).await
}

/// Мутация: `ManageGroupTypes` (Admin-only).
pub async fn build_group_types_delete(
    ctx: &AppCtx,
    caller: &Identity,
    id: i64,
) -> Result<(), AppError> {
    authorize(caller, &Action::ManageGroupTypes)?;
    ctx.group_types.delete_type(caller, id).await
}

/// Мутация: `ManageGroupTypes` (Admin-only).
pub async fn build_group_type_properties_create(
    ctx: &AppCtx,
    caller: &Identity,
    dto: PropertyCreateDto,
) -> Result<GroupTypePropertyDto, AppError> {
    authorize(caller, &Action::ManageGroupTypes)?;
    ctx.group_types.create_property(caller, dto).await
}

/// Мутация: `ManageGroupTypes` (Admin-only).
pub async fn build_group_type_properties_update(
    ctx: &AppCtx,
    caller: &Identity,
    id: i64,
    version: i64,
    dto: PropertyUpdateDto,
) -> Result<GroupTypePropertyDto, AppError> {
    authorize(caller, &Action::ManageGroupTypes)?;
    ctx.group_types
        .update_property(caller, id, version, dto)
        .await
}

/// Мутация: `ManageGroupTypes` (Admin-only).
pub async fn build_group_type_properties_delete(
    ctx: &AppCtx,
    caller: &Identity,
    id: i64,
) -> Result<PropertyDeleteOutcomeDto, AppError> {
    authorize(caller, &Action::ManageGroupTypes)?;
    ctx.group_types.delete_property(caller, id).await
}

/// Мутация: `ManageGroupTypes` (Admin-only).
pub async fn build_group_type_properties_unarchive(
    ctx: &AppCtx,
    caller: &Identity,
    id: i64,
) -> Result<GroupTypePropertyDto, AppError> {
    authorize(caller, &Action::ManageGroupTypes)?;
    ctx.group_types.unarchive_property(caller, id).await
}

/// Мутация: `ManageGroupTypes` (Admin-only).
pub async fn build_group_type_properties_reorder(
    ctx: &AppCtx,
    caller: &Identity,
    type_id: i32,
    ordered_ids: Vec<i32>,
) -> Result<Vec<GroupTypePropertyDto>, AppError> {
    authorize(caller, &Action::ManageGroupTypes)?;
    ctx.group_types
        .reorder_properties(caller, type_id, ordered_ids)
        .await
}

/// Чтение: `ReadGroups` (Admin|Manager) — группы-нарушители обязательности (D-14).
pub async fn build_group_type_properties_empty_groups(
    ctx: &AppCtx,
    caller: &Identity,
    property_id: i64,
) -> Result<Vec<GroupRefDto>, AppError> {
    authorize(caller, &Action::ReadGroups)?;
    ctx.group_types
        .empty_groups_for_property(caller, property_id)
        .await
}

// ---------------------------------------------------------------------------
// Tauri commands — thin wrappers
// ---------------------------------------------------------------------------

#[tauri::command]
#[specta::specta]
pub async fn group_types_list(
    state: tauri::State<'_, AppCtx>,
    include_archived: bool,
) -> Result<Vec<GroupTypeDto>, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_group_types_list(state.inner(), &caller, include_archived).await
}

#[tauri::command]
#[specta::specta]
pub async fn group_types_create(
    state: tauri::State<'_, AppCtx>,
    dto: GroupTypeCreateDto,
) -> Result<GroupTypeDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_group_types_create(state.inner(), &caller, dto).await
}

#[tauri::command]
#[specta::specta]
pub async fn group_types_update(
    state: tauri::State<'_, AppCtx>,
    id: i32,
    version: i32,
    dto: GroupTypeUpdateDto,
) -> Result<GroupTypeDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_group_types_update(state.inner(), &caller, id as i64, version as i64, dto).await
}

#[tauri::command]
#[specta::specta]
pub async fn group_types_delete(state: tauri::State<'_, AppCtx>, id: i32) -> Result<(), AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_group_types_delete(state.inner(), &caller, id as i64).await
}

#[tauri::command]
#[specta::specta]
pub async fn group_type_properties_create(
    state: tauri::State<'_, AppCtx>,
    dto: PropertyCreateDto,
) -> Result<GroupTypePropertyDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_group_type_properties_create(state.inner(), &caller, dto).await
}

#[tauri::command]
#[specta::specta]
pub async fn group_type_properties_update(
    state: tauri::State<'_, AppCtx>,
    id: i32,
    version: i32,
    dto: PropertyUpdateDto,
) -> Result<GroupTypePropertyDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_group_type_properties_update(state.inner(), &caller, id as i64, version as i64, dto).await
}

#[tauri::command]
#[specta::specta]
pub async fn group_type_properties_delete(
    state: tauri::State<'_, AppCtx>,
    id: i32,
) -> Result<PropertyDeleteOutcomeDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_group_type_properties_delete(state.inner(), &caller, id as i64).await
}

#[tauri::command]
#[specta::specta]
pub async fn group_type_properties_unarchive(
    state: tauri::State<'_, AppCtx>,
    id: i32,
) -> Result<GroupTypePropertyDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_group_type_properties_unarchive(state.inner(), &caller, id as i64).await
}

#[tauri::command]
#[specta::specta]
pub async fn group_type_properties_reorder(
    state: tauri::State<'_, AppCtx>,
    type_id: i32,
    ordered_ids: Vec<i32>,
) -> Result<Vec<GroupTypePropertyDto>, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_group_type_properties_reorder(state.inner(), &caller, type_id, ordered_ids).await
}

#[tauri::command]
#[specta::specta]
pub async fn group_type_properties_empty_groups(
    state: tauri::State<'_, AppCtx>,
    property_id: i32,
) -> Result<Vec<GroupRefDto>, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_group_type_properties_empty_groups(state.inner(), &caller, property_id as i64).await
}

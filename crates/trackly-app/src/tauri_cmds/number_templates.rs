//! Numbering-template Tauri commands — Phase 40.2, Plan 05.
//!
//! Pattern (S-1, mirrors `tauri_cmds/acts.rs`): `build_*` helper + thin
//! `#[tauri::command] #[specta::specta]` wrapper. Both transports (Tauri
//! invoke + axum POST) delegate to the same helper.
//!
//! ## Two gate categories (RESEARCH Pitfall 3 — the single non-trivial
//! decision of this plan)
//!
//! **Group A — CRUD** (`number_templates_create`/`_update_mask`/`_delete`/
//! `_list`/`_preview_mask`): `Action::ManageSettings`, Admin-only. These
//! endpoints do NOT accept a `context` parameter at all — there is no
//! parameter that could "switch" the gate to a weaker Action (T-40.2-09).
//!
//! **Group B — usage** (`number_templates_list_by_context`/`_peek_next`/
//! `number_template_contexts_get`/`_set`/`number_templates_is_occupied`):
//! the gate is derived from a `context: TemplateContextDto` parameter via
//! [`action_for_context`] — `DeviceCreate|PrinterCreate` → `MutateDevices`,
//! `ActCreate` → `MutateActs`, `CartridgeCreate|DrumCreate` →
//! `MutateCartridges`. This is the SAME `Action` the popup itself is already
//! gated on (Admin|Manager) — conflating this with `ManageSettings` would
//! either lock a Manager out of autofill in their own popup, or hand them a
//! backdoor into template CRUD.
//!
//! `action_for_context` is a single pure function (not inlined per `build_*`)
//! so the 5 Group-B functions cannot independently drift on this mapping —
//! both `tauri_cmds` and `http` import the SAME function from this module.

use crate::context::AppCtx;
use crate::dto::number_template::{
    NextNumberDto, NumberTemplateDto, OccupyingRecordDto, TemplateContextDto, TemplateTypeDto,
};
use crate::tauri_cmds::users::resolve_tauri_identity;
use trackly_core::auth::{authorize, Action, Identity};
use trackly_core::domain::number_templates::TemplateContext;
use trackly_core::error::AppError;

// ---------------------------------------------------------------------------
// action_for_context — single source of truth for the Group B gate mapping
// ---------------------------------------------------------------------------

/// Which `Action` a Group-B (usage) endpoint must be gated on for a given
/// `context` — the SAME `Action` the popup that hosts that context is
/// already gated on (Admin|Manager), never `Action::ManageSettings`.
pub fn action_for_context(context: TemplateContext) -> Action {
    match context {
        TemplateContext::DeviceCreate | TemplateContext::PrinterCreate => Action::MutateDevices,
        TemplateContext::ActCreate => Action::MutateActs,
        TemplateContext::CartridgeCreate | TemplateContext::DrumCreate => Action::MutateCartridges,
    }
}

// ---------------------------------------------------------------------------
// build_* helpers (shared with axum handlers) — Group A: CRUD, Admin-only
// ---------------------------------------------------------------------------

/// Мутация: требует `caller` с правом `ManageSettings` (Admin only).
pub async fn build_number_templates_create(
    ctx: &AppCtx,
    caller: &Identity,
    template_type: TemplateTypeDto,
    mask: String,
) -> Result<NumberTemplateDto, AppError> {
    authorize(caller, &Action::ManageSettings)?;
    ctx.number_templates.create(template_type, mask).await
}

/// Мутация: требует `caller` с правом `ManageSettings` (Admin only).
pub async fn build_number_templates_update_mask(
    ctx: &AppCtx,
    caller: &Identity,
    id: i64,
    mask: String,
    version: i64,
) -> Result<NumberTemplateDto, AppError> {
    authorize(caller, &Action::ManageSettings)?;
    ctx.number_templates.update_mask(id, mask, version).await
}

/// Мутация: требует `caller` с правом `ManageSettings` (Admin only).
pub async fn build_number_templates_delete(
    ctx: &AppCtx,
    caller: &Identity,
    id: i64,
) -> Result<(), AppError> {
    authorize(caller, &Action::ManageSettings)?;
    ctx.number_templates.delete(id).await
}

/// Требует `caller` с правом `ManageSettings` (Admin only) — список для
/// экрана Настройки → Организация (план 10), не для попапов создания.
pub async fn build_number_templates_list(
    ctx: &AppCtx,
    caller: &Identity,
    template_type: Option<TemplateTypeDto>,
) -> Result<Vec<NumberTemplateDto>, AppError> {
    authorize(caller, &Action::ManageSettings)?;
    ctx.number_templates.list(template_type).await
}

/// Требует `caller` с правом `ManageSettings` (Admin only) — предпросмотр
/// маски ДО сохранения (NUM-03, план 10). `today_utc` берётся из `ctx.clock`,
/// не приходит с фронтенда.
pub async fn build_number_templates_preview_mask(
    ctx: &AppCtx,
    caller: &Identity,
    template_type: TemplateTypeDto,
    mask: String,
) -> Result<NextNumberDto, AppError> {
    authorize(caller, &Action::ManageSettings)?;
    let today_utc = ctx.clock.unix_seconds();
    ctx.number_templates
        .preview_mask(template_type, mask, today_utc)
        .await
}

// ---------------------------------------------------------------------------
// build_* helpers (shared with axum handlers) — Group B: usage,
// context-derived gate (Admin|Manager, per popup)
// ---------------------------------------------------------------------------

/// Для меню «Вставка» (NUM-06) — список шаблонов НУЖНОГО типа для данного
/// попапа. Гейт — `action_for_context(context)`, НЕ `ManageSettings`.
pub async fn build_number_templates_list_by_context(
    ctx: &AppCtx,
    caller: &Identity,
    context: TemplateContextDto,
) -> Result<Vec<NumberTemplateDto>, AppError> {
    let core_context: TemplateContext = context.into();
    authorize(caller, &action_for_context(core_context.clone()))?;
    let template_type: TemplateTypeDto = core_context.template_type().into();
    ctx.number_templates.list(Some(template_type)).await
}

/// `context` задаёт гейт прав и проверяется на соответствие типу шаблона
/// (BE-WR-01); в вычислении следующего номера не участвует.
pub async fn build_number_templates_peek_next(
    ctx: &AppCtx,
    caller: &Identity,
    template_id: i64,
    context: TemplateContextDto,
) -> Result<NextNumberDto, AppError> {
    let core_context: TemplateContext = context.into();
    authorize(caller, &action_for_context(core_context))?;
    ctx.number_templates
        .peek_next_for_context(template_id, context)
        .await
}

/// При открытии попапа: какой шаблон запомнен для `context`.
pub async fn build_number_template_contexts_get(
    ctx: &AppCtx,
    caller: &Identity,
    context: TemplateContextDto,
) -> Result<Option<i64>, AppError> {
    let core_context: TemplateContext = context.into();
    authorize(caller, &action_for_context(core_context))?;
    ctx.number_templates.get_context(context).await
}

/// Вызывается ИЗ волны 5 после успешного создания записи — не напрямую с
/// фронтенда как отдельное действие пользователя, но эндпоинт всё равно
/// существует и гейтится правильно (Dual access path требует одной
/// бизнес-логики для обоих транспортов).
pub async fn build_number_template_contexts_set(
    ctx: &AppCtx,
    caller: &Identity,
    context: TemplateContextDto,
    template_id: Option<i64>,
) -> Result<(), AppError> {
    let core_context: TemplateContext = context.into();
    authorize(caller, &action_for_context(core_context))?;
    ctx.number_templates
        .remember_context(context, template_id)
        .await
}

/// Живая подсказка D-02 И пред-проверка D-01 используют этот же эндпоинт.
pub async fn build_number_templates_is_occupied(
    ctx: &AppCtx,
    caller: &Identity,
    context: TemplateContextDto,
    candidate: String,
    exclude_id: Option<i64>,
) -> Result<Option<OccupyingRecordDto>, AppError> {
    let core_context: TemplateContext = context.into();
    authorize(caller, &action_for_context(core_context.clone()))?;
    let pool = core_context.template_type();
    ctx.number_templates
        .is_occupied(pool, &candidate, exclude_id)
        .await
}

// ---------------------------------------------------------------------------
// Thin Tauri wrappers
// ---------------------------------------------------------------------------

#[tauri::command]
#[specta::specta]
pub async fn number_templates_create(
    state: tauri::State<'_, AppCtx>,
    template_type: TemplateTypeDto,
    mask: String,
) -> Result<NumberTemplateDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_number_templates_create(state.inner(), &caller, template_type, mask).await
}

#[tauri::command]
#[specta::specta]
pub async fn number_templates_update_mask(
    state: tauri::State<'_, AppCtx>,
    id: i32,
    mask: String,
    version: i32,
) -> Result<NumberTemplateDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_number_templates_update_mask(state.inner(), &caller, id as i64, mask, version as i64)
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn number_templates_delete(
    state: tauri::State<'_, AppCtx>,
    id: i32,
) -> Result<(), AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_number_templates_delete(state.inner(), &caller, id as i64).await
}

#[tauri::command]
#[specta::specta]
pub async fn number_templates_list(
    state: tauri::State<'_, AppCtx>,
    template_type: Option<TemplateTypeDto>,
) -> Result<Vec<NumberTemplateDto>, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_number_templates_list(state.inner(), &caller, template_type).await
}

#[tauri::command]
#[specta::specta]
pub async fn number_templates_preview_mask(
    state: tauri::State<'_, AppCtx>,
    template_type: TemplateTypeDto,
    mask: String,
) -> Result<NextNumberDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_number_templates_preview_mask(state.inner(), &caller, template_type, mask).await
}

#[tauri::command]
#[specta::specta]
pub async fn number_templates_list_by_context(
    state: tauri::State<'_, AppCtx>,
    context: TemplateContextDto,
) -> Result<Vec<NumberTemplateDto>, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_number_templates_list_by_context(state.inner(), &caller, context).await
}

#[tauri::command]
#[specta::specta]
pub async fn number_templates_peek_next(
    state: tauri::State<'_, AppCtx>,
    template_id: i32,
    context: TemplateContextDto,
) -> Result<NextNumberDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_number_templates_peek_next(state.inner(), &caller, template_id as i64, context).await
}

#[tauri::command]
#[specta::specta]
pub async fn number_template_contexts_get(
    state: tauri::State<'_, AppCtx>,
    context: TemplateContextDto,
) -> Result<Option<i32>, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    let id = build_number_template_contexts_get(state.inner(), &caller, context).await?;
    Ok(id.map(|v| v as i32))
}

#[tauri::command]
#[specta::specta]
pub async fn number_template_contexts_set(
    state: tauri::State<'_, AppCtx>,
    context: TemplateContextDto,
    template_id: Option<i32>,
) -> Result<(), AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_number_template_contexts_set(
        state.inner(),
        &caller,
        context,
        template_id.map(|v| v as i64),
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn number_templates_is_occupied(
    state: tauri::State<'_, AppCtx>,
    context: TemplateContextDto,
    candidate: String,
    exclude_id: Option<i32>,
) -> Result<Option<OccupyingRecordDto>, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_number_templates_is_occupied(
        state.inner(),
        &caller,
        context,
        candidate,
        exclude_id.map(|v| v as i64),
    )
    .await
}

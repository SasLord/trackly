//! Templates Tauri commands — Phase 3 Plan 04.
//!
//! Phase 3: get_active + render_preview (для будущего редактора Phase 7).
//! Phase 7 добавит CRUD-эндпойнты и `template_render_with_diff` UX.

use crate::context::AppCtx;
use crate::tauri_cmds::users::resolve_tauri_identity;
use trackly_core::auth::{authorize, Action, Identity};
use trackly_core::error::AppError;

// ---------------------------------------------------------------------------
// build_* helpers
// ---------------------------------------------------------------------------

/// INT-29: ранее без сессии/роли. UI-потребителей нет (используется редактор через
/// `templates_list_for_editor`), гейт ManageSettings как у соседей редактора.
pub async fn build_templates_get_active(
    ctx: &AppCtx,
    caller: &Identity,
    kind: String,
) -> Result<String, AppError> {
    authorize(caller, &Action::ManageSettings)?;
    ctx.templates.get_active(&kind).await
}

/// Render preview по sample act id. В Phase 3 — тонкая обёртка над
/// `ActService::render_pdf` (для act_handover) или `render_acceptance_pdf`
/// (для act_acceptance — sample_act_id трактуется как device_id). Phase 7
/// расширит до полноценного редактора с sample-context (без зависимости от
/// реальных IDs из БД).
///
/// Phase 16 (D-10): возвращает HTML-строку — см. `build_acts_render_pdf`.
pub async fn build_templates_render_preview(
    ctx: &AppCtx,
    caller: &Identity,
    kind: String,
    sample_act_id: i64,
) -> Result<String, AppError> {
    // INT-29: рендерит реальный акт по id — без сессии и без роли отдавал данные.
    authorize(caller, &Action::ManageSettings)?;
    match kind.as_str() {
        "act_handover" => ctx.acts.render_pdf(sample_act_id).await,
        "act_acceptance" => {
            ctx.acts
                .render_acceptance_pdf(
                    sample_act_id,
                    "Иванов И.И.".to_string(),
                    "Петров П.П.".to_string(),
                    ctx.clock.unix_seconds(),
                )
                .await
        }
        other => Err(AppError::Validation {
            field: "kind".to_string(),
            message: format!("Unknown template kind: {other}"),
        }),
    }
}

// ---------------------------------------------------------------------------
// Thin Tauri wrappers
// ---------------------------------------------------------------------------

#[tauri::command]
#[specta::specta]
pub async fn templates_get_active(
    state: tauri::State<'_, AppCtx>,
    kind: String,
) -> Result<String, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_templates_get_active(state.inner(), &caller, kind).await
}

#[tauri::command]
#[specta::specta]
pub async fn templates_render_preview(
    state: tauri::State<'_, AppCtx>,
    kind: String,
    sample_act_id: i32,
) -> Result<String, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_templates_render_preview(state.inner(), &caller, kind, sample_act_id as i64).await
}

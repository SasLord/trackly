//! Organization Tauri commands — Phase 3 Plan 04.
//!
//! Phase 3: read-only (`organization_get`). Phase 7 добавит edit endpoint
//! + file-watcher.

use crate::context::AppCtx;
use crate::dto::OrgDto;
use crate::tauri_cmds::users::resolve_tauri_identity;
use trackly_core::auth::{authorize, Action, Identity};
use trackly_core::error::AppError;

// ---------------------------------------------------------------------------
// build_* helpers (shared with axum handlers)
// ---------------------------------------------------------------------------

/// INT-29: ранее отвечал без сессии и без роли (реквизиты организации). UI-потребителей нет.
pub async fn build_organization_get(ctx: &AppCtx, caller: &Identity) -> Result<OrgDto, AppError> {
    authorize(caller, &Action::ReadData)?;
    let org = ctx.organization.read().await?;
    Ok(org.into())
}

// ---------------------------------------------------------------------------
// Thin Tauri wrappers
// ---------------------------------------------------------------------------

#[tauri::command]
#[specta::specta]
pub async fn organization_get(state: tauri::State<'_, AppCtx>) -> Result<OrgDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_organization_get(state.inner(), &caller).await
}

//! Organization Tauri commands — Phase 3 Plan 04.
//!
//! Phase 3: read-only (`organization_get`). Phase 7 добавит edit endpoint
//! + file-watcher.

use crate::context::AppCtx;
use crate::dto::OrgDto;
use trackly_core::error::AppError;

// ---------------------------------------------------------------------------
// build_* helpers (shared with axum handlers)
// ---------------------------------------------------------------------------

pub async fn build_organization_get(ctx: &AppCtx) -> Result<OrgDto, AppError> {
    let org = ctx.organization.read().await?;
    Ok(org.into())
}

// ---------------------------------------------------------------------------
// Thin Tauri wrappers
// ---------------------------------------------------------------------------

#[tauri::command]
#[specta::specta]
pub async fn organization_get(state: tauri::State<'_, AppCtx>) -> Result<OrgDto, AppError> {
    build_organization_get(state.inner()).await
}

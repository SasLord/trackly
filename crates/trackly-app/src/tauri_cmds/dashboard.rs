//! Dashboard Tauri commands — Phase 7 Plan 07.
//!
//! Thin adapters over DashboardService. Both commands are read-only;
//! no role restriction beyond a valid Tauri (desktop) context.

use crate::context::AppCtx;
use crate::dto::reports::{ConsumptionPoint, DashboardWidgetDto, PeriodDto};
use crate::tauri_cmds::users::resolve_tauri_identity;
use trackly_core::auth::{authorize, Action, Identity};
use trackly_core::error::AppError;

// ---------------------------------------------------------------------------
// build_* helpers
// ---------------------------------------------------------------------------

pub async fn build_dashboard_get_all_widgets(
    ctx: &AppCtx,
    caller: &Identity,
    period: Option<PeriodDto>,
) -> Result<DashboardWidgetDto, AppError> {
    ctx.dashboard.get_all_widgets(caller, period).await
}

pub async fn build_dashboard_get_consumption_chart(
    ctx: &AppCtx,
    caller: &Identity,
    window_months: u8,
) -> Result<Vec<ConsumptionPoint>, AppError> {
    authorize(caller, &Action::ReadData)?;
    ctx.dashboard.get_consumption_chart(window_months).await
}

// ---------------------------------------------------------------------------
// Tauri command wrappers
// ---------------------------------------------------------------------------

#[tauri::command]
#[specta::specta]
pub async fn dashboard_get_all_widgets(
    state: tauri::State<'_, AppCtx>,
    period: Option<PeriodDto>,
) -> Result<DashboardWidgetDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_dashboard_get_all_widgets(state.inner(), &caller, period).await
}

#[tauri::command]
#[specta::specta]
pub async fn dashboard_get_consumption_chart(
    state: tauri::State<'_, AppCtx>,
    window_months: u8,
) -> Result<Vec<ConsumptionPoint>, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_dashboard_get_consumption_chart(state.inner(), &caller, window_months).await
}

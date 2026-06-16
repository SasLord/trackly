//! Dashboard Tauri commands — Phase 7 Plan 07.
//!
//! Thin adapters over DashboardService. Both commands are read-only;
//! no role restriction beyond a valid Tauri (desktop) context.

use crate::context::AppCtx;
use crate::dto::reports::{ConsumptionPoint, DashboardWidgetDto, PeriodDto};
use trackly_core::error::AppError;

// ---------------------------------------------------------------------------
// build_* helpers
// ---------------------------------------------------------------------------

pub async fn build_dashboard_get_all_widgets(
    ctx: &AppCtx,
    period: Option<PeriodDto>,
) -> Result<DashboardWidgetDto, AppError> {
    ctx.dashboard.get_all_widgets(period).await
}

pub async fn build_dashboard_get_consumption_chart(
    ctx: &AppCtx,
    window_months: u8,
) -> Result<Vec<ConsumptionPoint>, AppError> {
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
    build_dashboard_get_all_widgets(state.inner(), period).await
}

#[tauri::command]
#[specta::specta]
pub async fn dashboard_get_consumption_chart(
    state: tauri::State<'_, AppCtx>,
    window_months: u8,
) -> Result<Vec<ConsumptionPoint>, AppError> {
    build_dashboard_get_consumption_chart(state.inner(), window_months).await
}

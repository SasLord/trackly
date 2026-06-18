//! Dashboard axum HTTP handlers — Phase 7 Plan 07.
//!
//! All handlers delegate to build_* helpers from tauri_cmds/dashboard.rs.
//! Authentication: session_identity required (any authenticated user, read-only).

use axum::{extract::State, routing::post, Json, Router};
use tower_sessions::Session;

use crate::context::AppCtx;
use crate::dto::reports::{ConsumptionPoint, DashboardWidgetDto, PeriodDto};
use crate::error_axum::AppErrorResponse;
use crate::http::auth::session_identity;
use crate::tauri_cmds::dashboard::{
    build_dashboard_get_all_widgets, build_dashboard_get_consumption_chart,
};

// ---------------------------------------------------------------------------
// Payload structs
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetAllWidgetsPayload {
    pub period: Option<PeriodDto>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetConsumptionChartPayload {
    pub window_months: u8,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

pub async fn handler_get_all_widgets(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<GetAllWidgetsPayload>,
) -> Result<Json<DashboardWidgetDto>, AppErrorResponse> {
    let _identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_dashboard_get_all_widgets(&ctx, p.period)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_get_consumption_chart(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<GetConsumptionChartPayload>,
) -> Result<Json<Vec<ConsumptionPoint>>, AppErrorResponse> {
    let _identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_dashboard_get_consumption_chart(&ctx, p.window_months)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub fn router() -> Router<AppCtx> {
    Router::new()
        .route(
            "/api/v1/dashboard_get_all_widgets",
            post(handler_get_all_widgets),
        )
        .route(
            "/api/v1/dashboard_get_consumption_chart",
            post(handler_get_consumption_chart),
        )
}

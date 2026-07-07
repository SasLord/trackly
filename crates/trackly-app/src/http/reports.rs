//! Reports axum HTTP handlers — Phase 7 Plan 07.
//!
//! All handlers delegate to build_* helpers from tauri_cmds/reports.rs.
//! Authentication: session_identity required for all handlers (any authenticated user).
//! CSV export returns text/csv + UTF-8 BOM. PDF export (Phase 17: migrated off
//! krilla/DocSpec) now returns text/html; charset=utf-8 — an HTML document,
//! not PDF bytes, mirroring the acts HTML-print pipeline (Phase 16).

use axum::extract::State;
use axum::{
    http::{header, StatusCode},
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use tower_sessions::Session;

use crate::context::AppCtx;
use crate::dto::reports::{PeriodDto, ReportFilter, ReportResponse};
use crate::error_axum::AppErrorResponse;
use crate::http::auth::session_identity;
use crate::tauri_cmds::reports::{
    build_reports_export_csv, build_reports_export_pdf, build_reports_list_cartridge_consumption,
    build_reports_list_cartridge_in_stock, build_reports_list_cartridge_in_use,
    build_reports_list_cartridge_refills, build_reports_list_device_acts,
    build_reports_list_device_in_stock, build_reports_list_device_in_use,
    build_reports_list_device_returns,
};

// ---------------------------------------------------------------------------
// Payload structs
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListWithPeriodPayload {
    pub filter: ReportFilter,
    pub period: PeriodDto,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListSnapshotPayload {
    pub filter: ReportFilter,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportPayload {
    pub report_type: String,
    pub filter: ReportFilter,
    pub period: Option<PeriodDto>,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

pub async fn handler_list_device_acts(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<ListWithPeriodPayload>,
) -> Result<Json<ReportResponse>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_reports_list_device_acts(&ctx, &identity, p.filter, p.period)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_list_device_returns(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<ListWithPeriodPayload>,
) -> Result<Json<ReportResponse>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_reports_list_device_returns(&ctx, &identity, p.filter, p.period)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_list_device_in_use(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<ListSnapshotPayload>,
) -> Result<Json<ReportResponse>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_reports_list_device_in_use(&ctx, &identity, p.filter)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_list_device_in_stock(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<ListSnapshotPayload>,
) -> Result<Json<ReportResponse>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_reports_list_device_in_stock(&ctx, &identity, p.filter)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_list_cartridge_consumption(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<ListWithPeriodPayload>,
) -> Result<Json<ReportResponse>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_reports_list_cartridge_consumption(&ctx, &identity, p.filter, p.period)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_list_cartridge_refills(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<ListWithPeriodPayload>,
) -> Result<Json<ReportResponse>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_reports_list_cartridge_refills(&ctx, &identity, p.filter, p.period)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_list_cartridge_in_use(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<ListSnapshotPayload>,
) -> Result<Json<ReportResponse>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_reports_list_cartridge_in_use(&ctx, &identity, p.filter)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub async fn handler_list_cartridge_in_stock(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<ListSnapshotPayload>,
) -> Result<Json<ReportResponse>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_reports_list_cartridge_in_stock(&ctx, &identity, p.filter)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

/// Export report as CSV. Returns text/csv with UTF-8 BOM.
pub async fn handler_export_csv(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<ExportPayload>,
) -> Result<impl IntoResponse, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    let bytes = build_reports_export_csv(&ctx, &identity, p.report_type, p.filter, p.period)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/csv;charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"report.csv\"",
            ),
        ],
        bytes,
    ))
}

/// Export report as HTML (Phase 17). Returns text/html; charset=utf-8.
pub async fn handler_export_pdf(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<ExportPayload>,
) -> Result<impl IntoResponse, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    let html = build_reports_export_pdf(&ctx, &identity, p.report_type, p.filter, p.period)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        html,
    ))
}

pub fn router() -> Router<AppCtx> {
    Router::new()
        .route(
            "/api/v1/reports_list_device_acts",
            post(handler_list_device_acts),
        )
        .route(
            "/api/v1/reports_list_device_returns",
            post(handler_list_device_returns),
        )
        .route(
            "/api/v1/reports_list_device_in_use",
            post(handler_list_device_in_use),
        )
        .route(
            "/api/v1/reports_list_device_in_stock",
            post(handler_list_device_in_stock),
        )
        .route(
            "/api/v1/reports_list_cartridge_consumption",
            post(handler_list_cartridge_consumption),
        )
        .route(
            "/api/v1/reports_list_cartridge_refills",
            post(handler_list_cartridge_refills),
        )
        .route(
            "/api/v1/reports_list_cartridge_in_use",
            post(handler_list_cartridge_in_use),
        )
        .route(
            "/api/v1/reports_list_cartridge_in_stock",
            post(handler_list_cartridge_in_stock),
        )
        .route("/api/v1/reports_export_csv", post(handler_export_csv))
        .route("/api/v1/reports_export_pdf", post(handler_export_pdf))
}

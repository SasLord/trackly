//! Reports Tauri commands — Phase 7 Plan 07.
//!
//! Thin adapters over ReportService. All build_* helpers called by
//! both Tauri commands and axum HTTP handlers.
//!
//! Auth: no role restriction beyond a valid Tauri (desktop) context.
//! HTTP handlers add session_identity check in http/reports.rs.

use crate::context::AppCtx;
use crate::dto::reports::{PeriodDto, ReportCountsDto, ReportFilter, ReportResponse};
use crate::tauri_cmds::users::resolve_tauri_identity;
use trackly_core::auth::{authorize, Action, Identity};
use trackly_core::error::AppError;

// ---------------------------------------------------------------------------
// Columns per report type (used for CSV/PDF header rows)
// ---------------------------------------------------------------------------

fn columns_for(report_type: &str) -> Vec<&'static str> {
    match report_type {
        "device_acts" | "device_returns" => {
            vec![
                "number",
                "device_name",
                "giver_name",
                "receiver_name",
                "location_name",
            ]
        }
        "device_in_use" | "device_in_stock" => {
            vec!["device_name", "status_name", "location_name"]
        }
        "cartridge_consumption" | "cartridge_refills" => {
            vec!["code", "model_label", "status_name", "location_name"]
        }
        "cartridge_in_use" | "cartridge_in_stock" => {
            vec!["code", "model_label", "status_name", "location_name"]
        }
        _ => vec!["id"],
    }
}

/// Human-readable report name (used in PDF header).
fn report_display_name(report_type: &str) -> &'static str {
    match report_type {
        "device_acts" => "Акты приёма-передачи устройств",
        "device_returns" => "Акты возврата устройств",
        "device_in_use" => "Устройства в работе",
        "device_in_stock" => "Устройства на складе",
        "cartridge_consumption" => "Расход картриджей",
        "cartridge_refills" => "Заправки картриджей",
        "cartridge_in_use" => "Картриджи в работе",
        "cartridge_in_stock" => "Картриджи на складе",
        _ => "Отчёт",
    }
}

// ---------------------------------------------------------------------------
// build_* helpers
// ---------------------------------------------------------------------------

pub async fn build_reports_list_device_acts(
    ctx: &AppCtx,
    caller: &Identity,
    filter: ReportFilter,
    period: PeriodDto,
) -> Result<ReportResponse, AppError> {
    authorize(caller, &Action::ReadData)?;
    ctx.reports.list_device_acts(filter, period).await
}

pub async fn build_reports_list_device_returns(
    ctx: &AppCtx,
    caller: &Identity,
    filter: ReportFilter,
    period: PeriodDto,
) -> Result<ReportResponse, AppError> {
    authorize(caller, &Action::ReadData)?;
    ctx.reports.list_device_returns(filter, period).await
}

pub async fn build_reports_list_device_in_use(
    ctx: &AppCtx,
    caller: &Identity,
    filter: ReportFilter,
) -> Result<ReportResponse, AppError> {
    authorize(caller, &Action::ReadData)?;
    ctx.reports.list_device_in_use(filter).await
}

pub async fn build_reports_list_device_in_stock(
    ctx: &AppCtx,
    caller: &Identity,
    filter: ReportFilter,
) -> Result<ReportResponse, AppError> {
    authorize(caller, &Action::ReadData)?;
    ctx.reports.list_device_in_stock(filter).await
}

pub async fn build_reports_list_cartridge_consumption(
    ctx: &AppCtx,
    caller: &Identity,
    filter: ReportFilter,
    period: PeriodDto,
) -> Result<ReportResponse, AppError> {
    authorize(caller, &Action::ReadData)?;
    ctx.reports.list_cartridge_consumption(filter, period).await
}

pub async fn build_reports_list_cartridge_refills(
    ctx: &AppCtx,
    caller: &Identity,
    filter: ReportFilter,
    period: PeriodDto,
) -> Result<ReportResponse, AppError> {
    authorize(caller, &Action::ReadData)?;
    ctx.reports.list_cartridge_refills(filter, period).await
}

pub async fn build_reports_list_cartridge_in_use(
    ctx: &AppCtx,
    caller: &Identity,
    filter: ReportFilter,
) -> Result<ReportResponse, AppError> {
    authorize(caller, &Action::ReadData)?;
    ctx.reports.list_cartridge_in_use(filter).await
}

pub async fn build_reports_list_cartridge_in_stock(
    ctx: &AppCtx,
    caller: &Identity,
    filter: ReportFilter,
) -> Result<ReportResponse, AppError> {
    authorize(caller, &Action::ReadData)?;
    ctx.reports.list_cartridge_in_stock(filter).await
}

/// Export report rows as UTF-8 BOM CSV bytes.
pub async fn build_reports_export_csv(
    ctx: &AppCtx,
    caller: &Identity,
    report_type: String,
    filter: ReportFilter,
    period: Option<PeriodDto>,
) -> Result<Vec<u8>, AppError> {
    authorize(caller, &Action::ReadData)?;
    let rows = fetch_report(ctx, &report_type, filter, period).await?;
    let cols = columns_for(&report_type);
    ctx.reports.export_csv(&rows, &cols).await
}

/// Export report rows as PDF bytes.
pub async fn build_reports_export_pdf(
    ctx: &AppCtx,
    caller: &Identity,
    report_type: String,
    filter: ReportFilter,
    period: Option<PeriodDto>,
) -> Result<Vec<u8>, AppError> {
    authorize(caller, &Action::ReadData)?;
    let rows = fetch_report(ctx, &report_type, filter, period.clone()).await?;
    let org = ctx.org_db.get().await?;
    let logo_bytes = ctx.org_db.get_logo_bytes().await?;
    let logo_mime = if logo_bytes.is_some() {
        // Fetch mime separately from org_settings
        let readers = ctx.readers.clone();
        tokio::task::spawn_blocking(move || -> Result<Option<String>, AppError> {
            let conn = readers.acquire();
            conn.query_row("SELECT logo_mime FROM org_settings WHERE id = 1", [], |r| {
                r.get(0)
            })
            .map_err(trackly_infra::error_conversions::map_rusqlite)
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking logo_mime: {e}"),
        })??
    } else {
        None
    };
    let cols = columns_for(&report_type);
    let report_name = report_display_name(&report_type);
    let period_label = period
        .as_ref()
        .map(|p| format!("{} {}", p.mode, p.year.unwrap_or(0)))
        .unwrap_or_default();
    ctx.reports
        .export_pdf(
            &rows,
            report_name,
            &period_label,
            &org,
            logo_bytes,
            logo_mime,
            &cols,
        )
        .await
}

/// Dispatch to the right list method based on report_type string.
async fn fetch_report(
    ctx: &AppCtx,
    report_type: &str,
    filter: ReportFilter,
    period: Option<PeriodDto>,
) -> Result<ReportResponse, AppError> {
    let default_period = period.unwrap_or_else(|| PeriodDto {
        mode: "month".to_string(),
        year: Some(2026),
        month: Some(1),
        date_from: None,
        date_to: None,
    });

    match report_type {
        "device_acts" => ctx.reports.list_device_acts(filter, default_period).await,
        "device_returns" => {
            ctx.reports
                .list_device_returns(filter, default_period)
                .await
        }
        "device_in_use" => ctx.reports.list_device_in_use(filter).await,
        "device_in_stock" => ctx.reports.list_device_in_stock(filter).await,
        "cartridge_consumption" => {
            ctx.reports
                .list_cartridge_consumption(filter, default_period)
                .await
        }
        "cartridge_refills" => {
            ctx.reports
                .list_cartridge_refills(filter, default_period)
                .await
        }
        "cartridge_in_use" => ctx.reports.list_cartridge_in_use(filter).await,
        "cartridge_in_stock" => ctx.reports.list_cartridge_in_stock(filter).await,
        other => Err(AppError::Validation {
            field: "report_type".to_string(),
            message: format!("Unknown report type: {other}"),
        }),
    }
}

// ---------------------------------------------------------------------------
// Tauri command wrappers
// ---------------------------------------------------------------------------

#[tauri::command]
#[specta::specta]
pub async fn reports_list_device_acts(
    state: tauri::State<'_, AppCtx>,
    filter: ReportFilter,
    period: PeriodDto,
) -> Result<ReportResponse, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_reports_list_device_acts(state.inner(), &caller, filter, period).await
}

#[tauri::command]
#[specta::specta]
pub async fn reports_list_device_returns(
    state: tauri::State<'_, AppCtx>,
    filter: ReportFilter,
    period: PeriodDto,
) -> Result<ReportResponse, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_reports_list_device_returns(state.inner(), &caller, filter, period).await
}

#[tauri::command]
#[specta::specta]
pub async fn reports_list_device_in_use(
    state: tauri::State<'_, AppCtx>,
    filter: ReportFilter,
) -> Result<ReportResponse, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_reports_list_device_in_use(state.inner(), &caller, filter).await
}

#[tauri::command]
#[specta::specta]
pub async fn reports_list_device_in_stock(
    state: tauri::State<'_, AppCtx>,
    filter: ReportFilter,
) -> Result<ReportResponse, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_reports_list_device_in_stock(state.inner(), &caller, filter).await
}

#[tauri::command]
#[specta::specta]
pub async fn reports_list_cartridge_consumption(
    state: tauri::State<'_, AppCtx>,
    filter: ReportFilter,
    period: PeriodDto,
) -> Result<ReportResponse, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_reports_list_cartridge_consumption(state.inner(), &caller, filter, period).await
}

#[tauri::command]
#[specta::specta]
pub async fn reports_list_cartridge_refills(
    state: tauri::State<'_, AppCtx>,
    filter: ReportFilter,
    period: PeriodDto,
) -> Result<ReportResponse, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_reports_list_cartridge_refills(state.inner(), &caller, filter, period).await
}

#[tauri::command]
#[specta::specta]
pub async fn reports_list_cartridge_in_use(
    state: tauri::State<'_, AppCtx>,
    filter: ReportFilter,
) -> Result<ReportResponse, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_reports_list_cartridge_in_use(state.inner(), &caller, filter).await
}

#[tauri::command]
#[specta::specta]
pub async fn reports_list_cartridge_in_stock(
    state: tauri::State<'_, AppCtx>,
    filter: ReportFilter,
) -> Result<ReportResponse, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_reports_list_cartridge_in_stock(state.inner(), &caller, filter).await
}

#[tauri::command]
#[specta::specta]
pub async fn reports_export_csv(
    state: tauri::State<'_, AppCtx>,
    report_type: String,
    filter: ReportFilter,
    period: Option<PeriodDto>,
) -> Result<Vec<u8>, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_reports_export_csv(state.inner(), &caller, report_type, filter, period).await
}

#[tauri::command]
#[specta::specta]
pub async fn reports_export_pdf(
    state: tauri::State<'_, AppCtx>,
    report_type: String,
    filter: ReportFilter,
    period: Option<PeriodDto>,
) -> Result<Vec<u8>, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_reports_export_pdf(state.inner(), &caller, report_type, filter, period).await
}

// ---------------------------------------------------------------------------
// reports_get_report_counts (G2-5b)
// ---------------------------------------------------------------------------

/// Build helper for reports_get_report_counts — callable from both Tauri and HTTP.
pub async fn build_reports_get_report_counts(
    ctx: &AppCtx,
    caller: &Identity,
    domain: String,
    filter: ReportFilter,
    period: PeriodDto,
) -> Result<ReportCountsDto, AppError> {
    authorize(caller, &Action::ReadData)?;
    ctx.reports.get_report_counts(&domain, filter, period).await
}

/// Return per-tab row counts for ALL report-type tabs in the active domain.
///
/// Runs COUNT(*)-only SQL (no row collection) for all 4 tabs in a single
/// spawn_blocking task.  Non-fatal per-tab errors return count = 0.
#[tauri::command]
#[specta::specta]
pub async fn reports_get_report_counts(
    state: tauri::State<'_, AppCtx>,
    domain: String,
    filter: ReportFilter,
    period: PeriodDto,
) -> Result<ReportCountsDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_reports_get_report_counts(state.inner(), &caller, domain, filter, period).await
}

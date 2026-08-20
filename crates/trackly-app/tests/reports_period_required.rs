//! WR-07 regression gate: a period-scoped report export with `period: null`
//! must be REJECTED, not silently answered with a hardcoded month.
//!
//! Before this fix, `fetch_report` substituted `PeriodDto { mode: "month",
//! year: Some(2026), month: Some(1) }` for a missing period, so
//! `POST /api/v1/reports_export_pdf {"period": null}` restricted the rows to
//! January 2026 while `format_period_label(None)` printed an EMPTY subtitle —
//! the printed document then looked like a full-history report. Snapshot
//! report types (which legitimately take no period) must keep working.

use std::sync::Arc;

use tempfile::TempDir;
use tokio_util::sync::CancellationToken;

use trackly_app::context::AppCtx;
use trackly_app::dto::reports::ReportFilter;
use trackly_app::tauri_cmds::reports::{build_reports_export_csv, build_reports_export_pdf};
use trackly_core::auth::Identity;
use trackly_core::error::AppError;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::test_support::test_app_ctx::test_writer_and_readers;

/// Minimal fully-wired `AppCtx` fixture (mirrors `templates_status.rs` /
/// `specta_roundtrip.rs`).
/// Minimal fully-wired `AppCtx` fixture (mirrors `specta_roundtrip.rs`'s
/// `minimal_ctx`) — `build_templates_status` only reads `ctx.paths`, but the
/// function's signature (per this plan's interface) is `&AppCtx`.
fn minimal_ctx() -> (AppCtx, TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let paths =
        trackly_infra::Paths::resolve_for_exe_dir(dir.path().to_path_buf()).expect("resolve paths");
    let config = trackly_infra::AppConfig::default();
    let (_nb, log_guard) = tracing_appender::non_blocking(std::io::sink());
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let devices = Arc::new(trackly_app::services::DeviceService::new(
        writer.clone(),
        readers.clone(),
        clock.clone(),
    ));
    let paths_arc = Arc::new(paths);
    let organization = Arc::new(trackly_app::services::OrganizationService::new(
        paths_arc.clone(),
    ));
    let templates = Arc::new(trackly_app::services::TemplateService::new(
        writer.clone(),
        readers.clone(),
        clock.clone(),
    ));
    let pdf = Arc::new(trackly_app::pdf::PdfRenderer::new());
    let acts = Arc::new(
        trackly_app::services::ActService::new(writer.clone(), readers.clone(), clock.clone())
            .with_pdf_pipeline(templates.clone(), organization.clone(), pdf.clone()),
    );
    let cartridges = Arc::new(trackly_app::services::CartridgeService::new(
        writer.clone(),
        readers.clone(),
        clock.clone(),
    ));
    let ad_client: Arc<dyn trackly_core::ports::ad::AdClient + Send + Sync> =
        Arc::new(trackly_infra::ad::mock::MockAdClient::default_fixtures());
    let (ws_tx, _) = tokio::sync::broadcast::channel::<trackly_app::dto::printer::WsEvent>(128);
    let ws_broadcast = Arc::new(ws_tx);
    let auth = Arc::new(trackly_app::services::AuthService::new(
        writer.clone(),
        readers.clone(),
        clock.clone(),
        ad_client,
        ws_broadcast.clone(),
        Arc::new(trackly_infra::ad::directory_mock::MockAdDirectory::default_fixtures()),
    ));
    let (poll_tx, _poll_rx) = tokio::sync::mpsc::channel::<i64>(64);
    let snmp_client: Arc<dyn trackly_core::ports::snmp::SnmpClient + Send + Sync> =
        Arc::new(trackly_infra::snmp::mock::MockSnmpClient::default_fixtures());
    let printers = Arc::new(trackly_app::services::PrinterService::new(
        writer.clone(),
        readers.clone(),
        clock.clone(),
        snmp_client,
        poll_tx,
        ws_broadcast.clone(),
    ));
    let requests = Arc::new(trackly_app::services::RequestService::new(
        writer.clone(),
        readers.clone(),
        clock.clone(),
        ws_broadcast.clone(),
    ));
    let org_db = Arc::new(trackly_app::services::OrgDbService::new(
        writer.clone(),
        readers.clone(),
        clock.clone(),
        paths_arc.clone(),
    ));
    let reports = Arc::new(trackly_app::services::ReportService::new(
        writer.clone(),
        readers.clone(),
        clock.clone(),
        Arc::new(config.clone()),
        pdf.clone(),
    ));
    let dashboard = Arc::new(trackly_app::services::DashboardService::new(
        writer.clone(),
        readers.clone(),
        clock.clone(),
        Arc::new(config.clone()),
    ));
    let backup = Arc::new(trackly_app::services::BackupService::new(
        writer.clone(),
        readers.clone(),
        clock.clone(),
        dir.path().join("trackly.db"),
    ));
    let ctx = AppCtx {
        writer,
        readers,
        paths: paths_arc,
        org_db,
        reports,
        dashboard,
        backup,
        config: Arc::new(config),
        clock,
        shutdown: CancellationToken::new(),
        log_guard: Arc::new(log_guard),
        schema_version: 15,
        devices,
        acts,
        organization,
        templates,
        pdf,
        cartridges,
        auth,
        server_ctl: Arc::new(tokio::sync::Mutex::new(None)),
        printers,
        requests,
        ws_broadcast,
    };
    (ctx, dir)
}

/// Every period-scoped report type must reject a `None` period with
/// `Validation { field: "period" }`, on BOTH export transports.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn period_based_exports_reject_missing_period() {
    let (ctx, _dir) = minimal_ctx();
    let caller = Identity::trusted_admin();

    for report_type in [
        "device_acts",
        "device_returns",
        "cartridge_consumption",
        "cartridge_refills",
        "requests_all",
        "requests_open",
        "requests_in_progress",
        "requests_completed",
    ] {
        let pdf = build_reports_export_pdf(
            &ctx,
            &caller,
            report_type.to_string(),
            ReportFilter::default(),
            None,
        )
        .await;
        match pdf {
            Err(AppError::Validation { field, .. }) => assert_eq!(
                field, "period",
                "{report_type}: export_pdf must reject a missing period on the `period` field"
            ),
            other => panic!(
                "{report_type}: export_pdf with period=None must be rejected, not silently \
                 answered with a hardcoded month — got {other:?}"
            ),
        }

        let csv = build_reports_export_csv(
            &ctx,
            &caller,
            report_type.to_string(),
            ReportFilter::default(),
            None,
        )
        .await;
        match csv {
            Err(AppError::Validation { field, .. }) => assert_eq!(
                field, "period",
                "{report_type}: export_csv must reject a missing period on the `period` field"
            ),
            other => panic!(
                "{report_type}: export_csv with period=None must be rejected — got {other:?}"
            ),
        }
    }
}

/// Snapshot report types take no period by design (`ReportsPage.svelte` sends
/// `period: undefined` for exactly these) — they must keep succeeding, so the
/// rejection above is narrow rather than a blanket "period always required".
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn snapshot_exports_still_succeed_without_period() {
    let (ctx, _dir) = minimal_ctx();
    let caller = Identity::trusted_admin();

    for report_type in [
        "device_in_use",
        "device_in_stock",
        "cartridge_in_use",
        "cartridge_in_stock",
    ] {
        let csv = build_reports_export_csv(
            &ctx,
            &caller,
            report_type.to_string(),
            ReportFilter::default(),
            None,
        )
        .await;
        assert!(
            csv.is_ok(),
            "{report_type}: snapshot export must not require a period, got {csv:?}"
        );
    }
}

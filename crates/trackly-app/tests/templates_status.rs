//! Phase 34 Plan 05 — D-17 integration test: `build_templates_status`
//! reports `Current` for untouched materialized defaults and `Customized`
//! for a hand-edited file, per-file (unaffected siblings stay `Current`).

use std::sync::Arc;

use tempfile::TempDir;
use tokio_util::sync::CancellationToken;

use trackly_app::context::AppCtx;
use trackly_app::dto::reports::TemplateFileStatus;
use trackly_app::pdf::html_templates::{
    materialize_defaults_on_startup, resolve_templates_dir, DEFAULT_HTML_TEMPLATES,
};
use trackly_app::tauri_cmds::settings_org::build_templates_status;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::test_support::test_app_ctx::test_writer_and_readers;

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

/// Given a fresh templates_dir materialized (never upgraded/edited),
/// `build_templates_status` reports `Current` for all 4 filenames.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fresh_materialized_dir_reports_current_for_all_four() {
    let (ctx, _dir) = minimal_ctx();
    let templates_dir = resolve_templates_dir(&ctx.paths);
    materialize_defaults_on_startup(&templates_dir).expect("materialize defaults");

    let statuses = build_templates_status(&ctx)
        .await
        .expect("build_templates_status");

    assert_eq!(statuses.len(), DEFAULT_HTML_TEMPLATES.len());
    for entry in &statuses {
        assert!(
            matches!(entry.status, TemplateFileStatus::Current),
            "expected Current for untouched file {}, got {:?}",
            entry.filename,
            entry.status
        );
        assert_eq!(entry.templates_dir, templates_dir.display().to_string());
    }
}

/// Given the same dir with `act_handover.html` hand-edited to arbitrary
/// content not matching any known default/legacy body,
/// `build_templates_status` reports `Customized` for `act_handover.html`
/// specifically, and `Current` for the other 3 (unaffected).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn hand_edited_file_reports_customized_others_unaffected() {
    let (ctx, _dir) = minimal_ctx();
    let templates_dir = resolve_templates_dir(&ctx.paths);
    materialize_defaults_on_startup(&templates_dir).expect("materialize defaults");

    // Fictional, non-privacy-sensitive placeholder content — matches neither
    // the current bundled default nor any KNOWN_LEGACY_DEFAULTS snapshot.
    std::fs::write(
        templates_dir.join("act_handover.html"),
        "<html><body>Custom hand-edited template — fictional content only</body></html>",
    )
    .expect("write hand-edited act_handover.html");

    let statuses = build_templates_status(&ctx)
        .await
        .expect("build_templates_status");

    for entry in &statuses {
        if entry.filename == "act_handover.html" {
            assert!(
                matches!(entry.status, TemplateFileStatus::Customized),
                "expected Customized for hand-edited act_handover.html, got {:?}",
                entry.status
            );
        } else {
            assert!(
                matches!(entry.status, TemplateFileStatus::Current),
                "expected Current (unaffected) for {}, got {:?}",
                entry.filename,
                entry.status
            );
        }
    }
}

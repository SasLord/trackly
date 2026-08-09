//! Phase 34 Plan 05 — D-17 integration test: `build_templates_status`
//! reports `Current` for untouched materialized defaults and `Customized`
//! for a hand-edited file, per-file (unaffected siblings stay `Current`).
//!
//! WR-10: these tests WRITE template files, so they must never resolve the
//! directory through `resolve_templates_dir`, which honours the
//! `TRACKLY_TEMPLATES_DIR` env override — a documented, supported dev/test
//! variable. A developer with it exported (or any future in-process test that
//! leaks it) would otherwise have their real `templates/act_handover.html`
//! overwritten with the fixture's junk and the other three files
//! force-materialized. `build_templates_status` itself resolves via the env
//! var, so each test pins the variable to its OWN tempdir under `ENV_GUARD`
//! instead — hermetic in both directions.

use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};

use tempfile::TempDir;
use tokio_util::sync::CancellationToken;

use trackly_app::context::AppCtx;
use trackly_app::dto::reports::TemplateFileStatus;
use trackly_app::pdf::html_templates::{materialize_defaults_on_startup, DEFAULT_HTML_TEMPLATES};
use trackly_app::tauri_cmds::settings_org::build_templates_status;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::test_support::test_app_ctx::test_writer_and_readers;

/// Serializes tests that set `TRACKLY_TEMPLATES_DIR` — `std::env` is
/// process-global and Rust test threads run in parallel by default (mirrors
/// the `ENV_GUARD` pattern in `pdf/html_templates.rs`).
static ENV_GUARD: Mutex<()> = Mutex::new(());

/// Pins `TRACKLY_TEMPLATES_DIR` to this test's own tempdir and returns the
/// guard the caller must hold for the whole test. Never resolves — always
/// binds — so nothing outside the fixture can be written to.
fn pin_templates_dir(dir: &Path) -> MutexGuard<'static, ()> {
    // A previous panicking test may have poisoned the mutex; the data is `()`
    // so recovering is safe and keeps one failure from cascading.
    let guard = ENV_GUARD.lock().unwrap_or_else(|e| e.into_inner());
    // SAFETY: guarded by ENV_GUARD for the duration of the calling test.
    unsafe {
        std::env::set_var("TRACKLY_TEMPLATES_DIR", dir);
    }
    guard
}

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
    let templates_dir = ctx.paths.templates_dir().to_path_buf();
    let _env_guard = pin_templates_dir(&templates_dir);
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
    let templates_dir = ctx.paths.templates_dir().to_path_buf();
    let _env_guard = pin_templates_dir(&templates_dir);
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

/// WR-03: a file that EXISTS but cannot be decoded as UTF-8 must report
/// `Unreadable`, not `Current`.
///
/// The realistic trigger on the target (Windows) platform is an admin editing
/// `act_handover.html` in Notepad and saving it as ANSI/Windows-1251 —
/// Cyrillic content guarantees the bytes are not valid UTF-8. Their edits then
/// silently do nothing (the embedded default renders instead), and before this
/// fix the one endpoint meant to flag hand-edited files reported `Current`,
/// leaving the failure undiagnosable from inside the app.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn non_utf8_file_reports_unreadable_not_current() {
    let (ctx, _dir) = minimal_ctx();
    let templates_dir = ctx.paths.templates_dir().to_path_buf();
    let _env_guard = pin_templates_dir(&templates_dir);
    materialize_defaults_on_startup(&templates_dir).expect("materialize defaults");

    // Windows-1251 bytes for a short Cyrillic string — invalid UTF-8, exactly
    // what a Notepad "ANSI" save produces for Russian template content.
    let cp1251_bytes: &[u8] = &[
        0xD8, 0xE0, 0xE1, 0xEB, 0xEE, 0xED, // "Шаблон"
    ];
    std::fs::write(templates_dir.join("act_handover.html"), cp1251_bytes)
        .expect("write cp1251 act_handover.html");

    let statuses = build_templates_status(&ctx)
        .await
        .expect("build_templates_status");

    for entry in &statuses {
        if entry.filename == "act_handover.html" {
            assert!(
                matches!(entry.status, TemplateFileStatus::Unreadable),
                "expected Unreadable for a non-UTF-8 act_handover.html, got {:?}",
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

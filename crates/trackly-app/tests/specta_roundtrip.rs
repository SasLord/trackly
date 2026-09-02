//! Phase 1 success criterion #5: ОДИН `HealthDto` Rust-тип проходит через
//! ДВА транспорта (Tauri command path и axum handler path) и даёт
//! **байт-идентичный** payload — `PartialEq` равенство на десериализованных
//! структурах.
//!
//! Если эта проверка упадёт — где-то транспорты разошлись (например,
//! axum-handler собрался не из `build_health`, а вручную). Это закроет
//! путь для regression в Phase 5+.

use std::sync::Arc;

use axum::body::Body;
use axum::http::Request;
use tempfile::TempDir;
use tokio_util::sync::CancellationToken;
use tower::ServiceExt;

use trackly_app::context::AppCtx;
use trackly_app::dto::HealthDto;
use trackly_app::http::health::router;
use trackly_app::tauri_cmds::health::build_health;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::test_support::test_app_ctx::test_writer_and_readers;

/// Минимальный AppCtx из Plan 04 fixture. Полноценный путь `AppCtx::build`
/// проверяет `tests/health_smoke.rs`.
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
    let places = Arc::new(trackly_app::services::PlaceService::new(
        writer.clone(),
        readers.clone(),
        clock.clone(),
    ));
    let place_movements = Arc::new(trackly_app::services::PlaceMovementService::new(
        readers.clone(),
    ));
    let ctx = AppCtx {
        writer,
        readers,
        places,
        place_movements,
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

/// 30-second hard timeout — guards against the Linux-CI deadlock pattern
/// (axum `oneshot` × tokio multi_thread runtime × `WriterHandle` drop) that
/// hangs ci-fast indefinitely. Test passes in ~50 ms when healthy; if the
/// 30 s budget is exceeded, the test fails with a clear timeout error
/// instead of stalling the whole workflow. Windows + macOS unaffected
/// (consistently pass in <1 s).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn health_dto_round_trips_identical_through_both_transports() -> anyhow::Result<()> {
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        let (ctx, _guard) = minimal_ctx();

        // Path 1: Tauri-command (через общий build_health helper — то же
        // самое, что вызывает `#[tauri::command] async fn health`).
        let from_tauri = build_health(&ctx).await;

        // Path 2: axum handler через in-process oneshot. axum::Router с
        // GET /api/v1/health смонтирован на `ctx.clone()`.
        let app = router().with_state(ctx);
        let res = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/health")
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(res.status(), 200);

        let body_bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024).await?;
        let from_axum: HealthDto = serde_json::from_slice(&body_bytes)?;

        // Phase 1 success criterion #5 — единый DTO в двух транспортах.
        assert_eq!(
            from_tauri, from_axum,
            "transport drift: tauri={from_tauri:?}, axum={from_axum:?}"
        );

        Ok::<_, anyhow::Error>(())
    })
    .await
    .map_err(|_| anyhow::anyhow!("specta_roundtrip exceeded 30s budget — likely Linux-CI deadlock; see history of ci-fast hangs"))?
}

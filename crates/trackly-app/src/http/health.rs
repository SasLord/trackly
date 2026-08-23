//! `GET /api/v1/health` axum handler. Делегирует `build_health` (общий с
//! Tauri command в `tauri_cmds/health.rs`), что обеспечивает байт-идентичный
//! JSON на проводе между двумя транспортами (Phase 1 success criterion #5).

use axum::{extract::State, routing::get, Json, Router};

use crate::context::AppCtx;
use crate::dto::HealthDto;
use crate::tauri_cmds::health::build_health;

/// axum handler `GET /api/v1/health`. Возвращает `Json<HealthDto>`
/// (`AppError` Phase 1 не возвращает — health не делает I/O; в Phase 5
/// можно вернуть 503 если probe-БД упадёт).
pub async fn get_health(State(ctx): State<AppCtx>) -> Json<HealthDto> {
    Json(build_health(&ctx).await)
}

/// Сборщик роутера. `AppCtx` подключается через `.with_state(ctx)` на
/// callsite (так удобнее тестам делать свой ctx).
pub fn router() -> Router<AppCtx> {
    Router::new().route("/api/v1/health", get(get_health))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use std::sync::Arc;
    use tempfile::TempDir;
    use tokio_util::sync::CancellationToken;
    use tower::ServiceExt;
    use trackly_core::primitives::clock::Clock;
    use trackly_infra::ad::directory_mock::MockAdDirectory;
    use trackly_infra::clock_impl::SystemClock;
    use trackly_infra::test_support::test_app_ctx::test_writer_and_readers;

    /// Минимальный AppCtx (тот же паттерн, что в `tauri_cmds::health::tests`).
    async fn minimal_ctx() -> (AppCtx, TempDir) {
        let (writer, readers, dir) = test_writer_and_readers();
        let paths = trackly_infra::Paths::resolve_for_exe_dir(dir.path().to_path_buf())
            .expect("resolve paths");
        let config = trackly_infra::AppConfig::default();
        let (_nb, log_guard) = tracing_appender::non_blocking(std::io::sink());
        let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
        let devices = Arc::new(crate::services::DeviceService::new(
            writer.clone(),
            readers.clone(),
            clock.clone(),
        ));
        let paths_arc = Arc::new(paths);
        let organization = Arc::new(crate::services::OrganizationService::new(paths_arc.clone()));
        let templates = Arc::new(crate::services::TemplateService::new(
            writer.clone(),
            readers.clone(),
            clock.clone(),
        ));
        let pdf = Arc::new(crate::pdf::PdfRenderer::new());
        templates
            .seed_defaults_on_startup()
            .await
            .expect("seed templates");
        let acts = Arc::new(
            crate::services::ActService::new(writer.clone(), readers.clone(), clock.clone())
                .with_pdf_pipeline(templates.clone(), organization.clone(), pdf.clone()),
        );
        let cartridges = Arc::new(crate::services::CartridgeService::new(
            writer.clone(),
            readers.clone(),
            clock.clone(),
        ));
        let ad_client: Arc<dyn trackly_core::ports::ad::AdClient + Send + Sync> =
            Arc::new(trackly_infra::ad::mock::MockAdClient::default_fixtures());
        let (ws_tx, _) = tokio::sync::broadcast::channel::<crate::dto::printer::WsEvent>(128);
        let ws_broadcast = Arc::new(ws_tx);
        let auth = Arc::new(crate::services::AuthService::new(
            writer.clone(),
            readers.clone(),
            clock.clone(),
            ad_client,
            ws_broadcast.clone(),
            Arc::new(MockAdDirectory::default_fixtures()),
        ));
        let (poll_tx, _poll_rx) = tokio::sync::mpsc::channel::<i64>(64);
        let snmp_client: Arc<dyn trackly_core::ports::snmp::SnmpClient + Send + Sync> =
            Arc::new(trackly_infra::snmp::mock::MockSnmpClient::default_fixtures());
        let printers = Arc::new(crate::services::PrinterService::new(
            writer.clone(),
            readers.clone(),
            clock.clone(),
            snmp_client,
            poll_tx,
            ws_broadcast.clone(),
        ));
        let requests = Arc::new(crate::services::RequestService::new(
            writer.clone(),
            readers.clone(),
            clock.clone(),
            ws_broadcast.clone(),
        ));
        let org_db = Arc::new(crate::services::OrgDbService::new(
            writer.clone(),
            readers.clone(),
            clock.clone(),
            paths_arc.clone(),
        ));
        let reports = Arc::new(crate::services::ReportService::new(
            writer.clone(),
            readers.clone(),
            clock.clone(),
            Arc::new(config.clone()),
            pdf.clone(),
        ));
        let dashboard = Arc::new(crate::services::DashboardService::new(
            writer.clone(),
            readers.clone(),
            clock.clone(),
            Arc::new(config.clone()),
        ));
        let backup = Arc::new(crate::services::BackupService::new(
            writer.clone(),
            readers.clone(),
            clock.clone(),
            dir.path().join("trackly.db"),
        ));
        let places = Arc::new(crate::services::PlaceService::new(
            writer.clone(),
            readers.clone(),
            clock.clone(),
        ));
        let ctx = AppCtx {
            writer,
            readers,
            places,
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

    /// 30 s hard timeout — same rationale as `tests/specta_roundtrip.rs`.
    /// Guards against the Linux-CI deadlock that previously stalled
    /// `cargo test --workspace` for 30+ minutes.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn get_health_returns_200_and_health_dto() {
        tokio::time::timeout(std::time::Duration::from_secs(30), async {
            let (ctx, _guard) = minimal_ctx().await;
            let app = router().with_state(ctx);
            let res = app
                .oneshot(
                    Request::builder()
                        .uri("/api/v1/health")
                        .body(Body::empty())
                        .expect("build request"),
                )
                .await
                .expect("oneshot");
            assert_eq!(res.status(), 200);
            let bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024)
                .await
                .expect("read body");
            let dto: HealthDto = serde_json::from_slice(&bytes).expect("deserialize");
            assert_eq!(dto.version, env!("CARGO_PKG_VERSION"));
            assert!(dto.db_ready);
            assert_eq!(dto.schema_version, 15);
        })
        .await
        .expect("get_health exceeded 30 s budget — Linux-CI deadlock pattern");
    }
}

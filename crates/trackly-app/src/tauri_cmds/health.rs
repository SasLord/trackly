//! `health` Tauri command + общий `build_health` хелпер.
//!
//! `build_health(&AppCtx)` — pure хелпер: testable без `tauri::State<'_, _>`
//! lifetime acrobatics. Используется обоими транспортами:
//! - `#[tauri::command] async fn health(state: tauri::State<'_, AppCtx>)` —
//!   делегирует `build_health(state.inner())`.
//! - `axum::handler::get_health(State(ctx))` (см. `http/health.rs`) —
//!   делегирует `build_health(&ctx)`.
//!
//! Это и есть единый источник истины для shape `HealthDto`. См. также
//! `tests/specta_roundtrip.rs` — `assert_eq!` сравнивает результаты обоих
//! путей через `PartialEq` derive на `HealthDto`.

use crate::context::AppCtx;
use crate::dto::HealthDto;
use trackly_core::error::AppError;

/// Собирает `HealthDto` из `AppCtx`. Никакой I/O — все три поля уже в `ctx`
/// после `AppCtx::build` (которая выполнила миграции и держит живые
/// connection'ы).
pub async fn build_health(ctx: &AppCtx) -> HealthDto {
    HealthDto {
        version: env!("CARGO_PKG_VERSION").to_string(),
        db_ready: true,
        schema_version: ctx.schema_version,
    }
}

/// Tauri command. Регистрируется в `specta_export::builder()` через
/// `collect_commands![crate::tauri_cmds::health::health]`. Атрибут
/// `#[specta::specta]` должен идти ПОСЛЕ `#[tauri::command]` (этого требует
/// tauri-specta v2 rc.21 — иначе макрос не видит сигнатуру команды).
#[tauri::command]
#[specta::specta]
pub async fn health(state: tauri::State<'_, AppCtx>) -> Result<HealthDto, AppError> {
    Ok(build_health(state.inner()).await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tempfile::TempDir;
    use tokio_util::sync::CancellationToken;
    use trackly_core::primitives::clock::Clock;
    use trackly_infra::ad::directory_mock::MockAdDirectory;
    use trackly_infra::clock_impl::SystemClock;
    use trackly_infra::test_support::test_app_ctx::test_writer_and_readers;

    /// Минимальный AppCtx для тестов — собирается вручную из
    /// `test_writer_and_readers` (Plan 04 fixture). Полный `AppCtx::build`
    /// path тестируется в `tests/health_smoke.rs`.
    async fn minimal_ctx() -> (AppCtx, TempDir) {
        let (writer, readers, dir) = test_writer_and_readers();
        let paths = trackly_infra::Paths::resolve_for_exe_dir(dir.path().to_path_buf())
            .expect("resolve paths");
        let config = trackly_infra::AppConfig::default();
        // log_guard placeholder: тестовый sink, чтобы Drop вызывался,
        // но никаких реальных tracing-вызовов в этих тестах нет.
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

    /// 30 s hard timeout — same rationale as `tests/specta_roundtrip.rs`.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn build_health_returns_expected_fields() {
        tokio::time::timeout(std::time::Duration::from_secs(30), async {
            let (ctx, _guard) = minimal_ctx().await;
            let dto = build_health(&ctx).await;
            assert_eq!(dto.version, env!("CARGO_PKG_VERSION"));
            assert!(dto.db_ready);
            assert_eq!(dto.schema_version, 15);
        })
        .await
        .expect("build_health exceeded 30 s budget — Linux-CI deadlock pattern");
    }
}

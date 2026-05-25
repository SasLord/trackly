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
    let ctx = AppCtx {
        writer,
        readers,
        paths: Arc::new(paths),
        config: Arc::new(config),
        clock,
        shutdown: CancellationToken::new(),
        log_guard: Arc::new(log_guard),
        schema_version: 12,
    };
    (ctx, dir)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn health_dto_round_trips_identical_through_both_transports() -> anyhow::Result<()> {
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

    Ok(())
}

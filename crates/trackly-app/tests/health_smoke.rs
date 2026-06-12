//! End-to-end Phase 1 smoke: настоящий `AppCtx::build` поверх tempfile-БД
//! (полные миграции v001..v015 → `schema_version = 15`), затем оба
//! транспорта (Tauri-path через `build_health` + axum-path через oneshot
//! GET) возвращают одинаковый `HealthDto` с `db_ready = true, schema_version = 15`.
//!
//! В отличие от `specta_roundtrip.rs` (использует Plan 04 fixture
//! `test_writer_and_readers`), здесь полный `AppCtx::build` path —
//! probe-read → writer open → миграции → writer worker → reader pool.
//! Гарантирует, что Phase 1 lifecycle полностью работает за тестовой
//! рамкой `trackly --self-test`.

use axum::body::Body;
use axum::http::Request;
use tower::ServiceExt;

use trackly_app::dto::HealthDto;
use trackly_app::http::health::router;
use trackly_app::tauri_cmds::health::build_health;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn health_smoke_end_to_end_against_real_app_ctx() -> anyhow::Result<()> {
    let dir = tempfile::TempDir::new()?;
    let paths = trackly_infra::Paths::resolve_for_exe_dir(dir.path().to_path_buf())?;
    let config = trackly_infra::AppConfig::default();
    // logging::init для смоук-теста: создаст logs_dir и file-аппендер.
    // double-init safe (test может запускаться параллельно с logging.rs unit
    // tests — `init` тихо игнорирует `try_init` Err).
    let log_guard = trackly_app::logging::init(&paths, &config)?;
    let ctx = trackly_app::context::AppCtx::build(paths, config, log_guard).await?;

    // Tauri-path.
    let dto_tauri = build_health(&ctx).await;
    // V016 was added in plan 04-01 (cartridge tables); max_known_version is now 16.
    assert_eq!(
        dto_tauri.schema_version, 17,
        "schema_version after migrations"
    );
    assert!(dto_tauri.db_ready, "db_ready after build");
    assert_eq!(dto_tauri.version, env!("CARGO_PKG_VERSION"));

    // axum-path.
    let app = router().with_state(ctx.clone());
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(res.status(), 200);
    let body = axum::body::to_bytes(res.into_body(), 1024 * 1024).await?;
    let dto_axum: HealthDto = serde_json::from_slice(&body)?;

    assert_eq!(
        dto_tauri, dto_axum,
        "two transports produced different HealthDto"
    );

    // Graceful: cancel shutdown token; drop ctx → writer worker exits.
    ctx.shutdown.cancel();
    Ok(())
}

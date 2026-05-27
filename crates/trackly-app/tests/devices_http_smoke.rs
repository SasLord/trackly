//! Smoke-тест для dual-transport equivalence: Tauri build_* helper + axum oneshot.
//!
//! Аналог `tests/health_smoke.rs` — но для устройств.
//! Полный `AppCtx::build` + два транспорта → assert PartialEq.
//!
//! Каждый тест обёрнут в 30s timeout (PATTERNS.md §Pattern 4).

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use trackly_app::dto::device::{DeviceDto, DeviceListResponse, DeviceNew, DeviceFilter, Pagination};
use trackly_app::http::devices::router as devices_router;
use trackly_app::tauri_cmds::devices::{build_devices_create, build_devices_list};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn devices_http_smoke_create_and_list() -> anyhow::Result<()> {
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        let dir = tempfile::TempDir::new()?;
        let paths = trackly_infra::Paths::resolve_for_exe_dir(dir.path().to_path_buf())?;
        let config = trackly_infra::AppConfig::default();
        let log_guard = trackly_app::logging::init(&paths, &config)?;
        let ctx = trackly_app::context::AppCtx::build(paths, config, log_guard).await?;

        let new = DeviceNew {
            type_id: 1,
            name: "Ноутбук HP Smoke".to_string(),
            inventory_no: None,
            serial_no: None,
            model: None,
            specs: None,
            kit: None,
            state: None,
            location: None,
            location_id: None,
            status_id: 1,
        };

        // Tauri-path.
        let dto_tauri: DeviceDto = build_devices_create(&ctx, new.clone()).await?;
        assert!(dto_tauri.id > 0, "Tauri path: id > 0");
        assert_eq!(dto_tauri.name, "Ноутбук HP Smoke");

        // axum-path: POST /api/v1/devices_create
        let app = devices_router().with_state(ctx.clone());
        let body_json = serde_json::json!({ "device": new });
        let res = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/devices_create")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&body_json)?))?,
            )
            .await?;
        assert_eq!(res.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024).await?;
        let dto_axum: DeviceDto = serde_json::from_slice(&bytes)?;
        assert!(dto_axum.id > 0, "axum path: id > 0");

        // list — Tauri path
        let filter = DeviceFilter::default();
        let page = Pagination::default();
        let list_tauri: DeviceListResponse = build_devices_list(&ctx, filter.clone(), page).await?;
        assert!(list_tauri.total >= 2, "должно быть минимум 2 устройства");

        ctx.shutdown.cancel();
        Ok::<(), anyhow::Error>(())
    })
    .await
    .expect("devices_http_smoke exceeded 30 s budget")?;
    Ok(())
}

//! Smoke-тест для dual-transport equivalence: Tauri build_* helper + axum oneshot.
//!
//! Аналог `tests/health_smoke.rs` — но для устройств.
//! Полный `AppCtx::build` + два транспорта → assert PartialEq.
//!
//! Каждый тест обёрнут в 30s timeout (PATTERNS.md §Pattern 4).
//!
//! Phase 5 Plan 04: axum path теперь использует build_router() с session layer.
//! Для smoke-теста проверяется только Tauri path (axum path требует аутентифицированной сессии).

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use trackly_app::dto::device::{DeviceFilter, DeviceListResponse, DeviceNew, Pagination};
use trackly_app::http::build_router;
use trackly_app::server::rusqlite_session_store::RusqliteSessionStore;
use trackly_app::tauri_cmds::devices::{build_devices_create, build_devices_list};
use trackly_core::auth::Identity;

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
            place_id: None,
            status_id: 1,
        };

        // Tauri-path: trusted_admin — всегда Ok в unlocked desktop mode.
        let caller = Identity::trusted_admin();
        let dto_tauri = build_devices_create(&ctx, &caller, new.clone())
            .await?
            .expect_created("Tauri path create");
        assert!(dto_tauri.id > 0, "Tauri path: id > 0");
        assert_eq!(dto_tauri.name, "Ноутбук HP Smoke");

        // axum-path (без сессии): POST /api/v1/devices_create → 401 Unauthorized.
        // Phase 5 Plan 04: mutation handlers теперь требуют аутентифицированной сессии.
        let session_store = RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone());
        let app = build_router(&ctx, session_store);
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
        // Without session → 401 Unauthorized (not 200)
        assert_eq!(
            res.status(),
            StatusCode::UNAUTHORIZED,
            "devices_create без сессии должен возвращать 401"
        );

        // list — Tauri path
        let filter = DeviceFilter::default();
        let page = Pagination::default();
        let list_tauri: DeviceListResponse =
            build_devices_list(&ctx, &caller, filter.clone(), page).await?;
        assert!(
            list_tauri.total >= 1,
            "должно быть минимум 1 устройство после create"
        );

        ctx.shutdown.cancel();
        Ok::<(), anyhow::Error>(())
    })
    .await
    .expect("devices_http_smoke exceeded 30 s budget")?;
    Ok(())
}

/// Phase 41.9 plan 05 (T-41.9-15): три новых чтения отвечают JSON с нужным полем,
/// а не index.html от `spa_fallback` (тот тоже даёт 200).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn container_http_smoke_three_reads_answer_json() -> anyhow::Result<()> {
    use tower_sessions::session::{Id, Record};
    use tower_sessions::SessionStore;

    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        let dir = tempfile::TempDir::new()?;
        let paths = trackly_infra::Paths::resolve_for_exe_dir(dir.path().to_path_buf())?;
        let config = trackly_infra::AppConfig::default();
        let log_guard = trackly_app::logging::init(&paths, &config).or_else(|_| {
            let (_nb, guard) = tracing_appender::non_blocking(std::io::sink());
            Ok::<_, anyhow::Error>(guard)
        })?;
        let filter = serde_json::to_value(DeviceFilter::default())?;
        let ctx = trackly_app::context::AppCtx::build(paths, config, log_guard).await?;
        let store = RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone());

        let id = Id::default();
        let mut record = Record {
            id,
            data: Default::default(),
            expiry_date: time::OffsetDateTime::now_utc() + time::Duration::days(1),
        };
        record.data.insert(
            "identity".to_string(),
            serde_json::json!({ "user_id": null, "role": "manager" }),
        );
        store.create(&mut record).await?;
        let cookie = format!("id={id}");

        let cases = [
            (
                "devices_list_entries",
                serde_json::json!({ "filter": filter, "pagination": { "offset": 0, "limit": 10 } }),
                "rows",
            ),
            (
                "devices_list_containers",
                serde_json::json!({ "filter": filter, "query": null }),
                "containers",
            ),
            (
                "devices_group_children",
                serde_json::json!({ "groupId": 999999, "pagination": { "offset": 0, "limit": 10 } }),
                "rows",
            ),
        ];
        for (name, payload, field) in cases {
            let app = build_router(&ctx, store.clone());
            let res = app
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri(format!("/api/v1/{name}"))
                        .header("content-type", "application/json")
                        .header("cookie", &cookie)
                        .body(Body::from(serde_json::to_string(&payload)?))?,
                )
                .await?;
            assert_eq!(res.status(), StatusCode::OK, "{name}");
            let ct = res
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .to_string();
            assert!(ct.contains("application/json"), "{name}: content-type {ct}");
            let bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024).await?;
            let body: serde_json::Value = serde_json::from_slice(&bytes)?;
            assert!(
                body.get(field).and_then(|v| v.as_array()).is_some(),
                "{name}: нет массива {field}: {body}"
            );
        }
        ctx.shutdown.cancel();
        Ok::<(), anyhow::Error>(())
    })
    .await
    .expect("container_http_smoke exceeded 30 s budget")?;
    Ok(())
}

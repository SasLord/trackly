//! Dual-transport smoke test for acts_create.
//!
//! Asserts that POST /api/v1/acts_create routed through axum matches the
//! shape returned by the Tauri build_* helper.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use rusqlite::params;
use tower::ServiceExt;

use trackly_app::dto::act::{ActCreateDto, ActDto, ActItemNewDto, ActReturnDto, ActReturnItemDto};
use trackly_app::http::acts::router as acts_router;
use trackly_infra::error_conversions::map_rusqlite;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn http_create_act_roundtrip() -> anyhow::Result<()> {
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        let dir = tempfile::TempDir::new()?;
        let paths = trackly_infra::Paths::resolve_for_exe_dir(dir.path().to_path_buf())?;
        let config = trackly_infra::AppConfig::default();
        let log_guard = trackly_app::logging::init(&paths, &config)?;
        let ctx = trackly_app::context::AppCtx::build(paths, config, log_guard).await?;

        // Seed 1 device via the writer
        let device_id: i64 = ctx
            .writer
            .execute(|conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;
                tx.execute(
                    "INSERT INTO devices \
                     (type_id, name, status_id, version, created_at_utc, updated_at_utc) \
                     VALUES (1, 'HttpSmokeDevice', 1, 1, ?1, ?1)",
                    params![1_700_000_000_i64],
                )
                .map_err(map_rusqlite)?;
                let id = tx.last_insert_rowid();
                tx.commit().map_err(map_rusqlite)?;
                Ok(id)
            })
            .await?;

        let payload = ActCreateDto {
            number_override: None,
            giver_name: "А".into(),
            receiver_name: "Б".into(),
            location_id: None,
            notes: None,
            deadline_utc: None,
            items: vec![ActItemNewDto {
                device_id,
                quantity: 1,
            }],
        };

        let app = acts_router().with_state(ctx.clone());
        let body_json = serde_json::json!({ "payload": payload });
        let res = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/acts_create")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&body_json)?))?,
            )
            .await?;
        assert_eq!(res.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024).await?;
        let dto: ActDto = serde_json::from_slice(&bytes)?;
        assert!(dto.id > 0);
        assert_eq!(dto.number, "1");
        assert_eq!(dto.act_type, "handover");
        assert_eq!(dto.items.len(), 1);

        ctx.shutdown.cancel();
        Ok::<(), anyhow::Error>(())
    })
    .await
    .expect("acts_http_smoke budget")?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn http_acts_return_smoke() -> anyhow::Result<()> {
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        let dir = tempfile::TempDir::new()?;
        let paths = trackly_infra::Paths::resolve_for_exe_dir(dir.path().to_path_buf())?;
        let config = trackly_infra::AppConfig::default();
        let log_guard = trackly_app::logging::init(&paths, &config)?;
        let ctx = trackly_app::context::AppCtx::build(paths, config, log_guard).await?;

        let device_id: i64 = ctx
            .writer
            .execute(|conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;
                tx.execute(
                    "INSERT INTO devices \
                     (type_id, name, status_id, version, created_at_utc, updated_at_utc) \
                     VALUES (1, 'HttpReturnDevice', 1, 1, ?1, ?1)",
                    params![1_700_000_000_i64],
                )
                .map_err(map_rusqlite)?;
                let id = tx.last_insert_rowid();
                tx.commit().map_err(map_rusqlite)?;
                Ok(id)
            })
            .await?;

        // 1. Create handover via service.
        let handover = ctx
            .acts
            .create(ActCreateDto {
                number_override: None,
                giver_name: "А".into(),
                receiver_name: "Б".into(),
                location_id: None,
                notes: None,
                deadline_utc: None,
                items: vec![ActItemNewDto {
                    device_id,
                    quantity: 1,
                }],
            })
            .await?;

        // 2. POST /api/v1/acts_return for that handover.
        let return_payload = ActReturnDto {
            bulk_condition: Some("Хорошее".into()),
            bulk_location_id: None,
            apply_to_all: true,
            items: vec![ActReturnItemDto {
                act_item_id: handover.items[0].id,
                device_id,
                quantity: 1,
                condition_override: None,
                location_id_override: None,
            }],
        };
        let app = acts_router().with_state(ctx.clone());
        let body_json = serde_json::json!({
            "act_id": handover.id,
            "payload": return_payload,
        });
        let res = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/acts_return")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&body_json)?))?,
            )
            .await?;
        assert_eq!(res.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024).await?;
        let dto: ActDto = serde_json::from_slice(&bytes)?;
        assert_eq!(dto.act_type, "return");
        assert_eq!(dto.sub_number, Some(1));
        assert_eq!(dto.parent_act_id, Some(handover.id));

        ctx.shutdown.cancel();
        Ok::<(), anyhow::Error>(())
    })
    .await
    .expect("acts_http_return_smoke budget")?;
    Ok(())
}

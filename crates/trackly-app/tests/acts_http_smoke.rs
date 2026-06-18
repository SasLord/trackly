//! Dual-transport smoke test for acts_create.
//!
//! Asserts that POST /api/v1/acts_create routed through axum matches the
//! shape returned by the Tauri build_* helper.
//!
//! Phase 5 Plan 04: axum path теперь использует build_router() с session layer.
//! Тесты создают сессию admin программно для проверки mutation эндпоинтов.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use rusqlite::params;
use time::{Duration, OffsetDateTime};
use tower::ServiceExt;
use tower_sessions::session::{Id, Record};
use tower_sessions::SessionStore;

use trackly_app::dto::act::{ActCreateDto, ActDto, ActItemNewDto, ActReturnDto, ActReturnItemDto};
use trackly_app::dto::auth::UserNew;
use trackly_app::http::auth::SessionIdentity;
use trackly_app::http::build_router;
use trackly_app::server::rusqlite_session_store::RusqliteSessionStore;
use trackly_core::auth::{Identity, Role};
use trackly_infra::error_conversions::map_rusqlite;

/// Создать admin сессию в store и вернуть cookie строку.
async fn create_admin_session(
    store: &RusqliteSessionStore,
    user_id: i64,
) -> anyhow::Result<String> {
    let session_id = Id::default();
    let si = SessionIdentity {
        user_id: Some(user_id),
        role: Role::Admin.as_str().to_string(),
    };
    let mut record = Record {
        id: session_id,
        data: Default::default(),
        expiry_date: OffsetDateTime::now_utc() + Duration::days(1),
    };
    record
        .data
        .insert("identity".to_string(), serde_json::to_value(&si)?);
    store.create(&mut record).await?;
    Ok(format!("id={}", session_id))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn http_create_act_roundtrip() -> anyhow::Result<()> {
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        let dir = tempfile::TempDir::new()?;
        let paths = trackly_infra::Paths::resolve_for_exe_dir(dir.path().to_path_buf())?;
        let config = trackly_infra::AppConfig::default();
        let log_guard = trackly_app::logging::init(&paths, &config)?;
        let ctx = trackly_app::context::AppCtx::build(paths, config, log_guard).await?;

        // Создаём admin пользователя
        let admin_dto = ctx
            .auth
            .create_user(
                UserNew {
                    login: "admin_acts".to_string(),
                    full_name: "Admin Acts".to_string(),
                    password: "password123".to_string(),
                    role: "admin".to_string(),
                    email: None,
                },
                &Identity::trusted_admin(),
            )
            .await?;

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
            location_name: None,
            notes: None,
            deadline_utc: None,
            handover_date_utc: None,
            items: vec![ActItemNewDto {
                device_id,
                device_ids: Vec::new(),
                quantity: 1,
            }],
        };

        // Создаём сессию admin
        let session_store = RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone());
        let admin_cookie = create_admin_session(&session_store, admin_dto.id).await?;

        let app = build_router(&ctx, session_store);
        let body_json = serde_json::json!({ "payload": payload });
        let res = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/acts_create")
                    .header("content-type", "application/json")
                    .header("cookie", &admin_cookie)
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

        // Создаём admin пользователя
        let admin_dto = ctx
            .auth
            .create_user(
                UserNew {
                    login: "admin_return".to_string(),
                    full_name: "Admin Return".to_string(),
                    password: "password123".to_string(),
                    role: "admin".to_string(),
                    email: None,
                },
                &Identity::trusted_admin(),
            )
            .await?;

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

        // 1. Create handover via service (bypass HTTP for setup).
        let handover = ctx
            .acts
            .create(ActCreateDto {
                number_override: None,
                giver_name: "А".into(),
                receiver_name: "Б".into(),
                location_id: None,
                location_name: None,
                notes: None,
                deadline_utc: None,
                handover_date_utc: None,
                items: vec![ActItemNewDto {
                    device_id,
                    device_ids: Vec::new(),
                    quantity: 1,
                }],
            })
            .await?;

        // 2. POST /api/v1/acts_return for that handover.
        let return_payload = ActReturnDto {
            bulk_condition: Some("Хорошее".into()),
            bulk_location_id: None,
            bulk_location_name: None,
            apply_to_all: true,
            items: vec![ActReturnItemDto {
                act_item_id: handover.items[0].id,
                device_id,
                device_ids: vec![device_id],
                quantity: 1,
                condition_override: None,
                location_id_override: None,
                location_name_override: None,
            }],
        };

        // Создаём сессию admin
        let session_store = RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone());
        let admin_cookie = create_admin_session(&session_store, admin_dto.id).await?;

        let app = build_router(&ctx, session_store);
        let body_json = serde_json::json!({
            // S-5: camelCase top-level arg key over HTTP (act_id -> actId).
            "actId": handover.id,
            "payload": return_payload,
        });
        let res = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/acts_return")
                    .header("content-type", "application/json")
                    .header("cookie", &admin_cookie)
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

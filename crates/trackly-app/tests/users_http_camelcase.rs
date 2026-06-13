//! Regression test for S-5 camelCase parity on the HTTP transport.
//!
//! Browser/server mode: `apiCall` (ui/src/lib/api/client.ts) sends args verbatim
//! as camelCase JSON via `fetch('/api/v1/<name>', ...)`. The axum payload structs
//! in http/*.rs must therefore accept camelCase top-level arg keys (the Tauri
//! transport gets the same camelCase from the frontend and tauri-specta converts
//! to snake_case Rust params).
//!
//! This was a latent bug: payload structs deserialized snake_case fields without
//! `#[serde(rename_all = "camelCase")]`, so every endpoint with a multi-word arg
//! (e.g. `userNew`) returned 422 over HTTP while working on desktop.
//!
//! There is no role-check middleware — auth runs inside the handler body, AFTER
//! the `Json` extractor — so a deserialization failure surfaces as 422 BEFORE the
//! 200/401/403 path. An authenticated admin session is created programmatically
//! to reach the 200 path and prove the body deserialized.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use time::{Duration, OffsetDateTime};
use tower::ServiceExt;
use tower_sessions::session::{Id, Record};
use tower_sessions::SessionStore;

use trackly_app::dto::auth::UserNew;
use trackly_app::http::auth::SessionIdentity;
use trackly_app::http::build_router;
use trackly_app::server::rusqlite_session_store::RusqliteSessionStore;
use trackly_core::auth::Role;

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

async fn post_users_create(
    ctx: &trackly_app::context::AppCtx,
    cookie: &str,
    body: serde_json::Value,
) -> anyhow::Result<StatusCode> {
    let session_store = RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone());
    let app = build_router(ctx, session_store);
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/users_create")
                .header("content-type", "application/json")
                .header("cookie", cookie)
                .body(Body::from(serde_json::to_string(&body)?))?,
        )
        .await?;
    Ok(res.status())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn users_create_accepts_camelcase_wrapper_key() -> anyhow::Result<()> {
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        let dir = tempfile::TempDir::new()?;
        let paths = trackly_infra::Paths::resolve_for_exe_dir(dir.path().to_path_buf())?;
        let config = trackly_infra::AppConfig::default();
        let log_guard = trackly_app::logging::init(&paths, &config)?;
        let ctx = trackly_app::context::AppCtx::build(paths, config, log_guard).await?;

        // Bootstrap an admin so users_create passes the role check (admin-only).
        let admin_dto = ctx
            .auth
            .create_user(
                UserNew {
                    login: "admin_camel".to_string(),
                    full_name: "Админ КамелКейс".to_string(),
                    password: "password123".to_string(),
                    role: "admin".to_string(),
                    email: None,
                },
                &trackly_core::auth::Identity::trusted_admin(),
            )
            .await?;

        let session_store = RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone());
        let admin_cookie = create_admin_session(&session_store, admin_dto.id).await?;

        // camelCase top-level wrapper key `userNew` (what the browser sends).
        // Nested UserNew fields stay snake_case on both transports (dto/auth.rs).
        let camel_body = serde_json::json!({
            "userNew": {
                "login": "newuser_camel",
                "full_name": "Новый Пользователь",
                "password": "password123",
                "role": "employee",
                "email": null,
            }
        });
        let status = post_users_create(&ctx, &admin_cookie, camel_body).await?;
        assert_eq!(
            status,
            StatusCode::OK,
            "camelCase userNew должен десериализоваться и вернуть 200, получили {status}"
        );

        // Guard: the old snake_case wrapper key must now be rejected (proves the
        // rename took effect — the wire contract is camelCase, not snake_case).
        let snake_body = serde_json::json!({
            "user_new": {
                "login": "newuser_snake",
                "full_name": "Старый Ключ",
                "password": "password123",
                "role": "employee",
                "email": null,
            }
        });
        let status_snake = post_users_create(&ctx, &admin_cookie, snake_body).await?;
        assert_eq!(
            status_snake,
            StatusCode::UNPROCESSABLE_ENTITY,
            "snake_case user_new больше не принимается (ожидали 422), получили {status_snake}"
        );

        ctx.shutdown.cancel();
        Ok::<(), anyhow::Error>(())
    })
    .await
    .expect("users_http_camelcase exceeded 30 s budget")?;
    Ok(())
}

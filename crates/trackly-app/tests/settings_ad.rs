//! AD settings HTTP endpoint test — Phase 9 Plan 04.
//!
//! Покрывает `<behavior>` плана: `settings_set_ad_requires_manage_settings`
//! (403 для non-admin), плюс happy-path get/set round-trip для admin.
//!
//! Session setup: программно через RusqliteSessionStore (как в
//! role_endpoint_matrix.rs) — обходит GovernorLayer на /auth_login (этот
//! роут на settings_get_ad/settings_set_ad не висит, но программная сессия
//! проще и быстрее реального TCP-логина).

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::json;
use time::{Duration, OffsetDateTime};
use tower::ServiceExt;
use tower_sessions::session::{Id, Record};
use tower_sessions::SessionStore;

use trackly_app::context::AppCtx;
use trackly_app::dto::auth::UserNew;
use trackly_app::http::auth::SessionIdentity;
use trackly_app::http::build_router;
use trackly_app::server::rusqlite_session_store::RusqliteSessionStore;
use trackly_core::auth::{Identity, Role};

async fn make_test_ctx() -> anyhow::Result<(AppCtx, tempfile::TempDir)> {
    let dir = tempfile::TempDir::new()?;
    let dir_path = dir.path().to_path_buf();
    let paths = trackly_infra::Paths::resolve_for_exe_dir(dir_path)?;
    let config = trackly_infra::AppConfig::default();
    let log_guard = trackly_app::logging::init(&paths, &config).or_else(|_| {
        let (_nb, guard) = tracing_appender::non_blocking(std::io::sink());
        Ok::<_, anyhow::Error>(guard)
    })?;
    let ctx = AppCtx::build(paths, config, log_guard).await?;
    Ok((ctx, dir))
}

/// Создать сессию программно в RusqliteSessionStore, вернуть cookie строку.
async fn create_session_cookie(
    store: &RusqliteSessionStore,
    user_id: i64,
    role: Role,
) -> anyhow::Result<String> {
    let session_id = Id::default();
    let si = SessionIdentity {
        user_id: Some(user_id),
        role: role.as_str().to_string(),
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
    Ok(format!("id={session_id}"))
}

async fn post_with_cookie(
    app: axum::Router,
    uri: &str,
    body: serde_json::Value,
    cookie: Option<&str>,
) -> (StatusCode, serde_json::Value) {
    let mut builder = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json");
    if let Some(c) = cookie {
        builder = builder.header("cookie", c);
    }
    let req = builder
        .body(Body::from(serde_json::to_string(&body).unwrap()))
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    let status = res.status();
    let body_bytes = axum::body::to_bytes(res.into_body(), 16 * 1024)
        .await
        .unwrap_or_default();
    let body_json = serde_json::from_slice(&body_bytes).unwrap_or(serde_json::Value::Null);
    (status, body_json)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn settings_set_ad_requires_manage_settings() {
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        let (ctx, _dir) = make_test_ctx().await.expect("make_test_ctx");
        let admin_identity = Identity::trusted_admin();

        let employee_dto = ctx
            .auth
            .create_user(
                UserNew {
                    login: "ad_settings_employee".to_string(),
                    full_name: "Employee".to_string(),
                    password: "password123".to_string(),
                    role: "employee".to_string(),
                    email: None,
                },
                &admin_identity,
            )
            .await
            .expect("create employee_user");

        let session_store = RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone());
        let employee_cookie =
            create_session_cookie(&session_store, employee_dto.id, Role::Employee)
                .await
                .expect("create employee session");

        let router = build_router(&ctx, session_store);

        // Employee → 403 Forbidden on settings_set_ad.
        let (status, _) = post_with_cookie(
            router.clone(),
            "/api/v1/settings_set_ad",
            json!({ "enabled": true, "autoAccept": false }),
            Some(&employee_cookie),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::FORBIDDEN,
            "employee должен получить 403 на settings_set_ad"
        );

        // No session → 401 Unauthorized.
        let (status_no_session, _) = post_with_cookie(
            router.clone(),
            "/api/v1/settings_set_ad",
            json!({ "enabled": true, "autoAccept": false }),
            None,
        )
        .await;
        assert_eq!(
            status_no_session,
            StatusCode::UNAUTHORIZED,
            "без сессии должен быть 401 на settings_set_ad"
        );

        // Employee → 403 Forbidden on settings_get_ad too.
        let (status_get, _) = post_with_cookie(
            router.clone(),
            "/api/v1/settings_get_ad",
            json!({}),
            Some(&employee_cookie),
        )
        .await;
        assert_eq!(
            status_get,
            StatusCode::FORBIDDEN,
            "employee должен получить 403 на settings_get_ad"
        );

        ctx.shutdown.cancel();
    })
    .await
    .expect("settings_set_ad_requires_manage_settings exceeded 30s budget");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn settings_ad_admin_get_set_round_trip() {
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        let (ctx, _dir) = make_test_ctx().await.expect("make_test_ctx");
        let admin_identity = Identity::trusted_admin();

        let admin_dto = ctx
            .auth
            .create_user(
                UserNew {
                    login: "ad_settings_admin".to_string(),
                    full_name: "Admin".to_string(),
                    password: "password123".to_string(),
                    role: "admin".to_string(),
                    email: None,
                },
                &admin_identity,
            )
            .await
            .expect("create admin_user");

        let session_store = RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone());
        let admin_cookie = create_session_cookie(&session_store, admin_dto.id, Role::Admin)
            .await
            .expect("create admin session");

        let router = build_router(&ctx, session_store);

        // Initial GET — defaults from TOML (enabled=false typically) + DB defaults.
        let (status_get, body_get) = post_with_cookie(
            router.clone(),
            "/api/v1/settings_get_ad",
            json!({}),
            Some(&admin_cookie),
        )
        .await;
        assert_eq!(status_get, StatusCode::OK, "admin GET должен вернуть 200");
        // AdSettingsDto (dto/auth.rs) is snake_case on the wire by convention
        // (snake_case_json_invariant test) — unlike the SetAdPayload request
        // wrapper, which is camelCase (matches http/settings.rs convention).
        assert!(
            body_get.get("enabled").is_some(),
            "AdSettingsDto должен содержать enabled: {body_get}"
        );
        assert!(
            body_get.get("auto_accept").is_some(),
            "AdSettingsDto должен содержать auto_accept (snake_case): {body_get}"
        );
        assert!(
            body_get.get("no_tls_verify").is_some(),
            "AdSettingsDto должен содержать no_tls_verify (snake_case): {body_get}"
        );
        // D-Sec-01 / T-09-17: AD password must never appear anywhere in this DTO.
        let body_str = body_get.to_string().to_lowercase();
        assert!(
            !body_str.contains("password"),
            "settings_get_ad НЕ должен содержать поле password: {body_get}"
        );

        // SET enabled=true, autoAccept=true.
        let (status_set, _) = post_with_cookie(
            router.clone(),
            "/api/v1/settings_set_ad",
            json!({ "enabled": true, "autoAccept": true }),
            Some(&admin_cookie),
        )
        .await;
        assert_eq!(status_set, StatusCode::OK, "admin SET должен вернуть 200");

        // Re-GET — confirm persisted.
        let (status_get2, body_get2) = post_with_cookie(
            router.clone(),
            "/api/v1/settings_get_ad",
            json!({}),
            Some(&admin_cookie),
        )
        .await;
        assert_eq!(status_get2, StatusCode::OK);
        assert_eq!(
            body_get2.get("enabled"),
            Some(&serde_json::Value::Bool(true)),
            "enabled должен персистироваться как true: {body_get2}"
        );
        assert_eq!(
            body_get2.get("auto_accept"),
            Some(&serde_json::Value::Bool(true)),
            "auto_accept должен персистироваться как true: {body_get2}"
        );

        ctx.shutdown.cancel();
    })
    .await
    .expect("settings_ad_admin_get_set_round_trip exceeded 30s budget");
}

/// `ad_test_connection` gating mirrors `settings_set_ad`: non-admin → 403,
/// no session → 401, admin → 200 (Phase 9 gap-closure — "Проверить
/// подключение" button must be admin-gated like the other AD settings ops).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ad_test_connection_requires_manage_settings() {
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        let (ctx, _dir) = make_test_ctx().await.expect("make_test_ctx");
        let admin_identity = Identity::trusted_admin();

        let employee_dto = ctx
            .auth
            .create_user(
                UserNew {
                    login: "ad_test_conn_employee".to_string(),
                    full_name: "Employee".to_string(),
                    password: "password123".to_string(),
                    role: "employee".to_string(),
                    email: None,
                },
                &admin_identity,
            )
            .await
            .expect("create employee_user");

        let session_store = RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone());
        let employee_cookie =
            create_session_cookie(&session_store, employee_dto.id, Role::Employee)
                .await
                .expect("create employee session");

        let router = build_router(&ctx, session_store);

        // Employee → 403 Forbidden on ad_test_connection.
        let (status, _) = post_with_cookie(
            router.clone(),
            "/api/v1/ad_test_connection",
            json!({}),
            Some(&employee_cookie),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::FORBIDDEN,
            "employee должен получить 403 на ad_test_connection"
        );

        // No session → 401 Unauthorized.
        let (status_no_session, _) = post_with_cookie(
            router.clone(),
            "/api/v1/ad_test_connection",
            json!({}),
            None,
        )
        .await;
        assert_eq!(
            status_no_session,
            StatusCode::UNAUTHORIZED,
            "без сессии должен быть 401 на ad_test_connection"
        );

        ctx.shutdown.cancel();
    })
    .await
    .expect("ad_test_connection_requires_manage_settings exceeded 30s budget");
}

/// `ad_test_connection` returns 200 for admin in mock mode — mock AD client
/// is always "reachable" by default (`MockAdClient::default_fixtures`).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ad_test_connection_admin_succeeds_in_mock_mode() {
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        let (ctx, _dir) = make_test_ctx().await.expect("make_test_ctx");
        let admin_identity = Identity::trusted_admin();

        let admin_dto = ctx
            .auth
            .create_user(
                UserNew {
                    login: "ad_test_conn_admin".to_string(),
                    full_name: "Admin".to_string(),
                    password: "password123".to_string(),
                    role: "admin".to_string(),
                    email: None,
                },
                &admin_identity,
            )
            .await
            .expect("create admin_user");

        let session_store = RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone());
        let admin_cookie = create_session_cookie(&session_store, admin_dto.id, Role::Admin)
            .await
            .expect("create admin session");

        let router = build_router(&ctx, session_store);

        let (status, _) = post_with_cookie(
            router.clone(),
            "/api/v1/ad_test_connection",
            json!({}),
            Some(&admin_cookie),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "admin должен получить 200 на ad_test_connection в mock-режиме"
        );

        ctx.shutdown.cancel();
    })
    .await
    .expect("ad_test_connection_admin_succeeds_in_mock_mode exceeded 30s budget");
}

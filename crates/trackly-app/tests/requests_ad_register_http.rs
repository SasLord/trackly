//! HTTP-transport regression tests for `ad_register` requests — Phase 9
//! Plan 04 (Task 2 `<behavior>`).
//!
//! `requests_ad_register.rs` already covers the SERVICE layer
//! (`RequestService::list`/`approve_ad_register` directly). This file
//! covers the same behaviors over the axum HTTP transport, confirming the
//! thin-adapter wiring in `http/requests.rs` correctly forwards `caller`
//! into the service (admin-only `ad_register` visibility, REQ-06) and that
//! `requests_approve_ad_register` is reachable end-to-end over HTTP.
//!
//! Session setup: programmatic via RusqliteSessionStore (mirrors
//! role_endpoint_matrix.rs / settings_ad.rs) — avoids GovernorLayer
//! entirely (not applicable to these routes anyway).

use axum::body::Body;
use axum::http::{Request, StatusCode};
use rusqlite::params;
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
use trackly_core::error::AppError;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;

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

/// Seed a "restore" ad_register scenario via raw writer access (mirrors
/// requests_ad_register.rs's seed_restore_request), returns (ad_user_id, request_id).
/// User row is blocked (`is_active=0`, `deleted_at_utc` NULL).
async fn seed_restore_request(ctx: &AppCtx, login: &str, full_name: &str) -> (i64, i64) {
    let now = SystemClock.unix_seconds();
    let login = login.to_string();
    let full_name = full_name.to_string();
    ctx.writer
        .execute(move |conn| {
            let tx = conn.transaction().map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            tx.execute(
                "INSERT INTO users \
                 (login, full_name, password_hash, role, ad_user, is_active, \
                  created_at_utc, updated_at_utc, version) \
                 VALUES (?1, ?2, NULL, 'employee', 1, 0, ?3, ?3, 1)",
                params![login, full_name, now],
            )
            .map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            let user_id = tx.last_insert_rowid();
            tx.execute(
                "INSERT INTO requests \
                 (request_type, status, requested_by_user_id, description, ad_subtype, \
                  created_at_utc, updated_at_utc, version) \
                 VALUES ('ad_register', 'open', ?1, ?2, 'restore', ?3, ?3, 1)",
                params![user_id, full_name, now],
            )
            .map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            let request_id = tx.last_insert_rowid();
            tx.commit().map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            Ok((user_id, request_id))
        })
        .await
        .expect("seed restore request")
}

/// Seed a SECOND open restore request for an EXISTING user (mirrors the
/// duplicate-request bug, Defect 1) — same shape as `seed_restore_request`
/// but reuses `user_id` instead of inserting a new user row.
async fn seed_second_restore_request(ctx: &AppCtx, user_id: i64, full_name: &str) -> i64 {
    let now = SystemClock.unix_seconds();
    let full_name = full_name.to_string();
    ctx.writer
        .execute(move |conn| {
            let tx = conn.transaction().map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            tx.execute(
                "INSERT INTO requests \
                 (request_type, status, requested_by_user_id, description, ad_subtype, \
                  created_at_utc, updated_at_utc, version) \
                 VALUES ('ad_register', 'open', ?1, ?2, 'restore', ?3, ?3, 1)",
                params![user_id, full_name, now],
            )
            .map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            let request_id = tx.last_insert_rowid();
            tx.commit().map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            Ok(request_id)
        })
        .await
        .expect("seed second restore request")
}

/// Seed a "pending" ad_register scenario via raw writer access (mirrors
/// requests_ad_register.rs's seed_pending_register), returns (ad_user_id, request_id).
async fn seed_pending_register(ctx: &AppCtx, login: &str, full_name: &str) -> (i64, i64) {
    let now = SystemClock.unix_seconds();
    let login = login.to_string();
    let full_name = full_name.to_string();
    ctx.writer
        .execute(move |conn| {
            let tx = conn.transaction().map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            tx.execute(
                "INSERT INTO users \
                 (login, full_name, password_hash, role, ad_user, is_active, \
                  created_at_utc, updated_at_utc, version) \
                 VALUES (?1, ?2, NULL, 'employee', 1, 0, ?3, ?3, 1)",
                params![login, full_name, now],
            )
            .map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            let user_id = tx.last_insert_rowid();
            tx.execute(
                "INSERT INTO requests \
                 (request_type, status, requested_by_user_id, description, ad_subtype, \
                  created_at_utc, updated_at_utc, version) \
                 VALUES ('ad_register', 'open', ?1, ?2, 'register', ?3, ?3, 1)",
                params![user_id, full_name, now],
            )
            .map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            let request_id = tx.last_insert_rowid();
            tx.commit().map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            Ok((user_id, request_id))
        })
        .await
        .expect("seed pending register")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ad_register_list_admin_only_http() {
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        let (ctx, _dir) = make_test_ctx().await.expect("make_test_ctx");
        let admin_identity = Identity::trusted_admin();

        let employee_dto = ctx
            .auth
            .create_user(
                UserNew {
                    login: "ad_list_employee".to_string(),
                    full_name: "Employee".to_string(),
                    password: "password123".to_string(),
                    role: "employee".to_string(),
                    email: None,
                },
                &admin_identity,
            )
            .await
            .expect("create employee_user");
        let admin_dto = ctx
            .auth
            .create_user(
                UserNew {
                    login: "ad_list_admin".to_string(),
                    full_name: "Admin".to_string(),
                    password: "password123".to_string(),
                    role: "admin".to_string(),
                    email: None,
                },
                &admin_identity,
            )
            .await
            .expect("create admin_user");

        seed_pending_register(&ctx, "us300http", "Сидоров Пётр HTTP").await;

        let session_store = RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone());
        let employee_cookie =
            create_session_cookie(&session_store, employee_dto.id, Role::Employee)
                .await
                .expect("employee session");
        let admin_cookie = create_session_cookie(&session_store, admin_dto.id, Role::Admin)
            .await
            .expect("admin session");

        let router = build_router(&ctx, session_store);

        let list_body = json!({
            "filter": { "status": null, "requestType": null, "assignedToUserId": null, "requestedByUserId": null },
            "pagination": { "offset": 0, "limit": 50 }
        });

        // Employee → ad_register requests must be excluded (REQ-06 / T-09-11 / T-09-18).
        let (status_emp, body_emp) = post_with_cookie(
            router.clone(),
            "/api/v1/requests_list",
            list_body.clone(),
            Some(&employee_cookie),
        )
        .await;
        assert_eq!(status_emp, StatusCode::OK, "employee list должен вернуть 200");
        let items_emp = body_emp["items"].as_array().expect("items array");
        assert!(
            items_emp
                .iter()
                .all(|r| r["requestType"] != "ad_register"),
            "employee list НЕ должен содержать ad_register заявки: {body_emp}"
        );

        // Admin → ad_register requests must be visible.
        let (status_admin, body_admin) = post_with_cookie(
            router.clone(),
            "/api/v1/requests_list",
            list_body,
            Some(&admin_cookie),
        )
        .await;
        assert_eq!(status_admin, StatusCode::OK, "admin list должен вернуть 200");
        let items_admin = body_admin["items"].as_array().expect("items array");
        assert!(
            items_admin
                .iter()
                .any(|r| r["requestType"] == "ad_register"),
            "admin list должен содержать ad_register заявки: {body_admin}"
        );

        ctx.shutdown.cancel();
    })
    .await
    .expect("ad_register_list_admin_only_http exceeded 30s budget");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn approve_ad_register_http() {
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        let (ctx, _dir) = make_test_ctx().await.expect("make_test_ctx");
        let admin_identity = Identity::trusted_admin();

        let admin_dto = ctx
            .auth
            .create_user(
                UserNew {
                    login: "approve_admin_http".to_string(),
                    full_name: "Admin".to_string(),
                    password: "password123".to_string(),
                    role: "admin".to_string(),
                    email: None,
                },
                &admin_identity,
            )
            .await
            .expect("create admin_user");

        let (ad_user_id, request_id) =
            seed_pending_register(&ctx, "us301http", "Кузнецов Олег HTTP").await;

        let session_store = RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone());
        let admin_cookie = create_session_cookie(&session_store, admin_dto.id, Role::Admin)
            .await
            .expect("admin session");

        let router = build_router(&ctx, session_store);

        let (status, body) = post_with_cookie(
            router.clone(),
            "/api/v1/requests_approve_ad_register",
            json!({
                "payload": { "requestId": request_id, "version": 1, "role": "manager" }
            }),
            Some(&admin_cookie),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "admin approve должен вернуть 200, body: {body}"
        );
        assert_eq!(
            body["status"], "completed",
            "заявка должна перейти в completed после approve: {body}"
        );

        // Confirm target user activated with selected role.
        let activated = ctx
            .auth
            .get_user_by_id(ad_user_id)
            .await
            .expect("get_user_by_id");
        assert!(activated.is_active, "пользователь должен быть активирован");
        assert_eq!(activated.role, "manager", "должна быть выбранная роль");

        ctx.shutdown.cancel();
    })
    .await
    .expect("approve_ad_register_http exceeded 30s budget");
}

// ---------------------------------------------------------------------------
// Test (Defect 2 repro, HTTP transport): reject a duplicate open restore
// request for a user whose companion restore request was already approved.
// Reproduces the real-world flow exactly — over `/api/v1/requests_transition`,
// not the service layer directly — to catch any axum/session/serde-specific
// leak that wouldn't surface when calling RequestService in-process.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn reject_restore_after_companion_already_approved_http() {
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        let (ctx, _dir) = make_test_ctx().await.expect("make_test_ctx");
        let admin_identity = Identity::trusted_admin();

        let admin_dto = ctx
            .auth
            .create_user(
                UserNew {
                    login: "reject_admin_http".to_string(),
                    full_name: "Admin".to_string(),
                    password: "password123".to_string(),
                    role: "admin".to_string(),
                    email: None,
                },
                &admin_identity,
            )
            .await
            .expect("create admin_user");

        let (user_id, request_id_a) =
            seed_restore_request(&ctx, "us200http", "Петрова Анна HTTP").await;
        let request_id_b = seed_second_restore_request(&ctx, user_id, "Петрова Анна HTTP").await;

        let session_store = RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone());
        let admin_cookie = create_session_cookie(&session_store, admin_dto.id, Role::Admin)
            .await
            .expect("admin session");

        let router = build_router(&ctx, session_store);

        // Approve request A over HTTP.
        let (status_a, body_a) = post_with_cookie(
            router.clone(),
            "/api/v1/requests_approve_ad_register",
            json!({
                "payload": { "requestId": request_id_a, "version": 1, "role": "employee" }
            }),
            Some(&admin_cookie),
        )
        .await;
        assert_eq!(
            status_a,
            StatusCode::OK,
            "approve запроса A должен вернуть 200, body: {body_a}"
        );
        assert_eq!(body_a["status"], "completed");

        let activated = ctx
            .auth
            .get_user_by_id(user_id)
            .await
            .expect("get_user_by_id");
        assert!(activated.is_active, "пользователь должен быть активирован");

        // Reject request B (companion duplicate, still open) over HTTP — this
        // is the EXACT real-world repro: admin approves one restore request,
        // then rejects the other for the same now-active user.
        let (status_b, body_b) = post_with_cookie(
            router.clone(),
            "/api/v1/requests_transition",
            json!({
                "payload": {
                    "op": "reject",
                    "requestId": request_id_b,
                    "version": 1,
                    "notes": "дубликат"
                }
            }),
            Some(&admin_cookie),
        )
        .await;

        // The bug report: this call returned a malformed/non-AppError body
        // that the frontend's parseAppError fallback couldn't parse (generic
        // "Не удалось связаться с приложением" toast). A correct response is
        // EITHER a 200 with status="rejected" OR a non-2xx response whose
        // body still has the `{code, message}` AppError shape — never an
        // empty body, raw panic text, or a body lacking those two fields.
        if status_b.is_success() {
            assert_eq!(
                body_b["status"], "rejected",
                "reject запроса B должен перевести его в rejected: {body_b}"
            );
        } else {
            assert!(
                body_b.get("code").and_then(|v| v.as_str()).is_some()
                    && body_b.get("message").and_then(|v| v.as_str()).is_some(),
                "ошибка должна иметь форму AppError {{code, message}}, получено: \
                 status={status_b}, body={body_b}"
            );
        }

        // The already-approved user's active state must be unaffected by the
        // reject of the companion duplicate.
        let after = ctx
            .auth
            .get_user_by_id(user_id)
            .await
            .expect("get_user_by_id");
        assert!(
            after.is_active,
            "reject дубликата не должен деактивировать уже одобренного пользователя"
        );

        ctx.shutdown.cancel();
    })
    .await
    .expect("reject_restore_after_companion_already_approved_http exceeded 30s budget");
}

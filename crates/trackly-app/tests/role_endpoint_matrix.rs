//! Role×Endpoint matrix CI test — Phase 5 Plan 04 (GREEN after RBAC retrofit).
//!
//! ROADMAP success criterion #3: при попытке через curl дёрнуть mutation-эндпоинт
//! устройств/актов/картриджей сотрудник получает 403 Forbidden.
//!
//! Test matrix (9 cases):
//! 1. No session → POST /api/v1/devices_create → 401 Unauthorized
//! 2. Employee session → POST /api/v1/devices_create → 403 Forbidden
//! 3. Manager session → POST /api/v1/devices_create → not 401/403 (200 or 422)
//! 4. Employee session → POST /api/v1/acts_create → 403 Forbidden
//! 5. Employee session → POST /api/v1/cartridges_create → 403 Forbidden
//! 6. Employee session → POST /api/v1/users_create → 403 Forbidden
//! 7. Manager session → POST /api/v1/users_create → 403 Forbidden (admin only)
//! 8. Admin session → POST /api/v1/users_create → not 401/403 (200 or 422)
//! 9. Employee session → POST /api/v1/devices_list → 200 OK (reads allowed)
//!
//! Session setup: sessions are created programmatically (bypassing /auth_login which
//! has GovernorLayer that requires real TCP peer IP unavailable in unit tests).

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

/// Построить тестовый AppCtx.
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
///
/// Это обходит GovernorLayer на /auth_login (требует реального TCP peer IP).
/// Сессия создаётся напрямую как tower-sessions Record.
async fn create_session_cookie(
    store: &RusqliteSessionStore,
    user_id: i64,
    role: Role,
) -> anyhow::Result<String> {
    let session_id = Id::default(); // random UUID

    let si = SessionIdentity {
        user_id: Some(user_id),
        role: role.as_str().to_string(),
    };

    // Serialize SessionIdentity into a tower-sessions Record.
    // Record stores arbitrary data — we insert "identity" key with SessionIdentity value.
    let mut record = Record {
        id: session_id,
        data: Default::default(),
        expiry_date: OffsetDateTime::now_utc() + Duration::days(1),
    };
    record
        .data
        .insert("identity".to_string(), serde_json::to_value(&si)?);

    store.create(&mut record).await?;

    // tower-sessions uses cookie name "id" by default; actual name depends on
    // SessionManagerLayer config. The default cookie name in tower-sessions is "id".
    // build_router uses SessionManagerLayer::new(store) without custom name → default "id".
    let cookie = format!("id={}", session_id);
    Ok(cookie)
}

/// Выполнить POST запрос с опциональным cookie.
async fn post_with_cookie(
    app: axum::Router,
    uri: &str,
    body: serde_json::Value,
    cookie: Option<&str>,
) -> StatusCode {
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
    res.status()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn role_endpoint_matrix_test() {
    tokio::time::timeout(std::time::Duration::from_secs(60), async {
        let (ctx, _dir) = make_test_ctx().await.expect("make_test_ctx failed");

        // --- Создаём тестовых пользователей ---
        let admin_identity = Identity::trusted_admin();

        let admin_dto = ctx
            .auth
            .create_user(
                UserNew {
                    login: "admin_user".to_string(),
                    full_name: "Admin User".to_string(),
                    password: "password123".to_string(),
                    role: "admin".to_string(),
                    email: None,
                },
                &admin_identity,
            )
            .await
            .expect("create admin_user");

        let manager_dto = ctx
            .auth
            .create_user(
                UserNew {
                    login: "manager_user".to_string(),
                    full_name: "Manager User".to_string(),
                    password: "password123".to_string(),
                    role: "manager".to_string(),
                    email: None,
                },
                &admin_identity,
            )
            .await
            .expect("create manager_user");

        let employee_dto = ctx
            .auth
            .create_user(
                UserNew {
                    login: "employee_user".to_string(),
                    full_name: "Employee User".to_string(),
                    password: "password123".to_string(),
                    role: "employee".to_string(),
                    email: None,
                },
                &admin_identity,
            )
            .await
            .expect("create employee_user");

        // --- Создаём сессии программно ---
        let session_store = RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone());

        let admin_cookie = create_session_cookie(&session_store, admin_dto.id, Role::Admin)
            .await
            .expect("create admin session");

        let manager_cookie = create_session_cookie(&session_store, manager_dto.id, Role::Manager)
            .await
            .expect("create manager session");

        let employee_cookie =
            create_session_cookie(&session_store, employee_dto.id, Role::Employee)
                .await
                .expect("create employee session");

        // --- Payloads ---
        let device_payload = json!({
            "device": {
                "type_id": 1,
                "name": "Test Device",
                "inventory_no": null,
                "serial_no": null,
                "model": null,
                "specs": null,
                "kit": null,
                "state": null,
                "location": null,
                "location_id": null,
                "status_id": 1
            }
        });

        let act_payload = json!({
            "payload": {
                "number_override": null,
                "giver_name": "Тест Тестов",
                "receiver_name": "Тест2 Тестов",
                "location_id": null,
                "location_name": null,
                "notes": null,
                "deadline_utc": null,
                "handover_date_utc": null,
                "items": []
            }
        });

        let cartridge_payload = json!({
            "payload": {
                "model_id": 1,
                "location": null,
                "notes": null
            }
        });

        // S-5: HTTP transport receives camelCase top-level arg keys (frontend sends
        // them verbatim via fetch). `user_new` wrapper field -> `userNew` on the wire.
        let user_create_payload = json!({
            "userNew": {
                "login": "newuser_test",
                "full_name": "New Test User",
                "password": "password123",
                "role": "employee",
                "email": null
            }
        });

        let device_list_payload = json!({
            "filter": {
                "type_id": null,
                "location_id": null,
                "status_id": null,
                "state": null,
                "name_prefix": null,
                "include_deleted": false,
                "group_by_condition": false
            },
            "pagination": { "offset": 0, "limit": 20 }
        });

        // Макрос для создания нового router + store на каждый тест (oneshot потребляет роутер).
        macro_rules! new_app {
            () => {{
                let ss = RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone());
                build_router(&ctx, ss)
            }};
        }

        // =====================================================================
        // Case 1: No session → POST /api/v1/devices_create → 401 Unauthorized
        // =====================================================================
        {
            let status = post_with_cookie(
                new_app!(),
                "/api/v1/devices_create",
                device_payload.clone(),
                None,
            )
            .await;
            assert_eq!(
                status,
                StatusCode::UNAUTHORIZED,
                "Case 1: No session → devices_create → expected 401, got {status}"
            );
        }

        // =====================================================================
        // Case 2: Employee session → POST /api/v1/devices_create → 403 Forbidden
        // =====================================================================
        {
            let status = post_with_cookie(
                new_app!(),
                "/api/v1/devices_create",
                device_payload.clone(),
                Some(&employee_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::FORBIDDEN,
                "Case 2: Employee → devices_create → expected 403, got {status}"
            );
        }

        // =====================================================================
        // Case 3: Manager session → POST /api/v1/devices_create → not 401/403
        // =====================================================================
        {
            let status = post_with_cookie(
                new_app!(),
                "/api/v1/devices_create",
                device_payload.clone(),
                Some(&manager_cookie),
            )
            .await;
            assert!(
                status != StatusCode::UNAUTHORIZED && status != StatusCode::FORBIDDEN,
                "Case 3: Manager → devices_create → expected not 401/403, got {status}"
            );
        }

        // =====================================================================
        // Case 4: Employee session → POST /api/v1/acts_create → 403 Forbidden
        // =====================================================================
        {
            let status = post_with_cookie(
                new_app!(),
                "/api/v1/acts_create",
                act_payload.clone(),
                Some(&employee_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::FORBIDDEN,
                "Case 4: Employee → acts_create → expected 403, got {status}"
            );
        }

        // =====================================================================
        // Case 5: Employee session → POST /api/v1/cartridges_create → 403 Forbidden
        // =====================================================================
        {
            let status = post_with_cookie(
                new_app!(),
                "/api/v1/cartridges_create",
                cartridge_payload.clone(),
                Some(&employee_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::FORBIDDEN,
                "Case 5: Employee → cartridges_create → expected 403, got {status}"
            );
        }

        // =====================================================================
        // Case 6: Employee session → POST /api/v1/users_create → 403 Forbidden
        // =====================================================================
        {
            let status = post_with_cookie(
                new_app!(),
                "/api/v1/users_create",
                user_create_payload.clone(),
                Some(&employee_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::FORBIDDEN,
                "Case 6: Employee → users_create → expected 403, got {status}"
            );
        }

        // =====================================================================
        // Case 7: Manager session → POST /api/v1/users_create → 403 Forbidden
        // =====================================================================
        {
            let status = post_with_cookie(
                new_app!(),
                "/api/v1/users_create",
                user_create_payload.clone(),
                Some(&manager_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::FORBIDDEN,
                "Case 7: Manager → users_create → expected 403, got {status}"
            );
        }

        // =====================================================================
        // Case 8: Admin session → POST /api/v1/users_create → not 401/403
        // =====================================================================
        {
            let status = post_with_cookie(
                new_app!(),
                "/api/v1/users_create",
                user_create_payload.clone(),
                Some(&admin_cookie),
            )
            .await;
            assert!(
                status != StatusCode::UNAUTHORIZED && status != StatusCode::FORBIDDEN,
                "Case 8: Admin → users_create → expected not 401/403, got {status}"
            );
        }

        // =====================================================================
        // Case 9: Employee session → GET-like POST /api/v1/devices_list → 200 OK
        // =====================================================================
        {
            let status = post_with_cookie(
                new_app!(),
                "/api/v1/devices_list",
                device_list_payload.clone(),
                Some(&employee_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::OK,
                "Case 9: Employee → devices_list (read) → expected 200, got {status}"
            );
        }

        ctx.shutdown.cancel();
    })
    .await
    .expect("role_endpoint_matrix_test exceeded 60s budget");
}

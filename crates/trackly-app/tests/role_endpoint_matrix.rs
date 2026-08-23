//! Role×Endpoint matrix CI test — Phase 5 Plan 04 (GREEN after RBAC retrofit).
//!
//! ROADMAP success criterion #3: при попытке через curl дёрнуть mutation-эндпоинт
//! устройств/актов/картриджей сотрудник получает 403 Forbidden.
//!
//! Test matrix (15 cases):
//! 1. No session → POST /api/v1/devices_create → 401 Unauthorized
//! 2. Employee session → POST /api/v1/devices_create → 403 Forbidden
//! 3. Manager session → POST /api/v1/devices_create → not 401/403 (200 or 422)
//! 4. Employee session → POST /api/v1/acts_create → 403 Forbidden
//! 5. Employee session → POST /api/v1/cartridges_create → 403 Forbidden
//! 6. Employee session → POST /api/v1/users_create → 403 Forbidden
//! 7. Manager session → POST /api/v1/users_create → 403 Forbidden (admin only)
//! 8. Admin session → POST /api/v1/users_create → not 401/403 (200 or 422)
//! 9. Employee session → POST /api/v1/devices_list → 403 Forbidden (reads now gated — D-GATE-01/02)
//! 10. Employee session → POST /api/v1/requests_list → 200 OK (own-requests read retained)
//! 11. Employee session → POST /api/v1/acts_list → 403 Forbidden (reads now gated — D-GATE-01/02)
//! 12. Manager session → POST /api/v1/acts_list → not 401/403 (200 or 422)
//! 13. Employee session → POST /api/v1/cartridges_list → 403 Forbidden (reads now gated — D-GATE-01/02)
//! 14. Manager session → POST /api/v1/cartridges_list → not 401/403 (200 or 422)
//! 15. Employee session → POST /api/v1/printers_list → 403 Forbidden (reads now gated — D-GATE-01/02)
//! 16. Manager session → POST /api/v1/printers_list → not 401/403 (200 or 422)
//! 17. Employee session → POST /api/v1/reports_list_device_acts → 403 Forbidden (reads now gated — D-GATE-01/02)
//! 18. Manager session → POST /api/v1/reports_list_device_acts → not 401/403 (200 or 422)
//! 19. Employee session → POST /api/v1/users_list → 403 Forbidden (regression-proof — already gated, CR-03)
//!
//! Plan 10-03 adds Cases 16-20 (renumbered 20-24 below to avoid colliding with
//! 10-02's Cases 16-19 listed above): own-requests override (D-REQ-01), BOLA
//! closure on requests_get/requests_get_history (D-REQ-01/BOLA), dashboard
//! scoping (D-GATE-03), Manager/Admin regression.
//! 20. Employee session → POST /api/v1/requests_list with requestedByUserId
//!     forged to another user's id → 200 OK, every item's requestedByUserId
//!     == employee_dto.id (server overrides, not just defaults — D-REQ-01)
//! 21. Employee session → POST /api/v1/requests_get_history on a
//!     manager-owned request id → 403 Forbidden (BOLA closure)
//! 22. Employee session → POST /api/v1/requests_get on a manager-owned
//!     request id → 403 Forbidden (BOLA closure)
//! 23. Employee session → POST /api/v1/dashboard_get_all_widgets → 200 OK,
//!     org-wide fields (devices/cartridges/printers) all zeroed/empty
//!     (D-GATE-03)
//! 24. Manager session regression: dashboard_get_all_widgets still returns
//!     the full org-wide shape; requests_get/requests_get_history against
//!     the employee-owned request are NOT Forbidden (Manager retains full
//!     visibility)
//!
//! Cases 25-30 (devices_export_csv, dashboard_get_consumption_chart,
//! request_printer_options gating) were added by later quick-tasks/plans
//! without updating this header — see their inline `// Case N` comments below.
//!
//! Plan 12-02 (T-12-01) adds Cases 31-32: closes a test-coverage gap on two
//! transition endpoints that were already RBAC-gated in the service layer
//! but never exercised by this matrix.
//! 31. Employee session → POST /api/v1/cartridges_transition → 403 Forbidden
//!     (Action::MutateCartridges, Admin|Manager only)
//! 32. Employee session → POST /api/v1/requests_transition on their OWN
//!     request → 403 Forbidden (Action::TransitionRequests, Admin|Manager
//!     only — the gate fires before any ownership check)
//!
//! Plan 12-05 (T-12-05-02, GAP-12-02) added Cases 33-35 for the V029
//! per-device junction compatibility commands (printers_get_compatible_models,
//! printers_set_compatible_models, cartridge_models_set_compatible_devices).
//! Plan 13-02 removed those commands from both transports (V029 table dropped
//! in Plan 13-01) — Cases 33-35 were removed accordingly. A replacement
//! read-only aggregate command (R4) is expected to land its own RBAC case in
//! Plan 13-03.
//!
//! Plan 12-14 (GAP-12-07/A4) adds Cases 36-39: Admin/Manager request
//! deletion (any status) + Employee self-cancel (own request, open only) —
//! a separate path from the Admin/Manager-only `transition()` dispatcher.
//! 36. Employee session → POST /api/v1/requests_delete → 403 Forbidden
//!     (Action::DeleteRequests, Admin|Manager only)
//! 37. Manager session → POST /api/v1/requests_delete on a "completed"
//!     request → 200 OK (delete allowed in ANY status, not just open)
//! 38. Employee session (author) → POST /api/v1/requests_cancel on their
//!     OWN "open" request → 200 OK, response status == "cancelled"
//! 39. Employee session (not author) → POST /api/v1/requests_cancel on the
//!     manager-owned "open" request → 403 Forbidden (BOLA)
//!
//! Plan 12-21 (Round 5 gap closure, GAP-12-13) adds Case 40: новая
//! device-id-keyed read команда — тот же класс гейта, что и printers_get.
//! 40. Employee session → POST /api/v1/printers_get_by_device_id → 403
//!     Forbidden (Action::ReadData, Admin|Manager only).
//!
//! Plan 13-03 adds Case 41: the new R4 read-only aggregate command replacing
//! the deleted V029 per-device junction commands (Cases 33-35, removed).
//! 41. Employee session → POST /api/v1/printers_get_compatible_aggregates →
//!     403 Forbidden (Action::ReadData, Admin|Manager only — same gate as
//!     printers_get/printers_get_by_device_id).
//!
//! Quick task 260819-wq5 adds Case 44: new mutation command, same
//! ManageSettings gate as settings_set_low_stock_threshold.
//! 44. Employee session → POST /api/v1/settings_set_low_stock_basis → 403
//!     Forbidden (Action::ManageSettings).
//!
//! Phase 39 Plan 12 adds Cases 45-48: D-20's non-standard Admin-only-mutate /
//! Admin+Manager-read Places split, proven on BOTH transports — this is the
//! one entity in the whole matrix where Manager is rejected on a mutation
//! that every other entity's equivalent endpoint would accept (T-39-12-01/02).
//! 45. Manager session (HTTP) → POST /api/v1/places_create /
//!     places_rename / places_move / places_archive / places_unarchive /
//!     places_delete → 403 Forbidden for all six (Action::MutatePlaces,
//!     Admin-only — the regression test explicitly designed to catch a
//!     copy-paste of the MutateDevices/MutateCartridges Admin|Manager bucket).
//! 46. Manager session (HTTP) → POST /api/v1/places_list_all /
//!     places_get → not 401/403 (Action::ReadPlaces, Admin|Manager — proves
//!     the split is precise, Manager is NOT blocked from everything
//!     places-related, only from mutations).
//! 47. Employee session (HTTP) → POST /api/v1/places_list_all /
//!     places_get → 403 Forbidden (Action::ReadPlaces denies Employee).
//! 48. Manager Identity (Tauri path — build_places_* helpers called
//!     directly, the exact function every `#[tauri::command]` wrapper
//!     delegates to) → create/rename/move/archive/unarchive/delete →
//!     Err(AppError::Forbidden) for all six, mirroring Case 45 on the
//!     second transport.
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
use trackly_app::dto::place::PlaceNewDto;
use trackly_app::dto::request::RequestCreateDto;
use trackly_app::http::auth::SessionIdentity;
use trackly_app::http::build_router;
use trackly_app::server::rusqlite_session_store::RusqliteSessionStore;
use trackly_app::tauri_cmds::places::{
    build_places_archive, build_places_create, build_places_delete, build_places_move,
    build_places_rename, build_places_unarchive,
};
use trackly_core::auth::{Identity, Role};
use trackly_core::error::AppError;

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

/// Выполнить POST запрос с опциональным cookie, вернуть статус И тело (JSON).
///
/// Body-aware вариант `post_with_cookie` — используется тестами, которым нужно
/// проверить не только status code, но и содержимое ответа (например, "только
/// свои заявки в списке" / "нет org-wide полей в employee-дашборде").
/// При ошибке парсинга тела (например, пустое тело на 403) возвращает `json!({})`.
async fn post_with_cookie_json(
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
    let bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024)
        .await
        .unwrap_or_default();
    let value = serde_json::from_slice(&bytes).unwrap_or_else(|_| json!({}));
    (status, value)
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

        // Phase 19 Plan 04 (ACT-02): acts_update — id/expected_version don't
        // need to reference a real act, RBAC must reject before any lookup.
        let act_update_payload = json!({
            "payload": {
                "id": 1,
                "expected_version": 1,
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

        // Phase 22 Plan 03 (ACT-03): acts_update_return — id/expected_version
        // don't need to reference a real return act, RBAC must reject before
        // any lookup (same shape precedent as act_update_payload above).
        let act_update_return_payload = json!({
            "payload": {
                "id": 1,
                "expected_version": 1,
                "giver_name": "Тест",
                "receiver_name": "Тест2",
                "location_id": null,
                "location_name": null,
                "notes": null,
                "deadline_utc": null,
                "handover_date_utc": 0,
                "bulk_condition": null,
                "bulk_location_id": null,
                "bulk_location_name": null,
                "apply_to_all": false,
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

        // Case 10: Employee's own requests_list — empty filter, ReadRequests
        // action (separate from ReadData, untouched by the Task 1 matrix fix).
        let requests_list_payload = json!({
            "filter": {
                "status": null,
                "requestType": null,
                "assignedToUserId": null,
                "requestedByUserId": null
            },
            "pagination": { "offset": 0, "limit": 20 }
        });

        // Case 11/12: acts_list — ActFilter (snake_case fields) + Pagination.
        let acts_list_payload = json!({
            "filter": {
                "act_type": null,
                "archived": null,
                "search": null,
                "include_deleted": false
            },
            "pagination": { "offset": 0, "limit": 20 }
        });

        // Case 13/14: cartridges_list — CartridgeFilter (snake_case fields) + Pagination.
        let cartridges_list_payload = json!({
            "filter": {
                "status_id": null,
                "kind_id": null,
                "model_id": null,
                "search": null,
                "include_deleted": false
            },
            "pagination": { "offset": 0, "limit": 20 }
        });

        // Case 15/16: printers_list — PrinterFilter (camelCase rename_all) + Pagination.
        let printers_list_payload = json!({
            "filter": {
                "status": null,
                "search": null
            },
            "pagination": { "offset": 0, "limit": 20 }
        });

        // Case 17/18: reports_list_device_acts — ReportFilter + PeriodDto
        // (both snake_case fields; only the wrapper payload struct is camelCase).
        let reports_list_device_acts_payload = json!({
            "filter": {
                "date_from_utc": null,
                "date_to_utc": null,
                "location_id": null,
                "status_id": null,
                "type_id": null,
                "act_type": null,
                "model_id": null,
                "color": null,
                "search": null
            },
            "period": {
                "mode": "year",
                "year": 2026,
                "month": null,
                "date_from": null,
                "date_to": null
            }
        });

        // Case 19: users_list — UserFilter (snake_case fields) + Pagination
        // (regression-proof, CR-03 — already gated, not part of this plan's fix).
        let users_list_payload = json!({
            "filter": {
                "search": null
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
        // Case 9: Employee session → POST /api/v1/devices_list → 403 Forbidden
        // (reads now gated — D-GATE-01/02)
        // =====================================================================
        {
            let (status, _body) = post_with_cookie_json(
                new_app!(),
                "/api/v1/devices_list",
                device_list_payload.clone(),
                Some(&employee_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::FORBIDDEN,
                "Case 9: Employee → devices_list (reads now gated — D-GATE-01/02) → expected 403, got {status}"
            );
        }

        // =====================================================================
        // Case 10: Employee session → POST /api/v1/requests_list → 200 OK
        // (own-requests read retained — ReadRequests is unaffected by the
        // ReadData matrix fix; ownership filtering is wired in Plan 10-03)
        // =====================================================================
        {
            let status = post_with_cookie(
                new_app!(),
                "/api/v1/requests_list",
                requests_list_payload.clone(),
                Some(&employee_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::OK,
                "Case 10: Employee → requests_list (own-requests read retained) → expected 200, got {status}"
            );
        }

        // =====================================================================
        // Case 11: Employee session → POST /api/v1/acts_list → 403 Forbidden
        // (reads now gated — D-GATE-01/02)
        // =====================================================================
        {
            let status = post_with_cookie(
                new_app!(),
                "/api/v1/acts_list",
                acts_list_payload.clone(),
                Some(&employee_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::FORBIDDEN,
                "Case 11: Employee → acts_list (reads now gated — D-GATE-01/02) → expected 403, got {status}"
            );
        }

        // =====================================================================
        // Case 12: Manager session → POST /api/v1/acts_list → not 401/403
        // =====================================================================
        {
            let status = post_with_cookie(
                new_app!(),
                "/api/v1/acts_list",
                acts_list_payload.clone(),
                Some(&manager_cookie),
            )
            .await;
            assert!(
                status != StatusCode::UNAUTHORIZED && status != StatusCode::FORBIDDEN,
                "Case 12: Manager → acts_list → expected not 401/403, got {status}"
            );
        }

        // =====================================================================
        // Case 13: Employee session → POST /api/v1/cartridges_list → 403 Forbidden
        // (reads now gated — D-GATE-01/02)
        // =====================================================================
        {
            let status = post_with_cookie(
                new_app!(),
                "/api/v1/cartridges_list",
                cartridges_list_payload.clone(),
                Some(&employee_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::FORBIDDEN,
                "Case 13: Employee → cartridges_list (reads now gated — D-GATE-01/02) → expected 403, got {status}"
            );
        }

        // =====================================================================
        // Case 14: Manager session → POST /api/v1/cartridges_list → not 401/403
        // =====================================================================
        {
            let status = post_with_cookie(
                new_app!(),
                "/api/v1/cartridges_list",
                cartridges_list_payload.clone(),
                Some(&manager_cookie),
            )
            .await;
            assert!(
                status != StatusCode::UNAUTHORIZED && status != StatusCode::FORBIDDEN,
                "Case 14: Manager → cartridges_list → expected not 401/403, got {status}"
            );
        }

        // =====================================================================
        // Case 15: Employee session → POST /api/v1/printers_list → 403 Forbidden
        // (reads now gated — D-GATE-01/02)
        // =====================================================================
        {
            let status = post_with_cookie(
                new_app!(),
                "/api/v1/printers_list",
                printers_list_payload.clone(),
                Some(&employee_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::FORBIDDEN,
                "Case 15: Employee → printers_list (reads now gated — D-GATE-01/02) → expected 403, got {status}"
            );
        }

        // =====================================================================
        // Case 16: Manager session → POST /api/v1/printers_list → not 401/403
        // =====================================================================
        {
            let status = post_with_cookie(
                new_app!(),
                "/api/v1/printers_list",
                printers_list_payload.clone(),
                Some(&manager_cookie),
            )
            .await;
            assert!(
                status != StatusCode::UNAUTHORIZED && status != StatusCode::FORBIDDEN,
                "Case 16: Manager → printers_list → expected not 401/403, got {status}"
            );
        }

        // =====================================================================
        // Case 17: Employee session → POST /api/v1/reports_list_device_acts → 403 Forbidden
        // (reads now gated — D-GATE-01/02)
        // =====================================================================
        {
            let status = post_with_cookie(
                new_app!(),
                "/api/v1/reports_list_device_acts",
                reports_list_device_acts_payload.clone(),
                Some(&employee_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::FORBIDDEN,
                "Case 17: Employee → reports_list_device_acts (reads now gated — D-GATE-01/02) → expected 403, got {status}"
            );
        }

        // =====================================================================
        // Case 18: Manager session → POST /api/v1/reports_list_device_acts → not 401/403
        // =====================================================================
        {
            let status = post_with_cookie(
                new_app!(),
                "/api/v1/reports_list_device_acts",
                reports_list_device_acts_payload.clone(),
                Some(&manager_cookie),
            )
            .await;
            assert!(
                status != StatusCode::UNAUTHORIZED && status != StatusCode::FORBIDDEN,
                "Case 18: Manager → reports_list_device_acts → expected not 401/403, got {status}"
            );
        }

        // =====================================================================
        // Case 19: Employee session → POST /api/v1/users_list → 403 Forbidden
        // (regression-proof — already gated via CR-03, not part of this plan's fix)
        // =====================================================================
        {
            let status = post_with_cookie(
                new_app!(),
                "/api/v1/users_list",
                users_list_payload.clone(),
                Some(&employee_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::FORBIDDEN,
                "Case 19: Employee → users_list (regression-proof, CR-03) → expected 403, got {status}"
            );
        }

        // =====================================================================
        // Fixtures for Cases 20-24 (Plan 10-03): two requests with distinct
        // owners — one belongs to the Employee, one to the Manager.
        // =====================================================================
        let employee_identity = Identity {
            user_id: Some(employee_dto.id),
            role: Role::Employee,
        };
        let manager_identity = Identity {
            user_id: Some(manager_dto.id),
            role: Role::Manager,
        };

        let employee_request = ctx
            .requests
            .create(
                RequestCreateDto {
                    request_type: "free_form".to_string(),
                    printer_device_id: None,
                    cartridge_model_id: None,
                    category_id: None,
                    description: Some("employee-owned request".to_string()),
                },
                &employee_identity,
            )
            .await
            .expect("create employee-owned request");

        let manager_request = ctx
            .requests
            .create(
                RequestCreateDto {
                    request_type: "free_form".to_string(),
                    printer_device_id: None,
                    cartridge_model_id: None,
                    category_id: None,
                    description: Some("manager-owned request".to_string()),
                },
                &manager_identity,
            )
            .await
            .expect("create manager-owned request");

        // =====================================================================
        // Case 20 (D-REQ-01, own-requests override): Employee POSTs
        // requests_list with requestedByUserId forged to the manager's id —
        // server must override, not trust, the client-supplied filter.
        // =====================================================================
        {
            let forged_payload = json!({
                "filter": {
                    "status": null,
                    "requestType": null,
                    "assignedToUserId": null,
                    "requestedByUserId": manager_dto.id
                },
                "pagination": { "offset": 0, "limit": 20 }
            });

            let (status, body) = post_with_cookie_json(
                new_app!(),
                "/api/v1/requests_list",
                forged_payload,
                Some(&employee_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::OK,
                "Case 20: Employee → requests_list (forged requestedByUserId) → expected 200, got {status}"
            );
            let items = body["items"]
                .as_array()
                .expect("Case 20: response body missing 'items' array");
            assert!(
                !items.is_empty(),
                "Case 20: requests_list returned an empty items array — assertion below would be \
                 vacuously true, which would mask a missing override; expected at least the \
                 employee-owned request fixture"
            );
            for item in items {
                let owner = item["requestedByUserId"]
                    .as_i64()
                    .expect("Case 20: item missing requestedByUserId");
                assert_eq!(
                    owner, employee_dto.id,
                    "Case 20: Employee's requests_list returned a request owned by {owner}, \
                     not the caller ({}) — server-side override failed (D-REQ-01)",
                    employee_dto.id
                );
            }
        }

        // =====================================================================
        // Case 21 (BOLA close on get_history): Employee POSTs
        // requests_get_history for the manager-owned request id.
        // =====================================================================
        {
            let payload = json!({ "id": manager_request.id });
            let (status, _body) = post_with_cookie_json(
                new_app!(),
                "/api/v1/requests_get_history",
                payload,
                Some(&employee_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::FORBIDDEN,
                "Case 21: Employee → requests_get_history (manager-owned id) → expected 403, got {status}"
            );
        }

        // =====================================================================
        // Case 22 (BOLA close on get): Employee POSTs requests_get for the
        // manager-owned request id.
        // =====================================================================
        {
            let payload = json!({ "id": manager_request.id });
            let (status, _body) = post_with_cookie_json(
                new_app!(),
                "/api/v1/requests_get",
                payload,
                Some(&employee_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::FORBIDDEN,
                "Case 22: Employee → requests_get (manager-owned id) → expected 403, got {status}"
            );
        }

        // =====================================================================
        // Case 23 (D-GATE-03 dashboard body assertion): Employee POSTs
        // dashboard_get_all_widgets — org-wide fields must be zeroed/empty.
        // NOTE: snake_case keys — DashboardWidgetDto has no
        // `#[serde(rename_all = "camelCase")]` (verified by direct read of
        // dto/reports.rs), unlike RequestDto/RequestFilter which do.
        // =====================================================================
        {
            let (status, body) = post_with_cookie_json(
                new_app!(),
                "/api/v1/dashboard_get_all_widgets",
                json!({ "period": null }),
                Some(&employee_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::OK,
                "Case 23: Employee → dashboard_get_all_widgets → expected 200, got {status}"
            );
            assert_eq!(
                body["devices_total"], 0,
                "Case 23: employee dashboard devices_total must be 0, got {:?}",
                body["devices_total"]
            );
            assert_eq!(
                body["devices_by_status"].as_array().map(|a| a.len()),
                Some(0),
                "Case 23: employee dashboard devices_by_status must be empty, got {:?}",
                body["devices_by_status"]
            );
            assert_eq!(
                body["cartridge_by_status"].as_array().map(|a| a.len()),
                Some(0),
                "Case 23: employee dashboard cartridge_by_status must be empty, got {:?}",
                body["cartridge_by_status"]
            );
            assert_eq!(
                body["low_stock_count"], 0,
                "Case 23: employee dashboard low_stock_count must be 0, got {:?}",
                body["low_stock_count"]
            );
            assert_eq!(
                body["low_stock_models"].as_array().map(|a| a.len()),
                Some(0),
                "Case 23: employee dashboard low_stock_models must be empty, got {:?}",
                body["low_stock_models"]
            );
            assert_eq!(
                body["printer_online"], 0,
                "Case 23: employee dashboard printer_online must be 0, got {:?}",
                body["printer_online"]
            );
            assert_eq!(
                body["printer_offline"], 0,
                "Case 23: employee dashboard printer_offline must be 0, got {:?}",
                body["printer_offline"]
            );
            assert_eq!(
                body["printer_problematic"], 0,
                "Case 23: employee dashboard printer_problematic must be 0, got {:?}",
                body["printer_problematic"]
            );
            // request_counts_* are NOT asserted to an exact value here — the
            // employee-owned request created above affects these counts;
            // only presence-as-number is implied by the snake_case keys
            // existing in the deserialized body (any access above would have
            // panicked on a totally missing/null shape).
        }

        // =====================================================================
        // Case 24 (Manager/Admin regression): dashboard_get_all_widgets keeps
        // the full org-wide shape for Manager; requests_get/get_history
        // against the employee-owned request are NOT Forbidden for Manager.
        // =====================================================================
        {
            let (status, body) = post_with_cookie_json(
                new_app!(),
                "/api/v1/dashboard_get_all_widgets",
                json!({ "period": null }),
                Some(&manager_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::OK,
                "Case 24: Manager → dashboard_get_all_widgets → expected 200, got {status}"
            );
            assert!(
                body.get("devices_by_status").is_some(),
                "Case 24: Manager dashboard response missing 'devices_by_status' key — \
                 org-wide shape must be preserved for Manager, got {body:?}"
            );
            assert!(
                body.get("cartridge_by_status").is_some(),
                "Case 24: Manager dashboard response missing 'cartridge_by_status' key — \
                 org-wide shape must be preserved for Manager, got {body:?}"
            );

            let get_payload = json!({ "id": employee_request.id });
            let (status, _body) = post_with_cookie_json(
                new_app!(),
                "/api/v1/requests_get",
                get_payload.clone(),
                Some(&manager_cookie),
            )
            .await;
            assert!(
                status != StatusCode::UNAUTHORIZED && status != StatusCode::FORBIDDEN,
                "Case 24: Manager → requests_get (employee-owned id) → expected not 401/403, got {status}"
            );

            let (status, _body) = post_with_cookie_json(
                new_app!(),
                "/api/v1/requests_get_history",
                get_payload,
                Some(&manager_cookie),
            )
            .await;
            assert!(
                status != StatusCode::UNAUTHORIZED && status != StatusCode::FORBIDDEN,
                "Case 24: Manager → requests_get_history (employee-owned id) → expected not 401/403, got {status}"
            );
        }

        // =====================================================================
        // Case 25/26: devices_export_csv — read-only data export, must be gated
        // (gap-closure: handler previously discarded _identity — D-GATE-02).
        // ExportCsvPayload = { filter: DeviceFilter }.
        // =====================================================================
        let devices_export_payload = json!({
            "filter": {
                "type_id": null,
                "location_id": null,
                "status_id": null,
                "state": null,
                "name_prefix": null,
                "include_deleted": false,
                "group_by_condition": false
            }
        });
        {
            let status = post_with_cookie(
                new_app!(),
                "/api/v1/devices_export_csv",
                devices_export_payload.clone(),
                Some(&employee_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::FORBIDDEN,
                "Case 25: Employee → devices_export_csv (read export now gated — D-GATE-02) → expected 403, got {status}"
            );
        }
        {
            let status = post_with_cookie(
                new_app!(),
                "/api/v1/devices_export_csv",
                devices_export_payload.clone(),
                Some(&manager_cookie),
            )
            .await;
            assert!(
                status != StatusCode::UNAUTHORIZED && status != StatusCode::FORBIDDEN,
                "Case 26: Manager → devices_export_csv → expected not 401/403, got {status}"
            );
        }

        // =====================================================================
        // Case 27/28: dashboard_get_consumption_chart — org-wide consumption
        // analytics, must be gated (gap-closure — D-GATE-02/D-GATE-03).
        // GetConsumptionChartPayload = { window_months: u8 }.
        // =====================================================================
        let consumption_chart_payload = json!({ "windowMonths": 6 });
        {
            let status = post_with_cookie(
                new_app!(),
                "/api/v1/dashboard_get_consumption_chart",
                consumption_chart_payload.clone(),
                Some(&employee_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::FORBIDDEN,
                "Case 27: Employee → dashboard_get_consumption_chart (org analytics gated — D-GATE-02/03) → expected 403, got {status}"
            );
        }
        {
            let status = post_with_cookie(
                new_app!(),
                "/api/v1/dashboard_get_consumption_chart",
                consumption_chart_payload.clone(),
                Some(&manager_cookie),
            )
            .await;
            assert!(
                status != StatusCode::UNAUTHORIZED && status != StatusCode::FORBIDDEN,
                "Case 28: Manager → dashboard_get_consumption_chart → expected not 401/403, got {status}"
            );
        }

        // =====================================================================
        // Case 29/30 (D-PRN-01, Plan 11-02): request_printer_options must stay
        // reachable by Employee (Action::CreateRequest gate, NOT ReadData/
        // ReadPrinters which Phase 10 closed for Employee) — regression guard
        // against this narrow read-endpoint accidentally being folded into
        // the ReadData gate in a future change.
        // =====================================================================
        {
            let status = post_with_cookie(
                new_app!(),
                "/api/v1/request_printer_options",
                json!({}),
                Some(&employee_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::OK,
                "Case 29: Employee → request_printer_options (CreateRequest-gated, D-PRN-01) → expected 200, got {status}"
            );
        }
        {
            let status = post_with_cookie(
                new_app!(),
                "/api/v1/request_printer_options",
                json!({}),
                None,
            )
            .await;
            assert_eq!(
                status,
                StatusCode::UNAUTHORIZED,
                "Case 30: No session → request_printer_options → expected 401, got {status}"
            );
        }

        // =====================================================================
        // Case 31 (T-12-01, Plan 12-02): Employee → cartridges_transition →
        // 403 Forbidden. RBAC gate (authorize(&Action::MutateCartridges))
        // fires before any DB read, so cartridge_id: 1 need not exist —
        // same pattern as Case 5 (cartridges_create not validating model_id).
        // =====================================================================
        {
            let cartridges_transition_payload = json!({
                "payload": {
                    "op": "install",
                    "cartridge_id": 1,
                    "version": 1,
                    "date_utc": 1_700_000_000,
                    "given_by_name": "Тест",
                    "given_to_name": "Тест2",
                    "location": "Каб. 1"
                }
            });
            let status = post_with_cookie(
                new_app!(),
                "/api/v1/cartridges_transition",
                cartridges_transition_payload,
                Some(&employee_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::FORBIDDEN,
                "Case 31: Employee → cartridges_transition → expected 403, got {status}"
            );
        }

        // =====================================================================
        // Case 32 (T-12-01, Plan 12-02): Employee → requests_transition on
        // their OWN request → 403 Forbidden. authorize(&Action::TransitionRequests)
        // (Admin|Manager only) fires before any ownership check — Employee is
        // denied even on a request they own, unlike ReadRequests/CreateRequest
        // which are scoped-but-allowed for Employee.
        // =====================================================================
        {
            let requests_transition_payload = json!({
                "payload": {
                    "op": "accept",
                    "requestId": employee_request.id,
                    "version": employee_request.version,
                    "assignedToUserId": null
                }
            });
            let status = post_with_cookie(
                new_app!(),
                "/api/v1/requests_transition",
                requests_transition_payload,
                Some(&employee_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::FORBIDDEN,
                "Case 32: Employee → requests_transition (even on own request) → expected 403, got {status}"
            );
        }

        // =====================================================================
        // Cases 33-35 (T-12-05-02, Plan 12-05) removed in Plan 13-02: the V029
        // per-device junction commands (printers_get_compatible_models,
        // printers_set_compatible_models, cartridge_models_set_compatible_devices)
        // no longer exist in either transport (Plan 13-01 dropped the table,
        // Plan 13-02 removed the service methods + their Tauri/HTTP wrappers).
        // A replacement read command (printers_get_compatible_aggregates, R4)
        // is planned for 13-03 and will get its own RBAC case there.
        // =====================================================================

        // =====================================================================
        // Case 36 (Plan 12-14, GAP-12-07/A4): Employee → requests_delete →
        // 403 Forbidden. authorize(&Action::DeleteRequests) (Admin|Manager
        // only) fires before any DB read, so id: 1 need not exist.
        // =====================================================================
        {
            let status = post_with_cookie(
                new_app!(),
                "/api/v1/requests_delete",
                json!({ "id": 1, "version": 1 }),
                Some(&employee_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::FORBIDDEN,
                "Case 36: Employee → requests_delete → expected 403, got {status}"
            );
        }

        // =====================================================================
        // Case 37 (Plan 12-14, GAP-12-07/A4): Manager → requests_delete on a
        // "completed" request → 200 OK. Delete must be allowed in ANY status,
        // not just "open" — drives a fresh request through accept→complete
        // via the service layer directly (transition() is already exercised
        // by Cases 31/32; this fixture only needs the end state).
        // =====================================================================
        {
            let to_complete = ctx
                .requests
                .create(
                    RequestCreateDto {
                        request_type: "free_form".to_string(),
                        printer_device_id: None,
                        cartridge_model_id: None,
                        category_id: None,
                        description: Some("to be completed then deleted".to_string()),
                    },
                    &employee_identity,
                )
                .await
                .expect("create request for Case 37 fixture");

            let accepted = ctx
                .requests
                .transition(
                    trackly_app::dto::request::RequestTransitionPayload::Accept {
                        request_id: to_complete.id,
                        version: to_complete.version,
                        assigned_to_user_id: None,
                    },
                    &manager_identity,
                )
                .await
                .expect("accept Case 37 fixture");

            let completed = ctx
                .requests
                .transition(
                    trackly_app::dto::request::RequestTransitionPayload::Complete {
                        request_id: accepted.id,
                        version: accepted.version,
                        notes: None,
                        linked_cartridge_id: None,
                    },
                    &manager_identity,
                )
                .await
                .expect("complete Case 37 fixture");
            assert_eq!(completed.status, "completed");

            let status = post_with_cookie(
                new_app!(),
                "/api/v1/requests_delete",
                json!({ "id": completed.id, "version": completed.version }),
                Some(&manager_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::OK,
                "Case 37: Manager → requests_delete (completed request) → expected 200, got {status}"
            );
        }

        // =====================================================================
        // Case 38 (Plan 12-14, GAP-12-07/A4): Employee (author) →
        // requests_cancel on their OWN "open" request → 200 OK, status
        // becomes "cancelled". Separate path from transition() (Case 32
        // proved transition() denies Employee outright).
        // =====================================================================
        {
            let (status, body) = post_with_cookie_json(
                new_app!(),
                "/api/v1/requests_cancel",
                json!({ "id": employee_request.id, "version": employee_request.version }),
                Some(&employee_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::OK,
                "Case 38: Employee → requests_cancel (own open request) → expected 200, got {status}"
            );
            assert_eq!(
                body["status"], "cancelled",
                "Case 38: cancelled request's status should be \"cancelled\", got {:?}",
                body["status"]
            );
        }

        // =====================================================================
        // Case 39 (Plan 12-14, GAP-12-07/A4): Employee (not author) →
        // requests_cancel on the manager-owned "open" request → 403
        // Forbidden (BOLA — ownership check inside RequestService::cancel).
        // =====================================================================
        {
            let status = post_with_cookie(
                new_app!(),
                "/api/v1/requests_cancel",
                json!({ "id": manager_request.id, "version": manager_request.version }),
                Some(&employee_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::FORBIDDEN,
                "Case 39: Employee → requests_cancel (manager-owned request) → expected 403, got {status}"
            );
        }

        // =====================================================================
        // Case 40 (Plan 12-21, Round 5 gap closure, GAP-12-13): Employee →
        // printers_get_by_device_id → 403 Forbidden. authorize(&Action::ReadData)
        // (Admin|Manager only) fires before any DB read, so device_id: 1 need
        // not exist — same gate class as printers_get.
        // =====================================================================
        {
            let status = post_with_cookie(
                new_app!(),
                "/api/v1/printers_get_by_device_id",
                json!({ "deviceId": 1 }),
                Some(&employee_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::FORBIDDEN,
                "Case 40: Employee → printers_get_by_device_id → expected 403, got {status}"
            );
        }

        // =====================================================================
        // Case 41 (Plan 13-03): Employee → printers_get_compatible_aggregates
        // → 403 Forbidden. Same authorize(&Action::ReadData) gate as
        // printers_get/printers_get_by_device_id — replaces the deleted V029
        // per-device junction commands (Cases 33-35).
        // =====================================================================
        {
            let status = post_with_cookie(
                new_app!(),
                "/api/v1/printers_get_compatible_aggregates",
                json!({ "deviceId": 1 }),
                Some(&employee_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::FORBIDDEN,
                "Case 41: Employee → printers_get_compatible_aggregates → expected 403, got {status}"
            );
        }

        // =====================================================================
        // Case 42 (Plan 19-04, ACT-02 transports): Employee session →
        // POST /api/v1/acts_update → 403 Forbidden. RBAC (Action::MutateActs)
        // must reject before any act lookup — id/expected_version don't need
        // to reference a real row.
        // =====================================================================
        {
            let status = post_with_cookie(
                new_app!(),
                "/api/v1/acts_update",
                act_update_payload.clone(),
                Some(&employee_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::FORBIDDEN,
                "Case 42: Employee → acts_update → expected 403, got {status}"
            );
        }

        // =====================================================================
        // Case 43 (Plan 22-03, ACT-03 transports): Employee session →
        // POST /api/v1/acts_update_return → 403 Forbidden. Same
        // authorize(&Action::MutateActs) gate as acts_update/acts_return/
        // acts_delete — RBAC must reject before any act lookup.
        // =====================================================================
        {
            let status = post_with_cookie(
                new_app!(),
                "/api/v1/acts_update_return",
                act_update_return_payload.clone(),
                Some(&employee_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::FORBIDDEN,
                "Case 43: Employee → acts_update_return → expected 403, got {status}"
            );
        }

        // =====================================================================
        // Case 44 (quick 260819-wq5): Employee session →
        // POST /api/v1/settings_set_low_stock_basis → 403 Forbidden. Same
        // authorize(&Action::ManageSettings) gate as settings_set_low_stock_threshold.
        // =====================================================================
        {
            let status = post_with_cookie(
                new_app!(),
                "/api/v1/settings_set_low_stock_basis",
                json!({"basis": "printer_model"}),
                Some(&employee_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::FORBIDDEN,
                "Case 44: Employee → settings_set_low_stock_basis → expected 403, got {status}"
            );
        }

        // =====================================================================
        // Case 45 (Phase 39 Plan 12, T-39-12-01): Manager session (HTTP) →
        // all six places_* mutations → 403 Forbidden. authorize(&Action::
        // MutatePlaces) is the FIRST line of every PlaceService mutation
        // method AND of every build_places_* helper (belt-and-suspenders),
        // so a nonexistent id/version is fine — the gate fires before any
        // DB lookup, same pattern as Case 5 (cartridges_create) / Case 31
        // (cartridges_transition).
        // =====================================================================
        {
            let create_payload = json!({
                "place": {
                    "parent_id": null,
                    "kind": "room",
                    "name": "D-20 Manager probe",
                    "level": null,
                    "is_storage": false,
                    "sort_order": null,
                    "notes": null
                }
            });
            let status = post_with_cookie(
                new_app!(),
                "/api/v1/places_create",
                create_payload,
                Some(&manager_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::FORBIDDEN,
                "Case 45: Manager → places_create → expected 403 (D-20 Admin-only), got {status}"
            );

            let rename_payload = json!({ "id": 1, "name": "Renamed", "version": 1 });
            let status = post_with_cookie(
                new_app!(),
                "/api/v1/places_rename",
                rename_payload,
                Some(&manager_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::FORBIDDEN,
                "Case 45: Manager → places_rename → expected 403 (D-20 Admin-only), got {status}"
            );

            let move_payload = json!({ "id": 1, "newParentId": null, "version": 1 });
            let status = post_with_cookie(
                new_app!(),
                "/api/v1/places_move",
                move_payload,
                Some(&manager_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::FORBIDDEN,
                "Case 45: Manager → places_move → expected 403 (D-20 Admin-only), got {status}"
            );

            let archive_payload = json!({ "id": 1, "version": 1 });
            let status = post_with_cookie(
                new_app!(),
                "/api/v1/places_archive",
                archive_payload,
                Some(&manager_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::FORBIDDEN,
                "Case 45: Manager → places_archive → expected 403 (D-20 Admin-only), got {status}"
            );

            let unarchive_payload = json!({ "id": 1, "version": 1 });
            let status = post_with_cookie(
                new_app!(),
                "/api/v1/places_unarchive",
                unarchive_payload,
                Some(&manager_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::FORBIDDEN,
                "Case 45: Manager → places_unarchive → expected 403 (D-20 Admin-only), got {status}"
            );

            let delete_payload = json!({ "id": 1, "version": 1 });
            let status = post_with_cookie(
                new_app!(),
                "/api/v1/places_delete",
                delete_payload,
                Some(&manager_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::FORBIDDEN,
                "Case 45: Manager → places_delete → expected 403 (D-20 Admin-only), got {status}"
            );
        }

        // =====================================================================
        // Case 46 (Phase 39 Plan 12, T-39-12-02): Manager session (HTTP) →
        // places_list_all / places_get → not 401/403. Proves the D-20 split
        // is precise: Manager CAN read places, only mutation is denied
        // (Case 45).
        // =====================================================================
        {
            let status = post_with_cookie(
                new_app!(),
                "/api/v1/places_list_all",
                json!({ "includeArchived": false }),
                Some(&manager_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::OK,
                "Case 46: Manager → places_list_all → expected 200, got {status}"
            );

            let status = post_with_cookie(
                new_app!(),
                "/api/v1/places_get",
                json!({ "id": 1 }),
                Some(&manager_cookie),
            )
            .await;
            assert!(
                status != StatusCode::UNAUTHORIZED && status != StatusCode::FORBIDDEN,
                "Case 46: Manager → places_get → expected not 401/403, got {status}"
            );
        }

        // =====================================================================
        // Case 47 (Phase 39 Plan 12, T-39-12-02): Employee session (HTTP) →
        // places_list_all / places_get → 403 Forbidden (Action::ReadPlaces
        // denies Employee, Admin|Manager only).
        // =====================================================================
        {
            let status = post_with_cookie(
                new_app!(),
                "/api/v1/places_list_all",
                json!({ "includeArchived": false }),
                Some(&employee_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::FORBIDDEN,
                "Case 47: Employee → places_list_all → expected 403, got {status}"
            );

            let status = post_with_cookie(
                new_app!(),
                "/api/v1/places_get",
                json!({ "id": 1 }),
                Some(&employee_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::FORBIDDEN,
                "Case 47: Employee → places_get → expected 403, got {status}"
            );
        }

        // =====================================================================
        // Case 48 (Phase 39 Plan 12, T-39-12-01): Manager Identity (Tauri
        // path) → build_places_* helpers called directly — the exact
        // function every #[tauri::command] wrapper delegates to after
        // resolve_tauri_identity — → Err(AppError::Forbidden) for all six
        // mutations, mirroring Case 45 on the second transport (mirrors the
        // devices_http_smoke.rs precedent of exercising build_devices_*
        // directly as "the Tauri path").
        // =====================================================================
        {
            let manager_id = Identity {
                user_id: Some(manager_dto.id),
                role: Role::Manager,
            };

            let new_place = PlaceNewDto {
                parent_id: None,
                kind: "room".to_string(),
                name: "D-20 Tauri-path Manager probe".to_string(),
                level: None,
                is_storage: false,
                sort_order: None,
                notes: None,
            };
            let result = build_places_create(&ctx, &manager_id, new_place).await;
            assert!(
                matches!(result, Err(AppError::Forbidden)),
                "Case 48: Manager (Tauri path) → build_places_create → expected \
                 Err(AppError::Forbidden), got {result:?}"
            );

            let result = build_places_rename(&ctx, &manager_id, 1, "Renamed".to_string(), 1).await;
            assert!(
                matches!(result, Err(AppError::Forbidden)),
                "Case 48: Manager (Tauri path) → build_places_rename → expected \
                 Err(AppError::Forbidden), got {result:?}"
            );

            let result = build_places_move(&ctx, &manager_id, 1, None, 1).await;
            assert!(
                matches!(result, Err(AppError::Forbidden)),
                "Case 48: Manager (Tauri path) → build_places_move → expected \
                 Err(AppError::Forbidden), got {result:?}"
            );

            let result = build_places_archive(&ctx, &manager_id, 1, 1).await;
            assert!(
                matches!(result, Err(AppError::Forbidden)),
                "Case 48: Manager (Tauri path) → build_places_archive → expected \
                 Err(AppError::Forbidden), got {result:?}"
            );

            let result = build_places_unarchive(&ctx, &manager_id, 1, 1).await;
            assert!(
                matches!(result, Err(AppError::Forbidden)),
                "Case 48: Manager (Tauri path) → build_places_unarchive → expected \
                 Err(AppError::Forbidden), got {result:?}"
            );

            let result = build_places_delete(&ctx, &manager_id, 1, 1).await;
            assert!(
                matches!(result, Err(AppError::Forbidden)),
                "Case 48: Manager (Tauri path) → build_places_delete → expected \
                 Err(AppError::Forbidden), got {result:?}"
            );
        }

        ctx.shutdown.cancel();
    })
    .await
    .expect("role_endpoint_matrix_test exceeded 60s budget");
}

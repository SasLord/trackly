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
//!     places_delete / places_set_path_variant → 403 Forbidden for all seven
//!     (Action::MutatePlaces, Admin-only — the regression test explicitly
//!     designed to catch a copy-paste of the
//!     MutateDevices/MutateCartridges Admin|Manager bucket). Седьмая мутация
//!     (places_set_path_variant) добавлена фазой 39.2 по IN-02: гейт у неё тот
//!     же Action::MutatePlaces, но Manager до этого был покрыт только на шести.
//! 46. Manager session (HTTP) → POST /api/v1/places_list_all /
//!     places_get → not 401/403 (Action::ReadPlaces, Admin|Manager — proves
//!     the split is precise, Manager is NOT blocked from everything
//!     places-related, only from mutations).
//! 47. Employee session (HTTP) → POST /api/v1/places_list_all /
//!     places_get → 403 Forbidden (Action::ReadPlaces denies Employee).
//! 48. Manager Identity (Tauri path — build_places_* helpers called
//!     directly, the exact function every `#[tauri::command]` wrapper
//!     delegates to) → create/rename/move/archive/unarchive/delete/
//!     set_path_variant → Err(AppError::Forbidden) for all seven,
//!     mirroring Case 45 on the second transport.
//!
//! Phase 40 Plan 14 (HST-01/02/04, D-12, IN-02) adds Cases 52-59: closes the
//! access-control regression coverage for every new endpoint this phase
//! introduced (timeline read — Plan 40-10, movements report list/export —
//! Plan 40-12, bulk-move — Plan 40-13). Each family gets one HTTP Case and
//! one Tauri Case (per the IN-02 lesson above — Case 45-48's own precedent —
//! a gate proven on only one transport is a real, previously-shipped gap in
//! this codebase).
//! 52. Manager session (HTTP) → POST /api/v1/place_movements_get_timeline →
//!     not 401/403; Employee session (HTTP) → same → 403 Forbidden
//!     (Action::ReadPlaces, D-12).
//! 53. Manager Identity (Tauri path) → build_place_movements_get_timeline →
//!     Ok; Employee Identity → same → Err(AppError::Forbidden).
//! 54. Manager session (HTTP) → POST /api/v1/reports_list_movements → 200;
//!     Employee session (HTTP) → same → 403 Forbidden.
//! 55. Manager Identity (Tauri path) → build_reports_list_movements → Ok;
//!     Employee Identity → same → Err(AppError::Forbidden). Asserts
//!     Action::ReadPlaces explicitly — the ONE report in the 13-report
//!     family gated on ReadPlaces instead of ReadData (D-12).
//! 56. Manager session (HTTP) → POST /api/v1/reports_export_csv and
//!     /api/v1/reports_export_pdf with reportType: "movements" → 200;
//!     Employee session (HTTP) → same two → 403 Forbidden.
//! 57. Manager Identity (Tauri path) → build_reports_export_csv /
//!     build_reports_export_pdf with report_type "movements" → Ok;
//!     Employee Identity → same two → Err(AppError::Forbidden).
//! 58. Manager session (HTTP) → POST /api/v1/places_move_subtree_contents →
//!     200; Employee session (HTTP) → same → 403 Forbidden (D-13: reuses
//!     MutateDevices + MutateCartridges, both Admin|Manager — NOT
//!     MutatePlaces, which would incorrectly deny Manager per D-20).
//! 59. Manager Identity (Tauri path) → build_places_move_subtree_contents →
//!     Ok; Employee Identity → same → Err(AppError::Forbidden).
//!
//! Phase 40.2 Plan 05 adds Cases 64-73: proves the Group A (CRUD, Admin-only
//! `Action::ManageSettings`) vs Group B (usage, per-popup `Action::Mutate{
//! Devices,Acts,Cartridges}` via `action_for_context`) split from RESEARCH
//! Pitfall 3 — the single non-trivial RBAC decision of this plan.
//! 64. Admin session (HTTP) → number_templates_create → 200, then
//!     number_templates_update_mask/_delete on the created id → 200, then
//!     number_templates_list → 200 (Group A, Admin allowed).
//! 65. Admin Identity (Tauri path) → build_number_templates_create/
//!     _update_mask/_delete/_list → Ok for all four, mirroring Case 64.
//! 66. Manager session (HTTP) → number_templates_create/_update_mask/
//!     _delete/_list → 403 Forbidden for all four — THE regression test for
//!     Pitfall 3: Manager must NOT be able to touch template CRUD even
//!     though Manager passes every other Mutate* gate in this matrix.
//! 67. Manager Identity (Tauri path) → build_number_templates_create/
//!     _update_mask/_delete/_list → Err(AppError::Forbidden) for all four,
//!     mirroring Case 66.
//! 68. Employee session (HTTP) → all 4 Group A endpoints AND all 5 Group B
//!     endpoints (list_by_context/peek_next/contexts_get/contexts_set/
//!     is_occupied, context=device_create) → 403 Forbidden for all nine.
//! 69. Employee Identity (Tauri path) → build_number_templates_* for the
//!     same nine → Err(AppError::Forbidden), mirroring Case 68.
//! 70. Manager session (HTTP) → number_templates_list_by_context with
//!     context=device_create / act_create / cartridge_create → not 401/403
//!     for all three — proves each `action_for_context` branch
//!     (MutateDevices/MutateActs/MutateCartridges) independently grants
//!     Manager access to their own popup's template menu.
//! 71. Manager Identity (Tauri path) → build_number_templates_list_by_context
//!     for the same three contexts → Ok, mirroring Case 70.
//! 72. Admin session (HTTP) → same three contexts → not 401/403 (Admin has
//!     every Mutate* right too).
//! 73. Admin Identity (Tauri path) → build_number_templates_list_by_context
//!     for the same three contexts → Ok, mirroring Case 72.
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
use trackly_app::dto::number_template::{TemplateContextDto, TemplateTypeDto};
use trackly_app::dto::place::PlaceNewDto;
use trackly_app::dto::reports::{PeriodDto, ReportFilter};
use trackly_app::dto::request::RequestCreateDto;
use trackly_app::http::auth::SessionIdentity;
use trackly_app::http::build_router;
use trackly_app::server::rusqlite_session_store::RusqliteSessionStore;
use trackly_app::tauri_cmds::number_templates::{
    build_number_template_contexts_get, build_number_template_contexts_set,
    build_number_templates_create, build_number_templates_delete,
    build_number_templates_is_occupied, build_number_templates_list,
    build_number_templates_list_by_context, build_number_templates_peek_next,
    build_number_templates_update_mask,
};
use trackly_app::tauri_cmds::place_movements::build_place_movements_get_timeline;
use trackly_app::tauri_cmds::places::{
    build_places_archive, build_places_create, build_places_delete, build_places_move,
    build_places_move_subtree_contents, build_places_rename, build_places_set_path_variant,
    build_places_unarchive,
};
use trackly_app::tauri_cmds::reports::{
    build_reports_export_csv, build_reports_export_pdf, build_reports_list_movements,
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
                "place_id": null,
                "status_id": 1
            }
        });

        let act_payload = json!({
            "payload": {
                "number_input": {
                    "value": "1",
                    "templateId": null,
                    "confirmMismatch": false,
                    "confirmScriptMix": false
                },
                "giver_name": "Тест Тестов",
                "receiver_name": "Тест2 Тестов",
                "place_id": null,
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
                "number_input": {
                    "value": "1",
                    "confirm_script_mix": false
                },
                "giver_name": "Тест Тестов",
                "receiver_name": "Тест2 Тестов",
                "place_id": null,
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
                "place_id": null,
                "notes": null,
                "deadline_utc": null,
                "handover_date_utc": 0,
                "bulk_condition": null,
                "bulk_place_id": null,
                "apply_to_all": false,
                "items": []
            }
        });

        // Phase 40.2 Plan 07 (NUM-13): `code_override: Option<String>` ->
        // `number_input: NumberFieldInput` (required, own camelCase rename_all)
        // — without it axum's `Json` extractor 422s before `authorize()` ever
        // runs, silently breaking the Case 5 RBAC assertion below (matches
        // Plan 06's identical acts_create fix).
        let cartridge_payload = json!({
            "payload": {
                "model_id": 1,
                "number_input": {
                    "value": "TEST-1",
                    "templateId": null,
                    "confirmMismatch": false,
                    "confirmScriptMix": false
                },
                "place_id": null,
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
                "place_id": null,
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

        // Case 60/61 (Plan 40-30, HST-01): cartridges_operation_default_place —
        // OperationDefaultPlacePayload (camelCase rename_all). Plan 40-33: op is
        // now "from_refill" — "to_refill" is no longer served by this endpoint
        // (superseded by cartridges_to_refill_last_send, UAT4-02). Authorization
        // is checked BEFORE op/cartridge_id validation, so the 403/not-403
        // asymmetry does not depend on payload validity.
        let operation_default_place_payload = json!({
            "op": "from_refill",
            "cartridgeId": null
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
                "place_id": null,
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

        // Cases 64-73 (Phase 40.2 Plan 05): numbering-template payloads.
        // Group A (CRUD, Action::ManageSettings, Admin-only).
        let number_template_create_payload = json!({
            "templateType": "device_inventory",
            "mask": "RBAC64-[XXXX]"
        });
        let number_template_list_payload = json!({ "templateType": null });
        // update_mask/delete payloads with a dummy id — RBAC denial (Manager/
        // Employee) fires in authorize() BEFORE any DB lookup, so a
        // nonexistent id is fine for those; the Admin-success case (64/65)
        // rewrites `id` to the real id returned by number_templates_create.
        let number_template_update_mask_dummy_payload = json!({
            "id": 999999,
            "mask": "RBAC64-UPD-[XXXX]",
            "version": 1
        });
        let number_template_delete_dummy_payload = json!({ "id": 999999 });

        // Group B (usage, gate derived from `context` via action_for_context).
        // device_create → Action::MutateDevices.
        let number_template_list_by_context_device_payload = json!({
            "context": "device_create"
        });
        let number_template_peek_next_device_payload = json!({
            "templateId": 999999,
            "context": "device_create"
        });
        let number_template_contexts_get_device_payload = json!({
            "context": "device_create"
        });
        let number_template_contexts_set_device_payload = json!({
            "context": "device_create",
            "templateId": null
        });
        let number_template_is_occupied_device_payload = json!({
            "context": "device_create",
            "candidate": "RBAC-64-CANDIDATE",
            "excludeId": null
        });
        // act_create → Action::MutateActs.
        let number_template_list_by_context_act_payload = json!({ "context": "act_create" });
        // cartridge_create → Action::MutateCartridges.
        let number_template_list_by_context_cartridge_payload =
            json!({ "context": "cartridge_create" });

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
                "place_id": null,
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
                    "place_id": null
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
        // Case 49 (Phase 39.1 Plan 02): Employee session → POST
        // /api/v1/settings_set_place_path_defaults → 403 Forbidden. Same
        // authorize(&Action::ManageSettings) gate as
        // settings_set_low_stock_basis/settings_set_low_stock_threshold.
        // =====================================================================
        {
            let status = post_with_cookie(
                new_app!(),
                "/api/v1/settings_set_place_path_defaults",
                json!({"patch": {"variant": "ends", "sep_ends": " // ", "sep_last_two": " / "}}),
                Some(&employee_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::FORBIDDEN,
                "Case 49: Employee → settings_set_place_path_defaults → expected 403, got {status}"
            );
        }

        // =====================================================================
        // Case 50 (Phase 39.1 Plan 07): Employee session → POST
        // /api/v1/places_set_path_variant → 403 Forbidden.
        // authorize(&Action::MutatePlaces) — D-12: та же семантика прав, что
        // places_rename, НЕ Action::ManageSettings.
        // =====================================================================
        {
            let status = post_with_cookie(
                new_app!(),
                "/api/v1/places_set_path_variant",
                json!({"id": 1, "pathVariantOverride": "ends", "version": 1}),
                Some(&employee_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::FORBIDDEN,
                "Case 50: Employee → places_set_path_variant → expected 403, got {status}"
            );
        }

        // =====================================================================
        // Case 45 (Phase 39 Plan 12, T-39-12-01): Manager session (HTTP) →
        // all seven places_* mutations → 403 Forbidden. authorize(&Action::
        // MutatePlaces) is the FIRST line of every PlaceService mutation
        // method AND of every build_places_* helper (belt-and-suspenders),
        // so a nonexistent id/version is fine — the gate fires before any
        // DB lookup, same pattern as Case 5 (cartridges_create) / Case 31
        // (cartridges_transition).
        //
        // Седьмая проверка (places_set_path_variant) добавлена фазой 39.2 по
        // IN-02: гейт тот же — Action::MutatePlaces, — но до неё Manager на
        // этой мутации не проверялся вообще, и её ослабление до Admin|Manager
        // прошло бы мимо матрицы. Employee на ней покрыт Case 50.
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

            let set_path_variant_payload =
                json!({ "id": 1, "pathVariantOverride": "ends", "version": 1 });
            let status = post_with_cookie(
                new_app!(),
                "/api/v1/places_set_path_variant",
                set_path_variant_payload,
                Some(&manager_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::FORBIDDEN,
                "Case 45: Manager → places_set_path_variant → expected 403 (D-20 Admin-only, \
                 IN-02), got {status}"
            );
        }

        // =====================================================================
        // Case 51 (Phase 39.2 Plan 03, IN-02): Manager session (HTTP) → POST
        // /api/v1/settings_set_place_path_defaults → 403 Forbidden.
        // authorize(&Action::ManageSettings) — Admin-only, как и у соседних
        // settings_set_low_stock_*. Симметрично Case 49 (тот же эндпоинт,
        // Employee): гейт был покрыт только на одной роли, поэтому его
        // ослабление до Admin|Manager матрица бы не заметила.
        // =====================================================================
        {
            let status = post_with_cookie(
                new_app!(),
                "/api/v1/settings_set_place_path_defaults",
                json!({"patch": {"variant": "ends", "sep_ends": " // ", "sep_last_two": " / "}}),
                Some(&manager_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::FORBIDDEN,
                "Case 51: Manager → settings_set_place_path_defaults → expected 403 \
                 (Action::ManageSettings, IN-02), got {status}"
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
        // resolve_tauri_identity — → Err(AppError::Forbidden) for all seven
        // mutations, mirroring Case 45 on the second transport (mirrors the
        // devices_http_smoke.rs precedent of exercising build_devices_*
        // directly as "the Tauri path").
        //
        // Седьмая мутация (set_path_variant) добавлена быстрозадачей 260901-qj7,
        // закрывающей остаток IN-02: Фаза 39.2 расширила только Case 45 (HTTP),
        // оставив здесь асимметрию. Право и тогда было покрыто — authorize()
        // стоит первой строкой ВНУТРИ build_places_set_path_variant, а оба
        // транспорта лишь делегируют, — но именно на случай, если Tauri-команда
        // однажды перестанет делегировать, этот кейс и заводится.
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

            let result =
                build_places_set_path_variant(&ctx, &manager_id, 1, Some("ends".to_string()), 1)
                    .await;
            assert!(
                matches!(result, Err(AppError::Forbidden)),
                "Case 48: Manager (Tauri path) → build_places_set_path_variant → expected \
                 Err(AppError::Forbidden), got {result:?}"
            );
        }

        // =====================================================================
        // Case 52 (Phase 40 Plan 14, HST-02, D-12/IN-02): Manager session
        // (HTTP) → POST /api/v1/place_movements_get_timeline → not 401/403;
        // Employee session (HTTP) → same endpoint → 403 Forbidden.
        // `build_place_movements_get_timeline` (Plan 40-10) gates on
        // `Action::ReadPlaces` (Admin|Manager). A nonexistent entity_id is
        // fine — the gate fires before any DB lookup (same pattern as Case 45
        // / Case 31), so this is a genuine success path, not an error the
        // Employee-deny half would be riding on.
        // =====================================================================
        {
            let timeline_payload = json!({ "entityType": "device", "entityId": 999_999 });

            let status = post_with_cookie(
                new_app!(),
                "/api/v1/place_movements_get_timeline",
                timeline_payload.clone(),
                Some(&manager_cookie),
            )
            .await;
            assert!(
                status != StatusCode::UNAUTHORIZED && status != StatusCode::FORBIDDEN,
                "Case 52: Manager → place_movements_get_timeline → expected not 401/403, got {status}"
            );

            let status = post_with_cookie(
                new_app!(),
                "/api/v1/place_movements_get_timeline",
                timeline_payload,
                Some(&employee_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::FORBIDDEN,
                "Case 52: Employee → place_movements_get_timeline → expected 403 \
                 (Action::ReadPlaces, D-12), got {status}"
            );
        }

        // =====================================================================
        // Case 53 (Phase 40 Plan 14, HST-02, D-12/IN-02): Manager Identity
        // (Tauri path) → build_place_movements_get_timeline (the exact
        // function the #[tauri::command] wrapper delegates to) → Ok;
        // Employee Identity → same call → Err(AppError::Forbidden). Mirrors
        // Case 52 on the second transport — the IN-02 lesson this plan
        // exists to prevent: a gate proven on only one transport is a real,
        // previously-shipped gap in this codebase.
        // =====================================================================
        {
            let manager_id = Identity {
                user_id: Some(manager_dto.id),
                role: Role::Manager,
            };
            let employee_id = Identity {
                user_id: Some(employee_dto.id),
                role: Role::Employee,
            };

            let result =
                build_place_movements_get_timeline(&ctx, &manager_id, "device".to_string(), 999_999)
                    .await;
            assert!(
                result.is_ok(),
                "Case 53: Manager (Tauri path) → build_place_movements_get_timeline → \
                 expected Ok, got {result:?}"
            );

            let result = build_place_movements_get_timeline(
                &ctx,
                &employee_id,
                "device".to_string(),
                999_999,
            )
            .await;
            assert!(
                matches!(result, Err(AppError::Forbidden)),
                "Case 53: Employee (Tauri path) → build_place_movements_get_timeline → \
                 expected Err(AppError::Forbidden), got {result:?}"
            );
        }

        // =====================================================================
        // Case 54 (Phase 40 Plan 14, HST-04, D-12/IN-02): Manager session
        // (HTTP) → POST /api/v1/reports_list_movements → 200; Employee
        // session (HTTP) → same endpoint → 403 Forbidden.
        // `build_reports_list_movements` (Plan 40-12) gates on
        // `Action::ReadPlaces` — the one report in the whole 13-report
        // family that diverges from `Action::ReadData` (see Case 55's Tauri
        // variant, which asserts the divergence explicitly).
        // =====================================================================
        let movements_filter_json = json!({
            "date_from_utc": null,
            "date_to_utc": null,
            "place_id": null,
            "from_place_id": null,
            "to_place_id": null,
            "status_id": null,
            "type_id": null,
            "act_type": null,
            "model_id": null,
            "color": null,
            "search": null,
            "request_category_filter": null,
            "is_storage": null
        });
        let movements_period_json = json!({
            "mode": "range",
            "year": null,
            "month": null,
            "date_from": "2000-01-01",
            "date_to": "2099-12-31"
        });
        {
            let movements_payload = json!({
                "filter": movements_filter_json.clone(),
                "period": movements_period_json.clone()
            });

            let status = post_with_cookie(
                new_app!(),
                "/api/v1/reports_list_movements",
                movements_payload.clone(),
                Some(&manager_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::OK,
                "Case 54: Manager → reports_list_movements → expected 200, got {status}"
            );

            let status = post_with_cookie(
                new_app!(),
                "/api/v1/reports_list_movements",
                movements_payload,
                Some(&employee_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::FORBIDDEN,
                "Case 54: Employee → reports_list_movements → expected 403 \
                 (Action::ReadPlaces, D-12), got {status}"
            );
        }

        // =====================================================================
        // Case 55 (Phase 40 Plan 14, HST-04, D-12/IN-02): Manager Identity
        // (Tauri path) → build_reports_list_movements → Ok; Employee
        // Identity → same call → Err(AppError::Forbidden). Explicitly
        // asserted against `Action::ReadPlaces` semantics (Admin|Manager) —
        // NOT `Action::ReadData`, which every other `build_reports_list_*`
        // uses — making the deliberate divergence visible in the test
        // itself, not just in the source comment on
        // `build_reports_list_movements`.
        // =====================================================================
        {
            let manager_id = Identity {
                user_id: Some(manager_dto.id),
                role: Role::Manager,
            };
            let employee_id = Identity {
                user_id: Some(employee_dto.id),
                role: Role::Employee,
            };
            let period = PeriodDto {
                mode: "range".to_string(),
                year: None,
                month: None,
                date_from: Some("2000-01-01".to_string()),
                date_to: Some("2099-12-31".to_string()),
            };

            let result = build_reports_list_movements(
                &ctx,
                &manager_id,
                ReportFilter::default(),
                period.clone(),
            )
            .await;
            assert!(
                result.is_ok(),
                "Case 55: Manager (Tauri path) → build_reports_list_movements → expected Ok \
                 (Action::ReadPlaces, D-12 — NOT Action::ReadData like every sibling report), \
                 got {result:?}"
            );

            let result =
                build_reports_list_movements(&ctx, &employee_id, ReportFilter::default(), period)
                    .await;
            assert!(
                matches!(result, Err(AppError::Forbidden)),
                "Case 55: Employee (Tauri path) → build_reports_list_movements → expected \
                 Err(AppError::Forbidden), got {result:?}"
            );
        }

        // =====================================================================
        // Case 56 (Phase 40 Plan 14, HST-04, D-12/IN-02): Manager session
        // (HTTP) → POST /api/v1/reports_export_csv and
        // /api/v1/reports_export_pdf with reportType: "movements" → 200;
        // Employee session (HTTP) → same two endpoints → 403 Forbidden. Both
        // handlers delegate to build_reports_export_csv/
        // build_reports_export_pdf, which gate on `Action::ReadData`
        // (unchanged, zero new export code per D-26) — currently the same
        // Admin|Manager/Employee-denied role split as `Action::ReadPlaces`,
        // so Manager-allow/Employee-deny holds for the movements report type
        // as well.
        // =====================================================================
        {
            let export_csv_payload = json!({
                "reportType": "movements",
                "filter": movements_filter_json.clone(),
                "period": movements_period_json.clone()
            });

            let status = post_with_cookie(
                new_app!(),
                "/api/v1/reports_export_csv",
                export_csv_payload.clone(),
                Some(&manager_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::OK,
                "Case 56: Manager → reports_export_csv (movements) → expected 200, got {status}"
            );

            let status = post_with_cookie(
                new_app!(),
                "/api/v1/reports_export_csv",
                export_csv_payload,
                Some(&employee_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::FORBIDDEN,
                "Case 56: Employee → reports_export_csv (movements) → expected 403, got {status}"
            );

            let export_pdf_payload = json!({
                "reportType": "movements",
                "filter": movements_filter_json.clone(),
                "period": movements_period_json.clone()
            });

            let status = post_with_cookie(
                new_app!(),
                "/api/v1/reports_export_pdf",
                export_pdf_payload.clone(),
                Some(&manager_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::OK,
                "Case 56: Manager → reports_export_pdf (movements) → expected 200, got {status}"
            );

            let status = post_with_cookie(
                new_app!(),
                "/api/v1/reports_export_pdf",
                export_pdf_payload,
                Some(&employee_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::FORBIDDEN,
                "Case 56: Employee → reports_export_pdf (movements) → expected 403, got {status}"
            );
        }

        // =====================================================================
        // Case 57 (Phase 40 Plan 14, HST-04, D-12/IN-02): Manager Identity
        // (Tauri path) → build_reports_export_csv / build_reports_export_pdf
        // with report_type "movements" → Ok; Employee Identity → same two
        // calls → Err(AppError::Forbidden). Mirrors Case 56 on the second
        // transport.
        // =====================================================================
        {
            let manager_id = Identity {
                user_id: Some(manager_dto.id),
                role: Role::Manager,
            };
            let employee_id = Identity {
                user_id: Some(employee_dto.id),
                role: Role::Employee,
            };
            let period = PeriodDto {
                mode: "range".to_string(),
                year: None,
                month: None,
                date_from: Some("2000-01-01".to_string()),
                date_to: Some("2099-12-31".to_string()),
            };

            let result = build_reports_export_csv(
                &ctx,
                &manager_id,
                "movements".to_string(),
                ReportFilter::default(),
                Some(period.clone()),
            )
            .await;
            assert!(
                result.is_ok(),
                "Case 57: Manager (Tauri path) → build_reports_export_csv (movements) → \
                 expected Ok, got {result:?}"
            );

            let result = build_reports_export_csv(
                &ctx,
                &employee_id,
                "movements".to_string(),
                ReportFilter::default(),
                Some(period.clone()),
            )
            .await;
            assert!(
                matches!(result, Err(AppError::Forbidden)),
                "Case 57: Employee (Tauri path) → build_reports_export_csv (movements) → \
                 expected Err(AppError::Forbidden), got {result:?}"
            );

            let result = build_reports_export_pdf(
                &ctx,
                &manager_id,
                "movements".to_string(),
                ReportFilter::default(),
                Some(period.clone()),
            )
            .await;
            assert!(
                result.is_ok(),
                "Case 57: Manager (Tauri path) → build_reports_export_pdf (movements) → \
                 expected Ok, got {result:?}"
            );

            let result = build_reports_export_pdf(
                &ctx,
                &employee_id,
                "movements".to_string(),
                ReportFilter::default(),
                Some(period),
            )
            .await;
            assert!(
                matches!(result, Err(AppError::Forbidden)),
                "Case 57: Employee (Tauri path) → build_reports_export_pdf (movements) → \
                 expected Err(AppError::Forbidden), got {result:?}"
            );
        }

        // =====================================================================
        // Case 58 (Phase 40 Plan 14, HST-01, D-13/IN-02): Manager session
        // (HTTP) → POST /api/v1/places_move_subtree_contents on an empty
        // subtree → 200 OK (D-13 reuses `Action::MutateDevices` +
        // `Action::MutateCartridges`, both Admin|Manager — deliberately NOT
        // `Action::MutatePlaces`, which would incorrectly deny Manager per
        // D-20); Employee session (HTTP) → same endpoint → 403 Forbidden. A
        // nonexistent root_id is fine — `list_subtree_contents`'s recursive
        // CTE simply returns zero rows, so the call still reaches a genuine
        // success path (Ok(0)), not an unrelated error the Employee-deny
        // half would be riding on.
        // =====================================================================
        {
            let bulk_move_payload = json!({
                "rootId": 1,
                "targetPlaceId": 1,
                "note": null
            });

            let status = post_with_cookie(
                new_app!(),
                "/api/v1/places_move_subtree_contents",
                bulk_move_payload.clone(),
                Some(&manager_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::OK,
                "Case 58: Manager → places_move_subtree_contents → expected 200, got {status}"
            );

            let status = post_with_cookie(
                new_app!(),
                "/api/v1/places_move_subtree_contents",
                bulk_move_payload,
                Some(&employee_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::FORBIDDEN,
                "Case 58: Employee → places_move_subtree_contents → expected 403, got {status}"
            );
        }

        // =====================================================================
        // Case 59 (Phase 40 Plan 14, HST-01, D-13/IN-02): Manager Identity
        // (Tauri path) → build_places_move_subtree_contents → Ok; Employee
        // Identity → same call → Err(AppError::Forbidden). Mirrors Case 58
        // on the second transport.
        // =====================================================================
        {
            let manager_id = Identity {
                user_id: Some(manager_dto.id),
                role: Role::Manager,
            };
            let employee_id = Identity {
                user_id: Some(employee_dto.id),
                role: Role::Employee,
            };

            let result =
                build_places_move_subtree_contents(&ctx, &manager_id, 1, 1, None).await;
            assert!(
                result.is_ok(),
                "Case 59: Manager (Tauri path) → build_places_move_subtree_contents → \
                 expected Ok, got {result:?}"
            );

            let result =
                build_places_move_subtree_contents(&ctx, &employee_id, 1, 1, None).await;
            assert!(
                matches!(result, Err(AppError::Forbidden)),
                "Case 59: Employee (Tauri path) → build_places_move_subtree_contents → \
                 expected Err(AppError::Forbidden), got {result:?}"
            );
        }

        // =====================================================================
        // Case 60: Employee session → POST /api/v1/cartridges_operation_default_place
        // → 403 Forbidden (Plan 40-30, HST-01)
        // =====================================================================
        {
            let status = post_with_cookie(
                new_app!(),
                "/api/v1/cartridges_operation_default_place",
                operation_default_place_payload.clone(),
                Some(&employee_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::FORBIDDEN,
                "Case 60: Employee → cartridges_operation_default_place → expected 403, got {status}"
            );
        }

        // =====================================================================
        // Case 61: Manager session → POST /api/v1/cartridges_operation_default_place
        // → not 401/403 (200)
        // =====================================================================
        {
            let status = post_with_cookie(
                new_app!(),
                "/api/v1/cartridges_operation_default_place",
                operation_default_place_payload.clone(),
                Some(&manager_cookie),
            )
            .await;
            assert!(
                status != StatusCode::UNAUTHORIZED && status != StatusCode::FORBIDDEN,
                "Case 61: Manager → cartridges_operation_default_place → expected not 401/403, got {status}"
            );
        }

        // =====================================================================
        // Case 62: Employee session → POST /api/v1/cartridges_to_refill_last_send
        // → 403 Forbidden (Plan 40-33, HST-01, UAT4-02)
        // =====================================================================
        {
            let status = post_with_cookie(
                new_app!(),
                "/api/v1/cartridges_to_refill_last_send",
                json!({}),
                Some(&employee_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::FORBIDDEN,
                "Case 62: Employee → cartridges_to_refill_last_send → expected 403, got {status}"
            );
        }

        // =====================================================================
        // Case 63: Manager session → POST /api/v1/cartridges_to_refill_last_send
        // → not 401/403 (200)
        // =====================================================================
        {
            let status = post_with_cookie(
                new_app!(),
                "/api/v1/cartridges_to_refill_last_send",
                json!({}),
                Some(&manager_cookie),
            )
            .await;
            assert!(
                status != StatusCode::UNAUTHORIZED && status != StatusCode::FORBIDDEN,
                "Case 63: Manager → cartridges_to_refill_last_send → expected not 401/403, got {status}"
            );
        }

        // =====================================================================
        // Case 64 (Phase 40.2 Plan 05, T-40.2-09): Admin session (HTTP) →
        // number_templates_create → 200 OK; number_templates_update_mask /
        // number_templates_delete on the just-created id → 200 OK;
        // number_templates_list → 200 OK. Group A, Admin allowed on all four.
        // =====================================================================
        {
            let (status, body) = post_with_cookie_json(
                new_app!(),
                "/api/v1/number_templates_create",
                number_template_create_payload.clone(),
                Some(&admin_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::OK,
                "Case 64: Admin → number_templates_create → expected 200, got {status}"
            );
            let created_id = body["id"].as_i64().expect("created template id");

            let status = post_with_cookie(
                new_app!(),
                "/api/v1/number_templates_update_mask",
                json!({ "id": created_id, "mask": "RBAC64-UPD-[XXXX]", "version": 1 }),
                Some(&admin_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::OK,
                "Case 64: Admin → number_templates_update_mask → expected 200, got {status}"
            );

            let status = post_with_cookie(
                new_app!(),
                "/api/v1/number_templates_delete",
                json!({ "id": created_id }),
                Some(&admin_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::OK,
                "Case 64: Admin → number_templates_delete → expected 200, got {status}"
            );

            let status = post_with_cookie(
                new_app!(),
                "/api/v1/number_templates_list",
                number_template_list_payload.clone(),
                Some(&admin_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::OK,
                "Case 64: Admin → number_templates_list → expected 200, got {status}"
            );
        }

        // =====================================================================
        // Case 65 (mirrors Case 64 on the Tauri path): Admin Identity →
        // build_number_templates_create/_update_mask/_delete/_list → Ok for
        // all four.
        // =====================================================================
        {
            let admin_id = Identity {
                user_id: Some(admin_dto.id),
                role: Role::Admin,
            };

            let created = build_number_templates_create(
                &ctx,
                &admin_id,
                TemplateTypeDto::DeviceInventory,
                "RBAC65-[XXXX]".to_string(),
            )
            .await
            .expect("Case 65: Admin (Tauri path) → build_number_templates_create → expected Ok");

            let updated = build_number_templates_update_mask(
                &ctx,
                &admin_id,
                created.id,
                "RBAC65-UPD-[XXXX]".to_string(),
                created.version,
            )
            .await;
            assert!(
                updated.is_ok(),
                "Case 65: Admin (Tauri path) → build_number_templates_update_mask → \
                 expected Ok, got {updated:?}"
            );

            let deleted = build_number_templates_delete(&ctx, &admin_id, created.id).await;
            assert!(
                deleted.is_ok(),
                "Case 65: Admin (Tauri path) → build_number_templates_delete → \
                 expected Ok, got {deleted:?}"
            );

            let listed = build_number_templates_list(&ctx, &admin_id, None).await;
            assert!(
                listed.is_ok(),
                "Case 65: Admin (Tauri path) → build_number_templates_list → \
                 expected Ok, got {listed:?}"
            );
        }

        // =====================================================================
        // Case 66 (Phase 40.2 Plan 05, T-40.2-09 — THE Pitfall 3 regression
        // test): Manager session (HTTP) → number_templates_create/
        // _update_mask/_delete/_list → 403 Forbidden for all four. Manager
        // passes every other Mutate* gate in this matrix — this is the ONE
        // family where Manager must be denied (Action::ManageSettings,
        // Admin-only).
        // =====================================================================
        {
            let status = post_with_cookie(
                new_app!(),
                "/api/v1/number_templates_create",
                number_template_create_payload.clone(),
                Some(&manager_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::FORBIDDEN,
                "Case 66: Manager → number_templates_create → expected 403, got {status}"
            );

            let status = post_with_cookie(
                new_app!(),
                "/api/v1/number_templates_update_mask",
                number_template_update_mask_dummy_payload.clone(),
                Some(&manager_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::FORBIDDEN,
                "Case 66: Manager → number_templates_update_mask → expected 403, got {status}"
            );

            let status = post_with_cookie(
                new_app!(),
                "/api/v1/number_templates_delete",
                number_template_delete_dummy_payload.clone(),
                Some(&manager_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::FORBIDDEN,
                "Case 66: Manager → number_templates_delete → expected 403, got {status}"
            );

            let status = post_with_cookie(
                new_app!(),
                "/api/v1/number_templates_list",
                number_template_list_payload.clone(),
                Some(&manager_cookie),
            )
            .await;
            assert_eq!(
                status,
                StatusCode::FORBIDDEN,
                "Case 66: Manager → number_templates_list → expected 403, got {status}"
            );
        }

        // =====================================================================
        // Case 67 (mirrors Case 66 on the Tauri path): Manager Identity →
        // build_number_templates_create/_update_mask/_delete/_list →
        // Err(AppError::Forbidden) for all four.
        // =====================================================================
        {
            let manager_id = Identity {
                user_id: Some(manager_dto.id),
                role: Role::Manager,
            };

            let result = build_number_templates_create(
                &ctx,
                &manager_id,
                TemplateTypeDto::DeviceInventory,
                "RBAC67-[XXXX]".to_string(),
            )
            .await;
            assert!(
                matches!(result, Err(AppError::Forbidden)),
                "Case 67: Manager (Tauri path) → build_number_templates_create → \
                 expected Err(AppError::Forbidden), got {result:?}"
            );

            let result = build_number_templates_update_mask(
                &ctx,
                &manager_id,
                999999,
                "RBAC67-UPD-[XXXX]".to_string(),
                1,
            )
            .await;
            assert!(
                matches!(result, Err(AppError::Forbidden)),
                "Case 67: Manager (Tauri path) → build_number_templates_update_mask → \
                 expected Err(AppError::Forbidden), got {result:?}"
            );

            let result = build_number_templates_delete(&ctx, &manager_id, 999999).await;
            assert!(
                matches!(result, Err(AppError::Forbidden)),
                "Case 67: Manager (Tauri path) → build_number_templates_delete → \
                 expected Err(AppError::Forbidden), got {result:?}"
            );

            let result = build_number_templates_list(&ctx, &manager_id, None).await;
            assert!(
                matches!(result, Err(AppError::Forbidden)),
                "Case 67: Manager (Tauri path) → build_number_templates_list → \
                 expected Err(AppError::Forbidden), got {result:?}"
            );
        }

        // =====================================================================
        // Case 68 (Phase 40.2 Plan 05): Employee session (HTTP) → all 4
        // Group A endpoints AND all 5 Group B endpoints (context=
        // device_create) → 403 Forbidden for all nine.
        // =====================================================================
        {
            let group_a: &[(&str, serde_json::Value)] = &[
                (
                    "/api/v1/number_templates_create",
                    number_template_create_payload.clone(),
                ),
                (
                    "/api/v1/number_templates_update_mask",
                    number_template_update_mask_dummy_payload.clone(),
                ),
                (
                    "/api/v1/number_templates_delete",
                    number_template_delete_dummy_payload.clone(),
                ),
                (
                    "/api/v1/number_templates_list",
                    number_template_list_payload.clone(),
                ),
            ];
            for (uri, payload) in group_a {
                let status =
                    post_with_cookie(new_app!(), uri, payload.clone(), Some(&employee_cookie))
                        .await;
                assert_eq!(
                    status,
                    StatusCode::FORBIDDEN,
                    "Case 68: Employee → {uri} (Group A) → expected 403, got {status}"
                );
            }

            let group_b: &[(&str, serde_json::Value)] = &[
                (
                    "/api/v1/number_templates_list_by_context",
                    number_template_list_by_context_device_payload.clone(),
                ),
                (
                    "/api/v1/number_templates_peek_next",
                    number_template_peek_next_device_payload.clone(),
                ),
                (
                    "/api/v1/number_template_contexts_get",
                    number_template_contexts_get_device_payload.clone(),
                ),
                (
                    "/api/v1/number_template_contexts_set",
                    number_template_contexts_set_device_payload.clone(),
                ),
                (
                    "/api/v1/number_templates_is_occupied",
                    number_template_is_occupied_device_payload.clone(),
                ),
            ];
            for (uri, payload) in group_b {
                let status =
                    post_with_cookie(new_app!(), uri, payload.clone(), Some(&employee_cookie))
                        .await;
                assert_eq!(
                    status,
                    StatusCode::FORBIDDEN,
                    "Case 68: Employee → {uri} (Group B, context=device_create) → \
                     expected 403, got {status}"
                );
            }
        }

        // =====================================================================
        // Case 69 (mirrors Case 68 on the Tauri path): Employee Identity →
        // build_number_templates_* for the same nine → Err(AppError::Forbidden).
        // =====================================================================
        {
            let employee_id = Identity {
                user_id: Some(employee_dto.id),
                role: Role::Employee,
            };

            let result = build_number_templates_create(
                &ctx,
                &employee_id,
                TemplateTypeDto::DeviceInventory,
                "RBAC69-[XXXX]".to_string(),
            )
            .await;
            assert!(
                matches!(result, Err(AppError::Forbidden)),
                "Case 69: Employee (Tauri) → build_number_templates_create → \
                 expected Err(AppError::Forbidden), got {result:?}"
            );

            let result = build_number_templates_update_mask(
                &ctx,
                &employee_id,
                999999,
                "RBAC69-UPD-[XXXX]".to_string(),
                1,
            )
            .await;
            assert!(
                matches!(result, Err(AppError::Forbidden)),
                "Case 69: Employee (Tauri) → build_number_templates_update_mask → \
                 expected Err(AppError::Forbidden), got {result:?}"
            );

            let result = build_number_templates_delete(&ctx, &employee_id, 999999).await;
            assert!(
                matches!(result, Err(AppError::Forbidden)),
                "Case 69: Employee (Tauri) → build_number_templates_delete → \
                 expected Err(AppError::Forbidden), got {result:?}"
            );

            let result = build_number_templates_list(&ctx, &employee_id, None).await;
            assert!(
                matches!(result, Err(AppError::Forbidden)),
                "Case 69: Employee (Tauri) → build_number_templates_list → \
                 expected Err(AppError::Forbidden), got {result:?}"
            );

            let result = build_number_templates_list_by_context(
                &ctx,
                &employee_id,
                TemplateContextDto::DeviceCreate,
            )
            .await;
            assert!(
                matches!(result, Err(AppError::Forbidden)),
                "Case 69: Employee (Tauri) → build_number_templates_list_by_context → \
                 expected Err(AppError::Forbidden), got {result:?}"
            );

            let result = build_number_templates_peek_next(
                &ctx,
                &employee_id,
                999999,
                TemplateContextDto::DeviceCreate,
            )
            .await;
            assert!(
                matches!(result, Err(AppError::Forbidden)),
                "Case 69: Employee (Tauri) → build_number_templates_peek_next → \
                 expected Err(AppError::Forbidden), got {result:?}"
            );

            let result = build_number_template_contexts_get(
                &ctx,
                &employee_id,
                TemplateContextDto::DeviceCreate,
            )
            .await;
            assert!(
                matches!(result, Err(AppError::Forbidden)),
                "Case 69: Employee (Tauri) → build_number_template_contexts_get → \
                 expected Err(AppError::Forbidden), got {result:?}"
            );

            let result = build_number_template_contexts_set(
                &ctx,
                &employee_id,
                TemplateContextDto::DeviceCreate,
                None,
            )
            .await;
            assert!(
                matches!(result, Err(AppError::Forbidden)),
                "Case 69: Employee (Tauri) → build_number_template_contexts_set → \
                 expected Err(AppError::Forbidden), got {result:?}"
            );

            let result = build_number_templates_is_occupied(
                &ctx,
                &employee_id,
                TemplateContextDto::DeviceCreate,
                "RBAC-69-CANDIDATE".to_string(),
                None,
            )
            .await;
            assert!(
                matches!(result, Err(AppError::Forbidden)),
                "Case 69: Employee (Tauri) → build_number_templates_is_occupied → \
                 expected Err(AppError::Forbidden), got {result:?}"
            );
        }

        // =====================================================================
        // Case 70 (Phase 40.2 Plan 05): Manager session (HTTP) →
        // number_templates_list_by_context with context=device_create /
        // act_create / cartridge_create → not 401/403 for all three — proves
        // each `action_for_context` branch independently grants Manager
        // access to their own popup's template menu.
        // =====================================================================
        {
            for (label, payload) in [
                (
                    "device_create",
                    number_template_list_by_context_device_payload.clone(),
                ),
                (
                    "act_create",
                    number_template_list_by_context_act_payload.clone(),
                ),
                (
                    "cartridge_create",
                    number_template_list_by_context_cartridge_payload.clone(),
                ),
            ] {
                let status = post_with_cookie(
                    new_app!(),
                    "/api/v1/number_templates_list_by_context",
                    payload,
                    Some(&manager_cookie),
                )
                .await;
                assert!(
                    status != StatusCode::UNAUTHORIZED && status != StatusCode::FORBIDDEN,
                    "Case 70: Manager → number_templates_list_by_context (context={label}) \
                     → expected not 401/403, got {status}"
                );
            }
        }

        // =====================================================================
        // Case 71 (mirrors Case 70 on the Tauri path): Manager Identity →
        // build_number_templates_list_by_context for the same three
        // contexts → Ok.
        // =====================================================================
        {
            let manager_id = Identity {
                user_id: Some(manager_dto.id),
                role: Role::Manager,
            };

            for (label, context) in [
                ("device_create", TemplateContextDto::DeviceCreate),
                ("act_create", TemplateContextDto::ActCreate),
                ("cartridge_create", TemplateContextDto::CartridgeCreate),
            ] {
                let result =
                    build_number_templates_list_by_context(&ctx, &manager_id, context).await;
                assert!(
                    result.is_ok(),
                    "Case 71: Manager (Tauri) → build_number_templates_list_by_context \
                     (context={label}) → expected Ok, got {result:?}"
                );
            }
        }

        // =====================================================================
        // Case 72 (Phase 40.2 Plan 05): Admin session (HTTP) → same three
        // contexts → not 401/403 (Admin has every Mutate* right too).
        // =====================================================================
        {
            for (label, payload) in [
                (
                    "device_create",
                    number_template_list_by_context_device_payload.clone(),
                ),
                (
                    "act_create",
                    number_template_list_by_context_act_payload.clone(),
                ),
                (
                    "cartridge_create",
                    number_template_list_by_context_cartridge_payload.clone(),
                ),
            ] {
                let status = post_with_cookie(
                    new_app!(),
                    "/api/v1/number_templates_list_by_context",
                    payload,
                    Some(&admin_cookie),
                )
                .await;
                assert!(
                    status != StatusCode::UNAUTHORIZED && status != StatusCode::FORBIDDEN,
                    "Case 72: Admin → number_templates_list_by_context (context={label}) \
                     → expected not 401/403, got {status}"
                );
            }
        }

        // =====================================================================
        // Case 73 (mirrors Case 72 on the Tauri path): Admin Identity →
        // build_number_templates_list_by_context for the same three
        // contexts → Ok.
        // =====================================================================
        {
            let admin_id = Identity {
                user_id: Some(admin_dto.id),
                role: Role::Admin,
            };

            for (label, context) in [
                ("device_create", TemplateContextDto::DeviceCreate),
                ("act_create", TemplateContextDto::ActCreate),
                ("cartridge_create", TemplateContextDto::CartridgeCreate),
            ] {
                let result =
                    build_number_templates_list_by_context(&ctx, &admin_id, context).await;
                assert!(
                    result.is_ok(),
                    "Case 73: Admin (Tauri) → build_number_templates_list_by_context \
                     (context={label}) → expected Ok, got {result:?}"
                );
            }
        }

        ctx.shutdown.cancel();
    })
    .await
    .expect("role_endpoint_matrix_test exceeded 60s budget");
}

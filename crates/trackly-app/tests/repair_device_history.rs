//! Phase 41.4, план 30: история ремонтов устройства (только чтение, только персонал).
//! Данные вымышленные: «Иванов И.И.», «Петров П.П.».

mod entities_support;
mod repair_support;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use entities_support::fixture::{admin, make_test_ctx, seed_device, seed_place};
use repair_support::{emp, seed_user};
use rusqlite::params;
use tower::ServiceExt;
use trackly_app::context::AppCtx;
use trackly_app::http::build_router;
use trackly_app::server::rusqlite_session_store::RusqliteSessionStore;
use trackly_app::tauri_cmds::repairs::build_repairs_device_history;
use trackly_core::error::AppError;
use trackly_infra::error_conversions::map_rusqlite;

/// Заявка-ремонт (закрытая, чтобы не упереться в «одна открытая на устройство»).
async fn seed_request(
    ctx: &AppCtx,
    user: i64,
    device: i64,
    created: i64,
    deleted: Option<i64>,
) -> i64 {
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO requests (request_type, status, requested_by_user_id, \
                 printer_device_id, description, created_at_utc, updated_at_utc, deleted_at_utc) \
                 VALUES ('repair', 'completed', ?1, ?2, 'Не включается', ?3, ?3, ?4)",
                params![user, device, created, deleted],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed request")
}

/// Закрытый заход; `seq` по заявке.
async fn seed_visit(
    ctx: &AppCtx,
    request: i64,
    device: i64,
    seq: i64,
    from_place: i64,
    service_place: i64,
    cost: Option<i64>,
) -> i64 {
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO repair_visits (request_id, device_id, seq, service_place_id, \
                 from_place_id, sent_at_utc, returned_at_utc, returned_to_place_id, giver_name, \
                 receiver_name, fault_description, works_done, cost_kopecks, act_number, \
                 created_at_utc, updated_at_utc, version) \
                 VALUES (?1, ?2, ?3, ?4, ?5, 1781654400, 1781740800, ?5, 'Иванов И.И.', \
                         'Петров П.П.', 'Не включается', 'Замена блока питания', ?6, ?7, \
                         1781654400, 1781654400, 1)",
                params![
                    request,
                    device,
                    seq,
                    service_place,
                    from_place,
                    cost,
                    format!("РЕМ-{request:03}-{seq}")
                ],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed visit")
}

struct Fx {
    ctx: AppCtx,
    _dir: tempfile::TempDir,
    user: i64,
    office: i64,
    service: i64,
    device: i64,
}

async fn fx() -> Fx {
    let (ctx, dir) = make_test_ctx().await;
    let user = seed_user(&ctx, "ivanov", "Иванов И.И.").await;
    let office = seed_place(&ctx, "Кабинет 12").await;
    let service = seed_place(&ctx, "Сервис-центр №1").await;
    let device = seed_device(&ctx, 1, "ПК-00014", "INV-14", Some(office), None).await;
    Fx {
        ctx,
        _dir: dir,
        user,
        office,
        service,
        device,
    }
}

#[tokio::test]
async fn no_visits_gives_empty_history() {
    let f = fx().await;
    // Заявка без заходов в историю не входит.
    seed_request(&f.ctx, f.user, f.device, 1_700_000_000, None).await;
    let h = f
        .ctx
        .repairs
        .device_history(f.device, &admin())
        .await
        .unwrap();
    assert_eq!(h.visit_count, 0);
    assert!(h.requests.is_empty());
    assert_eq!(h.total_cost_kopecks, None);
    assert_eq!(h.total_cost_display, None);
    assert_eq!(h.device_name, "ПК-00014");
    assert_eq!(h.inventory_no.as_deref(), Some("INV-14"));
}

#[tokio::test]
async fn requests_newest_first_visits_by_seq() {
    let f = fx().await;
    let old = seed_request(&f.ctx, f.user, f.device, 1_700_000_000, None).await;
    let new = seed_request(&f.ctx, f.user, f.device, 1_700_100_000, None).await;
    seed_visit(&f.ctx, old, f.device, 1, f.office, f.service, None).await;
    seed_visit(&f.ctx, old, f.device, 2, f.office, f.service, None).await;
    seed_visit(&f.ctx, new, f.device, 1, f.office, f.service, None).await;
    let h = f
        .ctx
        .repairs
        .device_history(f.device, &admin())
        .await
        .unwrap();
    assert_eq!(h.visit_count, 3);
    let ids: Vec<i64> = h.requests.iter().map(|r| r.request_id).collect();
    assert_eq!(ids, vec![new, old], "новые первыми");
    let seqs: Vec<i64> = h.requests[1].visits.iter().map(|v| v.seq).collect();
    assert_eq!(seqs, vec![1, 2]);
    assert_eq!(h.requests[0].requester_name.as_deref(), Some("Иванов И.И."));
}

#[tokio::test]
async fn cost_sums_only_visits_with_cost() {
    let f = fx().await;
    let r = seed_request(&f.ctx, f.user, f.device, 1_700_000_000, None).await;
    seed_visit(&f.ctx, r, f.device, 1, f.office, f.service, Some(150_050)).await;
    seed_visit(&f.ctx, r, f.device, 2, f.office, f.service, None).await;
    seed_visit(&f.ctx, r, f.device, 3, f.office, f.service, Some(30_000)).await;
    let h = f
        .ctx
        .repairs
        .device_history(f.device, &admin())
        .await
        .unwrap();
    assert_eq!(h.total_cost_kopecks, Some(180_050));
    assert_eq!(
        h.total_cost_display.as_deref(),
        Some(trackly_core::domain::repairs::format_kopecks(180_050).as_str())
    );

    // Все заходы без стоимости: итог отсутствует, а не «0».
    let d2 = seed_device(&f.ctx, 1, "ПК-00015", "INV-15", Some(f.office), None).await;
    let r2 = seed_request(&f.ctx, f.user, d2, 1_700_000_000, None).await;
    seed_visit(&f.ctx, r2, d2, 1, f.office, f.service, None).await;
    let h2 = f.ctx.repairs.device_history(d2, &admin()).await.unwrap();
    assert_eq!(h2.visit_count, 1);
    assert_eq!(h2.total_cost_kopecks, None);
    assert_eq!(h2.total_cost_display, None);
}

#[tokio::test]
async fn other_device_empty_and_deleted_requests_excluded() {
    let f = fx().await;
    let other = seed_device(&f.ctx, 1, "ПК-00016", "INV-16", Some(f.office), None).await;
    let mine = seed_request(&f.ctx, f.user, f.device, 1_700_000_000, None).await;
    let deleted = seed_request(&f.ctx, f.user, f.device, 1_700_000_500, Some(1_700_001_000)).await;
    let foreign = seed_request(&f.ctx, f.user, other, 1_700_000_600, None).await;
    let empty = seed_request(&f.ctx, f.user, f.device, 1_700_000_700, None).await;
    let _ = empty;
    seed_visit(&f.ctx, mine, f.device, 1, f.office, f.service, None).await;
    seed_visit(
        &f.ctx,
        deleted,
        f.device,
        1,
        f.office,
        f.service,
        Some(99_900),
    )
    .await;
    seed_visit(&f.ctx, foreign, other, 1, f.office, f.service, Some(77_700)).await;
    let h = f
        .ctx
        .repairs
        .device_history(f.device, &admin())
        .await
        .unwrap();
    let ids: Vec<i64> = h.requests.iter().map(|r| r.request_id).collect();
    assert_eq!(ids, vec![mine]);
    assert_eq!(h.visit_count, 1);
    assert_eq!(h.total_cost_kopecks, None);
}

#[tokio::test]
async fn visit_paths_and_scan_count_like_get_state() {
    let f = fx().await;
    let r = seed_request(&f.ctx, f.user, f.device, 1_700_000_000, None).await;
    let v = seed_visit(&f.ctx, r, f.device, 1, f.office, f.service, None).await;
    // Живой скан и удалённый скан.
    f.ctx
        .writer
        .execute(move |conn| {
            for (i, deleted) in [(1_i64, None), (2_i64, Some(1_700_000_001_i64))] {
                conn.execute(
                    "INSERT INTO attachments (parent_kind, parent_id, original_name, mime, \
                     size_bytes, content_hash, created_at_utc, deleted_at_utc) \
                     VALUES ('request', ?1, ?2, 'image/png', 1, ?2, 1700000000, ?3)",
                    params![r, format!("scan{i}"), deleted],
                )
                .map_err(map_rusqlite)?;
                let att = conn.last_insert_rowid();
                conn.execute(
                    "INSERT INTO repair_visit_scans (visit_id, attachment_id, linked_at_utc) \
                     VALUES (?1, ?2, 1700000000)",
                    params![v, att],
                )
                .map_err(map_rusqlite)?;
            }
            Ok(())
        })
        .await
        .expect("scans");
    let h = f
        .ctx
        .repairs
        .device_history(f.device, &admin())
        .await
        .unwrap();
    let hv = &h.requests[0].visits[0];
    assert_eq!(hv.service_place_path.as_deref(), Some("Сервис-центр №1"));
    assert_eq!(hv.from_place_path.as_deref(), Some("Кабинет 12"));
    assert_eq!(hv.returned_to_place_path.as_deref(), Some("Кабинет 12"));
    assert_eq!(hv.scan_count, 1);
    let state = f.ctx.repairs.get_state(r, &admin()).await.unwrap();
    assert_eq!(&state.visits[0], hv, "один код сборки с get_state");
}

#[tokio::test]
async fn employee_forbidden_and_missing_device_not_found() {
    let f = fx().await;
    let err = f
        .ctx
        .repairs
        .device_history(f.device, &emp(f.user))
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::Forbidden), "{err:?}");
    let err = f
        .ctx
        .repairs
        .device_history(999_999, &admin())
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::NotFound { .. }), "{err:?}");
}

/// Сотрудник получает отказ и через общий build_* (Tauri и HTTP идут через него).
#[tokio::test]
async fn build_rejects_employee_allows_staff() {
    let f = fx().await;
    let err = build_repairs_device_history(&f.ctx, &emp(f.user), f.device)
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::Forbidden), "{err:?}");
    let h = build_repairs_device_history(&f.ctx, &admin(), f.device)
        .await
        .unwrap();
    assert_eq!(h.device_id, f.device);
}

/// Маршрут зарегистрирован в настоящем `build_router`: без сессии 401 JSON, а не
/// 404 `api_route` или index.html от `spa_fallback`.
#[tokio::test]
async fn http_route_is_registered_not_spa_fallback() {
    let (ctx, _dir) = make_test_ctx().await;
    let store = RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone());
    let app = build_router(&ctx, store);
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/repairs_device_history")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"deviceId":1}"#))
                .expect("req"),
        )
        .await
        .expect("resp");
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    let ct = res
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    assert!(ct.contains("json"), "content-type: {ct}");
}

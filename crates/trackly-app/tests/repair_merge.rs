//! Phase 41.4, план 28: объединение заявок (D-33) — сервер.
//!
//! Текущая заявка «Другое» (в работе, без устройства) сливается в уже открытую
//! заявку занятого устройства. Данные вымышленные: «Иванов И.И.», «Петров П.П.».

mod entities_support;
mod repair_support;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use entities_support::fixture::{admin, make_test_ctx, seed_device, seed_place};
use repair_support::{emp, manager, seed_user};
use rusqlite::params;
use tower::ServiceExt;
use trackly_app::context::AppCtx;
use trackly_app::dto::printer::WsEvent;
use trackly_app::dto::repair::{RepairCreateDto, RepairMergeDto, RepairMoveChoiceDto};
use trackly_app::http::build_router;
use trackly_app::server::rusqlite_session_store::RusqliteSessionStore;
use trackly_app::tauri_cmds::repairs::build_repairs_merge_into_open;
use trackly_core::error::AppError;
use trackly_infra::error_conversions::map_rusqlite;

async fn scalar_i64(ctx: &AppCtx, sql: &'static str, a: i64) -> i64 {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(sql, params![a], |r| r.get::<_, i64>(0))
                .map_err(map_rusqlite)
        })
        .await
        .expect("scalar")
}

async fn scalar_str(ctx: &AppCtx, sql: &'static str, a: i64) -> String {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(sql, params![a], |r| r.get::<_, String>(0))
                .map_err(map_rusqlite)
        })
        .await
        .expect("scalar")
}

async fn audit_payloads(ctx: &AppCtx, request: i64, action: &'static str) -> Vec<String> {
    ctx.writer
        .execute(move |conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT COALESCE(payload_json, '') FROM audit_log \
                     WHERE entity_type = 'request' AND entity_id = ?1 AND action = ?2 \
                     ORDER BY id",
                )
                .map_err(map_rusqlite)?;
            let rows = stmt
                .query_map(params![request, action], |r| r.get::<_, String>(0))
                .map_err(map_rusqlite)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(map_rusqlite)
        })
        .await
        .expect("audit")
}

struct Fx {
    ctx: AppCtx,
    _dir: tempfile::TempDir,
    ivanov: i64,
    petrov: i64,
    device: i64,
    /// Целевая заявка B (открыта, автор Петров, в работе, на устройстве).
    target: i64,
    /// Поглощаемая A (автор Иванов, в работе, без устройства) и её версия.
    source: i64,
    source_version: i64,
}

async fn fixture() -> Fx {
    let (ctx, dir) = make_test_ctx().await;
    let ivanov = seed_user(&ctx, "ivanov", "Иванов И.И.").await;
    let petrov = seed_user(&ctx, "petrov", "Петров П.П.").await;
    let room = seed_place(&ctx, "Кабинет 12").await;
    let device = seed_device(&ctx, 1, "ПК-1", "INV-1", Some(room), None).await;

    let b = ctx
        .repairs
        .create(
            RepairCreateDto {
                device_id: Some(device),
                description: "Не включается".into(),
            },
            &manager(petrov),
        )
        .await
        .expect("create B");
    let target = b.request.id;
    ctx.repairs
        .accept(
            target,
            b.request.version,
            RepairMoveChoiceDto::Stay,
            &admin(),
        )
        .await
        .expect("accept B");

    let a = ctx
        .repairs
        .create(
            RepairCreateDto {
                device_id: None,
                description: "Шумит и греется".into(),
            },
            &manager(ivanov),
        )
        .await
        .expect("create A");
    let acc = ctx
        .repairs
        .accept(
            a.request.id,
            a.request.version,
            RepairMoveChoiceDto::Stay,
            &admin(),
        )
        .await
        .expect("accept A");
    Fx {
        ctx,
        _dir: dir,
        ivanov,
        petrov,
        device,
        target,
        source: a.request.id,
        source_version: acc.request.version,
    }
}

fn dto(fx: &Fx) -> RepairMergeDto {
    RepairMergeDto {
        request_id: fx.source,
        version: fx.source_version,
        device_id: fx.device,
    }
}

fn is_validation(e: &AppError) -> bool {
    matches!(e, AppError::Validation { .. })
}

async fn req_state(fx: &Fx, id: i64) -> (String, i64, Option<i64>) {
    fx.ctx
        .writer
        .execute(move |conn| {
            conn.query_row(
                "SELECT status, version, printer_device_id FROM requests WHERE id = ?1",
                params![id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("state")
}

#[tokio::test]
async fn merge_moves_author_and_leaves_trace_in_both() {
    let fx = fixture().await;
    let before = req_state(&fx, fx.target).await;
    let res = fx
        .ctx
        .repairs
        .merge_into_open(dto(&fx), &manager(fx.petrov))
        .await
        .expect("merge");
    assert_eq!(res.target_request_id, fx.target);
    assert_eq!(res.request.id, fx.source);
    assert_eq!(res.request.status, "cancelled");

    // Цель не изменилась (статус, версия, устройство).
    assert_eq!(req_state(&fx, fx.target).await, before);
    assert_eq!(req_state(&fx, fx.source).await.0, "cancelled");

    // Автор поглощённой заявки — участник целевой.
    let is_part = fx
        .ctx
        .writer
        .execute({
            let (t, u) = (fx.target, fx.ivanov);
            move |conn| {
                conn.query_row(
                    "SELECT COUNT(*) FROM request_participants WHERE request_id = ?1 AND user_id = ?2",
                    params![t, u],
                    |r| r.get::<_, i64>(0),
                )
                .map_err(map_rusqlite)
            }
        })
        .await
        .expect("part");
    assert_eq!(is_part, 1);

    // История цели: repair_merge с описанием и именем автора.
    let on_target = audit_payloads(&fx.ctx, fx.target, "repair_merge").await;
    assert_eq!(on_target.len(), 1);
    assert!(on_target[0].contains("Шумит и греется"), "{}", on_target[0]);
    assert!(
        on_target[0].contains("О поломке также сообщил(а) Иванов И.И."),
        "{}",
        on_target[0]
    );
    // История поглощённой: номер цели.
    let on_source = audit_payloads(&fx.ctx, fx.source, "custom:repair_merged_into").await;
    assert_eq!(on_source.len(), 1);
    assert!(
        on_source[0].contains(&format!("№{}", fx.target)),
        "{}",
        on_source[0]
    );
}

#[tokio::test]
async fn merge_keeps_single_open_request_and_device_untouched() {
    let fx = fixture().await;
    let status_before = scalar_str(
        &fx.ctx,
        "SELECT ds.code FROM devices d JOIN device_statuses ds ON ds.id = d.status_id WHERE d.id = ?1",
        fx.device,
    )
    .await;
    let place_before = scalar_i64(
        &fx.ctx,
        "SELECT COALESCE(place_id, 0) FROM devices WHERE id = ?1",
        fx.device,
    )
    .await;
    fx.ctx
        .repairs
        .merge_into_open(dto(&fx), &admin())
        .await
        .expect("merge");
    assert_eq!(
        scalar_i64(
            &fx.ctx,
            "SELECT COUNT(*) FROM requests WHERE request_type = 'repair' \
             AND printer_device_id = ?1 AND status IN ('open','in_progress')",
            fx.device
        )
        .await,
        1
    );
    assert_eq!(
        scalar_str(
            &fx.ctx,
            "SELECT ds.code FROM devices d JOIN device_statuses ds ON ds.id = d.status_id WHERE d.id = ?1",
            fx.device
        )
        .await,
        status_before
    );
    assert_eq!(
        scalar_i64(
            &fx.ctx,
            "SELECT COALESCE(place_id, 0) FROM devices WHERE id = ?1",
            fx.device
        )
        .await,
        place_before
    );
}

#[tokio::test]
async fn merge_without_target_is_validation_and_changes_nothing() {
    let fx = fixture().await;
    let room = seed_place(&fx.ctx, "Кабинет 13").await;
    let free = seed_device(&fx.ctx, 1, "ПК-2", "INV-2", Some(room), None).await;
    let err = fx
        .ctx
        .repairs
        .merge_into_open(
            RepairMergeDto {
                device_id: free,
                ..dto(&fx)
            },
            &admin(),
        )
        .await
        .expect_err("нет цели");
    assert!(is_validation(&err), "{err:?}");
    let s = req_state(&fx, fx.source).await;
    assert_eq!(s.0, "in_progress");
    assert_eq!(s.1, fx.source_version);
}

#[tokio::test]
async fn merge_rejects_wrong_source() {
    let fx = fixture().await;
    // Поглощаемая заявка уже с устройством: заявка на ДРУГОМ устройстве в работе.
    let room = seed_place(&fx.ctx, "Кабинет 14").await;
    let other = seed_device(&fx.ctx, 1, "ПК-3", "INV-3", Some(room), None).await;
    let d = fx
        .ctx
        .repairs
        .create(
            RepairCreateDto {
                device_id: Some(other),
                description: "Не загружается".into(),
            },
            &manager(fx.ivanov),
        )
        .await
        .expect("create D");
    let acc = fx
        .ctx
        .repairs
        .accept(
            d.request.id,
            d.request.version,
            RepairMoveChoiceDto::Stay,
            &admin(),
        )
        .await
        .expect("accept D");
    let err = fx
        .ctx
        .repairs
        .merge_into_open(
            RepairMergeDto {
                request_id: d.request.id,
                version: acc.request.version,
                device_id: fx.device,
            },
            &admin(),
        )
        .await
        .expect_err("уже с устройством");
    assert!(is_validation(&err), "{err:?}");
    assert_eq!(req_state(&fx, d.request.id).await.0, "in_progress");

    // Не в работе: свежая заявка «открыта».
    let c = fx
        .ctx
        .repairs
        .create(
            RepairCreateDto {
                device_id: None,
                description: "Скрипит".into(),
            },
            &manager(fx.ivanov),
        )
        .await
        .expect("create C");
    let err = fx
        .ctx
        .repairs
        .merge_into_open(
            RepairMergeDto {
                request_id: c.request.id,
                version: c.request.version,
                device_id: fx.device,
            },
            &admin(),
        )
        .await
        .expect_err("не в работе");
    assert!(is_validation(&err), "{err:?}");
    assert_eq!(req_state(&fx, c.request.id).await.0, "open");

    // Версия не совпадает.
    let err = fx
        .ctx
        .repairs
        .merge_into_open(
            RepairMergeDto {
                version: fx.source_version + 7,
                ..dto(&fx)
            },
            &admin(),
        )
        .await
        .expect_err("версия");
    assert!(
        matches!(err, AppError::OptimisticLockMismatch { .. }),
        "{err:?}"
    );
    assert_eq!(req_state(&fx, fx.source).await.0, "in_progress");
}

#[tokio::test]
async fn merge_twice_is_validation_and_no_duplicate_participant() {
    let fx = fixture().await;
    fx.ctx
        .repairs
        .merge_into_open(dto(&fx), &admin())
        .await
        .expect("merge");
    let err = fx
        .ctx
        .repairs
        .merge_into_open(dto(&fx), &admin())
        .await
        .expect_err("повтор");
    assert!(is_validation(&err), "{err:?}");
    assert_eq!(
        audit_payloads(&fx.ctx, fx.target, "repair_merge")
            .await
            .len(),
        1
    );
    assert_eq!(
        scalar_i64(
            &fx.ctx,
            "SELECT COUNT(*) FROM request_participants WHERE request_id = ?1",
            fx.target
        )
        .await,
        scalar_i64(
            &fx.ctx,
            "SELECT COUNT(DISTINCT user_id) FROM request_participants WHERE request_id = ?1",
            fx.target
        )
        .await
    );
}

#[tokio::test]
async fn merge_when_author_already_participant_has_no_duplicate() {
    let fx = fixture().await;
    let (t, u) = (fx.target, fx.ivanov);
    fx.ctx
        .writer
        .execute(move |conn| {
            conn.execute(
                "INSERT OR IGNORE INTO request_participants (request_id, user_id, joined_at_utc) \
                 VALUES (?1, ?2, 1700000000)",
                params![t, u],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("seed participant");
    fx.ctx
        .repairs
        .merge_into_open(dto(&fx), &admin())
        .await
        .expect("merge");
    let n = fx
        .ctx
        .writer
        .execute(move |conn| {
            conn.query_row(
                "SELECT COUNT(*) FROM request_participants WHERE request_id = ?1 AND user_id = ?2",
                params![t, u],
                |r| r.get::<_, i64>(0),
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("n");
    assert_eq!(n, 1);
}

#[tokio::test]
async fn merge_signals_status_changed_for_both_requests() {
    let fx = fixture().await;
    let mut rx = fx.ctx.ws_broadcast.subscribe();
    fx.ctx
        .repairs
        .merge_into_open(dto(&fx), &admin())
        .await
        .expect("merge");
    let mut seen = Vec::new();
    while let Ok(ev) = rx.try_recv() {
        if let WsEvent::RequestStatusChanged {
            request_id,
            participant_user_ids,
            ..
        } = ev
        {
            seen.push((request_id, participant_user_ids));
        }
    }
    assert!(seen.iter().any(|(id, _)| *id == fx.source), "{seen:?}");
    let target_ev = seen
        .iter()
        .find(|(id, _)| *id == fx.target)
        .unwrap_or_else(|| panic!("нет события по цели: {seen:?}"));
    assert!(target_ev.1.contains(&fx.ivanov), "{seen:?}");
}

#[tokio::test]
async fn employee_is_forbidden_on_tauri_builder() {
    let fx = fixture().await;
    let res = build_repairs_merge_into_open(&fx.ctx, &emp(fx.ivanov), dto(&fx)).await;
    assert!(matches!(res, Err(AppError::Forbidden)), "{res:?}");
    assert_eq!(req_state(&fx, fx.source).await.0, "in_progress");
}

/// Маршрут зарегистрирован в настоящем `build_router`: без сессии ответ 401 JSON,
/// а не 404 `api_route` от `spa_fallback`.
#[tokio::test]
async fn http_route_is_registered_not_spa_fallback() {
    let (ctx, _dir) = make_test_ctx().await;
    let store = RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone());
    let app = build_router(&ctx, store);
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/repairs_merge_into_open")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"dto":{"requestId":1,"version":1,"deviceId":1}}"#,
                ))
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

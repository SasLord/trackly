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

    // D-35: описание цели меняется, версия +1; статус и устройство прежние.
    let after = req_state(&fx, fx.target).await;
    assert_eq!(after.0, before.0);
    assert_eq!(after.1, before.1 + 1);
    assert_eq!(after.2, before.2);
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

// ---------- D-35: описание и вложения ----------

/// id, имя, mime, хеш, размер, загрузивший, created_at, deleted_at.
type AttRow = (i64, String, String, String, i64, i64, i64, Option<i64>);

const HASH_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const HASH_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

async fn seed_attachment(
    ctx: &AppCtx,
    request_id: i64,
    name: &'static str,
    hash: &'static str,
    uploader: i64,
    deleted: bool,
) -> i64 {
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO attachments (parent_kind, parent_id, original_name, mime, \
                 size_bytes, content_hash, uploaded_by_user_id, created_at_utc, deleted_at_utc) \
                 VALUES ('request', ?1, ?2, 'application/pdf', 10, ?3, ?4, 1700000000, ?5)",
                params![
                    request_id,
                    name,
                    hash,
                    uploader,
                    if deleted {
                        Some(1_700_000_001_i64)
                    } else {
                        None
                    }
                ],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("attachment")
}

async fn set_target_description(ctx: &AppCtx, id: i64, text: String) {
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "UPDATE requests SET description = ?2 WHERE id = ?1",
                params![id, text],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("set description");
}

async fn description_of(ctx: &AppCtx, id: i64) -> String {
    scalar_str(
        ctx,
        "SELECT COALESCE(description, '') FROM requests WHERE id = ?1",
        id,
    )
    .await
}

#[tokio::test]
async fn merge_appends_source_description_block_to_target() {
    let fx = fixture().await;
    let before = req_state(&fx, fx.target).await;
    fx.ctx
        .repairs
        .merge_into_open(dto(&fx), &admin())
        .await
        .expect("merge");
    assert_eq!(
        description_of(&fx.ctx, fx.target).await,
        "Не включается\n\nИванов И.И.:\nШумит и греется"
    );
    let after = req_state(&fx, fx.target).await;
    assert_eq!(after.0, before.0);
    assert_eq!(after.1, before.1 + 1);
    assert_eq!(after.2, before.2);
}

#[tokio::test]
async fn merge_appends_without_leading_blank_lines_when_target_empty() {
    let fx = fixture().await;
    set_target_description(&fx.ctx, fx.target, String::new()).await;
    fx.ctx
        .repairs
        .merge_into_open(dto(&fx), &admin())
        .await
        .expect("merge");
    assert_eq!(
        description_of(&fx.ctx, fx.target).await,
        "Иванов И.И.:\nШумит и греется"
    );
}

#[tokio::test]
async fn merge_description_capped_at_limit() {
    let fx = fixture().await;
    let long = "я".repeat(3990);
    set_target_description(&fx.ctx, fx.target, long.clone()).await;
    fx.ctx
        .repairs
        .merge_into_open(dto(&fx), &admin())
        .await
        .expect("merge");
    let d = description_of(&fx.ctx, fx.target).await;
    assert_eq!(d.chars().count(), 4000);
    assert!(d.starts_with(&long), "прежнее описание цели потеряно");
}

#[tokio::test]
async fn merge_copies_live_attachments_to_target() {
    use trackly_app::services::attachment_service::ParentRef;
    use trackly_core::domain::attachments::ParentKind;

    let fx = fixture().await;
    let live = seed_attachment(&fx.ctx, fx.source, "скан.pdf", HASH_A, fx.ivanov, false).await;
    let dead = seed_attachment(&fx.ctx, fx.source, "старый.pdf", HASH_B, fx.ivanov, true).await;
    fx.ctx
        .repairs
        .merge_into_open(dto(&fx), &admin())
        .await
        .expect("merge");

    let (t, s) = (fx.target, fx.source);
    // Ровно одна строка у цели (удалённая не копируется), поля те же.
    let rows: Vec<AttRow> = fx
        .ctx
        .writer
        .execute(move |conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT id, original_name, mime, content_hash, size_bytes, uploaded_by_user_id, \
                     created_at_utc, deleted_at_utc FROM attachments \
                     WHERE parent_kind = 'request' AND parent_id = ?1 ORDER BY id",
                )
                .map_err(map_rusqlite)?;
            let it = stmt
                .query_map(params![t], |r| {
                    Ok((
                        r.get(0)?,
                        r.get(1)?,
                        r.get(2)?,
                        r.get(3)?,
                        r.get(4)?,
                        r.get(5)?,
                        r.get(6)?,
                        r.get(7)?,
                    ))
                })
                .map_err(map_rusqlite)?;
            it.collect::<Result<Vec<_>, _>>().map_err(map_rusqlite)
        })
        .await
        .expect("rows");
    assert_eq!(rows.len(), 1, "{rows:?}");
    let new = &rows[0];
    assert_eq!(new.1, "скан.pdf");
    assert_eq!(new.2, "application/pdf");
    assert_eq!(new.3, HASH_A);
    assert_eq!(new.4, 10);
    assert_eq!(new.5, fx.ivanov);
    assert_eq!(new.6, 1_700_000_000);
    assert_eq!(new.7, None);

    // Строки источника не изменились.
    assert_eq!(
        scalar_i64(
            &fx.ctx,
            "SELECT COUNT(*) FROM attachments WHERE parent_kind = 'request' AND parent_id = ?1",
            s
        )
        .await,
        2
    );
    assert_eq!(
        scalar_i64(
            &fx.ctx,
            "SELECT COUNT(*) FROM attachments WHERE parent_kind = 'request' AND parent_id = ?1 \
             AND deleted_at_utc IS NOT NULL",
            s
        )
        .await,
        1
    );
    // Строк с этим хешем стало две (purge не снесёт блоб при удалении одной копии).
    let n_hash = fx
        .ctx
        .writer
        .execute(|conn| {
            conn.query_row(
                "SELECT COUNT(*) FROM attachments WHERE content_hash = ?1",
                params![HASH_A],
                |r| r.get::<_, i64>(0),
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("hash");
    assert_eq!(n_hash, 2);
    let _ = (live, dead);

    // История цели: одна запись custom:attachment_added.
    let added = audit_payloads(&fx.ctx, fx.target, "custom:attachment_added").await;
    assert_eq!(added.len(), 1, "{added:?}");
    let v: serde_json::Value = serde_json::from_str(&added[0]).expect("json");
    assert_eq!(v["attachment_id"].as_i64(), Some(new.0));
    assert_eq!(v["name"].as_str(), Some("скан.pdf"));
    assert_eq!(v["merged_from_request_id"].as_i64(), Some(fx.source));

    // Автор поглощённой (участник цели) видит вложение.
    let list = fx
        .ctx
        .attachments
        .list(
            ParentRef {
                kind: ParentKind::Request,
                id: fx.target,
            },
            &emp(fx.ivanov),
        )
        .await
        .expect("list");
    assert_eq!(list.items.len(), 1);
    assert_eq!(list.items[0].id, new.0);
}

#[tokio::test]
async fn merge_signals_attachments_changed_for_target() {
    let fx = fixture().await;
    seed_attachment(&fx.ctx, fx.source, "скан.pdf", HASH_A, fx.ivanov, false).await;
    let mut rx = fx.ctx.ws_broadcast.subscribe();
    fx.ctx
        .repairs
        .merge_into_open(dto(&fx), &admin())
        .await
        .expect("merge");
    let mut seen = Vec::new();
    while let Ok(ev) = rx.try_recv() {
        if let WsEvent::AttachmentsChanged {
            parent_kind,
            parent_id,
            participant_user_ids,
            ..
        } = ev
        {
            seen.push((parent_kind, parent_id, participant_user_ids));
        }
    }
    assert_eq!(seen.len(), 1, "{seen:?}");
    assert_eq!(seen[0].0, "request");
    assert_eq!(seen[0].1, fx.target);
    assert!(seen[0].2.contains(&fx.ivanov), "{seen:?}");

    // Без вложений событие не уходит.
    let fx2 = fixture().await;
    let mut rx2 = fx2.ctx.ws_broadcast.subscribe();
    fx2.ctx
        .repairs
        .merge_into_open(dto(&fx2), &admin())
        .await
        .expect("merge");
    while let Ok(ev) = rx2.try_recv() {
        assert!(
            !matches!(ev, WsEvent::AttachmentsChanged { .. }),
            "лишнее событие: {ev:?}"
        );
    }
}

#[tokio::test]
async fn history_exposes_related_request_id_for_merge_events() {
    let fx = fixture().await;
    fx.ctx
        .repairs
        .merge_into_open(dto(&fx), &admin())
        .await
        .expect("merge");
    let on_source = fx
        .ctx
        .requests
        .get_history(fx.source, &admin())
        .await
        .expect("history source");
    let merged = on_source
        .iter()
        .find(|e| e.action == "custom:repair_merged_into")
        .expect("запись об объединении");
    assert_eq!(merged.related_request_id, Some(fx.target));
    assert!(
        on_source
            .iter()
            .filter(|e| e.action != "custom:repair_merged_into")
            .all(|e| e.related_request_id.is_none()),
        "{on_source:?}"
    );

    let on_target = fx
        .ctx
        .requests
        .get_history(fx.target, &admin())
        .await
        .expect("history target");
    let merge = on_target
        .iter()
        .find(|e| e.action == "repair_merge")
        .expect("запись repair_merge");
    assert_eq!(merge.related_request_id, Some(fx.source));
    assert!(
        on_target
            .iter()
            .filter(|e| e.action != "repair_merge")
            .all(|e| e.related_request_id.is_none()),
        "{on_target:?}"
    );
}

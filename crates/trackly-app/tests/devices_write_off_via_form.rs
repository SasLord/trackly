//! Phase 41.8 Plan 21 (D-26, UAT-11): перевод участника группы на «Списано» через
//! обычное обновление устройства (`DeviceService::update`) требует явного выбора
//! `group_write_off` (оставить / вывести); якорь через форму не списывается.
//! Реальный `AppCtx` на временном каталоге; имена вымышленные.
//! Префикс тестов: `u11_`.

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
use trackly_app::dto::device::{DevicePatch, DeviceSaveOutcome, GroupWriteOffChoice};
use trackly_app::dto::printer::WsEvent;
use trackly_app::http::auth::SessionIdentity;
use trackly_app::http::build_router;
use trackly_app::server::rusqlite_session_store::RusqliteSessionStore;
use trackly_core::auth::{Identity, Role};
use trackly_core::error::AppError;
use trackly_infra::error_conversions::map_rusqlite;

async fn make_ctx() -> (AppCtx, tempfile::TempDir) {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let paths = trackly_infra::Paths::resolve_for_exe_dir(dir.path().to_path_buf()).expect("paths");
    let config = trackly_infra::AppConfig::default();
    let log_guard = trackly_app::logging::init(&paths, &config).unwrap_or_else(|_| {
        let (_nb, guard) = tracing_appender::non_blocking(std::io::sink());
        guard
    });
    let ctx = AppCtx::build(paths, config, log_guard).await.expect("ctx");
    (ctx, dir)
}

fn admin() -> Identity {
    Identity::trusted_admin()
}

async fn scalar(ctx: &AppCtx, sql: &'static str, p: Vec<rusqlite::types::Value>) -> i64 {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(sql, rusqlite::params_from_iter(p), |r| r.get::<_, i64>(0))
                .map_err(map_rusqlite)
        })
        .await
        .expect("scalar")
}

fn int(v: i64) -> rusqlite::types::Value {
    rusqlite::types::Value::Integer(v)
}

async fn status_id(ctx: &AppCtx, code: &'static str) -> i64 {
    scalar(
        ctx,
        "SELECT id FROM device_statuses WHERE code = ?1",
        vec![rusqlite::types::Value::Text(code.to_string())],
    )
    .await
}

async fn seed_place(ctx: &AppCtx, name: &str) -> i64 {
    let name = name.to_string();
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO places (kind, name, parent_id, is_storage, created_at_utc, \
                 updated_at_utc, version) VALUES ('room', ?1, NULL, 0, 1700000000, 1700000000, 1)",
                params![name],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed place")
}

async fn seed_group(ctx: &AppCtx, behavior: &'static str, name: &str, place: Option<i64>) -> i64 {
    let name = name.to_string();
    ctx.writer
        .execute(move |conn| {
            let t: i64 = conn
                .query_row(
                    "SELECT id FROM group_types WHERE code = ?1 OR behavior = ?1 LIMIT 1",
                    params![behavior],
                    |r| r.get(0),
                )
                .map_err(map_rusqlite)?;
            let seq: i64 = conn
                .query_row("SELECT COALESCE(MAX(seq), 0) + 1 FROM groups", [], |r| {
                    r.get(0)
                })
                .map_err(map_rusqlite)?;
            conn.execute(
                "INSERT INTO groups (type_id, name, seq, place_id, parent_group_id, \
                 created_at_utc, updated_at_utc, version) \
                 VALUES (?1, ?2, ?3, ?4, NULL, 1700000000, 1700000000, 1)",
                params![t, name, seq, place],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed group")
}

async fn seed_device(
    ctx: &AppCtx,
    name: &str,
    inv: &str,
    place: Option<i64>,
    status_code: &'static str,
    group: Option<i64>,
) -> i64 {
    let (name, inv) = (name.to_string(), inv.to_string());
    ctx.writer
        .execute(move |conn| {
            let st: i64 = conn
                .query_row(
                    "SELECT id FROM device_statuses WHERE code = ?1",
                    params![status_code],
                    |r| r.get(0),
                )
                .map_err(map_rusqlite)?;
            conn.execute(
                "INSERT INTO devices (type_id, name, inventory_number, place_id, status_id, \
                 created_at_utc, updated_at_utc, version) \
                 VALUES (1, ?1, ?2, ?3, ?4, 1700000000, 1700000000, 1)",
                params![name, inv, place, st],
            )
            .map_err(map_rusqlite)?;
            let id = conn.last_insert_rowid();
            if let Some(g) = group {
                conn.execute(
                    "INSERT INTO group_devices (device_id, group_id, added_at_utc) \
                     VALUES (?1, ?2, 1700000000)",
                    params![id, g],
                )
                .map_err(map_rusqlite)?;
            }
            Ok(id)
        })
        .await
        .expect("seed device")
}

async fn set_anchor(ctx: &AppCtx, group: i64, device: i64) {
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "UPDATE groups SET anchor_device_id = ?2 WHERE id = ?1",
                params![group, device],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("set anchor");
}

async fn set_origin(ctx: &AppCtx, device: i64, group: i64) {
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "UPDATE devices SET origin_group_id = ?2 WHERE id = ?1",
                params![device, group],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("set origin");
}

async fn version_of(ctx: &AppCtx, id: i64) -> i64 {
    scalar(
        ctx,
        "SELECT version FROM devices WHERE id = ?1",
        vec![int(id)],
    )
    .await
}

async fn status_code_of(ctx: &AppCtx, id: i64) -> String {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(
                "SELECT ds.code FROM devices d JOIN device_statuses ds ON ds.id = d.status_id \
                 WHERE d.id = ?1",
                params![id],
                |r| r.get::<_, String>(0),
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("status code")
}

async fn place_of(ctx: &AppCtx, id: i64) -> Option<i64> {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(
                "SELECT place_id FROM devices WHERE id = ?1",
                params![id],
                |r| r.get::<_, Option<i64>>(0),
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("place")
}

async fn member_rows(ctx: &AppCtx, id: i64) -> i64 {
    scalar(
        ctx,
        "SELECT COUNT(*) FROM group_devices WHERE device_id = ?1",
        vec![int(id)],
    )
    .await
}

async fn audit_count(ctx: &AppCtx, action: &'static str, id: i64) -> i64 {
    scalar(
        ctx,
        "SELECT COUNT(*) FROM audit_log WHERE action = ?1 AND entity_id = ?2 \
         AND entity_type = 'device'",
        vec![rusqlite::types::Value::Text(action.to_string()), int(id)],
    )
    .await
}

fn write_off_patch(st: i64, choice: Option<GroupWriteOffChoice>) -> DevicePatch {
    DevicePatch {
        status_id: Some(st),
        group_write_off: choice,
        ..Default::default()
    }
}

async fn update_err(ctx: &AppCtx, id: i64, patch: DevicePatch) -> AppError {
    let v = version_of(ctx, id).await;
    ctx.devices
        .update(&admin(), id, v, patch)
        .await
        .expect_err("ожидали отказ")
}

async fn update_ok(ctx: &AppCtx, id: i64, patch: DevicePatch) {
    let v = version_of(ctx, id).await;
    let out = ctx
        .devices
        .update(&admin(), id, v, patch)
        .await
        .expect("update");
    assert!(matches!(out, DeviceSaveOutcome::Created(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn u11_member_without_choice_is_rejected() {
    let (ctx, _d) = make_ctx().await;
    let place = seed_place(&ctx, "Кабинет 1").await;
    let g = seed_group(&ctx, "workstation", "АРМ #3", Some(place)).await;
    let dev = seed_device(
        &ctx,
        "Монитор #2",
        "INV-U11-1",
        Some(place),
        "в_работе",
        Some(g),
    )
    .await;
    let written = status_id(&ctx, "списано").await;
    let v = version_of(&ctx, dev).await;

    let err = update_err(&ctx, dev, write_off_patch(written, None)).await;
    match err {
        AppError::Validation { field, .. } => assert_eq!(field, "group_write_off"),
        other => panic!("ожидали Validation, получили {other:?}"),
    }
    assert_eq!(status_code_of(&ctx, dev).await, "в_работе");
    assert_eq!(version_of(&ctx, dev).await, v, "версия не меняется");
    assert_eq!(member_rows(&ctx, dev).await, 1, "членство не тронуто");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn u11_keep_keeps_membership_and_place() {
    let (ctx, _d) = make_ctx().await;
    let group_place = seed_place(&ctx, "Кабинет 1").await;
    let own_place = seed_place(&ctx, "Склад списания").await;
    let g = seed_group(&ctx, "workstation", "АРМ #3", Some(group_place)).await;
    // «На списание»: исключённый статус, место устройства расходится с местом группы.
    let dev = seed_device(
        &ctx,
        "Монитор #2",
        "INV-U11-2",
        Some(own_place),
        "на_списание",
        Some(g),
    )
    .await;
    let written = status_id(&ctx, "списано").await;

    update_ok(
        &ctx,
        dev,
        write_off_patch(written, Some(GroupWriteOffChoice::Keep)),
    )
    .await;

    assert_eq!(status_code_of(&ctx, dev).await, "списано");
    assert_eq!(member_rows(&ctx, dev).await, 1, "остался членом");
    assert_eq!(
        place_of(&ctx, dev).await,
        Some(own_place),
        "место не стянуто на место группы (D-24)"
    );
    assert_eq!(
        audit_count(&ctx, "update", dev).await,
        1,
        "одна запись update"
    );
    assert_eq!(
        audit_count(&ctx, "custom:group_member_released", dev).await,
        0,
        "вывода нет"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn u11_release_in_one_transaction() {
    let (ctx, _d) = make_ctx().await;
    let td = seed_group(&ctx, "teardown", "Разбор #7", None).await;
    let dev = seed_device(
        &ctx,
        "Блок питания",
        "INV-U11-3",
        None,
        "в_работе",
        Some(td),
    )
    .await;
    set_origin(&ctx, dev, td).await;
    let written = status_id(&ctx, "списано").await;

    update_ok(
        &ctx,
        dev,
        write_off_patch(written, Some(GroupWriteOffChoice::Release)),
    )
    .await;

    assert_eq!(status_code_of(&ctx, dev).await, "списано");
    assert_eq!(member_rows(&ctx, dev).await, 0, "выведен из состава");
    assert_eq!(
        audit_count(&ctx, "custom:group_member_released", dev).await,
        1,
        "запись вывода по устройству (D-29)"
    );
    assert_eq!(
        audit_count(&ctx, "update", dev).await,
        1,
        "одна запись update"
    );
    // Происхождение сохранено: деталь остаётся призраком группы разбора.
    assert_eq!(
        scalar(
            &ctx,
            "SELECT origin_group_id FROM devices WHERE id = ?1",
            vec![int(dev)]
        )
        .await,
        td
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn u11_anchor_rejected() {
    let (ctx, _d) = make_ctx().await;
    let place = seed_place(&ctx, "Кабинет 1").await;
    let g = seed_group(&ctx, "workstation", "АРМ #4", Some(place)).await;
    let anchor = seed_device(
        &ctx,
        "Системный блок #4",
        "INV-U11-4",
        Some(place),
        "в_работе",
        Some(g),
    )
    .await;
    set_anchor(&ctx, g, anchor).await;
    let written = status_id(&ctx, "списано").await;
    let v = version_of(&ctx, anchor).await;

    for choice in [
        None,
        Some(GroupWriteOffChoice::Keep),
        Some(GroupWriteOffChoice::Release),
    ] {
        let err = update_err(&ctx, anchor, write_off_patch(written, choice)).await;
        match err {
            AppError::Validation { message, .. } => {
                assert!(message.contains("спишите его через группу"), "{message}");
            }
            other => panic!("ожидали Validation, получили {other:?}"),
        }
    }
    assert_eq!(status_code_of(&ctx, anchor).await, "в_работе");
    assert_eq!(version_of(&ctx, anchor).await, v);
    assert_eq!(member_rows(&ctx, anchor).await, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn u11_non_member_unchanged() {
    let (ctx, _d) = make_ctx().await;
    let dev = seed_device(&ctx, "Ноутбук 5", "INV-U11-5", None, "в_работе", None).await;
    let written = status_id(&ctx, "списано").await;
    update_ok(&ctx, dev, write_off_patch(written, None)).await;
    assert_eq!(status_code_of(&ctx, dev).await, "списано");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn u11_edit_other_field_of_written_off_member_ok() {
    let (ctx, _d) = make_ctx().await;
    let place = seed_place(&ctx, "Кабинет 1").await;
    let g = seed_group(&ctx, "workstation", "АРМ #3", Some(place)).await;
    let dev = seed_device(
        &ctx,
        "Монитор #2",
        "INV-U11-6",
        Some(place),
        "списано",
        Some(g),
    )
    .await;
    let written = status_id(&ctx, "списано").await;

    update_ok(
        &ctx,
        dev,
        DevicePatch {
            name: Some("Монитор #2 (правка)".to_string()),
            status_id: Some(written),
            ..Default::default()
        },
    )
    .await;
    assert_eq!(status_code_of(&ctx, dev).await, "списано");
    assert_eq!(member_rows(&ctx, dev).await, 1);
}

async fn create_session_cookie(store: &RusqliteSessionStore, user_id: i64, role: Role) -> String {
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
        .insert("identity".to_string(), serde_json::to_value(&si).unwrap());
    store.create(&mut record).await.expect("create session");
    format!("id={session_id}")
}

async fn http_update(
    ctx: &AppCtx,
    cookie: &str,
    id: i64,
    patch: serde_json::Value,
) -> (StatusCode, String) {
    let version = version_of(ctx, id).await;
    let app = build_router(
        ctx,
        RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone()),
    );
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/devices_update")
        .header("content-type", "application/json")
        .header("cookie", cookie)
        .body(Body::from(
            serde_json::to_string(&json!({ "id": id, "version": version, "patch": patch }))
                .unwrap(),
        ))
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024)
        .await
        .unwrap_or_default();
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn u11_http_cannot_bypass() {
    let (ctx, _d) = make_ctx().await;
    let user = ctx
        .auth
        .create_user(
            UserNew {
                login: "u11_admin".to_string(),
                full_name: "Иванов И.И.".to_string(),
                password: "password123".to_string(),
                role: "admin".to_string(),
                email: None,
            },
            &Identity::trusted_admin(),
        )
        .await
        .expect("create user");
    let store = RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone());
    let cookie = create_session_cookie(&store, user.id, Role::Admin).await;

    let place = seed_place(&ctx, "Кабинет 1").await;
    let g = seed_group(&ctx, "workstation", "АРМ #3", Some(place)).await;
    let dev = seed_device(
        &ctx,
        "Монитор #2",
        "INV-U11-7",
        Some(place),
        "в_работе",
        Some(g),
    )
    .await;
    let written = status_id(&ctx, "списано").await;

    let (status, body) = http_update(&ctx, &cookie, dev, json!({ "status_id": written })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(
        body.contains("group_write_off") || body.contains("выберите"),
        "{body}"
    );
    assert_eq!(status_code_of(&ctx, dev).await, "в_работе");

    let (status, body) = http_update(
        &ctx,
        &cookie,
        dev,
        json!({ "status_id": written, "group_write_off": "keep" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(status_code_of(&ctx, dev).await, "списано");
    assert_eq!(member_rows(&ctx, dev).await, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn u11_event_carries_group() {
    let (ctx, _d) = make_ctx().await;
    let place = seed_place(&ctx, "Кабинет 1").await;
    let g = seed_group(&ctx, "workstation", "АРМ #3", Some(place)).await;
    let released = seed_device(
        &ctx,
        "Монитор #2",
        "INV-U11-8",
        Some(place),
        "в_работе",
        Some(g),
    )
    .await;
    let kept = seed_device(
        &ctx,
        "Монитор #3",
        "INV-U11-9",
        Some(place),
        "в_работе",
        Some(g),
    )
    .await;
    let written = status_id(&ctx, "списано").await;

    for (dev, choice) in [
        (released, GroupWriteOffChoice::Release),
        (kept, GroupWriteOffChoice::Keep),
    ] {
        let mut rx = ctx.ws_broadcast.subscribe();
        while rx.try_recv().is_ok() {}
        update_ok(&ctx, dev, write_off_patch(written, Some(choice))).await;
        let mut found = None;
        while let Ok(ev) = rx.try_recv() {
            if let WsEvent::EntitiesChanged {
                device_ids,
                group_ids,
                ..
            } = ev
            {
                found = Some((device_ids, group_ids));
            }
        }
        let (devices, groups) = found.expect("EntitiesChanged");
        assert_eq!(devices, vec![dev]);
        assert_eq!(
            groups,
            vec![g],
            "событие несёт группу устройства ({choice:?})"
        );
    }
}

/// Ревью WR-05 (D-06, D-07): списание через карточку устройства пишет ту же причину, что
/// списание через группу, — «при списании» (`device_write_off`) и у статуса, и у вывода.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn u11_release_history_cause_is_write_off() {
    let (ctx, _d) = make_ctx().await;
    let g = seed_group(&ctx, "container", "АРМ #31", None).await;
    let dev = seed_device(&ctx, "Монитор", "INV-WR05-1", None, "в_работе", Some(g)).await;
    let written = status_id(&ctx, "списано").await;

    update_ok(
        &ctx,
        dev,
        write_off_patch(written, Some(GroupWriteOffChoice::Release)),
    )
    .await;

    let sources: Vec<(String, String)> = ctx
        .writer
        .execute(move |conn| {
            let mut st = conn
                .prepare(
                    "SELECT kind, source FROM history_events \
                     WHERE subject_type = 'device' AND subject_id = ?1 ORDER BY id",
                )
                .map_err(map_rusqlite)?;
            let rows = st
                .query_map(params![dev], |r| Ok((r.get(0)?, r.get(1)?)))
                .map_err(map_rusqlite)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(map_rusqlite)?;
            Ok(rows)
        })
        .await
        .expect("events");
    let mut kinds: Vec<(String, String)> = sources;
    kinds.sort();
    assert_eq!(
        kinds,
        vec![
            ("group_left".to_string(), "device_write_off".to_string()),
            ("status_changed".to_string(), "device_write_off".to_string()),
        ],
        "причина списания одна на всех экранах"
    );
}

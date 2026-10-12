//! Phase 41.1 Plan 09: серверная охрана инварианта якоря группы (GRP-10, D-14/D-20/D-36).
//!
//! Якорь нельзя ни удалить (мягкое удаление FK не ловит, охрана в `DeviceService`),
//! ни вывести из состава (`remove_devices`), ни передать актом (`release_device_in_tx`).
//! Запись разбора с живыми деталями не удаляется. Реальный `AppCtx` на временном
//! каталоге; имена вымышленные.
//!
//! Префикс тестов: `anchor_`. Хелпер `make_anchor_group` переиспользует план 11.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use rusqlite::params;
use serde_json::json;
use time::{Duration, OffsetDateTime};
use tower::ServiceExt;
use tower_sessions::session::{Id, Record};
use tower_sessions::SessionStore;

use trackly_app::context::AppCtx;
use trackly_app::dto::act::{ActCreateDto, ActItemNewDto};
use trackly_app::dto::auth::UserNew;
use trackly_app::dto::groups::GroupCreateDto;
use trackly_app::dto::number_template::NumberFieldInput;
use trackly_app::http::auth::SessionIdentity;
use trackly_app::http::build_router;
use trackly_app::server::rusqlite_session_store::RusqliteSessionStore;
use trackly_core::auth::{Identity, Role};
use trackly_core::error::AppError;
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::repos::SqliteGroupRepository;

// ---------------------------------------------------------------------------
// Каркас
// ---------------------------------------------------------------------------

async fn make_test_ctx() -> (AppCtx, tempfile::TempDir) {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let paths =
        trackly_infra::Paths::resolve_for_exe_dir(dir.path().to_path_buf()).expect("resolve paths");
    let config = trackly_infra::AppConfig::default();
    let log_guard = trackly_app::logging::init(&paths, &config).or_else(|_| {
        let (_nb, guard) = tracing_appender::non_blocking(std::io::sink());
        Ok::<_, anyhow::Error>(guard)
    });
    let ctx = AppCtx::build(paths, config, log_guard.expect("log guard"))
        .await
        .expect("build ctx");
    (ctx, dir)
}

fn admin() -> Identity {
    Identity::trusted_admin()
}

async fn scalar_i64(ctx: &AppCtx, sql: &'static str, id: i64) -> i64 {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(sql, params![id], |r| r.get::<_, i64>(0))
                .map_err(map_rusqlite)
        })
        .await
        .expect("scalar query")
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

async fn seed_device(ctx: &AppCtx, name: &str, inv: &str, place: Option<i64>) -> i64 {
    let (name, inv) = (name.to_string(), inv.to_string());
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO devices (type_id, name, inventory_number, place_id, status_id, \
                 created_at_utc, updated_at_utc, version) \
                 VALUES (1, ?1, ?2, ?3, 1, 1700000000, 1700000000, 1)",
                params![name, inv, place],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed device")
}

/// Тип группы нужного поведения: берётся из БД, при отсутствии создаётся.
/// Числовые id не зашиваются.
async fn type_by_behavior(ctx: &AppCtx, behavior: &'static str) -> i64 {
    ctx.writer
        .execute(move |conn| {
            let found: Option<i64> = conn
                .query_row(
                    "SELECT id FROM group_types WHERE behavior = ?1 LIMIT 1",
                    params![behavior],
                    |r| r.get(0),
                )
                .ok();
            if let Some(id) = found {
                return Ok(id);
            }
            conn.execute(
                "INSERT INTO group_types (code, name, behavior, created_at_utc, updated_at_utc) \
                 VALUES (?1, ?2, ?3, 1700000000, 1700000000)",
                params![format!("anchor_t_{behavior}"), "Тип теста", behavior],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("type by behavior")
}

/// Группа заданного поведения с якорем-устройством в составе. Создание группы от
/// устройства (план 11) пользуется тем же приёмом: состав + `set_anchor_in_tx`.
/// Возвращает (id группы, id якорного устройства).
async fn make_anchor_group(
    ctx: &AppCtx,
    behavior: &'static str,
    group_name: &str,
    anchor_name: &str,
    inv: &str,
    place: Option<i64>,
) -> (i64, i64) {
    let type_id = type_by_behavior(ctx, behavior).await;
    let group = ctx
        .groups
        .create_group(
            &admin(),
            GroupCreateDto {
                type_id,
                name: Some(group_name.to_string()),
                place_id: None,
                source_device_id: None,
            },
        )
        .await
        .expect("create_group");
    let anchor = seed_device(ctx, anchor_name, inv, place).await;
    ctx.groups
        .add_devices(&admin(), group.id, vec![anchor])
        .await
        .expect("add anchor to composition");
    let group_id = group.id;
    ctx.writer
        .execute(move |conn| {
            let tx = conn.transaction().map_err(map_rusqlite)?;
            SqliteGroupRepository.set_anchor_in_tx(&tx, group_id, anchor)?;
            tx.commit().map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("set anchor");
    (group_id, anchor)
}

async fn device_version(ctx: &AppCtx, id: i64) -> i64 {
    scalar_i64(ctx, "SELECT version FROM devices WHERE id = ?1", id).await
}

async fn device_alive(ctx: &AppCtx, id: i64) -> bool {
    scalar_i64(
        ctx,
        "SELECT COUNT(*) FROM devices WHERE id = ?1 AND deleted_at_utc IS NULL",
        id,
    )
    .await
        == 1
}

async fn member_rows(ctx: &AppCtx, device_id: i64) -> i64 {
    scalar_i64(
        ctx,
        "SELECT COUNT(*) FROM group_devices WHERE device_id = ?1",
        device_id,
    )
    .await
}

async fn group_alive(ctx: &AppCtx, id: i64) -> bool {
    scalar_i64(
        ctx,
        "SELECT COUNT(*) FROM groups WHERE id = ?1 AND deleted_at_utc IS NULL",
        id,
    )
    .await
        == 1
}

fn validation(err: AppError, expected_field: &str) -> String {
    match err {
        AppError::Validation { field, message } => {
            assert_eq!(field, expected_field, "поле ошибки");
            message
        }
        other => panic!("ожидали Validation, получили {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Test 1-2: мягкое удаление якоря
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn anchor_delete_soft_is_rejected_and_device_stays_alive() {
    let (ctx, _dir) = make_test_ctx().await;
    let (g, anchor) = make_anchor_group(
        &ctx,
        "container",
        "АРМ #7",
        "Системный блок #4",
        "INV-AN-1",
        None,
    )
    .await;
    let version = device_version(&ctx, anchor).await;

    let err = ctx
        .devices
        .delete_soft(anchor, version)
        .await
        .expect_err("якорь удалять нельзя");
    let msg = validation(err, "id");
    assert!(msg.contains("Системный блок #4"), "{msg}");
    assert!(msg.contains("якорь группы «АРМ #7»"), "{msg}");

    assert!(device_alive(&ctx, anchor).await, "устройство живо");
    assert_eq!(member_rows(&ctx, anchor).await, 1, "членство не тронуто");
    assert!(group_alive(&ctx, g).await);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn anchor_after_group_delete_device_is_ordinary_again() {
    let (ctx, _dir) = make_test_ctx().await;
    let (g, anchor) =
        make_anchor_group(&ctx, "container", "АРМ #8", "Монитор #2", "INV-AN-2", None).await;

    ctx.groups
        .delete_group(&admin(), g)
        .await
        .expect("контейнерная группа удаляется");
    let version = device_version(&ctx, anchor).await;
    ctx.devices
        .delete_soft(anchor, version)
        .await
        .expect("после удаления группы устройство обычное");
    assert!(!device_alive(&ctx, anchor).await);
}

// ---------------------------------------------------------------------------
// Test 3: remove_devices
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn anchor_remove_devices_rejects_anchor_but_accepts_ordinary_member() {
    let (ctx, _dir) = make_test_ctx().await;
    let (g, anchor) = make_anchor_group(
        &ctx,
        "container",
        "АРМ #9",
        "Системный блок #4",
        "INV-AN-3",
        None,
    )
    .await;
    let mate = seed_device(&ctx, "Монитор #2", "INV-AN-3M", None).await;
    ctx.groups
        .add_devices(&admin(), g, vec![mate])
        .await
        .expect("add mate");

    // Якорь вместе с обычным членом: отказ целиком, состав не изменился.
    let err = ctx
        .groups
        .remove_devices(&admin(), g, vec![mate, anchor])
        .await
        .expect_err("якорь вывести нельзя");
    let msg = validation(err, "device_ids");
    assert!(msg.contains("якорь группы «АРМ #9»"), "{msg}");
    assert_eq!(member_rows(&ctx, anchor).await, 1, "якорь в составе");
    assert_eq!(
        member_rows(&ctx, mate).await,
        1,
        "откат: сосед тоже на месте"
    );

    // Обычный член выводится.
    let n = ctx
        .groups
        .remove_devices(&admin(), g, vec![mate])
        .await
        .expect("обычный член");
    assert_eq!(n, 1);
    assert_eq!(member_rows(&ctx, mate).await, 0);
    assert_eq!(member_rows(&ctx, anchor).await, 1);
}

// ---------------------------------------------------------------------------
// Test 4 + 7: передача якоря актом (release_device_in_tx)
// ---------------------------------------------------------------------------

fn act_number(value: &str) -> NumberFieldInput {
    NumberFieldInput {
        value: value.into(),
        template_id: None,
        confirm_mismatch: false,
        confirm_script_mix: false,
    }
}

fn handover_dto(number: &str, devices: &[i64], place: i64) -> ActCreateDto {
    ActCreateDto {
        number_input: act_number(number),
        giver_name: "Иванов И.И.".into(),
        receiver_name: "Петров П.П.".into(),
        place_id: Some(place),
        notes: None,
        deadline_utc: None,
        handover_date_utc: None,
        items: devices
            .iter()
            .map(|&id| ActItemNewDto {
                device_id: id,
                device_ids: Vec::new(),
                quantity: 1,
            })
            .collect(),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn anchor_act_handover_is_rejected_and_state_unchanged() {
    let (ctx, _dir) = make_test_ctx().await;
    let place_a = seed_place(&ctx, "Склад А").await;
    let place_b = seed_place(&ctx, "Склад Б").await;
    let (g, anchor) = make_anchor_group(
        &ctx,
        "container",
        "АРМ #10",
        "Системный блок #4",
        "INV-AN-4",
        Some(place_a),
    )
    .await;
    let outsider = seed_device(&ctx, "Ноутбук #1", "INV-AN-4O", Some(place_a)).await;

    // Якорь вместе с посторонним: акт отклонён целиком.
    let err = ctx
        .acts
        .create(&admin(), handover_dto("AN-1", &[outsider, anchor], place_b))
        .await
        .expect_err("якорь актом не передаётся");
    let msg = validation(err, "id");
    assert!(msg.contains("актом"), "{msg}");
    assert!(msg.contains("Системный блок #4"), "{msg}");

    assert_eq!(member_rows(&ctx, anchor).await, 1, "членство якоря цело");
    assert!(group_alive(&ctx, g).await);
    let place_of = |id: i64| {
        let ctx = &ctx;
        async move { scalar_i64(ctx, "SELECT place_id FROM devices WHERE id = ?1", id).await }
    };
    assert_eq!(place_of(anchor).await, place_a, "место якоря не менялось");
    assert_eq!(
        place_of(outsider).await,
        place_a,
        "откат: посторонний тоже не передан"
    );
    assert_eq!(
        scalar_i64(&ctx, "SELECT COUNT(*) FROM acts WHERE id > ?1", 0).await,
        0,
        "акт не создан"
    );

    // Контроль: обычное устройство вне групп передаётся как раньше.
    ctx.acts
        .create(&admin(), handover_dto("AN-2", &[outsider], place_b))
        .await
        .expect("обычное устройство передаётся");
    assert_eq!(place_of(outsider).await, place_b);
}

// ---------------------------------------------------------------------------
// Test 5: удаление группы разбора
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn anchor_teardown_group_with_live_parts_cannot_be_deleted() {
    let (ctx, _dir) = make_test_ctx().await;
    let (g, _anchor) = make_anchor_group(
        &ctx,
        "teardown",
        "Разбор #1",
        "Системный блок #4",
        "INV-AN-5",
        None,
    )
    .await;
    let mut parts = Vec::new();
    for i in 0..3 {
        let id = seed_device(
            &ctx,
            &format!("Планка памяти {i}"),
            &format!("INV-AN-5P{i}"),
            None,
        )
        .await;
        parts.push(id);
    }
    ctx.groups
        .add_devices(&admin(), g, parts.clone())
        .await
        .expect("add parts");

    let err = ctx
        .groups
        .delete_group(&admin(), g)
        .await
        .expect_err("с деталями удалять нельзя");
    let msg = validation(err, "id");
    assert!(msg.contains("в ней 3 детали"), "{msg}");
    assert!(msg.contains("Разбор #1"), "{msg}");
    assert!(group_alive(&ctx, g).await, "группа цела");
    assert_eq!(member_rows(&ctx, parts[0]).await, 1);

    // Мягко-удалённая деталь не считается: остаются две.
    let dead = parts[2];
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "UPDATE devices SET deleted_at_utc = 1700000500 WHERE id = ?1",
                params![dead],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("soft-delete part");
    let msg = validation(
        ctx.groups
            .delete_group(&admin(), g)
            .await
            .expect_err("две детали"),
        "id",
    );
    assert!(msg.contains("в ней 2 детали"), "{msg}");

    // Детали выведены -> группа с одним якорем удаляется.
    ctx.groups
        .remove_devices(&admin(), g, vec![parts[0], parts[1], parts[2]])
        .await
        .expect("вывод деталей");
    ctx.groups
        .delete_group(&admin(), g)
        .await
        .expect("без деталей удаляется");
    assert!(!group_alive(&ctx, g).await);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn anchor_non_teardown_group_with_parts_is_deleted() {
    let (ctx, _dir) = make_test_ctx().await;
    let (g, anchor) = make_anchor_group(
        &ctx,
        "substitute",
        "Замена #1",
        "Системный блок #4",
        "INV-AN-6",
        None,
    )
    .await;
    let part = seed_device(&ctx, "Планка памяти", "INV-AN-6P", None).await;
    ctx.groups
        .add_devices(&admin(), g, vec![part])
        .await
        .expect("add part");

    let out = ctx
        .groups
        .delete_group(&admin(), g)
        .await
        .expect("группа не-разбора с составом удаляется");
    assert_eq!(out.released_devices, 2);
    assert!(!group_alive(&ctx, g).await);
    assert_eq!(member_rows(&ctx, anchor).await, 0);
}

// ---------------------------------------------------------------------------
// Test 6: HTTP-транспорт (обход скрытых пунктов UI)
// ---------------------------------------------------------------------------

async fn session_cookie(store: &RusqliteSessionStore, user_id: i64) -> String {
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
        .insert("identity".to_string(), serde_json::to_value(&si).unwrap());
    store.create(&mut record).await.expect("create session");
    format!("id={session_id}")
}

async fn post_json(
    app: axum::Router,
    uri: &str,
    body: serde_json::Value,
    cookie: &str,
) -> (StatusCode, String) {
    let req = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .header("cookie", cookie)
        .body(Body::from(serde_json::to_string(&body).unwrap()))
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024)
        .await
        .unwrap_or_default();
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn anchor_http_devices_delete_rejects_anchor() {
    let (ctx, _dir) = make_test_ctx().await;
    let user = ctx
        .auth
        .create_user(
            UserNew {
                login: "anchor_admin".to_string(),
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
    let cookie = session_cookie(&store, user.id).await;

    let (_g, anchor) = make_anchor_group(
        &ctx,
        "container",
        "АРМ #11",
        "Системный блок #4",
        "INV-AN-7",
        None,
    )
    .await;
    let version = device_version(&ctx, anchor).await;

    let app = build_router(&ctx, store);
    let (status, body) = post_json(
        app,
        "/api/v1/devices_delete",
        json!({ "id": anchor, "version": version }),
        &cookie,
    )
    .await;
    assert!(
        !status.is_success(),
        "ожидали отказ, получили {status}: {body}"
    );
    assert!(body.contains("якорь группы"), "{body}");
    assert!(device_alive(&ctx, anchor).await, "устройство живо");
}

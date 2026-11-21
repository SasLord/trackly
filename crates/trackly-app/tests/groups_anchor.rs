//! Phase 41.1 Plan 09: серверная охрана инварианта якоря группы (GRP-10, D-14/D-20/D-36).
//!
//! Якорь нельзя ни удалить (мягкое удаление FK не ловит, охрана в `DeviceService`),
//! ни вывести из состава (`remove_devices`). Передача якоря актом разрешена (Phase 41.2,
//! D-13): он уходит вместе с группой, охрана в `release_device_in_tx` остаётся внутренним
//! инвариантом.
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
    if behavior == "teardown" {
        set_origin(ctx, anchor, group.id).await;
    }
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
    scalar_i64(ctx, "SELECT COUNT(*) FROM groups WHERE id = ?1", id).await == 1
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
// Test 4 + 7: передача якоря актом (группа переезжает целиком)
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
        confirm_group_release: false,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn anchor_act_handover_moves_group_whole() {
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

    // Phase 41.2 (D-13): акт якоря больше не отклоняется — якорь передаётся вместе с группой.
    // Якорь вместе с посторонним: акт создан, группа и якорь переехали на место акта.
    ctx.acts
        .create(&admin(), handover_dto("AN-1", &[outsider, anchor], place_b))
        .await
        .expect("якорь передаётся актом вместе с группой")
        .expect_created("anchor handover");

    assert_eq!(member_rows(&ctx, anchor).await, 1, "членство якоря цело");
    assert!(group_alive(&ctx, g).await);
    let place_of = |id: i64| {
        let ctx = &ctx;
        async move { scalar_i64(ctx, "SELECT place_id FROM devices WHERE id = ?1", id).await }
    };
    assert_eq!(place_of(anchor).await, place_b, "якорь на месте акта");
    assert_eq!(
        scalar_i64(&ctx, "SELECT place_id FROM groups WHERE id = ?1", g).await,
        place_b,
        "группа переехала вместе с якорем"
    );
    assert_eq!(
        place_of(outsider).await,
        place_b,
        "посторонний тоже передан"
    );
    assert_eq!(
        scalar_i64(&ctx, "SELECT COUNT(*) FROM acts WHERE id > ?1", 0).await,
        1,
        "акт создан"
    );

    // Контроль: обычное устройство вне групп передаётся как раньше.
    let plain = seed_device(&ctx, "Ноутбук #2", "INV-AN-4P", Some(place_a)).await;
    ctx.acts
        .create(&admin(), handover_dto("AN-2", &[plain], place_b))
        .await
        .expect("обычное устройство передаётся");
    assert_eq!(place_of(plain).await, place_b);
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
        set_origin(&ctx, id, g).await;
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
    assert!(msg.contains("связаны 3 детали"), "{msg}");
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
    assert!(msg.contains("связаны 2 детали"), "{msg}");

    // D-13: выведенные детали хранят происхождение — удаление по-прежнему отказывает.
    ctx.groups
        .remove_devices(&admin(), g, vec![parts[0], parts[1], parts[2]])
        .await
        .expect("вывод деталей");
    let msg = validation(
        ctx.groups
            .delete_group(&admin(), g)
            .await
            .expect_err("призраки держат запись"),
        "id",
    );
    assert!(msg.contains("связаны 3 детали, из них 3 выведены"), "{msg}");
    assert!(group_alive(&ctx, g).await, "группа цела");

    // Происхождение снято -> группа с одним якорем удаляется.
    for p in &parts {
        let p = *p;
        ctx.writer
            .execute(move |conn| {
                conn.execute(
                    "UPDATE devices SET origin_group_id = NULL WHERE id = ?1",
                    params![p],
                )
                .map_err(map_rusqlite)?;
                Ok(())
            })
            .await
            .expect("снять origin");
    }
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

// ===========================================================================
// Phase 41.1 Plan 11: создание группы от устройства (GRD-01..03), серверные
// отказы быстрых действий, происхождение детали (D-33). Числовые id типов и
// статусов не зашиты: тип группы берётся по behavior, статус — по code.
// ===========================================================================

/// Устройство произвольного типа (1 — «Устройство», 2 — принтер).
async fn seed_typed_device(
    ctx: &AppCtx,
    type_id: i64,
    name: &str,
    inv: &str,
    place: Option<i64>,
) -> i64 {
    let (name, inv) = (name.to_string(), inv.to_string());
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO devices (type_id, name, inventory_number, place_id, status_id, \
                 created_at_utc, updated_at_utc, version) \
                 VALUES (?1, ?2, ?3, ?4, 1, 1700000000, 1700000000, 1)",
                params![type_id, name, inv, place],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed typed device")
}

/// Принтер: устройство типа 2 + строка `printers`. `ip = Some` — сетевой,
/// `ip = None` — USB (задан `usb_host_device_id`, D-32).
async fn seed_printer(ctx: &AppCtx, name: &str, inv: &str, ip: Option<&'static str>) -> i64 {
    let id = seed_typed_device(ctx, 2, name, inv, None).await;
    let host = if ip.is_none() {
        Some(seed_device(ctx, &format!("Хост {name}"), &format!("{inv}-H"), None).await)
    } else {
        None
    };
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO printers (device_id, ip_address, usb_host_device_id, \
                 created_at_utc, updated_at_utc) VALUES (?1, ?2, ?3, 1700000000, 1700000000)",
                params![id, ip, host],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("seed printer");
    id
}

async fn set_status_by_code(ctx: &AppCtx, device: i64, code: &'static str) {
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "UPDATE devices SET status_id = (SELECT id FROM device_statuses WHERE code = ?2) \
                 WHERE id = ?1",
                params![device, code],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("set status");
}

async fn status_code_of(ctx: &AppCtx, device: i64) -> String {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(
                "SELECT s.code FROM devices d JOIN device_statuses s ON s.id = d.status_id \
                 WHERE d.id = ?1",
                params![device],
                |r| r.get::<_, String>(0),
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("status code")
}

/// Фикстура «деталь разбора» (D-08): происхождение задаётся ДО ввода, так как ввод его
/// больше не пишет.
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

async fn origin_of(ctx: &AppCtx, device: i64) -> Option<i64> {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(
                "SELECT origin_group_id FROM devices WHERE id = ?1",
                params![device],
                |r| r.get::<_, Option<i64>>(0),
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("origin")
}

async fn count_all(ctx: &AppCtx, table: &'static str) -> i64 {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
                .map_err(map_rusqlite)
        })
        .await
        .expect("count")
}

async fn create_from(
    ctx: &AppCtx,
    behavior: &'static str,
    name: &str,
    source: i64,
) -> Result<trackly_app::dto::groups::GroupDto, AppError> {
    let type_id = type_by_behavior(ctx, behavior).await;
    ctx.groups
        .create_group(
            &admin(),
            GroupCreateDto {
                type_id,
                name: Some(name.to_string()),
                place_id: None,
                source_device_id: Some(source),
            },
        )
        .await
}

async fn plain_group(ctx: &AppCtx, behavior: &'static str, name: &str) -> i64 {
    let type_id = type_by_behavior(ctx, behavior).await;
    ctx.groups
        .create_group(
            &admin(),
            GroupCreateDto {
                type_id,
                name: Some(name.to_string()),
                place_id: None,
                source_device_id: None,
            },
        )
        .await
        .expect("plain group")
        .id
}

// ---- Test 1-3: три поведения создания от устройства -------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn anchor_create_container_from_device_takes_place_and_membership() {
    let (ctx, _dir) = make_test_ctx().await;
    let place = seed_place(&ctx, "Кабинет 214").await;
    let dev = seed_device(&ctx, "Системный блок #4", "INV-C-1", Some(place)).await;
    let moves_before = count_all(&ctx, "place_movements").await;
    let version = device_version(&ctx, dev).await;
    let status_before = status_code_of(&ctx, dev).await;

    let g = create_from(&ctx, "container", "АРМ #21", dev)
        .await
        .expect("создание от устройства");

    assert_eq!(g.place_id, Some(place), "место группы = месту устройства");
    assert_eq!(g.anchor_device_id, None, "контейнер без якоря");
    assert_eq!(member_rows(&ctx, dev).await, 1, "устройство в составе");
    assert_eq!(
        status_code_of(&ctx, dev).await,
        status_before,
        "статус прежний"
    );
    assert_eq!(
        device_version(&ctx, dev).await,
        version,
        "устройство не тронуто"
    );
    assert_eq!(
        count_all(&ctx, "place_movements").await,
        moves_before,
        "создание не пишет place_movements"
    );
    let dev_place = scalar_i64(&ctx, "SELECT place_id FROM devices WHERE id = ?1", dev).await;
    assert_eq!(dev_place, place, "место устройства прежнее");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn anchor_create_substitute_from_device_sets_anchor_and_keeps_status() {
    let (ctx, _dir) = make_test_ctx().await;
    let dev = seed_device(&ctx, "Монитор #2", "INV-S-1", None).await;
    let status_before = status_code_of(&ctx, dev).await;

    let g = create_from(&ctx, "substitute", "Замена #1", dev)
        .await
        .expect("substitute");

    assert_eq!(g.anchor_device_id, Some(dev));
    assert_eq!(member_rows(&ctx, dev).await, 1);
    assert_eq!(
        status_code_of(&ctx, dev).await,
        status_before,
        "статус прежний"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn anchor_create_teardown_from_device_sets_anchor_and_write_off_status() {
    let (ctx, _dir) = make_test_ctx().await;
    let place = seed_place(&ctx, "Склад 3").await;
    let dev = seed_device(&ctx, "Системный блок #9", "INV-T-1", Some(place)).await;
    let version = device_version(&ctx, dev).await;
    let audit_before = count_all(&ctx, "audit_log").await;

    let g = create_from(&ctx, "teardown", "Разбор #1", dev)
        .await
        .expect("teardown");

    assert_eq!(g.anchor_device_id, Some(dev));
    assert_eq!(status_code_of(&ctx, dev).await, "на_списание");
    assert_eq!(device_version(&ctx, dev).await, version + 1, "version +1");
    let dev_place = scalar_i64(&ctx, "SELECT place_id FROM devices WHERE id = ?1", dev).await;
    assert_eq!(dev_place, place, "место прежнее");
    let device_audit = scalar_i64(
        &ctx,
        "SELECT COUNT(*) FROM audit_log WHERE entity_type = 'device' AND entity_id = ?1 \
         AND action = 'update'",
        dev,
    )
    .await;
    assert_eq!(device_audit, 1, "audit_log по устройству");
    assert!(
        count_all(&ctx, "audit_log").await >= audit_before + 2,
        "аудит группы и устройства"
    );
}

// ---- Test 4: отказы создания ------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn anchor_create_from_device_rejections_leave_no_group() {
    let (ctx, _dir) = make_test_ctx().await;
    let groups_before = count_all(&ctx, "groups").await;

    // Уже член другой группы.
    let member = seed_device(&ctx, "Монитор #5", "INV-R-1", None).await;
    let other = plain_group(&ctx, "container", "АРМ #30").await;
    ctx.groups
        .add_devices(&admin(), other, vec![member])
        .await
        .expect("add");
    let groups_with_other = count_all(&ctx, "groups").await;
    let msg = validation(
        create_from(&ctx, "container", "АРМ #31", member)
            .await
            .expect_err("в группе"),
        "source_device_id",
    );
    assert!(
        msg.contains("Монитор #5") && msg.contains("АРМ #30"),
        "{msg}"
    );

    // Списано.
    let written_off = seed_device(&ctx, "Монитор #6", "INV-R-2", None).await;
    set_status_by_code(&ctx, written_off, "списано").await;
    let msg = validation(
        create_from(&ctx, "container", "АРМ #32", written_off)
            .await
            .expect_err("списано"),
        "source_device_id",
    );
    assert!(msg.contains("«Списано»"), "{msg}");

    // На списание.
    let to_write_off = seed_device(&ctx, "Монитор #7", "INV-R-3", None).await;
    set_status_by_code(&ctx, to_write_off, "на_списание").await;
    let msg = validation(
        create_from(&ctx, "substitute", "Замена #2", to_write_off)
            .await
            .expect_err("на списание"),
        "source_device_id",
    );
    assert!(msg.contains("«На списание»"), "{msg}");

    // D-38: сетевой и USB-принтер якорем быть не могут.
    let net = seed_printer(&ctx, "Сетевой принтер", "INV-R-4", Some("192.0.2.10")).await;
    let usb = seed_printer(&ctx, "Принтер USB", "INV-R-5", None).await;
    for printer in [net, usb] {
        let msg = validation(
            create_from(&ctx, "substitute", "Замена #3", printer)
                .await
                .expect_err("принтер"),
            "source_device_id",
        );
        assert!(msg.contains("только от устройства"), "{msg}");
    }

    assert_eq!(
        count_all(&ctx, "groups").await,
        groups_with_other,
        "ни один отказ не оставил группу"
    );
    assert!(groups_with_other > groups_before);
    assert_eq!(status_code_of(&ctx, to_write_off).await, "на_списание");

    // D-16: сетевой принтер входит в УЖЕ существующую группу как любой (запрет D-32 снят,
    // «есть IP» не значит «общий»). USB-принтер входит по-прежнему.
    let target = plain_group(&ctx, "container", "АРМ #33").await;
    ctx.groups
        .add_devices(&admin(), target, vec![usb])
        .await
        .expect("USB-принтер вступает в группу");
    assert_eq!(member_rows(&ctx, usb).await, 1);
    ctx.groups
        .add_devices(&admin(), target, vec![net])
        .await
        .expect("D-16: сетевой принтер вступает в группу как любой");
    assert_eq!(member_rows(&ctx, net).await, 1);
}

// ---- Test 4b: сетевой принтер - обычный член (D-16) ---------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn add_devices_accepts_network_printer_like_any_member() {
    let (ctx, _dir) = make_test_ctx().await;

    // (а) контейнер: входит, состав растёт на единицу, повтор даёт прежнюю ошибку.
    let container = plain_group(&ctx, "container", "АРМ #50").await;
    let net = seed_printer(&ctx, "Сетевой принтер Б", "INV-N-1", Some("192.0.2.20")).await;
    let rows_before = count_all(&ctx, "group_devices").await;
    ctx.groups
        .add_devices(&admin(), container, vec![net])
        .await
        .expect("D-16: сетевой принтер входит в контейнер");
    assert_eq!(member_rows(&ctx, net).await, 1);
    assert_eq!(count_all(&ctx, "group_devices").await, rows_before + 1);
    let err = ctx
        .groups
        .add_devices(
            &admin(),
            plain_group(&ctx, "container", "АРМ #51").await,
            vec![net],
        )
        .await
        .expect_err("уже в группе");
    assert!(
        matches!(&err, AppError::AlreadyInGroup { items } if items.len() == 1 && items[0].id == net),
        "{err:?}"
    );
    assert_eq!(member_rows(&ctx, net).await, 1, "прочие отказы целы");

    // (в) карточка группы: строка принтера есть, происхождение member, явной ссылки нет.
    let card = ctx.groups.card(&admin(), container).await.expect("card");
    let row = card
        .printers
        .iter()
        .find(|p| p.device_id == net)
        .expect("сетевой принтер-член виден в карточке");
    assert_eq!(row.origin, "member");
    assert!(!row.has_explicit_link);

    // (б) группа-разбор: входит, происхождение пишется как у любого члена.
    let teardown = plain_group(&ctx, "teardown", "Разбор #50").await;
    let net2 = seed_printer(&ctx, "Сетевой принтер В", "INV-N-2", Some("192.0.2.21")).await;
    set_origin(&ctx, net2, teardown).await;
    ctx.groups
        .add_devices(&admin(), teardown, vec![net2])
        .await
        .expect("D-16: сетевой принтер входит в разбор");
    assert_eq!(member_rows(&ctx, net2).await, 1);
    assert_eq!(origin_of(&ctx, net2).await, Some(teardown), "origin цел");
}

// ---- Test 5: откат -----------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn anchor_create_teardown_failure_rolls_back_everything() {
    let (ctx, _dir) = make_test_ctx().await;
    let dev = seed_device(&ctx, "Системный блок #11", "INV-RB-1", None).await;
    let status_before = status_code_of(&ctx, dev).await;
    let version = device_version(&ctx, dev).await;
    // Тип создаётся до снимка счётчиков (иначе он сам изменит число строк).
    type_by_behavior(&ctx, "teardown").await;
    ctx.writer
        .execute(|conn| {
            conn.execute_batch(
                "CREATE TRIGGER fail_status BEFORE UPDATE OF status_id ON devices \
                 BEGIN SELECT RAISE(ABORT, 'boom'); END;",
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("trigger");
    let groups = count_all(&ctx, "groups").await;
    let members = count_all(&ctx, "group_devices").await;
    let audit = count_all(&ctx, "audit_log").await;

    let err = create_from(&ctx, "teardown", "Разбор #2", dev)
        .await
        .expect_err("сбой на смене статуса");
    assert!(format!("{err:?}").contains("boom"), "{err:?}");

    assert_eq!(count_all(&ctx, "groups").await, groups, "groups");
    assert_eq!(
        count_all(&ctx, "group_devices").await,
        members,
        "group_devices"
    );
    assert_eq!(count_all(&ctx, "audit_log").await, audit, "audit_log");
    assert_eq!(status_code_of(&ctx, dev).await, status_before);
    assert_eq!(device_version(&ctx, dev).await, version);
}

// ---- Test 6: отказы add_devices ----------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn anchor_add_devices_status_rules() {
    let (ctx, _dir) = make_test_ctx().await;
    let container = plain_group(&ctx, "container", "АРМ #40").await;
    let teardown = plain_group(&ctx, "teardown", "Разбор #5").await;

    let written_off = seed_device(&ctx, "Монитор #8", "INV-A-1", None).await;
    set_status_by_code(&ctx, written_off, "списано").await;
    // Своя деталь разбора: иначе первым сработает D-08, а не отказ по статусу.
    set_origin(&ctx, written_off, teardown).await;
    for g in [container, teardown] {
        let msg = validation(
            ctx.groups
                .add_devices(&admin(), g, vec![written_off])
                .await
                .expect_err("списано"),
            "device_ids",
        );
        assert!(
            msg.contains("Монитор #8") && msg.contains("«Списано»"),
            "{msg}"
        );
    }
    assert_eq!(member_rows(&ctx, written_off).await, 0);

    let to_write_off = seed_device(&ctx, "Планка памяти", "INV-A-2", None).await;
    set_status_by_code(&ctx, to_write_off, "на_списание").await;
    set_origin(&ctx, to_write_off, teardown).await;
    let msg = validation(
        ctx.groups
            .add_devices(&admin(), container, vec![to_write_off])
            .await
            .expect_err("контейнер"),
        "device_ids",
    );
    assert!(msg.contains("«На списание»"), "{msg}");
    assert_eq!(member_rows(&ctx, to_write_off).await, 0);
    // В разбор деталь «На списание» входит (D-18/D-35).
    ctx.groups
        .add_devices(&admin(), teardown, vec![to_write_off])
        .await
        .expect("в разбор можно");
    assert_eq!(member_rows(&ctx, to_write_off).await, 1);
}

// ---- Test 7: происхождение (D-33, D-08) -------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn anchor_origin_is_never_written_by_add_and_teardown_accepts_only_own() {
    let (ctx, _dir) = make_test_ctx().await;
    let a = plain_group(&ctx, "teardown", "Разбор #A").await;
    let b = plain_group(&ctx, "container", "АРМ #B").await;
    let c = plain_group(&ctx, "teardown", "Разбор #C").await;
    assert!(a != c);

    // Три различающихся фикстуры: пустой origin, чужой origin, свой origin.
    let empty = seed_device(&ctx, "Планка памяти", "INV-O-1", None).await;
    let foreign = seed_device(&ctx, "Жёсткий диск", "INV-O-2", None).await;
    set_origin(&ctx, foreign, c).await;
    let own = seed_device(&ctx, "Блок питания", "INV-O-3", None).await;
    set_origin(&ctx, own, a).await;

    for dev in [empty, foreign] {
        let msg = validation(
            ctx.groups
                .add_devices(&admin(), a, vec![dev])
                .await
                .expect_err("в разбор нельзя чужое"),
            "device_ids",
        );
        assert!(msg.contains("Создать деталь"), "{msg}");
        assert_eq!(member_rows(&ctx, dev).await, 0);
    }
    assert_eq!(origin_of(&ctx, empty).await, None, "отказ origin не пишет");
    assert_eq!(origin_of(&ctx, foreign).await, Some(c));

    ctx.groups
        .add_devices(&admin(), a, vec![own])
        .await
        .expect("своя деталь входит");
    assert_eq!(origin_of(&ctx, own).await, Some(a), "origin не меняется");

    ctx.groups
        .remove_devices(&admin(), a, vec![own])
        .await
        .expect("из A");
    assert_eq!(origin_of(&ctx, own).await, Some(a), "вывод не обнуляет");
    ctx.groups
        .add_devices(&admin(), b, vec![own])
        .await
        .expect("в контейнер B");
    assert_eq!(origin_of(&ctx, own).await, Some(a), "контейнер не меняет");
    ctx.groups
        .remove_devices(&admin(), b, vec![own])
        .await
        .expect("из B");
    // Чужой разбор C свою деталь A не принимает.
    validation(
        ctx.groups
            .add_devices(&admin(), c, vec![own])
            .await
            .expect_err("чужой разбор"),
        "device_ids",
    );
    assert_eq!(origin_of(&ctx, own).await, Some(a));
}

// ===========================================================================
// Phase 41.8 Plan 04: призраки в составе, отказ удаления разбора (D-10..D-14)
// ===========================================================================

/// Убрать деталь из состава без вывода через сервис (как при списании/удалении).
async fn drop_membership(ctx: &AppCtx, device: i64) {
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "DELETE FROM group_devices WHERE device_id = ?1",
                params![device],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("drop membership");
}

async fn soft_delete(ctx: &AppCtx, device: i64) {
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "UPDATE devices SET deleted_at_utc = 1700000900 WHERE id = ?1",
                params![device],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("soft delete");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn composition_ghosts_cover_removed_written_off_and_deleted_parts() {
    let (ctx, _dir) = make_test_ctx().await;
    let (g, anchor) = make_anchor_group(
        &ctx,
        "teardown",
        "Разбор #7",
        "Системный блок #4",
        "INV-GH-0",
        None,
    )
    .await;
    let other = plain_group(&ctx, "teardown", "Разбор #8").await;

    // Различающиеся фикстуры: выведенная, списанная, удалённая, чужой origin, живой член.
    let removed = seed_device(&ctx, "Планка памяти", "INV-GH-1", None).await;
    let written_off = seed_device(&ctx, "Блок питания", "INV-GH-2", None).await;
    let deleted = seed_device(&ctx, "Кулер", "INV-GH-3", None).await;
    let foreign = seed_device(&ctx, "Мышь", "INV-GH-4", None).await;
    let member = seed_device(&ctx, "Монитор", "INV-GH-5", None).await;
    for d in [removed, written_off, deleted, member] {
        set_origin(&ctx, d, g).await;
    }
    set_origin(&ctx, foreign, other).await;
    ctx.groups
        .add_devices(&admin(), g, vec![member])
        .await
        .expect("член с тем же origin");
    set_status_by_code(&ctx, written_off, "списано").await;
    soft_delete(&ctx, deleted).await;

    let before = ctx
        .groups
        .composition(&admin(), g)
        .await
        .expect("composition");
    let card_before = ctx.groups.get_group(&admin(), g).await.expect("card");

    let ids: Vec<i64> = before.ghosts.iter().map(|x| x.device_id).collect();
    // живые раньше удалённых, внутри — по имени: «Блок питания», «Планка памяти», «Кулер»
    assert_eq!(ids, vec![written_off, removed, deleted], "{ids:?}");
    assert!(!before.ghosts[0].is_deleted && before.ghosts[2].is_deleted);
    assert_eq!(before.ghosts[0].status_code.as_deref(), Some("списано"));
    assert!(!ids.contains(&member) && !ids.contains(&foreign) && !ids.contains(&anchor));
    let dev_ids: Vec<i64> = before.devices.iter().map(|x| x.device_id).collect();
    assert!(dev_ids.contains(&member), "живой член остался в devices");

    // D-11: призрак не меняет счётчики.
    let ghost_extra = seed_device(&ctx, "Корпус", "INV-GH-6", None).await;
    set_origin(&ctx, ghost_extra, g).await;
    let card_after = ctx.groups.get_group(&admin(), g).await.expect("card");
    assert_eq!(card_after.device_count, card_before.device_count);
    assert_eq!(
        card_after.direct_device_count,
        card_before.direct_device_count
    );

    // D-14: удалённая и списанная остаются; выведение в другую группу даёт current_group.
    let moved_to = plain_group(&ctx, "container", "АРМ #9").await;
    ctx.groups
        .add_devices(&admin(), moved_to, vec![removed])
        .await
        .expect("деталь ушла в АРМ");
    let after = ctx
        .groups
        .composition(&admin(), g)
        .await
        .expect("composition");
    let ghost = after
        .ghosts
        .iter()
        .find(|x| x.device_id == removed)
        .expect("призрак остался");
    assert_eq!(ghost.current_group_id, Some(moved_to));
    assert_eq!(ghost.current_group_name.as_deref(), Some("АРМ #9"));

    // Не-teardown: ghosts пуст, даже если у деталей есть origin.
    let arm = plain_group(&ctx, "container", "АРМ #10").await;
    let c = ctx.groups.composition(&admin(), arm).await.expect("c");
    assert!(c.ghosts.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn teardown_delete_refuses_with_removed_or_deleted_part_and_names_reason() {
    let (ctx, _dir) = make_test_ctx().await;
    let (g, _anchor) = make_anchor_group(
        &ctx,
        "teardown",
        "Разбор #7",
        "Системный блок #4",
        "INV-GD-0",
        None,
    )
    .await;
    let part = seed_device(&ctx, "Планка памяти", "INV-GD-1", None).await;
    set_origin(&ctx, part, g).await;
    ctx.groups
        .add_devices(&admin(), g, vec![part])
        .await
        .expect("add");

    // Деталь выведена, затем мягко удалена: живых деталей нет, остался только след.
    drop_membership(&ctx, part).await;
    soft_delete(&ctx, part).await;
    let msg = validation(
        ctx.groups
            .delete_group(&admin(), g)
            .await
            .expect_err("удалённая деталь держит запись"),
        "id",
    );
    assert!(
        msg.contains(
            "Группу «Разбор #7» нельзя удалить: с ней связаны 1 деталь, из них 1 выведены."
        ),
        "{msg}"
    );
    assert!(
        msg.contains("Запись разбора хранит происхождение деталей."),
        "{msg}"
    );
    assert!(group_alive(&ctx, g).await);

    // Пустой разбор (без деталей и призраков) удаляется.
    let (empty, _a) = make_anchor_group(
        &ctx,
        "teardown",
        "Разбор #8",
        "Системный блок #5",
        "INV-GD-2",
        None,
    )
    .await;
    ctx.groups
        .delete_group(&admin(), empty)
        .await
        .expect("пустой разбор удаляется");
    assert!(!group_alive(&ctx, empty).await);
}

// ===========================================================================
// Phase 41.8 Plan 05: списание устройства и группы (D-19..D-22, D-24, D-05)
// ===========================================================================

use trackly_app::dto::groups::{GroupWriteOffDeviceDto, GroupWriteOffGroupDto};

async fn write_off_audit_rows(ctx: &AppCtx, entity_id: i64) -> i64 {
    scalar_i64(
        ctx,
        "SELECT COUNT(*) FROM audit_log WHERE action = 'write_off' AND entity_id = ?1",
        entity_id,
    )
    .await
}

async fn wo_device(
    ctx: &AppCtx,
    device: i64,
    release: bool,
) -> Result<trackly_app::dto::groups::GroupWriteOffResultDto, AppError> {
    let version = device_version(ctx, device).await;
    ctx.groups
        .write_off_device(
            &admin(),
            GroupWriteOffDeviceDto {
                device_id: device,
                version,
                release_from_group: release,
            },
        )
        .await
}

async fn wo_group(
    ctx: &AppCtx,
    group: i64,
) -> Result<trackly_app::dto::groups::GroupWriteOffResultDto, AppError> {
    let version = scalar_i64(ctx, "SELECT version FROM groups WHERE id = ?1", group).await;
    ctx.groups
        .write_off_group(&admin(), GroupWriteOffGroupDto { id: group, version })
        .await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn write_off_device_status_version_and_rights() {
    let (ctx, _dir) = make_test_ctx().await;
    let free = seed_device(&ctx, "Монитор #2", "INV-W-1", None).await;
    let working = seed_device(&ctx, "Монитор #3", "INV-W-2", None).await;
    set_status_by_code(&ctx, free, "на_списание").await;

    // Не «На списание» — отказ по коду статуса.
    let msg = validation(
        wo_device(&ctx, working, false).await.expect_err("в работе"),
        "id",
    );
    assert!(msg.contains("Списать можно только"), "{msg}");
    assert_ne!(status_code_of(&ctx, working).await, "списано");

    // Устаревшая версия.
    let version = device_version(&ctx, free).await;
    let err = ctx
        .groups
        .write_off_device(
            &admin(),
            GroupWriteOffDeviceDto {
                device_id: free,
                version: version - 1,
                release_from_group: false,
            },
        )
        .await
        .expect_err("stale");
    assert!(
        matches!(err, AppError::OptimisticLockMismatch { .. }),
        "{err:?}"
    );

    // Employee.
    let employee = Identity {
        user_id: None,
        role: trackly_core::auth::Role::Employee,
    };
    let err = ctx
        .groups
        .write_off_device(
            &employee,
            GroupWriteOffDeviceDto {
                device_id: free,
                version,
                release_from_group: false,
            },
        )
        .await
        .expect_err("employee");
    assert!(matches!(err, AppError::Forbidden));
    assert_eq!(status_code_of(&ctx, free).await, "на_списание");

    wo_device(&ctx, free, false).await.expect("списано");
    assert_eq!(status_code_of(&ctx, free).await, "списано");
    assert_eq!(write_off_audit_rows(&ctx, free).await, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn write_off_member_stays_or_leaves_and_ghost_keeps_origin() {
    let (ctx, _dir) = make_test_ctx().await;
    let (sub, _anchor) = make_anchor_group(
        &ctx,
        "substitute",
        "Замена #1",
        "Системный блок #4",
        "INV-W-10",
        None,
    )
    .await;
    let stay = seed_device(&ctx, "Монитор #2", "INV-W-11", None).await;
    let leave = seed_device(&ctx, "Монитор #5", "INV-W-12", None).await;
    for d in [stay, leave] {
        ctx.groups
            .add_devices(&admin(), sub, vec![d])
            .await
            .expect("add");
        set_status_by_code(&ctx, d, "на_списание").await;
    }

    // D-24: остаётся прямым членом.
    wo_device(&ctx, stay, false).await.expect("stay");
    assert_eq!(status_code_of(&ctx, stay).await, "списано");
    assert_eq!(member_rows(&ctx, stay).await, 1, "остался в группе");

    // D-20: с выводом.
    wo_device(&ctx, leave, true).await.expect("leave");
    assert_eq!(status_code_of(&ctx, leave).await, "списано");
    assert_eq!(member_rows(&ctx, leave).await, 0);
    assert_eq!(write_off_audit_rows(&ctx, leave).await, 1);

    // Деталь разбора: вывод делает её призраком, origin цел.
    let (td, _a) = make_anchor_group(
        &ctx,
        "teardown",
        "Разбор #7",
        "Системный блок #6",
        "INV-W-13",
        None,
    )
    .await;
    let part = seed_device(&ctx, "Память 8 ГБ", "INV-W-14", None).await;
    set_origin(&ctx, part, td).await;
    ctx.groups
        .add_devices(&admin(), td, vec![part])
        .await
        .expect("add part");
    set_status_by_code(&ctx, part, "на_списание").await;
    wo_device(&ctx, part, true).await.expect("part off");
    assert_eq!(member_rows(&ctx, part).await, 0);
    assert_eq!(origin_of(&ctx, part).await, Some(td), "origin цел");
    let comp = ctx.groups.composition(&admin(), td).await.expect("comp");
    assert!(
        comp.ghosts.iter().any(|x| x.device_id == part),
        "деталь — призрак"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn write_off_device_refuses_anchor() {
    let (ctx, _dir) = make_test_ctx().await;
    let (_g, anchor) = make_anchor_group(
        &ctx,
        "substitute",
        "Замена #2",
        "Системный блок #7",
        "INV-W-20",
        None,
    )
    .await;
    set_status_by_code(&ctx, anchor, "на_списание").await;
    let msg = validation(
        wo_device(&ctx, anchor, false).await.expect_err("якорь"),
        "device_id",
    );
    assert!(msg.contains("спишите его через группу"), "{msg}");
    assert_eq!(status_code_of(&ctx, anchor).await, "на_списание");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn write_off_group_substitute_disbands_and_frees_members() {
    let (ctx, _dir) = make_test_ctx().await;
    let place = seed_place(&ctx, "Склад #1").await;
    let (g, anchor) = make_anchor_group(
        &ctx,
        "substitute",
        "Замена #3",
        "Системный блок #8",
        "INV-W-30",
        Some(place),
    )
    .await;
    let member = seed_device(&ctx, "Монитор #6", "INV-W-31", Some(place)).await;
    ctx.groups
        .add_devices(&admin(), g, vec![member])
        .await
        .expect("add");

    // Якорь не «На списание» — отказ.
    validation(wo_group(&ctx, g).await.expect_err("не на списание"), "id");
    assert!(group_alive(&ctx, g).await);

    set_status_by_code(&ctx, anchor, "на_списание").await;
    let status_before = status_code_of(&ctx, member).await;
    let res = wo_group(&ctx, g).await.expect("write off group");
    assert!(res.disbanded);
    assert_eq!(status_code_of(&ctx, anchor).await, "списано");
    assert!(!group_alive(&ctx, g).await);
    assert_eq!(member_rows(&ctx, member).await, 0);
    assert_eq!(status_code_of(&ctx, member).await, status_before);
    assert_eq!(
        scalar_i64(&ctx, "SELECT place_id FROM devices WHERE id = ?1", member).await,
        place
    );
    assert_eq!(write_off_audit_rows(&ctx, anchor).await, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn write_off_group_teardown_keeps_record_and_parts() {
    let (ctx, _dir) = make_test_ctx().await;
    let (g, anchor) = make_anchor_group(
        &ctx,
        "teardown",
        "Разбор #9",
        "Системный блок #9",
        "INV-W-40",
        None,
    )
    .await;
    let part = seed_device(&ctx, "SSD 256 ГБ", "INV-W-41", None).await;
    set_origin(&ctx, part, g).await;
    ctx.groups
        .add_devices(&admin(), g, vec![part])
        .await
        .expect("add part");
    set_status_by_code(&ctx, anchor, "на_списание").await;

    let res = wo_group(&ctx, g).await.expect("write off donor");
    assert!(!res.disbanded);
    assert_eq!(status_code_of(&ctx, anchor).await, "списано");
    assert!(group_alive(&ctx, g).await);
    assert_eq!(member_rows(&ctx, part).await, 1, "деталь на месте");
    assert_eq!(origin_of(&ctx, part).await, Some(g));
    // D-13: удаление записи разбора по-прежнему отказывает.
    validation(
        ctx.groups
            .delete_group(&admin(), g)
            .await
            .expect_err("нельзя удалить"),
        "id",
    );
    assert_eq!(write_off_audit_rows(&ctx, anchor).await, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn write_off_group_container_and_stale_version_rejected() {
    let (ctx, _dir) = make_test_ctx().await;
    let c = plain_group(&ctx, "container", "АРМ #1").await;
    let msg = validation(wo_group(&ctx, c).await.expect_err("container"), "id");
    assert!(msg.contains("нет якоря"), "{msg}");

    let (g, anchor) = make_anchor_group(
        &ctx,
        "substitute",
        "Замена #4",
        "Системный блок #10",
        "INV-W-50",
        None,
    )
    .await;
    set_status_by_code(&ctx, anchor, "на_списание").await;
    let err = ctx
        .groups
        .write_off_group(&admin(), GroupWriteOffGroupDto { id: g, version: 0 })
        .await
        .expect_err("stale");
    assert!(
        matches!(err, AppError::OptimisticLockMismatch { .. }),
        "{err:?}"
    );
    assert!(group_alive(&ctx, g).await);
    assert_eq!(status_code_of(&ctx, anchor).await, "на_списание");
}

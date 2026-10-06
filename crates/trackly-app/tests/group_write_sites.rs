//! Phase 41 Plan 14: write-site'ы `devices.place_id`, закрытые планом 14.
//!
//! Инвентарь ОТ СЕРВЕРНЫХ МУТАЦИЙ (не от экранов; урок фазы 40.1):
//!   S1  `DeviceService::update`                      — guard «место задаёт группа» (этот файл)
//!   S2  `PlaceService::move_subtree_contents`        — план 15
//!   S3-S7 `ActService` (create/update/do_return/update_return/undo) — план 16 (этот файл)
//!   S8  `DeviceService::delete_soft`                 — освобождение членства (этот файл)
//!   S9  `cartridges_sqlite.rs` backfill места принтера — сознательно БЕЗ guard'а (этот файл)
//!   S10 create / bulk / CSV                          — без изменений: новое устройство не член группы
//!
//! Таблица-драйвер: каждый сценарий — строка `Scenario`; фикстуры расходятся
//! (повтор текущего места против реальной смены), поэтому тест не вакуумен.
//! Реальный `AppCtx` на временном каталоге; имена вымышленные.
//!
//! Префиксы: `s1_table_`, `s1_http_`, `s1_tauri_path_`, `s8_`, `s9_`, `s3_`..`s7_`.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use rusqlite::params;
use serde_json::json;
use time::{Duration, OffsetDateTime};
use tower::ServiceExt;
use tower_sessions::session::{Id, Record};
use tower_sessions::SessionStore;

use trackly_app::context::AppCtx;
use trackly_app::dto::act::{
    ActCreateDto, ActDto, ActItemNewDto, ActNumberEditInput, ActReturnDto, ActReturnItemDto,
    ActUpdateDto, ActUpdateItemDto, ActUpdateReturnDto,
};
use trackly_app::dto::auth::UserNew;
use trackly_app::dto::cartridge::{
    CartridgeCreateDto, CartridgeModelCreateDto, CartridgeTransitionPayload,
};
use trackly_app::dto::device::DevicePatch;
use trackly_app::dto::number_template::NumberFieldInput;
use trackly_app::http::auth::SessionIdentity;
use trackly_app::http::build_router;
use trackly_app::server::rusqlite_session_store::RusqliteSessionStore;
use trackly_app::tauri_cmds::devices::build_devices_update;
use trackly_core::auth::{Identity, Role};
use trackly_core::error::AppError;
use trackly_infra::error_conversions::map_rusqlite;

// ---------------------------------------------------------------------------
// Фикстуры
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
    let log_guard = log_guard.expect("log guard");
    let ctx = AppCtx::build(paths, config, log_guard)
        .await
        .expect("build ctx");
    (ctx, dir)
}

fn admin() -> Identity {
    Identity::trusted_admin()
}

fn int(v: i64) -> rusqlite::types::Value {
    rusqlite::types::Value::Integer(v)
}

async fn scalar_i64(ctx: &AppCtx, sql: &'static str, p: Vec<rusqlite::types::Value>) -> i64 {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(sql, rusqlite::params_from_iter(p), |r| r.get::<_, i64>(0))
                .map_err(map_rusqlite)
        })
        .await
        .expect("scalar query")
}

async fn opt_i64(ctx: &AppCtx, sql: &'static str, p: Vec<rusqlite::types::Value>) -> Option<i64> {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(sql, rusqlite::params_from_iter(p), |r| {
                r.get::<_, Option<i64>>(0)
            })
            .map_err(map_rusqlite)
        })
        .await
        .expect("opt query")
}

async fn text_of(ctx: &AppCtx, sql: &'static str, p: Vec<rusqlite::types::Value>) -> String {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(sql, rusqlite::params_from_iter(p), |r| {
                r.get::<_, String>(0)
            })
            .map_err(map_rusqlite)
        })
        .await
        .expect("text query")
}

async fn device_place(ctx: &AppCtx, id: i64) -> Option<i64> {
    opt_i64(
        ctx,
        "SELECT place_id FROM devices WHERE id = ?1",
        vec![int(id)],
    )
    .await
}

async fn device_version(ctx: &AppCtx, id: i64) -> i64 {
    scalar_i64(
        ctx,
        "SELECT version FROM devices WHERE id = ?1",
        vec![int(id)],
    )
    .await
}

async fn device_name(ctx: &AppCtx, id: i64) -> String {
    text_of(ctx, "SELECT name FROM devices WHERE id = ?1", vec![int(id)]).await
}

async fn member_rows(ctx: &AppCtx, device_id: i64) -> i64 {
    scalar_i64(
        ctx,
        "SELECT COUNT(*) FROM group_devices WHERE device_id = ?1",
        vec![int(device_id)],
    )
    .await
}

async fn release_audit_count(ctx: &AppCtx, device_id: i64) -> i64 {
    scalar_i64(
        ctx,
        "SELECT COUNT(*) FROM audit_log \
         WHERE action = 'custom:group_member_released' AND entity_id = ?1",
        vec![int(device_id)],
    )
    .await
}

async fn type_id_by_code(ctx: &AppCtx, code: &str) -> i64 {
    ctx.group_types
        .list_types(&admin(), false)
        .await
        .expect("list_types")
        .into_iter()
        .find(|t| t.code == code)
        .unwrap_or_else(|| panic!("тип {code} не найден"))
        .id
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

async fn seed_group(ctx: &AppCtx, type_id: i64, name: &str, seq: i64, place: Option<i64>) -> i64 {
    let name = name.to_string();
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO groups (type_id, name, seq, place_id, parent_group_id, \
                 created_at_utc, updated_at_utc, version) \
                 VALUES (?1, ?2, ?3, ?4, NULL, 1700000000, 1700000000, 1)",
                params![type_id, name, seq, place],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed group")
}

/// Живое устройство (`device_type` 1 — обычное, 2 — принтер) и, при `group_id`,
/// сразу в составе группы.
async fn seed_device(
    ctx: &AppCtx,
    device_type: i64,
    name: &str,
    inv: &str,
    place: Option<i64>,
    group_id: Option<i64>,
) -> i64 {
    let (name, inv) = (name.to_string(), inv.to_string());
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO devices (type_id, name, inventory_number, place_id, status_id, \
                 created_at_utc, updated_at_utc, version) \
                 VALUES (?1, ?2, ?3, ?4, 1, 1700000000, 1700000000, 1)",
                params![device_type, name, inv, place],
            )
            .map_err(map_rusqlite)?;
            let id = conn.last_insert_rowid();
            if let Some(g) = group_id {
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

fn validation_message(err: AppError) -> String {
    match err {
        AppError::Validation { field, message } => {
            assert_eq!(field, "place_id", "поле ошибки");
            message
        }
        other => panic!("ожидали Validation, получили {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// S1: таблица-драйвер сервиса
// ---------------------------------------------------------------------------

/// Какое значение `place_id` приходит в патче.
#[derive(Clone, Copy, Debug)]
enum PlacePatch {
    /// поле не передано
    Absent,
    /// ТЕКУЩЕЕ место устройства (повтор: форма шлёт его при каждом сохранении)
    Current,
    /// другое место («Склад Б»)
    Other,
    /// очистка (null)
    Clear,
}

#[derive(Clone, Copy, Debug)]
enum Membership {
    /// не член ни одной группы
    None,
    /// член группы с местом «Склад А»
    GroupWithPlace,
    /// член группы БЕЗ места (D-21)
    GroupWithoutPlace,
    /// был членом группы с местом, затем выведен через `remove_devices`
    ReleasedFromGroupWithPlace,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Expect {
    Ok,
    Reject,
}

struct Scenario {
    name: &'static str,
    membership: Membership,
    /// текущее место устройства: `true` — «Склад А», `false` — «Склад Б»
    /// (расхождение с местом группы возможно, например в ремонтной фазе)
    device_at_a: bool,
    place: PlacePatch,
    rename: bool,
    expect: Expect,
}

const SCENARIOS: &[Scenario] = &[
    Scenario {
        name: "реальная смена места члена группы с местом",
        membership: Membership::GroupWithPlace,
        device_at_a: true,
        place: PlacePatch::Other,
        rename: false,
        expect: Expect::Reject,
    },
    Scenario {
        name: "повтор текущего места вместе с переименованием",
        membership: Membership::GroupWithPlace,
        device_at_a: true,
        place: PlacePatch::Current,
        rename: true,
        expect: Expect::Ok,
    },
    Scenario {
        name: "очистка места члена группы с местом",
        membership: Membership::GroupWithPlace,
        device_at_a: true,
        place: PlacePatch::Clear,
        rename: false,
        expect: Expect::Reject,
    },
    Scenario {
        name: "переименование без поля места",
        membership: Membership::GroupWithPlace,
        device_at_a: true,
        place: PlacePatch::Absent,
        rename: true,
        expect: Expect::Ok,
    },
    Scenario {
        name: "D-21: группа без места не запирает",
        membership: Membership::GroupWithoutPlace,
        device_at_a: true,
        place: PlacePatch::Other,
        rename: false,
        expect: Expect::Ok,
    },
    Scenario {
        name: "после вывода из состава смена проходит",
        membership: Membership::ReleasedFromGroupWithPlace,
        device_at_a: true,
        place: PlacePatch::Other,
        rename: false,
        expect: Expect::Ok,
    },
    Scenario {
        name: "контроль: устройство вне групп",
        membership: Membership::None,
        device_at_a: true,
        place: PlacePatch::Other,
        rename: false,
        expect: Expect::Ok,
    },
    Scenario {
        name: "место устройства расходится с местом группы: повтор текущего проходит",
        membership: Membership::GroupWithPlace,
        device_at_a: false,
        place: PlacePatch::Current,
        rename: true,
        expect: Expect::Ok,
    },
    Scenario {
        name: "место устройства расходится с местом группы: смена на место группы отклоняется",
        membership: Membership::GroupWithPlace,
        device_at_a: false,
        place: PlacePatch::Other,
        rename: false,
        expect: Expect::Reject,
    },
];

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn s1_table_service_update_guard() {
    let (ctx, _dir) = make_test_ctx().await;
    let arm = type_id_by_code(&ctx, "workstation").await;
    let place_a = seed_place(&ctx, "Склад А").await;
    let place_b = seed_place(&ctx, "Склад Б").await;

    let mut rejected = 0;
    let mut accepted = 0;
    for (i, sc) in SCENARIOS.iter().enumerate() {
        let seq = i as i64 + 1;
        let group_name = format!("АРМ #{seq}");
        let (group_id, group_name_shown) = match sc.membership {
            Membership::None => (None, String::new()),
            Membership::GroupWithPlace | Membership::ReleasedFromGroupWithPlace => (
                Some(seed_group(&ctx, arm, &group_name, seq, Some(place_a)).await),
                group_name.clone(),
            ),
            Membership::GroupWithoutPlace => (
                Some(seed_group(&ctx, arm, &group_name, seq, None).await),
                group_name.clone(),
            ),
        };
        let current_place = if sc.device_at_a { place_a } else { place_b };
        let other_place = if sc.device_at_a { place_b } else { place_a };
        let dev = seed_device(
            &ctx,
            1,
            "Ноутбук 1",
            &format!("INV-S1-{seq}"),
            Some(current_place),
            group_id,
        )
        .await;
        if let Membership::ReleasedFromGroupWithPlace = sc.membership {
            let n = ctx
                .groups
                .remove_devices(&admin(), group_id.unwrap(), vec![dev])
                .await
                .expect("remove_devices");
            assert_eq!(n, 1, "{}: устройство выведено", sc.name);
        }

        let place_before = device_place(&ctx, dev).await;
        let version_before = device_version(&ctx, dev).await;
        let new_name = format!("Ноутбук 1 (правка {seq})");
        let patch = DevicePatch {
            name: sc.rename.then(|| new_name.clone()),
            place_id: match sc.place {
                PlacePatch::Absent => None,
                PlacePatch::Current => Some(Some(current_place)),
                PlacePatch::Other => Some(Some(other_place)),
                PlacePatch::Clear => Some(None),
            },
            ..Default::default()
        };

        let res = ctx
            .devices
            .update(&admin(), dev, version_before, patch)
            .await;
        match sc.expect {
            Expect::Reject => {
                rejected += 1;
                let msg = validation_message(res.expect_err(sc.name));
                assert!(
                    msg.contains(&group_name_shown) && msg.contains("Место задаётся группой"),
                    "{}: текст ошибки с именем группы, получили «{msg}»",
                    sc.name
                );
                assert_eq!(device_place(&ctx, dev).await, place_before, "{}", sc.name);
                assert_eq!(
                    device_version(&ctx, dev).await,
                    version_before,
                    "{}: версия не растёт при отказе",
                    sc.name
                );
            }
            Expect::Ok => {
                accepted += 1;
                res.unwrap_or_else(|e| panic!("{}: ожидали успех, получили {e:?}", sc.name));
                let expected_place = match sc.place {
                    PlacePatch::Absent | PlacePatch::Current => place_before,
                    PlacePatch::Other => Some(other_place),
                    PlacePatch::Clear => None,
                };
                assert_eq!(device_place(&ctx, dev).await, expected_place, "{}", sc.name);
                if sc.rename {
                    assert_eq!(device_name(&ctx, dev).await, new_name, "{}", sc.name);
                }
                assert_eq!(
                    device_version(&ctx, dev).await,
                    version_before + 1,
                    "{}",
                    sc.name
                );
            }
        }
    }
    // невакуумность: таблица содержит и отказы, и успехи
    assert!(rejected >= 3 && accepted >= 5, "{rejected}/{accepted}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn s1_table_manager_is_also_guarded() {
    let (ctx, _dir) = make_test_ctx().await;
    let arm = type_id_by_code(&ctx, "workstation").await;
    let place_a = seed_place(&ctx, "Склад А").await;
    let place_b = seed_place(&ctx, "Склад Б").await;
    let g = seed_group(&ctx, arm, "АРМ #3", 3, Some(place_a)).await;
    let dev = seed_device(&ctx, 1, "Ноутбук 1", "INV-S1-M", Some(place_a), Some(g)).await;
    let manager = Identity {
        user_id: None,
        role: Role::Manager,
    };
    let err = ctx
        .devices
        .update(
            &manager,
            dev,
            1,
            DevicePatch {
                place_id: Some(Some(place_b)),
                ..Default::default()
            },
        )
        .await
        .expect_err("manager обходить запрет не может");
    validation_message(err);
    assert_eq!(device_place(&ctx, dev).await, Some(place_a));
}

// ---------------------------------------------------------------------------
// S8: мягкое удаление освобождает членство
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn s8_delete_soft_releases_membership() {
    let (ctx, _dir) = make_test_ctx().await;
    let arm = type_id_by_code(&ctx, "workstation").await;
    let place_a = seed_place(&ctx, "Склад А").await;
    let g = seed_group(&ctx, arm, "АРМ #3", 3, Some(place_a)).await;
    let keep = seed_device(&ctx, 1, "Монитор 1", "INV-S8-K", Some(place_a), Some(g)).await;
    let gone = seed_device(&ctx, 1, "Ноутбук 1", "INV-S8-G", Some(place_a), Some(g)).await;

    let comp = ctx
        .groups
        .composition(&admin(), g)
        .await
        .expect("composition");
    assert_eq!(comp.devices.len(), 2, "до удаления в составе двое");
    assert_eq!(release_audit_count(&ctx, gone).await, 0);

    let version = device_version(&ctx, gone).await;
    ctx.devices
        .delete_soft(gone, version)
        .await
        .expect("delete_soft");

    assert_eq!(
        member_rows(&ctx, gone).await,
        0,
        "строка group_devices исчезла"
    );
    assert_eq!(member_rows(&ctx, keep).await, 1, "соседний член не тронут");
    let comp = ctx
        .groups
        .composition(&admin(), g)
        .await
        .expect("composition");
    let ids: Vec<i64> = comp.devices.iter().map(|d| d.device_id).collect();
    assert_eq!(ids, vec![keep], "удалённого устройства в составе нет");
    assert_eq!(
        release_audit_count(&ctx, gone).await,
        1,
        "audit о выводе записан"
    );
    assert_eq!(
        device_place(&ctx, gone).await,
        Some(place_a),
        "место удалённого не меняется"
    );
}

/// Ветка «членства не было» в `release_device_in_tx` (долг плана 41-11): удаление
/// устройства вне групп проходит без ошибки и без записи о выводе из состава.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn s8_delete_soft_without_membership_writes_no_release() {
    let (ctx, _dir) = make_test_ctx().await;
    let place_a = seed_place(&ctx, "Склад А").await;
    let dev = seed_device(&ctx, 1, "Ноутбук 2", "INV-S8-N", Some(place_a), None).await;
    let version = device_version(&ctx, dev).await;
    ctx.devices
        .delete_soft(dev, version)
        .await
        .expect("delete_soft");

    assert!(
        scalar_i64(
            &ctx,
            "SELECT deleted_at_utc IS NOT NULL FROM devices WHERE id = ?1",
            vec![int(dev)],
        )
        .await
            == 1,
        "устройство удалено"
    );
    assert_eq!(
        release_audit_count(&ctx, dev).await,
        0,
        "записи о выводе нет"
    );
    assert_eq!(
        scalar_i64(
            &ctx,
            "SELECT COUNT(*) FROM audit_log WHERE entity_type='device' AND entity_id=?1 \
             AND action='delete'",
            vec![int(dev)],
        )
        .await,
        1,
        "обычный audit удаления записан"
    );
}

// ---------------------------------------------------------------------------
// S9: установка картриджа не трогает место принтера-члена группы с местом
// ---------------------------------------------------------------------------

fn number_input(value: &str) -> NumberFieldInput {
    NumberFieldInput {
        value: value.to_string(),
        template_id: None,
        confirm_mismatch: false,
        confirm_script_mix: false,
    }
}

async fn install_cartridge(ctx: &AppCtx, code: &str, printer: i64, cart_place: i64) {
    let model = ctx
        .cartridges
        .model_create(CartridgeModelCreateDto {
            brand: "HP".into(),
            model: format!("MODEL-{code}"),
            kind_id: 1,
            color: Some("Чёрный".into()),
            notes: None,
            compatibility: vec![],
        })
        .await
        .expect("model_create")
        .id;
    let cart = ctx
        .cartridges
        .create(CartridgeCreateDto {
            model_id: model,
            number_input: number_input(code),
            state_id: Some(1),
            place_id: None,
            notes: None,
        })
        .await
        .expect("create cartridge")
        .expect_created("create cartridge");
    ctx.cartridges
        .transition(
            &admin(),
            CartridgeTransitionPayload::Install {
                cartridge_id: cart.id,
                version: cart.version,
                date_utc: 1_700_000_000,
                given_by_name: "Иванов И.И.".into(),
                given_to_name: "Петров П.П.".into(),
                place_id: Some(cart_place),
                printer_device_id: Some(printer),
                previous_cartridge_state_id: None,
                previous_cartridge_place_id: None,
            },
        )
        .await
        .expect("transition Install");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn s9_install_does_not_move_member_printer_with_group_place() {
    let (ctx, _dir) = make_test_ctx().await;
    let arm = type_id_by_code(&ctx, "workstation").await;
    let place_a = seed_place(&ctx, "Склад А").await;
    let place_b = seed_place(&ctx, "Склад Б").await;

    // принтер-член группы С МЕСТОМ: backfill не срабатывает (place_id NOT NULL)
    let g_with = seed_group(&ctx, arm, "АРМ #3", 3, Some(place_a)).await;
    let member = seed_device(
        &ctx,
        2,
        "Принтер 1",
        "INV-S9-1",
        Some(place_a),
        Some(g_with),
    )
    .await;
    let version_before = device_version(&ctx, member).await;
    install_cartridge(&ctx, "C-GWS-0001", member, place_b).await;
    assert_eq!(
        device_place(&ctx, member).await,
        Some(place_a),
        "место группы цело"
    );
    assert_eq!(
        device_version(&ctx, member).await,
        version_before,
        "строка принтера не переписывалась"
    );

    // контроль: принтер вне групп без места — backfill РАБОТАЕТ (тест не вакуумен)
    let free = seed_device(&ctx, 2, "Принтер 2", "INV-S9-2", None, None).await;
    install_cartridge(&ctx, "C-GWS-0002", free, place_b).await;
    assert_eq!(
        device_place(&ctx, free).await,
        Some(place_b),
        "контроль: место заполнено"
    );

    // D-21: группа БЕЗ места — запрет «спит», принтер без места заполняется как обычный
    let g_without = seed_group(&ctx, arm, "АРМ #4", 4, None).await;
    let dormant = seed_device(&ctx, 2, "Принтер 3", "INV-S9-3", None, Some(g_without)).await;
    install_cartridge(&ctx, "C-GWS-0003", dormant, place_b).await;
    assert_eq!(
        device_place(&ctx, dormant).await,
        Some(place_b),
        "D-21: у группы без места место принтера заполняется"
    );
}

// ---------------------------------------------------------------------------
// Второй транспорт: HTTP `devices_update` / `devices_delete` и Tauri-путь
// ---------------------------------------------------------------------------

// Клон хелперов role_endpoint_matrix.rs (там они приватны).
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

struct HttpEnv {
    ctx: AppCtx,
    _dir: tempfile::TempDir,
    admin_cookie: String,
    manager_cookie: String,
    manager: Identity,
    arm: i64,
    place_a: i64,
    place_b: i64,
}

async fn http_env() -> HttpEnv {
    let (ctx, dir) = make_test_ctx().await;
    let mut ids = Vec::new();
    for (login, name, role, role_str) in [
        ("gws_admin", "Иванов И.И.", Role::Admin, "admin"),
        ("gws_manager", "Петров П.П.", Role::Manager, "manager"),
    ] {
        let dto = ctx
            .auth
            .create_user(
                UserNew {
                    login: login.to_string(),
                    full_name: name.to_string(),
                    password: "password123".to_string(),
                    role: role_str.to_string(),
                    email: None,
                },
                &Identity::trusted_admin(),
            )
            .await
            .expect("create user");
        ids.push((dto.id, role));
    }
    let store = RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone());
    let admin_cookie = create_session_cookie(&store, ids[0].0, Role::Admin).await;
    let manager_cookie = create_session_cookie(&store, ids[1].0, Role::Manager).await;
    let arm = type_id_by_code(&ctx, "workstation").await;
    let place_a = seed_place(&ctx, "Склад А").await;
    let place_b = seed_place(&ctx, "Склад Б").await;
    HttpEnv {
        ctx,
        _dir: dir,
        admin_cookie,
        manager_cookie,
        manager: Identity {
            user_id: Some(ids[1].0),
            role: Role::Manager,
        },
        arm,
        place_a,
        place_b,
    }
}

fn http_app(ctx: &AppCtx) -> axum::Router {
    build_router(
        ctx,
        RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone()),
    )
}

async fn http_update(
    env: &HttpEnv,
    cookie: &str,
    id: i64,
    patch: serde_json::Value,
) -> (StatusCode, String) {
    let version = device_version(&env.ctx, id).await;
    post_json(
        http_app(&env.ctx),
        "/api/v1/devices_update",
        json!({ "id": id, "version": version, "patch": patch }),
        cookie,
    )
    .await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn s1_http_update_rejects_member_but_accepts_outsider_and_dormant_group() {
    let env = http_env().await;
    let ctx = &env.ctx;
    for (who, cookie) in [
        ("admin", env.admin_cookie.clone()),
        ("manager", env.manager_cookie.clone()),
    ] {
        let inv = |tag: &str| format!("INV-HTTP-{who}-{tag}");
        let g = seed_group(
            ctx,
            env.arm,
            &format!("АРМ #{who}"),
            if who == "admin" { 1 } else { 2 },
            Some(env.place_a),
        )
        .await;
        let member = seed_device(ctx, 1, "Ноутбук 1", &inv("m"), Some(env.place_a), Some(g)).await;
        let outsider = seed_device(ctx, 1, "Ноутбук 2", &inv("o"), Some(env.place_a), None).await;

        // отказ для члена группы с местом: 400 + текст, БД не изменилась
        let (place_before, version_before) = (
            device_place(ctx, member).await,
            device_version(ctx, member).await,
        );
        let (status, body) =
            http_update(&env, &cookie, member, json!({ "place_id": env.place_b })).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{who}: {body}");
        assert!(body.contains("задаётся группой"), "{who}: {body}");
        assert_eq!(device_place(ctx, member).await, place_before, "{who}");
        assert_eq!(device_version(ctx, member).await, version_before, "{who}");
        assert_eq!(device_place(ctx, member).await, Some(env.place_a), "{who}");

        // очистка места — тоже реальная смена
        let (status, _) = http_update(&env, &cookie, member, json!({ "place_id": null })).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{who}: очистка");
        assert_eq!(device_place(ctx, member).await, Some(env.place_a), "{who}");

        // контроль: ТОТ ЖЕ маршрут и ТОТ ЖЕ патч для устройства вне группы -> 200 и смена
        let (status, body) =
            http_update(&env, &cookie, outsider, json!({ "place_id": env.place_b })).await;
        assert_eq!(status, StatusCode::OK, "{who}: контроль {body}");
        assert_eq!(
            device_place(ctx, outsider).await,
            Some(env.place_b),
            "{who}"
        );

        // повтор текущего места вместе с переименованием у члена группы -> 200
        let (status, body) = http_update(
            &env,
            &cookie,
            member,
            json!({ "place_id": env.place_a, "name": "Ноутбук 1 (правка)" }),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{who}: повтор {body}");
        assert_eq!(
            device_name(ctx, member).await,
            "Ноутбук 1 (правка)",
            "{who}"
        );
        assert_eq!(device_place(ctx, member).await, Some(env.place_a), "{who}");
    }

    // D-21: член группы БЕЗ места -> 200
    let dormant_group = seed_group(ctx, env.arm, "АРМ #9", 9, None).await;
    let dormant = seed_device(
        ctx,
        1,
        "Ноутбук 3",
        "INV-HTTP-dormant",
        Some(env.place_a),
        Some(dormant_group),
    )
    .await;
    let (status, body) = http_update(
        &env,
        &env.admin_cookie,
        dormant,
        json!({ "place_id": env.place_b }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "D-21: {body}");
    assert_eq!(device_place(ctx, dormant).await, Some(env.place_b));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn s1_http_update_passes_after_release_by_service() {
    let env = http_env().await;
    let ctx = &env.ctx;
    let g = seed_group(ctx, env.arm, "АРМ #3", 3, Some(env.place_a)).await;
    let member = seed_device(
        ctx,
        1,
        "Ноутбук 1",
        "INV-HTTP-rel",
        Some(env.place_a),
        Some(g),
    )
    .await;

    let (status, _) = http_update(
        &env,
        &env.admin_cookie,
        member,
        json!({ "place_id": env.place_b }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "до вывода из состава");
    assert_eq!(device_place(ctx, member).await, Some(env.place_a));

    // вывод — сервисом, не HTTP-маршрутом groups_remove_devices (его создаёт план 13)
    ctx.groups
        .remove_devices(&admin(), g, vec![member])
        .await
        .expect("remove_devices");

    let (status, body) = http_update(
        &env,
        &env.admin_cookie,
        member,
        json!({ "place_id": env.place_b }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "после вывода: {body}");
    assert_eq!(device_place(ctx, member).await, Some(env.place_b));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn s1_tauri_path_build_devices_update_rejects_member() {
    let env = http_env().await;
    let ctx = &env.ctx;
    let g = seed_group(ctx, env.arm, "АРМ #3", 3, Some(env.place_a)).await;
    let member = seed_device(
        ctx,
        1,
        "Ноутбук 1",
        "INV-TAURI-1",
        Some(env.place_a),
        Some(g),
    )
    .await;
    let outsider = seed_device(ctx, 1, "Ноутбук 2", "INV-TAURI-2", Some(env.place_a), None).await;
    let patch = || DevicePatch {
        place_id: Some(Some(env.place_b)),
        ..Default::default()
    };

    for caller in [admin(), env.manager.clone()] {
        let version = device_version(ctx, member).await;
        let err = build_devices_update(ctx, &caller, member, version, patch())
            .await
            .expect_err("член группы с местом");
        let msg = validation_message(err);
        assert!(msg.contains("АРМ #3"), "{msg}");
        assert_eq!(device_place(ctx, member).await, Some(env.place_a));
    }

    // контроль: тот же вызов для устройства вне группы проходит
    let version = device_version(ctx, outsider).await;
    build_devices_update(ctx, &admin(), outsider, version, patch())
        .await
        .expect("вне группы");
    assert_eq!(device_place(ctx, outsider).await, Some(env.place_b));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn s8_http_delete_releases_membership() {
    let env = http_env().await;
    let ctx = &env.ctx;
    let g = seed_group(ctx, env.arm, "АРМ #3", 3, Some(env.place_a)).await;
    let keep = seed_device(
        ctx,
        1,
        "Монитор 1",
        "INV-HTTP-k",
        Some(env.place_a),
        Some(g),
    )
    .await;
    let gone = seed_device(
        ctx,
        1,
        "Ноутбук 1",
        "INV-HTTP-g",
        Some(env.place_a),
        Some(g),
    )
    .await;

    let version = device_version(ctx, gone).await;
    let (status, body) = post_json(
        http_app(ctx),
        "/api/v1/devices_delete",
        json!({ "id": gone, "version": version }),
        &env.admin_cookie,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    // состав читаем сервисом: HTTP-маршрут groups_composition создаёт план 13
    let comp = ctx
        .groups
        .composition(&admin(), g)
        .await
        .expect("composition");
    let ids: Vec<i64> = comp.devices.iter().map(|d| d.device_id).collect();
    assert_eq!(ids, vec![keep], "удалённого устройства в составе нет");
    assert_eq!(member_rows(ctx, gone).await, 0);
    assert_eq!(release_audit_count(ctx, gone).await, 1);
}

// ===========================================================================
// Plan 41-16: S3-S7 — акты выводят устройство из состава группы (D-22)
// ===========================================================================
//
// Фикстуры различают «до/после»: у устройства-члена место (Склад А) ОТЛИЧНО от
// места акта (Склад Б), остальные члены группы остаются на месте группы.
// Для restore-путей («убрали из акта», правка возврата, undo) группа «Склад Б»
// принимает устройство ПОСЛЕ акта (так и выглядит реальный порядок: устройство
// уже на месте акта, в группу его добавили позже), а снимок хранит третье место.

/// Выданное в акте устройство, ставшее членом группы после акта (прямая вставка
/// в `group_devices`; место устройства не меняется).
async fn join_group(ctx: &AppCtx, device_id: i64, group_id: i64) {
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO group_devices (device_id, group_id, added_at_utc) \
                 VALUES (?1, ?2, 1700000100)",
                params![device_id, group_id],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("join group");
}

fn act_number(value: &str) -> NumberFieldInput {
    NumberFieldInput {
        value: value.into(),
        template_id: None,
        confirm_mismatch: false,
        confirm_script_mix: false,
    }
}

async fn handover_act(ctx: &AppCtx, number: &str, devices: &[i64], place: i64) -> ActDto {
    ctx.acts
        .create(
            &admin(),
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
            },
        )
        .await
        .expect("create handover")
        .expect_created("create handover")
}

async fn update_handover(ctx: &AppCtx, act: &ActDto, devices: &[i64]) -> ActDto {
    ctx.acts
        .update(
            &admin(),
            ActUpdateDto {
                id: act.id,
                expected_version: act.version,
                number_input: ActNumberEditInput {
                    value: act.number_raw.clone(),
                    confirm_script_mix: false,
                },
                giver_name: act.giver_name.clone(),
                receiver_name: act.receiver_name.clone(),
                place_id: act.place_id,
                notes: act.notes.clone(),
                deadline_utc: act.deadline_utc,
                handover_date_utc: None,
                items: devices
                    .iter()
                    .map(|&id| ActUpdateItemDto {
                        device_id: id,
                        complectation_at_time: None,
                    })
                    .collect(),
            },
        )
        .await
        .expect("update handover")
        .expect_created("update handover")
}

fn return_items(handover: &ActDto, devices: &[i64]) -> Vec<ActReturnItemDto> {
    devices
        .iter()
        .map(|&did| {
            let it = handover
                .items
                .iter()
                .find(|i| i.device_id == did)
                .expect("устройство есть в акте выдачи");
            ActReturnItemDto {
                act_item_id: it.id,
                device_id: did,
                device_ids: vec![did],
                quantity: 1,
                condition_override: None,
                place_id_override: None,
            }
        })
        .collect()
}

async fn return_act(
    ctx: &AppCtx,
    handover: &ActDto,
    devices: &[i64],
    condition: &str,
    place: i64,
) -> ActDto {
    ctx.acts
        .do_return(
            &admin(),
            handover.id,
            ActReturnDto {
                bulk_condition: Some(condition.into()),
                bulk_place_id: Some(place),
                apply_to_all: true,
                giver_name: None,
                receiver_name: None,
                handover_date_utc: None,
                items: return_items(handover, devices),
            },
        )
        .await
        .expect("do_return")
}

async fn update_return_act(
    ctx: &AppCtx,
    handover: &ActDto,
    ret: &ActDto,
    devices: &[i64],
    condition: &str,
    place: i64,
) -> ActDto {
    ctx.acts
        .update_return(
            &admin(),
            ActUpdateReturnDto {
                id: ret.id,
                expected_version: ret.version,
                giver_name: ret.giver_name.clone(),
                receiver_name: ret.receiver_name.clone(),
                place_id: ret.place_id,
                notes: None,
                deadline_utc: None,
                handover_date_utc: ret.handover_date_utc,
                bulk_condition: Some(condition.into()),
                bulk_place_id: Some(place),
                apply_to_all: true,
                items: return_items(handover, devices),
            },
        )
        .await
        .expect("update_return")
}

/// Единственная запись `custom:group_member_released` устройства: `(group_id, act_id)`.
async fn release_payload(ctx: &AppCtx, device_id: i64) -> (i64, i64) {
    assert_eq!(
        release_audit_count(ctx, device_id).await,
        1,
        "ожидалась ровно одна запись о выводе из состава"
    );
    let raw = text_of(
        ctx,
        "SELECT payload_json FROM audit_log \
         WHERE action = 'custom:group_member_released' AND entity_id = ?1",
        vec![int(device_id)],
    )
    .await;
    let v: serde_json::Value = serde_json::from_str(&raw).expect("payload json");
    (
        v["group_id"].as_i64().expect("group_id"),
        v["act_id"].as_i64().expect("act_id"),
    )
}

/// Состав группы по `group_devices`: сортированный список device_id.
async fn group_member_ids(ctx: &AppCtx, group_id: i64) -> Vec<i64> {
    ctx.writer
        .execute(move |conn| {
            let mut st = conn
                .prepare("SELECT device_id FROM group_devices WHERE group_id = ?1 ORDER BY 1")
                .map_err(map_rusqlite)?;
            let rows = st
                .query_map(params![group_id], |r| r.get::<_, i64>(0))
                .map_err(map_rusqlite)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(map_rusqlite)?;
            Ok(rows)
        })
        .await
        .expect("members")
}

// --- S3: create (handover) -------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn s3_handover_releases() {
    let (ctx, _dir) = make_test_ctx().await;
    let arm = type_id_by_code(&ctx, "workstation").await;
    let place_a = seed_place(&ctx, "Склад А").await;
    let place_b = seed_place(&ctx, "Склад Б").await;
    let g = seed_group(&ctx, arm, "АРМ #3", 3, Some(place_a)).await;
    let target = seed_device(&ctx, 1, "Монитор 1", "INV-S3-T", Some(place_a), Some(g)).await;
    let mate = seed_device(
        &ctx,
        1,
        "Системный блок 1",
        "INV-S3-M",
        Some(place_a),
        Some(g),
    )
    .await;
    let outsider = seed_device(&ctx, 1, "Ноутбук 1", "INV-S3-O", Some(place_a), None).await;
    assert_ne!(place_a, place_b);

    let act = handover_act(&ctx, "S3-1", &[target, outsider], place_b).await;

    assert_eq!(device_place(&ctx, target).await, Some(place_b));
    assert_eq!(member_rows(&ctx, target).await, 0, "вышел из состава");
    let (gid, aid) = release_payload(&ctx, target).await;
    assert_eq!((gid, aid), (g, act.id), "payload {{group_id, act_id}}");
    assert_eq!(
        group_member_ids(&ctx, g).await,
        vec![mate],
        "остальные члены остались в составе"
    );
    assert_eq!(device_place(&ctx, mate).await, Some(place_a));
    // Контроль: устройство вне групп — акт прошёл как раньше, записей нет.
    assert_eq!(device_place(&ctx, outsider).await, Some(place_b));
    assert_eq!(release_audit_count(&ctx, outsider).await, 0);
    assert_eq!(release_audit_count(&ctx, mate).await, 0);
}

// --- S4: update (added / removed) ------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn s4a_update_added_releases() {
    let (ctx, _dir) = make_test_ctx().await;
    let arm = type_id_by_code(&ctx, "workstation").await;
    let place_a = seed_place(&ctx, "Склад А").await;
    let place_b = seed_place(&ctx, "Склад Б").await;
    let g = seed_group(&ctx, arm, "АРМ #3", 3, Some(place_a)).await;
    let target = seed_device(&ctx, 1, "Монитор 1", "INV-S4A-T", Some(place_a), Some(g)).await;
    let mate = seed_device(
        &ctx,
        1,
        "Системный блок 1",
        "INV-S4A-M",
        Some(place_a),
        Some(g),
    )
    .await;
    let base = seed_device(&ctx, 1, "Ноутбук 1", "INV-S4A-B", Some(place_a), None).await;
    let extra = seed_device(&ctx, 1, "Ноутбук 2", "INV-S4A-E", Some(place_a), None).await;

    let act = handover_act(&ctx, "S4A-1", &[base], place_b).await;
    assert_eq!(
        member_rows(&ctx, target).await,
        1,
        "до правки — член группы"
    );

    let _ = update_handover(&ctx, &act, &[base, target, extra]).await;

    assert_eq!(device_place(&ctx, target).await, Some(place_b));
    assert_eq!(member_rows(&ctx, target).await, 0);
    let (gid, aid) = release_payload(&ctx, target).await;
    assert_eq!((gid, aid), (g, act.id));
    assert_eq!(group_member_ids(&ctx, g).await, vec![mate]);
    assert_eq!(device_place(&ctx, mate).await, Some(place_a));
    // Контроль: добавленное устройство вне групп.
    assert_eq!(device_place(&ctx, extra).await, Some(place_b));
    assert_eq!(release_audit_count(&ctx, extra).await, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn s4b_update_removed_releases_if_locked() {
    let (ctx, _dir) = make_test_ctx().await;
    let arm = type_id_by_code(&ctx, "workstation").await;
    let place_b = seed_place(&ctx, "Склад Б").await;
    let place_c = seed_place(&ctx, "Склад В").await;
    // Группа живёт на месте акта: устройство вступило в неё ПОСЛЕ акта.
    let g = seed_group(&ctx, arm, "АРМ #3", 3, Some(place_b)).await;
    let x = seed_device(&ctx, 1, "Монитор 1", "INV-S4B-X", Some(place_c), None).await;
    let y = seed_device(&ctx, 1, "Монитор 2", "INV-S4B-Y", Some(place_c), None).await;
    let z = seed_device(&ctx, 1, "Монитор 3", "INV-S4B-Z", Some(place_c), None).await;
    let act = handover_act(&ctx, "S4B-1", &[x, y, z], place_b).await;
    join_group(&ctx, x, g).await;
    assert_eq!(device_place(&ctx, x).await, Some(place_b));

    let _ = update_handover(&ctx, &act, &[z]).await;

    // x: restore из снимка (Склад В) и выход из состава; y — не член, без записей.
    assert_eq!(device_place(&ctx, x).await, Some(place_c));
    assert_eq!(member_rows(&ctx, x).await, 0);
    let (gid, aid) = release_payload(&ctx, x).await;
    assert_eq!((gid, aid), (g, act.id));
    assert_eq!(device_place(&ctx, y).await, Some(place_c));
    assert_eq!(release_audit_count(&ctx, y).await, 0);
}

/// D-21: у группы без места нет «запирающего» членства — restore не трогает состав.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn s4b_update_removed_keeps_membership_of_dormant_group() {
    let (ctx, _dir) = make_test_ctx().await;
    let arm = type_id_by_code(&ctx, "workstation").await;
    let place_b = seed_place(&ctx, "Склад Б").await;
    let place_c = seed_place(&ctx, "Склад В").await;
    let dormant = seed_group(&ctx, arm, "АРМ без места", 7, None).await;
    let x = seed_device(&ctx, 1, "Монитор 1", "INV-S4D-X", Some(place_c), None).await;
    let z = seed_device(&ctx, 1, "Монитор 2", "INV-S4D-Z", Some(place_c), None).await;
    let act = handover_act(&ctx, "S4D-1", &[x, z], place_b).await;
    join_group(&ctx, x, dormant).await;

    let _ = update_handover(&ctx, &act, &[z]).await;

    assert_eq!(
        device_place(&ctx, x).await,
        Some(place_c),
        "restore отработал"
    );
    assert_eq!(
        member_rows(&ctx, x).await,
        1,
        "состав спящей группы не тронут"
    );
    assert_eq!(release_audit_count(&ctx, x).await, 0);
}

// --- S5: do_return ---------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn s5_return_releases() {
    let (ctx, _dir) = make_test_ctx().await;
    let arm = type_id_by_code(&ctx, "workstation").await;
    let place_b = seed_place(&ctx, "Склад Б").await;
    let place_c = seed_place(&ctx, "Склад В").await;
    let place_d = seed_place(&ctx, "Склад Г").await;
    let g = seed_group(&ctx, arm, "АРМ #3", 3, Some(place_b)).await;
    let x = seed_device(&ctx, 1, "Монитор 1", "INV-S5-X", Some(place_c), None).await;
    let y = seed_device(&ctx, 1, "Монитор 2", "INV-S5-Y", Some(place_c), None).await;
    let act = handover_act(&ctx, "S5-1", &[x, y], place_b).await;
    join_group(&ctx, x, g).await;
    assert_eq!(device_place(&ctx, x).await, Some(place_b));

    let ret = return_act(&ctx, &act, &[x, y], "Хорошее", place_d).await;

    assert_eq!(device_place(&ctx, x).await, Some(place_d));
    assert_eq!(member_rows(&ctx, x).await, 0);
    let (gid, aid) = release_payload(&ctx, x).await;
    assert_eq!((gid, aid), (g, ret.id), "act_id — акт возврата");
    // Контроль: устройство вне групп вернулось как раньше.
    assert_eq!(device_place(&ctx, y).await, Some(place_d));
    assert_eq!(release_audit_count(&ctx, y).await, 0);
}

// --- S6: update_return (added / edited / removed) --------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn s6_update_return_added_releases() {
    let (ctx, _dir) = make_test_ctx().await;
    let arm = type_id_by_code(&ctx, "workstation").await;
    let place_b = seed_place(&ctx, "Склад Б").await;
    let place_c = seed_place(&ctx, "Склад В").await;
    let place_d = seed_place(&ctx, "Склад Г").await;
    let g = seed_group(&ctx, arm, "АРМ #3", 3, Some(place_b)).await;
    let d1 = seed_device(&ctx, 1, "Монитор 1", "INV-S6A-1", Some(place_c), None).await;
    let d2 = seed_device(&ctx, 1, "Монитор 2", "INV-S6A-2", Some(place_c), None).await;
    let act = handover_act(&ctx, "S6A-1", &[d1, d2], place_b).await;
    let ret = return_act(&ctx, &act, &[d1], "Хорошее", place_d).await;
    join_group(&ctx, d2, g).await; // d2 ещё не возвращён, стоит на Складе Б
    assert_eq!(device_place(&ctx, d2).await, Some(place_b));

    let _ = update_return_act(&ctx, &act, &ret, &[d1, d2], "Хорошее", place_d).await;

    assert_eq!(device_place(&ctx, d2).await, Some(place_d));
    assert_eq!(member_rows(&ctx, d2).await, 0);
    let (gid, aid) = release_payload(&ctx, d2).await;
    assert_eq!((gid, aid), (g, ret.id));
    assert_eq!(release_audit_count(&ctx, d1).await, 0, "d1 не член группы");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn s6_update_return_edited_releases() {
    let (ctx, _dir) = make_test_ctx().await;
    let arm = type_id_by_code(&ctx, "workstation").await;
    let place_b = seed_place(&ctx, "Склад Б").await;
    let place_c = seed_place(&ctx, "Склад В").await;
    let place_d = seed_place(&ctx, "Склад Г").await;
    let place_e = seed_place(&ctx, "Склад Д").await;
    let g = seed_group(&ctx, arm, "АРМ #3", 3, Some(place_d)).await;
    let d1 = seed_device(&ctx, 1, "Монитор 1", "INV-S6B-1", Some(place_c), None).await;
    let d2 = seed_device(&ctx, 1, "Монитор 2", "INV-S6B-2", Some(place_c), None).await;
    let act = handover_act(&ctx, "S6B-1", &[d1, d2], place_b).await;
    let ret = return_act(&ctx, &act, &[d1, d2], "Хорошее", place_d).await;
    join_group(&ctx, d1, g).await;
    assert_eq!(device_place(&ctx, d1).await, Some(place_d));

    // Правка возврата меняет и состояние, и место обоих (retained_with_change).
    let _ = update_return_act(&ctx, &act, &ret, &[d1, d2], "Требует ремонта", place_e).await;

    assert_eq!(device_place(&ctx, d1).await, Some(place_e));
    assert_eq!(member_rows(&ctx, d1).await, 0);
    let (gid, aid) = release_payload(&ctx, d1).await;
    assert_eq!((gid, aid), (g, ret.id));
    assert_eq!(device_place(&ctx, d2).await, Some(place_e));
    assert_eq!(release_audit_count(&ctx, d2).await, 0, "d2 не член группы");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn s6_update_return_removed_releases_if_locked() {
    let (ctx, _dir) = make_test_ctx().await;
    let arm = type_id_by_code(&ctx, "workstation").await;
    let place_b = seed_place(&ctx, "Склад Б").await;
    let place_c = seed_place(&ctx, "Склад В").await;
    let place_d = seed_place(&ctx, "Склад Г").await;
    let g = seed_group(&ctx, arm, "АРМ #3", 3, Some(place_d)).await;
    let d1 = seed_device(&ctx, 1, "Монитор 1", "INV-S6C-1", Some(place_c), None).await;
    let d2 = seed_device(&ctx, 1, "Монитор 2", "INV-S6C-2", Some(place_c), None).await;
    let act = handover_act(&ctx, "S6C-1", &[d1, d2], place_b).await;
    let ret = return_act(&ctx, &act, &[d1, d2], "Хорошее", place_d).await;
    join_group(&ctx, d2, g).await;
    assert_eq!(device_place(&ctx, d2).await, Some(place_d));

    // d2 убран из возврата: restore снимка (в работе, Склад Б) + выход из состава.
    let _ = update_return_act(&ctx, &act, &ret, &[d1], "Хорошее", place_d).await;

    assert_eq!(device_place(&ctx, d2).await, Some(place_b));
    assert_eq!(member_rows(&ctx, d2).await, 0);
    let (gid, aid) = release_payload(&ctx, d2).await;
    assert_eq!((gid, aid), (g, ret.id));
    assert_eq!(release_audit_count(&ctx, d1).await, 0);
    assert_eq!(device_place(&ctx, d1).await, Some(place_d));
}

// --- S7: undo при удалении акта --------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn s7_undo_releases_if_locked() {
    let (ctx, _dir) = make_test_ctx().await;
    let arm = type_id_by_code(&ctx, "workstation").await;
    let place_b = seed_place(&ctx, "Склад Б").await;
    let place_c = seed_place(&ctx, "Склад В").await;
    let g = seed_group(&ctx, arm, "АРМ #3", 3, Some(place_b)).await;
    let x = seed_device(&ctx, 1, "Монитор 1", "INV-S7-X", Some(place_c), None).await;
    let y = seed_device(&ctx, 1, "Монитор 2", "INV-S7-Y", Some(place_c), None).await;
    let act = handover_act(&ctx, "S7-1", &[x, y], place_b).await;
    join_group(&ctx, x, g).await;
    assert_eq!(device_place(&ctx, x).await, Some(place_b));

    ctx.acts
        .delete_soft(act.id, act.version)
        .await
        .expect("delete_soft handover");

    assert_eq!(device_place(&ctx, x).await, Some(place_c), "restore снимка");
    assert_eq!(member_rows(&ctx, x).await, 0);
    let (gid, aid) = release_payload(&ctx, x).await;
    assert_eq!((gid, aid), (g, act.id));
    assert_eq!(device_place(&ctx, y).await, Some(place_c));
    assert_eq!(release_audit_count(&ctx, y).await, 0);
}

/// Членство НЕ восстанавливается: бывший член группы, выведенный актом, после
/// удаления акта возвращается на старое место, но в состав не возвращается.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn s7_undo_does_not_restore_membership() {
    let (ctx, _dir) = make_test_ctx().await;
    let arm = type_id_by_code(&ctx, "workstation").await;
    let place_a = seed_place(&ctx, "Склад А").await;
    let place_b = seed_place(&ctx, "Склад Б").await;
    let g = seed_group(&ctx, arm, "АРМ #3", 3, Some(place_a)).await;
    let y = seed_device(&ctx, 1, "Монитор 1", "INV-S7N-Y", Some(place_a), Some(g)).await;
    let mate = seed_device(
        &ctx,
        1,
        "Системный блок 1",
        "INV-S7N-M",
        Some(place_a),
        Some(g),
    )
    .await;
    let act = handover_act(&ctx, "S7N-1", &[y], place_b).await;
    assert_eq!(member_rows(&ctx, y).await, 0, "акт вывел из состава");
    assert_eq!(release_audit_count(&ctx, y).await, 1);

    ctx.acts
        .delete_soft(act.id, act.version)
        .await
        .expect("delete_soft handover");

    assert_eq!(
        device_place(&ctx, y).await,
        Some(place_a),
        "место вернулось"
    );
    assert_eq!(member_rows(&ctx, y).await, 0, "членство не восстановлено");
    assert_eq!(
        release_audit_count(&ctx, y).await,
        1,
        "undo не пишет второго вывода: членства уже не было"
    );
    assert_eq!(group_member_ids(&ctx, g).await, vec![mate]);
}

/// Undo акта возврата — тот же helper, ветка `ActType::Return`.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn s7_undo_return_releases_if_locked() {
    let (ctx, _dir) = make_test_ctx().await;
    let arm = type_id_by_code(&ctx, "workstation").await;
    let place_b = seed_place(&ctx, "Склад Б").await;
    let place_c = seed_place(&ctx, "Склад В").await;
    let place_d = seed_place(&ctx, "Склад Г").await;
    let g = seed_group(&ctx, arm, "АРМ #3", 3, Some(place_d)).await;
    let x = seed_device(&ctx, 1, "Монитор 1", "INV-S7R-X", Some(place_c), None).await;
    let act = handover_act(&ctx, "S7R-1", &[x], place_b).await;
    let ret = return_act(&ctx, &act, &[x], "Хорошее", place_d).await;
    join_group(&ctx, x, g).await;
    assert_eq!(device_place(&ctx, x).await, Some(place_d));

    ctx.acts
        .delete_soft(ret.id, ret.version)
        .await
        .expect("delete_soft return");

    assert_eq!(
        device_place(&ctx, x).await,
        Some(place_b),
        "назад в акт выдачи"
    );
    assert_eq!(member_rows(&ctx, x).await, 0);
    let (gid, aid) = release_payload(&ctx, x).await;
    assert_eq!((gid, aid), (g, ret.id));
}

// ===========================================================================
// Plan 41-16: счётный гейт write-site'ов `devices.place_id`
// ===========================================================================
//
// Урок фазы 40.1 / V042: полноту перечня write-site'ов нельзя держать в голове
// и закрывать точечной правкой — нужен сплошной заход ОТ СЕРВЕРНЫХ МУТАЦИЙ и
// гейт, который краснеет при новом месте записи.
//
// Фактическая картина: `devices.place_id` пишут 8 методов `SqliteDeviceRepository`
// (замороженный реестр `PLACE_WRITER_METHODS`) плюс каскад картриджа. Вердикты:
//   - ТРЕБУЮТ release или guard: `update_status_and_place_in_tx`,
//     `update_full_in_tx`, `restore_from_snapshot_in_tx` и `update_in_tx`
//     (последний закрыт guard'ом S1 в `DeviceService::update`: он ОТКЛОНЯЕТ
//     перенос, а не выводит из состава). Needle'ы — `WRITE_SITE_CALLS`.
//   - ОСВОБОЖДЕНЫ как рождение: `create_in_tx`, `clone_device_in_tx`
//     (`BIRTH_EXEMPT_CALLS`, см. комментарий у константы).
//   - Не-tx пара трейта `create`/`update` — известна, продакшн-вызовов нет
//     (сервисы используют только `*_in_tx`), needle по имени невозможен
//     (`update(`/`create(` слишком общие); держится только реестром «от
//     репозитория» (`write_site_gate_repo_place_writers_match_frozen_registry`).
//
// Каскад картриджа: `cartridges_sqlite.rs:651` (`UPDATE devices SET place_id=?1
// ...`, сырой SQL) — известный третий класс write-site'ов. Он покрыт сценарием
// S9 выше, но НЕ входит ни в один инвентарь-гейт: гейт для сырого SQL не строился,
// потому что в тестах `report_service.rs` есть `INSERT INTO devices` и гейт был
// бы шумным.
//
// Как добавить новый write-site: (1) решить, должен ли он выводить устройство
// из состава группы (акт — да, D-22; перенос группой — нет, это и есть «место
// задаёт группа»); (2) добавить явный вызов `release_device_in_tx` /
// `release_if_locked_device_in_tx` на сайте (либо guard, отклоняющий перенос, как
// S1); (3) написать сценарий в этом файле; (4) только после этого поднять
// константу в реестре ниже. Вердикт нового метода репозитория вносится в
// `PLACE_WRITER_METHODS` и, для ReleaseRequired/BirthExempt, в needle'ы
// `WRITE_SITE_CALLS` / `BIRTH_EXEMPT_CALLS`.

/// Вызовы, требующие release ИЛИ guard, отклоняющий перенос.
/// `update_in_tx(` уникален по имени среди репозиториев на сегодня; будущий
/// одноимённый метод другого репозитория сделает гейт красным — это сознательно
/// безопасное направление (решается уточнением реестра, не отключением гейта).
const WRITE_SITE_CALLS: [&str; 4] = [
    "update_status_and_place_in_tx(",
    "update_full_in_tx(",
    "restore_from_snapshot_in_tx(",
    "update_in_tx(",
];
const RELEASE_CALLS: [&str; 2] = ["release_device_in_tx(", "release_if_locked_device_in_tx("];

/// Освобождены от release: это РОЖДЕНИЕ устройства (в т.ч. клона). Новое
/// устройство ещё не член группы, выводить из состава некого (сценарий S10 в
/// шапке файла); release не нужен и не должен добавляться.
///
/// Ловушка имён: `create_in_tx(` носит и `SqlitePrinterRepository`
/// (`printers_sqlite.rs`, `printer_service.rs`, `printer_repo.create_in_tx(` в
/// `device_service.rs`). Поэтому выбраны способы (а) needle привязан к
/// получателю `repo.` с границей идентификатора слева (`printer_repo.` не
/// считается) и (б) скан этого набора исключает `crates/trackly-infra/src/repos/`
/// (`BIRTH_SCAN_SKIP_PREFIX`). Вариант (в) — внести `printers_sqlite.rs` в реестр
/// — отвергнут: число вызовов в его тест-модуле меняется от любой несвязанной
/// правки, гейт стал бы шумным и его бы отключили. Оговорка: если сервис назовёт
/// репозиторий устройств иначе, чем `repo`, birth-гейт его не увидит; это
/// допустимо, потому что вердикт «рождение» не требует release, а полноту «от
/// репозитория» держит `write_site_gate_repo_place_writers_match_frozen_registry`.
const BIRTH_EXEMPT_CALLS: [&str; 2] = ["repo.create_in_tx(", "clone_device_in_tx("];
const BIRTH_SCAN_SKIP_PREFIX: &str = "crates/trackly-infra/src/repos/";

/// Реестр: файл (относительно корня репозитория) -> число вызовов методов из
/// `WRITE_SITE_CALLS`. act_service.rs: 8 = S3 create, S4 update (added, removed),
/// S5 do_return, S6 update_return (removed, added, edited), S7 undo.
/// place_service.rs: 1 = S2 (одиночные устройства массового переноса).
/// group_place.rs: 1 = свой путь группы (`apply_group_place_to_device_in_tx`).
/// device_service.rs: 1 = S1 `update_in_tx`, прикрыт guard'ом (см.
/// `uncovered_guarded_sites`). Определения методов (`pub fn ...` в
/// devices_sqlite.rs) в счёт не входят.
const WRITE_SITE_REGISTRY: [(&str, usize); 4] = [
    ("crates/trackly-app/src/services/act_service.rs", 8),
    ("crates/trackly-app/src/services/place_service.rs", 1),
    ("crates/trackly-app/src/services/group_place.rs", 1),
    ("crates/trackly-app/src/services/device_service.rs", 1),
];

/// Реестр мест рождения устройства: device_service.rs 2 (одиночное создание и
/// массовое), act_service.rs 1 (клон).
const BIRTH_EXEMPT_REGISTRY: [(&str, usize); 2] = [
    ("crates/trackly-app/src/services/device_service.rs", 2),
    ("crates/trackly-app/src/services/act_service.rs", 1),
];

/// Число вызовов `needles` в исходнике: строки-комментарии (`//`, `///`, `//!`),
/// хвосты `// ...` и определения (`fn `) не считаются. Вхождение принимается,
/// только если слева от него нет символа идентификатора (буква, цифра, `_`):
/// `printer_repo.create_in_tx(` не считается needle'ом `repo.create_in_tx(`, а
/// `foo_update_in_tx(` — needle'ом `update_in_tx(`.
fn count_calls(src: &str, needles: &[&str]) -> usize {
    src.lines()
        .map(|line| {
            let code = line.split("//").next().unwrap_or("");
            if code.contains("fn ") {
                return 0;
            }
            needles
                .iter()
                .map(|n| {
                    code.match_indices(n)
                        .filter(|(pos, _)| {
                            code[..*pos]
                                .chars()
                                .next_back()
                                .is_none_or(|c| !(c.is_alphanumeric() || c == '_'))
                        })
                        .count()
                })
                .sum::<usize>()
        })
        .sum()
}

/// Чистая проверка реестра: возвращает нарушения (пусто = всё в порядке).
/// Файлы с префиксом `skip_prefix` пропускаются.
fn registry_violations(
    sources: &[(String, String)],
    needles: &[&str],
    registry: &[(&str, usize)],
    skip_prefix: Option<&str>,
    new_site_hint: &str,
) -> Vec<String> {
    let mut violations = Vec::new();
    let mut seen: Vec<(String, usize)> = Vec::new();
    for (rel, text) in sources {
        if skip_prefix.is_some_and(|p| rel.starts_with(p)) {
            continue;
        }
        let n = count_calls(text, needles);
        if n > 0 {
            seen.push((rel.clone(), n));
        }
    }
    for (rel, n) in &seen {
        match registry.iter().find(|(f, _)| f == rel) {
            None => violations.push(format!("новое место {new_site_hint} в {rel} ({n} вызовов)")),
            Some((_, want)) if n != want => violations.push(format!(
                "число вызовов в {rel} изменилось ({n}, в реестре {want}) — {new_site_hint}"
            )),
            Some(_) => {}
        }
    }
    for (rel, want) in registry {
        if !seen.iter().any(|(f, _)| f == rel) {
            violations.push(format!(
                "файл реестра {rel} ({want} вызовов) больше не содержит вызовов — обнови реестр"
            ));
        }
    }
    violations
}

fn rust_sources_under_crates() -> Vec<(String, String)> {
    fn walk(dir: &std::path::Path, root: &std::path::Path, out: &mut Vec<(String, String)>) {
        for entry in std::fs::read_dir(dir).expect("read_dir") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                walk(&path, root, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let rel = path
                    .strip_prefix(root)
                    .expect("strip_prefix")
                    .to_string_lossy()
                    .replace('\\', "/");
                let text = std::fs::read_to_string(&path).expect("read source");
                out.push((rel, text));
            }
        }
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let root = root.canonicalize().expect("repo root");
    let mut out = Vec::new();
    for krate in std::fs::read_dir(root.join("crates")).expect("crates dir") {
        let src = krate.expect("crate entry").path().join("src");
        if src.is_dir() {
            walk(&src, &root, &mut out);
        }
    }
    out
}

/// Самопроверка счётчика: «зелёный» гейт отличим от «ничего не ищет».
#[test]
fn write_site_gate_counter_selftest() {
    let fixture = "\
let a = repo.update_full_in_tx(tx, 1);
// repo.update_full_in_tx(tx, 2);
/// restore_from_snapshot_in_tx( в докстроке
pub fn restore_from_snapshot_in_tx(
let b = repo.restore_from_snapshot_in_tx(tx, 3); // хвост update_full_in_tx(
release_device_in_tx(tx, g);
release_if_locked_device_in_tx(tx, g);
";
    assert_eq!(
        count_calls(fixture, &WRITE_SITE_CALLS),
        2,
        "вызовы, не комментарии и не определения"
    );
    assert_eq!(count_calls(fixture, &RELEASE_CALLS), 2);
    assert_eq!(
        count_calls("restore_from_snapshot_in_tx(\n", &WRITE_SITE_CALLS),
        1,
        "вызов без комментария считается"
    );
    assert_eq!(count_calls("", &WRITE_SITE_CALLS), 0);

    // Граница идентификатора слева (D-03): `printer_repo.` не есть `repo.`.
    assert_eq!(
        count_calls(
            "let _ = printer_repo.create_in_tx(a);\n",
            &["repo.create_in_tx("]
        ),
        0,
        "принтерный репозиторий не считается"
    );
    assert_eq!(
        count_calls("let id = repo.create_in_tx(a);\n", &["repo.create_in_tx("]),
        1
    );
    assert_eq!(
        count_calls(
            "foo_update_in_tx(x);\nr.update_in_tx(x);\n",
            &["update_in_tx("]
        ),
        1,
        "суффикс чужого имени не считается, вызов через точку считается"
    );
    assert_eq!(count_calls("update_in_tx(x);\n", &["update_in_tx("]), 1);
}

/// Гейт 1: в act_service.rs ровно 8 write-site'ов и ровно 8 release-вызовов, и
/// КАЖДЫЙ write-site сразу сопровождается своим helper'ом (прямые пути —
/// безусловный `release_device_in_tx`, restore-пути — `release_if_locked_...`).
#[test]
fn write_site_gate_act_service_every_site_has_release() {
    let src = include_str!("../src/services/act_service.rs");
    assert_eq!(
        count_calls(src, &WRITE_SITE_CALLS),
        8,
        "число write-site'ов devices.place_id в act_service.rs изменилось — добавь кейс release \
         в group_write_sites.rs и обнови реестр"
    );
    assert_eq!(
        count_calls(src, &RELEASE_CALLS),
        8,
        "каждый write-site act_service.rs обязан вызывать release-helper"
    );

    // Парность: release — в окне строк сразу после write-site'а, нужного вида.
    const WINDOW: usize = 14;
    let lines: Vec<&str> = src.lines().collect();
    let mut paired = 0;
    for (i, line) in lines.iter().enumerate() {
        let code = line.split("//").next().unwrap_or("");
        if code.contains("fn ") || count_calls(code, &WRITE_SITE_CALLS) == 0 {
            continue;
        }
        // `.restore_from_snapshot_in_tx(` стоит на строке цепочки — вид helper'а
        // определяется по самому вызову на этой строке.
        let is_restore = code.contains("restore_from_snapshot_in_tx(");
        let (want, forbid) = if is_restore {
            ("release_if_locked_device_in_tx(", "release_device_in_tx(")
        } else {
            ("release_device_in_tx(", "release_if_locked_device_in_tx(")
        };
        let window: Vec<&str> = lines[i + 1..(i + 1 + WINDOW).min(lines.len())]
            .iter()
            .map(|l| l.split("//").next().unwrap_or(""))
            .collect();
        // release должен идти ДО следующего write-site'а
        let end = window
            .iter()
            .position(|l| count_calls(l, &WRITE_SITE_CALLS) > 0)
            .unwrap_or(window.len());
        let scope = &window[..end];
        assert!(
            scope.iter().any(|l| l.contains(want)),
            "write-site act_service.rs:{} без {want} в {WINDOW} строках после него — добавь кейс \
             release в group_write_sites.rs",
            i + 1
        );
        assert!(
            !scope.iter().any(|l| l.contains(forbid)),
            "write-site act_service.rs:{} вызывает не тот release-helper ({forbid})",
            i + 1
        );
        paired += 1;
    }
    assert_eq!(paired, 8, "проверено парных write-site'ов");
}

/// Гейт 2: по всем `crates/*/src` вызовы методов из `WRITE_SITE_CALLS` разрешены
/// только в файлах реестра, с точным числом; определения (`fn `) не считаются.
#[test]
fn write_site_gate_no_unregistered_place_writers() {
    let sources = rust_sources_under_crates();
    assert!(
        sources.len() > 50,
        "скан исходников ничего не нашёл ({} файлов) — гейт бы молча позеленел",
        sources.len()
    );
    let violations = registry_violations(
        &sources,
        &WRITE_SITE_CALLS,
        &WRITE_SITE_REGISTRY,
        None,
        "записи devices.place_id — добавь кейс release в group_write_sites.rs и внеси файл в \
         реестр",
    );
    assert!(violations.is_empty(), "{}", violations.join("\n"));
}

/// Гейт рождения: вызовы `BIRTH_EXEMPT_CALLS` вне `crates/trackly-infra/src/repos/`
/// разрешены только в файлах реестра, с точным числом.
#[test]
fn write_site_gate_birth_exemptions_are_inventoried() {
    let sources = rust_sources_under_crates();
    assert!(
        sources.len() > 50,
        "скан исходников ничего не нашёл ({} файлов)",
        sources.len()
    );
    for n in BIRTH_EXEMPT_CALLS {
        assert!(
            !WRITE_SITE_CALLS.contains(&n),
            "освобождение закодировано, а не описано: {n} не может быть в WRITE_SITE_CALLS"
        );
    }
    let violations = registry_violations(
        &sources,
        &BIRTH_EXEMPT_CALLS,
        &BIRTH_EXEMPT_REGISTRY,
        Some(BIRTH_SCAN_SKIP_PREFIX),
        "РОЖДЕНИЯ устройства — подтверди, что оно не переносит существующее устройство, и внеси \
         в BIRTH_EXEMPT_REGISTRY",
    );
    assert!(violations.is_empty(), "{}", violations.join("\n"));
}

const RELEASE_WINDOW: usize = 14;
const GUARD_WINDOW: usize = 20;

fn code_of(line: &str) -> &str {
    line.split("//").next().unwrap_or("")
}

/// Номера (1-based) строк `update_in_tx(`-сайтов без прикрытия. Сайт покрыт, если
/// (а) в `RELEASE_WINDOW` строках ПОСЛЕ него (до следующего write-site'а) есть
/// release-helper ИЛИ (б) в `GUARD_WINDOW` строках ДО него есть и
/// `locked_group_for_device_in_tx(`, и `AppError::Validation` (guard отклоняет
/// перенос, D-02). Guard стоит ДО write-site'а, release ПОСЛЕ — окна
/// разнонаправленные; на сайте `device_service.rs:741` guard лежит на :729-730.
fn uncovered_guarded_sites(src: &str) -> Vec<usize> {
    let lines: Vec<&str> = src.lines().collect();
    let mut out = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let code = code_of(line);
        if code.contains("fn ") || count_calls(code, &["update_in_tx("]) == 0 {
            continue;
        }
        let after: Vec<&str> = lines[i + 1..(i + 1 + RELEASE_WINDOW).min(lines.len())]
            .iter()
            .map(|l| code_of(l))
            .collect();
        let end = after
            .iter()
            .position(|l| count_calls(l, &WRITE_SITE_CALLS) > 0)
            .unwrap_or(after.len());
        let released = after[..end]
            .iter()
            .any(|l| RELEASE_CALLS.iter().any(|r| l.contains(r)));
        let before: Vec<&str> = lines[i.saturating_sub(GUARD_WINDOW)..i]
            .iter()
            .map(|l| code_of(l))
            .collect();
        let guarded = before
            .iter()
            .any(|l| l.contains("locked_group_for_device_in_tx("))
            && before.iter().any(|l| l.contains("AppError::Validation"));
        if !released && !guarded {
            out.push(i + 1);
        }
    }
    out
}

/// Гейт парности: каждый `update_in_tx(`-сайт — с release-helper'ом или guard'ом S1.
#[test]
fn write_site_gate_update_in_tx_released_or_guarded() {
    let mut inspected = 0;
    let mut violations = Vec::new();
    for (rel, text) in rust_sources_under_crates() {
        if count_calls(&text, &["update_in_tx("]) == 0 {
            continue;
        }
        let total = text
            .lines()
            .filter(|l| count_calls(l, &["update_in_tx("]) > 0)
            .count();
        inspected += total;
        for line in uncovered_guarded_sites(&text) {
            violations.push(format!(
                "write-site update_in_tx в {rel}:{line} без release-helper'а и без guard'а S1 — \
                 перенос существующего устройства обойдёт членство в группе (D-22/GRP-07)"
            ));
        }
    }
    assert!(
        inspected > 0,
        "ни одного update_in_tx-сайта — гейт вакуумен"
    );
    assert!(violations.is_empty(), "{}", violations.join("\n"));
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum PlaceWriterVerdict {
    ReleaseRequired,
    BirthExempt,
    NoProductionCallers,
}

/// Замороженный реестр методов `SqliteDeviceRepository`, пишущих `devices.place_id`.
/// `create`/`update` — не-tx методы трейта `DeviceRepository`: известна, продакшн-
/// вызовов нет, needle по имени невозможен (`update(`/`create(` — слишком общие).
/// Новая запись добавляется только вместе с вердиктом и, для
/// ReleaseRequired/BirthExempt, с needle'ом в `WRITE_SITE_CALLS` /
/// `BIRTH_EXEMPT_CALLS`.
const PLACE_WRITER_METHODS: [(&str, PlaceWriterVerdict); 8] = [
    (
        "update_status_and_place_in_tx",
        PlaceWriterVerdict::ReleaseRequired,
    ),
    ("update_full_in_tx", PlaceWriterVerdict::ReleaseRequired),
    (
        "restore_from_snapshot_in_tx",
        PlaceWriterVerdict::ReleaseRequired,
    ),
    ("update_in_tx", PlaceWriterVerdict::ReleaseRequired),
    ("create_in_tx", PlaceWriterVerdict::BirthExempt),
    ("clone_device_in_tx", PlaceWriterVerdict::BirthExempt),
    ("create", PlaceWriterVerdict::NoProductionCallers),
    ("update", PlaceWriterVerdict::NoProductionCallers),
];

/// Имена методов, пишущих `devices.place_id`: SQL-литерал тела (`INSERT INTO
/// devices` / `UPDATE devices`, до закрывающей кавычки) упоминает `place_id`
/// (`writes_place_id_in_sql`; имя параметра `place_id` записью не считается). Берётся текст до первого `#[cfg(test)]`;
/// строка с `fn ` открывает метод, его тело — до следующей строки с `fn `.
/// Эвристика консервативна в сторону ложных срабатываний (метод с
/// `UPDATE devices ... WHERE place_id` покраснеет и потребует явной
/// классификации). Граница: запись через хелпер в другом файле эта проверка не
/// увидит, пока запись идёт SQL-строкой по `devices` — пропуск невозможен.
fn place_writing_methods(src: &str) -> Vec<String> {
    let src = src.split("#[cfg(test)]").next().unwrap_or(src);
    let mut methods: Vec<(String, String)> = Vec::new();
    for line in src.lines() {
        let code = code_of(line);
        if let Some(pos) = code.find("fn ") {
            let name: String = code[pos + 3..]
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            methods.push((name, String::new()));
        }
        if let Some((_, body)) = methods.last_mut() {
            body.push_str(code);
            body.push('\n');
        }
    }
    let mut out: Vec<String> = methods
        .into_iter()
        .filter(|(_, body)| writes_place_id_in_sql(body))
        .map(|(name, _)| name)
        .collect();
    out.sort();
    out.dedup();
    out
}

/// Есть ли в теле SQL-литерал `INSERT INTO devices` / `UPDATE devices`, в котором
/// (до закрывающей кавычки) упоминается `place_id`. Параметр функции или
/// аргумент `params![..]` с именем `place_id` сами по себе записью не считаются.
fn writes_place_id_in_sql(body: &str) -> bool {
    ["INSERT INTO devices", "UPDATE devices"]
        .iter()
        .flat_map(|marker| body.match_indices(marker).map(|(pos, _)| &body[pos..]))
        .any(|tail| tail.split('"').next().unwrap_or("").contains("place_id"))
}

/// (новые, исчезли) относительно `PLACE_WRITER_METHODS`.
fn place_writer_diff(src: &str) -> (Vec<String>, Vec<String>) {
    let found = place_writing_methods(src);
    let mut frozen: Vec<String> = PLACE_WRITER_METHODS
        .iter()
        .map(|(n, _)| n.to_string())
        .collect();
    frozen.sort();
    let new = found
        .iter()
        .filter(|n| !frozen.contains(n))
        .cloned()
        .collect();
    let gone = frozen
        .iter()
        .filter(|n| !found.contains(n))
        .cloned()
        .collect();
    (new, gone)
}

/// Гейт «от репозитория»: множество методов, пишущих `devices.place_id`, равно
/// замороженному реестру; новый метод краснит гейт, даже если его никто не вызывает.
#[test]
fn write_site_gate_repo_place_writers_match_frozen_registry() {
    let src = include_str!("../../trackly-infra/src/repos/devices_sqlite.rs");
    assert!(
        !place_writing_methods(src).is_empty(),
        "не найдено ни одного метода — гейт вакуумен"
    );
    let (new, gone) = place_writer_diff(src);
    assert!(
        new.is_empty() && gone.is_empty(),
        "методы devices_sqlite.rs, пишущие place_id, разошлись с реестром. новые: {new:?}; \
         исчезли: {gone:?}. Классифицируй метод: ReleaseRequired / BirthExempt / \
         NoProductionCallers"
    );
}

/// Связь трёх слоёв: вердикты <-> needle'ы.
#[test]
fn write_site_gate_verdicts_match_needles() {
    let with = |v: PlaceWriterVerdict| -> std::collections::BTreeSet<String> {
        PLACE_WRITER_METHODS
            .iter()
            .filter(|(_, x)| *x == v)
            .map(|(n, _)| n.to_string())
            .collect()
    };
    let release: std::collections::BTreeSet<String> = with(PlaceWriterVerdict::ReleaseRequired)
        .into_iter()
        .map(|n| format!("{n}("))
        .collect();
    let calls: std::collections::BTreeSet<String> =
        WRITE_SITE_CALLS.iter().map(|s| s.to_string()).collect();
    assert_eq!(release, calls, "ReleaseRequired != WRITE_SITE_CALLS");

    let birth: std::collections::BTreeSet<String> = BIRTH_EXEMPT_CALLS
        .iter()
        .map(|n| {
            n.strip_prefix("repo.")
                .unwrap_or(n)
                .strip_suffix('(')
                .expect("needle заканчивается на (")
                .to_string()
        })
        .collect();
    assert_eq!(
        with(PlaceWriterVerdict::BirthExempt),
        birth,
        "BirthExempt != BIRTH_EXEMPT_CALLS"
    );

    let none: std::collections::BTreeSet<String> =
        ["create", "update"].iter().map(|s| s.to_string()).collect();
    assert_eq!(with(PlaceWriterVerdict::NoProductionCallers), none);
}

/// Мутация 1: update_in_tx-сайт без release и без guard'а краснит проверку.
#[test]
fn write_site_gate_mutation_update_in_tx_without_guard_or_release() {
    // (i) голый сайт
    assert_eq!(
        uncovered_guarded_sites("let a = repo.update_in_tx(tx, 1);\n"),
        vec![1]
    );
    // (ii) release после
    assert!(uncovered_guarded_sites(
        "let a = repo.update_in_tx(tx, 1);\nlet x = 1;\nrelease_device_in_tx(tx, g);\n"
    )
    .is_empty());
    // (iii) guard до
    let guarded = "\
let g = groups.locked_group_for_device_in_tx(&tx, id)?;
return Err(AppError::Validation {
});
let a = repo.update_in_tx(tx, 1);
";
    assert!(uncovered_guarded_sites(guarded).is_empty());
    // (iv) маркер только в комментарии
    let commented = "\
// groups.locked_group_for_device_in_tx(&tx, id)?;
// AppError::Validation {
let a = repo.update_in_tx(tx, 1);
";
    assert_eq!(uncovered_guarded_sites(commented), vec![3]);

    // реальный текст
    let src = include_str!("../src/services/device_service.rs");
    let site_lines: Vec<usize> = src
        .lines()
        .enumerate()
        .filter(|(_, l)| count_calls(l, &["update_in_tx("]) > 0)
        .map(|(i, _)| i + 1)
        .collect();
    assert_eq!(site_lines.len(), 1, "сайт update_in_tx ровно один");
    assert!(
        uncovered_guarded_sites(src).is_empty(),
        "немутированная линия должна проходить"
    );
    let anchor = "groups.locked_group_for_device_in_tx(&tx, id)?";
    assert_eq!(
        src.matches(anchor).count(),
        1,
        "якорь мутации должен быть уникален (урок mutation_test_anchor_must_be_unique)"
    );
    let mutated = src.replace(anchor, "groups.some_other_lookup_in_tx(&tx, id)?");
    assert_eq!(uncovered_guarded_sites(&mutated), site_lines);
}

/// Мутация 2: новый/пропавший метод репозитория, пишущий place_id.
#[test]
fn write_site_gate_mutation_new_repo_method_writing_place_id() {
    let src = include_str!("../../trackly-infra/src/repos/devices_sqlite.rs");
    let (new, gone) = place_writer_diff(src);
    assert!(
        new.is_empty() && gone.is_empty(),
        "базовая линия: {new:?} {gone:?}"
    );

    let mut added = src.to_string();
    added.push_str(
        "\npub fn move_device_in_tx() {\n    let _ = \"UPDATE devices SET place_id = ?1 WHERE id = ?2\";\n}\n",
    );
    let (new, gone) = place_writer_diff(&added);
    assert_eq!(new, vec!["move_device_in_tx".to_string()]);
    assert!(gone.is_empty());

    let anchor = "status_id = ?1, place_id = ?2,";
    assert_eq!(
        src.matches(anchor).count(),
        1,
        "якорь мутации должен быть уникален (урок mutation_test_anchor_must_be_unique)"
    );
    let removed = src.replace(anchor, "status_id = ?1,");
    let (new, gone) = place_writer_diff(&removed);
    assert!(new.is_empty());
    assert_eq!(gone, vec!["update_status_and_place_in_tx".to_string()]);

    let negatives = "\
fn read_it() {
    let _ = \"SELECT d.place_id FROM devices d\";
}
fn notes_only() {
    let _ = \"UPDATE devices SET notes = ?1\";
}
";
    assert!(place_writing_methods(negatives).is_empty());
}

/// Мутация 3: birth-гейт и ловушка имён `create_in_tx`.
#[test]
fn write_site_gate_mutation_birth_inventory_name_trap() {
    const DEV: &str = "crates/trackly-app/src/services/device_service.rs";
    const ACT: &str = "crates/trackly-app/src/services/act_service.rs";
    let dev = include_str!("../src/services/device_service.rs");
    let act = include_str!("../src/services/act_service.rs");
    let check = |sources: &[(String, String)]| {
        registry_violations(
            sources,
            &BIRTH_EXEMPT_CALLS,
            &BIRTH_EXEMPT_REGISTRY,
            Some(BIRTH_SCAN_SKIP_PREFIX),
            "РОЖДЕНИЯ устройства",
        )
    };
    let base = |dev_text: String| {
        vec![
            (DEV.to_string(), dev_text),
            (ACT.to_string(), act.to_string()),
        ]
    };
    assert!(check(&base(dev.to_string())).is_empty(), "базовая линия");

    // A: новый вызов repo.create_in_tx
    let a = format!("{dev}\nlet _ = repo.create_in_tx(&tx, &domain_new, 0);\n");
    let v = check(&base(a));
    assert_eq!(v.len(), 1, "{v:?}");
    assert!(v[0].contains("device_service.rs"));

    // B: принтерный репозиторий не шумит
    let b = format!("{dev}\nlet _ = printer_repo.create_in_tx(&tx, &p, 0);\n");
    assert!(check(&base(b)).is_empty());

    // C: repos/ исключён префиксом
    let noisy = "let _ = repo.create_in_tx(&tx, &p, 0);\nlet _ = repo.create_in_tx(&tx, &q, 0);\n";
    let mut c = base(dev.to_string());
    c.push((
        "crates/trackly-infra/src/repos/printers_sqlite.rs".to_string(),
        noisy.to_string(),
    ));
    assert!(check(&c).is_empty());

    // D: тот же текст в новом сервисе — нарушение
    let mut d = base(dev.to_string());
    d.push((
        "crates/trackly-app/src/services/some_new_service.rs".to_string(),
        noisy.to_string(),
    ));
    let v = check(&d);
    assert_eq!(v.len(), 1, "{v:?}");
    assert!(v[0].contains("новое место") && v[0].contains("some_new_service.rs"));
}

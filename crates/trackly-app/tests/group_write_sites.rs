//! Phase 41 Plan 14: write-site'ы `devices.place_id`, закрытые планом 14.
//!
//! Инвентарь ОТ СЕРВЕРНЫХ МУТАЦИЙ (не от экранов; урок фазы 40.1):
//!   S1  `DeviceService::update`                      — guard «место задаёт группа» (этот файл)
//!   S2  `PlaceService::move_subtree_contents`        — план 15
//!   S3-S7 `ActService` (create/update/undo/…)         — план 16 (файл расширяется им)
//!   S8  `DeviceService::delete_soft`                 — освобождение членства (этот файл)
//!   S9  `cartridges_sqlite.rs` backfill места принтера — сознательно БЕЗ guard'а (этот файл)
//!   S10 create / bulk / CSV                          — без изменений: новое устройство не член группы
//!
//! Таблица-драйвер: каждый сценарий — строка `Scenario`; фикстуры расходятся
//! (повтор текущего места против реальной смены), поэтому тест не вакуумен.
//! Реальный `AppCtx` на временном каталоге; имена вымышленные.
//!
//! Префиксы: `s1_table_`, `s1_http_`, `s1_tauri_path_`, `s8_`, `s9_`.

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

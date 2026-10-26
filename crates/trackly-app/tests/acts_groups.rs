//! Phase 41.2 Plan 07: создание акта с группами (GAM-01..GAM-03, D-02, D-04, D-05, D-10..D-14).
//!
//! Перенос группы целиком, вынос части состава с подтверждением ДО первой записи,
//! якорь одной позицией с комплектацией, журнал пакета с картриджем принтера-члена.
//! Реальный `AppCtx` на временном каталоге; названия и имена вымышленные.
//!
//! Plan 41.2-09 добавляет правку акта (`update_*`): добавление позиций по тем же правилам,
//! удаление позиций возвращает членство и место.
//!
//! Plan 41.2-11 добавляет возвраты (`return_*`): якорь едет к месту возврата вместе с группой,
//! члены контейнеров освобождаются, правка возврата откатывается по эффектам возвратного акта.
//!
//! Префиксы тестов: `create_`, `update_`, `return_`.

use std::future::Future;
use std::time::Duration;

use rusqlite::params;
use rusqlite::types::Value;

use trackly_app::context::AppCtx;
use trackly_app::dto::act::{
    ActCreateDto, ActItemNewDto, ActNumberEditInput, ActReturnDto, ActReturnItemDto,
    ActSaveOutcome, ActUpdateDto, ActUpdateItemDto, ActUpdateReturnDto,
};
use trackly_app::dto::cartridge::{CartridgeCreateDto, CartridgeModelCreateDto};
use trackly_app::dto::number_template::NumberFieldInput;
use trackly_app::dto::printer::WsEvent;
use trackly_core::auth::Identity;
use trackly_core::error::AppError;
use trackly_infra::error_conversions::map_rusqlite;

const ON_WAREHOUSE: i64 = 1;
const IN_WORK: i64 = 2;
const GOLDEN_KIT: &str =
    "Корпус Mid Tower; SSD Kingston 480ГБ (инв. 114); БП 500W; Кулер R&D; ОЗУ 8ГБ (с/н AB123)";

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

/// Тест не должен зависнуть: любой сценарий укладывается в 30 секунд.
async fn timed<F: Future<Output = ()>>(f: F) {
    tokio::time::timeout(Duration::from_secs(30), f)
        .await
        .expect("тест не уложился в 30 секунд");
}

fn int(v: i64) -> Value {
    Value::Integer(v)
}

fn text(v: &str) -> Value {
    Value::Text(v.to_string())
}

async fn scalar_i64(ctx: &AppCtx, sql: &'static str, p: Vec<Value>) -> i64 {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(sql, rusqlite::params_from_iter(p), |r| r.get::<_, i64>(0))
                .map_err(map_rusqlite)
        })
        .await
        .expect("scalar query")
}

async fn opt_i64(ctx: &AppCtx, sql: &'static str, p: Vec<Value>) -> Option<i64> {
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

async fn opt_string(ctx: &AppCtx, sql: &'static str, p: Vec<Value>) -> Option<String> {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(sql, rusqlite::params_from_iter(p), |r| {
                r.get::<_, Option<String>>(0)
            })
            .map_err(map_rusqlite)
        })
        .await
        .expect("string query")
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
                params![format!("acts_t_{behavior}"), "Тип теста", behavior],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("type by behavior")
}

/// Живая группа контейнера напрямую в БД (мимо сервиса).
async fn seed_group(ctx: &AppCtx, name: &str, place: Option<i64>, parent: Option<i64>) -> i64 {
    let type_id = type_by_behavior(ctx, "container").await;
    let name = name.to_string();
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO groups (type_id, name, seq, place_id, parent_group_id, \
                 created_at_utc, updated_at_utc, version) \
                 VALUES (?1, ?2, (SELECT COALESCE(MAX(seq), 0) + 1 FROM groups WHERE type_id = ?1), \
                 ?3, ?4, 1700000000, 1700000000, 1)",
                params![type_id, name, place, parent],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed group")
}

struct Dev<'a> {
    name: &'a str,
    inv: Option<&'a str>,
    serial: Option<&'a str>,
    kit: Option<&'a str>,
    place: Option<i64>,
    status: i64,
    group: Option<i64>,
}

fn dev<'a>(name: &'a str, place: Option<i64>, group: Option<i64>) -> Dev<'a> {
    Dev {
        name,
        inv: None,
        serial: None,
        kit: None,
        place,
        status: ON_WAREHOUSE,
        group,
    }
}

async fn seed_device(ctx: &AppCtx, d: Dev<'_>) -> i64 {
    let name = d.name.to_string();
    let inv = d.inv.map(str::to_string);
    let serial = d.serial.map(str::to_string);
    let kit = d.kit.map(str::to_string);
    let (place, status, group) = (d.place, d.status, d.group);
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO devices (type_id, name, inventory_number, serial_number, complectation, \
                 place_id, status_id, created_at_utc, updated_at_utc, version) \
                 VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, 1700000000, 1700000000, 1)",
                params![name, inv, serial, kit, place, status],
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
                "UPDATE groups SET anchor_device_id = ?1 WHERE id = ?2",
                params![device, group],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("set anchor");
}

async fn device_place(ctx: &AppCtx, id: i64) -> Option<i64> {
    opt_i64(
        ctx,
        "SELECT place_id FROM devices WHERE id = ?1",
        vec![int(id)],
    )
    .await
}

async fn group_place(ctx: &AppCtx, id: i64) -> Option<i64> {
    opt_i64(
        ctx,
        "SELECT place_id FROM groups WHERE id = ?1",
        vec![int(id)],
    )
    .await
}

async fn member_rows(ctx: &AppCtx, device: i64) -> i64 {
    scalar_i64(
        ctx,
        "SELECT COUNT(*) FROM group_devices WHERE device_id = ?1",
        vec![int(device)],
    )
    .await
}

async fn group_members(ctx: &AppCtx, group: i64) -> i64 {
    scalar_i64(
        ctx,
        "SELECT COUNT(*) FROM group_devices WHERE group_id = ?1",
        vec![int(group)],
    )
    .await
}

async fn effects(ctx: &AppCtx, act: i64, kind: &str) -> i64 {
    scalar_i64(
        ctx,
        "SELECT COUNT(*) FROM act_group_effects WHERE act_id = ?1 AND kind = ?2",
        vec![int(act), text(kind)],
    )
    .await
}

/// Счётчики шести таблиц, которые запись акта меняет первой.
async fn counters(ctx: &AppCtx) -> Vec<i64> {
    ctx.writer
        .execute(move |conn| {
            let mut out = Vec::new();
            for t in [
                "acts",
                "act_items",
                "group_devices",
                "place_movements",
                "audit_log",
                "act_group_effects",
            ] {
                let n: i64 = conn
                    .query_row(&format!("SELECT COUNT(*) FROM {t}"), [], |r| r.get(0))
                    .map_err(map_rusqlite)?;
                out.push(n);
            }
            Ok(out)
        })
        .await
        .expect("counters")
}

fn dto(number: &str, ids: &[i64], place: Option<i64>, confirm: bool) -> ActCreateDto {
    ActCreateDto {
        number_input: NumberFieldInput {
            value: number.into(),
            template_id: None,
            confirm_mismatch: false,
            confirm_script_mix: false,
        },
        giver_name: "Иванов И.И.".into(),
        receiver_name: "Петров П.П.".into(),
        place_id: place,
        notes: None,
        deadline_utc: None,
        handover_date_utc: None,
        items: ids
            .iter()
            .map(|&id| ActItemNewDto {
                device_id: id,
                device_ids: vec![id],
                quantity: 1,
            })
            .collect(),
        confirm_group_release: confirm,
    }
}

async fn create_ok(ctx: &AppCtx, payload: ActCreateDto) -> i64 {
    match ctx.acts.create(&admin(), payload).await.expect("create") {
        ActSaveOutcome::Created(a) => a.id,
        other => panic!("ожидали Created, получили {other:?}"),
    }
}

async fn create_warning(ctx: &AppCtx, payload: ActCreateDto) -> Vec<(String, String)> {
    match ctx.acts.create(&admin(), payload).await.expect("create") {
        ActSaveOutcome::NeedsGroupRelease(w) => w
            .items
            .into_iter()
            .map(|i| (i.name, i.group_name))
            .collect(),
        other => panic!("ожидали NeedsGroupRelease, получили {other:?}"),
    }
}

/// Контейнер «АРМ #3» из трёх устройств на `place`.
struct Arm {
    group: i64,
    devices: Vec<i64>,
}

async fn seed_arm(ctx: &AppCtx, place: i64) -> Arm {
    seed_arm_named(ctx, place, "АРМ #3").await
}

async fn seed_arm_named(ctx: &AppCtx, place: i64, group_name: &str) -> Arm {
    let group = seed_group(ctx, group_name, Some(place), None).await;
    let mut devices = Vec::new();
    for name in ["Системный блок #4", "Монитор #2", "Клавиатура #5"] {
        devices.push(seed_device(ctx, dev(name, Some(place), Some(group))).await);
    }
    Arm { group, devices }
}

// ---------------------------------------------------------------------------
// (a) состав группы целиком
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_whole_group_moves_it_without_confirmation() {
    timed(async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Склад А").await;
        let b = seed_place(&ctx, "Склад Б").await;
        let arm = seed_arm(&ctx, a).await;
        // Одно из устройств уже «В работе»: состав группы принимается независимо от статуса.
        let working = arm.devices[1];
        ctx.writer
            .execute(move |conn| {
                conn.execute(
                    "UPDATE devices SET status_id = ?1 WHERE id = ?2",
                    params![IN_WORK, working],
                )
                .map_err(map_rusqlite)?;
                Ok(())
            })
            .await
            .expect("set in work");

        let act = create_ok(&ctx, dto("G-A1", &arm.devices, Some(b), false)).await;

        assert_eq!(group_place(&ctx, arm.group).await, Some(b));
        for d in &arm.devices {
            assert_eq!(member_rows(&ctx, *d).await, 1, "состав цел");
            assert_eq!(device_place(&ctx, *d).await, Some(b));
        }
        assert_eq!(
            scalar_i64(
                &ctx,
                "SELECT COUNT(*) FROM act_items WHERE act_id = ?1",
                vec![int(act)]
            )
            .await,
            3
        );
        assert_eq!(effects(&ctx, act, "move").await, 1);
        assert_eq!(effects(&ctx, act, "release").await, 0);
    })
    .await;
}

// ---------------------------------------------------------------------------
// (b) журнал пакета
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_batch_journal_rows_carry_act_and_one_batch() {
    timed(async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Склад А").await;
        let b = seed_place(&ctx, "Склад Б").await;
        let group = seed_group(&ctx, "АРМ #6", Some(a), None).await;
        let unit = seed_device(&ctx, dev("Системный блок #4", Some(a), Some(group))).await;
        let monitor = seed_device(&ctx, dev("Монитор #2", Some(a), Some(group))).await;
        let printer = seed_device(&ctx, dev("Принтер #1", Some(a), Some(group))).await;
        ctx.writer
            .execute(move |conn| {
                conn.execute(
                    "UPDATE devices SET type_id = 2 WHERE id = ?1",
                    params![printer],
                )
                .map_err(map_rusqlite)?;
                Ok(())
            })
            .await
            .expect("make printer");
        let model = ctx
            .cartridges
            .model_create(CartridgeModelCreateDto {
                brand: "HP".into(),
                model: "CE285A".into(),
                kind_id: 1,
                color: Some("Чёрный".into()),
                notes: None,
                compatibility: vec![],
            })
            .await
            .expect("model")
            .id;
        let cartridge = ctx
            .cartridges
            .create(CartridgeCreateDto {
                model_id: model,
                number_input: NumberFieldInput {
                    value: "C-ACTG-00001".into(),
                    template_id: None,
                    confirm_mismatch: false,
                    confirm_script_mix: false,
                },
                state_id: Some(1),
                place_id: Some(a),
                notes: None,
            })
            .await
            .expect("cartridge")
            .expect_created("cartridge")
            .id;
        ctx.writer
            .execute(move |conn| {
                conn.execute(
                    "UPDATE cartridges SET current_printer_device_id = ?1 WHERE id = ?2",
                    params![printer, cartridge],
                )
                .map_err(map_rusqlite)?;
                Ok(())
            })
            .await
            .expect("attach cartridge");
        let single = seed_device(&ctx, dev("Ноутбук #1", Some(a), None)).await;

        let mut all = vec![unit, monitor, printer, single];
        all.sort_unstable();
        let act = create_ok(&ctx, dto("G-B1", &all, Some(b), false)).await;

        // Все строки акта: источник act, ровно одна группа batch_id у пакета.
        let total = scalar_i64(
            &ctx,
            "SELECT COUNT(*) FROM place_movements WHERE act_id = ?1",
            vec![int(act)],
        )
        .await;
        assert_eq!(total, 6, "группа, три устройства, картридж и одиночное");
        assert_eq!(
            scalar_i64(
                &ctx,
                "SELECT COUNT(*) FROM place_movements WHERE act_id = ?1 AND source <> 'act'",
                vec![int(act)]
            )
            .await,
            0
        );
        assert_eq!(
            scalar_i64(
                &ctx,
                "SELECT COUNT(DISTINCT batch_id) FROM place_movements \
                 WHERE act_id = ?1 AND batch_id IS NOT NULL",
                vec![int(act)]
            )
            .await,
            1,
            "у пакета ровно один batch_id"
        );
        assert_eq!(
            scalar_i64(
                &ctx,
                "SELECT COUNT(*) FROM place_movements WHERE act_id = ?1 AND batch_id IS NOT NULL",
                vec![int(act)]
            )
            .await,
            5,
            "в пакете группа, три устройства и картридж"
        );
        assert_eq!(
            scalar_i64(
                &ctx,
                "SELECT COUNT(*) FROM place_movements WHERE act_id = ?1 \
                 AND entity_type = 'cartridge' AND entity_id = ?2 AND batch_id IS NOT NULL",
                vec![int(act), int(cartridge)]
            )
            .await,
            1,
            "картридж принтера-члена в пакете"
        );
        // Одиночное устройство: act_id без batch_id.
        assert_eq!(
            scalar_i64(
                &ctx,
                "SELECT COUNT(*) FROM place_movements WHERE act_id = ?1 \
                 AND entity_type = 'device' AND entity_id = ?2 AND batch_id IS NULL",
                vec![int(act), int(single)]
            )
            .await,
            1
        );
        // На устройство ровно одна строка.
        for d in &all {
            assert_eq!(
                scalar_i64(
                    &ctx,
                    "SELECT COUNT(*) FROM place_movements \
                     WHERE entity_type = 'device' AND entity_id = ?1",
                    vec![int(*d)]
                )
                .await,
                1,
                "устройство {d}: одна строка журнала"
            );
        }
    })
    .await;
}

// ---------------------------------------------------------------------------
// (c) часть состава: подтверждение без записей и вынос с флагом
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_partial_group_asks_first_then_releases_with_flag() {
    timed(async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Склад А").await;
        let b = seed_place(&ctx, "Склад Б").await;
        let arm = seed_arm(&ctx, a).await;
        let two = vec![arm.devices[0], arm.devices[1]];

        let before = counters(&ctx).await;
        let items = create_warning(&ctx, dto("G-C1", &two, Some(b), false)).await;
        assert_eq!(items.len(), 2);
        assert!(items.contains(&("Системный блок #4".to_string(), "АРМ #3".to_string())));
        assert!(items.contains(&("Монитор #2".to_string(), "АРМ #3".to_string())));
        assert_eq!(
            counters(&ctx).await,
            before,
            "подтверждение: ни одной записи"
        );
        assert_eq!(device_place(&ctx, arm.devices[0]).await, Some(a));

        let act = create_ok(&ctx, dto("G-C1", &two, Some(b), true)).await;
        assert_eq!(member_rows(&ctx, arm.devices[0]).await, 0);
        assert_eq!(member_rows(&ctx, arm.devices[1]).await, 0);
        assert_eq!(
            member_rows(&ctx, arm.devices[2]).await,
            1,
            "третье осталось"
        );
        assert_eq!(
            group_place(&ctx, arm.group).await,
            Some(a),
            "место группы не менялось"
        );
        assert_eq!(device_place(&ctx, arm.devices[0]).await, Some(b));
        assert_eq!(effects(&ctx, act, "release").await, 2);
        assert_eq!(effects(&ctx, act, "move").await, 0);
        assert_eq!(
            scalar_i64(
                &ctx,
                "SELECT COUNT(*) FROM audit_log WHERE action = 'custom:group_member_released'",
                vec![]
            )
            .await,
            2,
            "вынос аудируется"
        );
    })
    .await;
}

// ---------------------------------------------------------------------------
// (d) якорь: одна позиция, комплектация деталей
// ---------------------------------------------------------------------------

struct AnchorFixture {
    group: i64,
    anchor: i64,
    parts: Vec<i64>,
}

async fn seed_anchor_group(ctx: &AppCtx, place: i64) -> AnchorFixture {
    seed_anchor_group_with(ctx, place, "Сборка #4", ["114", "SN-SSD-1", "AB123"]).await
}

/// Тот же набор с другими уникальными полями: в одной базе нужны две якорные группы.
async fn seed_anchor_group_with(
    ctx: &AppCtx,
    place: i64,
    group_name: &str,
    [inv, ssd_serial, ram_serial]: [&str; 3],
) -> AnchorFixture {
    let group = seed_group(ctx, group_name, Some(place), None).await;
    let anchor = seed_device(
        ctx,
        Dev {
            kit: Some("Корпус Mid Tower"),
            ..dev("Системный блок #4", Some(place), Some(group))
        },
    )
    .await;
    set_anchor(ctx, group, anchor).await;
    let mut parts = Vec::new();
    parts.push(
        seed_device(
            ctx,
            Dev {
                inv: Some(inv),
                serial: Some(ssd_serial),
                ..dev("SSD Kingston 480ГБ", Some(place), Some(group))
            },
        )
        .await,
    );
    parts.push(
        seed_device(
            ctx,
            Dev {
                serial: Some(ram_serial),
                ..dev("ОЗУ 8ГБ", Some(place), Some(group))
            },
        )
        .await,
    );
    parts.push(seed_device(ctx, dev("БП 500W", Some(place), Some(group))).await);
    parts.push(seed_device(ctx, dev("Кулер R&D", Some(place), Some(group))).await);
    AnchorFixture {
        group,
        anchor,
        parts,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_anchor_goes_as_one_item_with_parts_kit() {
    timed(async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Склад А").await;
        let b = seed_place(&ctx, "Склад Б").await;
        let f = seed_anchor_group(&ctx, a).await;

        let act = create_ok(&ctx, dto("G-D1", &[f.anchor], Some(b), false)).await;

        assert_eq!(
            scalar_i64(
                &ctx,
                "SELECT COUNT(*) FROM act_items WHERE act_id = ?1",
                vec![int(act)]
            )
            .await,
            1,
            "одна позиция — якорь"
        );
        assert_eq!(
            scalar_i64(
                &ctx,
                "SELECT COUNT(*) FROM act_items WHERE act_id = ?1 AND device_id = ?2",
                vec![int(act), int(f.anchor)]
            )
            .await,
            1
        );
        assert_eq!(group_place(&ctx, f.group).await, Some(b));
        assert_eq!(device_place(&ctx, f.anchor).await, Some(b));
        for p in &f.parts {
            assert_eq!(
                device_place(&ctx, *p).await,
                Some(b),
                "деталь поехала с группой"
            );
            assert_eq!(member_rows(&ctx, *p).await, 1, "членство цело");
            assert_eq!(
                scalar_i64(
                    &ctx,
                    "SELECT status_id FROM devices WHERE id = ?1",
                    vec![int(*p)]
                )
                .await,
                ON_WAREHOUSE,
                "статус детали не менялся"
            );
        }
        assert_eq!(member_rows(&ctx, f.anchor).await, 1);
        assert_eq!(group_members(&ctx, f.group).await, 5);
        assert_eq!(
            opt_string(
                &ctx,
                "SELECT complectation_at_time FROM act_items WHERE act_id = ?1",
                vec![int(act)]
            )
            .await
            .as_deref(),
            Some(GOLDEN_KIT)
        );
    })
    .await;
}

// ---------------------------------------------------------------------------
// (e) статус-гейт
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_status_gate_accepts_group_member_but_not_lone_in_work() {
    timed(async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Склад А").await;
        let b = seed_place(&ctx, "Склад Б").await;
        let group = seed_group(&ctx, "АРМ #3", Some(a), None).await;
        let member = seed_device(
            &ctx,
            Dev {
                status: IN_WORK,
                ..dev("Монитор #2", Some(a), Some(group))
            },
        )
        .await;
        let lone = seed_device(
            &ctx,
            Dev {
                status: IN_WORK,
                ..dev("Ноутбук #1", Some(a), None)
            },
        )
        .await;

        let err = ctx
            .acts
            .create(&admin(), dto("G-E1", &[lone], Some(b), false))
            .await
            .expect_err("одиночное «В работе» не принимается");
        assert!(matches!(err, AppError::Conflict { .. }), "{err:?}");
        assert_eq!(device_place(&ctx, lone).await, Some(a));

        create_ok(&ctx, dto("G-E2", &[member], Some(b), false)).await;
        assert_eq!(device_place(&ctx, member).await, Some(b));
    })
    .await;
}

// ---------------------------------------------------------------------------
// (f) место обязательно для переноса группы
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_group_move_requires_place_but_release_does_not() {
    timed(async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Склад А").await;
        let arm = seed_arm(&ctx, a).await;

        let before = counters(&ctx).await;
        let err = ctx
            .acts
            .create(&admin(), dto("G-F1", &arm.devices, None, false))
            .await
            .expect_err("для группы нужно место");
        match err {
            AppError::Validation { field, .. } => assert_eq!(field, "place_id"),
            other => panic!("ожидали Validation, получили {other:?}"),
        }
        assert_eq!(counters(&ctx).await, before, "ничего не записано");

        // Вынос части состава без места проходит как раньше.
        let act = create_ok(&ctx, dto("G-F2", &arm.devices[..1], None, true)).await;
        assert_eq!(effects(&ctx, act, "release").await, 1);
        assert_eq!(member_rows(&ctx, arm.devices[0]).await, 0);
    })
    .await;
}

// ---------------------------------------------------------------------------
// (g) вложенная якорная группа при непокрытом контейнере
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_nested_anchor_group_is_detached_after_confirmation() {
    timed(async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Склад А").await;
        let b = seed_place(&ctx, "Склад Б").await;
        let container = seed_group(&ctx, "АРМ #3", Some(a), None).await;
        let monitor = seed_device(&ctx, dev("Монитор #2", Some(a), Some(container))).await;
        let nested = seed_group(&ctx, "Замена #5", Some(a), Some(container)).await;
        let laptop = seed_device(&ctx, dev("Ноутбук #5", Some(a), Some(nested))).await;
        set_anchor(&ctx, nested, laptop).await;
        let adapter = seed_device(&ctx, dev("Адаптер #5", Some(a), Some(nested))).await;

        let before = counters(&ctx).await;
        let items = create_warning(&ctx, dto("G-G1", &[laptop], Some(b), false)).await;
        assert_eq!(items, vec![("Замена #5".to_string(), "АРМ #3".to_string())]);
        assert_eq!(counters(&ctx).await, before);

        let act = create_ok(&ctx, dto("G-G1", &[laptop], Some(b), true)).await;
        assert_eq!(
            opt_i64(
                &ctx,
                "SELECT parent_group_id FROM groups WHERE id = ?1",
                vec![int(nested)]
            )
            .await,
            None,
            "вложенная группа отцеплена"
        );
        assert_eq!(group_place(&ctx, nested).await, Some(b));
        assert_eq!(device_place(&ctx, adapter).await, Some(b));
        assert_eq!(group_place(&ctx, container).await, Some(a));
        assert_eq!(device_place(&ctx, monitor).await, Some(a));
        assert_eq!(effects(&ctx, act, "detach").await, 1);
    })
    .await;
}

// ---------------------------------------------------------------------------
// (h) пара «до/после» на одной фикстуре
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_full_set_moves_but_minus_one_asks_on_same_fixture() {
    timed(async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Склад А").await;
        let b = seed_place(&ctx, "Склад Б").await;
        let arm = seed_arm(&ctx, a).await;

        let minus_one = &arm.devices[..2];
        let items = create_warning(&ctx, dto("G-H1", minus_one, Some(b), false)).await;
        assert_eq!(items.len(), 2, "без одного устройства — подтверждение");
        assert_eq!(group_place(&ctx, arm.group).await, Some(a));

        let act = create_ok(&ctx, dto("G-H1", &arm.devices, Some(b), false)).await;
        assert_eq!(
            group_place(&ctx, arm.group).await,
            Some(b),
            "весь состав — перенос"
        );
        assert_eq!(effects(&ctx, act, "move").await, 1);
        assert_eq!(effects(&ctx, act, "release").await, 0);
        assert_eq!(group_members(&ctx, arm.group).await, 3);
    })
    .await;
}

// ---------------------------------------------------------------------------
// Правка акта (plan 41.2-09)
// ---------------------------------------------------------------------------

async fn act_version(ctx: &AppCtx, act_id: i64) -> i64 {
    ctx.acts.get(act_id).await.expect("get act").version
}

async fn update_outcome(
    ctx: &AppCtx,
    act_id: i64,
    ids: &[i64],
    confirm: bool,
) -> Result<ActSaveOutcome, AppError> {
    let act = ctx.acts.get(act_id).await.expect("get act");
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
                items: ids
                    .iter()
                    .map(|&id| ActUpdateItemDto {
                        device_id: id,
                        complectation_at_time: None,
                    })
                    .collect(),
                confirm_group_release: confirm,
            },
        )
        .await
}

async fn update_ok(ctx: &AppCtx, act_id: i64, ids: &[i64], confirm: bool) {
    match update_outcome(ctx, act_id, ids, confirm)
        .await
        .expect("update")
    {
        ActSaveOutcome::Created(_) => {}
        other => panic!("ожидали Created, получили {other:?}"),
    }
}

async fn update_warning(ctx: &AppCtx, act_id: i64, ids: &[i64]) -> Vec<(String, String)> {
    match update_outcome(ctx, act_id, ids, false)
        .await
        .expect("update")
    {
        ActSaveOutcome::NeedsGroupRelease(w) => w
            .items
            .into_iter()
            .map(|i| (i.name, i.group_name))
            .collect(),
        other => panic!("ожидали NeedsGroupRelease, получили {other:?}"),
    }
}

async fn act_item_rows(ctx: &AppCtx, act: i64, device: i64) -> i64 {
    scalar_i64(
        ctx,
        "SELECT COUNT(*) FROM act_items WHERE act_id = ?1 AND device_id = ?2",
        vec![int(act), int(device)],
    )
    .await
}

async fn device_effects(ctx: &AppCtx, act: i64, kind: &str, device: i64) -> i64 {
    scalar_i64(
        ctx,
        "SELECT COUNT(*) FROM act_group_effects WHERE act_id = ?1 AND kind = ?2 AND device_id = ?3",
        vec![int(act), text(kind), int(device)],
    )
    .await
}

async fn audit_rows(ctx: &AppCtx, device: i64, action: &str) -> i64 {
    scalar_i64(
        ctx,
        "SELECT COUNT(*) FROM audit_log WHERE entity_type = 'device' AND entity_id = ?1 AND action = ?2",
        vec![int(device), text(action)],
    )
    .await
}

async fn group_version(ctx: &AppCtx, group: i64) -> i64 {
    scalar_i64(
        ctx,
        "SELECT version FROM groups WHERE id = ?1",
        vec![int(group)],
    )
    .await
}

// (u1) добавление правкой подчиняется тому же правилу, что создание.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_added_whole_group_moves_and_minus_one_asks_on_same_fixture() {
    timed(async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Склад А").await;
        let b = seed_place(&ctx, "Склад Б").await;
        let solo = seed_device(&ctx, dev("Ноутбук #1", Some(a), None)).await;
        let arm = seed_arm(&ctx, a).await;
        let arm2 = seed_arm_named(&ctx, a, "АРМ #4").await;
        // Одно из устройств уже «В работе»: состав группы принимается независимо от статуса.
        let working = arm.devices[1];
        ctx.writer
            .execute(move |conn| {
                conn.execute(
                    "UPDATE devices SET status_id = ?1 WHERE id = ?2",
                    params![IN_WORK, working],
                )
                .map_err(map_rusqlite)?;
                Ok(())
            })
            .await
            .expect("set in work");
        let act = create_ok(&ctx, dto("U-1", &[solo], Some(b), false)).await;

        // Весь состав: группа переезжает, подтверждения нет.
        let mut with_arm = vec![solo];
        with_arm.extend(&arm.devices);
        update_ok(&ctx, act, &with_arm, false).await;
        assert_eq!(group_place(&ctx, arm.group).await, Some(b));
        assert_eq!(group_members(&ctx, arm.group).await, 3, "состав цел");
        assert_eq!(effects(&ctx, act, "move").await, 1);
        assert_eq!(effects(&ctx, act, "release").await, 0);

        // Та же правка без одного устройства второй группы: подтверждение, ничего не записано.
        let mut minus_one = with_arm.clone();
        minus_one.extend(&arm2.devices[..2]);
        let before = counters(&ctx).await;
        let items = update_warning(&ctx, act, &minus_one).await;
        assert_eq!(items.len(), 2, "оба устройства выносятся из «АРМ #4»");
        assert_eq!(
            counters(&ctx).await,
            before,
            "до подтверждения ничего не записано"
        );
        assert_eq!(group_place(&ctx, arm2.group).await, Some(a));

        // С подтверждением: устройства выведены из состава, след release записан.
        update_ok(&ctx, act, &minus_one, true).await;
        assert_eq!(member_rows(&ctx, arm2.devices[0]).await, 0);
        assert_eq!(member_rows(&ctx, arm2.devices[1]).await, 0);
        assert_eq!(member_rows(&ctx, arm2.devices[2]).await, 1);
        assert_eq!(effects(&ctx, act, "release").await, 2);
        assert_eq!(device_place(&ctx, arm2.devices[0]).await, Some(b));
    })
    .await;
}

// (u2) убраны все позиции переехавшей группы: группа вернулась на прежнее место.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_removing_whole_group_returns_it_to_the_old_place() {
    timed(async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Склад А").await;
        let b = seed_place(&ctx, "Склад Б").await;
        let solo = seed_device(&ctx, dev("Ноутбук #1", Some(a), None)).await;
        let arm = seed_arm(&ctx, a).await;
        let mut all = vec![solo];
        all.extend(&arm.devices);
        let act = create_ok(&ctx, dto("U-2", &all, Some(b), false)).await;
        assert_eq!(group_place(&ctx, arm.group).await, Some(b));
        assert_eq!(effects(&ctx, act, "move").await, 1);

        update_ok(&ctx, act, &[solo], false).await;

        assert_eq!(
            group_place(&ctx, arm.group).await,
            Some(a),
            "группа вернулась"
        );
        assert_eq!(effects(&ctx, act, "move").await, 0, "эффект переноса снят");
        assert_eq!(
            group_members(&ctx, arm.group).await,
            3,
            "состав не разобран"
        );
        for d in &arm.devices {
            assert_eq!(device_place(&ctx, *d).await, Some(a));
            assert_eq!(act_item_rows(&ctx, act, *d).await, 0);
            assert_eq!(
                scalar_i64(
                    &ctx,
                    "SELECT status_id FROM devices WHERE id = ?1",
                    vec![int(*d)]
                )
                .await,
                ON_WAREHOUSE
            );
        }
        assert_eq!(
            device_place(&ctx, solo).await,
            Some(b),
            "оставшаяся позиция на месте акта"
        );
    })
    .await;
}

// (u3) убрана одна позиция переехавшей группы: устройство выведено, след release записан.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_removing_one_of_moved_group_releases_it_and_records_the_effect() {
    timed(async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Склад А").await;
        let b = seed_place(&ctx, "Склад Б").await;
        let solo = seed_device(&ctx, dev("Ноутбук #1", Some(a), None)).await;
        let arm = seed_arm(&ctx, a).await;
        let mut all = vec![solo];
        all.extend(&arm.devices);
        let act = create_ok(&ctx, dto("U-3", &all, Some(b), false)).await;
        let gone = arm.devices[2];

        let mut rest = vec![solo];
        rest.extend(&arm.devices[..2]);
        update_ok(&ctx, act, &rest, false).await;

        assert_eq!(
            member_rows(&ctx, gone).await,
            0,
            "устройство выведено из группы"
        );
        assert_eq!(
            device_place(&ctx, gone).await,
            Some(a),
            "стоит на исходном месте"
        );
        assert_eq!(group_members(&ctx, arm.group).await, 2);
        assert_eq!(
            group_place(&ctx, arm.group).await,
            Some(b),
            "группа осталась на месте акта"
        );
        for d in &arm.devices[..2] {
            assert_eq!(device_place(&ctx, *d).await, Some(b));
        }
        assert_eq!(effects(&ctx, act, "move").await, 1);
        // W-7: след release записан правкой и НЕ потрачен возвратом членства.
        assert_eq!(device_effects(&ctx, act, "release", gone).await, 1);
        assert_eq!(
            audit_rows(&ctx, gone, "custom:group_member_restored").await,
            0
        );
    })
    .await;
}

// (u4) вынесенное актом устройство вернулось в группу при удалении позиции.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_removing_a_released_member_puts_it_back_into_its_group() {
    timed(async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Склад А").await;
        let b = seed_place(&ctx, "Склад Б").await;
        let solo = seed_device(&ctx, dev("Ноутбук #1", Some(a), None)).await;
        let arm = seed_arm(&ctx, a).await;
        let (x, y) = (arm.devices[0], arm.devices[1]);
        let act = create_ok(&ctx, dto("U-4", &[solo, x, y], Some(b), true)).await;
        assert_eq!(member_rows(&ctx, x).await, 0);
        assert_eq!(device_effects(&ctx, act, "release", x).await, 1);

        update_ok(&ctx, act, &[solo, y], false).await;

        assert_eq!(member_rows(&ctx, x).await, 1, "x вернулся в группу");
        assert_eq!(device_place(&ctx, x).await, Some(a));
        assert_eq!(
            device_effects(&ctx, act, "release", x).await,
            0,
            "эффект потрачен"
        );
        assert_eq!(audit_rows(&ctx, x, "custom:group_member_restored").await, 1);
        assert_eq!(member_rows(&ctx, y).await, 0, "y по-прежнему вне группы");
        assert_eq!(device_effects(&ctx, act, "release", y).await, 1);
    })
    .await;
}

// (u5) то же, но группа удалена до правки: правка проходит, членство не восстановлено.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_removing_a_released_member_of_a_deleted_group_still_passes() {
    timed(async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Склад А").await;
        let b = seed_place(&ctx, "Склад Б").await;
        let solo = seed_device(&ctx, dev("Ноутбук #1", Some(a), None)).await;
        let arm = seed_arm(&ctx, a).await;
        let (x, y) = (arm.devices[0], arm.devices[1]);
        let act = create_ok(&ctx, dto("U-5", &[solo, x, y], Some(b), true)).await;
        let group = arm.group;
        ctx.writer
            .execute(move |conn| {
                conn.execute(
                    "UPDATE groups SET deleted_at_utc = 1700000100 WHERE id = ?1",
                    params![group],
                )
                .map_err(map_rusqlite)?;
                Ok(())
            })
            .await
            .expect("soft delete group");

        update_ok(&ctx, act, &[solo, y], false).await;

        assert_eq!(device_place(&ctx, x).await, Some(a), "x на своём месте");
        assert_eq!(member_rows(&ctx, x).await, 0, "членство не восстановлено");
        assert_eq!(audit_rows(&ctx, x, "custom:group_member_restored").await, 0);
        assert_eq!(
            device_effects(&ctx, act, "release", x).await,
            0,
            "эффект удалён"
        );
        assert_eq!(act_item_rows(&ctx, act, x).await, 0);
    })
    .await;
}

// (u6) убран якорь: группа и детали вернулись на прежнее место.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_removing_the_anchor_returns_group_and_parts() {
    timed(async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Склад А").await;
        let b = seed_place(&ctx, "Склад Б").await;
        let solo = seed_device(&ctx, dev("Ноутбук #1", Some(a), None)).await;
        let f = seed_anchor_group(&ctx, a).await;
        let act = create_ok(&ctx, dto("U-6", &[solo, f.anchor], Some(b), false)).await;
        assert_eq!(group_place(&ctx, f.group).await, Some(b));
        assert_eq!(effects(&ctx, act, "move").await, 1);

        update_ok(&ctx, act, &[solo], false).await;

        assert_eq!(group_place(&ctx, f.group).await, Some(a));
        assert_eq!(effects(&ctx, act, "move").await, 0);
        assert_eq!(device_place(&ctx, f.anchor).await, Some(a));
        assert_eq!(group_members(&ctx, f.group).await, 5, "состав цел");
        for p in &f.parts {
            assert_eq!(device_place(&ctx, *p).await, Some(a), "деталь вернулась");
            assert_eq!(
                act_item_rows(&ctx, act, *p).await,
                0,
                "у детали нет позиции акта"
            );
        }
        assert_eq!(act_item_rows(&ctx, act, f.anchor).await, 0);
    })
    .await;
}

// (u7) статус-гейт добавленных позиций.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_status_gate_accepts_group_member_but_not_lone_in_work() {
    timed(async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Склад А").await;
        let b = seed_place(&ctx, "Склад Б").await;
        let solo = seed_device(&ctx, dev("Ноутбук #1", Some(a), None)).await;
        let group = seed_group(&ctx, "АРМ #3", Some(a), None).await;
        let member = seed_device(
            &ctx,
            Dev {
                status: IN_WORK,
                ..dev("Монитор #2", Some(a), Some(group))
            },
        )
        .await;
        let lone = seed_device(
            &ctx,
            Dev {
                status: IN_WORK,
                ..dev("Ноутбук #9", Some(a), None)
            },
        )
        .await;
        let act = create_ok(&ctx, dto("U-7", &[solo], Some(b), false)).await;

        let before = counters(&ctx).await;
        let err = update_outcome(&ctx, act, &[solo, lone], false)
            .await
            .expect_err("одиночное «В работе» не принимается");
        assert!(matches!(err, AppError::Conflict { .. }), "{err:?}");
        assert_eq!(counters(&ctx).await, before);
        assert_eq!(device_place(&ctx, lone).await, Some(a));

        update_ok(&ctx, act, &[solo, member], false).await;
        assert_eq!(device_place(&ctx, member).await, Some(b));
        assert_eq!(act_item_rows(&ctx, act, member).await, 1);
    })
    .await;
}

// (u8) событие после правки несёт группы затронутых групп.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_broadcast_carries_the_moved_group() {
    timed(async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Склад А").await;
        let b = seed_place(&ctx, "Склад Б").await;
        let solo = seed_device(&ctx, dev("Ноутбук #1", Some(a), None)).await;
        let arm = seed_arm(&ctx, a).await;
        let act = create_ok(&ctx, dto("U-8", &[solo], Some(b), false)).await;
        let mut rx = ctx.ws_broadcast.subscribe();
        while rx.try_recv().is_ok() {}

        let mut all = vec![solo];
        all.extend(&arm.devices);
        update_ok(&ctx, act, &all, false).await;

        let mut seen: Vec<i64> = Vec::new();
        while let Ok(ev) = rx.try_recv() {
            if let WsEvent::EntitiesChanged { group_ids, .. } = ev {
                seen.extend(group_ids);
            }
        }
        assert!(
            seen.contains(&arm.group),
            "переехавшая группа в group_ids: {seen:?}"
        );
    })
    .await;
}

// (u9) якорную группу переместили ПОСЛЕ акта: правка проходит, группа остаётся на месте.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_removing_anchor_of_a_group_moved_after_the_act_keeps_the_group_in_place() {
    timed(async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Склад А").await;
        let b = seed_place(&ctx, "Склад Б").await;
        let c = seed_place(&ctx, "Склад В").await;
        let solo = seed_device(&ctx, dev("Ноутбук #1", Some(a), None)).await;
        let f = seed_anchor_group(&ctx, a).await;
        let act = create_ok(&ctx, dto("U-9", &[solo, f.anchor], Some(b), false)).await;
        let version = group_version(&ctx, f.group).await;
        ctx.groups
            .move_group(&admin(), f.group, version, c)
            .await
            .expect("move group after the act");

        update_ok(&ctx, act, &[solo], false).await;

        assert_eq!(
            group_place(&ctx, f.group).await,
            Some(c),
            "группа осталась в третьем месте"
        );
        assert_eq!(device_place(&ctx, f.anchor).await, Some(c));
        for p in &f.parts {
            assert_eq!(device_place(&ctx, *p).await, Some(c));
        }
        assert_eq!(member_rows(&ctx, f.anchor).await, 1, "членство цело");
        assert_eq!(act_item_rows(&ctx, act, f.anchor).await, 0);
        assert_eq!(
            audit_rows(&ctx, f.anchor, "custom:group_restore_skipped").await,
            1
        );
        let group_id = f.group;
        assert_eq!(
            scalar_i64(
                &ctx,
                "SELECT COUNT(*) FROM audit_log WHERE entity_id = ?1 \
                 AND action = 'custom:group_restore_skipped' \
                 AND json_extract(payload_json, '$.group_id') = ?2 \
                 AND json_extract(payload_json, '$.act_id') = ?3 \
                 AND json_extract(payload_json, '$.reason') IS NOT NULL",
                vec![int(f.anchor), int(group_id), int(act)]
            )
            .await,
            1,
            "причина пропуска записана в аудит"
        );
    })
    .await;
}

// (u10) контейнер переехал актом и был перемещён после: убранное устройство выводится.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_removing_one_of_a_container_moved_after_the_act_releases_it() {
    timed(async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Склад А").await;
        let b = seed_place(&ctx, "Склад Б").await;
        let c = seed_place(&ctx, "Склад В").await;
        let solo = seed_device(&ctx, dev("Ноутбук #1", Some(a), None)).await;
        let arm = seed_arm(&ctx, a).await;
        let mut all = vec![solo];
        all.extend(&arm.devices);
        let act = create_ok(&ctx, dto("U-10", &all, Some(b), false)).await;
        let version = group_version(&ctx, arm.group).await;
        ctx.groups
            .move_group(&admin(), arm.group, version, c)
            .await
            .expect("move group after the act");
        let gone = arm.devices[0];

        let mut rest = vec![solo];
        rest.extend(&arm.devices[1..]);
        update_ok(&ctx, act, &rest, false).await;

        assert_eq!(
            device_place(&ctx, gone).await,
            Some(a),
            "убранное устройство на исходном месте"
        );
        assert_eq!(
            member_rows(&ctx, gone).await,
            0,
            "выведено из перемещённой группы"
        );
        assert_eq!(group_members(&ctx, arm.group).await, 2);
        assert_eq!(group_place(&ctx, arm.group).await, Some(c));
        for d in &arm.devices[1..] {
            assert_eq!(device_place(&ctx, *d).await, Some(c));
        }
        assert_eq!(
            device_effects(&ctx, act, "release", gone).await,
            1,
            "режим Record"
        );
    })
    .await;
}

// (u11) следы деталей снятой группы отвязаны от акта: удаление акта не затирает их правки.
// Контроль расхождения — первое утверждение: до правки каждая деталь несёт след акта, и именно
// его читает удаление акта. Контроль через `delete_soft` акта с якорем без правки недоступен
// до плана 41.2-13 (путь удаления ещё отклоняет якорь).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_removing_the_anchor_detaches_part_traces_from_the_act() {
    timed(async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Склад А").await;
        let b = seed_place(&ctx, "Склад Б").await;
        let solo = seed_device(&ctx, dev("Ноутбук #1", Some(a), None)).await;
        let f = seed_anchor_group(&ctx, a).await;
        let act = create_ok(&ctx, dto("U-11", &[solo, f.anchor], Some(b), false)).await;
        let parts_json = serde_json::to_string(&f.parts).expect("ids");
        let traces = |act: i64, parts_json: String| {
            let ctx = &ctx;
            async move {
                ctx.writer
                    .execute(move |conn| {
                        conn.query_row(
                            "SELECT COUNT(*) FROM audit_log WHERE entity_type = 'device' \
                             AND json_extract(payload_json, '$.act_id') = ?1 \
                             AND entity_id IN (SELECT value FROM json_each(?2))",
                            params![act, parts_json],
                            |r| r.get::<_, i64>(0),
                        )
                        .map_err(map_rusqlite)
                    })
                    .await
                    .expect("traces")
            }
        };
        assert_eq!(
            traces(act, parts_json.clone()).await,
            f.parts.len() as i64,
            "до правки каждая деталь несёт след акта"
        );

        update_ok(&ctx, act, &[solo], false).await;

        assert_eq!(traces(act, parts_json.clone()).await, 0, "следы отвязаны");
        let mut reverted = 0;
        for p in &f.parts {
            reverted += audit_rows(&ctx, *p, "custom:group_moved_by_act_reverted").await;
        }
        assert_eq!(reverted, f.parts.len() as i64);

        // Поздняя правка детали переживает удаление акта.
        let part = f.parts[0];
        ctx.writer
            .execute(move |conn| {
                conn.execute(
                    "UPDATE devices SET complectation = 'Позднее изменение' WHERE id = ?1",
                    params![part],
                )
                .map_err(map_rusqlite)?;
                Ok(())
            })
            .await
            .expect("late edit");
        let version = act_version(&ctx, act).await;
        ctx.acts
            .delete_soft(act, version)
            .await
            .expect("delete act");
        assert_eq!(
            opt_string(
                &ctx,
                "SELECT complectation FROM devices WHERE id = ?1",
                vec![int(part)]
            )
            .await
            .as_deref(),
            Some("Позднее изменение"),
            "правка детали не затёрта устаревшим снимком"
        );
    })
    .await;
}

// (u12) деталь якорной группы, которую переместил сам акт, вписана в акт отдельной позицией;
// правка убирает её: это не «группу переместили после акта», деталь выводится из состава с
// эффектом release. Регресс анкера из плана 41.2-11: без исключения пропуск восстановления
// оставлял деталь в группе.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_removing_a_part_of_an_anchor_group_moved_by_this_act_releases_it() {
    timed(async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Склад А").await;
        let b = seed_place(&ctx, "Склад Б").await;
        let f = seed_anchor_group(&ctx, a).await;
        let part = f.parts[2];
        let act = create_ok(&ctx, dto("U-12", &[f.anchor, part], Some(b), false)).await;

        update_ok(&ctx, act, &[f.anchor], false).await;

        assert_eq!(
            member_rows(&ctx, part).await,
            0,
            "деталь выведена из группы"
        );
        assert_eq!(group_place(&ctx, f.group).await, Some(b));
        assert_eq!(
            device_place(&ctx, part).await,
            Some(a),
            "деталь на месте до акта"
        );
        assert_eq!(device_effects(&ctx, act, "release", part).await, 1);
        assert_eq!(
            audit_rows(&ctx, part, "custom:group_restore_skipped").await,
            0
        );
    })
    .await;
}

// ---------------------------------------------------------------------------
// Возвраты (план 41.2-11)
// ---------------------------------------------------------------------------

fn return_item(id: i64) -> ActReturnItemDto {
    ActReturnItemDto {
        act_item_id: id,
        device_id: id,
        device_ids: vec![id],
        quantity: 1,
        condition_override: Some("Исправно".into()),
        place_id_override: None,
    }
}

async fn return_ok(ctx: &AppCtx, parent: i64, ids: &[i64], place: Option<i64>) -> i64 {
    ctx.acts
        .do_return(
            &admin(),
            parent,
            ActReturnDto {
                bulk_condition: None,
                bulk_place_id: place,
                apply_to_all: true,
                items: ids.iter().map(|&id| return_item(id)).collect(),
                giver_name: None,
                receiver_name: None,
                handover_date_utc: None,
            },
        )
        .await
        .expect("do_return")
        .id
}

async fn update_return_ok(ctx: &AppCtx, ret: i64, ids: &[i64], place: Option<i64>) {
    let act = ctx.acts.get(ret).await.expect("get return");
    ctx.acts
        .update_return(
            &admin(),
            ActUpdateReturnDto {
                id: act.id,
                expected_version: act.version,
                giver_name: act.giver_name.clone(),
                receiver_name: act.receiver_name.clone(),
                place_id: act.place_id,
                notes: None,
                deadline_utc: None,
                handover_date_utc: act.handover_date_utc,
                bulk_condition: None,
                bulk_place_id: place,
                apply_to_all: true,
                items: ids.iter().map(|&id| return_item(id)).collect(),
            },
        )
        .await
        .expect("update_return");
}

async fn movement_rows_of_act(ctx: &AppCtx, act: i64, device: i64) -> i64 {
    scalar_i64(
        ctx,
        "SELECT COUNT(*) FROM place_movements WHERE act_id = ?1 \
         AND entity_type = 'device' AND entity_id = ?2",
        vec![int(act), int(device)],
    )
    .await
}

async fn device_status(ctx: &AppCtx, id: i64) -> i64 {
    scalar_i64(
        ctx,
        "SELECT status_id FROM devices WHERE id = ?1",
        vec![int(id)],
    )
    .await
}

/// Следы акта в аудите по устройствам: строки `custom:group_moved_by_act` с этим `act_id`.
async fn moved_traces(ctx: &AppCtx, act: i64, devices: &[i64]) -> i64 {
    let ids = serde_json::to_string(devices).expect("ids");
    ctx.writer
        .execute(move |conn| {
            conn.query_row(
                "SELECT COUNT(*) FROM audit_log WHERE entity_type = 'device' \
                 AND action = 'custom:group_moved_by_act' \
                 AND json_extract(payload_json, '$.act_id') = ?1 \
                 AND entity_id IN (SELECT value FROM json_each(?2))",
                params![act, ids],
                |r| r.get::<_, i64>(0),
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("moved traces")
}

// (r1) возврат якоря: группа едет к месту возврата вместе с якорём и деталями, якорь остаётся
// в составе, на устройство ровно одна строка журнала возвратного акта, пакет один.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn return_anchor_moves_the_group_and_keeps_membership() {
    timed(async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Склад А").await;
        let b = seed_place(&ctx, "Склад Б").await;
        let solo = seed_device(&ctx, dev("Ноутбук #1", Some(a), None)).await;
        let f = seed_anchor_group(&ctx, a).await;
        let handover = create_ok(&ctx, dto("R-1", &[solo, f.anchor], Some(b), false)).await;
        assert_eq!(
            group_place(&ctx, f.group).await,
            Some(b),
            "до возврата группа у места выдачи"
        );

        let ret = return_ok(&ctx, handover, &[f.anchor], Some(a)).await;

        assert_eq!(group_place(&ctx, f.group).await, Some(a));
        assert_eq!(device_place(&ctx, f.anchor).await, Some(a));
        assert_eq!(device_status(&ctx, f.anchor).await, ON_WAREHOUSE);
        for p in &f.parts {
            assert_eq!(
                device_place(&ctx, *p).await,
                Some(a),
                "деталь поехала с группой"
            );
        }
        assert_eq!(
            member_rows(&ctx, f.anchor).await,
            1,
            "якорь остался в составе"
        );
        assert_eq!(group_members(&ctx, f.group).await, 5);
        assert_eq!(effects(&ctx, ret, "move").await, 1);
        assert_eq!(effects(&ctx, ret, "release").await, 0);
        assert_eq!(act_item_rows(&ctx, ret, f.anchor).await, 1);
        assert_eq!(
            scalar_i64(
                &ctx,
                "SELECT COUNT(*) FROM act_items WHERE act_id = ?1",
                vec![int(ret)]
            )
            .await,
            1,
            "в возврате одна позиция, деталей нет"
        );
        // Журнал: группа, якорь и четыре детали — шесть строк пакета, все с источником act.
        assert_eq!(
            scalar_i64(
                &ctx,
                "SELECT COUNT(*) FROM place_movements WHERE act_id = ?1",
                vec![int(ret)]
            )
            .await,
            6
        );
        assert_eq!(
            scalar_i64(
                &ctx,
                "SELECT COUNT(*) FROM place_movements WHERE act_id = ?1 AND source <> 'act'",
                vec![int(ret)]
            )
            .await,
            0
        );
        assert_eq!(
            scalar_i64(
                &ctx,
                "SELECT COUNT(DISTINCT batch_id) FROM place_movements \
                 WHERE act_id = ?1 AND batch_id IS NOT NULL",
                vec![int(ret)]
            )
            .await,
            1,
            "у пакета ровно один batch_id"
        );
        assert_eq!(
            movement_rows_of_act(&ctx, ret, f.anchor).await,
            1,
            "якорь: одна строка"
        );
        for p in &f.parts {
            assert_eq!(
                movement_rows_of_act(&ctx, ret, *p).await,
                1,
                "деталь: одна строка"
            );
        }
    })
    .await;
}

// (r2) без места возврата группа и якорь остаются на месте, якорь не освобождён.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn return_anchor_without_a_return_place_keeps_group_and_anchor_together() {
    timed(async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Склад А").await;
        let b = seed_place(&ctx, "Склад Б").await;
        let solo = seed_device(&ctx, dev("Ноутбук #1", Some(a), None)).await;
        let f = seed_anchor_group(&ctx, a).await;
        let handover = create_ok(&ctx, dto("R-2", &[solo, f.anchor], Some(b), false)).await;

        let ret = return_ok(&ctx, handover, &[f.anchor], None).await;

        assert_eq!(
            group_place(&ctx, f.group).await,
            Some(b),
            "группа не двигалась"
        );
        assert_eq!(
            device_place(&ctx, f.anchor).await,
            Some(b),
            "место якоря = место группы"
        );
        for p in &f.parts {
            assert_eq!(device_place(&ctx, *p).await, Some(b));
        }
        assert_eq!(member_rows(&ctx, f.anchor).await, 1, "якорь не освобождён");
        assert_eq!(device_status(&ctx, f.anchor).await, ON_WAREHOUSE);
        assert_eq!(effects(&ctx, ret, "move").await, 0);
        assert_eq!(effects(&ctx, ret, "release").await, 0);
    })
    .await;
}

// (r3) тот же набор, что у выдачи контейнера: возврат всех трёх членов освобождает их по одному,
// контейнер не переезжает. Расхождение с (r1): там якорная группа едет, здесь контейнер стоит.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn return_whole_container_releases_members_and_does_not_move_the_group() {
    timed(async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Склад А").await;
        let b = seed_place(&ctx, "Склад Б").await;
        let arm = seed_arm(&ctx, a).await;
        let handover = create_ok(&ctx, dto("R-3", &arm.devices, Some(b), false)).await;
        assert_eq!(
            effects(&ctx, handover, "move").await,
            1,
            "выдача группы целиком"
        );

        let ret = return_ok(&ctx, handover, &arm.devices, Some(a)).await;

        assert_eq!(effects(&ctx, ret, "release").await, 3);
        assert_eq!(effects(&ctx, ret, "move").await, 0);
        assert_eq!(group_members(&ctx, arm.group).await, 0);
        assert_eq!(
            group_place(&ctx, arm.group).await,
            Some(b),
            "контейнер не переехал"
        );
        for d in &arm.devices {
            assert_eq!(device_place(&ctx, *d).await, Some(a));
            assert_eq!(movement_rows_of_act(&ctx, ret, *d).await, 1);
        }
    })
    .await;
}

// (r4) частичный возврат: освобождён один член, остальные в составе.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn return_one_member_of_a_container_releases_only_that_member() {
    timed(async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Склад А").await;
        let b = seed_place(&ctx, "Склад Б").await;
        let arm = seed_arm(&ctx, a).await;
        let handover = create_ok(&ctx, dto("R-4", &arm.devices, Some(b), false)).await;
        let one = arm.devices[0];

        let ret = return_ok(&ctx, handover, &[one], Some(a)).await;

        assert_eq!(member_rows(&ctx, one).await, 0);
        assert_eq!(group_members(&ctx, arm.group).await, 2);
        assert_eq!(device_effects(&ctx, ret, "release", one).await, 1);
        assert_eq!(effects(&ctx, ret, "release").await, 1);
        assert_eq!(group_place(&ctx, arm.group).await, Some(b));
        assert_eq!(device_place(&ctx, one).await, Some(a));
    })
    .await;
}

// (r5) правка возврата: добавление якоря переносит группу, удаление якоря возвращает её на
// место выдачи и отвязывает следы деталей, удаление позиции-члена возвращает его в группу.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn return_update_adds_and_removes_anchor_and_member_positions() {
    timed(async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Склад А").await;
        let b = seed_place(&ctx, "Склад Б").await;
        let solo = seed_device(&ctx, dev("Ноутбук #1", Some(a), None)).await;
        let f = seed_anchor_group(&ctx, a).await;
        let handover = create_ok(&ctx, dto("R-5", &[solo, f.anchor], Some(b), false)).await;
        let ret = return_ok(&ctx, handover, &[solo], Some(a)).await;
        assert_eq!(
            group_place(&ctx, f.group).await,
            Some(b),
            "якорь ещё не возвращён"
        );

        // Добавление якоря в возврат переносит его группу.
        update_return_ok(&ctx, ret, &[solo, f.anchor], Some(a)).await;
        assert_eq!(group_place(&ctx, f.group).await, Some(a));
        assert_eq!(device_place(&ctx, f.anchor).await, Some(a));
        assert_eq!(member_rows(&ctx, f.anchor).await, 1);
        assert_eq!(effects(&ctx, ret, "move").await, 1);
        assert_eq!(
            movement_rows_of_act(&ctx, ret, f.anchor).await,
            1,
            "одна строка на якорь"
        );
        assert_eq!(
            moved_traces(&ctx, ret, &f.parts).await,
            f.parts.len() as i64
        );

        // Удаление якоря возвращает группу на место выдачи.
        update_return_ok(&ctx, ret, &[solo], Some(a)).await;
        assert_eq!(group_place(&ctx, f.group).await, Some(b));
        assert_eq!(device_place(&ctx, f.anchor).await, Some(b));
        assert_eq!(device_status(&ctx, f.anchor).await, IN_WORK);
        for p in &f.parts {
            assert_eq!(device_place(&ctx, *p).await, Some(b));
        }
        assert_eq!(member_rows(&ctx, f.anchor).await, 1);
        assert_eq!(
            effects(&ctx, ret, "move").await,
            0,
            "эффект move возврата удалён"
        );
        assert_eq!(
            moved_traces(&ctx, ret, &f.parts).await,
            0,
            "следы деталей отвязаны"
        );

        // Член контейнера: освобождён возвратом, правка убирает позицию, членство возвращается.
        let arm = seed_arm_named(&ctx, a, "АРМ #7").await;
        let handover2 = create_ok(&ctx, dto("R-5B", &arm.devices, Some(b), false)).await;
        let ret2 = return_ok(&ctx, handover2, &[arm.devices[0], arm.devices[1]], Some(a)).await;
        assert_eq!(member_rows(&ctx, arm.devices[0]).await, 0);
        assert_eq!(
            device_effects(&ctx, ret2, "release", arm.devices[0]).await,
            1
        );

        update_return_ok(&ctx, ret2, &[arm.devices[1]], Some(a)).await;
        assert_eq!(
            member_rows(&ctx, arm.devices[0]).await,
            1,
            "член вернулся в группу"
        );
        assert_eq!(device_place(&ctx, arm.devices[0]).await, Some(b));
        assert_eq!(
            device_effects(&ctx, ret2, "release", arm.devices[0]).await,
            0,
            "потраченный эффект удалён"
        );
        assert_eq!(
            member_rows(&ctx, arm.devices[1]).await,
            0,
            "оставшийся в возврате не вернулся"
        );
    })
    .await;
}

// (r6) принтер-деталь с картриджем: строка журнала картриджа несёт source act, act_id возврата и
// batch_id пакета. Расходящаяся фикстура: картридж прикреплён к принтеру.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn return_anchor_carries_the_cartridge_of_a_printer_part_in_the_batch() {
    timed(async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Склад А").await;
        let b = seed_place(&ctx, "Склад Б").await;
        let f = seed_anchor_group(&ctx, a).await;
        let printer = f.parts[0];
        ctx.writer
            .execute(move |conn| {
                conn.execute(
                    "UPDATE devices SET type_id = 2 WHERE id = ?1",
                    params![printer],
                )
                .map_err(map_rusqlite)?;
                Ok(())
            })
            .await
            .expect("make printer");
        let model = ctx
            .cartridges
            .model_create(CartridgeModelCreateDto {
                brand: "HP".into(),
                model: "CE285A".into(),
                kind_id: 1,
                color: Some("Чёрный".into()),
                notes: None,
                compatibility: vec![],
            })
            .await
            .expect("model")
            .id;
        let cartridge = ctx
            .cartridges
            .create(CartridgeCreateDto {
                model_id: model,
                number_input: NumberFieldInput {
                    value: "C-ACTR-00001".into(),
                    template_id: None,
                    confirm_mismatch: false,
                    confirm_script_mix: false,
                },
                state_id: Some(1),
                place_id: Some(a),
                notes: None,
            })
            .await
            .expect("cartridge")
            .expect_created("cartridge")
            .id;
        ctx.writer
            .execute(move |conn| {
                conn.execute(
                    "UPDATE cartridges SET current_printer_device_id = ?1 WHERE id = ?2",
                    params![printer, cartridge],
                )
                .map_err(map_rusqlite)?;
                Ok(())
            })
            .await
            .expect("attach cartridge");
        let handover = create_ok(&ctx, dto("R-6", &[f.anchor], Some(b), false)).await;

        let ret = return_ok(&ctx, handover, &[f.anchor], Some(a)).await;

        assert_eq!(
            scalar_i64(
                &ctx,
                "SELECT COUNT(*) FROM place_movements WHERE act_id = ?1 \
                 AND entity_type = 'cartridge' AND entity_id = ?2 \
                 AND source = 'act' AND batch_id IS NOT NULL",
                vec![int(ret), int(cartridge)]
            )
            .await,
            1,
            "картридж принтера-детали в пакете возврата"
        );
        assert_eq!(
            scalar_i64(
                &ctx,
                "SELECT COUNT(DISTINCT batch_id) FROM place_movements \
                 WHERE act_id = ?1 AND batch_id IS NOT NULL",
                vec![int(ret)]
            )
            .await,
            1
        );
    })
    .await;
}

// (r7) B-2: группу переместили ПОСЛЕ возврата, правка убирает якорь -> правка проходит, всё
// остаётся в третьем месте. Расхождение: вторая группа без перемещения возвращается на место
// выдачи.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn return_update_removing_anchor_of_a_group_moved_after_the_return_passes() {
    timed(async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Склад А").await;
        let b = seed_place(&ctx, "Склад Б").await;
        let c = seed_place(&ctx, "Склад В").await;
        let solo = seed_device(&ctx, dev("Ноутбук #1", Some(a), None)).await;
        let f = seed_anchor_group(&ctx, a).await;
        let g = seed_anchor_group_with(&ctx, a, "Сборка #9", ["214", "SN-SSD-2", "AB456"]).await;
        let handover = create_ok(
            &ctx,
            dto("R-7", &[solo, f.anchor, g.anchor], Some(b), false),
        )
        .await;
        let ret = return_ok(&ctx, handover, &[solo, f.anchor, g.anchor], Some(a)).await;
        let version = group_version(&ctx, f.group).await;
        ctx.groups
            .move_group(&admin(), f.group, version, c)
            .await
            .expect("move group after the return");

        update_return_ok(&ctx, ret, &[solo, g.anchor], Some(a)).await;

        assert_eq!(
            group_place(&ctx, f.group).await,
            Some(c),
            "группа осталась в третьем месте"
        );
        assert_eq!(device_place(&ctx, f.anchor).await, Some(c));
        for p in &f.parts {
            assert_eq!(device_place(&ctx, *p).await, Some(c));
        }
        assert_eq!(member_rows(&ctx, f.anchor).await, 1, "членство цело");
        assert_eq!(act_item_rows(&ctx, ret, f.anchor).await, 0);
        assert_eq!(
            audit_rows(&ctx, f.anchor, "custom:group_restore_skipped").await,
            1
        );
        let group_id = f.group;
        assert_eq!(
            scalar_i64(
                &ctx,
                "SELECT COUNT(*) FROM audit_log WHERE entity_id = ?1 \
                 AND action = 'custom:group_restore_skipped' \
                 AND json_extract(payload_json, '$.group_id') = ?2 \
                 AND json_extract(payload_json, '$.act_id') = ?3 \
                 AND json_extract(payload_json, '$.reason') IS NOT NULL",
                vec![int(f.anchor), int(group_id), int(ret)]
            )
            .await,
            1,
            "причина пропуска записана в аудит"
        );
        // Контроль: вторая группа не перемещалась, её якорь остался в возврате.
        assert_eq!(group_place(&ctx, g.group).await, Some(a));
        update_return_ok(&ctx, ret, &[solo], Some(a)).await;
        assert_eq!(
            group_place(&ctx, g.group).await,
            Some(b),
            "без перемещения после возврата группа возвращается на место выдачи"
        );
        assert_eq!(
            audit_rows(&ctx, g.anchor, "custom:group_restore_skipped").await,
            0
        );
    })
    .await;
}

// (r8) V-4: деталь якорной группы вписана в возврат отдельной позицией; правка убирает её из
// возврата, якорь остаётся. Группа переехала этим возвратом (режим Record): деталь выводится из
// группы, эффект release пишет сама правка и он доживает до отката возврата. Расхождение с (r5):
// там эффект release существовал ДО правки и удаляется, здесь его записывает правка. Откат через
// delete_soft этого возврата добавляет план 41.2-13 (сейчас его останавливает охрана якоря).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn return_update_removing_a_part_of_a_moved_group_records_a_release_effect_that_survives() {
    timed(async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Склад А").await;
        let b = seed_place(&ctx, "Склад Б").await;
        let f = seed_anchor_group(&ctx, a).await;
        let part = f.parts[2];
        let handover = create_ok(&ctx, dto("R-8", &[f.anchor, part], Some(b), false)).await;
        let ret = return_ok(&ctx, handover, &[f.anchor, part], Some(a)).await;
        assert_eq!(group_place(&ctx, f.group).await, Some(a));
        assert_eq!(member_rows(&ctx, part).await, 1, "деталь едет с группой");
        assert_eq!(device_effects(&ctx, ret, "release", part).await, 0);

        update_return_ok(&ctx, ret, &[f.anchor], Some(a)).await;

        assert_eq!(
            member_rows(&ctx, part).await,
            0,
            "деталь выведена из группы"
        );
        assert_eq!(group_members(&ctx, f.group).await, 4);
        assert_eq!(
            group_place(&ctx, f.group).await,
            Some(a),
            "группа осталась у места возврата"
        );
        assert_eq!(device_place(&ctx, f.anchor).await, Some(a));
        assert_eq!(
            device_effects(&ctx, ret, "release", part).await,
            1,
            "эффект release записан правкой и не потрачен её же шагом возврата членства"
        );
        assert_eq!(
            audit_rows(&ctx, part, "custom:group_member_restored").await,
            0
        );

        // Эффект указывает на группу, в которую откат вернёт деталь. Само обращение через
        // `delete_soft` возврата с якорем недоступно до плана 41.2-13 (путь удаления ещё
        // упирается в охрану якоря); контроль «деталь вернулась в группу» добавляет тот план.
        assert_eq!(
            scalar_i64(
                &ctx,
                "SELECT COUNT(*) FROM act_group_effects WHERE act_id = ?1 AND kind = 'release' \
                 AND device_id = ?2 AND group_id = ?3",
                vec![int(ret), int(part), int(f.group)]
            )
            .await,
            1
        );
    })
    .await;
}

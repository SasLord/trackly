//! Phase 41.2 Plan 07: создание акта с группами (GAM-01..GAM-03, D-02, D-04, D-05, D-10..D-14).
//!
//! Перенос группы целиком, вынос части состава с подтверждением ДО первой записи,
//! якорь одной позицией с комплектацией, журнал пакета с картриджем принтера-члена.
//! Реальный `AppCtx` на временном каталоге; названия и имена вымышленные.
//!
//! Префикс тестов: `create_`.

use std::future::Future;
use std::time::Duration;

use rusqlite::params;
use rusqlite::types::Value;

use trackly_app::context::AppCtx;
use trackly_app::dto::act::{ActCreateDto, ActItemNewDto, ActSaveOutcome};
use trackly_app::dto::cartridge::{CartridgeCreateDto, CartridgeModelCreateDto};
use trackly_app::dto::number_template::NumberFieldInput;
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
    let group = seed_group(ctx, "АРМ #3", Some(place), None).await;
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
    let group = seed_group(ctx, "Сборка #4", Some(place), None).await;
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
                inv: Some("114"),
                serial: Some("SN-SSD-1"),
                ..dev("SSD Kingston 480ГБ", Some(place), Some(group))
            },
        )
        .await,
    );
    parts.push(
        seed_device(
            ctx,
            Dev {
                serial: Some("AB123"),
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

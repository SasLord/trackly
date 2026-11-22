//! Quick 261122-i5p (R7-01): акт передаёт устройства со статусом «На списание»
//! и сохраняет статус; возврат и откат его тоже не меняют.
//!
//! Реальный `AppCtx` на временном каталоге; названия и имена вымышленные.
//! Префикс тестов: `to_write_off_`.

#![allow(dead_code)]

use std::future::Future;
use std::time::Duration;

use rusqlite::params;
use rusqlite::types::Value;

use trackly_app::context::AppCtx;
use trackly_app::dto::act::{
    ActCreateDto, ActItemNewDto, ActReturnDto, ActReturnItemDto, ActSaveOutcome, ActUpdateReturnDto,
};
use trackly_app::dto::number_template::NumberFieldInput;
use trackly_core::auth::Identity;
use trackly_core::domain::repairs::DEVICE_STATUS_CODE_TO_WRITE_OFF;
use trackly_core::error::AppError;
use trackly_infra::error_conversions::map_rusqlite;

const ON_WAREHOUSE: i64 = 1;
const IN_WORK: i64 = 2;

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

async fn timed<F: Future<Output = ()>>(f: F) {
    tokio::time::timeout(Duration::from_secs(30), f)
        .await
        .expect("тест не уложился в 30 секунд");
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
                params![format!("wo_t_{behavior}"), "Тип теста", behavior],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("type by behavior")
}

async fn seed_group(ctx: &AppCtx, behavior: &'static str, name: &str, place: i64) -> i64 {
    let type_id = type_by_behavior(ctx, behavior).await;
    let name = name.to_string();
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO groups (type_id, name, seq, place_id, parent_group_id, \
                 created_at_utc, updated_at_utc, version) \
                 VALUES (?1, ?2, (SELECT COALESCE(MAX(seq), 0) + 1 FROM groups WHERE type_id = ?1), \
                 ?3, NULL, 1700000000, 1700000000, 1)",
                params![type_id, name, place],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed group")
}

async fn seed_device(ctx: &AppCtx, name: &str, place: i64, status: i64, group: Option<i64>) -> i64 {
    let name = name.to_string();
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO devices (type_id, name, place_id, status_id, created_at_utc, \
                 updated_at_utc, version) VALUES (1, ?1, ?2, ?3, 1700000000, 1700000000, 1)",
                params![name, place, status],
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

/// «На списание» по КОДУ статуса (общая константа домена), не по id и не по подписи.
async fn set_to_write_off(ctx: &AppCtx, device: i64) {
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "UPDATE devices SET status_id = \
                 (SELECT id FROM device_statuses WHERE code = ?2) WHERE id = ?1",
                params![device, DEVICE_STATUS_CODE_TO_WRITE_OFF],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("set to_write_off");
}

async fn write_off(ctx: &AppCtx, device: i64) {
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "UPDATE devices SET status_id = \
                 (SELECT id FROM device_statuses WHERE code = 'списано') WHERE id = ?1",
                params![device],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("write off");
}

async fn is_to_write_off(ctx: &AppCtx, device: i64) -> bool {
    scalar_i64(
        ctx,
        "SELECT COUNT(*) FROM devices d JOIN device_statuses s ON s.id = d.status_id \
         WHERE d.id = ?1 AND s.code = 'на_списание'",
        vec![Value::Integer(device)],
    )
    .await
        == 1
}

async fn place_of(ctx: &AppCtx, device: i64) -> i64 {
    scalar_i64(
        ctx,
        "SELECT place_id FROM devices WHERE id = ?1",
        vec![Value::Integer(device)],
    )
    .await
}

async fn movements_of(ctx: &AppCtx, device: i64) -> i64 {
    scalar_i64(
        ctx,
        "SELECT COUNT(*) FROM place_movements WHERE entity_id = ?1",
        vec![Value::Integer(device)],
    )
    .await
}

async fn status_of(ctx: &AppCtx, device: i64) -> i64 {
    scalar_i64(
        ctx,
        "SELECT status_id FROM devices WHERE id = ?1",
        vec![Value::Integer(device)],
    )
    .await
}

async fn member_rows(ctx: &AppCtx, device: i64) -> i64 {
    scalar_i64(
        ctx,
        "SELECT COUNT(*) FROM group_devices WHERE device_id = ?1",
        vec![Value::Integer(device)],
    )
    .await
}

async fn act_count(ctx: &AppCtx) -> i64 {
    scalar_i64(ctx, "SELECT COUNT(*) FROM acts", vec![]).await
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

fn conflict_reason(err: AppError) -> String {
    match err {
        AppError::Conflict { reason } => reason,
        other => panic!("ожидался Conflict, получено {other:?}"),
    }
}

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

fn return_dto(ids: &[i64], place: Option<i64>) -> ActReturnDto {
    ActReturnDto {
        bulk_condition: None,
        bulk_place_id: place,
        apply_to_all: true,
        items: ids.iter().map(|&id| return_item(id)).collect(),
        giver_name: None,
        receiver_name: None,
        handover_date_utc: None,
    }
}

async fn return_ok(ctx: &AppCtx, parent: i64, ids: &[i64], place: Option<i64>) -> i64 {
    ctx.acts
        .do_return(&admin(), parent, return_dto(ids, place))
        .await
        .expect("do_return")
        .id
}

async fn update_return_result(
    ctx: &AppCtx,
    ret: i64,
    ids: &[i64],
    place: Option<i64>,
) -> Result<(), AppError> {
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
        .map(|_| ())
}

// 1: одиночное «На списание» передаётся актом, статус сохраняется, история пишется.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn to_write_off_single_device_handover_keeps_status() {
    timed(async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Склад А").await;
        let b = seed_place(&ctx, "Склад Б").await;
        let dev = seed_device(&ctx, "Монитор #2", a, ON_WAREHOUSE, None).await;
        set_to_write_off(&ctx, dev).await;
        let moves_before = movements_of(&ctx, dev).await;
        create_ok(&ctx, dto("TW-1", &[dev], Some(b), false)).await;
        assert!(is_to_write_off(&ctx, dev).await, "статус сохранён");
        assert_eq!(place_of(&ctx, dev).await, b);
        assert!(movements_of(&ctx, dev).await > moves_before);
        let audits = scalar_i64(
            &ctx,
            "SELECT COUNT(*) FROM audit_log WHERE entity_id = ?1",
            vec![Value::Integer(dev)],
        )
        .await;
        assert!(audits >= 1);
    })
    .await;
}

// 2: член группы «На списание» передаётся актом и сохраняет статус.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn to_write_off_group_member_keeps_status() {
    timed(async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Склад А").await;
        let b = seed_place(&ctx, "Склад Б").await;
        let group = seed_group(&ctx, "container", "АРМ #3", a).await;
        let _monitor = seed_device(&ctx, "Монитор #2", a, IN_WORK, Some(group)).await;
        let kb = seed_device(&ctx, "Клавиатура #1", a, IN_WORK, Some(group)).await;
        set_to_write_off(&ctx, kb).await;
        create_ok(&ctx, dto("TW-2", &[kb], Some(b), true)).await;
        assert!(is_to_write_off(&ctx, kb).await, "статус сохранён");
        assert_eq!(place_of(&ctx, kb).await, b);
    })
    .await;
}

// 3: донор «Разбора» «На списание» передаётся и сохраняет статус.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn to_write_off_teardown_donor_keeps_status() {
    timed(async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Склад А").await;
        let b = seed_place(&ctx, "Склад Б").await;
        let group = seed_group(&ctx, "teardown", "Разбор #7", a).await;
        let donor = seed_device(&ctx, "Монитор #2", a, ON_WAREHOUSE, Some(group)).await;
        let _rest = seed_device(&ctx, "Клавиатура #1", a, ON_WAREHOUSE, Some(group)).await;
        set_to_write_off(&ctx, donor).await;
        create_ok(&ctx, dto("TW-3", &[donor], Some(b), true)).await;
        assert!(is_to_write_off(&ctx, donor).await, "статус сохранён");
        assert_eq!(place_of(&ctx, donor).await, b);
    })
    .await;
}

// 4: возврат проходит, статус «На списание» остаётся, место возврата записано;
// повторный возврат отказывает.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn to_write_off_return_keeps_status() {
    timed(async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Склад А").await;
        let b = seed_place(&ctx, "Склад Б").await;
        let dev = seed_device(&ctx, "Монитор #2", a, ON_WAREHOUSE, None).await;
        set_to_write_off(&ctx, dev).await;
        let handover = create_ok(&ctx, dto("TW-4", &[dev], Some(b), false)).await;
        let moves_before = movements_of(&ctx, dev).await;
        return_ok(&ctx, handover, &[dev], Some(a)).await;
        assert!(is_to_write_off(&ctx, dev).await, "статус сохранён");
        assert_eq!(place_of(&ctx, dev).await, a);
        assert!(movements_of(&ctx, dev).await > moves_before);
        let again = ctx
            .acts
            .do_return(&admin(), handover, return_dto(&[dev], Some(a)))
            .await;
        assert!(again.is_err(), "повторный возврат отказывает");
    })
    .await;
}

// 5: откат акта передачи возвращает «На списание» и прежнее место.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn to_write_off_undo_restores_status_and_place() {
    timed(async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Склад А").await;
        let b = seed_place(&ctx, "Склад Б").await;
        let dev = seed_device(&ctx, "Монитор #2", a, ON_WAREHOUSE, None).await;
        set_to_write_off(&ctx, dev).await;
        let handover = create_ok(&ctx, dto("TW-5", &[dev], Some(b), false)).await;
        let version = scalar_i64(
            &ctx,
            "SELECT version FROM acts WHERE id = ?1",
            vec![Value::Integer(handover)],
        )
        .await;
        ctx.acts
            .delete_soft(handover, version)
            .await
            .expect("откат акта");
        assert!(is_to_write_off(&ctx, dev).await, "статус восстановлен");
        assert_eq!(place_of(&ctx, dev).await, a);
    })
    .await;
}

// 6: «Списано» по-прежнему отклоняется.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn to_write_off_written_off_still_rejected() {
    timed(async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Склад А").await;
        let b = seed_place(&ctx, "Склад Б").await;
        let dev = seed_device(&ctx, "Монитор #2", a, ON_WAREHOUSE, None).await;
        write_off(&ctx, dev).await;
        let err = ctx
            .acts
            .create(&admin(), dto("TW-6", &[dev], Some(b), false))
            .await
            .expect_err("списанное не передать");
        let _ = conflict_reason(err);
    })
    .await;
}

// 7: обычный поток склад -> в работе -> склад не сломан.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn to_write_off_regular_flow_unchanged() {
    timed(async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Склад А").await;
        let b = seed_place(&ctx, "Склад Б").await;
        let dev = seed_device(&ctx, "Монитор #2", a, ON_WAREHOUSE, None).await;
        let handover = create_ok(&ctx, dto("TW-7", &[dev], Some(b), false)).await;
        assert_eq!(status_of(&ctx, dev).await, IN_WORK);
        return_ok(&ctx, handover, &[dev], Some(a)).await;
        assert_eq!(status_of(&ctx, dev).await, ON_WAREHOUSE);
    })
    .await;
}

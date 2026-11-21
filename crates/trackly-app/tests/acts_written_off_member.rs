//! Phase 41.8 Plan 15 (CR-01, D-20, D-27): списанное устройство не попадает в акты
//! и возвраты не отменяют списание.
//!
//! Реальный `AppCtx` на временном каталоге; названия и имена вымышленные.
//! Префикс тестов: `written_off_member_`.

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

/// Списание по КОДУ статуса, не по id и не по подписи.
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

async fn is_written_off(ctx: &AppCtx, device: i64) -> bool {
    scalar_i64(
        ctx,
        "SELECT COUNT(*) FROM devices d JOIN device_statuses s ON s.id = d.status_id \
         WHERE d.id = ?1 AND s.code = 'списано'",
        vec![Value::Integer(device)],
    )
    .await
        == 1
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

/// «АРМ #3» контейнер: «Монитор #2» (В работе) и «Клавиатура #1» (Списано, оставлена в группе).
async fn arm_with_written_off(ctx: &AppCtx, place: i64) -> (i64, i64, i64) {
    let group = seed_group(ctx, "container", "АРМ #3", place).await;
    let monitor = seed_device(ctx, "Монитор #2", place, IN_WORK, Some(group)).await;
    let keyboard = seed_device(ctx, "Клавиатура #1", place, IN_WORK, Some(group)).await;
    write_off(ctx, keyboard).await;
    (group, monitor, keyboard)
}

// 1: разворот контейнера не отдаёт списанного участника.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn written_off_member_container_expansion_skips_it() {
    timed(async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Склад А").await;
        let (group, monitor, _kb) = arm_with_written_off(&ctx, a).await;
        let out = ctx.groups.act_expansion(&admin(), group).await.unwrap();
        let ids: Vec<i64> = out.units.iter().map(|u| u.device_id).collect();
        assert_eq!(ids, vec![monitor]);
    })
    .await;
}

// 2: акт передачи группы не возвращает списанного участника в «В работе» и оставляет в составе.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn written_off_member_stays_written_off_and_in_group_after_handover() {
    timed(async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Склад А").await;
        let b = seed_place(&ctx, "Склад Б").await;
        let (group, monitor, kb) = arm_with_written_off(&ctx, a).await;
        let expansion = ctx.groups.act_expansion(&admin(), group).await.unwrap();
        let ids: Vec<i64> = expansion.units.iter().map(|u| u.device_id).collect();
        // Без подтверждения вывода: списанный участник не должен давать предупреждение.
        create_ok(&ctx, dto("WO-2", &ids, Some(b), false)).await;
        assert!(is_written_off(&ctx, kb).await, "списано остаётся списанным");
        assert_eq!(member_rows(&ctx, kb).await, 1, "остаётся в составе (D-20)");
        assert_eq!(status_of(&ctx, monitor).await, IN_WORK);
    })
    .await;
}

// 3: списанный участник живой группы не включается в акт напрямую.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn written_off_member_direct_inclusion_is_rejected() {
    timed(async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Склад А").await;
        let b = seed_place(&ctx, "Склад Б").await;
        let (_group, _monitor, kb) = arm_with_written_off(&ctx, a).await;
        let before = act_count(&ctx).await;
        let err = ctx
            .acts
            .create(&admin(), dto("WO-3", &[kb], Some(b), false))
            .await
            .expect_err("списанного не включить");
        let reason = conflict_reason(err);
        assert!(
            reason.contains("Списано"),
            "текст называет статус: {reason}"
        );
        assert!(is_written_off(&ctx, kb).await);
        assert_eq!(member_rows(&ctx, kb).await, 1, "состав не тронут");
        assert_eq!(act_count(&ctx).await, before);
    })
    .await;
}

// 4: списанный донор разбора не разворачивается.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn written_off_member_teardown_donor_is_not_expanded() {
    timed(async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Склад А").await;
        let group = seed_group(&ctx, "teardown", "Разбор #7", a).await;
        let donor = seed_device(&ctx, "Монитор #2", a, ON_WAREHOUSE, Some(group)).await;
        let rest = seed_device(&ctx, "Клавиатура #1", a, ON_WAREHOUSE, Some(group)).await;
        write_off(&ctx, donor).await;
        let out = ctx.groups.act_expansion(&admin(), group).await.unwrap();
        let ids: Vec<i64> = out.units.iter().map(|u| u.device_id).collect();
        assert_eq!(ids, vec![rest]);
    })
    .await;
}

/// Два одиночных устройства выданы одним актом в `b`.
async fn lone_pair(ctx: &AppCtx, a: i64, b: i64, number: &str) -> (i64, i64, i64) {
    let monitor = seed_device(ctx, "Монитор #2", a, ON_WAREHOUSE, None).await;
    let mouse = seed_device(ctx, "Мышь #4", a, ON_WAREHOUSE, None).await;
    let handover = create_ok(ctx, dto(number, &[monitor, mouse], Some(b), false)).await;
    (handover, monitor, mouse)
}

// 6: возврат списанного отказывает с причиной, операция откатывается, остальные уходят отдельным актом.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn written_off_member_return_conflicts_and_rolls_back() {
    timed(async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Склад А").await;
        let b = seed_place(&ctx, "Склад Б").await;
        let (handover, monitor, mouse) = lone_pair(&ctx, a, b, "WO-6").await;
        write_off(&ctx, monitor).await;
        let acts_before = act_count(&ctx).await;
        let err = ctx
            .acts
            .do_return(&admin(), handover, return_dto(&[monitor, mouse], Some(a)))
            .await
            .expect_err("списанное не вернуть");
        let reason = conflict_reason(err);
        assert!(
            reason.contains("Списано"),
            "текст называет статус: {reason}"
        );
        assert_eq!(act_count(&ctx).await, acts_before, "акт возврата не создан");
        assert_eq!(status_of(&ctx, mouse).await, IN_WORK, "остальное откатано");
        return_ok(&ctx, handover, &[mouse], Some(a)).await;
        assert_eq!(status_of(&ctx, mouse).await, ON_WAREHOUSE);
        assert!(is_written_off(&ctx, monitor).await);
    })
    .await;
}

// 7: правка возврата, добавление списанного.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn written_off_member_update_return_added_conflicts() {
    timed(async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Склад А").await;
        let b = seed_place(&ctx, "Склад Б").await;
        let (handover, monitor, mouse) = lone_pair(&ctx, a, b, "WO-7").await;
        let ret = return_ok(&ctx, handover, &[mouse], Some(a)).await;
        write_off(&ctx, monitor).await;
        let err = update_return_result(&ctx, ret, &[mouse, monitor], Some(a))
            .await
            .expect_err("списанное не добавить в возврат");
        let reason = conflict_reason(err);
        assert!(
            reason.contains("Списано"),
            "текст называет статус: {reason}"
        );
        assert!(is_written_off(&ctx, monitor).await);
        assert_eq!(status_of(&ctx, mouse).await, ON_WAREHOUSE);
    })
    .await;
}

// 8: правка удержанной позиции не стирает списание. Ветка retained в update_return сама
// статус не проверяет, но ей предшествует охрана «устройство изменилось после возврата»
// (сверка status_id с after_json журнала): списание после возврата её не проходит.
// Тест закрепляет охрану: отказ и статус «Списано» неизменен.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn written_off_member_update_return_retained_keeps_written_off() {
    timed(async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Склад А").await;
        let b = seed_place(&ctx, "Склад Б").await;
        let c = seed_place(&ctx, "Склад В").await;
        let (handover, monitor, mouse) = lone_pair(&ctx, a, b, "WO-8").await;
        let ret = return_ok(&ctx, handover, &[monitor, mouse], Some(a)).await;
        write_off(&ctx, monitor).await;
        let err = update_return_result(&ctx, ret, &[monitor, mouse], Some(c))
            .await
            .expect_err("правка списанного удержанного отказывает охраной");
        let _ = conflict_reason(err);
        assert!(
            is_written_off(&ctx, monitor).await,
            "статус «Списано» не перезаписан «На складе»"
        );
        let place = scalar_i64(
            &ctx,
            "SELECT place_id FROM devices WHERE id = ?1",
            vec![Value::Integer(monitor)],
        )
        .await;
        assert_eq!(place, a, "место не тронуто");
    })
    .await;
}

// 9 (план 24, D-27, restore_from_snapshot_in_tx): откат акта передачи не воскрешает
// устройство, списанное уже после акта.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn written_off_member_undo_handover_does_not_resurrect() {
    timed(async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Склад А").await;
        let b = seed_place(&ctx, "Склад Б").await;
        let (handover, monitor, _mouse) = lone_pair(&ctx, a, b, "WO-9").await;
        write_off(&ctx, monitor).await;
        let version = scalar_i64(
            &ctx,
            "SELECT version FROM acts WHERE id = ?1",
            vec![Value::Integer(handover)],
        )
        .await;
        let res = ctx.acts.delete_soft(handover, version).await;
        eprintln!("PROBE undo result ok={}", res.is_ok());
        assert!(
            is_written_off(&ctx, monitor).await,
            "статус «Списано» не перезаписан снимком до акта"
        );
    })
    .await;
}

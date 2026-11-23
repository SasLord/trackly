//! Фаза 41.10, план 07 (HST-05/HST-06): события `status_changed` у писателей статуса
//! `act_service` (ST-2 создание, ST-3 добавление позиции, ST-4 возврат, правка акта).
//! Префикс тестов: `history_status_act_`. Фикстуры вымышленные.

use rusqlite::params;

use trackly_app::context::AppCtx;
use trackly_app::dto::act::{
    ActCreateDto, ActItemNewDto, ActNumberEditInput, ActReturnDto, ActReturnItemDto, ActUpdateDto,
    ActUpdateItemDto,
};
use trackly_app::dto::number_template::NumberFieldInput;
use trackly_core::auth::Identity;
use trackly_infra::error_conversions::map_rusqlite;

async fn make_test_ctx() -> (AppCtx, tempfile::TempDir) {
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

async fn seed_place(ctx: &AppCtx, name: &str) -> i64 {
    let name = name.to_string();
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO places (kind, name, is_storage, created_at_utc, updated_at_utc, version) \
                 VALUES ('room', ?1, 0, 1700000000, 1700000000, 1)",
                params![name],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed place")
}

async fn seed_device(ctx: &AppCtx, name: &str, place: i64) -> i64 {
    let name = name.to_string();
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO devices (type_id, name, status_id, place_id, version, \
                 created_at_utc, updated_at_utc) VALUES (1, ?1, \
                 (SELECT id FROM device_statuses WHERE code = 'на_складе'), ?2, 1, 1700000000, 1700000000)",
                params![name, place],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed device")
}

fn create_dto(number: &str, place: i64, devices: &[i64]) -> ActCreateDto {
    ActCreateDto {
        number_input: NumberFieldInput {
            value: number.into(),
            template_id: None,
            confirm_mismatch: false,
            confirm_script_mix: false,
        },
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
        confirm_group_release: true,
    }
}

/// (kind, source, act_id, movement_id, status_from_code, status_to_code, id)
type Ev = (
    String,
    String,
    Option<i64>,
    Option<i64>,
    Option<String>,
    Option<String>,
    i64,
);

async fn events(ctx: &AppCtx, sql_where: &'static str, p: Vec<rusqlite::types::Value>) -> Vec<Ev> {
    ctx.writer
        .execute(move |conn| {
            let sql = format!(
                "SELECT kind, source, act_id, movement_id, status_from_code, status_to_code, id \
                 FROM history_events WHERE {sql_where} ORDER BY id"
            );
            let mut st = conn.prepare(&sql).map_err(map_rusqlite)?;
            let rows = st
                .query_map(rusqlite::params_from_iter(p), |r| {
                    Ok((
                        r.get(0)?,
                        r.get(1)?,
                        r.get(2)?,
                        r.get(3)?,
                        r.get(4)?,
                        r.get(5)?,
                        r.get(6)?,
                    ))
                })
                .map_err(map_rusqlite)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(map_rusqlite)?;
            Ok(rows)
        })
        .await
        .expect("events")
}

fn int(v: i64) -> rusqlite::types::Value {
    rusqlite::types::Value::Integer(v)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_status_act_create() {
    let (ctx, _d) = make_test_ctx().await;
    let a = seed_place(&ctx, "Каб. 101").await;
    let b = seed_place(&ctx, "Каб. 102").await;
    let dev = seed_device(&ctx, "Ноутбук Тест", a).await;
    let act = ctx
        .acts
        .create(&admin(), create_dto("1", b, &[dev]))
        .await
        .expect("create")
        .expect_created("create");
    let ev = events(
        &ctx,
        "subject_id = ?1 AND kind = 'status_changed'",
        vec![int(dev)],
    )
    .await;
    assert_eq!(ev.len(), 1, "{ev:?}");
    let e = &ev[0];
    assert_eq!(e.1, "act");
    assert_eq!(e.2, Some(act.id));
    assert_eq!(e.4.as_deref(), Some("на_складе"));
    assert_eq!(e.5.as_deref(), Some("в_работе"));
    // перемещение той же операции связано
    assert!(e.3.is_some(), "место сменилось: movement_id обязателен");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_status_act_order() {
    let (ctx, _d) = make_test_ctx().await;
    let a = seed_place(&ctx, "Каб. 201").await;
    let b = seed_place(&ctx, "Каб. 202").await;
    let dev = seed_device(&ctx, "Монитор Тест", a).await;
    ctx.acts
        .create(&admin(), create_dto("1", b, &[dev]))
        .await
        .expect("create")
        .expect_created("create");
    let ev = events(
        &ctx,
        "subject_id = ?1 AND kind = 'status_changed'",
        vec![int(dev)],
    )
    .await;
    let mv = ev[0].3.expect("movement_id");
    // D-25: перемещение записано раньше события, событие ссылается на него.
    let (mv_exists, mv_act): (i64, Option<i64>) = ctx
        .writer
        .execute(move |conn| {
            conn.query_row(
                "SELECT COUNT(*), MAX(act_id) FROM place_movements WHERE id = ?1",
                params![mv],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("mv");
    assert_eq!(mv_exists, 1);
    assert_eq!(mv_act, ev[0].2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_status_act_return() {
    let (ctx, _d) = make_test_ctx().await;
    let a = seed_place(&ctx, "Каб. 301").await;
    let b = seed_place(&ctx, "Склад 3").await;
    let dev = seed_device(&ctx, "Принтер Тест", a).await;
    let act = ctx
        .acts
        .create(&admin(), create_dto("1", a, &[dev]))
        .await
        .expect("create")
        .expect_created("create");
    let ret = ctx
        .acts
        .do_return(
            &admin(),
            act.id,
            ActReturnDto {
                bulk_condition: Some("Хорошее".into()),
                bulk_place_id: Some(b),
                apply_to_all: true,
                giver_name: None,
                receiver_name: None,
                handover_date_utc: None,
                items: vec![ActReturnItemDto {
                    act_item_id: act.items[0].id,
                    device_id: dev,
                    device_ids: vec![dev],
                    quantity: 1,
                    condition_override: None,
                    place_id_override: None,
                }],
            },
        )
        .await
        .expect("return");
    let ev = events(
        &ctx,
        "subject_id = ?1 AND kind = 'status_changed'",
        vec![int(dev)],
    )
    .await;
    assert_eq!(ev.len(), 2, "{ev:?}");
    let last = &ev[1];
    assert_eq!(last.1, "act");
    assert_eq!(last.2, Some(ret.id));
    assert_eq!(last.4.as_deref(), Some("в_работе"));
    assert_eq!(last.5.as_deref(), Some("на_складе"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_status_act_edit_remove_item() {
    let (ctx, _d) = make_test_ctx().await;
    let a = seed_place(&ctx, "Каб. 401").await;
    let b = seed_place(&ctx, "Каб. 402").await;
    let d1 = seed_device(&ctx, "Ноутбук А", a).await;
    let d2 = seed_device(&ctx, "Ноутбук Б", a).await;
    let act = ctx
        .acts
        .create(&admin(), create_dto("1", b, &[d1, d2]))
        .await
        .expect("create")
        .expect_created("create");
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
                notes: None,
                deadline_utc: None,
                handover_date_utc: None,
                items: vec![ActUpdateItemDto {
                    device_id: d2,
                    complectation_at_time: None,
                }],
                confirm_group_release: true,
            },
        )
        .await
        .expect("update")
        .expect_created("update");
    let ev = events(
        &ctx,
        "subject_id = ?1 AND kind = 'status_changed'",
        vec![int(d1)],
    )
    .await;
    // UAT-1: событие с act_id живёт, пока жива позиция акта. Снятая правкой позиция стирает
    // смену статуса этого акта, восстановление статуса идёт молча — компенсирующей записи нет.
    assert!(
        ev.is_empty(),
        "снятая позиция не несёт событий акта: {ev:?}"
    );
    let kept = events(
        &ctx,
        "subject_id = ?1 AND kind = 'status_changed'",
        vec![int(d2)],
    )
    .await;
    assert_eq!(
        kept.len(),
        1,
        "оставшаяся позиция сохраняет событие акта: {kept:?}"
    );
    assert_eq!(kept[0].2, Some(act.id));
}

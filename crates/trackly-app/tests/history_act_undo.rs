//! Фаза 41.10, план 07 (D-24): откат/удаление акта удаляет события истории по `act_id`,
//! компенсирующей записи нет. Префикс тестов: `history_act_undo_`. Фикстуры вымышленные.

use rusqlite::params;

use trackly_app::context::AppCtx;
use trackly_app::dto::act::{ActCreateDto, ActItemNewDto};
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

async fn count_all(ctx: &AppCtx) -> i64 {
    ctx.writer
        .execute(|conn| {
            conn.query_row("SELECT COUNT(*) FROM history_events", [], |r| r.get(0))
                .map_err(map_rusqlite)
        })
        .await
        .expect("count")
}

async fn seed_manual_event(ctx: &AppCtx, dev: i64) {
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO history_events (operation_id, subject_type, subject_id, kind, \
                 source, created_at_utc) VALUES ('manual-op', 'device', ?1, 'status_changed', 'manual', 1700000001)",
                params![dev],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("manual event");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_act_undo_deletes_events() {
    let (ctx, _d) = make_test_ctx().await;
    let a = seed_place(&ctx, "Каб. 501").await;
    let b = seed_place(&ctx, "Каб. 502").await;
    let d1 = seed_device(&ctx, "Ноутбук А", a).await;
    let d2 = seed_device(&ctx, "Ноутбук Б", a).await;
    seed_manual_event(&ctx, d1).await;
    let act1 = ctx
        .acts
        .create(&admin(), create_dto("1", b, &[d1]))
        .await
        .expect("c1")
        .expect_created("c1");
    let act2 = ctx
        .acts
        .create(&admin(), create_dto("2", b, &[d2]))
        .await
        .expect("c2")
        .expect_created("c2");
    assert!(!events(&ctx, "act_id = ?1", vec![int(act1.id)])
        .await
        .is_empty());
    ctx.acts
        .delete_soft(act1.id, act1.version)
        .await
        .expect("delete");
    assert!(events(&ctx, "act_id = ?1", vec![int(act1.id)])
        .await
        .is_empty());
    // чужой акт и ручное событие остались
    assert!(!events(&ctx, "act_id = ?1", vec![int(act2.id)])
        .await
        .is_empty());
    assert_eq!(events(&ctx, "source = 'manual'", vec![]).await.len(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_act_undo_no_compensating_event() {
    let (ctx, _d) = make_test_ctx().await;
    let a = seed_place(&ctx, "Каб. 601").await;
    let b = seed_place(&ctx, "Каб. 602").await;
    let dev = seed_device(&ctx, "Принтер Тест", a).await;
    let act = ctx
        .acts
        .create(&admin(), create_dto("1", b, &[dev]))
        .await
        .expect("create")
        .expect_created("create");
    ctx.acts
        .delete_soft(act.id, act.version)
        .await
        .expect("delete");
    // откат (restore_from_snapshot) молчит и не добавляет «акт отменён»
    assert_eq!(count_all(&ctx).await, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_act_undo_restore_silent() {
    let (ctx, _d) = make_test_ctx().await;
    let a = seed_place(&ctx, "Каб. 701").await;
    let b = seed_place(&ctx, "Каб. 702").await;
    let dev = seed_device(&ctx, "Монитор Тест", a).await;
    seed_manual_event(&ctx, dev).await;
    let act = ctx
        .acts
        .create(&admin(), create_dto("1", b, &[dev]))
        .await
        .expect("create")
        .expect_created("create");
    ctx.acts
        .delete_soft(act.id, act.version)
        .await
        .expect("delete");
    let ev = events(&ctx, "subject_id = ?1", vec![int(dev)]).await;
    assert_eq!(ev.len(), 1, "остаётся только ручное: {ev:?}");
    assert_eq!(ev[0].1, "manual");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_act_undo_group_left() {
    let (ctx, _d) = make_test_ctx().await;
    let a = seed_place(&ctx, "Каб. 801").await;
    let b = seed_place(&ctx, "Каб. 802").await;
    let dev = seed_device(&ctx, "Системный блок Тест", a).await;
    let mate = seed_device(&ctx, "Монитор Тест", a).await;
    seed_manual_event(&ctx, dev).await;
    let type_id: i64 = ctx
        .writer
        .execute(|conn| {
            conn.query_row(
                "SELECT id FROM group_types WHERE code = 'workstation'",
                [],
                |r| r.get(0),
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("group type");
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO groups (type_id, name, seq, created_at_utc, updated_at_utc, version) \
                 VALUES (?1, 'АРМ Тест', 1, 1700000000, 1700000000, 1)",
                params![type_id],
            )
            .map_err(map_rusqlite)?;
            let g = conn.last_insert_rowid();
            conn.execute(
                "INSERT INTO group_devices (device_id, group_id, added_at_utc) VALUES (?1, ?2, 1700000000)",
                params![dev, g],
            )
            .map_err(map_rusqlite)?;
            conn.execute(
                "INSERT INTO group_devices (device_id, group_id, added_at_utc) VALUES (?1, ?2, 1700000000)",
                params![mate, g],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("seed group");
    let act = ctx
        .acts
        .create(&admin(), create_dto("1", b, &[dev]))
        .await
        .expect("create")
        .expect_created("create");
    let left = events(
        &ctx,
        "subject_id = ?1 AND kind = 'group_left'",
        vec![int(dev)],
    )
    .await;
    assert_eq!(left.len(), 1, "{left:?}");
    assert_eq!(left[0].1, "act");
    assert_eq!(left[0].2, Some(act.id));
    ctx.acts
        .delete_soft(act.id, act.version)
        .await
        .expect("delete");
    assert!(events(&ctx, "act_id = ?1", vec![int(act.id)])
        .await
        .is_empty());
    let rest = events(&ctx, "subject_id = ?1", vec![int(dev)]).await;
    assert!(rest.iter().any(|e| e.1 == "manual"), "{rest:?}");
}

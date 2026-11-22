//! Фаза 41.10, план 10 (D-09, D-24): групповые акты пишут `group_unnested`/`group_left`
//! с причиной act, откат акта удаляет события по `act_id`, возврат вложенности событий не пишет.
//! Префикс тестов: `history_group_act_`. Фикстуры вымышленные.

use rusqlite::params;

use trackly_app::context::AppCtx;
use trackly_app::dto::act::{ActCreateDto, ActItemNewDto, ActSaveOutcome};
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

async fn seed_group(ctx: &AppCtx, name: &str, place: i64, parent: Option<i64>) -> i64 {
    let name = name.to_string();
    ctx.writer
        .execute(move |conn| {
            let type_id: i64 = match conn.query_row(
                "SELECT id FROM group_types WHERE behavior = 'container' LIMIT 1",
                [],
                |r| r.get(0),
            ) {
                Ok(id) => id,
                Err(_) => {
                    conn.execute(
                        "INSERT INTO group_types (code, name, behavior, created_at_utc, updated_at_utc) \
                         VALUES ('hga_t', 'Тип теста', 'container', 1700000000, 1700000000)",
                        [],
                    )
                    .map_err(map_rusqlite)?;
                    conn.last_insert_rowid()
                }
            };
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

async fn seed_device(ctx: &AppCtx, name: &str, place: i64, group: Option<i64>) -> i64 {
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

fn dto(number: &str, ids: &[i64], place: i64) -> ActCreateDto {
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
        items: ids
            .iter()
            .map(|&id| ActItemNewDto {
                device_id: id,
                device_ids: vec![id],
                quantity: 1,
            })
            .collect(),
        confirm_group_release: true,
    }
}

async fn create(ctx: &AppCtx, payload: ActCreateDto) -> (i64, i64) {
    match ctx.acts.create(&admin(), payload).await.expect("create") {
        ActSaveOutcome::Created(a) => (a.id, a.version),
        other => panic!("ожидали Created, получили {other:?}"),
    }
}

/// (kind, source, act_id, subject_id, from_group_id, id)
type Ev = (String, String, Option<i64>, i64, Option<i64>, i64);

async fn events(ctx: &AppCtx, kind: &'static str) -> Vec<Ev> {
    ctx.writer
        .execute(move |conn| {
            let mut st = conn
                .prepare(
                    "SELECT kind, source, act_id, subject_id, from_group_id, id \
                     FROM history_events WHERE kind = ?1 ORDER BY id",
                )
                .map_err(map_rusqlite)?;
            let rows = st
                .query_map(params![kind], |r| {
                    Ok((
                        r.get(0)?,
                        r.get(1)?,
                        r.get(2)?,
                        r.get(3)?,
                        r.get(4)?,
                        r.get(5)?,
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

async fn count_by_act(ctx: &AppCtx, act: i64) -> i64 {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(
                "SELECT COUNT(*) FROM history_events WHERE act_id = ?1",
                params![act],
                |r| r.get(0),
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("count")
}

/// Контейнер с вложенной группой; ноутбук — якорь вложенной группы (акт отцепляет её).
struct Nested {
    container: i64,
    nested: i64,
    laptop: i64,
}

async fn seed_nested(ctx: &AppCtx, a: i64) -> Nested {
    let container = seed_group(ctx, "АРМ #3", a, None).await;
    seed_device(ctx, "Монитор #2", a, Some(container)).await;
    let nested = seed_group(ctx, "Замена #5", a, Some(container)).await;
    let laptop = seed_device(ctx, "Ноутбук #5", a, Some(nested)).await;
    set_anchor(ctx, nested, laptop).await;
    seed_device(ctx, "Адаптер #5", a, Some(nested)).await;
    Nested {
        container,
        nested,
        laptop,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_group_act_detach_nested() {
    let (ctx, _d) = make_test_ctx().await;
    let a = seed_place(&ctx, "Склад А").await;
    let b = seed_place(&ctx, "Склад Б").await;
    let n = seed_nested(&ctx, a).await;
    let (act, _) = create(&ctx, dto("HGA-1", &[n.laptop], b)).await;
    let ev = events(&ctx, "group_unnested").await;
    assert_eq!(ev.len(), 1, "{ev:?}");
    assert_eq!(ev[0].1, "act");
    assert_eq!(ev[0].2, Some(act));
    assert_eq!(ev[0].3, n.nested);
    assert_eq!(ev[0].4, Some(n.container));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_group_act_member_release() {
    let (ctx, _d) = make_test_ctx().await;
    let a = seed_place(&ctx, "Склад А").await;
    let b = seed_place(&ctx, "Склад Б").await;
    let g = seed_group(&ctx, "АРМ #7", a, None).await;
    let d1 = seed_device(&ctx, "Системный блок #1", a, Some(g)).await;
    seed_device(&ctx, "Монитор #1", a, Some(g)).await;
    seed_device(&ctx, "Клавиатура #1", a, Some(g)).await;
    let (act, _) = create(&ctx, dto("HGA-2", &[d1], b)).await;
    let ev = events(&ctx, "group_left").await;
    assert_eq!(ev.len(), 1, "{ev:?}");
    assert_eq!(ev[0].1, "act");
    assert_eq!(ev[0].2, Some(act));
    assert_eq!(ev[0].3, d1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_group_act_undo_clean() {
    let (ctx, _d) = make_test_ctx().await;
    let a = seed_place(&ctx, "Склад А").await;
    let b = seed_place(&ctx, "Склад Б").await;
    let n = seed_nested(&ctx, a).await;
    let g = seed_group(&ctx, "АРМ #8", a, None).await;
    let d1 = seed_device(&ctx, "Системный блок #2", a, Some(g)).await;
    seed_device(&ctx, "Монитор #3", a, Some(g)).await;
    seed_device(&ctx, "Клавиатура #2", a, Some(g)).await;
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO history_events (operation_id, subject_type, subject_id, kind, \
                 source, created_at_utc) VALUES ('manual-op', 'device', ?1, 'status_changed', 'manual', 1700000001)",
                params![d1],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("manual");
    let (act, ver) = create(&ctx, dto("HGA-3", &[n.laptop, d1], b)).await;
    assert!(count_by_act(&ctx, act).await >= 2);
    assert!(!events(&ctx, "group_unnested").await.is_empty());
    assert!(!events(&ctx, "group_left").await.is_empty());
    ctx.acts.delete_soft(act, ver).await.expect("delete");
    assert_eq!(count_by_act(&ctx, act).await, 0);
    assert!(events(&ctx, "group_unnested").await.is_empty());
    assert!(events(&ctx, "group_left").await.is_empty());
    let manual = ctx
        .writer
        .execute(|conn| {
            conn.query_row(
                "SELECT COUNT(*) FROM history_events WHERE source = 'manual'",
                [],
                |r| r.get::<_, i64>(0),
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("manual count");
    assert_eq!(manual, 1, "события других операций целы");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_group_act_undo_no_nesting_event() {
    let (ctx, _d) = make_test_ctx().await;
    let a = seed_place(&ctx, "Склад А").await;
    let b = seed_place(&ctx, "Склад Б").await;
    let n = seed_nested(&ctx, a).await;
    let (act, ver) = create(&ctx, dto("HGA-4", &[n.laptop], b)).await;
    ctx.acts.delete_soft(act, ver).await.expect("delete");
    // вложенность возвращена, но ни group_nested, ни group_unnested не появилось
    let parent: Option<i64> = ctx
        .writer
        .execute(move |conn| {
            conn.query_row(
                "SELECT parent_group_id FROM groups WHERE id = ?1",
                params![n.nested],
                |r| r.get(0),
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("parent");
    assert_eq!(parent, Some(n.container), "вложенность вернулась");
    assert!(events(&ctx, "group_nested").await.is_empty());
    assert!(events(&ctx, "group_unnested").await.is_empty());
}

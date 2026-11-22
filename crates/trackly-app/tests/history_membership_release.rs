//! Фаза 41.10 план 06: события истории общих примитивов состава и переноса группы
//! (`group_left` при выводе, `status_changed` с причиной `group_move` при переносе группы).
//! Реальный `AppCtx` на временном каталоге; имена вымышленные.
//!
//! Префиксы тестов: `history_membership_release_`, `history_membership_group_move_`.
//! Сценарии с `act_id` и restore-путями живут рядом с примитивом (lib-тесты
//! `history_membership_release_*` в `services/group_membership.rs`).

use rusqlite::params;

use trackly_app::context::AppCtx;
use trackly_core::auth::Identity;
use trackly_infra::error_conversions::map_rusqlite;

async fn build_ctx_in(dir: &std::path::Path) -> anyhow::Result<AppCtx> {
    let paths = trackly_infra::Paths::resolve_for_exe_dir(dir.to_path_buf())?;
    let config = trackly_infra::AppConfig::default();
    let log_guard = trackly_app::logging::init(&paths, &config).or_else(|_| {
        let (_nb, guard) = tracing_appender::non_blocking(std::io::sink());
        Ok::<_, anyhow::Error>(guard)
    })?;
    AppCtx::build(paths, config, log_guard).await
}

async fn make_test_ctx() -> (AppCtx, tempfile::TempDir) {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let ctx = build_ctx_in(dir.path()).await.expect("build ctx");
    (ctx, dir)
}

fn admin() -> Identity {
    Identity::trusted_admin()
}

async fn scalar_i64(ctx: &AppCtx, sql: &'static str, p: Vec<rusqlite::types::Value>) -> i64 {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(sql, rusqlite::params_from_iter(p), |r| r.get::<_, i64>(0))
                .map_err(map_rusqlite)
        })
        .await
        .expect("scalar")
}

fn int(v: i64) -> rusqlite::types::Value {
    rusqlite::types::Value::Integer(v)
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

async fn seed_place(ctx: &AppCtx, name: &str, parent: Option<i64>, storage: bool) -> i64 {
    let name = name.to_string();
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO places (kind, name, parent_id, is_storage, created_at_utc, \
                 updated_at_utc, version) VALUES ('room', ?1, ?2, ?3, 1700000000, 1700000000, 1)",
                params![name, parent, i64::from(storage)],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed place")
}

async fn seed_group(
    ctx: &AppCtx,
    name: &str,
    seq: i64,
    place: Option<i64>,
    parent: Option<i64>,
) -> i64 {
    let type_id = type_id_by_code(ctx, "workstation").await;
    let name = name.to_string();
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO groups (type_id, name, seq, place_id, parent_group_id, \
                 created_at_utc, updated_at_utc, version) \
                 VALUES (?1, ?2, ?3, ?4, ?5, 1700000000, 1700000000, 1)",
                params![type_id, name, seq, place, parent],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed group")
}

async fn seed_device(
    ctx: &AppCtx,
    name: &str,
    inv: &str,
    place: Option<i64>,
    group: Option<i64>,
    status_code: &'static str,
) -> i64 {
    let (name, inv) = (name.to_string(), inv.to_string());
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO devices (type_id, name, inventory_number, place_id, status_id, \
                 created_at_utc, updated_at_utc, version) \
                 VALUES (1, ?1, ?2, ?3, (SELECT id FROM device_statuses WHERE code = ?4), \
                 1700000000, 1700000000, 1)",
                params![name, inv, place, status_code],
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

async fn dev_state(ctx: &AppCtx, id: i64) -> (Option<i64>, String) {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(
                "SELECT d.place_id, s.code FROM devices d \
                 JOIN device_statuses s ON s.id = d.status_id WHERE d.id = ?1",
                params![id],
                |r| Ok((r.get::<_, Option<i64>>(0)?, r.get::<_, String>(1)?)),
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("dev state")
}

async fn group_version(ctx: &AppCtx, id: i64) -> i64 {
    scalar_i64(
        ctx,
        "SELECT version FROM groups WHERE id = ?1",
        vec![int(id)],
    )
    .await
}

#[allow(clippy::type_complexity)]
async fn events(
    ctx: &AppCtx,
    kind: &'static str,
    device_id: i64,
) -> Vec<(String, Option<i64>, Option<i64>, Option<i64>, i64)> {
    ctx.writer
        .execute(move |conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT source, act_id, ref_group_id, movement_id, id FROM history_events \
                     WHERE kind = ?1 AND subject_id = ?2 ORDER BY id",
                )
                .map_err(map_rusqlite)?;
            let rows = stmt
                .query_map(params![kind, device_id], |r| {
                    Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
                })
                .map_err(map_rusqlite)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(map_rusqlite)?;
            Ok(rows)
        })
        .await
        .expect("events")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_membership_release_writes_left() {
    let (ctx, _d) = make_test_ctx().await;
    let room = seed_place(&ctx, "Кабинет 12", None, false).await;
    let g = seed_group(&ctx, "АРМ #3", 3, Some(room), None).await;
    let dev = seed_device(&ctx, "Монитор #2", "INV-2", Some(room), Some(g), "в_работе").await;
    assert!(events(&ctx, "group_left", dev).await.is_empty());

    ctx.groups
        .remove_devices(&admin(), g, vec![dev])
        .await
        .expect("remove");

    let ev = events(&ctx, "group_left", dev).await;
    assert_eq!(ev.len(), 1, "{ev:?}");
    assert_eq!(ev[0].0, "manual");
    assert_eq!(ev[0].1, None, "вне акта act_id пуст");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_membership_group_move_status_shift() {
    let (ctx, _d) = make_test_ctx().await;
    let store = seed_place(&ctx, "Склад А", None, true).await;
    let room = seed_place(&ctx, "Кабинет 12", None, false).await;
    let g = seed_group(&ctx, "АРМ #9", 9, Some(store), None).await;
    let dev = seed_device(
        &ctx,
        "Монитор #20",
        "INV-20",
        Some(store),
        Some(g),
        "на_складе",
    )
    .await;
    assert!(events(&ctx, "status_changed", dev).await.is_empty());

    let version = group_version(&ctx, g).await;
    ctx.groups
        .move_group_with_status(&admin(), g, version, room, true)
        .await
        .expect("move");
    assert_eq!(dev_state(&ctx, dev).await, (Some(room), "в_работе".into()));

    let ev = events(&ctx, "status_changed", dev).await;
    assert_eq!(ev.len(), 1, "{ev:?}");
    assert_eq!(ev[0].0, "group_move");
    assert_eq!(ev[0].2, Some(g), "группа-основание");
    let (mv_id, mv_dev): (i64, i64) = ctx
        .writer
        .execute(move |conn| {
            conn.query_row(
                "SELECT id, entity_id FROM place_movements \
                 WHERE entity_type = 'device' AND entity_id = ?1",
                params![dev],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("movement");
    assert_eq!(mv_dev, dev);
    assert_eq!(ev[0].3, Some(mv_id), "movement_id события = id перемещения");
}

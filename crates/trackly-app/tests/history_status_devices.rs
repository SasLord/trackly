//! Фаза 41.10, план 05 (HST-05/HST-06): события `status_changed` у писателей статуса
//! `device_service::update` (ST-1) и `place_service::move_subtree_contents` (ST-11).
//!
//! Фикстуры вымышленные; реальная БД во временном каталоге.

use rusqlite::params;

use trackly_app::context::AppCtx;
use trackly_app::dto::auth::UserNew;
use trackly_app::dto::device::{DeviceNew, DevicePatch};
use trackly_core::auth::{Identity, Role};
use trackly_infra::error_conversions::map_rusqlite;

async fn make_test_ctx() -> anyhow::Result<(AppCtx, tempfile::TempDir)> {
    let dir = tempfile::TempDir::new()?;
    let paths = trackly_infra::Paths::resolve_for_exe_dir(dir.path().to_path_buf())?;
    let config = trackly_infra::AppConfig::default();
    let log_guard = trackly_app::logging::init(&paths, &config).or_else(|_| {
        let (_nb, guard) = tracing_appender::non_blocking(std::io::sink());
        Ok::<_, anyhow::Error>(guard)
    })?;
    let ctx = AppCtx::build(paths, config, log_guard).await?;
    Ok((ctx, dir))
}

async fn manager(ctx: &AppCtx) -> Identity {
    let dto = ctx
        .auth
        .create_user(
            UserNew {
                login: "petrov.pp".to_string(),
                full_name: "Петров П.П.".to_string(),
                password: "password123".to_string(),
                role: "manager".to_string(),
                email: None,
            },
            &Identity::trusted_admin(),
        )
        .await
        .expect("create user");
    Identity {
        user_id: Some(dto.id),
        role: Role::Manager,
    }
}

async fn seed_place(ctx: &AppCtx, name: &str) -> i64 {
    let name = name.to_string();
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO places (kind, name, is_storage, created_at_utc, updated_at_utc, version) \
                 VALUES ('room', ?1, 0, ?2, ?2, 1)",
                params![name, 1_700_000_000_i64],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed place")
}

/// Два обычных (не исключённых из группы и не «Списано») статуса справочника.
async fn two_statuses(ctx: &AppCtx) -> (i64, i64) {
    ctx.writer
        .execute(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT id FROM device_statuses \
                     WHERE code NOT IN ('на ремонте','в ремонте','на списание','списано') \
                     ORDER BY id LIMIT 2",
                )
                .map_err(map_rusqlite)?;
            let ids: Vec<i64> = stmt
                .query_map([], |r| r.get(0))
                .map_err(map_rusqlite)?
                .collect::<Result<_, _>>()
                .map_err(map_rusqlite)?;
            Ok((ids[0], ids[1]))
        })
        .await
        .expect("statuses")
}

fn new_device(name: &str, place_id: Option<i64>, status_id: i64) -> DeviceNew {
    DeviceNew {
        type_id: 1,
        name: name.to_string(),
        inventory_no: None,
        serial_no: None,
        model: None,
        specs: None,
        kit: None,
        state: None,
        place_id,
        status_id,
        origin_group_id: None,
    }
}

fn patch(place_id: Option<Option<i64>>, status_id: Option<i64>) -> DevicePatch {
    DevicePatch {
        type_id: None,
        name: None,
        number_input: None,
        serial_no: None,
        model: None,
        specs: None,
        kit: None,
        state: None,
        place_id,
        status_id,
        usb_host_device_id: None,
        printer_ip_address: None,
        printer_community: None,
        group_write_off: None,
    }
}

type EventRow = (
    i64,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<i64>,
    Option<i64>,
    Option<String>,
);

/// (id, kind, source, status_from_code, status_to_code, movement_id, user_id, actor)
async fn events(ctx: &AppCtx, device_id: i64) -> Vec<EventRow> {
    ctx.writer
        .execute(move |conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT id, kind, source, status_from_code, status_to_code, movement_id, \
                     user_id, actor_name_snapshot FROM history_events \
                     WHERE subject_type='device' AND subject_id=?1 ORDER BY id",
                )
                .map_err(map_rusqlite)?;
            let rows: Vec<EventRow> = stmt
                .query_map(params![device_id], |r| {
                    Ok((
                        r.get(0)?,
                        r.get(1)?,
                        r.get(2)?,
                        r.get(3)?,
                        r.get(4)?,
                        r.get(5)?,
                        r.get(6)?,
                        r.get(7)?,
                    ))
                })
                .map_err(map_rusqlite)?
                .collect::<Result<_, _>>()
                .map_err(map_rusqlite)?;
            Ok(rows)
        })
        .await
        .expect("events")
}

async fn movement_ids(ctx: &AppCtx, device_id: i64) -> Vec<i64> {
    ctx.writer
        .execute(move |conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT id FROM place_movements WHERE entity_type='device' AND entity_id=?1 ORDER BY id",
                )
                .map_err(map_rusqlite)?;
            let rows: Vec<i64> = stmt
                .query_map(params![device_id], |r| r.get(0))
                .map_err(map_rusqlite)?
                .collect::<Result<_, _>>()
                .map_err(map_rusqlite)?;
            Ok(rows)
        })
        .await
        .expect("movements")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_status_manual_change_writes_event() {
    let (ctx, _dir) = make_test_ctx().await.unwrap();
    let who = manager(&ctx).await;
    let (s1, s2) = two_statuses(&ctx).await;
    let place = seed_place(&ctx, "Каб. 101").await;
    let dev = ctx
        .devices
        .create(new_device("Ноутбук Тест", Some(place), s1))
        .await
        .unwrap()
        .expect_created("create");
    ctx.devices
        .update(&who, dev.id, dev.version, patch(None, Some(s2)))
        .await
        .expect("update");
    let ev = events(&ctx, dev.id).await;
    assert_eq!(ev.len(), 1, "{ev:?}");
    let e = &ev[0];
    assert_eq!(e.1, "status_changed");
    assert_eq!(e.2, "manual");
    assert!(e.3.is_some() && e.4.is_some());
    assert_ne!(e.3, e.4);
    assert_eq!(e.6, who.user_id);
    assert_eq!(e.7.as_deref(), Some("Петров П.П."));
    assert_eq!(e.5, None, "место не менялось -> movement_id пуст");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_status_same_status_writes_nothing() {
    let (ctx, _dir) = make_test_ctx().await.unwrap();
    let who = manager(&ctx).await;
    let (s1, _) = two_statuses(&ctx).await;
    let dev = ctx
        .devices
        .create(new_device("Монитор Тест", None, s1))
        .await
        .unwrap()
        .expect_created("create");
    ctx.devices
        .update(&who, dev.id, dev.version, patch(None, Some(s1)))
        .await
        .expect("update");
    assert!(events(&ctx, dev.id).await.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_status_with_place_change_links_movement() {
    let (ctx, _dir) = make_test_ctx().await.unwrap();
    let who = manager(&ctx).await;
    let (s1, s2) = two_statuses(&ctx).await;
    let a = seed_place(&ctx, "Каб. 201").await;
    let b = seed_place(&ctx, "Каб. 202").await;
    let dev = ctx
        .devices
        .create(new_device("Принтер Тест", Some(a), s1))
        .await
        .unwrap()
        .expect_created("create");
    ctx.devices
        .update(&who, dev.id, dev.version, patch(Some(Some(b)), Some(s2)))
        .await
        .expect("update");
    let ev = events(&ctx, dev.id).await;
    let mv = movement_ids(&ctx, dev.id).await;
    assert_eq!(ev.len(), 1);
    assert_eq!(mv.len(), 1);
    assert_eq!(ev[0].5, Some(mv[0]));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_status_event_written_after_movement() {
    let (ctx, _dir) = make_test_ctx().await.unwrap();
    let who = manager(&ctx).await;
    let (s1, s2) = two_statuses(&ctx).await;
    let a = seed_place(&ctx, "Каб. 301").await;
    let b = seed_place(&ctx, "Каб. 302").await;
    let dev = ctx
        .devices
        .create(new_device("Сканер Тест", Some(a), s1))
        .await
        .unwrap()
        .expect_created("create");
    ctx.devices
        .update(&who, dev.id, dev.version, patch(Some(Some(b)), Some(s2)))
        .await
        .expect("update");
    let ev = events(&ctx, dev.id).await;
    let mv = movement_ids(&ctx, dev.id).await;
    assert!(ev[0].5.is_some(), "movement_id заполнен");
    assert_eq!(ev[0].5, mv.last().copied());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_status_create_device_writes_nothing() {
    let (ctx, _dir) = make_test_ctx().await.unwrap();
    let (s1, _) = two_statuses(&ctx).await;
    let dev = ctx
        .devices
        .create(new_device("Ноутбук Новый", None, s1))
        .await
        .unwrap()
        .expect_created("create");
    assert!(events(&ctx, dev.id).await.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_status_bulk_move_equal_status_writes_nothing() {
    let (ctx, _dir) = make_test_ctx().await.unwrap();
    let who = manager(&ctx).await;
    let (s1, _) = two_statuses(&ctx).await;
    let root = seed_place(&ctx, "Каб. 401").await;
    let target = seed_place(&ctx, "Склад-2").await;
    let dev = ctx
        .devices
        .create(new_device("Ноутбук Перенос", Some(root), s1))
        .await
        .unwrap()
        .expect_created("create");
    let moved = ctx
        .places
        .move_subtree_contents(&who, root, target, None)
        .await
        .expect("move");
    assert!(moved >= 1);
    assert_eq!(movement_ids(&ctx, dev.id).await.len(), 1);
    assert!(events(&ctx, dev.id).await.is_empty());
}

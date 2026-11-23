//! Фаза 41.10 план 09: события статуса и причины списания из `GroupService`
//! («Разбор», списание устройства, списание группы, перенос со сменой статуса).
//! Реальный `AppCtx` на временном каталоге; имена вымышленные.

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

async fn type_by_behavior(ctx: &AppCtx, behavior: &str) -> i64 {
    ctx.group_types
        .list_types(&admin(), false)
        .await
        .expect("list_types")
        .into_iter()
        .find(|t| t.behavior == behavior)
        .unwrap_or_else(|| panic!("тип с поведением {behavior} не найден"))
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
    let type_id = type_by_behavior(ctx, "container").await;
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

async fn group_version(ctx: &AppCtx, id: i64) -> i64 {
    scalar_i64(
        ctx,
        "SELECT version FROM groups WHERE id = ?1",
        vec![int(id)],
    )
    .await
}

use trackly_infra::repos::history_events_sqlite::HistoryEventRow;
use trackly_infra::repos::SqliteHistoryEventsRepository;

async fn device_events(ctx: &AppCtx, id: i64) -> Vec<HistoryEventRow> {
    ctx.writer
        .execute(move |conn| SqliteHistoryEventsRepository.list_for_device(conn, id))
        .await
        .expect("list_for_device")
}

fn of_kind<'a>(rows: &'a [HistoryEventRow], kind: &str) -> Vec<&'a HistoryEventRow> {
    rows.iter().filter(|r| r.kind == kind).collect()
}

fn create_dto(type_id: i64, source: i64) -> trackly_app::dto::groups::GroupCreateDto {
    trackly_app::dto::groups::GroupCreateDto {
        type_id,
        name: None,
        place_id: None,
        source_device_id: Some(source),
    }
}

async fn set_status(ctx: &AppCtx, device: i64, code: &'static str) {
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "UPDATE devices SET status_id = (SELECT id FROM device_statuses WHERE code = ?2) \
                 WHERE id = ?1",
                params![device, code],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("set status");
}

async fn device_version(ctx: &AppCtx, id: i64) -> i64 {
    scalar_i64(
        ctx,
        "SELECT version FROM devices WHERE id = ?1",
        vec![int(id)],
    )
    .await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_status_teardown_create() {
    let (ctx, _d) = make_test_ctx().await;
    let room = seed_place(&ctx, "Кабинет 12", None, false).await;
    let anchor = seed_device(
        &ctx,
        "Системный блок #4",
        "INV-4",
        Some(room),
        None,
        "в_работе",
    )
    .await;
    let type_id = type_by_behavior(&ctx, "teardown").await;

    ctx.groups
        .create_group(&admin(), create_dto(type_id, anchor))
        .await
        .expect("create");

    let ev = device_events(&ctx, anchor).await;
    let st = of_kind(&ev, "status_changed");
    assert_eq!(st.len(), 1, "{ev:?}");
    assert_eq!(st[0].source, "teardown");
    assert_eq!(st[0].status_from_code.as_deref(), Some("в_работе"));
    assert_eq!(st[0].status_to_code.as_deref(), Some("на_списание"));
    assert_eq!(st[0].movement_id, None);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_status_write_off_device() {
    let (ctx, _d) = make_test_ctx().await;
    let room = seed_place(&ctx, "Кабинет 12", None, false).await;
    let g = seed_group(&ctx, "АРМ-3", 3, Some(room), None).await;
    let dev = seed_device(
        &ctx,
        "Монитор #2",
        "INV-2",
        Some(room),
        Some(g),
        "на_списание",
    )
    .await;
    let version = device_version(&ctx, dev).await;

    ctx.groups
        .write_off_device(
            &admin(),
            trackly_app::dto::groups::GroupWriteOffDeviceDto {
                device_id: dev,
                version,
                release_from_group: true,
            },
        )
        .await
        .expect("write_off_device");

    let ev = device_events(&ctx, dev).await;
    let st = of_kind(&ev, "status_changed");
    assert_eq!(st.len(), 1, "{ev:?}");
    assert_eq!(st[0].source, "device_write_off");
    assert_eq!(st[0].status_from_code.as_deref(), Some("на_списание"));
    assert_eq!(st[0].status_to_code.as_deref(), Some("списано"));
    let left = of_kind(&ev, "group_left");
    assert_eq!(left.len(), 1, "{ev:?}");
    assert_eq!(left[0].source, "device_write_off");
    // R8-05 (D-04): одна операция списания — один operation_id у смены статуса и вывода.
    assert_eq!(
        st[0].operation_id, left[0].operation_id,
        "status_changed и group_left списания делят operation_id"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_status_write_off_group() {
    let (ctx, _d) = make_test_ctx().await;
    let room = seed_place(&ctx, "Кабинет 12", None, false).await;
    let anchor = seed_device(
        &ctx,
        "Системный блок #4",
        "INV-4",
        Some(room),
        None,
        "в_работе",
    )
    .await;
    let member = seed_device(&ctx, "Монитор #2", "INV-2", Some(room), None, "в_работе").await;
    let type_id = type_by_behavior(&ctx, "substitute").await;
    let g = ctx
        .groups
        .create_group(&admin(), create_dto(type_id, anchor))
        .await
        .expect("create");
    ctx.groups
        .add_devices(&admin(), g.id, vec![member])
        .await
        .expect("add");
    set_status(&ctx, anchor, "на_списание").await;
    let version = group_version(&ctx, g.id).await;

    ctx.groups
        .write_off_group(
            &admin(),
            trackly_app::dto::groups::GroupWriteOffGroupDto { id: g.id, version },
        )
        .await
        .expect("write_off_group");

    let aev = device_events(&ctx, anchor).await;
    let st = of_kind(&aev, "status_changed");
    assert_eq!(st.len(), 1, "{aev:?}");
    assert_eq!(st[0].source, "group_write_off");
    assert_eq!(st[0].ref_group_id, Some(g.id));
    assert!(!of_kind(&aev, "group_disbanded").is_empty(), "{aev:?}");

    let mev = device_events(&ctx, member).await;
    let left = of_kind(&mev, "group_left");
    assert_eq!(left.len(), 1, "{mev:?}");
    assert_eq!(left[0].source, "group_write_off");
    assert_eq!(left[0].ref_group_id, Some(g.id));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_status_group_move_shift() {
    let (ctx, _d) = make_test_ctx().await;
    let store = seed_place(&ctx, "Склад", None, true).await;
    let room = seed_place(&ctx, "Кабинет 12", None, false).await;
    let g = seed_group(&ctx, "АРМ-3", 3, Some(store), None).await;
    let dev = seed_device(
        &ctx,
        "Монитор #2",
        "INV-2",
        Some(store),
        Some(g),
        "на_складе",
    )
    .await;
    let version = group_version(&ctx, g).await;

    ctx.groups
        .move_group_with_status(&admin(), g, version, room, true)
        .await
        .expect("move");

    let ev = device_events(&ctx, dev).await;
    let st = of_kind(&ev, "status_changed");
    assert_eq!(st.len(), 1, "{ev:?}");
    assert_eq!(st[0].source, "group_move");
    assert_eq!(st[0].ref_group_id, Some(g));
    assert!(st[0].movement_id.is_some(), "{ev:?}");
}

//! Фаза 41.10 план 18: события состава группы из `GroupService` (создание, ввод, перенос,
//! вложение, каскад удаления). Реальный `AppCtx` на временном каталоге; имена вымышленные.
//!
//! Читаем через `SqliteHistoryEventsRepository::list_for_device` / `list_for_group`.

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

async fn group_events(ctx: &AppCtx, id: i64) -> Vec<HistoryEventRow> {
    ctx.writer
        .execute(move |conn| SqliteHistoryEventsRepository.list_for_group(conn, id))
        .await
        .expect("list_for_group")
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_membership_create_group_anchor_formed() {
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
    let type_id = type_by_behavior(&ctx, "substitute").await;

    let g = ctx
        .groups
        .create_group(&admin(), create_dto(type_id, anchor))
        .await
        .expect("create");

    let ev = device_events(&ctx, anchor).await;
    let formed = of_kind(&ev, "group_formed");
    assert_eq!(formed.len(), 1, "{ev:?}");
    assert_eq!(formed[0].subject_type, "device");
    assert_eq!(formed[0].source, "group_create");
    assert_eq!(formed[0].to_group_id, Some(g.id));
    assert!(
        of_kind(&ev, "group_joined").is_empty(),
        "якорь не получает joined"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_membership_create_group_without_anchor() {
    let (ctx, _d) = make_test_ctx().await;
    let room = seed_place(&ctx, "Кабинет 12", None, false).await;
    let dev = seed_device(&ctx, "Монитор #2", "INV-2", Some(room), None, "в_работе").await;
    let type_id = type_by_behavior(&ctx, "container").await;

    let g = ctx
        .groups
        .create_group(&admin(), create_dto(type_id, dev))
        .await
        .expect("create");

    let gev = group_events(&ctx, g.id).await;
    let formed = of_kind(&gev, "group_formed");
    assert_eq!(formed.len(), 1, "{gev:?}");
    assert_eq!(formed[0].subject_type, "group");
    assert_eq!(formed[0].subject_id, g.id);
    let dev_ev = device_events(&ctx, dev).await;
    let joined = of_kind(&dev_ev, "group_joined");
    assert_eq!(joined.len(), 1, "{dev_ev:?}");
    assert_eq!(joined[0].source, "group_create");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_membership_add_devices_joined_manual() {
    let (ctx, _d) = make_test_ctx().await;
    let room = seed_place(&ctx, "Кабинет 12", None, false).await;
    let g = seed_group(&ctx, "АРМ-3", 3, Some(room), None).await;
    let dev = seed_device(&ctx, "Монитор #2", "INV-2", Some(room), None, "в_работе").await;

    ctx.groups
        .add_devices(&admin(), g, vec![dev])
        .await
        .expect("add");

    let ev = device_events(&ctx, dev).await;
    let joined = of_kind(&ev, "group_joined");
    assert_eq!(joined.len(), 1, "{ev:?}");
    assert_eq!(joined[0].source, "manual");
    assert_eq!(joined[0].to_group_id, Some(g));
    assert_eq!(joined[0].to_group_label.as_deref(), Some("АРМ-3"));
    assert_eq!(joined[0].movement_id, None, "место не менялось");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_membership_move_is_single_event() {
    let (ctx, _d) = make_test_ctx().await;
    let room = seed_place(&ctx, "Кабинет 12", None, false).await;
    let from = seed_group(&ctx, "АРМ-3", 3, Some(room), None).await;
    let to = seed_group(&ctx, "АРМ-5", 5, Some(room), None).await;
    let dev = seed_device(
        &ctx,
        "Монитор #2",
        "INV-2",
        Some(room),
        Some(from),
        "в_работе",
    )
    .await;

    ctx.groups
        .move_devices(&admin(), to, vec![dev])
        .await
        .expect("move");

    let ev = device_events(&ctx, dev).await;
    let tr = of_kind(&ev, "group_transferred");
    assert_eq!(tr.len(), 1, "{ev:?}");
    assert_eq!(tr[0].from_group_id, Some(from));
    assert_eq!(tr[0].from_group_label.as_deref(), Some("АРМ-3"));
    assert_eq!(tr[0].to_group_id, Some(to));
    assert_eq!(tr[0].to_group_label.as_deref(), Some("АРМ-5"));
    assert!(of_kind(&ev, "group_left").is_empty(), "{ev:?}");
    assert!(of_kind(&ev, "group_joined").is_empty(), "{ev:?}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_membership_move_free_device_writes_joined() {
    let (ctx, _d) = make_test_ctx().await;
    let room = seed_place(&ctx, "Кабинет 12", None, false).await;
    let to = seed_group(&ctx, "АРМ-5", 5, Some(room), None).await;
    let dev = seed_device(&ctx, "Монитор #2", "INV-2", Some(room), None, "в_работе").await;

    ctx.groups
        .move_devices(&admin(), to, vec![dev])
        .await
        .expect("move");

    let ev = device_events(&ctx, dev).await;
    assert_eq!(of_kind(&ev, "group_joined").len(), 1, "{ev:?}");
    assert!(of_kind(&ev, "group_transferred").is_empty(), "{ev:?}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_membership_join_with_place_shares_movement() {
    let (ctx, _d) = make_test_ctx().await;
    let store = seed_place(&ctx, "Склад А", None, true).await;
    let room = seed_place(&ctx, "Кабинет 12", None, false).await;
    let g = seed_group(&ctx, "АРМ-3", 3, Some(room), None).await;
    let dev = seed_device(&ctx, "Монитор #2", "INV-2", Some(store), None, "на_складе").await;

    ctx.groups
        .add_devices(&admin(), g, vec![dev])
        .await
        .expect("add");

    let mv_id: i64 = ctx
        .writer
        .execute(move |conn| {
            conn.query_row(
                "SELECT id FROM place_movements WHERE entity_type = 'device' AND entity_id = ?1",
                params![dev],
                |r| r.get(0),
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("movement");
    let ev = device_events(&ctx, dev).await;
    let joined = of_kind(&ev, "group_joined");
    assert_eq!(joined.len(), 1, "{ev:?}");
    assert_eq!(joined[0].movement_id, Some(mv_id));
    // Статус при вводе в группу не меняется (D-34 — только перенос группы).
    assert!(of_kind(&ev, "status_changed").is_empty(), "{ev:?}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_membership_delete_group_cascade() {
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
    let child = seed_group(&ctx, "АРМ-5", 5, Some(room), Some(g.id)).await;

    ctx.groups
        .delete_group(&admin(), g.id)
        .await
        .expect("delete");

    let aev = device_events(&ctx, anchor).await;
    let dis = of_kind(&aev, "group_disbanded");
    assert_eq!(dis.len(), 1, "{aev:?}");
    assert_eq!(dis[0].source, "group_disband");
    assert!(
        of_kind(&aev, "group_left").is_empty(),
        "якорь не group_left"
    );
    let mev = device_events(&ctx, member).await;
    let left = of_kind(&mev, "group_left");
    assert_eq!(left.len(), 1, "{mev:?}");
    assert_eq!(left[0].source, "group_disband");
    assert_eq!(left[0].from_group_id, Some(g.id));
    let cev = group_events(&ctx, child).await;
    let un = of_kind(&cev, "group_unnested");
    assert_eq!(un.len(), 1, "{cev:?}");
    assert_eq!(un[0].subject_type, "group");
    assert_eq!(un[0].from_group_id, Some(g.id), "прежний родитель");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_membership_set_parent() {
    let (ctx, _d) = make_test_ctx().await;
    let room = seed_place(&ctx, "Кабинет 12", None, false).await;
    let p1 = seed_group(&ctx, "АРМ-3", 3, Some(room), None).await;
    let child = seed_group(&ctx, "АРМ-5", 5, Some(room), None).await;
    let dev = seed_device(
        &ctx,
        "Монитор #2",
        "INV-2",
        Some(room),
        Some(child),
        "в_работе",
    )
    .await;

    let v = group_version(&ctx, child).await;
    ctx.groups
        .set_parent(&admin(), child, v, Some(p1), false)
        .await
        .expect("nest");
    let v = group_version(&ctx, child).await;
    ctx.groups
        .set_parent(&admin(), child, v, None, false)
        .await
        .expect("unnest");

    let cev = group_events(&ctx, child).await;
    let nested = of_kind(&cev, "group_nested");
    let unnested = of_kind(&cev, "group_unnested");
    assert_eq!(nested.len(), 1, "{cev:?}");
    assert_eq!(unnested.len(), 1, "{cev:?}");
    assert_eq!(nested[0].subject_type, "group");
    assert_eq!(nested[0].to_group_id, Some(p1));
    assert_eq!(unnested[0].subject_type, "group");
    assert_eq!(unnested[0].from_group_id, Some(p1));
    // лента устройства вложенной группы событий вложенности не содержит
    let dev_ev = device_events(&ctx, dev).await;
    assert!(of_kind(&dev_ev, "group_nested").is_empty(), "{dev_ev:?}");
    assert!(of_kind(&dev_ev, "group_unnested").is_empty(), "{dev_ev:?}");
}

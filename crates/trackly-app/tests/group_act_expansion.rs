//! Phase 41.2 Plan 06: разворот группы в единицы акта (`GroupService::act_expansion`)
//! и подсказка-группа пикера (`GroupService::search` -> `place_path_short`).
//!
//! Реальный `AppCtx` на временном каталоге; имена вымышленные.
//! Префикс тестов: `expansion_`.

use rusqlite::params;

use trackly_app::context::AppCtx;
use trackly_app::dto::groups::GroupCreateDto;
use trackly_app::services::place_path_display::compute_place_path_short_with_conn;
use trackly_core::auth::Identity;
use trackly_core::error::AppError;
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::repos::SqliteGroupRepository;

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

async fn seed_place(ctx: &AppCtx, name: &str, parent: Option<i64>) -> i64 {
    let name = name.to_string();
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO places (kind, name, parent_id, is_storage, created_at_utc, \
                 updated_at_utc, version) VALUES ('room', ?1, ?2, 0, 1700000000, 1700000000, 1)",
                params![name, parent],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed place")
}

async fn seed_device(ctx: &AppCtx, name: &str, inv: &str, place: Option<i64>) -> i64 {
    let (name, inv) = (name.to_string(), inv.to_string());
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO devices (type_id, name, inventory_number, serial_number, place_id, \
                 status_id, created_at_utc, updated_at_utc, version) \
                 VALUES (1, ?1, ?2, ?3, ?4, 1, 1700000000, 1700000000, 1)",
                params![name, inv, format!("SN-{inv}"), place],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed device")
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
                params![format!("exp_t_{behavior}"), "Тип теста", behavior],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("type by behavior")
}

async fn make_group(
    ctx: &AppCtx,
    behavior: &'static str,
    name: &str,
    place: Option<i32>,
) -> trackly_app::dto::groups::GroupDto {
    let type_id = type_by_behavior(ctx, behavior).await;
    ctx.groups
        .create_group(
            &admin(),
            GroupCreateDto {
                type_id,
                name: Some(name.to_string()),
                place_id: place,
                source_device_id: None,
            },
        )
        .await
        .expect("create_group")
}

/// Группа-якорь: якорное устройство входит в состав, затем ставится якорем.
async fn make_anchor_group(
    ctx: &AppCtx,
    behavior: &'static str,
    name: &str,
    anchor_name: &str,
    inv: &str,
) -> (i64, i64) {
    let g = make_group(ctx, behavior, name, None).await;
    let anchor = seed_device(ctx, anchor_name, inv, None).await;
    ctx.groups
        .add_devices(&admin(), g.id, vec![anchor])
        .await
        .expect("add anchor");
    let gid = g.id;
    ctx.writer
        .execute(move |conn| {
            let tx = conn.transaction().map_err(map_rusqlite)?;
            SqliteGroupRepository.set_anchor_in_tx(&tx, gid, anchor)?;
            tx.commit().map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("set anchor");
    (gid, anchor)
}

async fn nest(ctx: &AppCtx, child: i64, parent: i64) {
    let dto = ctx.groups.get_group(&admin(), child).await.expect("child");
    ctx.groups
        .set_parent(&admin(), child, dto.version, Some(parent))
        .await
        .expect("set_parent");
}

/// Фикстура пункта (a): контейнер «АРМ #3» с двумя прямыми устройствами, вложенным
/// контейнером (одно устройство) и вложенной якорной группой (якорь + две детали).
struct Fixture {
    root: i64,
    direct: Vec<i64>,
    nested_dev: i64,
    anchor: i64,
    parts: Vec<i64>,
}

async fn build_fixture(ctx: &AppCtx) -> Fixture {
    let root = make_group(ctx, "container", "АРМ #3", None).await;
    let d1 = seed_device(ctx, "Системный блок #4", "INV-001", None).await;
    let d2 = seed_device(ctx, "Монитор #2", "INV-002", None).await;
    ctx.groups
        .add_devices(&admin(), root.id, vec![d1, d2])
        .await
        .unwrap();

    let inner = make_group(ctx, "container", "Периферия #1", None).await;
    let n1 = seed_device(ctx, "Клавиатура #1", "INV-003", None).await;
    ctx.groups
        .add_devices(&admin(), inner.id, vec![n1])
        .await
        .unwrap();
    nest(ctx, inner.id, root.id).await;

    let (anchor_group, anchor) =
        make_anchor_group(ctx, "substitute", "Замена #5", "Ноутбук #5", "INV-004").await;
    let p1 = seed_device(ctx, "Адаптер #5", "INV-005", None).await;
    let p2 = seed_device(ctx, "Мышь #5", "INV-006", None).await;
    ctx.groups
        .add_devices(&admin(), anchor_group, vec![p1, p2])
        .await
        .unwrap();
    nest(ctx, anchor_group, root.id).await;

    Fixture {
        root: root.id,
        direct: vec![d1, d2],
        nested_dev: n1,
        anchor,
        parts: vec![p1, p2],
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn expansion_container_flattens_nested_and_collapses_anchor_group() {
    let (ctx, _dir) = make_test_ctx().await;
    let fx = build_fixture(&ctx).await;
    let out = ctx.groups.act_expansion(&admin(), fx.root).await.unwrap();

    assert_eq!(out.group_id, fx.root);
    assert_eq!(out.group_name, "АРМ #3");
    assert_eq!(out.type_behavior, "container");
    assert_eq!(out.anchor_device_id, None);

    let ids: Vec<i64> = out.units.iter().map(|u| u.device_id).collect();
    // позитивная проверка: прямые, вложенный и ОДИН якорь
    for d in &fx.direct {
        assert!(
            ids.contains(d),
            "прямое устройство {d} должно быть в юнитах"
        );
    }
    assert!(ids.contains(&fx.nested_dev));
    assert!(ids.contains(&fx.anchor));
    // негативная проверка: деталей якорной группы в списке нет
    for p in &fx.parts {
        assert!(
            !ids.contains(p),
            "деталь {p} якорной группы не должна быть в юнитах"
        );
    }
    // (g) счёт: физические 2 + 1 + (1 якорь + 2 детали) = 6; минус детали, плюс якорь уже внутри
    assert_eq!(out.units.len(), 4);

    // via_group_name: корень — None; вложенные — имя их группы
    let by_id = |id: i64| out.units.iter().find(|u| u.device_id == id).unwrap();
    assert_eq!(by_id(fx.direct[0]).via_group_name, None);
    assert_eq!(by_id(fx.direct[1]).via_group_name, None);
    assert_eq!(
        by_id(fx.nested_dev).via_group_name.as_deref(),
        Some("Периферия #1")
    );
    assert_eq!(
        by_id(fx.anchor).via_group_name.as_deref(),
        Some("Замена #5")
    );

    // имя, инв. и серийный номер подгружены
    let sys = by_id(fx.direct[0]);
    assert_eq!(sys.name, "Системный блок #4");
    assert_eq!(sys.inventory_number.as_deref(), Some("INV-001"));
    assert_eq!(sys.serial_number.as_deref(), Some("SN-INV-001"));

    // порядок внутри группы-источника: (имя, id) — «Монитор #2» раньше «Системный блок #4»
    let root_units: Vec<&str> = out
        .units
        .iter()
        .filter(|u| u.via_group_name.is_none())
        .map(|u| u.name.as_str())
        .collect();
    assert_eq!(root_units, vec!["Монитор #2", "Системный блок #4"]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn expansion_anchor_group_is_a_single_anchor_unit() {
    let (ctx, _dir) = make_test_ctx().await;
    let (g, anchor) =
        make_anchor_group(&ctx, "substitute", "Замена #7", "Ноутбук #7", "INV-100").await;
    let part = seed_device(&ctx, "Адаптер #7", "INV-101", None).await;
    ctx.groups
        .add_devices(&admin(), g, vec![part])
        .await
        .unwrap();

    let out = ctx.groups.act_expansion(&admin(), g).await.unwrap();
    assert_eq!(out.anchor_device_id, Some(anchor));
    assert_eq!(out.type_behavior, "substitute");
    assert_eq!(out.units.len(), 1);
    assert_eq!(out.units[0].device_id, anchor);
    assert_eq!(out.units[0].via_group_name, None);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn expansion_empty_group_gives_empty_list() {
    let (ctx, _dir) = make_test_ctx().await;
    let g = make_group(&ctx, "container", "Пустой АРМ #1", None).await;
    let out = ctx.groups.act_expansion(&admin(), g.id).await.unwrap();
    assert!(out.units.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn expansion_sleeping_group_expands_like_an_ordinary_one() {
    let (ctx, _dir) = make_test_ctx().await;
    let g = make_group(&ctx, "container", "АРМ #8", None).await;
    assert_eq!(g.place_id, None, "фикстура: группа без места");
    let d = seed_device(&ctx, "Монитор #8", "INV-200", None).await;
    ctx.groups
        .add_devices(&admin(), g.id, vec![d])
        .await
        .unwrap();
    let out = ctx.groups.act_expansion(&admin(), g.id).await.unwrap();
    assert_eq!(out.units.len(), 1);
    assert_eq!(out.units[0].device_id, d);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn expansion_missing_group_is_not_found() {
    let (ctx, _dir) = make_test_ctx().await;
    let err = ctx
        .groups
        .act_expansion(&admin(), 987_654)
        .await
        .unwrap_err();
    assert!(
        matches!(
            err,
            AppError::NotFound {
                entity: "group",
                id: 987_654
            }
        ),
        "got {err:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn expansion_soft_deleted_device_is_not_included() {
    let (ctx, _dir) = make_test_ctx().await;
    let g = make_group(&ctx, "container", "АРМ #9", None).await;
    let keep = seed_device(&ctx, "Монитор #9", "INV-300", None).await;
    let gone = seed_device(&ctx, "Принтер #9", "INV-301", None).await;
    ctx.groups
        .add_devices(&admin(), g.id, vec![keep, gone])
        .await
        .unwrap();
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "UPDATE devices SET deleted_at_utc = 1700000100 WHERE id = ?1",
                params![gone],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .unwrap();
    let out = ctx.groups.act_expansion(&admin(), g.id).await.unwrap();
    let ids: Vec<i64> = out.units.iter().map(|u| u.device_id).collect();
    assert_eq!(ids, vec![keep]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn expansion_search_hit_carries_short_place_path_and_physical_count() {
    let (ctx, _dir) = make_test_ctx().await;
    // Глубина пути больше двух уровней, чтобы короткий и полный путь различались.
    let l1 = seed_place(&ctx, "Корпус А", None).await;
    let l2 = seed_place(&ctx, "Этаж 2", Some(l1)).await;
    let l3 = seed_place(&ctx, "Блок Б", Some(l2)).await;
    let l4 = seed_place(&ctx, "Кабинет 214", Some(l3)).await;

    let with_place = make_group(&ctx, "container", "АРМ #20", Some(l4 as i32)).await;
    let d1 = seed_device(&ctx, "Монитор #20", "INV-400", Some(l4)).await;
    ctx.groups
        .add_devices(&admin(), with_place.id, vec![d1])
        .await
        .unwrap();
    let inner = make_group(&ctx, "container", "Периферия #20", None).await;
    let d2 = seed_device(&ctx, "Мышь #20", "INV-401", None).await;
    ctx.groups
        .add_devices(&admin(), inner.id, vec![d2])
        .await
        .unwrap();
    nest(&ctx, inner.id, with_place.id).await;
    let sleeping = make_group(&ctx, "container", "АРМ #21", None).await;

    let hits = ctx
        .groups
        .search(&admin(), "АРМ #2".to_string(), None)
        .await
        .unwrap();
    let hit = hits.iter().find(|h| h.id == with_place.id).expect("hit");
    let full = ctx
        .groups
        .get_group(&admin(), with_place.id)
        .await
        .unwrap()
        .place_path
        .expect("full path");
    let expected = {
        let conn = ctx.readers.acquire();
        compute_place_path_short_with_conn(&conn, Some(l4), Some(full.clone()))
    };
    assert!(expected.is_some());
    assert_eq!(hit.place_path_short, expected);
    assert_ne!(
        hit.place_path_short.as_deref(),
        Some(full.as_str()),
        "короткий путь обязан отличаться от полного"
    );
    // device_count физический, с вложенными группами: 1 прямое + 1 во вложенной
    assert_eq!(hit.device_count, 2);

    let sleeping_hit = hits
        .iter()
        .find(|h| h.id == sleeping.id)
        .expect("sleeping hit");
    assert_eq!(sleeping_hit.place_path_short, None);
}

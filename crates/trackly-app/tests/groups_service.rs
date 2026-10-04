//! Phase 41 Plan 08: `GroupService` — чтения (список, карточка, состав, поиск,
//! членство пачкой) и CRUD (нумерация, переименование, удаление).
//! Реальный `AppCtx` на временном каталоге; имена вымышленные.
//!
//! Префиксы тестов: `composition_`, `search_`, `rights_read_`, `numbering_`,
//! `crud_`, `delete_`, `rights_write_`.

use rusqlite::params;

use trackly_app::context::AppCtx;
use trackly_app::dto::auth::UserNew;
use trackly_core::auth::{Identity, Role};
use trackly_core::error::AppError;
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

async fn create_identity(ctx: &AppCtx, login: &str, full_name: &str, role: Role) -> Identity {
    let role_str = match role {
        Role::Admin => "admin",
        Role::Manager => "manager",
        Role::Employee => "employee",
    };
    let dto = ctx
        .auth
        .create_user(
            UserNew {
                login: login.to_string(),
                full_name: full_name.to_string(),
                password: "password123".to_string(),
                role: role_str.to_string(),
                email: None,
            },
            &Identity::trusted_admin(),
        )
        .await
        .expect("create test user");
    Identity {
        user_id: Some(dto.id),
        role,
    }
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
        .expect("scalar query")
}

async fn opt_i64(ctx: &AppCtx, sql: &'static str, p: Vec<rusqlite::types::Value>) -> Option<i64> {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(sql, rusqlite::params_from_iter(p), |r| {
                r.get::<_, Option<i64>>(0)
            })
            .map_err(map_rusqlite)
        })
        .await
        .expect("opt query")
}

async fn scalar_string(ctx: &AppCtx, sql: &'static str, p: Vec<rusqlite::types::Value>) -> String {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(sql, rusqlite::params_from_iter(p), |r| {
                r.get::<_, String>(0)
            })
            .map_err(map_rusqlite)
        })
        .await
        .expect("string query")
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

/// Сеет место (`kind` — токен вида места) с необязательным родителем.
async fn seed_place(ctx: &AppCtx, kind: &'static str, name: &str, parent: Option<i64>) -> i64 {
    let name = name.to_string();
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO places (kind, name, parent_id, is_storage, created_at_utc, \
                 updated_at_utc, version) VALUES (?1, ?2, ?3, 0, 1700000000, 1700000000, 1)",
                params![kind, name, parent],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed place")
}

/// Сеет живую группу напрямую (мимо сервиса).
async fn seed_group(
    ctx: &AppCtx,
    type_id: i64,
    name: &str,
    seq: i64,
    place_id: Option<i64>,
    parent: Option<i64>,
) -> i64 {
    let name = name.to_string();
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO groups (type_id, name, seq, place_id, parent_group_id, \
                 created_at_utc, updated_at_utc, version) \
                 VALUES (?1, ?2, ?3, ?4, ?5, 1700000000, 1700000000, 1)",
                params![type_id, name, seq, place_id, parent],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed group")
}

/// Сеет живое устройство и (при `group_id`) сразу кладёт его в группу.
async fn seed_device(
    ctx: &AppCtx,
    name: &str,
    inv: &str,
    serial: &str,
    place_id: Option<i64>,
    group_id: Option<i64>,
) -> i64 {
    let (name, inv, serial) = (name.to_string(), inv.to_string(), serial.to_string());
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO devices (type_id, name, inventory_number, serial_number, place_id, \
                 status_id, created_at_utc, updated_at_utc, version) \
                 VALUES (1, ?1, ?2, ?3, ?4, 1, 1700000000, 1700000000, 1)",
                params![name, inv, serial, place_id],
            )
            .map_err(map_rusqlite)?;
            let id = conn.last_insert_rowid();
            if let Some(g) = group_id {
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

// ---------------------------------------------------------------------------
// composition_
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn composition_lists_direct_devices_and_nested_groups_with_counts() {
    let (ctx, _dir) = make_test_ctx().await;
    let arm = type_id_by_code(&ctx, "workstation").await;
    let building = seed_place(&ctx, "building", "Корпус Альфа", None).await;
    let room = seed_place(&ctx, "room", "Кабинет 101", Some(building)).await;

    let outer = seed_group(&ctx, arm, "АРМ #1", 1, Some(room), None).await;
    let inner = seed_group(&ctx, arm, "АРМ #2", 2, Some(room), Some(outer)).await;
    for i in 0..4 {
        seed_device(
            &ctx,
            &format!("Монитор {i}"),
            &format!("INV-A{i}"),
            &format!("SN-A{i}"),
            Some(room),
            Some(outer),
        )
        .await;
    }
    for i in 0..2 {
        seed_device(
            &ctx,
            &format!("Блок {i}"),
            &format!("INV-B{i}"),
            &format!("SN-B{i}"),
            Some(room),
            Some(inner),
        )
        .await;
    }

    let comp = ctx.groups.composition(&admin(), outer).await.unwrap();
    assert_eq!(comp.devices.len(), 4, "только прямые устройства");
    assert_eq!(comp.child_groups.len(), 1);
    assert_eq!(comp.child_groups[0].id, inner);
    assert_eq!(comp.child_groups[0].device_count, 2);
    assert_eq!(comp.child_groups[0].root_group_id, outer);
    assert_eq!(comp.child_groups[0].root_group_name, "АРМ #1");
    assert_eq!(comp.child_groups[0].parent_group_id, Some(outer));

    // Строка устройства несёт столбцы PlaceContents; ожидаемые значения читаются из БД.
    let d = comp.devices.iter().find(|d| d.name == "Монитор 2").unwrap();
    assert_eq!(d.inventory_number.as_deref(), Some("INV-A2"));
    assert_eq!(d.serial_number.as_deref(), Some("SN-A2"));
    assert_eq!(d.place_id, Some(room));
    let want_type = scalar_string(&ctx, "SELECT name FROM device_types WHERE id = 1", vec![]).await;
    assert_eq!(d.type_name, want_type);
    let want_path = scalar_string(
        &ctx,
        "SELECT full_path FROM place_full_paths WHERE place_id = ?1",
        vec![int(room)],
    )
    .await;
    assert_eq!(d.place_path.as_deref(), Some(want_path.as_str()));
    assert!(
        d.place_path_short.is_some(),
        "сокращённый путь считает сервер"
    );
    let want_status = scalar_string(
        &ctx,
        "SELECT name FROM device_statuses WHERE id = 1",
        vec![],
    )
    .await;
    assert_eq!(d.status_name.as_deref(), Some(want_status.as_str()));

    // Раскрытие вложенной строки грузит её состав тем же вызовом.
    let inner_comp = ctx.groups.composition(&admin(), inner).await.unwrap();
    assert_eq!(inner_comp.devices.len(), 2);
    assert!(inner_comp.child_groups.is_empty());

    // Счётчики карточки внешней группы: 6 всего, 4 прямых, 1 вложенная.
    let g = ctx.groups.get_group(&admin(), outer).await.unwrap();
    assert_eq!(g.device_count, 6);
    assert_eq!(g.direct_device_count, 4);
    assert_eq!(g.nested_group_count, 1);
    assert_eq!(g.type_code, "workstation");
    assert_eq!(g.root_group_id, outer);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn composition_empty_group_and_placeless_group() {
    let (ctx, _dir) = make_test_ctx().await;
    let arm = type_id_by_code(&ctx, "workstation").await;
    let g = seed_group(&ctx, arm, "АРМ #1", 1, None, None).await;

    let comp = ctx.groups.composition(&admin(), g).await.unwrap();
    assert!(comp.devices.is_empty());
    assert!(comp.child_groups.is_empty());

    // D-21: группа без места — place_id и place_path пусты.
    let dto = ctx.groups.get_group(&admin(), g).await.unwrap();
    assert_eq!(dto.place_id, None);
    assert_eq!(dto.place_path, None);

    let all = ctx.groups.list_groups(&admin()).await.unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].id, g);

    assert!(matches!(
        ctx.groups.get_group(&admin(), g + 1000).await,
        Err(AppError::NotFound { .. })
    ));
    assert!(matches!(
        ctx.groups.composition(&admin(), g + 1000).await,
        Err(AppError::NotFound { .. })
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn composition_deep_nested_group_points_to_root() {
    let (ctx, _dir) = make_test_ctx().await;
    let arm = type_id_by_code(&ctx, "workstation").await;
    let root = seed_group(&ctx, arm, "Корень", 1, None, None).await;
    let mid = seed_group(&ctx, arm, "Середина", 2, None, Some(root)).await;
    let leaf = seed_group(&ctx, arm, "Лист", 3, None, Some(mid)).await;
    let dto = ctx.groups.get_group(&admin(), leaf).await.unwrap();
    assert_eq!(dto.root_group_id, root);
    assert_eq!(dto.root_group_name, "Корень");
    assert_eq!(dto.parent_group_id, Some(mid));
}

// ---------------------------------------------------------------------------
// search_
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn search_is_case_insensitive_for_cyrillic_and_excludes_subtree() {
    let (ctx, _dir) = make_test_ctx().await;
    let arm = type_id_by_code(&ctx, "workstation").await;
    let me = seed_group(&ctx, arm, "АРМ #1", 1, None, None).await;
    let child = seed_group(&ctx, arm, "АРМ #2", 2, None, Some(me)).await;
    let grandchild = seed_group(&ctx, arm, "АРМ #5", 5, None, Some(child)).await;
    let other = seed_group(&ctx, arm, "АРМ #3", 3, None, None).await;
    let _unrelated = seed_group(&ctx, arm, "Кабинет 214", 4, None, None).await;

    let hits = ctx
        .groups
        .search(&admin(), "арм".to_string(), Some(me))
        .await
        .unwrap();
    let ids: Vec<i64> = hits.iter().map(|h| h.id).collect();
    assert_eq!(
        ids,
        vec![other],
        "найден «АРМ #3»; сама группа и потомки исключены"
    );
    assert!(!ids.contains(&me) && !ids.contains(&child) && !ids.contains(&grandchild));
    assert_eq!(hits[0].name, "АРМ #3");
    assert!(!hits[0].has_parent);

    // Без исключения находятся все четыре «АРМ…».
    let all = ctx
        .groups
        .search(&admin(), "аРМ".to_string(), None)
        .await
        .unwrap();
    assert_eq!(all.len(), 4);
    assert!(all.iter().any(|h| h.id == child && h.has_parent));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn search_caps_results_and_rejects_overlong_query() {
    let (ctx, _dir) = make_test_ctx().await;
    let arm = type_id_by_code(&ctx, "workstation").await;
    for i in 1..=55 {
        seed_group(&ctx, arm, &format!("АРМ #{i}"), i, None, None).await;
    }
    let hits = ctx
        .groups
        .search(&admin(), String::new(), None)
        .await
        .unwrap();
    assert_eq!(hits.len(), 50);

    let long = "а".repeat(101);
    assert!(matches!(
        ctx.groups.search(&admin(), long, None).await,
        Err(AppError::Validation { .. })
    ));
}

// ---------------------------------------------------------------------------
// for_devices
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn search_for_devices_returns_members_only_with_place_flag() {
    let (ctx, _dir) = make_test_ctx().await;
    let arm = type_id_by_code(&ctx, "workstation").await;
    let room = seed_place(&ctx, "room", "Кабинет 7", None).await;
    let placed = seed_group(&ctx, arm, "АРМ #1", 1, Some(room), None).await;
    let placeless = seed_group(&ctx, arm, "АРМ #2", 2, None, None).await;
    let d1 = seed_device(
        &ctx,
        "Устройство 1",
        "INV-1",
        "SN-1",
        Some(room),
        Some(placed),
    )
    .await;
    let d2 = seed_device(&ctx, "Устройство 2", "INV-2", "SN-2", None, Some(placeless)).await;
    let d3 = seed_device(&ctx, "Устройство 3", "INV-3", "SN-3", Some(room), None).await;

    let got = ctx
        .groups
        .for_devices(&admin(), vec![d1, d2, d3])
        .await
        .unwrap();
    assert_eq!(got.len(), 2, "устройство вне групп в выдачу не попадает");
    let m1 = got.iter().find(|m| m.device_id == d1).unwrap();
    assert_eq!(
        (m1.group_id, m1.group_name.as_str(), m1.group_has_place),
        (placed, "АРМ #1", true)
    );
    let m2 = got.iter().find(|m| m.device_id == d2).unwrap();
    assert_eq!((m2.group_id, m2.group_has_place), (placeless, false));

    assert!(ctx
        .groups
        .for_devices(&admin(), vec![])
        .await
        .unwrap()
        .is_empty());

    let too_many: Vec<i64> = (1..=501).collect();
    assert!(matches!(
        ctx.groups.for_devices(&admin(), too_many).await,
        Err(AppError::Validation { .. })
    ));
    let at_limit: Vec<i64> = (1..=500).collect();
    assert!(ctx.groups.for_devices(&admin(), at_limit).await.is_ok());
}

// ---------------------------------------------------------------------------
// rights_read_
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rights_read_employee_forbidden_manager_allowed() {
    let (ctx, _dir) = make_test_ctx().await;
    let arm = type_id_by_code(&ctx, "workstation").await;
    let g = seed_group(&ctx, arm, "АРМ #1", 1, None, None).await;
    let employee = create_identity(&ctx, "emp_one", "Иванов И.И.", Role::Employee).await;
    let manager = create_identity(&ctx, "mgr_one", "Петров П.П.", Role::Manager).await;

    assert!(matches!(
        ctx.groups.list_groups(&employee).await,
        Err(AppError::Forbidden)
    ));
    assert!(matches!(
        ctx.groups.get_group(&employee, g).await,
        Err(AppError::Forbidden)
    ));
    assert!(matches!(
        ctx.groups.composition(&employee, g).await,
        Err(AppError::Forbidden)
    ));
    assert!(matches!(
        ctx.groups.search(&employee, String::new(), None).await,
        Err(AppError::Forbidden)
    ));
    assert!(matches!(
        ctx.groups.for_devices(&employee, vec![1]).await,
        Err(AppError::Forbidden)
    ));

    assert_eq!(ctx.groups.list_groups(&manager).await.unwrap().len(), 1);
    assert!(ctx.groups.get_group(&manager, g).await.is_ok());
    let _ = (opt_i64, scalar_i64); // используются тестами записи
}

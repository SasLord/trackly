//! Phase 41 Plan 08: `GroupService` — чтения (список, карточка, состав, поиск,
//! членство пачкой) и CRUD (нумерация, переименование, удаление).
//! Реальный `AppCtx` на временном каталоге; имена вымышленные.
//!
//! Префиксы тестов: `composition_`, `search_`, `rights_read_`, `numbering_`,
//! `crud_`, `delete_`, `rights_write_`.

use rusqlite::params;

use trackly_app::context::AppCtx;
use trackly_app::dto::auth::UserNew;
use trackly_app::dto::groups::GroupCreateDto;
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
}

// ---------------------------------------------------------------------------
// helpers для записи
// ---------------------------------------------------------------------------

fn create_dto(type_id: i64, name: Option<&str>, place_id: Option<i32>) -> GroupCreateDto {
    GroupCreateDto {
        type_id,
        name: name.map(str::to_string),
        place_id,
    }
}

async fn exec(ctx: &AppCtx, sql: &'static str, p: Vec<rusqlite::types::Value>) {
    ctx.writer
        .execute(move |conn| {
            conn.execute(sql, rusqlite::params_from_iter(p))
                .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("exec");
}

// ---------------------------------------------------------------------------
// numbering_
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn numbering_default_names_follow_seq_and_rename_does_not_shift_counter() {
    let (ctx, _dir) = make_test_ctx().await;
    let arm = type_id_by_code(&ctx, "workstation").await;
    let type_name = scalar_string(
        &ctx,
        "SELECT name FROM group_types WHERE id = ?1",
        vec![int(arm)],
    )
    .await;

    let first = ctx
        .groups
        .create_group(&admin(), create_dto(arm, None, None))
        .await
        .unwrap();
    let second = ctx
        .groups
        .create_group(&admin(), create_dto(arm, None, None))
        .await
        .unwrap();
    assert_eq!((first.seq, second.seq), (1, 2));
    assert_eq!(first.name, format!("{type_name} #1"));
    assert_eq!(second.name, format!("{type_name} #2"));

    // Переименование первой в «АРМ #99» не сдвигает счётчик: имя не разбирается.
    let renamed = ctx
        .groups
        .update_group(
            &admin(),
            first.id,
            first.version,
            format!("{type_name} #99"),
        )
        .await
        .unwrap();
    assert_eq!(renamed.seq, 1, "переименование не трогает seq");

    let third = ctx
        .groups
        .create_group(&admin(), create_dto(arm, None, None))
        .await
        .unwrap();
    assert_eq!(third.seq, 3);
    assert_eq!(third.name, format!("{type_name} #3"));
    // seq второй группы остался прежним — читаем из БД.
    assert_eq!(
        scalar_i64(
            &ctx,
            "SELECT seq FROM groups WHERE id = ?1",
            vec![int(second.id)]
        )
        .await,
        2
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn numbering_is_per_type_and_duplicate_names_are_allowed() {
    let (ctx, _dir) = make_test_ctx().await;
    let arm = type_id_by_code(&ctx, "workstation").await;
    let unit = type_id_by_code(&ctx, "system_unit").await;

    let a = ctx
        .groups
        .create_group(&admin(), create_dto(arm, Some("Кабинет 214"), None))
        .await
        .unwrap();
    let b = ctx
        .groups
        .create_group(&admin(), create_dto(arm, Some("Кабинет 214"), None))
        .await
        .unwrap();
    assert_eq!(a.name, b.name);
    assert_ne!(a.id, b.id);
    assert_eq!((a.seq, b.seq), (1, 2), "явное имя не отменяет выдачу seq");
    assert_eq!(
        scalar_i64(
            &ctx,
            "SELECT COUNT(*) FROM groups WHERE name = ?1",
            vec![rusqlite::types::Value::Text("Кабинет 214".to_string())]
        )
        .await,
        2
    );

    // Счётчик у другого типа независим.
    let u = ctx
        .groups
        .create_group(&admin(), create_dto(unit, None, None))
        .await
        .unwrap();
    assert_eq!(u.seq, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn numbering_concurrent_creates_get_distinct_seq() {
    let (ctx, _dir) = make_test_ctx().await;
    let arm = type_id_by_code(&ctx, "workstation").await;
    let (c1, c2) = (ctx.clone(), ctx.clone());
    let (r1, r2) = tokio::join!(
        async move {
            c1.groups
                .create_group(&admin(), create_dto(arm, None, None))
                .await
        },
        async move {
            c2.groups
                .create_group(&admin(), create_dto(arm, None, None))
                .await
        },
    );
    let (g1, g2) = (r1.expect("create 1"), r2.expect("create 2"));
    let mut seqs = vec![g1.seq, g2.seq];
    seqs.sort();
    assert_eq!(seqs, vec![1, 2]);
    assert_ne!(g1.name, g2.name);
}

// ---------------------------------------------------------------------------
// crud_
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn crud_explicit_name_is_trimmed_and_validated() {
    let (ctx, _dir) = make_test_ctx().await;
    let arm = type_id_by_code(&ctx, "workstation").await;

    let g = ctx
        .groups
        .create_group(&admin(), create_dto(arm, Some("  Кабинет 214  "), None))
        .await
        .unwrap();
    assert_eq!(g.name, "Кабинет 214");
    assert_eq!(g.seq, 1);

    for bad in ["", "   "] {
        match ctx
            .groups
            .create_group(&admin(), create_dto(arm, Some(bad), None))
            .await
        {
            Err(AppError::Validation { message, .. }) => assert_eq!(message, "Укажите название."),
            other => panic!("ожидали Validation, получили {other:?}"),
        }
    }
    let long = "а".repeat(201);
    assert!(matches!(
        ctx.groups
            .create_group(&admin(), create_dto(arm, Some(&long), None))
            .await,
        Err(AppError::Validation { .. })
    ));
    assert!(matches!(
        ctx.groups
            .create_group(&admin(), create_dto(arm + 1000, None, None))
            .await,
        Err(AppError::NotFound { .. })
    ));
    // Отказы не оставили следов: seq следующей группы не «сгорел».
    let next = ctx
        .groups
        .create_group(&admin(), create_dto(arm, None, None))
        .await
        .unwrap();
    assert_eq!(next.seq, 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn crud_place_is_optional_checked_and_writes_no_movements() {
    let (ctx, _dir) = make_test_ctx().await;
    let arm = type_id_by_code(&ctx, "workstation").await;
    let room = seed_place(&ctx, "room", "Кабинет 12", None).await;
    let archived = seed_place(&ctx, "room", "Кабинет в архиве", None).await;
    exec(
        &ctx,
        "UPDATE places SET archived_at_utc = 1700000001 WHERE id = ?1",
        vec![int(archived)],
    )
    .await;
    let movements_before = scalar_i64(&ctx, "SELECT COUNT(*) FROM place_movements", vec![]).await;

    let placeless = ctx
        .groups
        .create_group(&admin(), create_dto(arm, None, None))
        .await
        .unwrap();
    assert_eq!(placeless.place_id, None);
    assert_eq!(placeless.place_path, None);

    let placed = ctx
        .groups
        .create_group(&admin(), create_dto(arm, None, Some(room as i32)))
        .await
        .unwrap();
    assert_eq!(placed.place_id, Some(room));
    let want_path = scalar_string(
        &ctx,
        "SELECT full_path FROM place_full_paths WHERE place_id = ?1",
        vec![int(room)],
    )
    .await;
    assert_eq!(placed.place_path.as_deref(), Some(want_path.as_str()));

    for bad in [room as i32 + 9999, archived as i32] {
        assert!(
            matches!(
                ctx.groups.create_group(&admin(), create_dto(arm, None, Some(bad))).await,
                Err(AppError::Validation { ref field, .. }) if field == "place_id"
            ),
            "место {bad} должно быть отклонено"
        );
    }
    assert_eq!(
        scalar_i64(&ctx, "SELECT COUNT(*) FROM place_movements", vec![]).await,
        movements_before,
        "создание группы не пишет place_movements"
    );
    assert_eq!(
        scalar_i64(&ctx, "SELECT COUNT(*) FROM groups", vec![]).await,
        2,
        "отклонённые создания не оставили групп"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn crud_rename_uses_cas_and_audits() {
    let (ctx, _dir) = make_test_ctx().await;
    let arm = type_id_by_code(&ctx, "workstation").await;
    let g = ctx
        .groups
        .create_group(&admin(), create_dto(arm, None, None))
        .await
        .unwrap();

    let renamed = ctx
        .groups
        .update_group(
            &admin(),
            g.id,
            g.version,
            "  Рабочее место Иванова И.И.  ".to_string(),
        )
        .await
        .unwrap();
    assert_eq!(renamed.name, "Рабочее место Иванова И.И.");
    assert_eq!(renamed.version, g.version + 1);

    // Устаревшая версия отклоняется, имя в БД не меняется.
    match ctx
        .groups
        .update_group(&admin(), g.id, g.version, "Другое имя".to_string())
        .await
    {
        Err(AppError::OptimisticLockMismatch {
            expected, actual, ..
        }) => {
            assert_eq!(expected, g.version);
            assert_eq!(actual, renamed.version);
        }
        other => panic!("ожидали OptimisticLockMismatch, получили {other:?}"),
    }
    assert_eq!(
        scalar_string(
            &ctx,
            "SELECT name FROM groups WHERE id = ?1",
            vec![int(g.id)]
        )
        .await,
        "Рабочее место Иванова И.И."
    );
    assert!(matches!(
        ctx.groups
            .update_group(&admin(), g.id, renamed.version, "   ".to_string())
            .await,
        Err(AppError::Validation { .. })
    ));
    assert!(matches!(
        ctx.groups
            .update_group(&admin(), g.id + 1000, 1, "Имя".to_string())
            .await,
        Err(AppError::NotFound { .. })
    ));
    assert_eq!(
        scalar_i64(
            &ctx,
            "SELECT COUNT(*) FROM audit_log WHERE entity_type = 'group' AND entity_id = ?1",
            vec![int(g.id)]
        )
        .await,
        2,
        "create + update"
    );
}

// ---------------------------------------------------------------------------
// delete_
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_releases_devices_keeps_places_and_unnests_children() {
    let (ctx, _dir) = make_test_ctx().await;
    let arm = type_id_by_code(&ctx, "workstation").await;
    let room = seed_place(&ctx, "room", "Кабинет 3", None).await;
    let other_room = seed_place(&ctx, "room", "Кабинет 4", None).await;

    let outer = seed_group(&ctx, arm, "АРМ #1", 1, Some(room), None).await;
    let inner = seed_group(&ctx, arm, "АРМ #2", 2, Some(room), Some(outer)).await;
    let mut direct = Vec::new();
    for i in 0..3 {
        direct.push(
            seed_device(
                &ctx,
                &format!("Прямое {i}"),
                &format!("INV-D{i}"),
                &format!("SN-D{i}"),
                Some(if i == 0 { other_room } else { room }),
                Some(outer),
            )
            .await,
        );
    }
    let nested_device =
        seed_device(&ctx, "Вложенное", "INV-N", "SN-N", Some(room), Some(inner)).await;

    // devices.place_id до удаления — читаем из БД.
    let mut before = Vec::new();
    for d in &direct {
        before.push(
            opt_i64(
                &ctx,
                "SELECT place_id FROM devices WHERE id = ?1",
                vec![int(*d)],
            )
            .await,
        );
    }

    let res = ctx.groups.delete_group(&admin(), outer).await.unwrap();
    assert_eq!(
        res.released_devices, 3,
        "только прямые устройства удаляемой группы"
    );

    for (d, was) in direct.iter().zip(&before) {
        let after = opt_i64(
            &ctx,
            "SELECT place_id FROM devices WHERE id = ?1",
            vec![int(*d)],
        )
        .await;
        assert_eq!(&after, was, "место устройства не меняется");
        assert_eq!(
            scalar_i64(
                &ctx,
                "SELECT COUNT(*) FROM group_devices WHERE device_id = ?1",
                vec![int(*d)]
            )
            .await,
            0,
            "членство исчезло"
        );
    }
    // Вложенная группа стала корневой с тем же местом и сохранила свой состав.
    assert_eq!(
        opt_i64(
            &ctx,
            "SELECT parent_group_id FROM groups WHERE id = ?1",
            vec![int(inner)]
        )
        .await,
        None
    );
    assert_eq!(
        opt_i64(
            &ctx,
            "SELECT place_id FROM groups WHERE id = ?1",
            vec![int(inner)]
        )
        .await,
        Some(room)
    );
    assert_eq!(
        scalar_i64(
            &ctx,
            "SELECT group_id FROM group_devices WHERE device_id = ?1",
            vec![int(nested_device)]
        )
        .await,
        inner
    );
    assert_eq!(
        scalar_i64(
            &ctx,
            "SELECT COUNT(*) FROM groups WHERE id = ?1",
            vec![int(outer)]
        )
        .await,
        0
    );
    // Аудит удаления с payload.
    let payload = scalar_string(
        &ctx,
        "SELECT payload_json FROM audit_log WHERE entity_type = 'group' AND entity_id = ?1 \
         AND action = 'delete'",
        vec![int(outer)],
    )
    .await;
    assert!(
        payload.contains("\"released_devices\":3"),
        "payload: {payload}"
    );
    assert!(
        payload.contains("\"nested_groups\":1"),
        "payload: {payload}"
    );

    assert!(matches!(
        ctx.groups.delete_group(&admin(), outer).await,
        Err(AppError::NotFound { .. })
    ));
}

// ---------------------------------------------------------------------------
// rights_write_
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rights_write_employee_forbidden_manager_allowed() {
    let (ctx, _dir) = make_test_ctx().await;
    let arm = type_id_by_code(&ctx, "workstation").await;
    let g = seed_group(&ctx, arm, "АРМ #1", 1, None, None).await;
    let employee = create_identity(&ctx, "emp_two", "Иванов И.И.", Role::Employee).await;
    let manager = create_identity(&ctx, "mgr_two", "Петров П.П.", Role::Manager).await;

    assert!(matches!(
        ctx.groups
            .create_group(&employee, create_dto(arm, None, None))
            .await,
        Err(AppError::Forbidden)
    ));
    assert!(matches!(
        ctx.groups
            .update_group(&employee, g, 1, "Имя".to_string())
            .await,
        Err(AppError::Forbidden)
    ));
    assert!(matches!(
        ctx.groups.delete_group(&employee, g).await,
        Err(AppError::Forbidden)
    ));
    assert_eq!(
        scalar_i64(&ctx, "SELECT COUNT(*) FROM groups", vec![]).await,
        1,
        "отказ по правам ничего не изменил"
    );

    let made = ctx
        .groups
        .create_group(&manager, create_dto(arm, None, None))
        .await
        .unwrap();
    assert!(ctx
        .groups
        .update_group(&manager, made.id, made.version, "Имя".to_string())
        .await
        .is_ok());
    assert!(ctx.groups.delete_group(&manager, made.id).await.is_ok());
}

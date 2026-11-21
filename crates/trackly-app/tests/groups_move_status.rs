//! Фаза 41.8 план 30 (D-34): перенос группы со сменой статуса и серверный предпросмотр
//! (`GroupService::move_group_with_status`, `move_group_status_preview`). Реальный
//! `AppCtx` на временном каталоге; имена вымышленные.
//!
//! Префикс тестов: `move_status_`.

use rusqlite::params;

use trackly_app::context::AppCtx;
use trackly_app::dto::auth::UserNew;
use trackly_app::dto::groups::{GroupMoveStatusPreviewDto, GroupMoveStatusTransitionDto};
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

fn admin() -> Identity {
    Identity::trusted_admin()
}

async fn employee(ctx: &AppCtx) -> Identity {
    let dto = ctx
        .auth
        .create_user(
            UserNew {
                login: "petrov".to_string(),
                full_name: "Петров П.П.".to_string(),
                password: "password123".to_string(),
                role: "employee".to_string(),
                email: None,
            },
            &Identity::trusted_admin(),
        )
        .await
        .expect("create user");
    Identity {
        user_id: Some(dto.id),
        role: Role::Employee,
    }
}

async fn sql_exec(ctx: &AppCtx, sql: &'static str, p: Vec<rusqlite::types::Value>) {
    ctx.writer
        .execute(move |conn| {
            conn.execute(sql, rusqlite::params_from_iter(p))
                .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("sql exec");
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

async fn group_place(ctx: &AppCtx, id: i64) -> Option<i64> {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(
                "SELECT place_id FROM groups WHERE id = ?1",
                params![id],
                |r| r.get::<_, Option<i64>>(0),
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("group place")
}

async fn audit_status_changed(ctx: &AppCtx, device_id: i64) -> i64 {
    scalar_i64(
        ctx,
        "SELECT COUNT(*) FROM audit_log WHERE action = 'custom:group_move_status_changed' \
         AND entity_id = ?1",
        vec![int(device_id)],
    )
    .await
}

/// Фикстура D-34: «Склад А» (склад), «Полка 1» в нём, «Кабинет 12/14», «Сервис-центр».
struct Fx {
    store: i64,
    shelf: i64,
    room12: i64,
    room14: i64,
    service: i64,
    arm: i64,
    /// «На складе» x2 (+ «На списание», «Списано», «Ожидание ремонта» на месте группы,
    /// «В ремонте» в сервисе, «Кабель» во вложенной группе).
    monitor: i64,
    mouse: i64,
    kb_to_write_off: i64,
    scanner_written_off: i64,
    laptop_awaiting: i64,
    laptop_in_repair: i64,
    cable_nested: i64,
}

async fn seed_fx(ctx: &AppCtx) -> Fx {
    let store = seed_place(ctx, "Склад А", None, true).await;
    let shelf = seed_place(ctx, "Полка 1", Some(store), false).await;
    let room12 = seed_place(ctx, "Кабинет 12", None, false).await;
    let room14 = seed_place(ctx, "Кабинет 14", None, false).await;
    let service = seed_place(ctx, "Сервис-центр", None, false).await;
    let arm = seed_group(ctx, "АРМ #9", 9, Some(store), None).await;
    let nested = seed_group(ctx, "Кабели #5", 5, Some(store), Some(arm)).await;
    let monitor = seed_device(
        ctx,
        "Монитор #20",
        "INV-20",
        Some(store),
        Some(arm),
        "на_складе",
    )
    .await;
    let mouse = seed_device(
        ctx,
        "Мышь #21",
        "INV-21",
        Some(store),
        Some(arm),
        "на_складе",
    )
    .await;
    let kb = seed_device(
        ctx,
        "Клавиатура #22",
        "INV-22",
        Some(store),
        Some(arm),
        "на_списание",
    )
    .await;
    let scanner = seed_device(
        ctx,
        "Сканер #23",
        "INV-23",
        Some(store),
        Some(arm),
        "списано",
    )
    .await;
    let l24 = seed_device(
        ctx,
        "Ноутбук #24",
        "INV-24",
        Some(store),
        Some(arm),
        "на_ремонте",
    )
    .await;
    let l25 = seed_device(
        ctx,
        "Ноутбук #25",
        "INV-25",
        Some(service),
        Some(arm),
        "в_ремонте",
    )
    .await;
    let cable = seed_device(
        ctx,
        "Кабель #26",
        "INV-26",
        Some(store),
        Some(nested),
        "на_складе",
    )
    .await;
    Fx {
        store,
        shelf,
        room12,
        room14,
        service,
        arm,
        monitor,
        mouse,
        kb_to_write_off: kb,
        scanner_written_off: scanner,
        laptop_awaiting: l24,
        laptop_in_repair: l25,
        cable_nested: cable,
    }
}

fn expect_validation(err: AppError) {
    assert!(matches!(err, AppError::Validation { .. }), "{err:?}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn move_status_to_work_applies_d34_table_and_preview_equals_actual() {
    let (ctx, _d) = make_test_ctx().await;
    let f = seed_fx(&ctx).await;
    // Исходное состояние утверждается до вызова (проверка не вакуумна).
    assert_eq!(
        dev_state(&ctx, f.monitor).await,
        (Some(f.store), "на_складе".into())
    );
    assert_eq!(
        dev_state(&ctx, f.laptop_in_repair).await,
        (Some(f.service), "в_ремонте".into())
    );

    let preview = ctx
        .groups
        .move_group_status_preview(&admin(), f.arm, f.room12)
        .await
        .expect("preview");
    assert_eq!(
        preview.transition,
        Some(GroupMoveStatusTransitionDto::ToWork)
    );
    assert_eq!(preview.devices_affected, 3);

    let version = group_version(&ctx, f.arm).await;
    let result = ctx
        .groups
        .move_group_with_status(&admin(), f.arm, version, f.room12, true)
        .await
        .expect("move");

    // Свойство: preview == фактическое.
    assert_eq!(preview.devices_affected, result.status_changed_devices);
    assert_eq!(result.status_changed_devices, 3);
    assert!(
        result.summary.contains("статус сменён у 3 устройств"),
        "{}",
        result.summary
    );

    for id in [f.monitor, f.mouse, f.cable_nested] {
        assert_eq!(
            dev_state(&ctx, id).await,
            (Some(f.room12), "в_работе".into()),
            "{id}"
        );
        assert_eq!(audit_status_changed(&ctx, id).await, 1, "аудит {id}");
    }
    assert_eq!(
        dev_state(&ctx, f.kb_to_write_off).await,
        (Some(f.room12), "на_списание".into())
    );
    assert_eq!(
        dev_state(&ctx, f.scanner_written_off).await,
        (Some(f.room12), "списано".into())
    );
    assert_eq!(
        dev_state(&ctx, f.laptop_awaiting).await,
        (Some(f.room12), "на_ремонте".into())
    );
    assert_eq!(
        dev_state(&ctx, f.laptop_in_repair).await,
        (Some(f.service), "в_ремонте".into())
    );
    for id in [
        f.kb_to_write_off,
        f.scanner_written_off,
        f.laptop_awaiting,
        f.laptop_in_repair,
    ] {
        assert_eq!(audit_status_changed(&ctx, id).await, 0, "аудита нет у {id}");
    }
    assert_eq!(group_place(&ctx, f.arm).await, Some(f.room12));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn move_status_flag_off_changes_no_status() {
    let (ctx, _d) = make_test_ctx().await;
    let f = seed_fx(&ctx).await;
    assert_eq!(dev_state(&ctx, f.monitor).await.1, "на_складе");
    let version = group_version(&ctx, f.arm).await;
    let result = ctx
        .groups
        .move_group_with_status(&admin(), f.arm, version, f.room12, false)
        .await
        .expect("move");
    assert_eq!(result.status_changed_devices, 0);
    assert_eq!(
        result.summary,
        format!("группа и {} устройств", result.moved_devices)
    );
    for id in [f.monitor, f.mouse, f.cable_nested] {
        assert_eq!(
            dev_state(&ctx, id).await,
            (Some(f.room12), "на_складе".into())
        );
        assert_eq!(audit_status_changed(&ctx, id).await, 0);
    }
    // Прежняя арность move_group - то же поведение.
    let version = group_version(&ctx, f.arm).await;
    let again = ctx
        .groups
        .move_group(&admin(), f.arm, version, f.room14)
        .await
        .expect("move_group");
    assert_eq!(again.status_changed_devices, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn move_status_no_storage_transition_keeps_statuses() {
    let (ctx, _d) = make_test_ctx().await;
    let f = seed_fx(&ctx).await;
    // Сначала группа попадает в кабинет без смены статусов, затем кабинет -> кабинет.
    let v = group_version(&ctx, f.arm).await;
    ctx.groups
        .move_group_with_status(&admin(), f.arm, v, f.room12, false)
        .await
        .expect("prep");
    assert_eq!(
        dev_state(&ctx, f.monitor).await,
        (Some(f.room12), "на_складе".into())
    );
    let preview = ctx
        .groups
        .move_group_status_preview(&admin(), f.arm, f.room14)
        .await
        .expect("preview");
    assert_eq!(preview.transition, None);
    assert_eq!(preview.devices_affected, 0);
    assert_eq!(preview.target_status_code, None);
    assert_eq!(preview.target_status_name, None);
    let v = group_version(&ctx, f.arm).await;
    let result = ctx
        .groups
        .move_group_with_status(&admin(), f.arm, v, f.room14, true)
        .await
        .expect("move");
    assert_eq!(result.status_changed_devices, 0);
    assert_eq!(
        dev_state(&ctx, f.monitor).await,
        (Some(f.room14), "на_складе".into())
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn move_status_to_storage_shifts_in_use_only() {
    let (ctx, _d) = make_test_ctx().await;
    let f = seed_fx(&ctx).await;
    // Группа в кабинете; устройства «В работе», «На складе» и «На списание» там же.
    let v = group_version(&ctx, f.arm).await;
    ctx.groups
        .move_group_with_status(&admin(), f.arm, v, f.room14, false)
        .await
        .expect("prep");
    sql_exec(
        &ctx,
        "UPDATE devices SET status_id = (SELECT id FROM device_statuses WHERE code = 'в_работе') \
         WHERE id IN (?1, ?2)",
        vec![int(f.monitor), int(f.mouse)],
    )
    .await;
    assert_eq!(
        dev_state(&ctx, f.monitor).await,
        (Some(f.room14), "в_работе".into())
    );
    assert_eq!(
        dev_state(&ctx, f.cable_nested).await,
        (Some(f.room14), "на_складе".into())
    );

    let preview = ctx
        .groups
        .move_group_status_preview(&admin(), f.arm, f.store)
        .await
        .expect("preview");
    assert_eq!(
        preview.transition,
        Some(GroupMoveStatusTransitionDto::ToStorage)
    );
    assert_eq!(preview.devices_affected, 2);
    assert_eq!(preview.target_status_code.as_deref(), Some("на_складе"));

    let v = group_version(&ctx, f.arm).await;
    let result = ctx
        .groups
        .move_group_with_status(&admin(), f.arm, v, f.store, true)
        .await
        .expect("move");
    assert_eq!(result.status_changed_devices, preview.devices_affected);
    for id in [f.monitor, f.mouse] {
        assert_eq!(
            dev_state(&ctx, id).await,
            (Some(f.store), "на_складе".into())
        );
        assert_eq!(audit_status_changed(&ctx, id).await, 1);
    }
    assert_eq!(
        dev_state(&ctx, f.cable_nested).await,
        (Some(f.store), "на_складе".into())
    );
    assert_eq!(audit_status_changed(&ctx, f.cable_nested).await, 0);
    assert_eq!(dev_state(&ctx, f.kb_to_write_off).await.1, "на_списание");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn move_status_storage_through_ancestor_counts_as_transition() {
    let (ctx, _d) = make_test_ctx().await;
    let f = seed_fx(&ctx).await;
    // «Полка 1» не складская сама, склад - её предок.
    let shelf_flag = scalar_i64(
        &ctx,
        "SELECT is_storage FROM places WHERE id = ?1",
        vec![int(f.shelf)],
    )
    .await;
    assert_eq!(shelf_flag, 0);
    // Со склада на полку склада - перехода нет.
    let none = ctx
        .groups
        .move_group_status_preview(&admin(), f.arm, f.shelf)
        .await
        .expect("preview shelf");
    assert_eq!(none.transition, None);
    // Из кабинета на полку - переход на склад.
    let v = group_version(&ctx, f.arm).await;
    ctx.groups
        .move_group_with_status(&admin(), f.arm, v, f.room12, true)
        .await
        .expect("to work");
    assert_eq!(
        dev_state(&ctx, f.monitor).await,
        (Some(f.room12), "в_работе".into())
    );
    let preview = ctx
        .groups
        .move_group_status_preview(&admin(), f.arm, f.shelf)
        .await
        .expect("preview");
    assert_eq!(
        preview.transition,
        Some(GroupMoveStatusTransitionDto::ToStorage)
    );
    assert_eq!(preview.devices_affected, 3);
    let v = group_version(&ctx, f.arm).await;
    let result = ctx
        .groups
        .move_group_with_status(&admin(), f.arm, v, f.shelf, true)
        .await
        .expect("to shelf");
    assert_eq!(result.status_changed_devices, 3);
    assert_eq!(
        dev_state(&ctx, f.monitor).await,
        (Some(f.shelf), "на_складе".into())
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn move_status_group_without_place_changes_no_status() {
    let (ctx, _d) = make_test_ctx().await;
    let f = seed_fx(&ctx).await;
    let loose = seed_group(&ctx, "АРМ #10", 10, None, None).await;
    let d = seed_device(
        &ctx,
        "Монитор #30",
        "INV-30",
        Some(f.room12),
        Some(loose),
        "в_работе",
    )
    .await;
    let preview = ctx
        .groups
        .move_group_status_preview(&admin(), loose, f.store)
        .await
        .expect("preview");
    assert_eq!(preview.transition, None);
    assert_eq!(preview.devices_affected, 0);
    let v = group_version(&ctx, loose).await;
    let result = ctx
        .groups
        .move_group_with_status(&admin(), loose, v, f.store, true)
        .await
        .expect("move");
    assert_eq!(result.status_changed_devices, 0);
    assert_eq!(dev_state(&ctx, d).await, (Some(f.store), "в_работе".into()));
    assert_eq!(group_place(&ctx, loose).await, Some(f.store));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn move_status_preview_is_read_only_and_names_come_from_dictionary() {
    let (ctx, _d) = make_test_ctx().await;
    let f = seed_fx(&ctx).await;
    let mut rx = ctx.ws_broadcast.subscribe();
    while rx.try_recv().is_ok() {}

    let before_version = group_version(&ctx, f.arm).await;
    let before = (
        dev_state(&ctx, f.monitor).await,
        dev_state(&ctx, f.laptop_awaiting).await,
        group_place(&ctx, f.arm).await,
    );
    let audit_before = scalar_i64(&ctx, "SELECT COUNT(*) FROM audit_log", vec![]).await;
    let mv_before = scalar_i64(&ctx, "SELECT COUNT(*) FROM place_movements", vec![]).await;

    let p1: GroupMoveStatusPreviewDto = ctx
        .groups
        .move_group_status_preview(&admin(), f.arm, f.room12)
        .await
        .expect("preview");
    assert_eq!(p1.devices_affected, 3);
    assert_eq!(p1.target_status_code.as_deref(), Some("в_работе"));
    assert!(p1.target_status_name.is_some());

    assert_eq!(group_version(&ctx, f.arm).await, before_version);
    assert_eq!(
        (
            dev_state(&ctx, f.monitor).await,
            dev_state(&ctx, f.laptop_awaiting).await,
            group_place(&ctx, f.arm).await
        ),
        before
    );
    assert_eq!(
        scalar_i64(&ctx, "SELECT COUNT(*) FROM audit_log", vec![]).await,
        audit_before
    );
    assert_eq!(
        scalar_i64(&ctx, "SELECT COUNT(*) FROM place_movements", vec![]).await,
        mv_before
    );
    assert!(rx.try_recv().is_err(), "предпросмотр не шлёт событий");

    // Подпись берётся из справочника, а не из клиента.
    sql_exec(
        &ctx,
        "UPDATE device_statuses SET name = 'Эксплуатация' WHERE code = 'в_работе'",
        vec![],
    )
    .await;
    let p2 = ctx
        .groups
        .move_group_status_preview(&admin(), f.arm, f.room12)
        .await
        .expect("preview 2");
    assert_eq!(p2.target_status_name.as_deref(), Some("Эксплуатация"));

    // Те же проверки, что у переноса: вложенная группа отказывает.
    let nested = scalar_i64(
        &ctx,
        "SELECT id FROM groups WHERE parent_group_id IS NOT NULL",
        vec![],
    )
    .await;
    expect_validation(
        ctx.groups
            .move_group_status_preview(&admin(), nested, f.room12)
            .await
            .expect_err("nested"),
    );
    expect_validation(
        ctx.groups
            .move_group_status_preview(&admin(), f.arm, 999_999)
            .await
            .expect_err("no place"),
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn move_status_open_repair_request_keeps_status_and_leaves_preview() {
    let (ctx, _d) = make_test_ctx().await;
    let f = seed_fx(&ctx).await;
    sql_exec(
        &ctx,
        "INSERT INTO users (login, full_name, role, created_at_utc, updated_at_utc) \
         VALUES ('sidorov', 'Сидоров С.С.', 'employee', 1700000000, 1700000000)",
        vec![],
    )
    .await;
    let user = scalar_i64(&ctx, "SELECT id FROM users WHERE login = 'sidorov'", vec![]).await;
    sql_exec(
        &ctx,
        "INSERT INTO requests (request_type, status, requested_by_user_id, printer_device_id, \
         created_at_utc, updated_at_utc) VALUES ('repair', 'open', ?1, ?2, 1700000000, 1700000000)",
        vec![int(user), int(f.monitor)],
    )
    .await;
    assert_eq!(dev_state(&ctx, f.monitor).await.1, "на_складе");

    let preview = ctx
        .groups
        .move_group_status_preview(&admin(), f.arm, f.room12)
        .await
        .expect("preview");
    assert_eq!(preview.devices_affected, 2, "монитор с заявкой не входит");
    let v = group_version(&ctx, f.arm).await;
    let result = ctx
        .groups
        .move_group_with_status(&admin(), f.arm, v, f.room12, true)
        .await
        .expect("move");
    assert_eq!(result.status_changed_devices, 2);
    assert_eq!(
        dev_state(&ctx, f.monitor).await,
        (Some(f.room12), "на_складе".into())
    );
    assert_eq!(audit_status_changed(&ctx, f.monitor).await, 0);
    assert_eq!(
        dev_state(&ctx, f.mouse).await,
        (Some(f.room12), "в_работе".into())
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn move_status_missing_target_code_rolls_back_everything() {
    let (ctx, _d) = make_test_ctx().await;
    let f = seed_fx(&ctx).await;
    sql_exec(
        &ctx,
        "UPDATE device_statuses SET code = 'в_работе_x' WHERE code = 'в_работе'",
        vec![],
    )
    .await;
    let version = group_version(&ctx, f.arm).await;
    let before: Vec<_> = {
        let mut v = Vec::new();
        for id in [
            f.monitor,
            f.mouse,
            f.kb_to_write_off,
            f.laptop_awaiting,
            f.cable_nested,
        ] {
            v.push(dev_state(&ctx, id).await);
        }
        v
    };
    let err = ctx
        .groups
        .move_group_with_status(&admin(), f.arm, version, f.room12, true)
        .await
        .expect_err("нет целевого статуса");
    expect_validation(err);
    let mut after = Vec::new();
    for id in [
        f.monitor,
        f.mouse,
        f.kb_to_write_off,
        f.laptop_awaiting,
        f.cable_nested,
    ] {
        after.push(dev_state(&ctx, id).await);
    }
    assert_eq!(before, after, "места и статусы устройств не изменены");
    assert_eq!(group_place(&ctx, f.arm).await, Some(f.store));
    assert_eq!(group_version(&ctx, f.arm).await, version);
    assert_eq!(audit_status_changed(&ctx, f.monitor).await, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn move_status_rights_denied_for_employee() {
    let (ctx, _d) = make_test_ctx().await;
    let f = seed_fx(&ctx).await;
    let emp = employee(&ctx).await;
    let v = group_version(&ctx, f.arm).await;
    let e1 = ctx
        .groups
        .move_group_with_status(&emp, f.arm, v, f.room12, true)
        .await
        .expect_err("move denied");
    assert!(
        matches!(e1, AppError::Forbidden | AppError::Unauthorized),
        "{e1:?}"
    );
    let e2 = ctx
        .groups
        .move_group_status_preview(&emp, f.arm, f.room12)
        .await
        .expect_err("preview denied");
    assert!(
        matches!(e2, AppError::Forbidden | AppError::Unauthorized),
        "{e2:?}"
    );
    assert_eq!(group_place(&ctx, f.arm).await, Some(f.store));
}

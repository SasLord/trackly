//! Phase 41 Plan 10: перенос группы (`GroupService::move_group`) — протаскивание
//! места на состав включая вложенные группы, атомарность, вложенная группа read-only,
//! CAS по версии, права. Реальный `AppCtx` на временном каталоге; имена вымышленные.
//!
//! Префиксы тестов: `move_`, `move_atomic_`, `move_rights_`.

use rusqlite::params;

use trackly_app::context::AppCtx;
use trackly_app::dto::auth::UserNew;
use trackly_app::dto::cartridge::{CartridgeCreateDto, CartridgeModelCreateDto};
use trackly_app::dto::number_template::NumberFieldInput;
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

/// Версия группы напрямую из БД (CAS).
async fn group_version(ctx: &AppCtx, id: i64) -> i64 {
    scalar_i64(
        ctx,
        "SELECT version FROM groups WHERE id = ?1",
        vec![int(id)],
    )
    .await
}

/// Состав для переноса: корневая «АРМ #3» (4 прямых устройства) и вложенная «АРМ #4»
/// (2 устройства), всё на «Складе А». Ids устройств растут: 4-е — последнее прямое.
struct Fixture {
    root: i64,
    nested: i64,
    devices: Vec<i64>,
    place_a: i64,
    place_b: i64,
}

async fn seed_fixture(ctx: &AppCtx) -> Fixture {
    let arm = type_id_by_code(ctx, "workstation").await;
    let place_a = seed_place(ctx, "room", "Склад А", None).await;
    let place_b = seed_place(ctx, "room", "Склад Б", None).await;
    let root = seed_group(ctx, arm, "АРМ #3", 3, Some(place_a), None).await;
    let nested = seed_group(ctx, arm, "АРМ #4", 4, Some(place_a), Some(root)).await;
    let mut devices = Vec::new();
    for i in 0..4 {
        devices.push(
            seed_device(
                ctx,
                &format!("Системный блок {i}"),
                &format!("INV-M-{i}"),
                &format!("SN-M-{i}"),
                Some(place_a),
                Some(root),
            )
            .await,
        );
    }
    for i in 4..6 {
        devices.push(
            seed_device(
                ctx,
                &format!("Монитор {i}"),
                &format!("INV-M-{i}"),
                &format!("SN-M-{i}"),
                Some(place_a),
                Some(nested),
            )
            .await,
        );
    }
    Fixture {
        root,
        nested,
        devices,
        place_a,
        place_b,
    }
}

async fn device_place(ctx: &AppCtx, id: i64) -> Option<i64> {
    opt_i64(
        ctx,
        "SELECT place_id FROM devices WHERE id = ?1",
        vec![int(id)],
    )
    .await
}

async fn group_place(ctx: &AppCtx, id: i64) -> Option<i64> {
    opt_i64(
        ctx,
        "SELECT place_id FROM groups WHERE id = ?1",
        vec![int(id)],
    )
    .await
}

/// Снимок всего, что перенос обязан откатывать целиком.
async fn state_snapshot(
    ctx: &AppCtx,
    f: &Fixture,
) -> (Vec<Option<i64>>, Vec<Option<i64>>, i64, i64) {
    let mut groups = Vec::new();
    for g in [f.root, f.nested] {
        groups.push(group_place(ctx, g).await);
    }
    let mut devs = Vec::new();
    for d in &f.devices {
        devs.push(device_place(ctx, *d).await);
    }
    let movements = scalar_i64(ctx, "SELECT COUNT(*) FROM place_movements", vec![]).await;
    let audit = scalar_i64(ctx, "SELECT COUNT(*) FROM audit_log", vec![]).await;
    (groups, devs, movements, audit)
}

// ---------------------------------------------------------------------------
// move_
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn move_propagates_to_all_devices_including_nested() {
    let (ctx, _dir) = make_test_ctx().await;
    let f = seed_fixture(&ctx).await;
    let v = group_version(&ctx, f.root).await;

    let out = ctx
        .groups
        .move_group(&admin(), f.root, v, f.place_b)
        .await
        .expect("move");

    for d in &f.devices {
        assert_eq!(device_place(&ctx, *d).await, Some(f.place_b));
    }
    assert_eq!(group_place(&ctx, f.root).await, Some(f.place_b));
    assert_eq!(group_place(&ctx, f.nested).await, Some(f.place_b));
    assert_eq!(out.moved_devices, 6);
    assert_eq!(out.moved_nested_groups, 1);
    assert_eq!(out.summary, "группа и 6 устройств");
    assert!(out.changed_place_ids.contains(&f.place_a));
    assert!(out.changed_place_ids.contains(&f.place_b));
    assert_eq!(out.changed_place_ids.len(), 2, "без дублей");
    assert!(out.batch_id.is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn move_summary_uses_singular_plural_forms() {
    let (ctx, _dir) = make_test_ctx().await;
    let arm = type_id_by_code(&ctx, "workstation").await;
    let a = seed_place(&ctx, "room", "Склад А", None).await;
    let b = seed_place(&ctx, "room", "Склад Б", None).await;
    let g = seed_group(&ctx, arm, "АРМ #1", 1, Some(a), None).await;
    seed_device(&ctx, "Ноутбук", "INV-S-1", "SN-S-1", Some(a), Some(g)).await;
    let v = group_version(&ctx, g).await;
    let out = ctx
        .groups
        .move_group(&admin(), g, v, b)
        .await
        .expect("move");
    assert_eq!(out.summary, "группа и 1 устройство");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn move_nested_group_is_rejected_and_db_untouched() {
    let (ctx, _dir) = make_test_ctx().await;
    let f = seed_fixture(&ctx).await;
    let before = state_snapshot(&ctx, &f).await;
    let v = group_version(&ctx, f.nested).await;

    let err = ctx
        .groups
        .move_group(&admin(), f.nested, v, f.place_b)
        .await
        .expect_err("вложенную нельзя переносить");
    match err {
        AppError::Validation { message, .. } => {
            assert!(message.contains("АРМ #3"), "имя корня в тексте: {message}");
        }
        other => panic!("ожидали Validation, получили {other:?}"),
    }
    assert_eq!(state_snapshot(&ctx, &f).await, before);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn move_group_without_place_assigns_place_and_propagates() {
    let (ctx, _dir) = make_test_ctx().await;
    let arm = type_id_by_code(&ctx, "workstation").await;
    let a = seed_place(&ctx, "room", "Склад А", None).await;
    let b = seed_place(&ctx, "room", "Склад Б", None).await;
    let g = seed_group(&ctx, arm, "АРМ #7", 7, None, None).await;
    let with_place = seed_device(
        &ctx,
        "Системный блок",
        "INV-N-1",
        "SN-N-1",
        Some(a),
        Some(g),
    )
    .await;
    let without_place = seed_device(&ctx, "Монитор", "INV-N-2", "SN-N-2", None, Some(g)).await;
    let v = group_version(&ctx, g).await;

    let out = ctx
        .groups
        .move_group(&admin(), g, v, b)
        .await
        .expect("move");

    assert_eq!(group_place(&ctx, g).await, Some(b));
    assert_eq!(device_place(&ctx, with_place).await, Some(b));
    assert_eq!(device_place(&ctx, without_place).await, Some(b));
    assert_eq!(out.moved_devices, 2);
    assert_eq!(
        scalar_i64(
            &ctx,
            "SELECT COUNT(*) FROM place_movements WHERE entity_type = 'group'",
            vec![]
        )
        .await,
        0,
        "NULL -> место: строки группы нет"
    );
    assert_eq!(
        scalar_i64(
            &ctx,
            "SELECT COUNT(*) FROM place_movements WHERE entity_type = 'device' AND entity_id = ?1",
            vec![int(with_place)]
        )
        .await,
        1
    );
    assert_eq!(
        scalar_i64(
            &ctx,
            "SELECT COUNT(*) FROM place_movements WHERE entity_id = ?1",
            vec![int(without_place)]
        )
        .await,
        0
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn move_printer_member_carries_attached_cartridge() {
    let (ctx, _dir) = make_test_ctx().await;
    let arm = type_id_by_code(&ctx, "workstation").await;
    let a = seed_place(&ctx, "room", "Склад А", None).await;
    let b = seed_place(&ctx, "room", "Склад Б", None).await;
    let g = seed_group(&ctx, arm, "АРМ #8", 8, Some(a), None).await;
    let printer = seed_device(&ctx, "Принтер", "INV-P-1", "SN-P-1", Some(a), Some(g)).await;
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "UPDATE devices SET type_id = 2 WHERE id = ?1",
                params![printer],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("make printer");
    let model = ctx
        .cartridges
        .model_create(CartridgeModelCreateDto {
            brand: "HP".into(),
            model: "CE285A".into(),
            kind_id: 1,
            color: Some("Чёрный".into()),
            notes: None,
            compatibility: vec![],
        })
        .await
        .expect("model")
        .id;
    let cartridge = ctx
        .cartridges
        .create(CartridgeCreateDto {
            model_id: model,
            number_input: NumberFieldInput {
                value: "C-MOVE-00001".into(),
                template_id: None,
                confirm_mismatch: false,
                confirm_script_mix: false,
            },
            state_id: Some(1),
            place_id: Some(a),
            notes: None,
        })
        .await
        .expect("cartridge")
        .expect_created("cartridge")
        .id;
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "UPDATE cartridges SET current_printer_device_id = ?1 WHERE id = ?2",
                params![printer, cartridge],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("attach");

    let v = group_version(&ctx, g).await;
    ctx.groups
        .move_group(&admin(), g, v, b)
        .await
        .expect("move");

    assert_eq!(
        opt_i64(
            &ctx,
            "SELECT place_id FROM cartridges WHERE id = ?1",
            vec![int(cartridge)]
        )
        .await,
        Some(b)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn move_stale_version_is_rejected_and_db_untouched() {
    let (ctx, _dir) = make_test_ctx().await;
    let f = seed_fixture(&ctx).await;
    let v = group_version(&ctx, f.root).await;
    let before = state_snapshot(&ctx, &f).await;

    let err = ctx
        .groups
        .move_group(&admin(), f.root, v - 1, f.place_b)
        .await
        .expect_err("устаревшая версия");
    assert!(
        matches!(err, AppError::OptimisticLockMismatch { .. }),
        "{err:?}"
    );
    assert_eq!(state_snapshot(&ctx, &f).await, before);

    ctx.groups
        .move_group(&admin(), f.root, v, f.place_b)
        .await
        .expect("актуальная версия проходит");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn move_missing_target_place_is_rejected_and_db_untouched() {
    let (ctx, _dir) = make_test_ctx().await;
    let f = seed_fixture(&ctx).await;
    let v = group_version(&ctx, f.root).await;
    let before = state_snapshot(&ctx, &f).await;

    let err = ctx
        .groups
        .move_group(&admin(), f.root, v, 999_999)
        .await
        .expect_err("нет такого места");
    assert!(
        matches!(err, AppError::Validation { .. } | AppError::NotFound { .. }),
        "{err:?}"
    );
    assert_eq!(state_snapshot(&ctx, &f).await, before);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn move_to_same_place_writes_nothing() {
    let (ctx, _dir) = make_test_ctx().await;
    let f = seed_fixture(&ctx).await;
    let v = group_version(&ctx, f.root).await;
    let before = state_snapshot(&ctx, &f).await;

    let out = ctx
        .groups
        .move_group(&admin(), f.root, v, f.place_a)
        .await
        .expect("move to same place");
    assert_eq!(out.moved_devices, 0);
    assert!(out.changed_place_ids.is_empty());
    assert_eq!(out.summary, "группа");
    assert_eq!(state_snapshot(&ctx, &f).await, before);
    assert_eq!(
        group_version(&ctx, f.root).await,
        v,
        "версия группы не растёт"
    );
}

// ---------------------------------------------------------------------------
// move_atomic_
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn move_atomic_failure_on_fourth_device_rolls_back_everything() {
    let (ctx, _dir) = make_test_ctx().await;
    let f = seed_fixture(&ctx).await;
    let v = group_version(&ctx, f.root).await;
    let fourth = f.devices[3];
    ctx.writer
        .execute(move |conn| {
            conn.execute_batch(&format!(
                "CREATE TRIGGER fail_fourth BEFORE UPDATE OF place_id ON devices \
                 WHEN NEW.id = {fourth} BEGIN SELECT RAISE(ABORT, 'boom'); END;"
            ))
            .map_err(map_rusqlite)
        })
        .await
        .expect("trigger");
    let before = state_snapshot(&ctx, &f).await;

    let res = ctx.groups.move_group(&admin(), f.root, v, f.place_b).await;
    let err = res.expect_err("сбой на 4-м устройстве должен вернуть ошибку");
    assert!(
        format!("{err:?}").contains("boom"),
        "ошибка должна быть от триггера, а не от иной причины: {err:?}"
    );

    let after = state_snapshot(&ctx, &f).await;
    assert_eq!(after.0, before.0, "groups.place_id");
    assert_eq!(after.1, before.1, "devices.place_id");
    assert_eq!(after.2, before.2, "place_movements");
    assert_eq!(after.3, before.3, "audit_log");
    // фикстура различает «до» и «после»: место «Б» нигде не появилось
    assert!(after.1.iter().all(|p| *p == Some(f.place_a)));
}

// ---------------------------------------------------------------------------
// move_rights_
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn move_rights_employee_forbidden_manager_allowed() {
    let (ctx, _dir) = make_test_ctx().await;
    let f = seed_fixture(&ctx).await;
    let v = group_version(&ctx, f.root).await;
    let employee = create_identity(&ctx, "emp1", "Петров П.П.", Role::Employee).await;
    let manager = create_identity(&ctx, "mgr1", "Иванов И.И.", Role::Manager).await;
    let before = state_snapshot(&ctx, &f).await;

    let err = ctx
        .groups
        .move_group(&employee, f.root, v, f.place_b)
        .await
        .expect_err("employee");
    assert!(matches!(err, AppError::Forbidden));
    assert_eq!(state_snapshot(&ctx, &f).await, before);

    ctx.groups
        .move_group(&manager, f.root, v, f.place_b)
        .await
        .expect("manager");
    assert_eq!(group_place(&ctx, f.root).await, Some(f.place_b));
}

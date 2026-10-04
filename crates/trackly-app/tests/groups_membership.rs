//! Phase 41 Plan 11: состав и вложенность групп (`GroupService::{add_devices,
//! remove_devices, set_parent}`) — атомарный пакет, правило «одна группа на устройство»,
//! место присваивает только группа с местом (D-21), производное место вложенных групп.
//! Реальный `AppCtx` на временном каталоге; имена вымышленные.
//!
//! Префиксы тестов: `membership_add_`, `membership_remove_`, `membership_rights_`,
//! `membership_nesting_`, `membership_place_invariant_`.

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

async fn text_of(ctx: &AppCtx, sql: &'static str, p: Vec<rusqlite::types::Value>) -> String {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(sql, rusqlite::params_from_iter(p), |r| {
                r.get::<_, String>(0)
            })
            .map_err(map_rusqlite)
        })
        .await
        .expect("text query")
}

async fn device_place(ctx: &AppCtx, id: i64) -> Option<i64> {
    opt_i64(
        ctx,
        "SELECT place_id FROM devices WHERE id = ?1",
        vec![int(id)],
    )
    .await
}

async fn member_count(ctx: &AppCtx) -> i64 {
    scalar_i64(ctx, "SELECT COUNT(*) FROM group_devices", vec![]).await
}

async fn movement_count_for_device(ctx: &AppCtx, id: i64) -> i64 {
    scalar_i64(
        ctx,
        "SELECT COUNT(*) FROM place_movements WHERE entity_type = 'device' AND entity_id = ?1",
        vec![int(id)],
    )
    .await
}

async fn audit_count(ctx: &AppCtx, action: &'static str, entity_id: i64) -> i64 {
    scalar_i64(
        ctx,
        "SELECT COUNT(*) FROM audit_log WHERE action = ?1 AND entity_id = ?2",
        vec![
            rusqlite::types::Value::Text(action.to_string()),
            int(entity_id),
        ],
    )
    .await
}

/// Два склада, тип «АРМ» и группа «АРМ #3» (с местом «Склад А» или без).
struct Base {
    arm: i64,
    place_a: i64,
    place_b: i64,
}

async fn seed_base(ctx: &AppCtx) -> Base {
    Base {
        arm: type_id_by_code(ctx, "workstation").await,
        place_a: seed_place(ctx, "room", "Склад А", None).await,
        place_b: seed_place(ctx, "room", "Склад Б", None).await,
    }
}

fn validation_message(err: AppError) -> String {
    match err {
        AppError::Validation { message, .. } => message,
        other => panic!("ожидали Validation, получили {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// membership_add_
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn membership_add_to_group_with_place_assigns_place_and_journals() {
    let (ctx, _dir) = make_test_ctx().await;
    let b = seed_base(&ctx).await;
    let g = seed_group(&ctx, b.arm, "АРМ #3", 3, Some(b.place_a), None).await;
    let moved = seed_device(
        &ctx,
        "Ноутбук 1",
        "INV-A-1",
        "SN-A-1",
        Some(b.place_b),
        None,
    )
    .await;
    let placeless = seed_device(&ctx, "Ноутбук 2", "INV-A-2", "SN-A-2", None, None).await;
    let already = seed_device(
        &ctx,
        "Ноутбук 3",
        "INV-A-3",
        "SN-A-3",
        Some(b.place_a),
        None,
    )
    .await;

    let out = ctx
        .groups
        .add_devices(&admin(), g, vec![moved, placeless, already])
        .await
        .expect("add");

    assert_eq!(out.added, 3);
    for d in [moved, placeless, already] {
        assert_eq!(device_place(&ctx, d).await, Some(b.place_a));
    }
    assert_eq!(member_count(&ctx).await, 3);
    assert!(out.changed_place_ids.contains(&b.place_a));
    assert!(out.changed_place_ids.contains(&b.place_b));
    assert_eq!(out.changed_place_ids.len(), 2, "без дублей");

    // Some -> Some: строка журнала без пакета, но с именем и id группы (ссылка D-28).
    let group_name = text_of(&ctx, "SELECT name FROM groups WHERE id = ?1", vec![int(g)]).await;
    assert_eq!(movement_count_for_device(&ctx, moved).await, 1);
    let label = text_of(
        &ctx,
        "SELECT entity_label FROM place_movements WHERE entity_id = ?1 AND source = 'group'",
        vec![int(moved)],
    )
    .await;
    assert_eq!(label, group_name);
    assert_eq!(
        opt_i64(
            &ctx,
            "SELECT group_id FROM place_movements WHERE entity_id = ?1 AND source = 'group'",
            vec![int(moved)]
        )
        .await,
        Some(g)
    );
    assert_eq!(
        opt_i64(
            &ctx,
            "SELECT COUNT(batch_id) FROM place_movements WHERE entity_id = ?1",
            vec![int(moved)]
        )
        .await,
        Some(0),
        "batch_id NULL"
    );

    // NULL -> место: только audit_log (D-30); место уже совпадало: ничего.
    assert_eq!(movement_count_for_device(&ctx, placeless).await, 0);
    assert_eq!(
        audit_count(&ctx, "custom:group_place_assigned", placeless).await,
        1
    );
    assert_eq!(movement_count_for_device(&ctx, already).await, 0);
    assert_eq!(
        audit_count(&ctx, "custom:group_place_assigned", already).await,
        0
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn membership_add_to_group_without_place_keeps_device_places() {
    let (ctx, _dir) = make_test_ctx().await;
    let b = seed_base(&ctx).await;
    let g = seed_group(&ctx, b.arm, "АРМ #3", 3, None, None).await;
    let with_place = seed_device(
        &ctx,
        "Ноутбук 1",
        "INV-D-1",
        "SN-D-1",
        Some(b.place_b),
        None,
    )
    .await;
    let without = seed_device(&ctx, "Ноутбук 2", "INV-D-2", "SN-D-2", None, None).await;
    let before = (
        device_place(&ctx, with_place).await,
        device_place(&ctx, without).await,
    );
    assert_eq!(before, (Some(b.place_b), None));

    let out = ctx
        .groups
        .add_devices(&admin(), g, vec![with_place, without])
        .await
        .expect("add");

    assert_eq!(out.added, 2);
    assert_eq!(member_count(&ctx).await, 2);
    let after = (
        device_place(&ctx, with_place).await,
        device_place(&ctx, without).await,
    );
    assert_eq!(after, before, "D-21: место устройства не тронуто");
    assert!(out.changed_place_ids.is_empty());
    assert_eq!(
        scalar_i64(&ctx, "SELECT COUNT(*) FROM place_movements", vec![]).await,
        0
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn membership_add_occupied_device_names_both_and_batch_is_atomic() {
    let (ctx, _dir) = make_test_ctx().await;
    let b = seed_base(&ctx).await;
    let target = seed_group(&ctx, b.arm, "АРМ #3", 3, Some(b.place_a), None).await;
    let other = seed_group(&ctx, b.arm, "АРМ #9", 9, Some(b.place_b), None).await;
    let free1 = seed_device(
        &ctx,
        "Ноутбук 1",
        "INV-O-1",
        "SN-O-1",
        Some(b.place_b),
        None,
    )
    .await;
    let busy = seed_device(
        &ctx,
        "Монитор 2",
        "INV-O-2",
        "SN-O-2",
        Some(b.place_b),
        Some(other),
    )
    .await;
    let free2 = seed_device(
        &ctx,
        "Ноутбук 3",
        "INV-O-3",
        "SN-O-3",
        Some(b.place_b),
        None,
    )
    .await;

    let members_before = member_count(&ctx).await;
    let movements_before = scalar_i64(&ctx, "SELECT COUNT(*) FROM place_movements", vec![]).await;
    let places_before = (
        device_place(&ctx, free1).await,
        device_place(&ctx, busy).await,
        device_place(&ctx, free2).await,
    );

    let err = ctx
        .groups
        .add_devices(&admin(), target, vec![free1, busy, free2])
        .await
        .expect_err("занятое устройство отменяет пакет");
    let message = validation_message(err);

    let device_name = text_of(
        &ctx,
        "SELECT name FROM devices WHERE id = ?1",
        vec![int(busy)],
    )
    .await;
    let other_name = text_of(
        &ctx,
        "SELECT name FROM groups WHERE id = ?1",
        vec![int(other)],
    )
    .await;
    assert!(message.contains(&device_name), "имя устройства: {message}");
    assert!(message.contains(&other_name), "имя группы: {message}");
    assert!(
        message.contains("Сначала выведите его из состава."),
        "{message}"
    );

    assert_eq!(member_count(&ctx).await, members_before, "group_devices");
    assert_eq!(
        scalar_i64(&ctx, "SELECT COUNT(*) FROM place_movements", vec![]).await,
        movements_before
    );
    assert_eq!(
        (
            device_place(&ctx, free1).await,
            device_place(&ctx, busy).await,
            device_place(&ctx, free2).await,
        ),
        places_before,
        "место первого свободного устройства откатилось"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn membership_add_deleted_or_missing_device_is_rejected_atomically() {
    let (ctx, _dir) = make_test_ctx().await;
    let b = seed_base(&ctx).await;
    let g = seed_group(&ctx, b.arm, "АРМ #3", 3, Some(b.place_a), None).await;
    let live = seed_device(
        &ctx,
        "Ноутбук 1",
        "INV-X-1",
        "SN-X-1",
        Some(b.place_b),
        None,
    )
    .await;
    let dead = seed_device(
        &ctx,
        "Ноутбук 2",
        "INV-X-2",
        "SN-X-2",
        Some(b.place_b),
        None,
    )
    .await;
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "UPDATE devices SET deleted_at_utc = 1700000100 WHERE id = ?1",
                params![dead],
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("soft delete");

    let err = ctx
        .groups
        .add_devices(&admin(), g, vec![live, dead])
        .await
        .expect_err("удалённое устройство");
    assert!(matches!(err, AppError::Validation { .. }), "{err:?}");
    let err = ctx
        .groups
        .add_devices(&admin(), g, vec![live, 999_999])
        .await
        .expect_err("несуществующее устройство");
    assert!(matches!(err, AppError::Validation { .. }), "{err:?}");

    assert_eq!(member_count(&ctx).await, 0);
    assert_eq!(device_place(&ctx, live).await, Some(b.place_b));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn membership_add_validates_list_dedups_and_skips_own_members() {
    let (ctx, _dir) = make_test_ctx().await;
    let b = seed_base(&ctx).await;
    let g = seed_group(&ctx, b.arm, "АРМ #3", 3, None, None).await;
    let d1 = seed_device(&ctx, "Ноутбук 1", "INV-L-1", "SN-L-1", None, None).await;

    let err = ctx
        .groups
        .add_devices(&admin(), g, vec![])
        .await
        .expect_err("пусто");
    assert!(matches!(err, AppError::Validation { .. }), "{err:?}");
    let too_many: Vec<i64> = (1..=501).collect();
    let err = ctx
        .groups
        .add_devices(&admin(), g, too_many)
        .await
        .expect_err("больше 500");
    assert!(matches!(err, AppError::Validation { .. }), "{err:?}");

    let out = ctx
        .groups
        .add_devices(&admin(), g, vec![d1, d1])
        .await
        .expect("дубликаты схлопываются");
    assert_eq!(out.added, 1);
    assert_eq!(member_count(&ctx).await, 1);

    let again = ctx
        .groups
        .add_devices(&admin(), g, vec![d1])
        .await
        .expect("уже в этой группе — без ошибки");
    assert_eq!(again.added, 0);
    assert_eq!(member_count(&ctx).await, 1);
}

// ---------------------------------------------------------------------------
// membership_remove_
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn membership_remove_keeps_place_and_audits_release() {
    let (ctx, _dir) = make_test_ctx().await;
    let b = seed_base(&ctx).await;
    let g = seed_group(&ctx, b.arm, "АРМ #3", 3, Some(b.place_a), None).await;
    let d1 = seed_device(
        &ctx,
        "Ноутбук 1",
        "INV-R-1",
        "SN-R-1",
        Some(b.place_a),
        Some(g),
    )
    .await;
    let d2 = seed_device(
        &ctx,
        "Ноутбук 2",
        "INV-R-2",
        "SN-R-2",
        Some(b.place_a),
        Some(g),
    )
    .await;

    let released = ctx
        .groups
        .remove_devices(&admin(), g, vec![d1])
        .await
        .expect("remove");

    assert_eq!(released, 1);
    assert_eq!(
        scalar_i64(
            &ctx,
            "SELECT COUNT(*) FROM group_devices WHERE device_id = ?1",
            vec![int(d1)]
        )
        .await,
        0
    );
    assert_eq!(
        scalar_i64(
            &ctx,
            "SELECT COUNT(*) FROM group_devices WHERE device_id = ?1",
            vec![int(d2)]
        )
        .await,
        1,
        "соседнее устройство осталось"
    );
    assert_eq!(
        device_place(&ctx, d1).await,
        Some(b.place_a),
        "D-02: место прежнее"
    );
    assert_eq!(
        audit_count(&ctx, "custom:group_member_released", d1).await,
        1
    );
    let payload = text_of(
        &ctx,
        "SELECT payload_json FROM audit_log WHERE action = 'custom:group_member_released' \
         AND entity_id = ?1",
        vec![int(d1)],
    )
    .await;
    let v: serde_json::Value = serde_json::from_str(&payload).expect("json");
    assert_eq!(v["group_id"], serde_json::json!(g));
    assert!(v["act_id"].is_null());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn membership_remove_is_idempotent_for_non_members_and_foreign_members() {
    let (ctx, _dir) = make_test_ctx().await;
    let b = seed_base(&ctx).await;
    let g = seed_group(&ctx, b.arm, "АРМ #3", 3, None, None).await;
    let other = seed_group(&ctx, b.arm, "АРМ #4", 4, None, None).await;
    let free = seed_device(&ctx, "Ноутбук 1", "INV-I-1", "SN-I-1", None, None).await;
    let foreign = seed_device(&ctx, "Ноутбук 2", "INV-I-2", "SN-I-2", None, Some(other)).await;

    let released = ctx
        .groups
        .remove_devices(&admin(), g, vec![free, foreign])
        .await
        .expect("идемпотентно, без ошибки");

    assert_eq!(released, 0);
    assert_eq!(member_count(&ctx).await, 1, "чужой член не тронут");
    assert_eq!(
        scalar_i64(
            &ctx,
            "SELECT COUNT(*) FROM audit_log WHERE action = 'custom:group_member_released'",
            vec![]
        )
        .await,
        0
    );
}

// ---------------------------------------------------------------------------
// membership_rights_
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn membership_rights_employee_forbidden_manager_allowed() {
    let (ctx, _dir) = make_test_ctx().await;
    let b = seed_base(&ctx).await;
    let g = seed_group(&ctx, b.arm, "АРМ #3", 3, None, None).await;
    let d1 = seed_device(&ctx, "Ноутбук 1", "INV-P-1", "SN-P-1", None, None).await;
    let employee = create_identity(&ctx, "emp1", "Петров П.П.", Role::Employee).await;
    let manager = create_identity(&ctx, "mgr1", "Иванов И.И.", Role::Manager).await;

    let err = ctx
        .groups
        .add_devices(&employee, g, vec![d1])
        .await
        .expect_err("employee add");
    assert!(matches!(err, AppError::Forbidden));
    assert_eq!(member_count(&ctx).await, 0);

    ctx.groups
        .add_devices(&manager, g, vec![d1])
        .await
        .expect("manager add");
    assert_eq!(member_count(&ctx).await, 1);

    let err = ctx
        .groups
        .remove_devices(&employee, g, vec![d1])
        .await
        .expect_err("employee remove");
    assert!(matches!(err, AppError::Forbidden));
    assert_eq!(member_count(&ctx).await, 1);

    ctx.groups
        .remove_devices(&manager, g, vec![d1])
        .await
        .expect("manager remove");
    assert_eq!(member_count(&ctx).await, 0);
}

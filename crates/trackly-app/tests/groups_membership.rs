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

async fn group_version(ctx: &AppCtx, id: i64) -> i64 {
    scalar_i64(
        ctx,
        "SELECT version FROM groups WHERE id = ?1",
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

async fn group_parent(ctx: &AppCtx, id: i64) -> Option<i64> {
    opt_i64(
        ctx,
        "SELECT parent_group_id FROM groups WHERE id = ?1",
        vec![int(id)],
    )
    .await
}

/// Инвариант: место КАЖДОЙ вложенной группы равно месту её корня. Обходит все группы БД
/// (корень ищется подъёмом по `parent_group_id`); возвращает число проверенных вложенных.
async fn assert_place_invariant(ctx: &AppCtx, step: &str) -> usize {
    let rows: Vec<(i64, Option<i64>, Option<i64>)> = ctx
        .writer
        .execute(|conn| {
            let mut stmt = conn
                .prepare("SELECT id, parent_group_id, place_id FROM groups")
                .map_err(map_rusqlite)?;
            let rows = stmt
                .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
                .map_err(map_rusqlite)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(map_rusqlite)?;
            Ok(rows)
        })
        .await
        .expect("groups walk");
    let find = |id: i64| rows.iter().find(|r| r.0 == id).expect("group exists");
    let mut nested = 0;
    for row in &rows {
        if row.1.is_none() {
            continue;
        }
        nested += 1;
        let mut root = row;
        for _ in 0..rows.len() {
            match root.1 {
                Some(p) => root = find(p),
                None => break,
            }
        }
        assert!(root.1.is_none(), "{step}: цикл вложенности");
        assert_eq!(
            row.2, root.2,
            "{step}: место вложенной группы {} != месту корня {}",
            row.0, root.0
        );
    }
    nested
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
    let AppError::AlreadyInGroup { items } = err else {
        panic!("ожидали AlreadyInGroup, получили {err:?}");
    };
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].id, busy);
    assert_eq!(items[0].group_id, other);
    assert_eq!(items[0].name, "Монитор 2");
    assert_eq!(items[0].group_name, "АРМ #9");

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

// ---------------------------------------------------------------------------
// membership_nesting_
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn membership_nesting_cycle_and_teardown_are_rejected() {
    let (ctx, _dir) = make_test_ctx().await;
    let b = seed_base(&ctx).await;
    let root = seed_group(&ctx, b.arm, "АРМ #3", 3, None, None).await;
    let child = seed_group(&ctx, b.arm, "АРМ #4", 4, None, Some(root)).await;
    let teardown_type = ctx
        .writer
        .execute(|conn| {
            conn.execute(
                "INSERT INTO group_types (code, name, behavior, created_at_utc, updated_at_utc) \
                 VALUES ('dismantle_t', 'Разбор', 'teardown', 1700000000, 1700000000)",
                [],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("teardown type");
    let teardown = seed_group(&ctx, teardown_type, "Разбор #1", 1, None, None).await;
    let loose = seed_group(&ctx, b.arm, "АРМ #5", 5, None, None).await;

    // корень -> в собственного потомка
    let v = group_version(&ctx, root).await;
    let msg = validation_message(
        ctx.groups
            .set_parent(&admin(), root, v, Some(child), false)
            .await
            .expect_err("цикл"),
    );
    assert_eq!(
        msg,
        "Нельзя вложить группу в саму себя или в свою вложенную группу."
    );
    assert_eq!(
        group_parent(&ctx, root).await,
        None,
        "parent_group_id не изменился"
    );
    assert_eq!(group_version(&ctx, root).await, v);

    // в саму себя
    let v = group_version(&ctx, loose).await;
    let err = ctx
        .groups
        .set_parent(&admin(), loose, v, Some(loose), false)
        .await
        .expect_err("сама в себя");
    assert!(matches!(err, AppError::Validation { .. }), "{err:?}");
    assert_eq!(group_parent(&ctx, loose).await, None);

    // в группу типа «Разбор»
    let msg = validation_message(
        ctx.groups
            .set_parent(&admin(), loose, v, Some(teardown), false)
            .await
            .expect_err("teardown"),
    );
    assert_eq!(
        msg,
        "Группа типа «Разбор» не может содержать другие группы."
    );
    assert_eq!(group_parent(&ctx, loose).await, None);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn membership_nesting_under_parent_with_place_propagates_with_journal_batch() {
    let (ctx, _dir) = make_test_ctx().await;
    let b = seed_base(&ctx).await;
    let parent = seed_group(&ctx, b.arm, "АРМ #3", 3, Some(b.place_a), None).await;
    let child = seed_group(&ctx, b.arm, "АРМ #4", 4, Some(b.place_b), None).await;
    let d1 = seed_device(
        &ctx,
        "Системный блок",
        "INV-N-1",
        "SN-N-1",
        Some(b.place_b),
        Some(child),
    )
    .await;
    let d2 = seed_device(
        &ctx,
        "Монитор",
        "INV-N-2",
        "SN-N-2",
        Some(b.place_b),
        Some(child),
    )
    .await;
    let v = group_version(&ctx, child).await;

    let out = ctx
        .groups
        .set_parent(&admin(), child, v, Some(parent), false)
        .await
        .expect("вложение");

    assert_eq!(group_parent(&ctx, child).await, Some(parent));
    assert_eq!(group_place(&ctx, child).await, Some(b.place_a));
    assert_eq!(device_place(&ctx, d1).await, Some(b.place_a));
    assert_eq!(device_place(&ctx, d2).await, Some(b.place_a));
    assert_eq!(out.moved_devices, 2);
    assert!(out.changed_place_ids.contains(&b.place_a));
    assert!(out.changed_place_ids.contains(&b.place_b));

    let child_name = text_of(
        &ctx,
        "SELECT name FROM groups WHERE id = ?1",
        vec![int(child)],
    )
    .await;
    let batch = text_of(
        &ctx,
        "SELECT DISTINCT batch_id FROM place_movements WHERE source = 'group'",
        vec![],
    )
    .await;
    assert_eq!(out.batch_id.as_deref(), Some(batch.as_str()));
    assert_eq!(
        scalar_i64(
            &ctx,
            "SELECT COUNT(*) FROM place_movements WHERE batch_id = ?1 AND entity_type = 'group' \
             AND entity_id = ?2 AND group_id = ?2 AND entity_label = ?3",
            vec![
                rusqlite::types::Value::Text(batch.clone()),
                int(child),
                rusqlite::types::Value::Text(child_name.clone()),
            ]
        )
        .await,
        1,
        "строка группы Some->Some"
    );
    assert_eq!(
        scalar_i64(
            &ctx,
            "SELECT COUNT(*) FROM place_movements WHERE batch_id = ?1 AND entity_type = 'device' \
             AND group_id = ?2 AND entity_label = ?3",
            vec![
                rusqlite::types::Value::Text(batch),
                int(child),
                rusqlite::types::Value::Text(child_name),
            ]
        )
        .await,
        2,
        "строки устройств с общим batch_id, group_id и именем группы"
    );
    assert_eq!(audit_count(&ctx, "set_parent", child).await, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn membership_nesting_under_parent_without_place_clears_group_place_only() {
    let (ctx, _dir) = make_test_ctx().await;
    let b = seed_base(&ctx).await;
    let parent = seed_group(&ctx, b.arm, "АРМ #3", 3, None, None).await;
    let child = seed_group(&ctx, b.arm, "АРМ #4", 4, Some(b.place_b), None).await;
    let grandchild = seed_group(&ctx, b.arm, "АРМ #5", 5, Some(b.place_b), Some(child)).await;
    let d1 = seed_device(
        &ctx,
        "Системный блок",
        "INV-W-1",
        "SN-W-1",
        Some(b.place_b),
        Some(child),
    )
    .await;
    let d2 = seed_device(
        &ctx,
        "Монитор",
        "INV-W-2",
        "SN-W-2",
        Some(b.place_b),
        Some(grandchild),
    )
    .await;
    let v = group_version(&ctx, child).await;

    let out = ctx
        .groups
        .set_parent(&admin(), child, v, Some(parent), false)
        .await
        .expect("вложение");

    assert_eq!(group_place(&ctx, child).await, None);
    assert_eq!(
        group_place(&ctx, grandchild).await,
        None,
        "вся ветка без места"
    );
    assert_eq!(
        device_place(&ctx, d1).await,
        Some(b.place_b),
        "запрет спит: место устройства цело"
    );
    assert_eq!(device_place(&ctx, d2).await, Some(b.place_b));
    assert_eq!(out.moved_devices, 0);
    assert_eq!(
        scalar_i64(&ctx, "SELECT COUNT(*) FROM place_movements", vec![]).await,
        0
    );
    assert_eq!(assert_place_invariant(&ctx, "без места").await, 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn membership_nesting_exit_from_parent_keeps_places() {
    let (ctx, _dir) = make_test_ctx().await;
    let b = seed_base(&ctx).await;
    let parent = seed_group(&ctx, b.arm, "АРМ #3", 3, Some(b.place_a), None).await;
    let child = seed_group(&ctx, b.arm, "АРМ #4", 4, Some(b.place_a), Some(parent)).await;
    let d1 = seed_device(
        &ctx,
        "Системный блок",
        "INV-E-1",
        "SN-E-1",
        Some(b.place_a),
        Some(child),
    )
    .await;
    let v = group_version(&ctx, child).await;

    let out = ctx
        .groups
        .set_parent(&admin(), child, v, None, false)
        .await
        .expect("выход");

    assert_eq!(group_parent(&ctx, child).await, None);
    assert_eq!(group_place(&ctx, child).await, Some(b.place_a));
    assert_eq!(device_place(&ctx, d1).await, Some(b.place_a));
    assert_eq!(out.moved_devices, 0);
    assert!(out.changed_place_ids.is_empty());
    assert_eq!(
        scalar_i64(&ctx, "SELECT COUNT(*) FROM place_movements", vec![]).await,
        0
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn membership_nesting_rights_and_stale_version() {
    let (ctx, _dir) = make_test_ctx().await;
    let b = seed_base(&ctx).await;
    let parent = seed_group(&ctx, b.arm, "АРМ #3", 3, Some(b.place_a), None).await;
    let child = seed_group(&ctx, b.arm, "АРМ #4", 4, None, None).await;
    let v = group_version(&ctx, child).await;
    let employee = create_identity(&ctx, "emp1", "Петров П.П.", Role::Employee).await;
    let manager = create_identity(&ctx, "mgr1", "Иванов И.И.", Role::Manager).await;

    let err = ctx
        .groups
        .set_parent(&employee, child, v, Some(parent), false)
        .await
        .expect_err("employee");
    assert!(matches!(err, AppError::Forbidden));
    assert_eq!(group_parent(&ctx, child).await, None);

    let err = ctx
        .groups
        .set_parent(&manager, child, v - 1, Some(parent), false)
        .await
        .expect_err("устаревшая версия");
    assert!(
        matches!(err, AppError::OptimisticLockMismatch { .. }),
        "{err:?}"
    );
    assert_eq!(group_parent(&ctx, child).await, None);
    assert_eq!(group_place(&ctx, child).await, None);

    ctx.groups
        .set_parent(&manager, child, v, Some(parent), false)
        .await
        .expect("manager");
    assert_eq!(group_parent(&ctx, child).await, Some(parent));
    assert_eq!(group_place(&ctx, child).await, Some(b.place_a));
}

// ---------------------------------------------------------------------------
// membership_place_invariant_
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn membership_place_invariant_walk() {
    let (ctx, _dir) = make_test_ctx().await;
    let b = seed_base(&ctx).await;
    let parent = seed_group(&ctx, b.arm, "АРМ #3", 3, Some(b.place_a), None).await;
    let placeless = seed_group(&ctx, b.arm, "АРМ #6", 6, None, None).await;
    let a = seed_group(&ctx, b.arm, "АРМ #4", 4, Some(b.place_b), None).await;
    let a2 = seed_group(&ctx, b.arm, "АРМ #5", 5, Some(b.place_b), Some(a)).await;
    seed_device(
        &ctx,
        "Системный блок",
        "INV-V-1",
        "SN-V-1",
        Some(b.place_b),
        Some(a2),
    )
    .await;
    assert_eq!(assert_place_invariant(&ctx, "старт").await, 1);

    // 1. вложение под родителя с местом
    let v = group_version(&ctx, a).await;
    ctx.groups
        .set_parent(&admin(), a, v, Some(parent), false)
        .await
        .expect("вложение");
    assert_eq!(assert_place_invariant(&ctx, "вложение").await, 2);
    assert_eq!(group_place(&ctx, a2).await, Some(b.place_a));

    // 2. перенос корня
    let v = group_version(&ctx, parent).await;
    ctx.groups
        .move_group(&admin(), parent, v, b.place_b)
        .await
        .expect("перенос");
    assert_eq!(assert_place_invariant(&ctx, "перенос корня").await, 2);
    assert_eq!(group_place(&ctx, a2).await, Some(b.place_b));

    // 3. вложение под родителя без места
    let v = group_version(&ctx, a).await;
    ctx.groups
        .set_parent(&admin(), a, v, Some(placeless), true)
        .await
        .expect("под без места");
    assert_eq!(assert_place_invariant(&ctx, "родитель без места").await, 2);
    assert_eq!(group_place(&ctx, a2).await, None);

    // 4. выход из родителя
    let v = group_version(&ctx, a).await;
    ctx.groups
        .set_parent(&admin(), a, v, None, false)
        .await
        .expect("выход");
    assert_eq!(assert_place_invariant(&ctx, "выход").await, 1);

    // 5. снова под родителя с местом и удаление родителя
    let v = group_version(&ctx, a).await;
    ctx.groups
        .set_parent(&admin(), a, v, Some(parent), false)
        .await
        .expect("вложение");
    assert_eq!(assert_place_invariant(&ctx, "повторное вложение").await, 2);
    ctx.groups
        .delete_group(&admin(), parent)
        .await
        .expect("удаление родителя");
    assert_eq!(group_parent(&ctx, a).await, None, "вложенная стала корнем");
    assert_eq!(assert_place_invariant(&ctx, "удаление родителя").await, 1);
}

// ---------------------------------------------------------------------------
// Phase 41.8 plan 03: модель типов, D-08, D-16, move_devices
// ---------------------------------------------------------------------------

/// Тип группы с заданным поведением; при отсутствии создаётся.
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
                params![format!("t_{behavior}"), format!("Тип {behavior}"), behavior],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("type by behavior")
}

async fn group_of_behavior(ctx: &AppCtx, behavior: &'static str, name: &str, seq: i64) -> i64 {
    let t = type_by_behavior(ctx, behavior).await;
    seed_group(ctx, t, name, seq, None, None).await
}

async fn set_origin(ctx: &AppCtx, device: i64, group: i64) {
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "UPDATE devices SET origin_group_id = ?2 WHERE id = ?1",
                params![device, group],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("set origin");
}

async fn set_anchor(ctx: &AppCtx, group: i64, device: i64) {
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "UPDATE groups SET anchor_device_id = ?2 WHERE id = ?1",
                params![group, device],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("set anchor");
}

async fn member_group(ctx: &AppCtx, device: i64) -> Option<i64> {
    opt_i64(
        ctx,
        "SELECT (SELECT group_id FROM group_devices WHERE device_id = ?1)",
        vec![int(device)],
    )
    .await
}

async fn audit_total(ctx: &AppCtx, action: &'static str) -> i64 {
    scalar_i64(
        ctx,
        "SELECT COUNT(*) FROM audit_log WHERE action = ?1",
        vec![rusqlite::types::Value::Text(action.to_string())],
    )
    .await
}

async fn plain_device(ctx: &AppCtx, name: &str, n: i64, group: Option<i64>) -> i64 {
    seed_device(
        ctx,
        name,
        &format!("INV-M-{n}"),
        &format!("SN-M-{n}"),
        None,
        group,
    )
    .await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn membership_model_container_and_substitute_accept_any_device() {
    // D-00: АРМ и «Системный блок» принимают любое устройство.
    let (ctx, _dir) = make_test_ctx().await;
    let arm = group_of_behavior(&ctx, "container", "АРМ #3", 3).await;
    let sys = group_of_behavior(&ctx, "substitute", "Системный блок #4", 4).await;
    let d1 = plain_device(&ctx, "Монитор #2", 1, None).await;
    let d2 = plain_device(&ctx, "SSD 256 ГБ", 2, None).await;
    ctx.groups
        .add_devices(&admin(), arm, vec![d1])
        .await
        .expect("в АРМ");
    ctx.groups
        .add_devices(&admin(), sys, vec![d2])
        .await
        .expect("в системный блок");
    assert_eq!(member_group(&ctx, d1).await, Some(arm));
    assert_eq!(member_group(&ctx, d2).await, Some(sys));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn membership_model_remove_allowed_for_any_behavior_but_anchor_is_guarded() {
    // D-01, D-00: вывод разрешён из любого типа, якорь не выводится.
    let (ctx, _dir) = make_test_ctx().await;
    for (i, behavior) in ["container", "substitute", "teardown"]
        .into_iter()
        .enumerate()
    {
        let g =
            group_of_behavior(&ctx, behavior, &format!("Группа #{}", i + 1), i as i64 + 1).await;
        let member = plain_device(&ctx, "Монитор #2", 10 + i as i64, Some(g)).await;
        let anchor = plain_device(&ctx, "Системный блок #4", 20 + i as i64, Some(g)).await;
        set_anchor(&ctx, g, anchor).await;
        let n = ctx
            .groups
            .remove_devices(&admin(), g, vec![member])
            .await
            .expect(behavior);
        assert_eq!(n, 1, "{behavior}: участник выведен");
        assert_eq!(member_group(&ctx, member).await, None);
        let msg = validation_message(
            ctx.groups
                .remove_devices(&admin(), g, vec![anchor])
                .await
                .expect_err("якорь"),
        );
        assert!(msg.contains("якорь"), "{behavior}: {msg}");
        assert_eq!(
            member_group(&ctx, anchor).await,
            Some(g),
            "{behavior}: якорь на месте"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn membership_add_all_occupied_devices_reported_in_one_refusal() {
    // D-16/D-18: все занятые одним отказом, ничего не записано.
    let (ctx, _dir) = make_test_ctx().await;
    let target = group_of_behavior(&ctx, "container", "АРМ #3", 3).await;
    let g1 = group_of_behavior(&ctx, "container", "АРМ #7", 7).await;
    let g2 = group_of_behavior(&ctx, "substitute", "Системный блок #8", 8).await;
    let free = plain_device(&ctx, "Ноутбук 1", 1, None).await;
    let busy1 = plain_device(&ctx, "Монитор #2", 2, Some(g1)).await;
    let busy2 = plain_device(&ctx, "SSD 256 ГБ", 3, Some(g2)).await;
    let err = ctx
        .groups
        .add_devices(&admin(), target, vec![busy1, free, busy2])
        .await
        .expect_err("занятые");
    assert_eq!(err.code(), "ALREADY_IN_GROUP");
    let AppError::AlreadyInGroup { items } = err else {
        panic!("не тот вариант")
    };
    assert_eq!(items.len(), 2);
    assert_eq!((items[0].id, items[0].group_id), (busy1, g1));
    assert_eq!((items[1].id, items[1].group_id), (busy2, g2));
    assert_eq!(items[0].group_name, "АРМ #7");
    assert_eq!(member_group(&ctx, free).await, None, "свободное не введено");
    assert_eq!(audit_total(&ctx, "add_devices").await, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn membership_teardown_add_origin_rules_with_distinct_fixtures() {
    // D-08, D-15: пустой и чужой origin отказывают, свой проходит (призрак).
    let (ctx, _dir) = make_test_ctx().await;
    let td = group_of_behavior(&ctx, "teardown", "Разбор #7", 7).await;
    let other_td = seed_group(
        &ctx,
        type_by_behavior(&ctx, "teardown").await,
        "Разбор #8",
        8,
        None,
        None,
    )
    .await;
    let empty = plain_device(&ctx, "SSD 256 ГБ", 1, None).await;
    let foreign = plain_device(&ctx, "Планка памяти", 2, None).await;
    set_origin(&ctx, foreign, other_td).await;
    let own = plain_device(&ctx, "Блок питания", 3, None).await;
    set_origin(&ctx, own, td).await;
    for dev in [empty, foreign] {
        let msg = validation_message(
            ctx.groups
                .add_devices(&admin(), td, vec![dev])
                .await
                .expect_err("чужое"),
        );
        assert!(msg.contains("Создать деталь"), "{msg}");
    }
    ctx.groups
        .add_devices(&admin(), td, vec![own])
        .await
        .expect("своя");
    assert_eq!(member_group(&ctx, own).await, Some(td));
    // Удалённая своя деталь не возвращается (D-15).
    let gone = plain_device(&ctx, "Кулер", 4, None).await;
    set_origin(&ctx, gone, td).await;
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "UPDATE devices SET deleted_at_utc = 1700000001 WHERE id = ?1",
                params![gone],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("soft delete");
    let msg = validation_message(
        ctx.groups
            .add_devices(&admin(), td, vec![gone])
            .await
            .expect_err("удалённая"),
    );
    assert!(msg.contains("Удалённую деталь"), "{msg}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn membership_move_devices_moves_with_single_audit_record() {
    let (ctx, _dir) = make_test_ctx().await;
    let from = group_of_behavior(&ctx, "container", "АРМ #3", 3).await;
    let to = group_of_behavior(&ctx, "container", "АРМ #5", 5).await;
    let occupied = plain_device(&ctx, "Монитор #2", 1, Some(from)).await;
    let free = plain_device(&ctx, "SSD 256 ГБ", 2, None).await;
    let released_before = audit_total(&ctx, "custom:group_member_released").await;
    let out = ctx
        .groups
        .move_devices(&admin(), to, vec![occupied, free])
        .await
        .expect("move");
    assert_eq!((out.added, out.moved), (2, 1));
    assert_eq!(member_group(&ctx, occupied).await, Some(to));
    assert_eq!(member_group(&ctx, free).await, Some(to));
    assert_eq!(
        audit_total(&ctx, "move_devices").await,
        1,
        "ровно одна запись"
    );
    assert_eq!(
        audit_total(&ctx, "custom:group_member_released").await,
        released_before + 1,
        "D-29: перенос освобождает через release_device_in_tx, одна запись на перенесённое"
    );
    assert_eq!(audit_total(&ctx, "add_devices").await, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn membership_move_devices_is_all_or_nothing() {
    let (ctx, _dir) = make_test_ctx().await;
    let from = group_of_behavior(&ctx, "container", "АРМ #3", 3).await;
    let to = group_of_behavior(&ctx, "container", "АРМ #5", 5).await;
    let ok = plain_device(&ctx, "Монитор #2", 1, Some(from)).await;
    // (а) второй — якорь исходной группы.
    let anchor_grp = group_of_behavior(&ctx, "substitute", "Системный блок #4", 4).await;
    let anchor = plain_device(&ctx, "Системный блок #9", 2, Some(anchor_grp)).await;
    set_anchor(&ctx, anchor_grp, anchor).await;
    let msg = validation_message(
        ctx.groups
            .move_devices(&admin(), to, vec![ok, anchor])
            .await
            .expect_err("якорь"),
    );
    assert!(msg.contains("якорь"), "{msg}");
    assert_eq!(
        member_group(&ctx, ok).await,
        Some(from),
        "первое не перенесено"
    );
    assert_eq!(member_group(&ctx, anchor).await, Some(anchor_grp));
    // (б) второй — Списано.
    let wo = plain_device(&ctx, "Клавиатура", 3, None).await;
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "UPDATE devices SET status_id = (SELECT id FROM device_statuses WHERE code = 'списано') WHERE id = ?1",
                params![wo],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("status");
    let msg = validation_message(
        ctx.groups
            .move_devices(&admin(), to, vec![ok, wo])
            .await
            .expect_err("списано"),
    );
    assert!(msg.contains("«Списано»"), "{msg}");
    assert_eq!(member_group(&ctx, ok).await, Some(from));
    assert_eq!(audit_total(&ctx, "move_devices").await, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn membership_move_devices_into_teardown_only_own_parts() {
    // D-17: сервер отказывает независимо от UI, перенос не обходит D-08; призрак переносится.
    let (ctx, _dir) = make_test_ctx().await;
    let td = group_of_behavior(&ctx, "teardown", "Разбор #7", 7).await;
    let arm = group_of_behavior(&ctx, "container", "АРМ #3", 3).await;
    let foreign = plain_device(&ctx, "Монитор #2", 1, Some(arm)).await;
    let msg = validation_message(
        ctx.groups
            .move_devices(&admin(), td, vec![foreign])
            .await
            .expect_err("чужое"),
    );
    assert!(msg.contains("Создать деталь"), "{msg}");
    assert_eq!(
        member_group(&ctx, foreign).await,
        Some(arm),
        "откат: членство цело"
    );
    let ghost = plain_device(&ctx, "SSD 256 ГБ", 2, Some(arm)).await;
    set_origin(&ctx, ghost, td).await;
    let out = ctx
        .groups
        .move_devices(&admin(), td, vec![ghost])
        .await
        .expect("призрак");
    assert_eq!((out.added, out.moved), (1, 1));
    assert_eq!(member_group(&ctx, ghost).await, Some(td));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn membership_move_devices_same_group_and_rights() {
    let (ctx, _dir) = make_test_ctx().await;
    let g = group_of_behavior(&ctx, "container", "АРМ #3", 3).await;
    let d = plain_device(&ctx, "Монитор #2", 1, Some(g)).await;
    let msg = validation_message(
        ctx.groups
            .move_devices(&admin(), g, vec![d])
            .await
            .expect_err("та же группа"),
    );
    assert_eq!(msg, "Устройство уже входит в эту группу.");
    let employee = create_identity(&ctx, "emp_move", "Петров П.П.", Role::Employee).await;
    let to = group_of_behavior(&ctx, "container", "АРМ #5", 5).await;
    let err = ctx
        .groups
        .move_devices(&employee, to, vec![d])
        .await
        .expect_err("employee");
    assert!(matches!(err, AppError::Forbidden), "{err:?}");
    assert_eq!(member_group(&ctx, d).await, Some(g));
}

// ---------------------------------------------------------------------------
// Phase 41.8 Plan 04: D-09 (запрет вложения «Разбора»), D-16 (смена родителя)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn nesting_teardown_group_under_another_is_rejected_by_service_only() {
    let (ctx, _dir) = make_test_ctx().await;
    let td = group_of_behavior(&ctx, "teardown", "Разбор #7", 7).await;
    let parent = group_of_behavior(&ctx, "container", "АРМ #3", 3).await;
    let v = group_version(&ctx, td).await;
    let msg = validation_message(
        ctx.groups
            .set_parent(&admin(), td, v, Some(parent), false)
            .await
            .expect_err("разбор нельзя вложить"),
    );
    assert_eq!(msg, "Группу типа «Разбор» нельзя вложить в другую группу.");
    // и с подтверждением смены родителя правило не снимается
    let msg = validation_message(
        ctx.groups
            .set_parent(&admin(), td, v, Some(parent), true)
            .await
            .expect_err("replace не снимает запрет"),
    );
    assert!(msg.contains("нельзя вложить"), "{msg}");
    assert_eq!(group_parent(&ctx, td).await, None);
    // вывод в корень разрешён
    ctx.groups
        .set_parent(&admin(), td, v, None, false)
        .await
        .expect("в корень");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn set_parent_of_nested_group_needs_replace_flag() {
    let (ctx, _dir) = make_test_ctx().await;
    let a = group_of_behavior(&ctx, "container", "АРМ #3", 3).await;
    let b = group_of_behavior(&ctx, "container", "АРМ #4", 4).await;
    let child = group_of_behavior(&ctx, "container", "АРМ #5", 5).await;
    let v = group_version(&ctx, child).await;
    ctx.groups
        .set_parent(&admin(), child, v, Some(a), false)
        .await
        .expect("первое вложение без родителя не требует флага");

    let v = group_version(&ctx, child).await;
    let err = ctx
        .groups
        .set_parent(&admin(), child, v, Some(b), false)
        .await
        .expect_err("уже в группе");
    match err {
        AppError::AlreadyInGroup { items } => {
            assert_eq!(items.len(), 1);
            assert_eq!(items[0].kind, "group");
            assert_eq!(items[0].id, child);
            assert_eq!(items[0].group_id, a);
            assert_eq!(items[0].group_name, "АРМ #3");
        }
        other => panic!("ожидали AlreadyInGroup, получили {other:?}"),
    }
    assert_eq!(group_parent(&ctx, child).await, Some(a), "не забрана");
    assert_eq!(group_version(&ctx, child).await, v);

    // тот же родитель — не смена; с флагом — успех
    ctx.groups
        .set_parent(&admin(), child, v, Some(b), true)
        .await
        .expect("с replace_parent");
    assert_eq!(group_parent(&ctx, child).await, Some(b));
}

async fn set_status_code(ctx: &AppCtx, device: i64, code: &'static str) {
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn d30_written_off_member_into_other_group_refuses_by_status_not_occupied() {
    // D-30: отказ по статусу раньше классификации «занято другой группой».
    let (ctx, _dir) = make_test_ctx().await;
    let arm3 = group_of_behavior(&ctx, "container", "АРМ #3", 3).await;
    let arm5 = group_of_behavior(&ctx, "container", "АРМ #5", 5).await;
    for (n, code, needle) in [
        (1, "списано", "«Списано»"),
        (2, "на_списание", "«На списание»"),
    ] {
        let dev = plain_device(&ctx, &format!("Клавиатура #{n}"), n, Some(arm3)).await;
        set_status_code(&ctx, dev, code).await;
        let err = ctx
            .groups
            .add_devices(&admin(), arm5, vec![dev])
            .await
            .expect_err("отказ по статусу");
        let msg = validation_message(err);
        assert!(msg.contains(needle), "{msg}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn d30_written_off_member_into_same_group_is_skipped_and_live_still_occupied() {
    let (ctx, _dir) = make_test_ctx().await;
    let arm3 = group_of_behavior(&ctx, "container", "АРМ #3", 3).await;
    let arm5 = group_of_behavior(&ctx, "container", "АРМ #5", 5).await;
    let dead = plain_device(&ctx, "Клавиатура #1", 1, Some(arm3)).await;
    set_status_code(&ctx, dead, "списано").await;
    ctx.groups
        .add_devices(&admin(), arm3, vec![dead])
        .await
        .expect("повторный ввод в ту же группу — Skipped");
    let live = plain_device(&ctx, "Мышь #2", 2, Some(arm3)).await;
    let err = ctx
        .groups
        .add_devices(&admin(), arm5, vec![live])
        .await
        .expect_err("живое занято");
    assert!(matches!(err, AppError::AlreadyInGroup { .. }), "{err:?}");
}

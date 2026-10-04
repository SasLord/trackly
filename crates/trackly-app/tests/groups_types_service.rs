//! Phase 41 Plan 07: `GroupTypeService` — типы групп, их свойства, засев,
//! правила защиты. Реальный `AppCtx` на временном каталоге; имена вымышленные.
//!
//! Префиксы тестов: `seed_`, `seed_props_`, `immutable_`, `delete_`, `rights_`,
//! `properties_`, `protect_`.
//!
//! Свойства, которые тест создаёт сам, он создаёт на пользовательском типе
//! (`create_type`): у «АРМ» уже засеяны пять свойств (D-31), одинаковые имена
//! дали бы отказ по уникальности.

use rusqlite::params;

use trackly_app::context::AppCtx;
use trackly_app::dto::auth::UserNew;
use trackly_app::dto::group_types::{
    GroupTypeCreateDto, GroupTypeDto, GroupTypeUpdateDto, PropertyCreateDto, PropertyUpdateDto,
};
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

fn int(v: i64) -> rusqlite::types::Value {
    rusqlite::types::Value::Integer(v)
}
fn text(v: &str) -> rusqlite::types::Value {
    rusqlite::types::Value::Text(v.to_string())
}

async fn type_by_code(ctx: &AppCtx, code: &str, include_archived: bool) -> GroupTypeDto {
    ctx.group_types
        .list_types(&admin(), include_archived)
        .await
        .expect("list_types")
        .into_iter()
        .find(|t| t.code == code)
        .unwrap_or_else(|| panic!("тип {code} не найден"))
}

async fn new_custom_type(ctx: &AppCtx, name: &str) -> GroupTypeDto {
    ctx.group_types
        .create_type(
            &admin(),
            GroupTypeCreateDto {
                name: name.to_string(),
                behavior: "container".to_string(),
            },
        )
        .await
        .expect("create_type")
}

/// Сеет живую группу типа `type_id` с порядковым номером `seq`.
async fn seed_group(ctx: &AppCtx, type_id: i64, name: &str, seq: i64) -> i64 {
    let name = name.to_string();
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO groups (type_id, name, seq, created_at_utc, updated_at_utc, version)
                 VALUES (?1, ?2, ?3, 1700000000, 1700000000, 1)",
                params![type_id, name, seq],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed group")
}

fn prop_create(type_id: i64, name: &str, data_type: &str) -> PropertyCreateDto {
    PropertyCreateDto {
        type_id,
        name: name.to_string(),
        data_type: data_type.to_string(),
        is_required: false,
        show_on_map: false,
    }
}

// ---------------------------------------------------------------------------
// seed_
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn seed_creates_three_builtin_types() {
    let (ctx, _dir) = make_test_ctx().await;
    let types = ctx.group_types.list_types(&admin(), false).await.unwrap();
    let got: Vec<(String, String, String, bool)> = types
        .iter()
        .map(|t| {
            (
                t.code.clone(),
                t.name.clone(),
                t.behavior.clone(),
                t.is_builtin,
            )
        })
        .collect();
    assert_eq!(
        got,
        vec![
            (
                "workstation".to_string(),
                "АРМ".to_string(),
                "container".to_string(),
                true
            ),
            (
                "system_unit".to_string(),
                "Системный блок".to_string(),
                "substitute".to_string(),
                true
            ),
            (
                "teardown".to_string(),
                "Разбор".to_string(),
                "teardown".to_string(),
                true
            ),
        ]
    );
    let labels: Vec<Option<String>> = types.iter().map(|t| t.quick_action_label.clone()).collect();
    assert_eq!(
        labels,
        vec![
            Some("Сформировать группу".to_string()),
            Some("Замещение группой".to_string()),
            Some("На разбор".to_string())
        ]
    );
    assert!(types.iter().all(|t| t.quick_action_enabled));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn seed_rename_survives_restart_and_reseed() {
    let (ctx, dir) = make_test_ctx().await;
    let before = type_by_code(&ctx, "workstation", false).await;
    ctx.group_types
        .update_type(
            &admin(),
            before.id,
            before.version,
            GroupTypeUpdateDto {
                name: Some("Рабочее место".to_string()),
                ..Default::default()
            },
        )
        .await
        .expect("rename builtin");

    ctx.group_types
        .seed_builtin_types_on_startup()
        .await
        .unwrap();
    let again = type_by_code(&ctx, "workstation", false).await;
    assert_eq!(again.name, "Рабочее место");
    assert_eq!(again.id, before.id);
    assert_eq!(
        scalar_i64(
            &ctx,
            "SELECT COUNT(*) FROM group_types WHERE code = ?1",
            vec![text("workstation")]
        )
        .await,
        1
    );

    drop(ctx);
    let ctx2 = build_ctx_in(dir.path()).await.expect("rebuild ctx");
    let after = type_by_code(&ctx2, "workstation", false).await;
    assert_eq!(after.name, "Рабочее место");
    assert_eq!(after.id, before.id);
    assert_eq!(
        scalar_i64(&ctx2, "SELECT COUNT(*) FROM group_types", vec![]).await,
        3
    );
}

// ---------------------------------------------------------------------------
// seed_props_ (D-31)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn seed_props_clean() {
    let (ctx, _dir) = make_test_ctx().await;
    let ws = type_by_code(&ctx, "workstation", false).await;
    let got: Vec<(String, String, bool, bool, i64)> = ws
        .properties
        .iter()
        .map(|p| {
            (
                p.name.clone(),
                p.data_type.clone(),
                p.show_on_map,
                p.is_required,
                p.sort_order,
            )
        })
        .collect();
    let expected: Vec<(String, String, bool, bool, i64)> = [
        ("Пользователи", "users", true),
        ("Подключённые принтеры", "device_refs", false),
        ("Хост", "text", false),
        ("IP", "ip", false),
        ("MAC", "mac", false),
    ]
    .iter()
    .enumerate()
    .map(|(i, (n, d, m))| (n.to_string(), d.to_string(), *m, false, i as i64))
    .collect();
    assert_eq!(got, expected);

    for code in ["system_unit", "teardown"] {
        let t = type_by_code(&ctx, code, true).await;
        assert!(t.properties.is_empty(), "{code}: свойств по умолчанию нет");
    }
    for (code, marker) in [("workstation", 1), ("system_unit", 0), ("teardown", 0)] {
        assert_eq!(
            scalar_i64(
                &ctx,
                "SELECT default_props_seeded FROM group_types WHERE code = ?1",
                vec![text(code)]
            )
            .await,
            marker,
            "маркер {code}"
        );
    }
}

async fn ws_prop_rows(ctx: &AppCtx) -> i64 {
    scalar_i64(
        ctx,
        "SELECT COUNT(*) FROM group_type_properties p JOIN group_types t ON t.id = p.type_id
         WHERE t.code = 'workstation'",
        vec![],
    )
    .await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn seed_props_rename() {
    let (ctx, dir) = make_test_ctx().await;
    let ws = type_by_code(&ctx, "workstation", false).await;
    let host = ws.properties.iter().find(|p| p.name == "Хост").unwrap();
    ctx.group_types
        .update_property(
            &admin(),
            host.id,
            host.version,
            PropertyUpdateDto {
                name: Some("Имя узла".to_string()),
                ..Default::default()
            },
        )
        .await
        .expect("rename property");

    ctx.group_types
        .seed_builtin_types_on_startup()
        .await
        .unwrap();
    drop(ctx);
    let ctx2 = build_ctx_in(dir.path()).await.expect("rebuild ctx");
    let ws = type_by_code(&ctx2, "workstation", true).await;
    assert_eq!(ws.properties.len(), 5);
    assert!(ws.properties.iter().any(|p| p.name == "Имя узла"));
    assert!(
        !ws.properties.iter().any(|p| p.name == "Хост"),
        "«Хост» не должен быть воссоздан"
    );
    assert_eq!(ws_prop_rows(&ctx2).await, 5);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn seed_props_hidden() {
    let (ctx, dir) = make_test_ctx().await;
    let ws = type_by_code(&ctx, "workstation", false).await;
    let ip = ws
        .properties
        .iter()
        .find(|p| p.name == "IP")
        .unwrap()
        .clone();
    let g = seed_group(&ctx, ws.id, "Стенд 1", 1).await;
    exec(
        &ctx,
        "INSERT INTO group_property_values (group_id, property_id, value_text, updated_at_utc)
         VALUES (?1, ?2, '10.0.0.1', 1700000000)",
        vec![int(g), int(ip.id)],
    )
    .await;
    let out = ctx
        .group_types
        .delete_property(&admin(), ip.id)
        .await
        .expect("delete filled property");
    assert!(out.archived);
    let rows_before = ws_prop_rows(&ctx).await;

    ctx.group_types
        .seed_builtin_types_on_startup()
        .await
        .unwrap();
    drop(ctx);
    let ctx2 = build_ctx_in(dir.path()).await.expect("rebuild ctx");

    let archived_at = scalar_i64(
        &ctx2,
        "SELECT archived_at_utc IS NOT NULL FROM group_type_properties WHERE id = ?1",
        vec![int(ip.id)],
    )
    .await;
    assert_eq!(archived_at, 1, "скрытое свойство остаётся скрытым");
    assert_eq!(
        scalar_i64(
            &ctx2,
            "SELECT COUNT(*) FROM group_type_properties WHERE name = 'IP'",
            vec![]
        )
        .await,
        1,
        "новых строк «IP» нет"
    );
    assert_eq!(ws_prop_rows(&ctx2).await, rows_before);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn seed_props_deleted() {
    let (ctx, dir) = make_test_ctx().await;
    let ws = type_by_code(&ctx, "workstation", false).await;
    let mac = ws.properties.iter().find(|p| p.name == "MAC").unwrap();
    let out = ctx
        .group_types
        .delete_property(&admin(), mac.id)
        .await
        .expect("delete empty property");
    assert!(!out.archived);

    ctx.group_types
        .seed_builtin_types_on_startup()
        .await
        .unwrap();
    drop(ctx);
    let ctx2 = build_ctx_in(dir.path()).await.expect("rebuild ctx");
    assert_eq!(
        scalar_i64(
            &ctx2,
            "SELECT COUNT(*) FROM group_type_properties WHERE name = 'MAC'",
            vec![]
        )
        .await,
        0,
        "удалённое «MAC» не возвращается"
    );
    assert_eq!(ws_prop_rows(&ctx2).await, 4);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn seed_props_custom() {
    let (ctx, dir) = make_test_ctx().await;
    let ws = type_by_code(&ctx, "workstation", false).await;
    let custom = ctx
        .group_types
        .create_property(&admin(), prop_create(ws.id, "Блок питания", "text"))
        .await
        .expect("create custom property");
    let order_before: Vec<(String, i64)> = type_by_code(&ctx, "workstation", false)
        .await
        .properties
        .iter()
        .map(|p| (p.name.clone(), p.sort_order))
        .collect();

    ctx.group_types
        .seed_builtin_types_on_startup()
        .await
        .unwrap();
    drop(ctx);
    let ctx2 = build_ctx_in(dir.path()).await.expect("rebuild ctx");
    let ws2 = type_by_code(&ctx2, "workstation", false).await;
    let order_after: Vec<(String, i64)> = ws2
        .properties
        .iter()
        .map(|p| (p.name.clone(), p.sort_order))
        .collect();
    assert_eq!(order_before, order_after);
    assert_eq!(
        ws2.properties
            .iter()
            .filter(|p| p.id == custom.id && p.name == "Блок питания")
            .count(),
        1
    );
    assert_eq!(ws2.properties.len(), 6);
}

// ---------------------------------------------------------------------------
// immutable_
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn immutable_code_rejected_and_row_unchanged() {
    let (ctx, _dir) = make_test_ctx().await;
    let ws = type_by_code(&ctx, "workstation", false).await;
    let err = ctx
        .group_types
        .update_type(
            &admin(),
            ws.id,
            ws.version,
            GroupTypeUpdateDto {
                code: Some("other_code".to_string()),
                ..Default::default()
            },
        )
        .await
        .expect_err("смена code должна быть отклонена");
    match err {
        AppError::Validation { field, message } => {
            assert_eq!(field, "code");
            assert_eq!(message, "Код и поведение типа после создания не меняются.");
        }
        other => panic!("ожидали Validation, получили {other:?}"),
    }
    let code = ctx
        .writer
        .execute(move |conn| {
            conn.query_row("SELECT code FROM group_types WHERE id = ?1", [ws.id], |r| {
                r.get::<_, String>(0)
            })
            .map_err(map_rusqlite)
        })
        .await
        .unwrap();
    assert_eq!(code, "workstation");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn immutable_behavior_rejected_and_row_unchanged() {
    let (ctx, _dir) = make_test_ctx().await;
    let ws = type_by_code(&ctx, "workstation", false).await;
    let err = ctx
        .group_types
        .update_type(
            &admin(),
            ws.id,
            ws.version,
            GroupTypeUpdateDto {
                behavior: Some("teardown".to_string()),
                ..Default::default()
            },
        )
        .await
        .expect_err("смена behavior должна быть отклонена");
    assert!(
        matches!(&err, AppError::Validation { field, .. } if field == "behavior"),
        "{err:?}"
    );
    assert_eq!(
        type_by_code(&ctx, "workstation", false).await.behavior,
        "container"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn immutable_same_code_and_new_name_ok() {
    let (ctx, _dir) = make_test_ctx().await;
    let ws = type_by_code(&ctx, "workstation", false).await;
    let out = ctx
        .group_types
        .update_type(
            &admin(),
            ws.id,
            ws.version,
            GroupTypeUpdateDto {
                name: Some("Рабочее место".to_string()),
                code: Some(ws.code.clone()),
                behavior: Some(ws.behavior.clone()),
                ..Default::default()
            },
        )
        .await
        .expect("тот же code/behavior и новое имя");
    assert_eq!(out.name, "Рабочее место");
    assert_eq!(out.code, "workstation");
    assert_eq!(out.version, ws.version + 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn immutable_create_with_unknown_behavior_rejected() {
    let (ctx, _dir) = make_test_ctx().await;
    let err = ctx
        .group_types
        .create_type(
            &admin(),
            GroupTypeCreateDto {
                name: "Стойка".to_string(),
                behavior: "flying".to_string(),
            },
        )
        .await
        .expect_err("неизвестное поведение");
    match err {
        AppError::Validation { field, message } => {
            assert_eq!(field, "behavior");
            assert!(message.contains("Допустимые значения"), "{message}");
        }
        other => panic!("ожидали Validation, получили {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// create / delete
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_type_generates_code_and_appends() {
    let (ctx, _dir) = make_test_ctx().await;
    let a = new_custom_type(&ctx, "Стойка").await;
    let b = new_custom_type(&ctx, "Шкаф").await;
    for t in [&a, &b] {
        let suffix = t.code.strip_prefix("custom_").expect("префикс custom_");
        assert_eq!(suffix.len(), 8);
        assert!(suffix.chars().all(|c| c.is_ascii_hexdigit()), "{}", t.code);
        assert!(!t.is_builtin);
    }
    assert_ne!(a.code, b.code);
    assert!(a.sort_order >= 3, "в конец после трёх встроенных");
    assert_eq!(b.sort_order, a.sort_order + 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_builtin_rejected() {
    let (ctx, _dir) = make_test_ctx().await;
    let ws = type_by_code(&ctx, "workstation", false).await;
    let err = ctx
        .group_types
        .delete_type(&admin(), ws.id)
        .await
        .expect_err("встроенный тип");
    match err {
        AppError::Validation { message, .. } => {
            assert!(
                message.starts_with("Встроенный тип удалить нельзя"),
                "{message}"
            );
        }
        other => panic!("ожидали Validation, получили {other:?}"),
    }
    assert_eq!(
        scalar_i64(
            &ctx,
            "SELECT COUNT(*) FROM group_types WHERE id = ?1",
            vec![int(ws.id)]
        )
        .await,
        1
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_custom_without_groups_ok() {
    let (ctx, _dir) = make_test_ctx().await;
    let t = new_custom_type(&ctx, "Стойка").await;
    ctx.group_types.delete_type(&admin(), t.id).await.unwrap();
    assert_eq!(
        scalar_i64(
            &ctx,
            "SELECT COUNT(*) FROM group_types WHERE id = ?1",
            vec![int(t.id)]
        )
        .await,
        0
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_custom_with_groups_rejected_with_count() {
    let (ctx, _dir) = make_test_ctx().await;
    let t = new_custom_type(&ctx, "Стойка").await;
    seed_group(&ctx, t.id, "Стойка А", 1).await;
    seed_group(&ctx, t.id, "Стойка Б", 2).await;
    let err = ctx
        .group_types
        .delete_type(&admin(), t.id)
        .await
        .expect_err("тип с группами");
    match err {
        AppError::Validation { message, .. } => {
            assert!(message.contains("2 группы"), "{message}");
        }
        other => panic!("ожидали Validation, получили {other:?}"),
    }
    assert_eq!(
        scalar_i64(
            &ctx,
            "SELECT COUNT(*) FROM group_types WHERE id = ?1",
            vec![int(t.id)]
        )
        .await,
        1
    );
}

// ---------------------------------------------------------------------------
// rights_
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rights_types_by_role() {
    let (ctx, _dir) = make_test_ctx().await;
    let manager = create_identity(&ctx, "mgr_ivanov", "Иванов И.И.", Role::Manager).await;
    let employee = create_identity(&ctx, "emp_petrov", "Петров П.П.", Role::Employee).await;
    let t = new_custom_type(&ctx, "Стойка").await;

    for who in [&manager, &employee] {
        assert!(matches!(
            ctx.group_types
                .create_type(
                    who,
                    GroupTypeCreateDto {
                        name: "Х".to_string(),
                        behavior: "container".to_string()
                    }
                )
                .await,
            Err(AppError::Forbidden)
        ));
        assert!(matches!(
            ctx.group_types
                .update_type(who, t.id, t.version, GroupTypeUpdateDto::default())
                .await,
            Err(AppError::Forbidden)
        ));
        assert!(matches!(
            ctx.group_types.delete_type(who, t.id).await,
            Err(AppError::Forbidden)
        ));
    }
    assert!(ctx.group_types.list_types(&manager, false).await.is_ok());
    assert!(matches!(
        ctx.group_types.list_types(&employee, false).await,
        Err(AppError::Forbidden)
    ));
}

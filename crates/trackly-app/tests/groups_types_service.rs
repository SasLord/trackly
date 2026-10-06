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
use trackly_app::dto::groups::GroupValueInputDto;
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

async fn group_name(ctx: &AppCtx, id: i64) -> String {
    ctx.writer
        .execute(move |conn| {
            conn.query_row("SELECT name FROM groups WHERE id = ?1", [id], |r| {
                r.get::<_, String>(0)
            })
            .map_err(map_rusqlite)
        })
        .await
        .expect("group name")
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

// ---------------------------------------------------------------------------
// properties_
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn properties_all_six_types_order_and_show_on_map() {
    let (ctx, _dir) = make_test_ctx().await;
    let t = new_custom_type(&ctx, "Стойка").await;
    let kinds = ["text", "number", "ip", "mac", "users", "device_refs"];
    let mut ids = Vec::new();
    for (i, k) in kinds.iter().enumerate() {
        let mut dto = prop_create(t.id, &format!("Свойство {k}"), k);
        dto.show_on_map = i == 4;
        let p = ctx
            .group_types
            .create_property(&admin(), dto)
            .await
            .unwrap_or_else(|e| panic!("create {k}: {e:?}"));
        assert_eq!(p.data_type, *k);
        assert_eq!(p.sort_order, i as i64);
        ids.push(p.id);
    }
    let listed = type_by_code(&ctx, &t.code, false).await;
    let got: Vec<String> = listed
        .properties
        .iter()
        .map(|p| p.data_type.clone())
        .collect();
    assert_eq!(got, kinds.iter().map(|s| s.to_string()).collect::<Vec<_>>());
    let on_map: Vec<bool> = listed.properties.iter().map(|p| p.show_on_map).collect();
    assert_eq!(on_map, vec![false, false, false, false, true, false]);

    // Перестановка: порядок разворачивается и виден в list_types.
    let reversed: Vec<i32> = ids.iter().rev().map(|i| *i as i32).collect();
    let out = ctx
        .group_types
        .reorder_properties(&admin(), t.id as i32, reversed.clone())
        .await
        .expect("reorder");
    assert_eq!(
        out.iter().map(|p| p.id as i32).collect::<Vec<_>>(),
        reversed
    );
    let after = type_by_code(&ctx, &t.code, false).await;
    assert_eq!(
        after
            .properties
            .iter()
            .map(|p| p.id as i32)
            .collect::<Vec<_>>(),
        reversed
    );

    let bad = ctx
        .group_types
        .create_property(&admin(), prop_create(t.id, "Дата", "date"))
        .await
        .expect_err("неизвестный data_type");
    assert!(matches!(bad, AppError::Validation { ref field, .. } if field == "data_type"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn properties_name_unique_case_insensitive_cyrillic() {
    let (ctx, _dir) = make_test_ctx().await;
    let t = new_custom_type(&ctx, "Стойка").await;
    let first = ctx
        .group_types
        .create_property(&admin(), prop_create(t.id, "Хост", "text"))
        .await
        .unwrap();
    let err = ctx
        .group_types
        .create_property(&admin(), prop_create(t.id, "хост", "text"))
        .await
        .expect_err("дубликат без учёта регистра");
    match err {
        AppError::Validation { message, .. } => {
            assert_eq!(message, "Свойство с таким названием уже есть.")
        }
        other => panic!("ожидали Validation, получили {other:?}"),
    }
    // Переименование в занятое имя тоже отклоняется.
    let second = ctx
        .group_types
        .create_property(&admin(), prop_create(t.id, "Узел", "text"))
        .await
        .unwrap();
    let err = ctx
        .group_types
        .update_property(
            &admin(),
            second.id,
            second.version,
            PropertyUpdateDto {
                name: Some("ХОСТ".to_string()),
                ..Default::default()
            },
        )
        .await
        .expect_err("переименование в занятое");
    assert!(matches!(err, AppError::Validation { ref field, .. } if field == "name"));

    // Скрытое имя можно занять живым; вернуть скрытое при занятом имени нельзя.
    let g = seed_group(&ctx, t.id, "Стенд", 1).await;
    exec(
        &ctx,
        "INSERT INTO group_property_values (group_id, property_id, value_text, updated_at_utc)
         VALUES (?1, ?2, 'node-1', 1700000000)",
        vec![int(g), int(first.id)],
    )
    .await;
    assert!(
        ctx.group_types
            .delete_property(&admin(), first.id)
            .await
            .unwrap()
            .archived
    );
    let reborn = ctx
        .group_types
        .create_property(&admin(), prop_create(t.id, "Хост", "text"))
        .await
        .expect("скрытое имя свободно");
    assert_ne!(reborn.id, first.id);
    let err = ctx
        .group_types
        .unarchive_property(&admin(), first.id)
        .await
        .expect_err("имя занято живым");
    match err {
        AppError::Validation { message, .. } => {
            assert_eq!(message, "Свойство с таким названием уже есть.")
        }
        other => panic!("ожидали Validation, получили {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn properties_cap_per_type() {
    let (ctx, _dir) = make_test_ctx().await;
    let t = new_custom_type(&ctx, "Стойка").await;
    for i in 0..50 {
        ctx.group_types
            .create_property(&admin(), prop_create(t.id, &format!("П{i}"), "text"))
            .await
            .unwrap_or_else(|e| panic!("создание {i}: {e:?}"));
    }
    let err = ctx
        .group_types
        .create_property(&admin(), prop_create(t.id, "Лишнее", "text"))
        .await
        .expect_err("51-е свойство");
    assert!(matches!(err, AppError::Validation { ref field, .. } if field == "type_id"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn properties_reorder_with_foreign_ids_rejected() {
    let (ctx, _dir) = make_test_ctx().await;
    let t = new_custom_type(&ctx, "Стойка").await;
    let other = new_custom_type(&ctx, "Шкаф").await;
    let a = ctx
        .group_types
        .create_property(&admin(), prop_create(t.id, "А", "text"))
        .await
        .unwrap();
    let b = ctx
        .group_types
        .create_property(&admin(), prop_create(t.id, "Б", "text"))
        .await
        .unwrap();
    let foreign = ctx
        .group_types
        .create_property(&admin(), prop_create(other.id, "Чужое", "text"))
        .await
        .unwrap();
    let err = ctx
        .group_types
        .reorder_properties(
            &admin(),
            t.id as i32,
            vec![b.id as i32, a.id as i32, foreign.id as i32],
        )
        .await
        .expect_err("смешанные id");
    assert!(matches!(err, AppError::Validation { ref field, .. } if field == "ordered_ids"));
    // Порядок не тронут.
    let now = type_by_code(&ctx, &t.code, false).await;
    assert_eq!(
        now.properties.iter().map(|p| p.id).collect::<Vec<_>>(),
        vec![a.id, b.id]
    );
}

// ---------------------------------------------------------------------------
// protect_a: удаление заполненного = скрытие (D-13)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn protect_a_filled_property_is_hidden_not_deleted() {
    let (ctx, _dir) = make_test_ctx().await;
    let t = new_custom_type(&ctx, "Стойка").await;
    let filled = ctx
        .group_types
        .create_property(&admin(), prop_create(t.id, "Заполненное", "text"))
        .await
        .unwrap();
    let empty = ctx
        .group_types
        .create_property(&admin(), prop_create(t.id, "Пустое", "text"))
        .await
        .unwrap();
    let g = seed_group(&ctx, t.id, "Стойка А", 1).await;
    exec(
        &ctx,
        "INSERT INTO group_property_values (group_id, property_id, value_text, updated_at_utc)
         VALUES (?1, ?2, 'значение', 1700000000)",
        vec![int(g), int(filled.id)],
    )
    .await;

    let out = ctx
        .group_types
        .delete_property(&admin(), filled.id)
        .await
        .unwrap();
    assert!(out.archived);
    let live = type_by_code(&ctx, &t.code, false).await;
    assert!(!live.properties.iter().any(|p| p.id == filled.id));
    let all = type_by_code(&ctx, &t.code, true).await;
    let hidden = all
        .properties
        .iter()
        .find(|p| p.id == filled.id)
        .expect("в списке со скрытыми");
    assert!(hidden.archived);
    assert_eq!(hidden.filled_group_count, 1);
    assert!(
        scalar_i64(
            &ctx,
            "SELECT COUNT(*) FROM group_property_values WHERE property_id = ?1",
            vec![int(filled.id)]
        )
        .await
            > 0,
        "значения должны остаться в БД"
    );

    // Пустое — физическое удаление.
    let out = ctx
        .group_types
        .delete_property(&admin(), empty.id)
        .await
        .unwrap();
    assert!(!out.archived);
    assert_eq!(
        scalar_i64(
            &ctx,
            "SELECT COUNT(*) FROM group_type_properties WHERE id = ?1",
            vec![int(empty.id)]
        )
        .await,
        0
    );

    // Скрытое можно вернуть.
    let back = ctx
        .group_types
        .unarchive_property(&admin(), filled.id)
        .await
        .unwrap();
    assert!(!back.archived);
    assert!(type_by_code(&ctx, &t.code, false)
        .await
        .properties
        .iter()
        .any(|p| p.id == filled.id));
}

// ---------------------------------------------------------------------------
// protect_b: смена data_type заполненного свойства (D-16)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn protect_b_data_type_change_rejected_when_filled() {
    let (ctx, _dir) = make_test_ctx().await;
    let t = new_custom_type(&ctx, "Стойка").await;
    let filled = ctx
        .group_types
        .create_property(&admin(), prop_create(t.id, "Заполненное", "text"))
        .await
        .unwrap();
    let empty = ctx
        .group_types
        .create_property(&admin(), prop_create(t.id, "Пустое", "text"))
        .await
        .unwrap();
    let g1 = seed_group(&ctx, t.id, "Стойка А", 1).await;
    let g2 = seed_group(&ctx, t.id, "Стойка Б", 2).await;
    for g in [g1, g2] {
        exec(
            &ctx,
            "INSERT INTO group_property_values (group_id, property_id, value_text, updated_at_utc)
             VALUES (?1, ?2, 'v', 1700000000)",
            vec![int(g), int(filled.id)],
        )
        .await;
    }
    let n = scalar_i64(
        &ctx,
        "SELECT COUNT(DISTINCT group_id) FROM group_property_values WHERE property_id = ?1",
        vec![int(filled.id)],
    )
    .await;
    assert_eq!(n, 2);

    let err = ctx
        .group_types
        .update_property(
            &admin(),
            filled.id,
            filled.version,
            PropertyUpdateDto {
                data_type: Some("number".to_string()),
                ..Default::default()
            },
        )
        .await
        .expect_err("смена типа заполненного");
    match err {
        AppError::Validation { field, message } => {
            assert_eq!(field, "data_type");
            assert!(
                message.contains(&format!("заполнено в {n} группах")),
                "{message}"
            );
            assert!(message.contains("Создайте новое свойство."), "{message}");
        }
        other => panic!("ожидали Validation, получили {other:?}"),
    }
    assert_eq!(
        type_by_code(&ctx, &t.code, false)
            .await
            .properties
            .iter()
            .find(|p| p.id == filled.id)
            .unwrap()
            .data_type,
        "text"
    );

    // Тот же data_type у заполненного — не смена.
    let same = ctx
        .group_types
        .update_property(
            &admin(),
            filled.id,
            filled.version,
            PropertyUpdateDto {
                data_type: Some("text".to_string()),
                name: Some("Заполненное 2".to_string()),
                ..Default::default()
            },
        )
        .await
        .expect("тот же тип данных");
    assert_eq!(same.name, "Заполненное 2");
    assert_eq!(same.filled_group_count, 2);

    // У пустого смена разрешена.
    let changed = ctx
        .group_types
        .update_property(
            &admin(),
            empty.id,
            empty.version,
            PropertyUpdateDto {
                data_type: Some("number".to_string()),
                ..Default::default()
            },
        )
        .await
        .expect("пустое свойство");
    assert_eq!(changed.data_type, "number");
}

// ---------------------------------------------------------------------------
// protect_c: обязательность (D-14)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn protect_c_required_rejected_names_violators() {
    let (ctx, _dir) = make_test_ctx().await;
    let t = new_custom_type(&ctx, "Стойка").await;
    let p = ctx
        .group_types
        .create_property(&admin(), prop_create(t.id, "Хост", "text"))
        .await
        .unwrap();
    let g1 = seed_group(&ctx, t.id, "Стойка Альфа", 1).await;
    let g2 = seed_group(&ctx, t.id, "Стойка Бета", 2).await;
    // Имена нарушителей читаем из БД, а не хардкодим.
    let (n1, n2) = (group_name(&ctx, g1).await, group_name(&ctx, g2).await);

    let err = ctx
        .group_types
        .update_property(
            &admin(),
            p.id,
            p.version,
            PropertyUpdateDto {
                is_required: Some(true),
                ..Default::default()
            },
        )
        .await
        .expect_err("есть группы без значения");
    match err {
        AppError::Validation { field, message } => {
            assert_eq!(field, "is_required");
            assert!(message.contains(&n1), "{message}");
            assert!(message.contains(&n2), "{message}");
        }
        other => panic!("ожидали Validation, получили {other:?}"),
    }

    let listed = ctx
        .group_types
        .empty_groups_for_property(&admin(), p.id)
        .await
        .unwrap();
    let got: Vec<(i64, String)> = listed.into_iter().map(|g| (g.id, g.name)).collect();
    assert_eq!(got, vec![(g1, n1.clone()), (g2, n2.clone())]);

    // После заполнения у первой отказ называет только вторую.
    exec(
        &ctx,
        "INSERT INTO group_property_values (group_id, property_id, value_text, updated_at_utc)
         VALUES (?1, ?2, 'host-1', 1700000000)",
        vec![int(g1), int(p.id)],
    )
    .await;
    let err = ctx
        .group_types
        .update_property(
            &admin(),
            p.id,
            p.version,
            PropertyUpdateDto {
                is_required: Some(true),
                ..Default::default()
            },
        )
        .await
        .expect_err("осталась вторая");
    match err {
        AppError::Validation { message, .. } => {
            assert!(message.contains(&n2), "{message}");
            assert!(!message.contains(&n1), "{message}");
        }
        other => panic!("ожидали Validation, получили {other:?}"),
    }

    // Нарушителей нет — успех; снятие флага всегда разрешено.
    exec(
        &ctx,
        "INSERT INTO group_property_values (group_id, property_id, value_text, updated_at_utc)
         VALUES (?1, ?2, 'host-2', 1700000000)",
        vec![int(g2), int(p.id)],
    )
    .await;
    let on = ctx
        .group_types
        .update_property(
            &admin(),
            p.id,
            p.version,
            PropertyUpdateDto {
                is_required: Some(true),
                ..Default::default()
            },
        )
        .await
        .expect("нарушителей нет");
    assert!(on.is_required);
    let off = ctx
        .group_types
        .update_property(
            &admin(),
            p.id,
            on.version,
            PropertyUpdateDto {
                is_required: Some(false),
                ..Default::default()
            },
        )
        .await
        .expect("снятие флага");
    assert!(!off.is_required);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn protect_c_create_required_with_existing_empty_groups_rejected() {
    let (ctx, _dir) = make_test_ctx().await;
    let t = new_custom_type(&ctx, "Стойка").await;
    seed_group(&ctx, t.id, "Стойка Альфа", 1).await;
    let mut dto = prop_create(t.id, "Хост", "text");
    dto.is_required = true;
    let err = ctx
        .group_types
        .create_property(&admin(), dto)
        .await
        .expect_err("обязательное при существующих пустых группах");
    assert!(matches!(err, AppError::Validation { ref field, .. } if field == "is_required"));
    assert_eq!(
        scalar_i64(
            &ctx,
            "SELECT COUNT(*) FROM group_type_properties WHERE type_id = ?1",
            vec![int(t.id)]
        )
        .await,
        0,
        "отказ не оставляет свойство"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn protect_c_hidden_property_has_no_violators() {
    let (ctx, _dir) = make_test_ctx().await;
    let t = new_custom_type(&ctx, "Стойка").await;
    let p = ctx
        .group_types
        .create_property(&admin(), prop_create(t.id, "Хост", "text"))
        .await
        .unwrap();
    let g = seed_group(&ctx, t.id, "Стойка Альфа", 1).await;
    exec(
        &ctx,
        "INSERT INTO group_property_values (group_id, property_id, value_text, updated_at_utc)
         VALUES (?1, ?2, 'h', 1700000000)",
        vec![int(g), int(p.id)],
    )
    .await;
    seed_group(&ctx, t.id, "Стойка Бета", 2).await;
    ctx.group_types
        .delete_property(&admin(), p.id)
        .await
        .unwrap();
    assert!(ctx
        .group_types
        .empty_groups_for_property(&admin(), p.id)
        .await
        .unwrap()
        .is_empty());
}

// ---------------------------------------------------------------------------
// protect_d: обход «обязательного» через скрытие (W-B01)
// ---------------------------------------------------------------------------

fn text_value(property_id: i64, text: &str) -> GroupValueInputDto {
    GroupValueInputDto {
        property_id,
        text: Some(text.to_string()),
        refs: vec![],
    }
}

async fn group_version_of(ctx: &AppCtx, id: i64) -> i64 {
    scalar_i64(
        ctx,
        "SELECT version FROM groups WHERE id = ?1",
        vec![int(id)],
    )
    .await
}

/// Ровно три шага из W-B01: скрыть -> отметить обязательным -> показать.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn protect_d_hide_then_require_is_rejected() {
    let (ctx, _dir) = make_test_ctx().await;
    let t = new_custom_type(&ctx, "Стойка").await;
    let host = ctx
        .group_types
        .create_property(&admin(), prop_create(t.id, "Хост", "text"))
        .await
        .unwrap();
    let label = ctx
        .group_types
        .create_property(&admin(), prop_create(t.id, "Метка", "text"))
        .await
        .unwrap();
    let alfa = seed_group(&ctx, t.id, "Стойка Альфа", 1).await;
    let beta = seed_group(&ctx, t.id, "Стойка Бета", 2).await;
    exec(
        &ctx,
        "INSERT INTO group_property_values (group_id, property_id, value_text, updated_at_utc)
         VALUES (?1, ?2, 'h-01', 1700000000)",
        vec![int(alfa), int(host.id)],
    )
    .await;

    // Шаг 1: скрыть (значение есть -> скрытие, а не удаление).
    let out = ctx
        .group_types
        .delete_property(&admin(), host.id)
        .await
        .unwrap();
    assert!(out.archived);

    // Шаг 2: отметить скрытое обязательным — отказ сервера.
    let hidden = type_by_code(&ctx, &t.code, true)
        .await
        .properties
        .into_iter()
        .find(|p| p.id == host.id)
        .expect("скрытое свойство в списке со скрытыми");
    let err = ctx
        .group_types
        .update_property(
            &admin(),
            host.id,
            hidden.version,
            PropertyUpdateDto {
                is_required: Some(true),
                ..Default::default()
            },
        )
        .await
        .expect_err("скрытое свойство нельзя сделать обязательным");
    match err {
        AppError::Validation { field, message } => {
            assert_eq!(field, "is_required");
            assert!(
                message.contains("Скрытое свойство нельзя сделать обязательным"),
                "{message}"
            );
        }
        other => panic!("ожидали Validation, получили {other:?}"),
    }

    // Шаг 3: показать — свойство не обязательное, правка несвязанного поля проходит.
    let back = ctx
        .group_types
        .unarchive_property(&admin(), host.id)
        .await
        .unwrap();
    assert!(!back.is_required);
    let ver = group_version_of(&ctx, beta).await;
    ctx.groups
        .set_values(&admin(), beta, ver, vec![text_value(label.id, "м-1")])
        .await
        .expect("set_values по несвязанному полю не должен падать на «Хост»");
}

/// Вторая дорога: обязательное -> скрыть -> создать группу -> показать.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn protect_d_hiding_required_property_clears_flag() {
    let (ctx, _dir) = make_test_ctx().await;
    let t = new_custom_type(&ctx, "Стойка").await;
    let host = ctx
        .group_types
        .create_property(&admin(), prop_create(t.id, "Хост", "text"))
        .await
        .unwrap();
    let label = ctx
        .group_types
        .create_property(&admin(), prop_create(t.id, "Метка", "text"))
        .await
        .unwrap();
    let alfa = seed_group(&ctx, t.id, "Стойка Альфа", 1).await;
    exec(
        &ctx,
        "INSERT INTO group_property_values (group_id, property_id, value_text, updated_at_utc)
         VALUES (?1, ?2, 'h-01', 1700000000)",
        vec![int(alfa), int(host.id)],
    )
    .await;

    // Нарушителей нет — обязательным сделать можно.
    ctx.group_types
        .update_property(
            &admin(),
            host.id,
            host.version,
            PropertyUpdateDto {
                is_required: Some(true),
                ..Default::default()
            },
        )
        .await
        .expect("нарушителей нет");

    let out = ctx
        .group_types
        .delete_property(&admin(), host.id)
        .await
        .unwrap();
    assert!(out.archived);
    let hidden = type_by_code(&ctx, &t.code, true)
        .await
        .properties
        .into_iter()
        .find(|p| p.id == host.id)
        .expect("скрытое свойство в списке со скрытыми");
    assert!(hidden.archived);
    assert!(!hidden.is_required, "скрытое свойство не обязательное");

    // Пока свойство скрыто, появляется группа без значения.
    let beta = seed_group(&ctx, t.id, "Стойка Бета", 2).await;
    let back = ctx
        .group_types
        .unarchive_property(&admin(), host.id)
        .await
        .expect("возврат не заперт нарушителями");
    assert!(!back.is_required);
    let ver = group_version_of(&ctx, beta).await;
    ctx.groups
        .set_values(&admin(), beta, ver, vec![text_value(label.id, "м-1")])
        .await
        .expect("set_values по несвязанному полю проходит");

    // След в аудите: состояние ДО скрытия содержит is_required=true.
    assert_eq!(
        scalar_i64(
            &ctx,
            "SELECT COUNT(*) FROM audit_log
             WHERE entity_type = 'group_type_property' AND action = 'archive'
               AND entity_id = ?1 AND before_json LIKE '%\"is_required\":true%'",
            vec![int(host.id)]
        )
        .await,
        1
    );
}

/// Ревью WR-01: наследное «скрыто + обязательное» (строка создана до плана 41-29)
/// возвращается из скрытых и при этом теряет обязательность — отказ запирал
/// пользователя без выхода.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn protect_d_unarchive_clears_legacy_required_flag() {
    let (ctx, _dir) = make_test_ctx().await;
    let t = new_custom_type(&ctx, "Стойка").await;
    let host = ctx
        .group_types
        .create_property(&admin(), prop_create(t.id, "Хост", "text"))
        .await
        .unwrap();
    let label = ctx
        .group_types
        .create_property(&admin(), prop_create(t.id, "Метка", "text"))
        .await
        .unwrap();
    let alfa = seed_group(&ctx, t.id, "Стойка Альфа", 1).await;
    let beta = seed_group(&ctx, t.id, "Стойка Бета", 2).await;
    exec(
        &ctx,
        "INSERT INTO group_property_values (group_id, property_id, value_text, updated_at_utc)
         VALUES (?1, ?2, 'h-01', 1700000000)",
        vec![int(alfa), int(host.id)],
    )
    .await;
    // Наследное состояние создаётся напрямую в БД (в обход сервиса).
    exec(
        &ctx,
        "UPDATE group_type_properties SET archived_at_utc = 1700000100, is_required = 1
         WHERE id = ?1",
        vec![int(host.id)],
    )
    .await;

    // Ревью WR-01: возврат НЕ отказывает, а нормализует наследное состояние —
    // иначе пользователь в тупике (скрытое свойство не заполнить, в меню строки
    // только «Показать»).
    let out = ctx
        .group_types
        .unarchive_property(&admin(), host.id)
        .await
        .expect("возврат наследной строки «скрыто + обязательное» не должен отказывать");

    // Свойство живое и уже НЕ обязательное.
    assert!(!out.archived, "свойство должно стать живым");
    assert!(
        !out.is_required,
        "возврат из скрытых обязан снять обязательность (симметрия со скрытием)"
    );
    let live = type_by_code(&ctx, &t.code, false).await;
    let row = live
        .properties
        .iter()
        .find(|p| p.id == host.id)
        .expect("свойство видно среди живых");
    assert!(!row.is_required, "в выдаче типа флаг тоже снят");

    // Настоящая гарантия W-B01: «Стойка Бета» значения по «Хост» не имеет, но
    // живого ОБЯЗАТЕЛЬНОГО свойства без значения не существует, поэтому правка
    // несвязанного поля проходит. Именно это и падало до закрытия W-B01.
    let ver = group_version_of(&ctx, beta).await;
    ctx.groups
        .set_values(&admin(), beta, ver, vec![text_value(label.id, "м-1")])
        .await
        .expect("set_values по несвязанному полю не должен падать на «Хост»");
}

// ---------------------------------------------------------------------------
// rights_properties_
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rights_properties_by_role() {
    let (ctx, _dir) = make_test_ctx().await;
    let manager = create_identity(&ctx, "mgr_ivanov", "Иванов И.И.", Role::Manager).await;
    let employee = create_identity(&ctx, "emp_petrov", "Петров П.П.", Role::Employee).await;
    let t = new_custom_type(&ctx, "Стойка").await;
    let p = ctx
        .group_types
        .create_property(&admin(), prop_create(t.id, "Хост", "text"))
        .await
        .unwrap();

    for who in [&manager, &employee] {
        assert!(matches!(
            ctx.group_types
                .create_property(who, prop_create(t.id, "Х", "text"))
                .await,
            Err(AppError::Forbidden)
        ));
        assert!(matches!(
            ctx.group_types
                .update_property(who, p.id, p.version, PropertyUpdateDto::default())
                .await,
            Err(AppError::Forbidden)
        ));
        assert!(matches!(
            ctx.group_types.delete_property(who, p.id).await,
            Err(AppError::Forbidden)
        ));
        assert!(matches!(
            ctx.group_types.unarchive_property(who, p.id).await,
            Err(AppError::Forbidden)
        ));
        assert!(matches!(
            ctx.group_types
                .reorder_properties(who, t.id as i32, vec![p.id as i32])
                .await,
            Err(AppError::Forbidden)
        ));
    }
    assert!(ctx
        .group_types
        .empty_groups_for_property(&manager, p.id)
        .await
        .is_ok());
    assert!(matches!(
        ctx.group_types
            .empty_groups_for_property(&employee, p.id)
            .await,
        Err(AppError::Forbidden)
    ));
}

// ---------------------------------------------------------------------------
// atomic: мутация и аудит — одна транзакция (W-B02)
// ---------------------------------------------------------------------------
//
// Приём: триггер обрывает вставку в `audit_log` для сущностей типов/свойств.
// Мутация обязана вернуть ошибку И не оставить следов; после снятия триггера
// та же мутация проходит (триггер — единственная причина отказа).

async fn install_audit_fault(ctx: &AppCtx) {
    ctx.writer
        .execute(move |conn| {
            conn.execute_batch(
                "CREATE TRIGGER audit_fault BEFORE INSERT ON audit_log \
                 WHEN NEW.entity_type IN ('group_type', 'group_type_property') \
                 BEGIN SELECT RAISE(ABORT, 'audit down'); END;",
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("install trigger");
}

async fn remove_audit_fault(ctx: &AppCtx) {
    exec(ctx, "DROP TRIGGER audit_fault", vec![]).await;
}

/// Ошибка обязана прийти от триггера, а не от иной причины.
fn assert_audit_fault<T: std::fmt::Debug>(res: Result<T, AppError>, what: &str) {
    let err = res.expect_err(what);
    assert!(
        format!("{err:?}").contains("audit down"),
        "{what}: ошибка должна быть от триггера аудита: {err:?}"
    );
}

async fn count_types(ctx: &AppCtx) -> i64 {
    scalar_i64(ctx, "SELECT COUNT(*) FROM group_types", vec![]).await
}

async fn count_props(ctx: &AppCtx, type_id: i64) -> i64 {
    scalar_i64(
        ctx,
        "SELECT COUNT(*) FROM group_type_properties WHERE type_id = ?1",
        vec![int(type_id)],
    )
    .await
}

async fn type_name_version_matches(ctx: &AppCtx, id: i64, name: &str, version: i64) -> i64 {
    scalar_i64(
        ctx,
        "SELECT COUNT(*) FROM group_types WHERE id = ?1 AND name = ?2 AND version = ?3",
        vec![int(id), text(name), int(version)],
    )
    .await
}

async fn type_exists(ctx: &AppCtx, id: i64) -> i64 {
    scalar_i64(
        ctx,
        "SELECT COUNT(*) FROM group_types WHERE id = ?1",
        vec![int(id)],
    )
    .await
}

async fn prop_matches(ctx: &AppCtx, id: i64, name: &str, req: i64, ver: i64) -> i64 {
    scalar_i64(
        ctx,
        "SELECT COUNT(*) FROM group_type_properties
         WHERE id = ?1 AND name = ?2 AND is_required = ?3 AND version = ?4",
        vec![int(id), text(name), int(req), int(ver)],
    )
    .await
}

async fn prop_is_live(ctx: &AppCtx, id: i64) -> i64 {
    scalar_i64(
        ctx,
        "SELECT COUNT(*) FROM group_type_properties
         WHERE id = ?1 AND archived_at_utc IS NULL",
        vec![int(id)],
    )
    .await
}

async fn prop_sort_order(ctx: &AppCtx, id: i64) -> i64 {
    scalar_i64(
        ctx,
        "SELECT sort_order FROM group_type_properties WHERE id = ?1",
        vec![int(id)],
    )
    .await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn atomic_create_type() {
    let (ctx, _dir) = make_test_ctx().await;
    let before = count_types(&ctx).await;
    install_audit_fault(&ctx).await;
    let res = ctx
        .group_types
        .create_type(
            &admin(),
            GroupTypeCreateDto {
                name: "Стойка Альфа".to_string(),
                behavior: "container".to_string(),
            },
        )
        .await;
    assert_audit_fault(res, "аудит оборван");
    assert_eq!(count_types(&ctx).await, before, "тип не должен появиться");

    remove_audit_fault(&ctx).await;
    ctx.group_types
        .create_type(
            &admin(),
            GroupTypeCreateDto {
                name: "Стойка Альфа".to_string(),
                behavior: "container".to_string(),
            },
        )
        .await
        .expect("без триггера создание проходит");
    assert_eq!(count_types(&ctx).await, before + 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn atomic_update_type() {
    let (ctx, _dir) = make_test_ctx().await;
    let t = new_custom_type(&ctx, "Стойка Альфа").await;
    let rename = || GroupTypeUpdateDto {
        name: Some("Стойка Бета".to_string()),
        ..Default::default()
    };
    install_audit_fault(&ctx).await;
    let res = ctx
        .group_types
        .update_type(&admin(), t.id, t.version, rename())
        .await;
    assert_audit_fault(res, "аудит оборван");
    assert_eq!(
        type_name_version_matches(&ctx, t.id, "Стойка Альфа", t.version).await,
        1,
        "имя и version прежние"
    );

    remove_audit_fault(&ctx).await;
    ctx.group_types
        .update_type(&admin(), t.id, t.version, rename())
        .await
        .expect("без триггера правка проходит");
    assert_eq!(
        type_name_version_matches(&ctx, t.id, "Стойка Бета", t.version + 1).await,
        1,
        "новое имя и version + 1"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn atomic_delete_type() {
    let (ctx, _dir) = make_test_ctx().await;
    let t = new_custom_type(&ctx, "Стойка Альфа").await;
    install_audit_fault(&ctx).await;
    let res = ctx.group_types.delete_type(&admin(), t.id).await;
    assert_audit_fault(res, "аудит оборван");
    assert_eq!(type_exists(&ctx, t.id).await, 1, "тип остался");

    remove_audit_fault(&ctx).await;
    ctx.group_types
        .delete_type(&admin(), t.id)
        .await
        .expect("без триггера удаление проходит");
    assert_eq!(type_exists(&ctx, t.id).await, 0, "тип удалён");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn atomic_create_property() {
    let (ctx, _dir) = make_test_ctx().await;
    let t = new_custom_type(&ctx, "Стойка Альфа").await;
    let before = count_props(&ctx, t.id).await;
    install_audit_fault(&ctx).await;
    let res = ctx
        .group_types
        .create_property(&admin(), prop_create(t.id, "Метка", "text"))
        .await;
    assert_audit_fault(res, "аудит оборван");
    assert_eq!(count_props(&ctx, t.id).await, before, "свойство не выросло");

    remove_audit_fault(&ctx).await;
    ctx.group_types
        .create_property(&admin(), prop_create(t.id, "Метка", "text"))
        .await
        .expect("без триггера создание проходит");
    assert_eq!(count_props(&ctx, t.id).await, before + 1);
}

/// ОХРАННЫЙ тест (без триггера): ветка `want_required` с существующей пустой
/// группой отказывает по нарушителям и не оставляет свойство. Зелёный и до,
/// и после правки (раньше откат делала ручная компенсация, теперь — транзакция):
/// его роль — не дать потерять откат при удалении компенсации.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn atomic_create_property_required_violation_leaves_nothing() {
    let (ctx, _dir) = make_test_ctx().await;
    let t = new_custom_type(&ctx, "Стойка Альфа").await;
    seed_group(&ctx, t.id, "Стойка Бета", 1).await;
    let mut dto = prop_create(t.id, "Хост", "text");
    dto.is_required = true;
    let err = ctx
        .group_types
        .create_property(&admin(), dto)
        .await
        .expect_err("есть нарушители обязательности");
    assert!(matches!(err, AppError::Validation { ref field, .. } if field == "is_required"));
    assert_eq!(count_props(&ctx, t.id).await, 0, "свойство не осталось");
    assert_eq!(
        scalar_i64(
            &ctx,
            "SELECT COUNT(*) FROM audit_log WHERE entity_type = 'group_type_property'",
            vec![]
        )
        .await,
        0,
        "аудита нет"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn atomic_update_property() {
    let (ctx, _dir) = make_test_ctx().await;
    let t = new_custom_type(&ctx, "Стойка Альфа").await;
    let p = ctx
        .group_types
        .create_property(&admin(), prop_create(t.id, "Метка", "text"))
        .await
        .unwrap();
    let patch = || PropertyUpdateDto {
        name: Some("Хост".to_string()),
        is_required: Some(true),
        ..Default::default()
    };
    install_audit_fault(&ctx).await;
    let res = ctx
        .group_types
        .update_property(&admin(), p.id, p.version, patch())
        .await;
    assert_audit_fault(res, "аудит оборван");
    assert_eq!(
        prop_matches(&ctx, p.id, "Метка", 0, p.version).await,
        1,
        "имя, is_required, version прежние"
    );

    remove_audit_fault(&ctx).await;
    ctx.group_types
        .update_property(&admin(), p.id, p.version, patch())
        .await
        .expect("без триггера правка проходит");
    assert_eq!(prop_matches(&ctx, p.id, "Хост", 1, p.version + 1).await, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn atomic_delete_property_hard() {
    let (ctx, _dir) = make_test_ctx().await;
    let t = new_custom_type(&ctx, "Стойка Альфа").await;
    let p = ctx
        .group_types
        .create_property(&admin(), prop_create(t.id, "Метка", "text"))
        .await
        .unwrap();
    install_audit_fault(&ctx).await;
    let res = ctx.group_types.delete_property(&admin(), p.id).await;
    assert_audit_fault(res, "аудит оборван");
    assert_eq!(count_props(&ctx, t.id).await, 1, "свойство осталось");

    remove_audit_fault(&ctx).await;
    let out = ctx
        .group_types
        .delete_property(&admin(), p.id)
        .await
        .expect("без триггера удаление проходит");
    assert!(!out.archived);
    assert_eq!(count_props(&ctx, t.id).await, 0, "удалено физически");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn atomic_delete_property_archive() {
    let (ctx, _dir) = make_test_ctx().await;
    let t = new_custom_type(&ctx, "Стойка Альфа").await;
    let p = ctx
        .group_types
        .create_property(&admin(), prop_create(t.id, "Метка", "text"))
        .await
        .unwrap();
    let g = seed_group(&ctx, t.id, "Стойка Бета", 1).await;
    exec(
        &ctx,
        "INSERT INTO group_property_values (group_id, property_id, value_text, updated_at_utc)
         VALUES (?1, ?2, 'м-01', 1700000000)",
        vec![int(g), int(p.id)],
    )
    .await;
    install_audit_fault(&ctx).await;
    let res = ctx.group_types.delete_property(&admin(), p.id).await;
    assert_audit_fault(res, "аудит оборван");
    assert_eq!(prop_is_live(&ctx, p.id).await, 1, "свойство осталось живым");

    remove_audit_fault(&ctx).await;
    let out = ctx
        .group_types
        .delete_property(&admin(), p.id)
        .await
        .expect("без триггера скрытие проходит");
    assert!(out.archived);
    assert_eq!(prop_is_live(&ctx, p.id).await, 0, "свойство скрыто");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn atomic_unarchive_property() {
    let (ctx, _dir) = make_test_ctx().await;
    let t = new_custom_type(&ctx, "Стойка Альфа").await;
    let p = ctx
        .group_types
        .create_property(&admin(), prop_create(t.id, "Метка", "text"))
        .await
        .unwrap();
    let g = seed_group(&ctx, t.id, "Стойка Бета", 1).await;
    exec(
        &ctx,
        "INSERT INTO group_property_values (group_id, property_id, value_text, updated_at_utc)
         VALUES (?1, ?2, 'м-01', 1700000000)",
        vec![int(g), int(p.id)],
    )
    .await;
    // Скрываем до установки триггера.
    let out = ctx
        .group_types
        .delete_property(&admin(), p.id)
        .await
        .unwrap();
    assert!(out.archived);
    assert_eq!(1 - prop_is_live(&ctx, p.id).await, 1);

    install_audit_fault(&ctx).await;
    let res = ctx.group_types.unarchive_property(&admin(), p.id).await;
    assert_audit_fault(res, "аудит оборван");
    assert_eq!(
        1 - prop_is_live(&ctx, p.id).await,
        1,
        "свойство осталось скрытым"
    );

    remove_audit_fault(&ctx).await;
    ctx.group_types
        .unarchive_property(&admin(), p.id)
        .await
        .expect("без триггера возврат проходит");
    assert_eq!(
        1 - prop_is_live(&ctx, p.id).await,
        0,
        "свойство снова живое"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn atomic_reorder_properties() {
    let (ctx, _dir) = make_test_ctx().await;
    let t = new_custom_type(&ctx, "Стойка Альфа").await;
    let mut ids = Vec::new();
    for name in ["Метка", "Хост", "Стойка"] {
        let p = ctx
            .group_types
            .create_property(&admin(), prop_create(t.id, name, "text"))
            .await
            .unwrap();
        ids.push(i32::try_from(p.id).unwrap());
    }
    let first = i64::from(ids[0]);
    let before = prop_sort_order(&ctx, first).await;
    let reversed: Vec<i32> = ids.iter().rev().copied().collect();
    let type_id = i32::try_from(t.id).unwrap();

    install_audit_fault(&ctx).await;
    let res = ctx
        .group_types
        .reorder_properties(&admin(), type_id, reversed.clone())
        .await;
    assert_audit_fault(res, "аудит оборван");
    assert_eq!(
        prop_sort_order(&ctx, first).await,
        before,
        "порядок прежний"
    );

    remove_audit_fault(&ctx).await;
    ctx.group_types
        .reorder_properties(&admin(), type_id, reversed)
        .await
        .expect("без триггера перестановка проходит");
    assert_ne!(
        prop_sort_order(&ctx, first).await,
        before,
        "порядок изменился"
    );
    assert_eq!(
        prop_sort_order(&ctx, first).await,
        2,
        "первое стало последним"
    );
}

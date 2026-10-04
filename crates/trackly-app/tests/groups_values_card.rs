//! Phase 41 Plan 12: значения свойств групп (`GroupService::set_values`), список
//! пользователей для выбора (`user_options`) и карточка группы (`card`) с принтерами.
//! Реальный `AppCtx` на временном каталоге; имена вымышленные.
//!
//! Префиксы тестов: `values_scalar_`, `values_users_`, `values_refs_`, `values_required_`,
//! `values_cas_`, `values_rights_`, `user_options_`, `card_usb_`, `card_dedup_`,
//! `card_primary_`, `card_hidden_`, `card_sc7_`.

use rusqlite::params;

use trackly_app::context::AppCtx;
use trackly_app::dto::auth::UserNew;
use trackly_app::dto::group_types::{GroupTypeCreateDto, PropertyCreateDto, PropertyUpdateDto};
use trackly_app::dto::groups::{
    GroupCardDto, GroupCreateDto, GroupRefInputDto, GroupValueInputDto, UserOptionDto,
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

fn validation_field_and_message(err: AppError) -> (String, String) {
    match err {
        AppError::Validation { field, message } => (field, message),
        other => panic!("ожидали Validation, получили {other:?}"),
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

async fn text_values(ctx: &AppCtx, group: i64, property: i64) -> Vec<String> {
    ctx.writer
        .execute(move |conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT value_text FROM group_property_values \
                     WHERE group_id = ?1 AND property_id = ?2 ORDER BY position",
                )
                .map_err(map_rusqlite)?;
            let rows = stmt
                .query_map(params![group, property], |r| r.get::<_, Option<String>>(0))
                .map_err(map_rusqlite)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(map_rusqlite)?;
            Ok(rows.into_iter().flatten().collect())
        })
        .await
        .expect("text values")
}

async fn value_row_count(ctx: &AppCtx, group: i64) -> i64 {
    scalar_i64(
        ctx,
        "SELECT COUNT(*) FROM group_property_values WHERE group_id = ?1",
        vec![int(group)],
    )
    .await
}

#[derive(Clone, Copy)]
struct Prop {
    id: i64,
    version: i64,
}

/// Свойство засеянного типа «АРМ» по имени.
async fn arm_prop(ctx: &AppCtx, name: &str) -> Prop {
    let types = ctx
        .group_types
        .list_types(&admin(), true)
        .await
        .expect("list_types");
    let arm = types
        .into_iter()
        .find(|t| t.code == "workstation")
        .expect("АРМ");
    let p = arm
        .properties
        .into_iter()
        .find(|p| p.name == name)
        .unwrap_or_else(|| panic!("свойство {name} не найдено"));
    Prop {
        id: p.id,
        version: p.version,
    }
}

fn scalar(property_id: i64, text: &str) -> GroupValueInputDto {
    GroupValueInputDto {
        property_id,
        text: Some(text.to_string()),
        refs: vec![],
    }
}

fn refs(property_id: i64, items: &[(i64, bool)]) -> GroupValueInputDto {
    GroupValueInputDto {
        property_id,
        text: None,
        refs: items
            .iter()
            .map(|(ref_id, is_primary)| GroupRefInputDto {
                ref_id: *ref_id,
                is_primary: *is_primary,
            })
            .collect(),
    }
}

/// Живая группа типа «АРМ» (через сервис, версия 1).
async fn arm_group(ctx: &AppCtx) -> i64 {
    let arm = type_id_by_code(ctx, "workstation").await;
    ctx.groups
        .create_group(
            &admin(),
            GroupCreateDto {
                type_id: arm,
                name: None,
                place_id: None,
            },
        )
        .await
        .expect("create arm group")
        .id
}

/// Пользовательский тип с одним свойством заданного типа данных.
async fn custom_type_with_property(
    ctx: &AppCtx,
    type_name: &str,
    prop_name: &str,
    data_type: &str,
) -> (i64, i64) {
    let t = ctx
        .group_types
        .create_type(
            &admin(),
            GroupTypeCreateDto {
                name: type_name.to_string(),
                behavior: "container".to_string(),
            },
        )
        .await
        .expect("create type");
    let p = ctx
        .group_types
        .create_property(
            &admin(),
            PropertyCreateDto {
                type_id: t.id,
                name: prop_name.to_string(),
                data_type: data_type.to_string(),
                is_required: false,
                show_on_map: false,
            },
        )
        .await
        .expect("create property");
    (t.id, p.id)
}

async fn group_of_type(ctx: &AppCtx, type_id: i64) -> GroupCardDto {
    let g = ctx
        .groups
        .create_group(
            &admin(),
            GroupCreateDto {
                type_id,
                name: None,
                place_id: None,
            },
        )
        .await
        .expect("create group");
    ctx.groups.card(&admin(), g.id).await.expect("card")
}

async fn seed_user_row(ctx: &AppCtx, login: &str, full_name: &str) -> i64 {
    let (login, full_name) = (login.to_string(), full_name.to_string());
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO users (login, full_name, role, ad_user, created_at_utc, \
                 updated_at_utc, version) VALUES (?1, ?2, 'employee', 0, 1700000000, 1700000000, 1)",
                params![login, full_name],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed user")
}

/// Сеет принтер: устройство + строка `printers` (опционально USB-хост).
async fn seed_printer(ctx: &AppCtx, name: &str, inv: &str, usb_host: Option<i64>) -> i64 {
    let id = seed_device(ctx, name, inv, &format!("SN-{inv}"), None, None).await;
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO printers (device_id, usb_host_device_id, created_at_utc, \
                 updated_at_utc) VALUES (?1, ?2, 1700000000, 1700000000)",
                params![id, usb_host],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("seed printer");
    id
}

// ---------------------------------------------------------------------------
// values_scalar_
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn values_scalar_ip_is_stored_canonical_and_invalid_rejected() {
    let (ctx, _dir) = make_test_ctx().await;
    let ip = arm_prop(&ctx, "IP").await;
    let g = arm_group(&ctx).await;

    let card = ctx
        .groups
        .set_values(&admin(), g, 1, vec![scalar(ip.id, "192.168.1.10")])
        .await
        .expect("plain ip");
    assert_eq!(text_values(&ctx, g, ip.id).await, vec!["192.168.1.10"]);

    ctx.groups
        .set_values(
            &admin(),
            g,
            card.group.version,
            vec![scalar(ip.id, " 2001:DB8:0:0:0:0:0:1 ")],
        )
        .await
        .expect("ipv6");
    // В БД лежит каноническая форма, а не введённая строка.
    assert_eq!(text_values(&ctx, g, ip.id).await, vec!["2001:db8::1"]);

    let ver = group_version(&ctx, g).await;
    let err = ctx
        .groups
        .set_values(&admin(), g, ver, vec![scalar(ip.id, "01.2.3.4")])
        .await
        .expect_err("bad ip");
    let (field, message) = validation_field_and_message(err);
    assert_eq!(field, format!("values.{}", ip.id));
    assert!(message.contains("IPv4"), "{message}");
    assert_eq!(text_values(&ctx, g, ip.id).await, vec!["2001:db8::1"]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn values_scalar_mac_accepts_three_separators_and_rejects_mixed() {
    let (ctx, _dir) = make_test_ctx().await;
    let mac = arm_prop(&ctx, "MAC").await;
    let g = arm_group(&ctx).await;

    for raw in ["AA-BB-CC-DD-EE-FF", "aa:bb:cc:dd:ee:ff", "aabbccddeeff"] {
        let ver = group_version(&ctx, g).await;
        // Перед каждой проверкой — другое значение, чтобы чтение не совпало случайно.
        ctx.groups
            .set_values(&admin(), g, ver, vec![scalar(mac.id, "00:00:00:00:00:01")])
            .await
            .expect("reset");
        let ver = group_version(&ctx, g).await;
        ctx.groups
            .set_values(&admin(), g, ver, vec![scalar(mac.id, raw)])
            .await
            .unwrap_or_else(|e| panic!("mac {raw}: {e:?}"));
        assert_eq!(
            text_values(&ctx, g, mac.id).await,
            vec!["aa:bb:cc:dd:ee:ff"],
            "raw {raw}"
        );
    }

    let ver = group_version(&ctx, g).await;
    let err = ctx
        .groups
        .set_values(&admin(), g, ver, vec![scalar(mac.id, "aa:bb-cc:dd:ee:ff")])
        .await
        .expect_err("mixed separators");
    let (field, _) = validation_field_and_message(err);
    assert_eq!(field, format!("values.{}", mac.id));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn values_scalar_number_and_text_normalization() {
    let (ctx, _dir) = make_test_ctx().await;
    let (type_id, num) = custom_type_with_property(&ctx, "Стойка", "Вес", "number").await;
    let card = group_of_type(&ctx, type_id).await;
    let g = card.group.id;

    ctx.groups
        .set_values(&admin(), g, card.group.version, vec![scalar(num, "1,5")])
        .await
        .expect("number");
    assert_eq!(text_values(&ctx, g, num).await, vec!["1.5"]);

    let ver = group_version(&ctx, g).await;
    let err = ctx
        .groups
        .set_values(&admin(), g, ver, vec![scalar(num, "abc")])
        .await
        .expect_err("not a number");
    let (field, _) = validation_field_and_message(err);
    assert_eq!(field, format!("values.{num}"));
    assert_eq!(text_values(&ctx, g, num).await, vec!["1.5"]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn values_scalar_text_is_trimmed_and_empty_removes_the_row() {
    let (ctx, _dir) = make_test_ctx().await;
    let host = arm_prop(&ctx, "Хост").await;
    let g = arm_group(&ctx).await;

    ctx.groups
        .set_values(&admin(), g, 1, vec![scalar(host.id, "  pc-01  ")])
        .await
        .expect("text");
    assert_eq!(text_values(&ctx, g, host.id).await, vec!["pc-01"]);

    let ver = group_version(&ctx, g).await;
    ctx.groups
        .set_values(&admin(), g, ver, vec![scalar(host.id, "   ")])
        .await
        .expect("clear");
    // «Пусто» = отсутствие строки, а не пустая строка.
    assert_eq!(value_row_count(&ctx, g).await, 0);
}

// ---------------------------------------------------------------------------
// values_users_
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn values_users_live_accepted_and_dead_rejected() {
    let (ctx, _dir) = make_test_ctx().await;
    let users = arm_prop(&ctx, "Пользователи").await;
    let g = arm_group(&ctx).await;
    let live = seed_user_row(&ctx, "ivanov", "Иванов И.И.").await;
    let inactive = seed_user_row(&ctx, "petrov", "Петров П.П.").await;
    let deleted = seed_user_row(&ctx, "sidorov", "Сидоров С.С.").await;
    exec(
        &ctx,
        "UPDATE users SET is_active = 0 WHERE id = ?1",
        vec![int(inactive)],
    )
    .await;
    exec(
        &ctx,
        "UPDATE users SET deleted_at_utc = 1700000001 WHERE id = ?1",
        vec![int(deleted)],
    )
    .await;

    for bad in [inactive, deleted, 987_654] {
        let ver = group_version(&ctx, g).await;
        let err = ctx
            .groups
            .set_values(&admin(), g, ver, vec![refs(users.id, &[(bad, false)])])
            .await
            .expect_err("dead user");
        let (field, message) = validation_field_and_message(err);
        assert_eq!(field, format!("values.{}", users.id));
        assert!(message.contains("уже заходил в приложение"), "{message}");
    }
    assert_eq!(value_row_count(&ctx, g).await, 0);

    let ver = group_version(&ctx, g).await;
    ctx.groups
        .set_values(&admin(), g, ver, vec![refs(users.id, &[(live, true)])])
        .await
        .expect("live user");
    assert_eq!(value_row_count(&ctx, g).await, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn values_users_primary_rules_and_duplicates() {
    let (ctx, _dir) = make_test_ctx().await;
    let users = arm_prop(&ctx, "Пользователи").await;
    let g = arm_group(&ctx).await;
    let a = seed_user_row(&ctx, "ivanov", "Иванов И.И.").await;
    let b = seed_user_row(&ctx, "petrov", "Петров П.П.").await;

    let ver = group_version(&ctx, g).await;
    let err = ctx
        .groups
        .set_values(
            &admin(),
            g,
            ver,
            vec![refs(users.id, &[(a, true), (b, true)])],
        )
        .await
        .expect_err("two primaries");
    assert!(matches!(err, AppError::Validation { .. }));

    let err = ctx
        .groups
        .set_values(
            &admin(),
            g,
            ver,
            vec![refs(users.id, &[(a, false), (a, false)])],
        )
        .await
        .expect_err("duplicate");
    assert!(matches!(err, AppError::Validation { .. }));
    assert_eq!(value_row_count(&ctx, g).await, 0);

    ctx.groups
        .set_values(
            &admin(),
            g,
            ver,
            vec![refs(users.id, &[(a, false), (b, true)])],
        )
        .await
        .expect("one primary");
    let primary_ref = opt_i64(
        &ctx,
        "SELECT value_ref FROM group_property_values WHERE group_id = ?1 AND is_primary = 1",
        vec![int(g)],
    )
    .await;
    assert_eq!(primary_ref, Some(b));
    let primaries = scalar_i64(
        &ctx,
        "SELECT COUNT(*) FROM group_property_values WHERE group_id = ?1 AND is_primary = 1",
        vec![int(g)],
    )
    .await;
    assert_eq!(primaries, 1);
}

// ---------------------------------------------------------------------------
// values_refs_
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn values_refs_live_devices_only_and_limit() {
    let (ctx, _dir) = make_test_ctx().await;
    let link = arm_prop(&ctx, "Подключённые принтеры").await;
    let g = arm_group(&ctx).await;
    let live = seed_device(&ctx, "Принтер А", "INV-R-1", "SN-R-1", None, None).await;
    let dead = seed_device(&ctx, "Принтер Б", "INV-R-2", "SN-R-2", None, None).await;
    exec(
        &ctx,
        "UPDATE devices SET deleted_at_utc = 1700000001 WHERE id = ?1",
        vec![int(dead)],
    )
    .await;

    for bad in [dead, 987_654] {
        let ver = group_version(&ctx, g).await;
        let err = ctx
            .groups
            .set_values(&admin(), g, ver, vec![refs(link.id, &[(bad, false)])])
            .await
            .expect_err("dead device");
        let (field, _) = validation_field_and_message(err);
        assert_eq!(field, format!("values.{}", link.id));
    }

    let ver = group_version(&ctx, g).await;
    let too_many: Vec<(i64, bool)> = (1..=101).map(|i| (i, false)).collect();
    let err = ctx
        .groups
        .set_values(&admin(), g, ver, vec![refs(link.id, &too_many)])
        .await
        .expect_err("101 refs");
    assert!(matches!(err, AppError::Validation { .. }));
    assert_eq!(value_row_count(&ctx, g).await, 0);

    ctx.groups
        .set_values(&admin(), g, ver, vec![refs(link.id, &[(live, false)])])
        .await
        .expect("live device");
    assert_eq!(value_row_count(&ctx, g).await, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn values_refs_foreign_and_hidden_property_rejected() {
    let (ctx, _dir) = make_test_ctx().await;
    let host = arm_prop(&ctx, "Хост").await;
    let g = arm_group(&ctx).await;
    let (type_id, foreign) = custom_type_with_property(&ctx, "Стойка", "Заметка", "text").await;

    // Свойство чужого типа.
    let err = ctx
        .groups
        .set_values(&admin(), g, 1, vec![scalar(foreign, "x")])
        .await
        .expect_err("foreign property");
    let (field, _) = validation_field_and_message(err);
    assert_eq!(field, format!("values.{foreign}"));

    // Скрытое свойство: заполняем, скрываем, затем запись отклоняется.
    let card = group_of_type(&ctx, type_id).await;
    let g2 = card.group.id;
    ctx.groups
        .set_values(
            &admin(),
            g2,
            card.group.version,
            vec![scalar(foreign, "заметка")],
        )
        .await
        .expect("fill");
    let outcome = ctx
        .group_types
        .delete_property(&admin(), foreign)
        .await
        .expect("hide");
    assert!(outcome.archived);
    let ver = group_version(&ctx, g2).await;
    let err = ctx
        .groups
        .set_values(&admin(), g2, ver, vec![scalar(foreign, "ещё")])
        .await
        .expect_err("hidden property");
    let (field, _) = validation_field_and_message(err);
    assert_eq!(field, format!("values.{foreign}"));

    // Дубликат property_id в одной форме.
    let err = ctx
        .groups
        .set_values(
            &admin(),
            g,
            1,
            vec![scalar(host.id, "a"), scalar(host.id, "b")],
        )
        .await
        .expect_err("same property twice");
    assert!(matches!(err, AppError::Validation { .. }));
}

// ---------------------------------------------------------------------------
// values_required_
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn values_required_checked_on_final_state() {
    let (ctx, _dir) = make_test_ctx().await;
    let host = arm_prop(&ctx, "Хост").await;
    let ip = arm_prop(&ctx, "IP").await;
    // Обязательность включается ДО создания групп: пустых групп ещё нет.
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
        .expect("make required");
    let g = arm_group(&ctx).await;

    // Без обязательного (ни передано, ни сохранено) и с пустым — отказ.
    for form in [vec![scalar(ip.id, "10.0.0.1")], vec![scalar(host.id, "  ")]] {
        let err = ctx
            .groups
            .set_values(&admin(), g, 1, form)
            .await
            .expect_err("required missing");
        let (field, message) = validation_field_and_message(err);
        assert_eq!(field, format!("values.{}", host.id));
        assert_eq!(message, "Заполните обязательное свойство «Хост».");
    }
    // Откат полный: IP из первой формы не записан, версия не выросла.
    assert_eq!(value_row_count(&ctx, g).await, 0);
    assert_eq!(group_version(&ctx, g).await, 1);

    let card = ctx
        .groups
        .set_values(&admin(), g, 1, vec![scalar(host.id, "pc-01")])
        .await
        .expect("with required");
    // Обязательное уже сохранено: форма без него (только IP) проходит.
    let card = ctx
        .groups
        .set_values(
            &admin(),
            g,
            card.group.version,
            vec![scalar(ip.id, "10.0.0.2")],
        )
        .await
        .expect("required already stored");

    // Снятие значения с обязательного — отказ, прежнее значение остаётся.
    let err = ctx
        .groups
        .set_values(&admin(), g, card.group.version, vec![scalar(host.id, "")])
        .await
        .expect_err("clear required");
    assert!(matches!(err, AppError::Validation { .. }));
    assert_eq!(text_values(&ctx, g, host.id).await, vec!["pc-01"]);
}

// ---------------------------------------------------------------------------
// values_cas_ / values_rights_
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn values_cas_stale_version_conflicts_and_success_bumps_by_one() {
    let (ctx, _dir) = make_test_ctx().await;
    let ip = arm_prop(&ctx, "IP").await;
    let mac = arm_prop(&ctx, "MAC").await;
    let g = arm_group(&ctx).await;

    let card = ctx
        .groups
        .set_values(&admin(), g, 1, vec![scalar(ip.id, "10.0.0.1")])
        .await
        .expect("first");
    assert_eq!(card.group.version, 2);
    assert_eq!(group_version(&ctx, g).await, 2);

    let err = ctx
        .groups
        .set_values(&admin(), g, 1, vec![scalar(ip.id, "10.0.0.9")])
        .await
        .expect_err("stale");
    assert!(matches!(
        err,
        AppError::OptimisticLockMismatch {
            expected: 1,
            actual: 2,
            ..
        }
    ));
    assert_eq!(text_values(&ctx, g, ip.id).await, vec!["10.0.0.1"]);

    // Атомарность: [корректное, некорректное] — не записано ничего.
    let err = ctx
        .groups
        .set_values(
            &admin(),
            g,
            2,
            vec![scalar(ip.id, "10.0.0.7"), scalar(mac.id, "zz")],
        )
        .await
        .expect_err("second invalid");
    assert!(matches!(err, AppError::Validation { .. }));
    assert_eq!(text_values(&ctx, g, ip.id).await, vec!["10.0.0.1"]);
    assert_eq!(group_version(&ctx, g).await, 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn values_rights_employee_forbidden_manager_allowed() {
    let (ctx, _dir) = make_test_ctx().await;
    let host = arm_prop(&ctx, "Хост").await;
    let g = arm_group(&ctx).await;
    let employee = create_identity(&ctx, "emp1", "Петров П.П.", Role::Employee).await;
    let manager = create_identity(&ctx, "mgr1", "Иванов И.И.", Role::Manager).await;

    let err = ctx
        .groups
        .set_values(&employee, g, 1, vec![scalar(host.id, "pc")])
        .await
        .expect_err("employee");
    assert!(matches!(err, AppError::Forbidden));
    assert_eq!(value_row_count(&ctx, g).await, 0);

    ctx.groups
        .set_values(&manager, g, 1, vec![scalar(host.id, "pc")])
        .await
        .expect("manager");
    assert_eq!(value_row_count(&ctx, g).await, 1);
}

// ---------------------------------------------------------------------------
// user_options_
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn user_options_filters_case_insensitively_and_hides_dead() {
    let (ctx, _dir) = make_test_ctx().await;
    let live = seed_user_row(&ctx, "ivanov", "Иванов И.И.").await;
    let inactive = seed_user_row(&ctx, "ivanova", "Иванова А.А.").await;
    let deleted = seed_user_row(&ctx, "ivankin", "Иванкин К.К.").await;
    seed_user_row(&ctx, "petrov", "Петров П.П.").await;
    exec(
        &ctx,
        "UPDATE users SET is_active = 0 WHERE id = ?1",
        vec![int(inactive)],
    )
    .await;
    exec(
        &ctx,
        "UPDATE users SET deleted_at_utc = 1700000001 WHERE id = ?1",
        vec![int(deleted)],
    )
    .await;
    let manager = create_identity(&ctx, "mgr1", "Сидоров С.С.", Role::Manager).await;

    // Регистр кириллицы не важен; манагер имеет право (users_list ему недоступен).
    let found = ctx
        .groups
        .user_options(&manager, "ИВАН".to_string())
        .await
        .expect("search");
    let ids: Vec<i64> = found.iter().map(|u| u.id).collect();
    assert_eq!(ids, vec![live]);
    assert_eq!(found[0].login, "ivanov");

    // Совпадение по логину.
    let by_login = ctx
        .groups
        .user_options(&manager, "PETR".to_string())
        .await
        .expect("login search");
    assert_eq!(by_login.len(), 1);
    assert_eq!(by_login[0].full_name, "Петров П.П.");

    let err = ctx
        .groups
        .user_options(&manager, "а".repeat(101))
        .await
        .expect_err("too long");
    assert!(matches!(err, AppError::Validation { .. }));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn user_options_caps_at_50_exposes_only_safe_keys_and_rejects_employee() {
    let (ctx, _dir) = make_test_ctx().await;
    for i in 0..55 {
        seed_user_row(&ctx, &format!("user{i:02}"), &format!("Сотрудник {i:02}")).await;
    }
    let employee = create_identity(&ctx, "emp1", "Петров П.П.", Role::Employee).await;

    let err = ctx
        .groups
        .user_options(&employee, String::new())
        .await
        .expect_err("employee");
    assert!(matches!(err, AppError::Forbidden));

    let all: Vec<UserOptionDto> = ctx
        .groups
        .user_options(&admin(), "сотрудник".to_string())
        .await
        .expect("search");
    assert_eq!(all.len(), 50);

    let json = serde_json::to_value(&all[0]).expect("json");
    let mut keys: Vec<&str> = json
        .as_object()
        .expect("object")
        .keys()
        .map(|k| k.as_str())
        .collect();
    keys.sort_unstable();
    assert_eq!(keys, vec!["full_name", "id", "login"]);
}

// ---------------------------------------------------------------------------
// card_
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn card_sc7_clean_db_arm_card_has_five_seeded_properties() {
    let (ctx, _dir) = make_test_ctx().await;
    let g = arm_group(&ctx).await;
    let link = arm_prop(&ctx, "Подключённые принтеры").await;
    let users = arm_prop(&ctx, "Пользователи").await;
    let u = seed_user_row(&ctx, "ivanov", "Иванов И.И.").await;

    let card = ctx.groups.card(&admin(), g).await.expect("card");
    let names: Vec<&str> = card.properties.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(
        names,
        vec!["Пользователи", "Подключённые принтеры", "Хост", "IP", "MAC"]
    );
    assert_eq!(card.link_property_id, Some(link.id));
    assert!(card.printers.is_empty());

    // Свойство «Пользователи» принимает людей и основного сразу, без действий над типом.
    let card = ctx
        .groups
        .set_values(&admin(), g, 1, vec![refs(users.id, &[(u, true)])])
        .await
        .expect("users");
    let p = card
        .properties
        .iter()
        .find(|p| p.property_id == users.id)
        .expect("users prop");
    assert_eq!(p.users.len(), 1);
    assert_eq!(p.users[0].full_name, "Иванов И.И.");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn card_primary_user_is_returned_with_flag() {
    let (ctx, _dir) = make_test_ctx().await;
    let users = arm_prop(&ctx, "Пользователи").await;
    let g = arm_group(&ctx).await;
    let a = seed_user_row(&ctx, "ivanov", "Иванов И.И.").await;
    let b = seed_user_row(&ctx, "petrov", "Петров П.П.").await;

    let card = ctx
        .groups
        .set_values(
            &admin(),
            g,
            1,
            vec![refs(users.id, &[(a, false), (b, true)])],
        )
        .await
        .expect("users");
    let p = card
        .properties
        .iter()
        .find(|p| p.property_id == users.id)
        .expect("prop");
    let flags: Vec<(i64, bool)> = p.users.iter().map(|u| (u.user_id, u.is_primary)).collect();
    assert_eq!(flags, vec![(a, false), (b, true)]);

    // Отдельное чтение карточки отдаёт то же.
    let again = ctx.groups.card(&admin(), g).await.expect("card");
    assert_eq!(again.properties, card.properties);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn card_usb_printers_come_from_composition_including_nested_group() {
    let (ctx, _dir) = make_test_ctx().await;
    let arm = type_id_by_code(&ctx, "workstation").await;
    let parent = seed_group(&ctx, arm, "АРМ #7", 7, None, None).await;
    let child = seed_group(&ctx, arm, "АРМ #8", 8, None, Some(parent)).await;
    let host_direct = seed_device(&ctx, "Ноутбук 1", "INV-U-1", "SN-U-1", None, Some(parent)).await;
    let host_nested = seed_device(&ctx, "Ноутбук 2", "INV-U-2", "SN-U-2", None, Some(child)).await;
    let outsider = seed_device(&ctx, "Ноутбук 3", "INV-U-3", "SN-U-3", None, None).await;
    let p1 = seed_printer(&ctx, "Принтер А", "INV-P-1", Some(host_direct)).await;
    let p2 = seed_printer(&ctx, "Принтер Б", "INV-P-2", Some(host_nested)).await;
    seed_printer(&ctx, "Принтер В", "INV-P-3", Some(outsider)).await;
    let dead = seed_printer(&ctx, "Принтер Г", "INV-P-4", Some(host_direct)).await;
    exec(
        &ctx,
        "UPDATE devices SET deleted_at_utc = 1700000001 WHERE id = ?1",
        vec![int(dead)],
    )
    .await;

    let card = ctx.groups.card(&admin(), parent).await.expect("card");
    let got: Vec<(i64, &str, bool)> = card
        .printers
        .iter()
        .map(|p| (p.device_id, p.origin.as_str(), p.has_explicit_link))
        .collect();
    assert_eq!(got, vec![(p1, "usb", false), (p2, "usb", false)]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn card_dedup_usb_and_link_collapse_to_one_row() {
    let (ctx, _dir) = make_test_ctx().await;
    let link = arm_prop(&ctx, "Подключённые принтеры").await;
    let g = arm_group(&ctx).await;
    let host = seed_device(&ctx, "Ноутбук 1", "INV-D-1", "SN-D-1", None, Some(g)).await;
    let both = seed_printer(&ctx, "Принтер А", "INV-D-2", Some(host)).await;
    let link_only = seed_printer(&ctx, "Принтер Б", "INV-D-3", None).await;
    let not_printer = seed_device(&ctx, "Монитор 1", "INV-D-4", "SN-D-4", None, None).await;

    let card = ctx
        .groups
        .set_values(
            &admin(),
            g,
            1,
            vec![refs(
                link.id,
                &[(both, false), (link_only, false), (not_printer, false)],
            )],
        )
        .await
        .expect("links");

    // Принтер, найденный и по USB, и по ссылке, — ОДНА строка, победил usb.
    let both_rows: Vec<_> = card
        .printers
        .iter()
        .filter(|p| p.device_id == both)
        .collect();
    assert_eq!(both_rows.len(), 1);
    assert_eq!(both_rows[0].origin, "usb");
    assert!(both_rows[0].has_explicit_link);
    // Принтер только по ссылке.
    let only = card
        .printers
        .iter()
        .find(|p| p.device_id == link_only)
        .expect("link only");
    assert_eq!(only.origin, "link");
    assert!(only.has_explicit_link);
    // Устройство без строки printers в список принтеров не попадает.
    assert_eq!(card.printers.len(), 2);
    // Но остаётся в ссылках свойства: форма сохраняет их целиком.
    let prop = card
        .properties
        .iter()
        .find(|p| p.property_id == link.id)
        .expect("prop");
    assert_eq!(prop.ref_device_ids, vec![both, link_only, not_printer]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn card_hidden_property_and_dead_refs_are_not_returned() {
    let (ctx, _dir) = make_test_ctx().await;
    let link = arm_prop(&ctx, "Подключённые принтеры").await;
    let users = arm_prop(&ctx, "Пользователи").await;
    let g = arm_group(&ctx).await;
    let live_dev = seed_device(&ctx, "Монитор 1", "INV-H-1", "SN-H-1", None, None).await;
    let dead_dev = seed_device(&ctx, "Монитор 2", "INV-H-2", "SN-H-2", None, None).await;
    let live_user = seed_user_row(&ctx, "ivanov", "Иванов И.И.").await;
    let off_user = seed_user_row(&ctx, "petrov", "Петров П.П.").await;
    ctx.groups
        .set_values(
            &admin(),
            g,
            1,
            vec![
                refs(link.id, &[(live_dev, false), (dead_dev, false)]),
                refs(users.id, &[(live_user, false), (off_user, false)]),
            ],
        )
        .await
        .expect("fill");
    exec(
        &ctx,
        "UPDATE devices SET deleted_at_utc = 1700000001 WHERE id = ?1",
        vec![int(dead_dev)],
    )
    .await;
    exec(
        &ctx,
        "UPDATE users SET is_active = 0 WHERE id = ?1",
        vec![int(off_user)],
    )
    .await;

    let card = ctx.groups.card(&admin(), g).await.expect("card");
    let lp = card
        .properties
        .iter()
        .find(|p| p.property_id == link.id)
        .expect("link prop");
    assert_eq!(lp.ref_device_ids, vec![live_dev]);
    let up = card
        .properties
        .iter()
        .find(|p| p.property_id == users.id)
        .expect("users prop");
    assert_eq!(up.users.len(), 1);
    assert_eq!(up.users[0].user_id, live_user);
    // Строки мёртвых ссылок в БД остались — карточка их только скрывает.
    assert_eq!(value_row_count(&ctx, g).await, 4);

    // Скрытое свойство: из карточки пропало, значение в БД на месте (D-13).
    let (type_id, note) = custom_type_with_property(&ctx, "Стойка", "Заметка", "text").await;
    let c2 = group_of_type(&ctx, type_id).await;
    ctx.groups
        .set_values(
            &admin(),
            c2.group.id,
            c2.group.version,
            vec![scalar(note, "важно")],
        )
        .await
        .expect("fill note");
    ctx.group_types
        .delete_property(&admin(), note)
        .await
        .expect("hide");
    let c2 = ctx.groups.card(&admin(), c2.group.id).await.expect("card");
    assert!(c2.properties.iter().all(|p| p.property_id != note));
    assert_eq!(text_values(&ctx, c2.group.id, note).await, vec!["важно"]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn card_rights_employee_forbidden() {
    let (ctx, _dir) = make_test_ctx().await;
    let g = arm_group(&ctx).await;
    let employee = create_identity(&ctx, "emp1", "Петров П.П.", Role::Employee).await;
    let manager = create_identity(&ctx, "mgr1", "Иванов И.И.", Role::Manager).await;
    let err = ctx
        .groups
        .card(&employee, g)
        .await
        .expect_err("employee card");
    assert!(matches!(err, AppError::Forbidden));
    ctx.groups.card(&manager, g).await.expect("manager card");
}

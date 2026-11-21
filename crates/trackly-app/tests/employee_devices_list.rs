//! Phase 41.6, план 07: список «Устройства» сотрудника (`build_employee_devices_list`).
//!
//! Настоящий `AppCtx::build`. Данные вымышленные: «Иванов И.И.», «Петров П.П.»,
//! «АРМ #1», «АРМ #2», «Принтер 2-го этажа», «Кабинет 12». Ожидаемые подписи
//! статусов берутся SELECT-ом из `device_statuses`, а не литералом.

mod entities_support;
mod repair_support;

use entities_support::fixture::{make_test_ctx, seed_device, seed_place};
use repair_support::{emp, seed_user};
use rusqlite::params;
use serde_json::Value;
use trackly_app::context::AppCtx;
use trackly_app::dto::employee_device::{EmployeeDeviceDto, EmployeeDevicesDto};
use trackly_app::tauri_cmds::employee_devices::build_employee_devices_list;
use trackly_core::auth::{Identity, Role};
use trackly_core::error::AppError;
use trackly_infra::error_conversions::map_rusqlite;

const NOW: i64 = 1_700_000_000;

/// Идентификаторы встроенного типа групп с двумя свойствами.
struct ArmType {
    type_id: i64,
    p_users: i64,
    p_refs: i64,
}

async fn seed_arm_type(ctx: &AppCtx) -> ArmType {
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO group_types (code, name, behavior, created_at_utc, updated_at_utc) \
                 VALUES ('arm_emp_list', 'АРМ-тест', 'workstation', ?1, ?1)",
                params![NOW],
            )
            .map_err(map_rusqlite)?;
            let type_id = conn.last_insert_rowid();
            let prop = |name: &str, dt: &str| -> Result<i64, AppError> {
                conn.execute(
                    "INSERT INTO group_type_properties (type_id, name, data_type, \
                     created_at_utc, updated_at_utc) VALUES (?1, ?2, ?3, ?4, ?4)",
                    params![type_id, name, dt, NOW],
                )
                .map_err(map_rusqlite)?;
                Ok(conn.last_insert_rowid())
            };
            let p_users = prop("Пользователи", "users")?;
            let p_refs = prop("Устройства", "device_refs")?;
            Ok(ArmType {
                type_id,
                p_users,
                p_refs,
            })
        })
        .await
        .expect("seed arm type")
}

async fn seed_group(ctx: &AppCtx, t: &ArmType, name: &str, parent: Option<i64>) -> i64 {
    let (type_id, name) = (t.type_id, name.to_string());
    ctx.writer
        .execute(move |conn| {
            let seq: i64 = conn
                .query_row("SELECT COALESCE(MAX(seq),0)+1 FROM groups", [], |r| {
                    r.get(0)
                })
                .map_err(map_rusqlite)?;
            conn.execute(
                "INSERT INTO groups (type_id, name, seq, parent_group_id, version, \
                 created_at_utc, updated_at_utc) VALUES (?1, ?2, ?3, ?4, 1, ?5, ?5)",
                params![type_id, name, seq, parent, NOW],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed group")
}

async fn bind_user(ctx: &AppCtx, t: &ArmType, group: i64, user: i64) {
    let p = t.p_users;
    prop_value(ctx, group, p, user).await;
}

async fn link_device(ctx: &AppCtx, t: &ArmType, group: i64, device: i64) {
    let p = t.p_refs;
    prop_value(ctx, group, p, device).await;
}

async fn prop_value(ctx: &AppCtx, group: i64, prop: i64, value_ref: i64) {
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO group_property_values (group_id, property_id, value_ref, \
                 updated_at_utc) VALUES (?1, ?2, ?3, ?4)",
                params![group, prop, value_ref, NOW],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("prop value");
}

async fn member(ctx: &AppCtx, group: i64, device: i64) {
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO group_devices (device_id, group_id, added_at_utc) \
                 VALUES (?1, ?2, ?3)",
                params![device, group, NOW],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("member");
}

/// Строка `printers` для устройства; возвращает `printers.id`.
async fn make_printer(ctx: &AppCtx, device: i64, usb_host: Option<i64>) -> i64 {
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO printers (device_id, usb_host_device_id, created_at_utc, \
                 updated_at_utc) VALUES (?1, ?2, ?3, ?3)",
                params![device, usb_host, NOW],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("make printer")
}

async fn add_reading(ctx: &AppCtx, printer_id: i64, ts: i64, levels: &str) {
    let levels = levels.to_string();
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO printer_readings (printer_id, ts_utc, toner_levels, status) \
                 VALUES (?1, ?2, ?3, 'ok')",
                params![printer_id, ts, levels],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("reading");
}

async fn set_status(ctx: &AppCtx, device: i64, code: &str) {
    let code = code.to_string();
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "UPDATE devices SET status_id = (SELECT id FROM device_statuses WHERE code = ?1) \
                 WHERE id = ?2",
                params![code, device],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("set status");
}

/// Статус без машинного кода (пользовательский): `ds.code IS NULL` в запросе списка.
async fn set_codeless_status(ctx: &AppCtx, device: i64, name: &str) {
    let name = name.to_string();
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO device_statuses (name, code) VALUES (?1, NULL)",
                params![name],
            )
            .map_err(map_rusqlite)?;
            let sid = conn.last_insert_rowid();
            conn.execute(
                "UPDATE devices SET status_id = ?1 WHERE id = ?2",
                params![sid, device],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("codeless status");
}

async fn status_name(ctx: &AppCtx, code: &'static str) -> String {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(
                "SELECT name FROM device_statuses WHERE code = ?1",
                params![code],
                |r| r.get::<_, String>(0),
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("status name")
}

/// Заявка напрямую SQL-ом. Возвращает id.
async fn add_request(
    ctx: &AppCtx,
    kind: &'static str,
    status: &'static str,
    author: i64,
    device: i64,
    created: i64,
) -> i64 {
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO requests (request_type, status, requested_by_user_id, \
                 printer_device_id, created_at_utc, updated_at_utc) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
                params![kind, status, author, device, created],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("add request")
}

async fn add_participant(ctx: &AppCtx, request: i64, user: i64) {
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO request_participants (request_id, user_id, joined_at_utc) \
                 VALUES (?1, ?2, ?3)",
                params![request, user, NOW],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("participant");
}

async fn list(ctx: &AppCtx, who: &Identity) -> EmployeeDevicesDto {
    build_employee_devices_list(ctx, who).await.expect("list")
}

fn names(dto: &EmployeeDevicesDto) -> Vec<String> {
    dto.items.iter().map(|d| d.name.clone()).collect()
}

fn by_name<'a>(dto: &'a EmployeeDevicesDto, name: &str) -> &'a EmployeeDeviceDto {
    dto.items
        .iter()
        .find(|d| d.name == name)
        .unwrap_or_else(|| panic!("устройство {name} отсутствует в списке: {:?}", names(dto)))
}

fn staff(role: Role, id: i64) -> Identity {
    Identity {
        user_id: Some(id),
        role,
    }
}

/// Все номера заявок, которые ответ раскрывает (myOpenRequests, myRepairRequestId, myRequests).
fn exposed_request_ids(dto: &EmployeeDevicesDto) -> Vec<i64> {
    let v = serde_json::to_value(dto).expect("to_value");
    let mut out = Vec::new();
    for item in v["items"].as_array().expect("items") {
        if let Some(n) = item["myRepairRequestId"].as_i64() {
            out.push(n);
        }
        for key in ["myOpenRequests", "myRequests"] {
            for r in item[key].as_array().expect("array") {
                out.push(r["id"].as_i64().expect("id"));
            }
        }
    }
    out
}

// ---------------------------------------------------------------- A: скоуп / BOLA

#[tokio::test]
async fn employee_devices_scope_is_own_only() {
    let (ctx, _dir) = make_test_ctx().await;
    let ivanov = seed_user(&ctx, "ivanov", "Иванов И.И.").await;
    let petrov = seed_user(&ctx, "petrov", "Петров П.П.").await;
    let boss = seed_user(&ctx, "boss", "Сидоров С.С.").await;
    let room = seed_place(&ctx, "Кабинет 12").await;
    let pc1 = seed_device(&ctx, 1, "ПК-1", "INV-1", Some(room), None).await;
    let pc2 = seed_device(&ctx, 1, "ПК-2", "INV-2", Some(room), None).await;
    let _pc3 = seed_device(&ctx, 1, "ПК-3 ничей", "INV-3", Some(room), None).await;
    let t = seed_arm_type(&ctx).await;
    let g1 = seed_group(&ctx, &t, "АРМ #1", None).await;
    let g2 = seed_group(&ctx, &t, "АРМ #2", None).await;
    bind_user(&ctx, &t, g1, ivanov).await;
    bind_user(&ctx, &t, g2, petrov).await;
    member(&ctx, g1, pc1).await;
    member(&ctx, g2, pc2).await;

    let a = list(&ctx, &emp(ivanov)).await;
    assert_eq!(names(&a), vec!["ПК-1"], "Иванов видит ровно своё");
    let b = list(&ctx, &emp(petrov)).await;
    assert_eq!(names(&b), vec!["ПК-2"], "Петров видит ровно своё");

    for role in [Role::Admin, Role::Manager] {
        let dto = list(&ctx, &staff(role.clone(), boss)).await;
        assert!(dto.items.is_empty(), "{role:?} по своему user_id: пусто");
        assert!(!dto.has_groups);
    }

    for role in [Role::Employee, Role::Manager, Role::Admin] {
        let who = Identity {
            user_id: None,
            role: role.clone(),
        };
        let r = build_employee_devices_list(&ctx, &who).await;
        assert!(
            matches!(r, Err(AppError::Forbidden)),
            "{role:?} без user_id"
        );
    }
}

// ---------------------------------------------------------------- B: «Списано»

#[tokio::test]
async fn employee_devices_written_off_hidden_others_shown() {
    let (ctx, _dir) = make_test_ctx().await;
    let ivanov = seed_user(&ctx, "ivanov", "Иванов И.И.").await;
    let t = seed_arm_type(&ctx).await;
    let g = seed_group(&ctx, &t, "АРМ #1", None).await;
    bind_user(&ctx, &t, g, ivanov).await;
    let wo = seed_device(&ctx, 1, "Списанный", "INV-1", None, None).await;
    let pending = seed_device(&ctx, 1, "К списанию", "INV-2", None, None).await;
    let work = seed_device(&ctx, 1, "Рабочий", "INV-3", None, None).await;
    let nul = seed_device(&ctx, 1, "Без кода статуса", "INV-4", None, None).await;
    for d in [wo, pending, work, nul] {
        member(&ctx, g, d).await;
    }
    set_status(&ctx, wo, "списано").await;
    set_status(&ctx, pending, "на_списание").await;
    set_status(&ctx, work, "в_работе").await;
    set_codeless_status(&ctx, nul, "Особый статус").await;

    let dto = list(&ctx, &emp(ivanov)).await;
    let got = names(&dto);
    assert!(!got.contains(&"Списанный".to_string()), "{got:?}");
    for n in ["К списанию", "Рабочий", "Без кода статуса"] {
        assert!(got.contains(&n.to_string()), "{n} потерян: {got:?}");
    }
    assert_eq!(
        by_name(&dto, "К списанию").status_name,
        status_name(&ctx, "на_списание").await
    );
    assert_eq!(
        by_name(&dto, "К списанию").status_code.as_deref(),
        Some("на_списание")
    );
    assert_eq!(by_name(&dto, "Без кода статуса").status_code, None);
    assert_eq!(
        by_name(&dto, "Без кода статуса").status_name,
        "Особый статус"
    );

    // Область не менялась: списанное устройство остаётся в employee_device_ids.
    let scope_has_wo = ctx
        .writer
        .execute(move |conn| {
            let repo = trackly_infra::repos::repairs_sqlite::SqliteRepairRepository;
            Ok(repo.employee_device_ids(conn, ivanov)?.contains(&wo))
        })
        .await
        .expect("scope");
    assert!(scope_has_wo, "фильтр «Списано» не должен менять область");
}

// ---------------------------------------------------------------- C: АРМ и происхождение

#[tokio::test]
async fn employee_devices_arm_names_and_printer_origin() {
    let (ctx, _dir) = make_test_ctx().await;
    let ivanov = seed_user(&ctx, "ivanov", "Иванов И.И.").await;
    let t = seed_arm_type(&ctx).await;
    let g1 = seed_group(&ctx, &t, "АРМ #1", None).await;
    let sub = seed_group(&ctx, &t, "Системный блок", Some(g1)).await;
    let g2 = seed_group(&ctx, &t, "АРМ #2", None).await;
    bind_user(&ctx, &t, g1, ivanov).await;
    bind_user(&ctx, &t, g2, ivanov).await;

    let pc = seed_device(&ctx, 1, "ПК-1", "INV-1", None, None).await;
    let sub_dev = seed_device(&ctx, 1, "Монитор вложенный", "INV-2", None, None).await;
    let shared = seed_device(&ctx, 2, "Принтер 2-го этажа", "INV-3", None, None).await;
    let both = seed_device(&ctx, 2, "Принтер член и USB", "INV-4", None, None).await;
    let link = seed_device(&ctx, 2, "Принтер по ссылке", "INV-5", None, None).await;
    let only_member = seed_device(&ctx, 2, "Принтер в составе", "INV-6", None, None).await;
    let usb_only = seed_device(&ctx, 2, "Принтер только USB", "INV-7", None, None).await;
    for (d, host) in [
        (shared, None),
        (both, Some(pc)),
        (link, None),
        (only_member, None),
        (usb_only, Some(pc)),
    ] {
        make_printer(&ctx, d, host).await;
    }
    member(&ctx, g1, pc).await;
    member(&ctx, sub, sub_dev).await;
    member(&ctx, g1, shared).await;
    link_device(&ctx, &t, g2, shared).await;
    member(&ctx, g1, both).await;
    link_device(&ctx, &t, g1, link).await;
    member(&ctx, g2, only_member).await;

    let dto = list(&ctx, &emp(ivanov)).await;

    let mut arms = by_name(&dto, "Принтер 2-го этажа").arm_names.clone();
    arms.sort();
    assert_eq!(arms, vec!["АРМ #1", "АРМ #2"], "общий принтер: обе группы");
    assert_eq!(
        dto.items
            .iter()
            .filter(|d| d.name == "Принтер 2-го этажа")
            .count(),
        1,
        "одна строка на устройство"
    );
    assert_eq!(by_name(&dto, "Монитор вложенный").arm_names, vec!["АРМ #1"]);

    assert_eq!(
        by_name(&dto, "Принтер член и USB")
            .printer_origin
            .as_deref(),
        Some("usb")
    );
    assert_eq!(
        by_name(&dto, "Принтер только USB")
            .printer_origin
            .as_deref(),
        Some("usb"),
        "достижим только через USB-хост, но присутствует"
    );
    assert_eq!(
        by_name(&dto, "Принтер по ссылке").printer_origin.as_deref(),
        Some("link"),
        "достижим только по device_refs, но присутствует"
    );
    assert_eq!(
        by_name(&dto, "Принтер 2-го этажа")
            .printer_origin
            .as_deref(),
        Some("link"),
        "ссылка важнее состава"
    );
    assert_eq!(
        by_name(&dto, "Принтер в составе").printer_origin.as_deref(),
        Some("member")
    );
    assert_eq!(by_name(&dto, "ПК-1").printer_origin, None);
    assert!(by_name(&dto, "Принтер в составе").is_printer);
    assert!(!by_name(&dto, "ПК-1").is_printer);
}

// ---------------------------------------------------------------- D: порядок

#[tokio::test]
async fn employee_devices_order_by_type_then_name() {
    let (ctx, _dir) = make_test_ctx().await;
    let ivanov = seed_user(&ctx, "ivanov", "Иванов И.И.").await;
    let t = seed_arm_type(&ctx).await;
    let g = seed_group(&ctx, &t, "АРМ #1", None).await;
    bind_user(&ctx, &t, g, ivanov).await;
    for (ty, name, inv) in [
        (1, "ПК-Я", "INV-1"),
        (2, "Принтер Я", "INV-2"),
        (1, "ПК-А", "INV-3"),
        (2, "Принтер А", "INV-4"),
    ] {
        let d = seed_device(&ctx, ty, name, inv, None, None).await;
        member(&ctx, g, d).await;
    }
    let dto = list(&ctx, &emp(ivanov)).await;
    // Типы по имени: «Принтер» < «Устройство».
    assert_eq!(
        names(&dto),
        vec!["Принтер А", "Принтер Я", "ПК-А", "ПК-Я"],
        "сначала тип, затем наименование"
    );
}

// ---------------------------------------------------------------- E: метки заявок

#[tokio::test]
async fn employee_devices_foreign_request_number_hidden() {
    let (ctx, _dir) = make_test_ctx().await;
    let ivanov = seed_user(&ctx, "ivanov", "Иванов И.И.").await;
    let petrov = seed_user(&ctx, "petrov", "Петров П.П.").await;
    let t = seed_arm_type(&ctx).await;
    let g1 = seed_group(&ctx, &t, "АРМ #1", None).await;
    let g2 = seed_group(&ctx, &t, "АРМ #2", None).await;
    bind_user(&ctx, &t, g1, ivanov).await;
    bind_user(&ctx, &t, g2, petrov).await;
    let pc = seed_device(&ctx, 1, "ПК-1", "INV-1", None, None).await;
    let prn = seed_device(&ctx, 2, "Принтер 2-го этажа", "INV-2", None, None).await;
    make_printer(&ctx, prn, None).await;
    member(&ctx, g1, pc).await;
    member(&ctx, g1, prn).await;
    // Петров видит те же устройства через ссылки (в области, но не участник).
    link_device(&ctx, &t, g2, pc).await;
    link_device(&ctx, &t, g2, prn).await;

    let repair = add_request(&ctx, "repair", "open", ivanov, pc, NOW).await;
    let cart = add_request(
        &ctx,
        "cartridge_replace",
        "in_progress",
        ivanov,
        prn,
        NOW + 1,
    )
    .await;

    // Автор.
    let a = list(&ctx, &emp(ivanov)).await;
    let pc_a = by_name(&a, "ПК-1");
    assert!(pc_a.in_repair);
    assert_eq!(pc_a.my_repair_request_id, Some(repair));
    assert_eq!(pc_a.my_open_requests.len(), 1);
    assert_eq!(pc_a.my_open_requests[0].id, repair);
    assert!(!pc_a.other_open_request);
    let prn_a = by_name(&a, "Принтер 2-го этажа");
    assert_eq!(prn_a.my_open_requests.len(), 1);
    assert_eq!(prn_a.my_open_requests[0].id, cart);

    // Неучастник: флаг есть, номера нет ни в одном поле.
    let b = list(&ctx, &emp(petrov)).await;
    let pc_b = by_name(&b, "ПК-1");
    assert!(pc_b.in_repair);
    assert!(pc_b.other_open_request);
    assert!(pc_b.my_open_requests.is_empty());
    assert_eq!(pc_b.my_repair_request_id, None);
    assert!(pc_b.my_requests.is_empty());
    let prn_b = by_name(&b, "Принтер 2-го этажа");
    assert!(prn_b.other_open_request, "замена картриджа: флаг");
    assert!(prn_b.my_open_requests.is_empty());
    assert!(prn_b.my_requests.is_empty());
    let exposed = exposed_request_ids(&b);
    assert!(
        !exposed.contains(&repair) && !exposed.contains(&cart),
        "номер чужой заявки раскрыт: {exposed:?}"
    );
    assert!(
        exposed.is_empty(),
        "неучастник не получает номеров: {exposed:?}"
    );

    // Присоединённый участник номер видит.
    add_participant(&ctx, repair, petrov).await;
    add_participant(&ctx, cart, petrov).await;
    let b2 = list(&ctx, &emp(petrov)).await;
    assert_eq!(by_name(&b2, "ПК-1").my_repair_request_id, Some(repair));
    assert_eq!(by_name(&b2, "ПК-1").my_open_requests[0].id, repair);
    assert!(!by_name(&b2, "ПК-1").other_open_request);
    assert_eq!(
        by_name(&b2, "Принтер 2-го этажа").my_open_requests[0].id,
        cart
    );
}

// ---------------------------------------------------------------- F: пустые состояния

#[tokio::test]
async fn employee_devices_empty_states() {
    let (ctx, _dir) = make_test_ctx().await;
    let ivanov = seed_user(&ctx, "ivanov", "Иванов И.И.").await;
    let petrov = seed_user(&ctx, "petrov", "Петров П.П.").await;
    let t = seed_arm_type(&ctx).await;
    let empty = seed_group(&ctx, &t, "Пустой АРМ", None).await;
    bind_user(&ctx, &t, empty, petrov).await;

    let none = list(&ctx, &emp(ivanov)).await;
    assert!(!none.has_groups);
    assert!(none.items.is_empty());

    let no_dev = list(&ctx, &emp(petrov)).await;
    assert!(no_dev.has_groups);
    assert!(no_dev.items.is_empty());

    // statusName из device_statuses.
    let g = seed_group(&ctx, &t, "АРМ #1", None).await;
    bind_user(&ctx, &t, g, ivanov).await;
    let d = seed_device(&ctx, 1, "ПК-1", "INV-1", None, None).await;
    member(&ctx, g, d).await;
    set_status(&ctx, d, "в_работе").await;
    let dto = list(&ctx, &emp(ivanov)).await;
    assert!(dto.has_groups);
    assert_eq!(
        by_name(&dto, "ПК-1").status_name,
        status_name(&ctx, "в_работе").await
    );
}

// ---------------------------------------------------------------- G: форма DTO / BOPLA

fn collect_keys(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::Object(m) => {
            for (k, x) in m {
                out.push(k.clone());
                collect_keys(x, out);
            }
        }
        Value::Array(a) => a.iter().for_each(|x| collect_keys(x, out)),
        _ => {}
    }
}

#[tokio::test]
async fn employee_devices_dto_shape_has_no_network_fields() {
    let (ctx, _dir) = make_test_ctx().await;
    let ivanov = seed_user(&ctx, "ivanov", "Иванов И.И.").await;
    let t = seed_arm_type(&ctx).await;
    let g = seed_group(&ctx, &t, "АРМ #1", None).await;
    bind_user(&ctx, &t, g, ivanov).await;
    let pc = seed_device(&ctx, 1, "ПК-1", "INV-1", None, None).await;
    let prn = seed_device(&ctx, 2, "Принтер 2-го этажа", "INV-2", None, None).await;
    let quiet = seed_device(&ctx, 2, "Принтер без чтений", "INV-3", None, None).await;
    let net = seed_device(&ctx, 2, "Сетевой принтер", "INV-4", None, None).await;
    let pid = make_printer(&ctx, prn, Some(pc)).await;
    make_printer(&ctx, quiet, None).await;
    // У отдельного принтера есть сетевые данные: они не должны попасть в ответ.
    // Показания для проверки «последнего чтения» живут на принтере без адреса:
    // при TRACKLY_SNMP_MOCK=1 (как в ci) опрос дописывает принтеру с адресом
    // своё показание с текущим временем, и сравнение по времени становится гонкой.
    let net_pid = make_printer(&ctx, net, None).await;
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "UPDATE printers SET ip_address = '192.0.2.10', community = 'secret-c' \
                 WHERE id = ?1",
                params![net_pid],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("net data");
    member(&ctx, g, pc).await;
    member(&ctx, g, prn).await;
    member(&ctx, g, quiet).await;
    member(&ctx, g, net).await;
    // Время показаний — текущее, а не NOW (2023): фоновая задача хранения
    // показаний удаляет старые чтения, и тонер пропадал из ответа (гонка).
    let fresh = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_secs() as i64
        - 60;
    add_reading(&ctx, pid, fresh + 10, r#"{"black":10}"#).await;
    add_reading(&ctx, pid, fresh + 20, r#"{"black":45,"cyan":-2}"#).await;

    let dto = list(&ctx, &emp(ivanov)).await;
    let v = serde_json::to_value(&dto).expect("json");
    let mut keys = Vec::new();
    collect_keys(&v, &mut keys);
    assert!(
        keys.iter().any(|k| k == "toner"),
        "тонер должен быть в ответе"
    );
    for k in &keys {
        let l = k.to_lowercase();
        assert!(
            !(l.starts_with("ip")
                || l.contains("community")
                || l.contains("snmp")
                || l.starts_with("oid")
                || l.contains("usbhost")
                || l.contains("usb_host")),
            "запрещённый ключ в ответе: {k}"
        );
    }
    let text = serde_json::to_string(&dto).expect("json");
    assert!(!text.contains("192.0.2.10") && !text.contains("secret-c"));

    let toner = by_name(&dto, "Принтер 2-го этажа")
        .toner
        .clone()
        .expect("toner");
    assert_eq!(
        toner.polled_at_utc,
        Some(fresh + 20),
        "берётся последнее чтение"
    );
    let lv = |l: &str| toner.levels.iter().find(|x| x.label == l).unwrap().percent;
    assert_eq!(lv("black"), Some(45));
    assert_eq!(lv("cyan"), None, "отрицательный уровень = неизвестен");
    assert!(by_name(&dto, "Принтер без чтений").toner.is_none());
    assert!(by_name(&dto, "ПК-1").toner.is_none());
}

// ---------------------------------------------------------------- H: история заявок

#[tokio::test]
async fn employee_devices_my_requests_history_limits() {
    let (ctx, _dir) = make_test_ctx().await;
    let ivanov = seed_user(&ctx, "ivanov", "Иванов И.И.").await;
    let petrov = seed_user(&ctx, "petrov", "Петров П.П.").await;
    let t = seed_arm_type(&ctx).await;
    let g1 = seed_group(&ctx, &t, "АРМ #1", None).await;
    let g2 = seed_group(&ctx, &t, "АРМ #2", None).await;
    bind_user(&ctx, &t, g1, ivanov).await;
    bind_user(&ctx, &t, g2, petrov).await;
    let pc = seed_device(&ctx, 1, "ПК-1", "INV-1", None, None).await;
    member(&ctx, g1, pc).await;
    link_device(&ctx, &t, g2, pc).await;

    let mut closed = Vec::new();
    for i in 0..12 {
        closed.push(add_request(&ctx, "repair", "completed", ivanov, pc, NOW + i).await);
    }
    let open = add_request(&ctx, "repair", "open", ivanov, pc, NOW + 100).await;
    let foreign_closed = add_request(&ctx, "repair", "completed", petrov, pc, NOW + 50).await;
    let foreign_open = add_request(&ctx, "cartridge_replace", "open", petrov, pc, NOW + 60).await;

    let dto = list(&ctx, &emp(ivanov)).await;
    let item = by_name(&dto, "ПК-1");
    let ids: Vec<i64> = item.my_requests.iter().map(|r| r.id).collect();
    assert_eq!(ids.len(), 11, "открытая + не более 10 закрытых: {ids:?}");
    assert_eq!(ids[0], open, "открытые сверху");
    assert!(!ids.contains(&foreign_closed) && !ids.contains(&foreign_open));
    assert!(
        !ids.contains(&closed[0]) && !ids.contains(&closed[1]),
        "старейшие отброшены"
    );
    assert!(ids.contains(&closed[11]));
    assert!(
        item.other_open_request,
        "открытая заявка Петрова: только флаг"
    );
}

// ------------------------------------------- NEW-5 / INT-31: один признак принтера

/// quick-261121-heh (NEW-5): `is_printer` в списке сотрудника и варианты
/// принтеров заявки считаются по ОДНОМУ правилу — `devices.type_id` =
/// `PRINTER_TYPE_ID`. Устройство, помеченное сотруднику принтером, всегда есть
/// в вариантах заявки (RequestFormModal предзаполняет его); строка `printers`
/// при type_id «Устройство» (дрейф старых БД) принтером не делает.
#[tokio::test]
async fn employee_is_printer_matches_request_printer_options() {
    let (ctx, _dir) = make_test_ctx().await;
    let ivanov = seed_user(&ctx, "ivanov", "Иванов И.И.").await;
    let room = seed_place(&ctx, "Кабинет 7").await;
    let with_row = seed_device(&ctx, 2, "Принтер А", "INV-P1", Some(room), None).await;
    make_printer(&ctx, with_row, None).await;
    let no_row = seed_device(&ctx, 2, "Принтер Б без строки", "INV-P2", Some(room), None).await;
    let drift = seed_device(&ctx, 1, "Устройство со строкой", "INV-D1", Some(room), None).await;
    make_printer(&ctx, drift, None).await;
    let t = seed_arm_type(&ctx).await;
    let g = seed_group(&ctx, &t, "АРМ #7", None).await;
    bind_user(&ctx, &t, g, ivanov).await;
    for d in [with_row, no_row, drift] {
        member(&ctx, g, d).await;
    }

    let dto = list(&ctx, &emp(ivanov)).await;
    let options =
        trackly_app::tauri_cmds::requests::build_request_printer_options(&ctx, &emp(ivanov))
            .await
            .expect("request_printer_options");
    let option_ids: Vec<i64> = options.iter().map(|o| o.id).collect();

    for item in &dto.items {
        assert_eq!(
            item.is_printer,
            option_ids.contains(&item.id),
            "is_printer «{}» и варианты принтеров заявки должны совпадать",
            item.name
        );
    }
    assert!(by_name(&dto, "Принтер А").is_printer);
    assert!(
        by_name(&dto, "Принтер Б без строки").is_printer,
        "type_id принтера без строки printers — принтер"
    );
    assert!(
        !by_name(&dto, "Устройство со строкой").is_printer,
        "строка printers при type_id «Устройство» — не принтер (признак — type_id)"
    );
}

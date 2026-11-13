//! Phase 41.4, план 47 (D-41): область принтеров сотрудника в форме «Замена картриджа».
//!
//! `RequestService::printer_options` и `RequestService::create` для сотрудника работают
//! в области `employee_device_ids` (та же, что у формы «Ремонт»); персонал видит все.
//! Настоящий `AppCtx::build`. Данные вымышленные: «Иванов И.И.», «Петров П.П.»,
//! «Сидоров С.С.».

mod entities_support;
mod repair_support;

use std::collections::BTreeSet;

use entities_support::fixture::{admin, make_test_ctx, seed_device};
use repair_support::{count, emp, manager, seed_user, seed_user_group};
use rusqlite::params;
use trackly_app::context::AppCtx;
use trackly_app::dto::request::RequestCreateDto;
use trackly_core::error::AppError;
use trackly_infra::error_conversions::map_rusqlite;

const DEVICE: i64 = 1;
const PRINTER: i64 = 2;

async fn seed_place(ctx: &AppCtx, name: &str) -> i64 {
    let name = name.to_string();
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO places (parent_id, kind, name, created_at_utc, updated_at_utc) \
                 VALUES (NULL, 'zone', ?1, 1700000000, 1700000000)",
                params![name],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed place")
}

/// Строка `printers` с USB-хостом: принтер входит в область владельца хоста.
async fn make_usb(ctx: &AppCtx, printer: i64, host: i64) {
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO printers (device_id, usb_host_device_id, created_at_utc, \
                 updated_at_utc) VALUES (?1, ?2, 1700000000, 1700000000)",
                params![printer, host],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("make usb");
}

/// Вложенная группа (того же типа) с составом `devices`.
async fn seed_child_group(ctx: &AppCtx, parent: i64, devices: &[i64]) -> i64 {
    let devices = devices.to_vec();
    ctx.writer
        .execute(move |conn| {
            let t: i64 = conn
                .query_row(
                    "SELECT type_id FROM groups WHERE id = ?1",
                    params![parent],
                    |r| r.get(0),
                )
                .map_err(map_rusqlite)?;
            conn.execute(
                "INSERT INTO groups (type_id, name, seq, version, parent_group_id, \
                 created_at_utc, updated_at_utc) \
                 VALUES (?1, 'Вложенная группа', 2, 1, ?2, 1700000000, 1700000000)",
                params![t, parent],
            )
            .map_err(map_rusqlite)?;
            let g = conn.last_insert_rowid();
            for d in devices {
                conn.execute(
                    "INSERT INTO group_devices (device_id, group_id, added_at_utc) \
                     VALUES (?1, ?2, 1700000000)",
                    params![d, g],
                )
                .map_err(map_rusqlite)?;
            }
            Ok(g)
        })
        .await
        .expect("child group")
}

/// Ссылка `device_refs` группы на устройство (тип группы получает свойство «Устройства»).
async fn add_ref(ctx: &AppCtx, group: i64, device: i64) {
    ctx.writer
        .execute(move |conn| {
            let t: i64 = conn
                .query_row(
                    "SELECT type_id FROM groups WHERE id = ?1",
                    params![group],
                    |r| r.get(0),
                )
                .map_err(map_rusqlite)?;
            let existing: Option<i64> = conn
                .query_row(
                    "SELECT id FROM group_type_properties \
                     WHERE type_id = ?1 AND data_type = 'device_refs'",
                    params![t],
                    |r| r.get(0),
                )
                .ok();
            let prop = match existing {
                Some(p) => p,
                None => {
                    conn.execute(
                        "INSERT INTO group_type_properties (type_id, name, data_type, \
                         created_at_utc, updated_at_utc) \
                         VALUES (?1, 'Устройства', 'device_refs', 1700000000, 1700000000)",
                        params![t],
                    )
                    .map_err(map_rusqlite)?;
                    conn.last_insert_rowid()
                }
            };
            conn.execute(
                "INSERT INTO group_property_values (group_id, property_id, value_ref, \
                 updated_at_utc) VALUES (?1, ?2, ?3, 1700000000)",
                params![group, prop, device],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("add ref");
}

struct World {
    ivanov: i64,
    petrov: i64,
    sidorov: i64,
    // видимые Иванову
    net_a: i64,
    usb_a: i64,
    nested_p: i64,
    ref_p: i64,
    // невидимые Иванову
    net_b: i64,
    usb_b: i64,
    free: i64,
    at_place_a: i64,
    nested_ref_p: i64,
    usb_ref_host: i64,
    non_printer_a: i64,
}

impl World {
    fn visible_to_ivanov(&self) -> BTreeSet<i64> {
        [self.net_a, self.usb_a, self.nested_p, self.ref_p]
            .into_iter()
            .collect()
    }
    fn all_printers(&self) -> BTreeSet<i64> {
        [
            self.net_a,
            self.usb_a,
            self.nested_p,
            self.ref_p,
            self.net_b,
            self.usb_b,
            self.free,
            self.at_place_a,
            self.nested_ref_p,
            self.usb_ref_host,
        ]
        .into_iter()
        .collect()
    }
}

async fn world(ctx: &AppCtx) -> World {
    let ivanov = seed_user(ctx, "ivanov", "Иванов И.И.").await;
    let petrov = seed_user(ctx, "petrov", "Петров П.П.").await;
    let sidorov = seed_user(ctx, "sidorov", "Сидоров С.С.").await;
    let pa = seed_place(ctx, "Кабинет А").await;
    let pb = seed_place(ctx, "Кабинет Б").await;
    let pf = seed_place(ctx, "Склад").await;

    // Группа А: системный блок + сетевой принтер в составе + USB-принтер хоста А.
    let pc_a = seed_device(ctx, DEVICE, "Системный блок А", "INV-A1", Some(pa), None).await;
    let net_a = seed_device(ctx, PRINTER, "Принтер А-сеть", "INV-A2", Some(pa), None).await;
    let usb_a = seed_device(ctx, PRINTER, "Принтер А-USB", "INV-A3", Some(pa), None).await;
    make_usb(ctx, usb_a, pc_a).await;
    let ga = seed_user_group(ctx, ivanov, "Комплект Иванова", &[pc_a, net_a]).await;

    // Вложенная группа А с принтером; ссылка device_refs у самой А на принтер.
    let nested_p = seed_device(ctx, PRINTER, "Принтер вложенный", "INV-A4", Some(pa), None).await;
    let nested_ref_p = seed_device(
        ctx,
        PRINTER,
        "Принтер ссылка во вложенной",
        "INV-A5",
        Some(pf),
        None,
    )
    .await;
    let ref_p = seed_device(ctx, PRINTER, "Принтер ссылка в А", "INV-A6", Some(pf), None).await;
    let child = seed_child_group(ctx, ga, &[nested_p]).await;
    add_ref(ctx, child, nested_ref_p).await; // (б) ссылка вложенной группы
    add_ref(ctx, ga, ref_p).await;

    // (в) USB-принтер, хост которого только ссылка в А, а не состав.
    let usb_ref_host_pc = seed_device(
        ctx,
        DEVICE,
        "Системный блок по ссылке",
        "INV-A7",
        Some(pf),
        None,
    )
    .await;
    add_ref(ctx, ga, usb_ref_host_pc).await;
    let usb_ref_host = seed_device(
        ctx,
        PRINTER,
        "Принтер USB хост-ссылка",
        "INV-A8",
        Some(pf),
        None,
    )
    .await;
    make_usb(ctx, usb_ref_host, usb_ref_host_pc).await;

    // (а) сетевой принтер на месте группы А, но вне состава.
    let at_place_a = seed_device(
        ctx,
        PRINTER,
        "Принтер на месте А вне состава",
        "INV-A9",
        Some(pa),
        None,
    )
    .await;

    // Не принтер в составе А.
    let non_printer_a = pc_a;

    // Группа Б (Петров).
    let pc_b = seed_device(ctx, DEVICE, "Системный блок Б", "INV-B1", Some(pb), None).await;
    let net_b = seed_device(ctx, PRINTER, "Принтер Б-сеть", "INV-B2", Some(pb), None).await;
    let usb_b = seed_device(ctx, PRINTER, "Принтер Б-USB", "INV-B3", Some(pb), None).await;
    make_usb(ctx, usb_b, pc_b).await;
    seed_user_group(ctx, petrov, "Комплект Петрова", &[pc_b, net_b]).await;

    // Свободный принтер без группы.
    let free = seed_device(ctx, PRINTER, "Принтер свободный", "INV-F1", Some(pf), None).await;

    World {
        ivanov,
        petrov,
        sidorov,
        net_a,
        usb_a,
        nested_p,
        ref_p,
        net_b,
        usb_b,
        free,
        at_place_a,
        nested_ref_p,
        usb_ref_host,
        non_printer_a,
    }
}

fn ids(opts: &[trackly_app::dto::request::RequestPrinterOptionDto]) -> BTreeSet<i64> {
    opts.iter().map(|o| o.id).collect()
}

fn dto(kind: &str, printer: Option<i64>) -> RequestCreateDto {
    RequestCreateDto {
        request_type: kind.to_string(),
        printer_device_id: printer.map(|p| p as i32),
        cartridge_model_id: None,
        category_id: None,
        description: if kind == "free_form" {
            Some("Нужна помощь".to_string())
        } else {
            None
        },
    }
}

async fn rows(ctx: &AppCtx) -> (i64, i64) {
    (
        count(ctx, "SELECT COUNT(*) FROM requests WHERE ?1 = ?1", 0).await,
        count(ctx, "SELECT COUNT(*) FROM audit_log WHERE ?1 = ?1", 0).await,
    )
}

#[tokio::test]
async fn employee_sees_only_printers_of_own_scope() {
    let (ctx, _dir) = make_test_ctx().await;
    let w = world(&ctx).await;
    let got = ctx
        .requests
        .printer_options(&emp(w.ivanov))
        .await
        .expect("options");
    assert_eq!(ids(&got), w.visible_to_ivanov(), "{got:?}");
}

#[tokio::test]
async fn employee_scope_excludes_foreign_free_and_ambiguous_printers() {
    let (ctx, _dir) = make_test_ctx().await;
    let w = world(&ctx).await;
    let got = ids(&ctx
        .requests
        .printer_options(&emp(w.ivanov))
        .await
        .expect("options"));
    // Чужие и свободный.
    for (id, why) in [
        (w.net_b, "сетевой принтер группы Б"),
        (w.usb_b, "USB-принтер группы Б"),
        (w.free, "принтер без группы"),
        // D-41 (а): на месте группы, но вне её состава.
        (w.at_place_a, "сетевой принтер на месте группы вне состава"),
        // D-41 (б): ссылка device_refs вложенной группы не доходит до родительской.
        (w.nested_ref_p, "принтер по ссылке из вложенной группы"),
        // D-41 (в): USB-принтер, хост которого лишь ссылка.
        (w.usb_ref_host, "USB-принтер с хостом-ссылкой"),
    ] {
        assert!(!got.contains(&id), "не должен быть виден: {why}");
    }
    // Не принтер из состава не попадает в список.
    assert!(!got.contains(&w.non_printer_a));
}

#[tokio::test]
async fn employee_without_groups_gets_empty_list() {
    let (ctx, _dir) = make_test_ctx().await;
    let w = world(&ctx).await;
    let got = ctx
        .requests
        .printer_options(&emp(w.sidorov))
        .await
        .expect("options");
    assert!(got.is_empty(), "{got:?}");
}

#[tokio::test]
async fn employee_without_user_id_is_forbidden() {
    let (ctx, _dir) = make_test_ctx().await;
    let _ = world(&ctx).await;
    let nobody = trackly_core::auth::Identity {
        user_id: None,
        role: trackly_core::auth::Role::Employee,
    };
    let err = ctx
        .requests
        .printer_options(&nobody)
        .await
        .expect_err("no user");
    assert!(matches!(err, AppError::Forbidden), "{err:?}");
    let err = ctx
        .requests
        .create(dto("cartridge_replace", None), &nobody)
        .await
        .expect_err("no user, validation first or forbidden");
    assert!(
        matches!(err, AppError::Forbidden | AppError::Validation { .. }),
        "{err:?}"
    );
    let err = ctx
        .requests
        .create(dto("free_form", None), &nobody)
        .await
        .expect_err("create without user_id");
    assert!(matches!(err, AppError::Forbidden), "{err:?}");
}

#[tokio::test]
async fn staff_sees_all_printers() {
    let (ctx, _dir) = make_test_ctx().await;
    let w = world(&ctx).await;
    let all = w.all_printers();
    let by_admin = ctx.requests.printer_options(&admin()).await.expect("admin");
    assert_eq!(ids(&by_admin), all);
    let by_manager = ctx
        .requests
        .printer_options(&manager(w.petrov))
        .await
        .expect("manager");
    assert_eq!(ids(&by_manager), all);
}

#[tokio::test]
async fn employee_creates_on_own_scope_printers() {
    let (ctx, _dir) = make_test_ctx().await;
    let w = world(&ctx).await;
    for p in [w.net_a, w.usb_a, w.nested_p, w.ref_p] {
        let r = ctx
            .requests
            .create(dto("cartridge_replace", Some(p)), &emp(w.ivanov))
            .await
            .unwrap_or_else(|e| panic!("create on {p}: {e:?}"));
        assert_eq!(r.printer_device_id, Some(p));
    }
}

#[tokio::test]
async fn employee_cannot_create_on_foreign_free_or_missing_printer() {
    let (ctx, _dir) = make_test_ctx().await;
    let w = world(&ctx).await;
    let before = rows(&ctx).await;
    for p in [
        w.net_b,
        w.usb_b,
        w.free,
        w.at_place_a,
        w.nested_ref_p,
        w.usb_ref_host,
        9_999_999,
    ] {
        let err = ctx
            .requests
            .create(dto("cartridge_replace", Some(p)), &emp(w.ivanov))
            .await
            .expect_err("foreign printer");
        assert!(matches!(err, AppError::Forbidden), "{p}: {err:?}");
    }
    assert_eq!(
        rows(&ctx).await,
        before,
        "отказ не пишет ни requests, ни audit_log"
    );
}

#[tokio::test]
async fn free_form_checks_printer_scope_too() {
    let (ctx, _dir) = make_test_ctx().await;
    let w = world(&ctx).await;
    let before = rows(&ctx).await;
    let err = ctx
        .requests
        .create(dto("free_form", Some(w.net_b)), &emp(w.ivanov))
        .await
        .expect_err("free_form on foreign printer");
    assert!(matches!(err, AppError::Forbidden), "{err:?}");
    assert_eq!(rows(&ctx).await, before);
    ctx.requests
        .create(dto("free_form", None), &emp(w.ivanov))
        .await
        .expect("free_form without printer");
    ctx.requests
        .create(dto("free_form", Some(w.net_a)), &emp(w.ivanov))
        .await
        .expect("free_form on own printer");
}

#[tokio::test]
async fn staff_creates_on_any_printer() {
    let (ctx, _dir) = make_test_ctx().await;
    let w = world(&ctx).await;
    ctx.requests
        .create(dto("cartridge_replace", Some(w.net_b)), &manager(w.petrov))
        .await
        .expect("manager");
    ctx.requests
        .create(dto("cartridge_replace", Some(w.free)), &admin())
        .await
        .expect("admin");
    // Фикстура использует поле `ivanov` только для сотрудника; убеждаемся, что оно значимо.
    assert_ne!(w.ivanov, w.petrov);
}

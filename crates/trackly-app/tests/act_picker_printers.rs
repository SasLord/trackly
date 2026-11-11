//! Phase 41.4, план 26 (gap 7, UAT 14): пикер устройств акта и принтеры.
//!
//! Тесты вызывают ровно те функции и с теми же полями `DeviceFilter`, что и
//! `ui/src/features/acts/ActFormItemsTable.svelte` (`fetchStorage` и `fetchMembers`),
//! на настоящем `AppCtx`. Принтер из обнаружения создаётся тем же путём, что в
//! `tauri_cmds/printers.rs` (`create_with_printer_block`, type_id 2, статус «На складе»,
//! без места). Данные вымышленные (адреса из 192.0.2.0/24).

mod entities_support;
mod repair_support;

use entities_support::fixture::{
    admin, join_group, make_test_ctx, seed_device, seed_group_type_and_group, seed_place,
};
use repair_support::{manager, seed_user};
use rusqlite::params;
use trackly_app::context::AppCtx;
use trackly_app::dto::act::{ActCreateDto, ActItemNewDto};
use trackly_app::dto::device::{
    DeviceFilter, DeviceGroup, DeviceNew, DeviceSaveOutcome, Pagination,
};
use trackly_app::dto::number_template::NumberFieldInput;
use trackly_app::dto::printer::PrinterCreateBlockDto;
use trackly_app::dto::repair::RepairCreateDto;
use trackly_infra::error_conversions::map_rusqlite;

const PRINTER_TYPE: i64 = 2;

async fn status_id(ctx: &AppCtx, code: &'static str) -> i64 {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(
                "SELECT id FROM device_statuses WHERE code = ?1",
                params![code],
                |r| r.get::<_, i64>(0),
            )
            .map_err(map_rusqlite)
        })
        .await
        .unwrap_or_else(|e| panic!("статус {code}: {e:?}"))
}

/// Принтер так, как его заводит обнаружение (admit_printer).
async fn discovered_printer(ctx: &AppCtx, name: &str, model: &str, ip: &str) -> i64 {
    let out = ctx
        .devices
        .create_with_printer_block(
            DeviceNew {
                type_id: PRINTER_TYPE,
                name: name.into(),
                inventory_no: None,
                serial_no: None,
                model: Some(model.into()),
                specs: None,
                kit: None,
                state: None,
                place_id: None,
                status_id: 1,
            },
            Some(PrinterCreateBlockDto {
                ip_address: Some(ip.into()),
                community: Some("public".into()),
                usb_host_device_id: None,
            }),
        )
        .await
        .expect("create printer");
    match out {
        DeviceSaveOutcome::Created(d) => d.id,
        other => panic!("неожиданный исход: {other:?}"),
    }
}

async fn set_status(ctx: &AppCtx, device_id: i64, status: i64) {
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "UPDATE devices SET status_id = ?1 WHERE id = ?2",
                params![status, device_id],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("set status");
}

/// `fetchStorage` пикера: «Устройства со склада».
async fn picker_storage(ctx: &AppCtx, query: &str) -> Vec<DeviceGroup> {
    picker_storage_typed(ctx, query, None).await
}

/// Тот же запрос с типом (поиск по названию типа в пикере, `storageGroups(.., typeId)`).
async fn picker_storage_typed(ctx: &AppCtx, query: &str, type_id: Option<i64>) -> Vec<DeviceGroup> {
    let storage = status_id(ctx, "на_складе").await;
    ctx.devices
        .list_grouped(
            DeviceFilter {
                type_id,
                place_id: None,
                status_id: Some(storage),
                state: None,
                name_prefix: Some(query.to_string()),
                include_deleted: false,
                group_by_condition: true,
                hide_anchor_members: false,
                anchors_only: false,
                exclude_anchors: false,
                member_of_group_id: None,
                group_members_only: false,
            },
            Pagination {
                offset: 0,
                limit: 20,
            },
        )
        .await
        .expect("list_grouped")
}

/// `fetchMembers` пикера: «Устройства в составе групп».
async fn picker_members(ctx: &AppCtx, query: &str) -> Vec<i64> {
    ctx.devices
        .list(
            DeviceFilter {
                type_id: None,
                place_id: None,
                status_id: None,
                state: None,
                name_prefix: Some(query.to_string()),
                include_deleted: false,
                group_by_condition: false,
                hide_anchor_members: false,
                anchors_only: false,
                exclude_anchors: false,
                member_of_group_id: None,
                group_members_only: true,
            },
            Pagination {
                offset: 0,
                limit: 20,
            },
        )
        .await
        .expect("list")
        .items
        .into_iter()
        .map(|d| d.id)
        .collect()
}

fn contains(groups: &[DeviceGroup], id: i64) -> bool {
    groups
        .iter()
        .any(|g| g.ids.iter().any(|&i| i64::from(i) == id))
}

// Тест 1: принтер из обнаружения виден складской секции и пустым запросом, и по имени.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn discovered_printer_visible_in_storage_section() {
    let (ctx, _dir) = make_test_ctx().await;
    let p = discovered_printer(
        &ctx,
        "HP LaserJet Pro M404n",
        "HP LaserJet Pro M404n",
        "192.0.2.10",
    )
    .await;

    let empty = picker_storage(&ctx, "").await;
    println!(
        "H1/пустой запрос: {:?}",
        empty.iter().map(|g| &g.repr.name).collect::<Vec<_>>()
    );
    assert!(contains(&empty, p), "принтер не виден при пустом запросе");

    for q in ["HP", "HP LaserJet", "M404n", "LaserJet Pro"] {
        let hits = picker_storage(&ctx, q).await;
        println!(
            "поиск {q:?}: {} свёрток, найден={}",
            hits.len(),
            contains(&hits, p)
        );
        assert!(contains(&hits, p), "принтер не найден по запросу {q:?}");
    }
}

// Тест 2 (H2): 25 свёрток других устройств с большим count вытесняют единичный принтер.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn printer_among_many_collapsed_groups() {
    let (ctx, _dir) = make_test_ctx().await;
    let p = discovered_printer(
        &ctx,
        "HP LaserJet Pro M404n",
        "HP LaserJet Pro M404n",
        "192.0.2.11",
    )
    .await;
    for n in 0..25 {
        for k in 0..3 {
            let name = format!("Монитор тип {n:02}");
            // без инвентарного номера: сворачиваются в одну строку с count 3
            ctx.writer
                .execute(move |conn| {
                    conn.execute(
                        "INSERT INTO devices (type_id, name, model, status_id, created_at_utc, \
                         updated_at_utc, version) VALUES (1, ?1, 'M', 1, 1700000000, 1700000000, 1)",
                        params![name],
                    )
                    .map_err(map_rusqlite)?;
                    Ok(k)
                })
                .await
                .expect("seed monitor");
        }
    }

    let empty = picker_storage(&ctx, "").await;
    println!(
        "H2/пустой запрос: принтер виден = {} (из {} свёрток)",
        contains(&empty, p),
        empty.len()
    );
    let by_name = picker_storage(&ctx, "LaserJet").await;
    println!(
        "H2/поиск LaserJet: принтер виден = {}",
        contains(&by_name, p)
    );
    assert!(
        contains(&by_name, p),
        "поиск по имени модели не находит принтер"
    );
    // H2 ПОДТВЕРЖДЕНА: при пустом запросе единичный принтер вытеснен 20 свёртками
    // с большим count (ORDER BY cnt DESC LIMIT 20). Фиксируем фактическое поведение
    // и проверяем исправление пикера: запрос с типом находит принтер.
    assert!(
        !contains(&empty, p),
        "если это утверждение упало, пустой запрос стал показывать принтер: обновить тест"
    );
    let typed = picker_storage_typed(&ctx, "", Some(PRINTER_TYPE)).await;
    assert!(
        contains(&typed, p),
        "запрос с типом «Принтер» не нашёл принтер"
    );
    assert!(typed.iter().all(|g| g.repr.type_id == PRINTER_TYPE));
}

// Тест 3 (H1): принтер «В работе» вне групп пикером не виден, сервер акт отклоняет.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn working_printer_outside_groups_not_in_picker_and_act_rejected() {
    let (ctx, _dir) = make_test_ctx().await;
    let p = discovered_printer(
        &ctx,
        "HP LaserJet Pro M404n",
        "HP LaserJet Pro M404n",
        "192.0.2.12",
    )
    .await;
    let working = status_id(&ctx, "в_работе").await;
    set_status(&ctx, p, working).await;

    let storage = picker_storage(&ctx, "").await;
    let members = picker_members(&ctx, "").await;
    println!(
        "H1: склад видит={}, члены групп видят={}",
        contains(&storage, p),
        members.contains(&p)
    );
    assert!(!contains(&storage, p));
    assert!(!members.contains(&p));

    let room = seed_place(&ctx, "Кабинет 12").await;
    let res = ctx
        .acts
        .create(
            &admin(),
            ActCreateDto {
                number_input: NumberFieldInput {
                    value: "9001".into(),
                    template_id: None,
                    confirm_mismatch: false,
                    confirm_script_mix: false,
                },
                giver_name: "Иванов И.И.".into(),
                receiver_name: "Петров П.П.".into(),
                place_id: Some(room),
                notes: None,
                deadline_utc: None,
                handover_date_utc: None,
                // Пикер передаёт состав свёртки в device_ids (канонический путь).
                items: vec![ActItemNewDto {
                    device_id: p,
                    device_ids: vec![p],
                    quantity: 1,
                }],
                confirm_group_release: false,
            },
        )
        .await;
    println!("H1: создание акта на принтер «В работе» -> {res:?}");
    assert!(
        res.is_err(),
        "сервер обязан отклонить устройство вне склада и групп"
    );
}

// Тест 4 (H3): принтер — член группы (АРМ) виден секции «в составе групп» при любом статусе.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn group_member_printer_visible_in_members_section() {
    let (ctx, _dir) = make_test_ctx().await;
    let p = discovered_printer(
        &ctx,
        "HP LaserJet Pro M404n",
        "HP LaserJet Pro M404n",
        "192.0.2.13",
    )
    .await;
    let working = status_id(&ctx, "в_работе").await;
    set_status(&ctx, p, working).await;
    let (_t, g) = seed_group_type_and_group(&ctx, "АРМ-тест", 1, None).await;
    join_group(&ctx, p, g).await;

    let members = picker_members(&ctx, "").await;
    println!("H3: члены групп = {members:?}");
    assert!(members.contains(&p));
    assert!(picker_members(&ctx, "LaserJet").await.contains(&p));
    // Контроль: другое устройство в группе тоже видно.
    let pc = seed_device(&ctx, 1, "ПК-00014", "INV-14", None, Some(g)).await;
    assert!(picker_members(&ctx, "").await.contains(&pc));
}

// Тест 5 (D-22): принтер с открытой заявкой возвращается пикером и помечается.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn printer_with_open_repair_is_listed_and_flagged() {
    let (ctx, _dir) = make_test_ctx().await;
    let p = discovered_printer(
        &ctx,
        "HP LaserJet Pro M404n",
        "HP LaserJet Pro M404n",
        "192.0.2.14",
    )
    .await;
    let requester = seed_user(&ctx, "requester", "Сидоров С.С.").await;
    ctx.repairs
        .create(
            RepairCreateDto {
                device_id: Some(p),
                description: "Не печатает".into(),
            },
            &manager(requester),
        )
        .await
        .expect("create repair");

    assert!(contains(&picker_storage(&ctx, "").await, p));
    let open = ctx
        .repairs
        .open_requests_for_devices(vec![p], &admin())
        .await
        .expect("open_requests_for_devices");
    assert!(
        open.iter().any(|r| r.device_id == p),
        "заявка не помечена: {open:?}"
    );
}

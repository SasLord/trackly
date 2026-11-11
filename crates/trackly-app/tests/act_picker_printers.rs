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
use trackly_app::dto::act::{
    ActCreateDto, ActDto, ActItemNewDto, ActNumberEditInput, ActReturnDto, ActReturnItemDto,
    ActSaveOutcome, ActUpdateDto, ActUpdateItemDto,
};
use trackly_app::dto::device::{
    DeviceFilter, DeviceGroup, DeviceNew, DeviceSaveOutcome, Pagination,
};
use trackly_app::dto::number_template::NumberFieldInput;
use trackly_app::dto::printer::PrinterCreateBlockDto;
use trackly_app::dto::repair::RepairCreateDto;
use trackly_core::error::AppError;
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

/// Положить устройство на место с нужным статусом (прямой SQL, как после обнаружения).
async fn put_device(ctx: &AppCtx, id: i64, place: Option<i64>, status: i64) {
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "UPDATE devices SET place_id = ?1, status_id = ?2 WHERE id = ?3",
                params![place, status, id],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("put device");
}

async fn device_state(ctx: &AppCtx, id: i64) -> (Option<i64>, i64) {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(
                "SELECT place_id, status_id FROM devices WHERE id = ?1",
                params![id],
                |r| Ok((r.get::<_, Option<i64>>(0)?, r.get::<_, i64>(1)?)),
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("device state")
}

fn create_dto(number: &str, place: Option<i64>, items: Vec<ActItemNewDto>) -> ActCreateDto {
    ActCreateDto {
        number_input: NumberFieldInput {
            value: number.into(),
            template_id: None,
            confirm_mismatch: false,
            confirm_script_mix: false,
        },
        giver_name: "Иванов И.И.".into(),
        receiver_name: "Петров П.П.".into(),
        place_id: place,
        notes: None,
        deadline_utc: None,
        handover_date_utc: None,
        items,
        confirm_group_release: false,
    }
}

/// Пикер передаёт состав в `device_ids` (канонический путь).
async fn try_create(
    ctx: &AppCtx,
    number: &str,
    ids: &[i64],
    place: Option<i64>,
) -> Result<ActSaveOutcome, AppError> {
    let items = ids
        .iter()
        .map(|&id| ActItemNewDto {
            device_id: id,
            device_ids: vec![id],
            quantity: 1,
        })
        .collect();
    ctx.acts
        .create(&admin(), create_dto(number, place, items))
        .await
}

async fn create_ok(ctx: &AppCtx, number: &str, ids: &[i64], place: Option<i64>) -> ActDto {
    match try_create(ctx, number, ids, place).await.expect("create") {
        ActSaveOutcome::Created(a) => *a,
        other => panic!("ожидали Created, получили {other:?}"),
    }
}

async fn try_update(ctx: &AppCtx, act_id: i64, ids: &[i64]) -> Result<ActSaveOutcome, AppError> {
    let act = ctx.acts.get(act_id).await.expect("get act");
    ctx.acts
        .update(
            &admin(),
            ActUpdateDto {
                id: act.id,
                expected_version: act.version,
                number_input: ActNumberEditInput {
                    value: act.number_raw.clone(),
                    confirm_script_mix: false,
                },
                giver_name: act.giver_name.clone(),
                receiver_name: act.receiver_name.clone(),
                place_id: act.place_id,
                notes: act.notes.clone(),
                deadline_utc: act.deadline_utc,
                handover_date_utc: None,
                items: ids
                    .iter()
                    .map(|&id| ActUpdateItemDto {
                        device_id: id,
                        complectation_at_time: None,
                    })
                    .collect(),
                confirm_group_release: false,
            },
        )
        .await
}

async fn update_ok(ctx: &AppCtx, act_id: i64, ids: &[i64]) {
    match try_update(ctx, act_id, ids).await.expect("update") {
        ActSaveOutcome::Created(_) => {}
        other => panic!("ожидали Created, получили {other:?}"),
    }
}

/// Возврат устройства по акту передачи (место возврата не задано).
async fn try_return(ctx: &AppCtx, act: &ActDto, device: i64) -> Result<ActDto, AppError> {
    let parent = ctx.acts.get(act.id).await.expect("get parent");
    let item = parent
        .items
        .iter()
        .find(|i| i.device_id == device)
        .expect("позиция устройства");
    ctx.acts
        .do_return(
            &admin(),
            act.id,
            ActReturnDto {
                bulk_condition: Some("Хорошее".into()),
                bulk_place_id: None,
                apply_to_all: true,
                giver_name: None,
                receiver_name: None,
                handover_date_utc: None,
                items: vec![ActReturnItemDto {
                    act_item_id: item.id,
                    device_id: device,
                    device_ids: vec![device],
                    quantity: 1,
                    condition_override: None,
                    place_id_override: None,
                }],
            },
        )
        .await
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

// Тест A (D-34, заменяет H1/D-02): принтер «В работе» вне групп передаётся актом,
// откат акта возвращает его на прежнее место и статус «В работе», а не на склад.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn working_printer_outside_groups_handed_over_and_rollback_restores() {
    let (ctx, _dir) = make_test_ctx().await;
    let p = discovered_printer(
        &ctx,
        "HP LaserJet Pro M404n",
        "HP LaserJet Pro M404n",
        "192.0.2.12",
    )
    .await;
    let working = status_id(&ctx, "в_работе").await;
    let origin = seed_place(&ctx, "Кабинет 12").await;
    put_device(&ctx, p, Some(origin), working).await;

    let room = seed_place(&ctx, "Кабинет 7").await;
    let act = create_ok(&ctx, "9001", &[p], Some(room)).await;
    assert_eq!(device_state(&ctx, p).await, (Some(room), working));

    ctx.acts
        .delete_soft(act.id, act.version)
        .await
        .expect("delete_soft");
    let (place, status) = device_state(&ctx, p).await;
    assert_ne!(
        status,
        status_id(&ctx, "на_складе").await,
        "откат вернул на склад"
    );
    assert_eq!(place, Some(origin), "откат потерял место");
    assert_eq!(status, working, "откат потерял статус «В работе»");
}

// Тест B (D-34): правка, удаляющая позицию «В работе», возвращает прежнее место и статус.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_removing_working_item_restores_origin() {
    let (ctx, _dir) = make_test_ctx().await;
    let working = status_id(&ctx, "в_работе").await;
    let origin = seed_place(&ctx, "Кабинет 12").await;
    let stock = seed_device(&ctx, 1, "Монитор-01", "INV-B1", None, None).await;
    let w = seed_device(&ctx, 1, "Монитор-02", "INV-B2", None, None).await;
    put_device(&ctx, w, Some(origin), working).await;
    let room = seed_place(&ctx, "Кабинет 7").await;
    let act = create_ok(&ctx, "9002", &[stock, w], Some(room)).await;
    assert_eq!(device_state(&ctx, w).await, (Some(room), working));

    update_ok(&ctx, act.id, &[stock]).await;
    assert_eq!(
        device_state(&ctx, w).await,
        (Some(origin), working),
        "правка не вернула прежние место и статус"
    );
    assert_eq!(device_state(&ctx, stock).await.0, Some(room));
}

// Тест C (D-34): добавление в правке устройства «В работе» вне групп принимается.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_adding_working_device_accepted() {
    let (ctx, _dir) = make_test_ctx().await;
    let working = status_id(&ctx, "в_работе").await;
    let origin = seed_place(&ctx, "Кабинет 12").await;
    let stock = seed_device(&ctx, 1, "Монитор-03", "INV-C1", None, None).await;
    let w = seed_device(&ctx, 1, "Монитор-04", "INV-C2", None, None).await;
    put_device(&ctx, w, Some(origin), working).await;
    let room = seed_place(&ctx, "Кабинет 7").await;
    let act = create_ok(&ctx, "9003", &[stock], Some(room)).await;

    update_ok(&ctx, act.id, &[stock, w]).await;
    assert_eq!(device_state(&ctx, w).await, (Some(room), working));
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

async fn count(ctx: &AppCtx, table: &'static str) -> i64 {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
                .map_err(map_rusqlite)
        })
        .await
        .expect("count")
}

// Тест D (D-34): вне групп допустимы только «На складе» и «В работе».
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn statuses_outside_groups_other_than_storage_and_working_rejected() {
    let (ctx, _dir) = make_test_ctx().await;
    let origin = seed_place(&ctx, "Кабинет 12").await;
    let room = seed_place(&ctx, "Кабинет 7").await;
    for (n, code) in ["на_ремонте", "в_ремонте", "на_списание", "списано"]
        .into_iter()
        .enumerate()
    {
        let st = status_id(&ctx, code).await;
        let label = status_name(&ctx, code).await;
        let d = seed_device(
            &ctx,
            1,
            &format!("Ноутбук-{n}"),
            &format!("INV-D{n}"),
            None,
            None,
        )
        .await;
        put_device(&ctx, d, Some(origin), st).await;
        let acts_before = count(&ctx, "acts").await;
        let err = try_create(&ctx, &format!("91{n}"), &[d], Some(room))
            .await
            .expect_err("статус не допускает акт");
        let AppError::Conflict { reason } = err else {
            panic!("ожидали Conflict: {err:?}")
        };
        assert!(reason.contains(&label), "нет подписи «{label}»: {reason}");
        assert_eq!(device_state(&ctx, d).await, (Some(origin), st));
        assert_eq!(count(&ctx, "acts").await, acts_before);
    }
}

// Тест E (D-22): открытая заявка отказывает и для «В работе» вне групп.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn working_device_with_open_repair_rejected() {
    let (ctx, _dir) = make_test_ctx().await;
    let working = status_id(&ctx, "в_работе").await;
    let origin = seed_place(&ctx, "Кабинет 12").await;
    let d = seed_device(&ctx, 1, "Ноутбук-E", "INV-E1", None, None).await;
    put_device(&ctx, d, Some(origin), working).await;
    let requester = seed_user(&ctx, "requester_e", "Сидоров С.С.").await;
    ctx.repairs
        .create(
            RepairCreateDto {
                device_id: Some(d),
                description: "Не включается".into(),
            },
            &manager(requester),
        )
        .await
        .expect("create repair");
    let room = seed_place(&ctx, "Кабинет 7").await;
    let err = try_create(&ctx, "9201", &[d], Some(room))
        .await
        .expect_err("заявка открыта");
    let AppError::Conflict { reason } = err else {
        panic!("ожидали Conflict: {err:?}")
    };
    assert!(reason.contains("Открыта заявка на ремонт"), "{reason}");
}

/// Позиция legacy-пути: `device_ids` пуст.
async fn try_create_legacy(
    ctx: &AppCtx,
    number: &str,
    device: i64,
    quantity: i64,
    place: Option<i64>,
) -> Result<ActSaveOutcome, AppError> {
    ctx.acts
        .create(
            &admin(),
            create_dto(
                number,
                place,
                vec![ActItemNewDto {
                    device_id: device,
                    device_ids: vec![],
                    quantity,
                }],
            ),
        )
        .await
}

// Тест F (D-34): проверка допустимости на legacy-пути (пустой device_ids).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn legacy_path_checks_assignability() {
    let (ctx, _dir) = make_test_ctx().await;
    let working = status_id(&ctx, "в_работе").await;
    let origin = seed_place(&ctx, "Кабинет 12").await;
    let room = seed_place(&ctx, "Кабинет 7").await;

    // quantity 1, «Списано» -> отказ.
    let written_off = status_id(&ctx, "списано").await;
    let d1 = seed_device(&ctx, 1, "Ноутбук-F1", "INV-F1", None, None).await;
    put_device(&ctx, d1, Some(origin), written_off).await;
    let err = try_create_legacy(&ctx, "9301", d1, 1, Some(room))
        .await
        .expect_err("списанное не передаётся");
    assert!(matches!(err, AppError::Conflict { .. }), "{err:?}");
    assert_eq!(device_state(&ctx, d1).await, (Some(origin), written_off));

    // quantity 1, «В работе» вне групп -> принято.
    let d2 = seed_device(&ctx, 1, "Ноутбук-F2", "INV-F2", None, None).await;
    put_device(&ctx, d2, Some(origin), working).await;
    try_create_legacy(&ctx, "9302", d2, 1, Some(room))
        .await
        .expect("в работе, quantity 1");
    assert_eq!(device_state(&ctx, d2).await, (Some(room), working));

    // quantity 2, «В работе» -> отказ, клон не создан.
    let d3 = seed_device(&ctx, 1, "Ноутбук-F3", "INV-F3", None, None).await;
    put_device(&ctx, d3, Some(origin), working).await;
    let devices_before = count(&ctx, "devices").await;
    let err = try_create_legacy(&ctx, "9303", d3, 2, Some(room))
        .await
        .expect_err("клонировать можно только складское");
    assert!(matches!(err, AppError::Conflict { .. }), "{err:?}");
    assert_eq!(count(&ctx, "devices").await, devices_before);

    // quantity 2, складское -> принято, клон создан.
    let d4 = seed_device(&ctx, 1, "Ноутбук-F4", "INV-F4", None, None).await;
    try_create_legacy(&ctx, "9304", d4, 2, Some(room))
        .await
        .expect("складское, quantity 2");
    assert_eq!(count(&ctx, "devices").await, devices_before + 2);
}

// Сценарий G–K: D на складе, A1 передаёт его на «Кабинет 12», A2 - на «Кабинет 7».
struct Chain {
    d: i64,
    other: i64,
    a1: ActDto,
    a2: ActDto,
    room1: i64,
    room2: i64,
}

async fn chain(ctx: &AppCtx) -> Chain {
    let room1 = seed_place(ctx, "Кабинет 12").await;
    let room2 = seed_place(ctx, "Кабинет 7").await;
    let d = seed_device(ctx, 1, "Принтер-цепь", "INV-G1", None, None).await;
    let other = seed_device(ctx, 1, "Монитор-цепь", "INV-G2", None, None).await;
    let a1 = create_ok(ctx, "9401", &[d, other], Some(room1)).await;
    let a2 = create_ok(ctx, "9402", &[d], Some(room2)).await;
    Chain {
        d,
        other,
        a1,
        a2,
        room1,
        room2,
    }
}

fn assert_superseded(err: AppError, a2_number: &str) {
    let AppError::Conflict { reason } = err else {
        panic!("ожидали Conflict: {err:?}")
    };
    assert!(
        reason.contains(a2_number),
        "нет номера позднего акта {a2_number}: {reason}"
    );
}

// Тест G: удаление раннего акта отклоняется.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn early_act_delete_rejected_when_superseded() {
    let (ctx, _dir) = make_test_ctx().await;
    let c = chain(&ctx).await;
    let working = status_id(&ctx, "в_работе").await;
    let err = ctx
        .acts
        .delete_soft(c.a1.id, c.a1.version)
        .await
        .expect_err("ранний акт затёр бы поздний");
    assert_superseded(err, &c.a2.number);
    assert_eq!(device_state(&ctx, c.d).await, (Some(c.room2), working));
    ctx.acts.get(c.a1.id).await.expect("A1 жив");
}

// Тест H: правка раннего акта, убирающая устройство, отклоняется.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn early_act_update_removing_superseded_rejected() {
    let (ctx, _dir) = make_test_ctx().await;
    let c = chain(&ctx).await;
    let err = try_update(&ctx, c.a1.id, &[c.other])
        .await
        .expect_err("правка раннего акта затёрла бы поздний");
    assert_superseded(err, &c.a2.number);
    assert_eq!(device_state(&ctx, c.d).await.0, Some(c.room2));
}

// Тест I: возврат по раннему акту отклоняется.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn early_act_return_rejected_when_superseded() {
    let (ctx, _dir) = make_test_ctx().await;
    let c = chain(&ctx).await;
    let working = status_id(&ctx, "в_работе").await;
    let err = try_return(&ctx, &c.a1, c.d)
        .await
        .expect_err("возврат по A1 затёр бы A2");
    assert_superseded(err, &c.a2.number);
    assert_eq!(device_state(&ctx, c.d).await, (Some(c.room2), working));
}

// Тест J (контроль): после удаления позднего акта ранний снова управляем.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn early_act_manageable_after_late_act_deleted() {
    let (ctx, _dir) = make_test_ctx().await;
    let c = chain(&ctx).await;
    let working = status_id(&ctx, "в_работе").await;
    let storage = status_id(&ctx, "на_складе").await;
    ctx.acts
        .delete_soft(c.a2.id, c.a2.version)
        .await
        .expect("delete A2");
    assert_eq!(device_state(&ctx, c.d).await, (Some(c.room1), working));
    let a1 = ctx.acts.get(c.a1.id).await.expect("get A1");
    ctx.acts
        .delete_soft(a1.id, a1.version)
        .await
        .expect("delete A1");
    let (_, status) = device_state(&ctx, c.d).await;
    assert_eq!(status, storage);
}

// Тест K (контроль): устройство возвращено по A2 -> A1 не блокируется проверкой «позднего акта».
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn early_act_not_blocked_when_late_act_returned() {
    let (ctx, _dir) = make_test_ctx().await;
    let c = chain(&ctx).await;
    try_return(&ctx, &c.a2, c.d).await.expect("возврат по A2");
    let res = ctx.acts.delete_soft(c.a1.id, c.a1.version).await;
    if let Err(AppError::Conflict { reason }) = &res {
        assert!(
            !reason.contains(&c.a2.number),
            "A1 заблокирован позднего актом, хотя устройство возвращено: {reason}"
        );
    }
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

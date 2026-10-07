//! Phase 41.7, план 08: слой (2) реестрового гейта для `act_service.rs`.
//!
//! На КАЖДУЮ строку `Broadcasts` файла `act_service.rs` приходится ровно один
//! `#[tokio::test] async fn scenario_<функция>`; привязка к реестру идёт по
//! исходнику этого файла (см. `entities_support::scenarios`). Ловушка P4,
//! откаты и массовая операция носят другие префиксы (`p4_`, `rollback_`,
//! `mass_`) и сценариями не считаются.
//!
//! Настоящий `AppCtx::build`, подписка на `ctx.ws_broadcast` ДО мутации,
//! `try_recv()` сразу после `.await`. Списки сравниваются как множества.
//! Фикстуры расходятся намеренно (устройства стоят на разных местах, акт и
//! возврат идут на третьи), чтобы неверный набор id в событии ронял тест.
//! У актов `device_ids` и `group_ids` события пусты всегда (Q4): сверяются
//! `place_ids`, и они обязаны совпасть с `changed_place_ids` ОТВЕТА (D-03).
//! Данные вымышленные.

mod entities_support;

use std::time::Duration;

use entities_support::fixture::{
    admin, assert_no_entities, assert_one_entities, drain, make_test_ctx, seed_device, seed_place,
};
use trackly_app::context::AppCtx;
use trackly_app::dto::act::{
    ActCreateDto, ActDto, ActItemNewDto, ActNumberEditInput, ActReturnDto, ActReturnItemDto,
    ActUpdateDto, ActUpdateItemDto, ActUpdateReturnDto,
};
use trackly_app::dto::number_template::NumberFieldInput;
use trackly_core::error::AppError;

/// Бюджет каждого теста (форма `number_space_broadcast_gate.rs`).
const BUDGET: Duration = Duration::from_secs(60);

/// Устройств в массовой операции (больше ёмкости канала 128, меньше потолка 1000).
const MASS_DEVICES: usize = 220;

fn sorted(mut v: Vec<i64>) -> Vec<i64> {
    v.sort_unstable();
    v.dedup();
    v
}

// ---------------------------------------------------------------------------
// Сидеры актов (локальные копии из group_write_sites.rs)
// ---------------------------------------------------------------------------

fn act_number(value: &str) -> NumberFieldInput {
    NumberFieldInput {
        value: value.into(),
        template_id: None,
        confirm_mismatch: false,
        confirm_script_mix: false,
    }
}

fn create_payload(number: &str, devices: &[i64], place: i64) -> ActCreateDto {
    ActCreateDto {
        number_input: act_number(number),
        giver_name: "Иванов И.И.".into(),
        receiver_name: "Петров П.П.".into(),
        place_id: Some(place),
        notes: None,
        deadline_utc: None,
        handover_date_utc: None,
        items: devices
            .iter()
            .map(|&id| ActItemNewDto {
                device_id: id,
                device_ids: Vec::new(),
                quantity: 1,
            })
            .collect(),
    }
}

async fn handover_act(ctx: &AppCtx, number: &str, devices: &[i64], place: i64) -> ActDto {
    ctx.acts
        .create(&admin(), create_payload(number, devices, place))
        .await
        .expect("create handover")
        .expect_created("create handover")
}

fn update_payload(act: &ActDto, number: &str, devices: &[i64]) -> ActUpdateDto {
    ActUpdateDto {
        id: act.id,
        expected_version: act.version,
        number_input: ActNumberEditInput {
            value: number.to_string(),
            confirm_script_mix: false,
        },
        giver_name: act.giver_name.clone(),
        receiver_name: act.receiver_name.clone(),
        place_id: act.place_id,
        notes: act.notes.clone(),
        deadline_utc: act.deadline_utc,
        handover_date_utc: None,
        items: devices
            .iter()
            .map(|&id| ActUpdateItemDto {
                device_id: id,
                complectation_at_time: None,
            })
            .collect(),
    }
}

fn return_items(handover: &ActDto, devices: &[i64]) -> Vec<ActReturnItemDto> {
    devices
        .iter()
        .map(|&did| {
            let it = handover
                .items
                .iter()
                .find(|i| i.device_id == did)
                .expect("устройство есть в акте выдачи");
            ActReturnItemDto {
                act_item_id: it.id,
                device_id: did,
                device_ids: vec![did],
                quantity: 1,
                condition_override: None,
                place_id_override: None,
            }
        })
        .collect()
}

fn return_payload(handover: &ActDto, devices: &[i64], condition: &str, place: i64) -> ActReturnDto {
    ActReturnDto {
        bulk_condition: Some(condition.into()),
        bulk_place_id: Some(place),
        apply_to_all: true,
        giver_name: None,
        receiver_name: None,
        handover_date_utc: None,
        items: return_items(handover, devices),
    }
}

async fn return_act(
    ctx: &AppCtx,
    handover: &ActDto,
    devices: &[i64],
    condition: &str,
    place: i64,
) -> ActDto {
    ctx.acts
        .do_return(
            &admin(),
            handover.id,
            return_payload(handover, devices, condition, place),
        )
        .await
        .expect("do_return")
}

fn update_return_payload(
    handover: &ActDto,
    ret: &ActDto,
    devices: &[i64],
    condition: &str,
    place: i64,
) -> ActUpdateReturnDto {
    ActUpdateReturnDto {
        id: ret.id,
        expected_version: ret.version,
        giver_name: ret.giver_name.clone(),
        receiver_name: ret.receiver_name.clone(),
        place_id: ret.place_id,
        notes: None,
        deadline_utc: None,
        handover_date_utc: ret.handover_date_utc,
        bulk_condition: Some(condition.into()),
        bulk_place_id: Some(place),
        apply_to_all: true,
        items: return_items(handover, devices),
    }
}

// ---------------------------------------------------------------------------
// Сценарии: по одному на строку Broadcasts
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_create() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let act_place = seed_place(&ctx, "Кабинет A").await;
        let dev_place = seed_place(&ctx, "Склад B").await;
        let dev = seed_device(&ctx, 1, "Ноутбук", "INV-1", Some(dev_place), None).await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        let act = handover_act(&ctx, "8101", &[dev], act_place).await;

        let (places, devices, groups) = assert_one_entities(&mut rx, "create");
        assert!(!act.changed_place_ids.is_empty(), "ответ несёт места");
        assert_eq!(
            sorted(places.clone()),
            sorted(act.changed_place_ids.clone()),
            "place_ids события == changed_place_ids ответа (D-03)"
        );
        assert_eq!(
            sorted(places),
            sorted(vec![act_place, dev_place]),
            "и место акта, и прежнее место устройства"
        );
        assert!(devices.is_empty(), "create: device_ids пуст (Q4)");
        assert!(groups.is_empty(), "create: group_ids пуст");
    })
    .await
    .expect("scenario_create вышел за бюджет");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_update() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let act_place = seed_place(&ctx, "Кабинет A").await;
        let b = seed_place(&ctx, "Склад B").await;
        let c = seed_place(&ctx, "Склад C").await;
        let d1 = seed_device(&ctx, 1, "Ноутбук", "INV-1", Some(b), None).await;
        let d2 = seed_device(&ctx, 1, "Монитор", "INV-2", Some(c), None).await;
        let act = handover_act(&ctx, "8201", &[d1], act_place).await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        // Меняется и номер (рядом уйдёт NumberSpaceChanged), и состав: добавлено d2 с места C.
        let out = ctx
            .acts
            .update(&admin(), update_payload(&act, "8202", &[d1, d2]))
            .await
            .expect("update")
            .expect_created("update");

        let (places, devices, groups) = assert_one_entities(&mut rx, "update");
        assert_eq!(
            sorted(places.clone()),
            sorted(out.changed_place_ids.clone()),
            "place_ids события == changed_place_ids ответа (D-03)"
        );
        assert!(places.contains(&c), "прежнее место добавленного устройства");
        assert!(places.contains(&act_place), "место акта");
        assert!(devices.is_empty() && groups.is_empty());
    })
    .await
    .expect("scenario_update вышел за бюджет");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_do_return() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let act_place = seed_place(&ctx, "Кабинет A").await;
        let b = seed_place(&ctx, "Склад B").await;
        let c = seed_place(&ctx, "Склад C").await;
        let back = seed_place(&ctx, "Склад D").await;
        let d1 = seed_device(&ctx, 1, "Ноутбук", "INV-1", Some(b), None).await;
        let d2 = seed_device(&ctx, 1, "Монитор", "INV-2", Some(c), None).await;
        let handover = handover_act(&ctx, "8301", &[d1, d2], act_place).await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        // Частичный возврат одного устройства: уходит с места акта на склад D.
        let ret = return_act(&ctx, &handover, &[d1], "Исправно", back).await;

        let (places, devices, groups) = assert_one_entities(&mut rx, "do_return");
        assert_eq!(
            sorted(places.clone()),
            sorted(ret.changed_place_ids.clone()),
            "place_ids события == changed_place_ids ответа (D-03)"
        );
        assert!(places.contains(&act_place), "источник: место акта");
        assert!(places.contains(&back), "приёмник: место возврата");
        assert!(
            !places.contains(&b) && !places.contains(&c),
            "нетронутые места не попадают в событие: {places:?}"
        );
        assert!(devices.is_empty() && groups.is_empty());
    })
    .await
    .expect("scenario_do_return вышел за бюджет");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_update_return() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let act_place = seed_place(&ctx, "Кабинет A").await;
        let b = seed_place(&ctx, "Склад B").await;
        let first = seed_place(&ctx, "Склад D").await;
        let second = seed_place(&ctx, "Склад E").await;
        let d1 = seed_device(&ctx, 1, "Ноутбук", "INV-1", Some(b), None).await;
        let handover = handover_act(&ctx, "8401", &[d1], act_place).await;
        let ret = return_act(&ctx, &handover, &[d1], "Исправно", first).await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        // Правка возврата: устройство переезжает со склада D на склад E.
        let edited = ctx
            .acts
            .update_return(
                &admin(),
                update_return_payload(&handover, &ret, &[d1], "Исправно", second),
            )
            .await
            .expect("update_return");

        let (places, devices, groups) = assert_one_entities(&mut rx, "update_return");
        assert!(!edited.changed_place_ids.is_empty(), "ответ несёт места");
        assert_eq!(
            sorted(places.clone()),
            sorted(edited.changed_place_ids.clone()),
            "place_ids события == changed_place_ids ответа (D-03)"
        );
        assert!(places.contains(&second), "новое место возврата");
        assert!(devices.is_empty() && groups.is_empty());
    })
    .await
    .expect("scenario_update_return вышел за бюджет");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_delete_soft() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let act_place = seed_place(&ctx, "Кабинет A").await;
        let b = seed_place(&ctx, "Склад B").await;
        let d1 = seed_device(&ctx, 1, "Ноутбук", "INV-1", Some(b), None).await;
        let act = handover_act(&ctx, "8501", &[d1], act_place).await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        // Откат акта возвращает устройство с места акта на прежнее.
        let touched = ctx
            .acts
            .delete_soft(act.id, act.version)
            .await
            .expect("delete_soft");

        let (places, devices, groups) = assert_one_entities(&mut rx, "delete_soft");
        assert!(!touched.is_empty(), "откат вернул затронутые места");
        assert_eq!(
            sorted(places.clone()),
            sorted(touched),
            "place_ids события == Vec<i64> результата (D-03)"
        );
        assert!(places.contains(&act_place), "источник: место акта");
        assert!(places.contains(&b), "приёмник: прежнее место устройства");
        assert!(devices.is_empty() && groups.is_empty());
    })
    .await
    .expect("scenario_delete_soft вышел за бюджет");
}

// ---------------------------------------------------------------------------
// Ловушка P4: событие НЕ зависит от смены номера
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn p4_update_without_number_change_still_broadcasts() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let act_place = seed_place(&ctx, "Кабинет A").await;
        let b = seed_place(&ctx, "Склад B").await;
        let c = seed_place(&ctx, "Склад C").await;
        let d1 = seed_device(&ctx, 1, "Ноутбук", "INV-1", Some(b), None).await;
        let d2 = seed_device(&ctx, 1, "Монитор", "INV-2", Some(c), None).await;
        let act = handover_act(&ctx, "8601", &[d1], act_place).await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        // Номер тот же ("8601"), меняется только состав: d2 с места C.
        let out = ctx
            .acts
            .update(&admin(), update_payload(&act, "8601", &[d1, d2]))
            .await
            .expect("update без смены номера")
            .expect_created("update без смены номера");

        let (places, _, _) = assert_one_entities(&mut rx, "update без смены номера");
        assert_eq!(
            sorted(places.clone()),
            sorted(out.changed_place_ids),
            "place_ids события == changed_place_ids ответа (D-03)"
        );
        assert!(places.contains(&c), "место добавленного устройства");
    })
    .await
    .expect("p4_update_without_number_change_still_broadcasts вышел за бюджет");
}

// ---------------------------------------------------------------------------
// Откаты до коммита: события нет
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn rollback_create_validation_and_occupied_number_send_nothing() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let act_place = seed_place(&ctx, "Кабинет A").await;
        let b = seed_place(&ctx, "Склад B").await;
        let d1 = seed_device(&ctx, 1, "Ноутбук", "INV-1", Some(b), None).await;
        let d2 = seed_device(&ctx, 1, "Монитор", "INV-2", Some(b), None).await;
        handover_act(&ctx, "8701", &[d1], act_place).await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        // Ошибка валидации: пустой номер.
        let err = ctx
            .acts
            .create(&admin(), create_payload("   ", &[d2], act_place))
            .await
            .expect_err("пустой номер");
        assert!(matches!(err, AppError::Validation { .. }), "{err:?}");
        assert_no_entities(&mut rx, "create: валидация");

        // Занятый номер.
        let occupied = ctx
            .acts
            .create(&admin(), create_payload("8701", &[d2], act_place))
            .await;
        assert!(occupied.is_err(), "занятый номер: {occupied:?}");
        assert_no_entities(&mut rx, "create: занятый номер");
    })
    .await
    .expect("rollback_create вышел за бюджет");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn rollback_update_and_delete_stale_version_send_nothing() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let act_place = seed_place(&ctx, "Кабинет A").await;
        let b = seed_place(&ctx, "Склад B").await;
        let d1 = seed_device(&ctx, 1, "Ноутбук", "INV-1", Some(b), None).await;
        let act = handover_act(&ctx, "8801", &[d1], act_place).await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        let mut stale = update_payload(&act, "8801", &[d1]);
        stale.expected_version = act.version + 99;
        let err = ctx
            .acts
            .update(&admin(), stale)
            .await
            .expect_err("устаревшая версия");
        assert!(
            matches!(err, AppError::OptimisticLockMismatch { .. }),
            "{err:?}"
        );
        assert_no_entities(&mut rx, "update: устаревшая версия");

        let err = ctx
            .acts
            .delete_soft(act.id, act.version + 99)
            .await
            .expect_err("устаревшая версия");
        assert!(
            matches!(err, AppError::OptimisticLockMismatch { .. }),
            "{err:?}"
        );
        assert_no_entities(&mut rx, "delete_soft: устаревшая версия");
    })
    .await
    .expect("rollback_update_and_delete вышел за бюджет");
}

// ---------------------------------------------------------------------------
// D-01: массовая операция даёт ровно одно событие
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mass_act_over_many_devices_sends_exactly_one_event_per_call() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let act_place = seed_place(&ctx, "Кабинет A").await;
        let b = seed_place(&ctx, "Склад B").await;
        let back = seed_place(&ctx, "Склад D").await;
        let mut ids = Vec::with_capacity(MASS_DEVICES);
        for i in 0..MASS_DEVICES {
            ids.push(
                seed_device(
                    &ctx,
                    1,
                    &format!("Устройство {i}"),
                    &format!("INV-M{i}"),
                    Some(b),
                    None,
                )
                .await,
            );
        }
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        // Один пункт акта несёт все устройства списком device_ids (потолок позиций — 100).
        let created = ctx
            .acts
            .create(
                &admin(),
                ActCreateDto {
                    items: vec![ActItemNewDto {
                        device_id: ids[0],
                        device_ids: ids.clone(),
                        quantity: ids.len() as i64,
                    }],
                    ..create_payload("8901", &ids[..1], act_place)
                },
            )
            .await
            .expect("mass create")
            .expect_created("mass create");
        let (places, devices, _) = assert_one_entities(&mut rx, "mass create");
        assert_eq!(
            sorted(places),
            sorted(created.changed_place_ids.clone()),
            "place_ids события == changed_place_ids ответа"
        );
        assert!(devices.is_empty());
        assert!(
            matches!(
                rx.try_recv(),
                Err(tokio::sync::broadcast::error::TryRecvError::Empty)
            ),
            "после одного события канал пуст, Lagged не было"
        );

        // Возврат всех 220 устройств одним вызовом.
        let ret = return_act(&ctx, &created, &ids, "Исправно", back).await;
        let (places, _, _) = assert_one_entities(&mut rx, "mass do_return");
        assert_eq!(sorted(places), sorted(ret.changed_place_ids));
        assert!(matches!(
            rx.try_recv(),
            Err(tokio::sync::broadcast::error::TryRecvError::Empty)
        ));
    })
    .await
    .expect("mass_act вышел за бюджет");
}

// ---------------------------------------------------------------------------
// Привязка к реестру (одна строка)
// ---------------------------------------------------------------------------

#[test]
fn scenarios_cover_every_broadcasting_row() {
    entities_support::scenarios::assert_behaviour_file_covers(
        "entities_changed_acts.rs",
        "act_service.rs",
    );
}

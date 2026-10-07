//! Phase 41.7, план 07: слой (2) реестрового гейта для `group_service.rs`.
//!
//! На КАЖДУЮ строку `Broadcasts` файла `group_service.rs` приходится ровно один
//! `#[tokio::test] async fn scenario_<функция>`; привязка к реестру идёт по
//! исходнику этого файла (см. `entities_support::scenarios`). Откаты, пустой
//! итог и массовая операция носят другие префиксы (`rollback_`, `empty_outcome_`,
//! `mass_`) и сценариями не считаются.
//!
//! Настоящий `AppCtx::build`, подписка на `ctx.ws_broadcast` ДО мутации,
//! `try_recv()` сразу после `.await`. Списки сравниваются как множества.
//! Фикстуры расходятся намеренно (разные места A/B/C, участники вне группы),
//! чтобы неверный набор id в событии ронял тест. Данные вымышленные.

mod entities_support;

use std::time::Duration;

use entities_support::fixture::{
    admin, assert_no_entities, assert_one_entities, drain, join_group, make_test_ctx, seed_device,
    seed_group_type_and_group, seed_place, type_id_by_code,
};
use trackly_app::context::AppCtx;
use trackly_app::dto::groups::{GroupCreateDto, GroupValueInputDto};
use trackly_core::error::AppError;

/// Бюджет каждого теста (форма `number_space_broadcast_gate.rs`).
const BUDGET: Duration = Duration::from_secs(60);

/// Устройств в массовой операции (больше ёмкости канала 128, меньше потолка 500).
const MASS_DEVICES: usize = 220;

fn sorted(mut v: Vec<i64>) -> Vec<i64> {
    v.sort_unstable();
    v.dedup();
    v
}

async fn group_version(ctx: &AppCtx, id: i64) -> i64 {
    ctx.groups
        .get_group(&admin(), id)
        .await
        .expect("get_group")
        .version
}

/// Свойство «IP» встроенного типа «АРМ».
async fn ip_property_id(ctx: &AppCtx) -> i64 {
    ctx.group_types
        .list_types(&admin(), true)
        .await
        .expect("list_types")
        .into_iter()
        .find(|t| t.code == "workstation")
        .expect("тип АРМ")
        .properties
        .into_iter()
        .find(|p| p.name == "IP")
        .expect("свойство IP")
        .id
}

// ---------------------------------------------------------------------------
// Сценарии: по одному на строку Broadcasts
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_create_group() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let place = seed_place(&ctx, "Кабинет 101").await;
        let type_id = type_id_by_code(&ctx, "workstation").await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        // С местом: место и сама группа.
        let with_place = ctx
            .groups
            .create_group(
                &admin(),
                GroupCreateDto {
                    type_id,
                    name: Some("АРМ Иванова И.И.".to_string()),
                    place_id: Some(place as i32),
                },
            )
            .await
            .expect("create with place");
        let (places, devices, groups) = assert_one_entities(&mut rx, "create_group с местом");
        assert_eq!(sorted(places), vec![place], "place_ids == [place_id]");
        assert!(devices.is_empty(), "create_group: device_ids пуст");
        assert_eq!(sorted(groups), vec![with_place.id], "group_ids == [id]");

        // Без места: событие ВСЁ РАВНО идёт (D-17), place_ids пуст.
        let without = ctx
            .groups
            .create_group(
                &admin(),
                GroupCreateDto {
                    type_id,
                    name: Some("АРМ Петрова П.П.".to_string()),
                    place_id: None,
                },
            )
            .await
            .expect("create without place");
        let (places, devices, groups) = assert_one_entities(&mut rx, "create_group без места");
        assert!(places.is_empty(), "без места place_ids пуст");
        assert!(devices.is_empty());
        assert_eq!(sorted(groups), vec![without.id]);
        assert_ne!(with_place.id, without.id);
    })
    .await
    .expect("scenario_create_group вышел за бюджет");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_update_group() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let place = seed_place(&ctx, "Кабинет 101").await;
        let (_t, g) = seed_group_type_and_group(&ctx, "АРМ Иванова И.И.", 1, Some(place)).await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        let updated = ctx
            .groups
            .update_group(&admin(), g, 1, "АРМ Сидорова С.С.".to_string())
            .await
            .expect("update_group");
        assert_eq!(updated.name, "АРМ Сидорова С.С.");
        let (places, devices, groups) = assert_one_entities(&mut rx, "update_group");
        assert!(places.is_empty(), "переименование: place_ids пуст (D-17)");
        assert!(devices.is_empty());
        assert_eq!(sorted(groups), vec![g], "group_ids == [id]");
    })
    .await
    .expect("scenario_update_group вышел за бюджет");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_delete_group() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let place = seed_place(&ctx, "Кабинет 101").await;
        let (_t, g) = seed_group_type_and_group(&ctx, "АРМ Иванова И.И.", 1, Some(place)).await;
        seed_device(&ctx, 1, "Системный блок", "INV-1", Some(place), Some(g)).await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        let res = ctx
            .groups
            .delete_group(&admin(), g)
            .await
            .expect("delete_group");
        assert_eq!(res.released_devices, 1);
        let (places, _devices, groups) = assert_one_entities(&mut rx, "delete_group");
        assert!(places.is_empty(), "delete_group: place_ids пуст");
        assert_eq!(sorted(groups), vec![g], "group_ids == [id]");
    })
    .await
    .expect("scenario_delete_group вышел за бюджет");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_move_group() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Кабинет A").await;
        let b = seed_place(&ctx, "Кабинет B").await;
        let c = seed_place(&ctx, "Кабинет C").await;
        let (_t, g) = seed_group_type_and_group(&ctx, "АРМ Иванова И.И.", 1, Some(a)).await;
        let d1 = seed_device(&ctx, 1, "Системный блок", "INV-1", Some(a), Some(g)).await;
        // Участник, чьё прежнее место (C) отличается от места группы (A).
        let d2 = seed_device(&ctx, 1, "Монитор", "INV-2", Some(c), None).await;
        join_group(&ctx, d2, g).await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        let result = ctx
            .groups
            .move_group(&admin(), g, 1, b)
            .await
            .expect("move_group");
        assert_eq!(
            result.moved_devices, 2,
            "перенесены оба участника ({d1}, {d2})"
        );
        let (places, devices, groups) = assert_one_entities(&mut rx, "move_group");
        // D-03: место события берётся из ответа, а не пересчитывается.
        assert_eq!(
            sorted(places.clone()),
            sorted(result.changed_place_ids.clone()),
            "place_ids события == changed_place_ids ответа"
        );
        assert!(places.contains(&b), "целевое место B в событии");
        assert!(places.contains(&c), "прежнее место участника C в событии");
        assert!(devices.is_empty(), "Q4: device_ids у move_group пуст");
        assert_eq!(sorted(groups), vec![g]);
    })
    .await
    .expect("scenario_move_group вышел за бюджет");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_add_devices() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Кабинет A").await;
        let b = seed_place(&ctx, "Кабинет B").await;
        let (_t, g) = seed_group_type_and_group(&ctx, "АРМ Иванова И.И.", 1, Some(b)).await;
        let d1 = seed_device(&ctx, 1, "Системный блок", "INV-1", Some(a), None).await;
        let d2 = seed_device(&ctx, 1, "Монитор", "INV-2", Some(a), None).await;
        // Не добавляется: остаётся вне события.
        let outsider = seed_device(&ctx, 1, "Клавиатура", "INV-3", Some(a), None).await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        let result = ctx
            .groups
            .add_devices(&admin(), g, vec![d1, d2])
            .await
            .expect("add_devices");
        assert_eq!(result.added, 2);
        let (places, devices, groups) = assert_one_entities(&mut rx, "add_devices");
        assert_eq!(
            sorted(places.clone()),
            sorted(result.changed_place_ids.clone()),
            "place_ids события == changed_place_ids ответа"
        );
        assert!(
            places.contains(&a) && places.contains(&b),
            "A (прежнее) и B (место группы)"
        );
        assert_eq!(
            sorted(devices.clone()),
            sorted(vec![d1, d2]),
            "device_ids == добавленные"
        );
        assert!(!devices.contains(&outsider));
        assert_eq!(sorted(groups), vec![g], "group_ids == [group_id]");
    })
    .await
    .expect("scenario_add_devices вышел за бюджет");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_remove_devices() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Кабинет A").await;
        let (_t, g) = seed_group_type_and_group(&ctx, "АРМ Иванова И.И.", 1, Some(a)).await;
        let d1 = seed_device(&ctx, 1, "Системный блок", "INV-1", Some(a), Some(g)).await;
        let d2 = seed_device(&ctx, 1, "Монитор", "INV-2", Some(a), Some(g)).await;
        let kept = seed_device(&ctx, 1, "Клавиатура", "INV-3", Some(a), Some(g)).await;
        // Не состоит в группе: идемпотентно игнорируется и в событие не попадает.
        let stranger = seed_device(&ctx, 1, "Мышь", "INV-4", Some(a), None).await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        let released = ctx
            .groups
            .remove_devices(&admin(), g, vec![d1, d2, stranger])
            .await
            .expect("remove_devices");
        assert_eq!(released, 2);
        let (places, devices, groups) = assert_one_entities(&mut rx, "remove_devices");
        // D-17: место не меняется, событие всё равно идёт с пустым place_ids.
        assert!(places.is_empty(), "remove_devices: place_ids пуст");
        assert_eq!(
            sorted(devices.clone()),
            sorted(vec![d1, d2]),
            "device_ids == выведенные"
        );
        assert!(!devices.contains(&kept) && !devices.contains(&stranger));
        assert_eq!(sorted(groups), vec![g], "group_ids == [group_id]");
    })
    .await
    .expect("scenario_remove_devices вышел за бюджет");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_set_parent() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Кабинет A").await;
        let b = seed_place(&ctx, "Кабинет B").await;
        let (_t, parent) = seed_group_type_and_group(&ctx, "АРМ Иванова И.И.", 1, Some(b)).await;
        let (_t, child) = seed_group_type_and_group(&ctx, "АРМ Петрова П.П.", 2, Some(a)).await;
        seed_device(&ctx, 1, "Системный блок", "INV-1", Some(a), Some(child)).await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        let result = ctx
            .groups
            .set_parent(&admin(), child, 1, Some(parent))
            .await
            .expect("set_parent");
        assert_eq!(result.moved_devices, 1);
        let (places, devices, groups) = assert_one_entities(&mut rx, "set_parent");
        // D-03: место события берётся из ответа, а не пересчитывается.
        assert_eq!(
            sorted(places.clone()),
            sorted(result.changed_place_ids.clone()),
            "place_ids события == changed_place_ids ответа"
        );
        assert!(
            places.contains(&a) && places.contains(&b),
            "A (прежнее) и B (место родителя)"
        );
        assert!(devices.is_empty(), "Q4: device_ids у set_parent пуст");
        assert_eq!(
            sorted(groups),
            vec![child],
            "group_ids == [id вложенной группы]"
        );
    })
    .await
    .expect("scenario_set_parent вышел за бюджет");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_set_values() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let place = seed_place(&ctx, "Кабинет 101").await;
        let (_t, g) = seed_group_type_and_group(&ctx, "АРМ Иванова И.И.", 1, Some(place)).await;
        let ip = ip_property_id(&ctx).await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        let card = ctx
            .groups
            .set_values(
                &admin(),
                g,
                1,
                vec![GroupValueInputDto {
                    property_id: ip,
                    text: Some("192.168.1.10".to_string()),
                    refs: vec![],
                }],
            )
            .await
            .expect("set_values");
        assert_eq!(card.group.id, g);
        let (places, devices, groups) = assert_one_entities(&mut rx, "set_values");
        assert!(places.is_empty(), "set_values: place_ids пуст (D-17)");
        assert!(devices.is_empty());
        assert_eq!(sorted(groups), vec![g], "group_ids == [id]");
    })
    .await
    .expect("scenario_set_values вышел за бюджет");
}

// ---------------------------------------------------------------------------
// Откаты: ложного события быть не должно (T-41.7-27)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn rollback_update_group_stale_version_sends_nothing() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let (_t, g) = seed_group_type_and_group(&ctx, "АРМ Иванова И.И.", 1, None).await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        let err = ctx
            .groups
            .update_group(&admin(), g, 99, "АРМ Сидорова С.С.".to_string())
            .await
            .expect_err("устаревшая версия");
        assert!(
            matches!(err, AppError::OptimisticLockMismatch { .. }),
            "{err:?}"
        );
        assert_no_entities(&mut rx, "update_group: откат");
    })
    .await
    .expect("rollback_update_group вышел за бюджет");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn rollback_move_group_stale_version_sends_nothing() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Кабинет A").await;
        let b = seed_place(&ctx, "Кабинет B").await;
        let (_t, g) = seed_group_type_and_group(&ctx, "АРМ Иванова И.И.", 1, Some(a)).await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        let err = ctx
            .groups
            .move_group(&admin(), g, 99, b)
            .await
            .expect_err("устаревшая версия");
        assert!(
            matches!(err, AppError::OptimisticLockMismatch { .. }),
            "{err:?}"
        );
        assert_no_entities(&mut rx, "move_group: откат");
    })
    .await
    .expect("rollback_move_group вышел за бюджет");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn rollback_add_devices_taken_device_sends_nothing() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Кабинет A").await;
        let (_t, g1) = seed_group_type_and_group(&ctx, "АРМ Иванова И.И.", 1, Some(a)).await;
        let (_t, g2) = seed_group_type_and_group(&ctx, "АРМ Петрова П.П.", 2, Some(a)).await;
        let free = seed_device(&ctx, 1, "Системный блок", "INV-1", Some(a), None).await;
        let taken = seed_device(&ctx, 1, "Монитор", "INV-2", Some(a), Some(g1)).await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        // Занятое устройство отменяет весь пакет.
        ctx.groups
            .add_devices(&admin(), g2, vec![free, taken])
            .await
            .expect_err("занятое устройство");
        assert_no_entities(&mut rx, "add_devices: откат пакета");
    })
    .await
    .expect("rollback_add_devices вышел за бюджет");
}

// ---------------------------------------------------------------------------
// Пустой итог: событие идёт и без мест (решение W2, D-17)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn empty_outcome_move_to_same_place_still_sends_event() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Кабинет A").await;
        let (_t, g) = seed_group_type_and_group(&ctx, "АРМ Иванова И.И.", 1, Some(a)).await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        let result = ctx
            .groups
            .move_group(&admin(), g, 1, a)
            .await
            .expect("move на то же место");
        assert!(result.changed_place_ids.is_empty(), "места не менялись");
        assert_eq!(result.moved_devices, 0);
        // group_ids = [id] не пуст, поэтому событие идёт; пустой place_ids клиент
        // читает как «перезагрузить всё».
        let (places, devices, groups) = assert_one_entities(&mut rx, "пустой итог move_group");
        assert!(places.is_empty());
        assert!(devices.is_empty());
        assert_eq!(sorted(groups), vec![g]);
    })
    .await
    .expect("empty_outcome вышел за бюджет");
}

// ---------------------------------------------------------------------------
// D-01: пакетная операция даёт ровно одно событие
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mass_add_devices_sends_exactly_one_event() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Кабинет A").await;
        let b = seed_place(&ctx, "Кабинет B").await;
        let (_t, g) = seed_group_type_and_group(&ctx, "АРМ Иванова И.И.", 1, Some(b)).await;
        let mut ids = Vec::with_capacity(MASS_DEVICES);
        for i in 0..MASS_DEVICES {
            ids.push(
                seed_device(
                    &ctx,
                    1,
                    &format!("Устройство {i}"),
                    &format!("INV-M{i}"),
                    Some(a),
                    None,
                )
                .await,
            );
        }
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        let result = ctx
            .groups
            .add_devices(&admin(), g, ids.clone())
            .await
            .expect("mass add_devices");
        assert_eq!(result.added as usize, MASS_DEVICES);

        // Ровно одно событие, второго нет; Lagged (ёмкость 128) не наступил.
        let (places, devices, groups) = assert_one_entities(&mut rx, "массовое add_devices");
        assert_eq!(sorted(places), sorted(result.changed_place_ids.clone()));
        assert_eq!(sorted(devices), sorted(ids));
        assert_eq!(sorted(groups), vec![g]);
        assert!(
            matches!(
                rx.try_recv(),
                Err(tokio::sync::broadcast::error::TryRecvError::Empty)
            ),
            "после единственного события канал пуст (без Lagged)"
        );
    })
    .await
    .expect("mass_add_devices вышел за бюджет");
}

// ---------------------------------------------------------------------------
// Покрытие: привязка сценариев к реестру по исходнику этого файла
// ---------------------------------------------------------------------------

#[test]
fn scenarios_cover_every_broadcasting_row() {
    entities_support::scenarios::assert_behaviour_file_covers(
        "entities_changed_groups.rs",
        "group_service.rs",
    );
}

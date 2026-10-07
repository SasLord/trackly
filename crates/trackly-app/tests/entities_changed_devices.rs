//! Phase 41.7, план 09: слой (2) реестрового гейта для `device_service.rs` (D-16).
//!
//! На КАЖДУЮ строку `Broadcasts` файла `device_service.rs` приходится ровно один
//! `#[tokio::test] async fn scenario_<функция>`; привязка к реестру идёт по
//! исходнику этого файла (см. `entities_support::scenarios`). Ловушка P4,
//! откаты, массовые операции и обёртки носят другие префиксы (`p4_`,
//! `rollback_`, `mass_`, `wrappers_`) и сценариями не считаются.
//!
//! Настоящий `AppCtx::build`, подписка на `ctx.ws_broadcast` ДО мутации,
//! `try_recv()` сразу после `.await`. Списки сравниваются как множества.
//! Фикстуры расходятся намеренно (несколько мест, нетронутое место, которое
//! в событии быть не должно), чтобы неверный набор id ронял тест.
//! Данные вымышленные.

mod entities_support;

use std::collections::HashMap;
use std::time::Duration;

use entities_support::fixture::{
    admin, assert_no_entities, assert_one_entities, drain, make_test_ctx, seed_device, seed_place,
};
use tokio::sync::broadcast::error::TryRecvError;
use trackly_app::dto::device::{
    DeviceDto, DeviceNew, DeviceNumberEditInput, DevicePatch, DeviceSaveOutcome,
};
use trackly_app::dto::number_template::{NumberFieldInput, TemplateContextDto};
use trackly_core::error::AppError;

/// Бюджет каждого теста (форма `number_space_broadcast_gate.rs`).
const BUDGET: Duration = Duration::from_secs(60);

/// Строк CSV в массовом импорте (больше ёмкости канала 128).
const MASS_ROWS: usize = 220;

fn sorted(mut v: Vec<i64>) -> Vec<i64> {
    v.sort_unstable();
    v.dedup();
    v
}

/// Устройство БЕЗ инвентарного номера (ловушка P4: соседняя рассылка номера
/// под `if inventory_no.is_some()` для него не сработает).
fn new_device(name: &str, place: Option<i64>) -> DeviceNew {
    DeviceNew {
        type_id: 1,
        name: name.to_string(),
        inventory_no: None,
        serial_no: None,
        model: Some("Модель X".to_string()),
        specs: None,
        kit: None,
        state: None,
        place_id: place,
        status_id: 1,
    }
}

fn empty_patch() -> DevicePatch {
    DevicePatch::default()
}

fn number_field(value: &str) -> NumberFieldInput {
    NumberFieldInput {
        value: value.into(),
        template_id: None,
        confirm_mismatch: false,
        confirm_script_mix: false,
    }
}

fn created(outcome: DeviceSaveOutcome, what: &str) -> DeviceDto {
    outcome.expect_created(what)
}

fn is_empty_channel<T: Clone>(rx: &mut tokio::sync::broadcast::Receiver<T>) -> bool {
    matches!(rx.try_recv(), Err(TryRecvError::Empty))
}

/// CSV из `rows` строк: «Наименование,Расположение», места по кругу.
fn csv_bytes(rows: usize, places: &[&str]) -> Vec<u8> {
    let mut s = String::from("Наименование,Расположение\n");
    for i in 0..rows {
        s.push_str(&format!("Монитор {i},{}\n", places[i % places.len()]));
    }
    s.into_bytes()
}

fn csv_mapping() -> HashMap<String, String> {
    HashMap::from([
        ("Наименование".to_string(), "name".to_string()),
        ("Расположение".to_string(), "place".to_string()),
    ])
}

// ---------------------------------------------------------------------------
// Сценарии (по одному на строку реестра)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_create() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Кабинет A").await;
        let untouched = seed_place(&ctx, "Кабинет Z").await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        // Без inventory_no: событие приходит БЕЗ условия соседней рассылки номера (P4).
        let dto = created(
            ctx.devices
                .create(new_device("Принтер Тест-1", Some(a)))
                .await
                .expect("create"),
            "create",
        );

        let (places, devices, groups) = assert_one_entities(&mut rx, "create");
        assert_eq!(
            sorted(places.clone()),
            vec![a],
            "place_ids == место устройства"
        );
        assert!(
            !places.contains(&untouched),
            "нетронутое место в событии не нужно"
        );
        assert_eq!(devices, vec![dto.id], "device_ids == id созданного");
        assert!(groups.is_empty());
    })
    .await
    .expect("scenario_create вышел за бюджет");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_create_single_with_number_check_with_printer() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Кабинет A").await;
        let untouched = seed_place(&ctx, "Кабинет Z").await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        // Без номера: событие приходит, а NumberSpaceChanged нет (его рассылка безусловна
        // в этой функции, но событие обязано жить и без неё).
        let dto = created(
            ctx.devices
                .create_single_with_number_check_with_printer(
                    new_device("Принтер Тест-2", Some(a)),
                    number_field(""),
                    TemplateContextDto::DeviceCreate,
                    None,
                )
                .await
                .expect("create_single_with_number_check_with_printer"),
            "create_single_with_number_check_with_printer",
        );

        let (places, devices, groups) =
            assert_one_entities(&mut rx, "create_single_with_number_check_with_printer");
        assert_eq!(sorted(places.clone()), vec![a]);
        assert!(!places.contains(&untouched));
        assert_eq!(devices, vec![dto.id]);
        assert!(groups.is_empty());
    })
    .await
    .expect("scenario_create_single_with_number_check_with_printer вышел за бюджет");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_update() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let old = seed_place(&ctx, "Кабинет A").await;
        let new_place = seed_place(&ctx, "Кабинет B").await;
        let untouched = seed_place(&ctx, "Кабинет Z").await;
        let id = seed_device(&ctx, 1, "Ноутбук Тест-3", "INV-9001", Some(old), None).await;
        let other = seed_device(&ctx, 1, "Монитор Тест-3", "INV-9002", Some(untouched), None).await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        // (1) Перенос со старого места на новое: оба места в событии, нетронутого нет.
        let moved = created(
            ctx.devices
                .update(
                    &admin(),
                    id,
                    1,
                    DevicePatch {
                        place_id: Some(Some(new_place)),
                        ..empty_patch()
                    },
                )
                .await
                .expect("update: перенос"),
            "update: перенос",
        );
        let (places, devices, groups) = assert_one_entities(&mut rx, "update: перенос");
        assert_eq!(
            sorted(places.clone()),
            sorted(vec![old, new_place]),
            "place_ids == [старое, новое]"
        );
        assert!(
            !places.contains(&untouched),
            "нетронутое место в событии не нужно"
        );
        assert_eq!(devices, vec![id]);
        assert!(!devices.contains(&other));
        assert!(groups.is_empty());

        // (2) Правка без смены места (имя) и без смены номера: событие всё равно приходит (P4).
        let _ = created(
            ctx.devices
                .update(
                    &admin(),
                    id,
                    moved.version,
                    DevicePatch {
                        name: Some("Ноутбук Тест-3 (новый)".to_string()),
                        ..empty_patch()
                    },
                )
                .await
                .expect("update: имя"),
            "update: имя",
        );
        let (places, devices, _) = assert_one_entities(&mut rx, "update: имя");
        assert_eq!(
            sorted(places),
            vec![new_place],
            "место то же, оно и в событии"
        );
        assert_eq!(devices, vec![id]);
    })
    .await
    .expect("scenario_update вышел за бюджет");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_delete_soft() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Кабинет A").await;
        let untouched = seed_place(&ctx, "Кабинет Z").await;
        let id = seed_device(&ctx, 1, "Ноутбук Тест-4", "INV-9101", Some(a), None).await;
        let _other =
            seed_device(&ctx, 1, "Монитор Тест-4", "INV-9102", Some(untouched), None).await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        ctx.devices.delete_soft(id, 1).await.expect("delete_soft");

        let (places, devices, groups) = assert_one_entities(&mut rx, "delete_soft");
        assert_eq!(
            sorted(places.clone()),
            vec![a],
            "place_ids == место устройства"
        );
        assert!(!places.contains(&untouched));
        assert_eq!(devices, vec![id]);
        assert!(groups.is_empty());
    })
    .await
    .expect("scenario_delete_soft вышел за бюджет");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_import_csv_commit() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Кабинет A").await;
        let b = seed_place(&ctx, "Кабинет B").await;
        let c = seed_place(&ctx, "Кабинет C").await;
        let untouched = seed_place(&ctx, "Кабинет Z").await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        // 6 строк на три места; без inventory_no (P4).
        let preview = ctx
            .devices
            .import_csv_preview(csv_bytes(6, &["Кабинет A", "Кабинет B", "Кабинет C"]))
            .await
            .expect("preview");
        let report = ctx
            .devices
            .import_csv_commit(preview.token, csv_mapping())
            .await
            .expect("import_csv_commit");
        assert_eq!(report.inserted, 6, "{:?}", report.failed);

        let (places, devices, groups) = assert_one_entities(&mut rx, "import_csv_commit");
        assert_eq!(
            sorted(places.clone()),
            sorted(report.affected_place_ids.clone()),
            "place_ids события == affected_place_ids отчёта (D-03)"
        );
        assert_eq!(sorted(places.clone()), sorted(vec![a, b, c]));
        assert!(!places.contains(&untouched));
        assert_eq!(devices.len(), 6, "device_ids == созданные устройства");
        assert!(groups.is_empty());
        assert!(
            is_empty_channel(&mut rx),
            "ровно одно событие на весь импорт"
        );
    })
    .await
    .expect("scenario_import_csv_commit вышел за бюджет");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_bulk_create_with_printer() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Кабинет A").await;
        let untouched = seed_place(&ctx, "Кабинет Z").await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        let made = ctx
            .devices
            .bulk_create_with_printer(new_device("Флешка Тест-5", Some(a)), 5, None)
            .await
            .expect("bulk_create_with_printer");
        assert_eq!(made.len(), 5);

        let (places, devices, groups) = assert_one_entities(&mut rx, "bulk_create_with_printer");
        assert_eq!(sorted(places.clone()), vec![a]);
        assert!(!places.contains(&untouched));
        assert_eq!(
            sorted(devices),
            sorted(made.iter().map(|d| d.id).collect()),
            "device_ids == созданные устройства"
        );
        assert!(groups.is_empty());
        assert!(is_empty_channel(&mut rx), "ровно одно событие на пакет");
    })
    .await
    .expect("scenario_bulk_create_with_printer вышел за бюджет");
}

// ---------------------------------------------------------------------------
// Тонкие обёртки не удваивают событие
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn wrappers_do_not_double_the_event() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Кабинет A").await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        let dto = created(
            ctx.devices
                .create_single_with_number_check(
                    new_device("Принтер Тест-6", Some(a)),
                    number_field(""),
                    TemplateContextDto::DeviceCreate,
                )
                .await
                .expect("create_single_with_number_check"),
            "create_single_with_number_check",
        );
        let (places, devices, _) = assert_one_entities(&mut rx, "create_single_with_number_check");
        assert_eq!(places, vec![a]);
        assert_eq!(devices, vec![dto.id]);
        assert!(is_empty_channel(&mut rx));

        let made = ctx
            .devices
            .bulk_create(new_device("Флешка Тест-6", Some(a)), 3)
            .await
            .expect("bulk_create");
        let (places, devices, _) = assert_one_entities(&mut rx, "bulk_create");
        assert_eq!(places, vec![a]);
        assert_eq!(devices.len(), made.len());
        assert!(is_empty_channel(&mut rx));
    })
    .await
    .expect("wrappers_do_not_double_the_event вышел за бюджет");
}

// ---------------------------------------------------------------------------
// Ловушка P4: событие НЕ зависит от условий соседней рассылки номера
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn p4_event_is_independent_of_number_space_changed() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Кабинет A").await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        // Устройство без номера: NumberSpaceChanged не уходит, EntitiesChanged уходит.
        ctx.devices
            .create(new_device("Принтер Тест-7", Some(a)))
            .await
            .expect("create без номера");
        let events = entities_support::fixture::collect_events(&mut rx);
        assert!(
            !events.iter().any(|e| matches!(
                e,
                trackly_app::dto::printer::WsEvent::NumberSpaceChanged { .. }
            )),
            "для устройства без номера NumberSpaceChanged не рассылается"
        );
        assert!(
            events.iter().any(|e| matches!(
                e,
                trackly_app::dto::printer::WsEvent::EntitiesChanged { .. }
            )),
            "EntitiesChanged рассылается и без номера"
        );
    })
    .await
    .expect("p4_event_is_independent_of_number_space_changed вышел за бюджет");
}

// ---------------------------------------------------------------------------
// Откаты до коммита: события нет
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn rollback_occupied_number_and_stale_version_send_nothing() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Кабинет A").await;
        let b = seed_place(&ctx, "Кабинет B").await;
        let id = seed_device(&ctx, 1, "Ноутбук Тест-8", "INV-9201", Some(a), None).await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        // Дубль номера: create.
        let mut dup = new_device("Дубль", Some(a));
        dup.inventory_no = Some("INV-9201".into());
        let err = ctx.devices.create(dup).await.expect_err("дубль номера");
        assert!(matches!(err, AppError::Conflict { .. }), "{err:?}");
        assert_no_entities(&mut rx, "create: дубль номера");

        // Дубль номера: create_single_with_number_check_with_printer.
        let mut dup = new_device("Дубль-2", Some(a));
        dup.inventory_no = Some("INV-9201".into());
        let err = ctx
            .devices
            .create_single_with_number_check_with_printer(
                dup,
                number_field("INV-9201"),
                TemplateContextDto::DeviceCreate,
                None,
            )
            .await
            .expect_err("дубль номера (single)");
        assert!(matches!(err, AppError::Conflict { .. }), "{err:?}");
        assert_no_entities(&mut rx, "create_single: дубль номера");

        // Устаревшая версия у update: OptimisticLockMismatch.
        let err = ctx
            .devices
            .update(
                &admin(),
                id,
                999,
                DevicePatch {
                    place_id: Some(Some(b)),
                    ..empty_patch()
                },
            )
            .await
            .expect_err("устаревшая версия");
        assert!(
            matches!(err, AppError::OptimisticLockMismatch { .. }),
            "{err:?}"
        );
        assert_no_entities(&mut rx, "update: устаревшая версия");

        // Дубль номера у update.
        let other = seed_device(&ctx, 1, "Монитор Тест-8", "INV-9202", Some(a), None).await;
        let err = ctx
            .devices
            .update(
                &admin(),
                other,
                1,
                DevicePatch {
                    number_input: Some(DeviceNumberEditInput {
                        value: "INV-9201".into(),
                        confirm_script_mix: false,
                    }),
                    ..empty_patch()
                },
            )
            .await
            .expect_err("update: дубль номера");
        assert!(matches!(err, AppError::Conflict { .. }), "{err:?}");
        assert_no_entities(&mut rx, "update: дубль номера");

        // Устаревшая версия у delete_soft.
        let err = ctx
            .devices
            .delete_soft(id, 999)
            .await
            .expect_err("delete_soft: устаревшая версия");
        assert!(
            matches!(err, AppError::OptimisticLockMismatch { .. }),
            "{err:?}"
        );
        assert_no_entities(&mut rx, "delete_soft: устаревшая версия");

        // Валидация пакета (count = 0) и пустой импорт (все строки с ошибкой).
        let err = ctx
            .devices
            .bulk_create_with_printer(new_device("Флешка", Some(a)), 0, None)
            .await
            .expect_err("count = 0");
        assert!(matches!(err, AppError::Validation { .. }), "{err:?}");
        assert_no_entities(&mut rx, "bulk_create: валидация");

        let preview = ctx
            .devices
            .import_csv_preview(csv_bytes(3, &["Нет такого места"]))
            .await
            .expect("preview");
        let report = ctx
            .devices
            .import_csv_commit(preview.token, csv_mapping())
            .await
            .expect("commit: все строки с ошибкой");
        assert_eq!(report.inserted, 0);
        assert_no_entities(&mut rx, "import_csv_commit: ничего не вставлено");
    })
    .await
    .expect("rollback_ вышел за бюджет");
}

// ---------------------------------------------------------------------------
// D-01: массовые операции дают ровно одно событие (канал на 128, Lagged нет)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mass_import_over_channel_capacity_sends_exactly_one_event() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Кабинет A").await;
        let b = seed_place(&ctx, "Кабинет B").await;
        let c = seed_place(&ctx, "Кабинет C").await;
        let untouched = seed_place(&ctx, "Кабинет Z").await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        let preview = ctx
            .devices
            .import_csv_preview(csv_bytes(
                MASS_ROWS,
                &["Кабинет A", "Кабинет B", "Кабинет C"],
            ))
            .await
            .expect("preview");
        let report = ctx
            .devices
            .import_csv_commit(preview.token, csv_mapping())
            .await
            .expect("mass import");
        assert_eq!(report.inserted as usize, MASS_ROWS, "{:?}", report.failed);

        let (places, devices, _) = assert_one_entities(&mut rx, "mass import");
        assert_eq!(sorted(places.clone()), sorted(report.affected_place_ids));
        assert_eq!(sorted(places.clone()), sorted(vec![a, b, c]));
        assert!(!places.contains(&untouched));
        assert_eq!(devices.len(), MASS_ROWS);
        assert!(
            is_empty_channel(&mut rx),
            "после одного события канал пуст, Lagged не было"
        );
    })
    .await
    .expect("mass_import вышел за бюджет");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mass_bulk_create_at_service_limit_sends_exactly_one_event() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = seed_place(&ctx, "Кабинет A").await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        // Потолок сервиса — 100 строк за вызов (>= 200 через bulk недостижимо).
        let made = ctx
            .devices
            .bulk_create_with_printer(new_device("Флешка Тест-9", Some(a)), 100, None)
            .await
            .expect("bulk 100");
        assert_eq!(made.len(), 100);

        let (places, devices, _) = assert_one_entities(&mut rx, "mass bulk");
        assert_eq!(places, vec![a]);
        assert_eq!(devices.len(), 100);
        assert!(is_empty_channel(&mut rx), "Lagged не было");
    })
    .await
    .expect("mass_bulk вышел за бюджет");
}

// ---------------------------------------------------------------------------
// Привязка к реестру (одна строка)
// ---------------------------------------------------------------------------

#[test]
fn scenarios_cover_every_broadcasting_row() {
    entities_support::scenarios::assert_behaviour_file_covers(
        "entities_changed_devices.rs",
        "device_service.rs",
    );
}

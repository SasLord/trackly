//! Phase 41.7, план 10: слой (2) реестрового гейта для `cartridge_service.rs` (D-16).
//!
//! На КАЖДУЮ строку `Broadcasts` файла `cartridge_service.rs` приходится ровно
//! один `#[tokio::test] async fn scenario_<функция>`; привязка к реестру идёт по
//! исходнику этого файла (см. `entities_support::scenarios`). Откаты, ловушка P4
//! и дополнительные случаи носят другие префиксы (`rollback_`, `p4_`, `create_`,
//! `transition_`, `update_`) и сценариями не считаются.
//!
//! `cartridges.place_id` входит в счётчик дерева мест (`device_count +
//! cartridge_count`), поэтому движение картриджа меняет счётчик и ПРЕЖНЕГО, и
//! НОВОГО места.
//!
//! Настоящий `AppCtx::build`, подписка на `ctx.ws_broadcast` ДО мутации,
//! `try_recv()` сразу после `.await`. Списки сравниваются как множества.
//! Фикстуры расходятся намеренно (несколько мест, нетронутое место, которое
//! в событии быть не должно). Данные вымышленные.
//!
//! Массовой операции у `cartridge_service` нет (каждая мутация работает с одним
//! картриджем), поэтому требование D-01 «N >= 200 → одно событие» здесь не
//! применимо; массовые пути покрыты в планах устройств и мест.

mod entities_support;

use std::time::Duration;

use entities_support::fixture::{
    admin, assert_no_entities, assert_one_entities, collect_events, drain, make_test_ctx,
    seed_device, seed_place,
};
use rusqlite::params;
use trackly_app::context::AppCtx;
use trackly_app::dto::cartridge::{
    CartridgeCreateDto, CartridgeDto, CartridgeModelCreateDto, CartridgeTransitionPayload,
};
use trackly_app::dto::number_template::NumberFieldInput;
use trackly_app::dto::printer::WsEvent;
use trackly_core::error::AppError;
use trackly_infra::error_conversions::map_rusqlite;

/// Бюджет каждого теста (форма `number_space_broadcast_gate.rs`).
const BUDGET: Duration = Duration::from_secs(60);

/// Тройка списков `EntitiesChanged`: (place_ids, device_ids, group_ids).
type Triple = (Vec<i64>, Vec<i64>, Vec<i64>);

fn sorted(mut v: Vec<i64>) -> Vec<i64> {
    v.sort_unstable();
    v.dedup();
    v
}

fn number_input(value: &str) -> NumberFieldInput {
    NumberFieldInput {
        value: value.to_string(),
        template_id: None,
        confirm_mismatch: false,
        confirm_script_mix: false,
    }
}

async fn seed_model(ctx: &AppCtx) -> i64 {
    ctx.cartridges
        .model_create(CartridgeModelCreateDto {
            brand: "Brand".into(),
            model: "TX-100".into(),
            kind_id: 1,
            color: Some("Чёрный".into()),
            notes: None,
            compatibility: vec![],
        })
        .await
        .expect("seed model")
        .id
}

async fn make_cartridge(
    ctx: &AppCtx,
    model_id: i64,
    code: &str,
    place: Option<i64>,
) -> CartridgeDto {
    ctx.cartridges
        .create(CartridgeCreateDto {
            model_id,
            number_input: number_input(code),
            state_id: Some(1),
            place_id: place,
            notes: None,
        })
        .await
        .expect("create cartridge")
        .expect_created("create cartridge")
}

fn install(
    cart: &CartridgeDto,
    printer: Option<i64>,
    place: Option<i64>,
) -> CartridgeTransitionPayload {
    CartridgeTransitionPayload::Install {
        cartridge_id: cart.id,
        version: cart.version,
        date_utc: 1_700_000_000,
        given_by_name: "Иванов И.И.".into(),
        given_to_name: "Петров П.П.".into(),
        place_id: place,
        printer_device_id: printer,
        previous_cartridge_state_id: None,
        previous_cartridge_place_id: None,
    }
}

async fn printer_place(ctx: &AppCtx, id: i64) -> Option<i64> {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(
                "SELECT place_id FROM devices WHERE id = ?1",
                params![id],
                |r| r.get::<_, Option<i64>>(0),
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("read printer place")
}

/// (число NumberSpaceChanged, тройки всех EntitiesChanged) из накопленного.
fn split_events(events: Vec<WsEvent>) -> (usize, Vec<Triple>) {
    let mut number_space = 0;
    let mut entities = Vec::new();
    for ev in events {
        match ev {
            WsEvent::NumberSpaceChanged { .. } => number_space += 1,
            WsEvent::EntitiesChanged {
                place_ids,
                device_ids,
                group_ids,
            } => entities.push((place_ids, device_ids, group_ids)),
            _ => {}
        }
    }
    (number_space, entities)
}

// ---------------------------------------------------------------------------
// Сценарии (по одному на строку реестра)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_create() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let model = seed_model(&ctx).await;
        let a = seed_place(&ctx, "Склад A").await;
        let untouched = seed_place(&ctx, "Склад Z").await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        make_cartridge(&ctx, model, "C-0001", Some(a)).await;

        let (places, devices, groups) = assert_one_entities(&mut rx, "create");
        assert_eq!(
            sorted(places.clone()),
            vec![a],
            "place_ids == место картриджа"
        );
        assert!(!places.contains(&untouched), "нетронутое место не нужно");
        assert!(devices.is_empty(), "device_ids пуст");
        assert!(groups.is_empty(), "group_ids пуст");
    })
    .await
    .expect("scenario_create вышел за бюджет");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_update() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let model = seed_model(&ctx).await;
        let old = seed_place(&ctx, "Склад A").await;
        let new = seed_place(&ctx, "Склад B").await;
        let untouched = seed_place(&ctx, "Склад Z").await;
        let cart = make_cartridge(&ctx, model, "C-0002", Some(old)).await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        // Перенос со старого места на новое: в событии ОБА (счётчик прежнего
        // места у второго клиента иначе останется устаревшим).
        let moved = ctx
            .cartridges
            .update(&admin(), cart.id, cart.version, Some(new), None)
            .await
            .expect("update");
        assert_eq!(moved.place_id, Some(new));

        let (places, devices, groups) = assert_one_entities(&mut rx, "update");
        assert_eq!(
            sorted(places.clone()),
            sorted(vec![old, new]),
            "прежнее и новое место"
        );
        assert!(!places.contains(&untouched), "нетронутое место не нужно");
        assert!(devices.is_empty());
        assert!(groups.is_empty());

        // Правка без смены места: событие всё равно приходит (место одно).
        ctx.cartridges
            .update(
                &admin(),
                cart.id,
                moved.version,
                Some(new),
                Some("заметка".into()),
            )
            .await
            .expect("update notes");
        let (places, _, _) = assert_one_entities(&mut rx, "update notes only");
        assert_eq!(sorted(places), vec![new]);
    })
    .await
    .expect("scenario_update вышел за бюджет");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_delete() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let model = seed_model(&ctx).await;
        let a = seed_place(&ctx, "Склад A").await;
        let untouched = seed_place(&ctx, "Склад Z").await;
        let cart = make_cartridge(&ctx, model, "C-0003", Some(a)).await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        ctx.cartridges
            .delete(cart.id, cart.version)
            .await
            .expect("delete");

        let (places, devices, groups) = assert_one_entities(&mut rx, "delete");
        assert_eq!(
            sorted(places.clone()),
            vec![a],
            "place_ids == место удалённого"
        );
        assert!(!places.contains(&untouched));
        assert!(devices.is_empty());
        assert!(groups.is_empty());
    })
    .await
    .expect("scenario_delete вышел за бюджет");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_transition() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let model = seed_model(&ctx).await;
        let storage = seed_place(&ctx, "Склад A").await;
        let office = seed_place(&ctx, "Кабинет B").await;
        let untouched = seed_place(&ctx, "Склад Z").await;
        // Принтер стоит в другом месте, чем картридж.
        let printer = seed_device(&ctx, 2, "Принтер Тест-1", "P-1", Some(office), None).await;
        let cart = make_cartridge(&ctx, model, "C-0004", Some(storage)).await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        // Install без явного места: место берётся от принтера.
        let installed = ctx
            .cartridges
            .transition(&admin(), install(&cart, Some(printer), None))
            .await
            .expect("install");
        assert_eq!(installed.place_id, Some(office));

        let (places, devices, groups) = assert_one_entities(&mut rx, "transition install");
        assert_eq!(
            sorted(places.clone()),
            sorted(vec![storage, office]),
            "прежнее место картриджа и новое (место принтера)"
        );
        assert!(!places.contains(&untouched));
        assert!(
            devices.is_empty(),
            "у принтера место уже было: устройства не менялись"
        );
        assert!(groups.is_empty());

        // Возврат на склад: движение обратно в `storage`.
        ctx.cartridges
            .transition(
                &admin(),
                CartridgeTransitionPayload::ReturnToStock {
                    cartridge_id: installed.id,
                    version: installed.version,
                    state_id: 3,
                    place_id: Some(storage),
                    notes: None,
                },
            )
            .await
            .expect("return to stock");
        let (places, _, _) = assert_one_entities(&mut rx, "transition return");
        assert_eq!(sorted(places), sorted(vec![office, storage]));
    })
    .await
    .expect("scenario_transition вышел за бюджет");
}

// ---------------------------------------------------------------------------
// Дополнительные случаи transition / update / create
// ---------------------------------------------------------------------------

/// Install с явным местом в принтер БЕЗ места: сервис дозаполняет место принтера,
/// поэтому меняется и устройство (device_ids), и счётчик нового места.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn transition_install_backfilling_printer_place_reports_the_device() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let model = seed_model(&ctx).await;
        let storage = seed_place(&ctx, "Склад A").await;
        let office = seed_place(&ctx, "Кабинет B").await;
        let untouched = seed_place(&ctx, "Склад Z").await;
        let printer = seed_device(&ctx, 2, "Принтер Тест-2", "P-2", None, None).await;
        let cart = make_cartridge(&ctx, model, "C-0005", Some(storage)).await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        ctx.cartridges
            .transition(&admin(), install(&cart, Some(printer), Some(office)))
            .await
            .expect("install");
        assert_eq!(
            printer_place(&ctx, printer).await,
            Some(office),
            "место принтера дозаполнено"
        );

        let (places, devices, _) = assert_one_entities(&mut rx, "install backfill");
        assert_eq!(sorted(places.clone()), sorted(vec![storage, office]));
        assert!(!places.contains(&untouched));
        assert_eq!(
            devices,
            vec![printer],
            "device_ids == принтер с дозаполненным местом"
        );
    })
    .await
    .expect("transition_install_backfilling вышел за бюджет");
}

/// Install вытесняет прежний картридж принтера (авто-возврат D-16): места
/// вытесненного картриджа (до и после) тоже в событии.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn transition_install_auto_return_includes_previous_cartridge_places() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let model = seed_model(&ctx).await;
        let fresh_stock = seed_place(&ctx, "Склад A").await;
        let office = seed_place(&ctx, "Кабинет B").await;
        let returned_to = seed_place(&ctx, "Склад C").await;
        let untouched = seed_place(&ctx, "Склад Z").await;
        let printer = seed_device(&ctx, 2, "Принтер Тест-3", "P-3", Some(office), None).await;

        let first = make_cartridge(&ctx, model, "C-0006", Some(fresh_stock)).await;
        ctx.cartridges
            .transition(&admin(), install(&first, Some(printer), None))
            .await
            .expect("install first");
        let second = make_cartridge(&ctx, model, "C-0007", Some(fresh_stock)).await;

        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        let mut payload = install(&second, Some(printer), None);
        if let CartridgeTransitionPayload::Install {
            previous_cartridge_place_id,
            ..
        } = &mut payload
        {
            *previous_cartridge_place_id = Some(returned_to);
        }
        ctx.cartridges
            .transition(&admin(), payload)
            .await
            .expect("install second");

        // Новый картридж: fresh_stock -> office. Вытесненный: office -> returned_to.
        let (places, devices, _) = assert_one_entities(&mut rx, "install auto-return");
        assert_eq!(
            sorted(places.clone()),
            sorted(vec![fresh_stock, office, returned_to])
        );
        assert!(!places.contains(&untouched));
        assert!(devices.is_empty());
    })
    .await
    .expect("transition_install_auto_return вышел за бюджет");
}

/// Списание не меняет место: событие всё равно приходит (место картриджа входит
/// в счётчик, а картридж «Списано» его покидает), без дублей мест.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn transition_write_off_reports_the_cartridge_place_once() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let model = seed_model(&ctx).await;
        let a = seed_place(&ctx, "Склад A").await;
        let cart = make_cartridge(&ctx, model, "C-0008", Some(a)).await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        ctx.cartridges
            .transition(
                &admin(),
                CartridgeTransitionPayload::WriteOff {
                    cartridge_id: cart.id,
                    version: cart.version,
                    date_utc: 1_700_000_000,
                    notes: None,
                },
            )
            .await
            .expect("write off");
        let (places, devices, groups) = assert_one_entities(&mut rx, "write off");
        assert_eq!(places, vec![a]);
        assert!(devices.is_empty());
        assert!(groups.is_empty());
    })
    .await
    .expect("transition_write_off вышел за бюджет");
}

/// Снятие места (update на None): в событии только прежнее место, без `None`.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_clearing_place_reports_only_the_previous_place() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let model = seed_model(&ctx).await;
        let a = seed_place(&ctx, "Склад A").await;
        let cart = make_cartridge(&ctx, model, "C-0009", Some(a)).await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        ctx.cartridges
            .update(&admin(), cart.id, cart.version, None, None)
            .await
            .expect("update");
        let (places, _, _) = assert_one_entities(&mut rx, "update clearing place");
        assert_eq!(places, vec![a]);
    })
    .await
    .expect("update_clearing_place вышел за бюджет");
}

/// Картридж без места не входит ни в один счётчик: EntitiesChanged нет,
/// NumberSpaceChanged идёт как раньше (решение W2).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_without_place_emits_number_space_only() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let model = seed_model(&ctx).await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        make_cartridge(&ctx, model, "C-0010", None).await;

        let (number_space, entities) = split_events(collect_events(&mut rx));
        assert_eq!(number_space, 1, "NumberSpaceChanged идёт как раньше");
        assert!(
            entities.is_empty(),
            "картридж без места: EntitiesChanged нет"
        );
    })
    .await
    .expect("create_without_place вышел за бюджет");
}

// ---------------------------------------------------------------------------
// Ловушка P4: рядом идёт NumberSpaceChanged, новый вызов его не заменяет
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn p4_create_emits_number_space_and_entities_events() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let model = seed_model(&ctx).await;
        let a = seed_place(&ctx, "Склад A").await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        make_cartridge(&ctx, model, "C-0011", Some(a)).await;

        let (number_space, entities) = split_events(collect_events(&mut rx));
        assert_eq!(number_space, 1, "ровно один NumberSpaceChanged");
        assert_eq!(entities.len(), 1, "ровно один EntitiesChanged");
        assert_eq!(entities[0].0, vec![a]);
    })
    .await
    .expect("p4_create вышел за бюджет");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn p4_delete_emits_number_space_and_entities_events() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let model = seed_model(&ctx).await;
        let a = seed_place(&ctx, "Склад A").await;
        let cart = make_cartridge(&ctx, model, "C-0012", Some(a)).await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        ctx.cartridges
            .delete(cart.id, cart.version)
            .await
            .expect("delete");

        let (number_space, entities) = split_events(collect_events(&mut rx));
        assert_eq!(number_space, 1, "ровно один NumberSpaceChanged");
        assert_eq!(entities.len(), 1, "ровно один EntitiesChanged");
        assert_eq!(entities[0].0, vec![a]);
    })
    .await
    .expect("p4_delete вышел за бюджет");
}

// ---------------------------------------------------------------------------
// Откаты: события нет
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn rollback_update_with_stale_version_sends_nothing() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let model = seed_model(&ctx).await;
        let a = seed_place(&ctx, "Склад A").await;
        let b = seed_place(&ctx, "Склад B").await;
        let cart = make_cartridge(&ctx, model, "C-0013", Some(a)).await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        let err = ctx
            .cartridges
            .update(&admin(), cart.id, cart.version + 7, Some(b), None)
            .await
            .expect_err("stale version");
        assert!(
            matches!(err, AppError::OptimisticLockMismatch { .. }),
            "ожидали OptimisticLockMismatch, получили {err:?}"
        );
        assert_no_entities(&mut rx, "update stale");
    })
    .await
    .expect("rollback_update вышел за бюджет");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn rollback_delete_with_stale_version_sends_nothing() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let model = seed_model(&ctx).await;
        let a = seed_place(&ctx, "Склад A").await;
        let cart = make_cartridge(&ctx, model, "C-0014", Some(a)).await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        let err = ctx
            .cartridges
            .delete(cart.id, cart.version + 7)
            .await
            .expect_err("stale version");
        assert!(
            matches!(err, AppError::OptimisticLockMismatch { .. }),
            "{err:?}"
        );
        let (number_space, entities) = split_events(collect_events(&mut rx));
        assert_eq!(number_space, 0, "NumberSpaceChanged тоже нет");
        assert!(entities.is_empty());
    })
    .await
    .expect("rollback_delete вышел за бюджет");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn rollback_transition_invalid_sends_nothing() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let model = seed_model(&ctx).await;
        let a = seed_place(&ctx, "Склад A").await;
        let cart = make_cartridge(&ctx, model, "C-0015", Some(a)).await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        // Картридж «На складе»: возврат на склад из «В работе» недопустим.
        let err = ctx
            .cartridges
            .transition(
                &admin(),
                CartridgeTransitionPayload::ReturnToStock {
                    cartridge_id: cart.id,
                    version: cart.version,
                    state_id: 3,
                    place_id: Some(a),
                    notes: None,
                },
            )
            .await
            .expect_err("invalid transition");
        assert!(!matches!(err, AppError::Internal { .. }), "{err:?}");
        assert_no_entities(&mut rx, "transition invalid");

        // Устаревшая версия.
        let err = ctx
            .cartridges
            .transition(&admin(), {
                let mut c = cart.clone();
                c.version += 5;
                install(&c, None, Some(a))
            })
            .await
            .expect_err("stale version");
        assert!(
            matches!(err, AppError::OptimisticLockMismatch { .. }),
            "{err:?}"
        );
        assert_no_entities(&mut rx, "transition stale");
    })
    .await
    .expect("rollback_transition вышел за бюджет");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn rollback_create_with_empty_number_sends_nothing() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let model = seed_model(&ctx).await;
        let a = seed_place(&ctx, "Склад A").await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        let err = ctx
            .cartridges
            .create(CartridgeCreateDto {
                model_id: model,
                number_input: number_input("   "),
                state_id: Some(1),
                place_id: Some(a),
                notes: None,
            })
            .await
            .expect_err("empty number");
        assert!(matches!(err, AppError::Validation { .. }), "{err:?}");
        let (number_space, entities) = split_events(collect_events(&mut rx));
        assert_eq!(number_space, 0, "NumberSpaceChanged тоже нет");
        assert!(entities.is_empty());
    })
    .await
    .expect("rollback_create вышел за бюджет");
}

// ---------------------------------------------------------------------------
// Привязка к реестру
// ---------------------------------------------------------------------------

#[test]
fn scenarios_cover_every_broadcasting_row() {
    entities_support::scenarios::assert_behaviour_file_covers(
        "entities_changed_cartridges.rs",
        "cartridge_service.rs",
    );
}

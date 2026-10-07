//! Phase 41.7, план 06: слой (2) реестрового гейта для `place_service.rs`.
//!
//! На КАЖДУЮ строку `Broadcasts` файла `place_service.rs` приходится ровно один
//! `#[tokio::test] async fn scenario_<функция>`; привязка к реестру идёт по
//! исходнику этого файла (см. `entities_support::scenarios`). Откаты, массовая
//! операция и покрытие носят другие префиксы (`rollback_`, `mass_`) и сценариями
//! не считаются.
//!
//! Настоящий `AppCtx::build`, подписка на `ctx.ws_broadcast` ДО мутации,
//! `try_recv()` сразу после `.await`. Ожидаемые списки заданы литералами из
//! фикстуры (не выведены той же формулой, что в сервисе) и сравниваются как
//! множества. Данные вымышленные.

mod entities_support;

use std::time::Duration;

use entities_support::fixture::{
    admin, assert_no_entities, assert_one_entities, drain, make_test_ctx, seed_device,
    seed_group_type_and_group,
};
use rusqlite::params;
use trackly_app::context::AppCtx;
use trackly_app::dto::place::PlaceDto;
use trackly_core::domain::places::{PlaceKind, PlaceNew};
use trackly_core::error::AppError;
use trackly_infra::error_conversions::map_rusqlite;

/// Бюджет каждого теста (форма `number_space_broadcast_gate.rs`).
const BUDGET: Duration = Duration::from_secs(60);

fn sorted(mut v: Vec<i64>) -> Vec<i64> {
    v.sort_unstable();
    v.dedup();
    v
}

fn new_place(kind: PlaceKind, name: &str, parent_id: Option<i64>) -> PlaceNew {
    PlaceNew {
        parent_id,
        kind,
        name: name.to_string(),
        level: None,
        is_storage: false,
        sort_order: None,
        notes: None,
    }
}

async fn mk(ctx: &AppCtx, kind: PlaceKind, name: &str, parent: Option<i64>) -> PlaceDto {
    ctx.places
        .create(&admin(), new_place(kind, name, parent))
        .await
        .expect("create place")
}

async fn version_of(ctx: &AppCtx, id: i64) -> i64 {
    ctx.places.get(&admin(), id).await.expect("get").version
}

// ---------------------------------------------------------------------------
// Сценарии: по одному на строку Broadcasts
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_create() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let mut rx = ctx.ws_broadcast.subscribe();

        // Корень: родителя нет, в событии только он сам.
        let root = ctx
            .places
            .create(&admin(), new_place(PlaceKind::Building, "Корпус 1", None))
            .await
            .expect("create root");
        let (places, devices, groups) = assert_one_entities(&mut rx, "create корня");
        assert_eq!(sorted(places), vec![root.id], "корень: place_ids == [id]");
        assert!(devices.is_empty(), "create: device_ids пуст");
        assert!(groups.is_empty(), "create: group_ids пуст");

        // Потомок: id и родитель.
        let child = ctx
            .places
            .create(
                &admin(),
                new_place(PlaceKind::Room, "Кабинет 101", Some(root.id)),
            )
            .await
            .expect("create child");
        assert_eq!(child.parent_id, Some(root.id));
        let (places, devices, groups) = assert_one_entities(&mut rx, "create потомка");
        assert_eq!(
            sorted(places),
            sorted(vec![child.id, root.id]),
            "потомок: place_ids == [id, parent]"
        );
        assert!(devices.is_empty() && groups.is_empty());
    })
    .await
    .expect("scenario_create вышел за бюджет");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_rename() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let root = mk(&ctx, PlaceKind::Building, "Корпус 1", None).await;
        let room = mk(&ctx, PlaceKind::Room, "Кабинет 101", Some(root.id)).await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        let renamed = ctx
            .places
            .rename(&admin(), room.id, "Кабинет 102".to_string(), room.version)
            .await
            .expect("rename");
        assert_eq!(renamed.name, "Кабинет 102");
        let (places, devices, groups) = assert_one_entities(&mut rx, "rename");
        assert_eq!(sorted(places), vec![room.id], "rename: place_ids == [id]");
        assert!(devices.is_empty() && groups.is_empty());
    })
    .await
    .expect("scenario_rename вышел за бюджет");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_set_path_variant() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let root = mk(&ctx, PlaceKind::Building, "Корпус 1", None).await;
        let room = mk(&ctx, PlaceKind::Room, "Кабинет 101", Some(root.id)).await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        let updated = ctx
            .places
            .set_path_variant(&admin(), room.id, Some("last".to_string()), room.version)
            .await
            .expect("set_path_variant");
        assert_eq!(updated.path_variant_override.as_deref(), Some("last"));
        let (places, devices, groups) = assert_one_entities(&mut rx, "set_path_variant");
        assert_eq!(
            sorted(places),
            vec![room.id],
            "set_path_variant: place_ids == [id]"
        );
        assert!(devices.is_empty() && groups.is_empty());
    })
    .await
    .expect("scenario_set_path_variant вышел за бюджет");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_move_node() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        // Фикстура расходится: старый родитель A, новый B, перемещаемый узел N.
        let a = mk(&ctx, PlaceKind::Building, "Корпус А", None).await;
        let b = mk(&ctx, PlaceKind::Building, "Корпус Б", None).await;
        let n = mk(&ctx, PlaceKind::Room, "Кабинет 101", Some(a.id)).await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        let moved = ctx
            .places
            .move_node(&admin(), n.id, Some(b.id), n.version)
            .await
            .expect("move_node");
        assert_eq!(moved.parent_id, Some(b.id), "ответ: новый родитель");
        let (places, devices, groups) = assert_one_entities(&mut rx, "move_node");
        assert_eq!(
            sorted(places),
            sorted(vec![n.id, a.id, b.id]),
            "move_node: place_ids == [id, старый parent, новый parent]"
        );
        assert!(devices.is_empty() && groups.is_empty());

        // Перенос в корень: нового родителя нет, None в список не попадает.
        let n_now = version_of(&ctx, n.id).await;
        ctx.places
            .move_node(&admin(), n.id, None, n_now)
            .await
            .expect("move_node to root");
        let (places, _, _) = assert_one_entities(&mut rx, "move_node в корень");
        assert_eq!(
            sorted(places),
            sorted(vec![n.id, b.id]),
            "в корень: [id, старый parent]"
        );
    })
    .await
    .expect("scenario_move_node вышел за бюджет");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_archive() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let root = mk(&ctx, PlaceKind::Building, "Корпус 1", None).await;
        let room = mk(&ctx, PlaceKind::Room, "Кабинет 101", Some(root.id)).await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        ctx.places
            .archive(&admin(), room.id, room.version)
            .await
            .expect("archive");
        let (places, devices, groups) = assert_one_entities(&mut rx, "archive");
        assert_eq!(sorted(places), vec![room.id], "archive: place_ids == [id]");
        assert!(devices.is_empty() && groups.is_empty());
    })
    .await
    .expect("scenario_archive вышел за бюджет");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_unarchive() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let root = mk(&ctx, PlaceKind::Building, "Корпус 1", None).await;
        let room = mk(&ctx, PlaceKind::Room, "Кабинет 101", Some(root.id)).await;
        ctx.places
            .archive(&admin(), room.id, room.version)
            .await
            .expect("archive");
        let v = version_of(&ctx, room.id).await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        ctx.places
            .unarchive(&admin(), room.id, v)
            .await
            .expect("unarchive");
        let (places, devices, groups) = assert_one_entities(&mut rx, "unarchive");
        assert_eq!(
            sorted(places),
            vec![room.id],
            "unarchive: place_ids == [id]"
        );
        assert!(devices.is_empty() && groups.is_empty());
    })
    .await
    .expect("scenario_unarchive вышел за бюджет");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_delete_hard() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let root = mk(&ctx, PlaceKind::Building, "Корпус 1", None).await;
        let room = mk(&ctx, PlaceKind::Room, "Кабинет 101", Some(root.id)).await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        ctx.places
            .delete_hard(&admin(), room.id, room.version)
            .await
            .expect("delete_hard");
        let (places, devices, groups) = assert_one_entities(&mut rx, "delete_hard");
        assert_eq!(
            sorted(places),
            sorted(vec![room.id, root.id]),
            "delete_hard: place_ids == [id, parent]"
        );
        assert!(devices.is_empty() && groups.is_empty());
    })
    .await
    .expect("scenario_delete_hard вышел за бюджет");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_move_subtree_contents() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        // Поддерево: корень R, потомки C и C2; цель T вне поддерева.
        let r = mk(&ctx, PlaceKind::Building, "Корпус А", None).await;
        let c = mk(&ctx, PlaceKind::Room, "Кабинет 101", Some(r.id)).await;
        let c2 = mk(&ctx, PlaceKind::Room, "Кабинет 102", Some(r.id)).await;
        let t = mk(&ctx, PlaceKind::Building, "Корпус Б", None).await;

        // Одиночные устройства в корне и в потомке.
        let _d_root =
            seed_device(&ctx, 1, "Ноутбук Иванов И.И.", "PL-INV-1", Some(r.id), None).await;
        let _d_child =
            seed_device(&ctx, 1, "Монитор Петров П.П.", "PL-INV-2", Some(c.id), None).await;
        // Группа в потомке C2 с устройством-членом: едет целиком.
        let (_ty, group) = seed_group_type_and_group(&ctx, "АРМ #1", 1, Some(c2.id)).await;
        let _d_member = seed_device(
            &ctx,
            1,
            "Системный блок",
            "PL-INV-3",
            Some(c2.id),
            Some(group),
        )
        .await;

        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        let moved = ctx
            .places
            .move_subtree_contents(&admin(), r.id, t.id, None)
            .await
            .expect("move_subtree_contents");
        assert_eq!(moved, 3, "счётчик ответа не меняется: все элементы");

        let (places, devices, groups) = assert_one_entities(&mut rx, "move_subtree_contents");
        assert_eq!(
            sorted(places),
            sorted(vec![r.id, c.id, c2.id, t.id]),
            "place_ids: корень, цель, прежние места элементов и места групп"
        );
        assert_eq!(
            sorted(groups),
            vec![group],
            "group_ids == корни перенесённых групп"
        );
        assert!(
            devices.is_empty(),
            "Q4: id устройств в замыкании недоступны, device_ids пуст"
        );
    })
    .await
    .expect("scenario_move_subtree_contents вышел за бюджет");
}

// ---------------------------------------------------------------------------
// Откат: события быть не должно
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn rollback_delete_hard_conflict_sends_nothing() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let root = mk(&ctx, PlaceKind::Building, "Корпус 1", None).await;
        let room = mk(&ctx, PlaceKind::Room, "Кабинет 101", Some(root.id)).await;
        let _dev = seed_device(
            &ctx,
            1,
            "Ноутбук Иванов И.И.",
            "PL-INV-9",
            Some(room.id),
            None,
        )
        .await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        let err = ctx
            .places
            .delete_hard(&admin(), room.id, room.version)
            .await
            .expect_err("в месте есть устройство");
        assert!(
            matches!(err, AppError::Conflict { .. }),
            "ожидали Conflict, получили {err:?}"
        );
        assert_no_entities(&mut rx, "delete_hard с Conflict");
    })
    .await
    .expect("rollback_delete_hard_conflict вышел за бюджет");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn rollback_rename_stale_version_sends_nothing() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let root = mk(&ctx, PlaceKind::Building, "Корпус 1", None).await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        let err = ctx
            .places
            .rename(&admin(), root.id, "Корпус 2".to_string(), root.version + 99)
            .await
            .expect_err("версия устарела");
        assert!(
            matches!(err, AppError::OptimisticLockMismatch { .. }),
            "ожидали OptimisticLockMismatch, получили {err:?}"
        );
        assert_no_entities(&mut rx, "rename с устаревшей версией");
    })
    .await
    .expect("rollback_rename вышел за бюджет");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn rollback_move_node_stale_version_sends_nothing() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let a = mk(&ctx, PlaceKind::Building, "Корпус А", None).await;
        let b = mk(&ctx, PlaceKind::Building, "Корпус Б", None).await;
        let n = mk(&ctx, PlaceKind::Room, "Кабинет 101", Some(a.id)).await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        let err = ctx
            .places
            .move_node(&admin(), n.id, Some(b.id), n.version + 99)
            .await
            .expect_err("версия устарела");
        assert!(
            matches!(err, AppError::OptimisticLockMismatch { .. }),
            "ожидали OptimisticLockMismatch, получили {err:?}"
        );
        assert_no_entities(&mut rx, "move_node с устаревшей версией");
    })
    .await
    .expect("rollback_move_node вышел за бюджет");
}

// ---------------------------------------------------------------------------
// D-01: массовая операция даёт ровно одно событие
// ---------------------------------------------------------------------------

const MASS_DEVICES: usize = 220;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mass_move_subtree_contents_sends_exactly_one_event() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let r = mk(&ctx, PlaceKind::Building, "Корпус А", None).await;
        let c = mk(&ctx, PlaceKind::Room, "Кабинет 101", Some(r.id)).await;
        let t = mk(&ctx, PlaceKind::Building, "Корпус Б", None).await;

        // Сидирование одним циклом внутри writer, а не сервисными вызовами.
        let (r_id, c_id) = (r.id, c.id);
        ctx.writer
            .execute(move |conn| {
                for i in 0..MASS_DEVICES {
                    let place = if i % 2 == 0 { r_id } else { c_id };
                    conn.execute(
                        "INSERT INTO devices (type_id, name, inventory_number, place_id, \
                         status_id, created_at_utc, updated_at_utc, version) \
                         VALUES (1, ?1, ?2, ?3, 1, 1700000000, 1700000000, 1)",
                        params![format!("Устройство {i}"), format!("MASS-{i}"), place],
                    )
                    .map_err(map_rusqlite)?;
                }
                Ok(())
            })
            .await
            .expect("seed mass devices");

        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        let moved = ctx
            .places
            .move_subtree_contents(&admin(), r.id, t.id, None)
            .await
            .expect("mass move");
        assert_eq!(moved, MASS_DEVICES, "перенесены все устройства");

        // Ровно одно событие, второго нет; Lagged (ёмкость 128) не наступил.
        let (places, devices, groups) = assert_one_entities(&mut rx, "массовый перенос");
        assert_eq!(sorted(places), sorted(vec![r.id, c.id, t.id]));
        assert!(devices.is_empty() && groups.is_empty());
        assert!(
            matches!(
                rx.try_recv(),
                Err(tokio::sync::broadcast::error::TryRecvError::Empty)
            ),
            "после единственного события канал пуст (без Lagged)"
        );
    })
    .await
    .expect("mass_move вышел за бюджет");
}

// ---------------------------------------------------------------------------
// Покрытие: привязка сценариев к реестру по исходнику этого файла
// ---------------------------------------------------------------------------

#[test]
fn scenarios_cover_every_broadcasting_row() {
    entities_support::scenarios::assert_behaviour_file_covers(
        "entities_changed_places.rs",
        "place_service.rs",
    );
}

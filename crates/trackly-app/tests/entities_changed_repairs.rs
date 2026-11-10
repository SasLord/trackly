//! Phase 41.4, план 10: слой (2) реестрового гейта для `repair_service.rs`.
//!
//! На КАЖДУЮ строку `Broadcasts` файла `repair_service.rs` приходится ровно один
//! `#[tokio::test] async fn scenario_<функция>` (привязка по исходнику этого файла,
//! см. `entities_support::scenarios`). Прочие случаи носят другие префиксы.
//!
//! Настоящий `AppCtx::build`, подписка на `ctx.ws_broadcast` ДО мутации,
//! `try_recv()` сразу после `.await`. Списки сравниваются как множества; нетронутое
//! место в событии быть не должно. Данные вымышленные.

mod entities_support;
mod repair_support;

use std::time::Duration;

use entities_support::fixture::{
    admin, assert_one_entities, drain, make_test_ctx, seed_device, seed_group_type_and_group,
    seed_place,
};
use repair_support::{manager, seed_user};
use rusqlite::params;
use trackly_app::context::AppCtx;
use trackly_app::dto::repair::{RepairCreateDto, RepairMoveChoiceDto};
use trackly_infra::error_conversions::map_rusqlite;

const BUDGET: Duration = Duration::from_secs(60);

fn sorted(mut v: Vec<i64>) -> Vec<i64> {
    v.sort_unstable();
    v.dedup();
    v
}

async fn storage_place(ctx: &AppCtx, name: &str) -> i64 {
    let p = seed_place(ctx, name).await;
    ctx.writer
        .execute(move |conn| {
            conn.execute("UPDATE places SET is_storage = 1 WHERE id = ?1", params![p])
                .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("storage");
    p
}

async fn new_request(ctx: &AppCtx, dev: i64) -> (i64, i64) {
    let requester = seed_user(ctx, "requester", "Сидоров С.С.").await;
    let r = ctx
        .repairs
        .create(
            RepairCreateDto {
                device_id: Some(dev),
                description: "Не включается".into(),
            },
            &manager(requester),
        )
        .await
        .expect("create");
    (r.request.id, r.request.version)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_accept() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let room = seed_place(&ctx, "Кабинет 12").await;
        let store = storage_place(&ctx, "Склад А").await;
        let untouched = seed_place(&ctx, "Склад Z").await;
        let dev = seed_device(&ctx, 1, "ПК-00014", "INV-14", Some(room), None).await;
        let (req, v) = new_request(&ctx, dev).await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        ctx.repairs
            .accept(
                req,
                v,
                RepairMoveChoiceDto::Move { place_id: store },
                &admin(),
            )
            .await
            .expect("accept");

        let (places, devices, groups) = assert_one_entities(&mut rx, "accept");
        assert_eq!(sorted(places.clone()), sorted(vec![room, store]));
        assert!(!places.contains(&untouched), "нетронутое место не нужно");
        assert_eq!(sorted(devices), vec![dev]);
        assert!(groups.is_empty());
    })
    .await
    .expect("scenario_accept вышел за бюджет");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_move_to_storage() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let room = seed_place(&ctx, "Кабинет 12").await;
        let store_a = storage_place(&ctx, "Склад А").await;
        let store_b = storage_place(&ctx, "Склад Б").await;
        let dev = seed_device(&ctx, 1, "ПК-00014", "INV-14", Some(room), None).await;
        let (req, v) = new_request(&ctx, dev).await;
        let res = ctx
            .repairs
            .accept(
                req,
                v,
                RepairMoveChoiceDto::Move { place_id: store_a },
                &admin(),
            )
            .await
            .expect("accept");
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        ctx.repairs
            .move_to_storage(req, res.request.version, store_b, &admin())
            .await
            .expect("move_to_storage");

        let (places, devices, groups) = assert_one_entities(&mut rx, "move_to_storage");
        assert_eq!(sorted(places.clone()), sorted(vec![store_a, store_b]));
        assert!(
            !places.contains(&room),
            "место до первого перемещения не затронуто"
        );
        assert_eq!(sorted(devices), vec![dev]);
        assert!(groups.is_empty());
    })
    .await
    .expect("scenario_move_to_storage вышел за бюджет");
}

/// Stay: места не менялись, но событие всё равно несёт устройство (статус сменился).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn stay_event_carries_device_without_places() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let room = seed_place(&ctx, "Кабинет 12").await;
        let dev = seed_device(&ctx, 1, "ПК-00014", "INV-14", Some(room), None).await;
        let (req, v) = new_request(&ctx, dev).await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        ctx.repairs
            .accept(req, v, RepairMoveChoiceDto::Stay, &admin())
            .await
            .expect("accept");

        let (places, devices, _g) = assert_one_entities(&mut rx, "accept stay");
        assert!(places.is_empty(), "{places:?}");
        assert_eq!(sorted(devices), vec![dev]);
    })
    .await
    .expect("stay вышел за бюджет");
}

/// Якорь: событие несёт блок целиком (устройства и группу).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn accept_anchor_event_carries_block() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;
        let room = seed_place(&ctx, "Кабинет 12").await;
        let store = storage_place(&ctx, "Склад А").await;
        let (_t, group) = seed_group_type_and_group(&ctx, "Комплект", 1, Some(room)).await;
        let anchor = seed_device(&ctx, 1, "ПК-00014", "INV-14", Some(room), Some(group)).await;
        let part = seed_device(&ctx, 1, "Монитор", "INV-15", Some(room), Some(group)).await;
        ctx.writer
            .execute(move |conn| {
                conn.execute(
                    "UPDATE groups SET anchor_device_id = ?1 WHERE id = ?2",
                    params![anchor, group],
                )
                .map_err(map_rusqlite)?;
                Ok(())
            })
            .await
            .expect("anchor");
        let (req, v) = new_request(&ctx, anchor).await;
        let mut rx = ctx.ws_broadcast.subscribe();
        drain(&mut rx);

        ctx.repairs
            .accept(
                req,
                v,
                RepairMoveChoiceDto::Move { place_id: store },
                &admin(),
            )
            .await
            .expect("accept");

        let (places, devices, groups) = assert_one_entities(&mut rx, "accept anchor");
        assert_eq!(sorted(places), sorted(vec![room, store]));
        assert_eq!(sorted(devices), sorted(vec![anchor, part]));
        assert_eq!(groups, vec![group]);
    })
    .await
    .expect("anchor вышел за бюджет");
}

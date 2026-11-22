//! Фаза 41.10, план 08 (HST-05/HST-06): события `status_changed` у писателя статуса
//! ремонта (`apply_device_state_in_tx`, ST-10).
//! Префикс тестов: `history_status_repair_`. Фикстуры вымышленные.

mod entities_support;
mod repair_support;

use entities_support::fixture::{
    admin, make_test_ctx, seed_device, seed_group_type_and_group, seed_place,
};
use repair_support::{manager, seed_user};
use rusqlite::params;
use trackly_app::context::AppCtx;
use trackly_app::dto::repair::{
    RepairCompleteDto, RepairCreateDto, RepairMoveChoiceDto, RepairRejectDto,
};
use trackly_infra::error_conversions::map_rusqlite;

/// (from, to, source, request_id, movement_id) событий статуса устройства по порядку.
type Ev = (
    Option<String>,
    Option<String>,
    String,
    Option<i64>,
    Option<i64>,
);

async fn status_events(ctx: &AppCtx, dev: i64) -> Vec<Ev> {
    ctx.writer
        .execute(move |conn| {
            let mut st = conn
                .prepare(
                    "SELECT status_from_code, status_to_code, source, request_id, movement_id \
                     FROM history_events \
                     WHERE subject_type = 'device' AND subject_id = ?1 AND kind = 'status_changed' \
                     ORDER BY id",
                )
                .map_err(map_rusqlite)?;
            let rows = st
                .query_map(params![dev], |r| {
                    Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
                })
                .map_err(map_rusqlite)?;
            let mut out = Vec::new();
            for r in rows {
                out.push(r.map_err(map_rusqlite)?);
            }
            Ok(out)
        })
        .await
        .expect("events")
}

async fn status_code(ctx: &AppCtx, dev: i64) -> String {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(
                "SELECT ds.code FROM devices d JOIN device_statuses ds ON ds.id = d.status_id \
                 WHERE d.id = ?1",
                params![dev],
                |r| r.get::<_, String>(0),
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("code")
}

async fn movement_id_of(ctx: &AppCtx, dev: i64, req: i64) -> Option<i64> {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(
                "SELECT id FROM place_movements \
                 WHERE entity_type = 'device' AND entity_id = ?1 AND request_id = ?2",
                params![dev, req],
                |r| r.get::<_, i64>(0),
            )
            .map(Some)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(map_rusqlite(other)),
            })
        })
        .await
        .expect("movement")
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

#[tokio::test]
async fn history_status_repair_takes_device() {
    let (ctx, _d) = make_test_ctx().await;
    let room = seed_place(&ctx, "Кабинет 12").await;
    let hold = storage_place(&ctx, "Склад А").await;
    let dev = seed_device(&ctx, 1, "ПК-00014", "INV-14", Some(room), None).await;
    let (req, v) = new_request(&ctx, dev).await;
    let before = status_events(&ctx, dev).await;
    ctx.repairs
        .accept(
            req,
            v,
            RepairMoveChoiceDto::Move { place_id: hold },
            &admin(),
        )
        .await
        .expect("accept");
    let all = status_events(&ctx, dev).await;
    let new = &all[before.len()..];
    assert_eq!(new.len(), 1, "{all:?}");
    assert_eq!(new[0].1.as_deref(), Some("на_ремонте"));
    assert_eq!(new[0].2, "repair");
    assert_eq!(new[0].3, Some(req));
    let mv = movement_id_of(&ctx, dev, req).await;
    assert!(mv.is_some(), "перемещение записано");
    assert_eq!(new[0].4, mv, "movement_id события = перемещение заявки");
}

#[tokio::test]
async fn history_status_repair_return() {
    let (ctx, _d) = make_test_ctx().await;
    let room = seed_place(&ctx, "Кабинет 12").await;
    let hold = storage_place(&ctx, "Склад А").await;
    let back = seed_place(&ctx, "Кабинет 14").await;
    let dev = seed_device(&ctx, 1, "ПК-00014", "INV-14", Some(room), None).await;
    let (req, v) = new_request(&ctx, dev).await;
    let acc = ctx
        .repairs
        .accept(
            req,
            v,
            RepairMoveChoiceDto::Move { place_id: hold },
            &admin(),
        )
        .await
        .expect("accept");
    let n = status_events(&ctx, dev).await.len();
    ctx.repairs
        .complete(
            RepairCompleteDto {
                request_id: req,
                version: acc.request.version,
                place_id: Some(back),
                notes: None,
            },
            &admin(),
        )
        .await
        .expect("complete");
    let all = status_events(&ctx, dev).await;
    assert_eq!(all.len(), n + 1, "{all:?}");
    let last = all.last().unwrap();
    assert_eq!(last.0.as_deref(), Some("на_ремонте"));
    assert_eq!(last.1.as_deref(), Some("в_работе"));
    assert_eq!(last.2, "repair");
    assert_eq!(last.3, Some(req));
    assert!(last.4.is_some(), "место сменилось: есть movement_id");
}

#[tokio::test]
async fn history_status_repair_same_status() {
    // Принять без перемещения (в_работе -> на_ремонте) и отклонить (обратно): по одному
    // событию на реальную смену; без смены места movement_id пуст.
    let (ctx, _d) = make_test_ctx().await;
    let room = seed_place(&ctx, "Кабинет 12").await;
    let dev = seed_device(&ctx, 1, "ПК-00014", "INV-14", Some(room), None).await;
    let (req, v) = new_request(&ctx, dev).await;
    let acc = ctx
        .repairs
        .accept(req, v, RepairMoveChoiceDto::Stay, &admin())
        .await
        .expect("accept");
    let after_accept = status_events(&ctx, dev).await;
    assert_eq!(after_accept.len(), 1, "{after_accept:?}");
    assert_eq!(after_accept[0].1.as_deref(), Some("на_ремонте"));
    assert_eq!(
        after_accept[0].4, None,
        "место не менялось: movement_id нет"
    );
    ctx.repairs
        .reject(
            RepairRejectDto {
                request_id: req,
                version: acc.request.version,
                notes: Some("Дубликат".into()),
            },
            &admin(),
        )
        .await
        .expect("reject");
    let all = status_events(&ctx, dev).await;
    assert_eq!(all.len(), 2, "{all:?}");
    assert_eq!(all[1].0.as_deref(), Some("на_ремонте"));
    assert_eq!(all[1].1.as_deref(), Some("в_работе"));
    assert_eq!(all[1].3, Some(req));
    assert_eq!(all[1].4, None, "место не менялось: movement_id нет");
    // Статус уже равен целевому: повторного события нет.
    assert_eq!(status_code(&ctx, dev).await, "в_работе");
}

#[tokio::test]
async fn history_status_repair_anchor_group_no_duplicate() {
    let (ctx, _d) = make_test_ctx().await;
    let office = seed_place(&ctx, "Кабинет 12").await;
    let hold = storage_place(&ctx, "Склад А").await;
    let (_t, gid) = seed_group_type_and_group(&ctx, "АРМ-1", 1, Some(office)).await;
    let anchor = seed_device(&ctx, 1, "ПК-00041", "ПК-00041", Some(office), Some(gid)).await;
    let member = seed_device(&ctx, 1, "ПК-00042", "ПК-00042", Some(office), Some(gid)).await;
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "UPDATE groups SET anchor_device_id = ?1 WHERE id = ?2",
                params![anchor, gid],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("anchor");
    let (req, v) = new_request(&ctx, anchor).await;
    ctx.repairs
        .accept(
            req,
            v,
            RepairMoveChoiceDto::Move { place_id: hold },
            &admin(),
        )
        .await
        .expect("accept");
    for dev in [anchor, member] {
        let evs = status_events(&ctx, dev).await;
        // Цепочка непрерывна (каждое «from» = предыдущее «to»), дублей нет,
        // последнее «to» = актуальный статус.
        for w in evs.windows(2) {
            assert_eq!(w[1].0, w[0].1, "разрыв цепочки: {evs:?}");
        }
        for e in &evs {
            assert_ne!(e.0, e.1, "событие без смены: {evs:?}");
        }
        if let Some(last) = evs.last() {
            assert_eq!(
                last.1.as_deref(),
                Some(status_code(&ctx, dev).await.as_str())
            );
        }
    }
}

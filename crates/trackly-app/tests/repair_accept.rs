//! Phase 41.4, план 10: принятие ремонтной заявки и «Забрать на склад» (REP-02, REP-03).
//!
//! Настоящий `AppCtx::build`. Данные вымышленные («Кабинет 12», «Склад А», «ПК-00014»).

mod entities_support;
mod repair_support;

use entities_support::fixture::{
    admin, make_test_ctx, seed_device, seed_group_type_and_group, seed_place,
};
use repair_support::{emp, manager, seed_user};
use rusqlite::params;
use trackly_app::context::AppCtx;
use trackly_app::dto::repair::{RepairCreateDto, RepairMoveChoiceDto};
use trackly_core::error::AppError;
use trackly_infra::error_conversions::map_rusqlite;

async fn sql_i64(ctx: &AppCtx, sql: &'static str, arg: i64) -> Option<i64> {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(sql, params![arg], |r| r.get::<_, Option<i64>>(0))
                .map_err(map_rusqlite)
        })
        .await
        .expect("sql")
}

async fn exec(ctx: &AppCtx, sql: &'static str, a: i64, b: i64) {
    ctx.writer
        .execute(move |conn| {
            conn.execute(sql, params![a, b]).map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("exec");
}

async fn storage_place(ctx: &AppCtx, name: &str) -> i64 {
    let p = seed_place(ctx, name).await;
    exec(
        ctx,
        "UPDATE places SET is_storage = 1 WHERE id = ?1 AND ?2 = ?2",
        p,
        0,
    )
    .await;
    p
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
        .expect("status")
}

async fn set_status(ctx: &AppCtx, dev: i64, code: &'static str) {
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "UPDATE devices SET status_id = (SELECT id FROM device_statuses WHERE code = ?1) \
                 WHERE id = ?2",
                params![code, dev],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("set status");
}

async fn dev_place(ctx: &AppCtx, dev: i64) -> Option<i64> {
    sql_i64(ctx, "SELECT place_id FROM devices WHERE id = ?1", dev).await
}

/// Новая открытая заявка администратором: (id, version).
async fn new_request(ctx: &AppCtx, dev: Option<i64>) -> (i64, i64) {
    let requester = seed_user(ctx, "requester", "Сидоров С.С.").await;
    let r = ctx
        .repairs
        .create(
            RepairCreateDto {
                device_id: dev,
                description: "Не включается".into(),
            },
            &manager(requester),
        )
        .await
        .expect("create");
    (r.request.id, r.request.version)
}

fn mv(p: i64) -> RepairMoveChoiceDto {
    RepairMoveChoiceDto::Move { place_id: p }
}

async fn fields(ctx: &AppCtx, req: i64) -> (Option<i64>, i64) {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(
                "SELECT repair_origin_place_id, repair_moved FROM requests WHERE id = ?1",
                params![req],
                |r| Ok((r.get::<_, Option<i64>>(0)?, r.get::<_, i64>(1)?)),
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("fields")
}

#[tokio::test]
async fn stay_keeps_place_and_sets_awaiting_repair() {
    let (ctx, _d) = make_test_ctx().await;
    let room = seed_place(&ctx, "Кабинет 12").await;
    let dev = seed_device(&ctx, 1, "ПК-00014", "INV-14", Some(room), None).await;
    let (req, v) = new_request(&ctx, Some(dev)).await;

    let res = ctx
        .repairs
        .accept(req, v, RepairMoveChoiceDto::Stay, &admin())
        .await
        .expect("accept");
    assert_eq!(res.request.status, "in_progress");
    assert_eq!(dev_place(&ctx, dev).await, Some(room));
    assert_eq!(status_code(&ctx, dev).await, "на_ремонте");
    assert_eq!(fields(&ctx, req).await, (None, 0));
}

#[tokio::test]
async fn move_to_storage_records_origin_journal_and_last_place() {
    let (ctx, _d) = make_test_ctx().await;
    let room = seed_place(&ctx, "Кабинет 12").await;
    let store = storage_place(&ctx, "Склад А").await;
    let dev = seed_device(&ctx, 1, "ПК-00014", "INV-14", Some(room), None).await;
    let (req, v) = new_request(&ctx, Some(dev)).await;

    ctx.repairs
        .accept(req, v, mv(store), &admin())
        .await
        .expect("accept");
    assert_eq!(dev_place(&ctx, dev).await, Some(store));
    assert_eq!(status_code(&ctx, dev).await, "на_ремонте");
    assert_eq!(fields(&ctx, req).await, (Some(room), 1));
    let logged = sql_i64(
        &ctx,
        "SELECT COUNT(*) FROM place_movements WHERE entity_type = 'device' AND entity_id = ?1 \
         AND source = 'manual' AND note LIKE 'по заявке №%'",
        dev,
    )
    .await;
    assert_eq!(logged, Some(1));
    let last = ctx.repairs.last_places(&admin()).await.expect("last");
    assert_eq!(last.move_place_id, Some(store));
    assert_eq!(last.service_place_id, None, "ключ службы ремонта отдельный");
}

#[tokio::test]
async fn later_move_to_storage_keeps_origin() {
    let (ctx, _d) = make_test_ctx().await;
    let room = seed_place(&ctx, "Кабинет 12").await;
    let store_a = storage_place(&ctx, "Склад А").await;
    let store_b = storage_place(&ctx, "Склад Б").await;
    let dev = seed_device(&ctx, 1, "ПК-00014", "INV-14", Some(room), None).await;
    let (req, v) = new_request(&ctx, Some(dev)).await;

    let res = ctx
        .repairs
        .accept(req, v, mv(store_a), &admin())
        .await
        .expect("accept");
    ctx.repairs
        .move_to_storage(req, res.request.version, store_b, &admin())
        .await
        .expect("move_to_storage");
    assert_eq!(dev_place(&ctx, dev).await, Some(store_b));
    assert_eq!(
        status_code(&ctx, dev).await,
        "на_ремонте",
        "статус не меняется"
    );
    assert_eq!(
        fields(&ctx, req).await,
        (Some(room), 1),
        "origin не перезаписан"
    );
    let last = ctx.repairs.last_places(&admin()).await.expect("last");
    assert_eq!(last.move_place_id, Some(store_b));
}

#[tokio::test]
async fn move_to_storage_after_stay_sets_origin_and_needs_in_progress() {
    let (ctx, _d) = make_test_ctx().await;
    let room = seed_place(&ctx, "Кабинет 12").await;
    let store = storage_place(&ctx, "Склад А").await;
    let dev = seed_device(&ctx, 1, "ПК-00014", "INV-14", Some(room), None).await;
    let (req, v) = new_request(&ctx, Some(dev)).await;

    // Не принята: перемещение отклонено.
    let err = ctx
        .repairs
        .move_to_storage(req, v, store, &admin())
        .await
        .expect_err("open");
    assert!(matches!(err, AppError::Validation { .. }), "{err:?}");

    let res = ctx
        .repairs
        .accept(req, v, RepairMoveChoiceDto::Stay, &admin())
        .await
        .expect("accept");
    ctx.repairs
        .move_to_storage(req, res.request.version, store, &admin())
        .await
        .expect("move_to_storage");
    assert_eq!(fields(&ctx, req).await, (Some(room), 1));
}

#[tokio::test]
async fn write_off_device_is_not_blocked() {
    let (ctx, _d) = make_test_ctx().await;
    let room = seed_place(&ctx, "Кабинет 12").await;
    let dev = seed_device(&ctx, 1, "ПК-00014", "INV-14", Some(room), None).await;
    set_status(&ctx, dev, "на_списание").await;
    let (req, v) = new_request(&ctx, Some(dev)).await;

    ctx.repairs
        .accept(req, v, RepairMoveChoiceDto::Stay, &admin())
        .await
        .expect("accept");
    assert_eq!(status_code(&ctx, dev).await, "на_ремонте");
}

#[tokio::test]
async fn other_without_device_is_accepted_without_movement() {
    let (ctx, _d) = make_test_ctx().await;
    let (req, v) = new_request(&ctx, None).await;
    let res = ctx
        .repairs
        .accept(req, v, RepairMoveChoiceDto::Stay, &admin())
        .await
        .expect("accept");
    assert_eq!(res.request.status, "in_progress");
    assert_eq!(res.request.printer_device_id, None);
    let n = sql_i64(&ctx, "SELECT COUNT(*) + ?1 FROM place_movements", 0).await;
    assert_eq!(n, Some(0));
}

#[tokio::test]
async fn anchor_moves_block_and_only_anchor_changes_status() {
    let (ctx, _d) = make_test_ctx().await;
    let room = seed_place(&ctx, "Кабинет 12").await;
    let store = storage_place(&ctx, "Склад А").await;
    let (_t, group) = seed_group_type_and_group(&ctx, "Комплект ПК-00014", 1, Some(room)).await;
    let anchor = seed_device(&ctx, 1, "ПК-00014", "INV-14", Some(room), Some(group)).await;
    let part1 = seed_device(&ctx, 1, "Монитор", "INV-15", Some(room), Some(group)).await;
    let part2 = seed_device(&ctx, 1, "Клавиатура", "INV-16", Some(room), Some(group)).await;
    exec(
        &ctx,
        "UPDATE groups SET anchor_device_id = ?1 WHERE id = ?2",
        anchor,
        group,
    )
    .await;
    let (req, v) = new_request(&ctx, Some(anchor)).await;

    ctx.repairs
        .accept(req, v, mv(store), &admin())
        .await
        .expect("accept");

    assert_eq!(dev_place(&ctx, anchor).await, Some(store));
    assert_eq!(dev_place(&ctx, part1).await, Some(store));
    assert_eq!(dev_place(&ctx, part2).await, Some(store));
    assert_eq!(
        sql_i64(&ctx, "SELECT place_id FROM groups WHERE id = ?1", group).await,
        Some(store)
    );
    assert_eq!(status_code(&ctx, anchor).await, "на_ремонте");
    // Отрицательная проверка: статусы деталей не изменились.
    assert_eq!(
        status_code(&ctx, part1).await,
        status_code_of_fresh(&ctx).await
    );
    assert_ne!(status_code(&ctx, part1).await, "на_ремонте");
    assert_ne!(status_code(&ctx, part2).await, "на_ремонте");
    assert_eq!(fields(&ctx, req).await, (Some(room), 1));
}

/// Статус, с которым сидер создаёт устройства (`status_id = 1`).
async fn status_code_of_fresh(ctx: &AppCtx) -> String {
    ctx.writer
        .execute(|conn| {
            conn.query_row("SELECT code FROM device_statuses WHERE id = 1", [], |r| {
                r.get::<_, String>(0)
            })
            .map_err(map_rusqlite)
        })
        .await
        .expect("fresh status")
}

/// Корневая «АРМ-1» (Кабинет 12) с членом-монитором и вложенная «Системный блок» с
/// якорем и деталью. Возвращает (room, store, root, nested, root_member, anchor, part).
async fn nested_fixture(ctx: &AppCtx) -> (i64, i64, i64, i64, i64, i64, i64) {
    let room = seed_place(ctx, "Кабинет 12").await;
    let store = storage_place(ctx, "Склад А").await;
    let (t, root) = seed_group_type_and_group(ctx, "АРМ-1", 1, Some(room)).await;
    let root_member = seed_device(ctx, 1, "Монитор", "INV-10", Some(room), Some(root)).await;
    let nested = ctx
        .writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO groups (type_id, name, seq, place_id, parent_group_id, \
                 created_at_utc, updated_at_utc, version) \
                 VALUES (?1, 'Системный блок', 2, ?2, ?3, 1700000000, 1700000000, 1)",
                params![t, room, root],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("nested");
    let anchor = seed_device(ctx, 1, "ПК-00014", "INV-14", Some(room), Some(nested)).await;
    let part = seed_device(ctx, 1, "Блок питания", "INV-15", Some(room), Some(nested)).await;
    exec(
        ctx,
        "UPDATE groups SET anchor_device_id = ?1 WHERE id = ?2",
        anchor,
        nested,
    )
    .await;
    (room, store, root, nested, root_member, anchor, part)
}

#[tokio::test]
async fn nested_group_anchor_moves_its_block_stay_passes() {
    let (ctx, _d) = make_test_ctx().await;
    let (room, store, root, nested, root_member, anchor, part) = nested_fixture(&ctx).await;
    let (req, v) = new_request(&ctx, Some(anchor)).await;

    ctx.repairs
        .accept(req, v, mv(store), &admin())
        .await
        .expect("перемещение якоря вложенной группы");

    assert_eq!(dev_place(&ctx, anchor).await, Some(store));
    assert_eq!(dev_place(&ctx, part).await, Some(store));
    assert_eq!(
        sql_i64(&ctx, "SELECT place_id FROM groups WHERE id = ?1", nested).await,
        Some(store)
    );
    // Корень и его прочие члены не тронуты (D-18).
    assert_eq!(
        sql_i64(&ctx, "SELECT place_id FROM groups WHERE id = ?1", root).await,
        Some(room)
    );
    assert_eq!(dev_place(&ctx, root_member).await, Some(room));
    assert_eq!(status_code(&ctx, anchor).await, "на_ремонте");
    assert_ne!(status_code(&ctx, part).await, "на_ремонте");
    assert_eq!(fields(&ctx, req).await, (Some(room), 1));
}

#[tokio::test]
async fn nested_group_anchor_stay_still_passes() {
    let (ctx, _d) = make_test_ctx().await;
    let (room, _store, _root, _nested, _m, anchor, _part) = nested_fixture(&ctx).await;
    let (req, v) = new_request(&ctx, Some(anchor)).await;
    ctx.repairs
        .accept(req, v, RepairMoveChoiceDto::Stay, &admin())
        .await
        .expect("stay");
    assert_eq!(status_code(&ctx, anchor).await, "на_ремонте");
    assert_eq!(dev_place(&ctx, anchor).await, Some(room));
}

#[tokio::test]
async fn employee_forbidden_and_stale_version_conflicts() {
    let (ctx, _d) = make_test_ctx().await;
    let room = seed_place(&ctx, "Кабинет 12").await;
    let dev = seed_device(&ctx, 1, "ПК-00014", "INV-14", Some(room), None).await;
    let ivanov = seed_user(&ctx, "ivanov", "Иванов И.И.").await;
    let (req, v) = new_request(&ctx, Some(dev)).await;

    let err = ctx
        .repairs
        .accept(req, v, RepairMoveChoiceDto::Stay, &emp(ivanov))
        .await
        .expect_err("forbidden");
    assert!(matches!(err, AppError::Forbidden), "{err:?}");

    let err = ctx
        .repairs
        .accept(req, v + 5, RepairMoveChoiceDto::Stay, &admin())
        .await
        .expect_err("stale");
    assert!(
        matches!(err, AppError::OptimisticLockMismatch { .. }),
        "{err:?}"
    );
    assert_eq!(
        status_code(&ctx, dev).await,
        status_code_of_fresh(&ctx).await
    );
}

#[tokio::test]
async fn archived_or_deleted_place_rejected_with_rollback() {
    let (ctx, _d) = make_test_ctx().await;
    let room = seed_place(&ctx, "Кабинет 12").await;
    let archived = storage_place(&ctx, "Склад архивный").await;
    let deleted = storage_place(&ctx, "Склад удалённый").await;
    exec(
        &ctx,
        "UPDATE places SET archived_at_utc = 1700000001 WHERE id = ?1 AND ?2 = ?2",
        archived,
        0,
    )
    .await;
    exec(
        &ctx,
        "UPDATE places SET deleted_at_utc = 1700000001 WHERE id = ?1 AND ?2 = ?2",
        deleted,
        0,
    )
    .await;
    let dev = seed_device(&ctx, 1, "ПК-00014", "INV-14", Some(room), None).await;
    let (req, v) = new_request(&ctx, Some(dev)).await;

    for bad in [archived, deleted] {
        let err = ctx
            .repairs
            .accept(req, v, mv(bad), &admin())
            .await
            .expect_err("bad place");
        assert!(matches!(err, AppError::Validation { .. }), "{err:?}");
        assert_eq!(dev_place(&ctx, dev).await, Some(room));
        assert_eq!(
            status_code(&ctx, dev).await,
            status_code_of_fresh(&ctx).await
        );
    }
    // Заявка осталась открытой: повторное принятие с той же версией проходит.
    ctx.repairs
        .accept(req, v, RepairMoveChoiceDto::Stay, &admin())
        .await
        .expect("accept after rollback");
}

#[tokio::test]
async fn accept_on_non_open_request_rejected() {
    let (ctx, _d) = make_test_ctx().await;
    let room = seed_place(&ctx, "Кабинет 12").await;
    let dev = seed_device(&ctx, 1, "ПК-00014", "INV-14", Some(room), None).await;
    let (req, v) = new_request(&ctx, Some(dev)).await;
    let res = ctx
        .repairs
        .accept(req, v, RepairMoveChoiceDto::Stay, &admin())
        .await
        .expect("accept");
    let err = ctx
        .repairs
        .accept(
            req,
            res.request.version,
            RepairMoveChoiceDto::Stay,
            &admin(),
        )
        .await
        .expect_err("second accept");
    assert!(matches!(err, AppError::Validation { .. }), "{err:?}");
}

async fn seed_attached_cartridge(
    ctx: &AppCtx,
    code: &'static str,
    printer: i64,
    place: i64,
) -> i64 {
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT OR IGNORE INTO cartridge_models \
                 (id, brand, model, kind_id, created_at_utc, updated_at_utc, version) \
                 VALUES (900, 'HP', 'CE285A', 1, 1700000000, 1700000000, 1)",
                [],
            )
            .map_err(map_rusqlite)?;
            conn.execute(
                "INSERT INTO cartridges (model_id, code, status_id, state_id, place_id, \
                 current_printer_device_id, created_at_utc, updated_at_utc, version) \
                 VALUES (900, ?1, 2, 1, ?2, ?3, 1700000000, 1700000000, 1)",
                params![code, place, printer],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("cartridge")
}

#[tokio::test]
async fn printer_move_cascades_attached_cartridges() {
    let (ctx, _d) = make_test_ctx().await;
    let room_a = seed_place(&ctx, "Кабинет А").await;
    let store_b = storage_place(&ctx, "Склад Б").await;
    let printer = seed_device(&ctx, 2, "Принтер-7", "INV-77", Some(room_a), None).await;
    let c1 = seed_attached_cartridge(&ctx, "C-0001", printer, room_a).await;
    let c2 = seed_attached_cartridge(&ctx, "C-0002", printer, room_a).await;

    // Stay: картриджи остаются на месте А.
    let (req, v) = new_request(&ctx, Some(printer)).await;
    let mut rx = ctx.ws_broadcast.subscribe();
    let res = ctx
        .repairs
        .accept(req, v, RepairMoveChoiceDto::Stay, &admin())
        .await
        .expect("accept stay");
    for c in [c1, c2] {
        assert_eq!(
            sql_i64(&ctx, "SELECT place_id FROM cartridges WHERE id = ?1", c).await,
            Some(room_a)
        );
    }

    // Move на склад Б: оба картриджа едут, журнал «вместе с принтером», событие несёт А и Б.
    entities_support::fixture::drain(&mut rx);
    ctx.repairs
        .move_to_storage(req, res.request.version, store_b, &admin())
        .await
        .expect("move");
    for c in [c1, c2] {
        assert_eq!(
            sql_i64(&ctx, "SELECT place_id FROM cartridges WHERE id = ?1", c).await,
            Some(store_b)
        );
        let logged = sql_i64(
            &ctx,
            "SELECT COUNT(*) FROM place_movements WHERE entity_type = 'cartridge' \
             AND entity_id = ?1 AND note = 'вместе с принтером' AND source = 'manual'",
            c,
        )
        .await;
        assert_eq!(logged, Some(1));
    }
    let (places, devices, _g) =
        entities_support::fixture::assert_one_entities(&mut rx, "move_to_storage");
    assert!(
        places.contains(&room_a) && places.contains(&store_b),
        "{places:?}"
    );
    assert!(devices.contains(&printer));
}

#[tokio::test]
async fn printer_without_place_does_not_wipe_cartridge_places() {
    let (ctx, _d) = make_test_ctx().await;
    let room_a = seed_place(&ctx, "Кабинет А").await;
    let printer = seed_device(&ctx, 2, "Принтер-7", "INV-77", None, None).await;
    let c1 = seed_attached_cartridge(&ctx, "C-0001", printer, room_a).await;
    let (req, v) = new_request(&ctx, Some(printer)).await;
    ctx.repairs
        .accept(req, v, RepairMoveChoiceDto::Stay, &admin())
        .await
        .expect("accept");
    assert_eq!(
        sql_i64(&ctx, "SELECT place_id FROM cartridges WHERE id = ?1", c1).await,
        Some(room_a)
    );
}

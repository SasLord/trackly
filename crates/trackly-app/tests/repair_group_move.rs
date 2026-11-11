//! Phase 41.4 план 05: перенос группы (D-15), массовый перенос мест (D-14), запрет
//! перемещения при открытой ремонтной заявке (D-21) и акты (D-22).
//!
//! Настоящий `AppCtx::build`. Все фикстуры РАСХОДЯТСЯ: место устройства («Кабинет 12»)
//! не равно месту группы («Склад А»), где это важно. Данные вымышленные.

mod entities_support;

use entities_support::fixture::{
    admin, make_test_ctx, seed_device, seed_group_type_and_group, seed_place,
};
use rusqlite::params;
use trackly_app::context::AppCtx;
use trackly_core::error::AppError;
use trackly_infra::error_conversions::map_rusqlite;

const CONFLICT_UPDATE: &str = "Нельзя изменить устройство: по нему открыта заявка на ремонт.";

async fn set_status_raw(ctx: &AppCtx, device: i64, code: &'static str) {
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "UPDATE devices SET status_id = (SELECT id FROM device_statuses WHERE code = ?1) \
                 WHERE id = ?2",
                params![code, device],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("set status");
}

async fn place_of(ctx: &AppCtx, id: i64) -> Option<i64> {
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
        .expect("place")
}

async fn movements(ctx: &AppCtx, id: i64) -> i64 {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(
                "SELECT COUNT(*) FROM place_movements WHERE entity_type = 'device' AND entity_id = ?1",
                params![id],
                |r| r.get::<_, i64>(0),
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("movements")
}

async fn open_request(ctx: &AppCtx, device: i64, status: &'static str) {
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT OR IGNORE INTO users (login, full_name, password_hash, role, ad_user, \
                 is_active, created_at_utc, updated_at_utc, version) \
                 VALUES ('petrov.pp', 'Петров П.П.', NULL, 'employee', 0, 1, 1700000000, 1700000000, 1)",
                [],
            )
            .map_err(map_rusqlite)?;
            conn.execute(
                "INSERT INTO requests (request_type, status, requested_by_user_id, printer_device_id, \
                 created_at_utc, updated_at_utc) \
                 VALUES ('repair', ?1, (SELECT MIN(id) FROM users), ?2, 1700000000, 1700000000)",
                params![status, device],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("insert repair request");
}

async fn group_version(ctx: &AppCtx, id: i64) -> i64 {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(
                "SELECT version FROM groups WHERE id = ?1",
                params![id],
                |r| r.get::<_, i64>(0),
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("group version")
}

struct Fx {
    old_group_place: i64, // «Склад А»
    elsewhere: i64,       // «Кабинет 12»
    target: i64,          // «Склад Б»
    group: i64,
    diverged: i64, // член в исключённом статусе, место расходится с местом группы
    same: i64,     // член в исключённом статусе, место = месту группы
    regular: i64,  // обычный член на месте группы
}

async fn fixture(ctx: &AppCtx) -> Fx {
    let old_group_place = seed_place(ctx, "Склад А").await;
    let elsewhere = seed_place(ctx, "Кабинет 12").await;
    let target = seed_place(ctx, "Склад Б").await;
    let (_t, group) = seed_group_type_and_group(ctx, "АРМ-1", 1, Some(old_group_place)).await;
    let diverged = seed_device(ctx, 1, "ПК-00031", "ПК-00031", Some(elsewhere), Some(group)).await;
    let same = seed_device(
        ctx,
        1,
        "ПК-00032",
        "ПК-00032",
        Some(old_group_place),
        Some(group),
    )
    .await;
    let regular = seed_device(
        ctx,
        1,
        "ПК-00033",
        "ПК-00033",
        Some(old_group_place),
        Some(group),
    )
    .await;
    set_status_raw(ctx, diverged, "в_ремонте").await;
    set_status_raw(ctx, same, "на_ремонте").await;
    Fx {
        old_group_place,
        elsewhere,
        target,
        group,
        diverged,
        same,
        regular,
    }
}

async fn move_group(ctx: &AppCtx, fx: &Fx) {
    let version = group_version(ctx, fx.group).await;
    ctx.groups
        .move_group(&admin(), fx.group, version, fx.target)
        .await
        .expect("move group");
}

// (1) D-15: исключённый член с разошедшимся местом остаётся, журнал не пишется.
#[tokio::test]
async fn move_group_diverged_exempt_member_stays() {
    let (ctx, _d) = make_test_ctx().await;
    let fx = fixture(&ctx).await;
    let before = movements(&ctx, fx.diverged).await;
    move_group(&ctx, &fx).await;
    assert_eq!(place_of(&ctx, fx.diverged).await, Some(fx.elsewhere));
    assert_eq!(movements(&ctx, fx.diverged).await, before);
    assert_eq!(place_of(&ctx, fx.regular).await, Some(fx.target));
    let _ = fx.old_group_place;
}

// (2) D-15: исключённый член с местом группы едет с ней.
#[tokio::test]
async fn move_group_exempt_member_on_group_place_rides() {
    let (ctx, _d) = make_test_ctx().await;
    let fx = fixture(&ctx).await;
    move_group(&ctx, &fx).await;
    assert_eq!(place_of(&ctx, fx.same).await, Some(fx.target));
    assert_eq!(place_of(&ctx, fx.regular).await, Some(fx.target));
}

// (3) D-14: массовый перенос — исключённое устройство с разошедшимся местом обработано
// как обычное устройство.
#[tokio::test]
async fn bulk_move_diverged_exempt_device_handled_as_ordinary() {
    let (ctx, _d) = make_test_ctx().await;
    let fx = fixture(&ctx).await;
    ctx.places
        .move_subtree_contents(&admin(), fx.elsewhere, fx.target, None)
        .await
        .expect("bulk move");
    assert_eq!(place_of(&ctx, fx.diverged).await, Some(fx.target));
}

// (4) D-21: перемещение через «Места» при открытой заявке — Conflict; закрытая не мешает.
#[tokio::test]
async fn bulk_move_blocked_by_open_repair_request() {
    let (ctx, _d) = make_test_ctx().await;
    let fx = fixture(&ctx).await;
    open_request(&ctx, fx.diverged, "open").await;
    let err = ctx
        .places
        .move_subtree_contents(&admin(), fx.elsewhere, fx.target, None)
        .await
        .expect_err("conflict");
    match err {
        AppError::Conflict { reason } => assert_eq!(reason, CONFLICT_UPDATE),
        other => panic!("ожидался Conflict, получено {other:?}"),
    }
    assert_eq!(place_of(&ctx, fx.diverged).await, Some(fx.elsewhere));
}

#[tokio::test]
async fn bulk_move_not_blocked_by_completed_repair_request() {
    let (ctx, _d) = make_test_ctx().await;
    let fx = fixture(&ctx).await;
    open_request(&ctx, fx.diverged, "completed").await;
    ctx.places
        .move_subtree_contents(&admin(), fx.elsewhere, fx.target, None)
        .await
        .expect("bulk move");
    assert_eq!(place_of(&ctx, fx.diverged).await, Some(fx.target));
}

async fn group_place_of(ctx: &AppCtx, id: i64) -> Option<i64> {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(
                "SELECT place_id FROM groups WHERE id = ?1",
                params![id],
                |r| r.get::<_, Option<i64>>(0),
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("group place")
}

/// Вложенная группа под `parent` (прямая вставка), с местом `place`.
async fn seed_nested_group(
    ctx: &AppCtx,
    parent: i64,
    name: &str,
    seq: i64,
    place: Option<i64>,
) -> i64 {
    let name = name.to_string();
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO groups (type_id, name, seq, place_id, parent_group_id, \
                 created_at_utc, updated_at_utc, version) \
                 VALUES ((SELECT type_id FROM groups WHERE id = ?1), ?2, ?3, ?4, ?1, \
                 1700000000, 1700000000, 1)",
                params![parent, name, seq, place],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed nested group")
}

// (5) D-15: вложенный блок «в отъезде» (место отличается от места корня) НЕ едет при
// переносе корня; прочие члены корня и синхронная вложенная группа едут как раньше.
#[tokio::test]
async fn move_root_leaves_away_nested_block_in_place() {
    let (ctx, _d) = make_test_ctx().await;
    let office = seed_place(&ctx, "Кабинет 12").await;
    let store = seed_place(&ctx, "Склад А").await;
    let target = seed_place(&ctx, "Склад Б").await;
    let (_t, root) = seed_group_type_and_group(&ctx, "АРМ-1", 1, Some(office)).await;
    let root_member = seed_device(&ctx, 1, "ПК-00041", "ПК-00041", Some(office), Some(root)).await;
    let synced = seed_nested_group(&ctx, root, "АРМ-1.1", 2, Some(office)).await;
    let synced_dev = seed_device(&ctx, 1, "ПК-00042", "ПК-00042", Some(office), Some(synced)).await;
    let away = seed_nested_group(&ctx, root, "АРМ-1.2", 3, Some(store)).await;
    let away_anchor = seed_device(&ctx, 1, "ПК-00043", "ПК-00043", Some(store), Some(away)).await;
    let away_part = seed_device(&ctx, 1, "ПК-00044", "ПК-00044", Some(store), Some(away)).await;
    let before_anchor = movements(&ctx, away_anchor).await;
    let before_part = movements(&ctx, away_part).await;

    let version = group_version(&ctx, root).await;
    ctx.groups
        .move_group(&admin(), root, version, target)
        .await
        .expect("move root");

    assert_eq!(group_place_of(&ctx, root).await, Some(target));
    assert_eq!(group_place_of(&ctx, synced).await, Some(target));
    assert_eq!(place_of(&ctx, root_member).await, Some(target));
    assert_eq!(place_of(&ctx, synced_dev).await, Some(target));
    assert_eq!(
        group_place_of(&ctx, away).await,
        Some(store),
        "блок в отъезде"
    );
    assert_eq!(place_of(&ctx, away_anchor).await, Some(store));
    assert_eq!(place_of(&ctx, away_part).await, Some(store));
    assert_eq!(movements(&ctx, away_anchor).await, before_anchor);
    assert_eq!(movements(&ctx, away_part).await, before_part);
}

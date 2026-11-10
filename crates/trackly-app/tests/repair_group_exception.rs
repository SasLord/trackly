//! Phase 41.4 план 04: исключение по статусу для устройств в группе (D-14, D-17) и запрет
//! ручной смены статуса и места при открытой ремонтной заявке (D-21).
//!
//! Настоящий `AppCtx::build`. Все фикстуры РАСХОДЯТСЯ: место устройства («Кабинет 12»)
//! не равно месту группы («Склад А»), иначе тест не отличил бы поведение до и после
//! правки. Данные вымышленные.

mod entities_support;

use entities_support::fixture::{
    admin, make_test_ctx, seed_device, seed_group_type_and_group, seed_place,
};
use rusqlite::params;
use trackly_app::context::AppCtx;
use trackly_app::dto::device::DevicePatch;
use trackly_core::error::AppError;
use trackly_infra::error_conversions::map_rusqlite;

const CONFLICT_UPDATE: &str = "Нельзя изменить устройство: по нему открыта заявка на ремонт.";
const CONFLICT_DELETE: &str = "Нельзя удалить устройство: по нему открыта заявка на ремонт.";

struct Fx {
    place_group: i64,  // «Склад А»
    place_device: i64, // «Кабинет 12»
    group: i64,
    device: i64, // «ПК-00014» — член группы, место РАСХОДИТСЯ с местом группы
}

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
        .expect("status id")
}

async fn set_status_raw(ctx: &AppCtx, device: i64, code: &'static str) {
    let sid = status_id(ctx, code).await;
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "UPDATE devices SET status_id = ?1 WHERE id = ?2",
                params![sid, device],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("set status");
}

async fn fixture(ctx: &AppCtx, status: &'static str) -> Fx {
    let place_group = seed_place(ctx, "Склад А").await;
    let place_device = seed_place(ctx, "Кабинет 12").await;
    let (_t, group) = seed_group_type_and_group(ctx, "АРМ-1", 1, Some(place_group)).await;
    let device = seed_device(
        ctx,
        1,
        "ПК-00014",
        "ПК-00014",
        Some(place_device),
        Some(group),
    )
    .await;
    set_status_raw(ctx, device, status).await;
    Fx {
        place_group,
        place_device,
        group,
        device,
    }
}

async fn dev_row(ctx: &AppCtx, id: i64) -> (i64, Option<i64>, i64, String) {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(
                "SELECT version, place_id, status_id, name FROM devices WHERE id = ?1",
                params![id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("dev row")
}

async fn count(ctx: &AppCtx, sql: &'static str, id: i64) -> i64 {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(sql, params![id], |r| r.get::<_, i64>(0))
                .map_err(map_rusqlite)
        })
        .await
        .expect("count")
}

async fn is_member(ctx: &AppCtx, device: i64) -> bool {
    count(
        ctx,
        "SELECT COUNT(*) FROM group_devices WHERE device_id = ?1",
        device,
    )
    .await
        == 1
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

async fn update(ctx: &AppCtx, id: i64, patch: DevicePatch) -> Result<(), AppError> {
    let (version, ..) = dev_row(ctx, id).await;
    ctx.devices
        .update(&admin(), id, version, patch)
        .await
        .map(|_| ())
}

fn place_patch(p: i64) -> DevicePatch {
    DevicePatch {
        place_id: Some(Some(p)),
        ..Default::default()
    }
}

fn status_patch(s: i64) -> DevicePatch {
    DevicePatch {
        status_id: Some(s),
        ..Default::default()
    }
}

// (1) регрессия: обычный статус, смена места члена группы отклонена.
#[tokio::test]
async fn exception_regular_member_move_rejected() {
    let (ctx, _d) = make_test_ctx().await;
    let fx = fixture(&ctx, "в_работе").await;
    let other = seed_place(&ctx, "Кабинет 13").await;
    // Место устройства != место группы: до правки и после ответ один и тот же.
    let err = update(&ctx, fx.device, place_patch(other))
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::Validation { ref field, .. } if field == "place_id"));
    assert!(is_member(&ctx, fx.device).await);
}

// (2) исключённый статус: место можно менять без заявки, членство остаётся.
#[tokio::test]
async fn exception_exempt_member_move_allowed_without_request() {
    let (ctx, _d) = make_test_ctx().await;
    let fx = fixture(&ctx, "на_ремонте").await;
    let other = seed_place(&ctx, "Кабинет 13").await;
    assert_ne!(fx.place_device, fx.place_group);
    update(&ctx, fx.device, place_patch(other))
        .await
        .expect("исключённый статус место менять может");
    assert_eq!(dev_row(&ctx, fx.device).await.1, Some(other));
    assert!(is_member(&ctx, fx.device).await, "членство сохранено");
    assert_eq!(
        count(
            &ctx,
            "SELECT COUNT(*) FROM group_devices WHERE group_id = ?1",
            fx.group
        )
        .await,
        1,
        "состав группы не тронут"
    );
}

// (2б) исключение по эффективному статусу из патча: статус и место за один раз.
#[tokio::test]
async fn exception_patch_status_exempt_and_place_together() {
    let (ctx, _d) = make_test_ctx().await;
    let fx = fixture(&ctx, "в_работе").await;
    let other = seed_place(&ctx, "Кабинет 13").await;
    let sid = status_id(&ctx, "в_ремонте").await;
    update(
        &ctx,
        fx.device,
        DevicePatch {
            status_id: Some(sid),
            place_id: Some(Some(other)),
            ..Default::default()
        },
    )
    .await
    .expect("эффективный статус из патча — исключённый");
    assert_eq!(dev_row(&ctx, fx.device).await.1, Some(other));
}

// (3) D-17: исключённый -> обычный при расходящемся месте переносит на место группы.
#[tokio::test]
async fn exception_status_back_to_regular_moves_to_group_place() {
    let (ctx, _d) = make_test_ctx().await;
    let fx = fixture(&ctx, "на_ремонте").await;
    // место устройства («Кабинет 12») != место группы («Склад А»)
    let moves_before = count(
        &ctx,
        "SELECT COUNT(*) FROM place_movements WHERE entity_type = 'device' AND entity_id = ?1",
        fx.device,
    )
    .await;
    let audit_before = count(
        &ctx,
        "SELECT COUNT(*) FROM audit_log WHERE entity_type = 'device' AND entity_id = ?1",
        fx.device,
    )
    .await;
    let sid = status_id(&ctx, "в_работе").await;
    update(&ctx, fx.device, status_patch(sid))
        .await
        .expect("смена статуса");
    let (_, place, status, _) = dev_row(&ctx, fx.device).await;
    assert_eq!(status, sid);
    assert_eq!(place, Some(fx.place_group), "перенос на место группы");
    assert_eq!(
        count(
            &ctx,
            "SELECT COUNT(*) FROM place_movements WHERE entity_type = 'device' AND entity_id = ?1",
            fx.device
        )
        .await,
        moves_before + 1,
        "новая запись журнала перемещений"
    );
    assert!(
        count(
            &ctx,
            "SELECT COUNT(*) FROM audit_log WHERE entity_type = 'device' AND entity_id = ?1",
            fx.device
        )
        .await
            > audit_before,
        "аудит пишется"
    );
    assert!(is_member(&ctx, fx.device).await);
}

// (3б) при совпадающем месте записей журнала нет.
#[tokio::test]
async fn exception_status_back_to_regular_same_place_no_movement() {
    let (ctx, _d) = make_test_ctx().await;
    let fx = fixture(&ctx, "на_ремонте").await;
    // Выравниваем место устройства с местом группы.
    let pg = fx.place_group;
    let dev = fx.device;
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "UPDATE devices SET place_id = ?1 WHERE id = ?2",
                params![pg, dev],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .unwrap();
    let sid = status_id(&ctx, "в_работе").await;
    update(&ctx, fx.device, status_patch(sid)).await.unwrap();
    assert_eq!(
        count(
            &ctx,
            "SELECT COUNT(*) FROM place_movements WHERE entity_type = 'device' AND entity_id = ?1",
            fx.device
        )
        .await,
        0
    );
    assert_eq!(dev_row(&ctx, fx.device).await.1, Some(fx.place_group));
}

// (4) открытая заявка: смена статуса / места — Conflict; имя проходит.
#[tokio::test]
async fn lock_open_request_blocks_status_and_place_change() {
    let (ctx, _d) = make_test_ctx().await;
    let fx = fixture(&ctx, "на_ремонте").await;
    open_request(&ctx, fx.device, "open").await;
    let other = seed_place(&ctx, "Кабинет 13").await;
    let sid = status_id(&ctx, "в_работе").await;

    let err = update(&ctx, fx.device, status_patch(sid))
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::Conflict { ref reason } if reason == CONFLICT_UPDATE));
    let err = update(&ctx, fx.device, place_patch(other))
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::Conflict { ref reason } if reason == CONFLICT_UPDATE));

    update(
        &ctx,
        fx.device,
        DevicePatch {
            name: Some("ПК-00014 новый".into()),
            ..Default::default()
        },
    )
    .await
    .expect("правка имени проходит");
    assert_eq!(dev_row(&ctx, fx.device).await.3, "ПК-00014 новый");
}

// (4б) форма-подобный патч: status_id и place_id прежние, меняется только имя.
#[tokio::test]
async fn lock_form_like_patch_with_unchanged_values_passes() {
    let (ctx, _d) = make_test_ctx().await;
    let fx = fixture(&ctx, "на_ремонте").await;
    open_request(&ctx, fx.device, "in_progress").await;
    let (_, place, status, _) = dev_row(&ctx, fx.device).await;
    update(
        &ctx,
        fx.device,
        DevicePatch {
            name: Some("ПК-00014 правка".into()),
            status_id: Some(status),
            place_id: Some(place),
            ..Default::default()
        },
    )
    .await
    .expect("форма шлёт прежние значения");
    let (_, place2, status2, name2) = dev_row(&ctx, fx.device).await;
    assert_eq!(name2, "ПК-00014 правка");
    assert_eq!((place2, status2), (place, status));
}

// (4в) завершённая заявка не блокирует.
#[tokio::test]
async fn lock_completed_request_does_not_block() {
    let (ctx, _d) = make_test_ctx().await;
    let fx = fixture(&ctx, "на_ремонте").await;
    open_request(&ctx, fx.device, "completed").await;
    let sid = status_id(&ctx, "в_работе").await;
    update(&ctx, fx.device, status_patch(sid))
        .await
        .expect("completed не блокирует");
}

// (4г) без заявки ручная смена статуса остаётся (41.1 D-04).
#[tokio::test]
async fn lock_without_request_manual_status_change_works() {
    let (ctx, _d) = make_test_ctx().await;
    let fx = fixture(&ctx, "в_работе").await;
    let sid = status_id(&ctx, "на_ремонте").await;
    update(&ctx, fx.device, status_patch(sid)).await.unwrap();
    assert_eq!(dev_row(&ctx, fx.device).await.2, sid);
}

// (5) delete_soft при открытой заявке.
#[tokio::test]
async fn lock_delete_soft_blocked_by_open_request() {
    let (ctx, _d) = make_test_ctx().await;
    let place = seed_place(&ctx, "Кабинет 12").await;
    let device = seed_device(&ctx, 1, "ПК-00015", "ПК-00015", Some(place), None).await;
    open_request(&ctx, device, "in_repair").await;
    let (version, ..) = dev_row(&ctx, device).await;
    let err = ctx.devices.delete_soft(device, version).await.unwrap_err();
    assert!(matches!(err, AppError::Conflict { ref reason } if reason == CONFLICT_DELETE));
}

// (6) вступление в группу устройства «в ремонте»: место не меняется.
#[tokio::test]
async fn exception_join_group_keeps_exempt_device_place() {
    let (ctx, _d) = make_test_ctx().await;
    let place_group = seed_place(&ctx, "Склад А").await;
    let place_device = seed_place(&ctx, "Кабинет 12").await;
    let (_t, group) = seed_group_type_and_group(&ctx, "АРМ-2", 2, Some(place_group)).await;
    let exempt = seed_device(&ctx, 1, "ПК-00016", "ПК-00016", Some(place_device), None).await;
    let regular = seed_device(&ctx, 1, "ПК-00017", "ПК-00017", Some(place_device), None).await;
    set_status_raw(&ctx, exempt, "в_ремонте").await;
    ctx.groups
        .add_devices(&admin(), group, vec![exempt, regular])
        .await
        .expect("add");
    assert!(is_member(&ctx, exempt).await);
    assert_eq!(
        dev_row(&ctx, exempt).await.1,
        Some(place_device),
        "не стянут"
    );
    assert_eq!(
        dev_row(&ctx, regular).await.1,
        Some(place_group),
        "обычный стянут, как прежде"
    );
}

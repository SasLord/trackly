//! Фаза 41.8, план 29 (D-35, D-26): контракт `DeviceService::update` на минимальном
//! патче «только статус». Подменю «Статус» шлёт именно такой патч
//! (`status_id` и, для участника группы при «Списано», `group_write_off`), поэтому
//! поведение сервера на нём закреплено тестом, а не предположением.
//!
//! Настоящий `AppCtx::build`. Данные вымышленные. Префикс тестов: `status_only_patch_`.

mod entities_support;

use entities_support::fixture::{
    admin, make_test_ctx, seed_device, seed_group_type_and_group, seed_place,
};
use rusqlite::params;
use trackly_app::context::AppCtx;
use trackly_app::dto::device::{DevicePatch, GroupWriteOffChoice};
use trackly_app::tauri_cmds::devices::build_devices_update;
use trackly_core::auth::{Identity, Role};
use trackly_core::error::AppError;
use trackly_infra::error_conversions::map_rusqlite;

const REPAIR_CONFLICT: &str = "Нельзя изменить устройство: по нему открыта заявка на ремонт.";

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
        .unwrap_or_else(|e| panic!("статус {code}: {e:?}"))
}

async fn set_status(ctx: &AppCtx, device: i64, code: &'static str) {
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

async fn set_place(ctx: &AppCtx, device: i64, place: i64) {
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "UPDATE devices SET place_id = ?1 WHERE id = ?2",
                params![place, device],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("set place");
}

async fn set_anchor(ctx: &AppCtx, group: i64, device: i64) {
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "UPDATE groups SET anchor_device_id = ?1 WHERE id = ?2",
                params![device, group],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("set anchor");
}

async fn scalar(ctx: &AppCtx, sql: &'static str, device: i64) -> i64 {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(sql, params![device], |r| r.get::<_, i64>(0))
                .map_err(map_rusqlite)
        })
        .await
        .expect("scalar")
}

async fn code_of(ctx: &AppCtx, device: i64) -> String {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(
                "SELECT s.code FROM devices d JOIN device_statuses s ON s.id = d.status_id \
                 WHERE d.id = ?1",
                params![device],
                |r| r.get::<_, String>(0),
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("status code")
}

async fn place_of(ctx: &AppCtx, device: i64) -> Option<i64> {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(
                "SELECT place_id FROM devices WHERE id = ?1",
                params![device],
                |r| r.get::<_, Option<i64>>(0),
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("place")
}

async fn version_of(ctx: &AppCtx, device: i64) -> i64 {
    scalar(ctx, "SELECT version FROM devices WHERE id = ?1", device).await
}

async fn member_rows(ctx: &AppCtx, device: i64) -> i64 {
    scalar(
        ctx,
        "SELECT COUNT(*) FROM group_devices WHERE device_id = ?1",
        device,
    )
    .await
}

async fn release_audit(ctx: &AppCtx, device: i64) -> i64 {
    scalar(
        ctx,
        "SELECT COUNT(*) FROM audit_log \
         WHERE action = 'custom:group_member_released' AND entity_id = ?1",
        device,
    )
    .await
}

async fn open_repair_request(ctx: &AppCtx, device: i64) {
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
                 VALUES ('repair', 'open', (SELECT MIN(id) FROM users), ?1, 1700000000, 1700000000)",
                params![device],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("insert repair request");
}

fn status_patch(status: i64, choice: Option<GroupWriteOffChoice>) -> DevicePatch {
    DevicePatch {
        status_id: Some(status),
        group_write_off: choice,
        ..Default::default()
    }
}

/// Свободное устройство в статусе `code` на месте `place`.
async fn free_device(ctx: &AppCtx, name: &str, inv: &str, place: i64, code: &'static str) -> i64 {
    let id = seed_device(ctx, 1, name, inv, Some(place), None).await;
    set_status(ctx, id, code).await;
    id
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn status_only_patch_free_device_changes_status_only() {
    let (ctx, _d) = make_test_ctx().await;
    let place = seed_place(&ctx, "Склад А").await;
    let dev = free_device(&ctx, "Монитор #2", "INV-SO-1", place, "в_работе").await;
    let v = version_of(&ctx, dev).await;
    assert_eq!(code_of(&ctx, dev).await, "в_работе", "исходное состояние");
    let target = status_id(&ctx, "на_складе").await;

    ctx.devices
        .update(&admin(), dev, v, status_patch(target, None))
        .await
        .expect("обычная смена статуса");

    assert_eq!(code_of(&ctx, dev).await, "на_складе");
    assert_eq!(place_of(&ctx, dev).await, Some(place), "место не тронуто");
    assert_eq!(version_of(&ctx, dev).await, v + 1, "версия выросла");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn status_only_patch_member_write_off_requires_choice() {
    let (ctx, _d) = make_test_ctx().await;
    let place = seed_place(&ctx, "Склад А").await;
    let (_t, g) = seed_group_type_and_group(&ctx, "АРМ #3", 3, Some(place)).await;
    let dev = seed_device(&ctx, 1, "Монитор #2", "INV-SO-2", Some(place), Some(g)).await;
    set_status(&ctx, dev, "в_работе").await;
    let v = version_of(&ctx, dev).await;
    assert_eq!(member_rows(&ctx, dev).await, 1, "исходно участник");
    let written_off = status_id(&ctx, "списано").await;

    let err = ctx
        .devices
        .update(&admin(), dev, v, status_patch(written_off, None))
        .await
        .expect_err("без выбора списание участника отклоняется");
    match err {
        AppError::Validation { field, .. } => assert_eq!(field, "group_write_off"),
        other => panic!("ожидали Validation, получили {other:?}"),
    }
    assert_eq!(code_of(&ctx, dev).await, "в_работе", "статус не изменён");
    assert_eq!(member_rows(&ctx, dev).await, 1, "состав не изменён");
    assert_eq!(version_of(&ctx, dev).await, v, "версия не выросла");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn status_only_patch_member_write_off_keep_and_release() {
    let (ctx, _d) = make_test_ctx().await;
    let place = seed_place(&ctx, "Склад А").await;
    let (_t, g) = seed_group_type_and_group(&ctx, "АРМ #3", 3, Some(place)).await;
    let keep = seed_device(&ctx, 1, "Монитор #2", "INV-SO-3K", Some(place), Some(g)).await;
    let release = seed_device(&ctx, 1, "Клавиатура #1", "INV-SO-3R", Some(place), Some(g)).await;
    set_status(&ctx, keep, "в_работе").await;
    set_status(&ctx, release, "в_работе").await;
    let written_off = status_id(&ctx, "списано").await;
    assert_eq!(member_rows(&ctx, keep).await, 1);
    assert_eq!(member_rows(&ctx, release).await, 1);
    assert_eq!(release_audit(&ctx, release).await, 0);

    let vk = version_of(&ctx, keep).await;
    ctx.devices
        .update(
            &admin(),
            keep,
            vk,
            status_patch(written_off, Some(GroupWriteOffChoice::Keep)),
        )
        .await
        .expect("keep");
    assert_eq!(code_of(&ctx, keep).await, "списано");
    assert_eq!(member_rows(&ctx, keep).await, 1, "остался в группе");
    assert_eq!(release_audit(&ctx, keep).await, 0, "вывода не было");

    let vr = version_of(&ctx, release).await;
    ctx.devices
        .update(
            &admin(),
            release,
            vr,
            status_patch(written_off, Some(GroupWriteOffChoice::Release)),
        )
        .await
        .expect("release");
    assert_eq!(code_of(&ctx, release).await, "списано");
    assert_eq!(member_rows(&ctx, release).await, 0, "выведен из группы");
    assert_eq!(release_audit(&ctx, release).await, 1, "аудит вывода");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn status_only_patch_anchor_write_off_refused() {
    let (ctx, _d) = make_test_ctx().await;
    let place = seed_place(&ctx, "Склад А").await;
    let (_t, g) = seed_group_type_and_group(&ctx, "АРМ #3", 3, Some(place)).await;
    let anchor = free_device(&ctx, "Корпус #4", "INV-SO-4", place, "в_работе").await;
    set_anchor(&ctx, g, anchor).await;
    let v = version_of(&ctx, anchor).await;
    let written_off = status_id(&ctx, "списано").await;

    for choice in [None, Some(GroupWriteOffChoice::Release)] {
        let err = ctx
            .devices
            .update(&admin(), anchor, v, status_patch(written_off, choice))
            .await
            .expect_err("якорь не списывается через патч статуса");
        match err {
            AppError::Validation { field, message } => {
                assert_eq!(field, "status_id");
                assert!(message.contains("спишите"), "текст отказа: {message}");
            }
            other => panic!("ожидали Validation, получили {other:?}"),
        }
        assert_eq!(code_of(&ctx, anchor).await, "в_работе");
        assert_eq!(version_of(&ctx, anchor).await, v);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn status_only_patch_exempt_member_returns_to_group_place() {
    let (ctx, _d) = make_test_ctx().await;
    let group_place = seed_place(&ctx, "Склад А").await;
    let elsewhere = seed_place(&ctx, "Кабинет 12").await;
    let (_t, g) = seed_group_type_and_group(&ctx, "АРМ #3", 3, Some(group_place)).await;
    let dev = seed_device(
        &ctx,
        1,
        "Монитор #2",
        "INV-SO-5",
        Some(group_place),
        Some(g),
    )
    .await;
    set_status(&ctx, dev, "в_ремонте").await;
    set_place(&ctx, dev, elsewhere).await;
    let v = version_of(&ctx, dev).await;
    assert_eq!(
        place_of(&ctx, dev).await,
        Some(elsewhere),
        "место разошлось"
    );
    let target = status_id(&ctx, "в_работе").await;

    ctx.devices
        .update(&admin(), dev, v, status_patch(target, None))
        .await
        .expect("возврат из ремонта");

    assert_eq!(code_of(&ctx, dev).await, "в_работе");
    assert_eq!(
        place_of(&ctx, dev).await,
        Some(group_place),
        "перенос на место группы той же транзакцией (D-17)"
    );
    assert_eq!(member_rows(&ctx, dev).await, 1, "членство сохранено");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn status_only_patch_stale_version_rejected() {
    let (ctx, _d) = make_test_ctx().await;
    let place = seed_place(&ctx, "Склад А").await;
    let dev = free_device(&ctx, "Монитор #2", "INV-SO-6", place, "в_работе").await;
    let v = version_of(&ctx, dev).await;
    let target = status_id(&ctx, "на_складе").await;

    let err = ctx
        .devices
        .update(&admin(), dev, v + 5, status_patch(target, None))
        .await
        .expect_err("устаревшая версия");
    assert!(
        matches!(err, AppError::OptimisticLockMismatch { .. }),
        "получили {err:?}"
    );
    assert_eq!(code_of(&ctx, dev).await, "в_работе");
    assert_eq!(version_of(&ctx, dev).await, v);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn status_only_patch_open_repair_request_blocks_change() {
    let (ctx, _d) = make_test_ctx().await;
    let place = seed_place(&ctx, "Склад А").await;
    let dev = free_device(&ctx, "Принтер #1", "INV-SO-7", place, "в_работе").await;
    let v = version_of(&ctx, dev).await;
    let target = status_id(&ctx, "на_складе").await;

    // До заявки смена проходит бы: отказ вызван именно заявкой.
    open_repair_request(&ctx, dev).await;
    let err = ctx
        .devices
        .update(&admin(), dev, v, status_patch(target, None))
        .await
        .expect_err("открытая заявка блокирует смену статуса");
    match err {
        AppError::Conflict { reason } => assert_eq!(reason, REPAIR_CONFLICT),
        other => panic!("ожидали Conflict, получили {other:?}"),
    }
    assert_eq!(code_of(&ctx, dev).await, "в_работе");
    assert_eq!(version_of(&ctx, dev).await, v);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn status_only_patch_employee_forbidden_at_transport() {
    let (ctx, _d) = make_test_ctx().await;
    let place = seed_place(&ctx, "Склад А").await;
    let dev = free_device(&ctx, "Монитор #2", "INV-SO-8", place, "в_работе").await;
    let v = version_of(&ctx, dev).await;
    let target = status_id(&ctx, "на_складе").await;
    let employee = Identity {
        user_id: None,
        role: Role::Employee,
    };

    let err = build_devices_update(&ctx, &employee, dev, v, status_patch(target, None))
        .await
        .expect_err("employee не меняет статус");
    assert!(matches!(err, AppError::Forbidden), "получили {err:?}");
    assert_eq!(code_of(&ctx, dev).await, "в_работе");

    // Контроль несущей способности: manager тем же путём проходит.
    let manager = Identity {
        user_id: None,
        role: Role::Manager,
    };
    build_devices_update(&ctx, &manager, dev, v, status_patch(target, None))
        .await
        .expect("manager меняет статус");
    assert_eq!(code_of(&ctx, dev).await, "на_складе");
}

//! Интеграционные тесты `PlaceService::delete_hard` (Phase 39 Plan 05, Task 2):
//! D-14 — удаление непустого узла блокируется с точными счётчиками (UI-SPEC
//! §11.5/§14.3), удаление пустого листа проходит успешно.

use std::sync::Arc;
use std::time::Duration;

use trackly_app::services::PlaceService;
use trackly_core::auth::Identity;
use trackly_core::domain::places::{PlaceKind, PlaceNew};
use trackly_core::error::AppError;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::test_support::test_writer_and_readers;

fn make_service() -> (PlaceService, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let svc = PlaceService::new(writer, readers, clock);
    (svc, dir)
}

fn admin_caller() -> Identity {
    Identity::trusted_admin()
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

/// Inserts a minimal device row directly (device_types id=1 / device_statuses
/// id=1 are seeded by V001) — mirrors `places_crud.rs`'s fixture style. No
/// `DeviceService` call needed; this is a raw fixture insert via the writer.
async fn insert_device(svc: &PlaceService, place_id: i64, name: &str) {
    let name = name.to_string();
    svc.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO devices \
                 (type_id, name, place_id, status_id, version, created_at_utc, updated_at_utc) \
                 VALUES (1, ?1, ?2, 1, 1, 0, 0)",
                rusqlite::params![name, place_id],
            )
            .map_err(trackly_infra::error_conversions::map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("insert fixture device");
}

/// Inserts a minimal `acts` row directly, referencing `place_id` through the
/// given `column` (`"place_id"` or `"bulk_place_id"`) — CR-01 (phase 39
/// review) regression fixture. Only fictional names, per the project's hard
/// privacy constraint. `number` must be unique among live acts
/// (`idx_acts_number_sub_unique`), so callers pass a distinct value per test.
async fn insert_act_referencing_place(
    svc: &PlaceService,
    column: &'static str,
    place_id: i64,
    number: i64,
) {
    svc.writer
        .execute(move |conn| {
            let sql = format!(
                "INSERT INTO acts \
                 (number, act_type, giver_name, receiver_name, {column}, created_at_utc, updated_at_utc) \
                 VALUES (?1, 'handover', 'Иванов И.И.', 'Петров П.П.', ?2, 0, 0)"
            );
            conn.execute(&sql, rusqlite::params![number, place_id])
                .map_err(trackly_infra::error_conversions::map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("insert fixture act");
}

/// Inserts a minimal `acts` row plus a single `act_items` row whose
/// `place_id_override` references `place_id` — CR-01's third FK path.
/// `act_items` requires a real `device_id` (NOT NULL FK), so a throwaway
/// device is inserted under `device_place_id` (deliberately NOT the place
/// under test, to prove the override column — not the device's own
/// location — is what the subtree-stats query must catch).
async fn insert_act_with_item_override(
    svc: &PlaceService,
    place_id: i64,
    device_place_id: i64,
    number: i64,
) {
    svc.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO devices \
                 (type_id, name, place_id, status_id, version, created_at_utc, updated_at_utc) \
                 VALUES (1, 'Ноутбук override-fixture', ?1, 1, 1, 0, 0)",
                rusqlite::params![device_place_id],
            )
            .map_err(trackly_infra::error_conversions::map_rusqlite)?;
            let device_id = conn.last_insert_rowid();

            conn.execute(
                "INSERT INTO acts (number, act_type, giver_name, receiver_name, created_at_utc, updated_at_utc) \
                 VALUES (?1, 'handover', 'Иванов И.И.', 'Петров П.П.', 0, 0)",
                rusqlite::params![number],
            )
            .map_err(trackly_infra::error_conversions::map_rusqlite)?;
            let act_id = conn.last_insert_rowid();

            conn.execute(
                "INSERT INTO act_items (act_id, device_id, place_id_override) VALUES (?1, ?2, ?3)",
                rusqlite::params![act_id, device_id, place_id],
            )
            .map_err(trackly_infra::error_conversions::map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("insert fixture act+item override");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn delete_blocked_on_non_empty_subtree_surfaces_exact_counts() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let admin = admin_caller();

        let building = svc
            .create(&admin, new_place(PlaceKind::Building, "Здание А", None))
            .await
            .expect("create building");
        let _room = svc
            .create(&admin, new_place(PlaceKind::Room, "214", Some(building.id)))
            .await
            .expect("create nested room");

        insert_device(&svc, building.id, "Ноутбук №1").await;
        insert_device(&svc, building.id, "Ноутбук №2").await;

        let err = svc
            .delete_hard(&admin, building.id, building.version)
            .await
            .expect_err(
                "удаление непустого узла (2 устройства, 1 вложенное место) должно быть отклонено",
            );

        match err {
            AppError::Conflict { reason } => {
                assert!(
                    reason.contains("2 устройства"),
                    "должен содержать точный счётчик устройств: {reason}"
                );
                assert!(
                    reason.contains("1 вложенное место"),
                    "должен содержать точный счётчик вложенных мест: {reason}"
                );
                assert!(
                    reason.starts_with("Место нельзя удалить:"),
                    "должен использовать литеральный шаблон UI-SPEC §11.5/§14.3: {reason}"
                );
                assert!(
                    reason.ends_with("Перенесите содержимое или архивируйте место."),
                    "должен использовать литеральный шаблон UI-SPEC §11.5/§14.3: {reason}"
                );
            }
            other => panic!("ожидали AppError::Conflict, получили {other:?}"),
        }
    })
    .await
    .expect("test timed out");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn delete_succeeds_on_empty_leaf_place() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let admin = admin_caller();

        let empty_leaf = svc
            .create(
                &admin,
                new_place(PlaceKind::Territory, "Пустая территория", None),
            )
            .await
            .expect("create empty leaf");

        svc.delete_hard(&admin, empty_leaf.id, empty_leaf.version)
            .await
            .expect("удаление пустого листа должно пройти успешно");

        let readers = svc.readers.clone();
        let id = empty_leaf.id;
        let count: i64 = tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            conn.query_row(
                "SELECT COUNT(*) FROM places WHERE id = ?1",
                rusqlite::params![id],
                |r| r.get(0),
            )
        })
        .await
        .expect("join")
        .expect("query places");

        assert_eq!(count, 0, "место должно быть физически удалено из таблицы");
    })
    .await
    .expect("test timed out");
}

// ---------------------------------------------------------------------------
// CR-01 (phase 39 review): a place with ZERO child places, ZERO devices and
// ZERO cartridges — otherwise "empty" — but still referenced by a live act
// through one of D-16's frozen snapshot columns must be blocked by the
// pre-flight check with a correct Russian message, NOT allowed through to a
// raw SQLite FK error. Before the fix, `subtree_stats_impl` never counted
// acts/act_items, so `delete_hard`'s pre-check reported "safe to delete" and
// the writer's `DELETE` hit `ON DELETE RESTRICT`, surfacing the raw English
// SQLite message verbatim as the Conflict reason.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn delete_blocked_by_act_place_id_even_when_otherwise_empty() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let admin = admin_caller();

        let room = svc
            .create(&admin, new_place(PlaceKind::Room, "214", None))
            .await
            .expect("create room");

        // Room is otherwise completely empty: no children, no devices, no
        // cartridges. Only a live act's `place_id` references it (D-16: the
        // act was issued here, then every device moved away — the act's
        // reference is deliberately frozen, not updated).
        insert_act_referencing_place(&svc, "place_id", room.id, 1001).await;

        let err = svc
            .delete_hard(&admin, room.id, room.version)
            .await
            .expect_err("otherwise-empty место, на которое ссылается акт, всё равно должно блокироваться");

        match err {
            AppError::Conflict { reason } => {
                assert!(
                    reason.contains("акт"),
                    "должен сообщать про ссылающийся акт по-русски, а не пропускать проверку: {reason}"
                );
                assert!(
                    !reason.contains("FOREIGN KEY"),
                    "не должен протекать сырое английское сообщение SQLite в диалог: {reason}"
                );
                assert!(
                    reason.starts_with("Место нельзя удалить:"),
                    "должен использовать литеральный шаблон UI-SPEC §11.5/§14.3: {reason}"
                );
            }
            other => panic!("ожидали AppError::Conflict, получили {other:?}"),
        }

        // The place must still exist — the pre-flight check should have
        // stopped this BEFORE the writer ever ran a DELETE.
        let readers = svc.readers.clone();
        let id = room.id;
        let count: i64 = tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            conn.query_row(
                "SELECT COUNT(*) FROM places WHERE id = ?1",
                rusqlite::params![id],
                |r| r.get(0),
            )
        })
        .await
        .expect("join")
        .expect("query places");
        assert_eq!(count, 1, "заблокированное место не должно быть удалено");
    })
    .await
    .expect("test timed out");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn delete_blocked_by_act_bulk_place_id_even_when_otherwise_empty() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let admin = admin_caller();

        let room = svc
            .create(&admin, new_place(PlaceKind::Room, "305", None))
            .await
            .expect("create room");

        insert_act_referencing_place(&svc, "bulk_place_id", room.id, 1002).await;

        let err = svc
            .delete_hard(&admin, room.id, room.version)
            .await
            .expect_err("акт, ссылающийся через bulk_place_id, тоже должен блокировать удаление");

        match err {
            AppError::Conflict { reason } => {
                assert!(reason.contains("акт"), "должен сообщать про акт: {reason}");
            }
            other => panic!("ожидали AppError::Conflict, получили {other:?}"),
        }
    })
    .await
    .expect("test timed out");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn delete_blocked_by_act_item_place_id_override_even_when_otherwise_empty() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let admin = admin_caller();

        let overridden_place = svc
            .create(&admin, new_place(PlaceKind::Room, "Каб. 12", None))
            .await
            .expect("create overridden place");
        let device_home = svc
            .create(&admin, new_place(PlaceKind::Room, "Каб. 13", None))
            .await
            .expect("create device's own place");

        // The device itself lives in `device_home`, NOT `overridden_place` —
        // proves the query must follow `act_items.place_id_override`, not
        // the device's own `place_id`.
        insert_act_with_item_override(&svc, overridden_place.id, device_home.id, 1003).await;

        let err = svc
            .delete_hard(&admin, overridden_place.id, overridden_place.version)
            .await
            .expect_err("акт, ссылающийся через act_items.place_id_override, тоже должен блокировать удаление");

        match err {
            AppError::Conflict { reason } => {
                assert!(reason.contains("акт"), "должен сообщать про акт: {reason}");
            }
            other => panic!("ожидали AppError::Conflict, получили {other:?}"),
        }

        // The device's own place is untouched by the act reference (it is
        // blocked by the device itself, not by the act) — confirms the act
        // reference did NOT leak onto the wrong place via the override.
        let err2 = svc
            .delete_hard(&admin, device_home.id, device_home.version)
            .await
            .expect_err("место с устройством всё ещё блокируется — но по устройству, не по акту");
        match err2 {
            AppError::Conflict { reason } => {
                assert!(
                    reason.contains("устройство") && !reason.contains("акт"),
                    "device_home должен блокироваться устройством, а не ошибочно приписанным актом: {reason}"
                );
            }
            other => panic!("ожидали AppError::Conflict, получили {other:?}"),
        }
    })
    .await
    .expect("test timed out");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn act_referencing_same_place_through_two_columns_counts_once() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let admin = admin_caller();

        let place = svc
            .create(&admin, new_place(PlaceKind::Room, "Каб. 20", None))
            .await
            .expect("create place");

        // Same act references `place` via BOTH place_id and bulk_place_id —
        // the subtree-stats query must count it once (DISTINCT a.id), not twice.
        let place_id = place.id;
        svc.writer
            .execute(move |conn| {
                conn.execute(
                    "INSERT INTO acts (number, act_type, giver_name, receiver_name, place_id, bulk_place_id, created_at_utc, updated_at_utc) \
                     VALUES (1004, 'handover', 'Иванов И.И.', 'Петров П.П.', ?1, ?1, 0, 0)",
                    rusqlite::params![place_id],
                )
                .map_err(trackly_infra::error_conversions::map_rusqlite)?;
                Ok(())
            })
            .await
            .expect("insert dual-reference act");

        let err = svc
            .delete_hard(&admin, place.id, place.version)
            .await
            .expect_err("должен блокироваться");

        match err {
            AppError::Conflict { reason } => {
                assert!(
                    reason.contains("1 акт") && !reason.contains("2 акт"),
                    "акт, ссылающийся через ДВЕ колонки, должен считаться один раз: {reason}"
                );
            }
            other => panic!("ожидали AppError::Conflict, получили {other:?}"),
        }
    })
    .await
    .expect("test timed out");
}

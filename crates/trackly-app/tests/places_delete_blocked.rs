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
            .create(
                &admin,
                new_place(PlaceKind::Room, "214", Some(building.id)),
            )
            .await
            .expect("create nested room");

        insert_device(&svc, building.id, "Ноутбук №1").await;
        insert_device(&svc, building.id, "Ноутбук №2").await;

        let err = svc
            .delete_hard(&admin, building.id, building.version)
            .await
            .expect_err("удаление непустого узла (2 устройства, 1 вложенное место) должно быть отклонено");

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
            .create(&admin, new_place(PlaceKind::Territory, "Пустая территория", None))
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

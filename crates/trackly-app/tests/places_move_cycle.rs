//! Интеграционные тесты `PlaceService::move_node` (Phase 39 Plan 05, Task 2):
//! cycle rejection (Pattern 3, UI-SPEC §14.3) и успешное перемещение в
//! несвязанного валидного родителя.

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

/// UI-SPEC §14.3's exact cycle-error copy, written verbatim by
/// `SqlitePlaceRepository::move_node` (Plan 04, Pattern 3) and propagated
/// unchanged by `PlaceService::move_node` (Plan 05, Task 2).
const CYCLE_ERROR_COPY: &str =
    "Нельзя переместить место внутрь самого себя или своего вложенного места.";

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn move_into_own_descendant_is_rejected_with_ui_spec_copy() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let admin = admin_caller();

        let building = svc
            .create(&admin, new_place(PlaceKind::Building, "Здание А", None))
            .await
            .expect("create building");
        let floor = svc
            .create(
                &admin,
                new_place(PlaceKind::Floor, "2 этаж", Some(building.id)),
            )
            .await
            .expect("create floor");
        let room = svc
            .create(&admin, new_place(PlaceKind::Room, "214", Some(floor.id)))
            .await
            .expect("create room");

        // Moving the building into its own great-grandchild (the room) is a cycle.
        let err = svc
            .move_node(&admin, building.id, Some(room.id), building.version)
            .await
            .expect_err("перемещение узла внутрь своего вложенного места должно быть отклонено");

        match err {
            AppError::Validation { field, message } => {
                assert_eq!(field, "parent_id");
                assert_eq!(
                    message, CYCLE_ERROR_COPY,
                    "текст ошибки должен точно совпадать с UI-SPEC §14.3"
                );
            }
            other => panic!("ожидали AppError::Validation, получили {other:?}"),
        }

        // Self-move is also a cycle.
        let self_move_err = svc
            .move_node(&admin, floor.id, Some(floor.id), floor.version)
            .await
            .expect_err("перемещение узла в самого себя должно быть отклонено");
        match self_move_err {
            AppError::Validation { message, .. } => {
                assert_eq!(message, CYCLE_ERROR_COPY);
            }
            other => panic!("ожидали AppError::Validation, получили {other:?}"),
        }
    })
    .await
    .expect("test timed out");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn move_to_unrelated_valid_parent_succeeds() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let admin = admin_caller();

        let building_a = svc
            .create(&admin, new_place(PlaceKind::Building, "Здание А", None))
            .await
            .expect("create building A");
        let building_b = svc
            .create(&admin, new_place(PlaceKind::Building, "Здание Б", None))
            .await
            .expect("create building B");
        let room = svc
            .create(
                &admin,
                new_place(PlaceKind::Room, "101", Some(building_a.id)),
            )
            .await
            .expect("create room under building A");

        let moved = svc
            .move_node(&admin, room.id, Some(building_b.id), room.version)
            .await
            .expect("перемещение в несвязанного родителя должно пройти успешно");

        assert_eq!(moved.parent_id, Some(building_b.id));
        assert_eq!(moved.version, room.version + 1);
    })
    .await
    .expect("test timed out");
}

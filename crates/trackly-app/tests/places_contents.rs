//! Интеграционные тесты `PlaceService`'s read half (Phase 39 Plan 08, Task 1):
//! `list_subtree_contents` D-24 nested-vs-"Только здесь" toggle, `list_children`
//! natural sibling ordering (D-05), `subtree_stats` nested-inclusive counters
//! (D-25).
//!
//! Каждый тест обёрнут в `tokio::time::timeout(30s)` — защита от Linux-CI
//! deadlock (PATTERNS.md §Pattern 4), mirrors `places_delete_blocked.rs`'s
//! idiom. Все имена мест/устройств вымышленные (приватность репозитория).

use std::sync::Arc;
use std::time::Duration;

use trackly_app::services::PlaceService;
use trackly_core::auth::Identity;
use trackly_core::domain::places::{sibling_cmp, PlaceKind, PlaceNew};
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
/// id=1 are seeded by V001) — mirrors `places_delete_blocked.rs`'s fixture
/// style. No `DeviceService` call needed; this is a raw fixture insert via
/// the writer.
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

// ---------------------------------------------------------------------------
// list_subtree_contents — D-24 nested-vs-"Только здесь" toggle
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn list_subtree_contents_nested_true_includes_nested_place_devices() {
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
            .expect("create nested floor");

        // 2 direct devices on the building, 1 nested device on the floor beneath it.
        insert_device(&svc, building.id, "Ноутбук №1").await;
        insert_device(&svc, building.id, "Ноутбук №2").await;
        insert_device(&svc, floor.id, "Ноутбук №3").await;

        let nested = svc
            .list_subtree_contents(&admin, building.id, true)
            .await
            .expect("list_subtree_contents nested=true");
        assert_eq!(
            nested.len(),
            3,
            "nested=true должен вернуть все 3 устройства (2 прямых + 1 вложенное)"
        );

        let direct_only = svc
            .list_subtree_contents(&admin, building.id, false)
            .await
            .expect("list_subtree_contents nested=false");
        assert_eq!(
            direct_only.len(),
            2,
            "nested=false («Только здесь») должен вернуть только 2 прямых устройства"
        );
    })
    .await
    .expect("test timed out");
}

// ---------------------------------------------------------------------------
// list_children — sibling_cmp natural ordering, not DB insertion order
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn list_children_sorted_by_sibling_cmp_not_insertion_order() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let admin = admin_caller();

        let building = svc
            .create(&admin, new_place(PlaceKind::Building, "Здание Б", None))
            .await
            .expect("create building");

        // Insert rooms out of natural order: "10" before "2" — natural_name_cmp
        // must still order them "2" < "10" (not lexicographic "10" < "2").
        let _room_10 = svc
            .create(&admin, new_place(PlaceKind::Room, "10", Some(building.id)))
            .await
            .expect("create room 10");
        let _room_2 = svc
            .create(&admin, new_place(PlaceKind::Room, "2", Some(building.id)))
            .await
            .expect("create room 2");

        let children = svc
            .list_children(&admin, Some(building.id))
            .await
            .expect("list_children");

        assert_eq!(children.len(), 2);
        assert_eq!(
            children.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(),
            vec!["2", "10"],
            "ожидали натуральный порядок «2» перед «10», а не порядок вставки в БД"
        );
    })
    .await
    .expect("test timed out");
}

// ---------------------------------------------------------------------------
// subtree_stats — D-25 nested-inclusive counters
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn subtree_stats_counts_nested_places_and_devices_inclusive() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let admin = admin_caller();

        let building = svc
            .create(&admin, new_place(PlaceKind::Building, "Здание В", None))
            .await
            .expect("create building");
        let floor = svc
            .create(
                &admin,
                new_place(PlaceKind::Floor, "3 этаж", Some(building.id)),
            )
            .await
            .expect("create nested floor");

        insert_device(&svc, building.id, "Ноутбук №1").await;
        insert_device(&svc, building.id, "Ноутбук №2").await;
        insert_device(&svc, floor.id, "Ноутбук №3").await;

        let stats = svc
            .subtree_stats(&admin, building.id)
            .await
            .expect("subtree_stats");

        assert_eq!(stats.nested_places, 1, "1 вложенное место (floor)");
        assert_eq!(
            stats.device_count, 3,
            "device_count должен учитывать вложенные места (D-25): 2 прямых + 1 вложенное"
        );
    })
    .await
    .expect("test timed out");
}

// ---------------------------------------------------------------------------
// quick 260827-rzq: list_children/list_all must not panic on a partial
// sort_order set that contradicts name/level order — the shape produced by
// drag-and-drop reordering a subset of siblings, which crashed
// places_list_all/places_list_children in production before the sibling_cmp fix.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn list_children_and_list_all_survive_partial_sort_order_without_panicking() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let admin = admin_caller();

        let building = svc
            .create(&admin, new_place(PlaceKind::Building, "Здание Г", None))
            .await
            .expect("create building");

        // ~15 rooms under the same building. Names deliberately contradict both
        // level and sort_order. Roughly a third get an explicit sort_order that
        // runs in reverse of insertion order (mirrors drag-and-drop reordering a
        // subset of siblings); the rest get None (natural order applies).
        for i in 0..15i64 {
            let sort_order = if i % 3 == 0 { Some(15 - i) } else { None };
            let level = Some((i % 4) - 1);
            let name = format!("Каб. {}", 15 - i);
            let place_new = PlaceNew {
                parent_id: Some(building.id),
                kind: PlaceKind::Room,
                name,
                level,
                is_storage: false,
                sort_order,
                notes: None,
            };
            svc.create(&admin, place_new)
                .await
                .expect("create room with partial sort_order/level");
        }

        // Neither call must panic (production regression: sort_by(sibling_cmp)
        // paniced with "user-provided comparison function does not correctly
        // implement a total order" on exactly this partial-sort_order shape).
        let children = svc
            .list_children(&admin, Some(building.id))
            .await
            .expect("list_children must not panic on partial sort_order");
        assert_eq!(children.len(), 15);
        for w in children.windows(2) {
            assert_ne!(
                sibling_cmp(&w[0], &w[1]),
                std::cmp::Ordering::Greater,
                "list_children: non-decreasing sibling order violated between {:?} and {:?}",
                w[0].name,
                w[1].name
            );
        }

        let all = svc
            .list_all(&admin, false)
            .await
            .expect("list_all must not panic on partial sort_order");
        assert!(all.len() >= 16, "building + 15 rooms");

        // list_all groups by parent_id then applies sibling_cmp within the group —
        // verify every adjacent pair sharing a parent_id is non-decreasing under
        // sibling_cmp (non-adjacent-parent pairs are not meaningfully comparable
        // and are not asserted on).
        for w in all.windows(2) {
            if w[0].parent_id == w[1].parent_id {
                assert_ne!(
                    sibling_cmp(&w[0], &w[1]),
                    std::cmp::Ordering::Greater,
                    "list_all: non-decreasing sibling order violated within parent_id={:?} between {:?} and {:?}",
                    w[0].parent_id,
                    w[0].name,
                    w[1].name
                );
            }
        }
    })
    .await
    .expect("test timed out");
}

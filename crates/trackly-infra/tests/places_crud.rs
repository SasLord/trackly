//! Integration tests: `SqlitePlaceRepository` — PLC-01 coverage.
//!
//! Each test is wrapped in `tokio::time::timeout(30s)` (PATTERNS.md §Pattern 4 —
//! Linux-CI deadlock defense), mirroring `crates/trackly-app/tests/devices_crud.rs`'s
//! convention even though this crate's repository calls are synchronous.
//!
//! Uses `trackly_infra::test_support::test_db()` directly against a raw
//! `rusqlite::Connection` — no `PlaceService` exists yet (that's Plan 05), so these
//! tests exercise the `PlaceRepository` trait impl at the repository layer.
//!
//! Only invented place names ("Здание А", "2 этаж", "214") — never real
//! organization data, per the project's hard privacy constraint.

use std::time::Duration;

use trackly_core::domain::places::{PlaceKind, PlaceNew};
use trackly_core::error::AppError;
use trackly_core::ports::places::PlaceRepository;
use trackly_infra::repos::SqlitePlaceRepository;
use trackly_infra::test_support::test_db;

const NOW: i64 = 1_700_000_000;

fn new_place(parent_id: Option<i64>, kind: PlaceKind, name: &str) -> PlaceNew {
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

// ---------------------------------------------------------------------------
// create + get: root and child, correct parent_id/full_path
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_root_and_child_get_returns_correct_parent_id_and_full_path() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (mut conn, _dir) = test_db();
        let repo = SqlitePlaceRepository;

        let root_id = repo
            .create(&mut conn, &new_place(None, PlaceKind::Building, "Здание А"), NOW)
            .expect("create root");
        let child_id = repo
            .create(
                &mut conn,
                &new_place(Some(root_id), PlaceKind::Floor, "2 этаж"),
                NOW,
            )
            .expect("create child");

        let root = repo.get(&conn, root_id).expect("get root");
        assert_eq!(root.parent_id, None);
        assert_eq!(root.name, "Здание А");
        assert_eq!(root.full_path.as_deref(), Some("Здание А"));

        let child = repo.get(&conn, child_id).expect("get child");
        assert_eq!(child.parent_id, Some(root_id));
        assert_eq!(child.name, "2 этаж");
        assert_eq!(child.full_path.as_deref(), Some("Здание А / 2 этаж"));
    })
    .await
    .expect("test timed out");
}

// ---------------------------------------------------------------------------
// rename: descendant full_path reflects new name immediately (no reindex call)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn rename_updates_descendant_full_path_without_separate_reindex_call() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (mut conn, _dir) = test_db();
        let repo = SqlitePlaceRepository;

        let root_id = repo
            .create(&mut conn, &new_place(None, PlaceKind::Building, "Здание А"), NOW)
            .expect("create root");
        let child_id = repo
            .create(
                &mut conn,
                &new_place(Some(root_id), PlaceKind::Floor, "2 этаж"),
                NOW,
            )
            .expect("create child");

        repo.rename(&mut conn, root_id, "Здание Б", 1, NOW + 1)
            .expect("rename root");

        // No separate reindex call — place_full_paths is a live-recomputed VIEW.
        let child = repo.get(&conn, child_id).expect("get child");
        assert_eq!(child.full_path.as_deref(), Some("Здание Б / 2 этаж"));
    })
    .await
    .expect("test timed out");
}

// ---------------------------------------------------------------------------
// move_node: FK survives (device.place_id unchanged after subtree move)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn move_node_preserves_device_place_id_fk() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (mut conn, _dir) = test_db();
        let repo = SqlitePlaceRepository;

        let building_id = repo
            .create(&mut conn, &new_place(None, PlaceKind::Building, "Здание А"), NOW)
            .expect("create building");
        let floor2_id = repo
            .create(
                &mut conn,
                &new_place(Some(building_id), PlaceKind::Floor, "2 этаж"),
                NOW,
            )
            .expect("create floor2");
        let floor3_id = repo
            .create(
                &mut conn,
                &new_place(Some(building_id), PlaceKind::Floor, "3 этаж"),
                NOW,
            )
            .expect("create floor3");
        let room_id = repo
            .create(&mut conn, &new_place(Some(floor2_id), PlaceKind::Room, "214"), NOW)
            .expect("create room");

        conn.execute(
            "INSERT INTO devices (type_id, name, place_id, status_id, version, created_at_utc, updated_at_utc) \
             VALUES (1, 'Ноутбук Test', ?1, 1, 1, ?2, ?2)",
            rusqlite::params![room_id, NOW],
        )
        .expect("insert device");
        let device_id = conn.last_insert_rowid();

        // Move the room (with the device inside it) from floor2 to floor3.
        repo.move_node(&mut conn, room_id, Some(floor3_id), 1, NOW + 1)
            .expect("move room");

        let device_place_id: i64 = conn
            .query_row(
                "SELECT place_id FROM devices WHERE id = ?1",
                rusqlite::params![device_id],
                |r| r.get(0),
            )
            .expect("query device place_id");
        assert_eq!(
            device_place_id, room_id,
            "device.place_id must be unchanged — the device still points at the room, only the room moved"
        );

        let room = repo.get(&conn, room_id).expect("get room");
        assert_eq!(room.parent_id, Some(floor3_id));
        assert_eq!(room.full_path.as_deref(), Some("Здание А / 3 этаж / 214"));
    })
    .await
    .expect("test timed out");
}

// ---------------------------------------------------------------------------
// move_node: cycle rejection (move into own descendant)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn move_node_into_own_descendant_rejected_as_validation_error() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (mut conn, _dir) = test_db();
        let repo = SqlitePlaceRepository;

        let building_id = repo
            .create(&mut conn, &new_place(None, PlaceKind::Building, "Здание А"), NOW)
            .expect("create building");
        let floor_id = repo
            .create(
                &mut conn,
                &new_place(Some(building_id), PlaceKind::Floor, "2 этаж"),
                NOW,
            )
            .expect("create floor");
        let room_id = repo
            .create(&mut conn, &new_place(Some(floor_id), PlaceKind::Room, "214"), NOW)
            .expect("create room");

        // Attempt to move the building (ancestor of room_id) into room_id (its own descendant).
        let err = repo
            .move_node(&mut conn, building_id, Some(room_id), 1, NOW + 1)
            .expect_err("moving a node into its own descendant must be rejected");
        match err {
            AppError::Validation { field, .. } => assert_eq!(field, "parent_id"),
            other => panic!("expected AppError::Validation, got {other:?}"),
        }

        // Self-move is also a cycle.
        let err2 = repo
            .move_node(&mut conn, floor_id, Some(floor_id), 1, NOW + 1)
            .expect_err("moving a node into itself must be rejected");
        assert!(matches!(err2, AppError::Validation { .. }));
    })
    .await
    .expect("test timed out");
}

// ---------------------------------------------------------------------------
// uniqueness (D-04): same name under same parent rejected, different parent OK
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn duplicate_name_under_same_parent_conflicts_different_parent_succeeds() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (mut conn, _dir) = test_db();
        let repo = SqlitePlaceRepository;

        let floor_a_id = repo
            .create(&mut conn, &new_place(None, PlaceKind::Floor, "2 этаж (крыло А)"), NOW)
            .expect("create floor A");
        let floor_b_id = repo
            .create(&mut conn, &new_place(None, PlaceKind::Floor, "2 этаж (крыло Б)"), NOW)
            .expect("create floor B");

        repo.create(&mut conn, &new_place(Some(floor_a_id), PlaceKind::Room, "214"), NOW)
            .expect("create room 214 under floor A");

        let err = repo
            .create(&mut conn, &new_place(Some(floor_a_id), PlaceKind::Room, "214"), NOW)
            .expect_err("duplicate name under same parent must conflict");
        assert!(matches!(err, AppError::Conflict { .. }), "expected Conflict, got {err:?}");

        // Same name under a DIFFERENT parent succeeds.
        repo.create(&mut conn, &new_place(Some(floor_b_id), PlaceKind::Room, "214"), NOW)
            .expect("same name under a different parent must succeed");
    })
    .await
    .expect("test timed out");
}

// ---------------------------------------------------------------------------
// delete_hard: non-empty subtree conflicts with exact counts, empty leaf succeeds
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn delete_hard_blocks_non_empty_subtree_and_succeeds_on_empty_leaf() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (mut conn, _dir) = test_db();
        let repo = SqlitePlaceRepository;

        let building_id = repo
            .create(&mut conn, &new_place(None, PlaceKind::Building, "Здание А"), NOW)
            .expect("create building");
        let _floor_id = repo
            .create(
                &mut conn,
                &new_place(Some(building_id), PlaceKind::Floor, "2 этаж"),
                NOW,
            )
            .expect("create floor");

        // Building has a child place — delete must be blocked with a Conflict.
        let err = repo
            .delete_hard(&mut conn, building_id, 1)
            .expect_err("delete of a non-empty subtree must conflict");
        match err {
            AppError::Conflict { reason } => {
                assert!(
                    reason.contains('1'),
                    "conflict reason should surface exact non-zero counts, got: {reason}"
                );
            }
            other => panic!("expected AppError::Conflict, got {other:?}"),
        }

        // A device directly under a leaf place also blocks delete.
        let room_id = repo
            .create(
                &mut conn,
                &new_place(Some(building_id), PlaceKind::Room, "214"),
                NOW,
            )
            .expect("create room");
        conn.execute(
            "INSERT INTO devices (type_id, name, place_id, status_id, version, created_at_utc, updated_at_utc) \
             VALUES (1, 'Ноутбук Test', ?1, 1, 1, ?2, ?2)",
            rusqlite::params![room_id, NOW],
        )
        .expect("insert device");
        let err2 = repo
            .delete_hard(&mut conn, room_id, 1)
            .expect_err("delete of a place with a linked device must conflict");
        assert!(matches!(err2, AppError::Conflict { .. }));

        // An empty leaf place deletes successfully.
        let empty_id = repo
            .create(&mut conn, &new_place(None, PlaceKind::Territory, "Пустая территория"), NOW)
            .expect("create empty leaf");
        repo.delete_hard(&mut conn, empty_id, 1).expect("delete empty leaf");

        let not_found = repo.get(&conn, empty_id).expect_err("deleted place must be gone");
        assert!(matches!(not_found, AppError::NotFound { .. }));
    })
    .await
    .expect("test timed out");
}

//! Integration tests: `SqlitePlaceMovementsRepository` (Phase 40 Plan 05, HST-01/02/03).
//!
//! Covers the write-side guard (D-04/D-06), the real insert path (D-09/D-10 snapshots),
//! and the act-scoped delete (D-03). The read-side `get_history` (D-20) is covered by
//! Task 2's tests, appended to this same file.
//!
//! Only invented place/user data ("Здание А", "1 этаж", "Иванов И.И.") — never real
//! organization data, per the project's hard privacy constraint.

use std::time::Duration;

use rusqlite::{params, Connection};
use trackly_core::domain::place_movements::{MovementEntityKind, MovementSource};
use trackly_core::domain::places::{PlaceKind, PlaceNew};
use trackly_core::ports::places::PlaceRepository;
use trackly_infra::repos::{SqlitePlaceMovementsRepository, SqlitePlaceRepository};
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

/// Seed two root-level places for use as "from"/"to" in movement tests. Returns
/// `(place_a_id, place_b_id)`.
fn seed_two_places(conn: &mut Connection) -> (i64, i64) {
    let places_repo = SqlitePlaceRepository;
    let a = places_repo
        .create(conn, &new_place(None, PlaceKind::Building, "Здание А"), NOW)
        .expect("create place A");
    let b = places_repo
        .create(conn, &new_place(None, PlaceKind::Building, "Здание Б"), NOW)
        .expect("create place B");
    (a, b)
}

/// Seed a user row, returns its id. Uses only invented ФИО, never real names.
fn seed_user(conn: &Connection, full_name: &str) -> i64 {
    conn.execute(
        "INSERT INTO users (login, full_name, role, ad_user, created_at_utc, updated_at_utc) \
         VALUES (?1, ?2, 'employee', 0, ?3, ?3)",
        params![format!("user-{full_name}"), full_name, NOW],
    )
    .expect("insert user");
    conn.last_insert_rowid()
}

/// Seed a minimal act row (satisfies `place_movements.act_id`'s FK), returns its id.
/// Only invented ФИО ("Иванов И.И." / "Петров П.П."), never real names.
fn seed_act(conn: &Connection, number: i64) -> i64 {
    conn.execute(
        "INSERT INTO acts (number, act_type, giver_name, receiver_name, created_at_utc, updated_at_utc) \
         VALUES (?1, 'handover', 'Иванов И.И.', 'Петров П.П.', ?2, ?2)",
        params![number, NOW],
    )
    .expect("insert act");
    conn.last_insert_rowid()
}

// ---------------------------------------------------------------------------
// Task 1: guard skips (D-04/D-06) — zero rows inserted in each case
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn record_movement_skips_when_place_unchanged() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (mut conn, _dir) = test_db();
        let (place_a, _place_b) = seed_two_places(&mut conn);

        let places_repo = SqlitePlaceRepository;
        let movements_repo = SqlitePlaceMovementsRepository;

        let tx = conn.transaction().expect("tx");
        movements_repo
            .record_movement_if_applicable(
                &tx,
                &places_repo,
                MovementEntityKind::Device,
                1,
                Some(place_a),
                Some(place_a),
                MovementSource::Manual,
                None,
                None,
                None,
                NOW,
            )
            .expect("record (no-op)");
        tx.commit().expect("commit");

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM place_movements", [], |r| r.get(0))
            .expect("count");
        assert_eq!(count, 0, "unchanged place_id must insert ZERO rows (D-04)");
    })
    .await
    .expect("test timed out");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn record_movement_skips_on_first_assignment_from_null() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (mut conn, _dir) = test_db();
        let (_place_a, place_b) = seed_two_places(&mut conn);

        let places_repo = SqlitePlaceRepository;
        let movements_repo = SqlitePlaceMovementsRepository;

        let tx = conn.transaction().expect("tx");
        movements_repo
            .record_movement_if_applicable(
                &tx,
                &places_repo,
                MovementEntityKind::Device,
                1,
                None,
                Some(place_b),
                MovementSource::Manual,
                None,
                None,
                None,
                NOW,
            )
            .expect("record (first assignment)");
        tx.commit().expect("commit");

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM place_movements", [], |r| r.get(0))
            .expect("count");
        assert_eq!(
            count, 0,
            "NULL -> place (first assignment) must insert ZERO rows (D-06)"
        );
    })
    .await
    .expect("test timed out");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn record_movement_skips_when_cleared_to_null() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (mut conn, _dir) = test_db();
        let (place_a, _place_b) = seed_two_places(&mut conn);

        let places_repo = SqlitePlaceRepository;
        let movements_repo = SqlitePlaceMovementsRepository;

        let tx = conn.transaction().expect("tx");
        movements_repo
            .record_movement_if_applicable(
                &tx,
                &places_repo,
                MovementEntityKind::Device,
                1,
                Some(place_a),
                None,
                MovementSource::Manual,
                None,
                None,
                None,
                NOW,
            )
            .expect("record (cleared)");
        tx.commit().expect("commit");

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM place_movements", [], |r| r.get(0))
            .expect("count");
        assert_eq!(
            count, 0,
            "place -> NULL (cleared) must insert ZERO rows (D-06)"
        );
    })
    .await
    .expect("test timed out");
}

// ---------------------------------------------------------------------------
// Task 1: real insert — one row, snapshots populated (D-09/D-10)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn record_movement_inserts_one_row_with_path_and_actor_snapshots() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (mut conn, _dir) = test_db();
        let (place_a, place_b) = seed_two_places(&mut conn);
        let user_id = seed_user(&conn, "Иванов И.И.");

        let places_repo = SqlitePlaceRepository;
        let movements_repo = SqlitePlaceMovementsRepository;

        let tx = conn.transaction().expect("tx");
        movements_repo
            .record_movement_if_applicable(
                &tx,
                &places_repo,
                MovementEntityKind::Device,
                42,
                Some(place_a),
                Some(place_b),
                MovementSource::Manual,
                Some("перемещено вручную"),
                None,
                Some(user_id),
                NOW,
            )
            .expect("record (real move)");
        tx.commit().expect("commit");

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM place_movements", [], |r| r.get(0))
            .expect("count");
        assert_eq!(
            count, 1,
            "an actual move between two places inserts exactly ONE row"
        );

        let (from_path, to_path, source, act_id, actor_name): (
            String,
            String,
            String,
            Option<i64>,
            Option<String>,
        ) = conn
            .query_row(
                "SELECT from_place_path, to_place_path, source, act_id, actor_name_snapshot \
                   FROM place_movements LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .expect("select inserted row");

        assert_eq!(from_path, "Здание А");
        assert_eq!(to_path, "Здание Б");
        assert_eq!(source, MovementSource::Manual.as_str());
        assert_eq!(act_id, None);
        assert_eq!(actor_name.as_deref(), Some("Иванов И.И."));
    })
    .await
    .expect("test timed out");
}

// ---------------------------------------------------------------------------
// Task 1: delete_by_act_id_in_tx scoping (D-03)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn delete_by_act_id_removes_only_matching_rows() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (mut conn, _dir) = test_db();
        let (place_a, place_b) = seed_two_places(&mut conn);

        let act_7 = seed_act(&conn, 7);
        let act_99 = seed_act(&conn, 99);

        let places_repo = SqlitePlaceRepository;
        let movements_repo = SqlitePlaceMovementsRepository;

        // Two rows tied to act_id = act_7.
        {
            let tx = conn.transaction().expect("tx");
            movements_repo
                .record_movement_if_applicable(
                    &tx,
                    &places_repo,
                    MovementEntityKind::Device,
                    1,
                    Some(place_a),
                    Some(place_b),
                    MovementSource::Act,
                    None,
                    Some(act_7),
                    None,
                    NOW,
                )
                .expect("record act-7 row 1");
            movements_repo
                .record_movement_if_applicable(
                    &tx,
                    &places_repo,
                    MovementEntityKind::Device,
                    2,
                    Some(place_a),
                    Some(place_b),
                    MovementSource::Act,
                    None,
                    Some(act_7),
                    None,
                    NOW + 1,
                )
                .expect("record act-7 row 2");
            // One row tied to a DIFFERENT act_id — must survive.
            movements_repo
                .record_movement_if_applicable(
                    &tx,
                    &places_repo,
                    MovementEntityKind::Device,
                    3,
                    Some(place_a),
                    Some(place_b),
                    MovementSource::Act,
                    None,
                    Some(act_99),
                    None,
                    NOW + 2,
                )
                .expect("record act-99 row");
            // One row with act_id = NULL (manual move) — must also survive.
            movements_repo
                .record_movement_if_applicable(
                    &tx,
                    &places_repo,
                    MovementEntityKind::Device,
                    4,
                    Some(place_a),
                    Some(place_b),
                    MovementSource::Manual,
                    None,
                    None,
                    None,
                    NOW + 3,
                )
                .expect("record manual row (act_id NULL)");
            tx.commit().expect("commit");
        }

        {
            let tx = conn.transaction().expect("tx");
            let deleted = movements_repo
                .delete_by_act_id_in_tx(&tx, act_7)
                .expect("delete by act_id");
            assert_eq!(
                deleted, 2,
                "must delete exactly the 2 rows tied to act_id=7"
            );
            tx.commit().expect("commit");
        }

        let remaining: i64 = conn
            .query_row("SELECT COUNT(*) FROM place_movements", [], |r| r.get(0))
            .expect("count remaining");
        assert_eq!(
            remaining, 2,
            "the act-99 row and the act_id-NULL row must remain untouched"
        );
    })
    .await
    .expect("test timed out");
}

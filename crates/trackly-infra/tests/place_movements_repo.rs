//! Integration tests: `SqlitePlaceMovementsRepository` (Phase 40 Plan 05, HST-01/02/03).
//!
//! Covers the write-side guard (D-04/D-06), the real insert path (D-09/D-10 snapshots),
//! the act-scoped delete (D-03), and the read-side ordering (D-20).
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

// ---------------------------------------------------------------------------
// UAT-2: forget_act_entity_rows_in_tx — строки акта по одной сущности
// ---------------------------------------------------------------------------

fn record(
    tx: &rusqlite::Transaction<'_>,
    entity_id: i64,
    from: i64,
    to: i64,
    act_id: Option<i64>,
    at: i64,
) -> i64 {
    SqlitePlaceMovementsRepository
        .record_movement_if_applicable(
            tx,
            &SqlitePlaceRepository,
            MovementEntityKind::Device,
            entity_id,
            Some(from),
            Some(to),
            if act_id.is_some() {
                MovementSource::Act
            } else {
                MovementSource::Manual
            },
            None,
            act_id,
            None,
            at,
        )
        .expect("record")
        .expect("row written")
}

fn rows_of(conn: &Connection, entity_id: i64) -> Vec<(i64, i64, Option<i64>)> {
    let mut stmt = conn
        .prepare(
            "SELECT from_place_id, to_place_id, act_id FROM place_movements \
             WHERE entity_type = 'device' AND entity_id = ?1 ORDER BY id",
        )
        .expect("prepare");
    stmt.query_map(params![entity_id], |r| {
        Ok((r.get(0)?, r.get(1)?, r.get(2)?))
    })
    .expect("query")
    .collect::<Result<Vec<_>, _>>()
    .expect("rows")
}

/// Без перемещения после акта: строки акта по сущности удалены, чужие строки целы, место
/// возврата — «откуда» забытой строки (возврат туда перемещением не является).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn forget_act_entity_rows_without_later_move() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (mut conn, _dir) = test_db();
        let (place_a, place_b) = seed_two_places(&mut conn);
        let act = seed_act(&conn, 11);
        let other = seed_act(&conn, 12);
        let tx = conn.transaction().expect("tx");
        // строка другого акта — РАНЬШЕ строки забываемого акта (не преемник)
        record(&tx, 1, place_b, place_a, Some(other), NOW - 10);
        record(&tx, 1, place_a, place_b, Some(act), NOW);
        record(&tx, 2, place_a, place_b, Some(act), NOW);
        let out = SqlitePlaceMovementsRepository
            .forget_act_entity_rows_in_tx(&tx, act, MovementEntityKind::Device, 1)
            .expect("forget");
        tx.commit().expect("commit");
        assert_eq!(out.deleted, 1);
        assert!(!out.moved_after);
        assert_eq!(out.first_from_place_id, Some(place_a));
        assert_eq!(out.return_from_place(Some(place_b)), Some(place_a));
        assert_eq!(rows_of(&conn, 1), vec![(place_b, place_a, Some(other))]);
        assert_eq!(rows_of(&conn, 2), vec![(place_a, place_b, Some(act))]);
    })
    .await
    .expect("test timed out");
}

/// Перемещение после акта: строка акта удалена, ближайший преемник получил её «откуда»,
/// итог сообщает настоящее изменение, место возврата — текущее.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn forget_act_entity_rows_rebases_later_move() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (mut conn, _dir) = test_db();
        let (place_a, place_b) = seed_two_places(&mut conn);
        let place_c = SqlitePlaceRepository
            .create(
                &mut conn,
                &new_place(None, PlaceKind::Building, "Здание В"),
                NOW,
            )
            .expect("place C");
        let act = seed_act(&conn, 13);
        let tx = conn.transaction().expect("tx");
        record(&tx, 1, place_a, place_b, Some(act), NOW);
        record(&tx, 1, place_b, place_c, None, NOW + 1);
        let out = SqlitePlaceMovementsRepository
            .forget_act_entity_rows_in_tx(&tx, act, MovementEntityKind::Device, 1)
            .expect("forget");
        tx.commit().expect("commit");
        assert_eq!(out.deleted, 1);
        assert!(out.moved_after);
        assert_eq!(out.return_from_place(Some(place_c)), Some(place_c));
        assert_eq!(rows_of(&conn, 1), vec![(place_a, place_c, None)]);
    })
    .await
    .expect("test timed out");
}

/// Строк акта по сущности нет: ничего не удаляется, место возврата — текущее.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn forget_act_entity_rows_noop_without_act_rows() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (mut conn, _dir) = test_db();
        let (place_a, place_b) = seed_two_places(&mut conn);
        let act = seed_act(&conn, 14);
        let tx = conn.transaction().expect("tx");
        record(&tx, 1, place_a, place_b, None, NOW);
        let out = SqlitePlaceMovementsRepository
            .forget_act_entity_rows_in_tx(&tx, act, MovementEntityKind::Device, 1)
            .expect("forget");
        tx.commit().expect("commit");
        assert_eq!(out, Default::default());
        assert_eq!(out.return_from_place(Some(place_b)), Some(place_b));
        assert_eq!(rows_of(&conn, 1), vec![(place_a, place_b, None)]);
    })
    .await
    .expect("test timed out");
}

/// UAT-3: «перемещали после акта» — есть строка другого акта или без акта позже первой
/// строки каскада; строки самого каскада (акт и его возврат) поздними не считаются.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn entities_moved_after_acts_counts_only_foreign_later_rows() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (mut conn, _dir) = test_db();
        let (place_a, place_b) = seed_two_places(&mut conn);
        let act = seed_act(&conn, 15);
        let ret = seed_act(&conn, 16);
        let tx = conn.transaction().expect("tx");
        // 1: акт, затем ручное — перемещали после акта.
        let first = record(&tx, 1, place_a, place_b, Some(act), NOW);
        record(&tx, 1, place_b, place_a, None, NOW + 1);
        // 2: акт, затем возврат того же каскада — не перемещали.
        record(&tx, 2, place_a, place_b, Some(act), NOW);
        record(&tx, 2, place_b, place_a, Some(ret), NOW + 1);
        // 3: ручное ДО акта, затем акт — не перемещали после.
        record(&tx, 3, place_b, place_a, None, NOW);
        record(&tx, 3, place_a, place_b, Some(act), NOW + 1);
        let repo = SqlitePlaceMovementsRepository;
        let got = repo
            .entities_moved_after_acts_in_tx(&tx, MovementEntityKind::Device, &[act, ret])
            .expect("moved after");
        assert_eq!(got, vec![(1, first)]);
        assert!(repo
            .entities_moved_after_acts_in_tx(&tx, MovementEntityKind::Device, &[])
            .expect("empty")
            .is_empty());
        assert_eq!(
            repo.journal_end_place_in_tx(&tx, MovementEntityKind::Device, 1)
                .expect("end"),
            Some(place_a)
        );
        assert_eq!(
            repo.journal_end_place_in_tx(&tx, MovementEntityKind::Device, 99)
                .expect("end"),
            None
        );
    })
    .await
    .expect("test timed out");
}

/// UAT-3 «стереть»: удаляются строки сущности позже `after_id`, кроме строк каскада; чужие
/// сущности и более ранние строки целы; возвращаются id удалённых.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn erase_rows_after_keeps_cascade_and_earlier_rows() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (mut conn, _dir) = test_db();
        let (place_a, place_b) = seed_two_places(&mut conn);
        let act = seed_act(&conn, 17);
        let tx = conn.transaction().expect("tx");
        record(&tx, 1, place_b, place_a, None, NOW - 1);
        let first = record(&tx, 1, place_a, place_b, Some(act), NOW);
        let manual = record(&tx, 1, place_b, place_a, None, NOW + 1);
        record(&tx, 1, place_a, place_b, Some(act), NOW + 2);
        record(&tx, 2, place_a, place_b, None, NOW + 3);
        let erased = SqlitePlaceMovementsRepository
            .erase_rows_after_in_tx(&tx, MovementEntityKind::Device, 1, first, &[act])
            .expect("erase");
        tx.commit().expect("commit");
        assert_eq!(erased, vec![manual]);
        assert_eq!(
            rows_of(&conn, 1),
            vec![
                (place_b, place_a, None),
                (place_a, place_b, Some(act)),
                (place_a, place_b, Some(act))
            ]
        );
        assert_eq!(rows_of(&conn, 2), vec![(place_a, place_b, None)]);
    })
    .await
    .expect("test timed out");
}

// ---------------------------------------------------------------------------
// Task 2: get_history — newest-first (D-20), empty Vec on no rows, no LIMIT
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn place_movements_history_order_newest_first() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (mut conn, _dir) = test_db();
        let (place_a, place_b) = seed_two_places(&mut conn);

        let places_repo = SqlitePlaceRepository;
        let movements_repo = SqlitePlaceMovementsRepository;

        // Empty history for an entity with zero rows.
        let empty = movements_repo
            .get_history(&conn, "device", 999)
            .expect("get_history (empty)");
        assert!(
            empty.is_empty(),
            "no rows must return an empty Vec, not an error"
        );

        // Seed 3 movement rows for the same (entity_type, entity_id) at increasing
        // created_at_utc — alternate from/to so every call is a real (reportable) move.
        {
            let tx = conn.transaction().expect("tx");
            for i in 0..3 {
                let (from, to) = if i % 2 == 0 {
                    (place_a, place_b)
                } else {
                    (place_b, place_a)
                };
                movements_repo
                    .record_movement_if_applicable(
                        &tx,
                        &places_repo,
                        MovementEntityKind::Device,
                        7,
                        Some(from),
                        Some(to),
                        MovementSource::Manual,
                        None,
                        None,
                        None,
                        NOW + i,
                    )
                    .expect("record seed row");
            }
            tx.commit().expect("commit");
        }

        let history = movements_repo
            .get_history(&conn, "device", 7)
            .expect("get_history");
        assert_eq!(history.len(), 3, "all 3 seeded rows must come back");
        assert!(
            history[0].created_at_utc >= history[1].created_at_utc
                && history[1].created_at_utc >= history[2].created_at_utc,
            "rows must be ordered newest-first (D-20)"
        );
        assert_eq!(history[0].created_at_utc, NOW + 2, "newest row first");
        assert_eq!(history[2].created_at_utc, NOW, "oldest row last");

        // No LIMIT — seed 25 more rows for a different entity and confirm all come back.
        {
            let tx = conn.transaction().expect("tx");
            for i in 0..25 {
                let (from, to) = if i % 2 == 0 {
                    (place_a, place_b)
                } else {
                    (place_b, place_a)
                };
                movements_repo
                    .record_movement_if_applicable(
                        &tx,
                        &places_repo,
                        MovementEntityKind::Device,
                        8,
                        Some(from),
                        Some(to),
                        MovementSource::Manual,
                        None,
                        None,
                        None,
                        NOW + i,
                    )
                    .expect("record unpaginated seed row");
            }
            tx.commit().expect("commit");
        }

        let unpaginated = movements_repo
            .get_history(&conn, "device", 8)
            .expect("get_history (unpaginated)");
        assert_eq!(
            unpaginated.len(),
            25,
            "get_history must never apply a LIMIT (D-20 — no pagination)"
        );
    })
    .await
    .expect("test timed out");
}

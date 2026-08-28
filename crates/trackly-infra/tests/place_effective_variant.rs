//! Integration tests: `place_effective_variant` view (V039, Phase 39.1 Plan 01).
//!
//! Each test is wrapped in `tokio::time::timeout(30s)` (mirrors
//! `crates/trackly-infra/tests/places_crud.rs`'s Linux-CI deadlock defense
//! convention), even though the queries exercised here are synchronous.
//!
//! There is no service-layer method for setting `path_variant_override` yet
//! (that lands in Plan 07) — these tests set it and the org-wide
//! `app_settings.place_path_variant` default directly via raw SQL, because
//! the behavior under test is the VIEW, not a mutation API surface.
//! Place creation/movement goes through the real `PlaceRepository` trait
//! (`SqlitePlaceRepository`), same as `places_crud.rs`.
//!
//! Only invented place names ("Здание А", "2 этаж", "214") — never real
//! organization data, per the project's hard privacy constraint.

use std::time::Duration;

use trackly_core::domain::places::{PlaceKind, PlaceNew};
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

fn set_override(conn: &rusqlite::Connection, place_id: i64, variant: Option<&str>) {
    conn.execute(
        "UPDATE places SET path_variant_override = ?1 WHERE id = ?2",
        rusqlite::params![variant, place_id],
    )
    .expect("set path_variant_override");
}

fn set_org_default(conn: &rusqlite::Connection, variant: &str) {
    conn.execute(
        "UPDATE app_settings SET value = ?1 WHERE key = 'place_path_variant'",
        rusqlite::params![variant],
    )
    .expect("set org default");
}

fn effective_variant(conn: &rusqlite::Connection, place_id: i64) -> String {
    conn.query_row(
        "SELECT effective_variant FROM place_effective_variant WHERE place_id = ?1",
        [place_id],
        |r| r.get::<_, String>(0),
    )
    .expect("read effective_variant")
}

// ---------------------------------------------------------------------------
// Root, no override -> organization default (D-06/D-23: fresh DB = 'ends')
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn no_parent_no_override_falls_back_to_org_default() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (mut conn, _dir) = test_db();
        let repo = SqlitePlaceRepository;

        let root_id = repo
            .create(
                &mut conn,
                &new_place(None, PlaceKind::Building, "Здание А"),
                NOW,
            )
            .expect("create root");

        assert_eq!(effective_variant(&conn, root_id), "ends");
    })
    .await
    .expect("test timed out");
}

// ---------------------------------------------------------------------------
// Explicit override wins regardless of parents / org default
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn explicit_override_wins_regardless_of_org_default() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (mut conn, _dir) = test_db();
        let repo = SqlitePlaceRepository;

        let root_id = repo
            .create(
                &mut conn,
                &new_place(None, PlaceKind::Building, "Здание А"),
                NOW,
            )
            .expect("create root");
        set_override(&conn, root_id, Some("last_two"));

        // Org default stays 'ends' — override must still win.
        assert_eq!(effective_variant(&conn, root_id), "last_two");

        // Even if org default changes too, explicit override still wins.
        set_org_default(&conn, "last");
        assert_eq!(effective_variant(&conn, root_id), "last_two");
    })
    .await
    .expect("test timed out");
}

// ---------------------------------------------------------------------------
// 3-level chain: override only on root, child + grandchild inherit it (D-03)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn descendants_inherit_ancestor_override_through_chain() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (mut conn, _dir) = test_db();
        let repo = SqlitePlaceRepository;

        let root_id = repo
            .create(
                &mut conn,
                &new_place(None, PlaceKind::Building, "Здание А"),
                NOW,
            )
            .expect("create root");
        let child_id = repo
            .create(
                &mut conn,
                &new_place(Some(root_id), PlaceKind::Floor, "2 этаж"),
                NOW,
            )
            .expect("create child");
        let grandchild_id = repo
            .create(
                &mut conn,
                &new_place(Some(child_id), PlaceKind::Room, "214"),
                NOW,
            )
            .expect("create grandchild");

        set_override(&conn, root_id, Some("last"));

        assert_eq!(effective_variant(&conn, root_id), "last");
        assert_eq!(effective_variant(&conn, child_id), "last");
        assert_eq!(effective_variant(&conn, grandchild_id), "last");
    })
    .await
    .expect("test timed out");
}

// ---------------------------------------------------------------------------
// Dynamic inheritance on move (D-03): moving to top-level drops inherited
// override, immediately visible on next SELECT (no reindex call).
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn moving_inheriting_place_to_top_level_switches_to_org_default_immediately() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (mut conn, _dir) = test_db();
        let repo = SqlitePlaceRepository;

        let b_id = repo
            .create(
                &mut conn,
                &new_place(None, PlaceKind::Building, "Здание А"),
                NOW,
            )
            .expect("create B");
        set_override(&conn, b_id, Some("ends"));

        let a_id = repo
            .create(
                &mut conn,
                &new_place(Some(b_id), PlaceKind::Floor, "2 этаж"),
                NOW,
            )
            .expect("create A as child of B (NULL override)");

        // Before move: A inherits B's override.
        assert_eq!(effective_variant(&conn, a_id), "ends");

        // Move A to top-level (parent_id = NULL) — org default not overridden
        // there, so A must fall back to it.
        repo.move_node(&mut conn, a_id, None, 1, NOW + 1)
            .expect("move A to top level");

        assert_eq!(
            effective_variant(&conn, a_id),
            "ends", // org default is still 'ends' at this point in the test
            "A must fall back to the org default immediately after losing its inheriting parent"
        );

        // Change org default — A (still NULL override, still top-level) must
        // see the new value on the very next read.
        set_org_default(&conn, "last_two");
        assert_eq!(effective_variant(&conn, a_id), "last_two");
    })
    .await
    .expect("test timed out");
}

// ---------------------------------------------------------------------------
// Override survives move, including move to top-level (D-04)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn override_is_not_reset_by_move_including_move_to_top_level() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (mut conn, _dir) = test_db();
        let repo = SqlitePlaceRepository;

        let old_parent_id = repo
            .create(
                &mut conn,
                &new_place(None, PlaceKind::Building, "Здание А"),
                NOW,
            )
            .expect("create old parent");
        let new_parent_id = repo
            .create(
                &mut conn,
                &new_place(None, PlaceKind::Building, "Здание Б"),
                NOW,
            )
            .expect("create new parent");
        let place_id = repo
            .create(
                &mut conn,
                &new_place(Some(old_parent_id), PlaceKind::Floor, "2 этаж"),
                NOW,
            )
            .expect("create place");
        set_override(&conn, place_id, Some("last"));

        // Move to a different parent — override must survive.
        repo.move_node(&mut conn, place_id, Some(new_parent_id), 1, NOW + 1)
            .expect("move to new parent");
        assert_eq!(effective_variant(&conn, place_id), "last");

        // Move to top-level — override must STILL survive (D-04's explicit
        // "including move to top level" clause).
        repo.move_node(&mut conn, place_id, None, 2, NOW + 2)
            .expect("move to top level");
        assert_eq!(effective_variant(&conn, place_id), "last");
    })
    .await
    .expect("test timed out");
}

// ---------------------------------------------------------------------------
// Org default change is visible immediately for every non-overridden place
// in the ancestor chain (no explicit "invalidation" step required)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn org_default_change_is_visible_immediately_for_non_overridden_places() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (mut conn, _dir) = test_db();
        let repo = SqlitePlaceRepository;

        let root_id = repo
            .create(
                &mut conn,
                &new_place(None, PlaceKind::Building, "Здание А"),
                NOW,
            )
            .expect("create root");
        let child_id = repo
            .create(
                &mut conn,
                &new_place(Some(root_id), PlaceKind::Floor, "2 этаж"),
                NOW,
            )
            .expect("create child");

        assert_eq!(effective_variant(&conn, root_id), "ends");
        assert_eq!(effective_variant(&conn, child_id), "ends");

        set_org_default(&conn, "last");

        assert_eq!(effective_variant(&conn, root_id), "last");
        assert_eq!(effective_variant(&conn, child_id), "last");
    })
    .await
    .expect("test timed out");
}

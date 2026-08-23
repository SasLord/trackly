//! Integration tests: `SqliteCartridgeRepository::search` place-path substring
//! matching (D-29/PLC-05, Phase 39 Plan 09 Task 4) — the cartridge-side sibling
//! of `devices_place_search.rs` (Plan 06 Task 6).
//!
//! Exercises the repository layer directly (`SqliteCartridgeRepository` +
//! `SqlitePlaceRepository`, domain types only) — no `CartridgeService`/DTO
//! layer involved, mirroring `places_crud.rs`'s and `devices_place_search.rs`'s
//! convention for the same reason (interface-first coverage, no service layer
//! needed to prove the SQL).
//!
//! Only invented place/cartridge data ("Здание А", "Корпус Б", "2 этаж") —
//! never real organization data, per the project's hard privacy constraint.

use std::time::Duration;

use rusqlite::Connection;
use trackly_core::domain::cartridges::{CartridgeFilter, CartridgeModelNew};
use trackly_core::domain::places::{PlaceKind, PlaceNew};
use trackly_core::ports::places::PlaceRepository;
use trackly_infra::repos::{SqliteCartridgeRepository, SqlitePlaceRepository};
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

/// Seed a cartridge model + one cartridge instance located at `place_id`
/// (or unplaced when `None`). Returns the new cartridge's id.
fn seed_cartridge(conn: &mut Connection, place_id: Option<i64>) -> i64 {
    let repo = SqliteCartridgeRepository;
    let tx = conn.transaction().expect("tx");
    let model_id = repo
        .insert_model_in_tx(
            &tx,
            &CartridgeModelNew {
                brand: "Pantum".to_string(),
                model: "TL-5120X".to_string(),
                kind_id: 1,
                color: Some("Чёрный".to_string()),
                notes: None,
            },
            NOW,
        )
        .expect("insert model");
    let (code, _) = SqliteCartridgeRepository::assign_code_in_tx(&tx, None, 1, NOW)
        .expect("assign code");
    let cart_id = repo
        .insert_cartridge_in_tx(&tx, &code, model_id, 1, None, place_id, None, None, NOW)
        .expect("insert cartridge");
    tx.commit().expect("commit");
    cart_id
}

fn filter() -> CartridgeFilter {
    CartridgeFilter::default()
}

// ---------------------------------------------------------------------------
// search finds a cartridge purely by its place's name (no intrinsic-field match)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_matches_by_place_path_substring() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (mut conn, _dir) = test_db();
        let place_repo = SqlitePlaceRepository;
        let cart_repo = SqliteCartridgeRepository;

        let building_id = place_repo
            .create(&mut conn, &new_place(None, PlaceKind::Building, "Здание А"), NOW)
            .expect("create place");

        seed_cartridge(&mut conn, Some(building_id));

        // "здание" matches no intrinsic cartridge field (code/holder_name/
        // brand/model) — only the place path.
        let rows = cart_repo
            .search(&conn, "здание", &filter())
            .expect("search");

        assert_eq!(
            rows.len(),
            1,
            "должен найти 1 картридж по подстроке пути места"
        );
        assert_eq!(
            rows[0].full_path.as_deref(),
            Some("Здание А"),
            "full_path должен быть заполнен для найденного картриджа"
        );
    })
    .await
    .expect("search_matches_by_place_path_substring exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// search matches a cartridge located in a DESCENDANT of the matching place
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_matches_cartridge_in_descendant_place() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (mut conn, _dir) = test_db();
        let place_repo = SqlitePlaceRepository;
        let cart_repo = SqliteCartridgeRepository;

        let building_id = place_repo
            .create(&mut conn, &new_place(None, PlaceKind::Building, "Здание А"), NOW)
            .expect("create building");
        let floor_id = place_repo
            .create(
                &mut conn,
                &new_place(Some(building_id), PlaceKind::Floor, "2 этаж"),
                NOW,
            )
            .expect("create floor");
        let room_id = place_repo
            .create(
                &mut conn,
                &new_place(Some(floor_id), PlaceKind::Room, "214"),
                NOW,
            )
            .expect("create room");

        seed_cartridge(&mut conn, Some(room_id));

        // A query matching the root building's name must also surface a
        // cartridge located several levels deeper in that subtree.
        let rows = cart_repo
            .search(&conn, "здание", &filter())
            .expect("search");

        assert_eq!(
            rows.len(),
            1,
            "картридж в потомке совпавшего места тоже должен найтись"
        );
        assert_eq!(
            rows[0].full_path.as_deref(),
            Some("Здание А / 2 этаж / 214")
        );
    })
    .await
    .expect("search_matches_cartridge_in_descendant_place exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// search reflects a place rename immediately — no reindex step
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_reflects_place_rename_without_reindex() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (mut conn, _dir) = test_db();
        let place_repo = SqlitePlaceRepository;
        let cart_repo = SqliteCartridgeRepository;

        let building_id = place_repo
            .create(&mut conn, &new_place(None, PlaceKind::Building, "Здание А"), NOW)
            .expect("create place");
        seed_cartridge(&mut conn, Some(building_id));

        // Sanity: old name matches before the rename.
        let rows_before = cart_repo
            .search(&conn, "здание", &filter())
            .expect("search before rename");
        assert_eq!(rows_before.len(), 1);

        // Rename the place (place_full_paths is a live VIEW — no cache to
        // invalidate, no reindex call of any kind).
        let place = place_repo.get(&conn, building_id).expect("get place");
        place_repo
            .rename(&mut conn, building_id, "Корпус Б", place.version, NOW)
            .expect("rename place");

        let rows_old_name = cart_repo
            .search(&conn, "здание", &filter())
            .expect("search old name after rename");
        assert!(
            rows_old_name.is_empty(),
            "старое имя места не должно больше находить картридж"
        );

        let rows_new_name = cart_repo
            .search(&conn, "корпус", &filter())
            .expect("search new name after rename");
        assert_eq!(
            rows_new_name.len(),
            1,
            "новое имя места должно немедленно находить картридж"
        );
    })
    .await
    .expect("search_reflects_place_rename_without_reindex exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// search("---", ...) — punctuation-only query (no alphanumeric token) still
// returns Ok via the existing LIKE-only fallback path (WR-01 regression);
// confirm the new place_hits branch doesn't break this fallback.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_punctuation_only_query_returns_ok() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (mut conn, _dir) = test_db();
        let place_repo = SqlitePlaceRepository;
        let cart_repo = SqliteCartridgeRepository;

        let building_id = place_repo
            .create(&mut conn, &new_place(None, PlaceKind::Building, "Здание А"), NOW)
            .expect("create place");
        seed_cartridge(&mut conn, Some(building_id));

        let result = cart_repo.search(&conn, "---", &filter());
        assert!(
            result.is_ok(),
            "search should return Ok for punctuation-only query, got: {result:?}"
        );
        assert!(result.expect("ok").is_empty());
    })
    .await
    .expect("search_punctuation_only_query_returns_ok exceeded 30 s budget");
}

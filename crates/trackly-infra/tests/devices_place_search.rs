//! Integration tests: `SqliteDeviceRepository::search_fts` place-path substring
//! matching (D-29/PLC-05, Phase 39 Plan 06 Task 6).
//!
//! Exercises the repository layer directly (`SqliteDeviceRepository` +
//! `SqlitePlaceRepository`, domain types only) — no `DeviceService`/DTO layer
//! involved, mirroring `places_crud.rs`'s convention for the same reason
//! (interface-first coverage, no service layer needed to prove the SQL).
//!
//! Only invented place/device data ("Здание А", "Корпус Б", "2 этаж") — never
//! real organization data, per the project's hard privacy constraint.

use std::time::Duration;

use trackly_core::domain::devices::{DeviceNew, Pagination};
use trackly_core::domain::places::{PlaceKind, PlaceNew};
use trackly_core::ports::devices::DeviceRepository;
use trackly_core::ports::places::PlaceRepository;
use trackly_infra::repos::{SqliteDeviceRepository, SqlitePlaceRepository};
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

fn new_device(name: &str, place_id: Option<i64>) -> DeviceNew {
    DeviceNew {
        type_id: 1,
        name: name.to_string(),
        inventory_no: None,
        serial_no: None,
        model: None,
        specs: None,
        kit: None,
        state: None,
        place_id,
        status_id: 1,
    }
}

fn page() -> Pagination {
    Pagination {
        offset: 0,
        limit: 50,
    }
}

// ---------------------------------------------------------------------------
// search_fts finds a device purely by its place's name (no intrinsic-field match)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_fts_matches_by_place_path_substring() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (mut conn, _dir) = test_db();
        let place_repo = SqlitePlaceRepository;
        let device_repo = SqliteDeviceRepository;

        let building_id = place_repo
            .create(
                &mut conn,
                &new_place(None, PlaceKind::Building, "Здание А"),
                NOW,
            )
            .expect("create place");

        device_repo
            .create(
                &mut conn,
                &new_device("Ноутбук Lenovo", Some(building_id)),
                NOW,
            )
            .expect("create device");

        // "здание" matches no intrinsic device field (name/inventory/serial/model)
        // — only the place path.
        let (rows, total) = device_repo
            .search_fts(&conn, "здание", &page())
            .expect("search_fts");

        assert_eq!(
            total, 1,
            "должен найти 1 устройство по подстроке пути места"
        );
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "Ноутбук Lenovo");
        assert_eq!(
            rows[0].full_path.as_deref(),
            Some("Здание А"),
            "full_path должен быть заполнен для найденного устройства"
        );
    })
    .await
    .expect("search_fts_matches_by_place_path_substring exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// search_fts matches a device located in a DESCENDANT of the matching place
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_fts_matches_device_in_descendant_place() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (mut conn, _dir) = test_db();
        let place_repo = SqlitePlaceRepository;
        let device_repo = SqliteDeviceRepository;

        let building_id = place_repo
            .create(
                &mut conn,
                &new_place(None, PlaceKind::Building, "Здание А"),
                NOW,
            )
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

        device_repo
            .create(&mut conn, &new_device("Принтер HP", Some(room_id)), NOW)
            .expect("create device");

        // A query matching the root building's name must also surface a device
        // located several levels deeper in that subtree — the substring check
        // runs against the whole resolved full_path ("Здание А / 2 этаж / 214"),
        // not just the device's immediate place.
        let (rows, total) = device_repo
            .search_fts(&conn, "здание", &page())
            .expect("search_fts");

        assert_eq!(
            total, 1,
            "устройство в потомке совпавшего места тоже должно найтись"
        );
        assert_eq!(rows[0].name, "Принтер HP");
        assert_eq!(
            rows[0].full_path.as_deref(),
            Some("Здание А / 2 этаж / 214")
        );
    })
    .await
    .expect("search_fts_matches_device_in_descendant_place exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// search_fts reflects a place rename immediately — no reindex step
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_fts_reflects_place_rename_without_reindex() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (mut conn, _dir) = test_db();
        let place_repo = SqlitePlaceRepository;
        let device_repo = SqliteDeviceRepository;

        let building_id = place_repo
            .create(
                &mut conn,
                &new_place(None, PlaceKind::Building, "Здание А"),
                NOW,
            )
            .expect("create place");
        device_repo
            .create(
                &mut conn,
                &new_device("Сканер Canon", Some(building_id)),
                NOW,
            )
            .expect("create device");

        // Sanity: old name matches before the rename.
        let (_rows, total_before) = device_repo
            .search_fts(&conn, "здание", &page())
            .expect("search_fts before rename");
        assert_eq!(total_before, 1);

        // Rename the place (place_full_paths is a live VIEW — no cache to
        // invalidate, no reindex call of any kind).
        let place = place_repo.get(&conn, building_id).expect("get place");
        place_repo
            .rename(&mut conn, building_id, "Корпус Б", place.version, NOW)
            .expect("rename place");

        let (_rows, total_old_name) = device_repo
            .search_fts(&conn, "здание", &page())
            .expect("search_fts old name after rename");
        assert_eq!(
            total_old_name, 0,
            "старое имя места не должно больше находить устройство"
        );

        let (rows_new_name, total_new_name) = device_repo
            .search_fts(&conn, "корпус", &page())
            .expect("search_fts new name after rename");
        assert_eq!(
            total_new_name, 1,
            "новое имя места должно немедленно находить устройство"
        );
        assert_eq!(rows_new_name[0].name, "Сканер Canon");
    })
    .await
    .expect("search_fts_reflects_place_rename_without_reindex exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// search_fts: a place-only match still succeeds when the FTS5 side of the
// query has zero hits (the OR-combined CTEs, not just the FTS5 branch, must
// decide the result set; the LEFT JOIN to fts_hits must not drop or error on
// a place-only row that has no corresponding fts_hits row).
// ---------------------------------------------------------------------------

/// Note on this test's design (deviation from the plan's literal scenario —
/// see 39-06-SUMMARY.md "Deviations"): `build_fts_query` (Plan 04, unrelated
/// file/plan, out of this task's scope) never actually sanitizes non-empty,
/// non-whitespace, non-null-only input down to an empty string — it only
/// strips NUL bytes and escapes `"`, it does NOT strip punctuation. Verified
/// with a standalone `sqlite3` harness against the real migration chain
/// (crate-wide compile is still blocked by Plans 07/09/10's pre-existing
/// errors, `cargo test` cannot run yet — same blocker 39-04-SUMMARY.md hit).
/// A punctuation-only query like `"!!! здание ???"` does NOT sanitize to an
/// empty `match_expr` — it becomes `"!!!"* "здание"* "???"*`, a non-empty
/// FTS5 query — and critically, the RAW multi-word string is also not a
/// literal substring of any real place path, so it doesn't drive a place hit
/// either. The genuinely reachable manifestation of "found via place path
/// even though the FTS5 side contributes nothing" is a query that (a) IS a
/// literal substring of some place's `full_path`, and (b) tokenizes to a
/// non-empty FTS5 `match_expr` that matches zero devices by intrinsic field —
/// exercised below with `"2 этаж"`.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_fts_place_only_match_when_fts5_side_has_zero_hits() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (mut conn, _dir) = test_db();
        let place_repo = SqlitePlaceRepository;
        let device_repo = SqliteDeviceRepository;

        let building_id = place_repo
            .create(
                &mut conn,
                &new_place(None, PlaceKind::Building, "Здание А"),
                NOW,
            )
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

        device_repo
            .create(&mut conn, &new_device("Принтер HP", Some(room_id)), NOW)
            .expect("create device");

        // "2 этаж" is a literal substring of "Здание А / 2 этаж / 214" (place
        // path) — but no device's intrinsic fields (name/inventory/serial/
        // model) contain "2" or "этаж", so the FTS5 MATCH side finds zero
        // rows for this device even though match_expr is non-empty.
        let (rows, total) = device_repo
            .search_fts(&conn, "2 этаж", &page())
            .expect("search_fts");

        assert_eq!(
            total, 1,
            "устройство должно найтись через place_hits, даже если fts_hits пуст для него"
        );
        assert_eq!(rows[0].name, "Принтер HP");
    })
    .await
    .expect("search_fts_place_only_match_when_fts5_side_has_zero_hits exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// search_fts: a genuinely empty query (both match_expr and place_ids empty)
// still returns nothing — locks in the exact boundary the moved early-return
// guards (`if !has_fts && !has_place`), preventing an empty search from
// degenerating into "every device that has any place" via an unconditional
// `full_path.contains("")` (always true for the empty substring).
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_fts_empty_query_returns_nothing_not_everything() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (mut conn, _dir) = test_db();
        let place_repo = SqlitePlaceRepository;
        let device_repo = SqliteDeviceRepository;

        let building_id = place_repo
            .create(
                &mut conn,
                &new_place(None, PlaceKind::Building, "Здание А"),
                NOW,
            )
            .expect("create place");
        device_repo
            .create(&mut conn, &new_device("Моноблок", Some(building_id)), NOW)
            .expect("create device");

        let (rows, total) = device_repo
            .search_fts(&conn, "", &page())
            .expect("search_fts empty query");
        assert_eq!(
            total, 0,
            "пустой запрос не должен возвращать все устройства с местом"
        );
        assert!(rows.is_empty());

        let (rows2, total2) = device_repo
            .search_fts(&conn, "   ", &page())
            .expect("search_fts whitespace-only query");
        assert_eq!(total2, 0);
        assert!(rows2.is_empty());
    })
    .await
    .expect("search_fts_empty_query_returns_nothing_not_everything exceeded 30 s budget");
}

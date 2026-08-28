//! Integration tests: `place_path_short` on `SqliteCartridgeRepository::list`
//! and `::search` (Phase 39.1 Plan 04, PLC-08, mirrors
//! `devices_place_path_short.rs` from Plan 03).
//!
//! Only invented place/cartridge data ("Здание А", "1 этаж", "1-05") — never
//! real organization data, per the project's hard privacy constraint.

use std::time::Duration;

use rusqlite::Connection;
use trackly_core::domain::cartridges::{CartridgeFilter, CartridgeModelNew, Pagination};
use trackly_core::domain::places::{PlaceKind, PlaceNew};
use trackly_core::ports::cartridges::CartridgeRepository;
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

fn page() -> Pagination {
    Pagination {
        offset: 0,
        limit: 50,
    }
}

fn seed_model(conn: &mut Connection) -> i64 {
    let tx = conn.transaction().expect("tx");
    let repo = SqliteCartridgeRepository;
    let id = repo
        .insert_model_in_tx(
            &tx,
            &CartridgeModelNew {
                brand: "Pantum".into(),
                model: "TL-5120X".into(),
                kind_id: 1,
                color: Some("Чёрный".into()),
                notes: None,
            },
            NOW,
        )
        .expect("insert model");
    tx.commit().expect("commit");
    id
}

fn create_cartridge(conn: &mut Connection, model_id: i64, place_id: Option<i64>) -> i64 {
    let tx = conn.transaction().expect("tx");
    let (code, _) = SqliteCartridgeRepository::assign_code_in_tx(&tx, None, 1, NOW).expect("code");
    let repo = SqliteCartridgeRepository;
    let id = repo
        .insert_cartridge_in_tx(&tx, &code, model_id, 1, Some(1), place_id, None, None, NOW)
        .expect("insert cartridge");
    tx.commit().expect("commit");
    id
}

/// `list()` on a cartridge in a 3-segment place, org default variant 'ends'
/// (V039 default) → `place_path_short` = "Здание А // 1-05" using the
/// V039-seeded default separator `' // '`.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn list_returns_shortened_path_for_ends_variant() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (mut conn, _dir) = test_db();
        let place_repo = SqlitePlaceRepository;
        let cartridge_repo = SqliteCartridgeRepository;

        let building = place_repo
            .create(
                &mut conn,
                &new_place(None, PlaceKind::Building, "Здание А"),
                NOW,
            )
            .expect("create building");
        let floor = place_repo
            .create(
                &mut conn,
                &new_place(Some(building), PlaceKind::Floor, "1 этаж"),
                NOW,
            )
            .expect("create floor");
        let room = place_repo
            .create(
                &mut conn,
                &new_place(Some(floor), PlaceKind::Room, "1-05"),
                NOW,
            )
            .expect("create room");

        let model_id = seed_model(&mut conn);
        create_cartridge(&mut conn, model_id, Some(room));

        let (rows, total) = cartridge_repo
            .list(&conn, &CartridgeFilter::default(), &page())
            .expect("list");

        assert_eq!(total, 1);
        let row = &rows[0];
        assert_eq!(
            row.full_path.as_deref(),
            Some("Здание А / 1 этаж / 1-05"),
            "full_path должен остаться нетронутым"
        );
        assert_eq!(
            row.place_path_short.as_deref(),
            Some("Здание А // 1-05"),
            "place_path_short должен использовать вариант 'ends' (умолчание \
             организации из V039) и разделитель ' // '"
        );
    })
    .await
    .expect("timeout");
}

/// Картридж без места (`place_id IS NULL`) — `place_path_short: None`,
/// мирроринг существующего поведения `full_path: None`.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn list_cartridge_without_place_has_no_short_path() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (mut conn, _dir) = test_db();
        let cartridge_repo = SqliteCartridgeRepository;

        let model_id = seed_model(&mut conn);
        create_cartridge(&mut conn, model_id, None);

        let (rows, total) = cartridge_repo
            .list(&conn, &CartridgeFilter::default(), &page())
            .expect("list");

        assert_eq!(total, 1);
        let row = &rows[0];
        assert_eq!(row.full_path, None);
        assert_eq!(row.place_path_short, None);
    })
    .await
    .expect("timeout");
}

/// Regression (code review CR-01): `search()` must shorten the path exactly like
/// `list()` does. `CartridgeListRow.svelte` renders `place_path_short` only, so
/// a bare `map_row` here blanked the «Место» column the moment the user typed
/// anything into the cartridge search box.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_returns_shortened_path_like_list() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (mut conn, _dir) = test_db();
        let place_repo = SqlitePlaceRepository;
        let cartridge_repo = SqliteCartridgeRepository;

        let building = place_repo
            .create(
                &mut conn,
                &new_place(None, PlaceKind::Building, "Здание А"),
                NOW,
            )
            .expect("create building");
        let floor = place_repo
            .create(
                &mut conn,
                &new_place(Some(building), PlaceKind::Floor, "1 этаж"),
                NOW,
            )
            .expect("create floor");
        let room = place_repo
            .create(
                &mut conn,
                &new_place(Some(floor), PlaceKind::Room, "1-05"),
                NOW,
            )
            .expect("create room");

        let model_id = seed_model(&mut conn);
        create_cartridge(&mut conn, model_id, Some(room));

        let rows = cartridge_repo
            .search(&conn, "Pantum", &CartridgeFilter::default())
            .expect("search");

        assert_eq!(rows.len(), 1, "поиск по бренду должен найти картридж");
        let row = &rows[0];
        assert_eq!(
            row.full_path.as_deref(),
            Some("Здание А / 1 этаж / 1-05"),
            "full_path должен остаться нетронутым и в поиске"
        );
        assert_eq!(
            row.place_path_short.as_deref(),
            Some("Здание А // 1-05"),
            "search() должен отдавать тот же сокращённый путь, что и list()"
        );
    })
    .await
    .expect("timeout");
}

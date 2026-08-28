//! Integration tests: `place_path_short` on `SqliteDeviceRepository::list`/
//! `search_fts` (Phase 39.1 Plan 03, PLC-08, D-17/D-19).
//!
//! Exercises the repository layer directly (`SqliteDeviceRepository` +
//! `SqlitePlaceRepository`, domain types only), same convention as
//! `devices_place_search.rs` — no `DeviceService`/DTO layer needed to prove
//! the SQL joins + formula.
//!
//! Only invented place/device data ("Здание А", "1 этаж", "1-05") — never
//! real organization data, per the project's hard privacy constraint.

use std::time::Duration;

use trackly_core::domain::devices::{DeviceFilter, DeviceNew, Pagination};
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

/// `list()` on a device in a 3-segment place, org default variant 'ends'
/// (V039 default) → `place_path_short` = "Здание А // 1-05" using the
/// V039-seeded default separator `' // '`.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn list_returns_shortened_path_for_ends_variant() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (mut conn, _dir) = test_db();
        let place_repo = SqlitePlaceRepository;
        let device_repo = SqliteDeviceRepository;

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

        device_repo
            .create(&mut conn, &new_device("Ноутбук", Some(room)), NOW)
            .expect("create device");

        let (rows, total) = device_repo
            .list(&conn, &DeviceFilter::default(), &page())
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

/// Устройство без места (`place_id IS NULL`) — `place_path_short: None`,
/// мирроринг существующего поведения `full_path: None`.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn list_device_without_place_has_no_short_path() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (mut conn, _dir) = test_db();
        let device_repo = SqliteDeviceRepository;

        device_repo
            .create(&mut conn, &new_device("Ноутбук без места", None), NOW)
            .expect("create device");

        let (rows, total) = device_repo
            .list(&conn, &DeviceFilter::default(), &page())
            .expect("list");

        assert_eq!(total, 1);
        let row = &rows[0];
        assert_eq!(row.full_path, None);
        assert_eq!(row.place_path_short, None);
    })
    .await
    .expect("timeout");
}

/// `search_fts()` на то же устройство и место возвращает ИДЕНТИЧНЫЙ
/// `place_path_short`, что и `list()` — единая формула на одном `place_id`.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_fts_matches_list_short_path_for_same_place() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (mut conn, _dir) = test_db();
        let place_repo = SqlitePlaceRepository;
        let device_repo = SqliteDeviceRepository;

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

        device_repo
            .create(
                &mut conn,
                &new_device("Уникальный Ноутбук Икс", Some(room)),
                NOW,
            )
            .expect("create device");

        let (list_rows, _) = device_repo
            .list(&conn, &DeviceFilter::default(), &page())
            .expect("list");
        let list_short = list_rows[0].place_path_short.clone();
        assert!(list_short.is_some());

        let (search_rows, search_total) = device_repo
            .search_fts(&conn, "Уникальный", &page())
            .expect("search_fts");
        assert_eq!(search_total, 1);
        assert_eq!(
            search_rows[0].place_path_short, list_short,
            "search_fts и list должны возвращать одинаковый place_path_short \
             для одного и того же place_id"
        );
    })
    .await
    .expect("timeout");
}

/// `list_grouped()` — обе SQL-ветки (`group_by_condition=false` и `=true`)
/// присоединяют `place_effective_variant` и вычисляют `place_path_short` по
/// той же формуле, что `list()`.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn list_grouped_returns_shortened_path_both_branches() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (mut conn, _dir) = test_db();
        let place_repo = SqlitePlaceRepository;
        let device_repo = SqliteDeviceRepository;

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

        device_repo
            .create(&mut conn, &new_device("Принтер Групповой", Some(room)), NOW)
            .expect("create device");

        // Branch 1: group_by_condition=false (sql_without_condition, DevicesPage).
        let groups_false = device_repo
            .list_grouped(&conn, &DeviceFilter::default(), &page())
            .expect("list_grouped false-branch");
        assert_eq!(groups_false.len(), 1);
        assert_eq!(
            groups_false[0].repr.place_path_short.as_deref(),
            Some("Здание А // 1-05"),
            "group_by_condition=false branch должна нести place_path_short"
        );

        // Branch 2: group_by_condition=true, no text filter
        // (sql_grouped_by_model_no_query).
        let groups_true = device_repo
            .list_grouped(
                &conn,
                &DeviceFilter {
                    group_by_condition: true,
                    ..Default::default()
                },
                &page(),
            )
            .expect("list_grouped true-branch");
        assert_eq!(groups_true.len(), 1);
        assert_eq!(
            groups_true[0].repr.place_path_short.as_deref(),
            Some("Здание А // 1-05"),
            "group_by_condition=true branch (без текстового фильтра) должна \
             нести place_path_short"
        );
    })
    .await
    .expect("timeout");
}

/// D-14: путь из 2 сегментов при варианте 'ends' (умолчание) ничего не
/// выбрасывает — `place_path_short` совпадает с `full_path`, разделитель
/// остаётся штатным `' / '`, а не `sep_ends`.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn list_two_segment_path_unchanged_by_ends_variant() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (mut conn, _dir) = test_db();
        let place_repo = SqlitePlaceRepository;
        let device_repo = SqliteDeviceRepository;

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

        device_repo
            .create(&mut conn, &new_device("Ноутбук", Some(floor)), NOW)
            .expect("create device");

        let (rows, _) = device_repo
            .list(&conn, &DeviceFilter::default(), &page())
            .expect("list");

        assert_eq!(
            rows[0].place_path_short.as_deref(),
            Some("Здание А / 1 этаж"),
            "2 сегмента: нечего выбрасывать, place_path_short == full_path с обычным ' / '"
        );
    })
    .await
    .expect("timeout");
}

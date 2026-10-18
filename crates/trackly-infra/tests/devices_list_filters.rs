//! Integration tests: `DeviceRepository::list` / `list_all` применяют фильтры
//! `place_id` (поддерево), `state` (нормализованное равенство) и `name_prefix`
//! (FTS5 по четырём полям) — Phase 41.11 Plan 01 (WR-02, D-01…D-06).
//! Только вымышленные данные.

use rusqlite::Connection;
use trackly_core::domain::devices::{DeviceFilter, DeviceNew, Pagination};
use trackly_core::domain::places::{PlaceKind, PlaceNew};
use trackly_core::ports::devices::DeviceRepository;
use trackly_core::ports::places::PlaceRepository;
use trackly_infra::repos::{SqliteDeviceRepository, SqlitePlaceRepository};
use trackly_infra::test_support::test_db;

const NOW: i64 = 1_700_000_000;

fn status(conn: &Connection) -> i64 {
    conn.query_row(
        "SELECT id FROM device_statuses WHERE code = 'на_складе'",
        [],
        |r| r.get(0),
    )
    .unwrap()
}

fn place(conn: &mut Connection, parent: Option<i64>, kind: PlaceKind, name: &str) -> i64 {
    SqlitePlaceRepository
        .create(
            conn,
            &PlaceNew {
                parent_id: parent,
                kind,
                name: name.to_string(),
                level: None,
                is_storage: false,
                sort_order: None,
                notes: None,
            },
            NOW,
        )
        .unwrap()
}

#[derive(Default)]
struct Spec<'a> {
    name: &'a str,
    inv: Option<&'a str>,
    serial: Option<&'a str>,
    model: Option<&'a str>,
    state: Option<&'a str>,
    place: Option<i64>,
}

fn add(conn: &mut Connection, spec: Spec) -> i64 {
    let st = status(conn);
    SqliteDeviceRepository
        .create(
            conn,
            &DeviceNew {
                type_id: 1,
                name: spec.name.to_string(),
                inventory_no: spec.inv.map(str::to_string),
                serial_no: spec.serial.map(str::to_string),
                model: spec.model.map(str::to_string),
                specs: None,
                kit: None,
                state: spec.state.map(str::to_string),
                place_id: spec.place,
                status_id: st,
            },
            NOW,
        )
        .unwrap()
}

/// Имена из `list` (большая страница) и из `list_all`; проверяет D-06:
/// `total` == число строк == длина `list_all`.
fn names(conn: &Connection, filter: &DeviceFilter) -> Vec<String> {
    let (rows, total) = SqliteDeviceRepository
        .list(
            conn,
            filter,
            &Pagination {
                offset: 0,
                limit: 200,
            },
        )
        .unwrap();
    let all = SqliteDeviceRepository
        .list_all(conn, filter, 10_000)
        .unwrap();
    assert_eq!(total as usize, rows.len(), "total должен равняться строкам");
    assert_eq!(all.len(), rows.len(), "list_all совпадает с list");
    let mut out: Vec<String> = rows.into_iter().map(|r| r.name).collect();
    out.sort();
    out
}

fn with_place(id: i64) -> DeviceFilter {
    DeviceFilter {
        place_id: Some(id),
        ..Default::default()
    }
}

fn with_state(s: &str) -> DeviceFilter {
    DeviceFilter {
        state: Some(s.to_string()),
        ..Default::default()
    }
}

fn with_text(s: &str) -> DeviceFilter {
    DeviceFilter {
        name_prefix: Some(s.to_string()),
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// place_id: поддерево (D-02)
// ---------------------------------------------------------------------------

struct Tree {
    korpus: i64,
    etazh: i64,
    kabinet: i64,
    sosed: i64,
}

fn tree(conn: &mut Connection) -> Tree {
    let korpus = place(conn, None, PlaceKind::Building, "Корпус А");
    let etazh = place(conn, Some(korpus), PlaceKind::Floor, "Этаж 1");
    let kabinet = place(conn, Some(etazh), PlaceKind::Room, "Кабинет 1");
    let sosed = place(conn, None, PlaceKind::Building, "Корпус Б");
    add(
        conn,
        Spec {
            name: "Устройство корпуса",
            place: Some(korpus),
            ..Default::default()
        },
    );
    add(
        conn,
        Spec {
            name: "Устройство этажа",
            place: Some(etazh),
            ..Default::default()
        },
    );
    add(
        conn,
        Spec {
            name: "Устройство кабинета",
            place: Some(kabinet),
            ..Default::default()
        },
    );
    add(
        conn,
        Spec {
            name: "Устройство соседа",
            place: Some(sosed),
            ..Default::default()
        },
    );
    add(
        conn,
        Spec {
            name: "Устройство без места",
            ..Default::default()
        },
    );
    Tree {
        korpus,
        etazh,
        kabinet,
        sosed,
    }
}

#[test]
fn place_filter_returns_subtree_and_not_neighbour() {
    let (mut conn, _d) = test_db();
    let t = tree(&mut conn);
    assert_eq!(
        names(&conn, &with_place(t.korpus)),
        vec![
            "Устройство кабинета",
            "Устройство корпуса",
            "Устройство этажа"
        ]
    );
    assert_eq!(
        names(&conn, &with_place(t.sosed)),
        vec!["Устройство соседа"]
    );
}

#[test]
fn place_filter_on_leaf_excludes_parent_devices() {
    let (mut conn, _d) = test_db();
    let t = tree(&mut conn);
    assert_eq!(
        names(&conn, &with_place(t.kabinet)),
        vec!["Устройство кабинета"]
    );
    assert_eq!(
        names(&conn, &with_place(t.etazh)),
        vec!["Устройство кабинета", "Устройство этажа"]
    );
}

#[test]
fn place_filter_deleted_or_missing_place_is_empty() {
    let (mut conn, _d) = test_db();
    let t = tree(&mut conn);
    assert!(names(&conn, &with_place(9_999_999)).is_empty());
    conn.execute(
        "UPDATE places SET deleted_at_utc = ?1 WHERE id = ?2",
        rusqlite::params![NOW, t.sosed],
    )
    .unwrap();
    assert!(names(&conn, &with_place(t.sosed)).is_empty());
}

#[test]
fn place_filter_deleted_descendant_is_cut_off() {
    let (mut conn, _d) = test_db();
    let t = tree(&mut conn);
    conn.execute(
        "UPDATE places SET deleted_at_utc = ?1 WHERE id = ?2",
        rusqlite::params![NOW, t.etazh],
    )
    .unwrap();
    // этаж удалён -> поддерево корпуса не заходит ни в этаж, ни в кабинет под ним
    assert_eq!(
        names(&conn, &with_place(t.korpus)),
        vec!["Устройство корпуса"]
    );
}

// ---------------------------------------------------------------------------
// state: нормализованное равенство (D-04)
// ---------------------------------------------------------------------------

fn state_fixture(conn: &mut Connection) {
    add(
        conn,
        Spec {
            name: "Монитор А",
            state: Some("Б/У"),
            ..Default::default()
        },
    );
    let b = add(
        conn,
        Spec {
            name: "Монитор Б",
            state: Some("x"),
            ..Default::default()
        },
    );
    // значение в БД с хвостовым пробелом и в нижнем регистре
    conn.execute("UPDATE devices SET condition = 'б/у ' WHERE id = ?1", [b])
        .unwrap();
    add(
        conn,
        Spec {
            name: "Монитор В",
            state: Some("Новое"),
            ..Default::default()
        },
    );
    add(
        conn,
        Spec {
            name: "Монитор Г",
            state: None,
            ..Default::default()
        },
    );
}

#[test]
fn state_filter_is_trim_and_case_insensitive() {
    let (mut conn, _d) = test_db();
    state_fixture(&mut conn);
    assert_eq!(
        names(&conn, &with_state("б/у")),
        vec!["Монитор А", "Монитор Б"]
    );
    assert_eq!(
        names(&conn, &with_state("  Б/У  ")),
        vec!["Монитор А", "Монитор Б"]
    );
    assert_eq!(names(&conn, &with_state("НОВОЕ")), vec!["Монитор В"]);
    assert!(names(&conn, &with_state("нет такого")).is_empty());
}

#[test]
fn state_filter_blank_means_not_set() {
    let (mut conn, _d) = test_db();
    state_fixture(&mut conn);
    assert_eq!(names(&conn, &with_state("   ")).len(), 4);
}

// ---------------------------------------------------------------------------
// name_prefix: FTS5 по четырём полям (D-03)
// ---------------------------------------------------------------------------

fn text_fixture(conn: &mut Connection) {
    add(
        conn,
        Spec {
            name: "Альфабукса",
            inv: Some("INV-0001"),
            serial: Some("SN-AAA111"),
            model: Some("Модельзет"),
            ..Default::default()
        },
    );
    add(
        conn,
        Spec {
            name: "Бетастанция",
            inv: Some("INV-0002"),
            serial: Some("SN-BBB222"),
            model: Some("Моделькью"),
            ..Default::default()
        },
    );
}

#[test]
fn text_filter_matches_each_of_four_fields_and_not_other() {
    let (mut conn, _d) = test_db();
    text_fixture(&mut conn);
    assert_eq!(names(&conn, &with_text("Альфа")), vec!["Альфабукса"]);
    assert_eq!(names(&conn, &with_text("INV-0002")), vec!["Бетастанция"]);
    assert_eq!(names(&conn, &with_text("SN-AAA111")), vec!["Альфабукса"]);
    assert_eq!(names(&conn, &with_text("Моделькью")), vec!["Бетастанция"]);
    assert!(names(&conn, &with_text("Гаммазона")).is_empty());
}

#[test]
fn text_filter_special_characters_do_not_break_parsing() {
    let (mut conn, _d) = test_db();
    text_fixture(&mut conn);
    for raw in ["\"", "-", "*", "AND", "OR", "a\"b", "name:x", "( )"] {
        // не должно быть ошибки разбора FTS; выборка — пустая или валидная
        let _ = names(&conn, &with_text(raw));
    }
}

#[test]
fn text_filter_blank_means_not_set_and_nul_only_is_empty() {
    let (mut conn, _d) = test_db();
    text_fixture(&mut conn);
    assert_eq!(names(&conn, &with_text("   ")).len(), 2);
    assert!(
        names(&conn, &with_text("\0")).is_empty(),
        "ввод без токенов после sanitizer-а даёт пустую выборку, а не «без фильтра»"
    );
}

// ---------------------------------------------------------------------------
// Комбинации и согласованность total (D-06)
// ---------------------------------------------------------------------------

#[test]
fn place_and_state_combine_with_total_equal_to_rows() {
    let (mut conn, _d) = test_db();
    let t = tree(&mut conn);
    add(
        &mut conn,
        Spec {
            name: "Комбо в корпусе",
            state: Some("Б/У"),
            place: Some(t.etazh),
            ..Default::default()
        },
    );
    add(
        &mut conn,
        Spec {
            name: "Комбо у соседа",
            state: Some("Б/У"),
            place: Some(t.sosed),
            ..Default::default()
        },
    );
    add(
        &mut conn,
        Spec {
            name: "Комбо новое в корпусе",
            state: Some("Новое"),
            place: Some(t.etazh),
            ..Default::default()
        },
    );
    let filter = DeviceFilter {
        place_id: Some(t.korpus),
        state: Some("б/у".to_string()),
        ..Default::default()
    };
    assert_eq!(names(&conn, &filter), vec!["Комбо в корпусе"]);

    let with_all = DeviceFilter {
        place_id: Some(t.korpus),
        state: Some("б/у".to_string()),
        name_prefix: Some("Комбо".to_string()),
        ..Default::default()
    };
    assert_eq!(names(&conn, &with_all), vec!["Комбо в корпусе"]);
}

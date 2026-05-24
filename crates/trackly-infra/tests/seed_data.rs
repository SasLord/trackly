//! Integration test: lookup tables seeded per D-Migrations-01.
//!
//! Asserts the EXACT seed rows for `device_types`, `device_statuses`,
//! `cartridge_states`, `cartridge_statuses`, and `counters`. Cyrillic
//! strings are compared as `String` (not `Cow`) so any silent encoding
//! drift fails loudly.

use trackly_infra::test_support::test_db;

fn select_names(conn: &rusqlite::Connection, table: &str) -> Vec<String> {
    let sql = format!("SELECT name FROM {table} ORDER BY id");
    let mut stmt = conn.prepare(&sql).expect("prepare");
    let rows = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .expect("query_map");
    rows.collect::<Result<Vec<_>, _>>().expect("collect")
}

#[test]
fn device_types_seed_matches_d_migrations_01() {
    let (conn, _guard) = test_db();
    let got = select_names(&conn, "device_types");
    assert_eq!(got, vec!["Устройство".to_string(), "Принтер".to_string()]);
}

#[test]
fn device_statuses_seed_matches_d_migrations_01() {
    let (conn, _guard) = test_db();
    let got = select_names(&conn, "device_statuses");
    assert_eq!(
        got,
        vec![
            "На складе".to_string(),
            "В работе".to_string(),
            "На ремонте".to_string(),
            "Списано".to_string(),
        ]
    );
}

#[test]
fn cartridge_states_seed_matches_d_migrations_01() {
    let (conn, _guard) = test_db();
    let got = select_names(&conn, "cartridge_states");
    assert_eq!(
        got,
        vec![
            "Полный".to_string(),
            "Частичный".to_string(),
            "Пустой".to_string(),
        ]
    );
}

#[test]
fn cartridge_statuses_seed_matches_d_migrations_01() {
    let (conn, _guard) = test_db();
    let got = select_names(&conn, "cartridge_statuses");
    assert_eq!(
        got,
        vec![
            "На складе".to_string(),
            "В работе".to_string(),
            "На заправке".to_string(),
            "Списано".to_string(),
        ]
    );
}

#[test]
fn counters_seeded_with_act_number_and_cartridge_seq() {
    let (conn, _guard) = test_db();
    let mut stmt = conn
        .prepare("SELECT name FROM counters ORDER BY name")
        .expect("prepare");
    let names: Vec<String> = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .expect("query_map")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect");
    assert_eq!(
        names,
        vec!["act_number".to_string(), "cartridge_seq".to_string()]
    );

    // Both counters start at 0.
    let act_value: i64 = conn
        .query_row(
            "SELECT current_value FROM counters WHERE name = 'act_number'",
            [],
            |r| r.get(0),
        )
        .expect("read act_number");
    assert_eq!(act_value, 0);
    let cart_value: i64 = conn
        .query_row(
            "SELECT current_value FROM counters WHERE name = 'cartridge_seq'",
            [],
            |r| r.get(0),
        )
        .expect("read cartridge_seq");
    assert_eq!(cart_value, 0);
}

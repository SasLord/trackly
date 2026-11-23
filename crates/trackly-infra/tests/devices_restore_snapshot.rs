//! Откат по снимку: `place_id` (долг v1.4 П2, WR-03). Вымышленные данные.
//!
//! Снимок без ключа `place_id` не должен стирать место устройства;
//! снимок с `"place_id": null` по-прежнему обнуляет место.

use rusqlite::{params, Connection};
use serde_json::json;
use trackly_core::domain::devices::DeviceNew;
use trackly_core::ports::devices::DeviceRepository;
use trackly_infra::repos::SqliteDeviceRepository;
use trackly_infra::test_support::test_db;

const NOW: i64 = 1_700_000_000;

fn add_place(conn: &Connection, name: &str) -> i64 {
    conn.execute(
        "INSERT INTO places (parent_id, kind, name, is_storage, created_at_utc, updated_at_utc)
         VALUES (NULL, 'building', ?1, 1, ?2, ?2)",
        params![name, NOW],
    )
    .expect("insert place");
    conn.last_insert_rowid()
}

fn device_in_place(conn: &mut Connection, place_id: i64) -> i64 {
    let status_id: i64 = conn
        .query_row(
            "SELECT id FROM device_statuses WHERE code = 'на_складе'",
            [],
            |r| r.get(0),
        )
        .expect("status");
    SqliteDeviceRepository
        .create(
            conn,
            &DeviceNew {
                type_id: 1,
                name: "Монитор Иванова".to_string(),
                inventory_no: Some("ИНВ-000001".to_string()),
                serial_no: None,
                model: None,
                specs: None,
                kit: None,
                state: None,
                place_id: Some(place_id),
                status_id,
                origin_group_id: None,
            },
            NOW,
        )
        .expect("create")
}

fn restore(conn: &mut Connection, id: i64, snapshot: serde_json::Value) -> Option<i64> {
    let status_id = snapshot["status_id"].as_i64().unwrap();
    assert!(status_id > 0);
    let tx = conn.transaction().unwrap();
    let row = SqliteDeviceRepository
        .restore_from_snapshot_in_tx(&tx, id, &snapshot, NOW + 10)
        .expect("restore");
    tx.commit().unwrap();
    row.place_id
}

fn status(conn: &Connection) -> i64 {
    conn.query_row(
        "SELECT id FROM device_statuses WHERE code = 'на_складе'",
        [],
        |r| r.get(0),
    )
    .unwrap()
}

/// A: ключа `place_id` нет -> место прежнее.
#[test]
fn snapshot_without_place_key_keeps_place() {
    let (mut conn, _dir) = test_db();
    let p = add_place(&conn, "Склад А");
    let id = device_in_place(&mut conn, p);
    let sid = status(&conn);
    assert_eq!(restore(&mut conn, id, json!({ "status_id": sid })), Some(p));
}

/// B: `"place_id": null` -> место обнуляется.
#[test]
fn snapshot_with_null_place_clears_place() {
    let (mut conn, _dir) = test_db();
    let p = add_place(&conn, "Склад А");
    let id = device_in_place(&mut conn, p);
    let sid = status(&conn);
    assert_eq!(
        restore(&mut conn, id, json!({ "status_id": sid, "place_id": null })),
        None
    );
}

/// C: `"place_id": N` -> место N.
#[test]
fn snapshot_with_place_sets_place() {
    let (mut conn, _dir) = test_db();
    let p1 = add_place(&conn, "Склад А");
    let p2 = add_place(&conn, "Склад Б");
    let id = device_in_place(&mut conn, p1);
    let sid = status(&conn);
    assert_eq!(
        restore(&mut conn, id, json!({ "status_id": sid, "place_id": p2 })),
        Some(p2)
    );
}

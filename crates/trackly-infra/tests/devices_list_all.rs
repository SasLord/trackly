//! Integration tests: `DeviceRepository::list_all` — выгрузка без потолка страницы
//! (Phase 41.1 Plan 27, WR-05/GRD-03). Вымышленные имена и номера.

use rusqlite::Connection;
use trackly_core::domain::devices::{DeviceFilter, DeviceNew, Pagination};
use trackly_core::error::AppError;
use trackly_core::ports::devices::DeviceRepository;
use trackly_infra::repos::SqliteDeviceRepository;
use trackly_infra::test_support::test_db;

const NOW: i64 = 1_700_000_000;

fn status_id(conn: &Connection, code: &str) -> i64 {
    conn.query_row(
        "SELECT id FROM device_statuses WHERE code = ?1",
        [code],
        |r| r.get(0),
    )
    .unwrap()
}

fn add_device(conn: &mut Connection, name: &str, inv: &str, status: i64) -> i64 {
    SqliteDeviceRepository
        .create(
            conn,
            &DeviceNew {
                type_id: 1,
                name: name.to_string(),
                inventory_no: Some(inv.to_string()),
                serial_no: None,
                model: None,
                specs: None,
                kit: None,
                state: None,
                place_id: None,
                status_id: status,
            },
            NOW,
        )
        .unwrap()
}

#[test]
fn list_all_returns_more_than_page_cap() {
    let (mut conn, _d) = test_db();
    let st = status_id(&conn, "на_складе");
    for i in 0..250 {
        add_device(
            &mut conn,
            &format!("Системный блок {i}"),
            &format!("INV-{i:04}"),
            st,
        );
    }
    let filter = DeviceFilter::default();
    let (page_rows, total) = SqliteDeviceRepository
        .list(
            &conn,
            &filter,
            &Pagination {
                offset: 0,
                limit: 1000,
            },
        )
        .unwrap();
    assert_eq!(page_rows.len(), 200, "список остаётся на потолке 200");
    assert_eq!(total, 250);
    let all = SqliteDeviceRepository
        .list_all(&conn, &filter, 1000)
        .unwrap();
    assert_eq!(all.len(), 250);
}

#[test]
fn list_all_rejects_when_over_ceiling() {
    let (mut conn, _d) = test_db();
    let st = status_id(&conn, "на_складе");
    for i in 0..5 {
        add_device(
            &mut conn,
            &format!("Системный блок {i}"),
            &format!("INV-{i:04}"),
            st,
        );
    }
    let err = SqliteDeviceRepository
        .list_all(&conn, &DeviceFilter::default(), 3)
        .unwrap_err();
    match err {
        AppError::Validation { field, message } => {
            assert_eq!(field, "export");
            assert!(message.contains('5'), "нет числа 5: {message}");
            assert!(message.contains('3'), "нет числа 3: {message}");
        }
        other => panic!("ожидали Validation, получили {other:?}"),
    }
}

#[test]
fn list_all_exactly_at_ceiling_is_allowed() {
    let (mut conn, _d) = test_db();
    let st = status_id(&conn, "на_складе");
    for i in 0..3 {
        add_device(
            &mut conn,
            &format!("Системный блок {i}"),
            &format!("INV-{i:04}"),
            st,
        );
    }
    let rows = SqliteDeviceRepository
        .list_all(&conn, &DeviceFilter::default(), 3)
        .unwrap();
    assert_eq!(rows.len(), 3);
}

#[test]
fn list_all_orders_by_name_then_id() {
    let (mut conn, _d) = test_db();
    let st = status_id(&conn, "на_складе");
    let a = add_device(&mut conn, "Системный блок", "INV-0001", st);
    let b = add_device(&mut conn, "Системный блок", "INV-0002", st);
    let c = add_device(&mut conn, "Системный блок", "INV-0003", st);
    let first = add_device(&mut conn, "Монитор", "INV-0004", st);
    let rows = SqliteDeviceRepository
        .list_all(&conn, &DeviceFilter::default(), 100)
        .unwrap();
    let ids: Vec<i64> = rows.iter().map(|r| r.id).collect();
    assert_eq!(ids, vec![first, a, b, c]);
}

#[test]
fn list_all_respects_filter() {
    let (mut conn, _d) = test_db();
    let st_stock = status_id(&conn, "на_складе");
    let st_other = status_id(&conn, "в_работе");
    let keep = add_device(&mut conn, "Системный блок 1", "INV-0001", st_stock);
    let _other = add_device(&mut conn, "Системный блок 2", "INV-0002", st_other);
    let gone = add_device(&mut conn, "Системный блок 3", "INV-0003", st_stock);
    SqliteDeviceRepository
        .delete_soft(&mut conn, gone, 1, NOW)
        .unwrap();

    let by_status = DeviceFilter {
        status_id: Some(st_stock),
        ..Default::default()
    };
    let rows = SqliteDeviceRepository
        .list_all(&conn, &by_status, 100)
        .unwrap();
    assert_eq!(rows.iter().map(|r| r.id).collect::<Vec<_>>(), vec![keep]);

    let with_deleted = DeviceFilter {
        status_id: Some(st_stock),
        include_deleted: true,
        ..Default::default()
    };
    let rows = SqliteDeviceRepository
        .list_all(&conn, &with_deleted, 100)
        .unwrap();
    assert_eq!(rows.len(), 2);
}

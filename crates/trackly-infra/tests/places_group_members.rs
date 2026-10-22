//! Integration tests: `PlaceRepository::list_group_members` — физический состав
//! группы для раскрытия строки «group» в «Местах» (Phase 41.2 Plan 05, GAM-04).
//! Вымышленные имена и номера.

use rusqlite::{params, Connection};
use trackly_core::ports::places::PlaceRepository;
use trackly_infra::repos::SqlitePlaceRepository;
use trackly_infra::test_support::test_db;

const NOW: i64 = 1_700_000_000;

fn add_place(conn: &Connection, name: &str) -> i64 {
    conn.execute(
        "INSERT INTO places (parent_id, kind, name, version, created_at_utc, updated_at_utc) \
         VALUES (NULL, 'room', ?1, 1, ?2, ?2)",
        params![name, NOW],
    )
    .unwrap();
    conn.last_insert_rowid()
}

fn add_device(
    conn: &Connection,
    type_name: &str,
    name: &str,
    inv: &str,
    place: Option<i64>,
) -> i64 {
    conn.execute(
        "INSERT INTO devices (type_id, name, inventory_number, place_id, status_id, version, \
         created_at_utc, updated_at_utc) \
         VALUES ((SELECT id FROM device_types WHERE name = ?1), ?2, ?3, ?4, \
                 (SELECT id FROM device_statuses WHERE code = 'на_складе'), 1, ?5, ?5)",
        params![type_name, name, inv, place, NOW],
    )
    .unwrap();
    conn.last_insert_rowid()
}

fn add_group(
    conn: &Connection,
    type_id: i64,
    seq: i64,
    parent: Option<i64>,
    place: Option<i64>,
) -> i64 {
    conn.execute(
        "INSERT INTO groups (type_id, name, seq, place_id, parent_group_id, version, \
         created_at_utc, updated_at_utc) VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, ?6)",
        params![type_id, format!("Комплект {seq}"), seq, place, parent, NOW],
    )
    .unwrap();
    conn.last_insert_rowid()
}

fn join(conn: &Connection, device: i64, group: i64) {
    conn.execute(
        "INSERT INTO group_devices (device_id, group_id, added_at_utc) VALUES (?1, ?2, ?3)",
        params![device, group, NOW],
    )
    .unwrap();
}

fn group_type(conn: &Connection) -> i64 {
    conn.execute(
        "INSERT INTO group_types (code, name, behavior, created_at_utc, updated_at_utc) \
         VALUES ('t_members', 'Рабочее место', 'repair', ?1, ?1)",
        params![NOW],
    )
    .unwrap();
    conn.last_insert_rowid()
}

#[test]
fn list_group_members_is_flat_physical_ordered_by_name_id() {
    let (conn, _d) = test_db();
    let place = add_place(&conn, "Кабинет 1");
    let ty = group_type(&conn);
    let g_root = add_group(&conn, ty, 1, None, Some(place));
    let g_nested = add_group(&conn, ty, 2, Some(g_root), Some(place));

    let anchor = add_device(&conn, "Устройство", "Якорь", "ИНВ-100001", Some(place));
    let monitor = add_device(&conn, "Устройство", "Монитор", "ИНВ-100002", Some(place));
    let printer = add_device(&conn, "Принтер", "Принтер Б", "ИНВ-100003", Some(place));
    let nested_part = add_device(&conn, "Устройство", "Мышь", "ИНВ-100004", Some(place));
    let outsider = add_device(&conn, "Устройство", "Чужая мышь", "ИНВ-100005", Some(place));
    let deleted = add_device(&conn, "Устройство", "Удалённая", "ИНВ-100006", Some(place));
    conn.execute(
        "UPDATE devices SET deleted_at_utc = ?2 WHERE id = ?1",
        params![deleted, NOW],
    )
    .unwrap();
    conn.execute(
        "UPDATE groups SET anchor_device_id = ?1 WHERE id = ?2",
        params![anchor, g_root],
    )
    .unwrap();
    for d in [anchor, monitor, printer, deleted] {
        join(&conn, d, g_root);
    }
    join(&conn, nested_part, g_nested);
    let _ = outsider;

    let rows = SqlitePlaceRepository
        .list_group_members(&conn, g_root)
        .unwrap();
    let got: Vec<(&str, i64)> = rows.iter().map(|r| (r.kind.as_str(), r.id)).collect();
    // ORDER BY name, id: «Якорь», «Монитор», «Мышь», «Принтер Б» -> по алфавиту.
    let mut expected = [
        ("device", anchor, "Якорь"),
        ("device", monitor, "Монитор"),
        ("printer", printer, "Принтер Б"),
        ("device", nested_part, "Мышь"),
    ];
    expected.sort_by(|a, b| a.2.cmp(b.2).then(a.1.cmp(&b.1)));
    let expected: Vec<(&str, i64)> = expected.iter().map(|(k, i, _)| (*k, *i)).collect();
    assert_eq!(got, expected);
    assert!(rows.iter().all(|r| r.group_type_name.is_none()
        && r.member_count.is_none()
        && r.printer_member_count.is_none()));
    assert!(rows.iter().all(|r| r.full_path.contains("Кабинет 1")));
}

#[test]
fn list_group_members_deleted_or_missing_group_is_empty() {
    let (conn, _d) = test_db();
    let place = add_place(&conn, "Кабинет 2");
    let ty = group_type(&conn);
    let g = add_group(&conn, ty, 1, None, Some(place));
    let d = add_device(&conn, "Устройство", "Монитор", "ИНВ-200001", Some(place));
    join(&conn, d, g);
    assert_eq!(
        SqlitePlaceRepository
            .list_group_members(&conn, g)
            .unwrap()
            .len(),
        1
    );
    conn.execute(
        "UPDATE groups SET deleted_at_utc = ?2 WHERE id = ?1",
        params![g, NOW],
    )
    .unwrap();
    assert!(SqlitePlaceRepository
        .list_group_members(&conn, g)
        .unwrap()
        .is_empty());
    assert!(SqlitePlaceRepository
        .list_group_members(&conn, 999_999)
        .unwrap()
        .is_empty());
}

#[test]
fn list_group_members_keeps_members_without_place() {
    let (conn, _d) = test_db();
    let ty = group_type(&conn);
    let g = add_group(&conn, ty, 1, None, None);
    let d = add_device(&conn, "Устройство", "Монитор", "ИНВ-300001", None);
    join(&conn, d, g);
    let rows = SqlitePlaceRepository.list_group_members(&conn, g).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].full_path, "");
}

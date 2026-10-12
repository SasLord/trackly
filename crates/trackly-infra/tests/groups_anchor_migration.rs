//! Integration test: `V048__group_anchor_device_origin.sql`
//! (Phase 41.1 Plan 01, GRD-02 / GRD-03 / GRP-10), прогнанный через НАСТОЯЩИЙ
//! раннер: `run_up_to(46)` -> посев -> `run`.
//!
//! Фиксируется поведение FK: физическое удаление якоря блокируется
//! (RESTRICT), а МЯГКОЕ — нет, поэтому запрет мягкого удаления якоря обязан
//! жить в сервисе. Только вымышленные данные.

use rusqlite::{params, Connection};
use tempfile::TempDir;

use trackly_infra::db::migrations;
use trackly_infra::db::pragmas::apply_writer_pragmas;

const NOW: i64 = 1_700_000_000;

fn conn_at_v046() -> (Connection, TempDir) {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("groups-anchor-migration.db");
    let mut conn = Connection::open(&path).expect("open");
    apply_writer_pragmas(&conn).expect("writer pragmas");
    let report = migrations::run_up_to(&mut conn, 46).expect("run migrations up to V046");
    assert_eq!(report.schema_version, 46, "fixture must be a real V046 DB");
    (conn, dir)
}

fn seed(conn: &Connection) {
    conn.execute_batch(&format!(
        "INSERT INTO users (id, login, full_name, role, created_at_utc, updated_at_utc)
           VALUES (1, 'petrov', 'Петров П.П.', 'admin', {NOW}, {NOW});
         INSERT INTO places (id, parent_id, kind, name, is_storage, created_at_utc, updated_at_utc)
           VALUES (1, NULL, 'building', 'Склад А', 1, {NOW}, {NOW});
         INSERT INTO devices (id, type_id, name, inventory_number, status_id, place_id,
                              created_at_utc, updated_at_utc)
           VALUES (1, 1, 'Монитор #2', 'ИНВ-000001', 2, 1, {NOW}, {NOW}),
                  (2, 1, 'Планка памяти', 'ИНВ-000002', 1, 1, {NOW}, {NOW});
         INSERT INTO acts (id, number, sub_number, parent_act_id, act_type, giver_name,
                           receiver_name, created_at_utc, updated_at_utc, version,
                           handover_date_utc)
           VALUES (1, '42', NULL, NULL, 'handover', 'Петров П.П.', 'Иванов И.И.', {NOW}, {NOW}, 1, {NOW});
         INSERT INTO act_items (id, act_id, device_id, quantity) VALUES (1, 1, 1, 1);"
    ))
    .expect("seed V046 data");
}

fn upgraded() -> (Connection, TempDir) {
    let (mut conn, guard) = conn_at_v046();
    seed(&conn);
    migrations::run(&mut conn).expect("upgrade V046 -> latest");
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .expect("enable fk");
    (conn, guard)
}

fn count(conn: &Connection, sql: &str) -> i64 {
    conn.query_row(sql, [], |r| r.get(0)).expect("count query")
}

fn has_column(conn: &Connection, table: &str, column: &str) -> bool {
    count(
        conn,
        &format!("SELECT COUNT(*) FROM pragma_table_info('{table}') WHERE name = '{column}'"),
    ) == 1
}

fn has_index(conn: &Connection, name: &str) -> bool {
    count(
        conn,
        &format!("SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name = '{name}'"),
    ) == 1
}

fn insert_type(conn: &Connection) -> i64 {
    conn.execute(
        "INSERT INTO group_types (code, name, behavior, created_at_utc, updated_at_utc)
         VALUES ('workstation_test', 'АРМ', 'container', ?1, ?1)",
        params![NOW],
    )
    .expect("insert type");
    conn.last_insert_rowid()
}

fn insert_group(
    conn: &Connection,
    type_id: i64,
    name: &str,
    seq: i64,
    anchor: Option<i64>,
) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO groups (type_id, name, seq, anchor_device_id, created_at_utc, updated_at_utc)
         VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
        params![type_id, name, seq, anchor, NOW],
    )?;
    Ok(conn.last_insert_rowid())
}

#[test]
fn v048_adds_columns_and_indexes() {
    let (conn, _guard) = upgraded();
    assert!(has_column(&conn, "groups", "anchor_device_id"));
    assert!(has_column(&conn, "devices", "origin_group_id"));
    assert!(has_index(&conn, "idx_groups_anchor"));
    assert!(has_index(&conn, "idx_devices_origin"));
}

#[test]
fn v048_anchor_is_unique_but_null_is_repeatable() {
    let (conn, _guard) = upgraded();
    let ty = insert_type(&conn);
    insert_group(&conn, ty, "АРМ #1", 1, Some(1)).expect("first anchored group");
    assert!(
        insert_group(&conn, ty, "АРМ #2", 2, Some(1)).is_err(),
        "one anchor device must not anchor two groups"
    );
    insert_group(&conn, ty, "АРМ #3", 3, None).expect("null anchor 1");
    insert_group(&conn, ty, "АРМ #4", 4, None).expect("null anchor 2");
}

#[test]
fn v048_physical_delete_of_anchor_device_is_restricted() {
    let (conn, _guard) = upgraded();
    // Устройство 2 без act_items: единственная причина отказа — якорь.
    let ty = insert_type(&conn);
    insert_group(&conn, ty, "АРМ #1", 1, Some(2)).expect("group");
    assert!(
        conn.execute("DELETE FROM devices WHERE id = 2", []).is_err(),
        "physical DELETE of an anchor device must fail (RESTRICT)"
    );
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM devices WHERE id = 2"), 1);
}

#[test]
fn v048_soft_delete_of_anchor_is_not_blocked_by_fk() {
    let (conn, _guard) = upgraded();
    let ty = insert_type(&conn);
    insert_group(&conn, ty, "АРМ #1", 1, Some(2)).expect("group");
    // Устройства удаляются мягко: FK этого не видит, поэтому запрет
    // обязан жить в сервисе (DeviceService::delete_soft).
    conn.execute(
        "UPDATE devices SET deleted_at_utc = ?1 WHERE id = 2",
        params![NOW],
    )
    .expect("soft delete passes the FK");
    let anchor: Option<i64> = conn
        .query_row(
            "SELECT anchor_device_id FROM groups WHERE name = 'АРМ #1'",
            [],
            |r| r.get(0),
        )
        .expect("read anchor");
    assert_eq!(anchor, Some(2), "anchor link survives a soft delete");
}

#[test]
fn v048_group_delete_nulls_origin_but_keeps_device() {
    let (conn, _guard) = upgraded();
    let ty = insert_type(&conn);
    let group = insert_group(&conn, ty, "Разбор #1", 1, None).expect("group");
    conn.execute(
        "UPDATE devices SET origin_group_id = ?1 WHERE id = 2",
        params![group],
    )
    .expect("set origin");
    conn.execute("DELETE FROM groups WHERE id = ?1", params![group])
        .expect("delete group");
    let origin: Option<i64> = conn
        .query_row("SELECT origin_group_id FROM devices WHERE id = 2", [], |r| {
            r.get(0)
        })
        .expect("read origin");
    assert_eq!(origin, None, "ON DELETE SET NULL on devices.origin_group_id");
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM devices WHERE id = 2"), 1);
}

#[test]
fn v048_preserves_rows_and_is_idempotent() {
    let (mut conn, _guard) = conn_at_v046();
    seed(&conn);
    let items_before = count(&conn, "SELECT COUNT(*) FROM act_items");
    let devices_before = count(&conn, "SELECT COUNT(*) FROM devices");
    assert!(items_before > 0 && devices_before > 0);

    let report = migrations::run(&mut conn).expect("upgrade V046 -> latest");
    assert_eq!(report.schema_version, migrations::max_known_version());
    assert!(migrations::max_known_version() >= 48);

    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM act_items"),
        items_before,
        "act_items must survive"
    );
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM devices"),
        devices_before,
        "devices must survive"
    );
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM pragma_foreign_key_check"),
        0,
        "foreign_key_check must be empty"
    );
    let again = migrations::run(&mut conn).expect("second run");
    assert_eq!(again.schema_version, report.schema_version);
}

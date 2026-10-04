//! Integration test: `V045__groups.sql` + `V046__place_movements_batch.sql`
//! (Phase 41 Plan 01, GRP-01/02/03/05/06/10), exercised through the REAL
//! migration runner exactly as `AppCtx::build` runs it on an upgrade:
//! `run_up_to(44)` -> seed -> `run`.
//!
//! The schema is purely additive (урок V042: перестройка таблицы в
//! refinery-транзакции стирала `act_items`). These tests pin zero data
//! loss, the immutability trigger on `group_types.code/behavior`, the
//! membership / numbering / value-uniqueness constraints and the FK
//! cascades. Fictional names only.

use rusqlite::{params, Connection};
use tempfile::TempDir;

use trackly_infra::db::migrations;
use trackly_infra::db::pragmas::apply_writer_pragmas;

const NOW: i64 = 1_700_000_000;

fn conn_at_v044() -> (Connection, TempDir) {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("groups-migration.db");
    let mut conn = Connection::open(&path).expect("open");
    apply_writer_pragmas(&conn).expect("writer pragmas");
    let report = migrations::run_up_to(&mut conn, 44).expect("run migrations up to V044");
    assert_eq!(report.schema_version, 44, "fixture must be a real V044 DB");
    (conn, dir)
}

/// Fully migrated DB (V044 -> latest through the real runner).
fn migrated() -> (Connection, TempDir) {
    let (mut conn, guard) = conn_at_v044();
    migrations::run(&mut conn).expect("upgrade V044 -> latest");
    (conn, guard)
}

fn count(conn: &Connection, sql: &str) -> i64 {
    conn.query_row(sql, [], |r| r.get(0)).expect("count query")
}

fn insert_type(conn: &Connection, code: &str, behavior: &str) -> i64 {
    conn.execute(
        "INSERT INTO group_types (code, name, behavior, created_at_utc, updated_at_utc)
         VALUES (?1, ?2, ?3, ?4, ?4)",
        params![code, format!("Тип {code}"), behavior, NOW],
    )
    .expect("insert group type");
    conn.last_insert_rowid()
}

fn insert_group(conn: &Connection, type_id: i64, name: &str, seq: i64) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO groups (type_id, name, seq, created_at_utc, updated_at_utc)
         VALUES (?1, ?2, ?3, ?4, ?4)",
        params![type_id, name, seq, NOW],
    )?;
    Ok(conn.last_insert_rowid())
}

fn insert_property(conn: &Connection, type_id: i64, name: &str, data_type: &str) -> i64 {
    conn.execute(
        "INSERT INTO group_type_properties (type_id, name, data_type, created_at_utc, updated_at_utc)
         VALUES (?1, ?2, ?3, ?4, ?4)",
        params![type_id, name, data_type, NOW],
    )
    .expect("insert property");
    conn.last_insert_rowid()
}

fn insert_device(conn: &Connection, id: i64, name: &str, place_id: i64) {
    conn.execute(
        "INSERT INTO devices (id, type_id, name, inventory_number, status_id, place_id,
                              created_at_utc, updated_at_utc)
         VALUES (?1, 1, ?2, ?3, 2, ?4, ?5, ?5)",
        params![id, name, format!("ИНВ-{id:06}"), place_id, NOW],
    )
    .expect("insert device");
}

fn insert_place(conn: &Connection, id: i64, name: &str) {
    conn.execute(
        "INSERT INTO places (id, parent_id, kind, name, is_storage, created_at_utc, updated_at_utc)
         VALUES (?1, NULL, 'building', ?2, 0, ?3, ?3)",
        params![id, name, NOW],
    )
    .expect("insert place");
}

fn column_names(conn: &Connection, table: &str) -> Vec<String> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .expect("table_info");
    stmt.query_map([], |r| r.get::<_, String>(1))
        .expect("query table_info")
        .map(|r| r.expect("col"))
        .collect()
}

fn act_items(conn: &Connection) -> Vec<(i64, i64, i64)> {
    let mut stmt = conn
        .prepare("SELECT id, act_id, device_id FROM act_items ORDER BY id")
        .expect("prepare act_items");
    stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .expect("query act_items")
        .map(|r| r.expect("row"))
        .collect()
}

fn movements(conn: &Connection) -> Vec<(i64, Option<i64>, String)> {
    let mut stmt = conn
        .prepare("SELECT id, act_id, to_place_path FROM place_movements ORDER BY id")
        .expect("prepare place_movements");
    stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .expect("query place_movements")
        .map(|r| r.expect("row"))
        .collect()
}

#[test]
fn migration_v045_v046_preserves_existing_rows() {
    let (mut conn, _guard) = conn_at_v044();
    conn.execute_batch(&format!(
        "INSERT INTO users (id, login, full_name, role, created_at_utc, updated_at_utc)
           VALUES (1, 'ivanov', 'Иванов И.И.', 'admin', {NOW}, {NOW});
         INSERT INTO places (id, parent_id, kind, name, is_storage, created_at_utc, updated_at_utc)
           VALUES (1, NULL, 'building', 'Склад А', 1, {NOW}, {NOW}),
                  (2, NULL, 'building', 'Корпус Б', 0, {NOW}, {NOW});
         INSERT INTO devices (id, type_id, name, inventory_number, status_id, place_id,
                              created_at_utc, updated_at_utc)
           VALUES (1, 1, 'Ноутбук', 'ИНВ-000001', 2, 2, {NOW}, {NOW});
         INSERT INTO acts (id, number, sub_number, parent_act_id, act_type, giver_name,
                           receiver_name, created_at_utc, updated_at_utc, version,
                           handover_date_utc)
           VALUES (1, '42', NULL, NULL, 'handover', 'Иванов И.И.', 'Петров П.П.', {NOW}, {NOW}, 1, {NOW});
         INSERT INTO act_items (id, act_id, device_id, quantity) VALUES (1, 1, 1, 1);
         INSERT INTO place_movements (id, entity_type, entity_id, from_place_id, from_place_path,
                                      to_place_id, to_place_path, source, act_id, user_id,
                                      created_at_utc)
           VALUES (1, 'device', 1, 1, 'Склад А', 2, 'Корпус Б', 'act', 1, 1, {NOW});"
    ))
    .expect("seed V044 data");

    let items_before = act_items(&conn);
    let movements_before = movements(&conn);
    assert!(!items_before.is_empty());
    assert!(!movements_before.is_empty());

    let report = migrations::run(&mut conn).expect("upgrade V044 -> latest");
    assert_eq!(report.schema_version, migrations::max_known_version());
    assert_eq!(migrations::max_known_version(), 46);

    assert_eq!(act_items(&conn), items_before, "act_items must survive");
    assert_eq!(
        movements(&conn),
        movements_before,
        "place_movements rows must survive"
    );

    let fk_violations = count(&conn, "SELECT COUNT(*) FROM pragma_foreign_key_check");
    assert_eq!(fk_violations, 0, "foreign_key_check must be empty");
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM sqlite_master WHERE name = 'place_movements_new'"
        ),
        0,
        "no table rebuild leftovers"
    );

    let cols = column_names(&conn, "place_movements");
    for needed in ["batch_id", "entity_label", "group_id"] {
        assert!(
            cols.iter().any(|c| c == needed),
            "place_movements.{needed} missing: {cols:?}"
        );
    }
    // Pre-existing rows get NULL in the new columns.
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM place_movements
             WHERE batch_id IS NOT NULL OR entity_label IS NOT NULL OR group_id IS NOT NULL"
        ),
        0
    );

    // Idempotent re-run.
    let again = migrations::run(&mut conn).expect("re-run is a no-op");
    assert_eq!(again.schema_version, migrations::max_known_version());
}

#[test]
fn group_types_code_behavior_immutable_trigger() {
    let (conn, _guard) = migrated();
    let id = insert_type(&conn, "workstation", "container");

    let seeded_default: i64 = conn
        .query_row(
            "SELECT default_props_seeded FROM group_types WHERE id = ?1",
            [id],
            |r| r.get(0),
        )
        .expect("default_props_seeded");
    assert_eq!(seeded_default, 0, "default_props_seeded defaults to 0");

    let read = |col: &str| -> String {
        conn.query_row(
            &format!("SELECT {col} FROM group_types WHERE id = ?1"),
            [id],
            |r| r.get(0),
        )
        .expect("read col")
    };

    let err = conn
        .execute("UPDATE group_types SET code = 'other' WHERE id = ?1", [id])
        .expect_err("changing code must abort");
    assert!(
        err.to_string().contains("immutable"),
        "unexpected error: {err}"
    );
    assert_eq!(
        read("code"),
        "workstation",
        "code must be unchanged after aborted UPDATE"
    );

    let err = conn
        .execute(
            "UPDATE group_types SET behavior = 'teardown' WHERE id = ?1",
            [id],
        )
        .expect_err("changing behavior must abort");
    assert!(
        err.to_string().contains("immutable"),
        "unexpected error: {err}"
    );
    assert_eq!(read("behavior"), "container", "behavior must be unchanged");

    conn.execute("UPDATE group_types SET name = 'АРМ' WHERE id = ?1", [id])
        .expect("rename is allowed");
    assert_eq!(read("name"), "АРМ");

    conn.execute(
        "UPDATE group_types SET default_props_seeded = 1 WHERE id = ?1",
        [id],
    )
    .expect("seeded marker is not blocked by the trigger");
    let seeded: i64 = conn
        .query_row(
            "SELECT default_props_seeded FROM group_types WHERE id = ?1",
            [id],
            |r| r.get(0),
        )
        .expect("read marker");
    assert_eq!(seeded, 1);

    conn.execute(
        "UPDATE group_types SET code = 'workstation', behavior = 'container' WHERE id = ?1",
        [id],
    )
    .expect("same-value UPDATE must not fire the trigger");
}

#[test]
fn group_constraints_membership_seq_values() {
    let (conn, _guard) = migrated();
    insert_place(&conn, 1, "Склад А");
    insert_device(&conn, 1, "Ноутбук", 1);

    let ty = insert_type(&conn, "workstation", "container");
    let g1 = insert_group(&conn, ty, "АРМ #3", 3).expect("group 1");
    let g2 = insert_group(&conn, ty, "АРМ #3", 4).expect("same name, different seq is fine");
    assert_ne!(g1, g2);
    assert!(
        insert_group(&conn, ty, "Другая", 3).is_err(),
        "duplicate (type_id, seq) must be rejected"
    );

    // Membership: at most one group per device (PK device_id).
    conn.execute(
        "INSERT INTO group_devices (device_id, group_id, added_at_utc) VALUES (1, ?1, ?2)",
        params![g1, NOW],
    )
    .expect("first membership");
    assert!(
        conn.execute(
            "INSERT INTO group_devices (device_id, group_id, added_at_utc) VALUES (1, ?1, ?2)",
            params![g2, NOW],
        )
        .is_err(),
        "a device may belong to only one group"
    );
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM group_devices"), 1);

    // Property values.
    let prop = insert_property(&conn, ty, "Примечание", "text");
    let put = |value_ref: Option<i64>, primary: i64| {
        conn.execute(
            "INSERT INTO group_property_values
               (group_id, property_id, value_text, value_ref, is_primary, updated_at_utc)
             VALUES (?1, ?2, 'x', ?3, ?4, ?5)",
            params![g1, prop, value_ref, primary, NOW],
        )
    };
    put(None, 0).expect("first scalar value");
    assert!(
        put(None, 0).is_err(),
        "second scalar value for (group, property) must fail"
    );

    put(Some(10), 1).expect("first ref, primary");
    put(Some(11), 0).expect("second distinct ref");
    assert!(
        put(Some(10), 0).is_err(),
        "duplicate (group, property, ref) must fail"
    );
    assert!(
        put(Some(12), 1).is_err(),
        "second primary for (group, property) must fail"
    );
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM group_property_values"),
        3
    );
}

#[test]
fn group_fk_cascades_and_restrict() {
    let (conn, _guard) = migrated();
    insert_place(&conn, 1, "Склад А");
    insert_device(&conn, 1, "Ноутбук", 1);

    let ty = insert_type(&conn, "workstation", "container");
    let parent = insert_group(&conn, ty, "АРМ #1", 1).expect("parent");
    let child = insert_group(&conn, ty, "АРМ #2", 2).expect("child");
    conn.execute(
        "UPDATE groups SET parent_group_id = ?1, place_id = 1 WHERE id = ?2",
        params![parent, child],
    )
    .expect("link child to parent and place");

    let prop = insert_property(&conn, ty, "Примечание", "text");
    conn.execute(
        "INSERT INTO group_devices (device_id, group_id, added_at_utc) VALUES (1, ?1, ?2)",
        params![child, NOW],
    )
    .expect("membership");
    conn.execute(
        "INSERT INTO group_property_values (group_id, property_id, value_text, updated_at_utc)
         VALUES (?1, ?2, 'x', ?3)",
        params![child, prop, NOW],
    )
    .expect("value");

    // Place referenced by a group cannot be deleted (RESTRICT).
    assert!(
        conn.execute("DELETE FROM places WHERE id = 1", []).is_err(),
        "DELETE of a place referenced by a group must fail"
    );

    // Deleting the parent group nulls the child's parent_group_id.
    conn.execute("DELETE FROM groups WHERE id = ?1", [parent])
        .expect("delete parent");
    let parent_ref: Option<i64> = conn
        .query_row(
            "SELECT parent_group_id FROM groups WHERE id = ?1",
            [child],
            |r| r.get(0),
        )
        .expect("read child");
    assert_eq!(parent_ref, None, "ON DELETE SET NULL on parent_group_id");

    // Deleting the group cascades to membership and values.
    conn.execute("DELETE FROM groups WHERE id = ?1", [child])
        .expect("delete child");
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM group_devices"), 0);
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM group_property_values"),
        0
    );
    // The device itself is untouched.
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM devices WHERE id = 1"), 1);

    // A type with groups cannot be deleted (RESTRICT).
    let _g = insert_group(&conn, ty, "АРМ #3", 3).expect("group");
    assert!(
        conn.execute("DELETE FROM group_types WHERE id = ?1", [ty])
            .is_err(),
        "DELETE of a type referenced by a group must fail"
    );
}

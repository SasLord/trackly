//! Integration test: `V044__devices_inventory_number_dedup_unique.sql` —
//! `devices.inventory_number` dedup rename (NUM-15/D-18) + partial
//! case-insensitive `UNIQUE INDEX` (NUM-09), Phase 40.2 Plan 08.
//!
//! Unlike V042 (acts)/V043 (cartridges), V044 does NOT rebuild the `devices`
//! table (no column-level `UNIQUE`/`CHECK` to remove) — only a data-fix
//! (`UPDATE`) followed by a plain `CREATE INDEX`. This test therefore only
//! needs the minimal pre-existing column set V044's own SQL touches
//! (`id`, `inventory_number`, `deleted_at_utc`) plus `audit_log`, not the
//! full V003 devices schema.

use rusqlite::Connection;
use tempfile::TempDir;

use trackly_infra::db::pragmas::apply_writer_pragmas;

/// The actual V044 migration SQL, embedded at compile time so this test
/// exercises the shipped file rather than a hand-copied duplicate.
const V044_SQL: &str =
    include_str!("../../../migrations/V044__devices_inventory_number_dedup_unique.sql");

fn fresh_conn() -> (Connection, TempDir) {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("devices-inventory-dedup-migration.db");
    let conn = Connection::open(&path).expect("open");
    apply_writer_pragmas(&conn).expect("writer pragmas");
    (conn, dir)
}

/// Minimal pre-V044 schema: just enough of `devices` for V044's own SQL
/// (id, inventory_number, deleted_at_utc) plus `audit_log` (V008 shape).
fn create_pre_v044_schema(conn: &Connection) {
    conn.execute_batch(
        "CREATE TABLE devices (
            id                INTEGER PRIMARY KEY AUTOINCREMENT,
            inventory_number  TEXT    NULL,
            created_at_utc    INTEGER NOT NULL,
            updated_at_utc    INTEGER NOT NULL,
            deleted_at_utc    INTEGER NULL,
            version           INTEGER NOT NULL DEFAULT 1
         );

         CREATE TABLE audit_log (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            entity_type     TEXT    NOT NULL,
            entity_id       INTEGER NOT NULL,
            action          TEXT    NOT NULL,
            user_id         INTEGER NULL,
            before_json     TEXT    NULL,
            after_json      TEXT    NULL,
            payload_json    TEXT    NULL,
            created_at_utc  INTEGER NOT NULL
         );",
    )
    .expect("create pre-V044 devices + audit_log");
}

fn insert_device(conn: &Connection, id: i64, inventory_number: Option<&str>, now: i64) {
    conn.execute(
        "INSERT INTO devices (id, inventory_number, created_at_utc, updated_at_utc, version) \
         VALUES (?1, ?2, ?3, ?3, 1)",
        rusqlite::params![id, inventory_number, now],
    )
    .unwrap_or_else(|e| panic!("insert device id={id}: {e}"));
}

/// SPEC NUM-15's literal example: ids 3/8/12 all share (after trim/lower) the
/// same fictional inventory number. id 3 (the earliest) must survive
/// untouched; id 8 -> `ДУБЛЬ-000001 (...)`, id 12 -> `ДУБЛЬ-000002 (...)`.
#[test]
fn duplicate_group_renamed_per_spec_num_15_example() {
    let (conn, _guard) = fresh_conn();
    create_pre_v044_schema(&conn);

    let now = 1_700_000_000_i64;
    // Fictional inventory number — no real organization data (CLAUDE.md).
    insert_device(&conn, 3, Some("ОРГ-00-000007"), now);
    insert_device(&conn, 8, Some("ОРГ-00-000007"), now + 1);
    insert_device(&conn, 12, Some("ОРГ-00-000007"), now + 2);

    conn.execute_batch(V044_SQL).expect("apply V044 migration");

    let get_number = |id: i64| -> String {
        conn.query_row(
            "SELECT inventory_number FROM devices WHERE id = ?1",
            [id],
            |r| r.get(0),
        )
        .unwrap_or_else(|e| panic!("read device id={id}: {e}"))
    };

    assert_eq!(
        get_number(3),
        "ОРГ-00-000007",
        "earliest row must be untouched"
    );
    assert_eq!(get_number(8), "ДУБЛЬ-000001 (ОРГ-00-000007)");
    assert_eq!(get_number(12), "ДУБЛЬ-000002 (ОРГ-00-000007)");
}

/// Every rename must produce exactly one `audit_log` row, `user_id IS NULL`,
/// action `custom:inventory_number_dedup`, payload carrying both numbers
/// (D-18).
#[test]
fn each_rename_writes_one_audit_log_row_with_null_user() {
    let (conn, _guard) = fresh_conn();
    create_pre_v044_schema(&conn);

    let now = 1_700_000_000_i64;
    insert_device(&conn, 3, Some("ОРГ-00-000007"), now);
    insert_device(&conn, 8, Some("ОРГ-00-000007"), now + 1);
    insert_device(&conn, 12, Some("ОРГ-00-000007"), now + 2);

    conn.execute_batch(V044_SQL).expect("apply V044 migration");

    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM audit_log WHERE action = 'custom:inventory_number_dedup'",
            [],
            |r| r.get(0),
        )
        .expect("count audit rows");
    assert_eq!(
        count, 2,
        "exactly one audit row per renamed (non-earliest) device"
    );

    let mut stmt = conn
        .prepare(
            "SELECT entity_id, user_id, payload_json FROM audit_log \
             WHERE action = 'custom:inventory_number_dedup' ORDER BY entity_id",
        )
        .expect("prepare");
    let rows: Vec<(i64, Option<i64>, String)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .expect("query_map")
        .map(|r| r.expect("row"))
        .collect();

    assert_eq!(rows.len(), 2);
    for (entity_id, user_id, payload_json) in &rows {
        assert!(*entity_id > 0);
        assert_eq!(
            *user_id, None,
            "system-initiated rename must have user_id=NULL"
        );
        assert!(
            payload_json.contains("ОРГ-00-000007"),
            "payload must reference the old number: {payload_json}"
        );
        assert!(
            payload_json.contains("ДУБЛЬ-"),
            "payload must reference the new number: {payload_json}"
        );
    }
}

/// Rows with `inventory_number IS NULL` must be ignored by the grouping
/// (never treated as a duplicate group), and distinct non-empty numbers must
/// never be renamed.
#[test]
fn null_numbers_ignored_and_distinct_numbers_untouched() {
    let (conn, _guard) = fresh_conn();
    create_pre_v044_schema(&conn);

    let now = 1_700_000_000_i64;
    insert_device(&conn, 1, None, now);
    insert_device(&conn, 2, None, now + 1);
    insert_device(&conn, 4, Some("ОРГ-00-000001"), now + 2);
    insert_device(&conn, 5, Some("ОРГ-00-000002"), now + 3);

    conn.execute_batch(V044_SQL).expect("apply V044 migration");

    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM audit_log WHERE action = 'custom:inventory_number_dedup'",
            [],
            |r| r.get(0),
        )
        .expect("count audit rows");
    assert_eq!(count, 0, "no duplicates -> no renames -> no audit rows");

    let n4: Option<String> = conn
        .query_row(
            "SELECT inventory_number FROM devices WHERE id = 4",
            [],
            |r| r.get(0),
        )
        .expect("read id=4");
    let n5: Option<String> = conn
        .query_row(
            "SELECT inventory_number FROM devices WHERE id = 5",
            [],
            |r| r.get(0),
        )
        .expect("read id=5");
    assert_eq!(n4.as_deref(), Some("ОРГ-00-000001"));
    assert_eq!(n5.as_deref(), Some("ОРГ-00-000002"));
}

/// After migration, the new partial unique index enforces case-insensitive,
/// trimmed uniqueness among LIVE rows — a byte-for-byte-different-case,
/// differently-spaced insert must be rejected.
#[test]
fn post_migration_index_is_case_insensitive_and_trims() {
    let (conn, _guard) = fresh_conn();
    create_pre_v044_schema(&conn);

    let now = 1_700_000_000_i64;
    insert_device(&conn, 1, Some("ОРГ-00-000007"), now);

    conn.execute_batch(V044_SQL).expect("apply V044 migration");

    let result = conn.execute(
        "INSERT INTO devices (inventory_number, created_at_utc, updated_at_utc, version) \
         VALUES (' орг-00-000007 ', ?1, ?1, 1)",
        [now + 100],
    );
    assert!(
        result.is_err(),
        "a case-differing, whitespace-padded duplicate must be rejected by \
         idx_devices_inventory_number_live: {result:?}"
    );

    // A genuinely distinct number must still succeed.
    let ok_result = conn.execute(
        "INSERT INTO devices (inventory_number, created_at_utc, updated_at_utc, version) \
         VALUES ('ОРГ-00-000099', ?1, ?1, 1)",
        [now + 200],
    );
    assert!(
        ok_result.is_ok(),
        "a distinct number must still be insertable: {ok_result:?}"
    );
}

#[test]
fn user_version_is_44_after_migration() {
    let (conn, _guard) = fresh_conn();
    create_pre_v044_schema(&conn);
    conn.execute_batch(V044_SQL).expect("apply V044 migration");

    let version: i64 = conn
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .expect("read user_version");
    assert_eq!(version, 44);
}

/// BE-WR-05: blank / whitespace-only numbers on several live devices must
/// not break the unique index creation — they are normalised to NULL, and
/// the index ignores blanks written afterwards.
#[test]
fn blank_numbers_become_null_and_do_not_block_the_index() {
    let (conn, _guard) = fresh_conn();
    create_pre_v044_schema(&conn);

    let now = 1_700_000_000_i64;
    insert_device(&conn, 1, Some(""), now);
    insert_device(&conn, 2, Some("   "), now + 1);
    insert_device(&conn, 3, Some("ИНВ-000001"), now + 2);

    conn.execute_batch(V044_SQL)
        .expect("V044 must apply with several blank numbers present");

    let blanks: Vec<Option<String>> = conn
        .prepare("SELECT inventory_number FROM devices WHERE id IN (1, 2) ORDER BY id")
        .expect("prepare")
        .query_map([], |r| r.get(0))
        .expect("query")
        .map(|r| r.expect("row"))
        .collect();
    assert_eq!(blanks, vec![None, None], "blank numbers must become NULL");

    for n in 0..2 {
        conn.execute(
            "INSERT INTO devices (inventory_number, created_at_utc, updated_at_utc, version) \
             VALUES ('', ?1, ?1, 1)",
            [now + 10 + n],
        )
        .expect("a stray blank number must not collide in the unique index");
    }
}

/// V044 through the REAL runner on a genuine V043 database (FK-off window,
/// one transaction per file) with a duplicate pair and blanks present.
#[test]
fn real_runner_upgrade_from_v043_dedups_and_indexes() {
    use trackly_infra::db::migrations;

    let dir = TempDir::new().expect("tempdir");
    let mut conn = Connection::open(dir.path().join("v044-real-runner.db")).expect("open");
    apply_writer_pragmas(&conn).expect("writer pragmas");
    migrations::run_up_to(&mut conn, 43).expect("run up to V043");

    let now = 1_700_000_000_i64;
    for (id, number) in [
        (1, Some("ОРГ-00-000007")),
        (2, Some(" орг-00-000007 ")),
        (3, Some("")),
        (4, Some("  ")),
    ] {
        conn.execute(
            "INSERT INTO devices (id, type_id, name, inventory_number, status_id, \
             created_at_utc, updated_at_utc) VALUES (?1, 1, 'Ноутбук', ?2, 1, ?3, ?3)",
            rusqlite::params![id, number, now],
        )
        .expect("seed device");
    }

    migrations::run(&mut conn).expect("upgrade V043 -> latest");

    let numbers: Vec<Option<String>> = conn
        .prepare("SELECT inventory_number FROM devices ORDER BY id")
        .expect("prepare")
        .query_map([], |r| r.get(0))
        .expect("query")
        .map(|r| r.expect("row"))
        .collect();
    assert_eq!(
        numbers,
        vec![
            Some("ОРГ-00-000007".to_string()),
            Some("ДУБЛЬ-000001 (орг-00-000007)".to_string()),
            None,
            None,
        ]
    );
}

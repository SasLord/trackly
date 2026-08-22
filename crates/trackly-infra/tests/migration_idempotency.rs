//! Integration test: refinery idempotency + WAL persistence (Pitfall #4).
//!
//! Manually opens a tempfile DB (not via `test_db()`) so we can run
//! migrations multiple times and then close + reopen the file to prove
//! WAL persists into the file header after the first migration's writes.

use rusqlite::Connection;
use tempfile::TempDir;

use trackly_infra::db::{migrations, pragmas};

#[test]
fn migrations_are_idempotent_and_wal_persists_across_reopens() {
    // Dynamic, not hardcoded (mirrors health_smoke.rs / test_db.rs pattern) —
    // avoids re-breaking this test every time a new migration lands.
    let expected_version = migrations::max_known_version();

    let dir = TempDir::new().expect("tempdir");
    let db_path = dir.path().join("idempotency.db");

    // First open: fresh DB, apply pragmas, run migrations — expect all known
    // migrations applied (V001..V{expected_version}).
    {
        let mut conn = Connection::open(&db_path).expect("open 1");
        pragmas::apply_writer_pragmas(&conn).expect("pragmas 1");
        let report = migrations::run(&mut conn).expect("run 1");
        assert_eq!(
            report.applied_count, expected_version as usize,
            "first run should apply all {expected_version}"
        );
        assert_eq!(report.schema_version, expected_version);

        // Second run on the same connection — no-op.
        let report2 = migrations::run(&mut conn).expect("run 2");
        assert_eq!(report2.applied_count, 0, "second run should be no-op");
        assert_eq!(report2.schema_version, expected_version);
        // conn dropped at end of this scope.
    }

    // Reopen the same file. WAL must still be active (persisted in file
    // header by first migration's writes — Pitfall #4 prevention).
    {
        let mut conn = Connection::open(&db_path).expect("open 2");
        pragmas::apply_writer_pragmas(&conn).expect("pragmas 2");

        let report3 = migrations::run(&mut conn).expect("run 3");
        assert_eq!(
            report3.applied_count, 0,
            "reopened DB already migrated → 0 applied"
        );
        assert_eq!(report3.schema_version, expected_version);

        let journal_mode: String = conn
            .pragma_query_value(None, "journal_mode", |r| r.get::<_, String>(0))
            .expect("read journal_mode");
        assert!(
            journal_mode.eq_ignore_ascii_case("wal"),
            "WAL must persist across reopens (Pitfall #4), got `{journal_mode}`"
        );
    }
}

/// Phase 39 Plan 01: places tree schema-only migration correctness (V037/V038).
///
/// Asserts the schema-only "existing DB upgrades without crashing" guarantee
/// on top of the full V001-V{max} migration chain: `locations` and every
/// `location_id`/`location` column are gone (PLC-04), and the new
/// `place_id`/`bulk_place_id`/`place_path_snapshot`/`place_id_override`
/// columns plus the `place_full_paths` view exist. Does not assert a
/// hardcoded migration count — mirrors the dynamic `max_known_version()`
/// pattern used above, so this test does not need updating when future
/// migrations land.
#[test]
fn places_migration_drops_locations_and_adds_place_columns() {
    let dir = TempDir::new().expect("tempdir");
    let db_path = dir.path().join("places-schema.db");

    let mut conn = Connection::open(&db_path).expect("open");
    pragmas::apply_writer_pragmas(&conn).expect("pragmas");
    migrations::run(&mut conn).expect("run migrations");

    // `locations` table is gone (PLC-04).
    let locations_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'locations'",
            [],
            |r| r.get(0),
        )
        .expect("query sqlite_master for locations");
    assert_eq!(
        locations_count, 0,
        "`locations` table must not exist after migration"
    );

    // devices: location_id gone, place_id present.
    let devices_columns = table_columns(&conn, "devices");
    assert!(
        !devices_columns.contains(&"location_id".to_string()),
        "devices.location_id must be dropped, got columns: {devices_columns:?}"
    );
    assert!(
        devices_columns.contains(&"place_id".to_string()),
        "devices.place_id must exist, got columns: {devices_columns:?}"
    );

    // cartridges: location gone, place_id present.
    let cartridges_columns = table_columns(&conn, "cartridges");
    assert!(
        !cartridges_columns.contains(&"location".to_string()),
        "cartridges.location must be dropped, got columns: {cartridges_columns:?}"
    );
    assert!(
        cartridges_columns.contains(&"place_id".to_string()),
        "cartridges.place_id must exist, got columns: {cartridges_columns:?}"
    );

    // acts: place_id / bulk_place_id / place_path_snapshot present, location_id gone.
    let acts_columns = table_columns(&conn, "acts");
    assert!(
        !acts_columns.contains(&"location_id".to_string()),
        "acts.location_id must be dropped, got columns: {acts_columns:?}"
    );
    for col in ["place_id", "bulk_place_id", "place_path_snapshot"] {
        assert!(
            acts_columns.contains(&col.to_string()),
            "acts.{col} must exist, got columns: {acts_columns:?}"
        );
    }

    // act_items: place_id_override present.
    let act_items_columns = table_columns(&conn, "act_items");
    assert!(
        act_items_columns.contains(&"place_id_override".to_string()),
        "act_items.place_id_override must exist, got columns: {act_items_columns:?}"
    );

    // place_full_paths view exists exactly once.
    let view_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'view' AND name = 'place_full_paths'",
            [],
            |r| r.get(0),
        )
        .expect("query sqlite_master for place_full_paths");
    assert_eq!(
        view_count, 1,
        "place_full_paths view must exist exactly once"
    );
}

/// `PRAGMA table_info(table)` column names, via `Connection::pragma`.
fn table_columns(conn: &Connection, table: &str) -> Vec<String> {
    let mut names = Vec::new();
    conn.pragma(None, "table_info", table, |row| {
        let name: String = row.get("name")?;
        names.push(name);
        Ok(())
    })
    .expect("pragma table_info");
    names
}

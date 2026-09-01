//! Integration test: V040 `place_movements` schema-only migration correctness
//! (Phase 40 Plan 01, HST-01).
//!
//! Mirrors `migration_idempotency.rs`'s harness pattern (fresh tempfile DB, apply
//! pragmas + all embedded migrations, then inspect the resulting schema) rather than
//! its assertions — this file asserts the table, its 5 indexes, and the empty-on-fresh-DB
//! invariant (D-02: no backfill from `audit_log`).

use rusqlite::Connection;
use tempfile::TempDir;

use trackly_infra::db::{migrations, pragmas};

/// Fresh DB with all embedded migrations applied.
fn fresh_migrated_db(dir: &TempDir, file_name: &str) -> Connection {
    let db_path = dir.path().join(file_name);
    let mut conn = Connection::open(&db_path).expect("open");
    pragmas::apply_writer_pragmas(&conn).expect("pragmas");
    migrations::run(&mut conn).expect("run migrations");
    conn
}

#[test]
fn place_movements_v040_creates_table_and_indexes() {
    let dir = TempDir::new().expect("tempdir");
    let conn = fresh_migrated_db(&dir, "place-movements-schema.db");

    let table_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'place_movements'",
            [],
            |r| r.get(0),
        )
        .expect("query sqlite_master for place_movements table");
    assert_eq!(
        table_count, 1,
        "place_movements table must exist exactly once"
    );

    let index_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND tbl_name = 'place_movements'",
            [],
            |r| r.get(0),
        )
        .expect("query sqlite_master for place_movements indexes");
    assert_eq!(
        index_count, 5,
        "place_movements must have exactly 5 indexes, got {index_count}"
    );

    let user_version: i64 = conn
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .expect("read user_version");
    assert!(
        user_version >= 40,
        "user_version must be at least 40, got {user_version}"
    );
}

#[test]
fn place_movements_starts_empty() {
    let dir = TempDir::new().expect("tempdir");
    let conn = fresh_migrated_db(&dir, "place-movements-empty.db");

    let row_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM place_movements", [], |r| r.get(0))
        .expect("query place_movements row count");
    assert_eq!(
        row_count, 0,
        "place_movements must start empty (D-02: no backfill from audit_log)"
    );
}

#[test]
fn place_movements_migration_reruns_idempotently() {
    let dir = TempDir::new().expect("tempdir");
    let db_path = dir.path().join("place-movements-idempotent.db");

    let mut conn = Connection::open(&db_path).expect("open");
    pragmas::apply_writer_pragmas(&conn).expect("pragmas");

    let report1 = migrations::run(&mut conn).expect("run 1");
    let version_after_first = report1.schema_version;

    // Re-run against the same connection: must be a no-op, no error.
    let report2 = migrations::run(&mut conn).expect("run 2");
    assert_eq!(
        report2.applied_count, 0,
        "second run must apply zero migrations"
    );
    assert_eq!(
        report2.schema_version, version_after_first,
        "user_version must not change on idempotent re-run"
    );

    // Independent fresh apply against a second connection must land on the same version.
    let dir2 = TempDir::new().expect("tempdir 2");
    let conn2 = fresh_migrated_db(&dir2, "place-movements-idempotent-2.db");
    let version2: i64 = conn2
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .expect("read user_version 2");
    assert_eq!(
        version2, version_after_first as i64,
        "independent fresh migration run must reach the same schema version"
    );
}

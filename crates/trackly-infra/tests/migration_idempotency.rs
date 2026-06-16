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
    let dir = TempDir::new().expect("tempdir");
    let db_path = dir.path().join("idempotency.db");

    // First open: fresh DB, apply pragmas, run migrations — expect 27 applied (V001..V027).
    {
        let mut conn = Connection::open(&db_path).expect("open 1");
        pragmas::apply_writer_pragmas(&conn).expect("pragmas 1");
        let report = migrations::run(&mut conn).expect("run 1");
        assert_eq!(report.applied_count, 27, "first run should apply all 27");
        assert_eq!(report.schema_version, 27);

        // Second run on the same connection — no-op.
        let report2 = migrations::run(&mut conn).expect("run 2");
        assert_eq!(report2.applied_count, 0, "second run should be no-op");
        assert_eq!(report2.schema_version, 27);
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
        assert_eq!(report3.schema_version, 27);

        let journal_mode: String = conn
            .pragma_query_value(None, "journal_mode", |r| r.get::<_, String>(0))
            .expect("read journal_mode");
        assert!(
            journal_mode.eq_ignore_ascii_case("wal"),
            "WAL must persist across reopens (Pitfall #4), got `{journal_mode}`"
        );
    }
}

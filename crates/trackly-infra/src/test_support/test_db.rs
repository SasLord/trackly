//! `test_db()` — the canonical SQLite fixture for integration tests.
//!
//! Returns a writable rusqlite `Connection` with all writer PRAGMAs applied
//! and every refinery migration (V001..V025) already run. The caller MUST
//! keep the returned `TempDir` alive for the lifetime of the connection;
//! dropping it removes the on-disk DB file and the WAL/SHM sidecars.
//!
//! **Why tempfile, not `:memory:`?** Per D-Test-01: `:memory:` does NOT
//! model WAL behaviour (no `.db-wal`/`.db-shm` sidecars, no busy_timeout
//! semantics under contention). We test against real WAL.

use rusqlite::Connection;
use tempfile::TempDir;

use crate::db::{migrations, pragmas};

/// Create a tempfile-backed SQLite DB with writer pragmas + all migrations
/// applied. Returns `(connection, tempdir_guard)` — keep the guard alive.
///
/// Panics on any error: this is test infrastructure, failures here mean
/// the test harness itself is broken.
pub fn test_db() -> (Connection, TempDir) {
    let dir = TempDir::new().expect("create tempdir for test DB");
    let db_path = dir.path().join("test.db");
    let mut conn = Connection::open(&db_path).expect("open test DB");
    pragmas::apply_writer_pragmas(&conn).expect("apply writer pragmas");
    migrations::run(&mut conn).expect("run migrations");
    (conn, dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_db_returns_fully_migrated_connection() {
        let (conn, _guard) = test_db();
        let user_version: i64 = conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .expect("read user_version");
        assert_eq!(user_version, 27);

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM device_types", [], |r| r.get(0))
            .expect("count device_types");
        assert_eq!(count, 2, "expected 2 seeded device_types");

        // Phase 6: oid_profiles seeded by V021
        let oid_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM oid_profiles", [], |r| r.get(0))
            .expect("count oid_profiles");
        assert_eq!(oid_count, 5, "expected 5 seeded oid_profiles (pantum/kyocera/hp/canon/rfc3805)");

        // Phase 6: cartridges.current_printer_device_id added by V025
        let col_exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('cartridges') WHERE name = 'current_printer_device_id'",
                [],
                |r| r.get(0),
            )
            .expect("check cartridges column");
        assert_eq!(col_exists, 1, "cartridges.current_printer_device_id column must exist (V025)");
    }
}

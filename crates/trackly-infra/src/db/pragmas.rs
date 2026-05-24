//! PRAGMA discipline for writer and reader connections.
//!
//! `apply_writer_pragmas` is called once on every writer-side connection at
//! open time (Plan 04's single-writer task; Plan 03 also calls it for the
//! `test_support::test_db` helper). `apply_reader_pragmas` is called on
//! every reader-pool connection (Plan 04).
//!
//! **Why not in migration SQL?** PRAGMAs like `journal_mode`, `busy_timeout`,
//! and `foreign_keys` are per-connection (not per-database) and would have
//! ambiguous semantics inside a refinery transaction. We apply them
//! explicitly at connection-open time so every connection has the same
//! enforced state. `journal_mode = WAL` is the one exception that DOES
//! persist into the database file header, but it has to be set BEFORE any
//! transaction starts — refinery wraps each migration in a transaction by
//! default, so we'd be too late.
//!
//! See `.planning/research/PITFALLS.md` #2 (SQLite locked) and #4 (WAL persistence).

use rusqlite::Connection;
use trackly_core::error::AppError;

/// 128 MiB — `mmap_size` for memory-mapped I/O on read paths.
const MMAP_SIZE_BYTES: i64 = 128 * 1024 * 1024;

/// `wal_autocheckpoint` — checkpoint after every 1000 frames in the WAL.
const WAL_AUTOCHECKPOINT_FRAMES: i64 = 1000;

/// `busy_timeout` — milliseconds rusqlite waits for the database lock before
/// returning `SQLITE_BUSY`. 5 s is comfortable for 20-LAN-user concurrency.
const BUSY_TIMEOUT_MS: i64 = 5000;

/// Apply writer-side PRAGMAs. Must be called BEFORE any transaction starts.
///
/// Order matters: `journal_mode=WAL` first (persists to file header on first
/// write), then the rest. See RESEARCH §Pattern 3.
pub fn apply_writer_pragmas(conn: &Connection) -> Result<(), AppError> {
    // journal_mode is a special PRAGMA: pragma_update doesn't read back
    // the value the engine actually used. We invoke it via `pragma_query`
    // which performs the SET and exposes the resulting mode.
    let mut journal_mode_set = String::new();
    conn.pragma_update_and_check(None, "journal_mode", "WAL", |row| {
        journal_mode_set = row.get::<_, String>(0)?;
        Ok(())
    })
    .map_err(|e| AppError::Internal {
        source_chain: format!("pragma journal_mode=WAL failed: {e}"),
    })?;
    if !journal_mode_set.eq_ignore_ascii_case("wal") {
        return Err(AppError::Internal {
            source_chain: format!(
                "pragma journal_mode=WAL returned unexpected mode '{journal_mode_set}'"
            ),
        });
    }

    set_pragma(conn, "synchronous", "NORMAL")?;
    set_pragma(conn, "busy_timeout", BUSY_TIMEOUT_MS)?;
    set_pragma(conn, "foreign_keys", "ON")?;
    set_pragma(conn, "wal_autocheckpoint", WAL_AUTOCHECKPOINT_FRAMES)?;
    set_pragma(conn, "temp_store", "MEMORY")?;
    set_pragma(conn, "mmap_size", MMAP_SIZE_BYTES)?;

    Ok(())
}

/// Apply reader-side PRAGMAs. `journal_mode` is already persisted in the
/// file header by the writer; `synchronous` is a write-only setting.
pub fn apply_reader_pragmas(conn: &Connection) -> Result<(), AppError> {
    set_pragma(conn, "busy_timeout", BUSY_TIMEOUT_MS)?;
    set_pragma(conn, "foreign_keys", "ON")?;
    set_pragma(conn, "temp_store", "MEMORY")?;
    set_pragma(conn, "mmap_size", MMAP_SIZE_BYTES)?;
    set_pragma(conn, "query_only", "ON")?;
    Ok(())
}

fn set_pragma<V: rusqlite::ToSql + std::fmt::Display + Copy>(
    conn: &Connection,
    name: &str,
    value: V,
) -> Result<(), AppError> {
    conn.pragma_update(None, name, value)
        .map_err(|e| AppError::Internal {
            source_chain: format!("pragma {name}={value} failed: {e}"),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn fresh_tempfile_conn() -> (Connection, TempDir) {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("pragmas-test.db");
        let conn = Connection::open(&path).expect("open");
        (conn, dir)
    }

    #[test]
    fn apply_writer_pragmas_sets_wal_busy_timeout_and_fk() {
        let (conn, _guard) = fresh_tempfile_conn();
        apply_writer_pragmas(&conn).expect("writer pragmas");

        let journal_mode: String = conn
            .pragma_query_value(None, "journal_mode", |r| r.get::<_, String>(0))
            .expect("read journal_mode");
        assert!(
            journal_mode.eq_ignore_ascii_case("wal"),
            "journal_mode should be wal, got {journal_mode}"
        );

        let busy_timeout: i64 = conn
            .pragma_query_value(None, "busy_timeout", |r| r.get::<_, i64>(0))
            .expect("read busy_timeout");
        assert_eq!(busy_timeout, 5000);

        let foreign_keys: i64 = conn
            .pragma_query_value(None, "foreign_keys", |r| r.get::<_, i64>(0))
            .expect("read foreign_keys");
        assert_eq!(foreign_keys, 1);

        let synchronous: i64 = conn
            .pragma_query_value(None, "synchronous", |r| r.get::<_, i64>(0))
            .expect("read synchronous");
        // 1 == NORMAL
        assert_eq!(synchronous, 1);

        let wal_autockpt: i64 = conn
            .pragma_query_value(None, "wal_autocheckpoint", |r| r.get::<_, i64>(0))
            .expect("read wal_autocheckpoint");
        assert_eq!(wal_autockpt, 1000);
    }

    #[test]
    fn apply_reader_pragmas_sets_query_only_and_fk() {
        let (conn, _guard) = fresh_tempfile_conn();
        apply_reader_pragmas(&conn).expect("reader pragmas");

        let query_only: i64 = conn
            .pragma_query_value(None, "query_only", |r| r.get::<_, i64>(0))
            .expect("read query_only");
        assert_eq!(query_only, 1);

        let foreign_keys: i64 = conn
            .pragma_query_value(None, "foreign_keys", |r| r.get::<_, i64>(0))
            .expect("read foreign_keys");
        assert_eq!(foreign_keys, 1);

        let busy_timeout: i64 = conn
            .pragma_query_value(None, "busy_timeout", |r| r.get::<_, i64>(0))
            .expect("read busy_timeout");
        assert_eq!(busy_timeout, 5000);
    }
}

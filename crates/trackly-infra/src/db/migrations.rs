//! Refinery embed + runner wrapper.
//!
//! `embed_migrations!` resolves at compile time relative to the crate's
//! `Cargo.toml`. From `crates/trackly-infra/Cargo.toml`, the workspace-root
//! `migrations/` directory is `../../migrations`.
//!
//! Each migration ends with `PRAGMA user_version = N;` (D-Migrations-02).
//! After running, we read the persisted `PRAGMA user_version` to confirm
//! the schema version. Downgrade protection (refusing to open a DB whose
//! `user_version` exceeds the embedded last migration) is implemented in
//! Plan 04 inside `AppCtx::build` — this module just runs migrations.

use refinery::embed_migrations;
use rusqlite::Connection;
use trackly_core::error::AppError;

embed_migrations!("../../migrations");

/// Максимальный `user_version`, который знает текущий бинарь — посчитан
/// в рантайме из embedded списка миграций. Используется в Plan 04
/// `AppCtx::build` для probe-read downgrade-протекции.
///
/// Реализовано как `fn` (не `const`), потому что refinery API не предоставляет
/// `const fn`-доступ к версиям миграций.
pub fn max_known_version() -> u32 {
    let max_i32: i32 = migrations::runner()
        .get_migrations()
        .iter()
        .map(|m| m.version())
        .max()
        .unwrap_or(0);
    u32::try_from(max_i32).expect("migration version must be non-negative")
}

/// Outcome of a `run` invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MigrationReport {
    /// `PRAGMA user_version` after refinery finishes.
    pub schema_version: u32,
    /// Number of migrations refinery actually applied during this call.
    /// On a freshly-created DB this equals the total number of migration
    /// files (12 at the time of writing); on a reopened, fully-migrated DB
    /// this is 0.
    pub applied_count: usize,
}

/// Run all embedded migrations against the given writable connection.
///
/// Refinery 0.9 defaults to one transaction per migration (`set_grouped(false)`),
/// which lets `journal_mode=WAL` (set by `pragmas::apply_writer_pragmas` BEFORE
/// this call) commit to the file header on the first migration's transaction.
pub fn run(conn: &mut Connection) -> Result<MigrationReport, AppError> {
    let report = migrations::runner()
        .run(conn)
        .map_err(|e| AppError::Internal {
            source_chain: format!("refinery migration failed: {e}"),
        })?;

    let schema_version: u32 = conn
        .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
        .map_err(|e| AppError::Internal {
            source_chain: format!("read PRAGMA user_version failed: {e}"),
        })?
        .try_into()
        .map_err(|e| AppError::Internal {
            source_chain: format!("user_version negative or too large: {e}"),
        })?;

    Ok(MigrationReport {
        schema_version,
        applied_count: report.applied_migrations().len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::pragmas::apply_writer_pragmas;
    use tempfile::TempDir;

    fn fresh_conn() -> (Connection, TempDir) {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("migrations-test.db");
        let conn = Connection::open(&path).expect("open");
        apply_writer_pragmas(&conn).expect("writer pragmas");
        (conn, dir)
    }

    #[test]
    fn run_applies_all_known_migrations_on_fresh_db() {
        let (mut conn, _guard) = fresh_conn();
        let report = run(&mut conn).expect("run migrations");
        let expected = max_known_version();
        assert_eq!(
            report.schema_version, expected,
            "expected schema_version {expected}"
        );
        assert_eq!(
            report.applied_count, expected as usize,
            "expected {expected} migrations applied"
        );
    }

    #[test]
    fn max_known_version_returns_current() {
        // Version increases as migrations are added; just verify it is at least 17
        assert!(max_known_version() >= 17);
    }

    #[test]
    fn run_is_idempotent_on_same_connection() {
        let (mut conn, _guard) = fresh_conn();
        let first = run(&mut conn).expect("first run");
        let expected = max_known_version();
        assert_eq!(first.applied_count, expected as usize);

        let second = run(&mut conn).expect("second run");
        assert_eq!(
            second.applied_count, 0,
            "second run should be a no-op (0 applied)"
        );
        assert_eq!(second.schema_version, expected);
    }
}

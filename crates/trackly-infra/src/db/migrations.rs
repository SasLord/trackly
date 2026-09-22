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

/// One row of `PRAGMA foreign_key_check`: `(table, rowid, parent, fkid)`.
type FkViolation = (String, Option<i64>, String, i64);

fn fk_violations(conn: &Connection) -> Result<std::collections::BTreeSet<FkViolation>, AppError> {
    let mut stmt = conn
        .prepare("PRAGMA foreign_key_check")
        .map_err(|e| AppError::Internal {
            source_chain: format!("prepare foreign_key_check failed: {e}"),
        })?;
    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, Option<i64>>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, i64>(3)?,
            ))
        })
        .map_err(|e| AppError::Internal {
            source_chain: format!("foreign_key_check failed: {e}"),
        })?;
    let mut out = std::collections::BTreeSet::new();
    for row in rows {
        out.insert(row.map_err(|e| AppError::Internal {
            source_chain: format!("foreign_key_check row failed: {e}"),
        })?);
    }
    Ok(out)
}

/// Highest migration version already recorded in refinery's history table,
/// or `None` on a fresh DB (history table not created yet).
fn last_applied_version(conn: &Connection) -> Result<Option<i64>, AppError> {
    let has_history: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master \
              WHERE type = 'table' AND name = 'refinery_schema_history')",
            [],
            |r| r.get(0),
        )
        .map_err(|e| AppError::Internal {
            source_chain: format!("probe refinery_schema_history failed: {e}"),
        })?;
    if !has_history {
        return Ok(None);
    }
    conn.query_row(
        "SELECT MAX(version) FROM refinery_schema_history",
        [],
        |r| r.get::<_, Option<i64>>(0),
    )
    .map_err(|e| AppError::Internal {
        source_chain: format!("read refinery_schema_history failed: {e}"),
    })
}

fn set_foreign_keys(conn: &Connection, on: bool) -> Result<(), AppError> {
    conn.pragma_update(None, "foreign_keys", if on { "ON" } else { "OFF" })
        .map_err(|e| AppError::Internal {
            source_chain: format!("PRAGMA foreign_keys toggle failed: {e}"),
        })?;
    let actual: i64 = conn
        .pragma_query_value(None, "foreign_keys", |r| r.get(0))
        .map_err(|e| AppError::Internal {
            source_chain: format!("read PRAGMA foreign_keys failed: {e}"),
        })?;
    if (actual == 1) != on {
        // SQLite silently ignores this PRAGMA inside an open transaction —
        // refuse to run table-rebuild migrations with the wrong FK mode.
        return Err(AppError::Internal {
            source_chain: format!(
                "PRAGMA foreign_keys={} did not take effect (open transaction?)",
                if on { "ON" } else { "OFF" }
            ),
        });
    }
    Ok(())
}

/// Run all embedded migrations against the given writable connection.
///
/// Refinery 0.9 defaults to one transaction per migration (`set_grouped(false)`),
/// which lets `journal_mode=WAL` (set by `pragmas::apply_writer_pragmas` BEFORE
/// this call) commit to the file header on the first migration's transaction.
///
/// ## Foreign keys are disabled on the connection for the whole run
///
/// Refinery executes every migration file inside `conn.transaction()`, and
/// SQLite ignores `PRAGMA foreign_keys` inside a transaction — so a
/// `PRAGMA foreign_keys = OFF;` line inside a migration file is a no-op.
/// With FKs left ON, the 12-step table rebuild (`CREATE x_new; INSERT ...
/// SELECT; DROP TABLE x; ALTER TABLE x_new RENAME TO x`) turns `DROP TABLE x`
/// into an implicit `DELETE FROM x` that fires FK actions on child tables:
/// `ON DELETE CASCADE` children are wiped (V042 would erase every
/// `act_items` row), `SET NULL` links are cleared (`place_movements.act_id`),
/// and `RESTRICT`/`NO ACTION` children abort the upgrade (returns'
/// `parent_act_id`, `requests.completed_cartridge_id` for V043).
///
/// So when there is at least one pending migration, FKs are switched OFF on
/// the connection (outside any transaction) BEFORE refinery starts, then
/// `PRAGMA foreign_key_check` is compared against a pre-run baseline and FKs
/// are switched back ON. Any violation that did not exist before the run
/// fails the startup loudly — a rebuild that broke referential integrity is
/// never silently accepted. Pre-existing violations (legacy data) are not
/// this run's fault and do not block startup.
pub fn run(conn: &mut Connection) -> Result<MigrationReport, AppError> {
    run_to(conn, None)
}

/// Test hook: run embedded migrations only up to (and including) `version`,
/// through the SAME runner path as [`run`] (FK-off window + integrity
/// check). Lets upgrade tests stand up a real pre-VNNN database, seed it,
/// and then run the remaining migrations exactly as `AppCtx::build` would.
#[doc(hidden)]
pub fn run_up_to(conn: &mut Connection, version: u32) -> Result<MigrationReport, AppError> {
    run_to(conn, Some(version))
}

fn build_runner(target: Option<u32>) -> Result<refinery::Runner, AppError> {
    let runner = migrations::runner();
    Ok(match target {
        None => runner,
        Some(v) => {
            let v = refinery::SchemaVersion::try_from(v).map_err(|e| AppError::Internal {
                source_chain: format!("migration target {v} out of range: {e}"),
            })?;
            runner.set_target(refinery::Target::Version(v))
        }
    })
}

fn run_to(conn: &mut Connection, target: Option<u32>) -> Result<MigrationReport, AppError> {
    let goal = i64::from(target.unwrap_or_else(max_known_version));
    let pending = match last_applied_version(conn)? {
        None => true,
        Some(v) => v < goal,
    };

    let report = if pending {
        let fk_was_on: bool = conn
            .pragma_query_value(None, "foreign_keys", |r| r.get::<_, i64>(0))
            .map_err(|e| AppError::Internal {
                source_chain: format!("read PRAGMA foreign_keys failed: {e}"),
            })?
            == 1;
        let baseline = fk_violations(conn)?;
        set_foreign_keys(conn, false)?;

        let result = build_runner(target)?.run(conn);

        // Integrity check BEFORE re-enabling FKs (re-enabling does not
        // validate existing rows either, but keep the order explicit).
        let after = fk_violations(conn);
        let restore = set_foreign_keys(conn, fk_was_on);

        let report = result.map_err(|e| AppError::Internal {
            source_chain: format!("refinery migration failed: {e}"),
        })?;
        let after = after?;
        restore?;

        let new_violations: Vec<&FkViolation> = after.difference(&baseline).collect();
        if !new_violations.is_empty() {
            let sample: Vec<String> = new_violations
                .iter()
                .take(10)
                .map(|(table, rowid, parent, _)| {
                    format!("{table}(rowid={}) -> {parent}", rowid.unwrap_or(-1))
                })
                .collect();
            return Err(AppError::Internal {
                source_chain: format!(
                    "migrations left {} new foreign key violation(s): {}",
                    new_violations.len(),
                    sample.join(", ")
                ),
            });
        }
        report
    } else {
        build_runner(target)?
            .run(conn)
            .map_err(|e| AppError::Internal {
                source_chain: format!("refinery migration failed: {e}"),
            })?
    };

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

    /// The actual V032 migration SQL, embedded at compile time so this test
    /// exercises the shipped file rather than a hand-copied duplicate.
    const V032_SQL: &str =
        include_str!("../../../../migrations/V032__cartridge_model_compatibility_printer_name.sql");

    /// V032 data-transform coverage (CR-01 / IN-05).
    ///
    /// The integration tests in `cartridges_crud.rs` only ever seed
    /// compatibility via `model_create` against the post-V032 single-column
    /// schema, so the V005 -> V032 `TRIM(printer_brand || ' ' || printer_model)`
    /// transform on pre-existing rows was unverified. This test stands up the
    /// V005-shaped tables, seeds legacy rows (including an empty/whitespace
    /// row), runs the real V032 SQL, and asserts:
    ///   - a populated legacy row survives with the concatenated printer_name,
    ///   - an empty/whitespace-only legacy row is DROPPED (so the D-05
    ///     "no compatibility => compatible with any printer" pass-through is
    ///     restored for that model rather than silently broken).
    #[test]
    fn v032_data_transform_drops_empty_and_preserves_populated() {
        let (conn, _guard) = fresh_conn();

        // Minimal V005-era schema needed by V032 (FK target + the two tables
        // V032 rebuilds/drops). FK enforcement is toggled OFF inside V032 itself.
        conn.execute_batch(
            "CREATE TABLE cartridge_models (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 brand TEXT NOT NULL,
                 model TEXT NOT NULL
             );
             CREATE TABLE cartridge_model_compatibility (
                 id                  INTEGER PRIMARY KEY AUTOINCREMENT,
                 cartridge_model_id  INTEGER NOT NULL REFERENCES cartridge_models(id) ON DELETE CASCADE,
                 printer_brand       TEXT NOT NULL,
                 printer_model       TEXT NOT NULL
             );
             CREATE TABLE printer_cartridge_models (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 device_id INTEGER NOT NULL,
                 cartridge_model_id INTEGER NOT NULL
             );
             INSERT INTO cartridge_models (id, brand, model) VALUES
                 (1, 'Pantum', 'TL-5120X'),
                 (2, 'HP', '85A');
             -- Populated: should survive as 'Pantum BM5100'.
             INSERT INTO cartridge_model_compatibility
                 (cartridge_model_id, printer_brand, printer_model)
                 VALUES (1, 'Pantum', 'BM5100');
             -- Brand-only: should survive as 'Pantum' (trailing-space collapsed).
             INSERT INTO cartridge_model_compatibility
                 (cartridge_model_id, printer_brand, printer_model)
                 VALUES (1, 'Pantum', '');
             -- Empty/whitespace: should be DROPPED (model 2 ends up with 0 rows).
             INSERT INTO cartridge_model_compatibility
                 (cartridge_model_id, printer_brand, printer_model)
                 VALUES (2, '', '');",
        )
        .expect("seed V005-shaped schema + rows");

        // Run the real V032 file.
        conn.execute_batch(V032_SQL).expect("apply V032 transform");

        // Model 1: both populated rows preserved with concatenated names.
        let mut names: Vec<String> = {
            let mut stmt = conn
                .prepare(
                    "SELECT printer_name FROM cartridge_model_compatibility \
                     WHERE cartridge_model_id = 1 ORDER BY printer_name",
                )
                .expect("prepare model 1 query");
            let rows = stmt
                .query_map([], |r| r.get::<_, String>(0))
                .expect("query model 1");
            rows.map(|r| r.expect("row")).collect()
        };
        names.sort();
        assert_eq!(
            names,
            vec!["Pantum".to_string(), "Pantum BM5100".to_string()],
            "populated legacy rows must survive with TRIM'd concatenated names"
        );

        // Model 2: empty/whitespace-only row dropped => zero rows.
        let model2_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM cartridge_model_compatibility WHERE cartridge_model_id = 2",
                [],
                |r| r.get(0),
            )
            .expect("count model 2 rows");
        assert_eq!(
            model2_count, 0,
            "empty/whitespace legacy row must be dropped so D-05 pass-through is restored"
        );

        // No empty-string printer_name leaked through anywhere.
        let empty_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM cartridge_model_compatibility WHERE TRIM(printer_name) = ''",
                [],
                |r| r.get(0),
            )
            .expect("count empty names");
        assert_eq!(
            empty_count, 0,
            "no empty printer_name rows may survive V032"
        );
    }
}

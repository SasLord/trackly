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

/// Stable signature key of an FK violation, IGNORING `rowid` (WR-01 in
/// `40.3-REVIEW.md`): a table rebuild (`CREATE TABLE new -> INSERT SELECT ->
/// DROP -> RENAME`, project norm — V042/V043/V044) can reassign `rowid` for
/// existing rows, so comparing raw `(table, rowid, parent, fkid)` tuples
/// either falsely reports an already-accepted legacy violation as "new"
/// (rowid changed, tuple no longer matches the baseline), or masks a
/// genuinely new violation if it lands on a reused SQLite rowid. Comparing
/// by (table, parent, fkid) MULTISET (tracked via count, not a plain set) is
/// robust to both cases.
type FkSignature = (String, String, i64);

/// Counts occurrences of each `FkSignature` within a violation set —
/// multiplicity matters: two distinct rows with the same signature (e.g. two
/// orphan `devices` rows both pointing at a missing `places` parent) must not
/// collapse into "one violation" when comparing against the baseline.
fn signature_counts(
    violations: &std::collections::BTreeSet<FkViolation>,
) -> std::collections::BTreeMap<FkSignature, usize> {
    let mut out = std::collections::BTreeMap::new();
    for (table, _rowid, parent, fkid) in violations {
        *out.entry((table.clone(), parent.clone(), *fkid))
            .or_insert(0) += 1;
    }
    out
}

/// Rowid-stable replacement for a raw set-difference of `after` against
/// `baseline`: returns the elements of `after` whose (table, parent, fkid)
/// signature occurs more often than in `baseline`. A table rebuild that
/// reassigns `rowid` for an already-baselined violation leaves the signature
/// count unchanged (not reported); a genuinely new violation — even one that
/// happens to land on a reused rowid — grows the signature count and is
/// still reported.
fn violations_with_grown_signature<'a>(
    after: &'a std::collections::BTreeSet<FkViolation>,
    baseline: &std::collections::BTreeSet<FkViolation>,
) -> Vec<&'a FkViolation> {
    let after_counts = signature_counts(after);
    let baseline_counts = signature_counts(baseline);
    let mut remaining: std::collections::BTreeMap<FkSignature, usize> =
        std::collections::BTreeMap::new();
    for (sig, count) in &after_counts {
        let base = baseline_counts.get(sig).copied().unwrap_or(0);
        if *count > base {
            remaining.insert(sig.clone(), count - base);
        }
    }
    if remaining.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for v @ (table, _rowid, parent, fkid) in after {
        let sig = (table.clone(), parent.clone(), *fkid);
        if let Some(left) = remaining.get_mut(&sig) {
            if *left > 0 {
                out.push(v);
                *left -= 1;
            }
        }
    }
    out
}

/// `app_settings` key under which the persistent FK-violation baseline is
/// stored (guarded single-key pattern, same shape as
/// `place_path_settings.rs` / `low_stock_threshold` in
/// `tauri_cmds/settings_org.rs` — an internal setting key, not a new
/// table/migration; D-07 discretion).
const FK_BASELINE_SETTING_KEY: &str = "migration_fk_baseline";

/// Serializes an `FkViolation` baseline into the `app_settings.value` TEXT
/// column: one violation per line, fields separated by `\t` (table/parent
/// names and rowids never contain tabs), `rowid = None` encoded as an empty
/// field. Symmetric with [`parse_fk_baseline`].
fn format_fk_baseline(violations: &std::collections::BTreeSet<FkViolation>) -> String {
    violations
        .iter()
        .map(|(table, rowid, parent, fkid)| {
            format!(
                "{table}\t{}\t{parent}\t{fkid}",
                rowid.map(|v| v.to_string()).unwrap_or_default()
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Parses the serialized form produced by [`format_fk_baseline`]. Any
/// malformed line is dropped rather than panicking — a corrupted persisted
/// baseline degrades to "fewer known-old violations" (stricter, not a
/// crash), never blocks startup on its own.
fn parse_fk_baseline(raw: &str) -> std::collections::BTreeSet<FkViolation> {
    raw.lines()
        .filter(|line| !line.is_empty())
        .filter_map(|line| {
            let mut parts = line.split('\t');
            let table = parts.next()?.to_string();
            let rowid = match parts.next()? {
                "" => None,
                s => s.parse::<i64>().ok(),
            };
            let parent = parts.next()?.to_string();
            let fkid = parts.next()?.parse::<i64>().ok()?;
            Some((table, rowid, parent, fkid))
        })
        .collect()
}

/// Guarded-read of the persisted FK-violation baseline from `app_settings`
/// (pattern: `place_path_settings.rs::read_setting`). Returns `None` on ANY
/// read failure — missing row, missing `app_settings` table entirely (the
/// very first run, before migrations have created it), or any other error —
/// never a panic. `None` means "no persisted baseline yet", handled by the
/// caller as "fall back to a fresh `fk_violations()` snapshot".
fn read_persisted_fk_baseline(
    conn: &Connection,
) -> Option<std::collections::BTreeSet<FkViolation>> {
    let raw: String = conn
        .query_row(
            "SELECT value FROM app_settings WHERE key = ?1",
            [FK_BASELINE_SETTING_KEY],
            |r| r.get::<_, String>(0),
        )
        .ok()?;
    Some(parse_fk_baseline(&raw))
}

/// Upsert the FK-violation baseline into `app_settings`
/// (`ON CONFLICT` pattern from `tauri_cmds/settings_org.rs` /
/// `services/backup_service.rs`). Called only when the current run found NO
/// new violations relative to the previous baseline — this is the self-heal
/// step: once integrity is clean again, the baseline is pulled back down.
fn persist_fk_baseline(
    conn: &Connection,
    violations: &std::collections::BTreeSet<FkViolation>,
) -> Result<(), AppError> {
    let value = format_fk_baseline(violations);
    // `migrations.rs` operates on a raw `&Connection`/`&mut Connection`
    // below the `Clock` boundary (trackly-core doesn't own this module) —
    // wall-clock seconds via `SystemTime` is fine here, this timestamp only
    // records "a baseline row exists", it carries no user-facing value.
    let now: i64 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    conn.execute(
        "INSERT INTO app_settings (key, value, created_at_utc, updated_at_utc) \
         VALUES (?1, ?2, ?3, ?3) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at_utc = excluded.updated_at_utc",
        rusqlite::params![FK_BASELINE_SETTING_KEY, value, now],
    )
    .map_err(|e| AppError::Internal {
        source_chain: format!("persist fk baseline failed: {e}"),
    })?;
    Ok(())
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

    // Read the FK-violation baseline UNCONDITIONALLY (not only when
    // `pending`) — this is the N-4 fix. A persisted baseline from a
    // previous run wins; if none exists yet (very first ever run, before
    // `app_settings` may even exist), fall back to a fresh snapshot taken
    // BEFORE this run's migrations touch anything, i.e. today's
    // "pre-existing violations don't block startup" behaviour.
    let baseline = match read_persisted_fk_baseline(conn) {
        Some(b) => b,
        None => fk_violations(conn)?,
    };

    let (report, after) = if pending {
        let fk_was_on: bool = conn
            .pragma_query_value(None, "foreign_keys", |r| r.get::<_, i64>(0))
            .map_err(|e| AppError::Internal {
                source_chain: format!("read PRAGMA foreign_keys failed: {e}"),
            })?
            == 1;
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

        (report, after)
    } else {
        // No pending migrations this run — but still check integrity
        // UNCONDITIONALLY against the persisted baseline (N-4 fix): a
        // violation introduced by a PRIOR run's migrations must keep
        // blocking every subsequent start, not just the run that created
        // it.
        let report = build_runner(target)?
            .run(conn)
            .map_err(|e| AppError::Internal {
                source_chain: format!("refinery migration failed: {e}"),
            })?;
        let after = fk_violations(conn)?;
        (report, after)
    };

    let new_violations: Vec<&FkViolation> = violations_with_grown_signature(&after, &baseline);
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
    // Self-heal: no new violations relative to the previous baseline, so
    // persist `after` as the new baseline. If an admin manually cleaned up
    // legacy data, the next clean start pulls the baseline back down
    // instead of leaving it stuck on a stale "dirty" snapshot forever.
    //
    // Guarded / best-effort, symmetric with `read_persisted_fk_baseline`'s
    // `.ok()`: `app_settings` does not exist yet before V016 runs, so
    // `run_up_to(conn, v)` with `v < 16` (the `#[doc(hidden)]` test hook)
    // would otherwise fail this whole function with "no such table:
    // app_settings" even though the migration itself succeeded and no new
    // FK violations exist (WARNING-1 / WR-03 in 40.3-REVIEW.md).
    if let Err(e) = persist_fk_baseline(conn, &after) {
        tracing::warn!(
            "FK baseline persist failed (non-fatal — e.g. app_settings not yet \
             created by run_up_to(<16), or DB opened read-only): {e:?}"
        );
    }

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

    /// Regression test for N-4 (`.planning/v1.4-MILESTONE-AUDIT.md`): before
    /// this fix, `run_to`'s `else` branch (taken whenever `pending` is
    /// `false`, i.e. every start after the migration that introduced the
    /// violation) never called `fk_violations()` at all — so a new FK
    /// violation blocked exactly the run that created it and NONE of the
    /// runs after that. This test seeds a violation post-migration, then
    /// asserts `run()` fails on the SECOND start (no pending migrations)
    /// AND again identically on a THIRD start (the persisted baseline must
    /// not silently swallow the violation after the first failure).
    #[test]
    fn run_blocks_every_subsequent_start_after_new_fk_violation_not_only_first() {
        let (mut conn, _guard) = fresh_conn();
        run(&mut conn).expect("initial clean run applies all migrations");

        // Corrupt referential integrity AFTER all migrations have run: an
        // orphan `devices.place_id` pointing at a place that does not
        // exist. FKs must be toggled OFF to even insert this row.
        conn.execute("PRAGMA foreign_keys = OFF", [])
            .expect("disable foreign_keys for the orphan insert");
        conn.execute(
            "INSERT INTO devices \
             (type_id, name, status_id, place_id, created_at_utc, updated_at_utc) \
             VALUES (1, 'Тестовое устройство', 1, 999999, 0, 0)",
            [],
        )
        .expect("insert orphan device row (place_id has no matching places row)");
        conn.execute("PRAGMA foreign_keys = ON", [])
            .expect("re-enable foreign_keys");

        // Second start: `pending` is false (no new migrations queued) —
        // this is exactly the N-4 regression path.
        let second = run(&mut conn);
        assert!(
            second.is_err(),
            "run() must fail on the second start: a new FK violation exists \
             and pending=false must no longer skip the integrity check (N-4)"
        );

        // Third start, still uncorrected: must fail AGAIN, identically —
        // proves the persisted baseline did NOT advance on a blocking Err,
        // so every later start keeps failing, not just the one right after
        // the violation was introduced.
        let third = run(&mut conn);
        assert!(
            third.is_err(),
            "run() must keep failing on every later start until the data is \
             actually fixed, not just once (N-4: 'not only the first run')"
        );
    }

    /// Self-heal regression test: once a previously-blocking FK violation is
    /// fixed (data corrected), the NEXT start must succeed again — the
    /// persisted baseline pulls back down to "clean" rather than staying
    /// stuck on a stale dirty snapshot forever.
    #[test]
    fn run_self_heals_baseline_after_violation_is_fixed() {
        let (mut conn, _guard) = fresh_conn();
        run(&mut conn).expect("initial clean run applies all migrations (persists empty baseline)");

        conn.execute("PRAGMA foreign_keys = OFF", [])
            .expect("disable foreign_keys for the orphan insert");
        conn.execute(
            "INSERT INTO devices \
             (type_id, name, status_id, place_id, created_at_utc, updated_at_utc) \
             VALUES (1, 'Тестовое устройство', 1, 999999, 0, 0)",
            [],
        )
        .expect("insert orphan device row (place_id has no matching places row)");
        conn.execute("PRAGMA foreign_keys = ON", [])
            .expect("re-enable foreign_keys");

        assert!(
            run(&mut conn).is_err(),
            "run() must fail while the orphan row exists"
        );

        // Fix the data by deleting the orphan row.
        conn.execute("DELETE FROM devices WHERE place_id = 999999", [])
            .expect("delete orphan device row");

        // Next start must succeed again: self-heal pulls the persisted
        // baseline back down once integrity is restored.
        let healed = run(&mut conn);
        assert!(
            healed.is_ok(),
            "run() must succeed again once the FK violation is fixed (self-heal): {:?}",
            healed.err()
        );
    }

    /// Regression test for WARNING-1 (WR-03 in `40.3-REVIEW.md`): before this
    /// fix, `persist_fk_baseline(conn, &after)?` propagated ANY write error
    /// (including "no such table: app_settings") through `run_to` via `?`,
    /// failing the whole function. `app_settings` is created by V016
    /// (`migrations/V016__cartridges_kind_color_settings.sql`), so stopping
    /// short of it via the `#[doc(hidden)]` `run_up_to` test hook must still
    /// succeed — the migrations themselves applied fine and there are no new
    /// FK violations, only the best-effort baseline write has nothing to
    /// write into yet.
    #[test]
    fn run_up_to_below_app_settings_creation_does_not_fail_on_baseline_persist() {
        let (mut conn, _guard) = fresh_conn();
        let result = run_up_to(&mut conn, 15);
        assert!(
            result.is_ok(),
            "run_up_to(conn, 15) must succeed even though app_settings (created by \
             V016) does not exist yet and the baseline persist has nowhere to write: {:?}",
            result.err()
        );
    }

    /// Unit-test half 1 of WARNING-2 (WR-01 in `40.3-REVIEW.md`): a
    /// previously-baselined violation whose `rowid` changed (a table rebuild
    /// reassigned it) must NOT be reported as new by
    /// `violations_with_grown_signature`, even though the raw tuple no
    /// longer matches the baseline.
    #[test]
    fn fk_signature_ignores_rowid_for_grandfathered_violation() {
        let baseline_set: std::collections::BTreeSet<FkViolation> =
            [("devices".to_string(), Some(17), "places".to_string(), 0)]
                .into_iter()
                .collect();
        let after_set: std::collections::BTreeSet<FkViolation> =
            [("devices".to_string(), Some(5), "places".to_string(), 0)]
                .into_iter()
                .collect();

        // Sanity check: the test data really does differ by raw rowid —
        // otherwise this test would prove nothing.
        assert!(
            !after_set
                .difference(&baseline_set)
                .collect::<Vec<_>>()
                .is_empty(),
            "test fixture must differ by rowid for this test to be meaningful"
        );

        let new_violations = violations_with_grown_signature(&after_set, &baseline_set);
        assert!(
            new_violations.is_empty(),
            "same (table, parent, fkid) signature with a different rowid must not be \
             reported as a new violation (WR-01 false-positive half): {new_violations:?}"
        );
    }

    /// Unit-test half 2 of WARNING-2 (WR-01): a genuinely new violation that
    /// happens to reuse a previously-baselined `rowid` (different
    /// (table, parent, fkid) signature, same rowid) must still be reported —
    /// signature comparison must not mask it.
    #[test]
    fn fk_signature_still_catches_new_violation_on_reused_rowid() {
        let baseline_set: std::collections::BTreeSet<FkViolation> =
            [("devices".to_string(), Some(17), "places".to_string(), 0)]
                .into_iter()
                .collect();
        let after_set: std::collections::BTreeSet<FkViolation> =
            [("devices".to_string(), Some(17), "places".to_string(), 1)]
                .into_iter()
                .collect();

        let new_violations = violations_with_grown_signature(&after_set, &baseline_set);
        assert_eq!(
            new_violations.len(),
            1,
            "a new (table, parent, fkid) signature on a reused rowid must still be \
             reported as a new violation (WR-01 masking half): {new_violations:?}"
        );
    }

    /// Integration test for WARNING-2 (WR-01): a real table rebuild
    /// (schema-preserving `CREATE TABLE` copy -> `INSERT ... SELECT` without
    /// the `id` column -> `DROP` -> `RENAME`, the project's established
    /// migration shape — V042/V043/V044, Phases 41/43 plan more) reassigns
    /// SQLite's physical rowid for existing rows. An already-baselined
    /// (accepted) violation must not falsely re-block startup after such a
    /// rebuild.
    ///
    /// The rebuilt table's `CREATE TABLE` DDL is copied verbatim from
    /// `sqlite_master` (not a hand-written `CREATE ... AS SELECT *`, which
    /// drops the `INTEGER PRIMARY KEY` constraint entirely and would break
    /// `PRAGMA foreign_key_check` for every OTHER table that references
    /// `devices(id)` — that is a schema-corruption bug in the test harness,
    /// not the rowid-reassignment scenario WR-01 targets). Omitting the
    /// `id` column from the `INSERT ... SELECT` column list lets SQLite's
    /// own `AUTOINCREMENT` counter (fresh for the new table name) assign a
    /// new physical rowid — modelling the worst case where a rebuild does
    /// NOT preserve id values, without corrupting the schema.
    #[test]
    fn table_rebuild_reassigning_rowid_does_not_falsely_block_grandfathered_violation() {
        let (mut conn, _guard) = fresh_conn();
        run(&mut conn).expect("initial clean run applies all migrations");

        // Orphan devices row with an EXPLICIT id — `devices.id` is an
        // `INTEGER PRIMARY KEY AUTOINCREMENT` rowid-alias (V003), so this
        // forces the physical rowid to 500.
        conn.execute("PRAGMA foreign_keys = OFF", [])
            .expect("disable foreign_keys for the orphan insert");
        conn.execute(
            "INSERT INTO devices \
             (id, type_id, name, status_id, place_id, created_at_utc, updated_at_utc) \
             VALUES (500, 1, 'Тестовое устройство', 1, 999999, 0, 0)",
            [],
        )
        .expect("insert orphan device row with explicit rowid=500");
        conn.execute("PRAGMA foreign_keys = ON", [])
            .expect("re-enable foreign_keys");

        // Seed the baseline directly (bypassing run()'s own first-appearance
        // acceptance path, which would otherwise fail before the rebuild
        // even runs) so this test isolates the rebuild's effect on an
        // ALREADY-accepted violation.
        let before_rebuild = fk_violations(&conn).expect("snapshot before rebuild");
        persist_fk_baseline(&conn, &before_rebuild).expect("seed baseline directly");

        // Real table rebuild that PRESERVES the schema (PK/FK-target shape)
        // but does NOT preserve rowid values.
        let create_sql: String = conn
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'devices'",
                [],
                |r| r.get(0),
            )
            .expect("read devices table DDL from sqlite_master");
        let rebuilt_sql = create_sql.replacen("devices", "devices_rebuilt", 1);
        let non_id_columns: Vec<String> = {
            let mut stmt = conn
                .prepare("PRAGMA table_info(devices)")
                .expect("prepare table_info(devices)");
            let rows = stmt
                .query_map([], |r| r.get::<_, String>(1))
                .expect("query table_info(devices)");
            rows.map(|r| r.expect("column name"))
                .filter(|c| c != "id")
                .collect()
        };
        let cols_csv = non_id_columns.join(", ");

        conn.execute("PRAGMA foreign_keys = OFF", [])
            .expect("disable foreign_keys for the rebuild");
        conn.execute(&rebuilt_sql, [])
            .expect("create devices_rebuilt with identical schema (incl. PRIMARY KEY)");
        conn.execute(
            &format!("INSERT INTO devices_rebuilt ({cols_csv}) SELECT {cols_csv} FROM devices"),
            [],
        )
        .expect("copy rows into devices_rebuilt WITHOUT preserving id/rowid");
        conn.execute("DROP TABLE devices", [])
            .expect("drop old devices table");
        conn.execute("ALTER TABLE devices_rebuilt RENAME TO devices", [])
            .expect("rename rebuilt table back to devices");
        conn.execute("PRAGMA foreign_keys = ON", [])
            .expect("re-enable foreign_keys");

        let after_rebuild = fk_violations(&conn).expect("snapshot after rebuild");
        assert_ne!(
            after_rebuild, before_rebuild,
            "sanity check: the rebuild must actually have changed the raw rowid, \
             otherwise this test proves nothing"
        );

        let result = run(&mut conn);
        assert!(
            result.is_ok(),
            "run() must not falsely block startup on an already-baselined violation \
             whose rowid changed due to a table rebuild (WR-01): {:?}",
            result.err()
        );
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

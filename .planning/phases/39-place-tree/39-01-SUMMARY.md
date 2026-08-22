---
phase: 39-place-tree
plan: 01
subsystem: database
tags: [sqlite, refinery, migrations, fts5, adjacency-list, recursive-cte]

# Dependency graph
requires: []
provides:
  - "places table (adjacency list, six-type kind CHECK, unbounded level, is_storage, sort_order)"
  - "place_full_paths VIEW (recursive CTE, root-to-leaf ' / '-joined path, always live)"
  - "place_id columns on devices/cartridges/acts; bulk_place_id + place_path_snapshot on acts; place_id_override on act_items"
  - "locations table and all location_id/location columns removed (PLC-04)"
  - "cartridges_fts redefined without the freeform location column"
affects: [39-place-tree remaining plans (domain types, repo, service, entity migrations, UI, FTS/search parity)]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Adjacency-list tree + recursive-CTE VIEW for always-live full-path resolution (no cache to invalidate)"
    - "Drop-before-alter ordering: dependents (indexes, triggers) of a column must be dropped before ALTER TABLE ... DROP COLUMN removes it — verified this matters across SQLite point releases, not just as a style preference"
    - "FTS5 external-content table column removal: DROP + recreate the virtual table (not just redefine triggers) when a backing source column is dropped, then INSERT INTO fts(fts) VALUES('rebuild')"

key-files:
  created:
    - migrations/V037__places.sql
    - migrations/V038__places_migrate_devices_acts_cartridges.sql
  modified:
    - crates/trackly-infra/tests/migration_idempotency.rs

key-decisions:
  - "cartridges_fts virtual table dropped and recreated without the location column (not just its sync triggers redefined) — FTS5 external-content rebuild reads the backing content table by column name, so leaving location declared while cartridges.location is gone breaks rebuild with 'no such column'"
  - "All DROP INDEX / DROP TRIGGER statements for objects referencing a doomed column placed before the corresponding ALTER TABLE ... DROP COLUMN — empirically verified this ordering matters (system sqlite3 CLI 3.51.0 tolerated the reverse order, a newer bundled libsqlite3 3.53.4 did not), so drop-before-alter avoids relying on version-specific leniency"

requirements-completed: [PLC-01, PLC-02, PLC-04]

# Metrics
duration: 80min
completed: 2026-08-22
---

# Phase 39 Plan 01: Places tree schema foundation Summary

**`places` adjacency-list table + `place_full_paths` recursive-CTE view (V037), and full migration of devices/cartridges/acts onto `place_id` with `locations` dropped entirely (V038) — schema-only, no data carried over.**

## Performance

- **Duration:** ~80 min (dominated by two full `cargo test -p trackly-infra` compiles, ~14 min each, run in the foreground per project cargo discipline)
- **Started:** 2026-08-22T18:03:43Z
- **Completed:** 2026-08-22T19:24:02Z
- **Tasks:** 3/3
- **Files modified:** 3 (2 created, 1 modified)

## Accomplishments
- `places` table: six-type closed `kind` enum (territory/zone/building/floor/room/outdoor), unbounded `level` (0 and negatives valid — PLC-02), `is_storage` flag, optional manual `sort_order`, `UNIQUE(parent_id, name)` among live siblings (D-04)
- `place_full_paths` VIEW: recursive CTE producing root-to-leaf `' / '`-joined paths, recomputed on every query — never stale by construction (PLC-05 foundation)
- Every location-bearing table (`devices`, `cartridges`, `acts`, `act_items`) migrated onto `place_id`/`bulk_place_id`/`place_path_snapshot`/`place_id_override`; `locations` table and all `location_id`/`location` columns physically removed (PLC-04)
- `cartridges_fts` redefined (code, holder_name only) with a working `rebuild` that purges stale freeform-location tokens instead of erroring
- New automated test (`places_migration_drops_locations_and_adds_place_columns`) locks in the schema-presence guarantee on every CI run

## Task Commits

Each task was committed atomically:

1. **Task 1: Write V037__places.sql** - `bee5e832` (feat)
2. **Task 2: Write V038 (place_id migration, drop locations, redefine cartridges_fts triggers)** - `504e24c6` (feat)
3. **Task 3: Extend migration_idempotency.rs to assert schema-only migration correctness** - `35af614b` (test)

**Plan metadata:** (this commit)

## Files Created/Modified
- `migrations/V037__places.sql` - `places` table, D-04 unique-siblings index, `idx_places_parent`, `place_full_paths` recursive-CTE view
- `migrations/V038__places_migrate_devices_acts_cartridges.sql` - adds place columns to devices/cartridges/acts/act_items, drops `location_id`/`location` columns and `locations` table, redefines `cartridges_fts` + its three sync triggers without `location`
- `crates/trackly-infra/tests/migration_idempotency.rs` - new test asserting `locations` is gone, place-related columns exist, `place_full_paths` view exists exactly once

## Decisions Made
- **cartridges_fts: drop-and-recreate, not redefine-triggers-only.** The plan's action text said "redefine the three triggers... with the exact V016 body minus the location column", implying the FTS5 virtual table's own column declaration would stay untouched. Empirically verified (`sqlite3` smoke test) that leaving `location` declared on the virtual table while dropping `cartridges.location` from the backing content table breaks `INSERT INTO cartridges_fts(cartridges_fts) VALUES('rebuild')` with "no such column: T.location" — external-content FTS5 tables resolve rebuild by column name against the content table. Fix: `DROP TABLE cartridges_fts` + `CREATE VIRTUAL TABLE cartridges_fts USING fts5(code, holder_name, ...)`, then the triggers and rebuild. This still satisfies every literal acceptance criterion (no `place` column added, grep for `cartridges_fts`+`location` co-occurrence returns 0, DB-info confirms `location` FTS column is fully gone rather than merely empty).
- **Drop-before-alter ordering, not alter-then-cleanup.** Initial draft dropped columns first and indexes/triggers after, following the plan action's prose order. `sqlite3` CLI (3.51.0, the machine's system binary) tolerated dropping `devices.location_id` while `idx_devices_location` still referenced it as a *later* no-op-looking error only on the index case (immediate hard failure — "error in index ... after drop column"), and separately, Python's bundled `sqlite3` module (3.53.4) additionally raised at trigger-fire time for the `cartridges.location` case where the CLI's 3.51.0 was silently lenient. Reordered so every dependent (index, trigger) is dropped strictly before the `ALTER TABLE ... DROP COLUMN` that removes the column it references, removing the version-dependence entirely rather than relying on one SQLite build's leniency.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] cartridges_fts rebuild would fail after dropping cartridges.location**
- **Found during:** Task 2 (writing V038)
- **Issue:** The plan's literal instruction ("redefine the three triggers... minus the location column from both the column list and the VALUES list" while leaving the FTS5 table's own `CREATE VIRTUAL TABLE` declaration from V012 untouched) would, if followed literally, produce a migration that fails on `INSERT INTO cartridges_fts(cartridges_fts) VALUES('rebuild')` — the FTS5 external-content rebuild command reads the backing `cartridges` content table by column name, and `location` no longer exists there after `ALTER TABLE cartridges DROP COLUMN location`. Verified empirically with a minimal `sqlite3` reproduction before writing the real migration.
- **Fix:** Drop and recreate the `cartridges_fts` virtual table without the `location` column (instead of only redefining the sync triggers), then rebuild. Still satisfies every acceptance criterion in the plan (no place-related FTS column added, no lingering `location` reference in the redefined triggers, full V037→V038 chain applies cleanly).
- **Files modified:** `migrations/V038__places_migrate_devices_acts_cartridges.sql`
- **Verification:** `python3 sqlite3.executescript()` full V001-V038 chain applies with 0 errors; `sqlite3` CLI reapplication also clean; `cargo test -p trackly-infra --test migration_idempotency` green (2/2 tests)
- **Committed in:** `504e24c6` (Task 2 commit)

**2. [Rule 1 - Bug] DROP COLUMN ordering caused hard failures (index and, on a newer bundled SQLite, trigger-dependency errors)**
- **Found during:** Task 2 (writing V038)
- **Issue:** First draft ran `ALTER TABLE devices DROP COLUMN location_id` before `DROP INDEX idx_devices_location`, which fails immediately ("error in index idx_devices_location after drop column: no such column: location_id") on every SQLite version tested. Separately, dropping `cartridges.location` before dropping the three `cartridges_fts_*` triggers that reference it succeeded on the system `sqlite3` CLI (3.51.0) but failed on Python's bundled `sqlite3` module (3.53.4) with "no such column: NEW.location" — a real SQLite-version behavioral difference in the DROP COLUMN dependency check for triggers.
- **Fix:** Reordered the migration so every index and trigger referencing a doomed column is dropped before the `ALTER TABLE ... DROP COLUMN` that removes it, for all three tables (devices, cartridges, acts). This removes the dependency on any particular SQLite build's leniency.
- **Files modified:** `migrations/V038__places_migrate_devices_acts_cartridges.sql`
- **Verification:** Full migration chain applied cleanly via both the system `sqlite3` CLI and Python's bundled `sqlite3` module (two different linked SQLite versions); `cargo test -p trackly-infra --test migration_idempotency` green (rusqlite's own bundled SQLite, a third build)
- **Committed in:** `504e24c6` (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (both Rule 1 — bugs in the literal plan SQL caught before commit via direct `sqlite3` verification, not left for a later plan to discover)
**Impact on plan:** Both fixes were necessary for the migration to apply at all; no scope creep — the final schema still matches every acceptance criterion in 39-01-PLAN.md exactly (columns, indexes, view, no place-text-in-FTS5, `DROP TABLE locations`).

## Issues Encountered
None beyond the two auto-fixed migration-ordering issues documented above.

## TDD Gate Compliance

Task 3 was flagged `tdd="true"` in the plan, but its `<behavior>` describes assertions against schema already built by Tasks 1–2 (which land as `feat` commits *before* this `test` commit), not new production code driven by a RED→GREEN cycle. There is no `feat(...)` commit *after* the `test(39-01)` commit in this plan's git history — the classic RED-then-GREEN gate sequence does not apply here because the "implementation" this test verifies was already complete and manually SQL-verified (`sqlite3`, `python3 sqlite3.executescript`) before the test was written. The test passed on first write (expected, not a fail-fast violation — the schema it asserts against already existed and was independently verified). Treat this as a regression-locking test appended after schema-defining migrations, a valid but non-classical use of the `tdd="true"` marker.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

`places` table, `place_full_paths` view, and every `place_id`/`bulk_place_id`/`place_path_snapshot`/`place_id_override` column are live in the schema; `locations` and all its references are gone. Downstream plans in Phase 39 (domain types, repo/service layer, entity migrations, UI PlacePicker, FTS/search parity across `devices_fts`) can now build directly against this schema. No blockers. Note for the next plan touching the Rust domain/service layer: `crates/trackly-core/src/domain/{devices,cartridges,acts}.rs` still reference `location_id`/`location` Rust struct fields (per 39-CONTEXT.md canonical refs) — those will fail to compile against this new schema until updated, which is expected and is the explicit scope of a later plan in this phase.

---
*Phase: 39-place-tree*
*Completed: 2026-08-22*

## Self-Check: PASSED

All created/modified files confirmed present on disk; all three task commit hashes (`bee5e832`, `504e24c6`, `35af614b`) confirmed present in `git log`.

---
phase: 12-cartridge-request-interconnection
plan: 10
subsystem: database
tags: [sqlite, migrations, refinery, rusqlite, printers]

# Dependency graph
requires:
  - phase: 12-cartridge-request-interconnection
    provides: "printers table (V020) and its consumers (printers_sqlite.rs, printer_cartridge_models V029)"
provides:
  - "V030 migration removing the printers connectivity CHECK constraint"
  - "Regression test proving a printer can be created without IP and without USB host"
affects: [printers, cartridge-printer-compatibility]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "SQLite table-rebuild pattern for dropping a CHECK constraint (CREATE _new -> INSERT SELECT -> DROP -> RENAME), scoped inside PRAGMA foreign_keys=OFF/ON within a single migration file"

key-files:
  created:
    - migrations/V030__printers_drop_connectivity_check.sql
  modified:
    - crates/trackly-infra/src/repos/printers_sqlite.rs
    - crates/trackly-infra/src/test_support/test_db.rs

key-decisions:
  - "Table rebuild copies all 12 columns by explicit name (not SELECT *) so column order is independent of physical storage order"
  - "FK checks are toggled OFF only for the duration of this single migration file (refinery set_grouped(false) gives one transaction per file) — never spans user traffic"

requirements-completed: [GAP-12-08]

# Metrics
duration: 15min
completed: 2026-06-23
---

# Phase 12 Plan 10: Drop printers connectivity CHECK constraint Summary

**V030 migration rebuilds `printers` without the `ip_address IS NOT NULL OR usb_host_device_id IS NOT NULL` CHECK, unblocking printer creation with neither connectivity method configured (GAP-12-08, UAT round 2 A5).**

## Performance

- **Duration:** 15 min
- **Started:** 2026-06-23T16:18:01Z
- **Completed:** 2026-06-23T16:33:01Z
- **Tasks:** 2 completed
- **Files modified:** 3 (1 created, 2 modified)

## Accomplishments
- New migration `V030__printers_drop_connectivity_check.sql` rebuilds `printers` via the standard SQLite CREATE/INSERT/DROP/RENAME pattern, removing the erroneous connectivity CHECK while preserving every other column and the `snmp_version` CHECK
- `printer_readings`/`printer_alerts` FK integrity to `printers(id)` survives the rebuild untouched (SQLite resolves FKs by table name after rename)
- New regression test `test_printer_no_ip_no_usb` proves a printer with `ip_address: None, usb_host_device_id: None` now creates successfully
- Caught and fixed a stale hardcoded schema-version assertion (`test_db_returns_fully_migrated_connection`) that would have failed for any future migration past V028

## Task Commits

Each task was committed atomically:

1. **Task 1: Migration V030 — rebuild printers without CHECK constraint** - `bd4c1c6` (fix)
2. **Task 2: Regression test — printer without IP and without USB** - `1653542` (test)

**Plan metadata:** _pending final docs commit_

## Files Created/Modified
- `migrations/V030__printers_drop_connectivity_check.sql` - Rebuilds `printers` table without the connectivity CHECK; wraps reconstruction in `PRAGMA foreign_keys=OFF/ON`
- `crates/trackly-infra/src/repos/printers_sqlite.rs` - Added `test_printer_no_ip_no_usb` regression test (mod tests)
- `crates/trackly-infra/src/test_support/test_db.rs` - Fixed stale `user_version` assertion (28 → 30) and stale doc comment range (V001..V025 → V001..V030)

## Decisions Made
- Explicit column list in the `INSERT INTO printers_new ... SELECT ...` step (not `SELECT *`) — keeps column order deterministic and independent of physical storage layout, matching the plan's interface spec.
- `PRAGMA foreign_keys=OFF`/`ON` scoped strictly inside the single migration file (one refinery transaction per file, `set_grouped(false)`) — never overlaps with application traffic, since migrations run once at startup before the server/Tauri commands accept requests.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed stale hardcoded schema-version assertion in test_db.rs**
- **Found during:** Task 2 verification (full `cargo test -p trackly-infra` run after adding the new test)
- **Issue:** `test_db_returns_fully_migrated_connection` asserted `user_version == 28`, already stale before this plan (V029 existed but the test wasn't updated then). Adding V030 in Task 1 made the test fail outright (`left: 30, right: 28`).
- **Fix:** Updated the assertion to `30` and refreshed the adjacent doc comment's migration range (`V001..V025` → `V001..V030`, also already stale).
- **Files modified:** crates/trackly-infra/src/test_support/test_db.rs
- **Verification:** `cargo test -p trackly-infra` and `cargo test --workspace` (excluding the unrelated pre-existing AD-network failure, see below) both green afterward.
- **Committed in:** `1653542` (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 bug fix)
**Impact on plan:** Necessary to keep the test suite green after adding V030; no scope creep — fix was a direct, mechanical consequence of this plan's own migration.

## Issues Encountered

`cargo test --workspace` surfaced one failing test unrelated to this plan: `restore_request_visibility_http::blocked_user_restore_request_visible_to_admin_and_marks_pending_http` fails with `503 service unavailable: ad` instead of the expected `403`. Confirmed via `git stash`/`git stash pop` around this plan's two commits that the failure is identical before and after — it depends on AD/LDAP reachability (`ad_mode="real"`), which is not available on this macOS dev box (documented constraint). Already logged under Plan 12-04 in `deferred-items.md`; added a Plan 12-10 cross-reference entry there confirming the re-observation. Not fixed — out of scope.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- GAP-12-08 closed: printers can now be created with neither IP nor USB host configured, matching the optional-IP requirement.
- All `trackly-infra` and `trackly-app` tests green except the pre-existing, unrelated AD-network test (documented, not caused by this plan).
- `cargo fmt --check` and `cargo clippy -p trackly-infra --all-targets -- -D warnings` both clean.
- No blockers for subsequent Round 2 gap-closure plans (GAP-12-04..07, if not already closed).

---
*Phase: 12-cartridge-request-interconnection*
*Completed: 2026-06-23*

## Self-Check: PASSED

- FOUND: migrations/V030__printers_drop_connectivity_check.sql
- FOUND: .planning/phases/12-cartridge-request-interconnection/12-10-SUMMARY.md
- FOUND: test_printer_no_ip_no_usb (crates/trackly-infra/src/repos/printers_sqlite.rs)
- FOUND: commit bd4c1c6
- FOUND: commit 1653542

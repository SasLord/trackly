---
phase: 260808-np4-unify-ad-register-visibility-predicate
plan: 01
subsystem: auth
tags: [rust, sqlite, rusqlite, refactor, regression-testing, mutation-testing]

# Dependency graph
requires:
  - phase: 09-ad
    provides: RequestService::list/counts, SqliteRequestRepository::list/counts, ad_register request type (T-09-11/REQ-06)
  - phase: 260805-nae-employee-dashboard-widget-must-exclude-a
    provides: DashboardService::get_employee_widgets ad_register exclusion (the deferred technical debt this task closes)
provides:
  - "trackly_core::auth::excludes_ad_register(&Role) -> bool — single source of truth for the REQ-06 role rule"
  - "trackly_infra::repos::requests_sqlite::ad_register_predicate/ad_register_exclude_clause — single source of the SQL literal"
  - "Manager-role regression test with a proven (mutation-checked) non-vacuous assertion"
affects: [request_service, requests_sqlite, dashboard_service, future ad_register-related work]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Domain-layer boolean predicate function in trackly-core::auth, called identically from every service-layer call site that needs the same authorization-adjacent rule"
    - "SQL-fragment extraction via module-level pub fn returning a String, interpolated with format! into the literal query strings — placeholder numbering owned entirely by the caller, never by the helper"

key-files:
  created:
    - crates/trackly-app/tests/requests_ad_register_visibility_manager.rs
  modified:
    - crates/trackly-core/src/auth.rs
    - crates/trackly-app/src/services/request_service.rs
    - crates/trackly-infra/src/repos/requests_sqlite.rs
    - crates/trackly-app/src/services/dashboard_service.rs

key-decisions:
  - "Collapsed 3 independently-duplicated implementations (2 role checks + 8 SQL literals + 1 dashboard literal = 11 places) of the REQ-06 ad_register visibility rule into exactly 2 shared functions, per the project's own recorded lesson: fixing a duplicated defect twice means the missing gate is the real defect."
  - "New regression test targets the ONE role (Manager) never previously exercised through the service layer for this predicate — existing coverage (requests_ad_register.rs) only drove list()/counts() through Employee."
  - "Mutation-check cycle performed and evidence captured (see below) rather than asserted from memory, per project lesson on non-vacuous tests."

requirements-completed: [REQ-06]

# Metrics
duration: 33min
completed: 2026-08-08
---

# Quick Task 260808-np4: Unify ad_register visibility predicate Summary

**Collapsed 3 independently-duplicated implementations of the REQ-06 "only Admin sees ad_register requests" rule (2 duplicated role checks, 8 literal SQL predicate occurrences, 1 dashboard literal) into `trackly_core::auth::excludes_ad_register` + `trackly_infra::repos::requests_sqlite::{ad_register_predicate, ad_register_exclude_clause}`, with a mutation-checked Manager-role regression test closing the last coverage gap.**

## Performance

- **Duration:** 33 min
- **Started:** 2026-08-08T10:23:13Z (approx, first task commit)
- **Completed:** 2026-08-08T10:56:45Z
- **Tasks:** 3
- **Files modified:** 4 (+1 created)

## Accomplishments

- Added `excludes_ad_register(role: &Role) -> bool` to `trackly-core::auth` as the single source of truth for the REQ-06 rule, with 3 unit tests covering all 3 roles; both duplicated call sites in `RequestService::list`/`counts` now call it.
- Added `ad_register_predicate(alias)` / `ad_register_exclude_clause(alias, placeholder)` to `requests_sqlite.rs`, replacing all 8 previously-literal `'ad_register'` occurrences (2 in `list()`, 6 in `counts()`) and the 1 hardcoded literal in `DashboardService::get_employee_widgets`.
- Added a Manager-role regression test (`requests_ad_register_visibility_manager.rs`) exercising `RequestService::list`/`counts` through a real Manager `Identity` — the one role never previously covered for this predicate — with control assertions proving the Manager's own non-ad_register request stays visible/counted, and an Admin comparison proving the exclusion is role-specific.
- Performed and documented the mandatory mutation-check cycle (see below), with captured RED-run evidence.

## Task Commits

Each task was committed atomically:

1. **Task 1: Extract the ad_register role predicate into trackly-core::auth** - `fea0ef3` (refactor)
2. **Task 2: Extract the ad_register SQL fragment and wire the dashboard's literal to it** - `210cee3` (refactor)
3. **Task 3: Manager-role regression test + mandatory mutation-check evidence** - `5d77a8f` (test), `1c7f73b` (style — cargo fmt on the new test file)

_Note: Task 3's mutation-check cycle (steps b–e below) intentionally left zero net diff on `auth.rs`, so there is no separate "mutation" commit — the RED/GREEN cycle was performed entirely against the working tree between the Task 2 and Task 3 commits, then reverted before staging._

## Files Created/Modified

- `crates/trackly-core/src/auth.rs` - Added `excludes_ad_register(role: &Role) -> bool` + 3 unit tests
- `crates/trackly-app/src/services/request_service.rs` - Both `list()`/`counts()` now call `trackly_core::auth::excludes_ad_register(&caller.role)` instead of the duplicated `!matches!` expression
- `crates/trackly-infra/src/repos/requests_sqlite.rs` - Added `ad_register_predicate`/`ad_register_exclude_clause`; all 8 literal SQL occurrences now interpolate the shared clause
- `crates/trackly-app/src/services/dashboard_service.rs` - `get_employee_widgets`'s `clauses` vec now sources its unconditional exclusion from `trackly_infra::repos::requests_sqlite::ad_register_predicate("r.")`
- `crates/trackly-app/tests/requests_ad_register_visibility_manager.rs` - New regression test (created)

## Decisions Made

- No architectural changes — pure refactor as specified. All 3 call sites (`RequestService::list`/`counts`, `SqliteRequestRepository::list`/`counts`, `DashboardService::get_employee_widgets`) now derive from exactly 2 shared functions instead of 11 independently-duplicated implementations.
- Kept the per-file-duplicated test-helper convention (`admin()`/`manager()`/`employee()`/`make_service()`/`seed_user`/`seed_ad_register` re-declared locally in the new test file) — matches the established pattern in `request_lifecycle.rs` and `requests_ad_register.rs`; integration test binaries in this codebase cannot import another test file's private functions.

## Deviations from Plan

**None** — plan executed exactly as written. `cargo fmt` was run only on the newly-created test file (not the whole workspace) after Task 3, per the plan's explicit fmt guidance (`cargo fmt --check may show pre-existing drift elsewhere... only run cargo fmt on the 5 files this plan touches if formatting them is needed`) — this is not a deviation, it is following the plan's own instruction. The 4 previously-existing files (`auth.rs`, `request_service.rs`, `requests_sqlite.rs`, `dashboard_service.rs`) already matched `cargo fmt`'s expected formatting with no changes needed.

## Mandatory Mutation-Check Evidence (Task 3, critical requirement)

Performed exactly per plan steps a–f, driven from Task 3:

**a. Baseline GREEN** (test run alone, before mutation):
```
running 1 test
test manager_cannot_see_ad_register_in_list_or_counts_admin_can ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
```

**b. Mutation applied:** `crates/trackly-core/src/auth.rs`'s `excludes_ad_register` body temporarily changed from `!matches!(role, Role::Admin)` to `false` (unconditionally — simulating the exact regression class this test exists to catch).

**c. Re-ran the exact same test command — CAPTURED RED FAILURE (verbatim):**
```
thread 'manager_cannot_see_ad_register_in_list_or_counts_admin_can' (102463166) panicked at crates/trackly-app/tests/requests_ad_register_visibility_manager.rs:166:5:
manager list must not contain any ad_register requests
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test manager_cannot_see_ad_register_in_list_or_counts_admin_can ... FAILED

failures:
    manager_cannot_see_ad_register_in_list_or_counts_admin_can

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
```
This is a real assertion failure (the `manager list must not contain any ad_register requests` check at line 166), not a compile error and not a false green — confirming the test is genuinely wired to the predicate under test.

**d. Reverted:** `excludes_ad_register`'s body restored to exactly `!matches!(role, Role::Admin)`.

**e. Re-ran the exact same test command — GREEN again:**
```
running 1 test
test manager_cannot_see_ad_register_in_list_or_counts_admin_can ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
```

**f. Zero-diff confirmation:** `git diff crates/trackly-core/src/auth.rs` produced no output — the revert restored the file to exactly its Task-1-committed state.

## Issues Encountered

None — all four targeted safety-net test invocations plus the new test passed with zero regressions, run strictly sequentially (never `cargo test --workspace`, never two `cargo test` invocations concurrently, per the hard environment rules).

**Final safety net (all run sequentially, one at a time, all with `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1` where applicable):**
- `cargo test -p trackly-core --lib auth::tests` — 19 passed (16 pre-existing + 3 new `excludes_ad_register_*`)
- `cargo test -p trackly-infra --lib repos::requests_sqlite` — 7 passed, unaffected by the extraction
- `cargo test -p trackly-app --test requests_ad_register` — 8 passed (Employee-role coverage unchanged)
- `cargo test -p trackly-app --test dashboard_widgets` — 3 passed (including 260805-nae's `dashboard_employee_widget_excludes_ad_register` regression test, unchanged)
- `cargo test -p trackly-app --test requests_ad_register_visibility_manager` — 1 passed (new Manager-role test)
- `cargo check -p trackly-core -p trackly-infra -p trackly-app` — all compile clean

## Known Stubs

None.

## Threat Flags

None — no new network endpoints, auth paths, file access patterns, or schema changes at trust boundaries. This task consolidates an existing authorization-adjacent data-visibility rule; see the plan's threat model for the full analysis (already captured pre-execution).

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- The REQ-06 ad_register visibility rule now exists in exactly 2 places total (down from 3 independently-duplicated implementations across 11 call sites), removing the structural cause of the two prior regressions (quick tasks 260804-l22 and 260805-nae) where fixing the rule in one place silently left it broken elsewhere.
- Any future change to who can see `ad_register` requests need only touch `excludes_ad_register` (role rule) and/or `ad_register_predicate`/`ad_register_exclude_clause` (SQL fragment) — it is now structurally impossible for a future edit to fix the rule in one place and leave the others behind.
- No blockers for future work.

---
*Phase: 260808-np4-unify-ad-register-visibility-predicate*
*Completed: 2026-08-08*

## Self-Check: PASSED

All created/modified files exist on disk and all 4 task commits (`fea0ef3`, `210cee3`, `5d77a8f`, `1c7f73b`) are present in `git log --oneline --all`.

---
phase: 260805-nae
plan: 01
subsystem: api
tags: [rusqlite, dashboard, requests, ad_register, regression-test]

# Dependency graph
requires:
  - phase: 260804-l22
    provides: "RequestService::counts excludes ad_register for non-admin callers (the second of three duplicated count paths, fixed prior)"
provides:
  - "DashboardService::get_employee_widgets request-count query excludes request_type = 'ad_register'"
  - "Regression test proving the employee dashboard widget no longer counts an employee's invisible auto-created ad_register request"
affects: [dashboard, requests, employee-ui]

# Tech tracking
tech-stack:
  added: []
  patterns: ["Unconditional literal SQL clause (not a bound parameter) for a query path reachable only from a single caller role"]

key-files:
  created: []
  modified:
    - crates/trackly-app/src/services/dashboard_service.rs
    - crates/trackly-app/tests/dashboard_widgets.rs

key-decisions:
  - "Used an unconditional literal clause (`r.request_type != 'ad_register'`) rather than the parameterised (?N = 0 OR ...) form used by RequestRepository::list/counts, because get_employee_widgets is reached only for Role::Employee callers per the D-GATE-03 dispatch in get_all_widgets — there is no admin/manager branch through this function to preserve."
  - "Did not refactor the three independently-implemented ad_register-exclusion code paths (RequestService::list, RequestService::counts, DashboardService::get_employee_widgets) into one shared predicate — recorded as a follow-up observation per the plan, not acted on in this task."

requirements-completed: [NAE-01]

# Metrics
duration: 8min
completed: 2026-08-05
---

# Quick Task 260805-nae: Employee dashboard widget must exclude ad_register Summary

**`get_employee_widgets`'s request-count SQL now filters `r.request_type != 'ad_register'`, closing the third and last of three independently-implemented code paths that leaked the invisible auto-created AD-registration row into an employee-visible count.**

## Performance

- **Duration:** 8 min
- **Started:** 2026-08-05T16:50:00+07:00 (approx, first commit 16:51)
- **Completed:** 2026-08-05T16:58:49+07:00
- **Tasks:** 2/2
- **Files modified:** 2

## Accomplishments
- An employee whose only request is the invisible auto-created `ad_register` row now sees `request_counts_open == 0` from the dashboard widget (previously phantom `1`), matching the empty list they see on the requests page.
- An employee's real (non-`ad_register`) requests are still counted normally — the exclusion is scoped to `ad_register`, not a blanket suppression.
- Regression test added and verified to fail against the unfixed code (see Verification section below).

## Task Commits

Each task was committed atomically:

1. **Task 1: Exclude ad_register rows from the employee dashboard widget's request counts** - `100938c` (fix)
2. **Task 2: Regression test — employee dashboard widget excludes ad_register-only request** - `a4f635c` (test)

_Note: TDD-flagged task (Task 2) here is a regression test written and verified against the already-landed Task 1 fix, per plan instructions — RED/GREEN was established by temporarily reverting Task 1's fix in the working tree, confirming failure, then restoring it (see Verification)._

## Files Created/Modified
- `crates/trackly-app/src/services/dashboard_service.rs` - `get_employee_widgets`'s request-count `clauses` vec gains a literal `"r.request_type != 'ad_register'".to_string()` entry, pushed after the `requested_by_user_id` clause and before the optional period-bound clauses. No new bound parameter, `owned` vec, or `pidx` change.
- `crates/trackly-app/tests/dashboard_widgets.rs` - New `seed_employee_with_request` helper (parameterised `request_type`, mirrors `requests_ad_register.rs`'s `seed_pending_register` INSERT shape) and new test `dashboard_employee_widget_excludes_ad_register`: Test A seeds an employee whose only request is `ad_register` and asserts all three widget counts are zero; Test B (control) then seeds a `free_form` request for the same employee and asserts `request_counts_open == 1`.

## Decisions Made
- Literal unconditional SQL clause instead of the parameterised `(?N = 0 OR ...)` form used elsewhere — justified by D-GATE-03 (this function is reached only for `Role::Employee` callers, so there's no admin/manager path to keep the exclusion switchable for). Documented inline in the plan's `<action>` and reproduced here per the plan's instruction to "record this reasoning in the SUMMARY."
- The three-way duplication of the ad_register-exclusion rule (`RequestService::list`, `RequestService::counts`, `DashboardService::get_employee_widgets`) was NOT refactored into a shared predicate in this task — the plan explicitly records this as a follow-up observation, not something to act on now.

## Deviations from Plan

None - plan executed exactly as written. Both tasks matched their `<action>` blocks; no Rule 1-4 triggers encountered.

## Issues Encountered

None. `cargo check -p trackly-app` and `cargo check -p trackly-app --tests` both compiled clean on first attempt after the edits; `cargo fmt -p trackly-app -- --check` on the two touched files passed with no reformatting needed (an initial direct `rustfmt --check` invocation without cargo's edition context produced spurious edition-2015 parse errors — expected, not a real issue, resolved by using `cargo fmt -p` instead).

## Verification

1. `cargo check -p trackly-app` — compiles clean.
2. `git diff crates/trackly-app/src/services/dashboard_service.rs` (pre-commit) showed edits scoped to exactly one line inside `get_employee_widgets`'s clauses vec — the admin/manager body of `get_all_widgets` above the Employee early-return is byte-for-byte unchanged.
3. **RED/GREEN proof (regression-test-is-real requirement):** After committing both tasks, temporarily removed the `"r.request_type != 'ad_register'".to_string()` line from the working tree (direct edit, not `git stash` — this repo is on the main branch, not a worktree) and re-ran `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test dashboard_widgets dashboard_employee_widget_excludes_ad_register -- --nocapture`. Result: **test FAILED** — `assertion left == right failed: ad_register-only employee must see request_counts_open = 0 / left: 1 / right: 0` — proving the test genuinely exercises the bug. Restored the fix (confirmed via `git diff` showing zero delta from the committed state), then re-ran `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test dashboard_widgets -- --nocapture`: all 3 tests passed (2 pre-existing + the new regression test).
4. Did NOT run the full `cargo test -p trackly-app` — avoided per plan instructions due to the known pre-existing hang on `auth_remember_cookie`. Ran only one `cargo` invocation at a time throughout.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- NAE-01 closed. All three independently-implemented `ad_register`-exclusion code paths (`RequestService::list`, `RequestService::counts`, `DashboardService::get_employee_widgets`) are now consistent for Employee callers.
- Follow-up (not scheduled, recorded for future review): a single shared predicate/helper so this three-way duplication cannot silently drift out of sync again. Also recorded: `RequestService::counts` excludes `ad_register` for both Employee and Manager callers, while the admin/manager dashboard branch (DASH-04, untouched by this task) does not exclude `ad_register` for Manager callers at all — whether a Manager should see these rows in the dashboard is an open product question, not decided here.

---
*Phase: 260805-nae*
*Completed: 2026-08-05*

## Self-Check: PASSED

- FOUND: crates/trackly-app/src/services/dashboard_service.rs
- FOUND: crates/trackly-app/tests/dashboard_widgets.rs
- FOUND: commit 100938c
- FOUND: commit a4f635c

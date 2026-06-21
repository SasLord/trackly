---
phase: 10-employee-employee-ui-role-gating-read
plan: 02
subsystem: auth
tags: [rbac, bfla, authorize, axum, tauri, rusqlite, role-gating]

# Dependency graph
requires:
  - phase: 10-employee-employee-ui-role-gating-read
    provides: "Plan 10-01 moved Action::ReadData out of the always-true authorize() arm into Admin|Manager only, and left role_endpoint_matrix Case 9 intentionally RED"
provides:
  - "authorize(caller, &Action::ReadData) wired into every read-path build_* helper for devices, acts, cartridges, printers (list/get only), and reports — both HTTP and Tauri transports"
  - "resolve_tauri_identity wired into every corresponding Tauri command wrapper"
  - "HTTP handlers thread the real session identity through instead of discarding it via _identity"
  - "role_endpoint_matrix.rs CI test extended from 10 to 19 cases, proving Employee gets 403 and Manager retains 200/422 across all 5 gated domains"
affects: [10-03, 10-04, employee-ui, ui-role-gating]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "3-layer authorize() gating: authorize(caller, &Action::ReadData)? as first line of build_* helper body, caller threaded from both transports"
    - "Tauri wrappers call resolve_tauri_identity(state.inner()).await? before delegating to build_*"
    - "HTTP handlers rename let _identity = session_identity(...) to let identity = ... and pass &identity through"
    - "Action::ReadPrinters stays a distinct action from Action::ReadData — build_printers_refresh untouched"

key-files:
  created:
    - .planning/phases/10-employee-employee-ui-role-gating-read/deferred-items.md
  modified:
    - crates/trackly-app/src/tauri_cmds/devices.rs
    - crates/trackly-app/src/http/devices.rs
    - crates/trackly-app/src/tauri_cmds/acts.rs
    - crates/trackly-app/src/http/acts.rs
    - crates/trackly-app/src/tauri_cmds/cartridges.rs
    - crates/trackly-app/src/http/cartridges.rs
    - crates/trackly-app/src/tauri_cmds/printers.rs
    - crates/trackly-app/src/http/printers.rs
    - crates/trackly-app/src/tauri_cmds/reports.rs
    - crates/trackly-app/src/http/reports.rs
    - crates/trackly-app/tests/role_endpoint_matrix.rs

key-decisions:
  - "Gated build_reports_export_csv/export_pdf and build_reports_get_report_counts in addition to the 8 list_* helpers, since they are read-only data exports with no mutation behavior — kept consistent with the plan's 'apply the same gating treatment' instruction"
  - "Gated build_devices_state_hints despite it returning a static whitelist, per the plan's discretion note, for consistency with sibling read endpoints"
  - "Left build_printers_refresh on Action::ReadPrinters (unchanged) — a separate action from Action::ReadData, explicitly out of scope"
  - "Fixed a rustfmt line-wrap nit in acts.rs (introduced by Task 1's caller parameter) inline during Task 3, since CLAUDE.md mandates cargo fmt as a CI gate"
  - "Deferred 3 pre-existing, out-of-scope clippy/fmt issues (template_service.rs len_zero, backup_service.rs disallowed_methods, ws_upgrade_serve_connection.rs fmt drift) to deferred-items.md instead of fixing them, per scope-boundary rule"

requirements-completed: [D-GATE-01, D-GATE-02, D-TEST-01]

# Metrics
duration: 45min
completed: 2026-06-21
---

# Phase 10 Plan 02: Gate Cartridges/Printers/Reports + Devices/Acts read paths Summary

**Wired `authorize(caller, &Action::ReadData)` into every read-path build_* helper across devices, acts, cartridges, printers, and reports (both HTTP and Tauri transports), closing the BFLA gap left open after Plan 10-01's permission-matrix fix, and extended the CI role×endpoint matrix from 10 to 19 cases to prove it.**

## Performance

- **Duration:** ~45 min (this execution segment; continued from a prior compacted session)
- **Completed:** 2026-06-21T06:18:02Z
- **Tasks:** 3/3
- **Files modified:** 12 (10 source files + 1 test file + 1 new deferred-items.md)

## Accomplishments
- Closed the BFLA gap (API5:2023) across all 5 read-heavy resource domains: devices, acts, cartridges, printers, reports
- 39 build_* helpers now call `authorize(caller, &Action::ReadData)?` as their first statement
- Both transports (axum HTTP handlers and Tauri command wrappers) thread the real caller identity instead of discarding it
- `role_endpoint_matrix.rs` CI test grew from 10 to 19 cases — Case 9 (devices_list) flipped from intentionally-RED to GREEN; 9 new cases added proving acts/cartridges/printers/reports/users read-gating

## Task Commits

Each task was committed atomically:

1. **Task 1: Gate Devices + Acts read paths** - `5d83f02` (feat)
2. **Task 2: Gate Cartridges + Printers + Reports read paths** - `b4ed01a` (feat)
3. **Task 3: Extend CI role×endpoint matrix with Cases 11-19** - `40b958c` (test)

**Plan metadata:** _(pending — final docs commit below)_

## Files Created/Modified

- `crates/trackly-app/src/tauri_cmds/devices.rs` - 9 build_* helpers gated; 9 Tauri wrappers resolve identity
- `crates/trackly-app/src/http/devices.rs` - 9 read handlers thread real identity (CSV import/export untouched, out of scope)
- `crates/trackly-app/src/tauri_cmds/acts.rs` - 6 build_* helpers gated; 6 Tauri wrappers resolve identity
- `crates/trackly-app/src/http/acts.rs` - 6 read handlers thread real identity
- `crates/trackly-app/src/tauri_cmds/cartridges.rs` - 12 build_* helpers gated (list/get/search/status_counts/get_history/low_stock/models_list/models_get/4x suggest_*); 12 Tauri wrappers resolve identity
- `crates/trackly-app/src/http/cartridges.rs` - 12 read handlers thread real identity
- `crates/trackly-app/src/tauri_cmds/printers.rs` - build_printers_list/get gated (2); build_printers_refresh's Action::ReadPrinters untouched
- `crates/trackly-app/src/http/printers.rs` - handler_list/handler_get thread real identity
- `crates/trackly-app/src/tauri_cmds/reports.rs` - 11 build_* helpers gated (8x list_*, export_csv, export_pdf, get_report_counts); fresh authorize/Action/Identity import added
- `crates/trackly-app/src/http/reports.rs` - 10 read/export handlers thread real identity
- `crates/trackly-app/tests/role_endpoint_matrix.rs` - extended from 10 to 19 cases
- `.planning/phases/10-employee-employee-ui-role-gating-read/deferred-items.md` - logged 3 pre-existing out-of-scope lint/fmt issues

## Decisions Made

- Gated the reports CSV/PDF export helpers and `get_report_counts` (Tauri-only, no HTTP route) using the same `Action::ReadData` pattern as the 8 list_* helpers, since they are read-only data operations with no mutation side effects.
- Kept `build_printers_refresh`'s pre-existing `Action::ReadPrinters` check completely untouched — confirmed via grep it remains the sole `ReadPrinters` reference and the new `ReadData` count for printers.rs is exactly 2 (list, get).
- Fixed a `cargo fmt` line-wrap nit in `acts.rs` (3 functions collapsed from 4-line to 1-line signatures) that was introduced when Task 1 added the `caller: &Identity` parameter — necessary because CLAUDE.md mandates `cargo fmt` as a CI gate, and this was a direct side effect of this plan's own edit.
- Logged 3 pre-existing, unrelated clippy/fmt issues (`template_service.rs` len_zero warnings, `backup_service.rs` disallowed_methods error, `ws_upgrade_serve_connection.rs` fmt drift) to `deferred-items.md` rather than fixing them — out of scope per the deviation-rules scope boundary.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed rustfmt line-wrap nit in acts.rs introduced by Task 1**
- **Found during:** Task 3 (pre-commit fmt check)
- **Issue:** Adding the `caller: &Identity` parameter to `build_acts_get`, `build_acts_counts`, `build_acts_peek_next_number` in Task 1 pushed their signatures to 4-line wrapped form even though they now fit under rustfmt's line-length limit on one line — a cosmetic but CI-blocking formatting violation (`cargo fmt` is a hard CI gate per CLAUDE.md).
- **Fix:** Ran `cargo fmt -p trackly-app`, reverted the unrelated formatting changes it also made to `ws_upgrade_serve_connection.rs` (out of scope), kept only the in-scope `acts.rs` fix.
- **Files modified:** `crates/trackly-app/src/tauri_cmds/acts.rs`
- **Verification:** `rustfmt --edition 2021 --check` on the file passes; `cargo build -p trackly-app` and `cargo test --test role_endpoint_matrix -- --test-threads=1` both still pass after the fix.
- **Committed in:** `40b958c` (Task 3 commit)

---

**Total deviations:** 1 auto-fixed (1 bug fix, formatting/CI-correctness)
**Impact on plan:** Necessary follow-up to Task 1's own edit; no scope creep, no behavior change.

## Issues Encountered

- `cargo clippy -p trackly-app --tests` fails at the workspace/test-binary level due to 2 pre-existing issues unrelated to this plan (`template_service.rs` len_zero warnings, `backup_service.rs` disallowed_methods error blocking compilation of that one test binary). Verified in isolation that `cargo clippy -p trackly-app --test role_endpoint_matrix` (the test actually touched by this plan) and the underlying lib compile cleanly with zero warnings. Logged the pre-existing issues to `deferred-items.md` rather than fixing them, per the scope-boundary rule (fixes must be directly caused by the current task's changes).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- All 5 read-domain BFLA gaps (devices, acts, cartridges, printers, reports) are closed; Employee role now correctly receives 403 Forbidden on every list/get/search/status-counts/history/low-stock/suggest endpoint across both HTTP and Tauri transports.
- `role_endpoint_matrix.rs` now has 19 passing cases serving as a regression guard for any future endpoint additions in these 5 domains.
- Plan 10-03 (employee-facing UI work) can proceed knowing the backend authorization boundary is now correctly enforced — UI-side conditional rendering for Employee role will be defending in depth, not the only line of defense.
- 3 pre-existing, unrelated lint/fmt issues remain logged in `deferred-items.md` for a future cleanup pass — they do not block this phase.

---
*Phase: 10-employee-employee-ui-role-gating-read*
*Completed: 2026-06-21*

## Self-Check: PASSED

All 13 claimed files verified present on disk; all 3 task commit hashes (`5d83f02`, `b4ed01a`, `40b958c`) verified present in git log.

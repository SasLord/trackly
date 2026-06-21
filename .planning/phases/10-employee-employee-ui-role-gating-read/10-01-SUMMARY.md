---
phase: 10-employee-employee-ui-role-gating-read
plan: 01
subsystem: auth
tags: [rbac, authorize, axum, rusqlite, integration-test, tdd-cross-plan]

# Dependency graph
requires:
  - phase: 05-auth-server-mode
    provides: authorize() permission matrix, Action enum, role_endpoint_matrix.rs CI test scaffold
provides:
  - "Action::ReadData restricted to Admin|Manager in authorize() — root-cause fix for D-GATE-01/02"
  - "post_with_cookie_json body-aware HTTP test helper (status + parsed JSON) for Plan 10-02/10-03 body assertions"
  - "role_endpoint_matrix.rs Case 9 flipped to FORBIDDEN (currently RED — Plan 10-02 turns it GREEN by wiring authorize() into devices_list)"
  - "role_endpoint_matrix.rs Case 10 proving Employee's own requests_list read survives the matrix fix"
affects: [10-02-devices-acts-cartridges-printers-reports-read-gating, 10-03-requests-ownership-scoping, 10-04-employee-frontend]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Cross-plan RED/GREEN TDD: a matrix-level fix and its CI assertion can land in one plan while the runtime call-sites that make the assertion pass land in a later plan, as long as the failure is isolated, bisectable, and the contract is documented in both plans' interface blocks"
    - "post_with_cookie_json: body-aware HTTP test helper coexisting with status-only post_with_cookie — callers choose per-assertion which they need"

key-files:
  created: []
  modified:
    - crates/trackly-core/src/auth.rs
    - crates/trackly-app/tests/role_endpoint_matrix.rs

key-decisions:
  - "Treated Plan 10-02's interface-block reference ('reflects post-Plan-10-01 state with ... flipped Case 9/new Case 10') as authoritative over this plan's literal verify-command text ('exits 0'); the two plans were authored as a single RED/GREEN TDD pair split across plan boundaries, and Plan 10-01's matrix fix has zero existing authorize(ReadData) call sites to land on, so Case 9 failing here is the documented, expected, bisectable state."
  - "Case 10 uses RequestFilter/Pagination camelCase JSON shape (requestType/assignedToUserId/requestedByUserId) matching the existing #[serde(rename_all=\"camelCase\")] on ListPayload — verified against requests.rs dto directly rather than guessing field names."

requirements-completed: [D-GATE-01, D-GATE-02, D-TEST-01]

# Metrics
duration: 12min
completed: 2026-06-21
---

# Phase 10 Plan 01: Fix authorize() ReadData matrix + body-aware test helper Summary

**Moved `Action::ReadData` out of the always-true arm in `authorize()` into Admin|Manager (root-cause fix for the Employee over-read bug), and added a body-aware HTTP test helper (`post_with_cookie_json`) plus a flipped/new CI matrix case — Case 9 is intentionally RED at the end of this plan pending Plan 10-02's call-site wiring.**

## Performance

- **Duration:** 12 min
- **Started:** 2026-06-21T05:51:08Z
- **Completed:** 2026-06-21T05:57:52Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- `authorize()` permission matrix: `Action::ReadData` now requires `Role::Admin | Role::Manager`; Employee is rejected. This is the single root-cause match-arm move every downstream `authorize(caller, &Action::ReadData)` call site (added across Plan 10-02) depends on.
- Flipped `authorize_employee_read_data_ok` → `authorize_employee_read_data_forbidden`; added `authorize_manager_read_data_ok` regression guard. All 13 `trackly-core` unit tests pass; `no_io_deps` gate still green (zero I/O imports in `auth.rs`).
- Added `post_with_cookie_json` test helper (status + parsed JSON body) in `role_endpoint_matrix.rs`, kept alongside the existing status-only `post_with_cookie` — both are now available for Plan 10-02/10-03 body-content assertions (e.g. "only own requests in the list", "no org-wide fields in employee dashboard").
- Flipped Case 9 (Employee → `devices_list`) from `200 OK` to `403 Forbidden`. Added Case 10 (Employee → `requests_list` with empty filter → `200 OK`), proving the `ReadData` matrix fix does not collaterally break Employee's own-requests read path (`ReadRequests` is a separate, untouched action).

## Task Commits

1. **Task 1: Fix the authorize() permission matrix — move ReadData out of the always-true arm** - `3cd751a` (fix)
2. **Task 2: Add body-aware HTTP test helper + flip Case 9 + add retained-access regression case** - `1f796bd` (test)

**Plan metadata:** (this commit, docs)

## Files Created/Modified
- `crates/trackly-core/src/auth.rs` — `Action::ReadData` moved into the `Admin | Manager` match arm; doc-comment permission table updated (ReadData row: ✓ ✓ ✗); `authorize_employee_read_data_ok` renamed/flipped to `authorize_employee_read_data_forbidden`; added `authorize_manager_read_data_ok`.
- `crates/trackly-app/tests/role_endpoint_matrix.rs` — added `post_with_cookie_json` helper; module doc-comment matrix list updated to 10 cases; Case 9 flipped to `FORBIDDEN`; Case 10 added (`requests_list` retained-access regression); `requests_list_payload` fixture added with correct camelCase `RequestFilter` field names.

## Decisions Made
- **Cross-plan TDD sequencing accepted as designed, not a defect to "fix locally."** Plan 10-01's `<files_modified>` frontmatter scopes this plan to `auth.rs` + the test file only — it explicitly does not touch `crates/trackly-app/src/tauri_cmds/devices.rs` or `http/devices.rs` ("no other file in this plan touches business logic"). Verified via `grep -rn "Action::ReadData"` that **zero** call sites exist anywhere in `trackly-app` before this plan — the plan's premise ("every downstream call-site that already invokes authorize(caller, &Action::ReadData)") describes Plan 10-02's *output*, not current reality. Plan 10-02's own `<read_first>` block for its Task 1 confirms it inherits this test file in "post-Plan-10-01 state with post_with_cookie_json helper and flipped Case 9/new Case 10," and its Task 1 behavior note says "Test (added in Task 3, this task only needs to make it pass)" — referring to *this* test, added here in 10-01. This is the intended RED (10-01) → GREEN (10-02) split. Resolved by following the cross-plan design intent (confirmed by direct inspection of 10-02-PLAN.md) over the literal but locally-impossible-to-satisfy verify-command text in 10-01-PLAN.md.
- Verified Case 10's payload shape against `crates/trackly-app/src/dto/request.rs` directly (`RequestFilter { status, request_type, assigned_to_user_id, requested_by_user_id }`, all `Option`, `#[serde(rename_all = "camelCase")]` on the wrapper) rather than assuming field names from the plan's `<interfaces>` block, which only showed `device_list_payload` as a reference shape.
- Temporarily verified Case 10 passes in isolation (by locally no-opping Case 9's assertion in a scratch copy, running the test, then restoring the committed file byte-for-byte from a `/tmp` backup) since the two cases run sequentially inside one `#[tokio::test]` function and a panic in Case 9 prevents Case 10 from executing in the same run. This is a verification-only step — the restored file is identical to what was committed in `1f796bd`.

## Deviations from Plan

### Auto-fixed Issues

None — no Rule 1/2/3 auto-fixes were needed; both tasks landed as specified in the plan text.

### Plan-design deviation (documented, not auto-fixed)

**1. [Plan defect — literal verify-command vs. cross-plan TDD intent] Task 2's `<verify>` says `cargo test --test role_endpoint_matrix -- --test-threads=1` must exit 0, but this is unsatisfiable within this plan's own file scope**
- **Found during:** Task 2 verification
- **Issue:** The plan's Task 2 `<done>` criteria states the matrix test "passes with Case 9 now asserting FORBIDDEN." Running the verify command after implementing Task 2 exactly as specified produces a real failure: `Case 9: Employee → devices_list ... expected 403, got 200 OK`, because no `build_*` read helper in `trackly-app` calls `authorize(caller, &Action::ReadData)` yet — confirmed via `grep -rn "Action::ReadData" crates/trackly-app/` returning zero hits before this plan, and zero hits after (this plan touches only `auth.rs` and the test file, per its own `files_modified` frontmatter and explicit "no other file in this plan touches business logic" instruction).
- **Resolution:** Cross-referenced Plan 10-02's PLAN.md directly. Its Task 1 `<read_first>` block states the test file "reflects post-Plan-10-01 state with post_with_cookie_json helper and flipped Case 9/new Case 10," and its Task 1 `<behavior>` says "Test (added in Task 3, this task only needs to make it pass)" for the devices_list 403 case — i.e., 10-02 Task 1 is the GREEN step for the RED test added here. Proceeded with Task 2 exactly as written (flip Case 9, add Case 10), accepting that `cargo test --test role_endpoint_matrix` exits non-zero at the end of this plan. This mirrors the project's own TDD plan-level gate semantics (RED commit before GREEN commit) but split across plan boundaries instead of within one plan.
- **Verification:** `cargo test --test role_endpoint_matrix -- --test-threads=1` → fails exactly at Case 9's assertion (`left: 200, right: 403`), confirming the failure is isolated to the intended line and bisectable. Confirmed Case 10 passes on its own merits by temporarily no-opping Case 9's assertion in a scratch copy of the file, running the suite (`1 passed`), then restoring the exact committed file from a `/tmp` backup (no net change to the committed diff). `cargo build -p trackly-app --tests` exits 0 (compiles cleanly). `cargo clippy -p trackly-app --test role_endpoint_matrix --no-deps -- -D warnings` and `cargo clippy -p trackly-core --no-deps -- -D warnings` both exit 0. `cargo fmt --check` reports no diff for either file I modified (the only fmt diffs present are pre-existing, in an unrelated file, `ws_upgrade_serve_connection.rs`, out of scope per the scope-boundary rule).
- **Committed in:** `1f796bd` (Task 2 commit) — the RED state is the committed state; Plan 10-02 will flip it GREEN.

---

**Total deviations:** 1 documented plan-design deviation (cross-plan TDD sequencing), 0 auto-fixes.
**Impact on plan:** No scope creep — this plan's diff is exactly `auth.rs` + `role_endpoint_matrix.rs`, matching its `files_modified` frontmatter. The "failure" is the plan's own intended test-first signal for Plan 10-02, not a defect introduced by this execution.

## Issues Encountered

`cargo test --test role_endpoint_matrix -- --test-threads=1` exits non-zero at the end of this plan (Case 9 fails: expected 403, got 200). This is expected and load-bearing — see "Plan-design deviation" above. **Plan 10-02 Task 1 must turn this test green** by adding `authorize(caller, &Action::ReadData)` to `build_devices_list` (and sibling read helpers) per its own plan text; until then, `cargo test --test role_endpoint_matrix` will report this single failure if run standalone. This is the correct bisectable state, not a regression to chase down within this plan.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Plan 10-02 can proceed immediately: the matrix fix (`auth.rs`) it depends on is in place and verified (13/13 `trackly-core` tests green, `no_io_deps` gate green).
- Plan 10-02 inherits `role_endpoint_matrix.rs` exactly as its own `<read_first>` describes ("post-Plan-10-01 state with post_with_cookie_json helper and flipped Case 9/new Case 10") — no further setup needed in that file before 10-02 adds its own cases.
- `post_with_cookie_json` is available at module scope in `role_endpoint_matrix.rs` for Plan 10-02/10-03 to reuse directly (same file, no visibility change needed, consistent with how `post_with_cookie` and other local helpers are already reused across cases in this file).
- **Known blocker for anyone running the full test suite standalone:** `cargo test --test role_endpoint_matrix -- --test-threads=1` will report 1 failure (Case 9) until Plan 10-02 lands. This is expected; do not attempt to "fix" it within Plan 10-01's scope.

---
*Phase: 10-employee-employee-ui-role-gating-read*
*Completed: 2026-06-21*

---
phase: 10-employee-employee-ui-role-gating-read
plan: 03
subsystem: auth
tags: [rbac, bola, ownership, authorize, dashboard, rusqlite, role-gating]

# Dependency graph
requires:
  - phase: 10-employee-employee-ui-role-gating-read
    provides: "Plan 10-01 (post_with_cookie_json body-aware helper, Action::ReadData Admin|Manager only) and Plan 10-02 (read-path gating across 5 domains, role_endpoint_matrix at 19 cases)"
provides:
  - "request_service.list force-overrides filter.requested_by_user_id = caller.user_id for Role::Employee (server-side scope, client input ignored) — D-REQ-01"
  - "request_service.get and get_history enforce ownership (resource.requested_by_user_id == caller.user_id else Forbidden) — BOLA close on both methods"
  - "request_service.counts scoped per-caller; RequestRepository::counts trait + rusqlite impl widened with requested_by_user_id: Option<i64>"
  - "dashboard_service employee branch is a separate narrow request-scoped query path (no devices/cartridges/printers tables touched); org-wide DashboardWidgetDto fields returned zeroed/empty — D-GATE-03"
  - "Both transports (HTTP + Tauri) thread real Identity into requests_get/get_history/list and dashboard_get_all_widgets"
  - "role_endpoint_matrix.rs extended to Cases 20-24 (own-requests scope, BOLA get/get_history, employee dashboard body assertions, Manager regression)"
affects: [10-04, employee-ui, ui-role-gating]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Server-side scope override: Employee callers have filter.requested_by_user_id forced to caller.user_id regardless of client-supplied value (tampering mitigation)"
    - "BOLA ownership guard: get/get_history compare resource owner to caller.user_id and return AppError::Forbidden for non-owner non-Admin/Manager"
    - "Separate narrow query path for the most-restricted role (employee dashboard) rather than filtering an org-wide payload — proves on the wire no org tables were queried"
    - "Trait signature widened before impl (E0050/E0053 avoidance); trackly-core no_io_deps gate stays green (Option<i64> adds no I/O import)"

key-files:
  created:
    - .planning/phases/10-employee-employee-ui-role-gating-read/10-03-SUMMARY.md
  modified:
    - crates/trackly-core/src/ports/requests.rs
    - crates/trackly-infra/src/repos/requests_sqlite.rs
    - crates/trackly-app/src/services/request_service.rs
    - crates/trackly-app/src/services/dashboard_service.rs
    - crates/trackly-app/src/http/requests.rs
    - crates/trackly-app/src/http/dashboard.rs
    - crates/trackly-app/src/tauri_cmds/requests.rs
    - crates/trackly-app/src/tauri_cmds/dashboard.rs
    - crates/trackly-app/tests/role_endpoint_matrix.rs

key-decisions:
  - "Case numbers shifted from the plan's stated 16-20 to actual 20-24 because Plan 10-02 had already grown the matrix to 19 cases; new cases were appended after the highest existing case to avoid renumbering churn"
  - "Employee dashboard returns org-wide fields zeroed/empty (not omitted) since DashboardWidgetDto is shared across roles — the zeroed values prove the employee path never queried devices/cartridges/printers tables"
  - "Ownership guard applied to BOTH get() and get_history() (not just get_history as originally flagged in research) — defense-in-depth per checker BLOCKER resolution"

requirements-completed: [D-REQ-01, D-GATE-03, D-TEST-01]

# Metrics
duration: 25min
completed: 2026-06-21
---

# Phase 10 Plan 03: Request ownership scope + BOLA close + employee-scoped dashboard Summary

**Enforced Employee request-scope server-side (own requests only), closed the BOLA gap on `requests_get`/`requests_get_history` for both transports, and gave the employee dashboard a separate narrow request-scoped query path so it cannot leak org-wide device/cartridge/printer data — proven by 5 new CI cases.**

## Performance

- **Duration:** ~25 min (executor tasks 1-2) + orchestrator finish of task 3 after the executor hit a session/usage limit mid-task
- **Completed:** 2026-06-21
- **Tasks:** 3/3
- **Files modified:** 9 source/test files

## Accomplishments
- **D-REQ-01:** `request_service.list` force-overrides `filter.requested_by_user_id = caller.user_id` for `Role::Employee`, ignoring any client-supplied value (tampering/privilege-escalation mitigation). `counts` scoped per-caller via a widened trait + rusqlite impl signature.
- **BOLA close:** `request_service.get` and `get_history` now compare the resource's `requested_by_user_id` to `caller.user_id` and return `AppError::Forbidden` for a non-owner Employee — an Employee can no longer fetch another user's request by guessing an id. All 7 internal `self.get(...)` call sites updated to thread `caller`.
- **D-GATE-03:** `dashboard_service` employee branch is a genuinely separate narrow query path that never touches devices/cartridges/printers tables; org-wide `DashboardWidgetDto` fields come back zeroed/empty.
- Both transports thread a real `Identity` into `requests_get`/`get_history`/`list` and `dashboard_get_all_widgets` (Tauri via `resolve_tauri_identity`, HTTP via the previously-discarded session identity).
- `role_endpoint_matrix.rs` Cases 20-24 added: Employee own-requests-only list, BOLA 403 on get/get_history for a manager-owned id, employee dashboard org fields zeroed (snake_case body assertions), Manager regression (dashboard keys present, get/get_history of employee-owned id not 401/403).

## Task Commits

1. **Task 1: Force-override requester scope + close BOLA on get/get_history** — `5c2f27f` (feat) — `request_service.rs`, `ports/requests.rs`, `requests_sqlite.rs`
2. **Task 2: Thread caller through both transports + employee-scoped dashboard** — `1508ab8` (feat) — `http/requests.rs`, `http/dashboard.rs`, `dashboard_service.rs`, `tauri_cmds/requests.rs`, `tauri_cmds/dashboard.rs`
3. **Task 3: Add Cases 20-24** — `0c02e6d` (test) — `role_endpoint_matrix.rs`

## Verification

- `cargo test --test role_endpoint_matrix -- --test-threads=1` → **ok** (all cases pass, including 20-24).
- Tasks 1-2 build verified clean by the executor (`cargo build -p trackly-app`, zero new warnings).
- `role_endpoint_matrix.rs` is rustfmt-clean (the only outstanding fmt drift is in the unrelated pre-existing `ws_upgrade_serve_connection.rs`, logged in `deferred-items.md` by Plan 10-02).

## Deviations from Plan

**1. Case numbering 16-20 → 20-24.** Plan 10-03 specified Cases 16-20, but Plan 10-02 had already extended the matrix to Case 19. New cases were appended as 20-24 to follow the highest existing case (the plan's own reminder anticipated this and instructed renumbering on collision). No behavioral difference.

**2. Task 3 finished by the orchestrator.** The executor agent completed and committed Tasks 1 and 2, then wrote the Task 3 test code but hit a provider session/usage limit before running, committing, or writing this SUMMARY. The orchestrator verified the uncommitted test code compiled and passed (`cargo test --test role_endpoint_matrix` → ok), then committed it (`0c02e6d`), wrote this SUMMARY, and updated tracking. No code was authored by the orchestrator beyond confirming the already-written test.

## Issues Encountered

- Provider session limit interrupted the executor mid-Task-3 (work was recoverable: backend tasks were already committed, test code was on disk uncommitted and passing).

## User Setup Required

None.

## Next Phase Readiness

- The entire backend authorization boundary for Phase 10 is now complete and CI-guarded: read-gating (10-02), own-requests scope + BOLA + scoped dashboard (10-03). Plan 10-04 (frontend employee shell + access-denied screen + dashboard card + client.ts 403) can build on a fully-enforced backend — UI gating will be defense-in-depth, not the only control.
- No DB migration was required (confirmed — `migrations/` untouched).

---
*Phase: 10-employee-employee-ui-role-gating-read*
*Completed: 2026-06-21*

## Self-Check: PASSED

All 9 claimed files verified modified in commits 5c2f27f / 1508ab8 / 0c02e6d; role_endpoint_matrix test passes.

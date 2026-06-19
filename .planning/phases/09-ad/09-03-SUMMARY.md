---
phase: 09-ad
plan: 03
subsystem: auth

tags: [requests, request_service, auth_service, ad_register, single-writer, authorize]

# Dependency graph
requires:
  - phase: 09-ad (plan 02)
    provides: AuthService AD fallback (try_local_login/try_ad_login), find_user_any_state read seam, ad_enabled/ad_auto_accept settings, V028 requests.ad_subtype column, on_ad_bind_success active-user happy path with typed TODOs for the unknown/blocked branches
provides:
  - on_ad_bind_success() complete for ALL states: active (09-02) / unknown-auto-accept / unknown-pending / blocked-or-deleted-restore (this plan)
  - AppError::RegistrationPending{request_id} / AccessBlocked{request_id} typed login outcomes, mapped to HTTP 403
  - RequestRepository::list(..., exclude_ad_register: bool) — admin-only ad_register visibility enforced at the SQL level
  - RequestService::approve_ad_register(payload, caller) — authorize(ManageUsers)-gated, role default employee, activates or revives the target user, completes the request directly (its own open->completed state machine)
  - RequestService::transition() Reject special-case for ad_register requests — dispatches to reject_ad_register() with three distinct semantics (pending discard / auto-accept soft-delete / restore reject)
  - ApproveAdRegisterDto + requests_approve_ad_register Tauri command + HTTP route
  - RequestRow/RequestNew/RequestDto.ad_subtype ("register"|"restore") threaded end-to-end
affects: [09-ad (plan 04/05 — UI for registration/restoration request screens, pending/blocked login screens)]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "ad_register's approve/complete transition bypasses transition_in_tx/RequestTransitionOp::Complete because that op's validate_from_status hard-codes the cartridge/printer state machine (in_progress -> completed); ad_register approve is open -> completed directly, so it does a manual UPDATE ... WHERE version = ? AND status = 'open' with an explicit OptimisticLockMismatch fallback instead"
    - "Reject branching for ad_register is detected by an extra read (self.get(request_id)) before dispatch inside the generic transition() entrypoint, rather than a separate Tauri/HTTP command — callers don't need to know which request_type they're rejecting"
    - "Auto-accept-vs-pending distinction at reject time is resolved by re-checking the target user's current is_active flag at reject time, not by trusting ad_subtype alone — ad_subtype only distinguishes register vs restore, not whether auto-accept already activated the user"
    - "Admin-only list filtering is a boolean computed from caller.role at the service boundary (RequestService::list), passed through to the repository as an explicit SQL predicate — never a post-query filter, never row-hidden in the DTO layer"

key-files:
  created:
    - crates/trackly-app/tests/ad_register.rs
    - crates/trackly-app/tests/requests_ad_register.rs
  modified:
    - crates/trackly-app/src/services/auth.rs
    - crates/trackly-app/src/services/request_service.rs
    - crates/trackly-core/src/domain/requests.rs
    - crates/trackly-core/src/ports/requests.rs
    - crates/trackly-core/src/error.rs
    - crates/trackly-infra/src/repos/requests_sqlite.rs
    - crates/trackly-app/src/dto/request.rs
    - crates/trackly-app/src/error_axum.rs
    - crates/trackly-app/src/context.rs
    - crates/trackly-app/src/tauri_cmds/requests.rs
    - crates/trackly-app/src/http/requests.rs
    - crates/trackly-app/src/specta_export.rs
    - crates/trackly-app/src/http/health.rs
    - crates/trackly-app/src/tauri_cmds/health.rs
    - crates/trackly-app/tests/ad_auth.rs
    - crates/trackly-app/tests/auth_smoke.rs
    - crates/trackly-app/tests/users_crud.rs
    - crates/trackly-app/tests/specta_roundtrip.rs

key-decisions:
  - "approve_ad_register does a manual UPDATE requests SET status='completed' ... WHERE version=? AND status='open' instead of reusing RequestTransitionOp::Complete — that op's validate_from_status requires the source status to be 'in_progress' (the cartridge/printer state machine), which would make every ad_register approval fail with a misleading 'Операция «Выполнить» недопустима для статуса «open»' validation error"
  - "Reject semantics for ad_register are resolved at runtime by querying the target user's current is_active, not by branching on ad_subtype alone — a 'register' request can still be sitting at is_active=0 (pending, never admitted) when admin auto-accept is OFF, so ad_subtype='register' alone can't tell pending-discard apart from auto-accept-then-rejected"
  - "AppError::RegistrationPending/AccessBlocked both map to HTTP 403 Forbidden (not 401) — the AD bind itself succeeded, so 401 (bad credentials) would be misleading; the caller's identity is known, just not yet admitted (D-REG-01/D-REG-03)"
  - "ws_tx (broadcast::Sender<WsEvent>) added as AuthService's 5th constructor argument, reusing the exact channel RequestService already broadcasts on (created once in context.rs before either service) — avoids a second broadcast channel or a RequestService dependency inside AuthService"

requirements-completed: [USR-09, USR-11, SET-10, REQ-06]

duration: ~110min
completed: 2026-06-19
---

# Phase 9 Plan 3: AD registration/restoration request flows Summary

**Completes `on_ad_bind_success`'s unknown/blocked branches (auto-accept create, pending create, restore request) and adds the matching admin approve-with-role / reject-with-mode-correct-semantics flow in RequestService, with admin-only visibility for `ad_register` requests enforced at the SQL level.**

## Performance

- **Duration:** ~110 min
- **Completed:** 2026-06-19T18:08:30Z
- **Tasks:** 2/2 completed
- **Files modified:** 18 (2 created, 16 modified)

## Accomplishments

- `on_ad_bind_success` now handles all four AD-bind outcomes: active (09-02), unknown+auto-accept-on (create active user + info request), unknown+auto-accept-off (create inactive user + pending request, no session), blocked-or-soft-deleted (restore request referencing the existing user row, no session)
- Each branch is exactly one `self.writer.execute` transaction: INSERT users + INSERT requests(ad_register) + 2 audit_log entries, then a WS `NewRequest` broadcast so admins see new requests live without polling
- `AppError::RegistrationPending{request_id}` / `AccessBlocked{request_id}` give the Tauri/HTTP/UI layers a typed, non-401 signal to show a pending/blocked screen instead of a generic auth failure
- `RequestRepository::list()` gained `exclude_ad_register: bool`, applied as a real SQL predicate (`AND (?5 = 0 OR r.request_type != 'ad_register')`) on both the COUNT and SELECT queries — non-admin callers structurally cannot retrieve `ad_register` rows, regardless of any filter they pass
- `RequestService::approve_ad_register` validates/defaults the admin-selected role (`Role::from_str`, default `"employee"` — D-REG-02), activates or revives the target user, and completes the request via a hand-rolled optimistic-lock UPDATE (the generic `Complete` transition op's state machine doesn't allow `open -> completed` directly)
- `RequestService::transition()`'s `Reject` arm now detects `request_type = "ad_register"` and dispatches to mode-correct semantics: pending discard (no user mutation), auto-accept soft-delete (T-09-14 audit trail), restore reject (blocked user stays blocked)

## Task Commits

Each task was committed atomically:

1. **Task 1: fill on_ad_bind_success unknown/blocked branches** - `419f99b` (feat)
2. **Task 2: admin approve/reject for ad_register requests** - `b39721f` (feat)

**Plan metadata:** (pending — final docs commit below)

## Files Created/Modified

- `crates/trackly-app/src/services/auth.rs` - `on_ad_bind_success` dispatch + `auto_register_ad_user`/`create_pending_registration`/`create_restore_request`; `ws_tx` field + 5th ctor arg
- `crates/trackly-app/tests/ad_register.rs` - 5 tests: auto-accept create, pending create, blocked restore, soft-deleted restore, single-writer atomicity
- `crates/trackly-core/src/error.rs` - `AppError::RegistrationPending`/`AccessBlocked` variants, `code()`/`details_value()`, unit tests
- `crates/trackly-app/src/error_axum.rs` - both new variants mapped to `StatusCode::FORBIDDEN`
- `crates/trackly-app/src/context.rs` - WS broadcast channel created before `AuthService::new()` so the same `ws_broadcast` feeds both `AuthService` and `RequestService`
- `crates/trackly-core/src/domain/requests.rs` - `RequestRow`/`RequestNew.ad_subtype: Option<String>` (no serde derives, per file header rule)
- `crates/trackly-core/src/ports/requests.rs` - `RequestRepository::list(..., exclude_ad_register: bool)`
- `crates/trackly-infra/src/repos/requests_sqlite.rs` - SELECT/COUNT predicate for `exclude_ad_register`, `ad_subtype` column mapping, insert
- `crates/trackly-app/src/dto/request.rs` - `RequestDto.ad_subtype`, new `ApproveAdRegisterDto`
- `crates/trackly-app/src/services/request_service.rs` - `list(caller)` admin-only filter, `approve_ad_register`, `reject_ad_register`, `transition()` ad_register dispatch
- `crates/trackly-app/tests/requests_ad_register.rs` - 6 tests: admin-only visibility, approve with role / default role, reject (pending/auto-accept/restore)
- `crates/trackly-app/src/tauri_cmds/requests.rs`, `crates/trackly-app/src/http/requests.rs` - `requests_approve_ad_register` command/route; `requests_list` now resolves and passes caller identity
- `crates/trackly-app/src/specta_export.rs` - registered the new command
- `crates/trackly-app/src/http/health.rs`, `tauri_cmds/health.rs`, `tests/ad_auth.rs`, `tests/auth_smoke.rs`, `tests/users_crud.rs`, `tests/specta_roundtrip.rs` - updated for `AuthService::new`'s new 5th argument

## Decisions Made

- `approve_ad_register` does a direct, hand-rolled `UPDATE requests SET status='completed' ... WHERE version=? AND status='open'` with a manual `OptimisticLockMismatch` fallback, rather than reusing `RequestTransitionOp::Complete`/`transition_in_tx` — that op's `validate_from_status` requires `in_progress` as the source state (the cartridge/printer Accept→Complete state machine), which would reject every `open`-status ad_register approval with a confusing validation error. ad_register's approve flow is its own single-step state machine (`open -> completed`), not a reuse of the generic one.
- Reject semantics check the target user's *current* `is_active` flag at reject time rather than trusting `ad_subtype` alone to distinguish "still pending" from "already auto-accepted" — `ad_subtype` only encodes register-vs-restore, and a `register` request can be sitting at `is_active=0` for the entire pending-mode lifetime, so only a live re-check correctly drives the soft-delete-vs-no-op branch.
- Both new typed login-outcome errors (`RegistrationPending`, `AccessBlocked`) map to HTTP 403, not 401 — the AD bind succeeded and the caller's identity is known; 401 would incorrectly suggest bad credentials.
- `ws_tx` reuses the single broadcast channel created once in `context.rs`, passed to both `AuthService` and `RequestService`, rather than creating a second channel or giving `AuthService` a `RequestService` dependency — keeps the WS notification plumbing flat and avoids a circular service dependency.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing critical functionality] AppError::RegistrationPending/AccessBlocked had no HTTP status mapping**
- **Found during:** Task 1, after implementing `on_ad_bind_success`'s real return types and rebuilding — `error_axum.rs`'s `IntoResponse` match was non-exhaustive (E0004) once the two new `AppError` variants existed.
- **Issue:** The plan specifies the typed outcomes but doesn't explicitly call out the HTTP transport wiring; without a status-code mapping, the LAN browser path would not compile, let alone surface the correct screen to a pending/blocked AD user.
- **Fix:** Mapped both variants to `StatusCode::FORBIDDEN` (403) with an inline rationale comment distinguishing them from a generic 401.
- **Files modified:** `crates/trackly-app/src/error_axum.rs`
- **Commit:** `419f99b` (Task 1 commit)

**2. [Rule 1 - Bug] approve_ad_register's initial implementation reused RequestTransitionOp::Complete, which rejects every approval**
- **Found during:** Task 2, while writing `approve_creates_user_with_selected_role`/`approve_default_role_employee`/`approve_restore_revives_user` — all three failed with `Validation { field: "status", message: "Операция «Выполнить» недопустима для статуса «open»" }`.
- **Issue:** `RequestTransitionOp::Complete`'s `validate_from_status` hard-codes `"in_progress"` as the only valid source state (the cartridge/printer Accept→Complete state machine); ad_register requests are created directly at `"open"` and approved in one step, so they never pass through `"in_progress"`.
- **Fix:** Replaced the `transition_in_tx`/`RequestTransitionOp::Complete` call inside `approve_ad_register` with a direct `UPDATE requests SET status='completed', ... WHERE id=? AND version=? AND status='open'` plus a manual `OptimisticLockMismatch` error path when the row-count is 0.
- **Files modified:** `crates/trackly-app/src/services/request_service.rs`
- **Verification:** All 6 `requests_ad_register.rs` tests pass.
- **Committed in:** `b39721f` (Task 2 commit)

---

**Total deviations:** 2 (1 Rule 2 — missing HTTP wiring for new error variants; 1 Rule 1 — bug in the first approve_ad_register draft, fixed before any commit)
**Impact on plan:** Both fixes were necessary for the plan's stated `must_haves` to actually hold; no scope creep beyond AD registration/restoration request handling.

## Issues Encountered

No new pre-existing/out-of-scope issues discovered beyond the two already logged in `.planning/phases/09-ad/deferred-items.md` from plans 09-02 and 09-03's own Task 1/2 verification pass (the `backup_service.rs` clippy::disallowed_methods entry was added during this plan's Task 1 verification and is documented there, confirmed pre-existing via `git status`/`git stash` and out of scope for the plan's literal verify command).

## Known Stubs

None. All write paths (auto-register, pending registration, restore request, approve, reject) are fully wired end-to-end: AuthService -> writer transaction -> WS broadcast -> RequestService admin-only list/approve/reject -> Tauri command + HTTP route -> specta export. No hardcoded empty values or placeholder UI data introduced by this plan (this plan is service-layer only; the UI screens for pending/blocked/admin-approve land in a later plan per the dependency graph).

## Threat Flags

| Flag | File | Description |
|------|------|-------------|
| threat_flag: new-endpoint | crates/trackly-app/src/http/requests.rs | New `/api/v1/requests_approve_ad_register` POST route — already covered by the plan's T-09-12 (mitigated: `authorize(ManageUsers)` gate + `Role::from_str` validation + default-employee), no additional mitigation needed beyond what's implemented. |

## User Setup Required

None — no external service configuration required. Continues to work with `TRACKLY_AD_MOCK=1` and zero AD infrastructure for dev/test on macOS.

## Next Phase Readiness

- Service layer for AD registration/restoration is complete: `AuthService`, `RequestService`, both transports (Tauri + HTTP), and specta bindings all expose the full create/list/approve/reject surface.
- The next plan(s) in this phase can build the UI: pending/blocked login screens (consuming `AppError::RegistrationPending`/`AccessBlocked`), the admin-only `ad_register` requests list/approve-with-role-picker/reject UI, and the live WS-driven "new registration request" notification.
- No blockers carried forward. The two deferred pre-existing failures (`graceful_shutdown_drain`, `template_service.rs`/`backup_service.rs` clippy under non-default flag sets) remain independent of this plan's scope.

---
*Phase: 09-ad*
*Completed: 2026-06-19*

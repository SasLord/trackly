---
phase: 11-requests-employee-ux-gaps
plan: 03
subsystem: requests
tags: [websocket, rust, svelte, rbac, bola, notifications]

# Dependency graph
requires:
  - phase: 11-requests-employee-ux-gaps
    provides: "11-01: category pipeline conventions; 11-02: request_printer_options BOLA-closure pattern, GroupedPrinterSelect component used by RequestFormModal"
provides:
  - "WsEvent::RequestStatusChanged carries requested_by_user_id; is_visible_to split-arm lets the employee-author see only their own status change"
  - "EmployeeLayout.svelte WS subscription — realtime RU toast / system Notification on the employee's own request status change"
  - "RequestFormModal.svelte delicate Notification.requestPermission gesture-gated prompt"
affects: [requests, employee-ux, websocket, notifications]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Split-arm is_visible_to: events with a clear 'owner' field (requested_by_user_id) get OR'd visibility (admin|manager OR identity.user_id == owner) instead of a blanket role check — applies the same closure pattern as 11-02's request_printer_options DTO trimming, but for the WS broadcast trust boundary instead of an HTTP endpoint."
    - "Page Visibility + Notification API graceful-degrade: canNotify gate (Notification in window && isSecureContext && permission==='granted') checked at event-time, document.hidden decides toast vs system notification — never throws outside secure context."
    - "Gesture-gated permission request: Notification.requestPermission() called only inside a success handler that already required a user-initiated submit, gated additionally on permission==='default' so it never re-prompts after grant/deny."

key-files:
  created: []
  modified:
    - crates/trackly-app/src/dto/printer.rs
    - crates/trackly-app/src/services/request_service.rs
    - crates/trackly-app/src/http/requests.rs
    - crates/trackly-app/tests/ws_broadcast_fanout.rs
    - ui/src/bindings-phase6.ts
    - ui/src/features/layout/EmployeeLayout.svelte
    - ui/src/features/requests/RequestFormModal.svelte

key-decisions:
  - "is_visible_to split into 3 explicit match arms (PrinterAlert / NewRequest / RequestStatusChanged) instead of the prior merged NewRequest|RequestStatusChanged arm — RequestStatusChanged alone gets the OR'd employee-author clause; NewRequest stays Admin|Manager-only since employees must not see other employees' new submissions."
  - "Fixed 2 additional pre-existing send-sites in http/requests.rs (handler_transition, handler_approve_ad_register) that the plan's interface section did not enumerate but which construct the same WsEvent::RequestStatusChanged struct literal and would not compile without the new field — applied as Rule 3 (blocking compile issue), same atomic change as the 3 named send-sites in request_service.rs."
  - "Updated the ws_broadcast_fanout.rs regression test (proves tokio::sync::broadcast fans an identical event to every subscriber) to carry requested_by_user_id in its fixture and pattern-match — kept the test meaningful rather than letting it bit-rot against the new field."
  - "EmployeeLayout.svelte owns the employee WS subscription (lives for the whole employee session, per plan's placement note) — RequestsPage.svelte's existing connectWs/onWsEvent subscription (used by all roles) was left untouched: its new_request toast branch is already gated to admin|manager and it never toasts on request_status_changed, so there is no double-toast even though both subscriptions are simultaneously active when an employee is on /requests."
  - "statusToastText defaults to a generic 'Статус вашей заявки изменён' for any status value outside the 3 named ones (in_progress/completed/rejected) rather than silently saying nothing — defensive against future status additions."

requirements-completed: [D-WS-01]

# Metrics
duration: ~35min
completed: 2026-06-21
---

# Phase 11 Plan 03: Employee realtime request-status notifications (D-WS-01) Summary

**Closed D-WS-01: the employee who submitted a request now gets a realtime WebSocket-driven notification — RU toast while the tab is active, or a system `Notification` when the tab is hidden and permission was granted — scoped server-side to ONLY their own request via a new `requested_by_user_id` field on `WsEvent::RequestStatusChanged` and a split `is_visible_to` arm; permission is requested gently, only after the employee's first successful request submission.**

## Performance

- **Duration:** ~35 min
- **Completed:** 2026-06-21
- **Tasks:** 2/2
- **Files modified:** 7

## Accomplishments

- `WsEvent::RequestStatusChanged` (dto/printer.rs) gained a `requested_by_user_id: i64` field (`#[specta(type = i32)]`); `is_visible_to` was split from a merged `NewRequest | RequestStatusChanged` arm into 3 distinct arms — `NewRequest` stays Admin|Manager-only, `RequestStatusChanged` is now `Admin|Manager OR identity.user_id == Some(requested_by_user_id)`, letting the request's employee-author see their own status changes without ever seeing a coworker's.
- 4 new unit tests in `dto/printer.rs` cover exactly the cases called out in the plan: author-employee → true, other-employee → false, admin/manager → true, and a `NewRequest` regression guard proving the split didn't accidentally widen that event's visibility to employees.
- All 5 `WsEvent::RequestStatusChanged` construction sites across the codebase now fill `requested_by_user_id` — the 3 service-layer send-sites named in the plan (`transition`, `approve_ad_register`, `reject_ad_register`, all reading the in-scope `dto.requested_by_user_id`) plus 2 pre-existing HTTP-transport re-broadcast sites in `http/requests.rs` that the plan's interface section didn't name but needed the same field to keep compiling.
- `EmployeeLayout.svelte` (the layout that lives for the entire employee session) now opens a WS subscription on mount for the employee role: on `request_status_changed` it shows a Russian toast (`Ваша заявка принята в работу` / `выполнена` / `отклонена`) if the tab is visible, or a system `Notification` if `document.hidden` and `Notification.permission === 'granted'` in a secure context — falling back to the toast in every other case (no secure context, no permission, denied).
- `RequestFormModal.svelte` requests Notification permission delicately: only inside the success branch of submitting a request (a genuine user gesture), only if `Notification.permission === 'default'` — never on page load, never re-prompting after the user has already answered.
- Confirmed (via code read, no change needed) that `RequestsPage.svelte`'s existing `new_request` toast is already gated to `admin|manager` and that branch never fires on `request_status_changed` — so the new `EmployeeLayout` subscription running concurrently with `RequestsPage`'s own WS subscription (both active when an employee is on `/requests`) produces no double-toast.

## Task Commits

Each task was committed atomically:

1. **Task 1: Backend — WsEvent payload += requested_by_user_id + is_visible_to split-arm + 3 (+2) send-sites** - `c13c2a5` (feat)
2. **Task 2: Frontend — employee WS subscription (toast/notification) + delicate permission prompt** - `b4719b4` (feat)

**Plan metadata:** _(pending — final docs commit below)_

## Files Created/Modified

- `crates/trackly-app/src/dto/printer.rs` - `WsEvent::RequestStatusChanged += requested_by_user_id`; `is_visible_to` split into `PrinterAlert` / `NewRequest` / `RequestStatusChanged` arms; +4 `#[cfg(test)]` unit tests
- `crates/trackly-app/src/services/request_service.rs` - 3 send-sites (`transition`, `approve_ad_register`, `reject_ad_register`) now pass `requested_by_user_id: dto.requested_by_user_id`
- `crates/trackly-app/src/http/requests.rs` - 2 pre-existing HTTP re-broadcast send-sites (`handler_transition`, `handler_approve_ad_register`) updated to compile against the new struct shape (`result.requested_by_user_id`)
- `crates/trackly-app/tests/ws_broadcast_fanout.rs` - test fixture + pattern-match extended with `requested_by_user_id` so the fan-out regression test stays meaningful
- `ui/src/bindings-phase6.ts` - `WsEvent`'s `request_status_changed` variant gained `requestedByUserId: number`
- `ui/src/features/layout/EmployeeLayout.svelte` - `onMount` WS subscription (employee-role gated) + `handleEmployeeWsEvent`/`statusToastText` — toast-or-Notification dispatch on `request_status_changed`
- `ui/src/features/requests/RequestFormModal.svelte` - `maybeRequestNotifyPermission()` called in the success branch of `handleSubmit`, after the existing success toast

## Decisions Made

- Split `is_visible_to` into 3 explicit arms rather than adding a conditional inside the merged arm — clearer at the call site that `NewRequest` and `RequestStatusChanged` now have genuinely different visibility rules, and matches the plan's explicit instruction not to leave them merged.
- Fixed the 2 unnamed `http/requests.rs` send-sites as Rule 3 (blocking compile error from the atomic struct change) rather than treating them as out of scope — they construct the exact same `WsEvent::RequestStatusChanged` literal the plan's `dto/printer.rs` edit affects, so leaving them broken would have failed `cargo build` entirely.
- Did not touch `RequestsPage.svelte` — the plan listed it as a file that might need a role-gate adjustment, but on inspection its `new_request` branch was already correctly gated to `admin|manager` and it has no toast at all on `request_status_changed`, so no edit was needed to avoid a double-toast.
- Kept `statusToastText`'s fallback case (any status string outside the 3 named ones) as a generic Russian message rather than silently doing nothing, so a future new status value doesn't produce a silent notification failure.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Fixed 2 additional WsEvent::RequestStatusChanged construction sites not named in the plan**
- **Found during:** Task 1, immediately after editing `dto/printer.rs`
- **Issue:** The plan's `<read_first>`/interface section named exactly 3 send-sites in `request_service.rs`, but `crates/trackly-app/src/http/requests.rs` (`handler_transition` line ~134, `handler_approve_ad_register` line ~154) also construct `WsEvent::RequestStatusChanged { request_id, new_status }` literals as part of the pre-existing HTTP-transport re-broadcast (a 09-ad-gaps-ws-bridge era pattern, documented in those handlers' own comments as "broadcast already done in service layer" — these are deliberate redundant re-broadcasts, not new code). Adding the new struct field without updating these 2 sites would have failed `cargo build -p trackly-app` outright.
- **Fix:** Added `requested_by_user_id: result.requested_by_user_id` to both HTTP handler send-sites, reading from the already-in-scope `result: RequestDto`.
- **Files modified:** `crates/trackly-app/src/http/requests.rs`
- **Commit:** `c13c2a5`

**2. [Rule 3 - Blocking] Updated ws_broadcast_fanout.rs test fixture for the new field**
- **Found during:** Task 1, after the struct-shape change
- **Issue:** `crates/trackly-app/tests/ws_broadcast_fanout.rs` constructs and pattern-matches a `WsEvent::RequestStatusChanged` literal as its test fixture; without the new field it would not compile.
- **Fix:** Added `requested_by_user_id: 7` to the fixture and extended the destructuring match + an extra `assert_eq!` to keep the test asserting something meaningful about the new field (not just ignoring it).
- **Files modified:** `crates/trackly-app/tests/ws_broadcast_fanout.rs`
- **Commit:** `c13c2a5`

## Issues Encountered

- `cargo clippy -p trackly-app --all-targets -- -D warnings` still fails on the same 2 pre-existing `clippy::len_zero` warnings in `crates/trackly-app/src/services/template_service.rs` (lines 379, 430) already tracked in `.planning/phases/09-ad/deferred-items.md` and reconfirmed in the 11-01 SUMMARY — unrelated to this plan's files. Confirmed clean via `cargo clippy -p trackly-app -- -D warnings` (without `--all-targets`), matching the precedent from 11-01/11-02.
- The plan's verify command names (`cargo test -p trackly-app ws_event_visibility`, `cargo test -p trackly-app request_service`) don't literally match any test binary/module name in this codebase's layout — `ws_event_visibility` tests live as `#[cfg(test)] mod tests` inside `dto/printer.rs` (run via `cargo test -p trackly-app --lib dto::printer::tests`), and `request_service`'s 3 send-sites are exercised by the integration tests `request_accept_assignee.rs`, `requests_ad_register.rs`/`requests_ad_register_http.rs`, and `ws_broadcast_fanout.rs`. Ran the equivalent correct invocations instead; all green (4 + 1 + 8 + 3 + 1 = 17 relevant tests passing). The behavioral coverage matches the plan's `<behavior>` spec exactly (author→true, other-employee→false, admin/manager→true, NewRequest regression).

## User Setup Required

None — no external service configuration required.

**Manual verification still pending** (per plan's `<acceptance_criteria>` MANUAL items, left for the user/verifier since they require a live LAN browser session and HTTPS :8443):
- LAN employee browser test: submit a request → confirm the delicate permission prompt appears once; admin changes status → confirm toast (active tab) or system notification (hidden tab + granted); confirm a SECOND employee does NOT receive the first employee's status-change event.
- HTTP first-run fallback test: confirm graceful-degrade to toast with no console errors when Notification API / secure context is unavailable.

`pnpm --dir ui build` was run successfully as part of this plan's automated verification; the bundle in `ui/dist` (gitignored build artifact) is current.

## Next Phase Readiness

- D-WS-01 is fully closed per the plan's success criteria: server-side `is_visible_to` is the sole security boundary (proven by 4 unit tests), payload + visibility were changed atomically, all 5 send-sites are in sync, and `cargo build`/targeted tests/`svelte-check`/`pnpm build` are all green.
- This was the last plan in Phase 11 (3 of 3 per ROADMAP) — no further plans are queued in this phase. The 3 D-requirements this phase targeted (D-CAT-01, D-PRN-01, D-WS-01) are all closed.

---
*Phase: 11-requests-employee-ux-gaps*
*Completed: 2026-06-21*

---
phase: 11-requests-employee-ux-gaps
fixed_at: 2026-06-22T00:05:00Z
review_path: .planning/phases/11-requests-employee-ux-gaps/11-REVIEW.md
iteration: 1
findings_in_scope: 7
fixed: 7
skipped: 0
status: all_fixed
---

# Phase 11: Code Review Fix Report

**Fixed at:** 2026-06-22
**Source review:** .planning/phases/11-requests-employee-ux-gaps/11-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 7 (1 Critical/Blocker + 6 Warning; the 4 Info findings are out of scope for `critical_warning`)
- Fixed: 7
- Skipped: 0

All fixes were applied in an isolated git worktree, verified to compile
(`cargo build -p trackly-app`, `cargo build --bin trackly`), pass `cargo clippy`
clean on the modified crates, and pass their targeted tests. WR-01/WR-02 and the
CR-01 regression test were exercised directly.

## Fixed Issues

### CR-01: Duplicate WebSocket broadcast on every HTTP request mutation (double-fire)

**Files modified:** `crates/trackly-app/src/http/requests.rs`, `crates/trackly-app/tests/ws_http_single_broadcast.rs`
**Commit:** e500b6c
**Applied fix:** Removed the redundant `ctx.ws_broadcast.send(...)` blocks from
all three HTTP handlers (`handler_create`, `handler_transition`,
`handler_approve_ad_register`). `ctx.ws_broadcast` and `RequestService.ws_tx`
are the SAME `Arc<broadcast::Sender>`, so the service layer is the single
broadcast owner and the handler-level re-send delivered every event twice (the
"WS toast spam" symptom). Removed the now-unused `WsEvent` import and corrected
the module doc comment. Added a new regression test
(`tests/ws_http_single_broadcast.rs`) that subscribes to `ctx.ws_broadcast`,
drives one HTTP `requests_create` through `build_router`, and asserts exactly ONE
`WsEvent::NewRequest` arrives (`try_recv` after the first event must return
`Empty`). Test passes.

### WR-01: `ad_register` requests can be self-created by any authenticated user

**Files modified:** `crates/trackly-app/src/services/request_service.rs`, `crates/trackly-app/tests/phase06_stubs.rs`
**Commit:** 202816d
**Applied fix:** Added an allowlist check at the top of `RequestService::create`:
`request_type` must be `cartridge_replace` or `free_form`, else
`AppError::Validation`. This closes the BFLA/invariant gap where a hand-crafted
`requests_create` with `requestType: "ad_register"` could plant a row in the
admin AD-register approval queue. (Committed together with WR-02 — same function,
tightly coupled.)

### WR-02: `create()` does not enforce type-specific required fields

**Files modified:** `crates/trackly-app/src/services/request_service.rs`, `crates/trackly-app/tests/phase06_stubs.rs`
**Commit:** 202816d
**Applied fix:** Mirrored the frontend validation server-side in
`RequestService::create`: `cartridge_replace` now requires
`printer_device_id.is_some()`; `free_form` now requires a non-empty (trimmed)
`description`. Both return `AppError::Validation`. Two existing lifecycle tests in
`phase06_stubs.rs` that created `free_form` requests with `description: None`
(incidental to those tests, which assert transition behaviour) were updated to
supply a description so they remain valid under the corrected server-side
boundary.

### WR-03: Misleading `actual` version in optimistic-lock mismatch after UPDATE

**Files modified:** `crates/trackly-infra/src/repos/requests_sqlite.rs`
**Commit:** e098afc
**Applied fix:** In `transition_in_tx`, the `affected == 0` branch (reached only
after `fetch_in_tx` already validated existence AND version match within the same
transaction) now returns `AppError::NotFound { entity: "request", id }` instead
of a fabricated `OptimisticLockMismatch` with `actual: current.version + 1`. The
only way the UPDATE touches 0 rows after the fetch succeeded is a concurrent
soft-delete, so `NotFound` is the semantically correct variant.
**Note:** This is a semantic/logic change to error-reporting behaviour. No
existing test exercised this specific branch, so the change is **flagged for
human verification** — confirm no client code relies on receiving
`OptimisticLockMismatch` (rather than `NotFound`) from a concurrently-deleted
request transition.

### WR-04: New printer-options query hardcodes magic `type_id = 2`

**Files modified:** `crates/trackly-app/src/services/request_service.rs`
**Commit:** b9e1cc7
**Applied fix:** Replaced the bare `WHERE d.type_id = 2` literal in
`printer_options` with a name-resolving subquery:
`WHERE d.type_id = (SELECT id FROM device_types WHERE name = 'Принтер')`. The
seed (`migrations/V001`) defines `(2, 'Принтер')`, so the existing integration
test still passes, but the endpoint is now resilient to a lookup-id reseed and no
longer requires lockstep edits with the test fixtures.

### WR-05: Desktop WS bridge forwards every event without `is_visible_to` filtering

**Files modified:** `crates/trackly-app/src/main.rs`
**Commit:** a4e27f6
**Applied fix:** Took the reviewer's documented-alternative path rather than
gating on a snapshotted identity. The desktop shell only ever runs under an
admin/manager-tier identity (unlocked → `trusted_admin`; locked → verified
desktop admin), for which every arm of `WsEvent::is_visible_to` already passes,
so a per-event gate is a no-op today. More importantly, the correct desktop
identity is resolved *per operation* via `resolve_tauri_identity` (depends on the
runtime `desktop_lock_enabled` setting + an async DB lookup and can change while
running); snapshotting one identity for the lifetime of the long-lived bridge
task would be stale-by-construction. Added an explicit code comment documenting
this rationale, the identity the bridge assumes, and the exact conditions under
which the bridge MUST be converted to a live per-event `is_visible_to` gate
(non-admin desktop identity, or a future `WsEvent` variant carrying
operator-restricted data).

### WR-06: `RequestPrinterOptionDto.name` mapped from a nullable column without guard

**Files modified:** `crates/trackly-app/src/services/request_service.rs`
**Commit:** b9e1cc7
**Applied fix:** Verified `devices.name` is `TEXT NOT NULL` in
`migrations/V003__devices.sql`, so the `row.get(1)` into the non-Option
`name: String` field cannot hit an `InvalidColumnType` NULL error. The null path
is unreachable; per the reviewer's guidance for the NOT-NULL case, added a brief
comment noting the invariant rather than changing the DTO type. (Committed
together with WR-04 — same query/function.)

---

_Fixed: 2026-06-22_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_

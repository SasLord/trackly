---
phase: 11-requests-employee-ux-gaps
reviewed: 2026-06-22T00:00:00Z
depth: standard
files_reviewed: 18
files_reviewed_list:
  - crates/trackly-app/src/dto/printer.rs
  - crates/trackly-app/src/dto/request.rs
  - crates/trackly-app/src/http/requests.rs
  - crates/trackly-app/src/services/request_service.rs
  - crates/trackly-app/src/specta_export.rs
  - crates/trackly-app/src/tauri_cmds/requests.rs
  - crates/trackly-app/tests/request_printer_options.rs
  - crates/trackly-app/tests/role_endpoint_matrix.rs
  - crates/trackly-app/tests/ws_broadcast_fanout.rs
  - crates/trackly-core/src/domain/requests.rs
  - crates/trackly-infra/src/repos/requests_sqlite.rs
  - ui/src/bindings-phase6.ts
  - ui/src/features/layout/EmployeeLayout.svelte
  - ui/src/features/requests/RequestDetail.svelte
  - ui/src/features/requests/RequestFormModal.svelte
  - ui/src/features/requests/api.ts
  - ui/src/lib/components/GroupedPrinterSelect.svelte
findings:
  critical: 1
  warning: 6
  info: 4
  total: 11
status: issues_found
---

# Phase 11: Code Review Report

**Reviewed:** 2026-06-22
**Depth:** standard
**Files Reviewed:** 18
**Status:** issues_found

## Summary

This gap-closure phase adds an Employee-facing `request_printer_options` endpoint, a
data-minimized `RequestPrinterOptionDto`, a split-arm `WsEvent::is_visible_to` for
`RequestStatusChanged`, and supporting UI (grouped printer select, employee toast/notification
delivery).

The headline authorization/visibility work is largely sound: the `is_visible_to` split arm is
correctly scoped (author-or-admin/manager), well unit-tested, and the new printer-options DTO
genuinely leaks only `{id, name, location}` with an integration test proving no
SNMP/IP/community keys escape. The `Action::CreateRequest` gate is applied both in the
`build_*` helper and the service layer (defense-in-depth) and is regression-guarded by the
role matrix test.

However there is one **BLOCKER**: a duplicate WebSocket broadcast on the HTTP transport that
fires every browser-originated request mutation event TWICE to all subscribers (the original
source of the "WS toast spam" symptom noted in project memory). There are also several
warnings around an unenforced `ad_register` self-creation invariant, a misleading
optimistic-lock diagnostic, the new printer-options SQL relying on a magic `type_id = 2`
literal, and an unfiltered desktop WS bridge that does not call `is_visible_to`.

## Critical Issues

### CR-01: Duplicate WebSocket broadcast on every HTTP request mutation (double-fire)

**File:** `crates/trackly-app/src/http/requests.rs:109-117, 132-139, 154-160`
**Issue:**
`RequestService::create`, `transition`, and `approve_ad_register` already broadcast their
`WsEvent` via `self.ws_tx.send(...)` (`services/request_service.rs:317`, `:456`, `:590`).
The corresponding axum handlers then call `ctx.ws_broadcast.send(...)` **again** for the same
mutation. `ctx.ws_broadcast` and `RequestService.ws_tx` are the *same* `Arc<broadcast::Sender>`
(constructed once in `context.rs:284` and cloned into the service at `:330`), so this is not a
second independent channel — it is a literal re-send of an identical event on the same channel.

Consequence: for any mutation that arrives over HTTP (i.e. every LAN-browser action — the
employee/admin web path this phase targets), each `NewRequest` / `RequestStatusChanged` is
delivered twice to every WS subscriber and to the desktop bridge. The author-employee sees two
toasts/notifications per status change; admin/manager dashboards receive duplicate events. This
matches the "WS toast spam" symptom recorded in project memory (phase9_followups). The handler
comments ("re-broadcast from HTTP transport as well for completeness") are factually wrong —
the broadcast is not transport-specific; the service already did it.

Note the Tauri command wrappers in `tauri_cmds/requests.rs` were *correctly* de-duplicated for
exactly this reason (see the module doc comment at lines 8-15 — direct `app.emit` was removed
because the service already broadcasts). The HTTP handlers were not given the same treatment.

**Fix:** Remove the redundant `ctx.ws_broadcast.send(...)` blocks from all three HTTP handlers;
the service layer is the single broadcast owner.
```rust
// handler_create — drop the whole ws_broadcast.send(...).ok() block:
pub async fn handler_create(/* ... */) -> Result<Json<RequestDto>, AppErrorResponse> {
    let identity = session_identity(&session).await.map_err(AppErrorResponse::from)?;
    let result = build_requests_create(&ctx, &identity, p.dto)
        .await
        .map_err(AppErrorResponse::from)?;
    // WS push is owned by RequestService::create — do NOT re-broadcast here.
    Ok(Json(result))
}
// Apply the same removal to handler_transition and handler_approve_ad_register.
```
A regression test belongs alongside `ws_broadcast_fanout.rs`: drive one HTTP `requests_create`
through `build_router` with a subscribed receiver and assert exactly ONE event is received.

## Warnings

### WR-01: `ad_register` requests can be self-created by any authenticated user

**File:** `crates/trackly-app/src/services/request_service.rs:268-289`
**Issue:**
`create()` passes `payload.request_type` straight into `RequestNew` with no allowlist check.
The DTO doc comment at `dto/request.rs:165` and the code comment at `request_service.rs:286`
both assert "user-facing `create()` never originates ad_register requests" — but nothing
enforces it. The DB CHECK constraint (`migrations/V006__requests.sql:12`) accepts
`'cartridge_replace' | 'free_form' | 'ad_register'`, so a hand-crafted
`requests_create` call with `requestType: "ad_register"` succeeds. The forged row then enters
the admin-only AD-register approval queue (`list` excludes ad_register for the *creator* since
they are non-admin, but it is fully visible to admins as a legitimate-looking registration).
If an admin approves it, `approve_ad_register` mutates the `users` row for
`requested_by_user_id` (the forger's own account) — `is_active = 1` and an admin-chosen role —
which is a role/state mutation path that was never meant to be reachable from the create
endpoint. This is an invariant/BFLA gap, not arbitrary injection (the CHECK blocks unknown
strings), hence Warning not Blocker.

**Fix:** Validate `request_type` against the user-creatable set in `create()` before the write:
```rust
if !matches!(payload.request_type.as_str(), "cartridge_replace" | "free_form") {
    return Err(AppError::Validation {
        field: "request_type".into(),
        message: "request_type must be cartridge_replace or free_form".into(),
    });
}
```

### WR-02: `create()` does not enforce type-specific required fields

**File:** `crates/trackly-app/src/services/request_service.rs:279-289`
**Issue:**
`RequestNew` is built unconditionally. A `cartridge_replace` request can be created with
`printer_device_id = None` (the domain comment at `domain/requests.rs:55` says it is
"Required for cartridge_replace type", but no code checks this), and a `free_form` request can
be created with no description. The frontend `validate()` in `RequestFormModal.svelte:97-114`
enforces these, but the service is the security boundary and is bypassable via direct
HTTP/Tauri calls. The result is a `cartridge_replace` row with a null printer that renders as
"Принтер: —" in `RequestDetail.svelte:378` and cannot be meaningfully completed.

**Fix:** Mirror the frontend validation server-side in `create()` (require
`printer_device_id.is_some()` for `cartridge_replace`; require non-empty `description` for
`free_form`), returning `AppError::Validation`.

### WR-03: Misleading `actual` version in optimistic-lock mismatch after UPDATE

**File:** `crates/trackly-infra/src/repos/requests_sqlite.rs:168-175`
**Issue:**
`transition_in_tx` first fetches the row (`fetch_in_tx` filters `deleted_at_utc IS NULL`) and
checks `current.version == version`. If that passes but the UPDATE affects 0 rows, the only way
that can happen inside the same transaction is that `deleted_at_utc` became non-NULL between
fetch and update (the version already matched and is in the WHERE clause). The code reports
`actual: current.version + 1`, which is a fabricated value — the row was not version-bumped, it
was soft-deleted. A client (or future debugger) reading this `OptimisticLockMismatch` will
chase a non-existent concurrent edit. Given the surrounding single-writer transaction, the
`affected == 0` branch is effectively dead for the version reason and only reachable for the
deleted-row reason, so the error variant is also semantically wrong (should be `NotFound` /
"request was deleted").

**Fix:** Distinguish the cause. Since the fetch already validated version + existence, treat
`affected == 0` here as the row having been concurrently soft-deleted:
```rust
if affected == 0 {
    return Err(AppError::NotFound { entity: "request", id: request_id });
}
```
Or, if keeping the lock variant, do not invent `current.version + 1` — re-fetch and report the
real `actual`.

### WR-04: New printer-options query hardcodes magic `type_id = 2`

**File:** `crates/trackly-app/src/services/request_service.rs:238`
**Issue:**
`WHERE d.type_id = 2` encodes "printer device type" as a bare literal with only an inline
intent (the seed/lookup tables define device types). The test seeds printers with the same
magic `2` (`tests/request_printer_options.rs:133`), so the test cannot catch a future reseed
that changes the printer type id — both sides would have to be edited in lockstep. This is the
same brittleness class the codebase elsewhere avoids by referencing lookup rows by name. A
silent reseed (or a second "printer-like" type) would make this endpoint return the wrong
device set to the create-request form, including potentially non-printer devices.

**Fix:** Resolve the printer type id by its stable name in a subquery rather than hardcoding:
```sql
WHERE d.type_id = (SELECT id FROM device_types WHERE name = 'printer')
  AND d.deleted_at_utc IS NULL
```
(adjust table/column names to the actual lookup schema), or pull the constant from a single
shared definition used by both production code and tests.

### WR-05: Desktop WS bridge forwards every event without `is_visible_to` filtering

**File:** `crates/trackly-app/src/main.rs:243` (cross-module impact of `dto/printer.rs:217`)
**Issue:**
`dto/printer.rs` documents `is_visible_to` as "the SOLE security boundary" and the browser WS
path (`http/ws.rs:85`) correctly gates each event through it. The desktop bridge added for the
gap-closure (`main.rs:238-243`) does `app.emit("trackly-event", &event)` for *every* broadcast
event with no visibility check. Today the desktop webview is admin/manager-operated so the
admin/manager arms of `is_visible_to` would pass anyway, making this benign in the current
product. But it is a latent authorization inconsistency: the doc comment in
`EmployeeLayout.svelte:26-31` and `dto/printer.rs:173` both promise server-side filtering as
the only boundary, and one of the two server paths does not honor it. If the desktop shell ever
runs under a non-admin identity (or a future event type carries data not meant for the desktop
operator), this leaks. Flagging because this phase is explicitly about WS visibility boundaries.

**Fix:** Resolve the desktop identity once in `.setup(...)` and gate the bridge emit with
`event.is_visible_to(&desktop_identity)` to match `http/ws.rs`, or document explicitly (in code)
why the desktop bridge is intentionally unfiltered and what identity it assumes.

### WR-06: `RequestPrinterOptionDto.name` mapped from a nullable column without guard

**File:** `crates/trackly-app/src/services/request_service.rs:235, 244-246`
**Issue:**
The query selects `d.name` into `RequestPrinterOptionDto.name: String` (non-Option). If any
printer device row has a NULL `name`, `row.get(1)` for a `String` target fails with
`InvalidColumnType`, which `map_rusqlite` turns into an Internal error — failing the entire
employee create-request form load rather than degrading. The frontend even anticipates a
missing name (`GroupedPrinterSelect.svelte:70` falls back to `Принтер #${p.id}`), implying the
team expects names can be absent, but the Rust DTO cannot represent that. Whether `devices.name`
is `NOT NULL` determines if this is reachable; the seed test always provides a name, so the
test does not exercise the null path.

**Fix:** Confirm `devices.name` is `NOT NULL` in the schema; if it is not, make the DTO field
`Option<String>` (and let the existing frontend fallback handle null), or `COALESCE(d.name, '')`
in SQL. If it is guaranteed `NOT NULL`, add a brief comment noting the invariant.

## Info

### IN-01: `PrinterDto::from` sets `community_configured = true` unconditionally with a self-contradicting comment

**File:** `crates/trackly-app/src/dto/printer.rs:62-65`
**Issue:** The comment says "the service layer sets it to true when community != default" but
the `From` impl hardcodes `true` and the comment's next line admits "Always true here since we
never store empty community." The middle sentence is stale/misleading and implies a conditional
that does not exist. `PrinterDto` is not the focus of this phase but the field touches the
data-minimization story.
**Fix:** Delete the inaccurate middle sentence; keep only the accurate "always true — community
is never empty" rationale.

### IN-02: `handler_list_categories` binds `_identity` only for the auth side effect

**File:** `crates/trackly-app/src/http/requests.rs:182-184`
**Issue:** `session_identity` is called to enforce authentication (good — categories should not
be public) but the result is discarded with `_identity`. This is intentional and correct, but a
one-line comment ("auth required; categories are role-agnostic") would prevent a future reader
from "cleaning up" the seemingly-unused call and accidentally opening the endpoint.
**Fix:** Add a clarifying comment.

### IN-03: `category_id` is not validated against `request_categories`

**File:** `crates/trackly-app/src/services/request_service.rs:284`
**Issue:** `category_id` is passed through to the insert. `migrations/V006__requests.sql` would
need a FK to `request_categories` for the DB to reject a bogus id; if there is no such FK, a
`free_form` request can carry a dangling `category_id` that joins to NULL `category_name`. Low
impact (display-only), and the UI only sends ids it received from `listCategories`, but a direct
caller could send anything.
**Fix:** Rely on a FK constraint (verify it exists) or validate existence in `create()`.

### IN-04: Duplicated error-message extraction boilerplate across RequestDetail handlers

**File:** `ui/src/features/requests/RequestDetail.svelte:174-180, 197-202, 231-236, 262-267, 315-321`
**Issue:** The identical `e && typeof e === 'object' && 'message' in e ? ... : fallback`
pattern is copy-pasted into five catch blocks. Not a bug, but it is the kind of duplication that
drifts (one fallback string already differs). Extracting a `errMessage(e, fallback)` helper
would reduce surface area.
**Fix:** Introduce a small shared helper (e.g. in `$lib/api/client` or a local util) and reuse.

---

_Reviewed: 2026-06-22_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_

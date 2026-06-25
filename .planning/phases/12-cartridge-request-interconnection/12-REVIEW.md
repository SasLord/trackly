---
phase: 12-cartridge-request-interconnection
reviewed: 2026-06-25T00:00:00Z
depth: standard
files_reviewed: 9
files_reviewed_list:
  - crates/trackly-core/src/ports/printers.rs
  - crates/trackly-infra/src/repos/printers_sqlite.rs
  - crates/trackly-app/src/services/printer_service.rs
  - crates/trackly-app/src/tauri_cmds/printers.rs
  - crates/trackly-app/src/http/printers.rs
  - crates/trackly-app/src/specta_export.rs
  - crates/trackly-app/tests/role_endpoint_matrix.rs
  - ui/src/features/cartridges/OperationModal.svelte
  - ui/src/features/printers/api.ts
findings:
  critical: 0
  warning: 1
  info: 2
  total: 3
status: issues_found
---

# Phase 12: Code Review Report (Round 5 gap-closure, plan 12-21)

**Reviewed:** 2026-06-25
**Depth:** standard
**Files Reviewed:** 9
**Status:** issues_found

## Summary

Scope: changes since `a5ce476` adding the dual-transport read command
`printers_get_by_device_id` and rewiring `OperationModal.svelte`'s printer
lookup `$effect` from `printers.get()` to `getByDeviceId()`, plus DEC-A
(hint branching) and DEC-B (location prefill).

The change is well-constructed and the core bug fix is correct. Verified
against the review checklist:

- **SQL correctness/parameterization** — `WHERE p.device_id = ?1` is fully
  parameterized via `params![device_id]`; no injection surface. `device_id`
  carries `NOT NULL UNIQUE` (migrations V020/V030), so `query_row` returning
  the first row is safe — at most one match. PASS.
- **Shared business logic across transports** — Both the Tauri command
  (`printers_get_by_device_id`) and the axum handler (`handler_get_by_device_id`)
  funnel through the same `build_printers_get_by_device_id`, which delegates to
  `PrinterService::get_by_device_id`. Satisfies the CLAUDE.md dual-transport
  rule. PASS.
- **Authorization parity** — `build_printers_get_by_device_id` calls
  `authorize(caller, &Action::ReadData)?` identically to `build_printers_get`.
  RBAC matrix Case 40 (Employee → 403) added, mirroring Case 33. PASS.
- **Reactive correctness of the rewired `$effect`** — No infinite loop: the
  `location.trim()` read that the prefill depends on happens inside the async
  `.then()` callback, which Svelte 5 does NOT track as a reactive dependency
  (dependency capture is synchronous-only). Verified `isSelectorVisible`
  (lines 184-186) is byte-for-byte identical to the actual PrinterSelect
  render guard (line 547), so the DEC-A hint branch matches selector
  visibility exactly. PASS.
- **`printers_get` record-id semantics untouched** — The existing `get()` /
  `handler_get` / `build_printers_get` path is unchanged; the new command is
  purely additive. PASS.
- **device_id contract** — Confirmed both `effectivePrinterId` sources carry a
  device_id: `PrinterSelect` emits `value={String(p.deviceId)}`
  (PrinterSelect.svelte:85,92) and `preFillPrinterId={request.printerDeviceId}`
  (RequestDetail.svelte:710). The switch to `getByDeviceId` therefore fixes a
  real bug (the old `printers.get(deviceId)` resolved by `printers.id` and
  returned NotFound, leaving `printerContext` null and the «Предыдущий
  картридж» block unreachable). PASS.

## Warnings

### WR-01: DEC-B never re-fills «Расположение» when the operator switches printers

**File:** `ui/src/features/cartridges/OperationModal.svelte:241-243`
**Issue:** The DEC-B prefill guard is `preFillPrinterId === undefined &&
printer.deviceLocation && !location.trim()`. In the cartridge-centric flow
the operator can change the PrinterSelect freely; each change re-runs the
lookup `$effect`. Once the first selected printer populates `location`, the
`!location.trim()` guard is permanently false, so selecting a different
printer afterward will NOT update «Расположение» to the new printer's
location — it silently retains the first printer's value. Because the field
was auto-filled (not typed), the operator may not notice it now points at the
wrong room/workstation, and the install act records a stale location. This is
a behavioral gray area: the stated intent is "never clobber manual operator
input," but the guard cannot distinguish auto-filled text from manually-typed
text, so it over-protects auto-filled values too.
**Fix:** Track whether the current `location` value was auto-populated, and
allow re-fill on printer change while it remains auto-sourced and untouched.
For example:
```svelte
let locationAutofilled = $state(false);
// in onChange for LocationAutocomplete:
onChange={(v) => { location = v; locationAutofilled = false; }}
// in the lookup .then():
if (
  preFillPrinterId === undefined &&
  printer.deviceLocation &&
  (!location.trim() || locationAutofilled)
) {
  location = printer.deviceLocation;
  locationAutofilled = true;
}
```
Reset `locationAutofilled = false` in the open/reset effect (line 135).
Alternatively, if "first pick wins, switching never re-fills" is the accepted
product decision, document it explicitly in the DEC-B comment so the next
reader does not read it as a bug.

## Info

### IN-01: NotFound carries device_id under an `id`-named field

**File:** `crates/trackly-infra/src/repos/printers_sqlite.rs:315-318`
**Issue:** On a miss, `get_by_device_id` returns
`AppError::NotFound { entity: "printer", id: device_id }`. The `id` field of
the error now holds a device_id, not a `printers.id`. Any log/UI that renders
"printer #{id} not found" will show the device_id, which is a different key
space and could mislead during debugging (e.g., "printer 7 not found" when
printers.id 7 actually exists but device_id 7 does not).
**Fix:** Either keep as-is (acceptable — the value is still a meaningful
identifier the caller passed in) or distinguish the message, e.g. set
`entity: "printer (by device_id)"` so logs disambiguate the key space.

### IN-02: No positive (Admin/Manager → 200) assertion for the new command

**File:** `crates/trackly-app/tests/role_endpoint_matrix.rs:1402-1421`
**Issue:** Case 40 only asserts the negative path (Employee → 403). There is
no test that an authorized role actually reaches the DB read and that
`get_by_device_id` resolves a real row (the whole point of the fix). This
matches the existing convention for `printers_get_compatible_models`
(Case 33 is also negative-only), so it is not a regression — but the bug this
round fixes (wrong resolve key returning NotFound) is exactly the kind of
defect a positive round-trip assertion would have caught earlier.
**Fix:** Optionally add a positive case: seed a device + printer, call
`/api/v1/printers_get_by_device_id` with `deviceId` = the seeded device id as
Manager, assert 200 and that the returned `id` is the `printers.id` (proving
device_id → printers.id resolution works end to end).

---

_Reviewed: 2026-06-25_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_

---
phase: 11-requests-employee-ux-gaps
plan: 02
subsystem: api
tags: [rust, axum, tauri, svelte, rbac, bola]

# Dependency graph
requires:
  - phase: 11-requests-employee-ux-gaps
    provides: "11-01: category_name display pipeline, bindings-phase6.ts hand-maintained convention"
  - phase: 10-employee-role-restriction
    provides: "Action::ReadData/ReadPrinters BFLA closure for Employee — the regression this plan fixes a side-effect of"
provides:
  - "RequestPrinterOptionDto {id, name, location} minimal printer DTO"
  - "RequestService::printer_options — CreateRequest-gated printer list for the create-request form"
  - "request_printer_options endpoint (Tauri command + HTTP route, both transports)"
  - "GroupedPrinterSelect.svelte — location-grouped printer dropdown component"
  - "RequestFormModal.svelte printer field now reachable by Employee again"
affects: [requests, employee-ux, rbac]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Narrow read endpoints for Employee-reachable forms should gate on the form's own write Action (CreateRequest) rather than the resource's ReadData/ReadPrinters action, with a DTO trimmed to exactly what the form needs (BOLA/BOPLA closure pattern)."

key-files:
  created:
    - crates/trackly-app/tests/request_printer_options.rs
    - ui/src/lib/components/GroupedPrinterSelect.svelte
  modified:
    - crates/trackly-app/src/dto/request.rs
    - crates/trackly-app/src/services/request_service.rs
    - crates/trackly-app/src/tauri_cmds/requests.rs
    - crates/trackly-app/src/http/requests.rs
    - crates/trackly-app/src/specta_export.rs
    - crates/trackly-app/tests/role_endpoint_matrix.rs
    - ui/src/bindings-phase6.ts
    - ui/src/features/requests/api.ts
    - ui/src/features/requests/RequestFormModal.svelte

key-decisions:
  - "request_printer_options gates on Action::CreateRequest (every role has it), deliberately NOT ReadData/ReadPrinters which Phase 10 closed for Employee — avoids regressing the Phase 10 BFLA fix while still unblocking the form."
  - "DTO is strictly {id, name, location} — no SNMP/community/IP/serial fields cross the wire, closing the BOLA/BOPLA gap (API1/API3:2023) that a naive devices.list reuse would have reopened."
  - "GroupedPrinterSelect groups client-side by location ?? 'Без расположения'; server is the sole source of sort order (ORDER BY l.name IS NULL, l.name, d.name) — component does not re-sort."

requirements-completed: [D-PRN-01]

# Metrics
duration: ~55min
completed: 2026-06-21
---

# Phase 11 Plan 02: Employee printer dropdown gap-closure Summary

**New `Action::CreateRequest`-gated `request_printer_options` endpoint (minimal `{id,name,location}` DTO) plus a location-grouped `GroupedPrinterSelect.svelte` dropdown, replacing the `devices.list({type_id:2})` call that Phase 10's `ReadData`/`ReadPrinters` closure had silently emptied for Employee.**

## Performance

- **Duration:** ~55 min
- **Completed:** 2026-06-21
- **Tasks:** 2
- **Files modified:** 11 (2 created, 9 modified)

## Accomplishments
- Closed the Phase 10 regression that left the cartridge-replace request form's printer dropdown empty for the Employee role.
- New endpoint returns strictly `{id, name, location}` — verified by an integration test that asserts the serialized JSON contains no other keys (no SNMP/community/IP/serial leakage).
- `GroupedPrinterSelect.svelte` groups printers by location with gray `<optgroup>` headers, matching the existing `Select.svelte` visual/prop contract.
- `role_endpoint_matrix.rs` extended with a regression guard (Employee 200 / anonymous 401) so a future refactor cannot silently fold this endpoint back into the closed `ReadData` gate.

## Task Commits

Each task was committed atomically:

1. **Task 1: Backend — RequestPrinterOptionDto + RequestService::printer_options + both transports + specta** - `c06ccaa` (feat)
2. **Task 2: Frontend — GroupedPrinterSelect + switch RequestFormModal to the new endpoint** - `67fb8b8` (feat)
3. **Fixup: reword stale comment to avoid acceptance-grep false match** - `c171fa3` (fix)

_No TDD RED/GREEN split — `tdd="true"` task wrote tests and implementation together since this was net-new code with no prior failing-test baseline to establish; all behavior assertions from the plan's `<behavior>` block are covered in `request_printer_options.rs` and `role_endpoint_matrix.rs`, all passing on first run._

**Plan metadata:** pending (this commit)

## Files Created/Modified
- `crates/trackly-app/src/dto/request.rs` - new `RequestPrinterOptionDto {id, name, location}`, camelCase, `#[specta(type=i32)]` on `id`
- `crates/trackly-app/src/services/request_service.rs` - new `printer_options` method: `authorize(CreateRequest)` + parameterized `SELECT ... type_id=2 LEFT JOIN locations ORDER BY l.name IS NULL, l.name, d.name`
- `crates/trackly-app/src/tauri_cmds/requests.rs` - `build_request_printer_options` helper + `request_printer_options` Tauri command
- `crates/trackly-app/src/http/requests.rs` - `handler_request_printer_options` (no `ws_broadcast` — read-only) + `/api/v1/request_printer_options` route
- `crates/trackly-app/src/specta_export.rs` - registered `request_printer_options` in `collect_commands!`
- `crates/trackly-app/tests/request_printer_options.rs` - new: employee 200 + minimal-DTO key assertion + sort-order assertion + no-session 401 + empty-list case
- `crates/trackly-app/tests/role_endpoint_matrix.rs` - extended with Case 29 (Employee 200) / Case 30 (anonymous 401) regression guard
- `ui/src/bindings-phase6.ts` - hand-maintained `RequestPrinterOptionDto` type added
- `ui/src/features/requests/api.ts` - `requests.printerOptions()` mirroring `listCategories`
- `ui/src/lib/components/GroupedPrinterSelect.svelte` - new component: groups options by `location ?? 'Без расположения'`, gray `<optgroup>` headers, "Принтеры не найдены" empty state
- `ui/src/features/requests/RequestFormModal.svelte` - `loadPrinters` now calls `requests.printerOptions()`; printer field uses `<GroupedPrinterSelect>`; removed the `devices`/`DeviceDto` import (no longer reachable for this form)

## Decisions Made
- Gated the new endpoint on `Action::CreateRequest` instead of `ReadData`/`ReadPrinters` — the only choice consistent with not regressing Phase 10's BFLA fix while still letting Employee populate the dropdown.
- Kept the DTO minimal by design (`{id, name, location}` only) rather than reusing/trimming `DeviceDto` client-side, since server-side minimality is the actual BOLA/BOPLA mitigation (a client-side trim would still leak the full payload over the wire).
- Seeded test fixtures via raw SQL through `ctx.writer.execute` (the established `requests_ad_register_http.rs` pattern) rather than via a domain service, since no existing test in this codebase seeds devices through `ctx.devices`.
- `GroupedPrinterSelect` does not re-sort; it trusts the server's `ORDER BY` and only buckets by `location` for rendering — avoids duplicating sort logic across the stack.

## Deviations from Plan

None — plan executed as written. One self-correction during execution: the first draft of explanatory comments in `RequestFormModal.svelte` referenced the literal string `devices.list({type_id:2})` (the removed call), which would have falsely matched the plan's acceptance grep for "must NOT contain `devices.list({type_id`" even though the actual import/call was already removed. Reworded the comments before committing — not a behavior change, included here for traceability since it triggered a separate small commit.

## Issues Encountered
- `<optgroup label>` (Svelte attribute shorthand) was initially misread by `svelte-check` as the boolean `label` HTML attribute rather than a binding to the `label` loop variable, producing a type error (`'boolean' is not assignable to type 'string'`). Fixed by writing it explicitly as `label={label}`. Caught immediately by `svelte-check --threshold error` before any commit.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- D-PRN-01 closed. The Employee role can again submit cartridge-replacement requests end-to-end (pending the still-open Phase 9 AD-login milestone item for full Employee auth, tracked separately).
- This was the last requirement-bearing plan needed before Phase 11's remaining plan (03) per ROADMAP; no blockers identified for that plan's wave.
- `role_endpoint_matrix.rs` now has explicit regression coverage for this endpoint — future read-domain gating changes (if any) must keep Cases 29/30 green.

---
*Phase: 11-requests-employee-ux-gaps*
*Completed: 2026-06-21*

## Self-Check: PASSED

All 12 created/modified files confirmed present on disk; all 3 commit hashes (`c06ccaa`, `67fb8b8`, `c171fa3`) confirmed present in `git log --oneline --all`.

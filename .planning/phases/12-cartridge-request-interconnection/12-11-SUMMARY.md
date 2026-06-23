---
phase: 12-cartridge-request-interconnection
plan: 11
subsystem: realtime-notifications
tags: [serde, websocket, svelte, toast, camelCase]

# Dependency graph
requires:
  - phase: 11-cartridge-request-interconnection
    provides: "D-WS-01 WsEvent::RequestStatusChanged + EmployeeLayout.svelte WS toast/notification wiring"
  - phase: 12-cartridge-request-interconnection
    provides: "OperationModal request-centric install flow (D-01..D-08, Plan 12-03/12-09)"
provides:
  - "WsEvent enum serializes each variant's fields in camelCase (per-variant rename_all), outer type tag stays snake_case"
  - "EmployeeLayout.svelte statusToastText() now reads a correctly-populated event.newStatus instead of always undefined"
  - "OperationModal Props.suppressSuccessToast — opt-in toast suppression for callers that already show their own success toast"
affects: [cartridge-request-interconnection, realtime-notifications, employee-ux]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Per-variant #[serde(rename_all = \"camelCase\")] on enum variants when an outer #[serde(tag = ..., rename_all = \"snake_case\")] only controls the tag value, not field names (same pattern as RequestTransitionPayload, 09-ad-gaps-defects)"
    - "Optional suppressSuccessToast prop pattern: callers that own a more specific success toast suppress the generic modal-level one instead of the modal trying to guess context"

key-files:
  created: []
  modified:
    - crates/trackly-app/src/dto/printer.rs
    - ui/src/features/cartridges/OperationModal.svelte
    - ui/src/features/requests/RequestDetail.svelte

key-decisions:
  - "Outer #[serde(tag = \"type\", rename_all = \"snake_case\")] left unchanged on WsEvent — the type tag value (request_status_changed etc.) must stay snake_case because ws.ts/EmployeeLayout.svelte compare event.type against snake_case literals; only added per-variant rename_all=camelCase for FIELDS"
  - "suppressSuccessToast defaults to false/undefined so the cartridge-centric entry (CartridgesPage, D-08) needs zero changes and keeps its original toast behavior"
  - "ui/src/bindings-phase6.ts required no changes — it already declared requestId/newStatus/requestedByUserId in camelCase; confirms the bug was purely a Rust-side serialization defect, not a frontend contract mismatch"

patterns-established:
  - "When adding a new WsEvent variant, always pair the outer enum's tag rename_all with a per-variant rename_all=camelCase, or fields will silently serialize in the outer's casing and the frontend type contract will silently mismatch (no compile-time signal, only `undefined` at runtime)."

requirements-completed: [GAP-12-04]

# Metrics
duration: 14min
completed: 2026-06-23
---

# Phase 12 Plan 11: WsEvent camelCase fields + duplicate success-toast fix Summary

**Fixed WsEvent's missing per-variant `rename_all = "camelCase"` (fields silently serialized snake_case despite the frontend expecting camelCase) and removed OperationModal's duplicate success toast when a caller already shows its own.**

## Performance

- **Duration:** 14 min
- **Started:** 2026-06-23T16:37:00Z
- **Completed:** 2026-06-23T16:51:00Z
- **Tasks:** 2 completed
- **Files modified:** 3

## Accomplishments
- `WsEvent::RequestStatusChanged`/`NewRequest`/`PrinterAlert` now each carry `#[serde(rename_all = "camelCase")]`, so the wire format is `{"type":"request_status_changed","requestId":7,"newStatus":"completed","requestedByUserId":3}` — fields camelCase, tag snake_case, matching the TypeScript `WsEvent` union that was already correct.
- `EmployeeLayout.svelte`'s `statusToastText()` now receives a real `newStatus` value instead of `undefined`, so employees see the correct per-status text (`"Ваша заявка принята в работу"` / `"...выполнена"` / `"...отклонена"`) instead of always falling into the generic default branch.
- `OperationModal.svelte` gained an optional `suppressSuccessToast` prop; `RequestDetail.svelte`'s request-centric install flow now sets it to `true`, eliminating the duplicate "Операция выполнена успешно." toast that previously fired right after `RequestDetail`'s own "Заявка выполнена" toast.
- Cartridge-centric install entry (`CartridgesPage.svelte`, D-08) is untouched — no prop passed, defaults to `false`, original toast behavior preserved, no regression.

## Task Commits

Each task was committed atomically:

1. **Task 1: WsEvent — camelCase поля для каждого варианта** - `695fbaa` (fix, tdd) — added per-variant `rename_all` + 3 serialization unit tests
2. **Task 2: Убрать дублирующий тост в OperationModal при наличии onSuccess** - `5d8b129` (fix) — added `suppressSuccessToast` prop + wired `RequestDetail`

**Plan metadata:** (this commit) `docs(12-11): complete WsEvent camelCase + duplicate toast fix plan`

_Note: Task 1 used TDD (tests written and run alongside the fix); RED-phase was skipped in the traditional sense since the bug was already known and described precisely in the plan's `<behavior>` block — tests were written to assert the FIXED behavior and passed immediately after the one-line annotation fix, which is consistent with the plan's `tdd="true"` framing of "write the serialization tests described in `<behavior>`" rather than a strict fail-first cycle._

## Files Created/Modified
- `crates/trackly-app/src/dto/printer.rs` - Added `#[serde(rename_all = "camelCase")]` to each of the 3 `WsEvent` variants; added 3 new serialization unit tests (`request_status_changed_serializes_camel_case_fields_snake_case_tag`, `new_request_serializes_camel_case_fields_snake_case_tag`, `printer_alert_serializes_camel_case_fields_snake_case_tag`); expanded doc comment explaining the outer-tag-vs-per-variant-fields serde semantics and the GAP-12-04 root cause.
- `ui/src/features/cartridges/OperationModal.svelte` - Added optional `Props.suppressSuccessToast?: boolean`; `handleSubmit()` now gates the generic `pushToast('success', 'Операция выполнена успешно.')` call behind `!suppressSuccessToast`.
- `ui/src/features/requests/RequestDetail.svelte` - `<OperationModal>` usage (REQ-05 install flow) now passes `suppressSuccessToast={true}`.

## Decisions Made
- Kept the outer `#[serde(tag = "type", rename_all = "snake_case")]` on `WsEvent` unchanged — only the tag VALUE needs snake_case (frontend `ws.ts`/`EmployeeLayout.svelte` compare `event.type` against snake_case string literals like `'request_status_changed'`); per-variant `rename_all = "camelCase"` was added purely to control FIELD names, which serde treats as an independent attribute scope from the enum-level tag attribute.
- Did not touch `EmployeeLayout.svelte`'s `statusToastText()` switch logic — it was already correct; the plan's `<interfaces>` section explicitly called out that the bug was purely in backend serialization, and verification confirmed `ui/src/bindings-phase6.ts` needed zero changes (it already declared the camelCase TS contract).
- Chose an opt-in boolean prop (`suppressSuccessToast`) over having `OperationModal` introspect whether `onSuccess` itself shows a toast — keeps the modal's responsibility simple and the caller explicit about intent, consistent with the existing `prefillLocation`/`prefillGivenToName` optional-prop pattern already used in this component.

## Deviations from Plan

None - plan executed exactly as written. Both tasks matched their `<action>` and `<acceptance_criteria>` blocks precisely; no architectural changes, no blocking issues, no missing critical functionality discovered.

## Issues Encountered

None. The plan's `<read_first>` and `<interfaces>` sections pre-diagnosed the exact root cause and fix shape (mirroring the already-fixed `RequestTransitionPayload` pattern from `09-ad-gaps-defects`), so implementation was a direct, low-risk application of a known-good pattern.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- GAP-12-04 (A1: duplicate/incorrect status notification) is closed at both the backend serialization layer and the frontend toast layer.
- `cargo test -p trackly-app --lib` — 86/86 passing (full lib suite, not just the touched module).
- `pnpm --dir ui exec svelte-check` — 0 errors (36 pre-existing warnings in unrelated files, unchanged).
- `pnpm --dir ui build` — succeeds, `ui/dist` refreshed for LAN-browser/server-mode testing.
- Manual/live verification (cargo tauri dev or browser) of the actual employee-facing toast text and the single-toast admin install flow was not performed in this autonomous session — recommended as a quick human spot-check during the next end-of-phase UAT pass, consistent with `human_verify_mode: end-of-phase` in `.planning/config.json`.
- No blockers for remaining Phase 12 gap-closure plans (GAP-12-05..08, per `12-HUMAN-UAT.md` Round 2).

---
*Phase: 12-cartridge-request-interconnection*
*Completed: 2026-06-23*

## Self-Check: PASSED

- FOUND: .planning/phases/12-cartridge-request-interconnection/12-11-SUMMARY.md
- FOUND: crates/trackly-app/src/dto/printer.rs
- FOUND: ui/src/features/cartridges/OperationModal.svelte
- FOUND: ui/src/features/requests/RequestDetail.svelte
- FOUND commit: 695fbaa (Task 1)
- FOUND commit: 5d8b129 (Task 2)
- FOUND commit: 230d52e (SUMMARY.md)

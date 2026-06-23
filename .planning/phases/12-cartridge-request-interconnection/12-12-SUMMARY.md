---
phase: 12-cartridge-request-interconnection
plan: 12
subsystem: ui
tags: [svelte, svelte5-runes, cartridges, request-centric-install]

# Dependency graph
requires:
  - phase: 12-cartridge-request-interconnection
    provides: "Plan 12-09's previous-cartridge block ($state previousCartridge/previousCartridgeStateId/previousCartridgeLocation) and Plan 12-05's PrinterDto.deviceName/ipAddress fields"
provides:
  - "printerContextHint shows deviceName+ipAddress (not raw #id) for the install-from-request flow"
  - "Informational hint explaining inverted Кто/Кому roles in the previous-cartridge auto-return block"
affects: [12-uat, 13-printer-cartridge-compatibility]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Reuse a single $effect's existing API call result by also storing it into a new $state (printerContext), avoiding a duplicate network round-trip"

key-files:
  created: []
  modified:
    - ui/src/features/cartridges/OperationModal.svelte

key-decisions:
  - "printerContext: $state<PrinterDto | null> populated inside the existing printers.get(preFillPrinterId) $effect (no second API call) — printerContextHint derives deviceName+ipAddress from it, falling back to #id only while the lookup is in flight or deviceName is null"
  - "printerContextHint render block moved before the cartridge-select picker in the template — operator sees the target printer first"
  - "Кто/Кому inversion hint is pure informational text inside previous-cartridge-block — no new form fields, no CartridgeTransitionPayload changes"

patterns-established: []

requirements-completed: [GAP-12-05]

# Metrics
duration: 12min
completed: 2026-06-24
---

# Phase 12 Plan 12: Install-dialog printer context + Кто/Кому inversion hint Summary

**OperationModal install flow now shows the target printer's name+IP first (not an abstract #id) and explains the inverted Кто/Кому roles for the auto-returned previous cartridge.**

## Performance

- **Duration:** 12 min
- **Started:** 2026-06-24T00:49:00Z
- **Completed:** 2026-06-24T00:61:00Z
- **Tasks:** 2 completed
- **Files modified:** 1

## Accomplishments
- Operator installing a cartridge from a request now sees "Устанавливается в принтер: {имя} ({IP})" as the FIRST line of the form, instead of a meaningless `#id`, and instead of it appearing after the cartridge picker.
- USB-only printers (no IP) render without empty parentheses; missing `deviceName` falls back gracefully to the old `#id` form.
- The previous-cartridge block (Plan 12-09) now explains that the Кто выдал/Кому выдал fields apply to the NEW cartridge, and that roles invert for the old cartridge being auto-returned to stock — closing the UAT confusion (A2 / GAP-12-05) without adding any new form fields.

## Task Commits

Each task was committed atomically:

1. **Task 1: Printer name+IP hint, rendered first** - `97d99f4` (feat)
2. **Task 2: Кто/Кому inversion hint for previous cartridge** - `6288a59` (feat)

**Plan metadata:** (this commit)

## Files Created/Modified
- `ui/src/features/cartridges/OperationModal.svelte` - Added `printerContext: $state<PrinterDto | null>`, reworked `printerContextHint` derived to prefer `deviceName`+`ipAddress`, reordered template so the hint renders before the cartridge-select, and added an informational paragraph inside `previous-cartridge-block` explaining the inverted Кто/Кому semantics.

## Decisions Made
- Reused the existing `printers.get(preFillPrinterId)` `$effect` call to populate the new `printerContext` state rather than issuing a second API request — keeps the single-call invariant documented in the plan's `key_links`.
- Reset `printerContext = null` both in the main open-reset `$effect` and in the lookup `$effect`'s early-return branch, for defense-in-depth against stale printer context leaking across modal reopens with different `preFillPrinterId` values.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- GAP-12-05 (A2) closed. Remaining Round 2 UAT gaps (GAP-12-04, GAP-12-06..08) are tracked in separate plans per `12-HUMAN-UAT.md`.
- No blockers for subsequent gap-closure plans in this phase.

---
*Phase: 12-cartridge-request-interconnection*
*Completed: 2026-06-24*

## Self-Check: PASSED

- FOUND: ui/src/features/cartridges/OperationModal.svelte
- FOUND: .planning/phases/12-cartridge-request-interconnection/12-12-SUMMARY.md
- FOUND commit: 97d99f4
- FOUND commit: 6288a59

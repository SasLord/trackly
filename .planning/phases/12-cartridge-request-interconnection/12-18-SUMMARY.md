---
phase: 12-cartridge-request-interconnection
plan: 18
subsystem: ui
tags: [svelte5, runes, cartridges, operation-modal, gap-closure]

# Dependency graph
requires:
  - phase: 12-cartridge-request-interconnection
    provides: "OperationModal printerContext/previousCartridge lookup (Plan 12-09/12-12), printerContextHint derived (Plan 12-12)"
provides:
  - "printerContext/previousCartridge lookup runs in BOTH install entry points (request-centric AND cartridge-centric)"
affects: [cartridge-lifecycle-ui, request-detail-ui]

# Tech tracking
tech-stack:
  added: []
  patterns: ["Single $effect gate condition broadened from cartridge===null && preFillPrinterId!==undefined to just preFillPrinterId!==undefined — sibling effects (compatibleModels, cartridgeOptions) intentionally kept narrower"]

key-files:
  created: []
  modified: ["ui/src/features/cartridges/OperationModal.svelte"]

key-decisions:
  - "Removed `cartridge === null` from the printerContext/previousCartridge lookup effect gate ONLY — left it on the compatibleModels and cartridgeOptions effects, since those (cartridge selector, compatibility filter) are still request-centric-only features (D-08 regression guard: cartridge-centric entry without preFillPrinterId still makes zero extra API calls)"
  - "Updated stale doc comment above the effect that claimed 'no lookup for the old cartridge-centric entry' — that claim is precisely what GAP-12-11 fixes, so left it accurate going forward"

patterns-established: []

requirements-completed: [CART-07, REQ-05, D-05, D-08]

# Metrics
duration: 6min
completed: 2026-06-24
---

# Phase 12 Plan 18: Cartridge-centric install printer hint + previous-cartridge block Summary

**Broadened the printerContext/previousCartridge `$effect` gate in `OperationModal.svelte` from `cartridge===null && preFillPrinterId!==undefined` to just `preFillPrinterId!==undefined`, so installing from a cartridge's own card now shows the printer's name+IP and the «Предыдущий картридж» block, matching the request-centric flow.**

## Performance

- **Duration:** 6 min
- **Started:** 2026-06-24T15:59:00Z
- **Completed:** 2026-06-24T16:05:00Z
- **Tasks:** 1 completed
- **Files modified:** 1

## Accomplishments
- Cartridge-centric install (menu → «Установить в принтер», `cartridge !== null`) now triggers the same `printers.get(preFillPrinterId)` + `cartridges.get(currentCartridgeId)` lookup as the request-centric flow, so `printerContextHint` renders `deviceName (ipAddress)` instead of the `#id` fallback.
- The «Предыдущий картридж» block (charge-state Select + LocationAutocomplete, default Пустой) now renders identically in both entry points whenever the target printer already has a cartridge «В работе».
- Confirmed via source assertions that the `compatibleModels` and `cartridgeOptions` effects retained their `cartridge === null` gate — selector and compatibility-filter UI remain request-centric only, preserving the D-08 no-extra-calls guarantee for the printer-less cartridge-centric flow.

## Task Commits

Each task was committed atomically:

1. **Task 1: Запускать printerContext/previousCartridge лукап в обоих входах установки** - `a6a284c` (fix)

**Plan metadata:** (this commit)

## Files Created/Modified
- `ui/src/features/cartridges/OperationModal.svelte` - Broadened the printer/previous-cartridge lookup `$effect` gate (line ~171) to drop `cartridge === null`; updated the stale doc comment above it; left `compatibleModels` (line 212) and `cartridgeOptions` (line 235) effects untouched with their `cartridge === null` gate intact.

## Decisions Made
- Single-condition fix scoped to exactly one `$effect` — no changes to `buildPayload`, `handleSubmit`, `validate`, backend, or bindings, per plan's explicit `НЕ менять` instruction.
- Updated an adjacent doc comment that had become factually wrong after the fix (it claimed the cartridge-centric entry never triggers the lookup) — Rule 1 (bug: stale/misleading comment) auto-fix, not a deviation requiring separate documentation since it's a same-line doc correction directly tied to the task's own change.

## Deviations from Plan

None — plan executed exactly as written. The one comment-text update was an in-scope correction to documentation directly describing the line being changed, not a separate fix.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- GAP-12-11 closed. `pnpm --dir ui exec svelte-check` → 0 errors (36 pre-existing warnings in unrelated files). `pnpm --dir ui build` succeeds.
- Source-assertions from the plan's acceptance criteria confirmed:
  - Lookup effect gate (`sed -n '170,176p'`) shows `op === 'install' && preFillPrinterId !== undefined` with no `cartridge === null`.
  - `grep -c "cartridge === null"` in the file returns 4 total occurrences (1 in a doc comment line 199, 1 in the compatibleModels effect gate line 212, 1 in the cartridgeOptions effect gate line 235, 1 in the template conditional line 460) — both sibling effects retained their gate, satisfying the ≥2 acceptance threshold.
- No remaining gap items from this plan; ready for human browser verification (open «Установить в принтер» from a cartridge card where the target printer already holds an in-use cartridge) when convenient, but no code-level follow-up required.

---
*Phase: 12-cartridge-request-interconnection*
*Completed: 2026-06-24*

## Self-Check: PASSED

- FOUND: ui/src/features/cartridges/OperationModal.svelte
- FOUND: .planning/phases/12-cartridge-request-interconnection/12-18-SUMMARY.md
- FOUND: a6a284c (task commit)

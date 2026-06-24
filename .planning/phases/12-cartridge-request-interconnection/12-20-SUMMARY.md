---
phase: 12-cartridge-request-interconnection
plan: 20
subsystem: ui
tags: [svelte5, runes, optgroup, select, cartridges, printers]

# Dependency graph
requires:
  - phase: 12-cartridge-request-interconnection
    provides: "printer_cartridge_models junction (D-11/D-12, plans 12-05/12-07) and current_printer_device_id auto-return/inverted-actor backend (plans 12-06/12-19) — this plan is a pure frontend consumer"
provides:
  - "PrinterSelect.svelte — compatibility-priority printer selector, reusable wherever a reverse cartridge-model→printer lookup is needed"
  - "OperationModal effectivePrinterId derived — single source of truth that unifies request-centric (preFillPrinterId prop) and cartridge-centric (selectedPrinterId local choice) printer context"
  - "Cartridge-centric install entry (menu → «Установить в принтер») can now optionally select a printer, see compatibility-prioritized options, and trigger the auto-return «Предыдущий картридж» block — closing GAP-12-11/GAP-12-12 п.1/3"
affects: [cartridges, printers, requests]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "effectivePrinterId = preFillPrinterId ?? selectedPrinterId — prop-priority derived pattern letting two independent UI entry points feed one downstream lookup/payload code path"
    - "Reverse compatibility lookup (printer-by-cartridge-model) mirrors the existing forward lookup (cartridge-by-printer) via the same junction table read endpoint, just inverted client-side"

key-files:
  created:
    - ui/src/lib/components/PrinterSelect.svelte
  modified:
    - ui/src/features/cartridges/OperationModal.svelte

key-decisions:
  - "D-20: printer selection in cartridge-centric install is optional — undefined selectedPrinterId == legacy D-08 behavior, no regression"
  - "D-21: PrinterSelect groups by compatibility (Совместимые принтеры / Остальные принтеры) when printer_cartridge_models links exist for the cartridge's model; falls back to a single flat list (no blocking) when none exist"
  - "D-22: selecting a printer that already has a cartridge «В работе» reuses the existing previousCartridge block (no new markup) — editable Расположение + Уровень заряда (default Пустой) flow into the same transition() call"
  - "Aliased bindings-phase6's PrinterDto as PrinterListItemDto in OperationModal.svelte to avoid a structural TS mismatch with bindings.ts's generated PrinterDto (tonerLevels: JsonValue vs Record<string, number|null>) — printers.list() returns the phase6 shape, printers.get() is assignable to both"

requirements-completed: [D-20, D-21, D-22]

# Metrics
duration: 35min
completed: 2026-06-24
---

# Phase 12 Plan 20: Optional printer selection in cartridge-centric install Summary

**Cartridge-centric "Установить в принтер" now has an optional PrinterSelect dropdown (compatibility-prioritized via printer_cartridge_models) that drives the same printer-context lookup and auto-return «Предыдущий картридж» block the request-centric flow already had — closing GAP-12-11 and GAP-12-12 п.1/3 from Round 3 verification.**

## Performance

- **Duration:** 35 min
- **Started:** 2026-06-24T22:45:00Z
- **Completed:** 2026-06-24T23:20:29Z
- **Tasks:** 2 completed
- **Files modified:** 2 (1 created, 1 modified)

## Accomplishments

- Built `PrinterSelect.svelte` — a new compatibility-aware printer dropdown, structurally copied from `GroupedPrinterSelect.svelte`'s markup/SCSS but grouping by `printer_cartridge_models` compatibility instead of by location, with a graceful flat-list fallback when no compatibility links are configured (D-21).
- Wired the cartridge-centric install entry in `OperationModal.svelte` with a new `effectivePrinterId` derived (`preFillPrinterId ?? selectedPrinterId`) that unifies both UI entry points into the single existing printer-context/previous-cartridge lookup and `buildPayload()` logic — no duplicated logic, no new backend calls beyond the two read-only lookups already used elsewhere in the phase.
- Closed the structural gap identified in `12-VERIFICATION-ROUND3.md`: the cartridge-centric flow can now reach the same printer-linkage/auto-return functionality as the request-centric flow, entirely optionally (D-20), with zero regression to the legacy "no printer" path.

## Task Commits

Each task was committed atomically:

1. **Task 1: Create PrinterSelect.svelte** - `70bef38` (feat)
2. **Task 2: Wire PrinterSelect into OperationModal** - `28f9945` (feat)

**Plan metadata:** (this commit)

## Files Created/Modified

- `ui/src/lib/components/PrinterSelect.svelte` - New compatibility-prioritized printer `<select>`/`<optgroup>` component (props: `options`, `compatibleDeviceIds`, `value`, `disabled?`, `invalid?`, `id?`, `onchange?`); placeholder option "Без привязки к принтеру" makes empty selection valid (D-20).
- `ui/src/features/cartridges/OperationModal.svelte` - Added `selectedPrinterId`/`printerOptions`/`compatibleDeviceIds` state, `effectivePrinterId` derived, a new `$effect` loading `printers.list()` + `cartridges.modelsGetCompatibleDevices()` gated on `cartridge !== null && preFillPrinterId === undefined`, and the new `PrinterSelect` render block in the install form template, before the existing `printerContextHint`.

## Decisions Made

- **D-20 (opt-in printer linkage):** No backend change needed — `printer_device_id` was already optional in `CartridgeTransitionPayload`. The only gap was a missing UI affordance in the cartridge-centric entry; `selectedPrinterId` defaults to `undefined`, which flows through `effectivePrinterId` and into `buildPayload()` as `null`, exactly matching pre-plan behavior.
- **D-21 (compatibility-first, never blocking):** Implemented as a single `$derived.by` computing two buckets (compatible/rest) from the existing `cartridge_models_get_compatible_devices(modelId)` reverse lookup; when the model has zero compatibility links, the component renders one flat ungrouped list rather than an empty/blocked state — mirrors the project's established D-13/D-14 "compatibility not configured → don't block" pattern, just inverted (printers-by-cartridge instead of cartridges-by-printer).
- **D-22 (operator-controlled return, not silent):** Zero new markup — the existing `previousCartridge` block (with its editable `previousCartridgeStateId`/`previousCartridgeLocation` fields, defaulting to Пустой/empty) already satisfied this exactly; it simply needed `effectivePrinterId` to be reachable from the cartridge-centric entry point, which the lookup `$effect` change (line ~205) provides.
- **Type aliasing for PrinterDto:** `OperationModal.svelte` already imported `PrinterDto` from the auto-generated `bindings.ts` for `printerContext`/single-`printers.get()` lookups. `printers.list()` (used by the new selector) is typed against the hand-maintained `bindings-phase6.ts` `PrinterDto`/`PrinterListResponse`. The two types differ only in `tonerLevels`'s precise shape (`JsonValue` vs `Record<string, number|null>`), which TypeScript treats as incompatible for array assignment (though tolerant for single-object assignment, which is why the pre-existing `printerContext = printer` line never surfaced this). Resolved by importing the phase6 type under an alias (`PrinterListItemDto`) for the two new state variables (`printerOptions`), rather than touching `printers/api.ts` or the generated bindings (out of scope, no backend regen needed).

## Deviations from Plan

None — plan executed exactly as written, with one minor implementation-detail addition not explicitly spelled out in the plan text (the `PrinterDto` type aliasing above), which falls under Rule 3 (auto-fix blocking issue — `svelte-check` reported a real type error blocking the build) and was the only deviation needed.

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Aliased bindings-phase6's PrinterDto to resolve a structural type mismatch**
- **Found during:** Task 2 (wiring `PrinterSelect` into `OperationModal.svelte`)
- **Issue:** `svelte-check` reported `Type 'PrinterDto[]' from bindings.ts is not assignable to type 'PrinterDto[]' from bindings-phase6.ts` (`tonerLevels: JsonValue` vs `Record<string, number|null> | null`) when assigning `printers.list()`'s result (typed against `bindings-phase6`) to a `$state<PrinterDto[]>` declared with the file's existing `bindings`-sourced `PrinterDto` import.
- **Fix:** Imported `PrinterDto` from `'../../bindings-phase6'` under the alias `PrinterListItemDto`, and typed only the new `printerOptions` state with it — leaving the existing `printerContext: PrinterDto | null` (from `bindings`) untouched, since single-object assignment from `printers.get()` was already tolerant of the mismatch.
- **Files modified:** `ui/src/features/cartridges/OperationModal.svelte` (same commit as Task 2, no separate commit).
- **Verification:** `pnpm --dir ui exec svelte-check` → 0 errors (was 1 error before the fix); `pnpm --dir ui build` → `✓ built`.
- **Committed in:** `28f9945` (part of Task 2 commit).

---

**Total deviations:** 1 auto-fixed (1 blocking — Rule 3)
**Impact on plan:** Necessary for the build to compile; no scope creep, no behavior change, purely a TypeScript type-resolution fix local to the new code path.

## Issues Encountered

None beyond the type-aliasing deviation documented above.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Both FAILED truths from `12-VERIFICATION-ROUND3.md` (GAP-12-11, GAP-12-12 п.1/3) are now structurally closed: the cartridge-centric install entry has a reachable, optional printer selector that feeds the same auto-return/previous-cartridge logic the request-centric flow already exercises.
- `svelte-check` (0 errors) and `pnpm --dir ui build` (✓ built) both pass; `TRACKLY_AD_MOCK=1 cargo test` remains fully green (no backend changes in this plan, as expected).
- `ui/dist` was rebuilt after the frontend changes, so LAN-browser/server-mode testing will see the new selector immediately.
- Manual/UAT verification (live browser, both desktop webview and LAN browser) of the actual selector behavior — compatible-printer grouping, previous-cartridge auto-return prompt, and the "no printer selected" legacy path — is still recommended before closing the phase, per the plan's own verification section (code-level review + automated checks were performed here; this plan did not include a live interactive checkpoint).
- Round 4 gap-closure (D-20/D-21/D-22) is now fully implemented; Phase 12 should be re-verified end-to-end before being marked complete, per the user's standing instruction not to mark the phase complete until all UAT gaps are closed and reverified.

---
*Phase: 12-cartridge-request-interconnection*
*Completed: 2026-06-24*

## Self-Check: PASSED

- FOUND: ui/src/lib/components/PrinterSelect.svelte
- FOUND: ui/src/features/cartridges/OperationModal.svelte
- FOUND: .planning/phases/12-cartridge-request-interconnection/12-20-SUMMARY.md
- FOUND commit: 70bef38 (Task 1)
- FOUND commit: 28f9945 (Task 2)

---
phase: 30-quality-a11y-platform-parity
plan: 02
subsystem: ui
tags: [a11y, focus-ring, css, svelte, scss]

# Dependency graph
requires:
  - phase: 30-quality-a11y-platform-parity
    plan: 01
    provides: "check-focus-outline.mjs lint gate + 30-PATTERNS.md pre-triage of the 3 defects fixed here"
provides:
  - "Dropdown.svelte .tr-dropdown-search-input — visible outward focus ring"
  - "ModelListRow.svelte .kebab-btn — inset focus ring, un-clipped by .models-list overflow:hidden"
  - "TableRow.svelte .tr-row-chevron — own inset focus ring, un-clipped by Table.svelte framed overflow:hidden"
  - "check-focus-outline.mjs now green (0 violations) — closes the regression 30-01 left open"
affects: [30-03]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Inset-ring idiom (outline: none; box-shadow: inset 0 0 0 2px var(--tr-accent);) reused verbatim from ActListRow/CartridgeListRow/PrinterListRow/RequestListRow for any focusable element sitting inside an overflow:hidden ancestor"
    - "Outward 2px ring idiom (box-shadow: 0 0 0 2px var(--tr-focus-ring);) reused from Sidebar .logout-btn for small inline elements that are NOT inside a clipping ancestor"

key-files:
  created: []
  modified:
    - ui/src/lib/components/Dropdown.svelte
    - ui/src/features/cartridges/ModelListRow.svelte
    - ui/src/lib/components/TableRow.svelte

key-decisions:
  - "Dropdown search-input got the outward 2px ring idiom (not inset) — its panel scrolls via overflow:auto (not hidden) and the search box is position:sticky, so nothing clips it vertically"
  - ".tr-dropdown-option (drill-in panel items) intentionally left untouched — flagged as a UAT candidate for plan 30-03's final checkpoint, not a blind fix in this plan"
  - "Tabs.svelte confirmed untouched — its focus-visible was already correct per 30-01/30-PATTERNS.md"

requirements-completed: [QA-02]

# Metrics
duration: ~5min
completed: 2026-07-24
---

# Phase 30 Plan 02: Focus-Ring Point Fixes Summary

**Three targeted CSS fixes closing the last check-focus-outline.mjs violation and two ancestor-overflow ring-clipping defects, using idioms already established elsewhere in the codebase — zero markup/logic changes.**

## Performance

- **Duration:** ~5 min
- **Started:** 2026-07-24T17:15:18Z
- **Completed:** 2026-07-24T17:16:50Z (approx.)
- **Tasks:** 3/3
- **Files modified:** 3

## Accomplishments

- `Dropdown.svelte`: `.tr-dropdown-search-input` had a bare `outline: none;` with no replacement — added `&:focus-visible { box-shadow: 0 0 0 2px var(--tr-focus-ring); }`, the same 2px inline-element scale as `Sidebar.svelte`'s `.logout-btn`.
- `ModelListRow.svelte`: `.kebab-btn`'s outward 3px ring was clipped by `.models-list { overflow: hidden; }` (`ModelsList.svelte:71`, `framed={false}` on `Table`) — swapped to the inset idiom (`box-shadow: inset 0 0 0 2px var(--tr-accent)`) already used by `ActListRow`/`CartridgeListRow`/`PrinterListRow`/`RequestListRow`.
- `TableRow.svelte`: `.tr-row-chevron` had no own focus rule at all, relying on the global outward baseline which is clipped by `Table.svelte`'s `framed=true` default `overflow: hidden`. Added a dedicated `&:focus-visible` inset ring. `TableRow` is the shared primitive for every grouped table in the app (Devices at minimum), so this is the widest-blast-radius fix in the plan and required zero changes in any consumer.
- `node ui/scripts/check-focus-outline.mjs` now exits 0 (0 violations) — the regression left open at the end of plan 30-01 is closed.
- `pnpm --dir ui lint` (eslint + prettier + check-tokens + check-contrast + check-focus-outline) is fully green.
- `pnpm --dir ui svelte-check`: 0 errors (48 pre-existing warnings, unrelated to this plan's files).
- `pnpm --dir ui build`: succeeds.

## Task Commits

1. **Task 1: Dropdown.svelte — видимое кольцо на .tr-dropdown-search-input** - `006b29f` (fix)
2. **Task 2: ModelListRow.svelte — inset-кольцо на .kebab-btn** - `8f87b2b` (fix)
3. **Task 3: TableRow.svelte — новое inset-кольцо на .tr-row-chevron** - `d01e24e` (fix)

_No TDD tasks in this plan (tdd="false" on all three)._

## Files Created/Modified

- `ui/src/lib/components/Dropdown.svelte` - Added `&:focus-visible { box-shadow: 0 0 0 2px var(--tr-focus-ring); }` to `.tr-dropdown-search-input`
- `ui/src/features/cartridges/ModelListRow.svelte` - Replaced `.kebab-btn`'s `&:focus-visible` box-shadow from outward `0 0 0 3px var(--tr-focus-ring)` to inset `inset 0 0 0 2px var(--tr-accent)`
- `ui/src/lib/components/TableRow.svelte` - Added new `&:focus-visible { outline: none; box-shadow: inset 0 0 0 2px var(--tr-accent); }` block to `.tr-row-chevron`

## Decisions Made

- Dropdown search-input uses the outward (not inset) ring idiom because its enclosing panel scrolls (`overflow: auto`) rather than clips (`overflow: hidden`), and the search box itself is `position: sticky` — there is no clipping ancestor to worry about, matching the reasoning already used for `Button.svelte` and 20+ other primitives.
- `.tr-dropdown-option` (drill-in panel list items) was deliberately left untouched per the plan's explicit scope boundary — it's flagged as a manual UAT candidate for plan 30-03's final checkpoint, not blindly fixed here.
- Confirmed `Tabs.svelte` was not touched — its focus-visible handling was already correct per the 30-01 summary and `30-PATTERNS.md`.

## Deviations from Plan

None - plan executed exactly as written. All three tasks' acceptance criteria were met on first pass; no bugs, missing functionality, or blocking issues were encountered.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `check-focus-outline.mjs` is green (0 violations) — the durable regression gate from plan 30-01 is now fully satisfied.
- `pnpm --dir ui lint` is fully green (all 3 automated a11y/token gates pass).
- Plan 30-03 (final cross-platform/manual UAT pass) can proceed; the one deferred UAT candidate from this plan (`.tr-dropdown-option` inside the drill-in panel) is documented above for that checkpoint.
- No blockers.

---
*Phase: 30-quality-a11y-platform-parity*
*Completed: 2026-07-24*

## Self-Check: PASSED

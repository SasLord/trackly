---
phase: 25-dropdown
plan: 01
subsystem: ui
tags: [svelte5, design-system, table, tokens, scss]

# Dependency graph
requires:
  - phase: 24-base-components
    provides: "--tr-* token layer (Phase 23) + Badge.svelte count/soft appearances reused by TableRow's group-row mode"
provides:
  - "TableRow.svelte — row-state primitive (normal/hover/selected/indent/last) + group-row mode (chevron/name/toggle)"
  - "Table.svelte — reusable shell (header row, loading skeleton, empty state)"
  - "--tr-group design token (light #e9edf5 / dark #1a212b) in _tokens.scss"
affects: [25-04-showcase-table-section, 25-05-devicelist-pilot]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "class-outside-:global() escape hatch for caller-rendered <td>/<th> cells inside a component's own scoped <style lang=\"scss\"> block — `.tr-row :global(> td)` (specificity 0,2,1) instead of `:global(.tr-row > td)` (0,1,1), which would lose to a consumer's own class"

key-files:
  created:
    - ui/src/lib/components/TableRow.svelte
    - ui/src/lib/components/Table.svelte
  modified:
    - ui/src/styles/_tokens.scss

key-decisions:
  - "TableRow owns ALL base <td> metrics (height 40px, padding 0 10px, border-bottom) per D-10 — consumers (Plans 25-04/25-05) must NOT redeclare them"
  - "Chevron transition stays at .15s (TableRows.dc verbatim value), not the design system's usual .12s micro-transition"
  - "No consumers wired in this plan (per plan scope) — DeviceList/DeviceListRow/DeviceGroupRow migration is Plan 25-05, showcase section is Plan 25-04"

patterns-established:
  - "Selector-shape gate for :global()-scoped caller content: class name stays OUTSIDE :global(), only the combinator+tag goes inside, to preserve component-scope specificity over consumer classes"

requirements-completed: [CMP-06]

# Metrics
duration: ~5min
completed: 2026-07-18
---

# Phase 25 Plan 01: TableRow + Table primitives Summary

**Built the two CMP-06 row/table-shell primitives (`TableRow.svelte`, `Table.svelte`) plus the missing `--tr-group` design token, all pixel/token-exact to `TableRows.dc.html` — no consumers wired yet.**

## Performance

- **Duration:** ~5 min
- **Completed:** 2026-07-18T19:52:56Z
- **Tasks:** 2 completed
- **Files modified:** 3

## Accomplishments
- `--tr-group` token added to both light and dark theme blocks of `_tokens.scss`, verbatim from `TableRows.dc.html` (closed-world `check-tokens.mjs` Rule 3 satisfied — token defined before first `.svelte` reference)
- `TableRow.svelte` implements the full normal-mode (selected/indent/last, CSS-class toggles) and group-mode (chevron/name/colspan/toggle) contract from the plan's `<interfaces>` block, with TableRow as the sole owner of base `<td>` cell metrics per D-10
- `Table.svelte` implements the reusable shell (header/skeleton/empty) so future Table-track consumers don't reimplement skeleton/empty markup per table

## Task Commits

Each task was committed atomically:

1. **Task 1: Add --tr-group token and build TableRow.svelte** - `d4aa52b` (feat)
2. **Task 2: Build Table.svelte shell** - `c428789` (feat)

**Plan metadata:** committed as part of this summary commit

## Files Created/Modified
- `ui/src/styles/_tokens.scss` - added `--tr-group: #e9edf5` (light) / `#1a212b` (dark) to the "Table row states" token group in both theme blocks
- `ui/src/lib/components/TableRow.svelte` - row-state primitive (selected/indent/last/hover via CSS) + group-row mode (chevron ▸ rotating 90deg, merged name cell, caller-supplied trailing cells snippet); owns base `<td>` metrics (height 40px, padding 0 10px, border-bottom) via `.tr-row :global(> td)` escape hatch
- `ui/src/lib/components/Table.svelte` - shell component: `columns`/`loading`/`empty`/`emptyTitle`/`emptyBody`/`skeletonRows`/`head`/`children` props; header row (34px, `--tr-border-strong` bottom border, `:global(> th)` styling for caller `<th>` cells); skeleton rows mirroring `TableRow`'s base cell metrics so real rows don't jump on load; empty-state branch with title/body

## Decisions Made
- Followed the plan's `<interfaces>` contract exactly: plain (non-bindable) `Props` interfaces with `$props()` destructuring and defaults, matching `DeviceListRow.svelte`'s existing shape (per D-11 — selection is one-way display state, not two-way bound)
- Used the CLASS-OUTSIDE-`:global()` selector shape (`.tr-row :global(> td)`) rather than the inside-out form (`:global(.tr-row > td)`) per the plan's explicit specificity-gate reasoning — the inside-out form would compile to a lower specificity than a consumer's leftover `.cell` class and silently drift cell padding
- `groupColspan` defaults to `1` (plan-specified default) — callers supply the real colspan matching however many left columns the group name cell should merge
- Chevron glyph `▸` with `transform: rotate(90deg)` on expand (not the old `DeviceGroupRow.svelte`'s `180deg`) and `.15s` transition kept as a local override, not generalized into a token, per UI-SPEC's explicit "this ONE transition stays at .15s" instruction

## Deviations from Plan

None — plan executed exactly as written. Both tasks' acceptance criteria (grep-verified selector shapes, token presence, absence of the old 8px padding, zero hex/rgba literals) and automated verification (`check-tokens.mjs`, `svelte-check`, `pnpm --dir ui build`) passed on first attempt with no fix-up commits needed.

## Known Stubs

None. Both `TableRow.svelte` and `Table.svelte` are unconsumed library primitives by design — this plan's explicit scope excludes wiring any consumer (`<objective>`: "No consumers are touched in this plan"). Downstream Plans 25-04 (showcase section) and 25-05 (`DeviceList` pilot) are the consumer/visual-UAT surface for this plan's output; nothing here renders on any live screen yet, so there is no incomplete-vs-hardcoded distinction to track.

## Issues Encountered

None.

## User Setup Required

None — no new dependencies, no environment variables, no manual steps.

## Threat Flags

None. Both files are presentation-only Svelte components: zero `{@html}` usage (verified via grep, matching threat register mitigation T-25-01-01), zero new npm dependencies, zero new data-fetching or API/Tauri-command surface. `groupName`/`emptyTitle`/`emptyBody` are plain caller-supplied strings rendered via Svelte's default-escaped text interpolation — no XSS sink introduced.

## Next Steps
- Plan 25-02/25-03: `Dropdown.svelte` (CMP-07) — independent track, not blocked by this plan
- Plan 25-04: showcase `TableSection` — first consumer proving all three row states + group-row + skeleton/empty visually
- Plan 25-05: `DeviceList`/`DeviceListRow`/`DeviceGroupRow` migrated onto these primitives (table pilot, D-05)

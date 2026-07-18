---
phase: 25-dropdown
plan: 05
subsystem: ui
tags: [svelte5, design-system, table, migration, devices]

# Dependency graph
requires:
  - phase: 25-dropdown
    plan: 01
    provides: "TableRow.svelte / Table.svelte primitives (CMP-06) — this plan is their live-data consumer"
provides:
  - "DeviceList.svelte / DeviceListRow.svelte / DeviceGroupRow.svelte migrated onto Table/TableRow — closes CMP-06's real-screen pilot (D-05)"
affects: []

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Cross-scope :global() passthrough class override needs enough class-selector count to out-specificity the primitive's own base <td> rule — a bare `:global(tr.some-class > td)` loses to TableRow's `.tr-row.hash > td` (2 classes); anchoring on the consumer's own local .cell class instead (`:global(tr.some-class) > .cell`) adds the needed third class"
    - "When collapsing a 3-way if/loading/empty/else branch into a single shell component, mirror the ORIGINAL branch condition into a named derived (not just the shell's own `loading`/`empty` props) for any sibling markup (e.g. a footer) that must stay hidden in the same branch as the removed skeleton table"

key-files:
  modified:
    - ui/src/features/devices/DeviceListRow.svelte
    - ui/src/features/devices/DeviceGroupRow.svelte
    - ui/src/features/devices/DeviceList.svelte

key-decisions:
  - "DeviceListRow's group-last-child divider selector changed from the plan's literal `:global(tr.group-last-child > td)` suggestion to `:global(tr.group-last-child) > .cell` — the literal form computes specificity (0,1,2), which LOSES to TableRow's own base border-bottom rule `.tr-row.hash > td` (0,2,1); anchoring on the local `.cell` class (which keeps this file's own scope hash) raises it to (0,3,1), correctly overriding TableRow's 1px border with the 2px group-end divider"
  - "Removed DeviceGroupRow's `.cell-name-wide` rule entirely (not kept as the plan's literal instruction implied) — the name-cell markup it styled moved into TableRow's own group-name <td>, leaving the selector unused; keeping it would trip svelte-check's unused-CSS-selector warning gate"
  - "DeviceList's footer visibility gated by a new `skeletonLoading` derived (loading && items.length===0 && groups.length===0) shared with Table's `loading` prop, not just `!isEmpty` — a naive `!isEmpty`-only condition would show the footer during the initial-load skeleton, since `isEmpty` is false while `loading` is true (a regression the plan's action text did not explicitly call out)"

requirements-completed: [CMP-06]

# Metrics
duration: ~25min
completed: 2026-07-19
---

# Phase 25 Plan 05: DeviceList/DeviceListRow/DeviceGroupRow migration to Table/TableRow Summary

**Migrated the live Devices screen (DeviceList.svelte shell + DeviceListRow/DeviceGroupRow rows) onto the Table/TableRow primitives from Plan 25-01 — the only in-scope real-screen pilot for CMP-06, business logic (expand/collapse, status mapping, group stable keys, edit/delete) fully unchanged.**

## Performance

- **Duration:** ~25 min
- **Completed:** 2026-07-19
- **Tasks:** 2 completed
- **Files modified:** 3

## Accomplishments

- `DeviceListRow.svelte` now wraps its `<td>` cells in `<TableRow>` instead of a hand-rolled `<tr class="device-row">`; `TableRow` owns base cell height/padding/border-bottom, and the migration fixes a pre-existing spec deviation (old hover color was `--tr-surface`, now correctly `--tr-row-hover` via TableRow's CSS)
- `DeviceGroupRow.svelte` now uses `TableRow`'s built-in group mode (`group`/`groupExpanded`/`groupName`/`groupColspan`/`onToggleGroup`) instead of a hand-rolled chevron SVG button and `color-mix` background; the count-pill is now `Badge variant="accent" appearance="count"`; chevron rotation corrected from `180deg` to `90deg` (UI-SPEC-exact)
- `DeviceList.svelte`'s 3-way loading-skeleton/empty/real-table `<table>` branches collapsed into a single `<Table columns loading empty emptyTitle emptyBody head>` call; header `<th>`s passed as a `head` snippet, the unchanged `showGroups`/`DeviceGroupRow`/`DeviceListRow` render loop passed as `children`
- The group-end "strong divider" behavior (`isLastInGroup` on the last nested row) survives via `TableRow`'s `class` passthrough prop, re-anchored with sufficient CSS specificity to beat `TableRow`'s own base border-bottom rule
- No row-selection mechanic introduced anywhere (D-11) — `TableRow`'s `selected` prop is never set to `true` in any of the 3 migrated files

## Task Commits

Each task was committed atomically:

1. **Task 1: Migrate DeviceListRow.svelte and DeviceGroupRow.svelte to TableRow** - `1f29be4` (feat)
2. **Task 2: Migrate DeviceList.svelte to the Table shell** - `2308613` (feat)

**Plan metadata:** committed as part of this summary commit

## Files Created/Modified

- `ui/src/features/devices/DeviceListRow.svelte` — `<tr class="device-row">` replaced with `<TableRow class={isLastInGroup ? 'group-last-child' : undefined}>`; `.device-row` rule removed entirely; `.cell` rule stripped of the 3 declarations now owned by `TableRow` (padding/border-bottom/vertical-align), keeping only content-specific ones (font-size/color/white-space/overflow/text-overflow/max-width); group-last-child divider re-implemented as a `:global()` passthrough selector anchored on the local `.cell` class for correct specificity
- `ui/src/features/devices/DeviceGroupRow.svelte` — hand-rolled `<tr onclick>` + chevron SVG `<button>` + name text replaced with `<TableRow group groupExpanded groupName groupColspan={4} onToggleGroup>`; `<span class="count-pill">` replaced with `<Badge variant="accent" appearance="count">`; `.group-row`/`.chevron-btn`/`.count-pill`/`.cell-name-wide` rules removed (superseded by TableRow's group mode); `.cell` rule stripped of padding/vertical-align/border-bottom (kept font-size/color); `.children-loading`'s own `border-bottom` (a plain, non-TableRow `<tr>`) left untouched
- `ui/src/features/devices/DeviceList.svelte` — 3-way `{#if loading}{:else if isEmpty}{:else}` `<table>` branching replaced with a single `<Table columns loading={skeletonLoading} empty={isEmpty} emptyTitle emptyBody head={tableHead}>` call; added `skeletonLoading` derived to keep the footer's visibility condition exactly matching the pre-migration behavior; removed `.device-table`/`.header-row`/`.th`/`.empty-state`/`.empty-title`/`.empty-body`/`.skeleton-row`/`.skeleton-cell`/`.skeleton-block`/`@keyframes pulse` (now owned by `Table.svelte`); kept `.th-name`/`.th-numeric`/`.th-condition`/`.th-status`/`.th-actions` (per-column widths) and `.list-footer`/`.pagination-info` (footer, unmoved)

## Decisions Made

- Followed the plan's exact prop-wiring contract for both `TableRow` (group mode) and `Table` (shell), matching Plan 25-01's `<interfaces>` and Plan 25-04's established usage patterns
- Deviated from the plan's literal group-last-child selector text (see Deviations below) to preserve the divider's actual visual behavior
- Removed `.cell-name-wide` from `DeviceGroupRow.svelte` rather than keeping it "unchanged" as the plan's read_first section implied — the class became unused once the name-cell markup moved into `TableRow`'s own group-name `<td>`, and an unused CSS selector would have surfaced as a `svelte-check` warning

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Group-last-child divider selector needed higher specificity than the plan's literal suggestion**
- **Found during:** Task 1, while wiring `DeviceListRow.svelte`'s style block
- **Issue:** The plan's action text suggested `:global(tr.group-last-child > td) { border-bottom: 2px solid var(--tr-border-strong); }`. This compiles to specificity (0,1,2) (1 class + 2 type selectors). `TableRow.svelte`'s own base `<td>` rule (`.tr-row :global(> td)`) compiles to (0,2,1) (2 classes incl. its scope hash + 1 type selector). Since (0,2,1) > (0,1,2), TableRow's 1px `border-bottom` would have silently WON over the intended 2px group-end divider — a visual regression on the last row of every expanded multi-device group.
- **Fix:** Rewrote the selector as `:global(tr.group-last-child) > .cell` — anchoring on the local `.cell` class (which keeps this file's own Svelte scope-hash) instead of the bare `td` type selector raises specificity to (0,3,1), correctly beating TableRow's (0,2,1).
- **Files modified:** `ui/src/features/devices/DeviceListRow.svelte`
- **Commit:** `1f29be4`

**2. [Rule 1 - Bug] Footer visibility would have regressed during initial-load skeleton**
- **Found during:** Task 2, while collapsing `DeviceList.svelte`'s 3-way branch into the single `<Table>` call
- **Issue:** The pre-migration code only rendered `<footer class="list-footer">` in the third (`{:else}`) branch — i.e. NOT during the skeleton-loading branch and NOT during the empty-state branch. A literal translation using only `{#if !isEmpty}` to gate the footer would have shown it during initial load too, since `isEmpty` is derived as `!loading && (...)` and is therefore `false` (making `!isEmpty` `true`) for the whole duration of the skeleton-loading state.
- **Fix:** Added a `skeletonLoading` derived (`loading && items.length === 0 && groups.length === 0`, the exact original skeleton-branch condition) and gated the footer with `{#if !skeletonLoading && !isEmpty}`; reused the same derived as `Table`'s `loading` prop so the condition has a single source of truth.
- **Files modified:** `ui/src/features/devices/DeviceList.svelte`
- **Commit:** `2308613`

**3. [Rule 1 - Bug] Removed `.cell-name-wide` from DeviceGroupRow.svelte instead of keeping it**
- **Found during:** Task 1, while cleaning `DeviceGroupRow.svelte`'s style block
- **Issue:** The plan's read_first note listed `.cell-name-wide` among the rules to "Keep ... (column-specific, unchanged)", but the `<td class="cell-name-wide">` element it styled was removed as part of the migration (the merged name cell is now rendered by `TableRow`'s own group-name `<td>`, which doesn't accept a passthrough class for this purpose). Keeping the now-unused selector would trigger `svelte-check`'s `css_unused_selector` warning.
- **Fix:** Deleted `.cell-name-wide` from the style block.
- **Files modified:** `ui/src/features/devices/DeviceGroupRow.svelte`
- **Commit:** `1f29be4`

## Known Stubs

None. All 3 files render real device data through the same API/props contract as before; no hardcoded/placeholder values were introduced.

## Issues Encountered

None beyond the 3 auto-fixed deviations above, all caught and fixed pre-commit via the plan's own acceptance-criteria greps and the `check-tokens.mjs`/`svelte-check`/`build` verification gates.

## User Setup Required

None — no new dependencies, no environment variables, no manual steps.

## Threat Flags

None. All 3 touched files remain presentation-only: zero `{@html}` usage (unchanged from pre-migration), zero new npm dependencies, zero new data-fetching or API/Tauri-command surface. Device field rendering (name/model/location/state) is unchanged — still default-escaped Svelte text interpolation. No new selection/action mechanic was added (D-11 out of scope); existing edit/delete/print callbacks are unchanged, still gated by the pre-existing role checks in the parent `DevicesPage`.

## Next Steps

- CMP-06 is now fully closed: primitives (25-01), showcase visual-UAT (25-04), and this real-screen pilot (25-05) all done
- Phases 26-28 migrate the remaining 5 tables (UsersList, ReportTable, DiscoveryResultsTable, CartridgeListRow, ModelListRow) onto the same `Table`/`TableRow` primitives, per D-08
- Plans 25-06/25-07 (remaining Phase 25 scope) proceed independently

## Self-Check: PASSED

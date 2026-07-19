---
phase: 26-windows-with-mockup
plan: 03
subsystem: ui
tags: [svelte, svelte5-snippets, design-system, table, input]

# Dependency graph
requires:
  - phase: 25-dropdown
    provides: Table.svelte and TableRow shell components, --tr-* token layer
provides:
  - "Table.svelte: framed?:boolean (default true) prop drawing border+radius(8px)+overflow:hidden+--tr-elev-1 frame"
  - "Table.svelte: footer?:Snippet prop rendered inside the frame below the scroller"
  - "Input.svelte: iconLeft?:Snippet prop rendering a left-positioned icon at 12px with 34px input padding-left"
affects: [26-04-devices-window, 26-05-device-filters]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Optional-Snippet 'absent = not rendered' convention (copied from Modal.svelte footer) applied to Table.footer and Input.iconLeft"
    - "Frame/scroll responsibility split: outer .tr-table-framed owns overflow:hidden (corner clipping), inner .tr-table-wrapper keeps overflow-x:auto (horizontal scroll) — never merged into one rule"

key-files:
  created: []
  modified:
    - ui/src/lib/components/Table.svelte
    - ui/src/lib/components/Input.svelte

key-decisions:
  - "Table's new outer wrapper is a plain div (not display:contents) so the optional footer can sit as a flex/block sibling of the scroller regardless of framed value"
  - "Input's new .input-wrap is unconditionally display:block;width:100% so zero layout change occurs for every existing call site that omits iconLeft"

patterns-established:
  - "Backward-compatible primitive extension: new optional prop + conditional wrapper, base styles untouched for the no-prop path — verified via grep for byte-identical base rules"

requirements-completed: [WIN-02]

# Metrics
duration: 6min
completed: 2026-07-19
---

# Phase 26 Plan 03: Table + Input primitive extensions Summary

**Table.svelte gains a `framed`+`footer` frame/footer pair and Input.svelte gains an `iconLeft` snippet slot, both fully backward-compatible with existing callers.**

## Performance

- **Duration:** 6 min
- **Started:** 2026-07-19T23:28:00Z
- **Completed:** 2026-07-19T23:34:13Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- `Table.svelte` wraps the existing scrollable `.tr-table-wrapper` in a new `.tr-table-framed` outer div with `framed?: boolean` (default `true`) driving border/radius(8px)/`overflow:hidden`/`--tr-elev-1`, kept as a separate rule from the wrapper's own `overflow-x:auto`
- `Table.svelte` gains an optional `footer?: Snippet`, rendered as a sibling inside the frame below the scroller, using the same `{#if footer}...{@render footer()}{/if}` guard as `Modal.svelte`
- `Input.svelte` gains `iconLeft?: Snippet`, rendered inside a new `.input-wrap` div (`position:relative`, unconditional `display:block;width:100%`) with the icon absolutely positioned at `left:12px`
- `Input.svelte`'s base `.input` rule and its `padding: 0 var(--tr-space-md)` are untouched; `.input.has-icon` adds `padding-left:34px` only when `iconLeft` is passed

## Task Commits

Each task was committed atomically:

1. **Task 1: Table.svelte — framed wrapper + optional footer snippet** - `8b6e61c` (feat)
2. **Task 2: Input.svelte — iconLeft prop, backward-compatible** - `1c654e8` (feat)

**Plan metadata:** (pending — this commit)

## Files Created/Modified
- `ui/src/lib/components/Table.svelte` - adds `framed`/`footer` props, `.tr-table-framed`/`.tr-table-footer` styles
- `ui/src/lib/components/Input.svelte` - adds `iconLeft` prop, `.input-wrap`/`.input-icon` styles, `.input.has-icon` padding rule

## Decisions Made
None - followed plan as specified.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- `Table`'s `framed`/`footer` props and `Input`'s `iconLeft` prop are ready for Plan 26-04 (`DeviceList.svelte`) and Plan 26-05 (`DeviceFilters.svelte`) to consume directly.
- No further primitive changes expected for these two components in this milestone.

---
*Phase: 26-windows-with-mockup*
*Completed: 2026-07-19*

## Self-Check: PASSED

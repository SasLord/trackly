---
phase: 18-autocomplete-dropdowns
plan: 02
subsystem: ui
tags: [svelte5, portal, dropdown-positioning, autocomplete, use-action]

# Dependency graph
requires:
  - phase: 18-autocomplete-dropdowns
    provides: "Plan 18-01 (phase scaffolding / earlier plan in this phase)"
provides:
  - "ui/src/lib/utils/dropdownAnchor.ts — reusable Svelte use-action computing fixed anchor-relative coordinates for portaled dropdowns, repositioning on capture-phase scroll/resize, flipping upward near viewport bottom"
  - "LocationAutocomplete.svelte migrated to use:portal + use:dropdownAnchor as the first real consumer / proof-of-concept"
affects: [18-03, 18-04, 18-05]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "dropdownAnchor use-action: getBoundingClientRect() anchor math + capture-phase scroll listener (window.addEventListener('scroll', reposition, true)) reposition-not-close contract for portal dropdowns (D-02)"
    - ":global() wrapping for scoped-CSS classes on elements moved into <body> via use:portal"

key-files:
  created:
    - ui/src/lib/utils/dropdownAnchor.ts
  modified:
    - ui/src/lib/components/LocationAutocomplete.svelte

key-decisions:
  - "dropdownAnchor computes both .dropdown and .dropdown-item CSS as :global() (not just the root), matching DeviceContextMenu.svelte precedent, to avoid Svelte's scoped-CSS pruning/dead-code-elimination risk on descendants of a portaled node"
  - "box-shadow switched from unused --shadow-md token to --shadow-elev-2 (real, existing token) per plan instruction; no dark-mode override introduced (project doesn't use --shadow-elev-2-dark anywhere yet)"

requirements-completed: [AUTO-01]

duration: ~12min
completed: 2026-07-10
---

# Phase 18 Plan 02: Reusable dropdownAnchor portal-positioning layer Summary

**New `dropdownAnchor` Svelte use-action computing fixed anchor-relative coordinates (with capture-phase scroll reposition and upward flip), applied to `LocationAutocomplete.svelte` as the first portal+anchor consumer.**

## Performance

- **Duration:** ~12 min
- **Started:** 2026-07-10T00:00:00Z
- **Completed:** 2026-07-10T00:12:01Z
- **Tasks:** 2
- **Files modified:** 2 (1 created, 1 modified)

## Accomplishments
- Created `ui/src/lib/utils/dropdownAnchor.ts`: reusable Svelte use-action exporting `dropdownAnchor(node, { anchorEl, gap?, maxHeight? })` — computes `position: fixed` top/left/width from `anchorEl.getBoundingClientRect()`, repositions (never closes) on capture-phase `scroll` (any ancestor container) and `resize`, flips upward when insufficient space below viewport bottom.
- Migrated `LocationAutocomplete.svelte`'s dropdown from `position: absolute` (wrapper-relative) to `use:portal` + `use:dropdownAnchor={{ anchorEl: inputEl }}`, rendering it as a direct child of `<body>`.
- `handleClickOutside` now checks both `wrapperEl` and the new `dropdownEl` ref (portal-move caveat: after the node leaves the wrapper's DOM subtree, `wrapperEl.contains()` alone no longer detects clicks on dropdown options).

## Task Commits

Each task was committed atomically:

1. **Task 1: Реализовать dropdownAnchor** - `73af1fe` (feat)
2. **Task 2: Мигрировать LocationAutocomplete на portal + dropdownAnchor** - `4f2151d` (feat)

**Plan metadata:** (this commit, docs: complete plan)

## Files Created/Modified
- `ui/src/lib/utils/dropdownAnchor.ts` - New reusable portal-anchoring use-action (AUTO-01/D-01/D-02)
- `ui/src/lib/components/LocationAutocomplete.svelte` - Dropdown migrated to portal+anchor; click-outside dual-ref check; CSS wrapped `:global()`; `--shadow-md` → `--shadow-elev-2`

## Decisions Made
- Wrapped `.dropdown-item` (and its `:hover`/`.active` states) in `:global()` in addition to `.dropdown` itself — the plan's acceptance criteria only named `.dropdown`, but the same portal-move rationale applies transitively to all descendants that were previously scoped, and this matches the established `DeviceContextMenu.svelte` precedent (which globalizes `.ctx-menu-item`/`.ctx-menu-sep` alongside `.ctx-menu-portal`). Verified via `svelte-check` (0 errors, no new "unused CSS selector" warnings for this file) and `pnpm build` (exit 0).
- Kept `z-index: 1000` per UI-SPEC AUTO-01 contract (below `DeviceContextMenu`'s `2000`, per its own comment about layering priority).

## Deviations from Plan

None - plan executed exactly as written. The `.dropdown-item` `:global()` wrap is a direct, same-rationale extension of the plan's explicit `.dropdown` `:global()` instruction, not a scope change (Rule 1/2 territory — without it, scoped CSS would risk not applying, or being pruned as an "unused selector", to option rows once portaled into `<body>`).

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- `dropdownAnchor.ts` is ready for reuse by Plan 18-03 (PersonAutocomplete, Select, CartridgeSelect, GroupedPrinterSelect, PrinterSelect, DeviceAutocompleteField) and Plan 18-04/18-05 (device picker in `ActFormItemsTable.svelte`), per the phase's Wave 0 contract.
- Visual/DOM-position confirmation (dropdown as `document.body.lastElementChild`, capture-scroll reposition, upward flip) is explicitly deferred to the phase's final checkpoint in Plan 18-05, per this plan's own `<verification>` section — not blocking here.
- No blockers.

---
*Phase: 18-autocomplete-dropdowns*
*Completed: 2026-07-10*

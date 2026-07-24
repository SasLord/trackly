---
phase: 30-quality-a11y-platform-parity
plan: 05
subsystem: ui
tags: [svelte, scss, accessibility, focus-ring, css-has]

# Dependency graph
requires:
  - phase: 30-quality-a11y-platform-parity
    provides: "check-focus-outline.mjs lint gate (30-01) and .tr-row-chevron focus ring precedent (30-02)"
provides:
  - "Row-wide keyboard focus ring in TableRow.svelte via `.tr-row:has(:focus-visible)`"
  - "4 consumer files (Acts/Cartridges/Printers/Requests) delegating to the shared row-level rule instead of duplicating cell-level box-shadow"
affects: [30-03-uat-checkpoint, future-table-row-consumers]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Row-wide focus ring via CSS :has(:focus-visible) — single primitive rule instead of per-consumer duplication"

key-files:
  created: []
  modified:
    - ui/src/lib/components/TableRow.svelte
    - ui/src/features/acts/ActListRow.svelte
    - ui/src/features/cartridges/CartridgeListRow.svelte
    - ui/src/features/printers/PrinterListRow.svelte
    - ui/src/features/requests/RequestListRow.svelte

key-decisions:
  - "Placed .tr-row:has(:focus-visible) as a sibling top-level rule right after .tr-row {...}, not nested with & — literal grep-matchable string required by Task 1 acceptance criteria"
  - "check-focus-outline: ignore marker must be the line IMMEDIATELY before outline: none; (single-line comment) — script only checks current+previous line, not any earlier line in a multi-line comment block"

patterns-established:
  - "check-focus-outline: ignore marker placement — must be the literal line directly preceding `outline: none;`, verified against ui/scripts/check-focus-outline.mjs isWhitelisted() logic (current-line OR previous-line only)"

requirements-completed: [QA-02]

# Metrics
duration: 12min
completed: 2026-07-25
---

# Phase 30 Plan 05: Row-Level Focus Ring Consolidation Summary

**Consolidated 4 duplicated cell-level focus-ring box-shadows into one shared `.tr-row:has(:focus-visible)` rule in TableRow.svelte, closing Gap 4 (focus ring drawn around first cell instead of the whole row, inconsistent across tables) without adding any new row interactivity.**

## Performance

- **Duration:** 12 min
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments
- Added one shared `.tr-row:has(:focus-visible)` rule to `TableRow.svelte` that fires from ANY focusable descendant (chevron, single-entry-point cell, kebab button), giving Devices/Users the same row-wide highlight as Acts/Cartridges/Printers/Requests without touching their `role`/`tabindex` model
- Removed the 4 duplicated cell-level `box-shadow: inset 0 0 0 2px var(--tr-accent)` rules from `ActListRow`/`CartridgeListRow`/`PrinterListRow`/`RequestListRow`, each whitelisted with a `check-focus-outline: ignore` marker
- `check-focus-outline.mjs`, `svelte-check`, `check-tokens.mjs`, full `pnpm --dir ui lint`, and `pnpm --dir ui build` all pass

## Task Commits

Each task was committed atomically:

1. **Task 1: TableRow.svelte — общее row-level кольцо через :has(:focus-visible)** - `02e29a7` (feat)
2. **Task 2: Консолидация 4 cell-level колец в общее правило** - `1f9acb6` (refactor)

_No plan-metadata commit yet — will follow per final_commit step._

## Files Created/Modified
- `ui/src/lib/components/TableRow.svelte` - added `.tr-row:has(:focus-visible) { box-shadow: inset 0 0 0 2px var(--tr-accent); }` immediately after the existing `.tr-row { &:hover{} &.selected{} }` block, before `.tr-row-group {`
- `ui/src/features/acts/ActListRow.svelte` - removed `.cell-number &:focus-visible` box-shadow, added ignore marker
- `ui/src/features/cartridges/CartridgeListRow.svelte` - removed `.cell-code &:focus-visible` box-shadow, added ignore marker
- `ui/src/features/printers/PrinterListRow.svelte` - removed `.cell-name &:focus-visible` box-shadow, added ignore marker
- `ui/src/features/requests/RequestListRow.svelte` - removed `.cell-author &:focus-visible` box-shadow, added ignore marker; replaced the stale "moved from .cell-type" comment with the current explanation

## Decisions Made
- Kept `.tr-row:has(:focus-visible)` as a standalone top-level selector (not nested via `&` inside `.tr-row {}`) so the literal source string matches the plan's grep-based acceptance criterion exactly.
- Discovered during Task 2 that `check-focus-outline.mjs`'s `isWhitelisted()` only inspects the current line and the single line immediately before a matched `outline: none;` — a two-line wrapped comment with the marker on the first line failed the gate. Reformatted all 4 markers to a single-line comment placed directly above `outline: none;` (Rule 1 auto-fix, self-discovered before it ever reached a failing gate run, so no separate fix-commit was needed — folded into the Task 2 commit).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Ignore-marker comment initially spanned 2 lines, would have failed check-focus-outline.mjs**
- **Found during:** Task 2 (before running verification)
- **Issue:** Plan's illustrative snippet wrapped the marker explanation across two `//` comment lines with `check-focus-outline: ignore` on the first of the two — but `check-focus-outline.mjs` (`isWhitelisted()`) only checks the current line and the ONE line immediately preceding `outline: none;`, so the marker text would not have been seen on the correct line.
- **Fix:** Reformatted all 4 files' markers to a single-line comment (`// ring теперь на уровне строки, см. TableRow.svelte .tr-row:has(:focus-visible) (Gap 4, план 30-05)` then a separate `// check-focus-outline: ignore` line directly above `outline: none;`).
- **Files modified:** ui/src/features/acts/ActListRow.svelte, ui/src/features/cartridges/CartridgeListRow.svelte, ui/src/features/printers/PrinterListRow.svelte, ui/src/features/requests/RequestListRow.svelte
- **Verification:** `node ui/scripts/check-focus-outline.mjs` → `PASS — 0 нарушений`
- **Committed in:** 1f9acb6 (Task 2 commit — caught before the first verification run, so only one commit was needed)

---

**Total deviations:** 1 auto-fixed (1 bug, caught pre-verification)
**Impact on plan:** No scope creep — purely a formatting correction to satisfy the exact whitelist mechanism specified in the plan's `<interfaces>` section.

## Issues Encountered
None beyond the marker-placement deviation above.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Gap 4 (30-VERIFICATION.md) is closed at the code level for all 6 tables (Acts/Cartridges/Printers/Requests via delegation, Devices/Users via the shared rule alone).
- Live visual re-confirmation (ring visibly wraps the whole row, incl. Devices chevron/kebab focus) remains part of the already-open blocking UAT checkpoint from 30-03 Task 3 — this plan does not open a new UAT gate, per its `<verification>` section.

---
*Phase: 30-quality-a11y-platform-parity*
*Completed: 2026-07-25*

## Self-Check: PASSED
All 5 modified files and the SUMMARY.md exist on disk; both task commits (02e29a7, 1f9acb6) verified present in git log.

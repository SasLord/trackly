---
phase: 30-quality-a11y-platform-parity
plan: 06
subsystem: ui
tags: [svelte, scss, a11y, focus-ring, flexbox, dashboard]

# Dependency graph
requires:
  - phase: 30-quality-a11y-platform-parity
    provides: "inset focus-ring idiom established in 30-02 (TableRow chevron, ModelListRow kebab) and min-height:0 scroll-isolation precedent in ActsPage.svelte / Phase 29 EmployeeLayout"
provides:
  - "Non-clipped inset focus ring on PeriodToggle.svelte's .toggle-btn"
  - "Scroll isolation on DashboardPage.svelte's .dashboard-grid (sidebar/header no longer scroll with content)"
affects: [30-03 human-UAT re-verification, dashboard, a11y]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "inset focus-ring idiom (box-shadow: inset 0 0 0 2px var(--tr-accent)) applied uniformly regardless of parent overflow context"
    - "min-height: 0 on flex:1 scrolling children to defeat automatic minimum size / content-based flex-basis trap"

key-files:
  created: []
  modified:
    - ui/src/features/dashboard/PeriodToggle.svelte
    - ui/src/features/dashboard/DashboardPage.svelte

key-decisions:
  - "Kept .period-toggle's overflow-x: auto untouched — the inset-ring idiom makes the implicit overflow-y:auto clip irrelevant without needing to remove the horizontal scroll affordance for narrow viewports"

patterns-established: []

requirements-completed: [QA-02, QA-03]

# Metrics
duration: 8min
completed: 2026-07-25
---

# Phase 30 Plan 06: Dashboard focus-ring + scroll-isolation gap closure Summary

**Fixed clipped focus ring on PeriodToggle (inset idiom) and app-shell-wide scroll bleed on DashboardPage (min-height:0) — two independent Gap 1/Gap 2 defects on the same screen.**

## Performance

- **Duration:** 8 min
- **Started:** 2026-07-24T18:47:00Z (approx)
- **Completed:** 2026-07-25T01:47:08+07:00
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- Gap 1: `.toggle-btn:focus-visible` on the dashboard's period toggle (3/6/12 мес.) now uses the inset ring idiom (`inset 0 0 0 2px var(--tr-accent)`) instead of the outward `box-shadow` that was silently clipped by `.period-toggle`'s implicit `overflow-y: auto` (a side effect of `overflow-x: auto` per CSS 2.1 §11.1.1)
- Gap 2: `.dashboard-grid` now has `min-height: 0`, so the flex item can shrink below its content's automatic minimum size — `overflow: auto` on `.dashboard-grid` now actually isolates scrolling to the content area instead of the whole app-shell (sidebar + header) scrolling along with it

## Task Commits

Each task was committed atomically:

1. **Task 1: PeriodToggle.svelte — inset-кольцо на .toggle-btn (Gap 1)** - `4e8e103` (fix)
2. **Task 2: DashboardPage.svelte — min-height:0 на .dashboard-grid (Gap 2)** - `97e90e4` (fix)

**Plan metadata:** (this commit)

_Note: no TDD tasks in this plan (pure CSS fixes, tdd="false")._

## Files Created/Modified
- `ui/src/features/dashboard/PeriodToggle.svelte` - `.toggle-btn:focus-visible` switched from outward `box-shadow: 0 0 0 3px var(--tr-focus-ring)` to inset `box-shadow: inset 0 0 0 2px var(--tr-accent)`
- `ui/src/features/dashboard/DashboardPage.svelte` - `.dashboard-grid` gained `min-height: 0;` between `flex: 1;` and `overflow: auto;`

## Decisions Made
- Did not touch `.period-toggle`'s `overflow-x: auto` — it remains a legitimate affordance for narrow viewports; the inset-ring idiom sidesteps the clip without removing that scroll behavior.
- Did not touch `Layout.svelte`'s `.content` (already correct: `overflow: auto; min-height: 0;`) — Gap 2's root cause was isolated entirely to `DashboardPage.svelte`'s own `.dashboard-grid`, confirmed at planning time and re-confirmed by inspection before editing.

## Deviations from Plan

None - plan executed exactly as written. Both fixes were single-line/single-value CSS edits, no additional flex-chain elements (`.stat-row`, `ChartWidget`, `StatWidget`) needed `min-height: 0` — the single `.dashboard-grid` fix was sufficient.

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Both gaps closed; `node ui/scripts/check-focus-outline.mjs`, `pnpm --dir ui svelte-check`, `pnpm --dir ui lint`, and `pnpm --dir ui build` all pass (exit 0, 0 errors — pre-existing warnings in unrelated files untouched).
- Visual/functional confirmation (ring not clipped; scroll isolated on narrow viewport / chart-heavy data) remains part of the already-open blocking UAT checkpoint from 30-03 Task 3 — this plan does not introduce a new UAT gate, it feeds into the existing re-run.

---
*Phase: 30-quality-a11y-platform-parity*
*Completed: 2026-07-25*

## Self-Check: PASSED

All created/modified files and commit hashes verified present in working tree and git history.

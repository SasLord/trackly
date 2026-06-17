---
phase: "07-reports-dashboard-settings"
plan: "10"
subsystem: "reports-ui"
tags: ["gap-closure", "reports", "export", "layout", "svelte"]
dependency_graph:
  requires: []
  provides: ["reports-export-fixed", "reports-layout-desktop", "reports-filter-clean"]
  affects: ["ReportsPage.svelte", "ReportSubNav.svelte", "ReportFilters.svelte", "PeriodSelector.svelte"]
tech_stack:
  added: []
  patterns:
    - "reportTypeKey() helper mapping domain+report key to backend API key"
    - "GAP-R2: flex-direction row layout for dual switch-bars on desktop"
    - "GAP-R5: Badge on all tabs, active shows count, inactive shows dash placeholder"
    - "GAP-R4: Props interface retained for parent compat, unused props prefixed with _ in destructure"
key_files:
  created: []
  modified:
    - ui/src/features/reports/ReportsPage.svelte
    - ui/src/features/reports/ReportSubNav.svelte
    - ui/src/features/reports/ReportFilters.svelte
    - ui/src/features/reports/PeriodSelector.svelte
decisions:
  - "GAP-R5: inactive tabs show '–' badge (variant=default) rather than no badge — user confirmed all tabs must show badge slot"
  - "GAP-R4: filter props kept in ReportFilters.svelte Props interface (prefixed with _ in destructure) rather than removing them, to avoid requiring parent changes"
  - "GAP-R3: override DatePicker height via :global(.date-picker) inside .range-label to constrain 36px → 28px without touching shared component"
  - "GAP-R1: use camelCase reportType in export calls to match specta-generated binding pattern"
metrics:
  duration_seconds: 176
  completed_date: "2026-06-17"
  tasks_completed: 2
  files_modified: 4
---

# Phase 07 Plan 10: Reports UI/UX Gap Closure Summary

Five frontend-only fixes to the Reports page: export broken by wrong `report_type` arg, layout issues (two nav rows instead of one, date input sizing), redundant filters in the filter row, and count badges only on the active tab.

## Tasks Completed

| # | Task | Commit | Files |
|---|------|--------|-------|
| 1 | GAP-R1: Fix export report_type key | 99d24df | ReportsPage.svelte |
| 2 | GAP-R2/R3/R4/R5: Layout + filter cleanup | 8ef2287 | ReportSubNav.svelte, ReportFilters.svelte, PeriodSelector.svelte, ReportsPage.svelte |

## What Was Built

**GAP-R1 (Export fix):** Added `reportTypeKey()` helper that maps `activeDomain + activeReport` → the correct backend key (`device_acts`, `cartridge_consumption`, etc.). All three export functions (`exportCsv`, `exportPdf`, `printReport`) now pass `reportType: reportTypeKey()` (camelCase, matching specta-generated binding). Period is now correctly `undefined` for snapshot reports in export calls.

**GAP-R2 (Layout — dual switch-bars in one row):** `ReportSubNav.svelte` changed from `flex-direction: column` to `flex-direction: row` with `flex-wrap: wrap`. Domain nav (Устройства/Картриджи) sits left (`flex-shrink: 0`); report type nav sits right (`flex: 1; justify-content: flex-end`). Removed the inner `border-bottom` on `.domain-nav` — the outer `.report-sub-nav` owns the single bottom border.

**GAP-R3 (Date input sizing):** `PeriodSelector.svelte` `.period-range` changed from `align-items: flex-start` to `align-items: center`. Added `:global(.date-picker)` override inside `.range-label` to constrain `DatePicker` from 36px → 28px, matching other filter controls.

**GAP-R4 (Remove redundant filters + move period selector):** `ReportFilters.svelte` strips the entire `{#if reportDomain === 'devices'} ... {:else} ... {/if}` block and the `.search-wrap` / `<Input>` search field. Export/print buttons are the only remaining rendered content. Props interface kept intact (unused props prefixed `_` in destructure to avoid svelte-check warnings). In `ReportsPage.svelte`, `<PeriodSelector>` and `<ReportFilters>` are now co-located inside a `<div class="controls-row">` flex container — period selector on the left, export buttons on the right.

**GAP-R5 (Badges on all tabs):** `ReportSubNav.svelte` now always renders a `<Badge>` on every report tab. Active tab: `variant="accent"` showing `{rowCount}`. Inactive tabs: `variant="default"` showing `–`. This satisfies the user decision: all tabs visually acknowledge the badge slot simultaneously.

## Deviations from Plan

None — plan executed exactly as written.

## Verification

- `pnpm svelte-check`: 0 errors, 36 warnings (all pre-existing) ✓
- `grep -c 'reportTypeKey' ReportsPage.svelte`: 4 (function def + 3 call sites) ✓
- `grep -c 'flex-direction: row' ReportSubNav.svelte`: 1 ✓
- `grep -c 'Локация|search-wrap|Модель|Поиск' ReportFilters.svelte`: 0 ✓
- `grep -c 'Badge' ReportSubNav.svelte`: 3 (import + active + inactive) ✓

## Known Stubs

None — all export, layout, and badge changes are fully wired.

## Threat Flags

None — no new network endpoints, auth paths, or schema changes introduced. All changes are frontend-only presentation and argument mapping.

## Self-Check: PASSED

- `ui/src/features/reports/ReportsPage.svelte` exists ✓
- `ui/src/features/reports/ReportSubNav.svelte` exists ✓
- `ui/src/features/reports/ReportFilters.svelte` exists ✓
- `ui/src/features/reports/PeriodSelector.svelte` exists ✓
- Commit 99d24df (Task 1) exists ✓
- Commit 8ef2287 (Task 2) exists ✓

---
phase: 07-reports-dashboard-settings
plan: "05"
subsystem: frontend-dashboard
tags: [svelte5, dashboard, svg-chart, accessibility, widgets, no-deps]

# Dependency graph
requires:
  - phase: 07-03
    provides: DashboardService (get_all_widgets + get_consumption_chart)
  - phase: 07-04
    provides: Tauri commands dashboard_get_all_widgets, dashboard_get_consumption_chart
provides:
  - ui/src/features/dashboard/DashboardPage.svelte (5-widget responsive grid, parallel loading)
  - ui/src/features/dashboard/StatWidget.svelte (generic stat card)
  - ui/src/features/dashboard/ChartWidget.svelte (SVG polyline chart, zero npm deps)
  - ui/src/features/dashboard/PeriodToggle.svelte (3/6/12 month switcher)
  - ui/src/pages/Dashboard.svelte (updated: replaces Placeholder with DashboardPage)
affects:
  - Dashboard route '/' now shows real data instead of placeholder (D-09)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Hand-drawn SVG polyline chart: toPoints(series, maxVal) maps numeric data to viewBox 0 0 400 200 coordinates"
    - "IIFE $derived pattern: $derived((() => { ... })()) for complex derivations returning arrays/objects"
    - "Parallel widget loading: two independent apiCall chains with .then/.catch — no Promise.all — gives per-widget error isolation"
    - "sr-only table as accessibility fallback for SVG chart (role=img + aria-label pattern)"
    - "COLORS array with CSS custom properties for multi-model chart series"

key-files:
  created:
    - ui/src/features/dashboard/StatWidget.svelte
    - ui/src/features/dashboard/PeriodToggle.svelte
    - ui/src/features/dashboard/ChartWidget.svelte
    - ui/src/features/dashboard/DashboardPage.svelte
  modified:
    - ui/src/pages/Dashboard.svelte

key-decisions:
  - "IIFE $derived pattern used for complex array/object derivations — avoids TypeScript error 'not callable' when $derived receives a lambda instead of an expression"
  - "windowMonths change triggers loadChart via $effect tracking; mounted flag prevents double-load on first render"
  - "snake_case field names in DashboardWidgetDto/ConsumptionPoint TypeScript interfaces — matches Phase 7 snake_case JSON decision (07-01)"
  - "Dashboard period selector (month/year) directly calls reloadWidgets on <select onchange> — matches existing UI onchange patterns"

# Metrics
duration: 4min
completed: 2026-06-16
---

# Phase 7 Plan 05: Dashboard UI Summary

**DashboardPage with 5 independent widgets in a responsive 2-column grid built on top of DashboardService; hand-drawn SVG polyline chart requires zero new npm dependencies**

## Performance

- **Duration:** ~4 min
- **Started:** 2026-06-16T12:18:50Z
- **Completed:** 2026-06-16T12:23:00Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments

- `StatWidget.svelte` — generic stat card with mainNumber/mainLabel/breakdown list and optional low-stock warning block (DASH-01..02 visual pattern, `--color-warning` tint border per UI-SPEC)
- `PeriodToggle.svelte` — compact 3/6/12 month button group following CartridgeFilters status-bar tab pattern (border-bottom: 2px solid --color-accent when active)
- `ChartWidget.svelte` — hand-drawn SVG polyline, zero new npm deps; supports up to 3 model series with COLORS array (`--color-accent`, `--color-success`, `--color-warning`); `role="img"` aria-label + `sr-only` data table for accessibility (DASH-03, D-11, UI-SPEC §Accessibility)
- `DashboardPage.svelte` — 5-widget fixed 2-column responsive grid (`grid-template-columns: 3fr 2fr`, single-column below 1280px); parallel `apiCall` for widgets + chart (independent loading states, D-10); period selector (month/year selects) for period-sensitive widgets (D-12)
- `Dashboard.svelte` — stub updated: `<Placeholder>` replaced with `<DashboardPage />` (D-09)
- All text in Russian (v1 constraint); Svelte 5 runes throughout (10 `$state` usages in DashboardPage)

## Task Commits

1. **Task 1: StatWidget + PeriodToggle + ChartWidget components** — `2e7fc7a`
2. **Task 2: DashboardPage assembly + Dashboard.svelte stub update** — `a2a7197`

## Files Created/Modified

- `ui/src/features/dashboard/StatWidget.svelte` — generic stat card (number + label + breakdown + warning)
- `ui/src/features/dashboard/PeriodToggle.svelte` — 3/6/12 month toggle buttons
- `ui/src/features/dashboard/ChartWidget.svelte` — SVG polyline chart; sr-only accessibility table
- `ui/src/features/dashboard/DashboardPage.svelte` — 5-widget page with parallel loading
- `ui/src/pages/Dashboard.svelte` — replaced Placeholder with DashboardPage

## Decisions Made

- IIFE `$derived` pattern for complex derivations: `$derived((() => { ... })())` — avoids TypeScript error where `$derived` receives a lambda expression instead of a reactive value
- `mounted` flag in `$effect` prevents double API call on initial render when `windowMonths` effect also fires
- snake_case field names in TypeScript interfaces match Phase 7 DTO decision (07-01: snake_case JSON)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `$derived` with lambda functions produces TypeScript errors**
- **Found during:** Task 1 verification (svelte-check run)
- **Issue:** Plan specified `const uniqueMonths = $derived(() => { ... })` but in Svelte 5 this captures a function reference, not the derived value. TypeScript error: "not callable — Type 'Record<string, number[]>' has no call signatures"
- **Fix:** Changed to IIFE pattern: `$derived((() => { ... })())` — the IIFE executes immediately inside $derived, producing the actual value
- **Files modified:** `ChartWidget.svelte`
- **Commit:** `2e7fc7a` (fixed before first commit)

## Known Stubs

None — DashboardPage calls real Tauri commands `dashboard_get_all_widgets` and `dashboard_get_consumption_chart` that are backed by DashboardService (plan 07-03). No hardcoded mock data.

## Threat Flags

None — no new network endpoints, auth paths, file access patterns, or schema changes. All SVG content is computed from numeric data (T-07-05-01 mitigated: no unescaped string interpolation into SVG).

---
*Phase: 07-reports-dashboard-settings*
*Completed: 2026-06-16*

## Self-Check: PASSED

- [x] ui/src/features/dashboard/StatWidget.svelte exists — FOUND
- [x] ui/src/features/dashboard/PeriodToggle.svelte exists — FOUND
- [x] ui/src/features/dashboard/ChartWidget.svelte exists — FOUND
- [x] ui/src/features/dashboard/DashboardPage.svelte exists — FOUND
- [x] ui/src/pages/Dashboard.svelte updated — FOUND (imports DashboardPage, 3 matches)
- [x] Commit 2e7fc7a exists — FOUND (Task 1)
- [x] Commit a2a7197 exists — FOUND (Task 2)
- [x] svelte-check 0 errors — VERIFIED (226 files, 0 errors)
- [x] role="img" in ChartWidget — 1 match
- [x] sr-only in ChartWidget — 2 matches
- [x] No chart.js/recharts/d3/apexcharts imports — 0 matches
- [x] dashboard_get_all_widgets in DashboardPage — 2 matches
- [x] dashboard_get_consumption_chart in DashboardPage — 2 matches
- [x] $state count >= 5 in DashboardPage — 10 matches

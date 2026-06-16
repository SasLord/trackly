---
phase: 07-reports-dashboard-settings
plan: "06"
subsystem: ui-reports
tags: [svelte5, reports, period-selector, csv-export, pdf-export, two-level-nav]

# Dependency graph
requires:
  - phase: 07-03
    provides: ReportService (8 queries + CSV + PDF), ReportFilter, ReportRow, ReportResponse, PeriodDto DTOs
  - phase: 07-01
    provides: ReportFilter, ReportRow, ReportResponse, PeriodDto (dto/reports.rs)
provides:
  - ui/src/features/reports/ReportsPage.svelte (orchestrator)
  - ui/src/features/reports/ReportSubNav.svelte (two-level navigation)
  - ui/src/features/reports/PeriodSelector.svelte (period selector with snapshot guard)
  - ui/src/features/reports/ReportFilters.svelte (contextual filters + export buttons)
  - ui/src/features/reports/ReportTable.svelte (universal table with month-separator rows)
  - ui/src/pages/ReportsPage.svelte (route page stub replaced)
affects:
  - Reports vertical slice complete: admin can browse all 8 report types

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Two-level report nav: domain sub-nav (Устройства/Картриджи) + report type switch-bar with $derived(activeDomain)"
    - "isSnapshot() helper gates PeriodSelector and changes separator grouping key in ReportTable"
    - "COLUMNS_MAP keyed by report type; cartridge in_use/in_stock use 'cartridge_' prefix to distinguish from device variants"
    - "onMount: locations_autocomplete (string[]) + cartridge_models_list (CartridgeModelDto[]) loaded once for filter dropdowns"
    - "Cartridge colors derived from cartridge_models_list — distinct non-null .color values"
    - "Export: Blob + URL.createObjectURL for browser; Tauri-plugin-fs writeFile + shell open for desktop"
    - "T-07-06-03: PeriodSelector date range validates start <= end via $effect, blocks onPeriodChange on invalid"

key-files:
  created:
    - ui/src/features/reports/ReportsPage.svelte
    - ui/src/features/reports/ReportSubNav.svelte
    - ui/src/features/reports/ReportFilters.svelte
    - ui/src/features/reports/PeriodSelector.svelte
    - ui/src/features/reports/ReportTable.svelte
  modified:
    - ui/src/pages/ReportsPage.svelte

key-decisions:
  - "location filter emits location_name (string) not location_id — locations_autocomplete returns string[], not {id,name} pairs"
  - "COLUMNS_MAP uses 'cartridge_in_use'/'cartridge_in_stock' keys to differentiate column sets for cartridge vs device snapshot reports"
  - "filterDeviceTypes and filterCartridgeStatuses are static inline (2 and 4 items) — no Tauri commands exist for these"
  - "filter reset on domain switch (activeDomain change sets filter = {}) to avoid cross-domain filter bleed"

# Metrics
duration: 15min
completed: 2026-06-16
---

# Phase 7 Plan 06: Reports UI Summary

**5 Svelte 5 components implement the full Reports page: two-level domain/report navigation, Месяц/Год/Диапазон period selector with snapshot guard, contextual filter row with export buttons, and a universal table with month-separator rows across all 8 report types**

## Performance

- **Duration:** ~15 min
- **Completed:** 2026-06-16
- **Tasks:** 2
- **Files modified:** 6

## Accomplishments

- `ReportSubNav.svelte`: domain sub-nav (Устройства/Картриджи) + report type switch-bar; Badge on active tab with rowCount; switching domain auto-resets to first report key
- `PeriodSelector.svelte`: Месяц/Год/Диапазон button group; snapshot reports render controls as disabled with helper text "Отчёт отражает текущее состояние"; date range validates start ≤ end via `$effect` (T-07-06-03)
- `ReportFilters.svelte`: device filters (локация/тип/статус) vs cartridge filters (модель/статус/цвет) switchable by `reportDomain`; search Input; right-aligned export buttons (CSV, PDF, Печать with print SVG icon)
- `ReportTable.svelte`: month-separator rows for temporal reports; location-separator rows for snapshot reports; loading/empty/error states; `formatMonthKey("2026-09")` → "Сентябрь 2026"; sticky `<thead>` with `scope="col"`; `aria-hidden="true"` on separator rows
- `ReportsPage.svelte` (features): full orchestrator; `onMount` loads `locations_autocomplete` + `cartridge_models_list`; `$effect` auto-reloads on domain/report/period/filter changes; `exportCsv`/`exportPdf`/`printReport` with Tauri-fs + shell-open + browser blob fallback
- `pages/ReportsPage.svelte`: Placeholder stub replaced with `<ReportsPage />`

## Task Commits

1. **Task 1: PeriodSelector + ReportFilters + ReportSubNav + ReportTable** — `7a7c84c` (prior session)
2. **Task 2: ReportsPage orchestrator + ReportsPage.svelte route update** — `345ebe7`

## Files Created/Modified

- `ui/src/features/reports/ReportSubNav.svelte` — two-level domain + report type navigation
- `ui/src/features/reports/PeriodSelector.svelte` — period mode selector with snapshot disable guard
- `ui/src/features/reports/ReportFilters.svelte` — contextual filter dropdowns + export buttons
- `ui/src/features/reports/ReportTable.svelte` — universal table with month/location separator rows
- `ui/src/features/reports/ReportsPage.svelte` — orchestrator; loads filter data; calls backend commands
- `ui/src/pages/ReportsPage.svelte` — route page delegating to features component

## Decisions Made

- `location_name` (string) used in filter instead of `location_id` — `locations_autocomplete` returns `string[]`, not ID-keyed pairs; backend resolves name → id
- `COLUMNS_MAP` prefixes cartridge snapshot variants (`cartridge_in_use`, `cartridge_in_stock`) to distinguish column sets from device equivalents
- Static inline lists for `filterDeviceTypes` (2 items) and `filterCartridgeStatuses` (4 items) — no backend commands exist for these enums
- Filter state reset on domain switch to avoid cross-domain filter values bleeding into new domain queries

## Deviations from Plan

None — plan executed exactly as written. All 5 components created per spec; svelte-check exits with 0 errors.

## Known Stubs

None — all filter dropdowns wired to real backend commands (`locations_autocomplete`, `cartridge_models_list`); all export functions call real backend commands (`reports_export_csv`, `reports_export_pdf`). Report data loads from verified Tauri commands per domain/report key.

## Threat Flags

None — no new network endpoints or auth paths. Filter values passed to backend as structured params; no client-side SQL construction.

---
*Phase: 07-reports-dashboard-settings*
*Completed: 2026-06-16*

## Self-Check: PASSED

- [x] ui/src/features/reports/ReportsPage.svelte — FOUND
- [x] ui/src/features/reports/ReportSubNav.svelte — FOUND
- [x] ui/src/features/reports/PeriodSelector.svelte — FOUND
- [x] ui/src/features/reports/ReportFilters.svelte — FOUND
- [x] ui/src/features/reports/ReportTable.svelte — FOUND
- [x] Commit 7a7c84c exists (Task 1) — FOUND
- [x] Commit 345ebe7 exists (Task 2) — FOUND
- [x] svelte-check exits 0 errors — VERIFIED
- [x] grep month-separator ReportTable.svelte >= 1 — 3 matches
- [x] grep Месяц/Год/Диапазон PeriodSelector.svelte >= 1 — 4 matches
- [x] grep reports_list_device_acts ReportsPage.svelte >= 1 — 2 matches
- [x] grep reports_export_csv ReportsPage.svelte >= 1 — 1 match

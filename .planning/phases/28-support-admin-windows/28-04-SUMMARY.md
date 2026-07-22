---
phase: 28-support-admin-windows
plan: 04
subsystem: ui
tags: [svelte, table, reports, design-system]

# Dependency graph
requires:
  - phase: 25-dropdown
    provides: "Table.svelte (dynamic head/children snippets, loading/empty states, fillHeight mode) and TableRow.svelte (row-state primitive, group-collapse mode)"
  - phase: 26-tables-with-layout
    provides: "PageHeader.svelte primitive (title + optional actions snippet)"
provides:
  - "ReportTable.svelte on Table/TableRow with dynamic Column[] rendering, no primitive extension"
  - "ReportsPage.svelte header on PageHeader primitive"
affects: [28-support-admin-windows, reports]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Dynamic-column tables: Column[] rendered directly via Table's head/children Snippet props — no primitive change needed for variable column sets"
    - "Static (non-collapsible) group-separator rows render as a bare <tr> inside Table's children, NOT via TableRow group={true} (that mode is a collapse contract requiring groupExpanded/onToggleGroup)"
    - "Table used with framed={false} + fillHeight inside a flex-1 wrapper div to replicate a bespoke sticky-header/internal-scroll table without introducing a new border/shadow frame (ActsList.svelte precedent)"

key-files:
  created: []
  modified:
    - ui/src/features/reports/ReportTable.svelte
    - ui/src/features/reports/ReportsPage.svelte

key-decisions:
  - "ReportTable's error state has no Table API equivalent (only loading/empty) — kept as a sibling {#if error}/{:else}<Table>{/if} branch outside Table, same pattern already used for RequestDetail/ActDetail loading branches"
  - "ReportFilters.svelte required zero code changes — GAP-R4 had already removed all filter fields, leaving it fully on Button; audit confirmed no residual bespoke classes"

patterns-established:
  - "D-07 closed: dynamic-column report tables map to Table/TableRow without extending either primitive"

requirements-completed: [WIN-07]

# Metrics
duration: 4min
completed: 2026-07-22
---

# Phase 28 Plan 04: Reports Table on Table/TableRow + PageHeader Summary

**ReportTable rebuilt on shared Table/TableRow primitives with dynamic per-report-type columns and a static (non-group) separator row; ReportsPage header moved to PageHeader — closes D-07 for the Reports window (WIN-07).**

## Performance

- **Duration:** 4 min
- **Started:** 2026-07-22T13:06:16+07:00 (prior plan's final commit)
- **Completed:** 2026-07-22T13:10:21+07:00
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- `ReportTable.svelte`: bespoke `<table>`/`thead`/`tbody` markup replaced with `Table`/`TableRow`. Column headers render via a `tableHead` snippet iterating the caller-supplied `Column[]` — no change to `Table`'s API was needed since `head`/`children` are plain `Snippet`s. The month/location separator renders as a bare `<tr class="report-separator">` inside `Table`'s `children`, deliberately NOT using `TableRow`'s `group={true}` mode (that mode is a collapse contract with `groupExpanded`/`onToggleGroup`; this separator is static). `.report-separator td` styling copied verbatim from the old `.month-separator td` rule. Loading/empty states now come from `Table`'s built-in skeleton/empty API (`emptyTitle`/`emptyBody` text preserved verbatim); the error branch (no Table equivalent) stays a sibling `{#if error}…{:else}<Table>…{/if}` outside `Table`, with the same "Не удалось загрузить отчёт. Попробуйте ещё раз." text. `Table` used with `framed={false} fillHeight` (ActsList.svelte precedent) to reproduce the original sticky-header/internal-scroll behavior without adding a visible border/shadow that wasn't in the original design.
- `ReportFilters.svelte`: audited per plan — already fully on `Button` since GAP-R4 removed all filter fields; zero code changes required.
- `ReportsPage.svelte`: bespoke `<header class="page-header"><h1 class="page-title">Отчёты</h1></header>` replaced with `<PageHeader title="Отчёты" />` (no `actions` snippet — export/print buttons live in `ReportFilters`, not the page header). Scoped `.page-header`/`.page-title` CSS removed.

## Task Commits

Each task was committed atomically:

1. **Task 1: ReportTable → Table/TableRow, dynamic columns + bare tr-separator (D-07)** - `9f7b481` (feat)
2. **Task 2: ReportFilters audit (D-04) + ReportsPage → PageHeader** - `dab32db` (feat)

**Plan metadata:** (this commit, docs: complete plan)

## Files Created/Modified
- `ui/src/features/reports/ReportTable.svelte` - dynamic-column report table on Table/TableRow, static tr-separator, verbatim copy/text preserved
- `ui/src/features/reports/ReportsPage.svelte` - page header on PageHeader primitive, bespoke header CSS removed

## Decisions Made
- **Error-state handling:** `Table`'s state API covers only `loading`/`empty`; error kept as a sibling branch outside `Table`, mirroring the existing `RequestDetail`/`ActDetail` loading-branch pattern from `28-PATTERNS.md` D-01.
- **`framed={false} fillHeight`:** not spelled out verbatim in the plan's `<action>` snippet but explicitly named as the applicable `Table` mode in `28-PATTERNS.md`'s Shared Patterns section (D-03/D-07) and directly precedented by `ActsList.svelte`. Chosen to keep the sticky-header + internal-scroll behavior byte-equivalent to the pre-migration table without introducing an unplanned bordered/shadowed frame (SC #4 — no visual regression).

## Deviations from Plan

None - plan executed exactly as written. `framed={false} fillHeight` is an application of the pattern file's own Shared Patterns guidance (Table.svelte source citation, D-03/D-07), not a deviation from the plan's intent.

## Issues Encountered

One acceptance-criteria near-miss: an inline code comment in `ReportTable.svelte` originally contained the literal string `group={true}` while explaining why the separator does NOT use it, which made `grep -c 'group={true}'` return 1 instead of the required 0. Reworded the comment to avoid the literal match before verification; no functional change.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

D-07 for Reports (WIN-07) is closed: `ReportTable` and `ReportsPage` are on the shared `Table`/`TableRow`/`PageHeader` primitives, no bespoke `<table>` or page-header markup remains. Automated verification (`check-tokens.mjs`, `svelte-check`, `pnpm build`) passed with 0 errors on both tasks; the plan's `<human-check>` visual verification (both themes, all 8 report types) is deferred to end-of-phase UAT per `human_verify_mode: "end-of-phase"` — consistent with how plan 28-03 handled its own human-check step. No blockers for remaining Phase 28 plans (Settings, Users windows).

---
*Phase: 28-support-admin-windows*
*Completed: 2026-07-22*

## Self-Check: PASSED

- FOUND: ui/src/features/reports/ReportTable.svelte
- FOUND: ui/src/features/reports/ReportsPage.svelte
- FOUND: .planning/phases/28-support-admin-windows/28-04-SUMMARY.md
- FOUND: 9f7b481 (Task 1 commit)
- FOUND: dab32db (Task 2 commit)
- FOUND: e8a2a73 (plan metadata commit)

---
phase: 28-support-admin-windows
plan: 03
subsystem: ui
tags: [svelte, tabs, select, reports, design-system]

# Dependency graph
requires:
  - phase: 24-base-components
    provides: "Tabs.svelte (segmented/underline variants, count slot, per-tab disabled) and Select.svelte (string value + onchange primitives)"
provides:
  - "ReportSubNav.svelte on two Tabs instances (segmented domain switch + underline report-type switch with count)"
  - "PeriodSelector.svelte on Tabs segmented (period mode) + Select (month/year)"
affects: [28-support-admin-windows, reports, settings-sub-nav]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Two-level sub-nav = two adjacent Tabs instances (segmented + underline), no primitive fork"
    - "Select used one-way (value=... + onchange=(v)=>...) without bind:, matching OperationModal/RequestDetail precedent"
    - "Select defaults to 100%/36px form-field sizing — constrain via :global(.select-wrapper)/:global(.select) scoped override when used inline in a filter row (same treatment DatePicker already uses via :global(.date-picker))"

key-files:
  created: []
  modified:
    - ui/src/features/reports/ReportSubNav.svelte
    - ui/src/features/reports/PeriodSelector.svelte

key-decisions:
  - "ReportSubNav count fallback for missing statusCounts changed from string '–' to number 0 (Tabs.count is typed number) — accepted minor edge-case, statusCounts is present in practice"
  - "PeriodSelector's onMonthChange/onYearChange changed signature from Event to string (native <select> removed) — same period-recalculation logic, just adapted input source, not a rewrite"

requirements-completed: [WIN-07]

# Metrics
duration: 4min
completed: 2026-07-22
---

# Phase 28 Plan 03: Reports Sub-Nav on Tabs Primitive Summary

**ReportSubNav (domain segmented + report-type underline/count) and PeriodSelector (mode segmented + month/year Select) migrated off bespoke button/select markup onto the shared Tabs/Select primitives, closing D-06 for the Reports window.**

## Performance

- **Duration:** 4 min
- **Started:** 2026-07-22T12:57:47+07:00 (prior plan's final commit)
- **Completed:** 2026-07-22T13:01:33+07:00
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- `ReportSubNav.svelte`: two `Tabs` instances replace the bespoke `.domain-nav`/`.report-nav`/`.tab` markup — `variant="segmented"` for the domain switch (Устройства/Картриджи), `variant="underline"` with `count` for the report-type switch. `Badge` import removed entirely — `Tabs`'s built-in count slot already renders the same accent-toned active count.
- `PeriodSelector.svelte`: `Tabs variant="segmented"` replaces `.period-buttons`/`.period-btn`, with per-tab `disabled={isSnapshot}` (no custom guard logic needed — `Tabs` already skips `onchange` for disabled tabs). Month/year `<select class="period-select">` replaced with the `Select` primitive in both `month` and `year` branches.
- Both files' props/consumer contracts (`ReportsPage.svelte` usage) are unchanged — no caller edits needed.

## Task Commits

Each task was committed atomically:

1. **Task 1: ReportSubNav → двойной Tabs (домен segmented + тип underline+count)** - `a30f126` (feat)
2. **Task 2: PeriodSelector → Tabs segmented (режим) + Select (месяц/год)** - `4fad1a3` (feat)

**Plan metadata:** (this commit, docs: complete plan)

## Files Created/Modified
- `ui/src/features/reports/ReportSubNav.svelte` - two-level Reports sub-nav (domain + report type) on Tabs, Badge/bespoke CSS removed
- `ui/src/features/reports/PeriodSelector.svelte` - period-mode switch on Tabs segmented + month/year on Select, DatePicker/range logic untouched

## Decisions Made
- **Count fallback type mismatch (T-28-03-01, accepted):** original code showed the string `'–'` for inactive tabs when `statusCounts` was absent; `Tabs.count` is typed `number`, so the fallback became `0`. In practice `statusCounts` is populated via `reports_get_report_counts` on every real render path, so this branch is rarely reached — documented per plan as a known, accepted minor visual difference, not a primitive extension.
- **Select month/year adapter (T-28-03-02, mitigated):** kept `onMonthChange`/`onYearChange`'s period-recalculation logic byte-identical, only changed their parameter from `Event` (reading `e.currentTarget.value`) to a plain `string` (what `Select`'s `onchange` now hands back directly) — since the native `<select>` element itself was removed, there was no `Event` left to adapt around; a thin wrapper around the old Event-based function would have been artificial with nothing left to wrap.
- **Select sizing override:** `Select.svelte` hardcodes `width: 100%; height: 36px` for standalone form-field use. Placed inline in `PeriodSelector`'s filter row (next to the 28px-tall segmented Tabs and DatePicker), it needed the same `:global()` override treatment `.range-label :global(.date-picker)` already established in this file for GAP-R3 — added `.period-controls :global(.select-wrapper)`/`:global(.select)` overrides (width: auto, height: 28px) so the row stays visually consistent, matching the file's own existing convention rather than inventing a new one.

## Deviations from Plan

None - plan executed exactly as written. Both known-tricky edge cases (count fallback type, Event→string adapter) were called out explicitly in the plan's `<action>` blocks and resolved exactly as specified there.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

D-06 for Reports (WIN-07) is closed: both `ReportSubNav` and `PeriodSelector` are on the shared `Tabs`/`Select` primitives, no bespoke tab/select CSS remains in either file. Remaining D-06 scope (Settings' `SettingsSubNav.svelte`) is a separate plan per `28-PATTERNS.md`. Next plans in this phase (28-04 onward) can proceed independently — no blockers introduced here.

---
*Phase: 28-support-admin-windows*
*Completed: 2026-07-22*

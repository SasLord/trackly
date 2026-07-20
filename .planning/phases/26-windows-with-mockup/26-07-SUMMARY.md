---
phase: 26-windows-with-mockup
plan: 07
subsystem: ui
tags: [svelte, scss, dashboard, design-tokens, data-viz]

# Dependency graph
requires:
  - phase: 26-01
    provides: token layer (--tr-elev-1, --tr-warning-soft/-text, --tr-accent-text, --tr-surface-sunken)
  - phase: 26-02
    provides: dashboard page shell / header restyle patterns
  - phase: 26-03
    provides: dashboard grid layout conventions
provides:
  - StatWidget.svelte matching UI-SPEC §3.10-3.11 (card shell, baseline-aligned number, pill breakdown)
  - ChartWidget.svelte matching UI-SPEC §3.12 (card/axis/legend restyle, literal data-viz palette)
  - PeriodToggle.svelte restyled while keeping role="group" semantics
  - Paired error-string fix across StatWidget + ChartWidget (§9 two-file trap closed)
affects: [26-08 (visual UAT / dark-theme legibility check), dashboard, ui-parity]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Literal hex color arrays for data-viz series (documented --tr-* token exception, not scanned by check-tokens.mjs)"
    - "Pill-row local markup for label+strong pairs instead of Badge.svelte (Badge doesn't support this shape)"

key-files:
  created: []
  modified:
    - ui/src/features/dashboard/StatWidget.svelte
    - ui/src/features/dashboard/ChartWidget.svelte
    - ui/src/features/dashboard/PeriodToggle.svelte

key-decisions:
  - "Breakdown pills use local .pill-row/.pill markup, not Badge.svelte — confirmed in UI-SPEC §3.10 that Badge lacks label+strong pair support"
  - "ChartWidget COLORS array kept as literal hex (#3b6fe0/#1a9d5f/#d8820e), a documented, intentional exception to the --tr-* token gate for data-viz series consistency across themes"
  - "Value-label-over-bar font-size raised to 9px (not 11px) per §3.12/§5 conditional-acceptance value; 11px fallback deferred to Plan 26-08 dark-theme UAT if found illegible"
  - "PeriodToggle kept role=\"group\"/aria-label unchanged — only CSS values borrowed from mockup, no semantic conversion to tablist"

patterns-established: []

requirements-completed: [WIN-01]

# Metrics
duration: ~10min
completed: 2026-07-20
---

# Phase 26 Plan 07: Dashboard Widgets Restyle Summary

**Restyled StatWidget/ChartWidget/PeriodToggle to mockup values (§3.10-3.12) — card shells (padding/elev-1), baseline-aligned 30px stat number with tabular-nums pill breakdown, literal hex chart palette, and an identical paired error string across both widget files — with zero changes to `loadWidgets`/`loadChart`/data derivations.**

## Performance

- **Duration:** ~10 min
- **Started:** 2026-07-20T06:54:00+07:00 (approx.)
- **Completed:** 2026-07-20T06:55:39+07:00
- **Tasks:** 3
- **Files modified:** 3

## Accomplishments
- StatWidget card shell now matches §3.10 (16px padding, elev-1 shadow, min-width:0), stat number 30px/700/lh1/tabular-nums baseline-aligned with its unit, breakdown rendered as pill-row (not Badge), warningItems retoned to `--tr-warning-soft`/`--tr-warning-text` with functionality untouched
- ChartWidget card/axis/legend match §3.12 (18px padding, elev-1 shadow, 16px title, legend top-border), data-viz palette switched to literal hex (documented token exception), value-label font-size raised 8→9px, all bar/tick/series derivations left byte-for-byte unchanged
- PeriodToggle padding and active/inactive weights match Д:254 while `role="group"`/`aria-label="Период графика"` semantics are fully preserved
- Error string ("Не удалось загрузить. Смените период или обновите страницу.") landed identically in both StatWidget and ChartWidget in the same plan (§9's explicitly-flagged two-file trap avoided)

## Task Commits

Each task was committed atomically:

1. **Task 1: StatWidget.svelte — card shell, pill row, warningItems retone, error string** - `82b11b2` (feat)
2. **Task 2: ChartWidget.svelte — card/axis/legend restyle, literal palette, paired error string** - `e4268b9` (feat)
3. **Task 3: PeriodToggle.svelte — pStyle restyle, role unchanged** - `8daf590` (feat)

**Plan metadata:** (this commit)

## Files Created/Modified
- `ui/src/features/dashboard/StatWidget.svelte` - card shell, baseline-aligned stat-value-row, pill-row breakdown, retoned warningItems, paired error string
- `ui/src/features/dashboard/ChartWidget.svelte` - card/header/legend restyle, literal COLORS palette, 9px value labels, paired error string
- `ui/src/features/dashboard/PeriodToggle.svelte` - toggle-btn padding/weight values, role/aria-label preserved

## Decisions Made
- Pill breakdown implemented as local markup (`.pill-row`/`.pill`), not via `Badge.svelte`, per UI-SPEC §3.10's explicit note that Badge doesn't support the label+strong pair shape.
- Data-viz series palette (`COLORS`) intentionally kept as literal hex values rather than `--tr-*` tokens — documented exception since chart series color must stay identical across themes and `check-tokens.mjs` only scans CSS, not JS string literals.
- Value-label-over-bar font-size set to 9px (conditional-acceptance value from §3.12/§5), not bumped to 11px preemptively — that fallback is reserved for Plan 26-08's dark-theme visual UAT if 9px proves illegible.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- All three widget components now match UI-SPEC §3.10-3.12 pixel/token values; `pnpm --dir ui build` and `svelte-check` both pass clean (0 errors) with no new warnings introduced.
- Manual verification of the error state (stop backend / disconnect network, confirm identical string in both widgets; dark-theme 9px value-label legibility) is explicitly deferred to Plan 26-08 per this plan's `<verification>` section.
- No blockers for 26-08.

---
*Phase: 26-windows-with-mockup*
*Completed: 2026-07-20*

## Self-Check: PASSED

- FOUND: .planning/phases/26-windows-with-mockup/26-07-SUMMARY.md
- FOUND: 82b11b2 (Task 1 commit)
- FOUND: e4268b9 (Task 2 commit)
- FOUND: 8daf590 (Task 3 commit)

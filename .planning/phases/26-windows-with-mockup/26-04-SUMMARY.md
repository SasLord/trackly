---
phase: 26-windows-with-mockup
plan: 04
subsystem: ui
tags: [svelte, design-system, table, page-header]

# Dependency graph
requires:
  - phase: 26-windows-with-mockup (26-01)
    provides: "PageHeader.svelte primitive (title/variant/actions props)"
  - phase: 26-windows-with-mockup (26-03)
    provides: "Table.svelte framed/footer props"
provides:
  - "DevicesPage.svelte header migrated to shared PageHeader(variant=wrap)"
  - "DeviceList.svelte footer relocated inside Table's framed border via footer snippet"
affects: [windows-with-mockup, 26-05, 26-08]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Page header markup delegated to shared PageHeader component with actions snippet, replacing bespoke <header> per-page CSS"
    - "List-footer content passed as a Svelte snippet prop into Table, letting Table own the visual frame around table+footer"

key-files:
  created: []
  modified:
    - ui/src/features/devices/DevicesPage.svelte
    - ui/src/features/devices/DeviceList.svelte

key-decisions:
  - "footer prop passed explicitly as footer={footer} (not shorthand {footer}) to match plan's grep-based acceptance criteria"

patterns-established:
  - "PageHeader(variant=\"wrap\") + actions snippet is the standard header pattern for windows migrating off bespoke header markup"

requirements-completed: [WIN-02]

# Metrics
duration: 6min
completed: 2026-07-20
---

# Phase 26 Plan 04: DevicesPage/DeviceList shell migration Summary

**DevicesPage header migrated to shared PageHeader(variant=wrap); DeviceList's pagination footer moved inside Table's framed border via a footer snippet — zero change to CRUD/CSV/print handlers or row-rendering logic.**

## Performance

- **Duration:** 6 min
- **Started:** 2026-07-19T23:36:00Z (approx)
- **Completed:** 2026-07-19T23:42:03Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- `DevicesPage.svelte` header now renders through the shared `PageHeader` component (`variant="wrap"`), with the same three `<Button>` handlers (`openCreate`, CSV-import toggle, `exportCsv`) passed via the `actions` snippet
- `.page-content` padding changed from `24px 32px` to `16px 24px` per UI-SPEC §3.7/§3.8
- `DeviceList.svelte`'s pagination footer (`Показано N из M` / `Групп: N`) now renders inside `Table`'s frame via the new `footer` snippet prop, using `Table`'s default `framed=true` border+radius8+elev-1 styling
- Removed the now-redundant `.device-list-wrapper` div/style (Table's internal `.tr-table-wrapper` owns the `overflow-x: auto` role)
- `DeviceListRow`/`DeviceGroupRow` untouched; all `$derived` state, the `skeletonLoading`/`isEmpty` condition, and empty-state copy left byte-for-byte identical (D-12)

## Task Commits

Each task was committed atomically:

1. **Task 1: DevicesPage.svelte — header migration to PageHeader + body padding** - `599e337` (feat)
2. **Task 2: DeviceList.svelte — footer snippet + framed table, D-12-safe** - `7e1ab72` (feat)

_No TDD tasks in this plan — plain `auto` tasks only._

## Files Created/Modified
- `ui/src/features/devices/DevicesPage.svelte` - Header markup replaced with `<PageHeader title="Устройства" variant="wrap">` wrapping the unchanged 3 Button calls in an `actions` snippet; removed `.page-header`/`.page-title`/`.header-actions` styles; `.page-content` padding changed to `16px 24px`
- `ui/src/features/devices/DeviceList.svelte` - Footer block wrapped in `{#snippet footer()}...{/snippet}` and passed to `<Table footer={footer}>`; removed `.device-list-wrapper` div and its style rule

## Decisions Made
- Passed the footer prop as explicit `footer={footer}` rather than Svelte's `{footer}` shorthand, so the literal string `footer={footer}` (used by the plan's acceptance-criteria grep) is present in the file.

## Deviations from Plan

None - plan executed exactly as written. Both tasks completed via the primary path (no fallback needed for Task 2's D-12 escape hatch — wrapping the footer in a snippet was structurally straightforward).

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `DevicesPage.svelte` and `DeviceList.svelte` now match the mockup's shell chrome (header, body padding, table frame) per UI-SPEC §3.7/§3.8/§3.14/§6.5
- CRUD/CSV/print behavior is unchanged code-for-code; full manual verification (create/edit/delete device, CSV import/export, acceptance-print) is deferred to Plan 26-08's UAT checklist per this plan's `<verification>` section
- Plan 26-05 (filters wiring) can proceed on top of this shell without further header/footer changes needed

---
*Phase: 26-windows-with-mockup*
*Completed: 2026-07-20*

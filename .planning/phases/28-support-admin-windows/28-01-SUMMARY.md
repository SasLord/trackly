---
phase: 28-support-admin-windows
plan: 01
subsystem: ui
tags: [svelte, design-system, tabs, table, tokens]

# Dependency graph
requires:
  - phase: 27-work-windows
    provides: "ActsMasterDetail/ActsSearchAndTabs/ActsList/ActListRow — D-02/D-05/D-03 migration playbook (byte-close precedent, applied 1:1 here)"
provides:
  - "RequestsMasterDetail on --tr-surface-raised + border + box-shadow var(--tr-elev-1) (D-02, closes Phase 26 D-13 regression for Заявки)"
  - "RequestsSearchAndTabs on shared Tabs primitive (variant=underline) with string-key adapter for StatusTab's null branch (D-05)"
  - "RequestsList/RequestListRow on shared Table/TableRow primitives, 4-column layout (D-03)"
affects: [28-02-request-detail-and-form, 30-quality]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "FIX B1 flex-fill master-detail layout (ported from ActsMasterDetail) — panels are flex columns so their single child (Table/DetailPanel) owns internal scroll instead of a viewport-relative min-height"
    - "String-key adapter for Tabs when a filter type includes null (String(key) / key === 'null' round-trip)"

key-files:
  created: []
  modified:
    - ui/src/features/requests/RequestsMasterDetail.svelte
    - ui/src/features/requests/RequestsSearchAndTabs.svelte
    - ui/src/features/requests/RequestsList.svelte
    - ui/src/features/requests/RequestListRow.svelte

key-decisions:
  - "RequestsSearchAndTabs layout: replaced bespoke .tabs{flex:1} wrapper with justify-content:space-between on the row so Tabs stays left-aligned and «Создать заявку» stays pushed to the far right, without reintroducing a bespoke flex-growing wrapper div"
  - "RequestListRow relative date placed as secondary text inside the Автор cell (Claude's Discretion per plan — final column layout, no fields removed)"

patterns-established: []

requirements-completed: [WIN-06]

# Metrics
duration: ~4min
completed: 2026-07-22
---

# Phase 28 Plan 01: Requests window structural migration Summary

**Заявки (WIN-06) window's master-detail surfaces, status-filter tabs, and list re-tokenized onto the shared Tabs/Table/TableRow design-system primitives — mechanical 1:1 port of the Phase 27 Acts playbook, zero field/action/workflow changes.**

## Performance

- **Duration:** ~4 min
- **Started:** 2026-07-22T05:36:40Z
- **Completed:** 2026-07-22T05:40:30Z
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments
- RequestsMasterDetail: both panels moved to `--tr-surface-raised` + border + `box-shadow: var(--tr-elev-1)`, closing the Phase 26 D-13 regression for the Заявки window (master no longer blends into the background); FIX B1 flex-fill layout ported so panels manage their own internal scroll
- RequestsSearchAndTabs: bespoke `<button class="tab">` switch-bar replaced with the shared `Tabs` primitive (`variant="underline"`); string-key adapter round-trips `StatusTab`'s `null` branch ("Все")
- RequestsList/RequestListRow: two-line card list rebuilt as a 4-column `Table`/`TableRow` (Тип/Описание/Автор/Статус); bespoke `.rows`/`.loading`/`.empty`/`.pagination` removed, `Table` now owns the frame/skeleton/empty-state

## Task Commits

Each task was committed atomically:

1. **Task 1: RequestsMasterDetail (D-02) + RequestsSearchAndTabs (D-05)** - `71b8390` (feat)
2. **Task 2: RequestsList + RequestListRow → Table/TableRow (D-03)** - `f69c54a` (feat)

**Plan metadata:** (this commit)

## Files Created/Modified
- `ui/src/features/requests/RequestsMasterDetail.svelte` - both panels on --tr-surface-raised + elev-1, FIX B1 flex-fill layout, grid 35%/65% and <1099px fallback unchanged
- `ui/src/features/requests/RequestsSearchAndTabs.svelte` - status switch-bar on shared Tabs primitive, string-key null-adapter, «Создать заявку» button unchanged
- `ui/src/features/requests/RequestsList.svelte` - rebuilt on shared Table primitive, no pagination (list never had one), footer keeps "N записей" + spinner
- `ui/src/features/requests/RequestListRow.svelte` - 4-column TableRow (Тип/Описание/Автор/Статус), script logic (statusVariant/statusLabel/typeLabel/shortDesc/relativeDate/isAdRestore/handleKeydown) unchanged — only markup moved

## Decisions Made
- RequestsSearchAndTabs: `justify-content: space-between` on `.search-and-tabs` replaces the old `.tabs{flex:1}` wrapper to keep the create-button right-aligned without a bespoke flex wrapper
- RequestListRow: relative date rendered as secondary text inside the Автор cell (column layout is Claude's Discretion per plan — no fields dropped)

## Deviations from Plan

None - plan executed exactly as written. Both tasks matched the Phase 27 Acts playbook 1:1, as anticipated by 28-PATTERNS.md.

## Issues Encountered
None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- Заявки window's master-detail/tabs/list structural layer is fully on `Tabs`/`Table`/`TableRow` and `--tr-*` tokens, no bespoke classes remain in these 4 files
- `RequestDetail.svelte`/`RequestFormModal.svelte`/`RequestsPage.svelte` (D-01, D-04, PageHeader migration) are out of scope for this plan — deferred to 28-02 per 28-PATTERNS.md
- `check-tokens.mjs` and `svelte-check` both pass with 0 errors; `pnpm --dir ui build` succeeds; no regression in shared `Table.svelte`/`TableRow.svelte` (not touched — verified via `git diff --name-only`)

---
*Phase: 28-support-admin-windows*
*Completed: 2026-07-22*

## Self-Check: PASSED
All 4 modified files and SUMMARY.md verified present on disk. Both task commits (71b8390, f69c54a) verified in git log.

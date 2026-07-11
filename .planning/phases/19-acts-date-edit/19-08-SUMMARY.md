---
phase: 19-acts-date-edit
plan: 08
subsystem: ui
tags: [svelte, acts, forms, date-handling, gap-closure]

# Dependency graph
requires:
  - phase: 19-acts-date-edit
    provides: "ActFormBody edit-mode prefill (itemsFromInitialAct), unixToIso()/isoToUnix() UTC round-trip (Plan 19-05)"
provides:
  - "Edit-mode items table forces single-device added rows (qty always 1, hidden editable spinner) so visible quantity always matches what ActUpdateItemDto persists"
  - "todayISO() unified on UTC calendar accessors, matching unixToIso()/isoToUnix()"
affects: [acts, act-editing]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "mode === 'edit' gating on both mutation sites (pickDevice/pickGroup) and render (qty column) to keep UI truthful without a DTO schema change"

key-files:
  created: []
  modified:
    - ui/src/features/acts/ActFormItemsTable.svelte
    - ui/src/features/acts/ActFormBody.svelte

key-decisions:
  - "WR-02 closed via option (a) from 19-REVIEW.md: clamp to single-device rows in edit mode rather than changing ActUpdateItemDto's schema (D-06 schema-consistency)"
  - "Qty column in edit mode shows a static '1' span (not a disabled input) — avoids a misleading spinner control per plan guidance"

patterns-established:
  - "mode === 'edit' gate applied symmetrically at both the state-mutation site (pick handlers) and the render site (qty column) — prevents drift between what's shown and what's clamped"

requirements-completed: [ACT-01, ACT-02]

# Metrics
duration: 5min
completed: 2026-07-12
---

# Phase 19 Plan 08: Gap Closure — Edit-Mode Single-Device Rows + UTC todayISO Summary

**Edit-mode item picker now clamps every added row to exactly one device (qty=1, editable spinner hidden) and `todayISO()` switched to UTC calendar accessors to match the rest of the date pipeline.**

## Performance

- **Duration:** ~5 min
- **Started:** 2026-07-11T19:36:04Z
- **Completed:** 2026-07-11T19:38:01Z
- **Tasks:** 2 completed
- **Files modified:** 2

## Accomplishments
- Closed WR-02: in edit mode, picking a group/quantity via `pickDevice`/`pickGroup` no longer silently drops N-1 devices — quantity is clamped to 1 at the moment of selection, and the qty column no longer offers an editable control that could suggest otherwise (a static "1" is shown instead).
- Closed IN-01: `todayISO()` now uses `getUTCFullYear/getUTCMonth/getUTCDate`, matching the UTC convention already used by `unixToIso()` and the `isoToUnix()` `T00:00:00Z` round-trip — removing the day-boundary off-by-one risk between local-calendar and UTC accessors.

## Task Commits

Each task was committed atomically:

1. **Task 1: WR-02 — force single-device added rows in edit mode** - `704cb99` (fix)
2. **Task 2: IN-01 — todayISO() uses UTC accessors** - `26dbf31` (fix)

**Plan metadata:** (this commit, docs: complete plan)

## Files Created/Modified
- `ui/src/features/acts/ActFormItemsTable.svelte` - `pickDevice`/`pickGroup` clamp `quantity` to 1 when `mode === 'edit'`; qty column renders a static "1" (`.qty-fixed` span) instead of the editable spinner in edit mode; added matching `.qty-fixed` SCSS rule for layout parity with `.qty-input`.
- `ui/src/features/acts/ActFormBody.svelte` - `todayISO()` switched from `getFullYear/getMonth/getDate` to `getUTCFullYear/getUTCMonth/getUTCDate`; stale "browser-local будет хорошо" comment replaced with a UTC-convention note referencing `unixToIso()`/`isoToUnix()`.

## Decisions Made
- Implemented WR-02 via option (a) from `19-REVIEW.md` (smaller, schema-consistent per D-06): clamp UI quantity to 1 in edit mode rather than extend `ActUpdateItemDto` to carry `quantity`/`device_ids`. Multi-device adds during an edit remain possible — the user just adds multiple single-device rows.
- Chose a static `<span class="qty-fixed">1</span>` over a `disabled` numeric input for the edit-mode qty column, per plan guidance to avoid a misleading spinner control.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Both gap-closure items (WR-02, IN-01) from `19-REVIEW.md` are closed. `svelte-check` reports 0 errors (48 pre-existing unrelated warnings). This plan completes the outstanding gap-closure work queued after Phase 19's initial completion/revert (see `d4872bf`).

---
*Phase: 19-acts-date-edit*
*Completed: 2026-07-12*

---
phase: 19-acts-date-edit
plan: 09
subsystem: ui
tags: [svelte5, act-form, uat-gap-closure]

# Dependency graph
requires:
  - phase: 19-acts-date-edit
    provides: "Plan 19-05 (complectation_at_time field + edit prefill), Plan 19-08 (edit-mode qty-fixed pattern), Plan 19-07 (WR-03 audit backend for complectation)"
provides:
  - "Read-only device name rendering for retained edit-mode positions (closes D-10 blank-picker bug)"
  - "комплектация UI fully removed from the edit dialog while the round-trip data path (FormItemRow.complectation_at_time + ActFormBody payload mapping) stays intact (D-09)"
affects: [acts, act-edit-dialog]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Retained-vs-new row marker: mode === 'edit' && row.complectation_at_time !== undefined distinguishes prefilled positions from rows added during the current edit session (only itemsFromInitialAct ever sets this field)"
    - "UI-hidden-but-data-retained field: complectation_at_time kept in the TS interface purely as a round-trip/marker field after its editable UI was removed"

key-files:
  created: []
  modified:
    - ui/src/features/acts/ActFormItemsTable.svelte

key-decisions:
  - "Read-only device cell condition is mode === 'edit' && row.complectation_at_time !== undefined (not row.picked, which is also true for retained rows but doesn't distinguish them from a freshly-picked new row)"
  - "ActFormBody.svelte intentionally left untouched — itemsFromInitialAct prefill and the edit payload's complectation_at_time: it.complectation_at_time ?? null mapping still round-trip the value unchanged"

patterns-established:
  - "device-readonly SCSS class: filled, non-editable cell matching .device-input's box metrics but without border/background/focus — same visual-analog technique as .qty-fixed (Plan 19-08)"

requirements-completed: [ACT-02]

# Metrics
duration: 12min
completed: 2026-07-12
---

# Phase 19 Plan 09: Edit-form Позиции — read-only device name + remove комплектация UI Summary

**Retained act-edit positions now show their device name as static text instead of a blank picker, and the unwanted «Комплектация» input is gone from the edit dialog — while the underlying round-trip data path stays untouched.**

## Performance

- **Duration:** 12 min
- **Started:** 2026-07-12T (session start)
- **Completed:** 2026-07-12T22:16:04Z
- **Tasks:** 2 completed
- **Files modified:** 1

## Accomplishments
- D-10: retained edit-mode rows (`mode === 'edit' && row.complectation_at_time !== undefined`) render `row.device_label` as read-only text in the device column instead of the blank `query: ''` picker input; fresh rows added during the edit session and create-mode rows keep the normal picker unchanged
- D-09: removed the «Комплектация» UI entirely — `handleComplectationInput` handler, the conditional markup block (label + input beneath retained rows), and the `.col-complectation` SCSS rule
- `complectation_at_time` retained in the `FormItemRow` TypeScript interface (doc comment updated) — it still marks retained rows for the D-10 read-only branch and still round-trips through `ActFormBody`'s existing edit payload mapping, which was verified unchanged
- `svelte-check` clean (0 errors) and `pnpm --dir ui build` succeeds after both tasks

## Task Commits

Each task was committed atomically:

1. **Task 1: Read-only device name for retained edit rows (D-10)** - `8f0107c` (feat)
2. **Task 2: Remove комплектация UI, keep round-trip (D-09)** - `b7dd271` (feat)

**Plan metadata:** (this commit)

## Files Created/Modified
- `ui/src/features/acts/ActFormItemsTable.svelte` - Device column now branches on the retained-row marker to render either a read-only `<span class="device-readonly">` or the existing picker `<input>`; комплектация input/handler/SCSS removed; `FormItemRow.complectation_at_time` field and its doc comment retained/updated

## Decisions Made
- Used `complectation_at_time !== undefined` (not `row.picked`) as the retained-vs-new-row condition, per the plan's documented interface semantics — `picked` is also true for a device chosen fresh during this edit session, which must NOT get the read-only treatment
- Left `ActFormBody.svelte` completely untouched (both tasks) — the plan explicitly required the round-trip mapping stay intact; verified via grep (3 occurrences: prefill assignment, comment, payload mapping) and by reading the file to confirm no drift

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None. Both tasks' acceptance-criteria grep assertions passed on first attempt; `svelte-check` and `pnpm --dir ui build` were clean without any auto-fixes needed.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Edit dialog's Позиции table is now visually consistent with the create dialog for retained rows (static device name, static qty="1" from Plan 19-08, no комплектация field); newly-added rows during an edit session still use the full picker
- Manual LAN verification step from the plan's `<verification>` section (open an existing handover act's edit dialog after `pnpm --dir ui build`) is recommended before closing out UAT round 2's Gap 1, but is out of scope for this autonomous plan
- Remaining phase 19 plan: 19-10 (not yet executed)

---
*Phase: 19-acts-date-edit*
*Completed: 2026-07-12*

## Self-Check: PASSED

- FOUND: ui/src/features/acts/ActFormItemsTable.svelte
- FOUND: .planning/phases/19-acts-date-edit/19-09-SUMMARY.md
- FOUND: commit 8f0107c (Task 1)
- FOUND: commit b7dd271 (Task 2)

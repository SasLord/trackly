---
phase: 19-acts-date-edit
plan: 10
subsystem: ui
tags: [svelte5, acts, uat-gap-closure, reactivity]

# Dependency graph
requires:
  - phase: 19-acts-date-edit
    provides: "Plan 19-05 (edit-mode wiring: handleEdit/handleEditSaved, ActFormModal edit props, D-07 edit-button gating)"
provides:
  - "Reactive detail-card refresh immediately after an act edit is saved (closes D-11)"
  - "Редактировать/Возврат buttons fully omitted (not disabled) on return and archived acts (closes D-12/D-13)"
affects: [acts, act-detail]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "selectedAct direct assignment for immediate reactive refresh: when a fresh full ActDto is already available from a mutation response (acts.update()), assign it directly to the $state selectedAct rather than relying on an id-keyed $effect that no-ops when the id is unchanged"
    - "Bare {#if} button omission over {#if}{:else}<disabled> placeholder: when an action is not applicable at all (not just temporarily unavailable), omit the control entirely instead of rendering a disabled stand-in with an explanatory title"

key-files:
  created: []
  modified:
    - ui/src/features/acts/ActsPage.svelte
    - ui/src/features/acts/ActDetail.svelte

key-decisions:
  - "handleEditSaved keeps selectedActId = act.id (harmless no-op) alongside the new selectedAct = act assignment — matches the plan's explicit instruction not to touch the id-keyed $effect or handleReturnSuccess"
  - "Redaktirovat/Vozvrat gates both require !act.archived in addition to onEdit/onReturn && act_type==='handover' — return-act editing stays out of scope per plan (deferred to a future phase)"

requirements-completed: [ACT-02]

# Metrics
duration: 8min
completed: 2026-07-12
---

# Phase 19 Plan 10: Reactive detail refresh + omit Редактировать/Возврат on return & archive Summary

**Act edits now refresh the open detail card immediately (no manual re-open), and Редактировать/Возврат buttons are fully omitted — not just disabled — on return acts and archived acts.**

## Performance

- **Duration:** 8 min
- **Started:** 2026-07-12T (session start)
- **Completed:** 2026-07-12T
- **Tasks:** 2 completed
- **Files modified:** 2

## Accomplishments
- D-11: `handleEditSaved` in `ActsPage.svelte` now assigns `selectedAct = act` directly, using the fresh full `ActDto` returned by `acts.update()` (server `self.get`, includes items + outstanding_device_ids). This closes the staleness bug where `selectedActId = act.id` was a no-op (edited act was already selected), so the id-keyed `$effect` never refetched.
- D-12/D-13: `ActDetail.svelte`'s Редактировать and Возврат buttons converted from `{#if}...{:else}<disabled span>{/if}` to bare `{#if}` blocks. Редактировать renders only when `onEdit && act.act_type === 'handover' && !act.archived`; Возврат renders only when `onReturn && act.act_type === 'handover' && !act.archived`. The disabled placeholder spans (with their explanatory `title` attributes) are gone entirely.
- Resulting button matrix: handover non-archived acts → Печать/Редактировать/Возврат/Удалить (all four); return acts → Печать/Удалить only; archived acts → Печать/Удалить only.
- `svelte-check` clean (0 errors, only pre-existing unrelated warnings) and `pnpm --dir ui build` succeeds after both tasks.

## Task Commits

Each task was committed atomically:

1. **Task 1: Reactive detail refresh after edit (D-11)** - `ace14e4` (fix)
2. **Task 2: Omit disabled Редактировать/Возврат buttons (D-12 + D-13)** - `cd2456b` (fix)

**Plan metadata:** (this commit)

## Files Created/Modified
- `ui/src/features/acts/ActsPage.svelte` - `handleEditSaved` now assigns `selectedAct = act` (fresh ActDto from `acts.update()`) in addition to `selectedActId = act.id`, `refresh()`, `refreshCounts()`
- `ui/src/features/acts/ActDetail.svelte` - Редактировать/Возврат buttons converted to bare `{#if}` (omission instead of disabled placeholder); gated on `onEdit`/`onReturn` && `act_type === 'handover'` && `!act.archived`; Печать and Удалить unchanged

## Decisions Made
- Kept `selectedActId = act.id` in `handleEditSaved` even though it's a no-op for the fix — the plan explicitly required not touching the detail `$effect` or `handleReturnSuccess`, and the existing assignment is harmless (id was already the same).
- Both button gates require `!act.archived` — this matches the plan's D-12/D-13 spec exactly (return-act editing remains out of scope, deferred to a future phase per the plan's explicit `<objective>` exclusion).

## Deviations from Plan

None - plan executed exactly as written. All acceptance-criteria grep assertions passed on the first attempt for both tasks; `svelte-check` was clean (0 errors) without requiring any code auto-fixes.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required. LAN-browser mode users should note the standard `pnpm --dir ui build` step (already run as part of verification) is required before the LAN server serves the updated UI.

## Next Phase Readiness

- This closes both UAT gap-round-2 items assigned to plan 19-10 (D-11 stale-detail bug, D-12/D-13 button omission). Phase 19 (acts-date-edit) plans are now 10/10 executed.
- Manual LAN verification from the plan's `<verification>` section (edit an act and confirm immediate detail refresh; open a return act and an archived act to confirm only Печать/Удалить show; open a handover non-archived act to confirm all four buttons show) is recommended before the phase is marked verified, but is out of scope for this autonomous plan.
- Return-act editing (making Редактировать active on returns) remains explicitly deferred to a future phase per this plan's `<objective>` and the D-13/Deferred note.

---
*Phase: 19-acts-date-edit*
*Completed: 2026-07-12*

## Self-Check: PASSED

- FOUND: ui/src/features/acts/ActsPage.svelte
- FOUND: ui/src/features/acts/ActDetail.svelte
- FOUND: .planning/phases/19-acts-date-edit/19-10-SUMMARY.md
- FOUND: commit ace14e4 (Task 1)
- FOUND: commit cd2456b (Task 2)

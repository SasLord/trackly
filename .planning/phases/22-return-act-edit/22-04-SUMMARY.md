---
phase: 22-return-act-edit
plan: 04
subsystem: ui
tags: [svelte, acts, return-lifecycle, edit-mode]

# Dependency graph
requires:
  - phase: 22-03-return-act-transports
    provides: "acts.updateReturn typed client method, ActUpdateReturnDto/extended
      ActReturnDto/ActItemDto bindings"
  - phase: 22-01-return-act-interface-contracts
    provides: "ActDto.archived_at_utc (D-07 compute-on-read), ActItemDto.device_location"
  - phase: 22-02-return-act-delta-service
    provides: "ActService::update_return() full delta reconciliation, do_return
      giver/receiver/handover_date_utc fix"
provides:
  - "Working «Редактировать» button on return-act cards (ActDetail edit-gate
    includes act_type==='return')"
  - "ReturnModal edit mode: dual-source row prefill, un-swapped ФИО, Дата возврата
    picker, submits acts.updateReturn"
  - "Create-mode ReturnModal payload now sends giver_name/receiver_name/
    handover_date_utc — Pitfall 1 closed end-to-end (not just backend-side)"
  - "«Дата архивации» displayed in ActDetail for archived parent handover acts (D-07)"
affects: []

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "ReturnModal single-instance dual-mode pattern: one component instance
      serves both create and edit via mode/editTarget/parentAct props (mirrors
      ActFormModal's mode/initialAct precedent), avoiding a second modal
      component for what is visually the same dialog"
    - "D-11 (Phase 19) reactive-refresh pattern reused verbatim for return-edit:
      assign selectedAct directly from the fresh server ActDto in the onSuccess
      callback rather than relying on the selectedActId-keyed $effect re-fetch"

key-files:
  created: []
  modified:
    - ui/src/features/acts/ReturnModal.svelte
    - ui/src/features/acts/ActDetail.svelte
    - ui/src/features/acts/ActsPage.svelte

key-decisions:
  - "ReturnModal edit mode defaults applyToAll=false on open — rows already
    carry their own saved per-row condition/location (from editTarget.items),
    so starting in per-row mode preserves those values instead of discarding
    them behind an unset bulk field"
  - "Dialog title sourced from displayNumber (editTarget.number in edit mode,
    act.number in create mode) rather than hardcoding a mode-specific title —
    keeps the existing «Возврат по акту №XXX» format in both modes"
  - "ActUpdateReturnDto's unused location_id/location_name/notes/deadline_utc
    fields (structurally present because the DTO reuses ActUpdateDto's shape,
    but never read by ActService::update_return — confirmed by reading the
    Rust source, which builds ActPatch.location_id from resolved_bulk_location_id
    instead) are sent as null/undefined from the edit-mode payload literal"
  - "Single ReturnModal instance reused for both create and edit (not two
    separate modal instances) — ActsPage tracks returnMode + parallel
    returnEditTargetAct/returnEditParentAct state alongside the existing
    returnTargetAct, closed together in one onClose handler"

patterns-established: []

requirements-completed: [ACT-03]

# Metrics
duration: ~25min
completed: 2026-07-13
---

# Phase 22 Plan 04: Return-Act Edit — UI Summary

**«Редактировать» is now functional on return-act cards: ReturnModal gained a dual-source-prefilled edit mode (own items + parent outstanding items, un-swapped ФИО, editable «Дата возврата»), the create-mode dialog now actually persists the ФИО/date it already collected (RESEARCH.md Pitfall 1 closed end-to-end), and ActDetail surfaces the backend's compute-on-read «Дата архивации» for archived parent acts.**

## Performance

- **Duration:** ~25 min
- **Completed:** 2026-07-13
- **Tasks:** 4/4 completed (Task 4 was verification-only, no source changes, no commit)
- **Files modified:** 3

## Accomplishments

- **Task 1 — ReturnModal edit mode + create-mode Pitfall 1 fix:** Added `mode?: 'create' | 'edit'`, `editTarget?: ActDto | null`, `parentAct?: ActDto | null` props. The row-seeding `$effect` now branches on `mode`: edit mode builds rows from BOTH `editTarget.items` (checked, prefilled `conditionOverride`/`locationOverrideName` from the item's own `condition_at_time`/`device_location`) and `parentAct.items[].outstanding_device_ids` (addable, unchecked) — create mode's existing single-source seeding is untouched. The giver/receiver swap is skipped in edit mode; `giverName`/`receiverName` are sourced directly from `editTarget.giver_name`/`editTarget.receiver_name` (D-12). A `returnDateISO` DatePicker («Дата возврата», D-03/D-04) is now rendered in both modes, `unixToIso`/`isoToUnix`/`todayISO` copied verbatim from `ActFormBody.svelte`. `handleSubmit` branches: edit mode builds an `ActUpdateReturnDto` and calls `acts.updateReturn`; create mode's `ActReturnDto` literal now ALSO includes `giver_name: giverName.trim()`, `receiver_name: receiverName.trim()`, `handover_date_utc: isoToUnix(returnDateISO)` — previously collected by the form but never sent, so the backend always fell back to the parent-swap default. `buildReturnItems()` is reused unchanged for both submit paths.
- **Task 2 — Un-gate ActDetail edit button + ActsPage orchestration:** `ActDetail.svelte`'s edit-gate condition changed from `act.act_type === 'handover'` to `(act.act_type === 'handover' || act.act_type === 'return')`. `ActsPage.svelte`'s `handleEdit` now branches: return-act rows `await acts.get(act.parent_act_id!)` to fetch the parent (needed for outstanding-items prefill), storing it plus the return act in new `returnEditParentAct`/`returnEditTargetAct` state, setting `returnMode = 'edit'` and reusing the existing `returnModalOpen` flag (single shared `ReturnModal` instance, not a second modal). Fetch failure surfaces as an error toast instead of opening a modal with incomplete data. `handleReturnSuccess` now checks `returnMode === 'edit'` and, when the edited return is the currently-selected act, assigns `selectedAct = returnDto` directly from the fresh server response — the D-11 (Phase 19) pattern reused verbatim, so the detail view refreshes without a second click.
- **Task 3 — D-07 «Дата архивации» display:** `ActDetail.svelte` gained `archivedAtLabel`, a `$derived` value computed from `act.archived && act.archived_at_utc != null`, reusing the existing `formatDate` helper. Rendered as an `ActHeaderField` directly after «Дата» in the header-grid, wrapped in `{#if archivedAtLabel}` so it never renders for non-archived acts or return-act detail views (whose own `archived` field is always `false` by construction). No new API call — `archived_at_utc` already arrives on every `acts.get()` response since Plan 22-01.
- **Task 4 — Build verification:** `pnpm --dir ui build` and `pnpm --dir ui exec svelte-check` both run clean across the whole `ui/` tree (242 files, 0 errors, only pre-existing unrelated warnings). No source changes — verification only, per plan. Manual end-of-phase UAT steps are documented in the plan's Task 4 `<human-check>` block (deferred per `human_verify_mode=end-of-phase`).

## Task Commits

Each task with source changes was committed atomically:

1. **Task 1: ReturnModal edit mode + create-mode payload fix** - `d29989c` (feat)
2. **Task 2: Un-gate ActDetail edit button; wire ActsPage orchestration** - `3058f9f` (feat)
3. **Task 3: D-07 Дата архивации display** - `962fc2d` (feat)
4. **Task 4: Build verification** - no commit (verification only, no source changes)

**Plan metadata:** (final docs commit recorded after STATE.md/ROADMAP.md updates)

## Files Created/Modified

- `ui/src/features/acts/ReturnModal.svelte` — `mode`/`editTarget`/`parentAct` props; dual-source row prefill; un-swapped ФИО prefill in edit mode; `returnDateISO` DatePicker in both modes; edit submit calls `acts.updateReturn`; create-mode payload now includes `giver_name`/`receiver_name`/`handover_date_utc`
- `ui/src/features/acts/ActDetail.svelte` — edit-gate includes `act.act_type === 'return'`; `archivedAtLabel` derived + conditional «Дата архивации» `ActHeaderField` row
- `ui/src/features/acts/ActsPage.svelte` — `returnMode`/`returnEditTargetAct`/`returnEditParentAct` state; `handleEdit` branches on `act_type` (async, fetches parent for return rows); `handleReturnSuccess` applies the D-11 direct-assignment pattern for edit-mode saves; `ReturnModal` instantiation extended with the three new props and reset in `onClose`

## Decisions Made

- **ReturnModal edit mode defaults `applyToAll=false` on open** — rows already carry their own saved per-row condition/location from `editTarget.items`; starting in per-row mode avoids silently discarding those saved values behind an unset bulk field the user hasn't touched yet.
- **Dialog title sourced from a `displayNumber` derived value** (`editTarget.number` in edit mode, `act.number` in create mode) rather than a mode-specific title string — keeps the existing «Возврат по акту №XXX» format consistent across both modes with one formula.
- **`ActUpdateReturnDto`'s unused `location_id`/`location_name`/`notes`/`deadline_utc` fields sent as `null`** from the edit-mode payload literal — confirmed by reading `ActService::update_return` in `act_service.rs` that these fields are structurally present (the DTO reuses `ActUpdateDto`'s shape) but never read; the service builds `ActPatch.location_id` from `resolved_bulk_location_id` (derived from `bulk_location_name`/`bulk_location_id`) instead.
- **Single `ReturnModal` instance reused for both create and edit** (not a second modal component) — `ActsPage` tracks `returnMode` plus parallel `returnEditTargetAct`/`returnEditParentAct` state alongside the pre-existing `returnTargetAct`, all reset together in one `onClose` handler, avoiding prop-shape duplication across two modal instances.

## Deviations from Plan

None — plan executed exactly as written. The plan's `<interfaces>` block showed a simplified `ActUpdateReturnDto` shape (omitting `location_id`/`location_name`/`notes`/`deadline_utc`); the actual generated `ui/src/bindings.ts` (Plan 22-01/22-02 output) includes these fields since the Rust DTO structurally mirrors `ActUpdateDto`. This was not a plan error requiring a deviation entry — the plan's own `<read_first>` for Task 1 pointed at the interfaces block as a naming-convention reference, not an exhaustive field list, and the real bindings.ts (read directly during implementation) was used as the source of truth for the payload literal's required fields.

## Issues Encountered

None. `pnpm --dir ui exec svelte-check` was clean (0 errors) after every task; `pnpm --dir ui build` succeeded on the first run.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- ACT-03 is now fully satisfied end-to-end: «Редактировать» is active and functional on return-act cards, the edit dialog prefills from both the return's own saved items and the parent's outstanding items, saves via `acts.updateReturn`, and the detail view refreshes reactively without a second click.
- RESEARCH.md's Pitfall 1 (silent-drop of `giver_name`/`receiver_name`/`handover_date_utc` from the create-return payload) is closed end-to-end — the backend fix (Plan 22-02) is now reachable from a real user submitting the create-return dialog.
- D-07's derived «Дата архивации» is now visible to a real user in `ActDetail.svelte`.
- `ui/dist` is rebuilt and reflects this plan's changes for LAN/browser manual testing.
- Manual end-of-phase UAT (create→persist→edit-prefill round trip, un-return/re-add flows, D-11 conflict-toast verification, D-07 archived-date visibility across handover/return/non-archived detail views) is documented in the plan's Task 4 and deferred to the phase-end UAT pass per `human_verify_mode=end-of-phase` — no blocking checkpoint was consumed in this plan.
- This is the final plan of Phase 22 — ACT-03 marked complete in REQUIREMENTS.md.
- No blockers.

---
*Phase: 22-return-act-edit*
*Completed: 2026-07-13*

## Self-Check: PASSED

- FOUND: `ui/src/features/acts/ReturnModal.svelte`
- FOUND: `ui/src/features/acts/ActDetail.svelte`
- FOUND: `ui/src/features/acts/ActsPage.svelte`
- FOUND: commit `d29989c` (Task 1)
- FOUND: commit `3058f9f` (Task 2)
- FOUND: commit `962fc2d` (Task 3)

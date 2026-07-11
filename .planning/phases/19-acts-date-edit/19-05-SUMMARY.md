---
phase: 19-acts-date-edit
plan: 05
subsystem: acts
tags: [svelte, ui, act-edit]

# Dependency graph
requires: [19-02, 19-03, 19-04]
provides:
  - ActFormBody edit mode (mode='create'|'edit' + initialAct prop, prefill, submit branch to acts.update)
  - ActFormItemsTable комплектация editing (per-row, retained positions only)
  - ActFormModal edit mode (mode/initialAct props, mode-aware title/footer label)
  - ActDetail D-07 button gating (Редактировать disabled+tooltip for return-acts, enabled incl. archived handover-acts)
  - ActsPage onEdit orchestration (editModalOpen/editTargetAct/handleEdit/handleEditSaved, second ActFormModal instance)
affects: []

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Edit-mode prefill sources directly from the initialAct prop (an
      acts.get(id) result passed down from ActsPage's selectedAct) rather
      than re-fetching or re-searching — existing act positions are
      в_работе, not на_складе, so the live device-search path used for
      new rows would never find them."
    - "Second, separate <ActFormModal mode=\"edit\"> instance in ActsPage
      rather than threading create/edit state through one modal — matches
      the existing pattern of ReturnModal/create-mode ActFormModal
      coexisting as independent top-level modal instances."

key-files:
  created: []
  modified:
    - ui/src/features/acts/ActFormBody.svelte
    - ui/src/features/acts/ActFormItemsTable.svelte
    - ui/src/features/acts/ActFormModal.svelte
    - ui/src/features/acts/ActDetail.svelte
    - ui/src/features/acts/ActsPage.svelte

key-decisions:
  - "unixToIso() written as the literal inverse of the existing isoToUnix()
    (UTC-midnight round-trip) to prefill DatePicker inputs from
    initialAct.deadline_utc/handover_date_utc."
  - "itemsFromInitialAct() bypasses fetchGroups/search entirely — each
    ActItemDto becomes a FormItemRow with picked:true, group_ids:[], and
    complectation_at_time carried through; presence of complectation_at_time
    (not its value) is what distinguishes a retained position from a
    freshly-added row in the same edit session, since new rows never set
    that field."
  - "Комплектация input rendered as a full-grid-width row (grid-column: 1/-1)
    beneath its device/qty/actions row, only when mode==='edit' &&
    row.picked && row.device_id!==null && row.complectation_at_time!==undefined."
  - "D-07 edit-button gating built as an if/else exactly mirroring the
    existing Возврат button's disabled-with-tooltip structure, but
    deliberately omitting the !act.archived condition Возврат uses —
    archived handover acts stay editable per D-07."
  - "ActsPage.handleEdit reuses the act argument directly instead of calling
    acts.get(act.id) again — onEdit is only ever invoked from ActDetail
    where act === selectedAct, already fresh via the pre-existing
    acts.get(id) $effect (Pitfall 5: only acts.get(id) populates
    outstanding_device_ids, list()/search() rows never do)."

requirements-completed: [ACT-02]

# Metrics
duration: 20min
completed: 2026-07-12
---

# Phase 19 Plan 05: Act Edit UI Wiring (ACT-02) Summary

**The «Редактировать» button on a handover act's detail card now opens a working, pre-filled edit form (reusing ActFormBody/ActFormModal's create-mode UI) that saves header changes, position add/remove, and per-item «Комплектация» edits through the already-plumbed `acts.update` API — closing the frontend half of ACT-02 and the phase's last plan.**

## Performance

- **Duration:** ~20 min
- **Completed:** 2026-07-12
- **Tasks:** 3/3 completed
- **Files modified:** 5 (no new files)

## Accomplishments

- `ActFormBody.svelte` gained `mode: 'create' | 'edit'` and `initialAct: ActDto | null` props. Edit-mode state (`giverName`, `receiverName`, `location`, `deadlineISO`, `handoverDateISO`, `notes`, `numberOverride`, `items`) initializes directly from `initialAct` at component-init time (each modal open remounts the component via `ActFormModal`'s `{#key openInstanceCounter}`, so this is safe without an `$effect`). A new `unixToIso()` helper (the literal inverse of the existing `isoToUnix()`) converts `deadline_utc`/`handover_date_utc` back to `YYYY-MM-DD` for the `DatePicker` inputs. `itemsFromInitialAct()` builds `FormItemRow[]` directly from `initialAct.items`, bypassing the live on-warehouse device search entirely — existing positions are `в_работе`, not `на_складе`, so a live re-search would never find them.
- `handleSubmit`'s `'edit'` branch builds a full-replacement `ActUpdateDto` (`id`, `expected_version: initialAct.version`, header fields, and `items: [{device_id, complectation_at_time}]`) and calls `acts.update(payload)`; the `'create'` branch is byte-for-byte unchanged. Success toast becomes `` Акт №${saved.number} обновлён ``. A new `OptimisticLockMismatch` branch shows "Акт был изменён другим пользователем — обновите страницу и попробуйте снова." instead of falling through to the generic error toast.
- `ActFormItemsTable.svelte`'s `FormItemRow` gained an optional `complectation_at_time?: string | null` field; a new optional `mode?: 'create' | 'edit'` prop (default `'create'`, backward-compatible with the unchanged create-mode caller) gates a new "Комплектация" text input rendered as a full-width row beneath any retained position (`row.picked && row.device_id !== null && row.complectation_at_time !== undefined` — the `!== undefined` check is what distinguishes a prefilled/retained row from a freshly-added one in the same session, since only the prefill path ever sets this field). «Технические характеристики» (`devices.notes`) stays read-only everywhere — confirmed by grep, no new editable input anywhere near that term.
- `ActFormModal.svelte` gained `mode`/`initialAct` props (passed straight through to `ActFormBody`); the `Modal` title is now `` `Редактировать акт №${initialAct?.number}` `` in edit mode vs. `'Новый акт'` in create mode, and the footer button label switches to "Сохранение…"/"Сохранить" vs. "Создание…"/"Создать акт".
- `ActDetail.svelte`'s "Редактировать" button now gates on `act.act_type !== 'handover'`, mirroring the existing "Возврат" button's disabled-with-tooltip structure exactly — but deliberately does **not** add an `!act.archived` condition, since D-07 explicitly keeps archived handover acts editable (unlike "Возврат", which the pre-existing code already disables for archived acts).
- `ActsPage.svelte` gained `editModalOpen`/`editTargetAct` state, `handleEdit(act)` (mirroring `handleReturn`), and `handleEditSaved(act)` (mirroring `handleSaved`). `handleEdit` reuses the `act` argument directly rather than re-fetching via `acts.get(act.id)` — `onEdit` is only ever invoked from `ActDetail` where `act === selectedAct`, and `selectedAct` is already guaranteed fresh via the pre-existing `acts.get(id)` `$effect` (Pitfall 5 — only `acts.get(id)` populates `outstanding_device_ids`, never `list()`/`search()` rows). A second, independent `<ActFormModal mode="edit">` instance was added alongside the existing create-mode instance (matching how `ReturnModal` already coexists as its own top-level modal), wired with `onEdit={handleEdit}` into `ActDetail` and `onSaved={handleEditSaved}`.

## Task Commits

Each task was committed atomically:

1. **Task 1: ActFormBody + ActFormItemsTable — edit-mode prefill, комплектация editing, submit branch** - `0b8af64` (feat)
2. **Task 2: ActFormModal edit-mode props + ActDetail D-07 button gating** - `e78c3f5` (feat)
3. **Task 3: ActsPage orchestration — wire onEdit, edit-modal state, save/refresh** - `8f9167c` (feat)

## Files Created/Modified

- `ui/src/features/acts/ActFormBody.svelte` - `mode`/`initialAct` props; `unixToIso()` helper; `itemsFromInitialAct()`; edit-mode state init; `handleSubmit` mode-branch (`ActUpdateDto` → `acts.update`); `OptimisticLockMismatch` toast branch; `mode`-aware error copy fallback
- `ui/src/features/acts/ActFormItemsTable.svelte` - `FormItemRow.complectation_at_time`; `mode` prop; `handleComplectationInput`; per-row "Комплектация" input (full-grid-width) gated on retained-position condition; `.col-complectation` styles
- `ui/src/features/acts/ActFormModal.svelte` - `mode`/`initialAct` props passed to `ActFormBody`; mode-aware `Modal` title; mode-aware footer button label
- `ui/src/features/acts/ActDetail.svelte` - "Редактировать" button gated on `act.act_type !== 'handover'` with disabled-with-tooltip pattern (mirrors "Возврат"), no `act.archived` condition
- `ui/src/features/acts/ActsPage.svelte` - `editModalOpen`/`editTargetAct` state; `handleEdit`/`handleEditSaved`; `onEdit={handleEdit}` wired into `ActDetail`; second `<ActFormModal mode="edit">` instance

## Decisions Made

- `unixToIso()` is the exact structural inverse of the existing `isoToUnix()` (UTC-midnight round-trip via `Date.parse`/`Date` UTC getters) — no new date library or format introduced.
- `itemsFromInitialAct()` sets `group_ids: []` on every prefilled row (unlike live-picked rows, which populate `group_ids` from the device-group search) since the edit submit path only ever reads `device_id`/`complectation_at_time` off each item — `group_ids` has no meaning for `ActUpdateDto`'s full-replacement-set contract.
- Used the presence of `complectation_at_time !== undefined` (not a separate boolean flag) as the discriminator between "retained position, prefilled" and "freshly added this session" — avoids adding a redundant field to `FormItemRow` since the prefill path is the only writer of that field at row-creation time.
- Second, fully independent `<ActFormModal mode="edit">` instance in `ActsPage` rather than threading a shared open/mode state through one instance — simpler, and consistent with how `ReturnModal` and the create-mode `ActFormModal` already coexist as separate top-level modals in the same file.

## Deviations from Plan

None — all three tasks were executed exactly as specified in the plan's `<action>` blocks. The only refinement was rewriting the `ActDetail.svelte` D-07 gate's `if`/`else` branch ordering once, mid-Task-2, to keep the literal `act.act_type !== 'handover'` substring inside the disabled `<Button>`'s attribute (matching the acceptance criteria's grep pattern exactly) while still mirroring the "Возврат" button's structural if/else shape — not a logic change, just a textual match adjustment.

## Issues Encountered

None — all three tasks passed `pnpm --dir ui exec svelte-check` (0 errors) and `pnpm --dir ui build` (succeeded) on the first attempt after implementation.

## User Setup Required

None — no external service configuration required.

## Manual Verification (deferred, workflow.human_verify_mode=end-of-phase)

Per the plan's Task 3 `<human-check>` block, the following require a live browser/Tauri session and were NOT run as part of this automated execution (per project config `human_verify_mode: "end-of-phase"`):

1. Open Acts page (Tauri + LAN browser after `pnpm --dir ui build`), select a handover act, confirm "Редактировать" is enabled and opens a form pre-filled with current header + positions.
2. Change a header field, save — confirm success toast + detail view reflects the change.
3. Add a position from stock, save — confirm device transitions на_складе→в_работе.
4. Remove a position, save — confirm device returns to prior status/location.
5. Edit a retained position's Комплектация field, save — confirm it persists and re-displays on reopen.
6. Two-tab stale-version save — confirm the "изменён другим пользователем" toast (409/OptimisticLockMismatch), not silent failure.
7. Select a return-act — confirm "Редактировать" is disabled with tooltip; select an archived handover act — confirm it remains enabled.

These should be run at phase-end verification, per `/gsd-verify-work` or equivalent.

## Next Phase Readiness

- This was the last plan in Phase 19 (acts-date-edit). ACT-02 ("Пользователь может отредактировать существующий акт — кнопка «Редактировать» активна") is now fully closed end-to-end: backend (`ActService::update`, Plan 19-03), dual-transport wiring (Plan 19-04), and this plan's UI (prefilled edit form, D-07 gating, комплектация editing, stale-version toast).
- ACT-01 (act date persisted as the act's date) was closed in Plans 19-01/19-02.
- Phase 19's ROADMAP.md Success Criteria 2 and 3 are satisfied by this plan's changes, pending the manual verification pass listed above.
- No further plans are queued in Phase 19 — ready for phase closure (`/gsd-transition` or equivalent) once manual verification passes.

---
*Phase: 19-acts-date-edit*
*Completed: 2026-07-12*

## Self-Check: PASSED

All 5 claimed modified files found on disk (`ActFormBody.svelte`,
`ActFormItemsTable.svelte`, `ActFormModal.svelte`, `ActDetail.svelte`,
`ActsPage.svelte`); this SUMMARY.md found on disk; all 3 claimed commit
hashes (`0b8af64`, `e78c3f5`, `8f9167c`) found in git log.

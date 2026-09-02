---
phase: 40-movement-history
plan: 19
subsystem: ui
tags: [svelte, svelte5-runes, place-service, bulk-move, modal, place-picker]

# Dependency graph
requires:
  - phase: 40-movement-history (plan 13)
    provides: "PlaceService::move_subtree_contents + places_move_subtree_contents Tauri command + handler_move_subtree_contents axum handler, both gated on MutateDevices+MutateCartridges (D-13)"
provides:
  - "PlaceContents.svelte: «Перенести всё содержимое в…» button + confirm Modal wired to places_move_subtree_contents, completing D-28's bulk-move UI"
affects: []

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Bulk-move confirm dialog clones DeviceContextMenu.svelte's Modal/footer-snippet/Button structure, and PlaceMoveModal.svelte's form-field/stats-loading visual pattern, rather than inventing a new confirm-dialog shape"
    - "Confirm-dialog {N} count is fetched fresh with nested=true on modal open (own $effect, own local state) rather than reused from the panel's already-loaded `rows`/`counts.all`, because those are filtered by the onlyHere toggle while the backend always walks the full nested subtree — reusing them would under-report N when 'Только здесь' is on"

key-files:
  created: []
  modified:
    - ui/src/features/places/PlaceContents.svelte

key-decisions:
  - "Trigger button uses variant=\"secondary\" (not \"primary\") — this panel already reserves \"primary\" for confirm/submit actions inside modals (PlaceMoveModal, PlaceFormModal); other toolbar-level actions in this codebase (Экспорт/Импорт CSV) use \"secondary\", and UI-SPEC's 'Primary CTA' label in the Copywriting Contract table reads as 'the main new call-to-action this phase adds', not literally Button variant=\"primary\" — the load-bearing constraint (must_haves.truths) only requires the CONFIRM button to be variant=\"primary\", which it is"
  - "Confirm body's {N} always reflects the full nested subtree (fetched via a dedicated places_contents(rootId, nested=true) call on modal open), independent of the panel's own onlyHere toggle, so the number shown always matches what move_subtree_contents will actually move"
  - "Failure toast uses the UI-SPEC's fixed literal copy ('Не удалось перенести содержимое. Попробуйте ещё раз.') unconditionally, not the server's error message — matching this task's literal acceptance criterion rather than DeviceContextMenu's message-passthrough convention"

requirements-completed: []  # HST-01 NOT marked complete here — orchestrator closes at phase end per bookkeeping_constraint

# Metrics
duration: ~25min
completed: 2026-09-02
---

# Phase 40 Plan 19: Bulk-Move Content UI Summary

**«Перенести всё содержимое в…» button + confirm Modal on `PlaceContents.svelte`, wired to Plan 40-13's `places_move_subtree_contents`, with a live re-fetched item count and UI-SPEC's exact non-destructive copy.**

## Performance

- **Duration:** ~25 min
- **Completed:** 2026-09-02
- **Tasks:** 1/1
- **Files modified:** 1

## Accomplishments

- Added a «Перенести всё содержимое в…» button to `PlaceContents.svelte`'s controls row (next to the existing «Только здесь» checkbox), opening a confirm `Modal` titled «Перенести содержимое в другое место?»
- Modal contains a `PlacePicker` labeled «Новое место» (target place, initially unselected) and a body paragraph with UI-SPEC's exact copy, interpolating a freshly-fetched (always `nested=true`) item count so the number always matches what the backend will actually move regardless of the panel's own «Только здесь» toggle state
- Footer: «Отмена» (`variant="secondary"`) / «Перенести» (`variant="primary"`, disabled until a target is chosen, `loading` during the call) — confirmed zero `variant="destructive"` usage in the new block
- On success: toast «Содержимое перенесено», modal closes, content list refreshes via the existing `reloadToken` re-fetch mechanism (same one `PlaceEntityViewModal`'s edit-save flow already uses)
- On failure: fixed toast copy «Не удалось перенести содержимое. Попробуйте ещё раз.», modal stays open so the user can retry or cancel

## Task Commits

Each task was committed atomically:

1. **Task 1: Bulk-move button + confirm modal + target-place picker** - `8bc921c9` (feat)

## Files Created/Modified

- `ui/src/features/places/PlaceContents.svelte` - added `moveModalOpen`/`moveTargetId`/`moving`/`moveCount`/`moveCountLoading` state, `openMoveModal`/`closeMoveModal`/`handleMoveConfirm` functions, a count-fetch `$effect`, the toolbar button, the confirm `Modal` markup, and supporting SCSS (`.controls-right`, `.move-form`, `.form-field`, `.form-label`, `.stats-loading`, `.confirm-body`)

## Decisions Made

See `key-decisions` in frontmatter. Summary:
- Trigger button `variant="secondary"`, confirm button `variant="primary"` (never `destructive`)
- Confirm-dialog count always reflects the full nested subtree, decoupled from the panel's `onlyHere` toggle
- Failure toast uses the literal UI-SPEC string unconditionally, not the server's own error message

## Deviations from Plan

None - plan executed exactly as written. The plan's `<action>` text explicitly anticipated the two judgment calls above ("match whichever ... convention" / interfaces note that UI-SPEC "does not show a picker in the dialog body copy, so add it") and both were resolved within the plan's own stated discretion, not as corrections to it.

## Issues Encountered

None. `pnpm --dir ui svelte-check` (284 files, 0 errors, only pre-existing unrelated warnings), `pnpm --dir ui lint` (eslint + prettier + all custom token/contrast/focus/print-isolation/place-path check scripts), and `pnpm --dir ui build` all pass clean.

## Known Stubs

None. The button is fully wired end-to-end (fetch count → pick target → confirm → invoke → toast → refresh); no hardcoded/empty data paths were introduced.

## Verification Reality Check

**Mechanically verified:**
- `svelte-check` (0 errors), `eslint`/`prettier`/custom lint scripts (all pass), `pnpm build` (succeeds, no new warnings)
- `grep` acceptance criteria: exact button copy present once; zero `variant="destructive"` in the new block
- Static read-through of the Modal/Button/PlacePicker prop contracts against their actual `Props` interfaces (confirmed via source read, not assumption)

**UNVERIFIED — requires a run of the real app (per this plan's `verification_reality` note, svelte-check/eslint/build do not catch Svelte 5 rune runtime errors):**
- That the confirm modal actually opens/closes correctly and the `$effect` count-fetch fires exactly once per open (not on every re-render)
- That `PlacePicker`'s tree/search UI works correctly inside this new Modal context (portal/dropdownAnchor interaction with a second, nested-ish modal)
- That the Tauri `invoke('places_move_subtree_contents', ...)` call round-trips correctly with real backend data (the `i32` return note in 40-13's summary means the response type is `number`, matched here — but never exercised against a live Tauri process in this plan)
- That the content list visibly refreshes after a successful move (the `reloadToken` bump triggers the same `$effect` that already re-fetches `rows` on `place.id`/`onlyHere` changes — confirmed by reading that effect's dependencies, not by running it)
- LAN-browser (axum) transport path for the same call (mechanically identical `apiCall` usage to every other mutation in this file, but not independently exercised)

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- HST-01's full UI surface (per-entity manual moves, act-linked moves, and now bulk subtree moves) is wired end-to-end on the frontend, matching Plan 40-13's fully-tested backend.
- HST-01 is NOT marked complete in `.planning/REQUIREMENTS.md` — left for the orchestrator to close at phase end, per this plan's `bookkeeping_constraint`.
- Recommend the phase's UAT pass explicitly exercise this button in the running app (Tauri desktop + a LAN browser session) given the UNVERIFIED items above — no automated harness in this codebase currently exercises Svelte 5 rune runtime behavior.
- No blockers identified.

---
*Phase: 40-movement-history*
*Completed: 2026-09-02*

## Self-Check: PASSED

- FOUND: ui/src/features/places/PlaceContents.svelte
- FOUND commit: 8bc921c9
- FOUND commit: ce52635b

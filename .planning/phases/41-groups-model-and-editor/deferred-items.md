# Deferred items (phase 41)

## 41-18 — svelte-check: 1 error outside plan scope — RESOLVED (commit 17d26df2, orchestrator)
- `ui/src/features/showcase/sections/MovementTimelineSection.svelte:16` — the fixture object lacks `batch_id`
  (`MovementEntryDto.batch_id: string | null` is required in the regenerated `bindings.ts`; the fixture gives `undefined`).
- Not caused by plan 41-18 (files untouched); appears since bindings gained `batch_id` for the group batch (D-25/D-26).
- Fix: add `batch_id: null` to the showcase fixture entries. Belongs to whichever plan owns MovementTimeline batch UI.
- DONE: the orchestrator added `batch_id`/`group_id`/`group_label` to the fixture factory in commit 17d26df2;
  svelte-check is back to the 0-errors / 68-warnings baseline. Root cause for the record: the backend waves'
  post-merge gates were cargo-only, so this frontend regression (introduced by plan 41-05) went unnoticed for
  seven waves until plan 41-18 ran the UI chain.

## 41-23 — вне границ плана
- `MovementTimeline.svelte`: пустой блок «Перемещений ещё не было…» остаётся устройство-ориентированным (для группы GroupPanel рисует свой); добавлен только `showInitialPlacementNote` для сноски под списком.

# Deferred items (phase 41)

## 41-18 — svelte-check: 1 error outside plan scope
- `ui/src/features/showcase/sections/MovementTimelineSection.svelte:16` — the fixture object lacks `batch_id`
  (`MovementEntryDto.batch_id: string | null` is required in the regenerated `bindings.ts`; the fixture gives `undefined`).
- Not caused by plan 41-18 (files untouched); appears since bindings gained `batch_id` for the group batch (D-25/D-26).
- Fix: add `batch_id: null` to the showcase fixture entries. Belongs to whichever plan owns MovementTimeline batch UI.

## 41-23 — вне границ плана
- `GroupTypePanel.svelte` (41-20): для read-only роли (manager) в панели типа остаётся текст «Код и поведение типа изменить нельзя. Название и набор свойств — можно.» — для manager это звучит как разрешение, которого у него нет (контролов правки нет). Решить копирайтом при верификации (UI-SPEC 9.1).
- `MovementTimeline.svelte`: пустой блок «Перемещений ещё не было…» остаётся устройство-ориентированным (для группы GroupPanel рисует свой); добавлен только `showInitialPlacementNote` для сноски под списком.

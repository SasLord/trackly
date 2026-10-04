# Deferred items (phase 41)

## 41-18 — svelte-check: 1 error outside plan scope
- `ui/src/features/showcase/sections/MovementTimelineSection.svelte:16` — the fixture object lacks `batch_id`
  (`MovementEntryDto.batch_id: string | null` is required in the regenerated `bindings.ts`; the fixture gives `undefined`).
- Not caused by plan 41-18 (files untouched); appears since bindings gained `batch_id` for the group batch (D-25/D-26).
- Fix: add `batch_id: null` to the showcase fixture entries. Belongs to whichever plan owns MovementTimeline batch UI.

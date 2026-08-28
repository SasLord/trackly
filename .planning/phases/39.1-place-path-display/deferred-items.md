# Deferred Items — Phase 39.1

## 39.1-08 — pre-existing svelte-check error (out of scope)

`ui/src/features/showcase/sections/PlacePickerSection.svelte:20:5` — `Error: Property
'path_variant_override' is missing in type '...' but required in type 'PlaceDto'.` Confirmed
pre-existing (reproduces on `git stash` before this plan's edits) — introduced by an earlier
plan in this phase that added `path_variant_override` to `PlaceDto` without updating the
showcase fixture. Not touched by plan 08 (files: `Input.svelte`, `placePath.ts`,
`OrgSettings.svelte`). Left as-is per scope boundary.

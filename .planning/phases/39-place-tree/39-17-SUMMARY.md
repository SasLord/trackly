---
phase: 39-place-tree
plan: 17
subsystem: ui
tags: [svelte, svelte5-runes, place-tree, acts, returns, place-picker]

# Dependency graph
requires:
  - phase: 39-place-tree plan 13
    provides: "PlacePicker.svelte — the reusable place-selection control (value/onChange/id/disabled/invalid, default apiCall-backed fetchers) this plan wires into all three act-family surfaces"
  - phase: 39-place-tree plan 07
    provides: "ActDto/ActCreateDto/ActUpdateDto onto place_id/full_path/place_path_snapshot — the DTO shapes this plan's ActFormBody.svelte and ActDetail.svelte consume"
  - phase: 39-place-tree plan 11
    provides: "ActReturnDto/ActReturnItemDto/ActUpdateReturnDto onto bulk_place_id/place_id_override — the DTO shapes this plan's ReturnModal.svelte and ReturnItemsTable.svelte/returnPayload.ts consume; D-16 place_path_snapshot server-side capture this plan's forms never bypass"
  - phase: 39-place-tree plan 09
    provides: "cartridge_storage_place_ids Tauri/HTTP command (place-tree-derived, entity-agnostic despite its cartridge-era name) — reused here for D-11.1's storage-places quick-pick"
provides:
  - "ActFormBody.svelte — own place_id via PlacePicker, replacing the freeform location text field"
  - "ReturnModal.svelte — bulk_place_id via PlacePicker for both create (ActReturnDto) and edit (ActUpdateReturnDto) submit paths, plus a D-11.1 storage-places quick-pick chip row with default preselection"
  - "ReturnItemsTable.svelte / returnPayload.ts — per-row place_id_override via PlacePicker, composite-key grouping keyed on place_id_override instead of a freeform location string"
  - "ActDetail.svelte / ActFormItemsTable.svelte — trailing location->place_id/full_path rename cleanup (act.full_path display, DeviceFilter.place_id search payload)"
affects: [39-18 (device-family act-adjacent surfaces — none left to touch in acts/), 39-20, 39-21 (end-to-end/UAT checkpoint should exercise act create -> bulk return -> per-row return -> printed-snapshot flow in a real webview)]

tech-stack:
  added: []
  patterns:
    - "D-11.1's 'raise storage places to the top of the list' requirement is implemented as a quick-pick chip row (Button variant=primary|secondary, size=sm) rendered ABOVE PlacePicker in the consuming form, not as a reordering/pinning feature inside PlacePicker itself — PlacePicker's Props interface (Plan 13) has no 'priority'/'pinned' concept, and adding one would change behavior for every other consumer in the app. Every future D-11.1-style requirement should follow the same external-chip-row pattern rather than proposing a PlacePicker API change."
    - "Default-preselection for a suggested (non-forced) value is applied once, inside the async fetch's resolve callback (`if (bulkPlaceId === null) bulkPlaceId = ids[0] ?? null`), NOT as a persistent reactive $effect keyed on the state itself — a reactive effect watching both the target state and the fetched list would re-clobber a value the user explicitly cleared after the fetch resolved. This one-shot-on-resolve pattern is the correct shape for any future 'suggest a default without forcing it' requirement in this codebase."
    - "When a shared exported type (ReturnRowState in ReturnItemsTable.svelte) is renamed across a plan's per-file task split, the type owner's task and its consumer's task must land in the same execution pass to keep svelte-check green at every git commit boundary — this plan committed ReturnModal.svelte (Task 2, the consumer) and ReturnItemsTable.svelte/returnPayload.ts (Task 3, the type owner) as separate atomic commits, but implemented and verified them as one coordinated code change before splitting the git history by file, since an isolated Task-2-only commit would not type-check against the pre-Task-3 ReturnRowState shape."

key-files:
  created: []
  modified:
    - ui/src/features/acts/ActFormBody.svelte
    - ui/src/features/acts/ReturnModal.svelte
    - ui/src/features/acts/ReturnItemsTable.svelte
    - ui/src/features/acts/returnPayload.ts
    - ui/src/features/acts/ActDetail.svelte
    - ui/src/features/acts/ActFormItemsTable.svelte

key-decisions:
  - "D-11.3's 'Перевести устройство в статус «На складе»' checkbox is NOT implemented anywhere in this plan. Per 39-16-SUMMARY.md's amendment (commit 8cfdb4b4, 2026-08-25), D-11.3 belongs exclusively to the device form (DeviceFormBody.svelte) — it names 'устройство' (device), not 'акт'/'возврат', and this plan's own PLAN.md text does not mention D-11.3 anywhere in its four tasks. Confirmed by literal re-read of the UI-SPEC line and the plan text before starting, per prior_wave_context's explicit warning about the 39-16 misplacement."
  - "ReturnModal.svelte's per-row disabled PlacePicker (when applyToAll=true or the row is unchecked) is bound to the EFFECTIVE place — `bulkPlaceId` when applyToAll, `row.placeIdOverride` otherwise — rather than an empty field. PlacePicker's Props interface has no free-text placeholder-override prop (unlike the old LocationAutocomplete it replaces), so the previous 'show the bulk value as ghost placeholder text' UX could not be reproduced literally; showing the actual resolved place in a disabled PlacePicker is the closest equivalent and arguably clearer (it shows a real, clickable-when-enabled place node rather than placeholder text)."
  - "ActUpdateReturnDto's own top-level `place_id` field (the return act's own place, distinct from `bulk_place_id`) is sent as `place_id: null` unchanged from the pre-existing `location_id: null, location_name: null` behavior — ReturnModal.svelte has never had a UI field for editing the return act's own place (only the bulk-return-target place), so this is a straight field-name rename with no behavior change, not a new capability. Out of this plan's declared scope (not mentioned in any task's action text)."

requirements-completed: [PLC-03, PLC-04]

# Metrics
duration: ~35min
completed: 2026-08-25
---

# Phase 39 Plan 17: Act-family PlacePicker wiring Summary

**Wired `PlacePicker` into all three act-family place surfaces — the act form's own `place_id`, the bulk-return modal's `bulk_place_id` (with a new D-11.1 storage-places quick-pick chip row + default preselection), and the per-row return-items table's `place_id_override` — closing the act side of D-17; runtime behavior is UNVERIFIED (see below).**

## Performance

- **Duration:** ~35 min
- **Started:** 2026-08-25 (est.)
- **Completed:** 2026-08-25
- **Tasks:** 4/4
- **Files modified:** 6

## Accomplishments

- `ActFormBody.svelte`: `location` text state replaced with `placeId: number | null` bound to `PlacePicker`; both create (`ActCreateDto.place_id`) and edit (`ActUpdateDto.place_id`) payload branches send the caller-resolved place id; label unified to "Место" per UI-SPEC §12
- `ReturnModal.svelte`: `bulkLocationName` replaced with `bulkPlaceId: number | null`; both `ActReturnDto.bulk_place_id` (create) and `ActUpdateReturnDto.bulk_place_id` (edit) submit paths updated; D-11.1 implemented as a quick-pick chip row (`Button` variant=primary when selected) fetching `cartridge_storage_place_ids` once per modal open, resolving each id's `full_path` via `places_get`, and preselecting the first storage place as `bulkPlaceId` when unset (never clobbering an already-set value); edit-mode per-row prefill reads `it.device_place_id` (the renamed `ActItemDto` field) instead of the dropped `device_location`
- `ReturnItemsTable.svelte`: `ReturnRowState.locationOverrideName: string` renamed to `placeIdOverride: number | null`; per-row `PlacePicker` replaces `LocationAutocomplete`; disabled-state cell now shows the *effective* place (bulk or override) rather than empty placeholder text; column header/CSS class renamed Расположение→Место / `.col-location`→`.col-place`
- `returnPayload.ts`: `buildReturnItems()`'s composite-key grouping now keys on `place_id_override` instead of a trimmed freeform location string; both the `applyToAll` and per-row-override branches emit `place_id_override` (the single DTO field — no separate name field); JSDoc `@example` blocks updated to match
- `ActDetail.svelte` / `ActFormItemsTable.svelte`: trailing rename cleanup — `act.location`→`act.full_path` (live-resolved display, distinct from the frozen `place_path_snapshot`), `DeviceFilter.location_id`→`place_id` in the "на складе" device-search payload (both calls always passed `null` here — a pure rename, no behavior change)

## Task Commits

Each task was committed atomically:

1. **Task 1: ActFormBody.svelte — own place_id** - `dce1ffc1` (feat)
2. **Task 2: ReturnModal.svelte — bulk_place_id + D-11.1 quick-pick** - `b3a77722` (feat)
3. **Task 3: ReturnItemsTable.svelte — per-row place_id_override** - `14ae4c3b` (feat)
4. **Task 4: ActDetail.svelte + ActFormItemsTable.svelte — location field rename** - `8921d065` (fix)

**Plan metadata:** `966779a9` (docs: log runtime verification debt to deferred-items.md)

## Files Created/Modified

- `ui/src/features/acts/ActFormBody.svelte` - own `place_id` via `PlacePicker`
- `ui/src/features/acts/ReturnModal.svelte` - `bulk_place_id` via `PlacePicker` + D-11.1 storage quick-pick chips
- `ui/src/features/acts/ReturnItemsTable.svelte` - per-row `place_id_override` via `PlacePicker`
- `ui/src/features/acts/returnPayload.ts` - `buildReturnItems()` composite-key grouping on `place_id_override`
- `ui/src/features/acts/ActDetail.svelte` - `act.full_path` display
- `ui/src/features/acts/ActFormItemsTable.svelte` - `DeviceFilter.place_id` search payload

## Decisions Made

See `key-decisions` in frontmatter for full rationale on: (1) D-11.3's checkbox correctly NOT implemented on any act surface (belongs to devices per the 39-16 amendment); (2) the disabled per-row `PlacePicker` showing the effective place instead of ghost placeholder text (PlacePicker has no placeholder-override prop, unlike the `LocationAutocomplete` it replaces); (3) `ActUpdateReturnDto.place_id: null` being an unchanged-behavior rename, not a new capability.

## Deviations from Plan

### Auto-fixed Issues

None — every code change maps directly to a plan task action. No Rule 1/2/3 bugs or missing-functionality gaps were found while implementing.

### Judgment calls (not bugs, tracked separately)

**1. Cross-task type-rename sequencing (ReturnModal.svelte / ReturnItemsTable.svelte)**
- **Found during:** Task 2, while renaming `ReturnRowState.locationOverrideName` usages in `ReturnModal.svelte`'s row-construction `$effect`
- **Issue:** Task 2's own acceptance criteria requires zero occurrences of `locationOverrideName` in `ReturnModal.svelte` after Task 2, which means the row literals must already use `placeIdOverride` — but that field is only declared on `ReturnRowState` by Task 3's edit to `ReturnItemsTable.svelte`. Committing Task 2 in isolation (before Task 3) would therefore leave the tree in a state that does not type-check.
- **Resolution:** Implemented and verified Tasks 2 and 3's code changes together as one coordinated edit (so `svelte-check` and `build` were only ever run against a fully-consistent tree), then split the already-verified changes into two atomic git commits by file (`ReturnModal.svelte` alone for Task 2's commit, `ReturnItemsTable.svelte` + `returnPayload.ts` for Task 3's commit) — matching each task's own declared `<files>` scope. No functional gap; documented as a `tech-stack.patterns` entry for future plans with the same shared-type-across-tasks shape.
- **Verification:** `pnpm --dir ui run svelte-check` run once after both tasks' code changes were in place — 0 errors in any file this plan touches (see Issues Encountered for the full before/after error count).

---

**Total deviations:** 0 auto-fixed bugs. One judgment call on commit sequencing (documented above, no functional gap). No architectural changes requiring Rule 4.

## Issues Encountered

**Only compile/lint/build gates were run — this is NOT runtime verification** (established project convention). Specifically:
- `pnpm --dir ui run svelte-check` — error count dropped from 14 (pre-plan baseline, per `39-16-SUMMARY.md`'s amendment) to **4**, all four remaining errors in `PrinterDetail.svelte`/`PrinterSelect.svelte`/`GroupedPrinterSelect.svelte` — Plan 18's declared territory, zero new errors introduced by this plan, zero act-family errors remain (`grep -i "acts/"` on the full svelte-check output returns only pre-existing rune-locality warnings, not errors)
- `pnpm exec eslint` on all six touched files — clean
- `node scripts/check-tokens.mjs` — PASS, 0 violations
- `pnpm --dir ui build` — succeeds (647 modules, no new warnings attributable to this plan's files)

**None of the above catch Svelte 5 rune runtime errors or WKWebView-specific rendering behavior.** Runtime behavior (PlacePicker opening/selecting/clearing inside these three forms, the D-11.1 chip row's actual visual layout and click-to-select interaction, the per-row disabled-PlacePicker "effective value" display, the full act-create → bulk-return → printed-snapshot flow) is **UNVERIFIED**. A detailed manual-verification checklist has been appended to `.planning/phases/39-place-tree/deferred-items.md` under "Plan 17 — Act-family PlacePicker wiring runtime verification NOT performed", to be executed in the batched UAT pass at Plan 20/21's checkpoint or via `/gsd-verify-work`.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

`ui/src/features/acts/` now speaks `place_id`/`bulk_place_id`/`place_id_override`/`full_path`/`place_path_snapshot` exclusively — zero freeform-location surfaces remain in this directory (confirmed via `grep -i location` across all six touched files: only historical doc-comment prose referencing the pre-fix G-6/G-10 bug remains, no code). `PlacePicker`'s injection-prop contract from Plan 13 held up unmodified for all new real consumers (`ActFormBody`, `ReturnModal` x2 usages — bulk field + quick-pick, `ReturnItemsTable` per-row) — no changes to `PlacePicker.svelte` itself were needed, consistent with Plans 15/16's own findings. This plan closes the act side of D-17; Plan 18 (the remaining device-family act-adjacent surfaces, if any) and Plans 20/21 (end-to-end UAT checkpoint) are unblocked to proceed.

---
*Phase: 39-place-tree*
*Completed: 2026-08-25*

## Self-Check: PASSED

All six code files + this SUMMARY confirmed present on disk. All five referenced commit hashes
(`dce1ffc1`, `b3a77722`, `14ae4c3b`, `8921d065`, `966779a9`) confirmed present in `git log`.

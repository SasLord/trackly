---
phase: 13-per-device-junction-chip-drum-state
plan: 06
subsystem: ui
tags: [svelte, scss, autocomplete, cartridges, printers]

# Dependency graph
requires:
  - phase: 13-per-device-junction-chip-drum-state
    provides: "Plan 13-02 (compatibility DTOs switched to Vec<String>), Plan 13-03 (printers_get_compatible_aggregates command + V029 command removal), Plan 13-05 (cartridges_suggest_compat_printer dropped field param, sources from devices.name)"
provides:
  - "ModelFormModal.svelte with a single 'Совместимые принтеры' compatibility block (free-text printer names, autocomplete)"
  - "CompatibilityEditor.svelte reworked to single-field row editor (string[] contract)"
  - "printers.getCompatibleAggregates() api.ts wrapper for the read-only printer-card aggregate block"
  - "cartridges/api.ts and printers/api.ts cleaned of dead V029 junction wrappers"
affects: ["13-07 (printer card aggregates block, deletes CompatibleModelsEditor.svelte)", "13-08 (OperationModal chip-task fix, must also fix its compat-narrowing call sites)"]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "CompatibilityEditor.svelte: single suggestFn(prefix) prop replaces dual suggestBrandFn/suggestModelFn — one input per row, free-text allowed even with no autocomplete match (D-04)"
    - "ModelFormModal compatibility submit: trim + dedupe via Array.from(new Set(...)) before payload, no per-field validation gate (any non-empty trimmed string is valid)"

key-files:
  created: []
  modified:
    - ui/src/features/cartridges/CompatibilityEditor.svelte
    - ui/src/features/cartridges/ModelFormModal.svelte
    - ui/src/features/cartridges/api.ts
    - ui/src/features/printers/api.ts
  deleted:
    - ui/src/features/cartridges/CompatibleDevicesEditor.svelte

key-decisions:
  - "filteredCompatibility (not raw compatibility) is sent in the submit payload — plan's own <action> text explicitly specifies this trimmed+deduped variable name, even though the key_links regex in frontmatter literally reads 'compatibility:\\s*compatibility'. Followed the more specific <action> instruction."
  - "Logged (not fixed) two out-of-scope compile breaks surfaced by Task 1's api.ts cleanup: CompatibleModelsEditor.svelte (printers, Plan 13-07 scope per UI-SPEC) and OperationModal.svelte's compat-narrowing call sites. Confirmed via git-stash diff that both were already broken before this plan (missing-type errors became missing-property errors) — not caused by 13-06, outside its files_modified list."

requirements-completed: [SPEC-13-R3]

# Metrics
duration: 25min
completed: 2026-06-26
---

# Phase 13 Plan 06: ModelFormModal Compatibility Collapse Summary

**ModelFormModal now shows exactly one "Совместимые принтеры" block — a free-text printer-name list editor with autocomplete, replacing the old dual brand/model pair editor plus per-device checklist.**

## Performance

- **Duration:** 25 min
- **Started:** 2026-06-26T00:50:00Z
- **Completed:** 2026-06-26T01:13:00Z
- **Tasks:** 2
- **Files modified:** 4 (1 deleted)

## Accomplishments
- Collapsed two compatibility blocks in `ModelFormModal.svelte` down to one, matching the V032/Phase-13 single-column `compatibility: string[]` data model (R3)
- `CompatibilityEditor.svelte` reworked from `(printer_brand, printer_model)` pair rows to single free-text "printer name" rows, with one `suggestFn(prefix)` autocomplete prop
- Deleted `CompatibleDevicesEditor.svelte` (V029 per-device checklist artifact, fully superseded)
- Cleaned `cartridges/api.ts` and `printers/api.ts` of dead wrappers for the four removed Tauri/HTTP commands (`cartridge_models_get/set_compatible_devices`, `printers_get/set_compatible_models`); added `printers.getCompatibleAggregates()` for the upcoming read-only printer-card block (Plan 13-07)
- `suggestCompatPrinter` call sites updated to the new single-`prefix` signature (the stale `field` argument is gone)

## Task Commits

Each task was committed atomically:

1. **Task 1: api.ts — удалить обёртки удалённых команд, обновить suggestCompatPrinter** - `f44a90d` (refactor)
2. **Task 2: CompatibilityEditor — переход на одно поле (имя принтера) + удаление CompatibleDevicesEditor** - `1fe3657` (feat)

**Plan metadata:** see Self-Check / final commit below.

_Note: an additional docs commit (`dc56a78`) logs an out-of-scope finding to `deferred-items.md` — not a plan task, but recorded per the deviation-tracking discipline._

## Files Created/Modified
- `ui/src/features/cartridges/api.ts` - removed `modelsGetCompatibleDevices`/`modelsSetCompatibleDevices`; `suggestCompatPrinter` now takes only `prefix`
- `ui/src/features/printers/api.ts` - removed `getCompatibleModels`/`setCompatibleModels`; added `getCompatibleAggregates(deviceId)` wrapping `printers_get_compatible_aggregates`
- `ui/src/features/cartridges/CompatibilityEditor.svelte` - single-field row editor (`rows: string[]`), single `suggestFn` prop, simplified `getKey`/autocomplete state (no more brand/model field discrimination)
- `ui/src/features/cartridges/ModelFormModal.svelte` - `compatibility: string[]` state (direct assignment, no pair-mapping); removed the second "Совместимые принтеры (по справочнику устройств)" block and its `CompatibleDevicesEditor` import; submit payload trims+dedupes via `Array.from(new Set(...))`; added UI-SPEC empty-state copy ("Совместимость не задана — картриджи этой модели подходят к любому принтеру.")
- `ui/src/features/cartridges/CompatibleDevicesEditor.svelte` - **deleted** (V029 per-device checklist, superseded)

## Decisions Made
- Sent `filteredCompatibility` (trimmed, deduped) in the create/update payload rather than the raw `compatibility` array — the plan's `<action>` text for Task 2 explicitly names this variable; the frontmatter `key_links` regex pattern (`compatibility:\s*compatibility`) is a looser/stale hint, not a literal requirement, so the more specific `<action>` instruction took precedence.
- Did not add a `compat-header` column-label row to the reworked editor — UI-SPEC leaves single-column header markup "на усмотрение"; one column doesn't need a header per the plan's own action text, and removing it simplifies the component without losing the established visual style (`.compat-row`/`.compat-field`/`.remove-btn` classes kept verbatim).

## Deviations from Plan

### Auto-fixed Issues

None — Task 1 and Task 2 matched their `<action>` instructions exactly; no bugs, missing functionality, or blocking issues required an unplanned code fix within this plan's `files_modified` scope.

### Logged, not fixed (out-of-scope discoveries)

**1. [Rule 3 boundary — out-of-scope, NOT fixed] `CompatibleModelsEditor.svelte` and `OperationModal.svelte` reference removed compat-junction commands**
- **Found during:** Task 1 verification (`pnpm exec svelte-check` after the api.ts cleanup)
- **Issue:** `ui/src/features/printers/CompatibleModelsEditor.svelte` calls `printers.getCompatibleModels`/`setCompatibleModels` (both removed in Task 1); `ui/src/features/cartridges/OperationModal.svelte` calls `printers.getCompatibleModels` (~line 301) and `cartridges.modelsGetCompatibleDevices` (~line 328), also removed. Both are TypeScript compile errors (6 total `svelte-check` errors).
- **Why not fixed here:** Neither file is in plan 13-06's `files_modified` list. `CompatibleModelsEditor.svelte` is explicitly marked **Delete** in `13-UI-SPEC.md`'s Component Inventory, scoped to **Plan 13-07** (printer-card aggregates block replaces it). `OperationModal.svelte`'s compat-narrowing call sites belong to whichever plan next touches that file (13-08 per the UI-SPEC inventory, or a gap-closure pass) — they need to be re-pointed at `printers.getCompatibleAggregates`/`cartridges.modelsList()` plus a printer-name filter to match the new `Vec<String>` contract (D-04/D-05), which is a design decision outside this plan's R3 scope.
- **Confirmed pre-existing:** Verified via `git stash` + checking out the pre-Task-1 `api.ts` files that both errors already existed before this plan started (5 `svelte-check` errors then, for a different reason — TS couldn't resolve `PrinterCompatibleModelsDto`/`CartridgeModelCompatibleDevicesDto`, which Plan 13-02/13-03 had already removed from `bindings.ts`). Task 1 only changed the error's shape (missing-type → missing-property), not its existence.
- **Files referenced (not modified):** `ui/src/features/printers/CompatibleModelsEditor.svelte`, `ui/src/features/cartridges/OperationModal.svelte`
- **Tracked in:** `.planning/phases/13-per-device-junction-chip-drum-state/deferred-items.md` (new "From Plan 13-06" section)
- **Commit:** `dc56a78` (docs only, logs the finding)

---

**Total deviations:** 0 auto-fixed; 1 out-of-scope discovery logged (not fixed, per scope-boundary rule)
**Impact on plan:** None on 13-06's own deliverable — `ModelFormModal.svelte` correctly shows exactly one compatibility block and builds clean. The logged item is a known, pre-existing gap that the next two plans in this phase (13-07, 13-08) are positioned to close.

## Issues Encountered

`cargo tsc --noEmit`/`svelte-check` both report a non-zero exit code at the whole-project level because of (a) the logged out-of-scope `CompatibleModelsEditor.svelte`/`OperationModal.svelte` errors and (b) one unrelated pre-existing error in `ui/src/features/acts/returnPayload.ts` (`ReturnRowState` export resolution — last touched in an unrelated Phase 4 commit, untouched by any Phase 13 plan). None of these are in files this plan modified. `pnpm --dir ui build` (the actual Vite/Rollup production build, which does not type-check) succeeds cleanly — confirmed twice, after Task 1 and after Task 2.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

`ModelFormModal.svelte` is ready for live use with the new `Vec<String>` compatibility contract. Plan 13-07 (printer-card aggregates + device-data block) can now safely delete `CompatibleModelsEditor.svelte` and wire `printers.getCompatibleAggregates()` (already added here) into `PrinterDetail.svelte`. Plan 13-08 (OperationModal chip-task fix) should additionally re-point the two narrowing call sites flagged in `deferred-items.md` while it's already touching that file, closing the last two `svelte-check` errors left from the V029 teardown.

---
*Phase: 13-per-device-junction-chip-drum-state*
*Completed: 2026-06-26*

## Self-Check: PASSED

All claimed files verified present/absent as documented; all 4 commits
(`f44a90d`, `1fe3657`, `dc56a78`, `3676d43`) verified in `git log --oneline --all`.

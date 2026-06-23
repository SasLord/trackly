---
phase: 12-cartridge-request-interconnection
plan: 08
subsystem: ui
tags: [svelte, cartridges, printers, compatibility-filter]

# Dependency graph
requires:
  - phase: 12-cartridge-request-interconnection
    provides: "12-05's CartridgeFilter.compatible_with_printer_device_id backend predicate (D-13/D-14 single-SQL narrowing); 12-07's printers.getCompatibleModels() API wrapper"
provides:
  - "OperationModal.svelte install picker filters cartridge options by real printer<->model compatibility (printer_cartridge_models) instead of the old WR-02 model_id-only placeholder"
  - "Install picker always excludes photo-drums (kind_id=1 hardcoded, kind_id: null fully removed)"
  - "New D-14 warning 'Совместимость не задана — проверьте вручную' shown exactly when a printer context exists with zero configured compatibility links"
affects: [cartridge-install-flow, request-detail-install-flow]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Two-stage compatibility check: one $effect calls printers.getCompatibleModels() purely to decide whether to show the D-14 warning (UX hint), while a separate $effect drives the actual filtered cartridges.list() call — the backend SQL predicate self-adjusts for the no-links case, so the warning-decision and the list-filtering are deliberately decoupled effects."

key-files:
  created: []
  modified:
    - ui/src/features/cartridges/OperationModal.svelte

key-decisions:
  - "compatibilityUnconfigured ($state<boolean>) replaces the old noModelScopeWarning derived value — gated on preFillPrinterId !== undefined (no printer context at all means no compatibility relationship to be missing, so no warning shown in that case, matching the old entry point's behavior)"
  - "Fail-safe default: if printers.getCompatibleModels() call errors (e.g. permission denial), compatibilityUnconfigured defaults to false — this is a UX hint, not a security boundary (T-12-08-01)"
  - "kind_id: null replaced with hardcoded kind_id: 1 in the install list-loading call — permanently excludes photo-drums from the install picker, no longer conditional on cartridgeModelId"
  - "model_id: cartridgeModelId ?? null kept alongside the new compatible_with_printer_device_id filter — both narrow by the same model axis, SQL AND semantics make the narrower of the two win with no special-casing needed"

patterns-established: []

requirements-completed: [D-13, D-14]

# Metrics
duration: 15min
completed: 2026-06-23
---

# Phase 12 Plan 08: Compatibility-Aware Install Picker Filter Summary

**Install picker in OperationModal.svelte now filters cartridge options by real printer↔model compatibility links (printer_cartridge_models) instead of the old model_id-only WR-02 placeholder, and always excludes photo-drums.**

## Performance

- **Duration:** ~15 min
- **Started:** 2026-06-23T00:47:36Z (per STATE.md session continuity)
- **Completed:** 2026-06-23T00:50:50Z
- **Tasks:** 1
- **Files modified:** 1

## Accomplishments

- Replaced the WR-02 placeholder warning (`noModelScopeWarning`, fired on missing `cartridgeModelId`) with a real D-13/D-14 compatibility check (`compatibilityUnconfigured`, fired on zero `printer_cartridge_models` links for the request's printer).
- Wired `compatible_with_printer_device_id: preFillPrinterId ?? null` into the install picker's `cartridges.list()` filter, activating 12-05's backend self-adjusting compatibility predicate.
- Hardcoded `kind_id: 1` in the install list-loading call, permanently excluding photo-drums (previously `kind_id: null`, unfiltered).
- New warning text "Совместимость не задана — проверьте вручную" (D-14 exact wording) renders only when a printer context exists (`preFillPrinterId !== undefined`) but has zero configured compatibility links.

## Task Commits

1. **Task 1: Replace placeholder filter/warning with compatibility-aware filter** - `9ebc38e` (feat)

**Plan metadata:** (this commit, below)

_Note: tdd="true" was set on the task, but per the task's own `<behavior>` block the four described tests are interaction-level Svelte component behaviors (effect timing, network-call argument shape) without an existing component-test harness in this codebase (no `*.test.ts`/Vitest setup found for `.svelte` files in `ui/src/features/cartridges/`). Verification was performed via the plan's own acceptance criteria (grep-based assertions + `svelte-check` + `pnpm build`), which directly encode the same Test 1-4 expectations (filter args present/absent, warning text present/absent, `kind_id: 1` always passed). No RED/GREEN test commits were created — see TDD Gate Compliance below._

## Files Created/Modified

- `ui/src/features/cartridges/OperationModal.svelte` - Install picker (`op === 'install' && cartridge === null`) now loads `cartridges.list()` with `kind_id: 1` (always) and `compatible_with_printer_device_id: preFillPrinterId ?? null`; new `compatibilityUnconfigured` state driven by a `printers.getCompatibleModels()` effect; old `noModelScopeWarning` derived value and its warning text fully removed.

## Decisions Made

- See `key-decisions` in frontmatter above.
- 12-07 had already landed `printers.getCompatibleModels()` in `ui/src/features/printers/api.ts` by the time this plan executed (same wave, dependency satisfied) — the plan's defensive inline `apiCall` fallback was not needed; imported `printers` directly from `../printers/api`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Resolved a pre-existing svelte-check error in OperationModal.svelte as a side effect of this task's own required change**
- **Found during:** Task 1 verification (`pnpm --dir ui exec svelte-check`)
- **Issue:** `deferred-items.md` (Plan 12-07 entry) documented a pre-existing `svelte-check` error at `OperationModal.svelte:143` — the old list-loading filter object was missing the `compatible_with_printer_device_id` field required by `CartridgeFilter` (added in 12-05). This plan's own task action (adding that exact field to the same filter object) was the prescribed fix.
- **Fix:** No extra action needed — the task's planned change inherently supplies the missing field.
- **Files modified:** `ui/src/features/cartridges/OperationModal.svelte` (same file/lines as the planned task change).
- **Verification:** `pnpm --dir ui exec svelte-check` no longer reports any error for `OperationModal.svelte` (confirmed via `grep -i "OperationModal"` on the check output — zero matches).
- **Committed in:** `9ebc38e` (Task 1 commit).

**2. [Documentation hygiene] Updated `deferred-items.md` to reflect resolution**
- **Found during:** Post-task verification.
- **Issue:** `deferred-items.md`'s Plan 12-07 entry described both `OperationModal.svelte:143` and `CartridgesPage.svelte:60` as unresolved twin errors. After this plan, only the latter remains unresolved.
- **Fix:** Appended a note to the existing entry recording that `OperationModal.svelte`'s instance is resolved by Plan 12-08, while `CartridgesPage.svelte:60` remains out of scope (this plan's `files_modified` only lists `OperationModal.svelte`).
- **Files modified:** `.planning/phases/12-cartridge-request-interconnection/deferred-items.md`.
- **Committed in:** included in the plan-metadata commit (docs-only change, not a task commit).

---

**Total deviations:** 2 (1 incidental fix via planned change, 1 documentation update)
**Impact on plan:** No scope creep — the svelte-check resolution was an automatic consequence of the planned task action, not separate work. `CartridgesPage.svelte:60` remains correctly deferred (out of scope per `files_modified`).

## TDD Gate Compliance

The task frontmatter declared `tdd="true"` and the plan body included a `<behavior>` block describing 4 test scenarios. No dedicated component-test harness exists in this codebase for `.svelte` files under `ui/src/features/cartridges/` (no Vitest/Testing-Library setup found), so no `test(...)`/`feat(...)` RED/GREEN commit pair was produced. Instead, the plan's own `<acceptance_criteria>` (grep assertions for `compatible_with_printer_device_id`, `kind_id: 1`, absence of `kind_id: null` and the old warning text, presence of the new warning text, plus `svelte-check`/`pnpm build` clean) were used as the verification gate, and each directly maps to one of the 4 described test scenarios:
- Test 1 (D-13 configured) -> `compatible_with_printer_device_id` present in filter args (grep, satisfied).
- Test 2 (D-14 unconfigured) -> new warning text present verbatim (grep, satisfied).
- Test 3 (cartridge-centric entry, no warning) -> warning effect gated on `cartridge === null`, mirroring the list-loading guard (code-reviewed, satisfied).
- Test 4 (photo-drum exclusion) -> `kind_id: 1` always passed, `kind_id: null` fully removed (grep, satisfied).

A single `feat(12-08): ...` commit was made for this task. No separate `test(...)` commit exists. This is a deviation from the canonical TDD gate sequence; flagging here per the gate-enforcement instructions rather than silently skipping it.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- GAP-12-02's frontend half (D-13/D-14) is now closed: `OperationModal.svelte`'s request-centric install picker correctly narrows by printer compatibility when configured, falls back with a clear warning when not, and never shows photo-drums.
- Remaining Phase 12 gap-closure plans (per roadmap, 12-09 etc.) are unaffected by this change — this plan touched only `OperationModal.svelte`.
- `CartridgesPage.svelte:60`'s pre-existing svelte-check error remains deferred and tracked in `deferred-items.md` for whichever future plan next touches that file's filter construction.

---
*Phase: 12-cartridge-request-interconnection*
*Completed: 2026-06-23*

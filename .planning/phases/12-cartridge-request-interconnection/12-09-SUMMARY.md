---
phase: 12-cartridge-request-interconnection
plan: 09
subsystem: ui
tags: [svelte5, runes, rust, rusqlite, specta, cartridges, install-flow]

# Dependency graph
requires:
  - phase: 12-cartridge-request-interconnection
    provides: "12-06's backend auto-return logic (hardcoded state_id=3/location='' defaults) and 12-08's compatibility-aware install picker filter in the same file"
provides:
  - "CartridgeTransitionOp/Payload::Install accepts optional previous_cartridge_state_id/previous_cartridge_location overrides"
  - "OperationModal.svelte install flow shows an editable «Предыдущий картридж» block when the target printer already has a cartridge В работе"
  - "D-16 fully closed (backend auto-return from 12-06 + frontend visibility/editability from this plan)"
affects: [cartridge-request-interconnection, future cartridge-lifecycle plans touching Install payload shape]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Override-with-fallback Option<T> fields on transition payloads: backend keeps unwrap_or() defaults for backward compat, frontend supplies overrides only when relevant data is present"
    - "Single-transaction multi-entity update exposed through one DTO: previous-cartridge edits piggyback on the same transition() call instead of a second request"

key-files:
  created: []
  modified:
    - crates/trackly-core/src/domain/cartridges.rs
    - crates/trackly-app/src/dto/cartridge.rs
    - crates/trackly-infra/src/repos/cartridges_sqlite.rs
    - crates/trackly-app/tests/cartridges_lifecycle.rs
    - ui/src/features/cartridges/OperationModal.svelte

key-decisions:
  - "Reused the existing Select component (value+onchange) instead of a raw <select bind:value> for the new charge-state field, matching this file's own established convention (op-state field a few lines below) rather than the plan's literal bind:value suggestion"
  - "previous-cartridge lookup runs only when op==='install' && cartridge===null && preFillPrinterId!==undefined, avoiding any API call for the old cartridge-centric entry (D-08) or when there is no printer context at all"

patterns-established:
  - "When widening a transition payload with optional override fields, always update every existing Rust struct-literal construction site (not just match arms with `..`), since #[serde(default)] only covers JSON deserialization, not literal construction"

requirements-completed: [D-16]

# Metrics
duration: ~55min
completed: 2026-06-23
---

# Phase 12 Plan 09: Previous-cartridge override block in install form Summary

**Install form now shows and lets the user edit the auto-returned previous cartridge's charge state and location, with both values flowing into the existing single `cartridges_transition` call via two new optional payload fields.**

## Performance

- **Duration:** ~55 min
- **Started:** 2026-06-23T08:00:00Z (approx, prior session)
- **Completed:** 2026-06-23T08:55:00Z (approx)
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments
- `CartridgeTransitionOp::Install` / `CartridgeTransitionPayload::Install` gained `previous_cartridge_state_id: Option<i64>` and `previous_cartridge_location: Option<String>`, threaded through `cartridges_sqlite.rs`'s auto-return UPDATE and audit payload via `unwrap_or(3)`/`unwrap_or("")` fallback, preserving 12-06's original hardcoded behavior when absent
- `OperationModal.svelte`'s install flow (request-centric entry) now looks up the target printer's current cartridge (`printers.get` → `cartridges.get`) and, when one exists, renders a read-only code+model line plus an editable "Состояние заряда" (default Пустой/3) and "Расположение" (default empty) — both values forwarded in the same `buildPayload()` install branch, no second API call
- D-16 is now fully closed end-to-end: 12-06 built the backend auto-return transaction, this plan made its defaults visible and user-editable in the UI

## Task Commits

Each task was committed atomically:

1. **Task 1: Thread previous-cartridge overrides through the Install payload (backend touch-up)** - `4cc9500` (feat)
2. **Task 2: Previous-cartridge UI block in OperationModal.svelte** - `0707a6f` (feat)

**Plan metadata:** (final docs commit recorded below)

_TDD: both tasks had `tdd="true"`; tests were written alongside the implementation and verified passing before each commit (RED/GREEN not split into separate commits since the plan's task granularity bundled behavior+implementation per task, consistent with prior 12-0x plans in this phase)._

## Files Created/Modified
- `crates/trackly-core/src/domain/cartridges.rs` - Added `previous_cartridge_state_id`/`previous_cartridge_location` optional fields to `CartridgeTransitionOp::Install`; updated 3 test construction sites
- `crates/trackly-app/src/dto/cartridge.rs` - Mirrored the two fields onto `CartridgeTransitionPayload::Install` with `#[specta(type = Option<i32>)]` + `#[serde(default)]`; updated the `From` impl's Install arm; added 2 new tests (forwarding + JSON-omitted defaulting)
- `crates/trackly-infra/src/repos/cartridges_sqlite.rs` - Auto-return block now resolves `previous_cartridge_state_id.unwrap_or(3)` / `previous_cartridge_location.as_deref().unwrap_or("")` instead of hardcoded literals, used in both the UPDATE statement and the audit `ReturnToStock` payload_json
- `crates/trackly-app/tests/cartridges_lifecycle.rs` - Updated all 10 existing `Install { .. }` construction sites with the two new `None` fields; added 2 new tests (`install_auto_return_uses_previous_cartridge_overrides_when_present`, `install_auto_return_falls_back_to_defaults_when_overrides_absent`)
- `ui/src/features/cartridges/OperationModal.svelte` - New `$state` (`previousCartridge`, `previousCartridgeStateId`, `previousCartridgeLocation`), new lookup `$effect`, new `{#if previousCartridge}` template block with read-only code/model + editable Select + LocationAutocomplete, `buildPayload()` install branch extended, new `.field-full`/`.previous-cartridge-block` SCSS
- `ui/src/bindings.ts` - Regenerated via `cargo test -p trackly-app --test export_bindings` (gitignored, not committed) to reflect the widened `CartridgeTransitionPayload::Install` shape

## Decisions Made
- Used the project's existing `Select` component (`value` prop + `onchange` callback) rather than a raw `<select bind:value>` for the new charge-state dropdown, matching the established convention already present a few lines below in the same file (`op-state` field) and avoiding Svelte's native-select string/number coercion footgun that a prior `CartridgeFilters.svelte` comment explicitly calls out
- Confirmed the plan's `<interfaces>` claim that `printer_device_id` lacked `#[serde(default)]` was stale (the actual code already had it); followed the actual code rather than the plan's outdated assumption, and applied the same `#[serde(default)]` convention to both new fields for consistency

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Used Select component instead of raw `<select bind:value>` as literally suggested in the plan's action text**
- **Found during:** Task 2
- **Issue:** The plan's action text suggested `<select bind:value={previousCartridgeStateId}>`, but this file's own established pattern (and a documented codebase-wide convention seen in `CartridgeFilters.svelte`) uses the shared `Select` component with `value`+`onchange`+explicit `parseInt`, specifically to avoid a known Svelte native-select numeric-value matching bug ("Числовые value (не строковые): Svelte select_option сравнивает строго...")
- **Fix:** Implemented the new charge-state field with `<Select value={String(previousCartridgeStateId)} onchange={(v) => (previousCartridgeStateId = parseInt(v, 10))}>` instead, matching the existing `op-state` field's exact pattern in the same file
- **Files modified:** ui/src/features/cartridges/OperationModal.svelte
- **Verification:** `pnpm --dir ui exec svelte-check` and `pnpm --dir ui build` both pass with zero new errors
- **Committed in:** 0707a6f (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 bug-prevention substitution)
**Impact on plan:** Necessary correctness fix consistent with established codebase conventions; no scope creep, no behavior change from the plan's intent.

## Issues Encountered
None - both tasks compiled, tested, and built cleanly with the one auto-fix noted above.

## User Setup Required
None - no external service configuration required.

## Known Stubs
None - the previous-cartridge block is fully wired: lookup via real `printers.get`/`cartridges.get` API calls, edits flow into the real `transition()` payload, backend persists overridden values via real SQL UPDATE.

## Threat Flags

No new threat surface beyond what the plan's own `<threat_model>` already declared (T-12-09-01, T-12-09-02 — both pre-assessed as no-new-exposure parity with existing `ReturnToStock.state_id`/`.location` fields, which have always lacked range validation and always used bound params respectively).

## Next Phase Readiness
- D-16 is fully closed (backend + frontend). Phase 12's gap-closure plans (GAP-12-01..03) are now all resolved: GAP-12-01 (autocomplete names), GAP-12-02 (printer→cartridge compatibility), GAP-12-03 (return previous cartridge — backend in 12-06, frontend in this plan).
- `CartridgesPage.svelte:60`'s pre-existing `compatible_with_printer_device_id` svelte-check error remains open and documented in `deferred-items.md` — out of scope for every plan in this phase so far; belongs to whichever future plan next touches that file's filter construction.
- Phase 12 appears ready for a final phase-level review/close-out given all three UAT gaps are now resolved.

---
*Phase: 12-cartridge-request-interconnection*
*Completed: 2026-06-23*

## Self-Check: PASSED

All created/modified files verified present on disk; all referenced commit hashes (4cc9500, 0707a6f, 21adf7e) verified present in `git log`.

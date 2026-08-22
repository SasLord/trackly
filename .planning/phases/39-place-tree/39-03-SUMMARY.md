---
phase: 39-place-tree
plan: 03
subsystem: database
tags: [rust, domain-types, sqlite, refactor]

# Dependency graph
requires:
  - phase: 39-place-tree (Plan 01)
    provides: V037/V038 migrations — `places` table, `place_full_paths` view, devices/acts/cartridges moved to `place_id`, `locations` dropped
  - phase: 39-place-tree (Plan 02)
    provides: PlaceKind, PlaceRow/PlaceNew/PlacePatch, PlaceRepository port, D-20 auth split
provides:
  - "acts.rs domain structs (ActNew/ActPatch/ActReturnNew/ActReturnItem/ActRow) renamed to place_id/bulk_place_id/place_id_override, plus new ActRow.place_path_snapshot field for D-16 print fidelity"
  - "cartridges.rs domain structs (CartridgeRow/CartridgeNew/all five CartridgeTransitionOp variants) renamed location -> place_id per D-12/D-13, with CartridgeRow gaining a separate full_path display field"
  - "printers.rs PrinterRow gains device_place (display) + device_place_id (raw id, for PlacePicker prefill)"
  - "requests.rs request row field renamed printer_location -> printer_place (pass-through display, Phase 42 owns semantics)"
affects: [39-06, 39-07, 39-09, 39-10, 39-11, 39-22]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Domain rows distinguish a live-resolved display field (full_path, joined from place_full_paths on every read) from a frozen print-time snapshot (place_path_snapshot, act-level only, D-16)"
    - "CartridgeTransitionOp variants now carry Option<i64> place_id instead of a required String location, matching D-07's 'place is optional' invariant"

key-files:
  created: []
  modified:
    - crates/trackly-core/src/domain/acts.rs
    - crates/trackly-core/src/domain/cartridges.rs
    - crates/trackly-core/src/domain/printers.rs
    - crates/trackly-core/src/domain/requests.rs

key-decisions:
  - "ActRow carries both full_path (live-resolved current path) and place_path_snapshot (frozen at write time) as two distinct fields per D-16 — never conflated"
  - "CartridgeTransitionOp.place_id changed from required String to Option<i64> even though the field wasn't previously optional, since D-13 lets a caller omit it and let cartridge_service.rs (Plan 09) apply the Install-default (printer's place)"
  - "PrinterRow gained device_place_id (new field, not a rename) because a display string alone can't drive PlacePicker's numeric-id binding in the Install operation modal (Plan 16)"

requirements-completed: [PLC-04]

# Metrics
duration: 12min
completed: 2026-08-22
---

# Phase 39 Plan 03: Domain-layer place_id rename Summary

**Renamed every location-bearing field in acts.rs, cartridges.rs, printers.rs, requests.rs to its place_id-based equivalent, matching the V038 schema column names exactly — no SQL/service changes.**

## Performance

- **Duration:** 12 min
- **Started:** 2026-08-22T19:35:00Z
- **Completed:** 2026-08-22T19:47:21Z
- **Tasks:** 3
- **Files modified:** 4

## Accomplishments
- `acts.rs`: every `location_id`/`bulk_location_id`/`location_id_override` field renamed to its `place_id` equivalent; `ActRow.location` split into `full_path` (live-resolved) and a new `place_path_snapshot` field (frozen print-time snapshot, D-16)
- `cartridges.rs`: `CartridgeRow.location` (single freeform text) split into `place_id: Option<i64>` + `full_path: Option<String>`; `CartridgeNew.location` renamed to `place_id`; all five `CartridgeTransitionOp` variants (Install/ReturnToStock/ToRefill/FromRefill/WriteOff) now carry `place_id: Option<i64>` instead of a required `location: String`; `previous_cartridge_location` renamed to `previous_cartridge_place_id`; test fixtures updated to placeholder `place_id` integers
- `printers.rs`: `PrinterRow.device_location` renamed to `device_place`; new `device_place_id: Option<i64>` field added for `PlacePicker` prefill
- `requests.rs`: request row's `printer_location` renamed to `printer_place` (pass-through display string, no semantic change)
- `cargo build -p trackly-core` succeeds

## Task Commits

Each task was committed atomically:

1. **Task 1: domain/acts.rs — location_id family → place_id family + place_path_snapshot** - `faf84e56` (feat)
2. **Task 2: domain/cartridges.rs — location:String → place_id:Option<i64> on all 5 transition ops** - `7cd3b00e` (feat)
3. **Task 3: domain/printers.rs + domain/requests.rs — mechanical field rename only** - `1f94d9aa` (feat)

_Note: no TDD tasks in this plan (pure domain-struct rename, tdd_mode=false project-wide)._

## Files Created/Modified
- `crates/trackly-core/src/domain/acts.rs` — `ActNew`, `ActReturnNew`, `ActReturnItem`, `ActPatch`, `ActRow` field renames + new `place_path_snapshot` field
- `crates/trackly-core/src/domain/cartridges.rs` — `CartridgeRow`, `CartridgeNew`, `CartridgeTransitionOp` (5 variants) field renames + new `full_path` field + test fixture updates
- `crates/trackly-core/src/domain/printers.rs` — `PrinterRow.device_location` → `device_place` + new `device_place_id` field
- `crates/trackly-core/src/domain/requests.rs` — request row's `printer_location` → `printer_place`

## Decisions Made
- `ActRow.full_path` and `ActRow.place_path_snapshot` deliberately kept as two separate fields — `full_path` reflects current tree state (recomputed every read via `place_full_paths`), `place_path_snapshot` is frozen at act-creation/write time for D-16 print fidelity. They must never be merged into one field.
- `CartridgeTransitionOp.place_id` widened from a required `String` to `Option<i64>` (not just a type swap) because D-13 and D-07 ("place is optional") mean a caller can now omit the place and let the service layer (Plan 09) apply a kind-aware default.
- `PrinterRow.device_place_id` is a net-new field (not a rename of `device_location`) since a display string alone cannot drive `PlacePicker`'s id-bound selection in the Install operation modal.
- `requests.rs`'s `printer_place` rename is mechanical only — Phase 42 owns any reinterpretation of the field's meaning per the CONTEXT.md deferred-scope note.

## Deviations from Plan

None - plan executed exactly as written. All three tasks matched their `<action>` blocks and acceptance criteria without requiring Rule 1-4 intervention.

## Issues Encountered
None. `cargo build -p trackly-core` succeeded on first attempt after all three tasks. As documented in the plan's `<verification>` section, `cargo build -p trackly-app` is now expected to fail (call sites in `act_service.rs`/`cartridge_service.rs`/`device_service.rs`/`report_service.rs`/`request_service.rs` still reference the old field names) — this is intentional and owned by Plans 06/07/09/10/11 in Wave 3; not attempted or verified in this plan per the prior-wave-context scope boundary.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- `trackly-core` domain layer now speaks `place_id`/`bulk_place_id`/`place_id_override`/`place_path_snapshot`/`full_path`/`device_place`/`device_place_id`/`printer_place` throughout `acts.rs`, `cartridges.rs`, `printers.rs`, `requests.rs`.
- Wave 3 plans (06 devices, 07 acts, 09 cartridges, 10 reports, 11 requests) can now migrate their respective `*_sqlite.rs` repos and `*_service.rs` call sites against a stable, final domain-struct shape without re-deriving field names.
- No blockers. `trackly-app` remains intentionally red until Wave 3 plans land — this is the documented, expected state per this plan's `<verification>` block.

---
*Phase: 39-place-tree*
*Completed: 2026-08-22*

## Self-Check: PASSED

All 4 modified source files and the SUMMARY.md itself exist on disk; all 3 task commit hashes (faf84e56, 7cd3b00e, 1f94d9aa) verified present in git log.

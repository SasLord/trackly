---
phase: 40-movement-history
plan: 13
subsystem: api
tags: [rust, rusqlite, place-service, tauri, axum, place-movements, bulk-move, tdd]

# Dependency graph
requires:
  - phase: 40-movement-history (plan 05)
    provides: "SqlitePlaceMovementsRepository::record_movement_if_applicable — the single D-01 write-side entry point owning the D-04/D-06 skip guard"
  - phase: 40-movement-history (plan 12)
    provides: "the build_* function delegation shape used by both Tauri and axum transports, so a single build_places_* function is the one gate for both"
provides:
  - "PlaceService::move_subtree_contents(caller, root_id, target_place_id, note) -> Result<usize, AppError> — D-28's atomic bulk relocation of a subtree's devices/printers/cartridges"
  - "build_places_move_subtree_contents (tauri_cmds/places.rs) — the shared gate both transports delegate to"
  - "places_move_subtree_contents Tauri command + handler_move_subtree_contents axum handler, both wired and registered (specta_export.rs, http/places.rs router)"
  - "place_movements_bulk_move.rs — Wave 0 test file: success, D-04 skip, atomicity-on-failure, Employee 403 on both transports"
affects: [40-19]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Seventh (and final) place_movements write site: single self.writer.execute transaction wraps list_subtree_contents(&tx, root_id, nested=true) + a per-item UPDATE+record_movement_if_applicable loop, committed once at the end"
    - "Devices/printers reuse SqliteDeviceRepository::update_status_and_place_in_tx, feeding back the item's own pre-fetched status_id so the bulk move never touches status — no caller-supplied expected version (system-initiated bulk op, not a user optimistic-lock edit)"
    - "Cartridges use a direct in-tx UPDATE (place_id/updated_at_utc/version+1, no version check) mirroring cartridge_service::update's shape, since no dedicated cartridges_sqlite _in_tx setter exists for place-only changes"
    - "WR-05-style fault injection for atomicity tests: a BEFORE UPDATE ... WHEN NEW.id = ? trigger RAISE(ABORT)s on the LAST item in the walk order (a cartridge — device rows are UNIONed before cartridge rows in list_subtree_contents), proving the whole transaction rolls back rather than asserting it in a comment"

key-files:
  created:
    - crates/trackly-app/tests/place_movements_bulk_move.rs
  modified:
    - crates/trackly-app/src/services/place_service.rs
    - crates/trackly-app/src/tauri_cmds/places.rs
    - crates/trackly-app/src/http/places.rs
    - crates/trackly-app/src/specta_export.rs

key-decisions:
  - "Gated on BOTH Action::MutateDevices AND Action::MutateCartridges (D-13) rather than introducing a new Action variant — both are Admin|Manager tier, confirmed via crates/trackly-core/src/auth.rs before writing the gate, so the double-gate is not accidentally more restrictive than either alone"
  - "The Tauri command's return type is i32, not usize — tauri-specta's TS export rejects usize with BigIntForbidden; build_places_move_subtree_contents itself still returns Result<usize, AppError> (matches the plan's stated interface), the i32 cast lives only in the thin #[tauri::command] wrapper. The axum handler is unaffected (Json<usize> serializes fine via serde, specta never touches it)."
  - "build_places_move_subtree_contents double-gates authorize() at the transport-boundary layer, matching every sibling build_places_* mutation in this file (move/archive/unarchive/delete all do the same even though PlaceService's own methods already gate internally) — consistency over the plan's 'only if convention demands it' hedge, since the convention is unambiguous in this file"
  - "Employee-403 test coverage for 'both transports' follows the established Plan 40-12 precedent (report_movements.rs::report_movements_gate_denies_employee): calling build_places_move_subtree_contents directly proves the Tauri path, and a genuine axum Router + programmatic session cookie (role_endpoint_matrix.rs's harness, duplicated locally) proves the real HTTP path returns StatusCode::FORBIDDEN — not just a second call to the same build_* function"

requirements-completed: []  # HST-01 NOT marked complete here — orchestrator closes at phase end, per this plan's bookkeeping_constraint

# Metrics
duration: ~40min
completed: 2026-09-02
---

# Phase 40 Plan 13: Bulk-Move Subtree Contents Summary

**`PlaceService::move_subtree_contents` — D-28's "Перенести всё содержимое в…" bulk relocation, atomically moving every device/printer/cartridge in a place subtree and recording one `place_movements` row per real place change, wired through both Tauri and axum with a single shared gate.**

## Performance

- **Duration:** ~40 min
- **Completed:** 2026-09-02
- **Tasks:** 2/2
- **Files modified:** 4 (1 created, 4 modified — see key-files)

## Accomplishments

- `PlaceService::move_subtree_contents` — reads `list_subtree_contents(&tx, root_id, nested=true)` inside a single `self.writer.execute` transaction, then for each `PlaceContentRow` (`device`/`printer` → `devices` table via `SqliteDeviceRepository::update_status_and_place_in_tx`, `cartridge` → a direct in-tx `UPDATE cartridges`), calls `record_movement_if_applicable` before moving to the next item; commits once at the end and returns the total item count
- Gated on `Action::MutateDevices` AND `Action::MutateCartridges` (D-13) — verified both are Admin|Manager tier before writing the gate, so the subtree's mixed device/cartridge contents never see a permission mismatch
- `build_places_move_subtree_contents` (the one function both transports call), `places_move_subtree_contents` Tauri command (registered in `specta_export.rs`), and `handler_move_subtree_contents` axum handler (registered on `/api/v1/places_move_subtree_contents`) — all delegate to the same `PlaceService` method, matching 40-12's proven delegation shape
- `place_movements_bulk_move.rs`: 5 integration tests — successful multi-item move (device + printer + cartridge, nested two levels under root), D-04's "already at target" skip (item still moved/counted, zero movement row), Employee-403 on the Tauri path (`Err(AppError::Forbidden)`), Employee-403 on the real HTTP path (genuine axum `Router` + programmatic session cookie → `StatusCode::FORBIDDEN`), and an atomicity-on-failure test that injects a `BEFORE UPDATE ... RAISE(ABORT)` trigger on the last item in the walk order (a cartridge) and asserts the device processed earlier in the SAME transaction was also rolled back

## Task Commits

Each task was committed atomically:

1. **Task 1: `PlaceService::move_subtree_contents`** - `3dde20a8` (feat)
2. **Task 2: Both transport adapters + Wave 0 test file** - `efd5e135` (test)

## Files Created/Modified

- `crates/trackly-app/src/services/place_service.rs` - added `move_subtree_contents`; new imports (`MovementEntityKind`, `MovementSource`, `SqliteDeviceRepository`, `SqlitePlaceMovementsRepository`, `rusqlite::OptionalExtension`)
- `crates/trackly-app/src/tauri_cmds/places.rs` - `build_places_move_subtree_contents` (double-gated) + `places_move_subtree_contents` Tauri command (returns `i32`, not `usize` — see Deviations)
- `crates/trackly-app/src/http/places.rs` - `MoveSubtreeContentsPayload`, `handler_move_subtree_contents`, route registration
- `crates/trackly-app/src/specta_export.rs` - registered `places_move_subtree_contents` in the command list (regenerates `ui/src/bindings.ts`, which is gitignored — nothing to commit there, verified by running `export_bindings` and confirming `git status` shows no change)
- `crates/trackly-app/tests/place_movements_bulk_move.rs` - 5 tests, local `make_test_ctx`/`create_identity`/`seed_*`/`create_session_cookie` helpers (duplicated from `role_endpoint_matrix.rs` and the two sibling `place_movements_write_sites_*.rs` files rather than shared, matching this codebase's existing per-test-binary duplication convention)

## Decisions Made

See `key-decisions` in frontmatter. Summary:
- Both `MutateDevices`/`MutateCartridges` gates, no new `Action` variant (D-13)
- Tauri wrapper returns `i32` (specta/TS can't represent `usize`); the underlying `build_*`/service method keeps `usize` per the plan's stated interface
- Double-gate at the transport-boundary `build_*` layer, matching this file's unanimous existing convention
- Employee-403 coverage includes a genuine HTTP-level `StatusCode::FORBIDDEN` assertion (not just a second `build_*` call), going one step further than 40-12's precedent per this plan's explicit acceptance criteria

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Tauri command return type `usize` fails specta export**
- **Found during:** Task 2, running `cargo test -p trackly-app --test export_bindings` after wiring the new command
- **Issue:** `places_move_subtree_contents` initially returned `Result<usize, AppError>` (matching `PlaceService::move_subtree_contents`'s own signature). `tauri-specta`'s TypeScript exporter panicked: `BigIntForbidden(usize)` — specta cannot represent `usize` in the generated bindings (same reason every other tauri-command in this codebase that returns a count uses `i32`, e.g. `acts_peek_next_number`).
- **Fix:** Changed the `#[tauri::command]` wrapper's return type to `i32`, casting the `usize` result from `build_places_move_subtree_contents` at the boundary. `PlaceService::move_subtree_contents` and `build_places_move_subtree_contents` both still return `usize`/`Result<usize, AppError>` exactly as the plan's interface specifies — only the thin Tauri wrapper's signature changed. The axum handler is unaffected (`Json<usize>` serializes fine via serde; specta never inspects axum handlers).
- **Files modified:** `crates/trackly-app/src/tauri_cmds/places.rs`
- **Verification:** `cargo test -p trackly-app --test export_bindings -- --test-threads=1` — 1/1 pass; `ui/src/bindings.ts` (gitignored) regenerated cleanly with `places_move_subtree_contents({ rootId, targetPlaceId, note })`
- **Committed in:** `efd5e135` (Task 2 commit — the wrapper was written and fixed before any commit, so no separate fix-commit was needed)

---

**Total deviations:** 1 auto-fixed (1 bug, caught by the export_bindings test itself, not a runtime bug)
**Impact on plan:** No scope creep — a one-line return-type fix confined to the Tauri command wrapper, required for the codebase's existing `cargo test` suite (which includes bindings export) to stay green.

## Issues Encountered

None beyond the deviation above. `cargo build --workspace`, `cargo fmt --check` (workspace-wide), and `cargo clippy -p trackly-app --all-targets -- -D warnings` all pass clean. Full regression run across `place_movements_bulk_move`, `place_movements_write_sites_devices`, `place_movements_write_sites_cartridges`, `place_movements_act_link`, `place_movements_timeline`, `report_movements`, and `role_endpoint_matrix` (16 tests total across the movement-history family plus the full role gate matrix) — all green, no behavior drift on any pre-existing test.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- All seven Phase 40 `place_movements` write sites (device manual, cartridge manual + transition main + transition auto-return, five act sites, and this plan's bulk move) are now wired to `record_movement_if_applicable` — HST-01's write-side scope is fully closed across the phase.
- `PlaceService::move_subtree_contents` + both transport adapters are ready for Plan 40-19's UI wiring (the place-contents panel's "Перенести всё содержимое в…" action).
- HST-01 is NOT marked complete in `.planning/REQUIREMENTS.md` — left for the orchestrator to close at phase end, per this plan's `bookkeeping_constraint`.
- No blockers identified.

---
*Phase: 40-movement-history*
*Completed: 2026-09-02*

## Self-Check: PASSED

- FOUND: crates/trackly-app/src/services/place_service.rs
- FOUND: crates/trackly-app/src/tauri_cmds/places.rs
- FOUND: crates/trackly-app/src/http/places.rs
- FOUND: crates/trackly-app/src/specta_export.rs
- FOUND: crates/trackly-app/tests/place_movements_bulk_move.rs
- FOUND: .planning/phases/40-movement-history/40-13-SUMMARY.md
- FOUND commit: 3dde20a8
- FOUND commit: efd5e135

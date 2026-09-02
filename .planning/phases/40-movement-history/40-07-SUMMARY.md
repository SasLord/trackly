---
phase: 40-movement-history
plan: 07
subsystem: api
tags: [rust, rusqlite, place-movements, device-service, tdd]

# Dependency graph
requires:
  - phase: 40-movement-history (plan 03)
    provides: "device_service::update(caller: &Identity, ...) — real actor identity + before/after device rows already in scope inside the writer closure"
  - phase: 40-movement-history (plan 05)
    provides: "SqlitePlaceMovementsRepository::record_movement_if_applicable — the single D-01 write-side entry point owning the D-04/D-06 skip guard"
provides:
  - "device_service::update records a place_movements row (source='manual') on a real place->place change, inside the same transaction as the device UPDATE and audit_log INSERT"
  - "place_movements_write_sites_devices.rs — first slice of the Wave 0 write-site test suite (device family), template for the cartridge (40-08) and act (40-09) siblings"
affects: [40-08, 40-09, 40-10, 40-11]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Write-site call shape: capture before_place_id from the pre-update snapshot already fetched for audit_log, call record_movement_if_applicable with the after row's place_id, inside the same &Transaction — no new transaction, no re-derivation of the D-04/D-06 guard"

key-files:
  created:
    - crates/trackly-app/tests/place_movements_write_sites_devices.rs
  modified:
    - crates/trackly-app/src/services/device_service.rs

key-decisions:
  - "Added place_movements_repo: Arc<SqlitePlaceMovementsRepository> as a new DeviceService field (zero-sized repo, same Arc-clone-into-closure convention as printer_repo/place_repo) rather than passing it as a per-call argument — matches the existing constructor-injection pattern for all other repos on this service"
  - "Test file uses the same tempfile-DB harness as devices_crud.rs and the same seed_place raw-SQL helper as cartridges_crud.rs, seeding an invented manager identity (Иванов И.И.) rather than reusing devices_crud.rs's seed_manager_user to keep this file self-contained per the plan's file scope"

requirements-completed: []  # HST-01 not marked complete here — orchestrator closes it at phase end once all write sites (40-07/08/09) + timeline UI land; see bookkeeping_constraint

# Metrics
duration: ~35min
completed: 2026-09-02
---

# Phase 40 Plan 07: Wire Device Write Site into place_movements Summary

**`device_service::update` now writes a `place_movements` row (source='manual') on a real place-to-place change, calling the shared `record_movement_if_applicable` helper inside the existing writer transaction — first of three parallel write-site wiring plans (device/cartridge/act) to land.**

## Performance

- **Duration:** ~35 min
- **Completed:** 2026-09-02
- **Tasks:** 2/2
- **Files modified:** 2 (1 created, 1 modified)

## Accomplishments

- `DeviceService` gained a `place_movements_repo: Arc<SqlitePlaceMovementsRepository>` field, constructed in `DeviceService::new` alongside the existing `place_repo`
- `update`'s writer closure now captures `before_place_id` from the pre-update snapshot (already fetched for `audit_log.before_json`) and calls `record_movement_if_applicable(&tx, place_repo.as_ref(), MovementEntityKind::Device, id, before_place_id, after.place_id, MovementSource::Manual, None, None, user_id_opt, now)` immediately after the `audit_log` INSERT, inside the same transaction
- `MovementSource::Manual` is hard-coded at the call site — `DevicePatch` has no `source` field, so the client cannot spoof the movement source (T-40-15)
- Three new integration tests in `place_movements_write_sites_devices.rs` prove the three required behaviors: a real A→B place change inserts exactly one row with correct `source`/`from_place_id`/`to_place_id`/`user_id` (D-27); a status-only edit inserts zero rows (D-04); a first-time NULL→place assignment inserts zero rows (D-06)

## Task Commits

Each task was committed atomically:

1. **Task 1: Wire record_movement_if_applicable into device_service::update** - `e00d9d95` (feat)
2. **Task 2: Wave 0 test file — device family write-site coverage** - `53825e87` (test)

## Files Created/Modified

- `crates/trackly-app/src/services/device_service.rs` - added `place_movements_repo` field + constructor wiring; `update`'s writer closure now calls `record_movement_if_applicable` with the before/after place ids, `MovementSource::Manual`, and `user_id_opt`
- `crates/trackly-app/tests/place_movements_write_sites_devices.rs` - `place_movements_manual_device`, `place_movements_manual_device_status_only_noop`, `place_movements_manual_device_first_assignment_noop` (3/3 pass)

## Decisions Made

- Kept the `place_movements` INSERT call ordered after the existing `audit_log` INSERT (plan allowed either order since both are in the same `tx`) — this keeps the two "write a history row" concerns adjacent for readability.
- New test file seeds its own manager `Identity` (invented name "Иванов И.И.", distinct from `devices_crud.rs`'s "Петров П.П.") rather than importing a helper from another test file — Rust integration test binaries don't share code across files without a `tests/common/` module, and the plan's file scope was limited to this one new file.

## Deviations from Plan

None - plan executed exactly as written. `DeviceService` already held a `place_repo: Arc<SqlitePlaceRepository>` field from prior phases (Plan 40-03's `<read_first>` correctly anticipated checking for this), so no new `PlaceRepository` injection question arose — only the new `place_movements_repo` field needed adding.

## Issues Encountered

The plan's stated verification command (`cargo test -p trackly-app place_movements_manual_device -- --test-threads=1`, no `--test` flag) builds and enumerates every integration test binary in the `trackly-app` package before filtering by name — with ~50+ test files in this package, that full-package invocation was still compiling/running after several minutes and was terminated in favor of the equivalent, narrower `cargo test -p trackly-app --test place_movements_write_sites_devices -- --test-threads=1`, which exercises the exact same three tests (no other file in the package contains a test whose name contains `place_movements_manual_device`) and passed 3/3. No hang was observed in the terminated run — it was progressing normally alphabetically through unrelated binaries (each reporting `0 passed; N filtered out`) when stopped; the known pre-existing `login_remember_persistent_cookie` hang was never reached because that test name doesn't match this filter.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- The write-site call shape established here (`before_place_id` from the pre-update snapshot → `record_movement_if_applicable` inside the existing writer `tx`) is ready to be replicated 1:1 by Plan 40-08 (cartridge) and Plan 40-09 (act) write sites.
- `place_movements_write_sites_devices.rs` is the template for the parallel `place_movements_write_sites_cartridges.rs` / `place_movements_write_sites_acts.rs` test files.
- HST-01 is NOT marked complete in `.planning/REQUIREMENTS.md` (device write site is only one of several; cartridge/act write sites and timeline UI are still pending) — left for the orchestrator to close at phase end, per this plan's `bookkeeping_constraint`.
- No blockers identified.

---
*Phase: 40-movement-history*
*Completed: 2026-09-02*

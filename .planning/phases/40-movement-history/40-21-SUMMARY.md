---
phase: 40-movement-history
plan: 21
subsystem: api
tags: [rust, rusqlite, place-movements, device-service, cartridge-service, gap-closure]

# Dependency graph
requires:
  - phase: 40-movement-history (plan 07)
    provides: "device_service::update writes place_movements via record_movement_if_applicable — cascade hooks into the SAME transaction right after this call"
  - phase: 40-movement-history (plan 04/09)
    provides: "cartridges_sqlite::transition_in_tx's step 5 (main mutation + its movement row) and step 5b (auto-return) — backfill step 5a sits between them"
provides:
  - "SqliteCartridgeRepository::cascade_place_for_printer_in_tx — cascades a printer's new place onto every cartridge with current_printer_device_id = printer_id, in the caller's open transaction, recording one place_movements row per affected cartridge via the shared D-04/D-06 gate"
  - "DeviceService::update calls the cascade whenever a device's place actually changed (before_place_id != after.place_id), gated wider than the device's own D-04/D-06 movement check because printers physically move even on transitions through NULL"
  - "cartridges_sqlite::transition_in_tx step 5a: Install with an explicit cartridge place backfills devices.place_id for a placeless target printer (race-guarded by WHERE place_id IS NULL), leaves an already-placed printer untouched"
affects: []

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Derived-state cascade with no optimistic lock: cascade_place_for_printer_in_tx's per-cartridge UPDATE has no WHERE version=? clause — it is synchronizing a value derived from the printer's own place, not accepting a user edit, so a concurrent transition() on the same cartridge simply overwrites place_id again on its own next step without conflict"
    - "Backfill-not-overwrite via a WHERE-clause race guard: printer place backfill uses UPDATE devices SET place_id=? WHERE id=? AND place_id IS NULL rather than a read-then-branch-then-write sequence, so a concurrent write to the same printer's place can never be silently clobbered"

key-files:
  created: []
  modified:
    - crates/trackly-infra/src/repos/cartridges_sqlite.rs
    - crates/trackly-app/src/services/device_service.rs
    - crates/trackly-app/tests/place_movements_write_sites_devices.rs
    - crates/trackly-app/tests/cartridges_lifecycle.rs

key-decisions:
  - "Cascade query source is current_printer_device_id read from the DB, never from the client payload (T-40-21-01) — device_service::update's caller cannot direct which cartridges get swept up in the cascade"
  - "Cascade gate on the DEVICE's own place change (before_place_id != after.place_id) is intentionally wider than the D-04/D-06 movement-recording gate applied to the device's own history row — a printer's physical relocation still needs to cascade even for transitions through NULL that the device's own gate would skip logging"
  - "Backfill (step 5a) only reads the printer's CURRENT place inside the same transaction, never trusts the auto-resolved place_id computed earlier in cartridge_service::transition (which only fires when payload.place_id was None) — so an operator-supplied explicit place is the only value that can trigger a backfill, and an already-placed printer is provably left alone"

requirements-completed: []  # HST-01 already tracked from earlier plans; orchestrator closes bookkeeping at phase end

# Metrics
duration: ~35min
completed: 2026-09-03
---

# Phase 40 Plan 21: Cartridge-Follows-Printer Cascade + Printer Place Backfill Summary

**Closes UAT-40 gap "cartridge-does-not-follow-printer" (variant B): moving a printer now cascades its new place onto every attached cartridge in the same transaction with its own movement row, and installing a cartridge with an explicit place into a placeless printer backfills that place onto the printer.**

## Performance

- **Duration:** ~35 min
- **Completed:** 2026-09-03
- **Tasks:** 3/3
- **Files modified:** 4

## Accomplishments

- `SqliteCartridgeRepository::cascade_place_for_printer_in_tx` reads every live cartridge with `current_printer_device_id = printer_device_id`, writes the printer's new `place_id` onto each, and records one `place_movements` row per cartridge (note "вместе с принтером") through the shared `record_movement_if_applicable` gate — old==new or NULL-involved transitions correctly produce zero rows.
- `DeviceService::update` now clones `cartridge_repo` into its writer closure (matching the existing `printer_repo`/`place_repo`/`place_movements_repo` convention) and calls the cascade immediately after the device's own movement-recording call and before `tx.commit()`, gated on `before_place_id != after.place_id`.
- `transition_in_tx`'s new step 5a: when `Install` carries both an explicit `place_id` and a `printer_device_id`, and that printer's own `place_id` is currently `NULL`, backfill `devices.place_id` from the cartridge's explicit place (`WHERE place_id IS NULL` as the concurrency guard) and record a device movement with note "заполнено по месту установленного картриджа". A printer that already has a place is left untouched — verified by a dedicated test.
- Four new integration tests (2 in each file) exercise both behaviors, including explicit no-op assertions (status-only device edit doesn't touch cartridges; Install into an already-placed printer doesn't overwrite it).

## Task Commits

Each task was committed atomically:

1. **Task 1: Cascade места «принтер → его картриджи» in device_service::update** - `95a81fc9` (feat)
2. **Task 2: Обратная запись места принтеру при явном указании места картриджу** - `b370e0d4` (feat)
3. **Task 3: Интеграционные тесты каскада и обратной записи** - `726e8876` (test)

## Files Created/Modified

- `crates/trackly-infra/src/repos/cartridges_sqlite.rs` - added `cascade_place_for_printer_in_tx` (new public method, no optimistic lock — synchronizes derived state); added step 5a inside `transition_in_tx` between the main mutation's movement record and the existing step 5b auto-return block.
- `crates/trackly-app/src/services/device_service.rs` - added `cartridge_repo: Arc<SqliteCartridgeRepository>` field + constructor wiring; `update()` clones it into the writer closure and calls the cascade after the device's own `record_movement_if_applicable` call, inside the same transaction.
- `crates/trackly-app/tests/place_movements_write_sites_devices.rs` - added `update_cascades_place_to_attached_cartridges` and `update_with_no_place_change_does_not_touch_cartridges`, plus local seed helpers (`seed_printer_device`, `seed_cartridge_attached_to_printer`, `cartridge_place_and_version`, `count_cartridge_movements`) that insert printer devices and cartridges directly via SQL, mirroring the file's existing `seed_place` pattern.
- `crates/trackly-app/tests/cartridges_lifecycle.rs` - added `install_with_explicit_place_backfills_printer_without_place` and `install_with_explicit_place_does_not_override_printer_with_existing_place`, plus a `device_place_id` read helper.

## Decisions Made

See `key-decisions` in frontmatter. Notably: the cascade's trigger condition (any real place change on the device, not just D-04/D-06-reportable ones) is deliberately broader than the device's own movement-logging gate, because a printer that moves through `NULL` (e.g. its place gets cleared then reassigned in two separate edits) must still sweep its attached cartridges each time — the cartridge-level gate (also D-04/D-06, applied per-cartridge inside the cascade) still decides independently whether each individual cartridge's move is reportable.

## Deviations from Plan

None — plan executed exactly as written. All three tasks match the plan's `<action>` and `<interfaces>` sections; the cascade method signature, the step 5a insertion point (between step 5 and step 5b), and all four test names match the plan verbatim.

## Issues Encountered

None. `cargo check`/`cargo test`/`cargo clippy --workspace --tests -- -D warnings` were all clean on first pass after each task; `cargo fmt` reformatted the new test file's line-wrapping (pre-existing drift in an unrelated file, `place_movements_act_link.rs`, was left untouched per the project's known `cargo fmt --check` drift note).

`cargo test -p trackly-app --test place_movements_write_sites_devices --test cartridges_lifecycle -- --test-threads=1` — 28 tests total (23 + 5), all green, including the 4 new ones. `cargo clippy --workspace --tests -- -D warnings` — clean, no new warnings.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- UAT-40 gap "cartridge-does-not-follow-printer" (test 5, root causes 1/2/4) is closed per the user's chosen variant B: cartridge place now stays synchronized with its printer's place across subsequent printer moves, and a printer without a place gets one backfilled from an explicit cartridge Install.
- No blockers identified. Remaining UAT-40 gaps (40-22 through 40-27) are independent gap-closure plans in the same wave — no shared file conflicts with this plan's four touched files.

---
*Phase: 40-movement-history*
*Completed: 2026-09-03*

## Self-Check: PASSED

- FOUND: crates/trackly-infra/src/repos/cartridges_sqlite.rs
- FOUND: crates/trackly-app/src/services/device_service.rs
- FOUND: crates/trackly-app/tests/place_movements_write_sites_devices.rs
- FOUND: crates/trackly-app/tests/cartridges_lifecycle.rs
- FOUND: .planning/phases/40-movement-history/40-21-SUMMARY.md
- FOUND commit: 95a81fc9
- FOUND commit: b370e0d4
- FOUND commit: 726e8876

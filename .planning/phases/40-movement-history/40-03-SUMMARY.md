---
phase: 40-movement-history
plan: 03
subsystem: api
tags: [rust, tauri, axum, identity, audit-log, tdd]

# Dependency graph
requires:
  - phase: 40-movement-history
    provides: "place_movements schema + domain types (V040, plan 40-01); compute_place_path_short single owner (plan 40-02)"
provides:
  - "device_service::update(caller: &Identity, id, version, patch) — real actor identity threaded end-to-end from both Tauri and HTTP transports"
  - "audit_log.user_id on manual device updates now reflects the real caller (Manager/Admin), not a hard-coded NULL"
affects: [40-07]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "caller: &Identity extracted to a Copy value (user_id_opt) BEFORE moving into the writer closure — mirrors place_service::create's established shape, since Identity is not Send across that boundary"

key-files:
  created: []
  modified:
    - crates/trackly-app/src/services/device_service.rs
    - crates/trackly-app/src/tauri_cmds/devices.rs
    - crates/trackly-app/tests/devices_crud.rs
    - crates/trackly-app/tests/devices_location_roundtrip.rs
    - crates/trackly-app/tests/devices_type_conversion.rs
    - crates/trackly-app/tests/acts_update_return.rs

key-decisions:
  - "http/devices.rs::handler_update needed zero code changes — it already forwarded &identity into build_devices_update; only the build_* helper's own delegating call and the service signature changed, exactly as RESEARCH predicted"
  - "New audit_log.user_id assertion test seeds a real users row (FK target) rather than hard-coding an arbitrary integer user_id, since audit_log.user_id has a REFERENCES users(id) FK enforced under this test harness's PRAGMA foreign_keys=ON"

patterns-established:
  - "device_service::update caller threading is the template call-site update pattern the remaining Plan 40-04..40-06 write-site plans (delete_soft, create, and other five write-site methods) should replicate 1:1"

requirements-completed: [HST-01]

# Metrics
duration: 9min
completed: 2026-09-01
---

# Phase 40 Plan 03: Thread caller identity into device_service::update Summary

**`device_service::update` now takes `caller: &Identity` end-to-end (Tauri + HTTP), so `audit_log.user_id` on a manual device edit reflects the real Manager/Admin who made it instead of a hard-coded `NULL` — closing RESEARCH.md's Pitfall 1 ahead of Plan 40-07's movement-insert logic.**

## Performance

- **Duration:** 9 min
- **Started:** 2026-09-01T17:29:00Z
- **Completed:** 2026-09-01T17:38:22Z
- **Tasks:** 1 (TDD: RED + GREEN)
- **Files modified:** 6

## Accomplishments
- `device_service::update`'s signature now requires a real `caller: &Identity`, and `user_id_opt` is sourced from `caller.user_id` instead of a hard-coded `None`
- `build_devices_update` (Tauri) forwards `caller` through to `ctx.devices.update` instead of dropping it after `authorize()`
- Confirmed `http/devices.rs::handler_update` needed no change — the compiler (via `cargo build -p trackly-app`) proved there is no orphaned 3-argument call site left anywhere in the crate, closing the T-40-05 transport-asymmetry threat from the plan's threat model
- Two new integration tests pin the exact contract: a real manager's `user_id` flows into `audit_log.user_id`, and the unlocked-desktop `trusted_admin()` identity still stores `NULL` (unchanged behavior)

## Task Commits

Each task was committed atomically (TDD RED → GREEN):

1. **Task 1 (RED): add failing test for caller threading** - `c9cd7190` (test)
2. **Task 1 (GREEN): thread caller identity into device_service::update** - `79efd1b4` (feat)

_No REFACTOR commit needed — the change was a minimal, already-clean signature/extraction edit._

## Files Created/Modified
- `crates/trackly-app/src/services/device_service.rs` - `update` now takes `caller: &Identity`; `user_id_opt` sourced from `caller.user_id`, extracted before the writer closure
- `crates/trackly-app/src/tauri_cmds/devices.rs` - `build_devices_update` forwards `caller` into `ctx.devices.update(...)`
- `crates/trackly-app/tests/devices_crud.rs` - added `update_stores_real_caller_user_id_in_audit_log` + `update_with_trusted_admin_caller_stores_null_user_id`; added `seed_manager_user` helper (real `users` row, FK-safe); updated 5 pre-existing `.update(...)` call sites to pass `&admin_caller()`
- `crates/trackly-app/tests/devices_location_roundtrip.rs` - updated 1 call site + `admin_caller()` helper
- `crates/trackly-app/tests/devices_type_conversion.rs` - updated 4 call sites + `admin_caller()` helper
- `crates/trackly-app/tests/acts_update_return.rs` - updated 1 call site to pass `&Identity::trusted_admin()`

## Decisions Made
- Used `Identity::trusted_admin()` (existing constructor, `user_id: None`) for all pre-existing test call sites that didn't previously assert on `user_id` — preserves current behavior with zero test-intent drift.
- For the new manager-identity test, seeded a real `users` row via the writer (mirroring `request_lifecycle.rs`'s `seed_user` pattern) rather than hard-coding `user_id: Some(1)` — `audit_log.user_id REFERENCES users(id)` is FK-enforced under this test harness (`PRAGMA foreign_keys = ON`), discovered when the naive `Some(1)` version failed with `Conflict { reason: "FOREIGN KEY constraint failed" }`.
- Invented name "Петров П.П." used for the seeded manager per CLAUDE.md's hard privacy constraint (no real names/data in the public repo).

## Deviations from Plan

None - plan executed exactly as written. The one implementation surprise (FK enforcement on `audit_log.user_id` requiring a real seeded `users` row rather than an arbitrary integer) was resolved within Task 1's normal RED/GREEN iteration, not a deviation from the plan's scope — the plan's `<read_first>` already anticipated `place_service::create`'s pattern as the template, and this is a direct consequence of following it correctly for a table with an FK the reference implementation's caller happened not to exercise.

## Issues Encountered
- Initial version of the new `update_stores_real_caller_user_id_in_audit_log` test used `Identity { user_id: Some(1), role: Role::Manager }` (mirroring `places_service_crud.rs`'s `manager_caller()` helper), which failed with a `FOREIGN KEY constraint failed` `Conflict` error because `audit_log.user_id REFERENCES users(id)` and no user with id 1 exists in the tempfile test DB. Resolved by seeding a real `users` row first (pattern borrowed from `request_lifecycle.rs::seed_user`) and using its real generated id.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- `device_service::update`'s caller-threading pattern is now proven end-to-end (Tauri + HTTP, compiler-enforced no orphaned call sites) and ready to be replicated by the remaining Plan 40-04..40-06 write-site plans (the other five write-site methods RESEARCH.md's Pitfall 1 covers) before Plan 40-07 adds the actual `place_movements` INSERT logic that will consume `caller.user_id`.
- No blockers.

---
*Phase: 40-movement-history*
*Completed: 2026-09-01*

## Self-Check: PASSED

All modified files and both task commit hashes (c9cd7190 test, 79efd1b4 feat) verified present.

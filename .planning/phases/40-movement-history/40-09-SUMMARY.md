---
phase: 40-movement-history
plan: 09
subsystem: api
tags: [rust, rusqlite, place-movements, act-service, act-lifecycle, tdd]

# Dependency graph
requires:
  - phase: 40-movement-history (plan 06)
    provides: "all four act mutation methods (create/update/do_return/update_return) already take caller: &Identity and their own user_id_opt local inside the writer closure"
  - phase: 40-movement-history (plan 05)
    provides: "SqlitePlaceMovementsRepository::record_movement_if_applicable — the single D-01 write-side entry point owning the D-04/D-06 skip guard"
  - phase: 40-movement-history (plan 07)
    provides: "device_service.rs's write-site call shape — the direct pattern replicated here"
provides:
  - "act_service::create records a place_movements row per device with act_id set, source='act' (HST-03)"
  - "act_service::update's added-devices loop records the same, using its own act_id (payload.id)"
  - "act_service::do_return records a place_movements row per returned device, act_id = the return act's id — correctly SKIPS when no place override is supplied (Pitfall 4/D-06 DEF-3 path)"
  - "act_service::update_return's BOTH device loops (added + retained_with_change) record/skip movements identically"
  - "place_movements_act_link.rs — Wave 0 act-family test suite, third sibling after device (40-07) and cartridge (40-08); reserved for Plan 40-20 to extend with place_movements_act_undo_deletes"
affects: [40-10, 40-11, 40-20]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Same write-site call shape as device/cartridge siblings: capture before.place_id from the pre-mutation snapshot already fetched for audit_log, call record_movement_if_applicable with the after row's place_id, inside the same &Transaction, act_id passed as Some(act_id)/Some(payload.id)/Some(return_act_id) per method"
    - "Pitfall 4 confirmed non-hypothetical: do_return's effective_location and update_return's added/retained_with_change loops can all legitimately produce None (no place override supplied) — record_movement_if_applicable's own both-Some guard skips these with zero extra branching at any of the 5 call sites"

key-files:
  created:
    - crates/trackly-app/tests/place_movements_act_link.rs
  modified:
    - crates/trackly-app/src/services/act_service.rs

key-decisions:
  - "Added place_movements_repo: Arc<SqlitePlaceMovementsRepository> as a new ActService field (constructor-injection, mirrors DeviceService/CartridgeService's existing field from Plans 40-07/40-08)"
  - "update_return's 'un-return' loop (restore-from-snapshot on a removed device) is deliberately NOT wired to record_movement_if_applicable in this plan — the plan's task 2 <action> and acceptance-criteria grep count (exactly 5 call sites: create, update, do_return, update_return's added, update_return's retained_with_change) scope this plan to those five write sites only; the un-return loop's place restoration is out of this plan's stated scope"
  - "Split the single already-written act_service.rs diff into two task-atomic commits (Task 1: create+update, Task 2: do_return+update_return) after the fact via targeted git apply --cached hunk patches, verifying each intermediate commit state builds and tests green before committing — matches Plan 40-05's precedent for splitting an atomically-authored change along task boundaries"

requirements-completed: []  # HST-01/HST-03 NOT marked complete here — orchestrator closes at phase end once timeline UI (40-10/40-11) also lands; see bookkeeping_constraint

# Metrics
duration: ~40min
completed: 2026-09-02
---

# Phase 40 Plan 09: Wire Act Write Sites into place_movements Summary

**All four act mutation methods (`create`, `update`, `do_return`, `update_return`) now write `place_movements` rows with `act_id` set, calling the shared `record_movement_if_applicable` helper — the return flow's documented `place -> NULL` code path (Pitfall 4/D-06) is proven to correctly record zero rows, never crashing on the `to_place_id NOT NULL` constraint.**

## Performance

- **Duration:** ~40 min
- **Completed:** 2026-09-02
- **Tasks:** 3/3
- **Files modified:** 2 (1 created, 1 modified)

## Accomplishments

- `ActService` gained a `place_movements_repo: Arc<SqlitePlaceMovementsRepository>` field, constructed in `ActService::new` alongside the existing `places_repo`
- `create`'s device-mutation loop and `update`'s added-devices loop both call `record_movement_if_applicable` right after their existing `audit_log` insert, with `MovementSource::Act` and the act's own id — HST-03 real: the timeline can link an act-driven move back to the act number
- `do_return`'s device loop and both of `update_return`'s device loops (`added` and `retained_with_change`) call the same helper with the return act's id; the DEF-3 "no place override" path (where `effective_location` is `None`) is proven — via a dedicated test — to correctly skip the insert instead of hitting the `NOT NULL` schema constraint
- 2 new integration tests in `place_movements_act_link.rs`: `place_movements_act_link` (a real handover place change produces one row per device, act-linked, `source='act'`) and `place_movements_null_place_skip` (a return with no place override produces zero rows, before/after full round-trip through `create` → `do_return`)

## Task Commits

Each task was committed atomically:

1. **Task 1: Wire movements into create/update (handover, act_id set)** - `6c52082b` (feat)
2. **Task 2: Wire movements into do_return/update_return — D-06 NULL-skip (Pitfall 4)** - `0aa53914` (feat)
3. **Task 3: Wave 0 test file — act-link and NULL-skip coverage** - `4be7ec8d` (test)

## Files Created/Modified

- `crates/trackly-app/src/services/act_service.rs` - added `place_movements_repo` field + constructor wiring (`use trackly_core::domain::place_movements::{MovementEntityKind, MovementSource}`, `SqlitePlaceMovementsRepository` import); 5 call sites: `create`'s device loop, `update`'s added-devices loop, `do_return`'s device loop, `update_return`'s `added` loop, `update_return`'s `retained_with_change` loop
- `crates/trackly-app/tests/place_movements_act_link.rs` - `place_movements_act_link`, `place_movements_null_place_skip` (2/2 pass); local `seed_place`/`seed_devices_at_place` helpers (device rows seeded with an explicit `place_id`, unlike `acts_returns.rs`'s existing `seed_devices` which always seeds `place_id = NULL`)

## Decisions Made

- Followed the plan's exact 5-call-site scope (not the un-return loop) — see `key-decisions` in frontmatter for the full rationale; the plan's own acceptance criteria (`grep -c ... is at least 5`) and task 2's `<action>` text both explicitly enumerate exactly these five sites.
- Wrote a dedicated `seed_devices_at_place` test helper rather than reusing `acts_returns.rs`'s `seed_devices` (which never sets `place_id`) — the act-link test needs devices to start at a real, non-NULL place so the handover's place change is genuinely reportable.

## Deviations from Plan

None - plan executed exactly as written. All 5 call sites, exact `act_id` threading, and the D-06 guard's zero-extra-code contract were followed literally per the plan's `<action>` and `<interfaces>` sections.

## Issues Encountered

None. `cargo build -p trackly-app`, `cargo build --workspace`, `cargo fmt --check`, and `cargo clippy -p trackly-app --all-targets -- -D warnings` all pass clean. Full regression across all `acts_*` integration test files (89 tests: `acts_crud`, `acts_clone_handover`, `acts_returns`, `acts_update`, `acts_update_return`, `acts_undo`, `acts_numbering`, `acts_place_snapshot`, `acts_place_path_short`, `acts_display_rule`, `acts_search`, `acts_suggest`) plus the sibling `place_movements_write_sites_devices`/`place_movements_write_sites_cartridges` and the infra `place_movements_repo` suite (6 tests) — all green, no behavior drift on any pre-existing test.

The plan's stated post-execution git split (author code once, then partition into task-atomic commits) was done via `git apply --cached` against extracted diff hunks rather than a literal task-by-task authoring pass, since the shared field/import/constructor addition (Task 1's prerequisite) and the four call sites naturally live in one contiguous edit session against the same large file. Each intermediate commit state was verified to build and test green before committing, so the resulting two-commit history for Task 1/Task 2 accurately reflects working states, not just a post-hoc split (same precedent as Plan 40-05's summary).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- All seven Phase 40 write sites (device, both cartridge sites, and all five act sites) are now fully wired to `record_movement_if_applicable` — Plans 40-07, 40-08, and this plan (40-09) collectively close HST-01/HST-03's write-side scope.
- `place_movements_act_link.rs` is ready for Plan 40-20 to extend with `place_movements_act_undo_deletes` (D-03's undo-scoped deletion), per this plan's explicitly reserved file-scope note.
- HST-01/HST-03 are NOT marked complete in `.planning/REQUIREMENTS.md` — left for the orchestrator to close at phase end, per this plan's `bookkeeping_constraint`.
- No blockers identified.

---
*Phase: 40-movement-history*
*Completed: 2026-09-02*

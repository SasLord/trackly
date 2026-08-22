---
phase: 39-place-tree
plan: 02
subsystem: database
tags: [rust, domain-model, auth, tdd, natural-sort]

# Dependency graph
requires:
  - phase: 39-place-tree plan 01
    provides: "places table (adjacency list), place_full_paths view, place_id columns on devices/cartridges/acts — the live schema PlaceRow/PlaceNew/PlacePatch model"
provides:
  - "domain::places — PlaceKind (closed 6-value enum), PlaceRow/PlaceNew/PlacePatch/PlaceFilter, SubtreeStats, PlaceContentRow, sibling_cmp/natural_name_cmp (D-05/PLC-02)"
  - "ports::places::PlaceRepository — 13-method trait, sole contract for place CRUD (no auto-create-by-name)"
  - "auth::Action::ReadPlaces/MutatePlaces — D-20 permission split (Admin-only mutate, Admin|Manager read)"
affects: [39-place-tree remaining plans (repo/service layer, entity migrations, UI PlacePicker/PlaceTree, FTS/search parity)]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Natural-order string comparison: split into alternating ASCII-digit/non-digit runs, compare digit runs as u64 — no external crate (natord) needed at this scale"
    - "sibling_cmp precedence chain: sort_order (manual override) > level (floors) > natural_name_cmp — matches D-05 exactly"

key-files:
  created:
    - crates/trackly-core/src/domain/places.rs
    - crates/trackly-core/src/ports/places.rs
  modified:
    - crates/trackly-core/src/domain/mod.rs
    - crates/trackly-core/src/ports/mod.rs
    - crates/trackly-core/src/auth.rs

key-decisions:
  - "PlacePatch mirrors PlaceNew as an all-Option<T> struct (parent_id included) even though rename()/move_node() are separate CAS methods on PlaceRepository — matches the plan's literal instruction to mirror DevicePatch's all-optional shape; downstream service-layer plan decides whether PlacePatch.parent_id is actually wired to anything or left unused (same as some DevicePatch fields today)"
  - "TDD RED phase used a deliberately wrong non-exhaustive-safe stub (bucket both new Action variants into the existing Admin|Manager arm) rather than an unreachable!()/todo!() stub, so exactly the target regression test (authorize_manager_mutate_places_forbidden) failed while everything else stayed green — makes the RED commit's diff-to-GREEN a clean, reviewable one-line move"

requirements-completed: [PLC-01, PLC-02, PLC-06]

# Metrics
duration: 25min
completed: 2026-08-22
---

# Phase 39 Plan 02: Places domain contracts + D-20 authorization Summary

**`domain::places` (PlaceKind closed enum, PlaceRow/PlaceNew/PlacePatch, D-05 natural sibling sort) + `ports::places::PlaceRepository` (13-method CRUD contract) + `auth::Action::ReadPlaces`/`MutatePlaces` (Admin-only mutate, Admin|Manager read) — the interface-first target every later Phase 39 plan builds against.**

## Performance

- **Duration:** ~25 min
- **Started:** 2026-08-22T19:15:00Z
- **Completed:** 2026-08-22T19:40:09Z
- **Tasks:** 3/3
- **Files modified:** 5 (2 created, 3 modified)

## Accomplishments
- `PlaceKind` — closed 6-value enum (territory/zone/building/floor/room/outdoor); `from_str` rejects unknown tokens with a Russian-language `AppError::Validation` message listing all six permitted values
- `sibling_cmp`/`natural_name_cmp` — D-05's precedence chain (sort_order > level > natural name) proven by unit tests, including PLC-02's negative-floor ordering and the "2 before 10" numeric-run case
- `PlaceRepository` — 13-method port trait (create/get/list_children/list_all/rename/move_node/archive/unarchive/delete_hard/subtree_stats/list_subtree_contents/list_storage_place_ids/full_path), documenting the Pattern 2 (subtree-stats conflict) and Pattern 3 (cycle check) contracts for the repo-layer implementer
- `Action::ReadPlaces`/`MutatePlaces` — D-20's non-standard Admin-only-mutate/Admin+Manager-read split, proven by a TDD RED commit that reproduces exactly the copy-paste pitfall RESEARCH.md warns against (MutatePlaces briefly in the Admin|Manager bucket, caught by `authorize_manager_mutate_places_forbidden`)

## Task Commits

Each task was committed atomically (Tasks 1 and 3 are `tdd="true"`, so each has a RED + GREEN commit pair):

1. **Task 1: domain/places.rs (RED)** - `192bc281` (test) — types + tests with stubbed sibling_cmp/natural_name_cmp
2. **Task 1: domain/places.rs (GREEN)** - `aa5443a6` (feat) — real Pattern-4 comparator implementation
3. **Task 2: ports/places.rs — PlaceRepository trait** - `1d021191` (feat)
4. **Task 3: auth.rs D-20 deviation (RED)** - `ad828c3c` (test) — ReadPlaces/MutatePlaces added, MutatePlaces deliberately misplaced
5. **Task 3: auth.rs D-20 deviation (GREEN)** - `49d8c146` (feat) — MutatePlaces moved to Admin-only arm

**Plan metadata:** (this commit)

## Files Created/Modified
- `crates/trackly-core/src/domain/places.rs` - `PlaceKind`, `PlaceRow`, `PlaceNew`, `PlacePatch`, `PlaceFilter`, `SubtreeStats`, `PlaceContentRow`, `sibling_cmp`, `natural_name_cmp` + 7 unit tests
- `crates/trackly-core/src/domain/mod.rs` - registered `pub mod places`
- `crates/trackly-core/src/ports/places.rs` - `PlaceRepository` trait (13 methods)
- `crates/trackly-core/src/ports/mod.rs` - registered `pub mod places`
- `crates/trackly-core/src/auth.rs` - `Action::ReadPlaces`/`MutatePlaces`, `authorize()` match arms, permission-matrix doc table, 4 new unit tests

## Decisions Made
- **`PlacePatch` mirrors `PlaceNew` 1:1 as all-`Option<T>`** (including `parent_id`), following the plan's explicit "mirrors DevicePatch's all-optional shape" instruction, even though `rename()`/`move_node()` already exist as dedicated CAS methods on the trait. Whether the service layer actually routes `PlacePatch.parent_id` anywhere is left to the plan that builds the service layer — this plan only defines the domain shape.
- **TDD RED stubs targeted the exact regression, not a generic failure.** For Task 1, `natural_name_cmp` was stubbed to plain lexicographic compare (fails only the numeric-run tests, kind tests stay green). For Task 3, `MutatePlaces` was temporarily bucketed with `Action::MutateDevices` (Admin|Manager) — this is verbatim the copy-paste mistake RESEARCH.md's Common Pitfall 3 warns against, so the RED commit doubles as a live demonstration of the bug the GREEN commit fixes.

## Deviations from Plan

None — plan executed exactly as written. Interfaces block's `PlaceRepository` trait shape was schema-complete and implemented verbatim (interface-first, no I/O).

## Issues Encountered

None.

## TDD Gate Compliance

Both `tdd="true"` tasks (1 and 3) followed the full RED→GREEN cycle:
- Task 1: `test(39-02)` `192bc281` (3 tests fail on stubbed comparator) → `feat(39-02)` `aa5443a6` (all 7 tests pass)
- Task 3: `test(39-02)` `ad828c3c` (1 test fails, the exact D-20 regression target) → `feat(39-02)` `49d8c146` (all 24 tests pass, no regressions)

Fail-fast rule honored: no test passed unexpectedly during RED in either task.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

`domain::places` and `ports::places::PlaceRepository` compile and are unit-tested; `Action::ReadPlaces`/`MutatePlaces` exist with the D-20 split proven. `cargo build -p trackly-core` succeeds. Downstream Phase 39 plans (SQLite repo implementation, service layer, entity migrations in `domain::{devices,cartridges,acts}`, UI PlacePicker/PlaceTree, FTS/search parity) can now build directly against this interface. No blockers.

Reminder inherited from 39-01: `crates/trackly-core/src/domain/{devices,cartridges,acts}.rs` still reference the old `location_id`/`location` fields and will not compile against the V037/V038 schema until Plan 03 renames them — out of scope for this plan by design.

---
*Phase: 39-place-tree*
*Completed: 2026-08-22*

## Self-Check: PASSED

All created files (`crates/trackly-core/src/domain/places.rs`, `crates/trackly-core/src/ports/places.rs`, this SUMMARY) confirmed present on disk; all five task commit hashes (`192bc281`, `aa5443a6`, `1d021191`, `ad828c3c`, `49d8c146`) plus this SUMMARY's own commit (`859950d3`) confirmed present in `git log`.

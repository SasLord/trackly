---
phase: 40-movement-history
plan: 05
subsystem: database
tags: [rusqlite, sqlite, repository-pattern, transaction-discipline, movement-history]

# Dependency graph
requires:
  - phase: 40-movement-history (plan 01)
    provides: "migration V040 (place_movements table + indexes), MovementSource/MovementEntityKind enums, is_reportable_place_change guard predicate"
provides:
  - "SqlitePlaceMovementsRepository::insert_in_tx / record_movement_if_applicable / delete_by_act_id_in_tx / get_history — the single shared data-access layer for all Phase 40 write sites and read sites"
  - "NewMovement / MovementRow structs — the insert-payload and read-row shapes for place_movements"
affects: [40-06, 40-07, 40-08, 40-09, 40-10, 40-11, 40-13, 40-17, 40-20]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Zero-sized repository struct + *_in_tx(&self, tx: &Transaction<'_>, ...) convention (mirrors SqliteAuditLogRepository) — write methods never open their own transaction"
    - "Guard-then-snapshot write helper: is_reportable_place_change checked FIRST, before any full_path/users resolution, so the common no-op/first-assignment/cleared cases do zero extra I/O"
    - "Soft-degrade actor lookup: users.full_name resolved via .and_then(...).ok() — a missing/unreadable users row falls back to None rather than failing the caller's mutation (Pitfall 6 / IN-01 discipline)"

key-files:
  created:
    - crates/trackly-infra/src/repos/place_movements_sqlite.rs
    - crates/trackly-infra/tests/place_movements_repo.rs
  modified:
    - crates/trackly-infra/src/repos/mod.rs

key-decisions:
  - "record_movement_if_applicable takes places_repo as &dyn PlaceRepository<Conn = Connection> (not a generic), matching the plan's exact interface spec — this keeps the seven downstream write-site call sites uniform regardless of which service module calls in"
  - "Both from_place_id.expect(...) / after_place_id.expect(...) calls immediately follow the is_reportable_place_change guard, proven safe by that check — the only .expect() calls in the file, distinct from the users-lookup soft-degrade path which uses .ok() and never panics"

requirements-completed: [HST-01, HST-02, HST-03]

# Metrics
duration: ~45min (execution was interrupted by an API timeout mid-verification; wall-clock includes that gap)
completed: 2026-09-02
---

# Phase 40 Plan 05: Place-Movements Repository Summary

**Single shared `place_movements` repo (insert/guard/delete/read) — every one of the seven Phase 40 write sites and both read-side consumers now call one tested helper instead of re-implementing the D-04/D-06 skip guard or the actor/path snapshot logic.**

## Performance

- **Duration:** ~45 min of active work (session was interrupted once by an API timeout between finishing implementation/testing and committing; no code was lost — git showed zero commits and all files intact on resume)
- **Completed:** 2026-09-02
- **Tasks:** 2/2
- **Files modified:** 3 (2 created, 1 modified)

## Accomplishments

- `SqlitePlaceMovementsRepository::record_movement_if_applicable` — the single D-01 write-side entry point every downstream write-site plan (40-07/08/09/13) must call: checks `is_reportable_place_change` first (D-04/D-06), then snapshots both place-path strings via `PlaceRepository::full_path` (D-10) and the actor's ФИО via a soft-degrading `users` lookup (D-09, Pitfall 6/IN-01)
- `insert_in_tx` — the plain unconditional INSERT, guard-free, used only by `record_movement_if_applicable`
- `delete_by_act_id_in_tx` — the sole owner of `DELETE FROM place_movements WHERE act_id = ?` (D-03), to be called by plan 40-20's undo path
- `get_history` — newest-first (`ORDER BY created_at_utc DESC, id DESC`), unpaginated (D-20, no `LIMIT`), scoped by `(entity_type, entity_id)`, mirroring `cartridges_sqlite::get_history`'s SQL shape
- 6 integration tests against a real migrated tempfile SQLite DB, covering all 5 Task-1 behavior cases (3 guard skips, 1 real insert with snapshot assertions, 1 scoped delete) plus Task-2's ordering/empty/no-LIMIT test

## Task Commits

Each task was committed atomically:

1. **Task 1: Write-side — insert_in_tx, record_movement_if_applicable, delete_by_act_id_in_tx** - `f220535a` (feat)
2. **Task 2: Read-side — get_history (D-20 ordering)** - `3ee55d3d` (feat)

_Both tasks had `tdd="true"`; tests were authored alongside each task's implementation and verified green (5/5, then 6/6) before each commit — see "Deviations" for why this ran as write-then-verify rather than a separate RED commit per task._

## Files Created/Modified

- `crates/trackly-infra/src/repos/place_movements_sqlite.rs` - `SqlitePlaceMovementsRepository` with `insert_in_tx`, `record_movement_if_applicable`, `delete_by_act_id_in_tx`, `get_history`; `NewMovement<'a>` and `MovementRow` payload/row structs
- `crates/trackly-infra/tests/place_movements_repo.rs` - 6 integration tests against `trackly_infra::test_support::test_db()`
- `crates/trackly-infra/src/repos/mod.rs` - registered `pub mod place_movements_sqlite;` and re-exported `SqlitePlaceMovementsRepository`

## Decisions Made

- Split the single implementation into two atomic commits along the plan's task boundary (Task 1: insert/guard/delete + 5 tests; Task 2: get_history + 1 test) even though both were authored together — each commit was independently re-verified green (`cargo test`, `cargo clippy -D warnings`, `cargo fmt --check`) before being committed, so the two-commit history accurately reflects working states at each step, not just a post-hoc split.
- `act_id`/`place_id` foreign keys are enforced by the schema (V040 `REFERENCES acts(id)` / `REFERENCES places(id)`), so the `delete_by_act_id_in_tx` test seeds real `acts` rows rather than using bare integers — this matches how the real write sites will always pass a genuine `act_id`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Test fixture needed real `acts` rows to satisfy the `act_id` foreign key**
- **Found during:** Task 1's `delete_by_act_id_removes_only_matching_rows` test
- **Issue:** The initial test passed bare integers (`Some(7)`, `Some(99)`) as `act_id`, but `place_movements.act_id REFERENCES acts(id)` (V040) rejected them with `Conflict { reason: "FOREIGN KEY constraint failed" }` — acts referenced by `place_movements` must exist.
- **Fix:** Added a `seed_act(conn, number) -> i64` test helper (mirrors the existing `INSERT INTO acts (...)` pattern from `places_crud.rs`) and used the returned real act ids in the test instead of literals.
- **Files modified:** `crates/trackly-infra/tests/place_movements_repo.rs`
- **Verification:** `cargo test -p trackly-infra --test place_movements_repo -- --test-threads=1` — 6/6 pass
- **Committed in:** `f220535a` (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking — test fixture only, no production code affected)
**Impact on plan:** No scope creep; the fix is confined to test setup and matches the schema's own FK constraint, which is itself a correctness feature (an orphaned `act_id` in `place_movements` would be a data-integrity bug).

## Issues Encountered

The execution session hit an API timeout after implementation and verification were complete but before any commits landed. On resume, git confirmed zero commits existed for this plan and all working-tree files (`place_movements_sqlite.rs`, `place_movements_repo.rs`, `mod.rs`) were intact and unmodified from the pre-timeout state. Re-ran the full verification suite (`cargo test`, `cargo clippy --all-targets -D warnings`, `cargo fmt --check`, `cargo build --workspace`) before committing to confirm nothing had drifted, then split the single already-written implementation into the plan's two task-atomic commits (verifying each commit's state builds and tests green independently before committing it).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `SqlitePlaceMovementsRepository` is ready for every downstream write-site plan (40-06 through 40-09, 40-13) to call `record_movement_if_applicable` with a single line at each of the seven write sites — no write site needs to re-derive the D-04/D-06 guard or the actor/path snapshot logic.
- `get_history` is ready for the timeline plan (40-10/40-11) and printer detail plan (40-17) to consume directly.
- `delete_by_act_id_in_tx` is ready for plan 40-20's undo path.
- No blockers identified.

---
*Phase: 40-movement-history*
*Completed: 2026-09-02*

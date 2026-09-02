---
phase: 40-movement-history
plan: 20
subsystem: api
tags: [rust, rusqlite, place-movements, act-service, act-lifecycle, tdd]

# Dependency graph
requires:
  - phase: 40-movement-history (plan 09)
    provides: "act write sites recording place_movements with act_id set on create/update/do_return/update_return; place_movements_act_link.rs test file reserved for this plan's extension"
  - phase: 40-movement-history (plan 05)
    provides: "SqlitePlaceMovementsRepository::delete_by_act_id_in_tx — the single repo-owned DELETE entry point, already tested"
provides:
  - "act_service::delete_soft calls place_movements_repo.delete_by_act_id_in_tx at each act's own soft-delete point (cascade loop's nested returns, the handover's own delete, and the standalone-return branch) — D-03's undo-scoped deletion, correctly ordered per Pitfall 5"
  - "place_movements_act_undo_deletes — nested handover+return cascade delete test proving exact act_id scoping, with a control act's rows surviving untouched"
affects: []

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Delete-at-own-point pattern: place_movements_repo.delete_by_act_id_in_tx(&tx, <act's own id>) sits immediately after that act's own acts_repo.soft_delete_in_tx call and before its audit_repo.insert, repeated independently at each of the three soft-delete call sites in delete_soft rather than as one blanket delete at function end — the exact fix for Pitfall 5's ordering hazard"

key-files:
  created: []
  modified:
    - crates/trackly-app/src/services/act_service.rs
    - crates/trackly-app/tests/place_movements_act_link.rs

key-decisions:
  - "Split the change into Task 1 (implementation) then Task 2 (test), per the plan's literal task ordering, rather than canonical RED-then-GREEN — the plan's frontmatter type is 'execute' (not 'tdd'), so the whole-plan TDD gate-ordering check does not apply; both cargo build and cargo test were run after each commit to confirm each intermediate state is real and green"
  - "Test re-fetches the handover's version via svc.get() before calling delete_soft, rather than reusing the version captured at create time — do_return's recompute_parent_archived bumps the parent handover's version as a side effect of creating the nested return, so the stale captured version would trip OptimisticLockMismatch"

requirements-completed: []  # HST-03 NOT marked complete here — orchestrator closes at phase end, per bookkeeping_constraint

# Metrics
duration: ~20min
completed: 2026-09-02
---

# Phase 40 Plan 20: D-03 Undo-Scoped Movement Deletion Summary

**`delete_soft` now deletes each act's own `place_movements` rows at its own point in the existing LIFO cascade loop, calling the repo-owned `delete_by_act_id_in_tx` helper — a nested handover+return delete removes exactly the two acts' own rows and proves, via a control act, that no other act's history is touched.**

## Performance

- **Duration:** ~20 min
- **Completed:** 2026-09-02
- **Tasks:** 2/2
- **Files modified:** 2

## Accomplishments

- `delete_soft`'s Handover-branch cascade loop now calls `self.place_movements_repo.delete_by_act_id_in_tx(&tx, ret.id)?` immediately after each nested return's `soft_delete_in_tx` and before that iteration's `audit_repo.insert` — each return's own rows are removed at its own LIFO iteration, never batched.
- The handover's own delete (after the loop) and the standalone `ActType::Return` branch's delete both get their own `delete_by_act_id_in_tx` call, at their own point in the flow, immediately after their own `soft_delete_in_tx`.
- No raw `DELETE FROM place_movements` SQL was introduced in `act_service.rs` — the deletion SQL stays owned exclusively by `SqlitePlaceMovementsRepository` (0 occurrences, verified by grep).
- New integration test `place_movements_act_undo_deletes` extends Plan 40-09's `place_movements_act_link.rs`: creates a handover H (real place change), a nested return R under H (another real place change via `place_id_override`), and an unrelated control handover C. After `delete_soft(H)`, both H's and R's own `place_movements` rows are gone while C's row survives untouched — the exact regression Pitfall 5 warns against.

## Task Commits

Each task was committed atomically:

1. **Task 1: D-03 undo scoping inside delete_soft's cascade loop (Pitfall 5)** - `ada43857` (feat)
2. **Task 2: Extend Wave 0 test file — undo-scoping coverage** - `bab369d1` (test)

## Files Created/Modified

- `crates/trackly-app/src/services/act_service.rs` - `delete_soft` now clones `place_movements_repo` before the writer closure and calls `delete_by_act_id_in_tx` at three points: inside the cascade loop (per nested return), after the handover's own `soft_delete_in_tx`, and after the standalone-return branch's `soft_delete_in_tx`
- `crates/trackly-app/tests/place_movements_act_link.rs` - added `place_movements_act_undo_deletes` (nested-cascade scoping proof with a control act); updated the file's top doc-comment to reflect the extension

## Decisions Made

See `key-decisions` in frontmatter: (1) task ordering followed the plan's literal Task 1/Task 2 split rather than canonical TDD RED-then-GREEN since this plan's frontmatter `type` is `execute`, not `tdd`; (2) the test re-fetches the handover's current version via `svc.get()` before `delete_soft`, since `do_return`'s `recompute_parent_archived` bumps the parent's `version` when the nested return is created.

## Deviations from Plan

None - plan executed exactly as written. All three `delete_by_act_id_in_tx` call sites match the plan's `<action>` placement instructions verbatim (each within 3 lines of its corresponding `soft_delete_in_tx`), and the standalone-return branch (which the plan flagged as "if a separate code branch exists") does exist in `act_service.rs` and got its own call site as instructed.

## Issues Encountered

The first test run failed with `OptimisticLockMismatch` because `do_return`'s cascade side effect (`recompute_parent_archived`) increments the parent handover's `version` — the test originally reused the `version` captured at `create` time. Fixed by re-fetching the handover via `svc.get()` immediately before calling `delete_soft`. This is a test-authoring correction (Rule 1 — bug in the new test itself, not in production code), verified by the subsequent green run.

`cargo build -p trackly-app`, `cargo fmt --check` (both modified files), and `cargo clippy -p trackly-app --tests -- -D warnings` all pass clean. `cargo test -p trackly-app --test place_movements_act_link -- --test-threads=1` — all 3 tests pass (Plan 40-09's two plus this plan's new one). Full-crate regression `cargo test -p trackly-app act_service -- --test-threads=1` (matches the plan's `<verification>` command) ran the module's 4 unit tests plus every integration test binary in the crate (106 `test result: ok` lines total, 0 `FAILED`) — no behavior drift on any pre-existing test.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- D-03's undo-scoped deletion is now wired at all three of `delete_soft`'s soft-delete points, closing the last of Phase 40's high-risk pitfalls (Pitfall 5) alongside Plan 40-09's Pitfall 4 (NULL-skip).
- HST-03 is NOT marked complete in `.planning/REQUIREMENTS.md` — left for the orchestrator to close at phase end, per this plan's `bookkeeping_constraint`.
- No blockers identified.

---
*Phase: 40-movement-history*
*Completed: 2026-09-02*

## Self-Check: PASSED

- FOUND: crates/trackly-app/src/services/act_service.rs
- FOUND: crates/trackly-app/tests/place_movements_act_link.rs
- FOUND: .planning/phases/40-movement-history/40-20-SUMMARY.md
- FOUND commit: ada43857
- FOUND commit: bab369d1

---
phase: 19-acts-date-edit
plan: 06
subsystem: acts
tags: [rusqlite, single-writer, tdd, gap-closure, ci-blocker]

# Dependency graph
requires:
  - phase: 19-acts-date-edit (plans 01-05)
    provides: ActService::update() CAS header edit + device add/remove item-set mutation
provides:
  - "acts.archived recomputed inside ActService::update() whenever the item set changes"
  - "Two regression tests proving the two silent-corruption scenarios are fixed"
affects: [19-07, 19-08, code-review, verification]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Gated derived-state recompute: recompute_parent_archived only fires when added/removed device sets are non-empty, preserving the version+1 contract for header-only edits"
    - "Recompute-after-CAS ordering: derived-state recompute functions that unconditionally bump version must run strictly after any CAS UPDATE in the same transaction, never before"

key-files:
  created: []
  modified:
    - crates/trackly-app/src/services/act_service.rs
    - crates/trackly-app/tests/acts_update.rs

key-decisions:
  - "recompute_parent_archived call placed after update_act_header_in_tx (CAS) and before the step-10 final-audit fetch, per plan-checker-verified sequencing constraint"
  - "Gated strictly on !added.is_empty() || !removed.is_empty() — header-only edits keep the single version+1 contract (header_only_edit_does_not_touch_devices still asserts version+1); item-changing edits now bump version by 2 in one transaction"

patterns-established:
  - "Derived boolean state (archived) must be recomputed by every service method that can change the input to its computation (act_items count vs return-act device count), not just the methods that were derived-state-aware at write time"

requirements-completed: [ACT-02]

# Metrics
duration: ~25min
completed: 2026-07-12
---

# Phase 19 Plan 06: Recompute acts.archived on update() device-set changes Summary

**Closed CR-01 blocker: `ActService::update()` now recomputes `acts.archived` after every add/remove of act items, via a version-ordering-safe gated call to `recompute_parent_archived` sequenced after the CAS header UPDATE.**

## Performance

- **Duration:** ~25 min
- **Started:** 2026-07-11T19:19Z (approx, from STATE.md session marker)
- **Completed:** 2026-07-11T19:32Z
- **Tasks:** 2 (RED, GREEN)
- **Files modified:** 2

## Accomplishments
- Fixed CR-01 (BLOCKER, code review): `update()`'s device-set mutation (add device → в_работе, remove device → restore) never recomputed the derived `acts.archived` flag, unlike `do_return`/`delete_soft`. This silently corrupted `archived` in two UI-reachable scenarios.
- Added the first `archived`-asserting regression tests in `acts_update.rs` (previously 0 matches for `archived` in that file).
- Verified via TDD: both new tests failed against the pre-fix `update()` (RED), then passed after the fix (GREEN), with the full 11/11 `acts_update` suite green and `cargo clippy -p trackly-app --tests -- -D warnings` clean.

## Task Commits

Each task was committed atomically:

1. **Task 1 (RED): Two failing regression tests asserting acts.archived after update()** - `e7c308b` (test)
2. **Task 2 (GREEN): Gated recompute_parent_archived inside update(), correctly sequenced vs CAS** - `043e514` (feat)

**Plan metadata:** (this commit)

## Files Created/Modified
- `crates/trackly-app/src/services/act_service.rs` - Added a gated `recompute_parent_archived(&tx, payload.id, now)?` call inside `update()`'s transaction, placed after the CAS header UPDATE (`update_act_header_in_tx`, ~line 878) and before the step-10 final-audit fetch (~line 907), guarded on `!added.is_empty() || !removed.is_empty()`.
- `crates/trackly-app/tests/acts_update.rs` - Added `remove_last_outstanding_archives_act` and `add_device_to_archived_unarchives` regression tests (Tests 10 and 11 in the suite).

## Decisions Made
- **Sequencing:** `recompute_parent_archived` runs strictly after `update_act_header_in_tx`'s CAS `WHERE version = expected_version` UPDATE. Both functions unconditionally bump `version`; running recompute first would advance version past `expected_version`, making the CAS match 0 rows and raising a spurious `OptimisticLockMismatch`. This was flagged by the plan-checker as the key hazard and confirmed correct by the passing `version_mismatch_returns_conflict` test (unaffected) and the new tests (which both require the CAS to succeed first).
- **Gating:** The recompute call is gated on `added`/`removed` being non-empty (i.e., only fires when the item set actually changes). This is required — not cosmetic — because `archived` is purely a function of act_items count vs. return-act device count; header-only edits cannot change it, and gating preserves `header_only_edit_does_not_touch_devices`'s `version == handover.version + 1` assertion. Item-changing edits now bump version by 2 (CAS + recompute) within the same transaction — no test asserts an absolute version number for those paths, so this is compatible.

## Deviations from Plan

None - plan executed exactly as written. The plan's `<action>` blocks for both tasks specified the exact insertion point, gating condition, and sequencing rationale; no interpretation or improvisation was required.

## Issues Encountered

None. Both tasks compiled and behaved exactly as the plan predicted:
- Task 1's two new tests failed with the expected assertion mismatches (Test A: expected `archived=true`, got `false`; Test B: expected `archived=false`, got `true`) — confirming genuine RED, not a compile error.
- Task 2's fix turned both tests green on the first attempt with no additional debugging.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- CR-01 blocker closed. The two silent-corruption scenarios identified in `19-REVIEW.md` (archived act + add device stranding a device в_работе; removing the last outstanding device leaving a falsely-active act) are now proven fixed by passing regression tests.
- No known blockers for the remaining gap-closure plans (19-07: WR-01/02/03, 19-08: IN-01).
- `acts_update.rs` now has 11/11 passing tests including the first `archived` coverage in the suite — future `update()` changes that touch the item-set path are protected by this regression guard.

---
*Phase: 19-acts-date-edit*
*Completed: 2026-07-12*

## Self-Check: PASSED

- FOUND: crates/trackly-app/src/services/act_service.rs
- FOUND: crates/trackly-app/tests/acts_update.rs
- FOUND: commit e7c308b (test)
- FOUND: commit 043e514 (feat)

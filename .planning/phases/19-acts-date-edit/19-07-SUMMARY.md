---
phase: 19-acts-date-edit
plan: 07
subsystem: acts
tags: [rusqlite, audit-log, sqlite-transaction, act-numbering]

# Dependency graph
requires:
  - phase: 19-acts-date-edit
    provides: "Plan 19-06's recompute_parent_archived integration inside ActService::update() (same transaction, after CAS header UPDATE)"
provides:
  - "Act-number rename cascade to child return acts (WR-01) — old numbers become reusable after rename"
  - "custom:act_item_complectation_edit audit row for retained-item комплектация edits (WR-03)"
affects: [act-numbering, audit-log, acts_update-tests]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Same-tx cascade UPDATE co-located with the entity-level audit row that triggers it (number rename → child return acts)"
    - "SELECT-before-UPDATE equality guard to make an audit row conditional on real change, not merely Some(value)"

key-files:
  created: []
  modified:
    - crates/trackly-app/src/services/act_service.rs
    - crates/trackly-app/tests/acts_update.rs

key-decisions:
  - "WR-01: cascade the renamed number to child return acts (option a) rather than excluding act_type='return' from the uniqueness check (option b) — keeps do_return's \"return copies parent number\" invariant intact for future reads instead of leaving return rows permanently mismatched with their parent"
  - "WR-01 cascade does not bump return acts' version (they are not the edited CAS entity); only updated_at_utc is touched"
  - "WR-03 audit is gated on stored != incoming value (not just Some(v)) so a no-op resubmit of the same комплектация writes zero additional rows"

patterns-established:
  - "custom:act_item_complectation_edit audit action follows the existing custom:act_number_override / custom:update_remove naming convention"

requirements-completed: [ACT-02]

# Metrics
duration: 20min
completed: 2026-07-12
---

# Phase 19 Plan 07: Act number-cascade on rename + комплектация audit trail Summary

**Renaming a handover with existing returns now frees the old act number for reuse, and retained-item комплектация edits write a conditional audit_log row instead of a silent bare UPDATE.**

## Performance

- **Duration:** ~20 min
- **Tasks:** 2 completed
- **Files modified:** 2

## Accomplishments
- WR-01: `update()`'s number-change guard now cascades the renamed number to all live child return acts in the same transaction (`UPDATE acts SET number=?1, updated_at_utc=?2 WHERE parent_act_id=?3 AND deleted_at_utc IS NULL`), co-located with the existing `custom:act_number_override` audit insert. The old number now has zero live rows after rename, so the pre-existing step-8b uniqueness check (unchanged) makes it reusable automatically.
- WR-03: step 7's retained-row `complectation_at_time` overwrite now SELECTs the currently stored value first; the UPDATE and a new `custom:act_item_complectation_edit` audit row (before/after JSON) only fire when the incoming value actually differs from what's stored. Resubmitting the same комплектация value is a true no-op (no UPDATE, no audit row).
- Two new regression tests added to `acts_update.rs`, bringing the suite to 13/13 green: `rename_with_return_frees_old_number` (fails without the cascade — the freed number is still blocked by the orphaned return row) and `complectation_edit_writes_audit` (asserts exactly one audit row on real change, zero additional rows on no-op resubmit).

## Task Commits

Each task was committed atomically:

1. **Task 1: WR-01 — cascade renamed number to child return acts + regression test** - `4fec80a` (fix)
2. **Task 2: WR-03 — audit retained-item комплектация changes + regression test** - `aeed218` (feat)

## Files Created/Modified
- `crates/trackly-app/src/services/act_service.rs` — number-cascade UPDATE in `update()`'s rename guard (step 9b); SELECT-before-UPDATE + conditional `audit_repo.insert` in step 7's retained-item комплектация branch
- `crates/trackly-app/tests/acts_update.rs` — `rename_with_return_frees_old_number` and `complectation_edit_writes_audit` regression tests

## Decisions Made
- WR-01: implemented cascade (option a) per plan's explicit decision, not the alternative of excluding `act_type='return'` from the uniqueness check — cascade also fixes the internal inconsistency the review flagged (return row's stored `number` diverging from its parent) and preserves `do_return`'s "copy parent number" invariant for any future code that reads a return act's `number` directly.
- WR-03: audit fires strictly on `stored != incoming`, not merely `Some(incoming)` — matches the plan's explicit no-duplicate-audit acceptance criterion and avoids audit-log noise from idempotent resubmits (e.g., a form re-save with unchanged комплектация text).

## Deviations from Plan

None — plan executed exactly as written. Both cascade/audit changes were placed exactly where the plan's `<action>` blocks specified (step 9b for the cascade, step 7 for the audit), and both regression tests match the plan's `<acceptance_criteria>` scenarios verbatim.

## Issues Encountered

Both tasks' `files_modified` lists were identical (`act_service.rs` + `acts_update.rs`) with non-overlapping hunks in each file, which made a git-native "stage only this task's changes" split impossible via `git add <path>`. Resolved by writing both tasks' full changes first, verifying the combined result (13/13 tests green, clippy clean), then temporarily reverting Task 2's hunks via `Edit`, committing Task 1 in isolation (12/12 tests green), and re-applying Task 2's hunks for its own atomic commit (13/13 tests green again). No code content was lost or altered by this — it only affected commit ordering/isolation.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Both remaining data-integrity/audit warnings from `19-REVIEW.md` (WR-01 number leak, WR-03 untraceable комплектация edits) are closed. `cargo test -p trackly-app --test acts_update` is 13/13 green; `cargo clippy -p trackly-app --tests -- -D warnings` is clean. No known blockers for Phase 19 closure.

---
*Phase: 19-acts-date-edit*
*Completed: 2026-07-12*

## Self-Check: PASSED

- FOUND: `.planning/phases/19-acts-date-edit/19-07-SUMMARY.md`
- FOUND: commit `4fec80a` (Task 1 — WR-01 cascade)
- FOUND: commit `aeed218` (Task 2 — WR-03 audit)
- FOUND: cascade UPDATE at `act_service.rs:952` (`WHERE parent_act_id = ?3 AND deleted_at_utc IS NULL`)
- FOUND: exactly 1 occurrence of `custom:act_item_complectation_edit` in `act_service.rs`
- `cargo test -p trackly-app --test acts_update`: 13/13 passed
- `cargo clippy -p trackly-app --tests -- -D warnings`: clean

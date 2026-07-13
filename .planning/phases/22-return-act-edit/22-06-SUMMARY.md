---
phase: 22-return-act-edit
plan: 06
subsystem: api
tags: [rust, rusqlite, act-service, return-lifecycle, validation, single-writer, migration, gap-closure, tdd]

# Dependency graph
requires:
  - phase: 22-05-return-act-edit-gap-closure
    provides: ActService::update_return() with CR-01 (location-preservation) + CR-02
      (audit-tag exclusion) BLOCKER fixes already landed on the same code region
      (act_service.rs update_return + acts_update_return.rs)
provides:
  - "WR-01 fix — validate_update_return now mirrors validate_return's dedup /
    non-empty-device_ids / per-item-override-required (apply_to_all=false) checks,
    closing the raw-HTTP server-side gap that let malformed edit payloads bypass the
    UI's own guards"
  - "WR-03 fix — update_return's step 8a 'added' loop now enforces the same
    already_returned + per_device_qty <= handover_qty bound do_return enforces, so a
    device re-issued via an unrelated handover cannot be double-covered by two sibling
    returns under the same parent"
  - "WR-02 fix — the .expect() on parent_act_id inside the single-writer closure is
    replaced with an AppError::Internal domain error; a NULL parent_act_id degrades to a
    domain error instead of panicking the dedicated writer task"
  - "WR-04 fix — V034's migration comment no longer claims the backfill UPDATE is
    naturally idempotent / safe to re-run manually post-Phase-22"
  - "IN-01 doc — the two D-11 change-detection baselines (stored condition_at_time vs
    live location_id, None = no change) are documented in-code"
  - "4 new regression tests: WR-01 dedup + missing-override rejection, WR-03 over-return
    bound, WR-02 NULL-parent-returns-error-not-panic"
affects: [22-verification]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Create/edit validation parity pattern: the edit-path validator
      (validate_update_return) mirrors the create-path validator (validate_return)
      check-for-check, MINUS the act_item_id dedup — the edit path uses act_item_id: 0 as
      a structural placeholder, so that one check would reject every legitimate multi-item
      edit payload"
    - "Single-writer no-panic discipline: an Option unwrap on a DB-sourced value inside the
      writer closure is expressed as .ok_or_else(|| AppError::Internal { source_chain })?,
      never .expect(), so corrupt/legacy row state degrades to a per-request domain error
      instead of tearing down the process-wide write path"

key-files:
  created: []
  modified:
    - crates/trackly-app/src/services/act_service.rs
    - crates/trackly-app/tests/acts_update_return.rs
    - migrations/V034__return_handover_date_backfill.sql

key-decisions:
  - "validate_update_return deliberately omits validate_return's act_item_id dedup check —
    edit-path items carry act_item_id: 0 as a structural placeholder (update_return never
    reads item.act_item_id), so mirroring that dedup would reject every valid multi-item
    edit"
  - "WR-03 bound substitutes parent_act_id for do_return's act_id parameter in both the
    handover_qty and already_returned SUM queries — same semantic value (the parent
    handover's id), the only value available at that call-site"
  - "Pre-existing D-11 test reject_edit_after_manual_device_relocation was updated to
    supply a location_id_override — WR-01 now validates apply_to_all=false payloads
    up-front, and that test previously sent condition-only (which WR-01 correctly rejects
    with Validation before the D-11 Conflict path is reached); the override value does not
    need to match the drifted location because the 3-field snapshot compare still fires"

requirements-completed: []  # ACT-03 gap-closure only — full ACT-03 completion is verified
  # after phase-level verification, not marked here per explicit orchestrator instruction.

# Metrics
duration: ~1h (code+test authoring ~25min; remainder consumed by two environment-killed
  full-workspace test runs, ultimately verified green by the orchestrator with mock env)
completed: 2026-07-13
---

# Phase 22 Plan 06: Return-Act Edit Gap-Closure (WR-01..WR-04 + IN-01) Summary

**Closed the four WARNING-severity findings and the one INFO finding from the Phase 22 code review — giving `update_return`'s edit path server-side validation parity with the create path (WR-01), the same over-return quantity bound (WR-03), a no-panic domain error for corrupt NULL parent_act_id inside the single-writer closure (WR-02), an accurate non-idempotent warning on V034's migration comment (WR-04), and in-code documentation of the two D-11 change-detection baselines (IN-01) — backed by 4 new regression tests.**

## Performance

- **Duration:** ~1h wall-clock. Code + test authoring was ~25 minutes; the remainder was consumed by two full-workspace `cargo test` runs that were each killed mid-run by the execution environment (first at ~90/95 binaries, second at ~42/95 — both with 0 failures observed in what ran). The suite was ultimately confirmed green end-to-end by the orchestrator.
- **Started:** 2026-07-13 (Task 1 commit 9a19f0c)
- **Completed:** 2026-07-13T21:51:22+07:00 (Task 2 commit 085087f)
- **Tasks:** 2/2 completed
- **Files modified:** 3 (0 created, 3 modified)

## Accomplishments

- **Task 1 — WR-01 (validation parity) + WR-03 (over-return bound):**
  - `validate_update_return` now runs, after the existing D-10 non-empty-items check, a loop over `p.items` mirroring `validate_return`'s three checks: intra-payload `device_id` dedup via a `HashSet<i64>` built with `effective_device_ids`, per-item non-empty `device_ids`, and — when `!p.apply_to_all` — required `condition_override` AND required (`location_id_override` OR `location_name_override`). This closes the raw-HTTP path that made CR-01 reachable independent of any UI guard. The `act_item_id` dedup from `validate_return` is deliberately NOT mirrored (edit items use `act_item_id: 0` placeholders).
  - `update_return`'s step 8a "added" loop now computes `handover_qty` (`SUM(quantity)` over the parent's `act_items` for the device) and `already_returned` (`SUM(rai.quantity)` over all non-deleted sibling returns under the parent), and rejects with `AppError::Validation` when `per_device_qty + already_returned > handover_qty` — the same bound `do_return` enforces. A device re-issued via an unrelated handover can no longer be double-covered.
  - 3 new tests: `reject_update_return_duplicate_device_id_across_items`, `reject_update_return_missing_override_when_apply_to_all_false`, `reject_add_when_device_already_returned_elsewhere_under_parent`.
- **Task 2 — WR-02 (no panic) + WR-04 (migration comment) + IN-01 (baseline doc):**
  - The `.expect("return act always has parent_act_id")` at the top of `update_return`'s writer closure is replaced with `.ok_or_else(|| AppError::Internal { source_chain: format!("update_return: return act {} has NULL parent_act_id", payload.id) })?`. No panic path remains reachable inside the single-writer task for this value.
  - V034's trailing comment is rewritten: the UPDATE is a ONE-TIME historical backfill, safe ONLY because refinery never re-runs an applied migration, and explicitly NOT safe to run manually after Phase 22 (a manual re-run would silently clobber every user-edited «Дата возврата» that D-05 lets diverge from `created_at_utc`).
  - A comment above the `condition_changed`/`location_changed` computation documents the intentional baseline asymmetry (stored `act_items.condition_at_time` vs the device's live `location_id`) and the `None = no change (cannot clear)` semantics.
  - 1 new test: `update_return_null_parent_act_id_returns_error_not_panic` (corrupts the return row's `parent_act_id` to NULL via direct SQL, asserts `Err(AppError::Internal { .. })` and no process panic).

## Verification

- `cargo test -p trackly-app --test acts_update_return -- --test-threads=1`: **18/18 passing** (14 carried over from 22-05 + 4 new from this plan). Run to completion locally.
- Full workspace suite: **GREEN — 95 test binaries, 0 failed**, run to completion by the orchestrator with `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test --workspace` (exit 0). `acts_update_return` included and passing. A local foreground run of the same command with mock env also completed exit 0.
- `cargo clippy --workspace -- -D warnings` and `cargo clippy --workspace --tests -- -D warnings`: clean.
- Acceptance-criteria greps all satisfied:
  - `grep "продублирован в возврате"` → 3 occurrences (validate_return once + validate_update_return once + the new test's assertion path in-source counts as source refs; ≥2 in service source).
  - `grep "apply_to_all = false"` in act_service.rs → 4 (2 messages × 2 functions).
  - `grep '\.expect("return act always has parent_act_id")'` → 0.
  - `grep "has NULL parent_act_id"` → present.
  - `grep "naturally idempotent"` in V034 → 0; `grep "NOT safe to run manually"` in V034 → present.

## Task Commits

1. **Task 1 — WR-01 + WR-03** — `9a19f0c` (fix)
2. **Task 2 — WR-02 + WR-04 + IN-01** — `085087f` (fix)

## Files Created/Modified

- `crates/trackly-app/src/services/act_service.rs` — WR-01: `validate_update_return` gains the dedup / non-empty-device_ids / per-item-override loop mirroring `validate_return` (minus act_item_id dedup); WR-03: step 8a "added" loop gains the `already_returned + per_device_qty <= handover_qty` bound; WR-02: `parent_act_id` `.expect()` → `AppError::Internal` domain error; IN-01: explanatory comment on the two D-11 baselines.
- `crates/trackly-app/tests/acts_update_return.rs` — 4 new regression tests appended (file now has 18 tests total); pre-existing `reject_edit_after_manual_device_relocation` updated to supply a `location_id_override` (required by WR-01 now that apply_to_all=false payloads are validated up-front).
- `migrations/V034__return_handover_date_backfill.sql` — trailing comment rewritten to remove the false idempotency claim and warn against a manual re-run post-Phase-22.

## Decisions Made

- **`validate_update_return` omits the `act_item_id` dedup check** that `validate_return` has: the edit path builds items with `act_item_id: 0` as a structural placeholder (`update_return` never reads `item.act_item_id`), so mirroring that dedup would reject every legitimate multi-item edit payload. All other `validate_return` checks are mirrored verbatim.
- **WR-03 bound uses `parent_act_id`** where `do_return` uses its `act_id` parameter — same semantic value (the parent handover's id), and the only value available inside `update_return`'s closure.
- **Updated a pre-existing test rather than weakening the WR-01 fix:** `reject_edit_after_manual_device_relocation` previously sent a condition-only, `apply_to_all=false` payload with no location override. WR-01 now correctly rejects that with `Validation` before the D-11 `Conflict` path is reached. The test was updated to supply a `location_id_override` (value need not match the drifted location — the 3-field snapshot compare still fires the intended D-11 Conflict). This preserves the test's original intent (D-11 relocation-drift rejection) while conforming to the new server-side contract.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Test correctness] Updated `reject_edit_after_manual_device_relocation` for the new WR-01 contract**
- **Found during:** Task 1 (first full `acts_update_return` run after adding the WR-01 validation)
- **Issue:** This pre-existing D-11 test sent an `apply_to_all=false` payload with `condition_override: Some(..)` but `location_id_override: None`. WR-01 now rejects that with `Validation { field: "items[0].location_id_override" }` before reaching the intended D-11 `Conflict` path, so the test failed with "expected Conflict, got Validation".
- **Fix:** Set `location_id_override: Some(loc_b)` on the test's item. The override value is intentionally arbitrary — the D-11 3-field snapshot compare still detects the manually-drifted location and fires the intended `Conflict`. Test intent (D-11 relocation-drift rejection) is preserved.
- **Files modified:** `crates/trackly-app/tests/acts_update_return.rs`
- **Commit:** `9a19f0c`

**Total deviations:** 1 (test-correctness follow-on to the WR-01 fix; no product-behavior deviation).

## Important Note — V034 checksum change

Editing the trailing comment in `migrations/V034__return_handover_date_backfill.sql` changes the file's refinery checksum. **Any local dev SQLite DB that already applied V034 must be recreated** (or its recorded V034 checksum manually reconciled) — refinery validates applied-migration checksums on startup and will error on a mismatch. Automated tests are unaffected: every test builds a fresh temp DB that applies all migrations from scratch, so the new checksum is what gets recorded. This is a comment-only change with zero behavioral/SQL impact — the `UPDATE` statement and `PRAGMA user_version = 34` are byte-for-byte unchanged.

## User Setup Required

None — no external service configuration required. (See the V034 checksum note above only if you have an existing local dev DB.)

## Next Phase Readiness

- All four WARNING findings (WR-01..WR-04) and the INFO finding (IN-01) from `.planning/phases/22-return-act-edit/22-REVIEW.md` are closed; the three reachable warnings (WR-01, WR-02, WR-03) are covered by new regression tests.
- Combined with 22-05 (CR-01, CR-02), every finding in `22-REVIEW.md` is now closed across 22-05/22-06.
- ACT-03 is NOT marked complete here — full phase verification happens after this plan lands, per explicit orchestrator instruction.
- No blockers for phase-level verification.

---
*Phase: 22-return-act-edit*
*Completed: 2026-07-13*

## Self-Check: PASSED

- FOUND: `crates/trackly-app/src/services/act_service.rs`
- FOUND: `crates/trackly-app/tests/acts_update_return.rs`
- FOUND: `migrations/V034__return_handover_date_backfill.sql`
- FOUND: commit `9a19f0c` (Task 1 — WR-01 + WR-03)
- FOUND: commit `085087f` (Task 2 — WR-02 + WR-04 + IN-01)

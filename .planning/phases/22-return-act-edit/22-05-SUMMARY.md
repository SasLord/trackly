---
phase: 22-return-act-edit
plan: 05
subsystem: api
tags: [rust, rusqlite, act-service, return-lifecycle, audit-log, gap-closure, tdd]

# Dependency graph
requires:
  - phase: 22-02-return-act-delta-service
    provides: ActService::update_return() delta-reconciliation (un-return / add-outstanding /
      retained-edit), select_latest_device_mutation / select_latest_device_mutation_pair audit
      helpers
provides:
  - "CR-01 fix — update_return's added (step 10) and retained_with_change (step 11) device
    loops preserve a device's current location when the payload carries no new location
    (location.or(before.location_id)), instead of writing NULL"
  - "CR-02 fix — update_return's retained-edit device audit rows (step 11) are tagged with a
    distinct action (custom:return_item_edit), excluded from select_latest_device_mutation's
    DESC LIMIT 1 restore lookup, so un-return (step 9) always restores the return's TRUE
    original pre-return snapshot, never an intermediate within-return edit snapshot"
  - "4 new regression tests proving both fixes (2 in acts_update_return.rs for CR-01, 1 in
    acts_update_return.rs + 1 in audit_log_sqlite.rs for CR-02), all following RED-then-GREEN
    TDD gates"
affects: [22-verification]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Location-preservation-on-no-op pattern: resolve the effective mutation value against the
      device's CURRENT stored value (before.location_id) when the payload supplies None, rather
      than passing None straight through to the writer (which SQLite interprets as an explicit
      NULL write) — applied at the point of consumption in both device-mutation loops, not at
      the earlier effective_by_device resolution step (which correctly keeps None = 'no override
      requested' semantics for D-11 change detection)"
    - "Audit-action-tag scoping pattern: a repo helper (select_latest_device_mutation) that
      restores a prior snapshot for undo/un-return purposes must exclude rows written by
      within-same-parent-act retained-edit paths via a distinct action string
      (custom:return_item_edit), so DESC LIMIT 1 always resolves to the act's own original
      state-transition row, not a subsequent intra-act edit's snapshot"

key-files:
  created: []
  modified:
    - crates/trackly-app/src/services/act_service.rs
    - crates/trackly-app/tests/acts_update_return.rs
    - crates/trackly-infra/src/repos/audit_log_sqlite.rs

key-decisions:
  - "CR-01 fix applied at consumption point only (inside the added/retained_with_change loops),
    not at effective_by_device's resolution (step 6) or the D-11 change-detection comparison
    (step 8b) — preserves the existing 'None = no override requested' semantics those two steps
    correctly rely on"
  - "CR-02 fix uses a distinct audit action tag (custom:return_item_edit) rather than filtering
    select_latest_device_mutation by before_json.status_id — the review's alternative 'filter by
    status в_работе' option would break for a device edited multiple times before being
    un-returned, whereas the action-tag exclusion generalizes correctly regardless of edit count"
  - "select_latest_device_mutation_pair (the D-11 drift-check sibling query) is deliberately left
    untouched by the CR-02 exclusion — that query correctly WANTS the newest row including
    retained-edits, since it answers 'what did this act's own most recent mutation set the
    device to', not 'what was the device's true original pre-act state'"

requirements-completed: []  # ACT-03 gap-closure only — full ACT-03 completion is verified after
  # both 22-05 and any remaining phase gap-closure plans land; not marked complete here per
  # explicit plan/orchestrator instruction.

# Metrics
duration: ~1h36m wall (includes an environment-triggered background-task kill mid-verification
  that required restarting the full workspace test run via a detached nohup process; actual
  code+test authoring was ~20min)
completed: 2026-07-13
---

# Phase 22 Plan 05: Return-Act Edit Gap-Closure (CR-01 + CR-02) Summary

**Fixed two BLOCKER-severity delta-engine defects in `ActService::update_return()` — a silent location-NULLing bug on condition-only edits (CR-01) and a corrupt-restore bug where un-returning a previously-edited device restored the wrong (post-return) snapshot instead of the true pre-return state (CR-02) — both via minimal, surgical changes proven by 4 new RED-then-GREEN regression tests.**

## Performance

- **Duration:** ~1h36m wall-clock (07:16–08:52). Code authoring + all four TDD RED/GREEN cycles took ~20 minutes (07:16–07:36); the remainder was consumed by the mandated `cargo test --workspace` full-suite verification, which was killed once by an environment/session background-task timeout mid-run and had to be restarted as a detached (`nohup`) process to survive further interruption.
- **Started:** 2026-07-13T07:16:04+07:00
- **Completed:** 2026-07-13T08:52:00+07:00 (approx, full workspace suite green)
- **Tasks:** 2/2 completed
- **Files modified:** 3 (0 created, 3 modified)

## Accomplishments
- **Task 1 — CR-01 (location-NULLing on condition-only edits):** `update_return`'s `added` (step 10) and `retained_with_change` (step 11) device-mutation loops now compute `let effective_location = location.or(before.location_id);` before calling `update_full_in_tx`, preserving a device's current warehouse location whenever the payload carries no new location (empty bulk location + apply_to_all=true, or adding an outstanding device with no target location). Previously this path silently wrote `NULL`, erasing the device's stored location — reachable through the shipped `ReturnModal`, whose own UI copy claimed the opposite ("location может остаться на текущем расположении").
- **Task 2 — CR-02 (wrong un-return restore snapshot):** step 11's retained-edit device audit rows are now tagged `action: "custom:return_item_edit"` instead of the generic `"update"` tag `do_return`'s own device mutation uses. `select_latest_device_mutation` (the query step 9's un-return restore depends on) now excludes `action != 'custom:return_item_edit'` in its `WHERE` clause, so its `DESC LIMIT 1` lookup always resolves to the return's own original pre-return audit row, never an intermediate within-return edit snapshot. The sibling `select_latest_device_mutation_pair` (used only by the D-11 drift guard, which correctly wants the newest row including retained-edits) is untouched.
- **4 new regression tests, all TDD RED-then-GREEN:** `retained_edit_condition_only_preserves_location`, `add_outstanding_device_without_bulk_location_preserves_current_location` (both confirmed failing against unfixed code before the CR-01 fix), `un_return_after_retained_edit_restores_original_pre_return_state` (integration, confirmed failing before the CR-02 fix), and `select_latest_device_mutation_excludes_return_item_edit_action` (repo-level unit test, confirmed failing before the CR-02 fix).
- **Full verification suite green:** `acts_update_return.rs` 14/14 passing (11 pre-existing + 4 new — note the plan's task-1 acceptance criteria mentioned 13/13 and 15/15 at different points; actual final count is 14 since both tasks' tests landed in the same file), `trackly-infra --lib` 3/3 audit_log_sqlite tests passing, `cargo clippy --workspace --tests -- -D warnings` clean, `cargo fmt --check` clean for all touched code (pre-existing unrelated formatting drift elsewhere in the codebase, untouched by this plan, was left as-is per scope boundary). `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test --workspace`: **95 test binaries, 721 individual tests, 0 failures.**

## Task Commits

Each task followed the plan's TDD RED→GREEN gate sequence, committed atomically:

1. **Task 1 RED — CR-01 failing regression tests** - `5730d4d` (test)
2. **Task 1 GREEN — CR-01 location-preservation fix** - `7f38aec` (fix)
3. **Task 2 RED — CR-02 failing regression tests** - `3b53cba` (test)
4. **Task 2 GREEN — CR-02 audit-tag + exclusion fix** - `a64ad7a` (fix)

**Plan metadata:** (final docs commit recorded after STATE.md/ROADMAP.md updates)

## Files Created/Modified
- `crates/trackly-app/src/services/act_service.rs` — CR-01: `effective_location = location.or(before.location_id)` in `update_return`'s `added` and `retained_with_change` loops (2 sites); CR-02: step-11 audit rows tagged `action: "custom:return_item_edit"` instead of `"update"`; explanatory comments referencing both CRs added at each change site
- `crates/trackly-app/tests/acts_update_return.rs` — 3 new regression tests appended (`retained_edit_condition_only_preserves_location`, `add_outstanding_device_without_bulk_location_preserves_current_location`, `un_return_after_retained_edit_restores_original_pre_return_state`); file now has 14 tests total
- `crates/trackly-infra/src/repos/audit_log_sqlite.rs` — `select_latest_device_mutation`'s SQL `WHERE` clause gains `AND action != 'custom:return_item_edit'`; doc comment updated to explain the exclusion and confirm it's inert for the handover-edit caller; 1 new unit test (`select_latest_device_mutation_excludes_return_item_edit_action`)

## Decisions Made
- **CR-01 fix scoped to consumption, not resolution:** `effective_by_device` (step 6) and the D-11 change-detection comparison (step 8b) both correctly treat a `None` location as "no override requested" — that semantic must be preserved for change-detection to work. The location-preservation fix is applied ONLY at the point where the resolved value is about to be written to the device row (`location.or(before.location_id)`), immediately before `update_full_in_tx`.
- **CR-02 fix uses a distinct audit action tag, not a status-based filter:** the review report's second proposed fix (filter `select_latest_device_mutation` to the row whose `before_json.status_id == в_работе`) would fail for a device edited multiple times within the same return before being un-returned (only the LAST edit's row would have the true в_работе-derived `before_json`; earlier edit rows would have на_складе `before_json`, and a status-based filter has no way to distinguish "true original" from "an earlier edit's before-state that happens to also not be в_работе"). The chosen action-tag exclusion generalizes correctly: ALL retained-edit rows are tagged and excluded, so `DESC LIMIT 1` over the remaining rows always lands on the return's own original `do_return` mutation row, regardless of how many intervening edits occurred.
- **`select_latest_device_mutation_pair` deliberately untouched:** this D-11 drift-check sibling query answers a different question ("what did THIS act's own most recent mutation set the device to") and correctly wants retained-edit rows included in its newest-row lookup. Excluding `custom:return_item_edit` there would break the D-11 guard's ability to detect drift after a retained edit.

## Deviations from Plan

### Auto-fixed Issues

None — both tasks executed exactly as specified in the plan's `<action>` blocks. No Rule 1/2/3 auto-fixes were needed; the plan's interfaces section accurately described the exact current code shape, and the prescribed fixes applied cleanly on the first attempt for both tasks.

---

**Total deviations:** 0. Plan executed exactly as written for both tasks.

## Issues Encountered

- **Full-workspace verification interrupted by an environment-level background-task kill:** the mandated `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test --workspace` run was initially piped through `| tail -150` and launched as a harness-managed background task; partway through (after progressing through ~90 of 95 test binaries with zero failures observed), the harness killed the background task itself (not a test failure — the task-notification reported `status: killed` for the shell wrapper, and the `tail -150` buffering meant zero output had been flushed at that point since `tail` only emits after EOF). Recovered by relaunching the identical command via `nohup ... > logfile 2>&1 & disown`, writing directly to a scratchpad log file (no `tail` buffering) so progress was visible in real time and the process was detached from any single tool-call's lifetime. The restarted run completed cleanly: 95 test binaries, 721 tests, 0 failures. No code or test changes were needed to resolve this — it was purely an execution-environment interruption, not a defect in the fix.
- **No `cargo test` concurrency violated:** per project convention (target/ lock contention), all `cargo test`/`cargo clippy` invocations in this plan ran strictly one at a time, confirmed via `ps aux | grep cargo` before each new invocation.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- CR-01 and CR-02 (the two BLOCKER findings from `.planning/phases/22-return-act-edit/22-REVIEW.md`) are both closed and covered by regression tests that fail on the pre-fix code and pass on the post-fix code.
- Per explicit executor instruction, ACT-03 is NOT marked complete in this plan — full phase verification (including the remaining `22-REVIEW.md` WARNING-level findings WR-01..WR-04 and INFO finding IN-01, none of which block this gap-closure plan's scope) happens after this and any sibling gap-closure plans land.
- No blockers for phase-level verification.

---
*Phase: 22-return-act-edit*
*Completed: 2026-07-13*

## Self-Check: PASSED

- FOUND: `crates/trackly-app/src/services/act_service.rs`
- FOUND: `crates/trackly-app/tests/acts_update_return.rs`
- FOUND: `crates/trackly-infra/src/repos/audit_log_sqlite.rs`
- FOUND: `.planning/phases/22-return-act-edit/22-05-SUMMARY.md`
- FOUND: commit `5730d4d` (Task 1 RED)
- FOUND: commit `7f38aec` (Task 1 GREEN)
- FOUND: commit `3b53cba` (Task 2 RED)
- FOUND: commit `a64ad7a` (Task 2 GREEN)

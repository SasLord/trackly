---
phase: 19-acts-date-edit
plan: 03
subsystem: acts
tags: [rust, rusqlite, act-edit, optimistic-concurrency, audit-log]

# Dependency graph
requires: [19-01, 19-02]
provides:
  - ActService::update — full ACT-02 backend implementation (header edit,
    add/remove device reconciliation, CAS, D-07/D-08 guards, A3 number
    uniqueness re-check)
  - validate_update (private) — mirrors validate_create's validation style
  - populate_outstanding_device_ids_in_tx (private) — _in_tx twin of the
    existing populate_outstanding_device_ids, used by update's D-08 guard
affects: [19-04]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Delta reconciliation via HashSet diff (d_old vs d_new) — added/
      unchanged/removed device_ids computed once, reused across the
      status-guard pass, the add-loop, the retained-complectation pass, and
      the D-08 removed-device guard."
    - "Most-recent-snapshot restore for edit-driven device removal —
      select_latest_device_mutation's DESC LIMIT 1 lookup (built in Plan
      19-02) replaces the bulk LIFO-undo query used by delete_soft, giving
      correct behavior across repeated add/remove cycles on the same
      device (Pitfall 2)."
    - "Validate-then-mutate for whole-transaction abort — D-08's outstanding
      check and the A3 number-uniqueness check both run before any removed-
      device mutation; any early Err before tx.commit() rolls back
      everything already written earlier in the same closure (added-device
      mutations included), so ordering inside the transaction is a
      correctness nicety, not a safety requirement."

key-files:
  created:
    - crates/trackly-app/tests/acts_update.rs
  modified:
    - crates/trackly-app/src/services/act_service.rs

key-decisions:
  - "ActPatch's 5 unconditional header fields (giver_name/receiver_name/
    location_id/notes/deadline_utc) are always built as Some(resolved_value)
    in update()'s ActPatch construction — never left as the outer None that
    Plan 19-02's SUMMARY flagged as a contract callers must honor since
    update_act_header_in_tx's SQL treats them unconditionally, not via
    COALESCE."
  - "custom:update_remove chosen as the audit action name for edit-driven
    device removal (distinct from delete_soft's custom:undo) so audit
    history stays legible about WHY a device was restored, while
    payload_json still carries {\"act_id\": ..} so a later full-act
    delete_soft's select_device_mutations_for_act bulk query still finds
    and unwinds it if needed."
  - "D-08's outstanding-set computation and the A3 number-uniqueness check
    both run BEFORE the removed-device restore loop's mutations (though
    Task 2's action text notes this ordering is a correctness nicety, not a
    strict requirement, since the whole closure is one transaction — any
    early Err before tx.commit() rolls back everything)."

requirements-completed: []

# Metrics
duration: 25min
completed: 2026-07-11
---

# Phase 19 Plan 03: Act Update Core (ACT-02) Summary

**`ActService::update` now exists and is the actual fix for the user-reported "Редактировать не работает" bug — header-only edits are device-inert (D-05), added positions transition на_складе→в_работе exactly like `create` (D-06), removed positions restore to their MOST RECENT prior state rather than the original pre-handover snapshot (D-06/Pitfall 2), and both D-07 (only handover acts editable) and D-08 (return-bound devices non-removable) are enforced server-side with whole-transaction abort on violation — all backed by 9 green integration tests.**

## Performance

- **Duration:** ~25 min (commit span 22:19:35 → 22:32:59, +file-reading/verification overhead)
- **Completed:** 2026-07-11
- **Tasks:** 2/2 completed (each following the RED→GREEN TDD gate)
- **Files modified:** 2 (1 new test file, 1 modified service file)

## Accomplishments

- `ActService::update` — the method that did not exist anywhere in the stack before this plan (RESEARCH.md's grep confirmed zero `fn update`/`fn patch` on `ActService`/`ActRepository`/`SqliteActRepository`) — is now fully implemented and covers every ACT-02 backend behavior: header-only edit (D-05, device-inert), add-position transition (D-06 add-half, reusing `create`'s exact add-loop body), remove-position restore to the most-recent prior state (D-06 remove-half, Pitfall 2-safe), CAS via `update_act_header_in_tx`'s `WHERE version=?` (plus an early defense-in-depth pre-check), D-07 (only `ActType::Handover` acts are editable — server-side, independent of the UI's disabled-button state), D-08 (a removed device_id already consumed by a completed/active return rejects the WHOLE update, no partial writes), and A3 (number-change uniqueness re-check + `custom:act_number_override` audit).
- `validate_update` mirrors `validate_create`'s validation style: non-empty giver/receiver, non-empty items (an edit that would leave zero positions is invalid, matching `create`'s rule), max 100 items, flat device_id dedup, and `number_override >= 1` bound.
- `populate_outstanding_device_ids_in_tx` added as the `_in_tx` twin of the existing `populate_outstanding_device_ids` — same `EXCEPT` predicate, operating on an open write `Transaction` and returning a `HashSet<i64>` directly, used exclusively by `update`'s D-08 guard.
- 9 integration tests in the new `crates/trackly-app/tests/acts_update.rs`, all green: `header_only_edit_does_not_touch_devices`, `add_position_transitions_device`, `version_mismatch_returns_conflict`, `reject_update_on_return_act` (Task 1), `remove_position_restores_prior_state`, `double_edit_restores_most_recent_snapshot`, `reject_removal_of_returned_device`, `header_edit_free_even_with_existing_return`, `number_change_rejects_duplicate` (Task 2).
- No transport exposes this method yet — Plan 19-04 wires Tauri/HTTP; this plan's method is complete and correct in isolation (confirmed via `grep -rn "\.update(" tauri_cmds/ http/"` returning zero act-related hits).

## Task Commits

Each task followed the RED→GREEN TDD gate protocol (2 commits per task):

1. **Task 1 (RED): failing tests for ActService::update core** - `9e934d6` (test)
2. **Task 1 (GREEN): ActService::update core — header edit, add position, CAS, D-07** - `1f722be` (feat)
3. **Task 2 (RED): failing tests for removed-device reconciliation** - `0b9684a` (test)
4. **Task 2 (GREEN): removed-device reconciliation + D-08 + A3** - `d8976a8` (feat)

_Both tasks are `tdd="true"` and each produced a RED (test) + GREEN (feat) commit pair, per the plan's TDD execution flow._

## Files Created/Modified

- `crates/trackly-app/src/services/act_service.rs` - Added `ActUpdateDto` import, `ActPatch` import; `validate_update` (private fn); `ActService::update` (public async fn, ~230 lines: load+guard act, D-07 check, CAS pre-check, status-id resolution, delta computation, added-device status-guard+add-loop+act_items insert, retained-device complectation overwrite, D-08 outstanding guard, A3 number-uniqueness re-check, removed-device most-recent-snapshot restore+act_items delete, `ActPatch` build+`update_act_header_in_tx` call, `custom:act_number_override` audit (conditional), final act-update audit row with real before/after diff); `populate_outstanding_device_ids_in_tx` (private fn, `_in_tx` twin of the existing bulk-query helper)
- `crates/trackly-app/tests/acts_update.rs` (new) - 9 integration tests + shared scaffolding (`make_acts_service`, `seed_devices_with_state`, `seed_location`, `create_handover_with_location`, `read_device_snap`, `update_dto_from` — all modeled on `acts_undo.rs`'s established conventions)

## Decisions Made

- `ActPatch`'s 5 unconditional header fields are always built as `Some(resolved_value)` in `update()`'s `ActPatch` construction — honoring the contract Plan 19-02's SUMMARY flagged (the repo helper's SQL treats these fields unconditionally, not via `COALESCE`, so a caller-side outer `None` would silently write `NULL`).
- `custom:update_remove` chosen as the audit action name for edit-driven device removal (distinct from `delete_soft`'s `custom:undo`), keeping audit history legible about *why* a device was restored, while `payload_json` still carries `{"act_id": ..}` so a later full-act `delete_soft`'s bulk undo query still finds and unwinds it if the handover act itself is later deleted.
- D-08's outstanding-set check and the A3 number-uniqueness check both run before the removed-device restore loop's mutations in the source, though this ordering is a correctness nicety rather than a strict safety requirement — the entire body executes inside one uncommitted transaction, so any early `Err` before `tx.commit()` rolls back everything regardless of textual position.

## Deviations from Plan

None — both tasks were executed exactly as specified in the plan's `<action>` blocks, including the exact D-06 add-loop body copy, the D-07 guard shape copy from `do_return`, and the D-08/A3 ordering guidance. No Rule 1-4 auto-fixes were required in the production code; the only in-flight correction was to Test 7's fixture (see below).

**`requirements-completed: []` (not `[ACT-02]`) despite the plan frontmatter listing `requirements: [ACT-02]`:** following Plan 19-02's established precedent (its SUMMARY recorded the identical decision for the same reason), ACT-02 ("Пользователь может отредактировать существующий акт — кнопка «Редактировать» активна") is a single user-facing requirement spanning Plans 19-02 through 19-05. This plan completes only the backend half (`ActService::update` itself, unreachable from any transport). Marking ACT-02 "Complete" in `REQUIREMENTS.md` now — before the Tauri/HTTP wiring (19-04) and the `ActDetail.svelte`/`ActFormModal` UI (19-05) exist — would misrepresent project state; the requirement is deferred to whichever of 19-04/19-05 actually closes the user-visible loop.

## Issues Encountered

- **Test 7 fixture correction (not a production-code deviation):** the plan's `<behavior>` spec for `reject_removal_of_returned_device` describes submitting an `ActUpdateDto` that "removes BOTH devices from `items`" from a 2-device act. Submitting a truly-empty `items: []` would trigger `validate_update`'s non-empty-items check (mirroring `create`'s rule, added in Task 1) BEFORE the D-08 conflict check ever runs — producing `AppError::Validation` instead of the expected `AppError::Conflict`. Fixed by seeding one replacement device and including it in `items` (keeping the list non-empty while still attempting to remove both original devices), which correctly exercises the D-08 guard and additionally verifies the replacement device was never added (whole-transaction rollback). No production code was changed for this — it was purely a test-fixture correction, discovered during Task 2's RED-phase verification.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `ActService::update` is complete and correct in isolation: all 9 `acts_update.rs` tests pass, `cargo clippy -p trackly-app --tests -- -D warnings` is clean, and the `TODO(Plan 19-03 Task 2)` marker is fully resolved (`grep -c` returns 0).
- No transport exposes `update` yet — Plan 19-04 is unblocked to wire it into both the Tauri command layer and the axum HTTP layer (dual-transport pattern established throughout the codebase), and to add the frontend `ActDetail.svelte` "Редактировать" button wiring that motivated this whole phase.
- `cargo test --workspace` (full regression, per the plan's `<verification>` step 3) confirmed no regression in `create`/`do_return`/`delete_soft`'s shared helpers (status-id resolution, `device_snapshot_json`, `AuditEntry` shape): exit code 0 across every crate's unit tests, integration test binary, and doc-test pass (trackly-core 48 unit tests, trackly-infra 81 unit tests + 7 integration test files, trackly-app's full integration suite including the new 9 `acts_update.rs` tests) — zero `FAILED`/`error[` occurrences anywhere in the run.

---
*Phase: 19-acts-date-edit*
*Completed: 2026-07-11*

## Self-Check: PASSED

All claimed files found on disk (`crates/trackly-app/tests/acts_update.rs`,
this SUMMARY.md); `ActService::update` confirmed present in
`crates/trackly-app/src/services/act_service.rs`; all 4 claimed commit
hashes (`9e934d6`, `1f722be`, `0b9684a`, `d8976a8`) found in git log.

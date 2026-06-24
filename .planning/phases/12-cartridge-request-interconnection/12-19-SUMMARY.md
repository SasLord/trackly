---
phase: 12-cartridge-request-interconnection
plan: 19
subsystem: database
tags: [rusqlite, audit-log, cartridges, gap-closure]

# Dependency graph
requires:
  - phase: 12-cartridge-request-interconnection
    provides: "Plan 12-18 — cartridge-centric install entry now passes printer_device_id (preFillPrinterId), making the auto-return cascade reachable from both request- and cartridge-centric entries"
provides:
  - "Auto-return of a previous cartridge now writes an INVERTED actor (given_by_name/given_to_name) into its own custom:return_to_stock audit payload_json, closing GAP-12-12"
  - "current_printer_device_id is now cleared on ANY transition that leaves status=2 (В работе) — not just the Install branch — fixing a latent bug where direct ReturnToStock left a stale printer link"
affects: [cartridges, printers, audit-history-ui]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Auto-return payload_json built manually (not via op_payload_json) when the cascade needs fields the domain enum variant doesn't carry — keeps the direct-path payload builder untouched"

key-files:
  created: []
  modified:
    - crates/trackly-infra/src/repos/cartridges_sqlite.rs
    - crates/trackly-app/tests/cartridges_lifecycle.rs

key-decisions:
  - "Inverted actor computed server-side from the SAME Install op's given_by_name/given_to_name fields (no new payload fields, no client-supplied actor for the auto-return) — closes Tampering threat T-12-19-02 by construction"
  - "Collapsed the two near-duplicate UPDATE branches (Install vs ReturnToStock/ToRefill/FromRefill/WriteOff) in transition_in_tx into one — current_printer_device_id is now always written (target printer, or NULL) instead of only on the Install path"

requirements-completed: [CART-07, CART-08, CART-10, PRN-07]

# Metrics
duration: 18min
completed: 2026-06-24
---

# Phase 12 Plan 19: Auto-return printer link + inverted actor (GAP-12-12) Summary

**Auto-return of a previous cartridge now records who handed it back and who received it (inverted relative to the new install), and `current_printer_device_id` is reliably cleared on every exit from "В работе", not just via the auto-return cascade.**

## Performance

- **Duration:** 18 min
- **Started:** 2026-06-24T15:21:48Z
- **Completed:** 2026-06-24T15:39:42Z
- **Tasks:** 2 completed
- **Files modified:** 2

## Accomplishments

- `transition_in_tx`'s auto-return block now destructures `given_by_name`/`given_to_name` from the triggering `Install` op and builds the previous cartridge's `custom:return_to_stock` payload_json with an inverted actor: `given_by_name` = the new install's `given_to_name` (recipient hands the old one back), `given_to_name` = the new install's `given_by_name` (issuer/warehouse receives it back). The existing history UI (`CartridgeDetail.svelte`) renders this without any frontend change, since the JSON keys match exactly.
- The direct (user-initiated) `ReturnToStock` payload builder (`op_payload_json`) is untouched — only the internal auto-return cascade gained the actor fields.
- Discovered and fixed a real bug while writing the round-trip regression test: `transition_in_tx`'s cartridge UPDATE only set `current_printer_device_id` on the `Install` branch; every other op (`ReturnToStock`, `ToRefill`, `FromRefill`, `WriteOff`) left the previous printer link in place on the row. Collapsed the two near-duplicate UPDATE branches into a single one that always writes `current_printer_device_id` (the install's target printer, or `NULL` for every other op / the legacy no-printer install path).

## Task Commits

1. **Task 1: Invert actor in auto-return payload_json** — `15faf38` (fix)
2. **Task 2: Tests for actor inversion + current_printer_device_id round-trip** — `b958f9c` (test, includes the Rule-1 bug fix discovered while writing the new test)

**Plan metadata:** pending (this commit)

## Files Created/Modified

- `crates/trackly-infra/src/repos/cartridges_sqlite.rs` — auto-return payload_json now includes inverted `given_by_name`/`given_to_name`; the cartridge UPDATE's two branches (Install vs. everything else) collapsed into one that always sets `current_printer_device_id`.
- `crates/trackly-app/tests/cartridges_lifecycle.rs` — extended `auto_return_writes_return_to_stock_audit_entry` to parse `payload_json` and assert the inverted actor (`Кузнецов`/`Сидоров`); added `return_to_stock_clears_current_printer_device_id` proving the printer link clears on a direct return, not just on auto-return.

## Decisions Made

- The inverted actor is computed purely from fields the client already legitimately supplied for the *new* install (`given_by_name`/`given_to_name`) — no new actor fields were added to the `ReturnToStock` domain variant, and the client cannot supply an arbitrary actor for the auto-return cascade it doesn't directly invoke. This closes threat T-12-19-02 (Tampering) by construction, as anticipated in the plan's threat register.
- `op_payload_json()` (shared with the direct, user-initiated `ReturnToStock` path) was deliberately left untouched; the inverted-actor JSON is built ad-hoc only inside the auto-return block, so a direct manual return still produces a payload with no actor fields, matching the domain's `ReturnToStock` shape.
- Fixed the `current_printer_device_id` UPDATE bug as a Rule 1 auto-fix (not a new task) — it directly relates to this plan's stated truth #1 ("снимается (NULL) при авто-возврате/возврате — в ОДНОЙ транзакции") and was caught by writing the regression test the plan asked for in Task 2.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `current_printer_device_id` was never cleared on a direct (non-auto) `ReturnToStock`/`ToRefill`/`FromRefill`/`WriteOff`**
- **Found during:** Task 2, while writing `return_to_stock_clears_current_printer_device_id` (the test failed on first run: `current_printer_device_id` stayed `Some(printer_id)` after a direct return).
- **Issue:** `transition_in_tx`'s cartridge UPDATE had two branches: `CartridgeTransitionOp::Install { .. }` set `current_printer_device_id=?5` in the SET clause; the catch-all `_` branch (covering `ReturnToStock`, `ToRefill`, `FromRefill`, `WriteOff`) omitted that column entirely, leaving a stale printer link on a cartridge that had just left "В работе". This only worked correctly via the auto-return cascade because that cascade runs its own separate `UPDATE ... current_printer_device_id=NULL` directly on the *previous* cartridge row — the *direct* user-initiated return path had no equivalent.
- **Fix:** Collapsed both branches into a single UPDATE statement that always writes `current_printer_device_id` — `install_printer_device_id` (the target printer, or `None`) for `Install`, and implicitly `None` for every other op since `install_printer_device_id` is computed as `None` in the `_` match arm already.
- **Files modified:** `crates/trackly-infra/src/repos/cartridges_sqlite.rs`
- **Verification:** New test `return_to_stock_clears_current_printer_device_id` passes; full `cartridges_lifecycle` suite (19 tests) green; full `trackly-app`+`trackly-infra` workspace test run (with `TRACKLY_AD_MOCK=1`) green, no regressions.
- **Committed in:** `b958f9c` (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 bug fix)
**Impact on plan:** Necessary for correctness — directly closes the plan's own stated truth #1 about printer-link round-trip. No scope creep; fix is scoped entirely to the same `transition_in_tx` function this plan was already modifying.

## Issues Encountered

None beyond the deviation above — the source-inspection in the plan's objective correctly identified the auto-return path as already correct; the gap was in the symmetric direct-return path, which the plan's own acceptance criteria (Task 2) led directly to discovering.

## Verification Performed

- `cargo test -p trackly-app --test cartridges_lifecycle -- --test-threads=1` → 19 passed, 0 failed.
- `cargo test -p trackly-app -p trackly-infra -- --test-threads=1` (with `TRACKLY_AD_MOCK=1`, full workspace) → all green; the one failure seen without the env var (`restore_request_visibility_http`, AD real-mode 503) is a pre-existing dev-environment configuration issue unrelated to this change (confirmed passing once `TRACKLY_AD_MOCK=1` is set).
- `cargo clippy -p trackly-infra -p trackly-app -- -D warnings` → clean.
- `cargo fmt --check` → no diff in either file touched by this plan (pre-existing diffs in `printers_sqlite.rs`/`requests_sqlite.rs` are out of scope, unrelated to this plan).
- Source-assertions: `grep -n "given_by_name\|given_to_name" crates/trackly-infra/src/repos/cartridges_sqlite.rs` shows the inverted-actor construction inside the auto-return block; `op_payload_json`'s `ReturnToStock` arm (around line 644-650) contains no actor keys.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- GAP-12-12 closed: auto-return now correctly attributes "выдал {получатель}" / "получил {выдававший}" in the previous cartridge's history, and the printer-link round-trip is now symmetric across the entire install→return lifecycle (not just the auto-return cascade).
- This was the last plan in the Round 3 gap-closure wave (GAP-12-09..12, 4 plans). Phase 12 should now be re-checked against the human-UAT checklist (`12-HUMAN-UAT.md`) before considering the phase complete.

---
*Phase: 12-cartridge-request-interconnection*
*Completed: 2026-06-24*

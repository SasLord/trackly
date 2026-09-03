---
phase: 40-movement-history
fixed_at: 2026-09-03T17:57:36Z
review_path: .planning/phases/40-movement-history/40-HUMAN-UAT.md
iteration: 1
findings_in_scope: 1
fixed: 1
skipped: 0
status: all_fixed
---

# Phase 40: Code Review Fix Report

**Fixed at:** 2026-09-03T17:57:36Z
**Source review:** .planning/phases/40-movement-history/40-HUMAN-UAT.md (finding UAT3-01a, gap-closure round 3)
**Iteration:** 1

**Summary:**
- Findings in scope: 1
- Fixed: 1
- Skipped: 0

## Fixed Issues

### UAT3-01a: Дефолт «Получение с заправки» возвращает саму Заправку, если она помечена складом

**Files modified:**
- `crates/trackly-infra/src/repos/cartridges_sqlite.rs`
- `crates/trackly-app/src/services/cartridge_service.rs`
- `crates/trackly-app/tests/cartridges_lifecycle.rs`

**Commit:** `3a5697cb`

**Applied fix:**

Root cause was in the plan (40-30), not the implementation: it reused
`last_known_storage_place_in_tx` (answers "last known STORAGE place",
owned by Plan 40-28's install auto-return) for a different question
("place before the cartridge was sent to refill"). When the refill place
itself is marked `is_storage = 1`, the reused resolver's
`CASE WHEN p_to.is_storage = 1 THEN pm.to_place_id ELSE pm.from_place_id END`
picked the refill place itself.

- Added a dedicated resolver `place_before_last_to_refill` in
  `cartridges_sqlite.rs`: selects `from_place_id` of the cartridge's most
  recent movement tagged `TO_REFILL_MOVEMENT_NOTE`, with no `is_storage`
  filter on the destination place at all.
- Rewired `CartridgeService::operation_default_place`'s `"from_refill"`
  branch to call the new resolver instead of
  `last_known_storage_place_in_tx`.
- `last_known_storage_place_in_tx` left semantically unchanged — it
  remains the sole owner of the auto-return question (Plan 40-28/CR-02).
  Verified via the 40-28 test suite (`install_auto_return_*`, all green).
- Updated doc comments on both functions and on
  `CartridgeService::operation_default_place` to describe the corrected
  split of responsibility and drop the stale "reuses CR-02 resolver"
  claim.
- Reconciled the existing regression test that had pinned the OLD (buggy)
  behavior: `operation_default_place_from_refill_reflects_manual_edit_during_refill`
  asserted that a manual place edit made while a cartridge is "На
  заправке" should override the `from_refill` default. With the new
  resolver tied to the `ToRefill` movement specifically, an unrelated
  manual edit (no `TO_REFILL_MOVEMENT_NOTE`) no longer shadows it — this
  is the intended, more correct behavior. Renamed the test to
  `operation_default_place_from_refill_ignores_manual_edit_during_refill`
  and updated its assertion + doc comment to explain the round-2 vs
  round-3 behavior change explicitly, rather than silently deleting an
  inconvenient test.
- Added the mandatory new regression test
  `operation_default_place_from_refill_prefers_pre_refill_place_when_refill_place_is_storage_too`
  in `cartridges_lifecycle.rs`, driving the real `CartridgeService`
  (create → transition ToRefill, no hand-seeded `place_movements`).
  Confirmed **RED** on pre-fix code first: `left: Some(2)` (refill place)
  vs `right: Some(1)` (storage A) — i.e. it reproduced the exact live UAT
  failure — then confirmed **GREEN** after the fix.
- Added 3 SQL-level unit tests for `place_before_last_to_refill` in
  `cartridges_sqlite.rs` (mirrors existing `most_common_to_refill_destination_*`
  unit-test style, explicitly documented as SQL-only, not a substitute for
  the real-service integration test above): ignores `is_storage` of the
  destination, `None` on no history, ignores an unrelated later manual
  movement.

**Verification performed:**
- New integration test confirmed RED on pre-fix source (temporarily held
  back via `git stash` on the two source files only, test file kept),
  then GREEN after restoring the fix.
- `cargo test -p trackly-app --test cartridges_lifecycle operation_default_place` — 4/4 passed.
- `cargo test -p trackly-app --test cartridges_lifecycle install_auto_return` — 5/5 passed (Plan 40-28 regression guard, unaffected).
- `cargo test -p trackly-infra --lib cartridges_sqlite` — 29/29 passed (26 pre-existing + 3 new).
- `cargo clippy --all-targets -p trackly-infra -p trackly-app -- -D warnings` — clean.
- `cargo fmt --all --check` — clean.
- `node scripts/check-privacy.mjs --hashes scripts/privacy-tokens.sha256` — PASS, 0 violations.

**Logic-bug flag:** this finding involved a resolver logic change (which
movement/field answers "place before refill"), not just a mechanical
patch. Both the RED-before/GREEN-after real-service integration test and
the reconciled round-2 regression test give strong confidence, but the
new intentional behavior change (manual edits during refill no longer
override the default) is a product-level decision documented inline —
worth a quick human skim of the updated doc comment in
`cartridge_service.rs` (~950-976) to confirm it matches intent.

---

_Fixed: 2026-09-03T17:57:36Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_

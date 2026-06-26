---
phase: 13-per-device-junction-chip-drum-state
plan: 04
subsystem: database
tags: [rusqlite, cartridges, printers, auto-return, pagination]

# Dependency graph
requires:
  - phase: 13-01
    provides: V005 printer_name compatibility redesign, CartridgeRow.model_kind_id
  - phase: 13-02
    provides: cleaned-up CartridgeService/PrinterService transport surface after V029 deletion
provides:
  - "Kind-aware default state for auto-returned previous cartridge on install (drum vs regular)"
  - "Uncapped printers_sqlite.rs::list() read (no .min(200) ceiling)"
affects: [13-08, frontend OperationModal auto-return UI (deferred fix)]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Kind-aware default-state branching computed AFTER fetching the row whose kind_id is needed (not before), to avoid using a value before its dependency is known"
    - "Uncapped read instead of raising a cap — explicit decision (D-13) to skip pagination at current LAN fleet scale"

key-files:
  created: []
  modified:
    - crates/trackly-infra/src/repos/cartridges_sqlite.rs
    - crates/trackly-infra/src/repos/printers_sqlite.rs

key-decisions:
  - "transition_in_tx: moved resolved_state_id computation to after prev_current.model_kind_id is fetched, since the branch depends on it"
  - "printers_sqlite.rs::list(): removed .min(200) entirely rather than raising it to 500 — D-13 explicit uncapped-read decision, no pagination introduced"

patterns-established:
  - "Kind-aware defaults via Option::unwrap_or_else closures keyed on a freshly-fetched row's discriminant field (model_kind_id), not a hardcoded constant"

requirements-completed: [SPEC-13-R7, SPEC-13-R8]

duration: 13min
completed: 2026-06-26
---

# Phase 13 Plan 04: Auto-return drum-state default + uncapped printer list Summary

**Server-side fix: drum (фотобарабан) auto-return now defaults to state_id=5 «Изношенный» instead of hardcoded 3; printers_sqlite.rs::list() no longer caps below the frontend's 500-row request.**

## Performance

- **Duration:** 13 min
- **Started:** 2026-06-26T00:26:57Z
- **Completed:** 2026-06-26T00:39:00Z
- **Tasks:** 2 completed
- **Files modified:** 2

## Accomplishments
- R7: `transition_in_tx`'s auto-return branch now picks `state_id=5` ("Изношенный") for drums (`model_kind_id == Some(2)`) and keeps `state_id=3` ("На заправке") for regular cartridges, only when `previous_cartridge_state_id` is not explicitly passed — explicit overrides still take priority.
- R8: `printers_sqlite.rs::list()` no longer caps results at 200 rows (`page.limit.min(200)` removed) — closes the gap with the frontend's `limit: 500` request in `OperationModal`, per D-13 (uncapped read, no pagination introduced at current fleet scale).
- Added 2 new regression tests in `cartridges_sqlite.rs` (`auto_return_uses_kind_aware_default_state_for_drum`, `auto_return_keeps_state_3_default_for_regular_cartridge`) plus reusable `seed_device`/`seed_drum_model` test helpers.
- Added 1 new regression test in `printers_sqlite.rs` (`list_returns_all_printers_above_old_cap`) seeding 250 printers and asserting `list()` returns all of them (including the highest-id row that the old `ORDER BY p.id DESC LIMIT 200` would have cut off).

## Task Commits

Each task was committed atomically:

1. **Task 1: R7 — kind-aware default state for auto-return** - `9a786ec` (fix)
2. **Task 2: R8 — uncapped read of printers list (D-13)** - `83c20cf` (fix)
3. **Style fixup: cargo fmt on new test assertions** - `d304c55` (style)

**Plan metadata:** (final commit recorded below)

_Note: a small follow-up `style` commit was needed because the new test block's two assertion lines exceeded `cargo fmt`'s line-wrap threshold; fixed and verified clean against the project's two changed files only (pre-existing unrelated fmt drift in `requests_sqlite.rs` and elsewhere left untouched — out of scope)._

## Files Created/Modified
- `crates/trackly-infra/src/repos/cartridges_sqlite.rs` — `transition_in_tx`'s auto-return branch is now kind-aware; added `seed_device`/`seed_drum_model` test helpers and 2 regression tests.
- `crates/trackly-infra/src/repos/printers_sqlite.rs` — removed `.min(200)` cap in `list()`; added 1 regression test (`list_returns_all_printers_above_old_cap`).

## Decisions Made
- Computed `resolved_state_id` strictly after `prev_current` (and thus `model_kind_id`) is fetched, rather than trying to thread the kind through earlier — keeps the diff minimal and the dependency explicit in code order.
- Did not introduce a new constant/ceiling to replace `.min(200)` — D-13 explicitly calls for uncapped read at the current small-fleet scale; pagination is deferred to a future phase if the fleet grows past a reasonable bound.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added FK-satisfying device seed helper for new tests**
- **Found during:** Task 1 (writing the kind-aware auto-return regression tests)
- **Issue:** The plan's test description used an arbitrary `printer_device_id` integer for the install op; `cartridges.current_printer_device_id` has a `REFERENCES devices(id)` FK constraint (V025), so an arbitrary integer caused `Conflict { reason: "FOREIGN KEY constraint failed" }` on the first test run.
- **Fix:** Added a `seed_device()` test helper (mirroring the existing one in `printers_sqlite.rs`'s test module) that inserts a real `devices` row and returns its id, then used that id as `printer_device_id` in both new tests.
- **Files modified:** `crates/trackly-infra/src/repos/cartridges_sqlite.rs`
- **Verification:** Both new tests pass after the fix; `cargo test -p trackly-infra --lib repos::cartridges_sqlite` green (13/13).
- **Committed in:** `9a786ec` (Task 1 commit)

**2. [Rule 1 - Bug] cargo fmt fixup on new test assertions**
- **Found during:** Post-Task-2 verification pass (`cargo fmt --check`)
- **Issue:** Two `let prev_row = repo.get(...).expect(...);` lines in the new Task 1 tests exceeded the project's line-wrap width and were not pre-wrapped to match `cargo fmt`'s expected multi-line form.
- **Fix:** Manually reformatted both lines to match `cargo fmt`'s expected output (method-chain wrap), scoped strictly to the two lines I added — did not run `cargo fmt` on the whole file to avoid touching pre-existing unrelated drift in the same file (an unrelated blank-line issue at printers_sqlite.rs:440 and an unrelated assert at :704, both out of scope per the Scope Boundary rule).
- **Files modified:** `crates/trackly-infra/src/repos/cartridges_sqlite.rs`
- **Verification:** `cargo fmt --check -p trackly-infra` no longer reports drift in either of my two new tests; full `cargo test`/`cargo build`/`cargo clippy -D warnings` re-run green.
- **Committed in:** `d304c55` (style commit)

---

**Total deviations:** 2 auto-fixed (1 blocking FK fix, 1 formatting fix)
**Impact on plan:** Both fixes were necessary to make the plan's own regression tests pass/format correctly. No scope creep — pre-existing unrelated fmt drift elsewhere in the codebase was logged as out-of-scope and left untouched.

## Issues Encountered
None beyond the auto-fixed deviations above.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- R7 (server-side kind-aware drum default) and R8 (uncapped printer list) are both closed; `cargo test -p trackly-infra --lib repos::cartridges_sqlite`, `cargo test -p trackly-infra --lib repos::printers_sqlite`, `cargo build -p trackly-infra`, and `cargo clippy -p trackly-infra -- -D warnings` all pass clean. Full `cargo build --workspace` also confirmed clean.
- Frontend half of R7 (OperationModal still hardcodes states 1/2/3 for its own auto-return UI hint) is intentionally deferred to Plan 13-08 per this plan's objective — not a gap in this plan's scope.
- No new threat-surface introduced beyond the threat model's own T-13-08 (DoS, accepted — LAN-only, session-authenticated) and T-13-09 (tampering, mitigated — `model_kind_id` is a DB-sourced `Option<i64>`, not user free-text).

---
*Phase: 13-per-device-junction-chip-drum-state*
*Completed: 2026-06-26*

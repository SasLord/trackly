---
phase: 40-movement-history
plan: 22
subsystem: api
tags: [rust, rusqlite, place-movements, cartridge-service, gap-closure]

# Dependency graph
requires:
  - phase: 40-movement-history (plan 21)
    provides: "cartridges_sqlite::transition_in_tx's step 5a (printer place backfill) and the auto-return branch (previous cartridge lookup, resolved_state_id fallback pattern) — this plan's fallback lives in the SAME auto-return branch, symmetric with the existing state fallback"
  - phase: 40-movement-history (plan 01/04)
    provides: "place_movements table + MovementEntityKind/MovementSource domain enums"
provides:
  - "SqliteCartridgeRepository::last_known_storage_place_in_tx — private helper querying place_movements for a cartridge's most recent movement into an is_storage=1 place"
  - "transition_in_tx's auto-return branch: when previous_cartridge_place_id is None, resolved_place_id derives from last_known_storage_place_in_tx instead of unconditionally clearing to NULL; explicit overrides and no-history cartridges are unchanged"
affects: []

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Symmetric fallback pair: resolved_place_id (this plan) now mirrors the existing resolved_state_id unwrap_or_else pattern in the same auto-return branch — an explicit operator override always wins, a server-side DB-derived default fills the gap only when the override is absent, and 'no derivable data' preserves the pre-existing behavior rather than erroring"
    - "Server-derived fallback, not client-trusted: the fallback's cartridge_id input comes from the already-validated `previous.id` lookup (current_printer_device_id = pid AND status_id = 2), never from client payload — closes the tampering surface noted in the plan's threat register (T-40-22-01)"

key-files:
  created: []
  modified:
    - crates/trackly-infra/src/repos/cartridges_sqlite.rs
    - crates/trackly-app/tests/cartridges_lifecycle.rs

key-decisions:
  - "resolved_place_id computation moved from before the `previous` cartridge lookup to inside the `if let Some((prev_id, prev_version)) = previous` block, because the fallback needs prev_id (the RETURNED cartridge's id, not the newly-installed one) to query its movement history — this is a structural reordering required by the interface spec, not a behavior change to the explicit-override path"
  - "Fallback only considers movements into is_storage=1 places (JOIN places p ON p.id = pm.to_place_id WHERE p.is_storage = 1), matching the user's decision that auto-return should restore a cartridge to its last WAREHOUSE location, not any arbitrary place it ever passed through (e.g. a desk it was briefly carried to)"
  - "No history -> Ok(None) -> place_id stays NULL: deliberately preserves the pre-40-22 behavior for cartridges that have never been logged into a storage place, rather than guessing or erroring — this is the explicit non-regression clause in must_haves.truths"

requirements-completed: [HST-01]

# Metrics
duration: ~20min
completed: 2026-09-03
---

# Phase 40 Plan 22: Auto-Return Last-Known-Storage-Place Fallback Summary

**Auto-return of a previous cartridge now derives its place from `place_movements` (last is_storage=1 destination) instead of silently clearing to NULL when the operator leaves the place field empty.**

## Performance

- **Duration:** ~20 min
- **Completed:** 2026-09-03
- **Tasks:** 2/2
- **Files modified:** 2

## Accomplishments

- Added `SqliteCartridgeRepository::last_known_storage_place_in_tx`, a private helper that runs `SELECT pm.to_place_id FROM place_movements pm JOIN places p ON p.id = pm.to_place_id WHERE pm.entity_type = 'cartridge' AND pm.entity_id = ?1 AND p.is_storage = 1 ORDER BY pm.created_at_utc DESC, pm.id DESC LIMIT 1` inside the caller's open transaction.
- Rewired `transition_in_tx`'s auto-return branch: `resolved_place_id` now matches on `previous_cartridge_place_id` — `Some(explicit)` is used as-is (existing override contract unchanged, verified by `install_auto_return_uses_previous_cartridge_overrides_when_present` staying green), `None` calls the new fallback keyed on the RETURNED cartridge's own id (`prev_id`), and a fallback `None` (no storage-place history) preserves the pre-existing NULL behavior.
- The computation of `resolved_place_id` was structurally moved inside the `if let Some((prev_id, prev_version)) = previous` block since it needs `prev_id`, which is only known after the previous-cartridge lookup succeeds — this reordering does not change any other logic in the block.
- Added test `install_auto_return_falls_back_to_last_known_storage_place`: seeds a real `is_storage=1` place via a new `seed_storage_place` helper, inserts a synthetic `place_movements` row (direct SQL) placing cartridge A into that storage place before either Install call, then asserts A's post-auto-return `place_id` equals the storage place — not `None`.
- Clarified two pre-existing tests' assert messages/doc-comments (`install_auto_returns_previous_cartridge_in_same_printer`, `install_auto_return_falls_back_to_defaults_when_overrides_absent`) to state their NULL assertions hold specifically because those cartridges have no prior storage-place history, not because place clearing is unconditional — prevents a future reader from mistaking these for a still-absolute contract.

## Task Commits

Each task was committed atomically:

1. **Task 1: Fallback последнего складского места в auto-return ветке** - `449b6fb2` (feat)
2. **Task 2: Уточнить существующие тесты-контракты и добавить тест на новый fallback** - `0cf1e822` (test)

## Files Created/Modified

- `crates/trackly-infra/src/repos/cartridges_sqlite.rs` - added `last_known_storage_place_in_tx` (private helper, placed right after `fetch_in_tx`); reworked `resolved_place_id` in the auto-return branch of `transition_in_tx` to use it as a `None`-branch fallback, moving the computation below the `previous` cartridge lookup so `prev_id` is available.
- `crates/trackly-app/tests/cartridges_lifecycle.rs` - added `seed_storage_place` helper (mirrors `seed_place` but writes `is_storage=1`); added `install_auto_return_falls_back_to_last_known_storage_place`; clarified assert messages/doc-comments in two existing tests without changing their asserted values.

## Decisions Made

See `key-decisions` in frontmatter. Notably: the fallback is scoped to `is_storage=1` places only (not "any prior place"), matching the user's product decision recorded in `.planning/debug/return-to-stock-empty-place-field.md` that auto-return should restore a cartridge to its last known *warehouse* location.

## Deviations from Plan

None — plan executed exactly as written. The interface spec's guidance to "передвинь вычисление `resolved_place_id` НИЖЕ строки, где `prev_id` уже извлечён" was followed literally; the SQL query matches the plan's `<action>` block verbatim (column order, `ORDER BY ... DESC, pm.id DESC LIMIT 1`, `.optional()`); the new test's scenario (seed storage place, seed non-storage place, direct SQL insert into `place_movements`, two Install calls, assert `Some(storage_place_id)`) matches the plan's task 2 description.

## Issues Encountered

One self-caused formatting issue: the new test's `params![...]` call on a single line exceeded rustfmt's line-length preference and needed multi-line formatting to match `cargo fmt`'s expected output (verified via `cargo fmt --check` scoped to the modified file) — fixed before committing. No other issues; `cargo check -p trackly-infra`, `cargo test -p trackly-app --test cartridges_lifecycle -- --test-threads=1` (24/24 green, including 40-21's tests), and `cargo clippy --workspace -- -D warnings` were all clean.

Pre-existing `cargo fmt --check` drift remains in two unrelated locations of this same test file (`install_auto_returns_previous_cartridge_in_same_printer`, `install_auto_return_uses_previous_cartridge_overrides_when_present` — both predate this plan) and in `place_movements_timeline.rs` — left untouched per the project's known pre-existing drift note (out of this plan's file scope).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- UAT-40 gap "return-to-stock-empty-place-field" (test 16) is closed: auto-return without an explicit place now restores the cartridge's last known storage place when derivable, and preserves the original NULL behavior when it isn't.
- No blockers identified. Remaining UAT-40 gap-closure plans (40-23, 40-27) are independent — no shared file conflicts with this plan's two touched files (40-24, 40-25, 40-26 are already merged per wave context).

---
*Phase: 40-movement-history*
*Completed: 2026-09-03*

## Self-Check: PASSED

- FOUND: crates/trackly-infra/src/repos/cartridges_sqlite.rs
- FOUND: crates/trackly-app/tests/cartridges_lifecycle.rs
- FOUND commit: 449b6fb2
- FOUND commit: 0cf1e822

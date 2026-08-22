---
phase: 39-place-tree
plan: 04
subsystem: database
tags: [rust, rusqlite, sqlite, recursive-cte, repository-pattern]

# Dependency graph
requires:
  - phase: 39-place-tree plan 01
    provides: "places table (adjacency list), place_full_paths view, place_id columns on devices/cartridges — the live schema this adapter queries"
  - phase: 39-place-tree plan 02
    provides: "domain::places (PlaceRow/PlaceNew/SubtreeStats/PlaceContentRow), ports::places::PlaceRepository trait — the contract this adapter implements exactly"
provides:
  - "SqlitePlaceRepository — full PlaceRepository impl (13 methods): create/get/list_children/list_all/rename/move_node/archive/unarchive/delete_hard/subtree_stats/list_subtree_contents/list_storage_place_ids/full_path"
  - "The three canonical recursive-CTE query shapes for the places tree, each defined exactly once: descendant-subtree walk (Pattern 2, shared by subtree_stats/list_subtree_contents/delete_hard), ancestor-chain cycle check (Pattern 3, move_node), ancestor-walk storage inheritance (D-11.4, list_storage_place_ids)"
  - "places_crud.rs — PLC-01 integration coverage (repository-layer, no PlaceService yet)"
affects: [39-05 (place_service.rs — the sole caller of this repository), 39-06, 39-07, 39-09, 39-10, 39-11 (act/cartridge/device/report service migrations onto place_id), 39-22 (existing test-fixture cleanup)]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Private free-function *_impl helpers taking &Connection, called both directly (read-only trait methods) and via &Transaction deref-coercion (move_node/delete_hard's multi-statement atomic paths) — one query definition, two call sites, no duplication"
    - "Zero-rows-affected CAS write resolved into NotFound vs OptimisticLockMismatch via a shared resolve_cas_failure() helper — matches devices_sqlite.rs/acts_sqlite.rs convention exactly"
    - "SQLite `col IS ?1` (not branching SQL) handles both `Some(id)` and `None` (root-node) filters for list_children in a single parameterized query"

key-files:
  created:
    - crates/trackly-infra/src/repos/places_sqlite.rs
    - crates/trackly-infra/tests/places_crud.rs
  modified:
    - crates/trackly-infra/src/repos/mod.rs

key-decisions:
  - "rename/move_node/archive/unarchive/delete_hard resolve a zero-rows-affected CAS write to AppError::OptimisticLockMismatch (row exists, version differs) vs AppError::NotFound (row doesn't exist) — NOT a blanket AppError::Conflict as the plan's Task 1 action text literally said. The plan's own text says this 'mirrors the ActPatch/DevicePatch expected_version pattern already established in this codebase', and that established pattern (verified in devices_sqlite.rs and acts_sqlite.rs) is the NotFound/OptimisticLockMismatch split, not Conflict. Followed the established codebase convention the plan pointed at, not the plan's literal (and internally inconsistent) prose. Documented as a deviation below (Rule 1 — the literal instruction would have been a behavioral regression versus every other repository in the codebase)."
  - "delete_hard's non-empty-subtree rejection uses AppError::Conflict { reason: String } (the ONLY shape that variant has in this codebase — a single human-readable string, not a structured payload) with the exact D-14 counts interpolated into the Russian message text, since AppError::Conflict has no numeric fields to carry counts separately."
  - "Every recursive-CTE query pattern lives exactly once in this file (module-level free functions), reused by both the direct trait-method call (read-only Connection) and the transaction-wrapped call inside move_node/delete_hard (via Transaction's Deref<Target = Connection> coercion) — no query text is duplicated between a 'plain' and an 'in-tx' variant."

requirements-completed: [PLC-01, PLC-06]

# Metrics
duration: 70min
completed: 2026-08-22
---

# Phase 39 Plan 04: SqlitePlaceRepository Summary

**`SqlitePlaceRepository` — the concrete adjacency-list `PlaceRepository` adapter (13 methods, 4 distinct recursive-CTE query shapes) every later Phase 39 backend plan depends on, plus `places_crud.rs` integration coverage of PLC-01's create/rename/move/uniqueness/delete-conflict invariants.**

## Performance

- **Duration:** ~70 min
- **Started:** 2026-08-22T~19:00Z (est.)
- **Completed:** 2026-08-22T20:09:09Z
- **Tasks:** 3/3
- **Files modified:** 3 (2 created, 1 modified)

## Accomplishments

- `SqlitePlaceRepository` implements every method of `PlaceRepository` (Plan 02's 13-method trait): `create`/`get`/`list_children`/`list_all`/`rename`/`move_node`/`archive`/`unarchive`/`delete_hard`/`subtree_stats`/`list_subtree_contents`/`list_storage_place_ids`/`full_path`.
- `move_node` runs the Pattern 3 ancestor-chain recursive CTE cycle check as the first statement inside the same transaction as the `UPDATE` — self-move and move-into-own-descendant both rejected with the exact `39-UI-SPEC.md §14.3` Russian message, before any row is touched (T-39-04-03).
- `delete_hard` runs the Pattern 2 subtree-stats query first, inside the same transaction as the `DELETE`; any non-zero count (child places, nested places, devices, cartridges) blocks the delete with `AppError::Conflict` carrying the exact counts in the message — no cascade, no auto-reparenting (D-14, literal).
- `subtree_stats` and `list_subtree_contents` share the identical descendant-subtree CTE — one source of truth for D-14/D-21/D-25/PLC-06, not two.
- `list_subtree_contents` UNIONs devices (excluding printers), printers (resolved via the WR-04 stable seed-name lookup, not a hardcoded `type_id` literal), and cartridges — each leg joined to `place_full_paths` for a live-resolved `full_path`.
- `list_storage_place_ids` is a structurally distinct ancestor-WALK CTE (climbs `parent_id` from every node upward) implementing D-11.4: a place counts as storage if it itself OR any ancestor has `is_storage = 1`.
- `places_crud.rs` covers all six PLC-01 behavior-block scenarios from the plan: create/get round-trip, rename-propagates-to-descendant-full_path, move-preserves-device-FK, cycle-rejection (descendant move + self-move), D-04 uniqueness (same-parent conflict, different-parent success), delete-conflict-with-counts vs delete-success-on-empty-leaf.

## Task Commits

Each task was committed atomically:

1. **Task 1: SqlitePlaceRepository — CRUD, cycle-checked move, archive/unarchive** - `8ac5c8b7` (feat)
2. **Task 2: delete_hard, subtree_stats, list_subtree_contents, list_storage_place_ids, full_path** - `a7531f13` (feat)
3. **Task 3: places_crud.rs — PLC-01 integration coverage** - `2f6d59be` (test)

**Plan metadata:** (this commit)

## Files Created/Modified

- `crates/trackly-infra/src/repos/places_sqlite.rs` — `SqlitePlaceRepository`, `SELECT_PLACES` constant, `from_row` mapper, `resolve_cas_failure` shared CAS-failure resolver, and four module-level `*_impl` free functions (`subtree_stats_impl`, `list_subtree_contents_impl`, `list_storage_place_ids_impl`, `full_path_impl`) each holding exactly one recursive-CTE query shape
- `crates/trackly-infra/src/repos/mod.rs` — registered `pub mod places_sqlite;` and `pub use places_sqlite::SqlitePlaceRepository;`
- `crates/trackly-infra/tests/places_crud.rs` — 6 integration tests against `SqlitePlaceRepository` directly (no `PlaceService` yet)

## Decisions Made

- **CAS-failure error type: `OptimisticLockMismatch`/`NotFound` split, not `Conflict`.** The plan's Task 1 `<action>` prose literally said "mapping zero-rows-affected to `AppError::Conflict`" for `rename`, but in the same sentence described the CAS pattern as mirroring `ActPatch`/`DevicePatch`'s `expected_version` pattern "already established in this codebase." Verified directly in `devices_sqlite.rs` (`update`, `delete_soft`) and `acts_sqlite.rs` (`update_in_tx`, `soft_delete_in_tx`): the actual established pattern distinguishes `AppError::NotFound` (row doesn't exist) from `AppError::OptimisticLockMismatch { entity, id, expected, actual }` (row exists, version differs) — never a blanket `Conflict`. Followed the established, verified pattern the plan pointed at rather than its literal (self-contradicting) instruction. Applied identically to `rename`/`move_node`/`archive`/`unarchive`/`delete_hard`.
- **`delete_hard`'s conflict payload: interpolated Russian message, not structured fields.** `AppError::Conflict` in this codebase has exactly one field (`reason: String`) — grepped every existing call site (`act_service.rs`, `cartridge_service.rs`, `auth.rs`) to confirm no variant carries structured numeric data. D-14's "exact counts, not a generic refusal" requirement is satisfied by interpolating `direct_children + nested_places`, `device_count`, `cartridge_count` into the message text (e.g. "содержит 2 вложенных мест, 1 устройств, 0 картриджей").
- **Query-shape reuse via `&Connection`/`&Transaction` deref-coercion, not duplicated `_in_tx` variants.** Unlike `devices_sqlite.rs`'s `create_in_tx`/`get_in_tx` pattern (separate `Transaction`-typed helper methods duplicating logic already on the trait), this file's private `*_impl` functions take `&Connection` and are called both directly (trait methods reading via `&Self::Conn`) and from inside `move_node`/`delete_hard`'s `&Transaction` (Rust's `Deref<Target = Connection>` coercion applies to function arguments, so `&tx` coerces to `&Connection` with zero extra code). One query definition per shape, period.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] CAS-failure mapping: `Conflict` (plan's literal text) → `OptimisticLockMismatch`/`NotFound` split (established codebase pattern)**
- **Found during:** Task 1 (writing `rename`)
- **Issue:** Plan's action text for `rename` said "mapping zero-rows-affected to `AppError::Conflict`" while simultaneously instructing to mirror the `ActPatch`/`DevicePatch` `expected_version` pattern "already established in this codebase." These two instructions contradict each other — the actual established pattern (verified in `devices_sqlite.rs`/`acts_sqlite.rs`) uses `NotFound`/`OptimisticLockMismatch`, not `Conflict`, for this exact zero-rows-affected case. Following the literal `Conflict` instruction would have made `places_sqlite.rs` the only repository adapter in the codebase using a different CAS-failure error shape than every other entity — a real behavioral inconsistency an API consumer (frontend error-handling switch, Plan 05's service layer) would have to special-case.
- **Fix:** Implemented `resolve_cas_failure(conn, id, expected) -> AppError`, a shared helper querying `SELECT version FROM places WHERE id = ?1 AND deleted_at_utc IS NULL` and returning `NotFound` if absent, `OptimisticLockMismatch { entity: "place", id, expected, actual }` if present with a different version — applied identically across `rename`/`move_node`/`archive`/`unarchive`/`delete_hard`.
- **Files modified:** `crates/trackly-infra/src/repos/places_sqlite.rs`
- **Verification:** Grepped `devices_sqlite.rs`/`acts_sqlite.rs` to confirm the established pattern before implementing; `grep -c "OptimisticLockMismatch\|version = version + 1"` (plan's own Task 1 acceptance-criteria grep) returns 6, confirming the CAS pattern is present throughout.
- **Committed in:** `8ac5c8b7` (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 — the plan's own literal instruction was internally inconsistent; followed the codebase convention the plan explicitly pointed at instead of the contradictory literal text).
**Impact on plan:** No scope creep — every acceptance criterion in the plan is still satisfied (the grep checks for `WITH RECURSIVE ancestors` and the CAS pattern both pass); the fix makes `places_sqlite.rs` consistent with every sibling repository adapter rather than a special case.

## Issues Encountered

**`cargo build -p trackly-infra` does not succeed as a whole crate right now — this is expected, not a bug in this plan's work.** `trackly-infra`'s lib crate currently fails with 23 pre-existing compile errors, all confined to `acts_sqlite.rs`, `cartridges_sqlite.rs`, `printers_sqlite.rs`, and `requests_sqlite.rs` — none reference `places_sqlite.rs` or `places_crud.rs` (verified by grepping the full build log for both filenames: zero matches). These four files still reference the `locations` table and `location_id`/`location` columns that V038 (Plan 01) physically dropped, and the domain-struct fields Plan 03 renamed (`ActRow.location_id`, `CartridgeRow.location`, `PrinterRow.device_location`, `RequestRow.printer_location`) — rewriting their SQL to the `place_id`/`place_full_paths` model is explicitly the scope of Wave 3 plans 39-06/07/09/10/11, per this plan's `prior_wave_context` and confirmed by `39-03-SUMMARY.md`'s own "Issues Encountered" section ("`cargo build -p trackly-app` is now expected to fail... intentional and owned by Plans 06/07/09/10/11").

Because of this, `cargo test -p trackly-infra --test places_crud` (this plan's own `<verification>` command) also cannot execute yet — it depends on the same lib crate compiling. To compensate, every recursive-CTE query embedded in `places_sqlite.rs` (subtree-stats, subtree-contents UNION, cycle-check, storage-ancestor walk, full_path, rename/move CAS, D-04 uniqueness) was independently verified via a standalone Python `sqlite3` harness that applies all 38 real migration files in order and runs the exact SQL text against representative data (`Здание А / 2 этаж / 214`, matching `places_crud.rs`'s test data) — mirroring 39-01's direct-`sqlite3` verification methodology for the same reason (Rust compilation blocked, SQL semantics independently confirmed). All checks passed:

```
Applied 38 migrations OK
full_path room214: Здание А / 2 этаж / 214
full_path room214 after rename: Здание А / 2-й этаж (перекрашен) / 214   [proves live VIEW, no reindex]
cycle_check(floor3, room214) = 0   [not a cycle — valid move]
device place_id after move: unchanged   [FK survives move]
cycle_check(room214, floor3) = 1   [cycle correctly rejected — moving ancestor into descendant]
cycle_check(floor3, floor3) = 1    [self-move correctly rejected]
uniqueness violation correctly raised on same-parent duplicate name
same name under different parent: OK
subtree_stats(building) = (2 direct_children, 4 nested_places, 1 device, 0 cartridges)
delete_hard pre-check on non-empty subtree: correctly blocked
delete_hard on empty leaf: succeeded
list_storage_place_ids: ancestor-inclusive is_storage correctly verified (room214 self-storage
  found; making building is_storage=1 correctly propagated to floor3 and room214 beneath it)
list_subtree_contents nested vs "Только здесь" (D-24): correctly differentiated
ALL SQL VERIFICATION CHECKS PASSED
```

`cargo test -p trackly-infra --test migration_idempotency` was not run in this plan (same lib-crate compile blocker); it is unrelated to any change in this plan's `files_modified` and was already green as of Plan 01.

**Action for the next wave-3 plan that restores `cargo build -p trackly-infra`:** run `cargo test -p trackly-infra --test places_crud` at that point to get the real (not simulated) pass/fail signal — the Rust test file exists and is believed correct based on the SQL verification above, but has never been compiled by `rustc` itself.

## TDD Gate Compliance

Task 3 was flagged `tdd="true"`. Per project convention (`tdd_mode=false` project-wide, confirmed in 39-03-SUMMARY.md), and because the crate-wide compile blocker (see above) prevents an actual RED-phase test run against a compiled binary, the classic RED→GREEN gate could not be executed in the literal sense (no `test(...)` commit showing the tests failing, followed by a `feat(...)` commit showing them pass) — Tasks 1 and 2 (the `feat` implementation commits) precede Task 3's `test` commit chronologically in this plan, matching a regression-locking-test pattern rather than the classic TDD cycle. This mirrors 39-01's precedent for the same non-classical `tdd="true"` usage. The test file's correctness is instead evidenced by the standalone SQL verification harness documented above, which exercises the identical query text against the identical schema and test data.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

`SqlitePlaceRepository` fully implements `PlaceRepository` (Plan 02's trait). Every recursive-CTE query PLC-06/D-14/D-21/D-25/D-11.4 depend on exists, is callable, and is independently SQL-verified against the real schema. Plan 05 (`place_service.rs`) can build directly against this adapter.

**Blocker inherited, not introduced, by this plan:** `cargo build -p trackly-infra` will keep failing until Wave 3 plans (39-06/07/09/10/11) rewrite `acts_sqlite.rs`/`cartridges_sqlite.rs`/`printers_sqlite.rs`/`requests_sqlite.rs` off the dropped `locations` table onto `place_id`/`place_full_paths`. Once any of those plans lands enough of that rewrite for the crate to compile, run `cargo test -p trackly-infra --test places_crud` to get the first real (compiler-verified) pass/fail signal on this plan's test file — it has only been validated via the standalone SQL harness documented above, never by `rustc`/`cargo test` itself.

---
*Phase: 39-place-tree*
*Completed: 2026-08-22*

## Self-Check: PASSED

All created/modified files confirmed present on disk (`crates/trackly-infra/src/repos/places_sqlite.rs`, `crates/trackly-infra/src/repos/mod.rs`, `crates/trackly-infra/tests/places_crud.rs`, this SUMMARY); all three task commit hashes (`8ac5c8b7`, `a7531f13`, `2f6d59be`) confirmed present in `git log`.

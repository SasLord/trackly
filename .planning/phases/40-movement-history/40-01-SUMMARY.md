---
phase: 40-movement-history
plan: 01
subsystem: database
tags: [sqlite, refinery, rusqlite, domain, tdd]

# Dependency graph
requires:
  - phase: 39-places
    provides: "places tree (places table + place_full_paths view, V037/V038/V039) — place_movements FKs to places(id)"
provides:
  - "place_movements table (migration V040) — standalone append-only journal, 5 indexes, no CHECK constraints on source/entity_type"
  - "MovementSource / MovementEntityKind pure domain enums with from_str_lenient soft-degrade parsing (crates/trackly-core/src/domain/place_movements.rs)"
  - "is_reportable_place_change(before, after) — D-04/D-06 guard every write-site plan in this phase will call"
affects: [40-02, 40-03, 40-04, 40-05, 40-06, 40-07, 40-08, 40-09, 40-10, 40-11, 40-12]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Unconstrained-TEXT-column-with-Rust-side-lenient-parser (mirrors places.kind/path_variant_override precedent from V037/V039) — used for place_movements.source and entity_type"
    - "Append-only journal schema shape (mirrors V008 audit_log: no deleted_at_utc, no version) — used for place_movements"

key-files:
  created:
    - migrations/V040__place_movements.sql
    - crates/trackly-core/src/domain/place_movements.rs
    - crates/trackly-infra/tests/place_movements_migration.rs
  modified:
    - crates/trackly-core/src/domain/mod.rs

key-decisions:
  - "place_movements is a standalone table, not a view/query over audit_log (D-01) — HST-04's two-filter report needs plain indexed WHERE clauses, not per-row JSON parsing"
  - "source/entity_type are bare TEXT with zero SQL CHECK constraints — validation happens only in Rust via from_str_lenient (Pitfall 6 / IN-01: a strict parse on this kind of evolving token column has crashed a screen before)"
  - "Migration performs zero backfill from audit_log (D-02) — table starts empty; history accumulates only from future write-site calls"

patterns-established:
  - "from_str_lenient(&str) -> Option<Self> as the canonical read-side parse signature for soft-degrading enum columns, distinct from the strict from_str(&str) -> Result<Self, AppError> write-time form already used by PlaceKind/PathDisplayVariant"

requirements-completed: [HST-01]

# Metrics
duration: 8min
completed: 2026-09-01
---

# Phase 40 Plan 01: Movement-History Schema Foundation Summary

**Migration V040 (`place_movements` table + 5 indexes) plus pure `MovementSource`/`MovementEntityKind`/`is_reportable_place_change` domain types, built TDD RED→GREEN, giving every later write-site plan in Phase 40 a tested foundation to build on.**

## Performance

- **Duration:** 8 min
- **Started:** 2026-09-01T17:05:15Z
- **Completed:** 2026-09-01T17:13:00Z
- **Tasks:** 3 completed
- **Files modified:** 4 (3 created, 1 modified)

## Accomplishments

- `place_movements` schema (migration V040) landed as a standalone append-only journal with 5 indexes covering the HST-02 timeline query, the HST-04 period/from-place/to-place report filters, and the D-03 act-undo delete path.
- Pure domain types (`MovementSource`, `MovementEntityKind`, `is_reportable_place_change`) built via a genuine TDD RED→GREEN cycle, with `no_io_deps` boundary intact.
- Migration idempotency + fresh-DB empty-seed test suite proves the table applies cleanly, twice, on a fresh DB and starts with zero rows.

## Task Commits

Each task was committed atomically:

1. **Task 1: Migration V040 — place_movements schema** - `ce976643` (feat)
2. **Task 2: Domain types — MovementSource, MovementEntityKind, is_reportable_place_change** - `bcd478b9` (test, RED) → `a955dcac` (feat, GREEN)
3. **Task 3: Wave 0 migration idempotency test** - `9b7fc222` (test)

**Plan metadata:** _(this commit, see final commit below)_

_TDD Task 2 produced two commits (test → feat) per the RED/GREEN gate — no refactor commit was needed._

## Files Created/Modified

- `migrations/V040__place_movements.sql` - `place_movements` table, 5 indexes, `PRAGMA user_version = 40`; no CHECK constraints, no backfill
- `crates/trackly-core/src/domain/place_movements.rs` - `MovementSource`, `MovementEntityKind` (both with `as_str`/`from_str_lenient`), `MovementEntityKind::label_ru`, `is_reportable_place_change` guard, 10 inline unit tests
- `crates/trackly-core/src/domain/mod.rs` - registered `pub mod place_movements;`
- `crates/trackly-infra/tests/place_movements_migration.rs` - 3 integration tests: table+index existence, empty-on-fresh-DB, idempotent re-run

## Decisions Made

- Followed the plan's explicit instruction to mirror `V008__audit_log.sql`'s append-only shape (no `deleted_at_utc`, no `version`) rather than the `places`/standard4 soft-delete convention — this is a journal, not an editable entity.
- Followed the plan's explicit instruction to leave `source`/`entity_type` as bare TEXT with no SQL CHECK, delegating all validation to `from_str_lenient` in Rust — matches the existing `places.kind` vs `path_variant_override` split precedent and avoids repeating Pitfall 6 (IN-01).

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None. `cargo fmt --check` and `cargo clippy --tests -D warnings` were both clean on the touched crates with no fixes required.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `place_movements` table and domain guard are ready for the six write-site plans (device/cartridge place changes, act-driven moves) to call in later plans of this phase.
- `MovementSource::Map` and `MovementSource::Workstation` variants exist and parse correctly even though no write-site writes them yet — later phases (map drag-and-drop, workstation assignment) can use them without a schema change.
- No blockers identified for Plan 40-02.

---
*Phase: 40-movement-history*
*Completed: 2026-09-01*

## Self-Check: PASSED

All 5 created/modified files confirmed present on disk; all 4 task commit hashes
(`ce976643`, `bcd478b9`, `a955dcac`, `9b7fc222`) confirmed present in git history.

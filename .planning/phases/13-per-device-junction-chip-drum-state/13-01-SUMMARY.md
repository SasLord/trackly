---
phase: 13-per-device-junction-chip-drum-state
plan: 01
subsystem: database
tags: [sqlite, rusqlite, refinery, migrations, cartridges, compatibility]

# Dependency graph
requires:
  - phase: 12-cartridge-request-interconnection
    provides: V029 printer_cartridge_models junction table + V005 cartridge_model_compatibility free-text table (both superseded here)
provides:
  - "V032 migration: cartridge_model_compatibility.printer_name (single free-text column, replaces printer_brand+printer_model); printer_cartridge_models (V029) dropped entirely"
  - "SqliteCartridgeRepository::upsert_compatibility_in_tx/get_compatibility on Vec<String> printer names"
  - "SqliteCartridgeRepository::list() compatibility filter rewritten to match printer_name against devices.name (case-insensitive, TRIM'd) with D-05 pass-through"
  - "SqliteCartridgeRepository::compatible_model_aggregates() — new read method for the printer-card compatible-models widget (R4), no pass-through"
  - "CompatibleModelAggregate domain type in trackly-core"
affects: [13-per-device-junction-chip-drum-state (remaining plans: printer repo, cartridge_service/DTO layer, frontend)]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "SQLite table-rebuild migration pattern (CREATE _new -> INSERT SELECT with TRIM-transform -> DROP -> RENAME) inside one PRAGMA foreign_keys OFF/ON block per V030/V031 precedent, now also used to collapse two TEXT columns into one"
    - "Printer-name compatibility matching: LOWER(TRIM(cmc.printer_name)) = LOWER(TRIM(d.name)) joined through devices, replacing per-device-id junction table lookups"

key-files:
  created:
    - migrations/V032__cartridge_model_compatibility_printer_name.sql
  modified:
    - crates/trackly-infra/src/repos/cartridges_sqlite.rs
    - crates/trackly-core/src/domain/cartridges.rs

key-decisions:
  - "upsert_compatibility_in_tx stores printer_name values exactly as given (no TRIM at write time) — normalisation (LOWER+TRIM) is applied only at comparison time in list()/compatible_model_aggregates, per D-02/D-03/D-04"
  - "D-05 pass-through (empty compatibility row set for a model = matches any printer) is scoped strictly to the cartridge-selection filter (list()'s ?6 condition) and intentionally NOT applied in compatible_model_aggregates — R4/D-07 require the printer-card aggregate to reflect only real compatibility rows so an empty aggregate ('Нет совместимых моделей картриджей.') stays reachable"
  - "installable_only (?5) SQL fragment left byte-for-byte unchanged in both the COUNT and main SELECT queries while rewriting the adjacent ?6 condition (D-12 guard), verified via grep count == 2"

requirements-completed: [SPEC-13-R1, SPEC-13-R2]

# Metrics
duration: 35min
completed: 2026-06-25
---

# Phase 13 Plan 01: V032 schema migration + cartridge repository rewrite Summary

**V032 migration collapses printer_brand+printer_model into a single printer_name column and drops the V029 per-device junction table; cartridges_sqlite.rs rewritten to match compatibility by printer_name against devices.name with D-05 pass-through scoped to the selection filter only.**

## Performance

- **Duration:** 35 min
- **Started:** 2026-06-25T22:39:00Z
- **Completed:** 2026-06-25T23:14:45Z
- **Tasks:** 3
- **Files modified:** 3 (1 created, 2 modified)

## Accomplishments
- Created `migrations/V032__cartridge_model_compatibility_printer_name.sql`: rebuilds `cartridge_model_compatibility` with a single `printer_name` column (data migrated via `TRIM(printer_brand || ' ' || printer_model)`, no rows lost), and drops `printer_cartridge_models` (V029) in the same migration file
- Rewrote `upsert_compatibility_in_tx`/`get_compatibility` in `cartridges_sqlite.rs` to operate on `Vec<String>` printer names instead of `(brand, model)` tuples
- Rewrote both the COUNT and main SELECT queries in `list()` to match `cartridge_model_compatibility.printer_name` against the target printer's `devices.name` (case-insensitive, TRIM'd), with D-05 pass-through when a model has zero compatibility rows at all — while leaving the adjacent `installable_only` (?5) SQL fragment byte-for-byte unchanged (D-12)
- Added `compatible_model_aggregates()` — a new read method that returns per-model in_stock/at_refill/in_use counts for a printer, intentionally without pass-through (R4/D-07)
- Added `CompatibleModelAggregate` domain type in `trackly-core` and updated `CartridgeFilter.compatible_with_printer_device_id`'s doc comment to describe the V005-only printer_name semantics

## Task Commits

Each task was committed atomically:

1. **Task 1: V032 миграция — printer_name + DROP printer_cartridge_models** - `8be2799` (feat)
2. **Task 3: domain/cartridges.rs — типы и doc-комментарии под V005-only схему** - `4d699f0` (feat) — executed before Task 2 so `CompatibleModelAggregate` was already available when writing the repository method
3. **Task 2: cartridges_sqlite.rs — совместимость на Vec\<String\>** - `262924c` (feat)

_Note: TDD not used in this plan (tdd="false" / task type=auto)._

## Files Created/Modified
- `migrations/V032__cartridge_model_compatibility_printer_name.sql` - Rebuilds cartridge_model_compatibility (printer_brand+printer_model -> printer_name), drops printer_cartridge_models
- `crates/trackly-infra/src/repos/cartridges_sqlite.rs` - upsert_compatibility_in_tx/get_compatibility on Vec<String>; list() filter rewritten; new compatible_model_aggregates()
- `crates/trackly-core/src/domain/cartridges.rs` - New CompatibleModelAggregate type; updated CartridgeFilter doc comment

## Decisions Made
- Executed Task 3 (domain type) before Task 2 (repository) so the repository code could reference `CompatibleModelAggregate` directly without a temporary placeholder — no functional difference from the plan's stated order, pure execution-order optimization
- See `key-decisions` in frontmatter for the three substantive plan decisions (write-time vs compare-time normalisation; pass-through scoping; D-12 guard verification)

## Deviations from Plan

None - plan executed exactly as written. The plan explicitly scoped `files_modified` and verification (`cargo build -p trackly-infra -p trackly-core`) to exclude `trackly-app`, anticipating that the service/DTO layer (`crates/trackly-app/src/services/cartridge_service.rs`, which still calls `upsert_compatibility_in_tx`/`get_compatibility` with the old `(String, String)` tuple shape) would not compile until a later Phase 13 plan updates it. Confirmed this is the case — see "Known Stubs / Temporary Build State" below.

## Issues Encountered

None - all three tasks completed without rework. First-pass SQL for `compatible_model_aggregates` had a JOIN-ordering slip (LEFT JOIN cartridges placed after WHERE instead of after the other JOINs) caught and fixed before running `cargo build` for the first time — not counted as a deviation since it was an in-progress authoring fix, not a deviation from an already-verified state.

## Known Stubs / Temporary Build State

`crates/trackly-app/src/services/cartridge_service.rs` (lines ~505, 523, 593, 662) still calls `get_compatibility`/`upsert_compatibility_in_tx` with the pre-Phase-13 `Vec<(String, String)>` shape — this crate **does not currently compile** (`cargo build -p trackly-app` fails with 4 type-mismatch errors). This is the explicit, intentional boundary of Plan 13-01 per its frontmatter (`files_modified` limited to migration + `cartridges_sqlite.rs` + `domain/cartridges.rs`; verification scoped to `-p trackly-infra -p trackly-core`). A subsequent Phase 13 plan (service/DTO layer) must update `cartridge_service.rs` and the `CartridgeModelCreateDto`/`CartridgeModelPatchDto` compatibility field types to `Vec<String>` before the workspace builds cleanly end-to-end. This is not a stub left for "the verifier to catch unexpectedly" — it is the documented, planned intermediate state of a multi-plan schema migration.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Ready for the next Phase 13 plan: the V032 schema and repository-layer contract (`Vec<String>` compatibility, `compatible_model_aggregates`) are in place. The printer repository plan and the cartridge service/DTO plan can now build directly on this foundation. `trackly-app` will need its compatibility-field DTOs and `cartridge_service.rs` call sites updated to the new `Vec<String>` shape before the full workspace compiles.

---
*Phase: 13-per-device-junction-chip-drum-state*
*Completed: 2026-06-25*

## Self-Check: PASSED

All created/modified files and all 4 commit hashes (8be2799, 4d699f0, 262924c, 9c5d68f) verified present.

---
phase: "04"
plan: "01"
subsystem: cartridges
tags: [migration, fts, schema, tdd-red, test-scaffold]
dependency_graph:
  requires: []
  provides:
    - V016 migration (cartridge_kinds, color, app_settings, FTS triggers, user_version=16)
    - 6 RED-phase test scaffolds for CartridgeService (plan 04-03 GREEN target)
  affects:
    - crates/trackly-infra (migration runner tests updated 15→16)
    - crates/trackly-app/tests (6 new test files added)
tech_stack:
  added: []
  patterns:
    - "ALTER TABLE ADD COLUMN NOT NULL DEFAULT N (Pitfall 2: required for existing rows)"
    - "FTS5 external-content trigger pattern (V013 analog)"
    - "RED-phase todo!() scaffold with #[allow(dead_code)] on unused setup fn"
key_files:
  created:
    - migrations/V016__cartridges_kind_color_settings.sql
    - crates/trackly-app/tests/cartridges_crud.rs
    - crates/trackly-app/tests/cartridges_numbering.rs
    - crates/trackly-app/tests/cartridges_lifecycle.rs
    - crates/trackly-app/tests/cartridges_search.rs
    - crates/trackly-app/tests/cartridges_low_stock.rs
    - crates/trackly-app/tests/cartridges_history.rs
  modified:
    - crates/trackly-infra/src/test_support/test_db.rs
    - crates/trackly-infra/src/db/migrations.rs
    - crates/trackly-infra/tests/migration_idempotency.rs
decisions:
  - "make_cartridge_service returns tempfile::TempDir (not CartridgeService) in scaffolds — CartridgeService does not exist until plan 04-03; return type placeholder allows compile-clean RED tests without phantom type"
  - "migrations.rs tests updated 15→16 (deviation Rule 2: required for CI not to break after V016)"
metrics:
  duration: "~6 minutes"
  completed: "2026-06-08"
  tasks: 2
  files: 10
---

# Phase 04 Plan 01: Migration V016 + RED Test Scaffolds Summary

Migration V016 adds cartridge_kinds lookup, color column, app_settings with low_stock_threshold, and three FTS sync triggers (cartridges_fts_ai/ad/au); six compile-clean RED-phase test files scaffold all CartridgeService behaviors for plan 04-03 GREEN phase.

## Tasks Completed

| # | Task | Commit | Files |
|---|------|--------|-------|
| 1 | Migration V016 + version assertions 15→16 | 36369c0 | migrations/V016, migrations.rs, test_db.rs, migration_idempotency.rs |
| 2 | RED scaffold: 6 cartridge test files (todo!() bodies) | 29a91b5 | 6 new test files |

## What Was Built

**Task 1 — Migration V016:**
- `CREATE TABLE cartridge_kinds` with two rows: (1, 'Картридж'), (2, 'Фотобарабан')
- `ALTER TABLE cartridge_models ADD COLUMN kind_id INTEGER NOT NULL DEFAULT 1` (FK → cartridge_kinds) and `color TEXT NULL`
- `CREATE TABLE app_settings` with seed row `('low_stock_threshold', '2', ...)`
- Three FTS sync triggers: `cartridges_fts_ai` (INSERT), `cartridges_fts_ad` (DELETE), `cartridges_fts_au` (UPDATE), mirroring V013 pattern for `cartridges_fts` virtual table (fields: code, location, holder_name)
- `PRAGMA user_version = 16` at end
- Updated `migrations.rs` lib tests, `test_db.rs` assertion, and `migration_idempotency.rs` integration test from 15 → 16

**Task 2 — RED Test Scaffolds:**
- `cartridges_crud.rs`: create_auto_code, create_custom_code, get-404, soft_delete, counts_by_status, rejects_invalid_custom_code
- `cartridges_numbering.rs`: concurrent_50_unique_codes, collision_retry_does_not_lose_counter
- `cartridges_lifecycle.rs`: install, return_to_stock, to_refill, from_refill, write_off, all_transitions_write_audit_log
- `cartridges_search.rs`: search_by_code, search_by_model_brand, search_by_location, empty_query_returns_all
- `cartridges_low_stock.rs`: low_stock_below_threshold, full_stock_excluded, threshold_from_app_settings
- `cartridges_history.rs`: returns_audit_entries, is_chronological

All 6 files: `cargo check -p trackly-app --tests` passes with zero errors and zero warnings.

## Verification Results

- `cargo test -p trackly-infra`: 49 passed, 0 failed (across all test suites)
- `cargo check -p trackly-app --tests`: clean
- `PRAGMA user_version = 16` present in V016
- Three FTS triggers (cartridges_fts_ai/ad/au) present in V016
- `assert_eq!(user_version, 16)` present in test_db.rs
- `idx_audit_log_entity` exists in V012, NOT duplicated in V016
- `rejects_invalid_custom_code` test present in cartridges_crud.rs

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Updated version assertions in migrations.rs and migration_idempotency.rs**
- **Found during:** Task 1 — first test run after creating V016
- **Issue:** `migrations.rs` lib tests hardcoded `assert_eq!(report.schema_version, 15)` and `assert_eq!(max_known_version(), 15)`; `migration_idempotency.rs` integration test also hardcoded 15. After V016, these immediately failed.
- **Fix:** Updated all three test files from 15 → 16. The plan only mentioned `test_db.rs`, but CI correctness required all migration-version assertions to be consistent.
- **Files modified:** `crates/trackly-infra/src/db/migrations.rs`, `crates/trackly-infra/tests/migration_idempotency.rs`
- **Commit:** 36369c0

## Known Stubs

All 6 test files in `crates/trackly-app/tests/cartridges_*.rs` have `todo!()` bodies — this is intentional per plan 04-01 design (RED phase). These stubs will be replaced in plan 04-03 (GREEN phase) when `CartridgeService` is implemented.

`make_cartridge_service()` returns `tempfile::TempDir` instead of `(CartridgeService, TempDir)` because `CartridgeService` does not exist until plan 04-03. This is an intentional placeholder; plan 04-03 Task 1 will rewrite these setup functions.

## Self-Check: PASSED

Files verified:
- `/Users/madsas/Projects/trackly/migrations/V016__cartridges_kind_color_settings.sql` — EXISTS
- `/Users/madsas/Projects/trackly/crates/trackly-infra/src/test_support/test_db.rs` — contains `assert_eq!(user_version, 16)`
- `/Users/madsas/Projects/trackly/crates/trackly-app/tests/cartridges_crud.rs` — EXISTS, contains `rejects_invalid_custom_code`
- `/Users/madsas/Projects/trackly/crates/trackly-app/tests/cartridges_numbering.rs` — EXISTS
- `/Users/madsas/Projects/trackly/crates/trackly-app/tests/cartridges_lifecycle.rs` — EXISTS
- `/Users/madsas/Projects/trackly/crates/trackly-app/tests/cartridges_search.rs` — EXISTS
- `/Users/madsas/Projects/trackly/crates/trackly-app/tests/cartridges_low_stock.rs` — EXISTS
- `/Users/madsas/Projects/trackly/crates/trackly-app/tests/cartridges_history.rs` — EXISTS

Commits verified:
- `36369c0` — feat(04-01): migration V016
- `29a91b5` — test(04-01): RED scaffold

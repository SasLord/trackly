---
phase: "04-cartridges"
plan: "03"
subsystem: cartridges-app-layer
tags: [cartridges, service, dto, tauri-commands, http-router, specta, green-tests]
dependency_graph:
  requires:
    - "04-01 (migration + RED tests)"
    - "04-02 (domain/ports/infra layer)"
  provides:
    - CartridgeService (create/update/delete/get/list/transition/search/counts/history/low_stock/models/suggest)
    - CartridgeDto + all DTO types (CartridgeTransitionPayload, CartridgeCreateDto, LowStockItemDto, AuditEntryDto, etc.)
    - 19 Tauri commands registered in specta_export
    - HTTP router /api/v1/cartridges_* (not bound until Phase 5)
    - AppCtx.cartridges: Arc<CartridgeService>
  affects:
    - "04-04+ (Svelte UI will use Tauri commands and types from bindings.ts)"
    - "05+ (HTTP server will bind http::cartridges::router())"
tech_stack:
  added: []
  patterns:
    - "CartridgeTransitionPayload as #[serde(tag = \"op\")] discriminated union"
    - "validate_create() guards code_override: empty, oversized (>32), control chars"
    - "writer.execute() for all mutations (single-writer pattern)"
    - "build_* helpers shared by Tauri commands and axum handlers (dual-transport pattern)"
    - "#[allow(clippy::too_many_arguments)] on infra-layer insert/update SQL functions"
key_files:
  created:
    - crates/trackly-app/src/dto/cartridge.rs
    - crates/trackly-app/src/services/cartridge_service.rs
    - crates/trackly-app/src/tauri_cmds/cartridges.rs
    - crates/trackly-app/src/http/cartridges.rs
  modified:
    - crates/trackly-app/src/dto/mod.rs
    - crates/trackly-app/src/services/mod.rs
    - crates/trackly-app/src/context.rs
    - crates/trackly-app/src/tauri_cmds/mod.rs
    - crates/trackly-app/src/http/mod.rs
    - crates/trackly-app/src/specta_export.rs
    - crates/trackly-app/tests/cartridges_crud.rs
    - crates/trackly-app/tests/cartridges_numbering.rs
    - crates/trackly-app/tests/cartridges_lifecycle.rs
    - crates/trackly-app/tests/cartridges_search.rs
    - crates/trackly-app/tests/cartridges_low_stock.rs
    - crates/trackly-app/tests/cartridges_history.rs
    - crates/trackly-infra/src/repos/cartridges_sqlite.rs
    - crates/trackly-app/tests/acts_clone_handover.rs
    - crates/trackly-app/tests/downgrade_protection.rs
    - crates/trackly-app/tests/health_smoke.rs
    - crates/trackly-app/tests/specta_roundtrip.rs
    - crates/trackly-app/src/http/health.rs
    - crates/trackly-app/src/tauri_cmds/health.rs
decisions:
  - "CartridgeTransitionPayload as #[serde(tag='op')] enum — each variant carries cartridge_id + version + op-specific fields; From<> impl converts to domain CartridgeTransitionOp for infra dispatch"
  - "validate_create enforces T-04-03-01 threat model: rejects empty code_override, >32 chars, control chars (< U+0020) — no SQL injection risk since all writes use params![]"
  - "#[allow(clippy::too_many_arguments)] on insert_cartridge_in_tx and update_model_in_tx — raw SQL functions that cannot be refactored into structs without adding serde/boilerplate to the infra layer"
metrics:
  duration_minutes: 19
  completed_date: "2026-06-08"
  tasks_completed: 2
  files_created: 4
  files_modified: 19
  tests_added: 23
  tests_fixed: 3
---

# Phase 04 Plan 03: Cartridges App Layer Summary

CartridgeService (817 LoC) + CartridgeDto layer (435 LoC) + 19 Tauri commands + axum HTTP router + all 23 RED integration tests turned GREEN.

## Tasks Completed

| Task | Description | Commit | Files |
|------|-------------|--------|-------|
| 1 | DTO layer + CartridgeService + AppCtx wire-up | 8ea67bc | dto/cartridge.rs, services/cartridge_service.rs, context.rs, dto/mod.rs, services/mod.rs |
| 2 | Tauri commands + HTTP router + specta + GREEN tests | 0fd6e20 | tauri_cmds/cartridges.rs, http/cartridges.rs, specta_export.rs, 6 test files, 3 fixture files, infra fix |

## What Was Built

### Task 1: DTO + Service + AppCtx

**`crates/trackly-app/src/dto/cartridge.rs`** (435 LoC)
- `CartridgeDto`: `From<CartridgeRow>`, all i64 fields annotated `#[specta(type=i32)]`
- `CartridgeTransitionPayload`: `#[serde(tag="op")]` enum — Install/ReturnToStock/ToRefill/FromRefill/WriteOff with cartridge_id + version + op-specific fields
- `impl From<CartridgeTransitionPayload> for CartridgeTransitionOp` for domain dispatch
- `CartridgeCreateDto`, `CartridgeFilter`, `Pagination`, `CartridgeListResponse`, `CartridgeCountsDto`, `LowStockItemDto`, `AuditEntryDto`
- `CartridgeModelDto`, `CartridgeModelCreateDto`, `CartridgeModelUpdateDto`

**`crates/trackly-app/src/services/cartridge_service.rs`** (817 LoC)
- `validate_create()`: rejects empty code_override, >32 chars, control chars — T-04-03-01 threat mitigation
- `create()`: writer.execute → assign_code_in_tx → insert_cartridge_in_tx → audit_log (atomic)
- `transition()`: writer.execute → cart_repo.transition_in_tx (atomic status change + audit)
- `low_stock()`: reads threshold from app_settings, counts in-stock+full cartridges per model
- Full CRUD for cartridges and cartridge_models
- Suggest autocomplete helpers (brand/model/compat_printer/location)
- All writes through single-writer WriterHandle; reads from reader pool

**`crates/trackly-app/src/context.rs`**
- Added `pub cartridges: Arc<CartridgeService>` field
- Wired in `build()`: `CartridgeService::new(writer, readers, clock)`

### Task 2: Commands + Router + GREEN Tests

**`crates/trackly-app/src/tauri_cmds/cartridges.rs`** (337 LoC)
- 19 thin `#[tauri::command] #[specta::specta]` wrappers calling `build_*` helpers
- All i64 params declared as i32 in command signatures (TS bigint avoidance)
- `build_*` helpers reused by axum handlers (dual-transport pattern)

**`crates/trackly-app/src/http/cartridges.rs`** (357 LoC)
- 19 POST routes at `/api/v1/cartridges_*`
- JSON payload extractor structs per handler
- `pub fn router() -> Router<AppCtx>` — built but not bound (Phase 5)

**6 test files turned GREEN** (23 tests total):
- `cartridges_crud.rs`: create/custom-code/get-404/soft-delete/counts/validation (6 tests)
- `cartridges_numbering.rs`: 50 concurrent unique codes/collision retry (2 tests)
- `cartridges_lifecycle.rs`: all 5 transitions + audit log write (6 tests)
- `cartridges_search.rs`: code/brand/location/empty-query (4 tests)
- `cartridges_low_stock.rs`: threshold/full-stock/app_settings (3 tests)
- `cartridges_history.rs`: entries + chronological order (2 tests)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Pre-existing test: `migration_v015_applies_clean` hardcoded version 15**
- **Found during:** Task 2 full test run
- **Issue:** `acts_clone_handover.rs:68` asserted `user_version == 15` but plan 04-01 added V016, bringing schema to version 16
- **Fix:** Changed assertion to `>= 15` with comment explaining V016 was added in plan 04-01
- **Files modified:** `crates/trackly-app/tests/acts_clone_handover.rs`
- **Commit:** 0fd6e20

**2. [Rule 1 - Bug] Pre-existing test: `appctx_build_rejects_newer_db_and_leaves_file_byte_identical` hardcoded binary=15**
- **Found during:** Task 2 full test run
- **Issue:** `downgrade_protection.rs:90` asserted `binary == 15` but max_known_version is now 16
- **Fix:** Changed assertion to `== 16`
- **Files modified:** `crates/trackly-app/tests/downgrade_protection.rs`
- **Commit:** 0fd6e20

**3. [Rule 1 - Bug] Pre-existing test: `health_smoke_end_to_end_against_real_app_ctx` hardcoded schema_version=15**
- **Found during:** Task 2 full test run
- **Issue:** `health_smoke.rs:34` asserted `schema_version == 15`; real AppCtx::build now returns 16
- **Fix:** Changed assertion to `== 16`
- **Files modified:** `crates/trackly-app/tests/health_smoke.rs`
- **Commit:** 0fd6e20

**4. [Rule 2 - Missing] clippy::too_many_arguments on infra SQL functions**
- **Found during:** workspace clippy run
- **Issue:** `insert_cartridge_in_tx` and `update_model_in_tx` have 10 parameters; clippy limit is 7
- **Fix:** Added `#[allow(clippy::too_many_arguments)]` — infra raw SQL functions cannot be meaningfully refactored without adding serde/wrapper boilerplate to the infra layer
- **Files modified:** `crates/trackly-infra/src/repos/cartridges_sqlite.rs`
- **Commit:** 0fd6e20

**5. [Rule 1 - Bug] Unused import `CartridgeTransitionPayload` in cartridges_low_stock test**
- **Found during:** workspace clippy run
- **Issue:** `cartridges_low_stock.rs:17` imported `CartridgeTransitionPayload` but never used it
- **Fix:** Removed the unused import
- **Files modified:** `crates/trackly-app/tests/cartridges_low_stock.rs`
- **Commit:** 0fd6e20

**6. [Rule 1 - Bug] Three test fixtures missing `cartridges` AppCtx field**
- **Found during:** Task 2 compilation
- **Issue:** `specta_roundtrip.rs`, `http/health.rs`, `tauri_cmds/health.rs` manually construct `AppCtx { ... }` and were missing the new `cartridges` field
- **Fix:** Added `CartridgeService::new(...)` construction + `cartridges` field to all three fixtures
- **Files modified:** `crates/trackly-app/tests/specta_roundtrip.rs`, `crates/trackly-app/src/http/health.rs`, `crates/trackly-app/src/tauri_cmds/health.rs`
- **Commit:** 0fd6e20

## Verification

- `cargo clippy --workspace --tests -- -D warnings`: clean
- `cargo test -p trackly-app`: all tests pass (0 failures across 40 test files)
- `cargo test -p trackly-app --test export_bindings`: bindings.ts regenerated with cartridge types

## Known Stubs

None — all CartridgeService methods are fully implemented. The HTTP router is intentionally not bound to the axum app (Phase 5 will wire it via `merge(http::cartridges::router())`).

## Threat Flags

No new network endpoints, auth paths, or trust boundary crossings introduced in this plan. The HTTP router `/api/v1/cartridges_*` is built but not reachable until Phase 5 binds it. The `validate_create()` function implements T-04-03-01 threat mitigation for code_override injection.

## Self-Check: PASSED

- `crates/trackly-app/src/dto/cartridge.rs`: FOUND
- `crates/trackly-app/src/services/cartridge_service.rs`: FOUND
- `crates/trackly-app/src/tauri_cmds/cartridges.rs`: FOUND
- `crates/trackly-app/src/http/cartridges.rs`: FOUND
- Task 1 commit 8ea67bc: FOUND
- Task 2 commit 0fd6e20: FOUND

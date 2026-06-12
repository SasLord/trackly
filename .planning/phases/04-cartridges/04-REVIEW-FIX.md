---
phase: 04-cartridges
fixed_at: 2026-06-12T19:20:00Z
review_path: .planning/phases/04-cartridges/04-REVIEW.md
iteration: 1
findings_in_scope: 8
fixed: 8
skipped: 0
status: all_fixed
---

# Phase 4: Code Review Fix Report

**Fixed at:** 2026-06-12T19:20:00Z
**Source review:** `.planning/phases/04-cartridges/04-REVIEW.md`
**Iteration:** 1

**Summary:**
- Findings in scope: 8 (CR-01, CR-02, WR-01..WR-06)
- Fixed: 8
- Skipped: 0

---

## Fixed Issues

### CR-01: Compatibility-printer autocomplete sends wrong `field` value

**Files modified:** `ui/src/features/cartridges/api.ts`, `ui/src/features/cartridges/ModelFormModal.svelte`
**Commit:** `92f8cf3`
**Applied fix:** Changed `suggestCompatPrinter` type annotation from `'brand' | 'model'` to `'printer_brand' | 'printer_model'` in `api.ts`. Updated both call sites in `ModelFormModal.svelte` from `'brand'`/`'model'` to `'printer_brand'`/`'printer_model'`. The autocomplete was calling the backend with field names that failed its whitelist check — every call returned a Validation error and the dropdown silently showed "Нет совпадений".

### CR-02: Cartridge history list keyed by `created_at_utc` — duplicate-second rows dropped

**Files modified:** `crates/trackly-app/src/dto/cartridge.rs`, `crates/trackly-infra/src/repos/cartridges_sqlite.rs`, `crates/trackly-app/src/services/cartridge_service.rs`, `ui/src/features/cartridges/CartridgeDetail.svelte`
**Commit:** `6aebccf`
**Applied fix:** Added `id: i64` (with `#[specta(type = i32)]`) to `AuditEntryDto`. Added `id: i64` to `AuditEntryRow`. Updated the `get_history` SELECT to include `id` as the first column with adjusted positional indices (0..8). Updated the service mapper to include `id: r.id`. Changed `{#each history as entry (entry.created_at_utc)}` to `{#each history as entry (entry.id)}` in `CartridgeDetail.svelte`. Bindings regenerated via `cargo test --test export_bindings` (file gitignored, regenerated on each cargo test run).

### WR-01: FTS search builds invalid/empty MATCH for punctuation-only queries

**Files modified:** `crates/trackly-infra/src/repos/cartridges_sqlite.rs`
**Commit:** `b32dc33`
**Applied fix:** Added a `has_token` guard (`query.chars().any(|c| c.is_alphanumeric())`). When false, the FTS5 CTE is omitted and the query falls back to LIKE-only. Also consolidated params — the FTS term is now inlined (escaped) into the SQL string rather than being a format param, so the remaining filter params are consistently `?1..?4` in both code paths. Added test `search_punctuation_only_query_returns_ok` covering `"---"`, `"..."`, `"\""`, `"   "`, `"!!"`.

### WR-02: Model create/update conflict surfaces as raw SQLite UNIQUE error

**Files modified:** `crates/trackly-app/src/services/cartridge_service.rs`
**Commit:** `a9d460d`
**Applied fix:** Added a pre-check for existing live `(brand, model)` in both `model_create` (inside the writer transaction before INSERT) and `model_update` (inside the writer transaction, excluding the current row by `id != ?3`). Returns `AppError::Conflict { reason: "Модель «X Y» уже существует" }` — matching the pattern from `assign_code_in_tx`. The UI's inline conflict matcher (`includes('уже')`) now correctly shows the inline message instead of falling through to a generic error toast.

### WR-03: OperationModal does not reset `stateId` when `op` changes without open→close

**Files modified:** `ui/src/features/cartridges/OperationModal.svelte`
**Commit:** `fd8fbe2`
**Applied fix:** Added `void op;` at the top of the reset `$effect` body. This creates an explicit Svelte 5 reactive dependency on `op` so the effect re-runs whenever `op` changes — not only when `open` toggles. Prevents stale `state_id` when a caller swaps op on an already-open modal (e.g., from `return_to_stock` → `from_refill` without a close/reopen cycle).

### WR-04: Dead/misleading code in `CartridgeService::delete`

**Files modified:** `crates/trackly-app/src/services/cartridge_service.rs`
**Commit:** `b63c8f9`
**Applied fix:** Removed the `let cart_repo = self.cart_repo.clone();` line that captured the repo into the writer closure, and removed the `drop(cart_repo); // not needed inside this tx` line along with the preceding stream-of-consciousness comment block. The soft-delete UPDATE SQL is the single source of truth; no repo helper duplication exists.

### WR-05: HTTP/Tauri transport parity for `id`/`version` integer widths

**Files modified:** `crates/trackly-app/src/http/cartridges.rs`
**Commit:** `513c0d8`
**Applied fix:** Changed `GetPayload.id`, `UpdatePayload.id`/`version`, and `DeletePayload.id`/`version` from `i64` to `i32` to match the `#[specta(type = i32)]` annotation on `CartridgeDto` fields and the Tauri command signatures. Cast to `i64` at each `build_*` call site. All handlers that use `GetPayload` (both `handler_get` and `handler_get_history`) and `handler_models_get`/`handler_models_delete` were updated consistently.

### WR-06: `low_stock` threshold silently becomes 0 for non-numeric settings value

**Files modified:** `crates/trackly-infra/src/repos/cartridges_sqlite.rs`
**Commit:** `8d36124`
**Applied fix:** Replaced the `SELECT CAST(value AS INTEGER) ...` query (which silently returns 0 for non-numeric strings, bypassing the `unwrap_or(2)` fallback) with a raw string select followed by Rust-side parse: `s.trim().parse::<i64>().ok().filter(|&t| t > 0).unwrap_or(2)`. A malformed, empty, missing, or non-positive setting now correctly falls back to the default threshold of 2.

---

## Skipped Issues

No in-scope findings were skipped. All 8 Critical + Warning findings were fixed.

Info findings (IN-01..IN-05) were out of scope per fix_scope=critical_warning.

---

## Test / Lint Status

- `cargo test --workspace`: **all pass** (35+ infra tests, cartridge-specific tests, export_bindings)
- `ui: svelte-check`: **0 errors**, 28 pre-existing warnings (unchanged)
- `ui: eslint`: **clean** (0 errors, 0 warnings)
- `ui: prettier --check`: **all unchanged** (no formatting drift)
- Bindings regenerated (`ui/src/bindings.ts` — gitignored, updated by export_bindings test)

---

_Fixed: 2026-06-12T19:20:00Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_

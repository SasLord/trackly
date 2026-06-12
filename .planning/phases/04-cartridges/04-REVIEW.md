---
phase: 04-cartridges
reviewed: 2026-06-12T11:57:21Z
depth: standard
files_reviewed: 28
files_reviewed_list:
  - crates/trackly-core/src/domain/cartridges.rs
  - crates/trackly-core/src/ports/cartridges.rs
  - crates/trackly-infra/src/repos/cartridges_sqlite.rs
  - crates/trackly-infra/src/db/migrations.rs
  - crates/trackly-app/src/dto/cartridge.rs
  - crates/trackly-app/src/services/cartridge_service.rs
  - crates/trackly-app/src/tauri_cmds/cartridges.rs
  - crates/trackly-app/src/http/cartridges.rs
  - crates/trackly-app/src/context.rs
  - crates/trackly-app/src/specta_export.rs
  - migrations/V016__cartridges_kind_color_settings.sql
  - ui/src/features/cartridges/api.ts
  - ui/src/features/cartridges/CartridgesPage.svelte
  - ui/src/features/cartridges/CartridgeContextMenu.svelte
  - ui/src/features/cartridges/OperationModal.svelte
  - ui/src/features/cartridges/CartridgeFormModal.svelte
  - ui/src/features/cartridges/CartridgeFormBody.svelte
  - ui/src/features/cartridges/ModelFormModal.svelte
  - ui/src/features/cartridges/CompatibilityEditor.svelte
  - ui/src/features/cartridges/LowStockBanner.svelte
  - ui/src/features/cartridges/CartridgesList.svelte
  - ui/src/features/cartridges/CartridgeDetail.svelte
  - ui/src/features/cartridges/CartridgeFilters.svelte
  - ui/src/features/cartridges/ModelsList.svelte
  - ui/src/features/cartridges/CartridgesMasterDetail.svelte
  - ui/src/features/cartridges/CartridgesSearchAndTabs.svelte
  - ui/src/features/layout/sidebar-config.ts
findings:
  critical: 2
  warning: 6
  info: 5
  total: 13
status: issues_found
fix_applied: 2026-06-12T19:20:00Z
fixed:
  - CR-01
  - CR-02
  - WR-01
  - WR-02
  - WR-03
  - WR-04
  - WR-05
  - WR-06
deferred:
  - IN-01
  - IN-02
  - IN-03
  - IN-04
  - IN-05
---

# Phase 4: Code Review Report

**Reviewed:** 2026-06-12T11:57:21Z
**Depth:** standard
**Files Reviewed:** 28
**Status:** issues_found

## Summary

Reviewed the Cartridges (Картриджи) phase across the full hexagonal stack: domain (`trackly-core`), SQLite adapter (`trackly-infra`), application services + DTOs + Tauri/HTTP transports (`trackly-app`), and the Svelte 5 UI feature module.

SQL parameterization is solid — every query uses `params![]`, the FTS5 MATCH input is bound (not concatenated), and the one place that interpolates a column name (`suggest_compat_printer`) uses a hard whitelist. The single-writer discipline and optimistic-lock pattern are applied consistently. Status-transition validation in the domain layer is correct and well-tested.

However, two real functional defects ship in this phase. The most serious is a **wire-format mismatch**: the compatibility-printer autocomplete in the UI sends `field: "brand" | "model"`, but the backend whitelist only accepts `"printer_brand" | "printer_model"` — so that autocomplete always returns a Validation error and is fully broken. Second, the cartridge-history list in `CartridgeDetail.svelte` is keyed by `created_at_utc` (a per-second timestamp), which collides whenever two audit rows land in the same second (common: create + transition, or rapid operations) and causes Svelte to drop/duplicate rows. Six warnings cover an FTS empty-query edge case, an inconsistent model-conflict error message that breaks the UI's inline-conflict matcher, a stale-`stateId` reset gap in `OperationModal`, and dead/confusing code in the service delete path.

## Critical Issues

### CR-01: Compatibility-printer autocomplete sends wrong `field` value — endpoint always rejects

**File:** `ui/src/features/cartridges/api.ts:67` (and `ui/src/features/cartridges/ModelFormModal.svelte:428-429`)
**Issue:** The backend `CartridgeService::suggest_compat_printer` (`crates/trackly-app/src/services/cartridge_service.rs:756-765`) whitelists the `field` argument and returns `AppError::Validation` for anything other than `"printer_brand"` or `"printer_model"`:

```rust
let col = match field.as_str() {
    "printer_brand" => "printer_brand",
    "printer_model" => "printer_model",
    other => return Err(AppError::Validation { ... }),
};
```

But the UI sends `"brand"` / `"model"`:

```ts
// api.ts
suggestCompatPrinter: (field: 'brand' | 'model', prefix: string) =>
  apiCall<string[]>('cartridges_suggest_compat_printer', { field, prefix }),
```

```svelte
<!-- ModelFormModal.svelte -->
suggestBrandFn={(prefix) => cartridges.suggestCompatPrinter('brand', prefix)}
suggestModelFn={(prefix) => cartridges.suggestCompatPrinter('model', prefix)}
```

Result: every call to the compatibility-printer autocomplete throws Validation (`"Недопустимое поле: brand"`). `CompatibilityEditor.fetchSuggestions` swallows the rejection in its `catch`, so the dropdown silently shows "Нет совпадений" forever. The feature is dead on arrival.

**Fix:** Map the UI value to the backend whitelist value. Simplest fix in `ModelFormModal.svelte`:

```svelte
suggestBrandFn={(prefix) => cartridges.suggestCompatPrinter('printer_brand', prefix)}
suggestModelFn={(prefix) => cartridges.suggestCompatPrinter('printer_model', prefix)}
```

and widen the type in `api.ts` to `field: 'printer_brand' | 'printer_model'`. Alternatively accept `'brand' | 'model'` at the API boundary and translate before the `apiCall`. Add a backend integration test that exercises the exact string the UI sends.

### CR-02: Cartridge history list keyed by `created_at_utc` — duplicate-second rows dropped

**File:** `ui/src/features/cartridges/CartridgeDetail.svelte:186`
**Issue:** The history list uses the audit timestamp as its `{#each}` key:

```svelte
{#each history as entry (entry.created_at_utc)}
```

`created_at_utc` is unix **seconds** (`self.clock.unix_seconds()`), and a single user action frequently writes two audit rows within the same wall-clock second — e.g. `create` immediately followed by a transition, or two quick lifecycle operations. When two `entry.created_at_utc` values are equal, Svelte 5 raises a duplicate-key error and renders only one of the colliding rows, silently hiding history. History integrity is a stated core value of this app ("потеря истории при возврате на склад"), so dropped audit rows are a correctness defect.

**Fix:** Key by a guaranteed-unique value. The repo already selects `audit_log.id` ordering (`ORDER BY created_at_utc DESC, id DESC`) but `AuditEntryDto` does not expose `id`. Either (a) add the audit row `id` to `AuditEntryDto` (`crates/trackly-app/src/dto/cartridge.rs:427`) and key on it, or (b) key by array index as a stopgap:

```svelte
{#each history as entry, i (i)}
```

Option (a) is preferred for stable keyed transitions.

## Warnings

### WR-01: FTS search builds an invalid/empty MATCH for punctuation-only or whitespace queries

**File:** `crates/trackly-infra/src/repos/cartridges_sqlite.rs:541`
**Issue:** `search` wraps the raw query as `format!("\"{}\"*", query.replace('"', "\"\""))`. The service trims and rejects empty strings, but a query that is non-empty yet tokenizes to nothing under `unicode61` (e.g. `"---"`, `"  "` that survives a partial trim, or a lone `"`) produces a phrase like `""""*` or `"---"*`. FTS5 MATCH on a phrase that yields zero tokens can return a `SQLITE_ERROR` ("fts5: syntax error" / "no such column") in some builds, which would surface to the user as a generic Internal error rather than an empty result set. The existing test only covers `' OR '1'='1` (which tokenizes fine), so this path is unverified.

**Fix:** After escaping, guard against an effectively-empty MATCH term, or fall back to LIKE-only when the FTS term has no alphanumeric content:

```rust
let has_token = query.chars().any(|c| c.is_alphanumeric());
// if !has_token, skip the fts_hits CTE and use only like_hits
```

Add a test with a punctuation-only query asserting `Ok`.

### WR-02: Model create/update conflict surfaces as `"UNIQUE constraint failed..."` — UI inline matcher misses it

**File:** `crates/trackly-app/src/services/cartridge_service.rs:528-580` (and `ModelFormModal.svelte:277`)
**Issue:** `model_create`/`model_update` rely on the partial unique index `idx_cartridge_models_brand_model_unique` firing and being mapped by `map_rusqlite` to `AppError::Conflict { reason: "UNIQUE constraint failed: cartridge_models.brand, cartridge_models.model" }`. The UI tries to show this inline:

```svelte
if (msg.toLowerCase().includes('уже') || msg.toLowerCase().includes('exist')) {
  conflictError = `Модель «...» уже создана`;
}
```

`"UNIQUE constraint failed: ..."` contains neither `"уже"` nor `"exist"`, so a duplicate brand+model falls through to the generic error toast instead of the intended inline conflict message. The cartridge create path does pre-check and returns a Russian `"уже существует"` reason (`assign_code_in_tx`), so the two flows are inconsistent.

**Fix:** Pre-check for an existing live `(brand, model)` in `model_create`/`model_update` and return `AppError::Conflict { reason: format!("Модель «{} {}» уже существует", brand, model) }`, mirroring `assign_code_in_tx`. This also avoids leaking raw SQLite text to the UI.

### WR-03: `OperationModal` does not reset `stateId` when `op` changes without an open→close cycle

**File:** `ui/src/features/cartridges/OperationModal.svelte:46-65`
**Issue:** `defaultStateId` is `$derived(op === 'from_refill' ? 1 : 3)`, but `stateId` is only assigned inside the `$effect` that fires on `open`. The reset effect reads `defaultStateId` but does not list `op` as an explicit dependency it reacts to — it only re-runs when `open` toggles. In the current `CartridgesPage` wiring `op` and `open` are set in the same batch, so it works today; but this is fragile: any future caller that reuses an already-open modal and only swaps `op` (e.g. switching from `return_to_stock` to `from_refill`) will keep the stale `stateId` and submit the wrong default charge state. Submitting `state_id = 3 (Пустой)` for a `from_refill` that should default to `1 (Полный)` is a silent data-correctness bug.

**Fix:** Make the dependency explicit, or reset on `op` change too:

```ts
$effect(() => {
  void op; // react to op changes as well
  if (open) {
    // ...
    stateId = defaultStateId;
  }
});
```

### WR-04: Dead/misleading code in `CartridgeService::delete` — cloned `cart_repo` is acquired then immediately dropped

**File:** `crates/trackly-app/src/services/cartridge_service.rs:259-301`
**Issue:** `delete` clones `self.cart_repo` into the closure, then the closure contains `drop(cart_repo); // not needed inside this tx` plus an inline-duplicated copy of the soft-delete SQL that already exists in `delete_soft` / `soft_delete_model_in_tx`. The comment block at lines 263-266 (`"...actually, the trait delete_soft takes &mut conn. We replicate inline here..."`) is stream-of-consciousness left in production code. The cloned `Arc` and `drop` are pure noise and signal an unfinished refactor; the SQL duplication risks the two copies drifting apart.

**Fix:** Remove the unused `cart_repo` clone and the `drop`. Either call the existing repo helper for the soft-delete inside the writer closure, or delete the misleading comment. Keep one source of truth for the soft-delete UPDATE.

### WR-05: Cartridge HTTP/Tauri transport accepts `i64` ids over the wire while Tauri wrappers truncate to `i32`

**File:** `crates/trackly-app/src/http/cartridges.rs:36-57` vs `crates/trackly-app/src/tauri_cmds/cartridges.rs:174-209`
**Issue:** The HTTP payloads (`GetPayload`, `UpdatePayload`, `DeletePayload`) deserialize `id`/`version` as `i64`, but the Tauri command wrappers declare them as `i32` and cast `id as i64`. The DTOs also annotate `#[specta(type = i32)]`. The two transports therefore disagree on the accepted integer range for the same logical operation: an id above `i32::MAX` is accepted by HTTP but unrepresentable via Tauri/bindings. With `AUTOINCREMENT` PKs this is not reachable in practice for a single-org install, but it is a real transport-parity inconsistency that contradicts the "schemas defined once, used by both transports" architecture note in CLAUDE.md.

**Fix:** Make the HTTP payload integer widths match the DTO contract (`i32` annotation), or standardize on `i64` end-to-end and drop the `as i64` casts in the Tauri wrappers. Pick one and apply consistently.

### WR-06: `low_stock` threshold silently becomes 0 when `app_settings.low_stock_threshold` is non-numeric

**File:** `crates/trackly-infra/src/repos/cartridges_sqlite.rs:593-600`
**Issue:** `CAST(value AS INTEGER)` in SQLite never errors — a non-numeric string (e.g. an accidental `'two'` or empty) casts to `0`, not a query error, so the `.unwrap_or(2)` fallback never triggers for malformed data. With threshold `0`, the `HAVING cnt < 0` clause matches nothing and the low-stock banner silently stops warning. The intended default of `2` is bypassed.

**Fix:** Validate the parsed value explicitly and clamp/fallback in Rust:

```rust
let threshold: i64 = conn
    .query_row("SELECT value FROM app_settings WHERE key='low_stock_threshold'", [], |r| r.get::<_, String>(0))
    .ok()
    .and_then(|s| s.trim().parse::<i64>().ok())
    .filter(|&t| t > 0)
    .unwrap_or(2);
```

## Info

### IN-01: `CartridgeContextMenu` `{#each menuItems as item}` has no key

**File:** `ui/src/features/cartridges/CartridgeContextMenu.svelte:139`
**Issue:** The menu items loop is unkeyed and mixes `sep` and `action` variants. For a static, derived list this is benign, but unkeyed each-blocks over heterogeneous objects are a known source of subtle re-render bugs if the list ever becomes dynamic.
**Fix:** Add a stable key, e.g. `(item.kind === 'sep' ? `sep-${i}` : item.label)`.

### IN-02: `assign_code_in_tx` retry loop has no upper bound

**File:** `crates/trackly-infra/src/repos/cartridges_sqlite.rs:120-134`
**Issue:** The auto-code loop increments the counter until it finds an unused `C-NNNNNN`. Because the counter only moves forward and codes are unique, this terminates in practice, but there is no defensive cap. A corrupted counter or a table pre-seeded with a dense range of custom codes could spin longer than expected inside the writer transaction (blocking all writes).
**Fix:** Add a sane retry cap (e.g. 10_000 iterations) returning `AppError::Internal` if exceeded, purely as a guardrail.

### IN-03: `_now_utc` parameter in `assign_code_in_tx` is unused

**File:** `crates/trackly-infra/src/repos/cartridges_sqlite.rs:101`
**Issue:** The function takes `_now_utc: i64` but never uses it. Dead parameter threaded through call sites.
**Fix:** Drop the parameter, or document why it is retained for signature symmetry.

### IN-04: `CartridgeFilters` kind/model `{#each}` and status `{#each STATUSES as s}` partially unkeyed

**File:** `ui/src/features/cartridges/CartridgeFilters.svelte:49`
**Issue:** `{#each STATUSES as s}` (status switch-bar) is unkeyed. The list is static so this is harmless, but inconsistent with the keyed `{#each models as m (m.id)}` a few lines below.
**Fix:** Add `(s.id ?? 'all')` for consistency.

### IN-05: `search` total reported as result-set length, not a true match count

**File:** `crates/trackly-app/src/services/cartridge_service.rs:441-442`
**Issue:** `search` returns `total = rows.len()`, but the repo caps results at `LIMIT 200`. When more than 200 cartridges match, the UI footer ("N из total") will display `200 из 200`, understating the real match count. `list` computes a real `COUNT(*)`; `search` does not.
**Fix:** Either document that search is capped at 200 (and label the footer accordingly), or run a parallel COUNT for the search predicate.

---

_Reviewed: 2026-06-12T11:57:21Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_

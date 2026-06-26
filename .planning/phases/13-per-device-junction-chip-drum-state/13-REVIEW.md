---
phase: 13-per-device-junction-chip-drum-state
reviewed: 2026-06-26T00:00:00Z
depth: standard
files_reviewed: 23
files_reviewed_list:
  - crates/trackly-app/src/dto/cartridge.rs
  - crates/trackly-app/src/dto/printer.rs
  - crates/trackly-app/src/http/cartridges.rs
  - crates/trackly-app/src/http/printers.rs
  - crates/trackly-app/src/services/cartridge_service.rs
  - crates/trackly-app/src/services/printer_service.rs
  - crates/trackly-app/src/specta_export.rs
  - crates/trackly-app/src/tauri_cmds/cartridges.rs
  - crates/trackly-app/src/tauri_cmds/printers.rs
  - crates/trackly-app/tests/cartridges_crud.rs
  - crates/trackly-app/tests/role_endpoint_matrix.rs
  - crates/trackly-core/src/domain/cartridges.rs
  - crates/trackly-core/src/ports/printers.rs
  - crates/trackly-infra/src/repos/cartridges_sqlite.rs
  - crates/trackly-infra/src/repos/printers_sqlite.rs
  - migrations/V032__cartridge_model_compatibility_printer_name.sql
  - ui/src/bindings.ts
  - ui/src/features/cartridges/CompatibilityEditor.svelte
  - ui/src/features/cartridges/ModelFormModal.svelte
  - ui/src/features/cartridges/OperationModal.svelte
  - ui/src/features/cartridges/api.ts
  - ui/src/features/printers/PrinterDetail.svelte
  - ui/src/features/printers/api.ts
findings:
  critical: 1
  warning: 6
  info: 5
  total: 12
status: issues_found
---

# Phase 13: Code Review Report

**Reviewed:** 2026-06-26
**Depth:** standard
**Files Reviewed:** 23
**Status:** issues_found

## Summary

Reviewed the Phase 13 compatibility redesign (V029 per-device junction → V032 free-text `printer_name` matching), the kind-aware drum auto-return default state (R7), the uncapped printer list (R8), and the new `printers_get_compatible_aggregates` read command (R4). The architecture is sound: SQL is consistently parameterized, RBAC gating on the new read command matches the existing `ReadData` class, and the case-insensitive/TRIM matching logic is correct and well-tested.

One BLOCKER: the V032 data migration silently drops compatibility rows whose source `printer_brand`/`printer_model` values are NULL, because `TRIM(a || ' ' || b)` yields NULL in SQLite when either operand is NULL — and the V005 columns are `NOT NULL` but pre-existing rows can still produce surprising results, plus the migration loses the `compatible_with_printer_device_id` filter's ability to distinguish "no rows" from "NULL-collapsed rows." See CR-01 for the precise failure mode.

The remaining findings are robustness gaps: the aggregate/filter queries match `devices.name` without scoping to printer-type or non-deleted devices, the new compatible-aggregates command does no existence check on `device_id`, and several UI effects can fire stale async writes. None of the WARNINGs block shipping on their own but each degrades correctness in a reachable edge case.

## Critical Issues

### CR-01: V032 migration drops compatibility data when concatenated printer name collapses to empty

**File:** `migrations/V032__cartridge_model_compatibility_printer_name.sql:48-50`

**Issue:** The data-preserving INSERT transforms `(printer_brand, printer_model)` into a single `printer_name` via:

```sql
SELECT id, cartridge_model_id, TRIM(printer_brand || ' ' || printer_model)
  FROM cartridge_model_compatibility;
```

Two reachable problems:

1. **Empty-string collapse → silent semantic data loss.** If a legacy row had `printer_brand = ''` and `printer_model = ''` (both columns are `NOT NULL` in V005 but empty strings are permitted), the result is `TRIM(' ')` = `''`. The new `printer_name` is an empty string, which then never matches any `devices.name` via `LOWER(TRIM(cmc.printer_name)) = LOWER(TRIM(d.name))` (unless a device is literally named empty). The compatibility link is preserved as a row but is functionally dead — worse than being dropped, because the model now silently has "compatibility configured" (so the D-05 pass-through in `list()` no longer applies — `NOT EXISTS` is false) yet matches no printer. A model that previously matched via one populated half (e.g. brand set, model empty) could change behavior post-migration.

2. **Single-space join artifact.** A row with brand `"Pantum"` and model `""` becomes `TRIM("Pantum" || " " || "")` = `"Pantum"` — acceptable. But brand `""` + model `"BM5100"` becomes `"BM5100"`. Whether these match the intended `devices.name` depends entirely on how the device was named; the migration assumes `"brand model"` is the canonical device name format, which is not enforced anywhere (devices.name is free text, often just a model or an asset tag). Rows that were meaningful under the two-column scheme may not match under the single concatenated form.

**Fix:** Migrate only rows that produce a usable name, and document the assumption. Filter out empties and normalize interior whitespace so the result is a stable comparison key:

```sql
INSERT INTO cartridge_model_compatibility_new (id, cartridge_model_id, printer_name)
SELECT id, cartridge_model_id, TRIM(printer_brand || ' ' || printer_model)
  FROM cartridge_model_compatibility
 WHERE TRIM(printer_brand || ' ' || printer_model) <> '';
```

If preserving the `id` values is not required downstream (the column is a surrogate AUTOINCREMENT key with no external references — `printer_cartridge_models` is being dropped, and nothing else FKs into `cartridge_model_compatibility`), drop the `id` from the projection and let AUTOINCREMENT reassign, avoiding any chance of a gap-induced surprise. Additionally, add a migration test asserting that a model whose only legacy row was empty/whitespace ends up with **zero** rows (so the D-05 pass-through still treats it as "compatible with any printer"), and that a populated row survives and still matches a like-named device. The existing `cartridges_crud.rs` tests only cover models created **after** V032 via `model_create`, so the data-transform path is currently unverified.

## Warnings

### WR-01: Aggregate + filter queries match `devices.name` without scoping to printer-type / non-deleted devices

**File:** `crates/trackly-infra/src/repos/cartridges_sqlite.rs:357` and `:1170`/`:1199`

**Issue:** Both `compatible_model_aggregates` (`JOIN devices d ON d.id = ?1`) and `list()`'s compatibility subquery (`JOIN devices d ON d.id = ?6`) resolve the device purely by id and compare names, with no `d.type_id = 2` (Принтер) or `d.deleted_at_utc IS NULL` guard. Consequences:

- A caller can pass the `device_id` of a **non-printer** device (or a **soft-deleted** printer). If that device's `name` happens to collide with a model's compatibility entry, the model is incorrectly reported as compatible. The HTTP/Tauri command (`printers_get_compatible_aggregates`) takes a raw `device_id` with no validation that it is actually a printer (see WR-02), so this is reachable from the API surface.
- The autocomplete source (`suggest_compat_printer`, service line 836) correctly filters `type_id = 2 AND deleted_at_utc IS NULL`, so there is an internal inconsistency: names are suggested only from live printers, but matching at query time accepts any device name.

**Fix:** Add the type/soft-delete scope to the device join in both queries, e.g. `JOIN devices d ON d.id = ?1 AND d.type_id = 2 AND d.deleted_at_utc IS NULL`. With an inner join this also makes the aggregate return empty for a bad `device_id` instead of cross-joining against an arbitrary row.

### WR-02: `build_printers_get_compatible_aggregates` performs no existence/type check on `device_id`

**File:** `crates/trackly-app/src/tauri_cmds/printers.rs:207-224`

**Issue:** The command authorizes `ReadData` then calls `ctx.cartridges.compatible_aggregates_for_printer(device_id)` directly. If `device_id` does not exist (or is not a printer), the underlying query (`JOIN devices d ON d.id = ?1`) simply returns zero rows, and the handler returns `PrinterCompatibleAggregatesDto { device_id, models: [] }` with HTTP 200. The caller cannot distinguish "this printer has no compatible models" from "this device id is bogus / not a printer." Every other printer read path (`build_printers_get`, `get_by_device_id`) returns `NotFound` for a missing id; this command silently diverges, which can mask UI bugs that pass the wrong id (e.g. a printers.id instead of a device_id).

**Fix:** Resolve the printer first (reuse `ctx.printers.get_by_device_id(device_id).await?` to assert it exists and is a printer) before computing aggregates, or have the service return `NotFound` when the device row is absent. At minimum, scope the join per WR-01 so a non-printer id yields an empty set deterministically rather than a name-collision false positive.

### WR-03: `compatible_model_aggregates` ignores `installable_only`/state semantics — counts written-off-equivalent stock

**File:** `crates/trackly-infra/src/repos/cartridges_sqlite.rs:351-368`

**Issue:** The aggregate sums `status_id IN (1,3,2)` for in_stock/at_refill/in_use, joining `cartridges c ON c.model_id = m.id AND c.deleted_at_utc IS NULL`. It does **not** account for `state_id`. For drums (kind_id=2), a status=1 (На складе) cartridge with state=6 (Отработанный) is not actually installable (the transition layer rejects installing an Отработанный drum — domain `transition_in_tx` line 434). The printer card's "На складе N" count therefore overstates usable stock for drums by including spent units. This is a display-correctness gap on the very widget Phase 13 R4 introduces.

**Fix:** Either document that `in_stock` is a raw status count (not an "installable" count) in `CompatibleModelAggregate`'s doc comment, or mirror the kind-aware `installable` predicate used in `list()` (`(m.kind_id=1 AND state_id IN (1,2)) OR (m.kind_id=2 AND state_id IN (4,5))`) so the card shows installable stock. Given the widget's purpose (помочь оператору выбрать картридж), the installable count is the more useful and less misleading number.

### WR-04: `PrinterDto::community_configured` hardcoded `true` regardless of actual community state

**File:** `crates/trackly-app/src/dto/printer.rs:62-65`

**Issue:** `From<PrinterRow>` sets `community_configured: true` with the comment "the service layer sets it to true when community != default" — but no service code in `printer_service.rs` overrides it (the `get`/`list`/`get_by_device_id` paths never touch `community_configured`). The field is therefore always `true`, including for printers created with the default `"public"` community. The DTO's stated contract (a safe boolean indicating a non-default community was set, T-06-07-I) is silently violated. While not introduced by Phase 13, the printer DTO is in scope for this review and the field is dead/misleading.

**Fix:** Either compute it in the repository (`community != 'public'` selected into `PrinterRow`) and propagate, or remove the field if no consumer relies on it. As-is it gives the UI a constant `true` that conveys no information.

### WR-05: Stale-write race in `OperationModal` async compatibility effects

**File:** `ui/src/features/cartridges/OperationModal.svelte:338-374` (and the printer-context effect at `:232-284`)

**Issue:** The `$effect` that loads `printerOptions`/`compatibleDeviceIds` runs `Promise.all([...]).then(...)` and assigns to `$state` on resolution, with no guard against the inputs (`cartridge`, `open`, `preFillPrinterId`) having changed before the promise settles. If the operator switches the selected cartridge (or closes/reopens the modal for a different cartridge) while the first `printers.list` + `cartridges.modelsGet` round-trip is in flight, the late-resolving `.then` overwrites `compatibleDeviceIds` with results computed for the *previous* cartridge's `model_id`. The same pattern exists in the printer-context effect (`getByDeviceId(...).then(...)`), which can write a stale `previousCartridge`/`printerContext` for a printer the operator already deselected. Result: the PrinterSelect highlights the wrong "compatible" printers, or the previous-cartridge block shows a cartridge from a different printer.

**Fix:** Capture a cancellation token at effect entry and ignore the resolution if it changed, e.g.:
```ts
$effect(() => {
  let cancelled = false;
  // ...guard...
  Promise.all([...]).then(([printersRes, modelRes]) => {
    if (cancelled) return;
    // ...assign...
  });
  return () => { cancelled = true; };
});
```
Apply the same guard to the `getByDeviceId`/`getCompatibleAggregates` effects.

### WR-06: `compatibility` prop mutated as a fresh array reference but `CompatibilityEditor` only reads it once

**File:** `ui/src/features/cartridges/CompatibilityEditor.svelte:17` + `ui/src/features/cartridges/ModelFormModal.svelte:444-448`

**Issue:** `CompatibilityEditor` initializes `let rows = $state<string[]>([...compatibility])` from the prop **once** at construction. `ModelFormModal` resets `compatibility = target?.compatibility ?? []` inside its open-effect (line 67) to clear the form on reopen, but because `CompatibilityEditor` is mounted inside `{#key openInstanceCounter}`, it is remounted on reopen, so the reset usually propagates. However, if `compatibility` is reset for any reason *without* bumping `openInstanceCounter` (e.g. a future edit to the reset logic, or editing a different `target` while open), the editor's internal `rows` will not re-sync from the prop — it is effectively a one-way snapshot. This is a latent state-desync foot-gun: the component name implies two-way binding via `onChange`, but inbound prop changes are ignored after mount.

**Fix:** Make the inbound sync explicit so the contract is robust to remount changes: either document that the prop is mount-time-only and the parent MUST remount on reset (it currently does, via `{#key}`), or add a `$effect` that re-seeds `rows` when the `compatibility` prop identity changes. The cheapest correct fix is to keep the `{#key}` remount and add a comment in `CompatibilityEditor` stating the prop is read once by design.

## Info

### IN-01: `compatible_model_aggregates` cross-joins `devices` even though only `d.name` is used

**File:** `crates/trackly-infra/src/repos/cartridges_sqlite.rs:356-365`

**Issue:** The query `JOIN devices d ON d.id = ?1` then references `d.name` only inside the correlated `EXISTS`. Pulling `d` into the outer FROM with a `GROUP BY m.id` works but is structurally awkward and contributed to WR-01 (the missing type/soft-delete scope). Folding the device-name lookup into the `EXISTS` subquery (`SELECT 1 FROM cartridge_model_compatibility cmc JOIN devices d ON d.id = ?1 WHERE ...`) mirrors the `list()` query's shape and removes the outer join entirely.

**Fix:** Move the `devices` reference into the `EXISTS` subquery for parity with `list()`.

### IN-02: `current_cartridge_for_printer` picks arbitrarily among multiple linked cartridges

**File:** `crates/trackly-infra/src/repos/printers_sqlite.rs:431-441`

**Issue:** `SELECT id ... WHERE current_printer_device_id = ?1 ... ORDER BY updated_at_utc DESC LIMIT 1`. If an invariant break ever leaves two cartridges pointing at the same printer (the auto-return logic is designed to prevent this, but nothing enforces it at the schema level — no unique index on `current_printer_device_id`), this silently returns the most-recently-updated one. A partial unique index (`WHERE current_printer_device_id IS NOT NULL AND deleted_at_utc IS NULL AND status_id = 2`) would make the invariant structural. Not a Phase 13 regression, but adjacent to the install/auto-return path under review.

**Fix:** Consider a partial unique index to enforce "at most one active cartridge per printer."

### IN-03: Comment in domain enum is stale — `ReturnToStock` default documented as "3=Пустой" but R7 makes it kind-aware

**File:** `crates/trackly-core/src/domain/cartridges.rs:124-129`

**Issue:** The `ReturnToStock` doc comment says "state_id set to payload value (default: 3=Пустой)" and `Install`'s `previous_cartridge_state_id` says "None = дефолт 3 (Пустой)" (line 117-118). After R7, the auto-return default is kind-aware (5 for drums, 3 for cartridges) — implemented at `cartridges_sqlite.rs:555-561`. The domain comment still states the old flat default, which is now misleading for the drum path.

**Fix:** Update the comment to note the kind-aware default applied at the repository layer when `previous_cartridge_state_id` is `None`.

### IN-04: `printers/api.ts` references endpoints that are not in `specta_export.rs`

**File:** `ui/src/features/printers/api.ts:32,44`

**Issue:** `printers.delete` calls `printers_delete` and `printers.getReadings` calls `printers_get_readings`, but neither command is registered in `specta_export.rs` (the registered printer commands are list/get/get_by_device_id/get_compatible_aggregates/create/discover/admit/refresh/acknowledge_alert). `PrinterDetail.svelte` actively calls `printers.getReadings(p.id)` (line 40). Either these commands exist but are registered elsewhere/under a different builder, or these are dead/broken calls. This is a wiring inconsistency worth confirming — a missing command surfaces as a runtime invoke error, not a compile error.

**Fix:** Confirm `printers_get_readings` is registered for the Tauri transport; if it is registered in a separate builder, leave as-is; if not, register it or remove the dead UI call.

### IN-05: Data-transform path of V032 has no test coverage

**File:** `crates/trackly-app/tests/cartridges_crud.rs:268-498`

**Issue:** All compatibility tests seed via `model_create(... compatibility: vec![...])`, which writes directly to the post-V032 single-column schema. None exercise the V005→V032 `printer_brand || ' ' || printer_model` transform on pre-existing rows. The migration's central data-preservation claim (D-02, file header lines 14-17) is therefore unverified end-to-end. See CR-01 for the concrete fix; this entry flags the coverage gap independently.

**Fix:** Add a migration-level test that inserts rows into the V005-shaped table at the appropriate version and asserts the V032 result (covered by CR-01's fix recommendation).

---

_Reviewed: 2026-06-26_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_

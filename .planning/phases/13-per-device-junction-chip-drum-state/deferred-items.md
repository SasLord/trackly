# Deferred Items — Phase 13

Out-of-scope discoveries logged during plan execution. Not fixed by the
executing plan; tracked here for a future plan to pick up.

## From Plan 13-02 (backend V029 junction removal)

**Frontend TypeScript/Svelte still references the removed compatibility
commands/DTOs.** Plan 13-02's `files_modified` only covers Rust crates
(`trackly-core`, `trackly-infra`, `trackly-app`); it removed
`printers_get_compatible_models` / `printers_set_compatible_models` /
`cartridge_models_get_compatible_devices` / `cartridge_models_set_compatible_devices`
Tauri commands and HTTP routes, and the `PrinterCompatibleModelsDto` /
`CartridgeModelCompatibleDevicesDto` types, because they could not compile
against the new `Vec<String>` compatibility contract. The following frontend
files still call the removed bindings and will fail at the TypeScript level
(`ui/bindings.ts` regen) and at runtime once rebuilt against the new backend:

- `ui/src/features/cartridges/api.ts`
- `ui/src/features/cartridges/CompatibleDevicesEditor.svelte`
- `ui/src/features/printers/api.ts`
- `ui/src/features/printers/CompatibleModelsEditor.svelte`
- `ui/src/lib/components/PrinterSelect.svelte`
- `ui/src/bindings.ts` (generated — will regenerate cleanly once a later plan
  reruns `specta_export`, removing the stale bindings automatically)

This is expected — the plan-checker-revised 13-PLAN.md sequences the UI
contract update (R7 frontend) into a later plan in this phase (compatibility
editor rebuilt around `Vec<String>` printer names instead of per-device
junction rows). Do not attempt to patch these files under 13-02; they are in
scope for the plan that rebuilds the compatibility editor UI.

**Update (Plan 13-05):** `suggest_compat_printer`'s backend signature changed
again — the `field: String` parameter (`"printer_brand"`/`"printer_model"`)
was dropped entirely (it no longer makes sense against the single
`printer_name` column), and the data source switched from
`cartridge_model_compatibility` history to the real printer roster
(`devices.name WHERE type_id = 2`, D-06). `ui/src/features/cartridges/api.ts`'s
`suggestCompatPrinter(field, prefix)` and its two call sites in
`ModelFormModal.svelte` (`'printer_brand'`/`'printer_model'`) still send the
old `field` argument — harmless at the wire level (axum's `SuggestCompatPayload`
only deserializes `prefix`, ignoring unknown JSON fields; Tauri's command
signature drops unknown args similarly), but the call site is stale and
should be rewritten to `suggestCompatPrinter(prefix)` (single positional
arg, no field) when the compatibility editor UI is rebuilt.

**Pre-existing `clippy::len_zero` warnings in `template_service.rs` (unrelated
subsystem).** `cargo clippy -p trackly-app --tests -- -D warnings` fails on
two `assert!(bytes.len() > 0, ...)` calls in
`crates/trackly-app/src/services/template_service.rs` (lines ~379, ~430) —
PDF-generation test code, untouched by Plan 13-02 and outside its
`files_modified` list. Not fixed here per the scope-boundary rule. The
plan's own verification gate (`cargo build -p trackly-app` /
`cargo clippy -p trackly-app -- -D warnings`, lib-only, no `--tests`) does
not exercise this path and passes cleanly. Flag for whichever phase next
touches `template_service.rs`, or a dedicated `/gsd-debug` cleanup pass.

## From Plan 13-06 (ModelFormModal compatibility collapse — R3)

**`ui/src/features/printers/CompatibleModelsEditor.svelte` and
`ui/src/features/cartridges/OperationModal.svelte` still call removed
printer/cartridge compat-junction commands; both are OUTSIDE 13-06's
`files_modified` list.** Plan 13-06 collapsed `ModelFormModal.svelte` to a
single compatibility block and cleaned `cartridges/api.ts` +
`printers/api.ts` of the dead `getCompatibleModels`/`setCompatibleModels`/
`modelsGetCompatibleDevices`/`modelsSetCompatibleDevices` wrappers (Task 1),
per its own scope. Two files elsewhere in the tree still reference those now-
fully-removed wrappers and fail `svelte-check`/`tsc` (6 errors as of this
plan's completion):

- `ui/src/features/printers/CompatibleModelsEditor.svelte` — V029 per-device
  checklist on the printer card (`printers.getCompatibleModels`/
  `setCompatibleModels`). Per `13-UI-SPEC.md`'s Component Inventory, this file
  is marked **Delete** and is explicitly in scope for **Plan 13-07** (printer
  card aggregates block replaces it).
- `ui/src/features/cartridges/OperationModal.svelte` (~line 301:
  `printers.getCompatibleModels(preFillPrinterId)`; ~line 328:
  `cartridges.modelsGetCompatibleDevices(cartridge.model_id)`) — narrows the
  cartridge-model picker by printer compatibility (D-12/D-13/D-14 narrowing,
  Phase 12). `13-UI-SPEC.md` lists `OperationModal.svelte` as **Edit** scope
  (chip-task front fix, D-11/D-13) but does not explicitly call out this
  narrowing call site — whichever plan touches `OperationModal.svelte` next
  (13-08 per the UI-SPEC inventory, or a gap-closure pass) must re-point these
  two call sites at `printers.getCompatibleAggregates` /
  `cartridges.modelsList()` + a printer-name compatibility filter, matching
  the new `Vec<String>` contract (D-04/D-05).

**Verification status:** confirmed via `git stash` + checkout of the
pre-Task-1 `api.ts` files that both errors pre-date Plan 13-06 — they were
already broken (TS could not resolve `PrinterCompatibleModelsDto` /
`CartridgeModelCompatibleDevicesDto`, which Plan 13-02/13-03 had already
removed from `bindings.ts`) before this plan started; Task 1 only changed
*how* they fail (missing-type errors → missing-property errors). Not caused
by 13-06; not fixed by 13-06 per the scope-boundary rule (#files_modified).
`pnpm --dir ui build` (Vite/Rollup) still succeeds despite these `svelte-
check`/`tsc` errors because Vite's esbuild-based transform does not
type-check — only `tsc --noEmit` and `svelte-check` surface them. Runtime
risk is real: if a user opens the printer card's old compat checklist or
triggers an install/replace from `OperationModal.svelte`'s narrowing path,
the underlying Tauri command no longer exists server-side and the call will
reject with a transport-level error.

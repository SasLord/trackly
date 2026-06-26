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

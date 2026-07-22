# Deferred Items — Phase 28 (support-admin-windows)

Out-of-scope discoveries logged during plan execution (not fixed, per executor
scope-boundary rule — pre-existing issues in unrelated files are out of scope
for the plan that happened to run into them).

## 28-11 — pre-existing backend compile error blocks `ui/src/bindings.ts` generation

**Found during:** Plan 28-11 (RequestDetail.svelte / RequestFormModal.svelte
Select -> Dropdown migration), while attempting to regenerate
`ui/src/bindings.ts` via `cargo test -p trackly-app --test export_bindings` to
run a clean `pnpm --dir ui svelte-check` / `pnpm --dir ui build`.

**Issue:** `crates/trackly-app/src/http/mod.rs:185,190` — `SpaAssets::get(...)`
fails to compile: `error[E0599]: no function or associated item named 'get'
found for struct 'SpaAssets' in the current scope`. `SpaAssets` derives
`rust_embed::Embed` (or similar) but the trait providing `::get` is not in
scope in that module — `use rust_embed::RustEmbed;` (or the current crate's
equivalent trait) appears to be missing or the import was renamed.

**Impact:** `cargo test -p trackly-app --test export_bindings` cannot compile
`trackly-app` (lib), so `ui/src/bindings.ts` (gitignored, generated artifact)
cannot be regenerated in this environment. `pnpm --dir ui svelte-check`
therefore reports pre-existing "Cannot find module '../../bindings'" errors
across ~30 files project-wide — none of which touch
`ui/src/features/requests/RequestDetail.svelte` or
`ui/src/features/requests/RequestFormModal.svelte` (verified: zero
errors/warnings for these two files specifically after the GAP-1 changes).

**Scope:** Out of scope for plan 28-11 — `SpaAssets`/`http/mod.rs` is
unrelated to the Заявки window's Select -> Dropdown component swap. Not
auto-fixed (Rule 1 scope-boundary: pre-existing failures in unrelated files
are out of scope for the current task).

**Recommendation:** Fix `http/mod.rs`'s missing trait import for `SpaAssets`
in a follow-up (likely a one-line `use` fix), then re-run
`cargo test -p trackly-app --test export_bindings` to regenerate
`ui/src/bindings.ts` and get a clean whole-project `svelte-check`/`build`.

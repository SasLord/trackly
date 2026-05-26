# Deferred items (Phase 01)

## From Plan 01-05

- **`ui/src/bindings.ts` references `@tauri-apps/api/{core,event,webviewWindow}`**. These TS packages are not in `ui/package.json` `devDependencies` yet. Generation works; `svelte-check` against the bindings fails with 5 "Cannot find module" errors. Fix in **Phase 2 plan that adds the Tauri runtime** — add `@tauri-apps/api` to `ui/package.json` dependencies. Out of scope for Plan 01-05 (which only generates the bindings file).

  **Status:** ✅ Resolved in Phase 2 (Plan 02-02, Task 4 — 2026-05-26).
  `@tauri-apps/api ^2.11.0`, `@tauri-apps/plugin-dialog ^2.7.1`, `svelte-spa-router ^5.1.0` добавлены в `ui/package.json` dependencies; `continue-on-error: true` снят со step `pnpm svelte-check` в `ci-fast.yml` + `ci-full.yml`. `pnpm svelte-check` теперь blocking gate.

- **`tests/export_bindings.rs` skipped on Windows.** On `windows-latest` CI runner the test binary fails to load with `STATUS_ENTRYPOINT_NOT_FOUND` (0xc0000139) — a Windows DLL-loader error caused by `specta-typescript = "0.0.9"`. Upgrade path is blocked on stable Rust: `tauri-specta` newer than `=2.0.0-rc.21` (which pins `specta-typescript = ^0.0.9` exactly) requires `specta = rc.24+`, which in turn uses nightly-only `debug_closure_helpers` (issue #117729) and `const_type_id`. Test gated via `#![cfg(not(target_os = "windows"))]`. Coverage retained on Linux + macOS (2 of 3 CI platforms) — threat T-05-02 (export-drift) still gated. **Revisit triggers:** `specta` reaches a stable-Rust-compatible release ≥ rc.24, OR Phase 8 needs Windows-CI bindings.ts regeneration for the release pipeline. The runtime Windows app itself (`trackly.exe`) is unaffected — `bindings.ts` is a build-time artefact for the frontend.

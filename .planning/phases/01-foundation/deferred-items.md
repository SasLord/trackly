# Deferred items (Phase 01)

## From Plan 01-05

- **`ui/src/bindings.ts` references `@tauri-apps/api/{core,event,webviewWindow}`**. These TS packages are not in `ui/package.json` `devDependencies` yet. Generation works; `svelte-check` against the bindings fails with 5 "Cannot find module" errors. Fix in **Phase 2 plan that adds the Tauri runtime** — add `@tauri-apps/api` to `ui/package.json` dependencies. Out of scope for Plan 01-05 (which only generates the bindings file).

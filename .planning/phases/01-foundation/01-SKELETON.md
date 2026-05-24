# Walking Skeleton — Trackly

**Phase:** 1 — Фундамент
**Generated:** 2026-05-24

## Capability Proven End-to-End

`trackly --self-test` (a special CLI mode) runs the **complete infrastructural happy path** without opening a UI: resolve portable paths from `current_exe()`, set `WEBVIEW2_USER_DATA_FOLDER` before any Tauri call, parse `trackly.config.toml`, initialize tracing to `./logs/trackly.log`, open the WRITER `rusqlite::Connection` with WAL + busy_timeout PRAGMAs, verify `PRAGMA user_version` against the embedded refinery max (graceful exit if newer), run all 12 refinery migrations on the write connection, hand the writer connection to the `spawn_blocking` worker behind a bounded `mpsc::channel::<WriteJob>(256)`, open the read pool of 4 read-only connections, build `AppCtx`, submit one `WriteJob` through the writer and read it back via the read pool, regenerate `ui/src/bindings.ts` via `tauri-specta`, then drop `AppCtx` cleanly and exit(0) with no writes outside `<exe_dir>` (proven by `procmon-check` on Windows CI).

This single command exercises every cross-cutting invariant that **all** later phases depend on: portable-mode discipline, schema/migration discipline, single-writer pattern, shared `AppCtx` and `AppError`, structured tracing, and the Tauri↔HTTP DTO round-trip pipeline. **No business UI is shipped in Phase 1** — Phase 2 builds the first user-facing vertical slice (Devices CRUD) on top of this skeleton without re-deciding any architectural choice below.

## Architectural Decisions

| Decision | Choice | Rationale |
|---|---|---|
| Desktop shell | Tauri 2.11 (v2 capability/ACL model) | User-locked in CLAUDE.md; v1 EOL Oct 2024 |
| Frontend framework | Vanilla Svelte 5 (NOT SvelteKit) SPA, Vite 6 root in `ui/` | Same SPA bundle serves Tauri webview AND LAN browser in Phase 5; SvelteKit's router/data-loading subtract value |
| Styling | SCSS via `vitePreprocess({ scss: { ... } })` | User-locked |
| Async runtime | `tokio` 1.x multi-thread | Required by axum, ldap3, snmp2; Tauri provides one in normal mode, `--self-test` builds its own |
| HTTP server (Phase 5+ surface) | `axum` 0.8 on tower 0.5 / tower-http 0.6 | Tower middleware ecosystem; Phase 1 designs `AppCtx` to receive it without rework |
| Database driver | `rusqlite` 0.39 with `bundled` feature | User-locked + Evan Schwartz PSA against sqlx-sqlite write-tx lock starvation; `bundled` = no SQLite DLL in portable build |
| Migrations | `refinery` 0.8 with `rusqlite` driver via `embed_migrations!("../../migrations")` from `trackly-infra` | Forward-only, per-migration transactions, checksum-tracked in `refinery_schema_history`, embedded in binary so portable build needs no `migrations/` sidecar |
| Single-writer pattern | `tokio::sync::mpsc::channel::<WriteJob>(256)` → `spawn_blocking` worker owning one `rusqlite::Connection`; `send_timeout(5s)` → `AppError::WriteQueueBusy` | D-WriterChannel-01; structural prevention of `SQLITE_BUSY` under 20-LAN-user concurrency |
| Read pool | 4 read-only connections (`SQLITE_OPEN_READ_ONLY \| SQLITE_OPEN_NO_MUTEX`); behind `Arc<Mutex<Vec<Connection>>>` or `deadpool`-style pool | WAL allows N readers + 1 writer; 4 is conservative for LAN-scale |
| Error model | Single `AppError` flat enum in `trackly-core::error`; `impl Serialize` (Tauri) + `impl IntoResponse` (axum in Phase 5); identical JSON shape `{code, message, details}` | D-AppError-01; one frontend error parser across both transports |
| DTO/TypeScript pipeline | `tauri-specta` 2.0.0-rc.21 with `#[derive(specta::Type)]` on DTOs and `#[specta::specta]` on commands; bindings generated in `cargo test --test export_bindings`; `ui/src/bindings.ts` is gitignored; `pnpm prebuild` re-runs the test | D-Workspace-02; bindings always in sync with Rust DTOs at build time |
| Identifiers (DB) | `id INTEGER PRIMARY KEY AUTOINCREMENT` everywhere; human-visible numbers (`act.number`, `cartridge.code`) live in separate columns | D-Schema-01; UUID v7 is premature for single-process LAN app |
| Timestamps (DB) | `INTEGER NOT NULL` unix seconds, columns suffixed `_at_utc`; UTC only in DB; TZ formatting on UI via `chrono-tz` | D-Schema-02; `chrono::Local::now` banned via clippy |
| Soft delete + optimistic lock | All user-mutable tables get `deleted_at_utc INTEGER NULL` + `version INTEGER NOT NULL DEFAULT 1`; system tables (`audit_log`, `counters`, `sessions`, `scheduled_tasks`, lookups) are hard-delete | D-Schema-03, D-Schema-04 |
| Audit log | Full before/after JSON in `audit_log(before_json TEXT, after_json TEXT, payload_json TEXT)`; diff computed on read for history UI | D-Schema-05; no retention in v1 |
| Path resolution | All paths through `trackly_infra::paths::Paths` rooted at `std::env::current_exe()?.parent()?`; portable detected via sentinel (`portable.txt` OR `trackly.config.toml`); `dirs::*_dir()` + `tauri::Manager::path` banned via clippy | D-Config-01 + D-CI-02 |
| Config file | `trackly.config.toml` (TOML, NOT JSON) next to .exe; sections `[server]`, `[paths]`, `[logging]`, `[organization]`; parsed with `toml` 0.8 | D-Config-01 |
| WebView2 isolation | `unsafe { std::env::set_var("WEBVIEW2_USER_DATA_FOLDER", paths.webview_data_dir()) }` is the **first** non-trivial statement in `main()`, before any thread spawn or Tauri call | FOUND-05 + Pitfall #1; mandatory for portable mode |
| Logging | `tracing` 0.1 + `tracing-subscriber` 0.3 (EnvFilter + fmt) + `tracing-appender` 0.2 (`rolling::daily("./logs", "trackly.log")` + non-blocking); `WorkerGuard` stored on `AppCtx`, dropped on shutdown | D-Logging-01; format toggle via `[logging.format]` config |
| Crate layout | 3 crates: `trackly-core` (NO tokio/rusqlite/tauri), `trackly-infra` (adapters + paths + migrations + writer), `trackly-app` (bin `trackly`, tauri + axum composition root) | D-Workspace-01; hexagonal — core has no I/O |
| UI directory | `ui/` at repo root (NOT `frontend/`, NOT inside `trackly-app/src-ui/`); Vite root = `ui/`; Tauri `frontendDist = "../ui/dist"` | D-Workspace-01 |
| Test fixtures | Tempfile per test (NOT `:memory:`) — `:memory:` does not exercise WAL semantics; helpers in `trackly_infra::test_support` | D-Test-01 |
| Rust MSRV | 1.85 pinned via `rust-toolchain.toml` | Keeps `ldap3` NTLM door open for Phase 8; matches CLAUDE.md |
| CI strategy | `ci-fast.yml` (every push, ubuntu-only: fmt+clippy+test+svelte-check+lint) and `ci-full.yml` (PR + main: matrix ubuntu/macos/windows + Windows ProcMon test + release build); separate `cargo-deny.yml` on daily schedule | D-CI-01 |
| Portable verification | `tools/procmon-check/` Windows-only utility runs Sysinternals ProcMon while invoking `trackly.exe --self-test`, parses CSV, asserts every WriteFile path stays inside the sandbox `%TEMP%\Документы\Trackly_<uuid>\` (cyrillic intentional) | D-CI-03; covers FOUND-11, BLD-06, success criterion #1 (cyrillic + no APPDATA) in one fixture |
| Downgrade protection | On startup, after opening writer connection: read `PRAGMA user_version`; if > embedded max → `AppError::DatabaseFromNewerVersion` + graceful shutdown; file must remain byte-identical (test asserts via SHA256 before/after) | D-Migrations-02; success criterion #4 |

## Stack Touched in Phase 1

- [x] **Project scaffold** — Cargo workspace + 3 crates + `ui/` Vite scaffold + clippy.toml + rustfmt.toml + rust-toolchain.toml + .gitignore + deny.toml + GitHub Actions ci-fast.yml
- [x] **Routing** — `health` Tauri command + (designed-for-Phase-5) axum `GET /api/v1/health` shape; round-trip tested via `health_smoke.rs` calling the service layer directly (axum mount itself lives in Phase 5)
- [x] **Database** — All 14 v1 domain tables + 4 cross-cutting tables created via V001..V012 refinery migrations; concurrent write test (`concurrent_writes.rs`) exercises 25+25 parallel writes via writer-channel + read-back through reader pool
- [x] **UI** — `ui/` Vite project with Svelte 5 + svelte-check + eslint configured and passing (no user-facing screens; Phase 2 fills in)
- [x] **Deployment** — `trackly --self-test` runs the full lifecycle in `<exe_dir>` with zero APPDATA writes; verified by `procmon-check` on `windows-latest` CI runner with cyrillic sandbox path
- [x] **Cross-cutting invariants** — `Secret<T>` (zeroize Drop + `Debug = "***"`), `Clock` trait + `SystemClock` impl, `AppError` enum + Serialize for Tauri, `AppCtx` cloneable handle for both transports, `tauri-specta` bindings round-trip test, downgrade-protection test, ProcMon portability test

## Out of Scope (Deferred to Later Slices)

- **User-facing UI screens** — Phase 2 builds the first vertical slice (Devices CRUD with autocomplete + search + CSV)
- **Authentication / authorization** — Phase 5 (argon2id, roles, `authorize()` single source of truth)
- **HTTPS server / `axum` HTTP mounted** — Phase 5 (`tower-sessions` 0.13, rustls 0.23, rcgen 0.13 self-signed). Phase 1 ships the schema for `sessions` (V010) but NO `tower-sessions::SessionStore` impl
- **PDF generation** — Phase 3 (`krilla` 0.7 with embedded Cyrillic font; Typst-as-lib spike alternative)
- **SNMP printer monitoring** — Phase 6 (`snmp2` 0.4)
- **LDAP/AD bind** — Phase 8 (`ldap3` 0.12 `simple_bind` over `ldaps://636`)
- **Pantum auto-restart** — v2 milestone (Phase 6 only ships alert-only detection)
- **`scheduled_tasks` worker** — Phase 7 (Phase 1 lays down the table in V011 but runs no supervisor)
- **Logo BLOB in DB / templates editing** — Phase 7
- **Backup retention policy** — Phase 7 (Phase 1 establishes the `std::fs::copy` clippy ban so Phase 7 must use `rusqlite::backup::Backup`)
- **`activeCodePage=UTF-8` NSIS manifest** — Phase 8 (release pipeline). Phase 1's cyrillic ProcMon test catches regressions earlier
- **Custom device fields (`device_custom_fields`)** — explicitly NOT in v1; defer to v2 only if users complain
- **`tauri-plugin-updater`** — incompatible with portable mode; if a non-portable installer variant ever ships, enable updater only for that build

## Subsequent Slice Plan

Each later phase adds one vertical slice on top of this skeleton **without altering any architectural decision above**:

- **Phase 2:** Devices CRUD vertical slice — DTOs + `DeviceRepository` in `trackly-infra`, services in `trackly-core`, `#[tauri::command]` adapters, Svelte 5 list/form components, CSV import/export with encoding detection. Sidebar navigation + theme toggle.
- **Phase 3:** Acts vertical slice with PDF — Acts schema usage, sub-numbering counters via single-writer atomic increment, `krilla` PDF rendering with embedded Cyrillic font, document template versioning in DB.
- **Phase 4:** Cartridges vertical slice — `cartridge_models` + `cartridges` with auto-code `C-000001`, lifecycle state machine, low-stock banner.
- **Phase 5:** Authentication + HTTPS server — `argon2id` for local users, `rcgen` self-signed cert + `rustls` listener, `axum` 0.8 mounted on the same `AppCtx`, `tower-sessions` 0.13 with custom `RusqliteSessionStore` against the V010 schema, `authorize()` enforced in service layer for both transports.
- **Phase 6:** SNMP printer monitoring + Requests portal — `snmp2` 0.4 for vendor OIDs, in-app alert on Pantum hang, browser portal for employee requests.
- **Phase 7:** Reports + Dashboard + Settings — month-grouped report queries, dashboard widgets, organization/logo/backup settings; activates `scheduled_tasks` supervisor.
- **Phase 8:** AD login + release pipeline — `ldap3` simple_bind, registration auto-approve setting, GitHub Actions Release matrix with signed artifacts and `portable.txt`.

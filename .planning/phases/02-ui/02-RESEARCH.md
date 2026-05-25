# Phase 2: Устройства и базовый UI — Research

**Researched:** 2026-05-25
**Domain:** Rust hexagonal services on top of single-writer SQLite + Svelte 5 SPA with dual transport (Tauri invoke + future axum) + Russian-locale CSV + FTS5
**Confidence:** HIGH overall (28 D-* decisions already locked in CONTEXT.md; Phase 1 ships a working composition root + DTO pattern; external crates and JS packages confirmed against current npm/docs.rs; remaining LOW-confidence areas explicitly flagged)

## Summary

Phase 2 is a thin vertical slice over already-shipped Phase 1 infrastructure. The work is **mostly composition** — extending `AppCtx` with one new service (`DeviceService`), adding one new migration (`V013__devices_fts_triggers.sql`), writing ~12 Tauri commands following the `build_health` pattern, and growing the Svelte SPA from a "Phase 1 — Фундамент" placeholder into a navigable shell with one feature implemented (Devices). Every architectural pattern (hexagonal layout, single-writer enforcement, DTO sharing across transports, snake_case JSON, AppError 9-variant shape, RAII reader handles) is already established by Plans 01-04 and 01-05; Phase 2 must not invent new patterns, only use them.

Three areas need careful attention: **(1) the existing `migrations/V003__devices.sql` column names DIVERGE from the field names assumed in CONTEXT.md** (V003 has `inventory_number`/`serial_number`/`condition`/`complectation`/`notes`; CONTEXT.md drafts DTOs with `inventory_no`/`serial_no`/`state`/`kit`/`specs`) — the planner must either map between them or add V013 columns; **(2) the CSV preview-then-commit token store is the only piece of net-new in-memory infrastructure** and needs an explicit design (separate Arc-wrapped service field, not stuffed into DeviceService); **(3) the UI tier is being scaffolded from near-zero** — App.svelte is currently 22 lines including the `<style>` block; everything from router shell to design tokens to inline no-flash theme bootstrap to the toast host needs to be written.

**Primary recommendation:** Plan Phase 2 as 6-8 plans following the natural dependency order: (1) Phase 1 cleanup + ui/package.json wiring → (2) V013 migration + domain types → (3) DeviceRepository + DeviceService CRUD → (4) Search/autocomplete/group-by → (5) CSV import/export → (6) UI shell + theme + sidebar → (7) Devices feature UI + transport client + toast/error layer → (8) bindings export regeneration + cleanup. Each plan is end-to-end testable; plans 2-5 land Tauri commands incrementally so the UI plans can `apiCall(...)` against a real backend.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Devices CRUD validation | trackly-app (DeviceService) | trackly-core (domain types) | Required-field validation, optimistic-lock checks live with the service; DTO shape lives in trackly-app/dto |
| SQL access (devices) | trackly-infra (SqliteDeviceRepository) | — | Hexagonal port-adapter: repo implements the trait declared in trackly-core::ports |
| FTS5 trigger maintenance | Database (V013 SQL triggers) | — | Triggers fire transactionally on INSERT/UPDATE/DELETE — kept in SQL so consistency holds even if a future writer path bypasses the service layer |
| Autocomplete (DISTINCT + partial indexes) | trackly-infra (read path) | trackly-app (cache the universal endpoint shape) | Pure SQL — DISTINCT against indexed columns; service is a thin wrapper |
| Grouping non-unique devices | Database (GROUP BY) | trackly-app (DeviceGroup DTO assembly) | Group on SQL side — cheaper than aggregating in Rust over a fetched list |
| CSV encoding sniff + decode | trackly-app (csv module) | — | Pure-Rust, no DB access; live in trackly-app because uses DTO types directly |
| CSV preview-state TTL | trackly-app (ImportSessionStore) | — | In-memory only (CONTEXT.md D-CSV-01); separate Arc field on AppCtx, NOT part of DeviceService |
| Audit-log writes | trackly-app (DeviceService writer closure) | Database (audit_log table) | Same transaction as the mutation — written inside `writer.execute(|conn| { ... })` |
| Transport dispatch (Tauri vs HTTP) | UI lib/api/client.ts | — | Runtime detect via `'__TAURI_INTERNALS__' in window`; lazy `import('@tauri-apps/api/core')` |
| Routing (#/devices, #/devices/123) | UI features/layout (svelte-spa-router) | — | Hash routing — same SPA works in Tauri webview and future browser-served bundle |
| Theme persistence + no-flash | UI inline `<head>` script + lib/stores/theme.svelte.ts | Browser localStorage | Inline script runs BEFORE Vite-bundled module loads — kills FOUC |
| Error rendering | UI lib/components/Toast.svelte + lib/stores/toast.svelte.ts | trackly-app (Russian AppError.message) | UI parses AppError, never formats messages itself |
| Sidebar nav + placeholder pages | UI features/layout/Sidebar.svelte + sidebar-config.ts | — | Static config array; placeholders are 5-line components per non-Phase-2 section |

## Standard Stack

### Core (already pinned in Cargo.toml workspace; no version changes in Phase 2)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `rusqlite` | `0.38` (workspace pin, Plan 01-01) | SQLite driver | `[VERIFIED: Cargo.toml]` Already wired; `bundled` feature ships portable |
| `refinery` | `0.9` (workspace pin) | Forward-only migrations embedded via `embed_migrations!("../../migrations")` | `[VERIFIED: 01-03-SUMMARY]` V013 will be the 13th file refinery discovers |
| `axum` | `0.8` | HTTP router (router built in Phase 2; bind in Phase 5) | `[VERIFIED: 01-05-SUMMARY]` Already used for `/api/v1/health` |
| `tauri-specta` | `=2.0.0-rc.21` (pinned, do NOT bump) | DTO bindings.ts generation | `[CITED: deferred-items.md]` Upgrade blocked on stable-Rust `debug_closure_helpers` |
| `specta` | `=2.0.0-rc.22` | Type derives + `Type` impls | `[VERIFIED: Cargo.toml]` features `["derive", "serde", "serde_json"]` enabled |
| `serde` / `serde_json` | `1.x` | Snake_case JSON DTOs | `[VERIFIED: workspace]` Established Phase 1 invariant |
| `tokio` / `tokio-util` | `1.x` / `0.7` | mpsc + CancellationToken | `[VERIFIED: 01-04-SUMMARY]` writer worker, AppCtx.shutdown |
| `tracing` / `tracing-appender` | `0.1` / `0.2` | Logging | `[VERIFIED: 01-05-SUMMARY]` |
| `time` | `0.3` | UTC timestamps (NOT `chrono::Local`) | `[VERIFIED: 01-04-SUMMARY]` `SystemClock` uses `OffsetDateTime::now_utc()` |
| `uuid` | `1.x` | Token generation for CSV import sessions | `[ASSUMED]` workspace pin needs verification — Cargo.toml not re-read this session |
| `anyhow` / `thiserror` | `1.x` | Error handling | `[VERIFIED: workspace]` |
| `tauri` | `^2.11` | Desktop shell | `[VERIFIED: workspace]` |

### New Crate Dependencies for Phase 2

| Library | Version | Purpose | When to Use | Provenance |
|---------|---------|---------|-------------|------------|
| `chardetng` | `0.1.17` | CP1251/UTF-8 byte-stream encoding sniff | CSV import — feed bytes, get `&'static Encoding` | `[VERIFIED: docs.rs/chardetng/0.1.17]` confirmed `EncodingDetector::new() / .feed(&[u8], last: bool) -> bool / .guess(tld: Option<&[u8]>, allow_utf8: bool) -> &'static Encoding` |
| `encoding_rs` | `0.8` | Decode bytes to `String` once encoding is known | CSV import — `WINDOWS_1251.decode(bytes) -> (Cow<str>, &'static Encoding, bool)` | `[VERIFIED: docs.rs/encoding_rs/0.8]` `WINDOWS_1251` static exists; `decode()` returns 3-tuple `(Cow<'a, str>, &'static Encoding, bool)` |
| `csv` | `1.3` | CSV parsing (delimiter `,` / `;`) | CSV import/export | `[VERIFIED: docs.rs/csv/1.3]` `ReaderBuilder::new().delimiter(b';').has_headers(true).from_reader(&bytes[..])` |
| `tauri-plugin-dialog` | `^2` | Native file open/save dialog | CSV import (open) + CSV export (save-as) | `[VERIFIED: STACK.md + Tauri docs]` already named in CONTEXT.md D-Cleanup-01 area; install via Cargo + JS package below |

**Tauri plugins for Phase 2 — Rust side**:
```toml
# crates/trackly-app/Cargo.toml (illustrative — verify workspace.dependencies first)
tauri-plugin-dialog = "2"
```
Tauri plugin v2 also requires JS-side install (below). Capabilities/permissions for `dialog:default` must be declared in `tauri.conf.json` or per-window `capabilities/*.json` — Phase 1 has no Tauri runtime yet, so capability scaffolding is a Phase 2 first.

### Frontend JS packages (additions to ui/package.json)

| Package | Version (npm-verified) | Purpose | Notes |
|---------|------------------------|---------|-------|
| `@tauri-apps/api` | `2.11.0` | `invoke()`, `event`, `webviewWindow` runtime | `[VERIFIED: npm view 2026-05-25 → 2.11.0]` Add to `dependencies` (NOT devDependencies) per D-Cleanup-01 |
| `@tauri-apps/plugin-dialog` | `2.7.1` | JS bridge for `tauri-plugin-dialog` | `[VERIFIED: npm view 2026-05-25 → 2.7.1]` |
| `svelte-spa-router` | `5.1.0` | Hash router for Svelte 5 | `[VERIFIED: npm view 2026-05-25 → 5.1.0]` Released 2026-04-28; explicit Svelte 5 support per README |

### Version verification (run before writing into Cargo.toml / package.json):

```bash
# Rust
cargo search chardetng | head -1
cargo search encoding_rs | head -1
cargo search csv | head -1

# JS — already verified above:
# npm view @tauri-apps/api version          → 2.11.0
# npm view @tauri-apps/plugin-dialog version → 2.7.1
# npm view svelte-spa-router version        → 5.1.0
```

### Alternatives Considered (and rejected)

| Instead of | Alternative | Why rejected for Phase 2 |
|------------|-------------|--------------------------|
| svelte-spa-router | svelte-routing | History-mode routing requires server-side rewrites; hash routing works equally in Tauri webview and future browser-served bundle without any axum route config |
| chardetng + encoding_rs | encoding | `encoding` crate is unmaintained since 2020; `encoding_rs` is the canonical WHATWG-compliant choice and powers Firefox |
| hand-rolled toast | svelte-french-toast / svelte-sonner | CONTEXT.md D-UI-Errors-01 locks "hand-rolled ~80 LoC" — keeps zero dep footprint, full control over Russian message formatting |
| svelte stores (writable/readable) | Svelte 5 runes (`$state` in `.svelte.ts`) | CONTEXT.md D-UI-State-01 locks runes; module-level `$state` is canonical Svelte 5 sharing pattern |
| formsnap / superforms | manual `$derived` validation | CONTEXT.md D-UI-Validation-01: 4 required fields don't justify a library; SvelteKit coupling is also a non-starter |
| Pre-computed `device_field_values` table | DISTINCT + partial indexes | CONTEXT.md D-Autocomplete-01: premature optimization; <10k rows DISTINCT is <50ms |
| `tauri-plugin-fs` for file pick | `tauri-plugin-dialog` | dialog is the native open/save picker; fs is for read/write — we want the OS-native picker, then read the bytes via Rust `std::fs` |

## Package Legitimacy Audit

slopcheck was not available in this research environment (`pip install slopcheck --break-system-packages` not exercised). All packages below were verified via authoritative sources (official npm registry, docs.rs) AND cross-checked against existing project research (CLAUDE.md, STACK.md) which lists each as the canonical choice.

| Package | Registry | Age (first release era) | Source Repo | Disposition |
|---------|----------|------------------------|-------------|-------------|
| `chardetng` 0.1.17 | crates.io | 6+ years (Mozilla, hsivonen) | github.com/hsivonen/chardetng | Approved — authoritative author, used by Firefox |
| `encoding_rs` 0.8 | crates.io | 9+ years (Mozilla, hsivonen) | github.com/hsivonen/encoding_rs | Approved — Firefox-internal, WHATWG-compliant |
| `csv` 1.3 | crates.io | 11+ years (BurntSushi) | github.com/BurntSushi/rust-csv | Approved — canonical Rust CSV crate |
| `tauri-plugin-dialog` 2.x | crates.io | Maintained by tauri-apps org | github.com/tauri-apps/plugins-workspace | Approved — official Tauri plugin |
| `@tauri-apps/api` 2.11.0 | npmjs.com | Official tauri-apps publisher | github.com/tauri-apps/tauri | Approved |
| `@tauri-apps/plugin-dialog` 2.7.1 | npmjs.com | Official tauri-apps publisher | github.com/tauri-apps/plugins-workspace | Approved |
| `svelte-spa-router` 5.1.0 | npmjs.com | 7+ years (ItalyPaleAle, Microsoft engineer) | github.com/ItalyPaleAle/svelte-spa-router | Approved — Svelte 5 support explicit since v5.0.0 |

**Packages removed due to slopcheck [SLOP]:** none (slopcheck not run; verified via official docs + npm registry + cross-reference to CLAUDE.md/STACK.md)
**Packages flagged as suspicious [SUS]:** none

**Recommendation:** the planner SHOULD insert a `checkpoint:human-verify` task at the first install step (`pnpm add` / `cargo add`) per safety policy when slopcheck is unavailable, even though every package is independently corroborated. Cost: <60 seconds of user confirmation.

## Architecture Patterns

### System Architecture Diagram

```
                                 ┌─────────────────────────────────────────┐
                                 │  ui/ (Svelte 5 SPA — vanilla, no Kit)   │
                                 │                                         │
                                 │  index.html ──[inline no-flash script]──┤
                                 │       │                                 │
                                 │       ↓                                 │
                                 │  main.ts → App.svelte                   │
                                 │              │                          │
                                 │              ↓                          │
                                 │     <Router routes={ROUTES}> ◄──────┐   │
                                 │       │                              │   │
                                 │   ┌───┼────────┬───────────┐         │   │
                                 │   ↓   ↓        ↓           ↓         │   │
                                 │ Layout Sidebar Pages     placeholders │   │
                                 │           │       │                  │   │
                                 │           ↓       ↓                  │   │
                                 │     [theme   features/devices/       │   │
                                 │      switcher]  DevicesPage.svelte   │   │
                                 │                       │              │   │
                                 │                       ↓              │   │
                                 │              [DeviceList,            │   │
                                 │               DeviceFormModal,       │   │
                                 │               DeviceImportCsvModal]  │   │
                                 │                       │              │   │
                                 │                       ↓              │   │
                                 │              lib/api/devices.ts  ────┘   │
                                 │                       │                  │
                                 │                       ↓                  │
                                 │   lib/api/client.ts: apiCall(name,args)  │
                                 │     if isTauri: invoke(...)              │
                                 │     else:       fetch('/api/v1/' + name) │
                                 │                                          │
                                 │   lib/stores/{theme,toast,transport}     │
                                 │   .svelte.ts ($state runes, module-scoped)│
                                 └──────────┬──────────────────────────┬────┘
                                            │ Tauri invoke              │ HTTP (Phase 5 only)
                                            ↓                           ↓
        ┌───────────────────────────────────────────────────────────────────────┐
        │  trackly-app (composition root)                                       │
        │                                                                       │
        │  AppCtx { writer, readers, paths, config, clock, shutdown,            │
        │           log_guard, schema_version, devices: Arc<DeviceService> }    │
        │                                                                       │
        │  tauri_cmds/devices.rs (~12 thin commands)                            │
        │    ─ devices_list, devices_get, devices_create, devices_update,       │
        │      devices_delete, devices_search, devices_autocomplete,            │
        │      devices_state_hints, devices_import_csv_preview,                 │
        │      devices_import_csv_commit, devices_export_csv,                   │
        │      devices_list_grouped                                             │
        │    Each: 5-15 LOC, calls build_*() helper, returns Result<Dto,AppErr> │
        │                                                                       │
        │  http/devices.rs : router() -> Router<AppCtx>                         │
        │    Mirrors Tauri commands as POST /api/v1/<cmd> (Phase 5 binds)       │
        │                                                                       │
        │  services/device_service.rs : DeviceService { writer, readers,        │
        │                                               clock, csv_sessions }  │
        │    .create(NewDevice)  → writer.execute(|c| repo.create + audit_log) │
        │    .update(id, version, patch) → writer.execute(... optimistic-lock) │
        │    .list(filter, pagination) → spawn_blocking(|| readers.acquire())  │
        │    .search_fts(q) → spawn_blocking(...)                              │
        │    .autocomplete(field, prefix, ctx_name) → spawn_blocking(...)      │
        │    .list_grouped(filter) → spawn_blocking(...)                       │
        │    .import_csv_preview(bytes) → in-memory token; csv_sessions.put()  │
        │    .import_csv_commit(token, mapping) → writer.execute(... bulk)     │
        │    .export_csv(filter) → spawn_blocking + serialize                  │
        │                                                                       │
        │  csv/                                                                 │
        │    sniff.rs : detect_encoding_and_delimiter(&[u8]) -> CsvProfile     │
        │    decode.rs : decode_to_string(bytes, encoding) -> String           │
        │    parse.rs : parse_rows(&str, delimiter) -> Result<Vec<RawRow>>     │
        │    session_store.rs : ImportSessionStore (5-min TTL token map)       │
        │                                                                       │
        │  dto/device.rs : DeviceDto, DeviceNew, DevicePatch, DeviceFilter,    │
        │                  Pagination, CsvImportPreview, CsvImportReport,      │
        │                  DeviceGroup, STATE_HINTS const                      │
        └────────────┬──────────────────────────────────────────────────┬──────┘
                     │ writes through WriterHandle::execute              │ reads via ReaderPool
                     ↓                                                   ↓
        ┌────────────────────────────────────────────────────────────────────┐
        │  trackly-infra (adapters)                                          │
        │                                                                    │
        │  repos/devices_sqlite.rs : SqliteDeviceRepository                  │
        │    Implements trackly_core::ports::devices::DeviceRepository       │
        │    Functions take &Connection / &mut Connection — no async        │
        │    SQL strings hand-written, parameterized                         │
        │                                                                    │
        │  db/writer_worker.rs  (Phase 1)                                    │
        │  db/pools.rs           (Phase 1)                                   │
        │  db/migrations.rs      (Phase 1 — embed_migrations! picks up V013)│
        └────────────┬───────────────────────────────────────────────┬──────┘
                     │                                                │
                     ↓                                                ↓
        ┌────────────────────────────────────────────────────────────────────┐
        │  trackly-core (pure domain)                                        │
        │                                                                    │
        │  ports/devices.rs : trait DeviceRepository {                       │
        │    fn create(&self, conn: &mut Connection, new: &NewDevice,       │
        │              now_utc: i64) -> Result<i64, AppError>;              │
        │    fn update(&self, ...) -> Result<DeviceRow, AppError>;          │
        │    fn delete_soft(&self, ...) -> Result<(), AppError>;            │
        │    fn get(&self, conn: &Connection, id: i64) ...;                 │
        │    fn list(&self, ...) -> Result<(Vec<DeviceRow>, u64), AppError>;│
        │    fn search_fts(&self, ...) -> Result<Vec<DeviceRow>, AppError>; │
        │    fn list_grouped(&self, ...) -> Result<Vec<DeviceGroupRow>, _>; │
        │    fn autocomplete(&self, ...) -> Result<Vec<String>, AppError>;  │
        │  }                                                                 │
        │                                                                    │
        │  domain/devices.rs : NewDevice, DevicePatch, DeviceFilter,        │
        │                       Pagination, DeviceRow, DeviceGroupRow       │
        │                       (raw Rust types — NOT specta::Type-derived) │
        └────────────────────────────────────────────────────────────────────┘
                                       ↓
                              ┌──────────────────────┐
                              │  SQLite (WAL, single │
                              │  writer + 4 readers) │
                              │                      │
                              │  V013 migration adds:│
                              │   - 3 FTS5 triggers  │
                              │   - 5 partial idx    │
                              │     for autocomplete │
                              └──────────────────────┘
```

### Recommended Project Structure

**Rust additions** (relative to repo root):
```
crates/trackly-core/src/
├── domain/
│   ├── mod.rs            # pub mod devices;
│   └── devices.rs        # NewDevice, DevicePatch, DeviceFilter, Pagination, DeviceRow, DeviceGroupRow
├── ports/
│   ├── mod.rs            # pub mod devices;
│   └── devices.rs        # trait DeviceRepository
└── lib.rs                # pub mod domain; pub mod ports;

crates/trackly-infra/src/
├── repos/
│   ├── mod.rs            # pub mod devices_sqlite; pub use devices_sqlite::SqliteDeviceRepository;
│   └── devices_sqlite.rs # impl DeviceRepository for SqliteDeviceRepository
└── lib.rs                # pub mod repos;

crates/trackly-app/src/
├── csv/
│   ├── mod.rs
│   ├── sniff.rs          # detect_encoding_and_delimiter
│   ├── decode.rs         # decode_to_string + replacement handling
│   ├── parse.rs          # csv::ReaderBuilder wrapper
│   └── session_store.rs  # ImportSessionStore (5-min TTL)
├── services/
│   ├── mod.rs
│   └── device_service.rs # DeviceService::new(writer, readers, clock, csv_sessions)
├── dto/
│   └── device.rs         # DeviceDto, DeviceNew, DevicePatch, DeviceFilter,
│                         #   Pagination, CsvImportPreview, CsvImportReport,
│                         #   DeviceGroup, STATE_HINTS
├── tauri_cmds/
│   └── devices.rs        # 12 thin commands; each calls build_* helper
├── http/
│   └── devices.rs        # router() -> Router<AppCtx>
├── context.rs            # MODIFIED: AppCtx + devices: Arc<DeviceService>
└── specta_export.rs      # MODIFIED: collect_commands![..., devices_list, ...]

migrations/
└── V013__devices_fts_triggers.sql   # NEW

crates/trackly-app/tests/
├── devices_crud.rs                     # NEW: create/get/update/delete/optimistic-lock
├── devices_search.rs                   # NEW: FTS5 + trigger sync
├── devices_autocomplete.rs             # NEW: prefix + contextual
├── devices_grouping.rs                 # NEW: GROUP BY non-unique
├── devices_csv_import.rs               # NEW: 4 encodings × 2 delimiters; malformed rows
├── devices_csv_export.rs               # NEW: UTF-8 BOM + ; + Russian headers
├── fixtures/
│   └── devices/
│       ├── utf8.csv                    # NEW (text-checkable in PR review)
│       ├── utf8_bom.csv                # NEW (3-byte BOM prefix)
│       ├── cp1251_comma.csv            # NEW (binary blob — see Pitfall #17)
│       ├── cp1251_semicolon.csv        # NEW (binary blob)
│       └── malformed_mixed_rows.csv    # NEW (mostly valid + 2 broken rows)
└── export_bindings.rs                  # MODIFIED: assert new DTO substrings
```

**Frontend additions** (under `ui/`):
```
ui/
├── package.json          # MODIFIED: add @tauri-apps/api, @tauri-apps/plugin-dialog, svelte-spa-router
├── index.html            # MODIFIED: inline <head> no-flash script + mount point
├── src/
│   ├── main.ts           # UNCHANGED (mounts App)
│   ├── App.svelte        # REWRITTEN: <Router routes={ROUTES}/> + <ToastHost/> + initial theme apply
│   ├── routes.ts         # NEW: { '/': Dashboard, '/devices': DevicesPage, ... }
│   ├── lib/
│   │   ├── api/
│   │   │   ├── client.ts          # apiCall(name, args) — transport detect + lazy import
│   │   │   ├── errors.ts          # parseAppError(resp|err) — extract {code,message,details}
│   │   │   ├── devices.ts         # api.devices.list/get/create/update/delete/...
│   │   │   └── index.ts
│   │   ├── stores/
│   │   │   ├── theme.svelte.ts    # export const themeStore = $state({...})
│   │   │   ├── toast.svelte.ts    # export const toastStore = $state({...})
│   │   │   └── transport.svelte.ts # exports `isTauri` derived once
│   │   ├── components/
│   │   │   ├── Button.svelte
│   │   │   ├── Input.svelte
│   │   │   ├── Textarea.svelte
│   │   │   ├── Modal.svelte
│   │   │   ├── Select.svelte
│   │   │   ├── Toast.svelte
│   │   │   ├── ToastHost.svelte
│   │   │   ├── ThemeSwitcher.svelte
│   │   │   └── Placeholder.svelte    # "Раздел в разработке"
│   │   └── utils/
│   │       └── date.ts               # format unix-seconds → ru-RU display
│   ├── features/
│   │   ├── layout/
│   │   │   ├── Layout.svelte         # sidebar + main flex
│   │   │   ├── Sidebar.svelte        # iterates sidebar-config; use:active
│   │   │   └── sidebar-config.ts     # array of {kind, route?, label, icon?}
│   │   └── devices/
│   │       ├── DevicesPage.svelte    # top-level route component
│   │       ├── DeviceList.svelte     # table w/ pagination + group toggle
│   │       ├── DeviceListRow.svelte
│   │       ├── DeviceGroupRow.svelte # expandable group row
│   │       ├── DeviceFilters.svelte  # status switch-bar + FTS search input
│   │       ├── DeviceFormModal.svelte
│   │       ├── DeviceAutocompleteField.svelte
│   │       ├── DeviceImportCsvModal.svelte
│   │       └── api.ts                # feature-scoped wrappers
│   ├── pages/                        # tiny route components — placeholders
│   │   ├── Dashboard.svelte          # <Placeholder/>
│   │   ├── MapPage.svelte
│   │   ├── ActsPage.svelte
│   │   ├── PrintersPage.svelte
│   │   ├── CartridgesPage.svelte
│   │   ├── RequestsPage.svelte
│   │   ├── ReportsPage.svelte
│   │   ├── UsersPage.svelte
│   │   ├── SettingsPage.svelte
│   │   └── NotFound.svelte
│   └── styles/
│       ├── _tokens.scss              # REWRITTEN: real palette + spacing + typography + radii + shadows
│       └── global.scss               # NEW: reset, body styles, scrollbar, focus-ring
```

### Pattern 1: Single-helper-two-transport for every command

Phase 1 Plan 05 locked this. Every `#[tauri::command]` and matching axum handler delegate to one `build_*` helper that takes `&AppCtx`.

**Example for `devices_get`:**
```rust
// crates/trackly-app/src/tauri_cmds/devices.rs
use crate::context::AppCtx;
use crate::dto::device::DeviceDto;
use trackly_core::error::AppError;

pub async fn build_devices_get(ctx: &AppCtx, id: i64) -> Result<DeviceDto, AppError> {
    ctx.devices.get(id).await
}

#[tauri::command]
#[specta::specta]
pub async fn devices_get(
    state: tauri::State<'_, AppCtx>,
    id: i64,
) -> Result<DeviceDto, AppError> {
    build_devices_get(state.inner(), id).await
}
```

```rust
// crates/trackly-app/src/http/devices.rs
use axum::{extract::{State, Path}, routing::get, Json, Router};
use crate::context::AppCtx;
use crate::dto::device::DeviceDto;
use crate::tauri_cmds::devices::build_devices_get;
use trackly_core::error::AppError;

pub async fn handle_get(
    State(ctx): State<AppCtx>,
    Path(id): Path<i64>,
) -> Result<Json<DeviceDto>, AppError> {
    Ok(Json(build_devices_get(&ctx, id).await?))
}

pub fn router() -> Router<AppCtx> {
    Router::new().route("/api/v1/devices/{id}", get(handle_get))
    // NOTE: axum 0.8 path-capture syntax is {id} not :id
}
```

**Source:** [01-05-SUMMARY.md "Single-helper-two-transport pattern for DTOs"]

### Pattern 2: Write closure with audit_log in same transaction

```rust
// Inside DeviceService::create
let now = self.clock.unix_seconds();
let new_owned = new.clone(); // captured by closure
let user_id_opt: Option<i64> = None;  // Phase 2: no auth — always NULL
let id = self.writer.execute(move |conn| {
    let tx = conn.transaction().map_err(map_rusqlite)?;
    let id = repo.create(&tx, &new_owned, now)?;
    let after = repo.get(&tx, id)?;
    let after_json = serde_json::to_string(&after).map_err(|e| AppError::Internal {
        source_chain: format!("audit_log after-json: {e}"),
    })?;
    tx.execute(
        "INSERT INTO audit_log (entity_type, entity_id, action, user_id, before_json, after_json, payload_json, created_at_utc) \
         VALUES ('device', ?1, 'create', ?2, NULL, ?3, NULL, ?4)",
        rusqlite::params![id, user_id_opt, after_json, now],
    ).map_err(map_rusqlite)?;
    tx.commit().map_err(map_rusqlite)?;
    Ok(id)
}).await?;
```

**Why same-transaction:** if audit_log INSERT fails after entity write succeeds, we'd have inconsistent history. Same transaction either both commit or both roll back.

**Source:** [01-CONTEXT.md D-Schema-05 "audit_log after successful write in same transaction"]

### Pattern 3: Optimistic-lock UPDATE — surface mismatch, do NOT auto-retry

```rust
// Inside DeviceService::update
let affected = tx.execute(
    "UPDATE devices SET name = ?1, ..., version = version + 1, updated_at_utc = ?N \
     WHERE id = ?id AND version = ?expected_version AND deleted_at_utc IS NULL",
    rusqlite::params![...],
).map_err(map_rusqlite)?;
if affected == 0 {
    let actual: i64 = tx.query_row(
        "SELECT version FROM devices WHERE id = ?1",
        rusqlite::params![id],
        |row| row.get(0),
    ).map_err(map_rusqlite)?;
    return Err(AppError::OptimisticLockMismatch {
        entity: "device", id, expected: expected_version, actual,
    });
}
```

**Decision:** Do NOT auto-retry. Surface `OptimisticLockMismatch` to UI; the toast says "Данные были изменены другим пользователем — обновите страницу". Auto-retry hides concurrent edits from the user, which is the opposite of what optimistic locking is for.

**Source:** [01-CONTEXT.md D-Schema-04; reasoning per pitfalls #2]

### Pattern 4: FTS5 trigger sync (V013 SQL)

```sql
-- V013__devices_fts_triggers.sql

-- Sync devices_fts (declared in V012) with devices via triggers.
-- Soft-delete handling: when deleted_at_utc transitions NULL→NOT NULL, DELETE from FTS;
-- when it transitions NOT NULL→NULL (restore), INSERT into FTS.

CREATE TRIGGER devices_fts_ai AFTER INSERT ON devices
WHEN NEW.deleted_at_utc IS NULL
BEGIN
  INSERT INTO devices_fts(rowid, name, inventory_number, serial_number, model)
  VALUES (NEW.id, NEW.name, NEW.inventory_number, NEW.serial_number, NEW.model);
END;

CREATE TRIGGER devices_fts_ad AFTER DELETE ON devices
BEGIN
  INSERT INTO devices_fts(devices_fts, rowid, name, inventory_number, serial_number, model)
  VALUES ('delete', OLD.id, OLD.name, OLD.inventory_number, OLD.serial_number, OLD.model);
END;

CREATE TRIGGER devices_fts_au AFTER UPDATE ON devices
BEGIN
  -- delete the old row from the index (no-op if it wasn't there)
  INSERT INTO devices_fts(devices_fts, rowid, name, inventory_number, serial_number, model)
  VALUES ('delete', OLD.id, OLD.name, OLD.inventory_number, OLD.serial_number, OLD.model);
  -- re-insert if the new row is not soft-deleted
  INSERT INTO devices_fts(rowid, name, inventory_number, serial_number, model)
  SELECT NEW.id, NEW.name, NEW.inventory_number, NEW.serial_number, NEW.model
  WHERE NEW.deleted_at_utc IS NULL;
END;

-- Partial indexes for autocomplete (D-Autocomplete-01 + D-Schema-Phase2-01).
-- Each index is partial (filtered to deleted_at_utc IS NULL) — keeps autocomplete
-- queries off soft-deleted rows AND keeps index size proportional to live data.

CREATE INDEX idx_devices_autocomplete_name
  ON devices(name) WHERE deleted_at_utc IS NULL;

CREATE INDEX idx_devices_autocomplete_name_model
  ON devices(name, model) WHERE deleted_at_utc IS NULL AND model IS NOT NULL;

CREATE INDEX idx_devices_autocomplete_name_location
  ON devices(name, location_id) WHERE deleted_at_utc IS NULL AND location_id IS NOT NULL;

CREATE INDEX idx_devices_autocomplete_name_condition
  ON devices(name, condition) WHERE deleted_at_utc IS NULL AND condition IS NOT NULL;

CREATE INDEX idx_devices_autocomplete_name_complectation
  ON devices(name, complectation) WHERE deleted_at_utc IS NULL AND complectation IS NOT NULL;

PRAGMA user_version = 13;
```

**Notes:**
- `devices_fts` was declared in V012 with `content='devices'` and `content_rowid='id'` — this is "external content" mode where SQLite mirrors a separate table. With external content, deletes from the FTS index use the magic `INSERT INTO <tbl>(<tbl>, rowid, ...)` form with `'delete'` as the first argument.
- Triggers are AFTER (not BEFORE) so the canonical row exists in `devices` before we mirror it.
- `WHEN NEW.deleted_at_utc IS NULL` on INSERT trigger handles the (rare) case of creating something already soft-deleted (we never do this, but defensive).
- The UPDATE trigger does delete-then-conditional-insert to handle both content changes AND deletion-state transitions (restore from soft-delete → re-index; soft-delete → de-index).
- Partial indexes use SQLite predicate-index syntax which is supported since 3.8.0; "bundled" rusqlite 0.38 ships a modern SQLite that supports this.

**Source:** [SQLite FTS5 docs §4.4.3 "External content tables"](https://sqlite.org/fts5.html#external_content_tables), `[CITED: V012 already uses content='devices' mode]`

### Pattern 5: Vite + inline `<head>` no-flash script

Vite leaves `<script>` tags inside `index.html` untouched by default — it only rewrites module imports. An inline `<script>` (not a module) with no `src` attribute is shipped verbatim into the build output. It runs synchronously BEFORE the Vite-bundled `<script type="module" src="/src/main.ts">` loads.

```html
<!-- ui/index.html — add to <head>, before the Vite entry script -->
<script>
  (function () {
    try {
      var saved = localStorage.getItem('trackly:theme'); // 'light' | 'dark' | 'system' | null
      var prefers = window.matchMedia('(prefers-color-scheme: dark)').matches;
      var theme = (saved === 'light' || saved === 'dark') ? saved
        : (prefers ? 'dark' : 'light');
      document.documentElement.dataset.theme = theme;
    } catch (e) { /* localStorage unavailable (privacy mode); fall through to light */ }
  })();
</script>
```

**CSP consideration:** Tauri 2 enforces CSP via `tauri.conf.json`. If we set a strict CSP (Phase 5 / Phase 8 hardening), the inline script needs either `unsafe-inline` (bad) or a hash/nonce. Phase 2 doesn't set CSP yet, so this is fine; flag for Phase 5 to add a hash if CSP tightens.

**Source:** [Vite docs — index.html as entry point](https://vitejs.dev/guide/#index-html-and-project-root)

### Pattern 6: CSV encoding sniff + decode + parse pipeline

```rust
// crates/trackly-app/src/csv/sniff.rs
use chardetng::EncodingDetector;
use encoding_rs::{Encoding, UTF_8};

pub struct CsvProfile {
    pub encoding: &'static Encoding,
    pub delimiter: u8,
}

pub fn detect(bytes: &[u8]) -> CsvProfile {
    // 1. BOM check (fast path).
    let encoding = if bytes.starts_with(b"\xEF\xBB\xBF") {
        UTF_8
    } else {
        let mut det = EncodingDetector::new();
        det.feed(bytes, true);
        det.guess(None, true) // allow UTF-8 as a candidate
    };
    // 2. Delimiter sniff: count `,` vs `;` in the first non-empty decoded line.
    let (decoded, _, _) = encoding.decode(bytes);
    let first_line = decoded.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
    let comma = first_line.bytes().filter(|b| *b == b',').count();
    let semi = first_line.bytes().filter(|b| *b == b';').count();
    let delimiter = if semi > comma { b';' } else { b',' };
    CsvProfile { encoding, delimiter }
}
```

```rust
// crates/trackly-app/src/csv/decode.rs
use encoding_rs::Encoding;

pub fn decode_to_string(bytes: &[u8], encoding: &'static Encoding) -> (String, bool) {
    let (cow, _used, had_replacements) = encoding.decode(bytes);
    (cow.into_owned(), had_replacements)
}
```

```rust
// crates/trackly-app/src/csv/parse.rs
use csv::ReaderBuilder;

pub fn parse_rows(text: &str, delimiter: u8) -> Result<(Vec<String>, Vec<Vec<String>>), csv::Error> {
    let mut rdr = ReaderBuilder::new()
        .delimiter(delimiter)
        .has_headers(true)
        .flexible(true)  // tolerate ragged rows; we report per-row errors at commit
        .from_reader(text.as_bytes());
    let headers: Vec<String> = rdr.headers()?.iter().map(|s| s.to_string()).collect();
    let mut rows = Vec::new();
    for rec in rdr.records() {
        let r = rec?;
        rows.push(r.iter().map(|s| s.to_string()).collect());
    }
    Ok((headers, rows))
}
```

**API confirmations** `[VERIFIED: docs.rs 2026-05-25]`:
- `EncodingDetector::new() -> Self`
- `EncodingDetector::feed(&mut self, buffer: &[u8], last: bool) -> bool`
- `EncodingDetector::guess(&self, tld: Option<&[u8]>, allow_utf8: bool) -> &'static Encoding`
- `Encoding::decode<'a>(&'static self, bytes: &'a [u8]) -> (Cow<'a, str>, &'static Encoding, bool)`
- `encoding_rs::{UTF_8, WINDOWS_1251}` statics both exist
- `csv::ReaderBuilder::new().delimiter(b';').has_headers(true).from_reader(rdr)` works against `&[u8]`

### Pattern 7: CSV preview-then-commit token store (in-memory, 5-min TTL)

```rust
// crates/trackly-app/src/csv/session_store.rs
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use uuid::Uuid;

pub struct ImportSession {
    pub encoding: &'static encoding_rs::Encoding,
    pub delimiter: u8,
    pub headers: Vec<String>,
    pub all_rows: Vec<Vec<String>>,  // full decoded file, kept for commit step
    pub created: Instant,
}

const TTL: Duration = Duration::from_secs(5 * 60);

pub struct ImportSessionStore {
    inner: Mutex<HashMap<Uuid, ImportSession>>,
}

impl ImportSessionStore {
    pub fn new() -> Self { Self { inner: Mutex::new(HashMap::new()) } }

    pub fn put(&self, session: ImportSession) -> Uuid {
        let token = Uuid::new_v4();
        let mut g = self.inner.lock().expect("poisoned");
        // Lazy sweep — remove expired entries on every put.
        let now = Instant::now();
        g.retain(|_, s| now.duration_since(s.created) < TTL);
        g.insert(token, session);
        token
    }

    pub fn take(&self, token: Uuid) -> Option<ImportSession> {
        let mut g = self.inner.lock().expect("poisoned");
        let now = Instant::now();
        if let Some(s) = g.remove(&token) {
            if now.duration_since(s.created) < TTL { return Some(s); }
        }
        None
    }
}
```

**Design notes:**
- `take` (not `get`): single-use token. If user reloads preview UI without committing, they get a NEW preview + NEW token; old one stays in map until next `put` triggers lazy sweep.
- Lazy sweep on `put` only (no background task) — simpler, avoids tokio scheduling overhead, and the map churns slowly (one session per CSV import attempt).
- Lives as a separate `Arc<ImportSessionStore>` field on `DeviceService` (NOT a separate AppCtx field — only DeviceService methods touch it). Alternative considered: put on AppCtx for symmetry with other services — rejected because no other code reads/writes it.
- Memory bound: each session holds the full decoded CSV in `all_rows`. For a 5000-row × 10-col CSV with ~50-char fields, that's ~2.5MB per active session. Acceptable for the LAN-scale use case.

### Pattern 8: Transport-detect client (Svelte 5)

```typescript
// ui/src/lib/api/client.ts
import { parseAppError } from './errors';

const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

export async function apiCall<R>(name: string, args: Record<string, unknown> = {}): Promise<R> {
  if (isTauri) {
    const { invoke } = await import('@tauri-apps/api/core');
    try {
      return await invoke<R>(name, args);
    } catch (e) {
      // invoke() rejects with the serialized AppError {code, message, details}
      throw parseAppError(e);
    }
  }
  // Phase 5+ HTTP path. Phase 2 stub — never executed in Tauri runtime.
  const res = await fetch(`/api/v1/${name}`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(args),
  });
  if (!res.ok) throw parseAppError(await res.json().catch(() => ({})));
  return res.json();
}
```

**`isTauri` runtime detection** `[VERIFIED: Tauri Discussion #6119 — `'__TAURI_INTERNALS__' in window` is the Tauri 2 sentinel; in Tauri 1 it was `__TAURI__`]`. The dynamic `import('@tauri-apps/api/core')` lets Vite tree-shake it out of a browser-only build if needed, but since we ship one bundle for both contexts, both module copies will be in the bundle either way — the dynamic import just defers evaluation until actually needed (saves <5KB parse time on first browser load).

**`invoke()` error semantics** `[VERIFIED: v2.tauri.app/develop/calling-rust/]`: when the Rust command returns `Err(AppError)`, `invoke()` REJECTS the promise with the serialized `AppError` value. Our hand-written `Serialize` impl produces `{code, message, details}` so the JS `catch (e) => e` receives that exact object.

### Pattern 9: Svelte 5 module-level `$state` runes

```typescript
// ui/src/lib/stores/theme.svelte.ts
// .svelte.ts extension REQUIRED — the Svelte 5 compiler only processes runes in .svelte/.svelte.ts/.svelte.js files

type Resolved = 'light' | 'dark';
type Preference = 'light' | 'dark' | 'system';

export const themeStore = $state({
  preference: 'system' as Preference,
  resolved: 'light' as Resolved,
});

const KEY = 'trackly:theme';
const mql = typeof window !== 'undefined'
  ? window.matchMedia('(prefers-color-scheme: dark)')
  : null;

export function initTheme(): void {
  const saved = (localStorage.getItem(KEY) ?? 'system') as Preference;
  themeStore.preference = saved;
  applyResolved();
  mql?.addEventListener('change', () => {
    if (themeStore.preference === 'system') applyResolved();
  });
}

export function setTheme(p: Preference): void {
  themeStore.preference = p;
  localStorage.setItem(KEY, p);
  applyResolved();
}

function applyResolved(): void {
  const r: Resolved = themeStore.preference === 'system'
    ? (mql?.matches ? 'dark' : 'light')
    : themeStore.preference;
  themeStore.resolved = r;
  document.documentElement.dataset.theme = r;
}
```

**Critical syntax confirmations** `[VERIFIED: svelte.dev/docs/svelte/$state]`:
- Use `export const x = $state({ ... })` — `const` works because the variable itself is never re-assigned, only its properties are mutated.
- `export let x = $state(0)` followed by `x = 1` does NOT work — assignment to the exported binding is rejected by the compiler. For scalars: wrap in a small object `{ value: ... }` (as themeStore does for `resolved`) or expose a setter function.
- Filename MUST end in `.svelte.ts` (or `.svelte.js`) — plain `.ts` files don't get rune transformation.

### Pattern 10: Sidebar active-link via `use:active`

```svelte
<!-- ui/src/features/layout/Sidebar.svelte -->
<script lang="ts">
  import { link } from 'svelte-spa-router';
  import active from 'svelte-spa-router/active';
  import { SIDEBAR_ITEMS } from './sidebar-config';
</script>

<nav>
  <ul>
    {#each SIDEBAR_ITEMS as item}
      {#if item.kind === 'divider'}
        <li class="divider"></li>
      {:else}
        <li>
          <a
            href={item.route}
            use:link
            use:active={{ path: item.route, className: 'is-active' }}
          >{item.label}</a>
        </li>
      {/if}
    {/each}
  </ul>
</nav>
```

`use:active` adds the `is-active` class when the current hash route matches. `use:link` upgrades plain `<a href="/devices">` so it navigates via the router instead of full-page reload. **Source:** `[VERIFIED: svelte-spa-router README — `use:link` + `use:active` actions, 5.1.0]`.

### Pattern 11: axum router for Phase-2 (build-only — Phase 5 binds)

```rust
// crates/trackly-app/src/http/devices.rs
use axum::{routing::{get, post, patch, delete}, Router};
use crate::context::AppCtx;

pub fn router() -> Router<AppCtx> {
    Router::new()
        .route("/api/v1/devices",            post(handle_create).get(handle_list))
        .route("/api/v1/devices/{id}",       get(handle_get).patch(handle_update).delete(handle_delete))
        .route("/api/v1/devices/search",     get(handle_search))
        .route("/api/v1/devices/autocomplete", get(handle_autocomplete))
        .route("/api/v1/devices/state-hints", get(handle_state_hints))
        .route("/api/v1/devices/grouped",    get(handle_list_grouped))
        .route("/api/v1/devices/import/preview", post(handle_import_preview))
        .route("/api/v1/devices/import/commit",  post(handle_import_commit))
        .route("/api/v1/devices/export.csv",  get(handle_export_csv))
}
```

**Composition:** Phase 5 will add an umbrella `http::router(ctx)` in `trackly-app/src/http/mod.rs` that does:
```rust
pub fn build_app_router(ctx: AppCtx) -> Router {
    Router::new()
        .merge(health::router())
        .merge(devices::router())
        .with_state(ctx)
}
```
Phase 2 deliverable is JUST `devices::router()` — it must compile and be testable via `tower::ServiceExt::oneshot` (same as Phase 1's `health_smoke.rs`). Phase 5 will compose and bind.

**axum 0.8 path-capture syntax:** `{id}` instead of `:id` per axum 0.8 changelog (renamed in 0.8). `[VERIFIED: 01-05-SUMMARY pattern + axum 0.8 docs]`.

### Anti-Patterns to Avoid

- **Building UI before commands compile.** Plan order matters: backend commands must register in `specta_export::builder()` BEFORE the UI can call them via `bindings.ts`. Each backend plan should regenerate bindings (`cargo test -p trackly-app --test export_bindings`) before its commit — UI plans depend on the latest `ui/src/bindings.ts`.
- **Putting CSV-import state in a global `static`.** Use a field on `DeviceService` (which is itself `Arc`-wrapped in AppCtx). Statics break test isolation.
- **`@tauri-apps/api` in `devDependencies`.** It's a RUNTIME dep of the UI bundle. Put in `dependencies` (D-Cleanup-01).
- **Routing via `history.pushState` (svelte-routing).** Hash routing is required so the same bundle works in browser without server-side rewrites.
- **Storing `themeStore.resolved` directly via `themeStore = ...` reassignment.** Only mutate properties; never reassign the exported state binding.
- **Returning camelCase JSON.** Snake_case is the locked Phase 1 invariant (HealthDto serializes `db_ready`, `schema_version` — Phase 2 DTOs follow the same rule).
- **Reading `state_id` from devices vs `condition` column.** The actual V003 schema does NOT have a `state_id` or `state` column — it has `condition` and `complectation`. See Critical Conflict below.
- **Spawning background TTL sweep for ImportSessionStore.** Lazy sweep on `put` is enough; avoid scheduling overhead.
- **Calling `tracing::init` from tests.** Plan 1 Plan 05 established `with_default(scoped, || ...)` for test-scoped subscribers.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Encoding detection | Manual byte-frequency analysis | `chardetng` 0.1.17 | Mozilla-grade detector, used by Firefox; 7+ years of edge-case tuning |
| Byte→String decode | `String::from_utf8_lossy` for CP1251 | `encoding_rs::WINDOWS_1251.decode(bytes)` | UTF-8 lossy mangles every Cyrillic byte in CP1251; encoding_rs has every Russian variant |
| CSV parsing | Hand-written tokenizer | `csv` 1.3 | Handles quoting, embedded newlines, escaped delimiters; 11 years mature |
| Hash routing | Custom `window.location.hash` + popstate | `svelte-spa-router` 5.1.0 | Includes `use:link`, `use:active`, route params, nested routes |
| Toast component | Anything beyond ~80 LoC `Toast.svelte` + host | CONTEXT.md locks hand-roll | (Locked by D-UI-Errors-01) — `svelte-french-toast` is 10KB + opinions we don't want |
| Form validation | `formsnap` / `superforms` | Manual `$derived` checks | (Locked by D-UI-Validation-01) — 4 required fields don't justify a library |
| FTS5 token escaping | Custom regex | Sanitize via simple `escape: '\'' → '\'\''` + append `*` per term | Built-in FTS5 query syntax is well-documented and our sanitizer is 1 function |
| Optimistic-lock retry loop | Auto-retry-with-fresh-read | Surface `OptimisticLockMismatch` to UI | User decides whether to overwrite — hiding concurrency from user defeats the purpose |
| UUID v4 token | `rand` + bit-twiddling | `uuid::Uuid::new_v4()` | One LOC vs subtle entropy bugs |
| Tauri dialog file picker | HTML `<input type="file">` in Tauri webview | `tauri-plugin-dialog` save/open | Native OS dialog respects "недавние документы", file-type filters; HTML input doesn't return absolute path in Tauri 2 by default |
| Russian Excel-friendly CSV | Manual BOM byte-prefix | `csv::WriterBuilder::new().delimiter(b';').from_writer(...)` + explicit `b"\xEF\xBB\xBF"` prefix | csv crate handles quoting; we only add BOM, not custom serialization |

**Key insight:** Phase 2 has zero infrastructure-level new dependencies — everything we need is either already in the workspace (rusqlite, axum, tracing) or is a well-trodden adapter library. The bespoke surface is small: ~80 LoC toast, ~150 LoC CSV pipeline, ~200 LoC theme/sidebar layer. Anything bigger than 200 LoC of bespoke UI infrastructure is a code smell — pause and re-check whether an established library exists.

## Runtime State Inventory

Phase 2 is greenfield for devices — no rename/migration. **Section omitted per researcher instructions.**

## Common Pitfalls

### Pitfall 1: V003 schema field names DIVERGE from CONTEXT.md DTO names — CRITICAL

**What goes wrong:** CONTEXT.md drafts DTOs and SQL queries assuming columns `inventory_no`, `serial_no`, `specs`, `kit`, `state`, `status`. Actual `migrations/V003__devices.sql` has columns `inventory_number`, `serial_number` (NOT `_no`), `condition` (NOT `state`), `complectation` (NOT `kit`), `notes` (NOT `specs`), `status_id` (FK to `device_statuses` — NOT `status` string), `location_id` (FK — NOT `location`). The V012 FTS5 table references `inventory_number`, `serial_number`, `model` — these names must be used verbatim in V013 triggers and in repository SQL.

**Why it happens:** Phase 1 schema was specced separately from Phase 2 CONTEXT.md; CONTEXT.md used domain-language column names while V003 used SQL-conventional names. CONTEXT.md was written assuming names would be reconcilable later.

**How to avoid:**
- **Repository SQL must use the V003 column names** (`inventory_number`, `serial_number`, `condition`, `complectation`, `notes`, `status_id`, `location_id`).
- **DTOs (`DeviceDto`, `DeviceNew`, `DevicePatch`) decide one of two paths:**
  - **Path A (recommended):** Match V003 — fields are `inventory_number`, `serial_number`, `condition`, `complectation`, `notes` (or rename `notes` → `specs` because the requirement REQ-DEV-01 calls it "Технические характеристики"). Mapping is identity. Audit-log JSON shape is stable.
  - **Path B:** Use the CONTEXT.md names in DTO and rename via `#[serde(rename = "inventory_number")]` in the repo struct OR via SELECT aliasing. More work, more surfaces for mismatch.
- **`status` field:** V003 has `status_id INTEGER` referencing `device_statuses`. UI wants a label string. Repository must JOIN to `device_statuses` to return the label; DTO carries both `status_id` (for write-back) and `status_label` (for display).
- **`location` field:** same pattern — JOIN `locations`; DTO carries `location_id` + `location_name`. Autocomplete on "location" returns names from the `locations` table OR distinct location_ids — for v1, simplest is to autocomplete on `locations.name` directly (D-Autocomplete-01 needs adjustment).
- **`type` field:** V003 has `type_id INTEGER REFERENCES device_types`. Same JOIN pattern.

**Action for planner:** First plan (after cleanup) should LOCK the DTO field-name choice in a short ADR-style decision note. Recommend Path A with `notes → specs` rename in V013 (`ALTER TABLE devices RENAME COLUMN notes TO specs`) — cleanest match to REQ-DEV-01 wording.

**Warning signs:** `cargo build` errors mentioning `no field 'inventory_no' on type 'DeviceRow'`; SQL errors `no such column: state`; FTS5 trigger references nonexistent column.

### Pitfall 2: FTS5 external-content triggers and `INSERT INTO <tbl>(<tbl>, ...)` delete-form syntax

**What goes wrong:** Standard `DELETE FROM devices_fts WHERE rowid = ?` against a `content='devices'` external-content FTS5 table doesn't actually work the way you'd expect — for external content tables, deletes are done via a magic INSERT with `'delete'` as the first column value, listing the OLD values of all FTS columns (so SQLite can locate the row in the auxiliary tables).

**Why it happens:** External content mode treats the FTS5 virtual table as an index whose source-of-truth is the content table; deletes need to know the old values to update internal stats.

**How to avoid:** Use this exact form in the AFTER DELETE trigger (and the prefix of the AFTER UPDATE trigger):
```sql
INSERT INTO devices_fts(devices_fts, rowid, name, inventory_number, serial_number, model)
VALUES ('delete', OLD.id, OLD.name, OLD.inventory_number, OLD.serial_number, OLD.model);
```
All FTS columns must appear, with their OLD values, in the order they were declared in the virtual-table creation.

**Warning signs:** Searching for a deleted device's name still returns hits; `INTEGRITY_CHECK` reports FTS index corruption; INSERT triggers fail silently because the old row blocks the new one.

**Source:** [SQLite FTS5 §4.4.3 External Content Tables](https://sqlite.org/fts5.html#external_content_tables)

### Pitfall 3: FTS5 prefix-search requires `*` in the query, not in the trigger

**What goes wrong:** Users type "lenov" expecting to find "Lenovo" devices. Without `*` suffix, FTS5 does whole-token matching only — "lenov" finds nothing. Adding `*` only at index-build time does nothing; the wildcard belongs in the QUERY.

**How to avoid:** In `DeviceService::search_fts`, sanitize and append `*`:
```rust
fn build_fts_query(user_input: &str) -> String {
    user_input
        .split_whitespace()
        .map(|t| t.replace('"', "\"\"").replace('\0', ""))  // escape quotes; drop NUL
        .filter(|t| !t.is_empty())
        .map(|t| format!("\"{t}\"*"))  // quote each token + prefix-match
        .collect::<Vec<_>>()
        .join(" ")
}
```
Quoting protects FTS5 operators (`AND`, `OR`, `NEAR`, parentheses) in user input. Trailing `*` enables prefix-match per token.

**Warning signs:** Search for "lenov" returns 0 results when devices named "Lenovo ThinkPad" exist; search containing `(` or `)` returns FTS5 syntax error.

### Pitfall 4: Tauri 2 dialog plugin requires explicit capability declaration

**What goes wrong:** `await open({ multiple: false, filters: [...] })` from `@tauri-apps/plugin-dialog` rejects with "dialog.open not allowed" at runtime even though both the Rust plugin and JS package are installed.

**Why it happens:** Tauri 2's capability/ACL model defaults to NOTHING-allowed; every plugin command needs an explicit permission declaration in `src-tauri/capabilities/*.json` (or `tauri.conf.json`'s `app.security.capabilities` per-window list).

**How to avoid:** When wiring `tauri-plugin-dialog`, create `src-tauri/capabilities/main.json` (or equivalent name) with:
```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "main",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "dialog:default"
  ]
}
```
Then register the plugin in `main.rs`:
```rust
tauri::Builder::default()
    .plugin(tauri_plugin_dialog::init())
    // ... rest of builder ...
```

**Phase 2 first-time note:** Phase 1's `main.rs` does NOT yet construct a `tauri::Builder` — it only runs `--self-test` against `AppCtx`. The first Phase 2 plan that wires real UI must add the full `tauri::Builder::default()...run()` block, register `tauri-plugin-single-instance` (already in Cargo.toml from Plan 01-01 per carry-forward), register `tauri-plugin-dialog`, attach `.invoke_handler(specta_export::builder().invoke_handler())`, manage `AppCtx` as `tauri::State`. This is non-trivial — budget an hour.

**Warning signs:** "command not allowed" errors at runtime even after plugin is installed; the `capabilities/` directory doesn't exist.

### Pitfall 5: Russian Excel writes CSV with `;` and reads `,` as text — exporting `,` produces "all in column A"

**What goes wrong:** Export devices to `.csv`, double-click in Russian-locale Excel — every row is in column A, fields visible separated by literal commas. Or: comma-delimited CSV with Russian numbers like "1,5" gets parsed with the comma as field separator.

**Why it happens:** Russian Excel uses `;` as the list separator (List separator setting in regional settings is `;` by default in Russian locale). Importing CSV with `,` requires a manual import wizard.

**How to avoid (D-CSV-02):**
- Export ALWAYS uses `;` as delimiter.
- Prepend UTF-8 BOM (`\xEF\xBB\xBF`) so Excel recognizes UTF-8 (without BOM, Russian Excel guesses CP1251 → mojibake).
- Russian column headers literal: `"Тип";"Наименование";"Инвентарный №";...`
- Test fixture: write a sample export, open in real Russian Excel manually, verify column split. (Cannot automate in CI without an actual Excel install.)

**Warning signs:** User reports "all in one column"; CSV opens with `Тип;Наименование` as the visible row contents in column A.

### Pitfall 6: Cyrillic FTS5 `unicode61 remove_diacritics 2` and the ё/е equivalence

**What goes wrong:** User searches "Петров", devices stored as "Петрoв" (with Latin `o` typo) return nothing. Or search "ёлка" returns nothing for "елка"-stored devices.

**Why it happens:** V012 specifies `tokenize='unicode61 remove_diacritics 2'` — which normalizes diacritics (acute, grave, etc.) AND folds `ё → е` because Russian diacritic conventions treat ё as e with diaeresis. This is mostly what we want, but searching "ёлка" actually works because both query and index normalize. Mixed-script typos (Cyrillic+Latin look-alikes) are NOT handled by FTS5.

**How to avoid:**
- `remove_diacritics 2` is the right choice — confirmed by the V012 author.
- Mixed-script Latin/Cyrillic homograph confusion (lookalikes like Latin `o` vs Cyrillic `о`) is a data-quality problem, not a search problem. Don't try to "fix" in FTS5; surface as a Phase 7 audit.
- Test fixture: `«ёлка vs елка should both match query "елка"»`.

**Warning signs:** Users report "I can't find Петрова" — check ё/е, then check Latin/Cyrillic mixing.

### Pitfall 7: CSV import where the first 5 preview rows decode fine but row 47 has invalid UTF-8

**What goes wrong:** Encoding detection sees the first chunk, says UTF-8, decode succeeds with replacement chars on some rows. Preview shows fine. Commit fails halfway, partial inserts happen.

**Why it happens:** Real-world CSVs are sometimes mixed-encoding (saved-as-UTF-8 on top of CP1251-saved file). `encoding_rs::decode` returns `(Cow, &Encoding, had_replacements: bool)` — the third tuple element flags this.

**How to avoid:**
- Surface `had_replacements: true` in `CsvImportPreview` as a warning field; UI shows "Внимание: возможные ошибки декодирования в N строках".
- Commit MUST be all-rows-in-one-tx so partial failure rolls back — OR strict accumulation: each row commits individually inside a single `writer.execute(...)` closure that loops over rows and per-row errors accumulate into `RowError[]`. Choice depends on user expectation — Atomic-commit-on-fail is safer; per-row accumulation is what CONTEXT.md D-CSV-01 specifies ("ошибки на каждую строку аккумулируются в `Vec<(row_index, AppError)>`").
- **Recommendation:** Accumulate per-row errors as CONTEXT specs; if ANY row has a non-validation error (e.g., DB locked), abort and roll back the whole transaction.

### Pitfall 8: `tauri::State<'_, AppCtx>` lifetime acrobatics in tests

**What goes wrong:** Writing unit tests for `#[tauri::command]` functions directly is painful because `tauri::State<'_, T>` needs a real Tauri runtime to construct.

**How to avoid (Plan 01-05 pattern):** Test the `build_*` helper which takes `&AppCtx`:
```rust
#[tokio::test]
async fn build_devices_create_inserts_row() {
    let (ctx, _guard) = minimal_ctx().await;
    let id = build_devices_create(&ctx, DeviceNew { ... }).await.unwrap();
    assert!(id > 0);
}
```
The thin `#[tauri::command]` wrapper has no logic to test beyond compilation, which `cargo build` catches.

**Source:** [01-05-SUMMARY "Single-helper-two-transport pattern" + tests/health_smoke.rs structure]

### Pitfall 9: `WriterHandle::execute` is `#[must_use]` — silent loss of write errors

**What goes wrong:** Code like `ctx.writer.execute(|c| { ... });` (no `.await`) silently drops the returned future, the write never happens. Plan 01-04 marked `execute` with `#[must_use]` to catch this at compile time as a clippy warning.

**How to avoid:** Always `.await` the result and propagate via `?`. Plan check should ban `let _ = writer.execute(...)` patterns in code review.

**Source:** [01-04-SUMMARY key-decisions]

### Pitfall 10: Hexagonal port traits that take `async fn` lock out sync repository impls

**What goes wrong:** Declaring `trait DeviceRepository { async fn create(&self, ...) -> ... }` forces the impl into async-fn-in-trait territory (Rust 1.75+) and makes the SQLite-backed impl awkward because rusqlite is sync. Wrapping each repo call in `spawn_blocking` is the right place — at the SERVICE layer, not inside the repo.

**How to avoid:** Port methods are `fn`, not `async fn`. They take `&Connection` or `&mut Connection`. The service layer wraps the call in `writer.execute(|c| repo.create(c, ...))` or `tokio::task::spawn_blocking({ let r = readers.clone(); move || { let g = r.acquire(); repo.list(&g, ...) } })`.

### Pitfall 11: Forgetting to add new commands to `collect_commands![...]` → frontend can't see them

**What goes wrong:** `devices_search` works in unit tests but the UI gets "command not found" because it wasn't registered in `specta_export::builder()`.

**How to avoid:** Update `specta_export.rs` `collect_commands![...]` in the same plan that adds the command. The Phase 2 ROADMAP says `~13 commands total` (health + 12 device-related). Verification: `cargo test -p trackly-app --test export_bindings` regenerates bindings.ts; grep it for the new command name.

**Source:** [01-05-SUMMARY "Carry-forward notes"]

### Pitfall 12: `non-NULL serial_number BUT empty string` — DEV-03 "уникальное" vs grouping classification

**What goes wrong:** CONTEXT.md D-Group-01 defines "не-уникальное = оба `inventory_no` и `serial_no` пусты (NULL)". But CSV imports often produce empty strings (`""`) instead of NULL. The classification SQL `WHERE inventory_number IS NULL AND serial_number IS NULL` then misses empty-string entries.

**How to avoid:** Either
- (A) Normalize on insert: empty string → NULL in repo create/update;
- (B) Classification SQL uses `WHERE COALESCE(NULLIF(inventory_number,''), NULL) IS NULL AND COALESCE(NULLIF(serial_number,''), NULL) IS NULL`.

Recommend (A) — consistent storage shape, simpler queries. Document this in DeviceService.

### Pitfall 13: `Vite` dev server vs Tauri webview `file://` — different origins, CORS

**What goes wrong:** During `pnpm dev` Tauri loads `http://localhost:1420`; in production build Tauri loads `tauri://localhost`. Code that hard-codes URL origins breaks on production build.

**How to avoid:** `apiCall` uses relative paths (`/api/v1/...`) and `invoke()` — no absolute origins. Don't write `fetch('http://localhost:1420/api/...')` anywhere.

### Pitfall 14: localStorage access in SSR/non-browser contexts (theme bootstrap)

**What goes wrong:** Inline `<head>` script crashes on `localStorage.getItem` if running in some headless/test context — but for Tauri webview + browser this is not an issue (both have localStorage). The wrapping `try { ... } catch (e) {}` covers privacy-mode browsers that disable storage.

**How to avoid:** Always wrap localStorage access in try/catch. Theme bootstrap script already does this.

### Pitfall 15: `module-level $state` in `.svelte.ts` — direct binding reassignment is not allowed

**What goes wrong:** Writing `export let count = $state(0)` and later `count++` fails to compile — Svelte 5 forbids reassigning an exported state binding.

**How to avoid:** Wrap scalars in objects, OR use `let count = $state(0)` (non-exported) + `export function getCount() { return count; }` and `export function setCount(n) { count = n; }`. Already documented in Pattern 9.

**Source:** [svelte.dev/docs/svelte/$state "Sharing state"]

## Code Examples

### Example A: `DeviceService::create` with audit log

```rust
// crates/trackly-app/src/services/device_service.rs
use std::sync::Arc;
use trackly_core::{
    error::AppError,
    domain::devices::{NewDevice, DeviceRow},
    ports::devices::DeviceRepository,
    primitives::clock::Clock,
};
use trackly_infra::{
    db::{writer_worker::WriterHandle, pools::ReaderPool},
    error_conversions::map_rusqlite,
    repos::SqliteDeviceRepository,
};
use crate::csv::session_store::ImportSessionStore;
use crate::dto::device::DeviceDto;

#[derive(Clone)]
pub struct DeviceService {
    writer: Arc<WriterHandle>,
    readers: Arc<ReaderPool>,
    clock: Arc<dyn Clock + Send + Sync>,
    repo: Arc<SqliteDeviceRepository>,         // stateless — could be a free fn module too
    csv_sessions: Arc<ImportSessionStore>,
}

impl DeviceService {
    pub fn new(
        writer: Arc<WriterHandle>,
        readers: Arc<ReaderPool>,
        clock: Arc<dyn Clock + Send + Sync>,
    ) -> Self {
        Self {
            writer, readers, clock,
            repo: Arc::new(SqliteDeviceRepository),
            csv_sessions: Arc::new(ImportSessionStore::new()),
        }
    }

    pub async fn create(&self, new: NewDevice) -> Result<DeviceDto, AppError> {
        // Validation in trackly-app (uses AppError::Validation).
        if new.name.trim().is_empty() {
            return Err(AppError::Validation {
                field: "name".into(),
                message: "Наименование обязательно".into(),
            });
        }
        // ... validate type_id, status_id, location_id ...

        let now = self.clock.unix_seconds();
        let repo = self.repo.clone();
        let new_clone = new.clone();
        let user_id_opt: Option<i64> = None; // Phase 4 will wire real session user

        let row: DeviceRow = self.writer.execute(move |conn| {
            let tx = conn.transaction().map_err(map_rusqlite)?;
            let id = repo.create(&tx, &new_clone, now)?;
            let after = repo.get(&tx, id)?;
            let after_json = serde_json::to_string(&after)
                .map_err(|e| AppError::Internal { source_chain: format!("audit: {e}") })?;
            tx.execute(
                "INSERT INTO audit_log (entity_type, entity_id, action, user_id, before_json, after_json, payload_json, created_at_utc) \
                 VALUES ('device', ?1, 'create', ?2, NULL, ?3, NULL, ?4)",
                rusqlite::params![id, user_id_opt, after_json, now],
            ).map_err(map_rusqlite)?;
            tx.commit().map_err(map_rusqlite)?;
            Ok(after)
        }).await?;

        Ok(DeviceDto::from(row))
    }
    // ... update, delete_soft, list, search, autocomplete, list_grouped, etc.
}
```

### Example B: Reader pool usage from async code

```rust
pub async fn list(
    &self,
    filter: DeviceFilter,
    pagination: Pagination,
) -> Result<(Vec<DeviceDto>, u64), AppError> {
    let readers = self.readers.clone();
    let repo = self.repo.clone();
    let rows_total: (Vec<DeviceRow>, u64) = tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        repo.list(&conn, &filter, &pagination)
    })
    .await
    .map_err(|e| AppError::Internal { source_chain: format!("spawn_blocking join: {e}") })??;
    let (rows, total) = rows_total;
    let dtos: Vec<DeviceDto> = rows.into_iter().map(DeviceDto::from).collect();
    Ok((dtos, total))
}
```

**Source:** [01-04-SUMMARY "Carry-forward notes — ReaderPool::acquire wrapped in spawn_blocking"]

### Example C: Autocomplete query (DISTINCT + partial index)

```rust
// crates/trackly-infra/src/repos/devices_sqlite.rs

pub fn autocomplete(
    &self,
    conn: &rusqlite::Connection,
    field: AutocompleteField,
    prefix: &str,
    ctx_name: Option<&str>,
) -> Result<Vec<String>, AppError> {
    let column = match field {
        AutocompleteField::Name => "name",
        AutocompleteField::Model => "model",
        AutocompleteField::Specs => "notes",            // map per V003 column name
        AutocompleteField::Complectation => "complectation",
        AutocompleteField::Condition => "condition",
        AutocompleteField::Location => return self.autocomplete_locations(conn, prefix, ctx_name),
    };

    let pattern = format!("{prefix}%");
    let (sql, params): (String, Vec<&dyn rusqlite::ToSql>) = match ctx_name {
        Some(name) => (
            format!(
                "SELECT DISTINCT {column} FROM devices \
                 WHERE deleted_at_utc IS NULL AND name = ?1 AND {column} LIKE ?2 \
                 ORDER BY {column} LIMIT 30"
            ),
            vec![&name, &pattern],
        ),
        None => (
            format!(
                "SELECT DISTINCT {column} FROM devices \
                 WHERE deleted_at_utc IS NULL AND {column} LIKE ?1 \
                 ORDER BY {column} LIMIT 30"
            ),
            vec![&pattern],
        ),
    };
    let mut stmt = conn.prepare(&sql).map_err(map_rusqlite)?;
    let rows = stmt
        .query_map(rusqlite::params_from_iter(params), |row| row.get::<_, String>(0))
        .map_err(map_rusqlite)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(map_rusqlite)
}
```

**Note:** Column whitelisting is enforced via the enum — never interpolate user input into the `{column}` placeholder, only enum-derived strings.

### Example D: Group-by SQL for non-unique devices

```sql
-- DeviceRepository::list_grouped
SELECT
  MIN(id)         AS repr_id,
  COUNT(*)        AS count,
  GROUP_CONCAT(id) AS member_ids,
  type_id, name, model, notes, complectation, condition, location_id, status_id
FROM devices
WHERE deleted_at_utc IS NULL
  AND (inventory_number IS NULL OR inventory_number = '')
  AND (serial_number    IS NULL OR serial_number    = '')
GROUP BY type_id, name, model, notes, complectation, condition, location_id, status_id
ORDER BY name, model
LIMIT ?1 OFFSET ?2
```
For UNIQUE devices (`inventory_number IS NOT NULL OR serial_number IS NOT NULL`) the regular `list` query is used and each row is its own "group of 1". UI toggles between grouped / flat views.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Svelte 4 stores (`writable`, `readable`) | Svelte 5 runes (`$state`, `$derived`) in `.svelte.ts` | Svelte 5 GA Oct 2024 | Eliminates `$store` subscription syntax; module-level `$state` is the canonical sharing pattern |
| `@tauri-apps/api/tauri` invoke import | `@tauri-apps/api/core` | Tauri 2 GA Oct 2024 | Path renamed; old path removed in Tauri 2 |
| `window.__TAURI__` runtime sniff | `window.__TAURI_INTERNALS__` | Tauri 2 GA | Old sentinel still present for back-compat but new code uses INTERNALS |
| SvelteKit `adapter-static` for SPA in Tauri | Vanilla Svelte 5 + svelte-spa-router | Confirmed Phase 1 STACK.md decision | One file `index.html` works in Tauri webview AND browser; no Kit boilerplate |
| `axum 0.7` `:id` path syntax | `axum 0.8` `{id}` path syntax | axum 0.8 release | All path captures must use `{...}` — old `:id` rejected at compile time |
| `tauri-plugin-fs` for file picker | `tauri-plugin-dialog` save/open dialogs | Tauri 2 plugin reorganization | fs is for read/write; dialog is for the native picker UX |

**Deprecated/outdated:**
- `chrono::Local::now()` — banned by clippy via `disallowed-methods` (use `time::OffsetDateTime::now_utc()` via `Clock` trait).
- `dirs::*_dir()` — banned by clippy (use `Paths::resolve()`).
- `tauri-plugin-updater` in portable mode — incompatible with portable; do not register.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `uuid` crate is already a workspace dependency | Standard Stack | If not, add `uuid = { version = "1", features = ["v4"] }` to workspace.dependencies — trivial fix; planner should verify by reading workspace Cargo.toml |
| A2 | `tauri-plugin-dialog` Rust crate version `2.x` matches `@tauri-apps/plugin-dialog` npm `2.7.1` family | Standard Stack | Plugins-workspace publishes Rust + JS together; mismatch unlikely but worth `cargo search tauri-plugin-dialog \| head -1` |
| A3 | Vite leaves inline `<script>` in `<head>` untouched | Pattern 5 | If Vite rewrites it (e.g., due to a new plugin), FOUC returns; fallback is `<script type="module">` in `<head>` which loads slightly later — still acceptable, just less-good FOUC suppression |
| A4 | `axum 0.8` path-capture syntax is `{id}` not `:id` | Pattern 11 | High confidence (axum 0.8 changelog explicit); first compile error catches it |
| A5 | `device_state_hints()` returning `Vec<&'static str>` works through specta::Type | Standard Stack | LOW confidence — `&'static str` may not have `Type`; safer fallback `Vec<String>` |
| A6 | `tauri-specta` rc.21 + `collect_commands!` accepts a long list of commands without rc-version-specific bugs | Composition | Tested in Plan 01-05 with 1 command; risk only at scale (>20 commands), Phase 2 is 12 — well within proven scale |
| A7 | `5-min TTL` for CSV preview is sufficient for typical user; D-CSV-01 specifies but doesn't justify | Pattern 7 | LOW — if users take 10+ minutes (rare), they'll retry preview; not a correctness issue |
| A8 | Russian Excel uses `;` as list separator in default locale config | Pitfall #5 | HIGH — well-documented Excel/Office behavior across decades |
| A9 | The `notes` column in V003 maps semantically to "Технические характеристики" (specs) from REQ-DEV-01 | Pitfall #1 | HIGH — V003 has no other text column suitable for spec; rename `notes → specs` is the cleanest path |
| A10 | `chardetng::EncodingDetector::guess(None, true)` will return UTF-8 for UTF-8 input and WINDOWS_1251 for CP1251 input in our specific use case | Standard Stack | MEDIUM — chardetng is heuristic; tested in Firefox but Russian text is the canonical use case; integration tests with both encodings give confidence |
| A11 | `tauri::Builder` construction in Phase 2 doesn't break Phase 1 `--self-test` path | Pitfall #4 | Need plan-level care: `--self-test` short-circuit must run BEFORE `tauri::Builder::default().run()` is invoked |

## Open Questions

1. **DTO field naming policy — Path A or Path B per Pitfall #1?**
   - **What we know:** V003 columns are `inventory_number / serial_number / condition / complectation / notes`. CONTEXT.md DTO sketch uses `inventory_no / serial_no / state / kit / specs`.
   - **What's unclear:** Whether the planner adopts the database names everywhere (Path A) or the domain names with serde rename (Path B).
   - **Recommendation:** Path A. Add a `V013` line `ALTER TABLE devices RENAME COLUMN notes TO specs;` (REQ-DEV-01 explicitly says "Технические характеристики"). All other columns keep their V003 names. Planner records this as a Phase-2 ADR in the first plan.

2. **Validation feedback timing (D-UI-Validation-01 doesn't pin):**
   - **What we know:** Manual validation via runes; server validates regardless.
   - **What's unclear:** Inline-on-blur vs. on-submit vs. live.
   - **Recommendation:** Hybrid — inline error on blur for individual fields (gives gentle feedback), full validation gate on submit (so users can interact freely without distracting error UI). Live (every keystroke) is too noisy for required-field checks.

3. **Multi-line `specs`/`notes` input (REQ-DEV-01 "Технические характеристики"):**
   - **What we know:** Text field, no character limit specified.
   - **What's unclear:** `<textarea>` rows? Auto-grow?
   - **Recommendation:** `<textarea rows="3">` with manual resize handle. Auto-grow adds JS complexity for marginal UX win.

4. **CSV fixture file generation — committed binary or build.rs?**
   - **What we know:** Need CP1251 + BOM variants in `tests/fixtures/devices/`.
   - **What's unclear:** Generate in `build.rs` (no binary in repo) or commit binary blobs.
   - **Recommendation:** Commit binary blobs (~1-2KB each). Reproducible, no build-time complexity, PR reviewers can verify with `file` and `iconv`. Add a README in `fixtures/devices/README.md` explaining how to regenerate (one line per file: `iconv -f UTF-8 -t WINDOWS-1251 utf8.csv > cp1251_comma.csv`).

5. **Where exactly to call `init_theme()` in main.ts?**
   - **What we know:** Must run after DOM ready, before first render of `<App>`.
   - **What's unclear:** Inside `App.svelte` `onMount` vs `main.ts` before `mount(App, ...)`.
   - **Recommendation:** In `main.ts` after `target` resolution, before `mount`. Ensures theme is fully applied before any component subscribes to `themeStore.resolved`.

6. **Sidebar placeholders — one shared `<Placeholder>` component or per-section?**
   - **What we know:** Sections like Dashboard, Карта, Принтеры, Картриджи show "Раздел в разработке".
   - **Recommendation:** One `<Placeholder section="Дашборд"/>` component; each route component is a one-liner `<Placeholder section="..." />`. Saves files and ensures consistent visuals.

7. **DeviceFormModal vs. inline DeviceForm component?**
   - **What we know:** REQ-DEV-01..02 specify CRUD with required fields.
   - **What's unclear:** Modal overlay vs. full-page form.
   - **Recommendation:** Modal — keeps the list visible behind, supports quick consecutive entries. Phase 3 (Акты) likely needs full-page forms; defer the pattern choice to that phase.

## Environment Availability

Phase 2 has no NEW environmental dependencies beyond Phase 1. All required tools (Rust 1.88, Cargo, pnpm 10.17.1, Node 20+) are already pinned in Phase 1. Section omitted.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Rust framework | `cargo test` (built-in `#[test]` + `#[tokio::test]`) |
| JS framework | `pnpm svelte-check` (type gate) + `pnpm lint` (eslint+prettier) — no JS unit test framework yet (out of Phase 2 scope; integration via real Tauri runtime tested manually + axum tests via `tower::ServiceExt::oneshot`) |
| Quick test command | `cargo test -p trackly-app --lib && cargo test -p trackly-app --test devices_crud` |
| Full suite command | `cargo test --workspace && pnpm svelte-check && pnpm lint` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| DEV-01 | CRUD with full field set | integration | `cargo test -p trackly-app --test devices_crud` | ❌ Wave 0 |
| DEV-02 | Required-field validation (4 fields) | integration | `cargo test -p trackly-app --test devices_crud -- validation` | ❌ Wave 0 |
| DEV-03 | Unique vs non-unique classification | integration | `cargo test -p trackly-app --test devices_grouping` | ❌ Wave 0 |
| DEV-04 | Seeded device types | unit | Already covered by Phase 1 `crates/trackly-infra/tests/seed_data.rs` | ✅ |
| DEV-05 | Seeded statuses | unit | Already covered by Phase 1 `crates/trackly-infra/tests/seed_data.rs` | ✅ |
| DEV-06 | FTS5 search across name/inventory/serial/model | integration | `cargo test -p trackly-app --test devices_search` | ❌ Wave 0 |
| DEV-07 | Status switch-bar w/ counts | integration | `cargo test -p trackly-app --test devices_crud -- list_with_status_counts` | ❌ Wave 0 |
| DEV-08 | Per-field autocomplete | integration | `cargo test -p trackly-app --test devices_autocomplete` | ❌ Wave 0 |
| DEV-09 | Contextual autocomplete (filter by selected name) | integration | `cargo test -p trackly-app --test devices_autocomplete -- contextual` | ❌ Wave 0 |
| DEV-10 | Static state-hints array | unit | `cargo test -p trackly-app --lib dto::device::tests::state_hints` | ❌ Wave 0 |
| DEV-11 | Group-by non-unique | integration | `cargo test -p trackly-app --test devices_grouping` | ❌ Wave 0 |
| DEV-12 | CSV import 4 encodings × 2 delimiters + per-row errors | integration | `cargo test -p trackly-app --test devices_csv_import` | ❌ Wave 0 |
| DEV-13 | CSV export UTF-8 BOM + `;` | integration | `cargo test -p trackly-app --test devices_csv_export` | ❌ Wave 0 |
| UI-01 | Sidebar with exact items + dividers | manual-only | Open dev build, eyeball | n/a |
| UI-02 | Theme switcher (3 options) + persistence + no-flash | manual + smoke | Reload page, observe no flash; `localStorage.setItem('trackly:theme','dark')` then reload | n/a |
| UI-03 | 100% Russian strings | smoke | `grep -rn '[a-zA-Z]\{4,\}' ui/src/features ui/src/lib/components -- *.svelte` should be empty of user-facing English | n/a |
| UI-04 | 1280×720 layout works | manual | Resize window | n/a |
| UI-05 | Same bundle runs in Tauri + browser; transport dispatches via `isTauri` | integration (Phase 2 partial) | `pnpm build && grep -q "__TAURI_INTERNALS__" dist/assets/*.js` | n/a (Phase 5 binds browser path) |
| UI-06 | Toast on AppError | integration (UI) | Manual: trigger validation error, observe toast | n/a |

### Sampling Rate

- **Per task commit:** `cargo test -p trackly-app --lib` (~2-5s once warm) + the specific integration test for that task.
- **Per wave merge:** `cargo test --workspace --no-fail-fast` + `pnpm svelte-check` + `pnpm lint` (~30-60s).
- **Phase gate:** Full suite green AND `cargo run -p trackly-app -- --self-test` still passes AND a manual launch via `pnpm tauri dev` (once wired) shows the device-create flow end-to-end.

### Wave 0 Gaps

- [ ] `crates/trackly-core/src/ports/mod.rs` + `ports/devices.rs` — new module
- [ ] `crates/trackly-core/src/domain/mod.rs` + `domain/devices.rs` — new module
- [ ] `crates/trackly-infra/src/repos/mod.rs` + `repos/devices_sqlite.rs` — new module
- [ ] `crates/trackly-app/src/services/mod.rs` + `services/device_service.rs` — new module
- [ ] `crates/trackly-app/src/csv/{mod,sniff,decode,parse,session_store}.rs` — new module
- [ ] `crates/trackly-app/src/dto/device.rs` — new file
- [ ] `crates/trackly-app/src/tauri_cmds/devices.rs` — new file
- [ ] `crates/trackly-app/src/http/devices.rs` — new file
- [ ] `crates/trackly-app/tests/devices_*.rs` — 6 new integration test files
- [ ] `crates/trackly-app/tests/fixtures/devices/*` — 5 CSV fixture files (committed binary)
- [ ] `migrations/V013__devices_fts_triggers.sql` — new file
- [ ] `ui/src/lib/{api,stores,components,utils}/*` — many new files
- [ ] `ui/src/features/{layout,devices}/*` — many new files
- [ ] `ui/src/pages/*.svelte` — 10 placeholder + 1 NotFound
- [ ] `ui/src/routes.ts`, `ui/index.html` (rewritten head), `ui/src/App.svelte` (rewritten)
- [ ] `ui/package.json` — add 3 dependencies, run `pnpm install`, commit lockfile

## Security Domain

Security enforcement is enabled (Phase 1 invariant — `Secret<T>` discipline, no AD passwords stored, AppError unified shape, etc.). Phase 2 adds zero auth surface (no login flow yet — Phase 4 handles USR-*), but several ASVS categories still apply:

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | NO — Phase 4 | — |
| V3 Session Management | NO — Phase 4 | — |
| V4 Access Control | PARTIAL — DEV-* commands accept all callers in Phase 2; audit_log records `user_id=NULL`. Phase 4 wires real authorization. | Phase 2 audit-log invariant: every CRUD writes an audit row (defense-in-depth even without auth) |
| V5 Input Validation | YES | trackly-app service layer validates required fields → `AppError::Validation`; SQL is parameterized via rusqlite; FTS5 user input is escaped + quoted (Pitfall #3) |
| V6 Cryptography | NO direct use | — (Phase 4 adds argon2id; Phase 5 adds rustls) |
| V7 Error Handling | YES | `AppError` 9-variant unified shape with stable `code` field; Russian `message` field — no raw stack traces escape to UI (`Internal { source_chain }` is logged at INFO level, never serialized verbatim to client) |
| V8 Data Protection | YES — soft-delete + audit log | `deleted_at_utc IS NOT NULL` for soft delete; FTS triggers de-index soft-deleted rows; audit_log retains before/after JSON |
| V12 File and Resources | YES — CSV upload | File size limit on import (recommend 50MB cap in Tauri command; reject larger with `AppError::Validation`); content-type validation via encoding sniff (chardetng) |
| V13 API and Web Service | YES | snake_case JSON contract documented; specta-generated `bindings.ts` is the typed contract |

### Known Threat Patterns for Trackly Phase 2

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| SQL injection via dynamic column in autocomplete | Tampering | Enum-whitelisted column name; never interpolate user string into SQL identifier position |
| FTS5 injection via free-text search | Tampering | Quote each token + escape `"` per Pattern 7 — query is data, not code |
| CSV CSRF / file-content attack | Tampering | Preview-then-commit token gates execution; UUID v4 token not enumerable; 5-min TTL limits replay window |
| Memory DoS via huge CSV preview | DoS | File size cap at command boundary (50MB); per-session map kept small via single-use token + lazy sweep |
| Audit log gaps | Repudiation | Audit row written in SAME transaction as mutation — never can have mutation without log |
| Optimistic-lock silent overwrite | Tampering | `OptimisticLockMismatch` surfaced to UI; auto-retry rejected explicitly (Pattern 3) |
| Cyrillic/Latin homograph in device names | Spoofing | Out of scope — surface as Phase 7 data-quality audit |
| Theme localStorage tampering (XSS payload in theme value) | Tampering | Theme value validated to one of `'light' | 'dark' | 'system'` before applying — invalid value falls back to `'light'` |

## Sources

### Primary (HIGH confidence)
- [Phase 1 SUMMARY 01-04](/.planning/phases/01-foundation/01-04-SUMMARY.md) — AppError, WriterHandle, ReaderPool, AppCtx, error_conversions, test_writer_and_readers fixture
- [Phase 1 SUMMARY 01-05](/.planning/phases/01-foundation/01-05-SUMMARY.md) — HealthDto pattern, build_health helper, specta_export::builder, sibling-marker for AppError, logging::init
- [Phase 1 VERIFICATION](/.planning/phases/01-foundation/01-VERIFICATION.md) — 19/19 must-haves passed; what's already proven
- [Phase 1 deferred-items.md](/.planning/phases/01-foundation/deferred-items.md) — items Phase 2 must close
- [.planning/research/ARCHITECTURE.md](/.planning/research/ARCHITECTURE.md) — hexagonal layout, dual transport, write-pool pattern
- [.planning/research/STACK.md](/.planning/research/STACK.md) — pinned versions
- [.planning/research/PITFALLS.md](/.planning/research/PITFALLS.md) — top 15 pitfalls
- [migrations/V003__devices.sql](/migrations/V003__devices.sql) — actual column names (NOT what CONTEXT.md assumes)
- [migrations/V012__indexes_and_fts.sql](/migrations/V012__indexes_and_fts.sql) — FTS5 declared with content='devices'
- [Tauri 2 invoke docs](https://v2.tauri.app/develop/calling-rust/) — confirmed import path `@tauri-apps/api/core`, error reject semantics
- [Svelte 5 $state docs](https://svelte.dev/docs/svelte/$state) — confirmed module-level export pattern + .svelte.ts convention
- [docs.rs/chardetng/0.1.17](https://docs.rs/chardetng/0.1.17) — EncodingDetector API confirmed
- [docs.rs/encoding_rs/0.8](https://docs.rs/encoding_rs/0.8) — WINDOWS_1251 static + decode() tuple shape confirmed
- [docs.rs/csv/1.3](https://docs.rs/csv/1.3) — ReaderBuilder delimiter + from_reader confirmed
- [SQLite FTS5 §4.4.3 External Content](https://sqlite.org/fts5.html#external_content_tables) — trigger delete-form syntax
- [GitHub ItalyPaleAle/svelte-spa-router](https://github.com/ItalyPaleAle/svelte-spa-router) — README confirmed routes map + use:link + use:active for Svelte 5 in 5.1.0

### Secondary (MEDIUM confidence)
- [`npm view` 2026-05-25] — @tauri-apps/api 2.11.0, @tauri-apps/plugin-dialog 2.7.1, svelte-spa-router 5.1.0 — all verified live
- [Tauri Discussion #6119](https://github.com/tauri-apps/tauri/discussions/6119) — `__TAURI_INTERNALS__` is the v2 sentinel
- [CLAUDE.md](/CLAUDE.md) — stack constraints + "what NOT to use"

### Tertiary (LOW confidence — flagged in Assumptions Log)
- Assumption A5 (`Vec<&'static str>` and `specta::Type`) — needs first-build verification, fallback `Vec<String>` always works
- Assumption A10 (chardetng correctly classifies CP1251 vs UTF-8 for short Russian samples) — confirm via integration test on real fixtures

## Project Constraints (from CLAUDE.md)

Locked constraints the planner MUST verify in every plan:

- **Tauri 2.x, Svelte 5.x, SCSS, SQLite, Rust** — fixed by user (CLAUDE.md "Constraints")
- **Portable mode mandatory** — no `dirs::*_dir()`, no `%APPDATA%` writes. Already structurally enforced by clippy `disallowed-methods` (Phase 1) — Phase 2 must not bypass.
- **`Secret<T>` for sensitive data** — Phase 2 has no auth/secret surface but must not introduce plaintext password fields anywhere.
- **SQLite WAL + single-writer** — every Phase 2 write goes through `WriterHandle::execute`. No `Connection::open` outside infra::db/test_support/context::build.
- **Russian-only UI in v1** — hard-code Russian strings; defer i18n. Backend AppError.message is already Russian.
- **GitHub CI gates** — `cargo clippy -- -D warnings`, `cargo test`, `cargo fmt --check`, `pnpm svelte-check`, `pnpm lint` must stay green. Phase 2 REMOVES `continue-on-error: true` from svelte-check (D-Cleanup-01).
- **snake_case JSON** in DTOs (Phase 1 Plan 05 invariant — applies to all Phase 2 DTOs).
- **GSD Workflow Enforcement** — every Phase 2 plan executes via `/gsd-execute-phase`, never bypassed for direct edits.
- **No project skills directory exists yet** — no skill rules to load.
- **MSRV 1.88** (per Phase 1 Plan 01-01 deviation log) — Phase 2 must not require a newer Rust feature.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — every crate/package version verified via authoritative source this session
- Architecture patterns: HIGH — directly extends Phase 1 patterns with one ADR-level decision (DTO field naming) flagged
- Pitfalls: HIGH — top three (V003 column mismatch, FTS5 external-content delete syntax, Tauri capability declaration) are the most likely to bite; rest are well-documented elsewhere
- CSV pipeline: MEDIUM-HIGH — chardetng + encoding_rs + csv well-understood; only LOW area is real-world malformed CSV behavior (covered by per-row error pattern in D-CSV-01)
- UI scaffolding: MEDIUM — Svelte 5 runes API is stable but module-level `$state` reassignment rules and `.svelte.ts` filename convention have caught teams; documented in pattern + pitfall

**Research date:** 2026-05-25
**Valid until:** 2026-06-24 (30 days for stable stack; revisit if a Svelte 5.6+ or Tauri 2.12+ release brings API changes)

# Project Research Summary

**Project:** Trackly — учёт и отслеживание техники, принтеров и картриджей
**Domain:** Self-hosted, single-org IT-asset / cartridge / printer tracker — Russian-language, Windows-AD environment, portable desktop + LAN browser hybrid
**Researched:** 2026-05-24
**Confidence:** HIGH overall (table-stakes feature set is well-documented in Snipe-IT/GLPI/ServiceDesk Plus; Rust + Tauri 2 + Svelte 5 + SQLite stack is mature; portable-mode and Cyrillic pitfalls are well-understood); MEDIUM for Pantum spooler-hang detection (vendor SNMP support is shallow, requires host-side cross-check); MEDIUM-LOW for PDF Cyrillic engine choice (decision deferred to a spike) and Windows 7 32-bit feasibility (treat as best-effort).

## Project at a Glance

Trackly is the **opinionated portable replacement for Snipe-IT / GLPI for a small Russian organization**: it ships as one Tauri 2 executable that doubles as a LAN HTTP server, stores everything in a single SQLite (WAL) file beside the .exe, and natively speaks РФ-conventions — инвентарные номера, акты приёма-передачи with sub-numbered partial returns ("№42 в1", "№42 в2"), редактируемые шаблоны печатных форм, и заправляемые картриджи. The product is built to one-and-only-one organization, runs on Windows + macOS + Linux from one CI matrix, and reaches an AD domain only for optional bind-based authentication.

The recommended approach is a **hexagonal-core Rust workspace** (3 crates: `trackly-core` / `trackly-infra` / `trackly-app`) so that Tauri commands and `axum` HTTP handlers are thin transport adapters around the same service layer; a Svelte 5 SPA detects its transport at runtime and works in both the Tauri webview and any LAN browser; SQLite is accessed through a **split read/write connection pattern with a single dedicated writer task** (the most important architectural pattern in the whole project — both ARCHITECTURE.md and STACK.md converge on this even though they suggested different drivers). Phase 1 must be a foundation phase that nails the schema, write-path discipline, portable path resolution, and a small set of cross-cutting invariants (audit log, soft delete, optimistic locking, UTC-only timestamps) — retrofitting any of these later is much more expensive than putting them in on day one.

The key risks are not in the stack — they are in the **operational seams**: portable mode silently leaking into `%APPDATA%` (especially WebView2's `EBWebView` cache), Pantum BM5100ADN spooler hangs that SNMP cannot detect from the device side, Cyrillic glyphs missing from PDFs because the default PDF library is Windows-1252, and AD bind code that "works" against a dev mock but fails against a real Windows Server 2022 with channel binding enforced. The mitigation pattern is the same in all four cases: enforce the discipline structurally in Phase 1 (env vars, traits, embedded fonts, `Secret<T>` newtypes) rather than relying on devs to remember it in a feature phase.

## Key Findings

### Stack (final recommendations after resolving tensions)

**Single source of truth: `rusqlite 0.39` + `refinery 0.8` + split read/write pools, single dedicated writer task.**
This resolves the divergence between STACK.md (rusqlite, citing sqlx-sqlite's lock-starvation footgun) and ARCHITECTURE.md (sqlx with split pools). The split-pool *pattern* from ARCHITECTURE.md is correct and required; the *driver* should be `rusqlite` per STACK.md because the sqlx-sqlite write-transaction footgun is a documented production hazard. `rusqlite` connections wrap cleanly inside `tokio::task::spawn_blocking` and the writer-pool-of-one becomes structural rather than accidental. `refinery` embeds migrations into the binary (portable-friendly).

**Core technologies (locked):**
- **Tauri 2.x** (`tauri ^2.11`) — desktop shell with capability/ACL model, NSIS + MSI bundlers, plugin v2 ecosystem; Tauri v1 is EOL.
- **Svelte 5.x** with runes (`$state`, `$derived`) — runes scale to the app's cross-cutting state (filters, role context, switch-bars) far better than Svelte 4 stores.
- **Vanilla Svelte 5 SPA, no SvelteKit** — the same `index.html` must boot in both the Tauri WebView2 and any LAN browser; SvelteKit's router and SSR conventions add cost without value here.
- **Vite 6** — Tauri's default; HMR works inside the webview.
- **SCSS** via `vitePreprocess` with shared design tokens auto-injected.
- **axum 0.8** on **tokio 1.x** (multi-thread, single runtime owned by Tauri) — Tower middleware ecosystem is the deciding factor; LAN traffic at 20 concurrent users is not where raw throughput matters.
- **rusqlite 0.39** (`bundled`, `chrono`, `serde_json`, `backup` features) — bundled SQLite ships portable, no DLL hunt.
- **refinery 0.8** — embedded forward-only migrations; compile-time `embed_migrations!`.
- **tower-sessions 0.13** with a hand-rolled rusqlite-backed `SessionStore` (~80 LoC) — cookie sessions are simpler and revocable; JWT is the wrong tool for a single-process LAN server.
- **snmp2 0.4** with `crypto-rust` (no OpenSSL dependency) — v1/v2c/v3 support with sync + async sessions.
- **ldap3 0.12** with `tls-rustls` — simple bind for AD; channel binding via `tls-server-end-point` only over TLS.
- **argon2 0.5** (argon2id, m=19456, t=2, p=1) — OWASP 2024+ defaults for local passwords.
- **rustls 0.23** + **rcgen 0.13** — pure-Rust TLS, no OpenSSL drag-along; self-signed cert generation on first server-mode launch.
- **tracing + tracing-subscriber + tracing-appender** — structured logging with daily rotation into `./logs/` beside the executable.
- **MSRV: Rust 1.85** — leaves the NTLM door open in `ldap3` for an eventual SSO milestone.

**Pinned versions that matter:** `tauri 2.11`, `wry 0.55`, `tao 0.35`, `axum 0.8`, `tokio 1.x`, `rusqlite 0.39`, `refinery 0.8`, `svelte 5.55+`, `vite 6.x`, `krilla 0.7` (MSRV 1.92).

### Features (table stakes + recommended additions to v1)

PROJECT.md's feature scope is **strongly aligned with the industry table stakes** captured by Snipe-IT, GLPI, ManageEngine ServiceDesk Plus, and Lansweeper. The product is at the intersection of Snipe-IT (clean check-out/check-in), GLPI (first-class printer/cartridge modules with SNMP), and ServiceDesk Plus (lifecycle states), with two genuine differentiators on top: **Russian-native печатные формы and партиционные возвраты ("N в1", "N в2")**, and the **portable-binary + LAN-server toggle** that replaces a LAMP stack for orgs that can't run one.

**Must have (table stakes, in spec):**
- Devices CRUD with инвентарный/серийный №, lifecycle statuses (На складе / В работе / На ремонте / Списано), full-text search (SQLite FTS5), CSV import/export.
- Acts CRUD with sequential auto-suggested numbering + override, partial returns with sub-numbering, archive on full return, printable PDF with editable templates.
- Cartridges with separate Модели и Экземпляры, two orthogonal lifecycle dimensions (charge state + location/state), low-stock banner.
- Printers with subnet discovery and SNMP polling (toner, status, page count) for Pantum/Kyocera/HP/Canon.
- Requests workflow (created → in progress → done/rejected), three roles (Admin / Specialist / Employee), local-auth-first with AD bind as a later phase.
- Editable document templates stored in DB (portable with the binary), organization branding (logo + реквизиты), backup (manual + scheduled), portable + LAN-server modes.

**Recommended additions to v1 — gaps in PROJECT.md that should be closed in Phase 1's schema (not retrofitted later):**
- **Audit log** (`activity_log(entity_type, entity_id, action, user_id, before_json, after_json, created_at)`) — required to answer "who changed this?" and to make the "undo return restores prior state" feature actually undoable. Write-side first, UI later.
- **Soft delete** (`deleted_at` column) on Acts, Devices, Cartridges — cheap to add now, expensive to bolt on later; gives a "Корзина" feature post-MVP.
- **Optimistic concurrency** (`version` column) on Acts, Devices, Cartridges — without it the 20-concurrent-user LAN scenario produces silent overwrites.
- **`assigned_to` denormalized field** on Device — answers "у кого сейчас?" directly on the device card; updated transactionally with act create/return.
- **Device types & statuses as tables, not enums** — every researched competitor learned this lesson; through customer feedback "Монитор", "Сетевое оборудование", "Утеряно" will appear within a year. Cheap to seed-load, expensive to enum-then-migrate.
- **Department/Подразделение справочник** — РФ-specific, will be requested.

**Should have (post-MVP, v1.1–v1.3):** SNMP printer monitoring + Pantum hang detection (alert-only, no auto-fix), browser self-service for сотрудников, отчёты, in-app notifications + SMTP email, custom fields, корзина UI on top of soft delete.

**Defer (v2+):** AD bind + auto-registration requests, Pantum auto-restart spooler, Telegram + webhook outputs, signature pad для актов, REST API для интеграций, печать этикеток с QR-кодами, карта помещений (explicitly out of scope in PROJECT.md).

### Architecture (layering, dual transport, portable mode discipline)

Three-crate Rust workspace with strict dependency rules:

```
trackly/
├── crates/
│   ├── trackly-core/      # domain + ports (traits) + services. NO tokio, NO rusqlite. Pure logic.
│   ├── trackly-infra/     # adapters: SqliteRepos (rusqlite), LdapAdClient, Snmp2Client, Mailer, mocks/
│   └── trackly-app/       # bin: Tauri shell + axum router + AppCtx + background tasks + bindings.ts
└── ui/                    # Svelte 5 SPA, dual transport (Tauri invoke OR fetch + cookie)
```

The **cardinal rule**: `#[tauri::command]` and axum handler functions are both 5–15 line transport adapters. They decode input, call a single service method on `AppCtx`, encode output. No business logic lives in either transport layer. The frontend's `transport.ts` detects `window.isTauri === true` (Tauri 2.0-beta.9+) or `'__TAURI_INTERNALS__' in window` once at startup and routes every call through one wrapper.

**Database access pattern (the most important architectural decision):**
- **One writer task** owns one `rusqlite::Connection`, runs inside `tokio::task::spawn_blocking`, receives jobs via `mpsc`. All writes — from Tauri commands, axum handlers, and background workers — flow through this single channel.
- **Reader pool** of 4 connections served via `r2d2` or a hand-rolled `Arc<Mutex<Vec<Connection>>>`. Reads do not contend with writes under WAL.
- **PRAGMAs at connection open**: `journal_mode=WAL`, `busy_timeout=5000`, `synchronous=NORMAL`, `foreign_keys=ON`, `wal_autocheckpoint=1000`, `temp_store=memory`, `mmap_size=128MB`.
- **Migrations run on the write pool only**, before any reader is opened or any handler accepts a request.
- **No DB on network shares** — WAL requires shared memory between processes; SMB corrupts it. Reject in the Settings UI.

**Typed RPC via `tauri-specta v2`** — every DTO derives `serde::{Serialize, Deserialize}` + `specta::Type`; `bindings.ts` is generated at build time (or in a `cargo test` step); the same DTOs are used by axum (`Json<NewDeviceDto>` → `Json<DeviceDto>`) so the TS types are the contract for both transports. (Do not use `ts-rs` — it exports types individually and doesn't follow transitive dependencies.)

**Portable mode discipline:**
- `WEBVIEW2_USER_DATA_FOLDER` env var set to `<exe_dir>/data/webview` **before** `tauri::Builder::default()` runs in `main()`. Otherwise WebView2 writes to `%LOCALAPPDATA%\Trackly\EBWebView` regardless of any other setting.
- All path resolution flows through one `paths.rs` module rooted at `std::env::current_exe()?.parent()?`. Sentinel: presence of `portable.txt` or `trackly.config.json` next to the .exe forces portable mode.
- Ban `dirs::*_dir()`, `app.path().app_data_dir()`, and `&str`-typed paths via custom clippy `disallowed-methods` lint. Always pass `&Path` / `PathBuf`.
- Disable `tauri-plugin-updater` in portable variant. (Optionally enable it for a separate NSIS-installer variant.)
- Add a CI integration test that runs the app under Process Monitor in a sandbox temp folder and asserts zero writes outside `<exe_dir>`.

**File layout next to the executable:**
```
<exe_dir>/
├── trackly.exe
├── portable.txt                  # sentinel
├── trackly.config.json
├── trackly.db (+ .db-wal, .db-shm)
├── data/webview/                 # WebView2 user data
├── logos/, templates/, backups/, logs/, certs/
```

**Single tokio runtime** owned by Tauri. axum is `tokio::spawn`'d from `tauri::Builder::setup`. Background workers (SNMP poll, backup, low-stock checker, alert dispatcher) all live as spawned tasks coordinated by one `CancellationToken` stored on `AppCtx`. CPU-bound operations (PDF render, CSV parse of 5000 rows) use `tokio::task::spawn_blocking`.

**Dual transport, identical authorization:** Both `#[tauri::command]` and axum handlers call the same `authorize(user, Permission)` function before doing work. Tauri capabilities prevent the *frontend* from calling commands but do **not** apply to axum endpoints; UI-only role checks are a security bug. Every mutating endpoint gets a curl-based "Specialist returns 403" test in CI.

### Pitfalls (top 7 most critical, with prevention)

1. **Portable mode silently leaks into `%APPDATA%` (Pitfall #1).** WebView2's `EBWebView` cache and Tauri's `BaseDirectory::AppData` both default to system paths. *Prevention:* set `WEBVIEW2_USER_DATA_FOLDER` before `tauri::Builder`; ban `dirs::*_dir()`, `app.path().app_data_dir()` via clippy lint; ProcMon-based CI test on Windows runner. **Phase 1.**

2. **SQLite "database is locked" under server-mode concurrency (Pitfall #2).** WAL only handles reader-writer, not writer-writer; a long write tx (e.g., CSV import) blocks every other writer. *Prevention:* single-writer pool (size 1) + reader pool (size 4) at the application layer; `BEGIN IMMEDIATE` for write txs; `busy_timeout=5000`; no I/O inside a tx; optimistic concurrency via `version` column; refuse DB on SMB. **Phase 1.**

3. **Pantum BM5100ADN spooler hangs are invisible to SNMP (Pitfall #3).** The hang is on the Windows print spooler service; the printer itself returns `idle(3)` over SNMP while jobs are queued and stuck. *Prevention:* combine signals — `Win32_PrintJob` count on the print host (local agent or remote WMI/RPC) as primary; `prtMarkerLifeCount` not advancing over 5 min as confirmation; TCP 9100 + ICMP as device-alive sanity. **Alert-only in v1; auto-fix is a later phase.** Choice between local agent and remote WMI is a Phase-printer-monitoring spike.

4. **PDF generation with Cyrillic produces empty glyph boxes (Pitfall #7).** `genpdf` is Windows-1252; `printpdf` requires explicit font embedding; subsetting can silently drop Cyrillic codepoints. *Prevention:* embed a Cyrillic-capable TTF (DejaVu Sans, PT Sans) via `include_bytes!`; render through `krilla 0.7` (primary) or `typst-as-lib` (alternative); CI test that hashes a fixture PDF rendered with «Сидоров-Петроградский Иван Александрович (ё) №42». **First PDF phase.**

5. **Backend authorization gap — UI-only role checks (Pitfall #5).** Tauri's capability allowlist does not apply to axum endpoints; the same business operation is exposed via two transports and easy to authorize once and forget the other. *Prevention:* single `authorize(user, Permission)` function called from every Tauri command **and** every axum handler; curl-based role × endpoint matrix test in CI; UI hiding is UX only, never security. **Auth/roles phase, never retrofitted.**

6. **AD bind works in dev mock, fails against real Windows Server 2022 (Pitfall #4).** Post-2023 hardening enforces LDAP channel binding and signing; corp internal CA rejected by `native-tls`; `sAMAccountName` cannot be used as a bind DN. *Prevention:* bind via UPN (`us100@corp.local`); always `ldaps://636` + rustls; configurable conn_timeout (default 5s); discover DCs via DNS SRV records; **never store the AD password** (`Secret<String>` newtype with `Drop` zeroize, bind-and-discard). Phase 1 establishes `Secret<T>` discipline; AD bind itself is a late phase.

7. **Cyrillic Windows paths silently break file APIs (Pitfall #6).** `C:\Документы\Учёт\Trackly\` works in some crates and not others; `.to_str()` on a non-UTF-8 `OsStr` returns `None`. *Prevention:* always `&Path` / `PathBuf` (clippy lint bans `.to_str().unwrap()` on paths); add `activeCodePage=UTF-8` to Windows manifest; CI Windows job tests with a Cyrillic install path from day 1. **Phase 1.**

**Cross-cutting Russian-locale traps to set policy on in Phase 1:**
- **UTC-only timestamps in DB** (`chrono::Utc`, never `chrono::Local`); organization TZ in Settings; `chrono-tz` (audited quarterly) for display. Russia abolished DST in 2014 and several regions changed offsets in 2014 — historical reports break if `Local` ever sneaks in. Make `chrono::Local::now()` a clippy denial.
- **CSV with UTF-8 BOM + `;` delimiter** for Russian Excel; encoding sniffing (BOM → UTF-8 strict → CP1251 fallback) on import; preview rows before commit.
- **Secret newtype** for all sensitive fields (AD password, SNMP community, future SMTP password) with custom `Debug` printing `***` to avoid log leakage.

## Recommended Phase 1 Schema Must-Haves (consolidated checklist)

Phase 1 is foundation — the schema and write-path discipline. Everything below is cheap to put in now and expensive (sometimes month-of-rewrite expensive) to add later. This list reconciles requirements from FEATURES.md, ARCHITECTURE.md, and PITFALLS.md.

**Schema / tables:**
- [ ] `device_types(id, code, label_ru, sort_order)` and `device_statuses(id, code, label_ru, sort_order)` — as **tables with seed rows**, never as Rust enums. (FEATURES.md cross-ref + ARCHITECTURE.md domain types.)
- [ ] `locations` (or equivalent autocomplete-source table) — справочник behind the Расположение field.
- [ ] `activity_log(id, entity_type, entity_id, action, user_id, before_json, after_json, created_at_utc)` — write-side enabled from day one; UI is later. Required for "undo return" to actually restore state.
- [ ] `acts` with `parent_act_id NULLABLE` + `sub_number INTEGER NULL` for partial returns ("N в1", "N в2"). Unique index on `(parent_act_id, sub_number)`.
- [ ] `cartridges` with `cartridge_seq INTEGER NOT NULL UNIQUE` for the numeric part of `C-000001`; format on display.
- [ ] Two orthogonal columns on `cartridges`: `charge_state` (full/partial/empty) and `lifecycle_state` (in_stock/in_use/refilling/disposed). UI may collapse them; schema must keep them separate.
- [ ] `devices.assigned_to_id` denormalized + updated transactionally with act create/return — answers "у кого сейчас?" without a join.
- [ ] `users` with `source` enum (`local` | `ad`) and `password_hash TEXT NULL` (NULL for AD users); **never** any plaintext or recoverable password column.
- [ ] `document_templates(id, kind, content, version, is_default, created_at_utc)` — row-per-version model so customized templates survive default-template upgrades.
- [ ] `sessions` table for `tower-sessions` backend (so server-mode survives restart).
- [ ] `scheduled_tasks(id, kind, next_run_at_utc, last_run_at_utc, payload_json)` — for the backup + low-stock + SNMP-poll-scheduling supervisor.
- [ ] `counters` table for sequential numbering (acts.number, etc.) with `UPDATE … RETURNING` for atomic increment under `BEGIN IMMEDIATE`.

**Per-record invariants on all writable entities:**
- [ ] `deleted_at_utc INTEGER NULL` (soft delete) — Acts, Devices, Cartridges minimum.
- [ ] `version INTEGER NOT NULL DEFAULT 1` (optimistic concurrency lock) — increment on every update; reject on mismatch with a 409-style error.
- [ ] All timestamps are `INTEGER` (Unix seconds) or `TEXT` ISO-8601 in **UTC only**. Never local time.

**Connection / pool / pragma discipline:**
- [ ] One `rusqlite::Connection` writer task owning a `tokio::sync::mpsc` job queue (`tokio::task::spawn_blocking`).
- [ ] Reader pool of 4 connections.
- [ ] On every open: `journal_mode=WAL`, `busy_timeout=5000`, `synchronous=NORMAL`, `foreign_keys=ON`, `wal_autocheckpoint=1000`, `temp_store=memory`.
- [ ] Migrations via `refinery::embed_migrations!()` run on the write pool **before** any reader opens or any handler accepts a request.
- [ ] Refuse to start if DB path resolves to a UNC / SMB share; refuse if `current_exe().parent()` is unwritable in portable mode.

**Schema versioning + restore safety:**
- [ ] `PRAGMA user_version` set by every migration; pre-flight check on startup refuses to open a DB whose `user_version` is higher than the binary knows about (no silent downgrade).
- [ ] All backups via `rusqlite::backup::Backup` or `VACUUM INTO` — never `std::fs::copy`. Run `PRAGMA integrity_check` on the backup file after write; mark bad backups, keep the prior good one.

**Cross-cutting Rust-level invariants:**
- [ ] `Secret<T>` newtype with custom `Debug = "***"` and `Drop` zeroization — used for AD passwords (future), SNMP community strings, SMTP credentials.
- [ ] `paths.rs` is the single source of truth for every disk path; rooted at `std::env::current_exe()?.parent()?`; sentinel-file (`portable.txt` / `trackly.config.json`) selection.
- [ ] `WEBVIEW2_USER_DATA_FOLDER` set in `main()` first line, before `tauri::Builder::default()`.
- [ ] Clippy `disallowed-methods` list: `dirs::*_dir()`, `app.path().app_data_dir()`, `chrono::Local::now()`, `.to_str().unwrap()` on paths.
- [ ] `AppError` defined once, `Serialize` for Tauri + `IntoResponse` for axum, identical JSON shape in both transports.

## Resolved Decisions

These are tensions where the four researchers diverged or surfaced a choice that the roadmapper needs answered with a clear direction. The chosen direction is opinionated and final unless a Phase spike overturns it.

| Tension | Resolution | Rationale |
|---------|------------|-----------|
| **SQLite driver: rusqlite vs sqlx** (STACK.md said `rusqlite` + `refinery`; ARCHITECTURE.md showed `sqlx` with split read/write pools) | **`rusqlite 0.39` + `refinery 0.8` + the split-pool pattern from ARCHITECTURE.md.** Writer is a dedicated `tokio::task::spawn_blocking` task owning one connection + an `mpsc` job queue. Reader pool of 4 via `r2d2_sqlite`. | The split-pool *pattern* is correct regardless of driver and is the most important DB decision. The *driver* choice goes to `rusqlite` because sqlx-sqlite has a documented write-transaction lock-starvation footgun (read tx that touches a write upgrades the lock and blocks all other writers), and `rusqlite` makes the writer-singleton structural rather than accidental. `refinery` embeds migrations into the binary for portable distribution. Cost: a tiny custom `tower-sessions::SessionStore` impl (~80 LoC) since `tower-sessions-sqlx-store` doesn't apply. |
| **PDF engine for Cyrillic: `krilla` vs Typst-as-lib** | **`krilla 0.7` is the default.** Embed DejaVu Sans / PT Sans via `include_bytes!`. A small Typst-as-lib spike happens during the first PDF phase (Phase "Акты приёма-передачи"); if templates need designer-editable markup or krilla's Cyrillic round-trip has unexpected issues, the spike outcome flips the default to `typst-as-lib`. | Both researchers flagged this as MEDIUM-LOW confidence. krilla has the better OpenType/subsetting story in 2026 and is Rust-native; Typst is more powerful for end-user-editable templates but raises the bar on the template-safety story (Pitfall #14). Decide with a real fixture render of «Сидоров-Петроградский (ё) №42», hashed in CI. |
| **"Расходник" device type vs separate Картриджи section** (FEATURES.md flagged the conflict) | **Drop "Расходник" from `device_types`.** Device types are seeded as **Устройство (default), Принтер** only (plus "Монитор", "Сетевое оборудование", "Утеряно" added later through the seedable table). Cartridges live entirely in their own section with their own model and lifecycle. Other consumables (paper, etc.) — if they ever need tracking — go through the non-unique device pattern with a count, not through a separate type. | Snipe-IT and GLPI both learned that Assets vs Consumables have divergent lifecycles (consumable is depleted, asset is returned). PROJECT.md already builds a dedicated Картриджи section — having "Расходник" inside device types creates a confused boundary the user won't navigate ("куда заводить тонер?"). Cartridges are first-class; "Расходник" type is the wrong abstraction. **This change should be reflected back into PROJECT.md at the next /gsd-transition.** |
| **Phase 1 schema must-includes consolidation** | See the [Recommended Phase 1 Schema Must-Haves checklist](#recommended-phase-1-schema-must-haves-consolidated-checklist) above. | Each item is independently cheap-now / expensive-later and was surfaced by at least one researcher. Consolidating them into one Phase 1 checklist prevents the "we'll add audit log in v1.1" trap. |
| **Pantum hang detection — SNMP alone is insufficient** | **The printer-monitoring phase plan must include an explicit spike** to choose between (a) a small local agent installed on each print host that reports `Win32_PrintJob` status via HTTP to Trackly, or (b) remote WMI/RPC from Trackly to print hosts with an AD service account. SNMP `prtMarkerLifeCount` (page counter) is used as confirmation only. Auto-restart spooler stays out of v1 per PROJECT.md. | PITFALLS.md is clear: the hang is on Windows print spooler, not on the device; SNMP returns "healthy" while jobs are queued. The two options have very different operational footprints (local agent = software to deploy + update on each host; remote WMI = AD service account + firewall rules + DCOM exposure). Picking is a real architecture decision that needs hands-on time with a real BM5100ADN, so it belongs in a spike not in this synthesis. |

## Open Questions (deferred to specific phases via spikes)

These remain unresolved and are explicitly **not** decisions for the roadmap — they are spike work scheduled inside specific phases.

1. **Logo storage:** BLOB in the DB or a file under `<exe_dir>/logos/`? *Recommendation (defer to Phase "Настройки / Organization"):* BLOB. Portable backup is a single .db file; one less moving part.
2. **Custom fields:** add a generic `device_custom_fields(device_id, key, value)` table in Phase 1 or live with the freeform fields PROJECT.md already has (Техн. характеристики, Комплектация)? *Recommendation:* live with freeform in v1; revisit only if users complain. The cost of adding the table later is moderate and the benefit is speculative.
3. **HTTPS UX for self-signed certs in server mode:** mDNS `.local` hostname strategy vs. one-click `.cer` download with `certutil -addstore` instructions vs. encourage corp-CA cert in Settings. *Decided in Phase "Сервер-режим."* No HTTP listener is non-negotiable (Pitfall #11).
4. **USB-printer monitoring on workstations:** local agent vs. remote WMI/RPC — same architectural seam as Pantum hang detection (Pitfall #3). Combine into one spike.
5. **`tauri-specta` bindings.ts generation:** committed to git or generated in `cargo test`? *Recommendation:* generated in a `cargo test` step, gitignored, regenerated by an `npm prebuild` script — picks one and sticks.
6. **AD authentication detail:** simple UPN bind (`us100@corp.local`) vs. service-account search-then-bind for `sAMAccountName` flow. *Decided in Phase "AD-вход"* after testing against real WS2022 with channel binding enforced.
7. **Notifications phase decomposition:** the four channels (in-app, SMTP, Telegram, webhook) should not ship in one phase per FEATURES.md prioritization. *Decision deferred:* roadmapper to split into in-app + SMTP (one phase) and Telegram + webhook (separate, later).
8. **Windows 7 32-bit feasibility:** Tauri NSIS with `embedBootstrapper` + `i686-pc-windows-msvc` works in theory; WebView2 on Win7 needs manual TLS 1.2 enablement; `krilla` MSRV 1.92 may close the door. *Spike in Phase "Инфраструктура / выпуск."* Treat as experimental in release notes.

## Implications for Roadmap

Based on the combined research, this is the suggested phase structure with rationale for each. Phase 1 is non-negotiable in shape; phases 2–10 can be reordered modestly without breaking dependencies (annotated below).

### Phase 1: Фундамент — схема БД, портативность, дисциплина записи

**Rationale:** Every other phase depends on the schema, the path-resolution discipline, the single-writer pattern, and the cross-cutting invariants (audit log, soft delete, optimistic lock, UTC time, `Secret<T>`). Retrofitting any of these is rewrite-grade work. PROJECT.md's own Key Decisions table identifies this as the right starting point: "большой объём связей… хочется устаканить схему до строительства UI."
**Delivers:** Cargo workspace with 3 crates; `paths.rs` resolves portable paths with sentinel detection + clippy lint blocking `dirs::*_dir()`; `WEBVIEW2_USER_DATA_FOLDER` set in `main()`; rusqlite + refinery wiring with split read/write pools + all WAL pragmas; full schema migrations including all tables listed in the Phase 1 checklist (audit_log, soft delete, optimistic lock, device_types/statuses tables, parent_act_id + sub_number on returns, denormalized assigned_to, sessions, scheduled_tasks, counters, document_templates with versioning); `Secret<T>` newtype; `AppError` type unified across both transports; `Clock` trait for testability; CI on Windows runner with Cyrillic install path + ProcMon "no AppData writes" assertion + load test for SQLite concurrency.
**Addresses:** Foundation for every feature in FEATURES.md.
**Avoids:** Pitfalls #1, #2, #6, #15 (portable leak, SQLite locked, Cyrillic paths, timezones) — all of which are Phase 1 prevention. Establishes `Secret<T>` discipline used later for Pitfall #4 (AD) and #13 (SNMP community). Schema versioning groundwork for Pitfall #10 (backup).

### Phase 2: Вертикальный slice — Devices CRUD (Tauri-only)

**Rationale:** Validate the hexagonal architecture and the rusqlite writer-task pattern end-to-end on the simplest entity. Ship the simplest possible Tauri command path before introducing axum and dual transport.
**Delivers:** `DeviceService` + `SqliteDeviceRepo` + service-level mock tests; `AppCtx` construction; first `#[tauri::command]` (`create_device`, `list_devices`); `tauri-specta` bindings.ts pipeline; Svelte page rendering devices from SQLite; контекстный автокомплит pattern (autocomplete dropdowns powered by previously-entered values, scoped by Наименование).
**Uses:** Tauri 2, rusqlite + refinery, Svelte 5 runes, vite, tauri-specta v2.
**Implements:** Hexagonal core + thin transport adapter pattern (ARCHITECTURE.md Pattern 1, 2, 4).

### Phase 3: Dual transport — axum + browser SPA

**Rationale:** Prove the dual-transport pattern before more features pile up. Doing it once for one entity (devices) is cheap; doing it later for every entity in retrospect is painful.
**Delivers:** `axum::Router` mounted on `tokio::spawn` from `tauri::Builder::setup`; `with_state(AppCtx)`; HTTP endpoints mirroring the Tauri commands from Phase 2; `transport.ts` runtime detection; same Svelte build serves both contexts; `tower-sessions` skeleton with rusqlite-backed `SessionStore`; CSRF synchronizer-token middleware; server-mode toggle in Settings.
**Uses:** axum 0.8, tower-http, tower-sessions, rcgen + rustls for first-launch self-signed cert.
**Implements:** ARCHITECTURE.md Patterns 1–5; addresses Pitfall #5 (single `authorize()` from both transports, curl-based 403 test scaffolded), Pitfall #11 (HTTPS only, cert SAN with LAN IPs).

### Phase 4: Authentication + roles — local users (Tauri unlocked default)

**Rationale:** The browser path from Phase 3 needs login; can't ship server mode without it. Tauri desktop stays unlocked by default ("locked desktop" is a setting). Cleanest place to land the `authorize(user, Permission)` function that every subsequent phase will use.
**Delivers:** `User` entity with `source` (local | ad); `argon2id` password hashing; 3 roles (Admin / Specialist / Сотрудник); `authorize()` function called from every command and every axum handler; curl-based role × endpoint matrix test in CI; login screen for browser, bypass for desktop; password reset by admin.
**Uses:** argon2 0.5, tower-sessions, rand_core::OsRng.
**Implements:** Addresses Pitfall #5 fully; lays groundwork for Pitfall #4 (no AD passwords ever stored).

### Phase 5: Акты приёма-передачи + Возвраты + первая PDF-печать

**Rationale:** This is the core value of PROJECT.md ("одной кнопкой"); blocks no later phase but unblocks the differentiator features. PDF infrastructure introduced here is reused by every later printable.
**Delivers:** `ActService` with sequential numbering (counter table + `UPDATE … RETURNING` under `BEGIN IMMEDIATE`); full + partial returns with sub-numbering ("N в1", "N в2"); archive-on-full-return; undo return with prior-state restore (reads from `activity_log`); editable templates table loaded into Tera/MiniJinja **in safe mode** (no I/O, render timeout); PDF rendering through `krilla 0.7` with embedded DejaVu/PT Sans + CI fixture hash test for «Сидоров (ё) №42»; печать Документа приёма.
**Uses:** krilla 0.7 (default) or `typst-as-lib` (if Phase-5 spike flips it), MiniJinja or Tera safe-mode, refinery for template versioning.
**Implements:** Addresses Pitfalls #7 (PDF Cyrillic — first encounter), #9 (sequential ID race — pattern established), #14 (template injection — safe-mode + version field).
**Research flag:** Spike (1–2 days) for krilla-vs-typst on a real fixture before final commit.

### Phase 6: CSV import/export для Devices

**Rationale:** Critical for adoption (first-time migration from Excel/paper). Should land after Phase 2 so the entity exists, and after Phase 1's optimistic-lock infrastructure so chunked imports don't blow up concurrent edits.
**Delivers:** Encoding sniffing (BOM → UTF-8 → CP1251 fallback); delimiter auto-detect; preview-before-commit UI with first 5 rows; chunked imports (500-row transactions); progress events; `import_id` foreign key on imported rows enabling "undo last import"; UTF-8 BOM + `;` delimiter default on export with "Excel-совместимый" checkbox.
**Uses:** csv crate, encoding_rs.
**Implements:** Addresses Pitfall #8 (CSV encoding mojibake).

### Phase 7: Картриджи (Модели + Экземпляры + lifecycle)

**Rationale:** Independent of Phases 5–6; can run in parallel. Lands the two-orthogonal-state pattern (charge_state × lifecycle_state) and the auto-coded `C-000001` sequence.
**Delivers:** CartridgeModel CRUD with compatibility matrix (Brand+Model pairs with autocomplete); Cartridge instance CRUD with `cartridge_seq` + display-formatted code; switch-bar by lifecycle_state; contextual actions per state; logging of передача (Дата / Кто выдал / Кому / Расположение); low-stock threshold in Settings; in-app dashboard banner; (future) link `current_printer_id` once Printers ship.
**Uses:** Same pattern as Acts; FTS5 for cartridge search.

### Phase 8: Принтеры — SNMP мониторинг + subnet discovery (no Pantum auto-fix)

**Rationale:** Independent of Cartridges in dependencies but together they're the printer/cartridge story. Big phase: subnet discovery, multi-vendor SNMP profiles, history table, Pantum hang detection alert pipeline. **Auto-restart spooler is explicitly NOT in this phase** per PROJECT.md.
**Delivers:** `SnmpClient` trait (mock first so UI ships without real printers) + `snmp2`-backed impl; per-vendor OID profile tables (Pantum / Kyocera / HP / Canon + RFC 3805 fallback); subnet scan; per-printer poll loop with bounded concurrency (`tokio::sync::Semaphore`, 10 permits); status history table; retry strategy (2s timeout, 3 retries, `degraded` → `offline` after 3 cycles); SNMP community string stored as `Secret<String>`, encrypted at rest with a key file beside the DB; **Pantum hang detection prototype** combining SNMP page-counter cross-check with the chosen host-side mechanism.
**Uses:** snmp2 0.4 with crypto-rust, tokio Semaphore.
**Implements:** Addresses Pitfall #13 (SNMP community + UDP retry).
**Research flag:** **Hands-on spike required** before final design — pick local agent vs remote WMI/RPC for the host-side hang signal (Pitfall #3 resolution). Test against a real BM5100ADN with the spooler hang reproduced.

### Phase 9: Заявки (Requests) + Сотрудник web-UI + Дашборд

**Rationale:** Заявки depend on Users (Phase 4) and the browser SPA (Phase 3); link-to-cartridge requests depend on Cartridges (Phase 7). Dashboard widgets are computed from existing tables — natural fit here.
**Delivers:** Request CRUD; two types (картридж-замена связан с моделью/принтером + свободная форма); 3-state lifecycle; печать заявки (PDF); browser-режим UI для сотрудников (только заявки, login flow); dashboard widgets (Devices / Cartridges / Заявки / Динамика); тёмная/светлая/системная тема toggle in sidebar.

### Phase 10: Отчёты

**Rationale:** Touches every core entity, so should come after they exist. Cheap if Phase 1 invariants (UTC + audit log) are honored — expensive if not.
**Delivers:** Отчёты Devices + Cartridges; period selection (месяц / год / диапазон) with proper TZ boundary handling via organization TZ in Settings; group-by-month visual separators; filter/search inside reports; export (CSV first, PDF reusing Phase 5 infrastructure).
**Implements:** Addresses Pitfall #15 (TZ boundaries) directly.

### Phase 11: Backup (manual + scheduled) + Settings polish

**Rationale:** Backup uses scheduled-tasks supervisor scaffolded in Phase 1 + entity-level data established in all earlier phases. Settings polish gathers organization metadata, low-stock thresholds, server-mode toggles, etc. — most fields land naturally in earlier phases; this phase is the cleanup pass.
**Delivers:** `rusqlite::backup::Backup` API or `VACUUM INTO`; integrity_check after each backup; retention policy (N daily + M weekly + K monthly); restore flow with schema-version check; quarterly CI restore test; manual + scheduled backup UI in Settings.
**Implements:** Addresses Pitfall #10 (backup correctness).

### Phase 12: Notifications — in-app + SMTP email

**Rationale:** Depends on stable event sources from Phases 5/7/8 (low-stock, new request, printer hang). Splitting from Telegram/webhook avoids cramming four channels into one phase (FEATURES.md warning).
**Delivers:** `Notifier` trait; in-app banners hooked into existing events; SMTP via `lettre` with rustls; Settings UI for SMTP credentials (`Secret<String>`).
**Uses:** lettre with tokio-rustls transport.

### Phase 13: AD-вход (LDAP bind + ФИО pull + registration requests + auto-accept setting)

**Rationale:** Late phase per PROJECT.md and FEATURES.md. Needs HTTPS (Phase 3) and the `Secret<T>` discipline (Phase 1).
**Delivers:** `AdClient` trait (mock first) + `ldap3 0.12`-backed impl with `tls-rustls`; UPN-bind by default + service-account search-then-bind for `sAMAccountName` path; SRV-record DC discovery; configurable conn_timeout; "Trust CA from PEM" Setting; `users.source = 'ad'`; AD-registration request flow with auto-accept toggle (default OFF, with warning).
**Implements:** Addresses Pitfall #4 (AD bind issues) fully.
**Research flag:** **Must test against a real Windows Server 2022 with LDAP signing + channel binding enforced** — mocks pass everything.

### Phase 14: Pantum auto-restart spooler + WMI/RPC integration

**Rationale:** Per PROJECT.md, only after the hang-detection hypothesis from Phase 8 is confirmed in production. High-risk feature (broad `Restart-Service Spooler` kills all queues for all printers on the host); requires operator confirmation and per-queue scoping.
**Delivers:** Operator-confirmed restart action; per-queue scoping (`Remove-PrintJob` + driver-restart instead of full spooler restart where possible); audit-log entry for every restart; rate-limit per printer.
**Implements:** PROJECT.md's "Pantum-автофикс отдельная фаза" decision.

### Phase 15: Telegram + Webhook outputs + (stretch) Windows 7 32-bit build

**Rationale:** Niche channels; webhook is power-user; Win7 32-bit is best-effort.
**Delivers:** `teloxide` or raw `reqwest` Telegram client; webhook POST with JSON; `i686-pc-windows-msvc` matrix step in GH Actions Release with NSIS `embedBootstrapper`; release-notes "experimental" label for Win7.
**Implements:** Addresses Pitfall #12 (cross-compile / SmartScreen) gradually — start with `windows-latest` runner from Phase 1 CI, add 32-bit best-effort here.

### Phase Ordering Rationale

- **Phase 1 must be first.** The cost of retrofitting audit log, soft delete, optimistic lock, single-writer pattern, UTC discipline, `Secret<T>`, and portable path resolution is much higher than the cost of designing them in. PROJECT.md's own Key Decisions table acknowledges this ("Phase 1 — фундамент").
- **Phases 2 → 3 → 4 are a strict chain.** Vertical slice (Tauri-only) before dual transport before auth. Each unblocks the next.
- **Phase 5 (Acts/Returns/PDF) is the differentiator MVP** and naturally hosts the first PDF infrastructure that every subsequent printable reuses.
- **Phases 6, 7, 8, 10 can be reordered or partially parallelized** if multiple devs are available — they share no hard dependencies.
- **Phase 9 (Requests/Dashboard/Web-UI for сотрудники) must follow Phase 7** (cartridge-replacement requests link to cartridge models) and Phase 4 (auth).
- **Phases 11 (Backup), 12 (Notifications), 13 (AD), 14 (Pantum auto-fix), 15 (Telegram/Webhook/Win7)** are explicitly "later" per PROJECT.md and FEATURES.md. They depend on stable data and event sources.
- **The Pantum hang-detection spike in Phase 8 is the highest-risk research item in the project** and should run as early in Phase 8 as possible — its outcome affects whether Phase 14 is even buildable.

### Research Flags

Phases that should run `/gsd-plan-phase --research-phase <N>` (need deeper research during planning):

- **Phase 1:** moderate — most patterns are established in ARCHITECTURE/STACK/PITFALLS but the **WEBVIEW2_USER_DATA_FOLDER timing**, the **Cyrillic Windows manifest setup**, and the **ProcMon-in-CI scaffolding** are likely to need a half-day spike each.
- **Phase 5:** **krilla vs Typst-as-lib spike** with real Cyrillic fixture render hashed in CI. 1–2 days.
- **Phase 8:** **largest research surface in the whole project.** Pantum spooler-hang detection needs hands-on time with a real BM5100ADN. Architecture choice between local agent vs remote WMI/RPC is operationally consequential. SNMP per-vendor OID profile tables need empirical confirmation against Pantum/Kyocera/HP/Canon — published MIBs are partial. Budget a week for the spike before commit.
- **Phase 13:** **must validate against a real Windows Server 2022** with LDAP signing + channel binding enforced. Mock-only AD passes everything; reality breaks at first bind. Half-day spike with a real DC required.
- **Phase 14:** depends on Phase 8 spike outcome — if the host-side mechanism is local agent, this phase reuses that agent; if it's remote WMI/RPC, this phase scopes restarts per-queue (`Remove-PrintJob` + driver-restart, not blanket `Restart-Service`). Plan after Phase 8 validation.
- **Phase 15 (Win7 32-bit stretch):** spike on a real Win7 SP1 VM. `krilla 0.7` MSRV 1.92 + WebView2 + TLS 1.2 manual enablement may close the door — decide best-effort vs drop.

Phases with standard patterns (likely skip the research-phase step):

- **Phase 2, 3, 4, 6, 7, 10, 11, 12** — well-trodden Rust + Tauri + axum + Svelte patterns already covered in ARCHITECTURE.md and STACK.md; planning can lean on the existing research files.
- **Phase 9 (Requests/Dashboard/Web-UI)** — UX-heavy but architecturally straightforward; only the Server-Sent-Events vs polling decision for real-time updates may need a small spike.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | Tauri 2 (GA Oct 2024), Svelte 5 (stable Oct 2024), axum 0.8, rusqlite 0.39, snmp2 0.4, ldap3 0.12, argon2 0.5, rustls 0.23 — all current, all production-tested. Multiple authoritative sources (official docs, recent benchmark posts, active GitHub discussions). The one MEDIUM area is PDF (krilla 0.7 chosen as default, Typst as spike-decided backup). |
| Features | HIGH | Cross-referenced against Snipe-IT, GLPI, ManageEngine ServiceDesk Plus / AssetExplorer, Lansweeper, and PaperCut; Russian-org conventions (инвентарный номер, акт приёма-передачи, заправка картриджей, ФИО, МОЛ) cross-referenced against РФ-specific sources. Five gaps in PROJECT.md identified and addressed with Phase 1 schema recommendations (audit log, soft delete, optimistic lock, denormalized assigned_to, тables-not-enums for types/statuses). One genuine product question (Расходник type vs Картриджи section) resolved with a chosen direction. |
| Architecture | HIGH | Hexagonal-in-Rust, dual-transport, split read/write SQLite pools, `tauri-specta v2`, `tower-sessions`, single tokio runtime — all verified across official Tauri docs, sqlx WAL benchmark post (the canonical rationale), and active community walkthrough posts. The split-pool pattern + the single-writer-task discipline is the most important architectural decision in the project and both research files converge on it. |
| Pitfalls | HIGH | All 15 pitfalls are well-documented in their respective ecosystems (Tauri/SQLite/AD/SNMP/Cyrillic). Pantum-specific MEDIUM (vendor SNMP support is shallow publicly; resolved with a dedicated spike). Russian-locale traps (Cyrillic paths, CP1251, TZ 2014 quirks) are HIGH-confidence and addressable. |

**Overall confidence:** HIGH.

### Gaps to Address

- **Pantum BM5100ADN hands-on validation** (Phase 8) — the hang-detection design must be confirmed against a real device with the spooler-hang scenario reproduced. The spike outcome determines Phase 14's feasibility.
- **Real Windows Server 2022 AD environment** (Phase 13) — mocks cannot validate channel binding / LDAP signing / corp CA / SRV discovery. A real DC is required for confidence.
- **PDF Cyrillic engine choice** (Phase 5 spike) — krilla 0.7 is the default but Typst-as-lib is a credible alternative; decide with a hashed fixture render in CI.
- **Windows 7 32-bit feasibility** — multiple MSRV constraints (`krilla` 1.92, WebView2 manual TLS 1.2 on Win7 SP1) make this best-effort; spike in Phase 15 to confirm or drop.
- **USB-printer monitoring architecture** (Phase 8) — same local-agent-vs-WMI seam as Pantum hang detection. Combine into one spike.
- **CSV preview UX for partial-decode-success scenarios** (Phase 6) — when some rows are UTF-8 and others are CP1251 (real after multi-step manual editing), preview must be honest about per-row decoding. UX detail, not architectural.
- **HTTPS UX for self-signed certs** (Phase 3 / Phase "Сервер-режим") — mDNS `.local` strategy vs one-click cert install vs corp-CA encouragement. Real-deployment UX, not technical.

## Sources

### Primary (HIGH confidence) — official docs and authoritative posts
- [Tauri 2.0 Stable Release Blog](https://v2.tauri.app/blog/tauri-20/) — v2 GA Oct 2024
- [Tauri Core Releases](https://v2.tauri.app/release/) — current versions
- [Tauri Webview Versions](https://v2.tauri.app/reference/webview-versions/) — Win7 + WebView2 caveats
- [Tauri Windows Installer Docs](https://v2.tauri.app/distribute/windows-installer/) — NSIS `embedBootstrapper`
- [Tauri Capabilities](https://v2.tauri.app/security/capabilities/) — confirms backend authz is required
- [Tauri Discussion #8029 — WebView2 cache cleanup](https://github.com/orgs/tauri-apps/discussions/8029) — `WEBVIEW2_USER_DATA_FOLDER`
- [Tauri Issue #7491 — EBWebView in AppData](https://github.com/tauri-apps/tauri/issues/7491)
- [Detecting Tauri webview in frontend (Discussion #6119)](https://github.com/tauri-apps/tauri/discussions/6119) — `isTauri` vs `__TAURI_INTERNALS__`
- [Svelte Releases](https://github.com/sveltejs/svelte/releases) — Svelte 5.55+
- [Svelte 5 Migration Guide](https://svelte.dev/docs/svelte/v5-migration-guide)
- [SQLite WAL official docs](https://sqlite.org/wal.html) — single-writer rule, no network FS
- [SQLite Forum — Hot backup in WAL mode](https://sqlite.org/forum/forumpost/2ea989bbe9)
- [SQLite Autoincrement](https://sqlite.org/autoinc.html)
- [rusqlite 0.39 docs](https://docs.rs/rusqlite/latest/rusqlite/)
- [refinery (rust-db/refinery)](https://github.com/rust-db/refinery)
- [axum 0.8.9 docs](https://docs.rs/axum/latest/axum/)
- [PSA: Write Transactions are a Footgun with SQLx and SQLite — Evan Schwartz](https://emschwartz.me/psa-write-transactions-are-a-footgun-with-sqlx-and-sqlite/) — canonical rusqlite-over-sqlx rationale
- [SQLx + WAL split read/write pools — Evan Schwartz](https://emschwartz.me/psa-your-sqlite-connection-pool-might-be-ruining-your-write-performance/)
- [tauri-specta v2 (specta-rs/tauri-specta)](https://github.com/specta-rs/tauri-specta)
- [tower-sessions](https://docs.rs/tower-sessions)
- [snmp2 crate docs](https://docs.rs/crate/snmp2/latest/features)
- [ldap3 crate docs](https://docs.rs/ldap3/latest/ldap3/)
- [ldap3 LdapConnSettings](https://docs.rs/ldap3/latest/ldap3/struct.LdapConnSettings.html)
- [argon2 (RustCrypto)](https://github.com/RustCrypto/password-hashes)
- [krilla (LaurenzV/krilla)](https://github.com/LaurenzV/krilla)
- [RFC 3805 — Printer MIB v2](https://datatracker.ietf.org/doc/html/rfc3805)

### Secondary (MEDIUM confidence) — community sources, vendor docs
- [Snipe-IT product features](https://snipeitapp.com/product), [managing assets](https://snipe-it.readme.io/docs/managing-assets), [custom fields](https://snipe-it.readme.io/docs/custom-fields), [LDAP sync](https://snipe-it.readme.io/docs/ldap-sync-login)
- [GLPI features overview](https://www.glpi-project.org/en/features/), [Printers](https://help.glpi-project.org/documentation/modules/assets/printers), [Inventory FAQ](https://help.glpi-project.org/faq/glpi/inventory)
- [ManageEngine ServiceDesk Plus Asset Mgmt](https://www.manageengine.com/products/service-desk-msp/help/adminguide/configurations/asset_management/inventory-configurations.html)
- [Lansweeper asset/printer discovery](https://www.lansweeper.com/product/asset-discovery/)
- [PaperCut toner levels via SNMP](https://www.papercut.com/help/manuals/ng-mf/applicationserver/printer-toner-levels/)
- [Master Hexagonal Architecture in Rust (howtocodeit.com)](https://www.howtocodeit.com/guides/master-hexagonal-architecture-in-rust)
- [Tauri + SQLite + Axum walkthrough (Medium)](https://ritik-chopra28.medium.com/build-a-cross-platform-desktop-app-in-rust-tauri-2-0-sqlite-axum-2b9b7b732e0d)
- [Microsoft Learn — Troubleshooting printing scenarios](https://learn.microsoft.com/en-us/troubleshoot/windows-server/printing/troubleshoot-printing-scenarios) — confirms host-side spooler hang
- [Pantum BM5100 Series Manual](https://www.manualslib.com/manual/2115030/Pantum-Bm5100-Series.html)
- [Russian — инвентарные номера](https://ppt.ru/art/inventarizaciya/kak-prisvaivayutsya-inventarnye-nomera), [акт приёма-передачи ТМЦ](https://assistentus.ru/forma/akt-priema-peredachi-materialnyh-cennostej-rabotniku/), [учёт ИТ-оборудования (Habr)](https://habr.com/ru/articles/750256/)
- [Bert Hubert — SQLITE_BUSY despite timeout](https://berthub.eu/articles/posts/a-brief-post-on-sqlite3-database-locked-despite-timeout/)
- [Ten Thousand Meters — SQLite concurrent writes / BEGIN IMMEDIATE](https://tenthousandmeters.com/blog/sqlite-concurrent-writes-and-database-is-locked-errors/)
- [Password Hashing Guide 2025/2026](https://guptadeepak.com/research/password-hashing-guide-2026/) — argon2id params
- [Time in Russia — Wikipedia](https://en.wikipedia.org/wiki/Time_in_Russia) — 2011/2014 DST history

### Tertiary (LOW confidence) — single sources, needs validation in-phase
- [Typst-as-lib (crates.io)](https://crates.io/crates/typst-as-lib) — credible but newer; validate in Phase 5 spike
- [async-snmp](https://github.com/lukeod/async-snmp) — marked unstable; `snmp2` chosen instead
- [Tauri Issue #12331 — Self-signed cert hosting](https://github.com/tauri-apps/tauri/issues/12331) — open issue, not a recipe
- Pantum-specific SNMP behavior — sparse public documentation; validated only via vendor manual + community troubleshooting; **requires hands-on spike in Phase 8**.
- Windows 7 32-bit + `krilla` MSRV 1.92 + WebView2 TLS 1.2 — composite of three constraints with no single source confirming all three together; **spike in Phase 15 or drop**.

---
*Research completed: 2026-05-24*
*Ready for roadmap: yes*

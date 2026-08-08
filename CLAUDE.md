<!-- GSD:project-start source:PROJECT.md -->
## Project

**Trackly**

Trackly — приложение для учёта и отслеживания техники, принтеров и картриджей в организации с несколькими локациями и складами. Десктоп-приложение (Tauri + Svelte) с встроенным режимом сервера, позволяющим сотрудникам подключаться через браузер из локальной сети для отправки заявок и работы с системой. Целевая среда — Windows-сеть с Active Directory, разработка ведётся на macOS.

**Core Value:** Учёт устройств и картриджей с актами приёма-передачи и историей перемещений должен работать надёжно и быстро в режиме «одной кнопкой» — без обращения к Excel-таблицам, ручного присвоения номеров актов или потери истории при возврате на склад.

### Constraints

- **🔒 ПРИВАТНОСТЬ ДАННЫХ — ЖЁСТКОЕ УСЛОВИЕ, ДЕЙСТВУЕТ ВСЕГДА:** репозиторий **публичный**.
  Реальные данные организации и людей НИКОГДА не попадают в репозиторий — ни в код, ни в
  шаблоны, ни в тесты и фикстуры, ни в скриншоты, ни в `.planning/`-артефакты (PLAN, SUMMARY,
  STATE, CONTEXT, брифы). Под запретом: название организации и её реквизиты (ИНН, КПП, ОКПО,
  ОГРН, адрес, телефон, e-mail), ФИО сотрудников, содержимое рабочих конфигов и любые данные,
  вбитые в БД для проверок. В шаблонах и коде — только переменные и placeholder'ы
  (`org.name`, `{{ act.giver_name }}`), никогда не хардкод реального названия. В тестах —
  вымышленные имена («Иванов И.И.», «Петров П.П.»). Описывая живой UAT, обезличивать:
  «ФИО сотрудника обрезалось», а не само ФИО. Всё закоммиченное остаётся в истории git даже
  после удаления из HEAD — проверять ДО коммита, а не после.
- **Тех-стек:** Rust (бэкенд), Tauri (десктоп-обёртка), Svelte (фронтенд), SCSS (стили), SQLite (БД) — фиксировано пользователем.
- **Целевая платформа:** Windows 64-bit (primary), macOS Apple Silicon (dev + use), Linux (вторичная). Опционально Windows 7 32-bit, если позволит выбранный Rust toolchain и Tauri версия.
- **Portable:** приложение не должно требовать установки и записывать данные в `%APPDATA%`/`%LOCALAPPDATA%`/системные пути. БД и конфиг — рядом с исполняемым файлом (или в каталоге, указанном пользователем).
- **Безопасность:** пароли пользователей — только хэш (argon2 / bcrypt); чувствительные данные AD (пароли) — не сохранять, только использовать для bind/проверки. В режиме сервера — HTTPS-сертификат (self-signed по умолчанию, путь к собственному — настраиваемый).
- **Concurrent-доступ:** SQLite в режиме WAL, единая точка записи через бэкенд-слой (никаких прямых записей из нескольких процессов).
- **Языковая локализация:** UI и шаблоны документов — только русский в v1.
- **Размещение кода:** GitHub. CI: проверки кода на push, релизы по тегам.
- **Документы:** редактируемые шаблоны печатных форм должны храниться в БД (или рядом с БД), чтобы переноситься вместе с portable-сборкой.
<!-- GSD:project-end -->

<!-- GSD:stack-start source:research/STACK.md -->
## Technology Stack

## Recommended Stack
### Core Technologies
| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| **Tauri** | `2.x` (currently `tauri ^2.11`, `tauri-bundler ^2.9`, `wry ^0.55`, `tao ^0.35`) | Desktop shell, packaging, system integration | Tauri 2 is GA since Oct 2024; v1 is EOL. v2 gives us a stable plugin system (`tauri-plugin-fs`, `tauri-plugin-shell`, `tauri-plugin-os`, `tauri-plugin-updater`, `tauri-plugin-log`), proper multi-window support, capability/ACL model, mobile targets we don't need but won't fight us. The bundler emits MSI + NSIS for Windows, DMG for macOS, AppImage/deb/rpm for Linux from one config. |
| **Svelte** | `5.x` (currently `5.55+`) | UI framework | Svelte 5 ships runes (`$state`, `$derived`, `$effect`, `$props`) as the explicit reactivity model, stable since Oct 2024. Runes scale to the kind of cross-cutting state this app has (filters, switch-bars, dashboard widgets, AD/role context) much better than Svelte 4 stores would. |
| **Build tool** | **Vite** `6.x` | Dev server, bundler for the Svelte frontend | Tauri's official Svelte template uses Vite. HMR works inside the Tauri webview during dev. No reason to deviate. |
| **Frontend framework** | **Vanilla Svelte 5 (no SvelteKit)** | SPA shell for both Tauri webview and LAN browser | The Svelte frontend has to run in two webviews (Tauri's WebView2/WKWebView **and** any LAN browser via the axum server). Vanilla Svelte compiles to a flat SPA — same `index.html` + JS bundle works in both. SvelteKit (even with `adapter-static` + `ssr=false` + `prerender=false`) adds the router and conventions but no value here: routing is light, we don't need file-system routes, and we'd just have to disable everything that makes SvelteKit "Kit." Pick `svelte-spa-router` or `svelte-routing` for the ~10 sections. |
| **Styling** | **SCSS** via `svelte-preprocess` (or `vitePreprocess`+`sass`) | Component styles | User-fixed. Set up via `vitePreprocess({ scss: { prependData: '@use "src/styles/_tokens.scss" as *;' } })` so design tokens are auto-available in every `<style lang="scss">` block. |
| **Database** | **SQLite** via `rusqlite 0.39` with the `bundled` feature | Embedded relational DB | User-fixed. `bundled` ships SQLite source compiled in — no system dependency, no DLL hunt on Windows, portable mode actually portable. We **deliberately pick `rusqlite` over `sqlx-sqlite`** — see "What NOT to Use" for the write-lock-starvation rationale. |
| **HTTP server (server mode)** | **axum** `0.8.x` (on `tokio` `1.x`) | LAN HTTP API + static asset serving + WebSocket | Tower ecosystem, modular middleware, first-class WebSocket via `axum::extract::ws`, first-class static serving via `tower-http::services::ServeDir`. Cleaner integration with the rest of the Rust async world than actix-web's actor model, and the perf gap (~10–15% raw throughput in benches) is irrelevant at 20 concurrent LAN users. |
| **Async runtime** | **tokio** `1.x` (multi-thread) | Underlies axum, ldap3 async, snmp2 async, tracing | Already required by every async lib in the stack. |
| **SQL migrations** | **`sqlx-cli`-style isn't in scope (we don't use sqlx) — use `refinery 0.8.x` with the `rusqlite` driver** | Schema versioning, embed migrations in the binary | Refinery embeds `.sql` files at compile time, runs them in a single transaction by default, works with `rusqlite` natively. Portable mode bonus: no separate `migrations/` directory needs to ship beside the .exe — they're in the binary. |
### Supporting Libraries
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `tokio` | `1.x` (`rt-multi-thread`, `macros`, `signal`, `sync`, `fs`, `net`, `time`) | Async runtime | Always (in `main.rs` via `#[tokio::main]`). |
| `tower` / `tower-http` | `0.5+` / `0.6+` | Middleware: tracing, CORS, compression, static files, request limits | Always with axum. Use `tower-http::services::ServeDir` to serve the Svelte build to LAN browsers. |
| `tower-sessions` | `0.13+` | Session middleware for axum (cookie-based) | Use for LAN web auth. **Prefer over JWT** for this app — sessions are revocable, simpler, and you don't need cross-domain stateless tokens on a single-org LAN. |
| `tower-sessions-sqlx-store` | latest compatible | **Skip — we're on rusqlite, not sqlx**. Use the in-process `MemoryStore` for sessions, or write a tiny `SessionStore` impl backed by `rusqlite` (~80 LoC). | Server mode only. |
| `serde` / `serde_json` | `1.x` | JSON serialization for both Tauri `invoke` and HTTP API | Always — payload schemas should be defined **once** and used by both transports. |
| `thiserror` | `1.x` | Error types in the domain/data layer | Always. |
| `anyhow` | `1.x` | Top-level error handling in `main` and CLI/cron paths | At boundaries only — never in the library/domain crate. |
| `tracing` | `0.1.x` | Structured logging | Always. |
| `tracing-subscriber` | `0.3.x` (with `env-filter`, `fmt`, `json`) | Subscriber config: stdout + file | Always. |
| `tracing-appender` | `0.2.x` (`RollingFileAppender`, `non_blocking`) | File rotation (daily) — writes to `./logs/` next to the executable in portable mode | Always. Keep a `WorkerGuard` alive for the process lifetime. |
| `snmp2` | `0.4.x` (`crypto-rust` feature) | SNMPv1/v2c/v3 client; supports `get`, `getnext`, `getbulk`, walk, trap reception (sync + async sessions) | Printer monitoring (toner levels, status OIDs, ifTable for discovery). Pick `crypto-rust` over `crypto-openssl` so portable build has no OpenSSL DLL dependency. |
| `ldap3` | `0.12.x` (default features incl. `tls-native`) | LDAP/AD bind for username + password | AD-login phase. Use simple bind (`user@domain.tld` or `DOMAIN\user`) over LDAPS — that's how `us100`-style logins map. Defer `gssapi`/Kerberos feature until SSO milestone. |
| `argon2` (RustCrypto) | `0.5.x` | Password hashing for local users | Always. Pick `argon2id`, defaults: `m=19456 KiB, t=2, p=1` (matches OWASP 2024+). Pure Rust — works in portable build, no system crypto deps. |
| `rand` / `rand_core` | `0.8.x` / `0.6.x` | Salt generation for Argon2 (`OsRng`) | Always. |
| `uuid` | `1.x` (`v4`, `v7`, `serde`) | Public IDs for devices/acts/cartridges if we want non-sequential IDs alongside human numbers | Recommended for primary keys; keep human "C-000001" numbers as a separate column. |
| `time` | `0.3.x` | Date/time arithmetic, formatting, timezones | Prefer over `chrono` — smaller, no `time` zone DB headaches for a Russia/Moscow single-tz app. |
| `csv` | `1.3.x` | CSV import/export of devices | Always for the import/export feature. |
| `calamine` | `0.27+` | Read .xlsx (if users have legacy Excel inventories to import) | Optional, add only if users actually have .xlsx to import. Read-only — fast. |
| `rust_xlsxwriter` | `0.78+` | Write .xlsx (if reports need Excel output) | Optional — defer until users ask. CSV covers most "give me a list" needs. |
| `krilla` | `0.7+` | PDF generation with first-class OpenType + subsetting (Cyrillic-safe) | **Primary PDF choice.** Embed a Cyrillic-capable TTF (DejaVu Sans, PT Sans, or Roboto) in the binary via `include_bytes!`. Modern, actively maintained (April 2026 release), MSRV 1.92. |
| `typst` / `typst-as-lib` | latest | Alternative: render Typst templates → PDF | **Backup plan** if document templates need a designer-friendly markup language for users. `typst-as-lib` removes CLI startup overhead. Cyrillic works fine with a Cyrillic-capable font in the font path. |
| `which` | `7.x` | Locate system executables (e.g., for SNMP fallback, optional WMI shellouts) | Optional, only if we shell out. |
| `regex` | `1.x` | Used by tracing-subscriber env-filter and ad-hoc parsing | Transitive — keep version unified to avoid duplicate compiles. |
| `dirs` | `5.x` | **AVOID in portable mode** — it returns APPDATA paths | Use only if we add a non-portable install option later. Default code path must resolve config + DB relative to the executable (`std::env::current_exe().parent()`). |
| `notify` | `6.x` | Watch DB file path for the "DB moved" setting flow | Optional. |
| `rustls` + `rustls-pemfile` | `0.23` / `2.x` | TLS for the LAN axum server when HTTPS is required | Use **rustls**, not `native-tls`/OpenSSL — pure Rust, no DLL drama on Windows, ships in portable build cleanly. Provide a self-signed cert generator (`rcgen 0.13+`) on first server-mode launch. |
| `rcgen` | `0.13+` | Generate self-signed TLS cert for first run | Pair with rustls. |
| `paraglide-js` (deferred) | latest | i18n for Svelte when added | Officially recommended by SvelteKit team, compile-time, tree-shakable. For v1 (Russian only) we still skip it; flag for the i18n milestone. |
### Tauri Plugins (official `plugins-workspace`, all v2)
| Plugin | Use |
|--------|-----|
| `tauri-plugin-fs` | File picker for "выбрать расположение БД", export DB |
| `tauri-plugin-dialog` | Native open/save dialogs |
| `tauri-plugin-shell` | "Open containing folder" (БД, логи), open print preview in default app |
| `tauri-plugin-os` | OS info for diagnostics |
| `tauri-plugin-log` | Optional bridge: pipe `tracing` to the webview console during dev |
| `tauri-plugin-process` | Graceful restart from settings |
| `tauri-plugin-updater` | **Skip / make optional** — incompatible with portable mode by default. If we ship a non-portable installer variant, enable updater only for that variant. |
| `tauri-plugin-single-instance` | Prevent two desktop instances racing on the same DB file |
### Development Tools
| Tool | Purpose | Notes |
|------|---------|-------|
| `cargo` (Rust ≥ **1.85**) | Build, test | MSRV pinned by `ldap3 0.12` (NTLM needs 1.85; non-NTLM 1.82). Pick 1.85 to leave NTLM door open. |
| `cargo-watch` | Backend hot reload during dev | Optional dev dep. |
| `cargo clippy` | Lints | CI gate on every push. `-D warnings` in CI. |
| `cargo fmt` | Formatting | CI gate. |
| `cargo nextest` | Faster test runner | Optional but recommended for the data-layer test suite. |
| `cargo deny` | License + advisory audit | CI weekly job. |
| `cross` / `cargo-zigbuild` | Cross-compile from macOS dev box | `cargo-zigbuild` for **Linux** targets from macOS; **GitHub Actions Windows runners** for Windows builds (don't try cross-building MSVC from mac — zig can do `*-windows-gnu` but Tauri's WebView2 bindings prefer MSVC). |
| `pnpm` (or `npm`) | Frontend deps | Tauri docs are agnostic; pick one and stick. |
| `svelte-check` | Type checks across `.svelte` + TS | CI gate. |
| `vite-plugin-checker` | Optional in-editor TS+svelte-check overlay | Dev only. |
| `prettier` + `prettier-plugin-svelte` | Formatting | CI gate. |
| `eslint` + `eslint-plugin-svelte` | Lint | CI gate. |
### CI / Release
| Tool | Purpose |
|------|---------|
| GitHub Actions matrix | `windows-latest` (MSVC, x86_64), `windows-latest` + `rustup target add i686-pc-windows-msvc` (32-bit, optional), `macos-latest` (aarch64-apple-darwin), `ubuntu-latest` (x86_64-unknown-linux-gnu) |
| `tauri-apps/tauri-action@v0` | Drives `tauri build`, uploads artifacts to a draft release on tag push |
| `dtolnay/rust-toolchain@stable` + `swatinem/rust-cache@v2` | Toolchain + cache |
| Windows 7 32-bit | **Best-effort only.** Tauri's NSIS installer with `webviewInstallMode = "embedBootstrapper"` is the only viable path; MSI does not work on Win7 because the WebView2 bootstrapper download path needs TLS 1.2. Mark this build as "experimental" in release notes. |
## Installation
### Rust dependencies (`src-tauri/Cargo.toml` — illustrative)
### Frontend (`package.json` — illustrative)
## Alternatives Considered
| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|-------------------------|
| `rusqlite` + `refinery` | `sqlx` (with `sqlite` feature) | Only if the project is async-end-to-end at the DB layer **and** writes are funneled through a single task. SQLx + SQLite has a well-documented write-transaction footgun: a transaction that starts as a read gets upgraded to a write, holds the only writer slot, and starves all other writers — under our 20-concurrent-user assumption this surfaces as random hangs. We work around it by putting all writes behind a single `tokio::task::spawn_blocking` worker that owns one `rusqlite::Connection` (so the writer-singleton is structural, not accidental). |
| `rusqlite` | `diesel` (SQLite) | If we wanted a typed DSL ORM and were okay with sync-only + macro-heavy schema. Diesel's SQLite story is mature but the DSL adds friction for the kind of dynamic queries this app needs (autocomplete dropdowns, contextual filters). |
| `rusqlite` | `sea-orm` | If we needed an async ORM with relations + migrations. Overkill for ~10 tables; couples us to async writes (back to the SQLite-single-writer problem). |
| Vanilla Svelte 5 | SvelteKit (`adapter-static`, `ssr=false`, `prerender=false`) | If we want file-system routing and Kit's data-loading conventions and we're okay with Kit's build emitting an SPA that ignores most of Kit's value. The hybrid Tauri+browser delivery makes "no router opinion" the right call. |
| `axum` | `actix-web` | If we need the actor model anywhere, or if maximum raw throughput matters. Neither is true here. |
| `axum` | `poem` / `salvo` / `warp` | Smaller communities → smaller library halo (middleware, sessions, OpenAPI). axum's Tower middleware ecosystem is the deciding factor. |
| `snmp2` | `rasn-snmp` + `rasn` | If we wanted purely safe Rust ASN.1 codecs and were building generic SNMP tooling. For our use case (poll specific OIDs from known printer brands), `snmp2`'s ready-to-use client sessions ship faster. |
| `snmp2` | `csnmp` | Async-first, decent API, but smaller and less complete on v3 crypto. Pick if `snmp2`'s sync-first API becomes a pain point. |
| `snmp2` | `modern_snmp` | SNMPv3-only. Use if v3 is the only protocol we touch (it isn't — Pantum/Kyocera/HP defaults run v2c). |
| `ldap3` | `simple-ldap` wrappers | All wrap `ldap3`. Stay on the canonical one. |
| `argon2` | `bcrypt` (RustCrypto) | Only for compat with legacy hashes. New installs: argon2id. |
| `argon2` | `scrypt` | Memory-hard like argon2id but less standardized. Pick argon2id unless a specific compliance regime requires scrypt. |
| `krilla` | `genpdf` / `printpdf` (low-level) | `printpdf` works but defaults to Windows-1252 — Cyrillic forces you to embed a custom font and hand-measure text. `genpdf` builds on top of `printpdf`. krilla's subsetting + OpenType story is better in 2026. |
| `krilla` | Typst (`typst-as-lib`) | When document templates need a richer markup language editable by non-programmers (e.g., the user wants to tweak the act template themselves). Worth a spike in the "templates" phase. |
| `krilla` | HTML → PDF via the Tauri webview (`window.print()` / `webview.printToPdf`) | Works for "print preview" UX inside the desktop app, but **doesn't work in server mode** (the LAN user's browser is doing the printing, no canonical PDF). Use as a *preview* path only, not the source of truth. |
| `krilla` | wkhtmltopdf / weasyprint shellout | External binary, painful in portable mode (extra files to ship), security surface. Avoid. |
| `tower-sessions` cookies | JWT (e.g., `jsonwebtoken` crate) | If we ever need stateless multi-process auth. We don't on a single-process LAN server; revocation is much simpler with sessions. |
| `time` | `chrono` | If we need rich timezone DB / format compat with legacy systems. For RU-only single-tz, `time` is leaner. |
| `tracing-appender` daily rotation | `flexi_logger` / `fern` | Both work; `tracing-appender` is the canonical choice in the `tracing` ecosystem. |
## What NOT to Use
| Avoid | Why | Use Instead |
|-------|-----|-------------|
| **Tauri v1** | EOL, smaller plugin ecosystem, no capability model, harder mobile path (we won't take it, but the codebase shape matters) | Tauri v2 |
| **Svelte 4** | Stores-only reactivity doesn't scale to this app's cross-cutting state; eventual migration cost rises | Svelte 5 with runes |
| **SvelteKit (default config)** | SSR / file-system-router complexity is fighting Tauri's webview model; static-adapter half-disables Kit anyway | Vanilla Svelte 5 SPA |
| **`sqlx` with SQLite for write-heavy paths** | Documented lock-starvation footgun: any read tx that touches write upgrades the lock and blocks all other writers; with 20 concurrent users this manifests as occasional hangs that are hard to root-cause | `rusqlite` + single dedicated writer task via `tokio::task::spawn_blocking` |
| **`diesel` SQLite** | Sync-only; DSL friction for dynamic queries (autocomplete, contextual filters) | `rusqlite` + hand-written SQL + `refinery` |
| **`native-tls` / OpenSSL** for the LAN HTTPS server | Drags in a system OpenSSL or a vendored OpenSSL build; portable mode does not want a DLL | `rustls` |
| **`tauri-plugin-updater`** in portable mode | Updater writes to install dir and assumes a fixed install path; conflicts with "ship a folder anywhere" model | Skip in portable build; if shipping an MSI/NSIS install variant later, enable updater only there |
| **`dirs` crate as the default path resolver** | Returns `%APPDATA%` etc. — violates portable constraint | Resolve all paths relative to `std::env::current_exe()`; only fall back to `dirs::data_dir()` for an explicit non-portable install variant |
| **`bcrypt` for new password hashes** | CPU-only hard, GPU-friendly to attackers; NIST/OWASP now point at argon2id | `argon2id` |
| **`reqwest` with `native-tls`** | Same OpenSSL drag-along problem if we add outbound HTTP (notifications, webhooks later) | `reqwest` with `rustls-tls` feature, default-features = false |
| **`printpdf` directly** (no font embed) | Defaults to Windows-1252; Cyrillic is silently mangled or missing | `krilla` with an embedded Cyrillic font (DejaVu Sans / PT Sans / Roboto) |
| **`chrono` time-zone DB on Windows** | Historically pulled in `chrono-tz` weirdness on portable Windows builds | `time` crate (UTC + offsets is enough for RU-only) |
| **In-memory session store in multi-process scenarios** | Fine for our single-process server mode; **never** use if we ever fork worker procs | If multi-process ever happens: SQLite-backed `SessionStore` impl |
| **Shell-outs to `wkhtmltopdf` / `weasyprint`** | External binary, large, abandoned (wkhtmltopdf), packaging hell on portable Windows | `krilla` or Typst |
## Stack Patterns by Variant
- Resolve all paths from `std::env::current_exe()?.parent()?`:
- Use `tauri-plugin-single-instance` to prevent two .exe instances racing on the same SQLite file.
- Do **not** enable `tauri-plugin-updater`.
- SQLite `PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL; PRAGMA busy_timeout = 5000; PRAGMA foreign_keys = ON;` at connection-open time.
- axum binds to the configured host:port (default `0.0.0.0:8443` HTTPS, fall back to HTTP for first-run if cert generation hasn't happened).
- The Svelte SPA build is served by `tower-http::ServeDir` at `/`.
- A `/api/v1/*` namespace exposes the same operations as Tauri commands.
- All writes go through the same "data" service module — Tauri command handlers and axum HTTP handlers are thin adapters; the service module owns the single writer.
- Session middleware (`tower-sessions`) gates `/api/*` except `/api/v1/auth/login`.
- For local users: argon2id verify against stored hash. For AD users (later phase): `ldap3::Ldap::simple_bind(user_principal, password)` and inspect result code.
- One **writer connection** lives in a long-lived `tokio::task` that owns a `mpsc` queue of write jobs. All write paths (Tauri invoke, axum handler) push a job onto the queue and `await` a oneshot reply.
- **Reader connections** come from a small `r2d2`/`deadpool` pool (or hand-rolled `Arc<Mutex<Vec<Connection>>>`); reads do not contend with writes in WAL mode, so a pool of 4–8 readers is plenty.
- Both Tauri and axum import the same `data` crate — there is no duplication of business logic across transports.
- Use the NSIS installer (`bundle.targets.nsis`), `webviewInstallMode = "embedBootstrapper"`.
- Add `i686-pc-windows-msvc` to `rustup target add` in the CI matrix step.
- Test on a real Win7 VM — WebView2 on Win7 needs TLS 1.2 enabled, which is not the default on stock Win7 SP1.
- Document this in release notes as experimental.
- Enable `ldap3` feature `gssapi` (requires Clang + Kerberos libs at build time) or `ntlm` (raises MSRV to 1.85).
- This is a Windows-machine-only feature path; macOS/Linux dev boxes won't build it without extra setup. Gate it behind a Cargo feature.
## Version Compatibility
| Package A | Compatible With | Notes |
|-----------|-----------------|-------|
| `tauri ^2.11` | `wry ^0.55`, `tao ^0.35`, `tauri-build ^2`, `tauri-plugin-* ^2` | All on the `v2` line; do not mix v1 plugins. |
| `svelte ^5.55` | `vite ^6`, `svelte-preprocess ^6` or `@sveltejs/vite-plugin-svelte ^4` | Stable. Some older Svelte 4 community libs may not have updated APIs (check `bits-ui`/`shadcn-svelte` if used). |
| `rusqlite 0.39` | `libsqlite3-sys 0.37` (transitive, with `bundled` feature) | Bundled SQLite avoids platform variance. |
| `refinery 0.8` | `rusqlite 0.39` | `refinery` ships a matching driver feature flag. |
| `axum 0.8` | `tokio 1`, `tower 0.5`, `tower-http 0.6`, `hyper 1.x` | Tower 0.5 is the current line. |
| `tower-sessions 0.13` | `axum 0.8`, `tower 0.5` | Pluggable store; we likely roll our own rusqlite-backed store. |
| `ldap3 0.12` | Rust ≥ 1.82 (or 1.85 with NTLM); `tokio 1`, `rustls 0.23` if `tls-rustls` feature | NTLM bumps MSRV. |
| `snmp2 0.4` | `tokio 1`; `crypto-rust` avoids OpenSSL | Pure Rust crypto path is portable-friendly. |
| `krilla 0.7` | Rust ≥ 1.92 | High MSRV — verify against the toolchain pinned for Win7 32-bit if attempted. |
| `argon2 0.5` | RustCrypto stack (`password-hash 0.5`) | Stable. |
| `rustls 0.23` | `rustls-pemfile 2`, `rcgen 0.13` | All v0.23+ co-released. |
| `time 0.3` | `serde 1` | OK. |
## Critical Architectural Notes
### Dual access path (Tauri invoke + HTTP) must share business logic
### SQLite WAL + single-writer pattern (the most important pitfall)
### Russian Cyrillic in PDFs
### Portable mode discipline
## Sources
- [Tauri 2.0 Stable Release Blog](https://v2.tauri.app/blog/tauri-20/) — confirmed v2 GA Oct 2024
- [Tauri Core Releases](https://v2.tauri.app/release/) — current versions of tauri, wry, tao
- [Tauri Webview Versions](https://v2.tauri.app/reference/webview-versions/) — Windows 7 + WebView2 caveats
- [Tauri Windows Installer Docs](https://v2.tauri.app/distribute/windows-installer/) — NSIS vs MSI on Win7, `embedBootstrapper` mode
- [Tauri SvelteKit Frontend Setup](https://v2.tauri.app/start/frontend/sveltekit/) — confirms `adapter-static` + `ssr=false` if using Kit
- [Svelte Releases (GitHub)](https://github.com/sveltejs/svelte/releases) — Svelte 5.55+ current
- [Svelte 5 Runes Guide](https://www.pkgpulse.com/guides/svelte-5-runes-complete-guide-2026) — runes model
- [Tauri with Svelte vs SvelteKit (Medium / Tauri Discussion #5322)](https://github.com/tauri-apps/tauri/discussions/5322) — vanilla Svelte is sufficient for SPA
- [sqlx 0.9.0 docs (docs.rs)](https://docs.rs/sqlx/latest/sqlx/) — current sqlx version
- [PSA: Write Transactions are a Footgun with SQLx and SQLite (Evan Schwartz)](https://emschwartz.me/psa-write-transactions-are-a-footgun-with-sqlx-and-sqlite/) — the canonical rationale for `rusqlite` over `sqlx-sqlite`
- [rusqlite 0.39 docs](https://docs.rs/rusqlite/latest/rusqlite/) — current rusqlite + bundled feature
- [refinery (rust-db/refinery)](https://github.com/rust-db/refinery) — embedded migrations
- [axum 0.8.9 docs](https://docs.rs/axum/latest/axum/) — current axum version
- [Rust Web Frameworks 2025/2026 comparisons](https://aarambhdevhub.medium.com/rust-web-frameworks-in-2026-axum-vs-actix-web-vs-rocket-vs-warp-vs-salvo-which-one-should-you-2db3792c79a2) — axum vs actix-web tradeoffs
- [tower-sessions](https://github.com/maxcountryman/tower-sessions) + [tower-sessions-sqlx-store](https://lib.rs/crates/tower-sessions-sqlx-store) — session middleware
- [snmp2 0.4.9 docs](https://docs.rs/crate/snmp2/latest/features) — SNMP v1/v2/v3 + traps + getbulk
- [snmp2 GitHub (roboplc/snmp2)](https://github.com/roboplc/snmp2) — feature matrix
- [rasn-snmp](https://crates.io/crates/rasn-snmp) — alternative SNMP codec stack
- [modern_snmp](https://github.com/davedufresne/modern_snmp) — SNMPv3-only alternative
- [ldap3 0.12.1 docs](https://docs.rs/ldap3/latest/ldap3/) — AD bind, NTLM/GSSAPI features, MSRV
- [ldap3 GitHub (inejge/ldap3)](https://github.com/inejge/ldap3) — release notes for 0.10/0.11/0.12
- [krilla 0.7 (LaurenzV/krilla)](https://github.com/LaurenzV/krilla) — PDF lib with OpenType subsetting
- [Typst as a Rust library (Typst blog)](https://typst.app/blog/2025/automated-generation/) and [typst-as-lib](https://crates.io/crates/typst-as-lib) — backup PDF route
- [Rust: Multilingual PDFs intro (behainguyen)](https://behainguyen.wordpress.com/2025/11/11/rust-multilingual-pdfs-an-introductory-study/comment-page-1/) — Cyrillic/UTF-8 PDF pitfalls
- [argon2 (RustCrypto)](https://github.com/RustCrypto/password-hashes) — current `argon2` crate
- [Password Hashing Guide 2025/2026](https://guptadeepak.com/research/password-hashing-guide-2026/) — argon2id parameters and NIST guidance
- [tracing-appender docs](https://docs.rs/tracing-appender/latest/tracing_appender/) — rolling file logging
- [calamine GitHub](https://github.com/tafia/calamine) and [rust_xlsxwriter](https://crates.io/keywords/xlsx) — XLSX read/write libs
- [cargo-zigbuild](https://github.com/rust-cross/cargo-zigbuild) — cross-compile from macOS for Linux
- [Tauri GitHub Actions guide](https://v2.tauri.app/distribute/pipelines/github/) — official CI matrix
- [Ship Your Tauri v2 App Like a Pro (dev.to)](https://dev.to/tomtomdu73/ship-your-tauri-v2-app-like-a-pro-github-actions-and-release-automation-part-22-2ef7) — release pipeline example
- [Paraglide JS for SvelteKit (inlang)](https://inlang.com/m/dxnzrydw/paraglide-sveltekit-i18n) — future i18n choice
<!-- GSD:stack-end -->

<!-- GSD:conventions-start source:CONVENTIONS.md -->
## Conventions

Conventions not yet established. Will populate as patterns emerge during development.
<!-- GSD:conventions-end -->

<!-- GSD:architecture-start source:ARCHITECTURE.md -->
## Architecture

Architecture not yet mapped. Follow existing patterns found in the codebase.
<!-- GSD:architecture-end -->

<!-- GSD:skills-start source:skills/ -->
## Project Skills

No project skills found. Add skills to any of: `.claude/skills/`, `.agents/skills/`, `.cursor/skills/`, `.github/skills/`, or `.codex/skills/` with a `SKILL.md` index file.
<!-- GSD:skills-end -->

<!-- GSD:workflow-start source:GSD defaults -->
## GSD Workflow Enforcement

Before using Edit, Write, or other file-changing tools, start work through a GSD command so planning artifacts and execution context stay in sync.

Use these entry points:
- `/gsd-quick` for small fixes, doc updates, and ad-hoc tasks
- `/gsd-debug` for investigation and bug fixing
- `/gsd-execute-phase` for planned phase work

Do not make direct repo edits outside a GSD workflow unless the user explicitly asks to bypass it.
<!-- GSD:workflow-end -->



<!-- GSD:profile-start -->
## Developer Profile

> Profile not yet configured. Run `/gsd-profile-user` to generate your developer profile.
> This section is managed by `generate-claude-profile` -- do not edit manually.
<!-- GSD:profile-end -->

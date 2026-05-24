# Phase 1: Фундамент — Research

**Researched:** 2026-05-24
**Domain:** Rust workspace foundation (Tauri 2 + axum + SQLite-WAL + Svelte 5 SPA), portable-mode discipline, single-writer DB pattern, schema/migrations, CI with ProcMon-test
**Confidence:** HIGH overall — virtually every decision is locked in CONTEXT.md and the research file set; this document is the *implementation recipe* layer, not a re-investigation.

## Summary

Phase 1 is the foundation phase for Trackly: it establishes the 3-crate Cargo workspace (`trackly-core` / `trackly-infra` / `trackly-app` + `ui/` SPA), the portable-mode path discipline (`std::env::current_exe()` + `WEBVIEW2_USER_DATA_FOLDER` env var set in `main()` line 1), the full v1 SQLite schema (all 14 domain tables + `audit_log` / `counters` / `sessions` / `scheduled_tasks` cross-cutting tables) via refinery forward-only migrations, the single-writer mpsc channel + `spawn_blocking` worker pattern, the read-pool of 4 connections, the `AppCtx` shared by Tauri commands and axum handlers, the unified `AppError` (Serialize for Tauri, IntoResponse for axum), the `tauri-specta v2` bindings pipeline, structured tracing logs to `./logs/`, and the CI matrix with Windows-runner ProcMon-test that asserts zero writes outside `<exe_dir>`. **No UI in Phase 1** — only an empty Tauri shell that opens, runs migrations, opens pools, exposes one smoke command (`health`), and exits cleanly.

Every architectural decision is **locked** in `01-CONTEXT.md` (16 D-* decisions). The research here documents *how* to implement each one — exact crate APIs, version pins, file layouts, CI snippets, ProcMon CLI invocation, tauri-specta call patterns, refinery transaction semantics, `tower-sessions::SessionStore` trait surface for the rusqlite-backed store, and the WebView2 env-var timing recipe.

**Primary recommendation:** Carry every CONTEXT.md decision through to plans verbatim. Plan structure should split into **6 plans** (recommended): (P1) workspace + Cargo + CI fast scaffold; (P2) `paths.rs` + `config.rs` + WEBVIEW2 env-var; (P3) schema + migrations + connection PRAGMAs + downgrade-check; (P4) writer-channel + reader-pool + `AppCtx` + `Clock`/`Secret`/`AppError`; (P5) tauri-specta pipeline + `health` smoke + tracing/logging; (P6) ProcMon-check tool + CI full matrix + concurrent-writes test. Plans (P3) and (P4) can run in parallel after (P2). (P6) depends on (P5) producing a runnable binary with `--self-test` flag.

## User Constraints (from CONTEXT.md)

### Locked Decisions

**D-Schema-01: Идентификаторы — INTEGER PRIMARY KEY AUTOINCREMENT**
- Все таблицы используют `id INTEGER PRIMARY KEY AUTOINCREMENT` (rowid с гарантией монотонности и без переиспользования).
- Человеко-видимые номера (`act.number INTEGER`, `cartridge.code TEXT GENERATED AS '...'` или поле `code` + counter) — отдельные колонки, не PK.
- UUID v7 НЕ используем для PK в v1.
- Если в будущем понадобятся стабильные public-facing IDs, добавим `public_id TEXT UNIQUE` отдельной миграцией.

**D-Schema-02: Timestamps — INTEGER (unix seconds, UTC only)**
- Все timestamp-колонки: `INTEGER NOT NULL` (или `NULL` для опциональных), хранят unix epoch seconds.
- Колонки именуются с суффиксом `_at_utc`: `created_at_utc`, `updated_at_utc`, `deleted_at_utc`.
- В Rust: `time::OffsetDateTime` или `i64` (unix), сериализация через `serde_with::TimestampSeconds`.
- Запрет `chrono::Local::now()` через clippy `disallowed-methods`.

**D-Schema-03: Soft-delete — все user-mutable; system-таблицы — hard delete**
- Soft-delete (`deleted_at_utc INTEGER NULL`): `devices`, `acts`, `cartridges`, `cartridge_models`, `users`, `requests`, `document_templates`, `locations`.
- Hard delete: `audit_log`, `counters`, `sessions`, `scheduled_tasks`, `device_types`, `device_statuses`, `cartridge_states`, `cartridge_statuses`.
- `deleted_at_utc IS NULL` = «живая запись»; все SELECT по умолчанию фильтруют через helper в трейте репозитория.

**D-Schema-04: Optimistic lock — `version INTEGER NOT NULL DEFAULT 1`**
- На тех же сущностях, что и soft-delete.
- Инкремент через `UPDATE ... SET version = version + 1 WHERE id = ? AND version = ?`; 0 affected rows → `AppError::OptimisticLockMismatch`.
- Запись в `audit_log` — после успешного UPDATE, в той же транзакции.

**D-Schema-05: audit_log — полный before/after JSON, без отдельной ретенции в Phase 1**
- Схема: `id, entity_type, entity_id, action ('create'|'update'|'delete'|'restore'|'custom:*'), user_id NULL, before_json, after_json, payload_json, created_at_utc`.
- `before_json`/`after_json` — JSON всей записи (не diff).
- Индексы: `(entity_type, entity_id, created_at_utc)` и `(user_id, created_at_utc)`.
- Ретенция — не настраивается в v1.

**D-Workspace-01: Crate layout**
```
trackly/
├── Cargo.toml                 # workspace root
├── crates/
│   ├── trackly-core/          # домен + ports/traits + services. БЕЗ tokio, БЕЗ rusqlite.
│   ├── trackly-infra/         # adapters: SqliteRepos, paths.rs, refinery embed_migrations!
│   └── trackly-app/           # bin "trackly": tauri + axum + AppCtx + tracing + tauri-specta export
├── ui/                        # Svelte 5 SPA, vite root
├── migrations/                # refinery .sql files
├── .github/workflows/         # ci-fast.yml + ci-full.yml + cargo-deny.yml
└── tools/
    └── procmon-check/         # Windows-only утилита для CI ProcMon-теста
```
- Binary name: `trackly`. UI folder: `ui/` в корне. Tauri конфиг: `frontendDist = "../ui/dist"`.

**D-Workspace-02: tauri-specta — generate в `cargo test`, gitignored, pnpm prebuild**
- `trackly-app` экспортирует `#[tauri::command]` функции и `#[derive(specta::Type)]` DTO через `tauri_specta::collect_commands!` + `Builder::export`.
- Bindings: `ui/src/bindings.ts` через `cargo test --package trackly-app --test export_bindings`.
- `ui/src/bindings.ts` в `.gitignore`.
- `package.json` script: `"prebuild": "cargo test -p trackly-app --test export_bindings"`.
- Smoke-тест: `HealthDto { version: String, db_ready: bool, schema_version: u32 }` + `#[tauri::command] fn health(...)` + axum `GET /api/v1/health`, оба возвращают идентичный JSON.

**D-Migrations-01: split по доменам, refinery convention, seed — отдельной миграцией**
- Файлы: `V001__init_pragmas_and_lookups.sql` … `V012__indexes_and_fts.sql` (12 миграций).
- Forward-only. Refinery convention `V{n}__{description}.sql`, `embed_migrations!()` в `trackly-infra`.
- Seed в V001. Эволюция через `INSERT OR IGNORE`.
- Seed-rows перечислены явно (device_types: Устройство id=1, Принтер id=2; device_statuses: На складе/В работе/На ремонте/Списано; cartridge_states: Полный/Частичный/Пустой; cartridge_statuses: На складе/В работе/На заправке/Списано).

**D-Migrations-02: PRAGMA user_version + downgrade-protection**
- Каждая миграция оканчивается `PRAGMA user_version = N;`
- На старте: open write-conn → `PRAGMA user_version` → if > embedded → `AppError::DatabaseFromNewerVersion` graceful shutdown → if < → run refinery → if == → skip → open read pool.
- Тест восстановления (success criterion #4): fixture с user_version=999 → assert error + assert файл побайтово identical.

**D-WriterChannel-01: bounded mpsc capacity 256, backpressure через timeout**
- `tokio::sync::mpsc::channel::<WriteJob>(256)`.
- Writer-task: `tokio::task::spawn_blocking` с одним `rusqlite::Connection`, loop `while let Some(job) = rx.blocking_recv()`.
- Job-payload: `enum WriteJob { ... }` + `oneshot::Sender<Result<R, AppError>>`.
- Backpressure: `tx.send_timeout(job, Duration::from_secs(5))` → `AppError::WriteQueueBusy` (HTTP 503).
- Не unbounded — маскирует утечку памяти.

**D-AppError-01: единый flat enum, идентичный JSON shape в Tauri и axum**
- Один `AppError` в `trackly-core::error`, variants: `NotFound`, `Conflict`, `OptimisticLockMismatch`, `WriteQueueBusy`, `DatabaseFromNewerVersion`, `Validation`, `Unauthorized`, `Forbidden`, `Internal`.
- `impl Serialize`: `{ "code": "OPTIMISTIC_LOCK_MISMATCH", "message": "Ru-сообщение", "details": { ... } }`.
- `impl IntoResponse for AppError` (axum): mapping code → HTTP status.
- В Tauri: `#[tauri::command]` возвращает `Result<T, AppError>`.

**D-Config-01: `trackly.config.toml` (НЕ JSON), минимальный набор полей**
- Имя файла: `trackly.config.toml`. Рядом с .exe (или с БД).
- Секции: `[server]` (enabled, host, port, cert_path), `[paths]` (db_path), `[logging]` (level, format, retention_days), `[organization]` (timezone).
- Маркер портативности: `portable.txt` ИЛИ `trackly.config.toml` рядом с .exe.
- Парсинг: `toml` crate.

**D-Logging-01: tracing + tracing-appender, daily rotation, compact human по умолчанию**
- Subscriber: `tracing_subscriber::Registry::default().with(EnvFilter).with(fmt_layer).with(file_layer)`.
- Stdout: compact с цветами.
- File: `tracing_appender::rolling::daily("<exe_dir>/logs", "trackly.log")`, non-blocking; формат из `[logging.format]`.
- `WorkerGuard` на `AppCtx`, drop на graceful shutdown.
- Retention в Phase 7. Default level `info,hyper=warn,tower_http=warn`. Env override `TRACKLY_LOG`.

**D-CI-01: GitHub Actions — fast checks на каждый push, full matrix на PR + main**
- `ci-fast.yml`: ubuntu-latest, `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cd ui && pnpm install && pnpm svelte-check && pnpm lint`.
- `ci-full.yml`: matrix `[ubuntu-latest, macos-latest, windows-latest]`, full + `cargo build --release -p trackly-app`. Windows runner — ProcMon-тест.
- `cargo-deny` — отдельный workflow по `schedule`.
- Cache: `Swatinem/rust-cache@v2` + `actions/setup-node@v4` (pnpm cache).

**D-CI-02: clippy.toml — disallowed-methods list**
```toml
disallowed-methods = [
  { path = "dirs::data_dir", reason = "portable mode: use trackly_infra::paths" },
  { path = "dirs::data_local_dir", reason = "portable mode: use trackly_infra::paths" },
  { path = "dirs::config_dir", reason = "portable mode: use trackly_infra::paths" },
  { path = "dirs::cache_dir", reason = "portable mode: use trackly_infra::paths" },
  { path = "dirs::home_dir", reason = "portable mode: use trackly_infra::paths" },
  { path = "tauri::Manager::path", reason = "use trackly_infra::paths" },
  { path = "chrono::Local::now", reason = "UTC only; use time::OffsetDateTime::now_utc" },
  { path = "chrono::offset::Local::now", reason = "UTC only" },
  { path = "std::fs::copy", reason = "for DB backup use rusqlite::backup::Backup" },
]
disallowed-types = [
  { path = "chrono::DateTime<chrono::Local>", reason = "UTC only" },
]
```

**D-CI-03: ProcMon-тест — Windows-only, headless, через `procmon-check`**
- В `tools/procmon-check/`: создать sandbox `%TEMP%\trackly_procmon_<uuid>\` (с кириллицей в пути), скопировать `trackly.exe`, запустить ProcMon с фильтром на process name + WriteFile/CreateFile, запустить `trackly.exe --self-test`, остановить ProcMon, парсить CSV, assert нет записей вне sandbox.
- Test fixture: `%TEMP%\Документы\Trackly\` — покрывает cyrillic-path одновременно.

**D-Test-01: тестовая БД — tempfile per test (НЕ `:memory:`)**
- Хелпер `test_db()` в `trackly-infra::test_support`: `tempfile::NamedTempFile`, `rusqlite::Connection`, refinery-миграции.
- `test_app_ctx()` — полный AppCtx с tempfile-БД.
- Concurrent-тест: 25 task'ов (Tauri-invoke pattern) + 25 task'ов (axum handler pattern) через writer-канал, без `SQLITE_BUSY`.
- Не `:memory:` — не моделирует WAL.

### Claude's Discretion
- Точные имена fields в DTO (`HealthDto`) — на усмотрение планировщика.
- Структура `paths.rs` API (`Paths::db()`, `Paths::config()`, `Paths::logs_dir()`) — общая идея ясна, детали — у планировщика.
- Конкретный wire-format `AppError.details` — общий shape залочен, поля под domain — на усмотрение.
- Имена тестовых файлов и модулей.
- Конкретные индексы в `V012__indexes_and_fts.sql` (помимо очевидных PK/FK/UNIQUE) — планировщик решает по плану запросов из Phase 2+.

### Deferred Ideas (OUT OF SCOPE)
- Корзина UI поверх soft-delete — Phase 7.
- `device_custom_fields` — не добавляем в Phase 1.
- Логотип BLOB в БД (SET-02) — Phase 7.
- Backup retention policy + scheduled_tasks worker — Phase 7. Phase 1 создаёт `scheduled_tasks` таблицу, supervisor — позже.
- Cleanup audit_log retention — отложено.
- `activeCodePage=UTF-8` Windows manifest — Phase 8.
- mDNS `.local` hostname для HTTPS — Phase 5.
- `tauri-plugin-single-instance` — best practice уже в Phase 1, на усмотрение планировщика.

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| FOUND-01 | 3-крейтный workspace с границами (core без I/O) | `## Architecture Patterns` § Crate boundary rules; `D-Workspace-01` |
| FOUND-02 | SQLite WAL split-pool (write=1 через spawn_blocking, read=3-4) | `## Architecture Patterns` § Writer-channel + reader-pool; `D-WriterChannel-01`; rusqlite Code Example |
| FOUND-03 | Refinery forward-only + PRAGMA user_version | `## Standard Stack` § refinery; `D-Migrations-01`, `D-Migrations-02`; Refinery API in Code Examples |
| FOUND-04 | Портативный режим: БД+конфиг рядом с .exe, `portable.txt`, запрет `app_data_dir()` | `## Architecture Patterns` § paths.rs; `D-Config-01`; clippy `disallowed-methods` (D-CI-02) |
| FOUND-05 | `WEBVIEW2_USER_DATA_FOLDER` set before `tauri::Builder` | `## Code Examples` § WebView2 env-var; verified pattern from Tauri issue #1365 |
| FOUND-06 | `Secret<T>` newtype с кастомным Debug → `***` | `## Code Examples` § Secret<T>; cross-cutting Rust invariants |
| FOUND-07 | UTC timestamps только; форматирование через chrono-tz на UI | `D-Schema-02`; clippy `chrono::Local::now` ban (D-CI-02) |
| FOUND-08 | Seeded таблицы device_types/statuses/cartridge_states/statuses | `D-Migrations-01` seed-rows enumerated |
| FOUND-09 | created_at/updated_at/deleted_at/version на user-mutable | `D-Schema-03`, `D-Schema-04` |
| FOUND-10 | audit_log запись всех мутаций | `D-Schema-05` |
| FOUND-11 | ProcMon test в CI Windows runner | `D-CI-03`; ProcMon CLI invocation in Code Examples |
| FOUND-12 | tauri-specta v2 generate TypeScript из общих DTO | `D-Workspace-02`; tauri-specta v2 recipe in Code Examples |
| BLD-01 | GitHub Actions CI на push в main и PR | `D-CI-01` |
| BLD-06 | ProcMon test интегрирован в CI matrix | `D-CI-03` |

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Path resolution (portable detection) | `trackly-infra` (paths.rs) | `trackly-app` (consumes at startup) | I/O concern (`std::env::current_exe`, `std::fs::create_dir_all`) — belongs in infra; core has no FS dependency |
| DB schema (migrations) | `trackly-infra` (`migrations/*.sql` + embed_migrations!) | `trackly-app` (triggers run on startup) | Schema is an infra artifact; core knows only domain types |
| Writer-channel + worker task | `trackly-infra` (impl) → `trackly-app` (spawns) | `trackly-core` (defines `WriteJob` enum variants? — see below) | The worker uses rusqlite directly (infra). The job enum can live in core if jobs are domain-level operations; alternative is to keep jobs in infra and have services emit them. **Decision: jobs in infra; services in core depend on a `Writer` trait** |
| `AppError` enum | `trackly-core::error` | Implemented for both transports in `trackly-app` (Serialize via derive, IntoResponse handwritten in app) | Domain error must be portable; transport mappings live where the transports do |
| `Secret<T>`, `Clock` trait | `trackly-core::primitives` | Implementations (`SystemClock`) in `trackly-app` or `trackly-infra` | Pure types belong in core |
| Tauri commands | `trackly-app::tauri_cmds/` | Calls services from core | Transport adapter — thin |
| Axum handlers | `trackly-app::http/` | Calls services from core | Transport adapter — thin |
| `WEBVIEW2_USER_DATA_FOLDER` env-var set | `trackly-app::main` (line 1) | Reads value from `trackly-infra::paths` | Tauri-specific glue; must run before `tauri::Builder` per FOUND-05 |
| Tracing init | `trackly-app::main` | Uses paths from infra for log dir | Init order: paths → config → logging → DB → AppCtx → Tauri |
| tauri-specta export test | `trackly-app/tests/export_bindings.rs` | Imports DTO + commands from `trackly-app::dto` and `trackly-app::tauri_cmds` | Test binary owned by app crate; output is `ui/src/bindings.ts` |
| ProcMon-check tool | `tools/procmon-check/` (separate workspace member) | Invokes `trackly.exe --self-test` | Windows-only binary; CI consumes |

## Standard Stack

### Core (locked by CLAUDE.md + CONTEXT.md)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `tauri` | `2.11` | Desktop shell | Tauri 2 GA Oct 2024; v1 EOL [CITED: v2.tauri.app/blog/tauri-20] |
| `tauri-build` | `2` | Build script for Tauri | Companion to tauri |
| `wry` | `0.55` | Webview wrapper | Transitive via tauri 2.11 [CITED: v2.tauri.app/release] |
| `tao` | `0.35` | Windowing primitives | Transitive via tauri 2.11 [CITED: v2.tauri.app/release] |
| `tauri-plugin-fs` | `2` | File picker | Required for D-Config-01 settings; Phase 7 surface |
| `tauri-plugin-dialog` | `2` | Native open/save dialogs | Phase 7; safe to add now |
| `tauri-plugin-shell` | `2` | Open containing folder | Phase 7; safe to add now |
| `tauri-plugin-os` | `2` | OS info for diagnostics | Optional for Phase 1; recommend defer |
| `tauri-plugin-single-instance` | `2` | Prevent two .exe racing on same DB | Mentioned as best-practice in CONTEXT deferred list; **recommend INCLUDE in Phase 1** — addresses dev-time risk of two processes on tempfile DB |
| `tokio` | `1.x` (`rt-multi-thread`, `macros`, `signal`, `sync`, `fs`, `net`, `time`) | Async runtime | Required by every async lib |
| `axum` | `0.8` (`macros`, `ws`) | HTTP server | Phase 5 surface; **not exposed in Phase 1** but `AppCtx` is designed to receive it later |
| `tower` | `0.5` | Middleware base | Required transitively |
| `tower-http` | `0.6` (`fs`, `trace`, `cors`, `compression-gzip`, `limit`) | Middleware: tracing, static files | Phase 5 |
| `tower-sessions` | `0.13` | Session middleware | Phase 5; included in `Cargo.toml` if planner wants the rusqlite-backed store skeleton, otherwise defer |
| `rusqlite` | `0.39` (`bundled`, `chrono`, `serde_json`, `backup`) | SQLite driver | User-fixed; bundled = portable [CITED: docs.rs/rusqlite/0.39] |
| `refinery` | `0.8` (`rusqlite`) | Embedded migrations | Per CONTEXT D-Migrations-01 [CITED: github.com/rust-db/refinery] |
| `serde` | `1` (`derive`) | Serialization | DTO base |
| `serde_json` | `1` | JSON | Audit log before/after, axum bodies |
| `serde_with` | `3` (`macros`, `time_0_3`) | `TimestampSeconds` helper for `time::OffsetDateTime` ↔ unix-seconds JSON | Per D-Schema-02 |
| `thiserror` | `2` (or `1`) | Error derive for domain errors | `AppError` derives this |
| `anyhow` | `1` | Top-level error handling | Only at process boundaries (main, tests) |
| `tracing` | `0.1` | Structured logging | Per D-Logging-01 |
| `tracing-subscriber` | `0.3` (`env-filter`, `fmt`, `json`) | Subscriber | Per D-Logging-01 |
| `tracing-appender` | `0.2` | RollingFileAppender (daily) + non-blocking | Per D-Logging-01 |
| `time` | `0.3` (`serde`, `macros`, `formatting`, `parsing`) | Date/time arithmetic UTC | Per D-Schema-02; preferred over chrono |
| `toml` | `0.8` | Parse `trackly.config.toml` | Per D-Config-01 |
| `tempfile` | `3` | Test fixtures (dev-dep) | Per D-Test-01 |
| `tauri-specta` | `2.0.0-rc.21` (`typescript`, `derive`) | Type-safe Tauri command bindings | Per D-Workspace-02 [CITED: deepwiki.com/specta-rs/tauri-specta] |
| `specta` | `2.0.0-rc.22` | Backing type system | Companion to tauri-specta |
| `specta-typescript` | `0.0.9` (dev-dep) | TypeScript exporter for `cargo test` | Companion to tauri-specta |
| `tokio_util` | `0.7` (`sync`) | `CancellationToken` for graceful shutdown | Used by AppCtx |
| `zeroize` | `1` (`derive`) | Memory wiping for `Secret<T>` (optional but recommended) | FOUND-06; nice to wire from day 1 |

**Phase 1 deliberately defers:**
- `argon2` (Phase 5)
- `ldap3` (Phase 8)
- `snmp2` (Phase 6)
- `krilla` (Phase 3)
- `rustls`/`rcgen` (Phase 5)
- `tower-sessions` (Phase 5 — recommend leaving out of Phase 1 Cargo.toml entirely to avoid unused-dep noise; only `sessions` table schema lands in V010)

### Alternatives Considered (locked decisions — DO NOT revisit in Phase 1)
| Instead of | Could Use | Why we don't |
|------------|-----------|--------------|
| `rusqlite` + `refinery` | `sqlx` + `sqlx::migrate!` | sqlx-sqlite write-tx lock starvation footgun; rusqlite makes single-writer structural [CITED: emschwartz.me PSA] |
| Vanilla Svelte 5 | SvelteKit `adapter-static` | Hybrid Tauri+browser delivery; SvelteKit value subtracted |
| `time` crate | `chrono` | Banned `chrono::Local` via clippy; `time` is leaner for UTC-only |
| `toml` config | JSON | Hand-edited; comments allowed; no trailing-comma footgun |
| dedicated writer task | unbounded mpsc | Masks back-pressure memory leak |
| tempfile-per-test | `:memory:` | `:memory:` does not exercise WAL files |

**Version verification:** Versions above match CLAUDE.md's pinned table and current registry state as of 2026-05. The planner should run `cargo search <crate>` at plan time to confirm `0.39.x` is current for rusqlite and `0.8.x` for refinery before committing to `Cargo.toml`. (Refinery 0.8 confirmed via [docs.rs/refinery]; rusqlite 0.39 confirmed via [docs.rs/rusqlite].)

## Package Legitimacy Audit

> All packages listed below are pinned in CLAUDE.md by the user (Tauri/Rust ecosystem mainstays) or confirmed via authoritative sources (docs.rs, crates.io, GitHub orgs). slopcheck was not run in this research session — every entry below is `[VERIFIED via CLAUDE.md user pin]` or `[CITED: <authoritative source>]`. The planner SHOULD run slopcheck against the final `Cargo.toml` before commit; given that the user supplied these pins, no entries need to be removed.

| Package | Registry | Source Repo | Verification | Disposition |
|---------|----------|-------------|--------------|-------------|
| tauri | crates.io | tauri-apps/tauri (Apache-2.0/MIT) | CLAUDE.md pin + [v2.tauri.app] | Approved |
| tauri-build | crates.io | tauri-apps/tauri | Companion to tauri | Approved |
| tauri-plugin-* (fs, dialog, shell, os, single-instance) | crates.io | tauri-apps/plugins-workspace | CLAUDE.md | Approved |
| rusqlite | crates.io | rusqlite/rusqlite | CLAUDE.md pin + [docs.rs/rusqlite/0.39] | Approved |
| refinery | crates.io | rust-db/refinery | CLAUDE.md pin + [github.com/rust-db/refinery] | Approved |
| axum | crates.io | tokio-rs/axum | CLAUDE.md pin + [docs.rs/axum/0.8] | Approved |
| tokio | crates.io | tokio-rs/tokio | Ubiquitous | Approved |
| tower / tower-http | crates.io | tower-rs/tower | CLAUDE.md | Approved |
| tower-sessions | crates.io | maxcountryman/tower-sessions | CLAUDE.md | Approved (Phase 5; defer Cargo entry) |
| tauri-specta | crates.io | specta-rs/tauri-specta | [deepwiki.com/specta-rs/tauri-specta] + [github.com/specta-rs/tauri-specta] | Approved |
| specta | crates.io | specta-rs/specta | Companion to tauri-specta | Approved |
| specta-typescript | crates.io | specta-rs/specta | Companion exporter | Approved |
| tracing / tracing-subscriber / tracing-appender | crates.io | tokio-rs/tracing | Ubiquitous | Approved |
| time | crates.io | time-rs/time | CLAUDE.md preferred over chrono | Approved |
| toml | crates.io | toml-rs/toml | Ubiquitous | Approved |
| tempfile | crates.io | Stebalien/tempfile | Ubiquitous dev dep | Approved |
| serde / serde_json / serde_with | crates.io | serde-rs | Ubiquitous | Approved |
| thiserror / anyhow | crates.io | dtolnay/thiserror, dtolnay/anyhow | Ubiquitous | Approved |
| tokio_util | crates.io | tokio-rs/tokio | Ubiquitous | Approved |
| zeroize | crates.io | RustCrypto/utils | Ubiquitous | Approved |

**Packages removed due to slopcheck [SLOP] verdict:** none.
**Packages flagged as suspicious [SUS]:** none.

**Action for planner:** Run `slopcheck install <each package> --json` before committing root `Cargo.toml`. If any package returns `[SLOP]` or `[SUS]`, add `checkpoint:human-verify` task before the `cargo add` step.

## Architecture Patterns

### System Architecture Diagram

```
Entry: trackly.exe (Tauri 2)
   │
   ▼
main() — runs in this exact order:
  1. set_var("WEBVIEW2_USER_DATA_FOLDER", <exe_dir>/data/webview)        [FOUND-05]
  2. paths.rs: resolve <exe_dir>, sentinel detection (portable.txt | trackly.config.toml)
  3. config.rs: load trackly.config.toml or apply defaults
  4. tracing-appender setup → ./logs/trackly.log + stdout (WorkerGuard kept alive)
  5. Open WRITER rusqlite::Connection → PRAGMAs (WAL, busy_timeout=5000, ...)
  6. PRAGMA user_version check → graceful exit if file > binary
  7. refinery::Runner::run(&mut writer_conn)
  8. Move writer_conn into spawn_blocking worker; create mpsc<WriteJob>(256)
  9. Open 4 READER connections → same PRAGMAs, read_only=true
  10. Build AppCtx { writer_tx, reader_pool, paths, config, clock, _log_guard, cancel }
  11. If --self-test → close, exit(0). Else → tauri::Builder::default().manage(ctx).run()
   │
   ▼
AppCtx (cloneable; one struct serves both transports)
   │
   ├──► Tauri command adapters (#[tauri::command])
   │    └──► services (in trackly-core, generic over ports)
   │         ├──► reader pool (read queries)
   │         └──► writer_tx.send_timeout(WriteJob { ..., oneshot_reply })
   │              ▼
   │         WRITER worker (spawn_blocking)
   │              ▼
   │         rusqlite::Connection (exclusive — only writer in process)
   │              ▼
   │         trackly.db (+ -wal + -shm) in <exe_dir>
   │
   └──► axum handlers (Phase 5 — not in Phase 1 surface but design accommodates)
        └──► same services, same AppCtx, same writer_tx
```

### Recommended Project Structure
```
trackly/
├── Cargo.toml                     # [workspace] members = [...]
├── clippy.toml                    # disallowed-methods, disallowed-types (D-CI-02)
├── rustfmt.toml                   # optional; defaults are fine
├── rust-toolchain.toml            # pin Rust 1.85 (CLAUDE.md MSRV)
├── .gitignore                     # ui/src/bindings.ts, target/, ui/dist/, ui/node_modules/
├── deny.toml                      # cargo-deny config (advisories, licenses)
├── crates/
│   ├── trackly-core/
│   │   ├── Cargo.toml             # NO tokio, NO rusqlite, NO tauri; only serde, time, thiserror, async-trait
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── domain/            # entity types (later phases add Device, Act, etc.; Phase 1: empty or only shared primitives)
│   │       ├── error.rs           # AppError enum + Serialize derive
│   │       ├── primitives/
│   │       │   ├── secret.rs      # Secret<T> with custom Debug
│   │       │   └── clock.rs       # Clock trait (testability for "now")
│   │       └── ports/
│   │           └── writer.rs      # Writer trait (services use; impl in infra)
│   │
│   ├── trackly-infra/
│   │   ├── Cargo.toml             # rusqlite, refinery, tokio (writer task), tempfile (dev)
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── paths.rs           # Paths struct, portable detection, sentinel check
│   │       ├── config.rs          # AppConfig (toml de/serialize)
│   │       ├── db/
│   │       │   ├── mod.rs
│   │       │   ├── pragmas.rs     # apply_pragmas(&Connection) — WAL, busy_timeout, etc.
│   │       │   ├── pools.rs       # ReaderPool struct + open_reader / open_writer helpers
│   │       │   ├── writer_worker.rs   # mpsc loop, spawn_blocking, WriteJob handling
│   │       │   ├── migrations.rs  # embed_migrations!("../../migrations") + run + downgrade check
│   │       │   └── session_store.rs   # OMITTED in Phase 1 (lands in Phase 5)
│   │       ├── clock_impl.rs      # SystemClock impl of trackly-core::Clock
│   │       └── test_support/
│   │           ├── mod.rs
│   │           ├── test_db.rs     # tempfile-backed Connection + migrations
│   │           └── test_app_ctx.rs# AppCtx with tempfile DB + in-process channels
│   │
│   └── trackly-app/
│       ├── Cargo.toml             # tauri, tauri-specta, all the above
│       ├── tauri.conf.json        # frontendDist = "../../ui/dist", devUrl = "http://localhost:1420"
│       ├── build.rs               # tauri_build::build()
│       ├── icons/                 # placeholder PNGs OK for Phase 1
│       └── src/
│           ├── main.rs            # ordered init (see diagram); parses --self-test
│           ├── context.rs         # AppCtx struct + construction
│           ├── shutdown.rs        # CancellationToken plumbing
│           ├── logging.rs         # tracing-subscriber setup
│           ├── webview_env.rs     # set_var("WEBVIEW2_USER_DATA_FOLDER", ...)
│           ├── dto.rs             # HealthDto + future shared types (serde::Serialize/Deserialize + specta::Type)
│           ├── error_axum.rs      # impl IntoResponse for AppError (lives in app to avoid pulling axum into core)
│           ├── tauri_cmds/
│           │   ├── mod.rs
│           │   └── health.rs      # #[tauri::command] fn health(state) -> HealthDto
│           └── specta_export.rs   # builder + commands list (called from tests)
│       └── tests/
│           ├── export_bindings.rs # cargo test triggers ui/src/bindings.ts regeneration
│           ├── concurrent_writes.rs # 25+25 writers, no SQLITE_BUSY (success criterion #2)
│           ├── downgrade_protection.rs # user_version=999 fixture, assert error + file unchanged (success criterion #4)
│           └── health_smoke.rs    # invoke health via Tauri-style and via direct service call, assert identical JSON
│
├── ui/
│   ├── package.json               # "prebuild": "cargo test -p trackly-app --test export_bindings"
│   ├── vite.config.ts             # vitePreprocess + scss preprocessor
│   ├── tsconfig.json
│   ├── index.html                 # minimal placeholder
│   └── src/
│       ├── main.ts                # minimal Svelte 5 mount (Phase 2 fills in)
│       ├── App.svelte             # placeholder
│       ├── bindings.ts            # gitignored, generated
│       └── styles/_tokens.scss    # design tokens placeholder
│
├── migrations/
│   ├── V001__init_pragmas_and_lookups.sql
│   ├── V002__core_entities.sql       # users, locations
│   ├── V003__devices.sql
│   ├── V004__acts.sql                # parent_act_id + sub_number
│   ├── V005__cartridges.sql          # cartridge_models, cartridges + cartridge_seq
│   ├── V006__requests.sql
│   ├── V007__document_templates.sql
│   ├── V008__audit_log.sql
│   ├── V009__counters.sql            # generic numbering
│   ├── V010__sessions.sql            # tower-sessions backend (schema only in Phase 1)
│   ├── V011__scheduled_tasks.sql
│   └── V012__indexes_and_fts.sql     # FTS5 virtual tables + cross-table indexes
│
├── tools/
│   └── procmon-check/
│       ├── Cargo.toml                # Windows-only via [target.'cfg(windows)'.dependencies]
│       └── src/main.rs               # spawn ProcMon, run trackly.exe --self-test, parse CSV, assert
│
└── .github/
    └── workflows/
        ├── ci-fast.yml               # push to any branch; ubuntu only
        ├── ci-full.yml               # PR + push to main; matrix; Windows = ProcMon test
        └── cargo-deny.yml            # daily cron
```

### Pattern 1: Single-Writer mpsc Channel (THE central pattern)

**What:** All writes serialize through one `tokio::sync::mpsc::channel::<WriteJob>(256)` whose receiver is owned by a `tokio::task::spawn_blocking` worker that owns one `rusqlite::Connection`. Callers (Tauri commands, axum handlers, background tasks) send a `WriteJob` containing a `tokio::sync::oneshot::Sender<Result<R, AppError>>` and `await` the reply.

**When to use:** Every write to SQLite in this project. No exceptions.

**Example:**
```rust
// trackly-core/src/ports/writer.rs
use async_trait::async_trait;
use crate::error::AppError;

// Writer trait keeps trackly-core free of tokio/rusqlite.
// Services depend on this trait; the concrete impl lives in trackly-infra.
#[async_trait]
pub trait Writer: Send + Sync {
    async fn execute<F, R>(&self, job: F) -> Result<R, AppError>
    where
        F: FnOnce(&mut rusqlite::Connection) -> Result<R, AppError> + Send + 'static,
        R: Send + 'static;
}
// NOTE: leaking `rusqlite::Connection` into the trait pulls rusqlite into core's deps.
// ALTERNATIVE pattern (preferred): make `WriteJob` an enum of named operations,
// `Writer::send(job)` returns the response, and only infra knows rusqlite. The planner
// chooses one of these two shapes; both satisfy the locked decisions.

// trackly-infra/src/db/writer_worker.rs
use tokio::sync::{mpsc, oneshot};
use std::time::Duration;
use rusqlite::Connection;
use crate::error::AppError;

pub struct WriterHandle {
    tx: mpsc::Sender<Box<dyn FnOnce(&mut Connection) + Send>>,
}

impl WriterHandle {
    pub fn spawn(mut conn: Connection) -> Self {
        let (tx, mut rx) = mpsc::channel::<Box<dyn FnOnce(&mut Connection) + Send>>(256);
        tokio::task::spawn_blocking(move || {
            while let Some(job) = rx.blocking_recv() {
                job(&mut conn);
            }
            // rx closed → graceful exit
        });
        Self { tx }
    }

    pub async fn execute<F, R>(&self, op: F) -> Result<R, AppError>
    where
        F: FnOnce(&mut Connection) -> Result<R, AppError> + Send + 'static,
        R: Send + 'static,
    {
        let (reply_tx, reply_rx) = oneshot::channel();
        let job = Box::new(move |conn: &mut Connection| {
            let result = op(conn);
            let _ = reply_tx.send(result);
        });
        self.tx
            .send_timeout(job, Duration::from_secs(5))
            .await
            .map_err(|_| AppError::WriteQueueBusy)?;
        reply_rx.await.map_err(|_| AppError::Internal {
            source_chain: "writer task dropped reply channel".into(),
        })?
    }
}
```

### Pattern 2: AppCtx — single cloneable handle

```rust
// trackly-app/src/context.rs
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing_appender::non_blocking::WorkerGuard;

#[derive(Clone)]
pub struct AppCtx {
    pub writer: Arc<trackly_infra::db::WriterHandle>,
    pub readers: Arc<trackly_infra::db::ReaderPool>,
    pub paths: Arc<trackly_infra::Paths>,
    pub config: Arc<trackly_infra::AppConfig>,
    pub clock: Arc<dyn trackly_core::Clock>,
    pub shutdown: CancellationToken,
    pub _log_guard: Arc<WorkerGuard>,   // drop on shutdown
    pub schema_version: u32,
}
```

### Pattern 3: PRAGMA application order at connection open

```rust
// trackly-infra/src/db/pragmas.rs
use rusqlite::Connection;
use crate::error::AppError;

pub fn apply_writer_pragmas(conn: &Connection) -> Result<(), AppError> {
    // Order matters: journal_mode is the only PRAGMA that persists to the file header
    // (so WAL mode is "sticky" once set). The rest are per-connection and must be set
    // on every open.
    conn.pragma_update(None, "journal_mode", &"WAL")?;
    conn.pragma_update(None, "synchronous", &"NORMAL")?;
    conn.pragma_update(None, "busy_timeout", 5000)?;
    conn.pragma_update(None, "foreign_keys", true)?;
    conn.pragma_update(None, "wal_autocheckpoint", 1000)?;
    conn.pragma_update(None, "temp_store", &"MEMORY")?;
    conn.pragma_update(None, "mmap_size", 134_217_728_i64)?; // 128 MB
    Ok(())
}

pub fn apply_reader_pragmas(conn: &Connection) -> Result<(), AppError> {
    // Read-only connection: open with SQLITE_OPEN_READ_ONLY; still apply WAL et al.
    conn.pragma_update(None, "busy_timeout", 5000)?;
    conn.pragma_update(None, "foreign_keys", true)?;
    conn.pragma_update(None, "temp_store", &"MEMORY")?;
    conn.pragma_update(None, "mmap_size", 134_217_728_i64)?;
    Ok(())
}
```

**Caveats:**
- `journal_mode=WAL` persists in the file header, so a second open of the same file inherits WAL. Setting it again is a no-op.
- `query_row` is required to read the result of `pragma_update` for some pragmas in rusqlite; use `conn.pragma_query_value(None, "journal_mode", |r| r.get(0))` to confirm.
- Open reader connections with `OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX` so they cannot accidentally write.

### Pattern 4: tauri-specta v2 generated-in-test pipeline

See `## Code Examples` for the full recipe.

### Anti-Patterns to Avoid
- **Opening DB inside a Tauri command handler** — use `AppCtx.writer`/`AppCtx.readers`.
- **Calling `dirs::*_dir()` anywhere** — banned by clippy (D-CI-02). Use `trackly_infra::paths::Paths`.
- **Holding a write transaction across an `.await` on I/O** — even though writer worker uses `spawn_blocking`, any `await` inside the closure breaks the model. Closures passed to `Writer::execute` must be pure sync rusqlite code.
- **Running migrations on the read pool** — refinery would fail (read-only). Run before opening readers.
- **Using `:memory:` in tests that touch concurrency** — does not exercise WAL semantics (D-Test-01).
- **Putting business logic in Tauri command files** — handlers are 5–15 line adapters per ARCHITECTURE.md Pattern 1.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Embedded SQL migrations | Hand-roll a SQL file runner | `refinery 0.8` with `embed_migrations!("../../migrations")` | Forward-only, checksum-tracked, transaction-wrapped per migration, recorded in `refinery_schema_history` table |
| TS bindings from Rust DTOs | Hand-write `.d.ts` files | `tauri-specta 2.0.0-rc.21` + `specta-typescript 0.0.9` | Transitive dependency tracking; one round-trip type system [CITED: deepwiki.com/specta-rs/tauri-specta]. ts-rs is explicitly rejected in ARCHITECTURE.md |
| Async daily log rotation | Roll your own appender + non-blocking writer | `tracing-appender::rolling::daily(...).with_writer(non_blocking)` | Battle-tested, integrates with tracing-subscriber |
| Portable-path detection | If/else on env vars | A single `Paths` struct rooted at `std::env::current_exe()?.parent()?` with sentinel check on `portable.txt`/`trackly.config.toml` | One place to test; clippy-banned alternatives |
| Single-writer enforcement | Hope `Mutex<Connection>` is enough | `mpsc<WriteJob>(256)` + `spawn_blocking` worker | `Mutex` blocks the calling tokio task synchronously; can wedge the runtime under contention |
| Cancellation propagation | Bool flag + polling | `tokio_util::sync::CancellationToken` | Cooperative cancel, async-aware, child-token semantics |
| Sentinel-based portable detection | Probe writability of `<exe_dir>` | Sentinel file (`portable.txt` OR `trackly.config.toml`) per ARCHITECTURE.md | Writability heuristic gives false positives on admin-elevated Program Files |
| Backup file copy | `std::fs::copy("trackly.db", "backup.db")` | `rusqlite::backup::Backup` (Phase 7) — but `std::fs::copy` is **clippy-banned** at workspace scope from Phase 1 to prevent regression | WAL-aware backup |
| Custom JSON error shape per transport | Different shapes for Tauri vs HTTP | One `AppError` enum, single `impl Serialize` (Tauri), single `impl IntoResponse` (axum) | Frontend has one error parser; PITFALLS #8 |

**Key insight:** every custom solution in this list is a multi-week rewrite when it goes wrong in production. Phase 1's job is to lock the right libraries in `Cargo.toml` so later phases never have to reach for the hand-rolled versions.

## Runtime State Inventory

This is a greenfield phase — no existing runtime state to migrate. Section retained for downstream phases that touch existing data.

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None — first phase, no existing DB | None |
| Live service config | None — no deployed services | None |
| OS-registered state | None — no installer artifacts yet | None |
| Secrets / env vars | None for Phase 1; `TRACKLY_LOG` env-var is *consumed* but not stored anywhere | Document in README that `TRACKLY_LOG` overrides log level |
| Build artifacts | None pre-existing | Phase 1 emits `target/`, `ui/dist/`, `ui/node_modules/`, `ui/src/bindings.ts` — all gitignored |

**Nothing found in any category** — verified by `git status` (only `.planning/` artifacts present in repo).

## Common Pitfalls

(Cross-referenced to `.planning/research/PITFALLS.md` — Phase 1 prevention items in detail)

### Pitfall 1: WebView2 silently writes to `%LOCALAPPDATA%\<app>\EBWebView`
**What goes wrong:** Even with `portable.txt` next to .exe, WebView2 defaults its user-data folder to `%LOCALAPPDATA%`. Verifying portability with explorer.exe looks fine — ProcMon reveals the leak.
**Why:** Tauri does not redirect `WEBVIEW2_USER_DATA_FOLDER` automatically. The env-var read happens inside WebView2 initialization, which is triggered by `tauri::Builder::default().run(...)`.
**How to avoid:** `std::env::set_var("WEBVIEW2_USER_DATA_FOLDER", paths.webview_data_dir())` MUST be the **first** non-trivial statement in `main()` — before tracing-subscriber setup, before any tokio runtime, before any `tauri::*` call. The path must already be UTF-8 and exist (`create_dir_all` it).
**Warning signs:** ProcMon-check fails with paths under `\AppData\Local\trackly\EBWebView\`. The `data/webview/` folder next to .exe stays empty after first launch.
**Verification:** D-CI-03 ProcMon-test in CI Windows runner directly catches this.

### Pitfall 2: SQLite "database is locked" under concurrent writers
**What goes wrong:** Two transports submit writes simultaneously; one fails with `SQLITE_BUSY`.
**Why:** WAL allows many readers + one writer. Two concurrent writers without serialization race.
**How to avoid:** Locked by D-WriterChannel-01. All writes go through `WriterHandle::execute` → `mpsc<WriteJob>(256)` → single worker → single `rusqlite::Connection`. `busy_timeout=5000ms` is a belt-and-suspenders safety net.
**Warning signs:** Any `database is locked` error in `concurrent_writes.rs` test under load.
**Verification:** Success criterion #2 — 50 concurrent jobs from two transports, zero failures.

### Pitfall 3: Cyrillic Windows paths break SQLite/WebView2/file APIs
**What goes wrong:** `C:\Документы\Учёт\Trackly\` works in some crates, not others. Path crosses a `&str` boundary, `.to_str()` returns `None`, app reports "file not found" with `�` chars.
**Why:** Windows OsStr is WTF-8/UTF-16; Rust `Path::to_str()` validates UTF-8.
**How to avoid:**
- Always `&Path` / `PathBuf` in module boundaries (not `&str`).
- `WEBVIEW2_USER_DATA_FOLDER` value: the env-var system on Windows accepts UTF-16 via `SetEnvironmentVariableW`; Rust's `std::env::set_var` calls the W variant on Windows since 1.0 — safe.
- Verify with the D-CI-03 ProcMon-test using cyrillic sandbox path (`%TEMP%\Документы\Trackly\`).
- Defer the `activeCodePage=UTF-8` manifest setting to Phase 8 unless CI fails — Rust file APIs work without it for our use cases.

### Pitfall 4: `journal_mode=WAL` not persisted on first open
**What goes wrong:** First open of a fresh DB starts in `journal_mode=delete`. The PRAGMA upgrade to WAL succeeds, but if the connection is closed before any write transaction, the mode does not persist.
**Why:** SQLite writes the WAL mode bit to the file header during the next write transaction.
**How to avoid:** After applying writer PRAGMAs, run `CREATE TABLE IF NOT EXISTS __init_marker (id INTEGER PRIMARY KEY);` or run the first refinery migration immediately. This forces a write that persists WAL.
**Warning signs:** A second open of the file reads `journal_mode=delete` despite the first open claiming success.
**Verification:** Test fixture: open writer → close → reopen → assert `pragma_query_value("journal_mode")` == "wal".

### Pitfall 5: Refinery migration failure leaves partial state
**What goes wrong:** Migration V005 fails mid-way; some V001–V004 changes are committed, V005 partially applied.
**Why:** By default `refinery 0.8` wraps each migration in its own transaction [CITED: docs.rs/refinery Runner::set_grouped] — V005 rolls back cleanly but V001-V004 stay. The next launch sees V001–V004 done and retries V005. **This is correct behavior** but the planner must verify failed migrations are recoverable (idempotent enough to retry).
**How to avoid:**
- Keep each migration idempotent within its own scope.
- Consider `runner().set_grouped(true)` to wrap **all** migrations in one transaction — but this fails noisily if any single migration is non-trivial in size (SQLite holds the write lock for the whole batch).
- **Recommend per-migration transactions (default)** + verify each `.sql` file is `CREATE TABLE IF NOT EXISTS`-friendly.
**Warning signs:** `refinery_schema_history` table shows version mismatch with `PRAGMA user_version` (we run both, so they must match).
**Verification:** Downgrade-protection test (success criterion #4) catches the inverse case.

### Pitfall 6: Background tracing-appender worker dropped before logs flush
**What goes wrong:** `WorkerGuard` from `tracing_appender::non_blocking` is dropped at end of `main()` setup, async writes get cut off, last 100ms of logs vanish.
**Why:** `non_blocking` returns a `(NonBlocking, WorkerGuard)` tuple; the `WorkerGuard` must outlive every `tracing` call.
**How to avoid:** Store `Arc<WorkerGuard>` on `AppCtx`, drop on graceful shutdown explicitly. Avoid leaking the guard with `std::mem::forget` — proper drop on shutdown ensures buffered logs flush.
**Warning signs:** Last entries of `trackly.log` are truncated after a clean `Ctrl+C` exit.

### Pitfall 7: tauri-specta export drift between Rust DTOs and TS bindings
**What goes wrong:** Developer adds a field to a DTO, forgets to run `cargo test`; `ui/src/bindings.ts` stale; svelte-check passes locally but fails in CI.
**Why:** Export is gated on `cargo test --test export_bindings`. If `pnpm prebuild` is not run before `pnpm dev`, stale bindings are used.
**How to avoid:**
- `package.json` `prebuild` script: `"prebuild": "cargo test -p trackly-app --test export_bindings"` — runs before `vite build`.
- For `pnpm dev`: have `predev` or instruct devs to run `pnpm prebuild` once after Rust changes. (Optionally use `concurrently` to run `cargo watch -x 'test -p trackly-app --test export_bindings'` alongside dev.)
- CI runs `cargo test` first; if drift exists, svelte-check fails on bindings.ts symbol changes — visible early.
**Warning signs:** svelte-check reports `Property 'xxx' does not exist on type 'YyyDto'`.

### Pitfall 8: `std::env::set_var` is unsafe in Rust 2024
**What goes wrong:** Rust 2024 edition (and recent stable since ~1.83) marked `std::env::set_var` as `unsafe` (audit-tracked); using it in safe code triggers a warning, in 2024 edition it requires `unsafe { ... }`.
**Why:** Race conditions with other threads reading env vars. In our case it's safe because we call it as the first thing in `main()` before spawning any threads.
**How to avoid:** Wrap in `unsafe { std::env::set_var(...) }` and add a `// SAFETY: called before tokio runtime / Tauri / any thread spawn` comment. Pin Rust to 1.85 per CLAUDE.md so behavior is consistent.
**Warning signs:** Compile error or warning about unsafe env var mutation on Rust 1.83+.

## Code Examples

### Code Example 1: WebView2 env-var + portable path resolution in `main()`

```rust
// trackly-app/src/main.rs
fn main() -> anyhow::Result<()> {
    // ─── Step 1: resolve paths (cannot fail; falls back to OS dir only as last resort) ───
    let paths = trackly_infra::Paths::resolve()?;

    // ─── Step 2: set WEBVIEW2_USER_DATA_FOLDER before anything else (FOUND-05) ───
    // SAFETY: called before any thread spawn, tokio runtime, or tauri::Builder.
    // No other thread can be reading env vars yet.
    unsafe {
        std::env::set_var("WEBVIEW2_USER_DATA_FOLDER", paths.webview_data_dir());
    }

    // ─── Step 3: parse CLI flags (--self-test) ───
    let self_test = std::env::args().any(|a| a == "--self-test");

    // ─── Step 4: load config ───
    let config = trackly_infra::AppConfig::load_or_default(&paths.config_file())?;

    // ─── Step 5: tracing-subscriber + tracing-appender ───
    let _log_guard = trackly_app::logging::init(&paths, &config)?;

    // ─── Step 6+: tokio runtime — Tauri provides one; --self-test path needs its own ───
    if self_test {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;
        rt.block_on(async move {
            let ctx = trackly_app::context::AppCtx::build(paths, config, _log_guard).await?;
            tracing::info!(schema_version = ctx.schema_version, "self-test completed");
            ctx.shutdown.cancel();
            Ok::<_, anyhow::Error>(())
        })?;
        return Ok(());
    }

    // Normal launch: hand control to Tauri (it provides the tokio runtime)
    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let ctx = trackly_app::context::AppCtx::build(paths, config, _log_guard).await?;
                handle.manage(ctx);
                Ok::<_, anyhow::Error>(())
            });
            Ok(())
        })
        .invoke_handler(trackly_app::specta_export::invoke_handler())
        .run(tauri::generate_context!())?;

    Ok(())
}
```

### Code Example 2: refinery migration runner with downgrade protection

```rust
// trackly-infra/src/db/migrations.rs
use rusqlite::Connection;
use refinery::embed_migrations;
use crate::error::AppError;

embed_migrations!("../../migrations");

pub struct MigrationReport {
    pub schema_version: u32,
    pub applied_count: usize,
}

pub fn run(conn: &mut Connection) -> Result<MigrationReport, AppError> {
    // Step 1: read on-disk user_version
    let on_disk: u32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;

    // Step 2: determine what refinery would migrate TO
    let runner = migrations::runner();
    let known: u32 = runner
        .get_migrations()
        .iter()
        .map(|m| m.version())
        .max()
        .unwrap_or(0) as u32;

    // Step 3: downgrade protection (success criterion #4)
    if on_disk > known {
        return Err(AppError::DatabaseFromNewerVersion {
            binary: known,
            file: on_disk,
        });
    }

    // Step 4: run (each migration in its own tx by default; do NOT set_grouped(true))
    let report = runner.run(conn)?;

    // Step 5: confirm user_version matches refinery_schema_history
    let after: u32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    debug_assert_eq!(after, known, "PRAGMA user_version drift vs embedded migrations");

    Ok(MigrationReport {
        schema_version: after,
        applied_count: report.applied_migrations().len(),
    })
}
```

### Code Example 3: tauri-specta v2 — collect_commands + Builder + export-in-test

```rust
// trackly-app/src/dto.rs
use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct HealthDto {
    pub version: String,
    pub db_ready: bool,
    pub schema_version: u32,
}

// trackly-app/src/tauri_cmds/health.rs
use crate::context::AppCtx;
use crate::dto::HealthDto;
use crate::error::AppError;

#[tauri::command]
#[specta::specta]
pub async fn health(state: tauri::State<'_, AppCtx>) -> Result<HealthDto, AppError> {
    Ok(HealthDto {
        version: env!("CARGO_PKG_VERSION").into(),
        db_ready: true,
        schema_version: state.schema_version,
    })
}

// trackly-app/src/specta_export.rs
use tauri_specta::{Builder, collect_commands};

pub fn builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .commands(collect_commands![crate::tauri_cmds::health::health])
}

// Wired into tauri::Builder::invoke_handler at runtime (`fn main()`):
pub fn invoke_handler() -> impl Fn(tauri::ipc::Invoke<tauri::Wry>) -> bool + Send + Sync + 'static {
    builder().build().unwrap().invoke_handler()
}

// trackly-app/tests/export_bindings.rs
#[test]
fn export_bindings() {
    use specta_typescript::Typescript;

    crate::specta_export::builder()
        .export(
            Typescript::default(),
            "../../ui/src/bindings.ts",
        )
        .expect("failed to export TypeScript bindings");
}
// NOTE for planner: `tests/` cannot use `crate::` to refer to the binary crate.
// Either:
//   (a) move `specta_export` and command modules into a library target alongside the binary
//       in trackly-app/Cargo.toml: `[lib] path = "src/lib.rs"` + binary stays at `src/main.rs`
//   (b) duplicate the builder in the test
// Recommend (a). Confirmed pattern from tauri-specta docs.
```

**Required `trackly-app/Cargo.toml` excerpt:**
```toml
[dependencies]
tauri = { version = "2", features = ["devtools"] }
tauri-specta = { version = "=2.0.0-rc.21", features = ["typescript", "derive"] }
specta = "=2.0.0-rc.22"
serde = { version = "1", features = ["derive"] }
# ... (rest)

[dev-dependencies]
specta-typescript = "0.0.9"
tempfile = "3"
```

(Source: [deepwiki.com/specta-rs/tauri-specta § Getting Started] + [github.com/specta-rs/tauri-specta])

### Code Example 4: Secret<T> newtype

```rust
// trackly-core/src/primitives/secret.rs
use std::fmt;
use zeroize::Zeroize;

#[derive(Clone)]
pub struct Secret<T: Zeroize + Clone>(T);

impl<T: Zeroize + Clone> Secret<T> {
    pub fn new(value: T) -> Self { Self(value) }
    pub fn expose(&self) -> &T { &self.0 }
}

impl<T: Zeroize + Clone> fmt::Debug for Secret<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("***")
    }
}

impl<T: Zeroize + Clone> Drop for Secret<T> {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

// Serde — never auto-derive Serialize/Deserialize on Secret:
// require explicit "I know what I'm doing" wrappers in DTOs.
```

### Code Example 5: ProcMon CLI invocation for `procmon-check` tool

```bash
# Step 1 — start capture (PML file is the native binary format)
procmon.exe /AcceptEula /Minimized /Quiet /Runtime 30 \
  /BackingFile "%TEMP%\trackly_procmon_<uuid>\trace.pml" \
  /LoadConfig "tools\procmon-check\filter.pmc"

# Step 2 — run the app under test (in a separate process / shell)
"%TEMP%\trackly_procmon_<uuid>\trackly.exe" --self-test

# Step 3 — terminate ProcMon cleanly (or rely on /Runtime expiry)
procmon.exe /Terminate

# Step 4 — convert PML to CSV with filter applied
procmon.exe /OpenLog "%TEMP%\trackly_procmon_<uuid>\trace.pml" \
  /LoadConfig "tools\procmon-check\filter.pmc" \
  /SaveAs "%TEMP%\trackly_procmon_<uuid>\trace.csv" \
  /SaveApplyFilter
```

**Flag reference (verified via [learn.microsoft.com] + community sources):**
- `/AcceptEula` — bypass first-run EULA prompt
- `/Quiet` — suppress filter dialog
- `/Minimized` — start hidden
- `/BackingFile <path>` — log to file (PML format)
- `/LoadConfig <path>` — apply filter (PMC file)
- `/Runtime <seconds>` — auto-terminate after N seconds
- `/Terminate` — kill all ProcMon instances
- `/OpenLog <path>` — open existing PML for re-export
- `/SaveAs <path>` — export to CSV/XML/PML by extension
- `/SaveApplyFilter` — apply current filter during export

**Filter file (`filter.pmc`) recommendation for our use case:**
Create programmatically via ProcMon UI (Filter → Export PMC). Filter must include:
- `Process Name == trackly.exe` (Include)
- `Operation == WriteFile` (Include)
- `Operation == CreateFile` with `Detail` containing write access (Include)
- All other entries (Exclude)

In the `procmon-check` Rust tool, after CSV parse:
```rust
// pseudocode
let csv = std::fs::read_to_string(&csv_path)?;
let allowed_prefix = sandbox_dir.to_string_lossy().to_ascii_uppercase();
let forbidden = ["APPDATA", "LOCALAPPDATA", "\\APPDATA\\", "\\PROGRAMDATA\\"];
for row in csv.lines().skip(1) {
    let path_field = extract_path(row);
    let upper = path_field.to_ascii_uppercase();
    if upper.starts_with(&allowed_prefix) { continue; }
    if upper.contains("\\TEMP\\") { continue; } // OS temp files OK
    for bad in &forbidden {
        if upper.contains(bad) {
            return Err(format!("portable leak: {row}"));
        }
    }
}
```

**Downloading ProcMon in GitHub Actions Windows runner:**
```yaml
- name: Download Process Monitor
  shell: pwsh
  run: |
    Invoke-WebRequest -Uri https://download.sysinternals.com/files/ProcessMonitor.zip -OutFile ProcessMonitor.zip
    Expand-Archive ProcessMonitor.zip -DestinationPath C:\ProcMon
    echo "C:\ProcMon" | Out-File -FilePath $env:GITHUB_PATH -Append
```

### Code Example 6: tower-sessions custom SessionStore (Phase 5 — schema only in Phase 1)

```rust
// REFERENCE for Phase 5 — not implemented in Phase 1.
// trackly-infra/src/db/session_store.rs
use async_trait::async_trait;
use tower_sessions::{
    session::{Id, Record},
    session_store::{Result, SessionStore},
};

#[derive(Clone)]
pub struct RusqliteSessionStore {
    writer: std::sync::Arc<crate::db::WriterHandle>,
    readers: std::sync::Arc<crate::db::ReaderPool>,
}

#[async_trait]
impl SessionStore for RusqliteSessionStore {
    async fn create(&self, record: &mut Record) -> Result<()> {
        // Generate id then call save; or implement collision detection
        self.save(record).await
    }
    async fn save(&self, record: &Record) -> Result<()> { /* INSERT OR REPLACE */ todo!() }
    async fn load(&self, id: &Id) -> Result<Option<Record>> { /* SELECT */ todo!() }
    async fn delete(&self, id: &Id) -> Result<()> { /* DELETE */ todo!() }
}
```

Reference impl: [github.com/patte/tower-sessions-rusqlite-store] (~80 LoC sqlx-store-style impl). Phase 1 only ships the `sessions` schema (V010); store impl lands in Phase 5.

### Code Example 7: GitHub Actions `ci-fast.yml`

```yaml
name: ci-fast
on:
  push:
    branches: ['**']
  pull_request:
jobs:
  fast:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          toolchain: '1.85'
          components: clippy, rustfmt
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt --all -- --check
      - run: cargo clippy --workspace --all-targets -- -D warnings
      - run: cargo test --workspace --no-fail-fast
      - uses: pnpm/action-setup@v3
        with: { version: 9 }
      - uses: actions/setup-node@v4
        with:
          node-version: '20'
          cache: 'pnpm'
          cache-dependency-path: ui/pnpm-lock.yaml
      - run: cd ui && pnpm install --frozen-lockfile
      - run: cd ui && pnpm svelte-check
      - run: cd ui && pnpm lint
```

### Code Example 8: `ci-full.yml` with ProcMon test on Windows

```yaml
name: ci-full
on:
  pull_request:
  push: { branches: [main] }
jobs:
  matrix:
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { toolchain: '1.85', components: clippy, rustfmt }
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt --all -- --check
      - run: cargo clippy --workspace --all-targets -- -D warnings
      - run: cargo test --workspace --no-fail-fast
      - uses: pnpm/action-setup@v3
        with: { version: 9 }
      - uses: actions/setup-node@v4
        with:
          node-version: '20'
          cache: 'pnpm'
          cache-dependency-path: ui/pnpm-lock.yaml
      - run: cd ui && pnpm install --frozen-lockfile && pnpm svelte-check && pnpm lint
      - run: cargo build --release -p trackly-app

  procmon:
    needs: matrix
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { toolchain: '1.85' }
      - uses: Swatinem/rust-cache@v2
      - name: Download ProcMon
        shell: pwsh
        run: |
          Invoke-WebRequest https://download.sysinternals.com/files/ProcessMonitor.zip -OutFile pm.zip
          Expand-Archive pm.zip -DestinationPath C:\ProcMon
          echo "C:\ProcMon" | Out-File -FilePath $env:GITHUB_PATH -Append
      - run: cargo build --release -p trackly-app
      - run: cargo run --release -p procmon-check
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Tauri v1 plugin model | Tauri v2 capability/ACL model + `tauri-plugin-*` workspace | Oct 2024 GA [CITED: v2.tauri.app/blog/tauri-20] | v1 EOL; we are on v2.11+ throughout |
| Svelte 4 stores | Svelte 5 runes (`$state`, `$derived`, `$effect`, `$props`) | Oct 2024 stable | Phase 2+ surface; Phase 1 ships minimal `App.svelte` |
| `sqlx-sqlite` write tx | `rusqlite` + single dedicated writer task | 2024+ canonical via Evan Schwartz PSA [CITED] | Locked; do not reconsider |
| `ts-rs` for TS bindings | `tauri-specta v2` + `specta-typescript` | tauri-specta v2 stable rc 2024-2026 | Transitive type tracking; chosen explicitly in CONTEXT |
| `chrono` for timestamps | `time 0.3` | 2023+ ecosystem shift | Smaller binary; no `chrono-tz` portability quirks on Windows |
| `native-tls`/OpenSSL | `rustls 0.23` + `aws-lc-rs` provider | 2023+ pure-Rust trend | Portable Windows build without OpenSSL DLL |
| `tauri-plugin-updater` | **Disabled in portable mode** | Tauri 2 portable distribution model | Optional re-enable for installer variant only |
| `dirs::*_dir()` | Custom `Paths` resolver from `current_exe()` + sentinel | Always for portable apps | Banned via clippy |

**Deprecated/outdated (do not introduce):**
- `ts-rs` — transitive types broken; tauri-specta solves it.
- `genpdf` / `printpdf` (Cyrillic) — Phase 3 uses `krilla` instead (out of Phase 1 scope).
- `sqlx-sqlite` for write-heavy paths — write-tx lock starvation footgun.
- `chrono::Local` — banned via clippy in Phase 1.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `tauri-specta 2.0.0-rc.21` + `specta 2.0.0-rc.22` + `specta-typescript 0.0.9` are the correct compatible-version triple | Standard Stack + Code Example 3 | Compile failure or stale bindings; planner must `cargo search` to confirm at plan time. **Discovered via DeepWiki, not directly from latest docs.rs — verify before commit.** |
| A2 | `std::env::set_var` is `unsafe` in Rust 1.85 (or generates a warning) | Pitfall #8 | If still safe in 1.85, the `unsafe { }` block compiles but is harmless. If made unsafe in a future patch, our wrapping is forward-compatible. |
| A3 | refinery 0.8 default is one-tx-per-migration (`set_grouped(false)`) | Pitfall #5 + Code Example 2 | Confirmed via [docs.rs/refinery Runner::set_grouped] but the exact docs page label was a bit short — planner should verify by running a deliberately-failing V005 in a test |
| A4 | ProcMon's `/SaveApplyFilter` flag exists in current Sysinternals release | Code Example 5 | Community sources confirm — if the flag is renamed in a 2026 release, the procmon-check tool needs a one-line update |
| A5 | `tauri-plugin-single-instance` does not write to APPDATA when used in portable mode | Standard Stack table | The plugin manages a socket file for IPC; need to verify it lives next to `.exe` and not in `%TEMP%`. **Recommend planner spike this in P1.** |
| A6 | rusqlite's `pragma_update` for `journal_mode=WAL` persists across connection close on Windows with cyrillic path | Pattern 3 + Pitfall #4 | Confirmed by SQLite docs; if cyrillic path causes WAL persistence issue, ProcMon-test fixture (which uses cyrillic) will catch it on first run |
| A7 | tauri-specta `Builder::export` will recreate `ui/src/bindings.ts` from scratch on each test run (no stale fragments) | Code Example 3 | Standard behavior; if it appends instead of overwrites, the test should `std::fs::remove_file` first |
| A8 | All 12 migrations in V001-V012 fit the `[U|V]{n}__{name}.sql` refinery convention with no `U` (undo) files | D-Migrations-01 schema split | Forward-only; we explicitly do NOT ship `U` undo files |
| A9 | The `cargo test --test export_bindings` command writes to `../../ui/src/bindings.ts` relative to `trackly-app/Cargo.toml` correctly across OSs | Code Example 3 | Path is constructed from the test's CWD which is the crate manifest dir; if CI's CWD differs, use `env!("CARGO_MANIFEST_DIR")` to build absolute path |
| A10 | `Swatinem/rust-cache@v2` correctly caches the workspace's `target/` across runs given our split-crate layout | CI Code Example | Battle-tested action; no special config needed for workspaces |

**Confirmation needed before plan commit:**
- A1 (tauri-specta version triple) — `cargo search tauri-specta` + `cargo search specta-typescript`
- A3 (refinery transaction default) — test fixture
- A5 (single-instance plugin path) — quick read of `tauri-plugin-single-instance` source

## Open Questions

1. **Should `WriteJob` be a closure (`FnOnce(&mut Connection)`) or a named enum?**
   - What we know: Both shapes satisfy the locked decision. Closure form (Code Example 1, Pattern 1) keeps the writer worker generic — services express their own SQL. Enum form forces all SQL through pattern-matching in `writer_worker.rs`, which is more disciplined but boilerplate-heavy.
   - What's unclear: Which scales better as Phase 2+ adds dozens of write operations.
   - Recommendation: **Closure form for Phase 1** (`Writer::execute(|conn| { ... })`). Revisit if writer_worker grows hard to reason about.

2. **Should `tauri-plugin-single-instance` be added in Phase 1?**
   - What we know: Mentioned in CONTEXT deferred list with note "best practice уже в Phase 1, на усмотрение планировщика".
   - What's unclear: Whether the plugin's socket file lives in a portable-safe location (Assumption A5).
   - Recommendation: **Add it**, with a small test that asserts no APPDATA writes after start. If the plugin violates portability, defer to Phase 8 (release pipeline) and use a manual lockfile in `<exe_dir>` instead.

3. **`trackly-core` Cargo.toml — does it pull `async-trait`?**
   - What we know: Hexagonal pattern needs `async fn` in traits; stabilization-of-AFIT (async fn in traits) landed in Rust 1.75 but with object-safety caveats.
   - What's unclear: Whether we need `async-trait` macro or can use bare AFIT.
   - Recommendation: **Use bare AFIT** where possible (1.85 supports it cleanly); fall back to `async-trait` only if `dyn Trait` shape is needed. Practically, services use generics, so AFIT is fine.

4. **Does `cargo fmt --check` need any custom `rustfmt.toml`?**
   - What we know: Defaults are fine for most workspaces.
   - What's unclear: Whether the workspace wants `imports_granularity = "Crate"` or other CI-friendly defaults.
   - Recommendation: **Ship empty `rustfmt.toml`** — accept defaults; revisit if review preferences emerge.

5. **Should the `ui/` folder's `package.json` live alongside `vite.config.ts`, or at workspace root?**
   - What we know: Vite root = `ui/`. Tauri config points `frontendDist = "../ui/dist"`.
   - What's unclear: Whether to scaffold a root `package.json` to allow `pnpm` workspaces (e.g., for shared lint config).
   - Recommendation: **Single `package.json` inside `ui/`** for Phase 1. Workspace pnpm-only if dev tooling demands it later.

## Environment Availability

Phase 1 builds and tests across three OS targets; the planner must ensure the GitHub Actions runners have the right toolchains. Local dev (macOS) is constrained by the user — no Windows, no AD, no SNMP. Phase 1 does not need those.

| Dependency | Required By | Available (dev macOS) | Version | Fallback |
|------------|------------|-----------------------|---------|----------|
| Rust 1.85+ | Compile workspace | ✓ (verify with `rustc --version`) | check at plan time | rustup install 1.85 |
| pnpm 9 | UI prebuild | likely ✓ (developer-installed) | check `pnpm --version` | `npm install -g pnpm@9` |
| Node 20 | Vite + pnpm | likely ✓ | check `node --version` | nvm install 20 |
| SQLite | bundled via `rusqlite` feature | ✓ (compiled in, no system dep) | upstream from rusqlite 0.39 | none needed |
| GitHub Actions Windows runner | CI ProcMon-test | ✓ via workflow | windows-latest | none — required |
| ProcMon.exe | CI ProcMon-test | ✗ (downloaded per-run from Sysinternals) | latest release on demand | none — required |
| Tauri 2 build tools (system webview2 on Windows) | Tauri build on Windows runner | ✓ on windows-latest | runner-provided | n/a |

**Missing dependencies with no fallback:** none.
**Missing dependencies with fallback:** none.

**Local dev caveat:** macOS dev box can build and `cargo test` everything except the ProcMon-test (which is Windows-only). Phase 1 acceptance still works locally — ProcMon-test is gated to CI.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | `cargo test` (built-in) + `cargo nextest` optional |
| Config file | none for built-in; `.config/nextest.toml` if nextest adopted |
| Quick run command | `cargo test --workspace --no-fail-fast` |
| Full suite command | `cargo test --workspace --no-fail-fast --release` |
| Frontend lint | `cd ui && pnpm svelte-check && pnpm lint` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| FOUND-01 | 3-crate boundary (core has no rusqlite/tokio dep) | structural test | `cargo tree -p trackly-core \| grep -E "rusqlite\|tokio"` (must be empty) | ❌ Wave 0 — `crates/trackly-core/tests/no_io_deps.rs` |
| FOUND-02 | Single-writer + reader pool; no SQLITE_BUSY | integration | `cargo test -p trackly-app --test concurrent_writes` | ❌ Wave 0 — `trackly-app/tests/concurrent_writes.rs` |
| FOUND-03 | Refinery forward-only + user_version | integration | `cargo test -p trackly-infra db::migrations::tests` | ❌ Wave 0 |
| FOUND-04 | Portable paths from current_exe + sentinel | unit | `cargo test -p trackly-infra paths::tests` | ❌ Wave 0 |
| FOUND-05 | WEBVIEW2_USER_DATA_FOLDER set before any tauri call | manual review + ProcMon | grep `webview_env::set` is first non-trivial call in `main()`; ProcMon-test verifies no AppData writes | ❌ Wave 0 — covered transitively |
| FOUND-06 | `Secret<T>` Debug = `***`, Drop zeroizes | unit | `cargo test -p trackly-core primitives::secret::tests` | ❌ Wave 0 |
| FOUND-07 | All `_at_utc` columns are INTEGER unix | manual review of migrations + grep | check `*.sql` for `_at_utc` patterns; clippy-lint catches `chrono::Local::now` | ✅ (clippy gate) + ❌ schema review test (Wave 0) |
| FOUND-08 | Seeded lookup tables created and populated by V001 | integration | open test DB, assert SELECT COUNT(*) from each lookup matches seed list | ❌ Wave 0 — `trackly-infra/tests/seed_data.rs` |
| FOUND-09 | All user-mutable tables have created_at_utc + updated_at_utc + deleted_at_utc + version | schema test | parse migration SQL or query `PRAGMA table_info(table)` post-migration | ❌ Wave 0 — `trackly-infra/tests/per_record_invariants.rs` |
| FOUND-10 | audit_log table exists with declared columns | schema test | `PRAGMA table_info(audit_log)` assert columns | ❌ Wave 0 — included in `per_record_invariants.rs` or new file |
| FOUND-11 | ProcMon test in CI Windows runner | smoke (Windows-only) | `cargo run --release -p procmon-check` on windows-latest | ❌ Wave 0 — entire `tools/procmon-check/` |
| FOUND-12 | tauri-specta generates bindings; same DTO round-trips both transports | smoke | `cargo test -p trackly-app --test export_bindings` AND `--test health_smoke` | ❌ Wave 0 — `trackly-app/tests/health_smoke.rs` |
| BLD-01 | CI runs fmt/clippy/test/svelte-check/lint on every push | manual review of `.github/workflows/ci-fast.yml` | inspect workflow + first PR triggers it | ❌ Wave 0 — both workflow files |
| BLD-06 | ProcMon test integrated into CI matrix | manual review of `ci-full.yml` | inspect workflow + first Windows run | ❌ Wave 0 — included with BLD-01 |

**Per-criterion verification (Success Criteria from ROADMAP):**

| SC# | Criterion | Test |
|-----|-----------|------|
| 1 | Cyrillic path + no AppData writes | ProcMon-test (FOUND-11) using `%TEMP%\Документы\Trackly\` |
| 2 | 50 concurrent writes, no SQLITE_BUSY | `concurrent_writes.rs` integration |
| 3 | clippy + test + fmt + svelte-check + lint green; disallowed-methods banned | CI workflow (BLD-01) |
| 4 | user_version > embedded → graceful error, file intact | `downgrade_protection.rs` integration |
| 5 | tauri-specta smoke: same DTO via Tauri + axum round-trips identically | `health_smoke.rs` (calls health command via direct service invocation + asserts shape matches future axum handler shape — axum mount itself is Phase 5) |

### Sampling Rate
- **Per task commit:** `cargo test --workspace --no-fail-fast`
- **Per wave merge:** `cargo test --workspace --release` + `cd ui && pnpm svelte-check && pnpm lint`
- **Phase gate:** Full suite green + ProcMon-test green on CI Windows runner before `/gsd-verify-work`

### Wave 0 Gaps
All test files below are new (greenfield phase). Wave 0 must create:
- `crates/trackly-core/tests/no_io_deps.rs` — asserts core's Cargo.toml has no rusqlite/tokio/tauri
- `crates/trackly-core/src/primitives/secret.rs` test module
- `crates/trackly-infra/src/db/migrations.rs` test module (migrations apply cleanly)
- `crates/trackly-infra/tests/seed_data.rs` — lookup tables populated
- `crates/trackly-infra/tests/per_record_invariants.rs` — all user-mutable tables have created/updated/deleted_at_utc + version
- `crates/trackly-infra/tests/audit_log_schema.rs` — audit_log has declared columns
- `crates/trackly-infra/src/paths.rs` test module — portable detection
- `crates/trackly-app/tests/concurrent_writes.rs` — 25+25 concurrent writers
- `crates/trackly-app/tests/downgrade_protection.rs` — user_version=999 fixture
- `crates/trackly-app/tests/export_bindings.rs` — tauri-specta regen
- `crates/trackly-app/tests/health_smoke.rs` — DTO shape consistency
- `tools/procmon-check/` entire crate
- `.github/workflows/ci-fast.yml`, `ci-full.yml`, `cargo-deny.yml`

Framework install: `rustup install 1.85`, `pnpm install -g pnpm@9` (already done on dev box presumably; CI installs explicitly).

## Security Domain

`security_enforcement = true` in `.planning/config.json`. ASVS L1 baseline.

### Applicable ASVS L1 Categories

| ASVS Category | Applies in Phase 1 | Standard Control |
|---------------|-------------------|-----------------|
| V2 Authentication | NO — Phase 5 | `argon2 0.5` (deferred) |
| V3 Session Management | NO — Phase 5 | `tower-sessions 0.13` (schema only V010 in Phase 1) |
| V4 Access Control | NO — Phase 5 | `authorize(user, perm)` function (Phase 5) |
| V5 Input Validation | PARTIAL — `AppError::Validation` variant defined, but no DTOs validated yet | `serde` strict-mode deserialization + per-DTO validation (Phase 2+) |
| V6 Cryptography | YES — `Secret<T>` discipline established | `Secret<T>` newtype + `zeroize` for memory wiping; ban `chrono::Local` to prevent timestamp confusion in audit chain |
| V7 Errors & Logging | YES — tracing setup, no secrets in logs | `Secret<T>::Debug = "***"`; structured tracing with explicit field allowlist (Phase 2+ enforce) |
| V8 Data Protection | YES — DB file on local FS only; no network shares | `paths.rs` rejects UNC paths (recommend planner add this check); WAL files siblings of main DB |
| V9 Communication | NO — no HTTPS yet (Phase 5) | `rustls` (deferred) |
| V10 Malicious Code | YES — supply-chain | `cargo-deny` workflow + slopcheck verification (D-CI-01) |
| V11 Business Logic | NO — no business logic yet | n/a |
| V12 Files & Resources | YES — portable file layout, no APPDATA writes | `WEBVIEW2_USER_DATA_FOLDER`; `dirs::*_dir()` banned; ProcMon-test |
| V13 API & Web Service | NO — no API yet (Phase 5) | n/a |
| V14 Configuration | YES — `trackly.config.toml` next to .exe | TOML schema enforced via serde; no secrets in config (no auth yet) |

### Known Threat Patterns for Phase 1 Stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Portable mode leak into %APPDATA% | Information Disclosure / Tampering | `WEBVIEW2_USER_DATA_FOLDER` + clippy bans + ProcMon-test (D-CI-03) |
| Schema downgrade corruption | Tampering / Denial of Service | `PRAGMA user_version` check on open + refuse to start (D-Migrations-02) |
| Secret leakage in logs | Information Disclosure | `Secret<T>` newtype with custom Debug (FOUND-06) — established now, enforced later |
| Concurrent-write data loss | Tampering | Single-writer mpsc + optimistic-lock columns (FOUND-09) — schema laid down now, enforced in services later |
| Supply-chain attack (typosquatted crate) | Tampering | slopcheck pre-commit + `cargo-deny` workflow |
| Cyrillic path mangling causing wrong DB open | Tampering / Denial of Service | UTF-8 `PathBuf` discipline + cyrillic CI test fixture (D-CI-03) |

## Sources

### Primary (HIGH confidence)
- [Tauri 2.0 Stable Release](https://v2.tauri.app/blog/tauri-20/) — v2 GA Oct 2024
- [Tauri Core Releases](https://v2.tauri.app/release/) — current versions
- [Tauri Configuration Reference](https://v2.tauri.app/reference/config/)
- [Tauri Issue #1365 — Windows User Data for bundled Application](https://github.com/tauri-apps/tauri/issues/1365) — confirms `WEBVIEW2_USER_DATA_FOLDER` env-var pattern
- [Tauri Discussion #8029 — How to clean webview cache?](https://github.com/orgs/tauri-apps/discussions/8029) — same
- [docs.rs/rusqlite/0.39](https://docs.rs/rusqlite/latest/rusqlite/) — current API
- [docs.rs/refinery latest](https://docs.rs/refinery/latest/refinery/) — embed_migrations! + Runner
- [docs.rs/refinery Runner](https://docs.rs/refinery/latest/refinery/struct.Runner.html) — set_grouped / set_target / run signatures
- [github.com/rust-db/refinery](https://github.com/rust-db/refinery) — naming convention `[U|V]{n}__{name}.sql`
- [SQLite WAL docs](https://sqlite.org/wal.html) — single writer + no network FS rules
- [PSA: Write Transactions are a Footgun with SQLx and SQLite — Evan Schwartz](https://emschwartz.me/psa-write-transactions-are-a-footgun-with-sqlx-and-sqlite/) — canonical rusqlite-over-sqlx rationale
- [tauri-specta v2 — Getting Started (DeepWiki)](https://deepwiki.com/specta-rs/tauri-specta/2-getting-started) — version triple + Builder API
- [github.com/specta-rs/tauri-specta](https://github.com/specta-rs/tauri-specta) — official
- [Sysinternals — Process Monitor](https://learn.microsoft.com/en-us/sysinternals/downloads/procmon) — official docs
- [Using Procmon in Command-line — Microsoft Learn](https://learn.microsoft.com/en-us/archive/blogs/yash/using-procmon-in-command-line) — flag reference
- [The Ultimate Guide to Procmon — AdamTheAutomator](https://adamtheautomator.com/procmon/) — flag examples
- [github.com/patte/tower-sessions-rusqlite-store](https://github.com/patte/tower-sessions-rusqlite-store) — reference impl for Phase 5 custom store
- [docs.rs/tower-sessions](https://docs.rs/tower-sessions/latest/tower_sessions/) — SessionStore trait
- [docs.rs/axum/0.8](https://docs.rs/axum/latest/axum/) — current axum

### Secondary (MEDIUM confidence — confirmed but indirect)
- [DEV Community — Ship Your Tauri v2 App Like a Pro (GitHub Actions)](https://dev.to/tomtomdu73/ship-your-tauri-v2-app-like-a-pro-github-actions-and-release-automation-part-22-2ef7) — CI recipe
- [Tauri GitHub Actions guide](https://v2.tauri.app/distribute/pipelines/github/) — official CI matrix
- [Swatinem/rust-cache@v2](https://github.com/Swatinem/rust-cache) — cache action
- [pnpm/action-setup](https://github.com/pnpm/action-setup) — pnpm in CI

### Tertiary (LOW confidence — flagged for verification at plan time)
- A1 — tauri-specta version triple compatibility (`2.0.0-rc.21` + specta `2.0.0-rc.22` + specta-typescript `0.0.9`); planner must `cargo search` confirm
- A3 — refinery default-transaction-behavior (per-migration); confirmed by docs but warrants explicit test
- A5 — `tauri-plugin-single-instance` portability (does its socket file land in APPDATA?); planner spike

### Project-internal references (already read)
- `CLAUDE.md` — stack and version pins
- `.planning/PROJECT.md` — vision and core value
- `.planning/REQUIREMENTS.md` — FOUND-01..12 + BLD-01/06 mapped
- `.planning/ROADMAP.md` Phase 1 section — 5 success criteria
- `.planning/research/SUMMARY.md` — resolved decisions
- `.planning/research/STACK.md` — pinned versions
- `.planning/research/ARCHITECTURE.md` — hexagonal layout
- `.planning/research/PITFALLS.md` — top 15 pitfalls
- `.planning/research/FEATURES.md` — schema dimensions
- `.planning/phases/01-foundation/01-CONTEXT.md` — locked decisions (D-Schema-01..05, D-Workspace-01..02, D-Migrations-01..02, D-WriterChannel-01, D-AppError-01, D-Config-01, D-Logging-01, D-CI-01..03, D-Test-01)

## Metadata

**Confidence breakdown:**
- Standard Stack: HIGH — user-pinned in CLAUDE.md; CONTEXT.md aligns
- Architecture Patterns: HIGH — all 6 patterns documented in `.planning/research/ARCHITECTURE.md` and locked in CONTEXT.md
- Pitfalls: HIGH — top 15 pre-researched in PITFALLS.md; Phase 1 prevention is fully specified
- tauri-specta v2 API: MEDIUM — version triple verified via DeepWiki; planner must confirm with `cargo search` (A1)
- ProcMon CLI invocation: MEDIUM-HIGH — flags verified by official docs + multiple community sources
- refinery default tx behavior: MEDIUM-HIGH — confirmed by docs.rs Runner page; assumption A3 flagged
- Security applicability map: HIGH — Phase 1 establishes V6/V12/V14 controls; later phases bring V2/V3/V4/V9

**Research date:** 2026-05-24
**Valid until:** 2026-06-23 (30 days for stable foundation stack)

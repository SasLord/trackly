# Architecture Research

**Domain:** Tauri 2 desktop + embedded axum HTTP server hybrid; single SQLite (WAL) data store; portable mode; AD + SNMP integrations; Russian-language UI; small LAN (≤20 concurrent users).
**Researched:** 2026-05-24
**Confidence:** HIGH for stack-shape decisions (verified against official docs and active community sources); MEDIUM for tauri-specta v2 / async-snmp choices (live ecosystem, slight version flux); LOW for Windows-specific portable mode edge cases (must be probed during Phase 1 hands-on).

---

## Standard Architecture

### System Overview

```
┌──────────────────────────────────────────────────────────────────────────┐
│                              PRESENTATION                                │
│                                                                          │
│  ┌────────────────────────────────┐    ┌────────────────────────────┐   │
│  │  Svelte SPA (Tauri webview)    │    │  Svelte SPA (LAN browser)  │   │
│  │  - Admin UI                    │    │  - Specialists / Сотрудники│   │
│  │  - All features                │    │  - Requests, login, etc.   │   │
│  └───────────────┬────────────────┘    └──────────────┬─────────────┘   │
│                  │ invoke()                            │ fetch() + cookie│
│                  │ (transport: tauri)                  │ (transport: http)│
└──────────────────┼─────────────────────────────────────┼─────────────────┘
                   │                                     │
┌──────────────────▼─────────────────────────────────────▼─────────────────┐
│                        TRANSPORT / EDGE LAYER                            │
│  ┌──────────────────────────┐         ┌──────────────────────────────┐   │
│  │  Tauri command handlers  │         │  axum routers / handlers     │   │
│  │  #[tauri::command]       │         │  Router::new().route(...)    │   │
│  │  - args from invoke      │         │  - args from HTTP/JSON       │   │
│  │  - auth: trust desktop   │         │  - auth: session middleware  │   │
│  └────────────┬─────────────┘         └──────────────┬───────────────┘   │
│               │  both call the same API surface ↓     │                   │
│               └─────────────────┬───────────────────  ┘                   │
└─────────────────────────────────┼─────────────────────────────────────────┘
                                  │
┌─────────────────────────────────▼─────────────────────────────────────────┐
│                     APPLICATION (use-case services)                       │
│  ┌────────────┐ ┌────────────┐ ┌────────────┐ ┌──────────────┐           │
│  │ DeviceSvc  │ │ ActSvc     │ │ CartridgeSvc│ │ RequestSvc   │           │
│  │ - CRUD     │ │ - issue    │ │ - lifecycle │ │ - workflow   │           │
│  │ - import   │ │ - return   │ │ - low-stock │ │              │           │
│  └─────┬──────┘ └─────┬──────┘ └──────┬─────┘ └──────┬───────┘           │
│        │              │               │              │                    │
│  ┌────────────┐ ┌────────────┐ ┌────────────┐ ┌──────────────┐           │
│  │ AuthSvc    │ │ PrinterSvc │ │ ReportSvc  │ │ SettingsSvc  │           │
│  │ - sessions │ │ - SNMP loop│ │ - aggreg.  │ │ - templates  │           │
│  │ - AD bind  │ │ - status   │ │            │ │ - org info   │           │
│  └─────┬──────┘ └─────┬──────┘ └──────┬─────┘ └──────┬───────┘           │
└────────┼──────────────┼───────────────┼──────────────┼────────────────────┘
         │              │               │              │
┌────────▼──────────────▼───────────────▼──────────────▼────────────────────┐
│                          DOMAIN (no IO, pure types)                       │
│  Entities: Device, Act, Cartridge, Printer, Request, User, OrgSettings    │
│  Value objects: ActNumber, CartridgeCode (C-000001), DeviceStatus enum    │
│  Domain rules: numbering, return-to-stock, low-stock thresholds           │
└─────────────────────────────────┬─────────────────────────────────────────┘
                                  │ (depends-on inverted: services depend on traits)
┌─────────────────────────────────▼─────────────────────────────────────────┐
│                 INFRASTRUCTURE (IO, all behind traits)                    │
│  ┌──────────────┐ ┌──────────────┐ ┌─────────────┐ ┌──────────────┐      │
│  │ SqliteRepos  │ │ Snmp client  │ │ Ldap client │ │ Smtp/Tg/Webhook│    │
│  │ (sqlx + WAL) │ │ (snmp2/csnmp)│ │ (ldap3)     │ │ (lettre, ...)  │    │
│  └──────┬───────┘ └──────┬───────┘ └─────┬───────┘ └──────┬─────────┘    │
│         │                │               │                │              │
│  ┌──────▼───────┐ ┌──────▼───────┐ ┌─────▼───────┐ ┌──────▼─────────┐    │
│  │ Filesystem   │ │ PathResolver │ │ Pdf/Template│ │ BackgroundTask │    │
│  │ (logos,      │ │ (portable    │ │ engine      │ │ scheduler      │    │
│  │  backups)    │ │  detection)  │ │ (typst/...)  │ │                │    │
│  └──────────────┘ └──────────────┘ └─────────────┘ └────────────────┘    │
└───────────────────────────────────────────────────────────────────────────┘

                ┌───────────── one tokio runtime ──────────────┐
                │  - tauri commands                            │
                │  - axum server                               │
                │  - SNMP poll task                            │
                │  - backup scheduler                          │
                │  - low-stock checker                         │
                │  - alert dispatcher                          │
                └──────────────────────────────────────────────┘
```

The cardinal rule: **`#[tauri::command]` handlers and axum handlers are both thin transport adapters.** They parse input, call into the same application service, and serialize output. No business logic in either handler.

### Component Responsibilities

| Component | Responsibility | Typical Implementation |
|-----------|----------------|------------------------|
| **Domain** | Entities, value objects, invariants. Zero IO, zero async. Pure Rust. | Plain `struct`/`enum`, `Display`/`From`/`TryFrom`, domain errors as `thiserror`. |
| **Application services** | Use-case orchestration (transactions, multi-step workflows, validation, cross-entity rules). | `struct DeviceService<R: DeviceRepo, ...> { repo: R, ... }` with `async fn` methods. |
| **Ports (traits)** | Abstract dependencies the services need (repositories, AD client, SNMP poller, mailer). | `trait DeviceRepo { async fn save(...); async fn find_by_id(...); }`. |
| **Repositories (adapters)** | Concrete sqlx-backed implementations. Translate DTOs ↔ rows. | `struct SqliteDeviceRepo { pool: Arc<SqlitePool> }` impl `DeviceRepo`. |
| **Tauri command handlers** | Thin adapter. Extract args, call service, return JSON-able DTO. | `#[tauri::command] async fn create_device(state: State<AppCtx>, dto: NewDeviceDto) -> Result<DeviceDto, AppError>`. |
| **Axum HTTP handlers** | Thin adapter. Same call, different transport. | `async fn post_device(State(ctx): State<AppCtx>, Json(dto): Json<NewDeviceDto>) -> Result<Json<DeviceDto>, AppError>`. |
| **AppContext (state)** | Holds Arc references to all services + pools. Cloned into both `tauri::Manager` and `axum::Router::with_state`. | `#[derive(Clone)] struct AppCtx { devices: Arc<DeviceService<...>>, ... }`. |
| **Background scheduler** | Owns long-running tokio tasks (SNMP poll, backup, low-stock check). Coordinates start/stop with the server toggle. | `tokio::spawn` from `tauri::Builder::setup`, plus a `CancellationToken` for clean shutdown. |
| **Path resolver** | Single source of truth for "where do files live?". Detects portable vs installed mode at startup. | Pure function returning `Paths { db, config, logos, backups, templates }`. |
| **Session store** | Stores web sessions. In-memory + persisted-to-DB hybrid for restart survival. | `tower-sessions` with a custom `SessionStore` backed by SQLite. |

---

## Recommended Project Structure

**Decision: workspace with three crates, not a monolith and not a sprawling 10-crate microcosm.**

Rationale: At this scale (≈10–20k LOC by v1 done), a single crate gets unwieldy for compile times and makes the "no IO in domain" rule a convention rather than a compile-time guarantee. Ten crates is overkill and slows iteration. Three crates draws the only boundary that actually pays for itself: **domain vs everything else, and binary vs library**.

```
trackly/                              # workspace root
├── Cargo.toml                        # [workspace] members = [...]
├── README.md
├── .planning/                        # (already in repo)
│
├── crates/
│   ├── trackly-core/                 # crate 1: domain + application + ports
│   │   ├── Cargo.toml                # no tokio, no sqlx as direct dep (re-exported types only via feature flags)
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── domain/               # zero-IO entities
│   │       │   ├── mod.rs
│   │       │   ├── device.rs         # Device, DeviceStatus, DeviceType
│   │       │   ├── act.rs            # Act, ActNumber, ReturnVariant
│   │       │   ├── cartridge.rs      # Cartridge, CartridgeCode, ChargeState
│   │       │   ├── printer.rs        # Printer, PrinterStatus, SnmpProfile
│   │       │   ├── request.rs        # Request, RequestKind, RequestState
│   │       │   ├── user.rs           # User, Role, UserSource (Local | Ad)
│   │       │   └── errors.rs         # DomainError (thiserror)
│   │       ├── ports/                # traits — what services need from outside
│   │       │   ├── mod.rs
│   │       │   ├── repos.rs          # DeviceRepo, ActRepo, CartridgeRepo, ...
│   │       │   ├── ad.rs             # AdClient trait
│   │       │   ├── snmp.rs           # SnmpClient trait
│   │       │   ├── mailer.rs         # Mailer trait
│   │       │   ├── filesystem.rs     # FileStore trait (logos, backups)
│   │       │   └── clock.rs          # Clock trait (testability for "now")
│   │       └── services/             # use-case orchestration
│   │           ├── mod.rs
│   │           ├── device_service.rs
│   │           ├── act_service.rs    # numbering, return flow, archive
│   │           ├── cartridge_service.rs
│   │           ├── request_service.rs
│   │           ├── report_service.rs
│   │           ├── auth_service.rs   # login (local + AD), sessions
│   │           └── settings_service.rs
│   │
│   ├── trackly-infra/                # crate 2: adapters (sqlx, ldap3, snmp, smtp, fs)
│   │   ├── Cargo.toml                # depends on trackly-core
│   │   ├── migrations/               # embedded via sqlx::migrate!()
│   │   │   ├── 0001_init.sql
│   │   │   ├── 0002_acts.sql
│   │   │   └── ...
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── sqlite/               # impls of *Repo traits
│   │       │   ├── mod.rs            # pool setup, WAL pragmas
│   │       │   ├── device_repo.rs
│   │       │   ├── act_repo.rs
│   │       │   ├── cartridge_repo.rs
│   │       │   ├── request_repo.rs
│   │       │   ├── user_repo.rs
│   │       │   ├── session_store.rs  # tower-sessions backend
│   │       │   └── migrations.rs     # run on startup
│   │       ├── ldap/
│   │       │   └── ad_client.rs      # ldap3-backed AdClient impl
│   │       ├── snmp/
│   │       │   ├── client.rs         # snmp2-backed SnmpClient impl
│   │       │   └── profiles.rs       # OID tables per vendor (Pantum, Kyocera, HP, Canon)
│   │       ├── mail/
│   │       │   ├── smtp.rs           # lettre-backed Mailer
│   │       │   ├── telegram.rs
│   │       │   └── webhook.rs
│   │       ├── files/
│   │       │   └── disk_store.rs     # FileStore on local disk
│   │       └── mocks/                # mock adapters behind cfg(feature = "mocks") or always-on
│   │           ├── mock_ad.rs
│   │           ├── mock_snmp.rs
│   │           └── mock_mailer.rs
│   │
│   └── trackly-app/                  # crate 3: the binary — Tauri + axum + bootstrap
│       ├── Cargo.toml                # depends on trackly-core, trackly-infra
│       ├── tauri.conf.json
│       ├── icons/
│       ├── build.rs                  # tauri build, runs tauri-specta type emission
│       └── src/
│           ├── main.rs               # entry: build paths → load config → build AppCtx → start tauri (+ optional axum)
│           ├── paths.rs              # PathResolver: portable detection, settings file
│           ├── config.rs             # AppConfig (port, host, server-mode toggle, backup schedule)
│           ├── context.rs            # AppCtx (Arc'd services), construction
│           ├── shutdown.rs           # CancellationToken plumbing
│           ├── tauri_cmds/           # #[tauri::command] adapters
│           │   ├── mod.rs
│           │   ├── devices.rs
│           │   ├── acts.rs
│           │   └── ... (one file per service)
│           ├── http/                 # axum
│           │   ├── mod.rs            # Router builder; mounts routes + middleware
│           │   ├── auth_mw.rs        # session extraction; CSRF for non-GET
│           │   ├── error.rs          # AppError → HTTP status mapping
│           │   ├── devices.rs
│           │   ├── acts.rs
│           │   └── ... (one file per service)
│           ├── tasks/                # background workers
│           │   ├── mod.rs
│           │   ├── snmp_poll.rs
│           │   ├── backup.rs
│           │   ├── low_stock.rs
│           │   └── alerts.rs
│           ├── dto.rs                # wire types shared by both transports (Serialize + specta::Type)
│           ├── bindings.rs           # generated by tauri-specta (gitignored, regenerated)
│           └── pdf/                  # PDF/document rendering (templates from DB)
│               ├── mod.rs
│               ├── act_template.rs
│               └── reception_template.rs
│
├── ui/                               # Svelte SPA (single codebase, dual transport)
│   ├── package.json
│   ├── svelte.config.js
│   ├── vite.config.ts
│   ├── tsconfig.json
│   └── src/
│       ├── app.html
│       ├── main.ts
│       ├── lib/
│       │   ├── api/
│       │   │   ├── transport.ts      # detects Tauri vs browser; exports `invoke()` polyfill
│       │   │   ├── bindings.ts       # symlink/copy from trackly-app/src/bindings.ts
│       │   │   ├── devices.ts        # typed wrappers
│       │   │   └── ...
│       │   ├── stores/
│       │   ├── components/
│       │   └── i18n/                 # ru.ts only in v1
│       ├── routes/                   # SvelteKit-style or pages/
│       └── styles/                   # SCSS
│
└── .github/
    └── workflows/
        ├── ci.yml                    # clippy + cargo test + svelte-check
        └── release.yml               # tag → matrix build (Win x64, macOS arm64, Linux)
```

### Structure Rationale

- **`trackly-core` has no `tokio`, no `sqlx`, no `ldap3` in its `Cargo.toml`.** This is the compile-time enforcement that "domain is pure". Services are `async fn` only because they call ports — but the port traits use `async_trait` so `trackly-core` only depends on `async-trait`, `thiserror`, `serde`, `chrono`, and `uuid`. If you reach for `tokio::fs` in a service, the build breaks. Good.
- **`trackly-infra` is where IO happens.** Splitting it from `-core` lets unit tests against services run with all-mock adapters, with no SQLite needed.
- **`trackly-app` is the only crate that knows about Tauri or axum.** This is the boundary at which the dual-transport pattern lives. Both `tauri_cmds/devices.rs` and `http/devices.rs` import the same `DeviceService` from `trackly-core` and the same `DeviceDto` from `dto.rs`.
- **`ui/` is a sibling, not a subfolder of `trackly-app`.** Tauri's default scaffold puts the frontend at `src-tauri/../`. Mirror that. The generated `bindings.ts` is the bridge.
- **No separate `trackly-cli`, `trackly-bench`, `trackly-fuzz` crates yet.** Add when needed, not preemptively.

---

## Architectural Patterns

### Pattern 1: Hexagonal core + thin transport adapters

**What:** Business logic lives once, in `trackly-core::services`. Each service depends on trait-typed ports. Tauri commands and axum handlers are 5–15 line adapters that decode the request, call the service, encode the response.

**When to use:** Always, for this project. The whole reason the project exists in this shape is to serve two transports. Skipping this pattern means duplicating logic.

**Trade-offs:**
- **Pro:** One bug-fix, one place. Both transports always behave identically. Mocking is trivial. Tests for services do not need a server or a webview.
- **Con:** More files, more `Arc<dyn ...>` (or generic-parameter explosion). One layer of indirection.

**Example:**
```rust
// trackly-core/src/services/device_service.rs
pub struct DeviceService<R: DeviceRepo, C: Clock> {
    repo: R,
    clock: C,
}

impl<R: DeviceRepo, C: Clock> DeviceService<R, C> {
    pub async fn create(&self, input: NewDevice) -> Result<Device, DomainError> {
        input.validate()?;
        let device = Device::new(input, self.clock.now());
        self.repo.save(&device).await?;
        Ok(device)
    }
}

// trackly-app/src/tauri_cmds/devices.rs
#[tauri::command]
#[specta::specta]
pub async fn create_device(
    state: tauri::State<'_, AppCtx>,
    input: NewDeviceDto,
) -> Result<DeviceDto, AppError> {
    let device = state.devices.create(input.into()).await?;
    Ok(device.into())
}

// trackly-app/src/http/devices.rs
pub async fn post_device(
    State(ctx): State<AppCtx>,
    _: AuthSession,                  // session middleware extractor
    Json(input): Json<NewDeviceDto>,
) -> Result<Json<DeviceDto>, AppError> {
    let device = ctx.devices.create(input.into()).await?;
    Ok(Json(device.into()))
}
```

Both adapters: parse → call → encode. The `AppError` type implements both `Serialize` (for Tauri) and `IntoResponse` (for axum), mapping to the same JSON shape.

### Pattern 2: AppContext as cloneable, Arc'd service bundle

**What:** A `#[derive(Clone)] struct AppCtx { devices: Arc<DeviceService<...>>, acts: Arc<ActService<...>>, ... }` constructed once at startup and handed to both `tauri::Builder::manage(ctx.clone())` and `axum::Router::new().with_state(ctx.clone())`.

**When to use:** This is the only sane way to share state across Tauri and axum given both want owned/cloneable handles.

**Trade-offs:**
- **Pro:** Single construction, single ownership story, identical state everywhere.
- **Con:** All services live for the whole process lifetime — no scoping. That is fine for this app; there is one user-session granularity at the request level, not the service level.

```rust
#[derive(Clone)]
pub struct AppCtx {
    pub devices: Arc<DeviceService<SqliteDeviceRepo, SystemClock>>,
    pub acts: Arc<ActService<SqliteActRepo, SqliteDeviceRepo, SystemClock>>,
    pub cartridges: Arc<CartridgeService<...>>,
    pub auth: Arc<AuthService<SqliteUserRepo, Box<dyn AdClient>>>,
    pub printers: Arc<PrinterService<Box<dyn SnmpClient>, SqlitePrinterRepo>>,
    pub settings: Arc<SettingsService<...>>,
    pub paths: Arc<Paths>,
    pub shutdown: CancellationToken,
}
```

> Use `Arc<dyn Trait + Send + Sync>` for ports where the concrete type varies between prod and tests (AD, SNMP, mailer). Use concrete generics where there is exactly one impl in prod (sqlx repos). Mix is fine; do not over-generalize.

### Pattern 3: Dual transport in the frontend via a runtime switch

**What:** One Svelte build serves both contexts. At startup, the frontend detects whether it is inside the Tauri webview (`'isTauri' in window && !!window.isTauri`, with a fallback check for `window.__TAURI_INTERNALS__`). All subsequent calls go through a single `invoke(cmd, args)` function that dispatches to either `@tauri-apps/api/core`'s real `invoke` or to a `fetch('/api/' + cmd, { ... })` call.

**When to use:** Mandatory here.

**Trade-offs:**
- **Pro:** One bundle to build, one to ship. Feature parity is automatic. Iteration in the browser is instant (no Tauri rebuild).
- **Con:** Two error shapes to normalize (Tauri's `Promise.reject(string|object)` vs `fetch` non-2xx with JSON body). The transport wrapper hides that.

**Example:**
```typescript
// ui/src/lib/api/transport.ts
const inTauri = typeof window !== 'undefined'
  && ('isTauri' in window && (window as any).isTauri === true
      || '__TAURI_INTERNALS__' in window);

let tauriInvoke: (<T>(cmd: string, args?: unknown) => Promise<T>) | null = null;
if (inTauri) {
  // dynamic import so browser bundle does not crash
  tauriInvoke = (await import('@tauri-apps/api/core')).invoke;
}

export async function call<T>(cmd: string, args?: unknown): Promise<T> {
  if (tauriInvoke) return tauriInvoke<T>(cmd, args);
  const res = await fetch(`/api/${cmd}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', 'X-CSRF-Token': getCsrfToken() },
    credentials: 'include',
    body: args ? JSON.stringify(args) : '{}',
  });
  if (!res.ok) throw await res.json();
  return res.json() as Promise<T>;
}
```

> **Use `'isTauri' in window && window.isTauri === true` (added in Tauri 2.0.0-beta.9), falling back to `'__TAURI_INTERNALS__' in window`. Do NOT rely on `window.__TAURI__` — that only exists if `withGlobalTauri` is enabled.**

### Pattern 4: Typed RPC via tauri-specta (and same DTOs over HTTP)

**What:** Annotate every command with `#[specta::specta]`, collect them with `tauri_specta::collect_commands![]`, and emit a `bindings.ts` at build time (or in a `cargo test` step). All DTOs in `dto.rs` derive `serde::{Serialize, Deserialize}` and `specta::Type`. The same DTOs are used by axum handlers (because axum is just `Json<NewDeviceDto>` → `Json<DeviceDto>`), so the generated TypeScript types are the contract for HTTP too.

**When to use:** Mandatory here. Hand-written TS types will rot.

**Trade-offs:**
- **Pro:** End-to-end type safety. Removing a field in Rust breaks the TS build. tauri-specta handles dependent types correctly (unlike ts-rs, which handles types individually).
- **Con:** Generated file must be either committed or regenerated in CI; pick one and stick to it (recommend: generated in `cargo test` step, gitignored, regenerated by `npm` prebuild script).

> **Use tauri-specta v2, not ts-rs.** ts-rs exports types individually and cannot follow transitive dependencies cleanly; this becomes painful as the DTO graph grows.

### Pattern 5: Single tokio runtime, owned by Tauri

**What:** Tauri 2 starts a multi-threaded tokio runtime under the hood. Do not start a second one. The axum server runs as `tokio::spawn`'d task launched from `tauri::Builder::setup`. Background workers (SNMP poll, backup, low-stock) are also `tokio::spawn`'d. A single `CancellationToken` (or `tokio_util::sync::CancellationToken`) is held in `AppCtx` and dropped on shutdown.

**When to use:** Always here.

**Trade-offs:**
- **Pro:** No runtime conflicts, single thread pool, simple shutdown.
- **Con:** A CPU-bound task (PDF rendering for a large report, CSV import of 5000 rows) can starve other tasks. Mitigate with `tokio::task::spawn_blocking` for those specific operations.

### Pattern 6: Repository trait with `&self` — single writer is enforced inside the impl

**What:** All repo traits take `&self` (not `&mut self`). Concurrency is handled inside the sqlx-backed impl by keeping a `read_pool` (e.g., 4 connections) and a `write_pool` (1 connection). Reads grab from `read_pool`; writes grab from `write_pool` which naturally serializes.

**When to use:** Always with SQLite + sqlx.

**Trade-offs:**
- **Pro:** Eliminates writer-starvation under load (verified ~20× speedup in published benchmarks for offline-first apps using SQLx + WAL).
- **Con:** Two pools to wire. Worth it.

```rust
pub struct SqliteRepos {
    read: SqlitePool,   // SqlitePoolOptions::new().max_connections(4)
    write: SqlitePool,  // SqlitePoolOptions::new().max_connections(1)
}

// On startup:
let read = SqlitePoolOptions::new()
    .max_connections(4)
    .connect_with(
        SqliteConnectOptions::new()
            .filename(&paths.db)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .busy_timeout(Duration::from_secs(5))
            .read_only(true)
            .foreign_keys(true),
    ).await?;

let write = SqlitePoolOptions::new()
    .max_connections(1)
    .connect_with(
        SqliteConnectOptions::new()
            .filename(&paths.db)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .busy_timeout(Duration::from_secs(5))
            .foreign_keys(true)
            .pragma("temp_store", "memory")
            .pragma("mmap_size", "134217728"),  // 128 MB
    ).await?;

// migrations always run on the write pool
sqlx::migrate!("./migrations").run(&write).await?;
```

### Pattern 7: Mocks behind the same trait — feature-flagged or always present

**What:** Each port has a mock implementation in `trackly-infra::mocks`. Either gate behind `#[cfg(feature = "mocks")]` and enable in dev, or keep always-compiled (cost is tiny) and select at startup via config. For Trackly: **keep always-compiled** because the dev machine (macOS, no AD, no SNMP printers) must be able to start the app with mocks while developing.

**When to use:** Whenever the dev environment cannot exercise the integration. Here: always for AD and SNMP.

**Trade-offs:**
- **Pro:** Build the printer monitoring UI without a printer. Build AD login screens without a domain controller. Snapshot tests stay fast.
- **Con:** Small binary-size overhead. Negligible.

```rust
// Selection at startup:
let ad: Box<dyn AdClient> = match config.ad.enabled {
    true  => Box::new(LdapAdClient::new(&config.ad)?),
    false => Box::new(MockAdClient::with_seed_users(/* dev fixtures */)),
};
```

> Mock SNMP should return canned status sequences (e.g., "online → low-toner → offline → online") with configurable delays, so the UI's history view has something to look at.

---

## Data Flow

### Request Flow — Tauri (desktop admin)

```
User clicks "Create device" in webview
        ↓
Svelte component → call('create_device', { name, type, ... })
        ↓
transport.ts: detects inTauri=true → tauriInvoke('create_device', ...)
        ↓
@tauri-apps/api/core sends IPC to Rust
        ↓
#[tauri::command] create_device(state, dto)         ← tauri_cmds/devices.rs
        ↓
state.devices.create(dto.into())                    ← AppCtx
        ↓
DeviceService::create                                ← trackly-core
        ↓
self.repo.save(&device)                              ← DeviceRepo trait
        ↓
SqliteDeviceRepo::save (uses write_pool)             ← trackly-infra
        ↓
sqlx INSERT
        ↓
Returns Device → DeviceDto → JSON over IPC → Svelte updates store
```

### Request Flow — Browser (LAN user)

```
User clicks "Create request" in Chrome on a workstation
        ↓
Svelte component → call('create_request', { ... })
        ↓
transport.ts: detects inTauri=false → fetch('/api/create_request', POST + cookie + CSRF)
        ↓
axum router matches POST /api/create_request
        ↓
session middleware: validates session cookie → loads UserId
        ↓
CSRF middleware: validates X-CSRF-Token header against session-stored hash
        ↓
post_create_request(State(ctx), session, Json(dto))   ← http/requests.rs
        ↓
ctx.requests.create(dto.into(), user_id)              ← same service as above
        ↓
... identical path from here on ...
        ↓
Returns JSON; cookie refreshed by middleware
```

The lower half of both paths is byte-for-byte identical. That is the win.

### State Management (frontend)

```
Svelte stores (writable + derived)
    ↑                        ↓
    │  subscribe              │ call('...')
    │                         ↓
Components ←─── api/devices.ts (typed wrapper around call())
                              ↓
                         transport.ts
                              ↓
                       Tauri invoke  |  fetch()
```

Use plain Svelte stores. No Redux-style state library; the data lives in SQLite, the UI just caches the last query result. For real-time updates (SNMP status changes, new requests arriving), use Tauri's event system in desktop mode and Server-Sent Events (`text/event-stream`) in browser mode — both go through the same `subscribe(channel, handler)` wrapper, same way as `call()`.

### Key Data Flows

1. **Create device:** UI → transport → service → write pool → SQLite. Returns DTO.
2. **Search devices (full-text):** UI → transport → service → read pool → SQLite FTS5 virtual table. Returns paginated DTOs.
3. **Issue act (act_service.create_issue):** Multi-row transaction in single write txn. Acquires write_pool connection, begins txn: (a) reserves next act number using a counters table (with `UPDATE ... RETURNING` for atomicity), (b) updates device rows to `В работе`, (c) inserts act row, (d) inserts act_items rows, (e) commit. Either all succeed or none.
4. **CSV import (5000 rows):** Service receives parsed records, chunks into batches of 500, each batch is one txn on the write_pool. Background tokio task; progress reported via Tauri event or SSE. Use `tokio::task::spawn_blocking` for the CSV *parsing* step (it is sync and CPU-bound), then back to async for inserts.
5. **SNMP poll loop:** Background task spawned at startup. Every N seconds (configurable per printer), iterate enabled printers, query OIDs, persist new status via PrinterService. On status change, enqueue alert task.
6. **Backup task:** Scheduler fires (e.g., daily at 02:00 local time). Acquires the write_pool connection, runs `VACUUM INTO 'backups/trackly-YYYY-MM-DD.db'` (this is the SQLite-blessed way to snapshot a live WAL DB without races), prunes old backups per retention setting.
7. **Web login:** POST /api/auth/login → AuthService.login(username, password) → if user is local: argon2 verify; if user has `source = AD`: AdClient.bind(username, password). On success: create session in tower-sessions store, set HttpOnly+Secure+SameSite=Strict cookie, return CSRF token in response body.
8. **Tauri desktop session:** The webview hosts the admin. There is no login screen by default; the admin is implicitly trusted because they have local file access anyway. AuthService exposes `current_user_for_tauri()` which returns a synthetic "DesktopAdmin" identity. Optionally, a setting "lock desktop with password" can require login locally too — but ship the unlocked flow first.

---

## Scaling Considerations

| Scale | Architecture Adjustments |
|-------|--------------------------|
| 0–20 concurrent users, ≤5000 devices (v1 target) | Architecture as described. WAL + one writer + four readers comfortably handles this. SQLite database expected to be < 200 MB. |
| 50+ concurrent or > 50k devices | Increase read pool to 8, add caching layer (in-memory LRU in services) for hot reads (device listings). Consider FTS5 result caching. |
| 200+ concurrent or > 500k devices | Time to consider Postgres. Migration path is straightforward because sqlx is database-agnostic and repos are behind traits — swap `SqliteDeviceRepo` for `PostgresDeviceRepo`, change migration files, done. Not a v1 concern. |

### Scaling Priorities

1. **First bottleneck:** Write contention during bulk imports. Mitigation: chunked transactions, progress events, and background task isolation. Already in the design.
2. **Second bottleneck:** SNMP poll loop CPU during simultaneous polls of 50+ printers. Mitigation: bounded concurrency (`tokio::sync::Semaphore` with permit count of, say, 10).
3. **Third bottleneck:** PDF rendering blocking IPC. Mitigation: `tokio::task::spawn_blocking` for the render call.

> Do not preempt these. Ship, measure, then optimize.

---

## Anti-Patterns

### Anti-Pattern 1: Business logic in Tauri command handlers

**What people do:** Put validation, DB calls, and orchestration directly inside `#[tauri::command]` functions because "it's just easier".

**Why it's wrong:** Every behavior has to be duplicated in axum handlers, or worse, the HTTP endpoint silently behaves differently from the desktop one. Bug fixes go to one transport and not the other. Reviewers cannot tell which is canonical.

**Do this instead:** The `#[tauri::command]` is *exactly* the same shape as the axum handler — argument decode, single service call, response encode. If a handler is more than 20 lines, the logic belongs in a service.

### Anti-Pattern 2: Two SQLite pools opened by two parts of the app

**What people do:** Tauri code opens one `SqlitePool`, the axum bootstrap opens another, the backup task opens a third. Each thinks it owns the database.

**Why it's wrong:** WAL coordination is per-process for in-memory state. Two pools in one process can both try to checkpoint, can both hold locks, and busy-timeouts start interacting badly. Worse: a query in one pool can see a write from another only after the WAL is checkpointed, leading to "I just saved this but the next read does not see it" bugs.

**Do this instead:** One write pool (max_connections = 1), one read pool (max_connections = 4), constructed exactly once in `main.rs` and stored in `AppCtx`. Every IO goes through `AppCtx.repos`.

### Anti-Pattern 3: Using Tauri's path APIs for portable mode

**What people do:** Call `app.path().app_data_dir()` and expect it to return the executable directory.

**Why it's wrong:** Tauri's path APIs return OS-standard locations (`%APPDATA%`, `~/Library/Application Support`, `~/.local/share`). That is the opposite of portable.

**Do this instead:** Implement `paths.rs` manually. On startup:

```rust
fn resolve_paths() -> Result<Paths> {
    let exe = std::env::current_exe()?.canonicalize()?;
    let exe_dir = exe.parent().context("exe has no parent")?.to_path_buf();

    // Sentinel: if "portable.txt" or "trackly.config.json" sits next to the exe, use exe_dir.
    let portable_marker = exe_dir.join("portable.txt");
    let local_config    = exe_dir.join("trackly.config.json");

    let data_dir = if portable_marker.exists() || local_config.exists() {
        exe_dir.clone()
    } else if is_writable(&exe_dir) {
        // Heuristic for portable distribution (zipped, dropped on desktop)
        exe_dir.clone()
    } else {
        // Installed mode fallback: OS standard
        dirs::data_local_dir().context("no data dir")?.join("Trackly")
    };

    std::fs::create_dir_all(&data_dir)?;
    Ok(Paths {
        root: data_dir.clone(),
        db: data_dir.join("trackly.db"),
        config: data_dir.join("trackly.config.json"),
        logos: data_dir.join("logos"),
        backups: data_dir.join("backups"),
        templates: data_dir.join("templates"),
    })
}
```

> **DO ship a `portable.txt` sentinel file in the portable zip distribution. DO NOT rely solely on a writability probe — on Windows, Program Files might be writable for an admin user, leading to the wrong branch.**

### Anti-Pattern 4: Running migrations on the read pool

**What people do:** "Just run migrations on whichever pool starts first."

**Why it's wrong:** Read pool is opened with `read_only(true)`. Migrations will fail. Also you want migrations to complete before any handler — Tauri or axum — accepts a request.

**Do this instead:** Open the write pool first. Run `sqlx::migrate!("./migrations").run(&write_pool)`. Then open the read pool. Then construct services. Then start servers.

### Anti-Pattern 5: Long-running operations on the IPC/HTTP thread

**What people do:** Synchronously parse a 5000-row CSV inside a command handler, blocking the response for 30 seconds.

**Why it's wrong:** The webview/browser thinks the request died. Other users' requests queue. WAL writes from other tasks pile up.

**Do this instead:** Return a job id immediately. Spawn a background task with `tokio::spawn`, optionally `spawn_blocking` for the parsing step. Publish progress via Tauri events (desktop) and SSE (browser). UI shows progress bar.

### Anti-Pattern 6: Storing the AD password

**What people do:** Cache the user's domain password locally so they "do not have to keep typing it".

**Why it's wrong:** Compliance, security, and the user did not ask for it.

**Do this instead:** Bind to LDAP only to verify credentials. On success, mint a session cookie (web) and discard the password immediately. Never persist it, not in memory beyond the bind call, not in logs.

### Anti-Pattern 7: Two CSRF strategies, one for desktop and one for browser

**What people do:** Skip CSRF entirely for browser because "it's internal LAN".

**Why it's wrong:** A logged-in user on the same LAN visiting a malicious page is a credible threat. Trackly is reachable on a known port from the LAN.

**Do this instead:** CSRF middleware on axum for all non-GET routes. Use the synchronizer-token pattern (`axum-csrf-sync-pattern` crate). Token issued in login response, stored by frontend, sent in `X-CSRF-Token` header. Tauri path bypasses CSRF entirely because IPC is not subject to it.

### Anti-Pattern 8: Letting axum and Tauri define independent error shapes

**What people do:** axum returns `{ "error": "...", "code": 500 }`, Tauri returns a raw string. Frontend has two error parsers.

**Why it's wrong:** Doubles the surface area. Bugs in error handling become hard to spot.

**Do this instead:** Define `AppError` once. Derive `Serialize`. Implement `IntoResponse` for axum mapping. Both transports always emit the same JSON shape: `{ "kind": "ValidationError" | "NotFound" | "AuthRequired" | ..., "message": "...", "details": { ... } }`.

---

## Integration Points

### External Services

| Service | Integration Pattern | Notes |
|---------|---------------------|-------|
| **Active Directory (LDAP)** | `ldap3` crate, `LdapConnAsync`, simple bind for verification, optional StartTLS. Behind `AdClient` trait. | Use `Box<dyn AdClient>` so the mock is swappable. Connection per bind (do not pool — bind state is per-connection). Timeouts: 5 s connect, 10 s op. |
| **SNMP printers** | `snmp2` crate (dependency-free, tokio feature flag) for v2c/v3 GET. Behind `SnmpClient` trait. | Maintain per-vendor OID tables in `infra::snmp::profiles`. Pantum, Kyocera, HP, Canon get explicit profiles; others fall back to RFC 3805 standard printer MIB. |
| **SMTP** | `lettre` crate, async tokio transport. Behind `Mailer` trait. | One transport instance, reused. Async send queue with retry. |
| **Telegram bot** | `teloxide` or raw `reqwest` to Bot API. Behind `Notifier` trait (same trait as SMTP via dynamic dispatch, or separate traits depending on shape). | Defer to last phase. |
| **Webhook (outbound)** | `reqwest` POST with JSON body. Behind `Webhook` trait. | Configurable per-event. |
| **WMI / RPC (Pantum spooler restart, future)** | `wmi-rs` (Windows only) or PowerShell spawned via `tokio::process::Command`. Gated behind cfg(target_os = "windows") and explicit user confirmation. | Phase post-v1. Only after observation phase confirms the hypothesis. |
| **PDF rendering** | `typst` or `printpdf` or `genpdf` for native rendering; alternative is `chromium`-based via Tauri's webview itself — render an HTML template in a hidden window and capture. | Start with `genpdf` (simpler, deterministic). Reassess if templates need rich CSS. |

### Internal Boundaries

| Boundary | Communication | Notes |
|----------|---------------|-------|
| **Svelte UI ↔ trackly-app** | JSON over Tauri IPC or HTTP/JSON. Types generated by tauri-specta. | Single contract; one set of bindings.ts consumed by both. |
| **trackly-app ↔ trackly-core** | Direct function calls. `tauri-app` constructs services and calls service methods. | `trackly-core` re-exports the DTO-input/output types via `pub use`. |
| **trackly-core (services) ↔ trackly-core (ports)** | `&self` method calls on trait objects or generics. `async-trait` for the traits. | Services do not know about sqlx/ldap/snmp. |
| **trackly-core (ports) ↔ trackly-infra (adapters)** | Trait impl. | Replaceable by mocks for tests and dev. |
| **trackly-app axum router ↔ trackly-app tauri commands** | None directly. They share `AppCtx` but never call each other. | Each is a peer of the other. |
| **Background tasks ↔ services** | Tasks hold an `Arc<AppCtx>` (or `Arc<DeviceService<...>>` etc.) and call service methods like any other caller. | Shutdown via shared `CancellationToken`. |
| **Server toggle (UI control) ↔ axum lifecycle** | Service method `settings.set_server_enabled(bool)` → emits an internal event → supervisor task starts/stops axum. | Bind/unbind without restarting the desktop app. |

---

## Cross-Cutting Concerns

### Authentication & sessions

- **Local users (v1):** username + argon2-verified password. Sessions in tower-sessions (Axum integration) with a SQLite-backed store living in the same DB (`sessions` table). HttpOnly + Secure (when HTTPS) + SameSite=Strict cookies, 8-hour rolling expiry. CSRF synchronizer token issued at login.
- **AD users (later phase):** Same flow, but verification is `AdClient::bind`. User row in DB has `source = 'ad'` and no password hash. AD users register either automatically (if auto-accept setting enabled) or via a request that admin approves.
- **Tauri desktop:** No login required by default; the admin is whoever has the desktop. The `AuthService` exposes `desktop_identity()` returning a hardcoded admin. Optionally gate behind a "lock" setting; ship unlocked first.
- **HTTPS:** Self-signed certificate auto-generated on first run (use `rcgen` crate). Path to user-provided cert is configurable in settings. Server runs on configurable port (default 8443).

### Background tasks & persistence of timers

- All scheduled tasks compute "next run at" timestamps and persist them in a `scheduled_tasks` table on every reschedule. On startup, the supervisor reads this table and immediately fires tasks whose next-run is in the past (with debouncing to avoid storms after a long downtime).
- One supervisor task spawns each worker. Workers receive a `CancellationToken` clone; on shutdown, the supervisor cancels the token and `join_all`s the workers.
- SNMP polling: per-printer next-poll timestamp in DB. Supervisor picks the next-due printer and dispatches.
- Backup: cron-like config (default daily 02:00). Compute next run from now + interval, persist.

### Database migrations

- Use `sqlx::migrate!("./migrations")` in `trackly-infra`. Migrations are embedded in the binary at compile time — no migration files needed at runtime, perfect for portable distribution.
- Forward-only. SQLite does not support reversible DDL cleanly; reversal is a new forward migration. Practice this discipline from day one.
- User-edited templates live in DB (`document_templates` table). Migrations that *replace* template content should be careful: change template *schema* freely, change template *content* only with a "is_default" flag and a row-per-version model so users who customized can keep their version.
- Pre-flight check on startup: open the DB, query `PRAGMA user_version`, and if the on-disk version is *higher* than what the binary knows about (e.g., user opened an older binary against a newer DB), refuse to start with a clear error. Do NOT silently downgrade.

### Frontend transport detection — concrete recipe

```typescript
// ui/src/lib/api/transport.ts
type TauriWindow = Window & { isTauri?: boolean; __TAURI_INTERNALS__?: unknown };

function detectTauri(): boolean {
  if (typeof window === 'undefined') return false;
  const w = window as TauriWindow;
  if (typeof w.isTauri === 'boolean') return w.isTauri;
  return '__TAURI_INTERNALS__' in w;
}

export const inTauri = detectTauri();
```

The detection runs once at module load. Subsequent decisions are O(1).

### File storage layout (portable)

```
<exe_dir>/
├── trackly.exe                       (or trackly on macOS/Linux)
├── portable.txt                      (sentinel — opt-in marker; can be empty)
├── trackly.config.json               (port, host, server-enabled, backup schedule, log level)
├── trackly.db                        (SQLite WAL file is trackly.db-wal, shm is trackly.db-shm)
├── logos/
│   └── org-logo.png
├── templates/                        (initially empty; templates default-load from DB)
├── backups/
│   ├── trackly-2026-05-24.db
│   └── trackly-2026-05-23.db
├── logs/
│   ├── trackly.log
│   └── trackly.log.1
└── certs/
    ├── server.crt
    └── server.key
```

When the user moves the .exe (with siblings) to another folder, everything works. When the user zips it and emails it to a colleague, everything works.

> **Do NOT use `\\?\C:\...` long-path-prefixed paths in stored config.** When the user moves the directory, the prefix breaks. Always store *relative* paths in config (relative to exe_dir) and resolve at runtime.

### Build & release

- **GitHub Actions CI** (push): `cargo clippy -- -D warnings`, `cargo test --workspace`, `cargo test -p trackly-infra --features sqlx-test` (if integration tests), `cd ui && npm ci && npm run check && npm run build`.
- **GitHub Actions Release** (tag): matrix build on `windows-latest` (msi + portable zip), `macos-14` (arm64 .dmg), `ubuntu-latest` (.AppImage + .deb). Upload artifacts to the GH Release.
- **Portable build differs from installed build only by:** (a) packaging into a zip with `portable.txt` next to the exe; (b) the installer build runs `tauri build` with a `--config tauri.installed.conf.json` that has installer metadata. Same binary, different packaging.
- **Windows 7 32-bit** (stretch): pin Rust toolchain to one known to support Win7 (currently any 1.77+); pin Tauri to a version with WebView2 fallback / Edge legacy support — this is a research item for Phase 2, not a v1 blocker.

---

## Suggested Build Order

> This is the phase-decomposition input for the roadmap. Each step delivers something testable.

1. **Foundation: workspace + core types + paths + config + sqlx pool + WAL + migrations skeleton.**
   - Deliverable: `cargo run` starts, opens DB next to exe (or in OS dir as fallback), runs zero migrations, exits.
   - Why first: every later step relies on the storage layer existing. Schema iterations are cheapest now.
2. **Domain + first service (DeviceService) + first repo (SqliteDeviceRepo) + service-level tests using a mock repo.**
   - Deliverable: `cargo test` runs services against mocks. `trackly-app` does not exist yet (or is a no-op).
3. **Bootstrapping `trackly-app`: AppCtx construction, paths resolution, single Tauri window showing static Svelte page.**
   - Deliverable: app opens, shows "hello", no IPC yet.
4. **First Tauri command (create_device, list_devices). tauri-specta wiring. bindings.ts generated.**
   - Deliverable: Svelte page shows a list of devices read from SQLite, create button works. End-to-end vertical slice on the Tauri path.
5. **Axum router + AppCtx::clone() into `with_state`. First HTTP endpoint mirroring the Tauri command. Browser-only Svelte build (vite dev server proxied to axum).**
   - Deliverable: same Svelte page works in Chrome at `http://localhost:8443`. Dual transport proven.
6. **Transport detection in Svelte. Same source serves both.**
7. **Schema build-out: Acts, Cartridges, Requests, Users, Settings, Document Templates. Migration per entity.**
   - Long phase; pieces ship in sub-phases per the roadmap's feature decomposition.
8. **Auth: local users, sessions (tower-sessions), CSRF, login screen for browser. Desktop bypass.**
9. **Background scheduler skeleton + first scheduled job (backup).**
10. **SNMP behind `SnmpClient` trait. Mock impl first (so UI ships before any printer is plugged in), then real `snmp2`-backed impl.**
11. **AD behind `AdClient` trait. Mock first, real ldap3-backed later.**
12. **PDF rendering for Acts and Reception documents.**
13. **Reports + dashboard widgets.**
14. **Notifications (SMTP, Telegram, webhook). Last because they depend on stable event sources.**
15. **Pantum auto-restart spooler (post-v1).**

**Hard dependency: 1 → 2 → 3 → 4 → 5 → 6. Then 7 can run in parallel with 8 and 9. Then 10 and 11 are independent. 12–14 are independent of each other but depend on their respective data sources.**

---

## Confidence Notes

- **HIGH** confidence: hexagonal-in-Rust, sqlx + WAL with split read/write pools, tauri-specta over ts-rs, transport detection via `isTauri`/`__TAURI_INTERNALS__`, single tokio runtime, embedded migrations, portable-mode path resolution shape. Verified across official docs and active-community sources.
- **MEDIUM** confidence: choice of `snmp2` vs `csnmp` vs `async-snmp` — all three are credible, ecosystem is young, pick after a one-day spike against a real Pantum or a netSNMP simulator. The recommendation is `snmp2` because it is dependency-free and supports v1/v2c/v3 in one crate.
- **MEDIUM** confidence: `lettre` for SMTP, `axum-csrf-sync-pattern` for CSRF, `tower-sessions` for sessions — standard choices but exact API shapes drift; verify when wiring.
- **LOW** confidence: PDF library choice; depends on template complexity and Russian font handling. Reassess in Phase 12.
- **LOW** confidence: Windows 7 32-bit feasibility. Treat as stretch goal; do not block v1.

---

## Sources

- [Tauri 2 path API](https://v2.tauri.app/reference/javascript/api/namespacepath/) — official
- [Tauri SQL plugin](https://v2.tauri.app/plugin/sql/) — official (note: we are NOT using this plugin; we use sqlx directly because we share the pool with axum)
- [Tauri portable mode discussion (data dir next to exe)](https://github.com/tauri-apps/tauri/discussions/8719) — community, real-world recipes
- [Detecting Tauri webview in frontend](https://github.com/tauri-apps/tauri/discussions/6119) — official discussion, confirms `isTauri` vs `__TAURI_INTERNALS__`
- [Tauri + SQLite + Axum walkthrough (Medium, 2024–2025)](https://ritik-chopra28.medium.com/build-a-cross-platform-desktop-app-in-rust-tauri-2-0-sqlite-axum-2b9b7b732e0d) — pattern reference for AppCtx and dual transport
- [Tauri shared database pool](https://medium.com/@deejiw/tauri-with-shared-database-pool-e25aec033ed3) — confirms `tauri::State` pattern with `Arc<SqlitePool>`
- [tauri-specta v2](https://github.com/specta-rs/tauri-specta) — official repo
- [tauri-specta docs](https://docs.rs/tauri-specta/latest/tauri_specta/) — official
- [Specta on dependent-type handling vs ts-rs](https://github.com/specta-rs/specta) — explicit comparison
- [SQLx + WAL: separate read/write pools (Evan Schwartz)](https://emschwartz.me/psa-your-sqlite-connection-pool-might-be-ruining-your-write-performance/) — benchmark + recipe
- [SQLite WAL official docs](https://sqlite.org/wal.html) — authoritative on writer serialization
- [SqliteConnectOptions in sqlx](https://docs.rs/sqlx/latest/sqlx/sqlite/struct.SqliteConnectOptions.html) — official
- [sqlx::migrate! macro](https://docs.rs/sqlx/latest/sqlx/macro.migrate.html) — official, confirms compile-time embedding
- [ldap3 crate](https://github.com/inejge/ldap3) — official; async, simple bind, optional GSSAPI/NTLM features
- [snmp2 crate](https://crates.io/crates/snmp2) — dependency-free SNMP v1/v2c/v3, tokio feature
- [async-snmp](https://github.com/lukeod/async-snmp) — alternative; modern async-first, ecosystem note: marked unstable
- [axum-csrf-sync-pattern](https://lib.rs/crates/axum-csrf-sync-pattern) — synchronizer-token CSRF middleware
- [tower-sessions](https://docs.rs/tower-sessions) — session middleware for axum
- [Master Hexagonal Architecture in Rust (howtocodeit.com)](https://www.howtocodeit.com/guides/master-hexagonal-architecture-in-rust) — pattern reference for trait-bound services + mockability
- [Hexagonal architecture in Rust (Barrage)](https://www.barrage.net/blog/technology/how-to-apply-hexagonal-architecture-to-rust) — companion reference

---
*Architecture research for: Tauri 2 + axum hybrid desktop/LAN server with SQLite WAL portable mode*
*Researched: 2026-05-24*

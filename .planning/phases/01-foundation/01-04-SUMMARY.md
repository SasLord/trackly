---
phase: 01-foundation
plan: 04
subsystem: core+infra+app
tags: [rust, app-error, secret, clock, zeroize, sqlite, wal, single-writer, mpsc, reader-pool, raii, appctx, probe-read, downgrade-protection, sha256]

requires:
  - 01-01 (workspace + MSRV 1.88 + rusqlite 0.38 + refinery 0.9 + tokio-util)
  - 01-02 (Paths::resolve_for_exe_dir test seam + AppConfig with paths.db_path empty sentinel + webview_env)
  - 01-03 (db::pragmas::{apply_writer_pragmas, apply_reader_pragmas}, db::migrations::run + MigrationReport, test_support::test_db; SCHEMA_VERSION=12)
provides:
  - trackly_core::error::AppError — full 9-variant enum per D-AppError-01 (NotFound/Conflict/OptimisticLockMismatch/WriteQueueBusy/DatabaseFromNewerVersion/Validation/Unauthorized/Forbidden/Internal) with unified {code, message, details} JSON shape via manual `impl Serialize`
  - trackly_core::primitives::Secret<T: Zeroize + Clone> — newtype with Debug=`"***"`, Drop calls zeroize, NO Serialize/Deserialize (compile-time gated by static_assertions in tests/secret_zeroize.rs)
  - trackly_core::primitives::Clock trait (now/unix_seconds) — dyn-compatible, Send+Sync
  - trackly_infra::clock_impl::SystemClock — production `Clock` impl returning `OffsetDateTime::now_utc()`
  - trackly_infra::error_conversions::{map_rusqlite, map_refinery, map_send_timeout, map_oneshot_recv} — free-fn mappers (orphan rule prevents `impl From` here)
  - trackly_infra::db::writer_worker::WriterHandle — single-writer mpsc(256) + spawn_blocking worker; `execute<F,R>(closure) -> Result<R, AppError>` with 5s send_timeout → WriteQueueBusy; `spawn_with_capacity(conn, n)` test-only constructor; `with_send_timeout(d)` test-only override; consts DEFAULT_WRITER_CAPACITY=256 + DEFAULT_SEND_TIMEOUT=5s
  - trackly_infra::db::pools::{ReaderPool, ReaderHandle} — Mutex<Vec<Connection>> LIFO pool; READ_ONLY | NO_MUTEX flags; query_only=ON; RAII ReaderHandle Derefs to Connection, drops back to pool
  - trackly_infra::db::migrations::max_known_version() -> u32 — returns 12 (Plan 04 AppCtx probe-read compares against this)
  - trackly_infra::test_support::test_writer_and_readers() -> (Arc<WriterHandle>, Arc<ReaderPool>, TempDir) — canonical fixture for tests
  - trackly_app::context::AppCtx — Clone struct {writer, readers, paths, config, clock, shutdown, log_guard, schema_version} + AppCtx::build(paths, config, log_guard).await → anyhow::Result<Self> with mandatory probe-read downgrade check (SQLITE_OPEN_READ_ONLY | SQLITE_OPEN_URI) BEFORE writer open
  - trackly_app::shutdown::install_signal_handler — Ctrl-C → CancellationToken.cancel()
  - trackly_app::error_axum — Plan 05 stub
  - `trackly --self-test` prints `self-test OK: schema_version=12, portable=<bool>` then exits 0; full lifecycle: paths → webview env → config → tracing-appender placeholder → tokio rt → AppCtx::build (probe + writer + migrations + worker spawn + reader pool) → diagnostics → drop ctx (graceful shutdown of writer)
  - Phase 1 success criterion #2 satisfied: tests/concurrent_writes.rs — 50 parallel writes (25 tauri + 25 axum labels), 0 SQLITE_BUSY, 0 WriteQueueBusy, all 50 rows present
  - Phase 1 success criterion #4 satisfied: tests/downgrade_protection.rs — user_version=999 forced, AppCtx::build returns DatabaseFromNewerVersion {binary:12, file:999}, SHA256 of `.db` + `.db-wal` byte-identical via single String==String
affects: [05-tauri-specta-axum, 06-procmon-ci, all-future-phases]

tech-stack:
  added:
    - sha2 = "0.10" (trackly-app dev-dep, downgrade_protection.rs SHA256 byte-identity)
    - static_assertions = "1" (trackly-core dev-dep, compile-time check Secret: !Serialize)
  patterns:
    - "free-fn error mappers (`map_rusqlite`/`map_refinery`/`map_send_timeout`/`map_oneshot_recv`) at I/O callsites — orphan rule forbids `impl From<rusqlite::Error> for AppError` in trackly-infra"
    - "single-writer mpsc + spawn_blocking — `WriterHandle::execute(|conn| { ... })` is the ONLY supported write path; readers go through `ReaderPool::acquire()`"
    - "probe-read downgrade-protection pattern in AppCtx::build — SQLITE_OPEN_READ_ONLY | SQLITE_OPEN_URI conn, explicit drop, before writer open; guarantees byte-identical file on rejection"
    - "AppCtx as Arc-wrapped composition root; all sub-states wrapped in Arc so cheap to Clone for handler dispatch"
    - "RAII ReaderHandle with Deref to Connection — caller writes `let guard = pool.acquire(); guard.query_row(...)`"

key-files:
  created:
    - crates/trackly-core/src/primitives/mod.rs
    - crates/trackly-core/src/primitives/secret.rs
    - crates/trackly-core/src/primitives/clock.rs
    - crates/trackly-core/tests/secret_zeroize.rs
    - crates/trackly-infra/src/clock_impl.rs
    - crates/trackly-infra/src/error_conversions.rs
    - crates/trackly-infra/src/db/writer_worker.rs
    - crates/trackly-infra/src/db/pools.rs
    - crates/trackly-infra/src/test_support/test_app_ctx.rs
    - crates/trackly-app/src/context.rs
    - crates/trackly-app/src/shutdown.rs
    - crates/trackly-app/src/error_axum.rs
    - crates/trackly-app/tests/concurrent_writes.rs
    - crates/trackly-app/tests/downgrade_protection.rs
  modified:
    - crates/trackly-core/src/error.rs (full 9-variant enum, manual Serialize, was 2-variant stub)
    - crates/trackly-core/src/lib.rs (pub mod primitives + re-exports)
    - crates/trackly-core/Cargo.toml (dev-dep static_assertions)
    - crates/trackly-infra/src/lib.rs (pub mod clock_impl + error_conversions; re-export SystemClock)
    - crates/trackly-infra/src/db/mod.rs (pub mod pools + writer_worker; re-exports)
    - crates/trackly-infra/src/db/migrations.rs (added max_known_version() + test)
    - crates/trackly-infra/src/test_support/mod.rs (pub mod test_app_ctx + re-export)
    - crates/trackly-app/Cargo.toml (rusqlite as runtime dep; sha2 + tracing-appender as dev-deps)
    - crates/trackly-app/src/lib.rs (declare context + shutdown + error_axum modules)
    - crates/trackly-app/src/main.rs (full Steps 1-8 ordering with AppCtx::build inside tokio rt)

key-decisions:
  - "Free-fn error mappers in trackly_infra::error_conversions (NOT `impl From`) — Rust orphan rule forbids `impl From<rusqlite::Error> for AppError` in trackly-infra because trackly-infra owns neither the From trait nor AppError. Callsites use `.map_err(map_rusqlite)?` etc."
  - "ReaderPool kept simple: std::sync::Mutex<Vec<Connection>> LIFO. Phase 1 accepts the panic-on-exhaustion semantics; if real contention emerges in Phase 2+ swap to deadpool without changing the public surface (`acquire() -> ReaderHandle`)."
  - "Probe-read downgrade check uses `SQLITE_OPEN_READ_ONLY | SQLITE_OPEN_URI` and explicit drop(probe) BEFORE Connection::open(writer) — verified by downgrade_protection.rs SHA256 byte-identity assertion (single String == String, no size/header fallback needed)."
  - "rusqlite promoted from [dev-dependencies] to [dependencies] in trackly-app/Cargo.toml — context.rs needs `rusqlite::{Connection, OpenFlags}` for the probe-read step. The hexagonal split is preserved (trackly-core remains rusqlite-free; no_io_deps.rs gate still green)."
  - "WriterHandle::execute is `#[must_use]` — accidental `let _ = writer.execute(...)` (no `.await`) loses the Result silently; the attribute makes it a clippy warning at every call site."
  - "WriterHandle is `Clone` (mpsc::Sender is cheaply clonable) — callers don't need to wrap in Arc themselves. AppCtx.writer is still Arc<WriterHandle> for consistency with readers (also Arc-wrapped) and to support 0-cost ctx.clone() in Phase 5+ axum handlers."
  - "WriterHandle::spawn_with_capacity + WriterHandle::with_send_timeout exposed as #[doc(hidden)] pub for the backpressure test (need capacity=1 + timeout=100ms to provoke WriteQueueBusy within a test). Documentation marks them test-only."
  - "AppCtx not derived Debug (rusqlite::Connection inside WriterHandle isn't Debug). downgrade_protection.rs uses `match result { Ok(_) => panic!(...), Err(e) => e }` instead of `.expect_err`."
  - "main.rs Step 5 tracing-appender is a TRUE placeholder: `let (non_blocking, log_guard) = tracing_appender::non_blocking(std::io::stderr()); let _ = non_blocking;` — the WorkerGuard threads through AppCtx::build per the plan's signature; Plan 05 will replace with a real tracing subscriber."

requirements-completed: [FOUND-01, FOUND-02, FOUND-03, FOUND-05, FOUND-06]

duration: ~25 min
completed: 2026-05-25
---

# Phase 1 Plan 04: AppError + Secret + Clock + WriterHandle + ReaderPool + AppCtx + concurrent_writes + downgrade_protection Summary

**Full Phase 1 spine landed: `AppError` (9 variants, unified JSON shape), `Secret<T>` (zeroize on drop, no Serialize via compile-time gate), `Clock` trait + `SystemClock` impl, single-writer `WriterHandle` (mpsc 256 + spawn_blocking + 5s send_timeout → `WriteQueueBusy`), `ReaderPool` (RAII guards over 4 read-only conns), `AppCtx` composition root with mandatory probe-read downgrade protection (`SQLITE_OPEN_READ_ONLY | SQLITE_OPEN_URI` before writer open, explicit drop, file BYTE-UNTOUCHED on rejection). `trackly --self-test` now exits 0 after the full lifecycle prints `self-test OK: schema_version=12, portable=<bool>`. Phase 1 success criteria #2 (50 concurrent writes, 0 SQLITE_BUSY) and #4 (newer DB → graceful error + SHA256-equal file) locked by `tests/concurrent_writes.rs` and `tests/downgrade_protection.rs`.**

## Performance

- **Duration:** ~25 min wall clock
- **Started:** 2026-05-25T (per session)
- **Tasks:** 3 / 3 (all type=auto; per-task atomic commits)
- **Files created:** 14
- **Files modified:** 10

## Accomplishments

- **`AppError` is complete** — 9 variants per D-AppError-01 (NotFound, Conflict, OptimisticLockMismatch, WriteQueueBusy, DatabaseFromNewerVersion, Validation, Unauthorized, Forbidden, Internal); `code()` returns stable SCREAMING_SNAKE; manual `impl Serialize` produces the unified `{code, message, details}` JSON; all 10 lib unit tests in `crates/trackly-core/src/error.rs` exercise round-trip serialization for every variant.
- **`Secret<T>`** zeroize-on-drop newtype with manual `impl Debug` writing the literal `"***"`. Compile-time gate via `static_assertions::assert_not_impl_all!(Secret<String>: serde::Serialize)` in `crates/trackly-core/tests/secret_zeroize.rs` makes accidental `#[derive(Serialize)]` a compilation failure.
- **`Clock` trait + `SystemClock`** — clock lives in `trackly-core::primitives::clock` (trait); `SystemClock` impl in `trackly-infra::clock_impl` using `time::OffsetDateTime::now_utc()`. The `disallowed-methods` lint banning `chrono::Local::now` is unaffected — `time` crate is the canonical UTC source.
- **`error_conversions` free-fns** — `map_rusqlite` (SQLITE_BUSY/LOCKED → `WriteQueueBusy`, ConstraintViolation → `Conflict`, else → `Internal`), `map_refinery` → `Internal`, `map_send_timeout` → `WriteQueueBusy`, `map_oneshot_recv` → `Internal { source_chain: "writer worker dropped reply channel" }`. Live in `trackly-infra` because of Rust orphan rule.
- **`WriterHandle`** — single-writer pattern: `mpsc::channel::<BoxedJob>(256)` + `tokio::task::spawn_blocking` worker that owns the `rusqlite::Connection`. `execute(closure)` boxes the closure + a `oneshot::Sender<R>` and `send_timeout(5s)`s the envelope. Worker uses `rx.blocking_recv()` inside the blocking task. `Clone`, `#[must_use]` on `execute`, fire-and-forget worker (graceful shutdown when last sender drops).
- **`ReaderPool`** — `std::sync::Mutex<Vec<Connection>>` LIFO; `OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX` + `query_only=ON` pragma. RAII `ReaderHandle` derefs to `&Connection` and pushes back on drop. Acquire-on-exhaustion panics; Phase 2+ can swap to `deadpool` without changing the public surface.
- **`max_known_version()`** in `db::migrations` — returns `12` (computed at call time from `migrations::runner().get_migrations()` max version). Plan 04's AppCtx probe-read compares against this.
- **`test_writer_and_readers()`** in `test_support::test_app_ctx` — full fixture (writer + readers over fresh tempfile DB with all migrations); used by `tests/concurrent_writes.rs` directly.
- **`AppCtx::build` lifecycle exact per `<interfaces>`:** Step 6a resolves `db_path` (config override if non-empty, else `paths.db_path()`). Step 6b: if file exists, opens **read-only probe** with `SQLITE_OPEN_READ_ONLY | SQLITE_OPEN_URI`, reads `PRAGMA user_version`, explicitly drops the probe, and returns `DatabaseFromNewerVersion` if `on_disk > 12`. Step 7: opens writer with `Connection::open(&db_path)`. Step 8: writer pragmas + migrations. Step 9: `WriterHandle::spawn(writer_conn)`. Step 10: `ReaderPool::new(&db_path, 4)`. Returns `AppCtx { writer, readers, paths, config, clock, shutdown, log_guard, schema_version }` all `Arc`-wrapped.
- **`main.rs` ordered init complete** — Steps 1-8: Paths → WEBVIEW2 → `--self-test` parse → config → tracing-appender placeholder + WorkerGuard → tokio multi-thread runtime → `AppCtx::build(paths, config, log_guard).await?` → self-test branch prints `self-test OK: schema_version=12, portable=...` and drops ctx (which graceful-stops the writer worker).
- **`tests/concurrent_writes.rs`** — `#[tokio::test(flavor = "multi_thread", worker_threads = 4)]`. Pre-creates a `concurrent_test` table via one `writer.execute(...)`. Spawns 50 `tokio::spawn` tasks (25 with `tauri:` payload prefix, 25 with `axum:`). Joins all; asserts 0 errors. Then `tokio::task::spawn_blocking({ let r = readers.clone(); move || { let g = r.acquire(); g.query_row("SELECT COUNT(*) ...") })` to read back; asserts count == 50, tauri-count == 25, axum-count == 25.
- **`tests/downgrade_protection.rs`** — `#[tokio::test(flavor = "multi_thread")]`. Workflow: open writer manually → migrations → `pragma_update(None, "user_version", 999_u32)` → drop conn → `snapshot()` returns SHA256 of `.db` (+ `.db-wal` if present). Call `AppCtx::build`. Downcast `anyhow::Error` to `&AppError`; match `DatabaseFromNewerVersion { binary: 12, file: 999 }`. Snapshot again; **`assert_eq!(before, after, ...)` is a single `String == String` comparison** (no size/header fallback). The probe-read pattern guarantees byte-identity.
- **All gates green:** 21 trackly-infra lib unit tests + 13 trackly-infra integration tests + 14 trackly-core lib tests + 3 trackly-core integration tests (secret_zeroize) + 2 trackly-app integration tests (concurrent_writes, downgrade_protection) + 1 trackly-core no_io_deps gate. `cargo clippy --workspace --all-targets -- -D warnings` clean. `cargo fmt --all -- --check` clean. `cargo run -p trackly-app -- --self-test` exits 0 with `self-test OK: schema_version=12`.

## Task Commits

1. **Task 1: AppError full enum + Secret<T> + Clock + error_conversions** — `5a95270`
2. **Task 2: WriterHandle + ReaderPool + max_known_version + test_app_ctx fixture** — `2a135ec`
3. **Task 3: AppCtx::build probe-read lifecycle + main.rs full ordered init + concurrent_writes + downgrade_protection** — `ade8f0c`

_Final plan-metadata commit will be added by the orchestrator after this SUMMARY is written._

## Decisions Made

See `key-decisions` frontmatter above for the full list with rationale.

Most impactful for downstream consumers:

- **`map_rusqlite` / `map_refinery` / `map_send_timeout` / `map_oneshot_recv` are free-fns**, NOT `impl From`. Phase 2+ code that maps SQLite errors writes `.map_err(map_rusqlite)?` at every callsite. Orphan rule prevents the `impl` form (trackly-infra owns neither `From` nor `AppError`).
- **`AppCtx` is the parameter shape for every Tauri command and axum handler from Phase 2+.** It's `Clone` (everything wrapped in `Arc`) so cheap to clone per request.
- **`WriterHandle::execute<F, R>(closure)`** is the ONLY supported write path. Bypassing it (e.g., grabbing a `Connection` directly) reintroduces SQLITE_BUSY. The `#[must_use]` attribute catches accidental dropped futures at compile time.
- **`ReaderPool::acquire()`** is sync; from async code call inside `tokio::task::spawn_blocking({ let r = readers.clone(); move || { let g = r.acquire(); ... } }).await?`.
- **`max_known_version() -> u32`** — Plans 05+ that want to display schema version use this OR `AppCtx.schema_version` (which is the same value after a successful `build`).
- **rusqlite was promoted to a runtime dep of trackly-app** because context.rs needs `Connection::open_with_flags`. The hexagonal boundary still holds (trackly-core remains pure-domain; no_io_deps gate green).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `impl From<rusqlite::Error> for AppError` impossible in trackly-infra (Rust orphan rule)**

- **Found during:** Task 1 (initial wiring of `error_conversions.rs`)
- **Issue:** Plan's `<interfaces>` says "Conversion impls (`From<rusqlite::Error>`, …) live in trackly-infra". Rust orphan rule requires the impl crate to own EITHER the trait OR the type; trackly-infra owns neither (`From` is std, `AppError` is in trackly-core). Adding rusqlite as a feature-gated dep of trackly-core would violate the `no_io_deps` test invariant.
- **Fix:** Switched to free-fn mappers (`map_rusqlite`, `map_refinery`, `map_send_timeout`, `map_oneshot_recv`) at every call site. Same semantics, ergonomic at callsites via `.map_err(map_rusqlite)?`. Documented this in error_conversions.rs module doc-comment.
- **Files modified:** `crates/trackly-infra/src/error_conversions.rs`
- **Verification:** `cargo test -p trackly-infra --lib error_conversions` passes (5 tests covering query-no-rows, busy, constraint, oneshot recv, send timeout).
- **Committed in:** `5a95270`

**2. [Rule 3 - Blocking] `refinery::Migration::version()` returns i32, not u32**

- **Found during:** Task 2 (first build of `max_known_version()`)
- **Issue:** Plan's `<interfaces>` showed `pub fn max_known_version() -> u32` summing `m.version() as u32`. Refinery 0.9's `Migration::version()` returns `i32` (not the older `usize` form some examples show). Direct `.max().unwrap_or(0)` returns `i32`, not `u32`.
- **Fix:** Compute as `i32` first, then `u32::try_from(max_i32).expect("migration version must be non-negative")` — migration versions are non-negative by refinery's contract.
- **Files modified:** `crates/trackly-infra/src/db/migrations.rs`
- **Verification:** `cargo test -p trackly-infra --lib db::migrations::tests::max_known_version_returns_twelve` passes.
- **Committed in:** `2a135ec`

**3. [Rule 2 - Critical] rusqlite needed as runtime dep of trackly-app (not just dev-dep)**

- **Found during:** Task 3 (first build of `crates/trackly-app/src/context.rs`)
- **Issue:** `context.rs` uses `rusqlite::{Connection, OpenFlags}` for the probe-read step. Plan's `files_modified` listed only `sha2` + `tracing-appender` for Cargo.toml changes — rusqlite was not in the list because the planner assumed all DB interactions would route through trackly-infra. The probe-read step inherently needs the rusqlite types in context.rs (we don't want to expose `Connection::open_with_flags` through trackly-infra just for this one bootstrap path).
- **Fix:** Added `rusqlite = { workspace = true }` to `[dependencies]` of `trackly-app/Cargo.toml`. The hexagonal boundary remains (trackly-core is still rusqlite-free; `no_io_deps.rs` green); trackly-app is the composition root and is allowed any I/O dep it needs.
- **Files modified:** `crates/trackly-app/Cargo.toml`
- **Verification:** `cargo build -p trackly-app` succeeds; `cargo test -p trackly-core --test no_io_deps` still passes.
- **Committed in:** `ade8f0c`

**4. [Rule 1 - Bug] `AppCtx` is not `Debug`, so `result.expect_err(...)` fails to compile in downgrade_protection.rs**

- **Found during:** Task 3 (first build of `tests/downgrade_protection.rs`)
- **Issue:** `Result::expect_err` requires `T: Debug`. `AppCtx` cannot derive `Debug` because internal types (`WriterHandle` containing `mpsc::Sender<Box<dyn FnOnce...>>`; `ReaderPool` containing `Mutex<Vec<rusqlite::Connection>>`) are not `Debug`. Deriving `Debug` on `AppCtx` would require manually-implementing `Debug` for half the chain or using `#[debug(skip)]` — premature.
- **Fix:** Replaced `result.expect_err(...)` with `match result { Ok(_) => panic!(...), Err(e) => e }`. Same semantics, no `T: Debug` requirement.
- **Files modified:** `crates/trackly-app/tests/downgrade_protection.rs`
- **Verification:** `cargo test -p trackly-app --test downgrade_protection` passes.
- **Committed in:** `ade8f0c`

**5. [Rule 1 - Lint] rustfmt re-flowed multi-line assertions and closure formatting across Task 1/2/3 files**

- **Found during:** Final `cargo fmt --all -- --check` after Task 3
- **Issue:** Several files (writer_worker.rs, test_app_ctx.rs, clock_impl.rs, secret.rs, secret_zeroize.rs, main.rs) had collapsible single-line/multi-line tradeoffs that rustfmt 1.88 normalised differently from how I wrote them.
- **Fix:** Ran `cargo fmt --all` — purely cosmetic changes, no semantic impact. Verified `cargo test --workspace` still green after fmt.
- **Files modified:** 6 files (all minor reformatting)
- **Verification:** `cargo fmt --all -- --check` clean.
- **Committed in:** Folded into `ade8f0c` (since fmt sweep happened during Task 3).

---

**Total deviations:** 5 auto-fixed (2× Rule 3 Blocking, 1× Rule 2 Critical, 1× Rule 1 Bug, 1× Rule 1 Lint). No architectural changes. No checkpoints surfaced to the user.

## Backpressure Test Approach

`tests::backpressure_returns_write_queue_busy_when_channel_saturated` uses:
- `spawn_with_capacity(conn, 1)` (capacity = 1 instead of 256 default).
- `.with_send_timeout(Duration::from_millis(100))` override (instead of 5s default) so the test runs in <1s.
- Job-1: sleeps 500ms inside `spawn_blocking` to lock the worker.
- Job-2: queued (capacity = 1, fits).
- Job-3: channel full → `send_timeout` 100ms fires → asserts `Err(WriteQueueBusy)`.

This pattern is reusable for any future backpressure tests in Phase 7 (rate-limiting, etc.).

## Probe-Read Pattern Confirmation

The probe-read pattern landed exactly as specified in `<interfaces>`:

```rust
let probe = Connection::open_with_flags(
    &db_path,
    OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
)?;
let on_disk: u32 = probe.pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))? as u32;
drop(probe);   // explicit drop BEFORE writer open
if on_disk > max_known_version() {
    return Err(AppError::DatabaseFromNewerVersion { binary, file: on_disk }.into());
}
let mut writer_conn = Connection::open(&db_path)?;
```

`downgrade_protection.rs` asserts SHA256 byte-equality of `.db` + `.db-wal` via a single `String == String` comparison — no size/header-prefix fallback was needed because the probe-read open is provably non-mutating. Test passes consistently.

## ReaderPool Implementation Note

Kept as simple `std::sync::Mutex<Vec<Connection>>` LIFO pool (NOT `deadpool`). Phase 1 LAN scale (4 readers, ~20 typical concurrent users) doesn't exhaust. Phase 2+ can swap to `deadpool` for queue-on-exhaustion without changing the public surface (`acquire() -> ReaderHandle`).

## Issues Encountered

- `tracing_appender::non_blocking(std::io::sink())` in tests produces no warnings; `non_blocking(std::io::stderr())` in main.rs is the placeholder per plan and Plan 05 will replace with a real `tracing_subscriber` chain.

## User Setup Required

None.

## Next Phase Readiness

**Ready for Plan 05** (Tauri specta export + axum):

- `AppCtx` is the parameter shape for the `health` Tauri command (Plan 05) and the future axum router state (Phase 5+).
- `error_axum.rs` stub is the splice-point for `axum::IntoResponse for AppError` (Plan 05 task).
- `main.rs` Step 5 (`tracing-appender placeholder`) is the splice-point for `tracing_subscriber::fmt().with_writer(non_blocking)` (Plan 05 task).
- `WriterHandle::execute(closure)` is the canonical write path — `health` command (and every future repo) routes through it.
- `ReaderPool::acquire()` (sync) wrapped in `spawn_blocking` is the canonical read path.

**Carry-forward notes for downstream plans:**

- Use `.map_err(map_rusqlite)?` / `.map_err(map_refinery)?` at every I/O callsite — these are the canonical error conversions; `?` won't auto-work because we deliberately did NOT impl `From`.
- `AppCtx` is `Clone`; pass it by value into Tauri commands (`#[tauri::command] async fn health(ctx: tauri::State<'_, AppCtx>) -> ...`).
- `AppCtx::build` returns `anyhow::Result<Self>`. Downstream calls that want to match on `AppError` should `result.map_err(|e| e.downcast::<AppError>().expect("AppError"))` — pattern used in downgrade_protection.rs.
- `Secret<T>` requires `T: Zeroize + Clone`. For passwords use `Secret<String>`; for byte arrays use `Secret<Vec<u8>>`. DO NOT impl `Serialize`/`Deserialize` on Secret — there's a compile-time gate.
- `Clock` trait is `Send + Sync`; production is `Arc::new(SystemClock)`. Tests can substitute via `Arc::new(MyFixedClock)`.
- `tauri-plugin-single-instance` (already in Cargo.toml from Plan 01-01) needs to be wired in `main.rs` BEFORE `AppCtx::build` in Plan 05/Phase 2 (when Tauri Builder is added) — otherwise a second `trackly.exe` instance racing on the same DB file would defeat the single-writer invariant. Note for Plan 05's task list.

**No blockers** for Plan 05.

## Threat Flags

None — threat surface within `<threat_model>` scope. The probe-read pattern mitigates T-04-01 (rejected newer DB leaves file untouched); the bounded mpsc(256) + 5s send_timeout mitigates T-04-02 (DoS via flood); `Secret<T>` + `assert_not_impl_all` mitigates T-04-03 (Debug/Serialize leak); panic-then-Internal mitigates T-04-04 (worker panic); panic on pool exhaustion documented as T-04-05 accept.

## Self-Check: PASSED

Verified after writing SUMMARY:

- `crates/trackly-core/src/error.rs` declares all 9 variants (grep for `NotFound`, `Conflict`, `OptimisticLockMismatch`, `WriteQueueBusy`, `DatabaseFromNewerVersion`, `Validation`, `Unauthorized`, `Forbidden`, `Internal`).
- `crates/trackly-core/src/primitives/secret.rs` does NOT contain `#[derive(...Serialize...)]` on `Secret`.
- `crates/trackly-core/src/primitives/secret.rs` `impl Drop` calls `.zeroize()`.
- `crates/trackly-infra/src/clock_impl.rs` uses `time::OffsetDateTime::now_utc()` (no chrono).
- `crates/trackly-infra/src/db/writer_worker.rs` contains `mpsc::channel::<BoxedJob>(capacity)` and `send_timeout(self.send_timeout)` and DEFAULT_WRITER_CAPACITY=256 + DEFAULT_SEND_TIMEOUT=5s.
- `crates/trackly-infra/src/db/writer_worker.rs` does NOT contain `mpsc::unbounded_channel`.
- `crates/trackly-infra/src/db/pools.rs` contains `OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX`.
- `crates/trackly-app/src/main.rs` calls `set_webview2_data_folder` as Step 2 of main() (before tokio runtime).
- `crates/trackly-app/src/context.rs` opens `Connection::open_with_flags(..., OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI)` BEFORE any `Connection::open` (writer) call and returns `AppError::DatabaseFromNewerVersion` before `migrations::run` is invoked.
- `crates/trackly-app/tests/downgrade_protection.rs` hashes both `.db` and `.db-wal` (if exists) into a single SHA256 digest with single `String == String` equality assertion.
- `crates/trackly-app/Cargo.toml` `[dev-dependencies]` includes `sha2 = "0.10"` and `tracing-appender = { workspace = true }`.
- All 3 task commits present in `git log --oneline`: `5a95270`, `2a135ec`, `ade8f0c`.
- `cargo test --workspace` passes.
- `cargo clippy --workspace --all-targets -- -D warnings` passes.
- `cargo fmt --all -- --check` passes.
- `cargo run -p trackly-app -- --self-test` exits 0 with `self-test OK: schema_version=12, portable=false` (or `true` if sentinel adjacent).
- `cargo test -p trackly-core --test no_io_deps` still passes (FOUND-01 invariant from Plan 01 still holds — trackly-core has zero I/O deps).

---

*Phase: 01-foundation*
*Completed: 2026-05-25*

---
phase: 05-auth-server-mode
plan: "02"
subsystem: auth
tags: [argon2id, tower-sessions, rustls, rcgen, tokio-rustls, sqlite, cancellation-token, tls, session-store]

# Dependency graph
requires:
  - phase: 05-01
    provides: "auth DTOs (LoginRequest, UserDto, AuthStatusDto), V018 migration (users+app_settings tables), trackly-core::auth::authorize(), AppCtx scaffold, 8 RED test scaffolds"
provides:
  - "AuthService: full user lifecycle (needs_bootstrap, login, create_user, update_user, delete_user, list_users, change_password, reset_password)"
  - "AuthService: argon2id hash/verify free functions (OWASP 2024 params m=19456/t=2/p=1)"
  - "AuthService: desktop_identity() LIMIT-2 query (D-Desktop-01 exactly-one-admin rule)"
  - "AuthService: get_desktop_lock_enabled/set_desktop_lock_enabled via app_settings (D-Desktop-02)"
  - "RusqliteSessionStore: tower-sessions 0.15 SessionStore backed by V010 sessions table; sessions survive restart"
  - "server/tls.rs: generate_self_signed + load_from_pem + SHA-256 fingerprint (XX:XX 95-char format)"
  - "server/mod.rs: start_server(TcpListener) accept-loop + CancellationToken shutdown, start_server_on_addr convenience wrapper"
  - "AppCtx: auth: Arc<AuthService> + server_ctl: Arc<Mutex<Option<ServerHandle>>> fields"
  - "V019 migration: users.is_active column"
  - "21 GREEN integration tests for auth, session, TLS, server lifecycle"
affects: [05-03, 05-04, 05-05]

# Tech tracking
tech-stack:
  added:
    - "argon2 0.5 (workspace) — argon2id password hashing, OWASP 2024 params"
    - "tower-sessions 0.15 (workspace) — SessionStore trait for custom SQLite store"
    - "tokio-rustls 0.26 (workspace) — TLS acceptor for axum server"
    - "rcgen 0.14 (workspace) — self-signed cert generation"
    - "rustls 0.23 (direct in trackly-app) — ServerConfig builder"
    - "rustls-pemfile 2 (workspace) — PEM parsing for load_from_pem"
    - "rmp-serde 1 (workspace) — MessagePack session data serialization"
    - "sha2 (workspace) — SHA-256 fingerprint computation"
    - "async-trait (workspace) — SessionStore impl macro"
    - "secrecy (workspace) — Secret<String> for password parameters"
  patterns:
    - "hash_password / verify_password free functions in spawn_blocking (T-05-03 pattern)"
    - "RusqliteSessionStore: writes via writer.execute(), reads via spawn_blocking+readers.acquire()"
    - "start_server(TcpListener) — caller pre-binds for testability; start_server_on_addr for prod"
    - "CancellationToken child pattern for hot start/stop: biased select! checks shutdown first"
    - "Session ID serialization: record.id.0.to_le_bytes().to_vec() as BLOB"
    - "desktop_identity() LIMIT 2 exact-1-admin guard (D-Desktop-01)"
    - "optimistic locking via version column in user UPDATE"
    - "soft-delete via deleted_at_utc in users table"

key-files:
  created:
    - "crates/trackly-app/src/services/auth.rs"
    - "crates/trackly-app/src/server/mod.rs"
    - "crates/trackly-app/src/server/tls.rs"
    - "crates/trackly-app/src/server/rusqlite_session_store.rs"
    - "migrations/V019__users_is_active.sql"
  modified:
    - "crates/trackly-app/src/context.rs"
    - "crates/trackly-app/src/lib.rs"
    - "crates/trackly-app/src/services/mod.rs"
    - "crates/trackly-app/Cargo.toml"
    - "crates/trackly-app/tests/auth_smoke.rs"
    - "crates/trackly-app/tests/users_crud.rs"
    - "crates/trackly-app/tests/session_survives_restart.rs"
    - "crates/trackly-app/tests/tls_server_smoke.rs"
    - "crates/trackly-app/tests/server_hot_toggle.rs"
    - "crates/trackly-app/tests/graceful_shutdown_drain.rs"

key-decisions:
  - "start_server accepts TcpListener (not SocketAddr) — caller pre-binds for precise port control in tests; start_server_on_addr added as convenience wrapper"
  - "hash_password/verify_password run in spawn_blocking — argon2id is CPU-intensive, must not block tokio reactor"
  - "RusqliteSessionStore manual Debug impl — WriterHandle/ReaderPool don't derive Debug, but SessionStore requires Debug bound"
  - "Session expiry filter in load() SQL query (WHERE expiry_date > NOW) — expired sessions silently return None without cleanup"
  - "background_cleanup() exists but is called once at server start, not as continuous background task (LAN scale: adequate)"
  - "biased select! with shutdown checked first — ensures graceful exit even if new connections arrive simultaneously"

patterns-established:
  - "ArgonHasher pattern: hash_password(secret) + verify_password(secret, hash) as free functions in spawn_blocking"
  - "SessionStore on SQLite: INSERT OR IGNORE for create, INSERT OR REPLACE for save, WHERE expiry_date > NOW in load"
  - "Server lifecycle: CancellationToken child token per ServerHandle instance, never cancel master AppCtx.shutdown"
  - "TLS bundle: generate once, store cert_pem/key_pem for disk persistence, fingerprint_hex for UI display"

requirements-completed:
  - USR-01
  - USR-03
  - USR-04
  - USR-05
  - USR-07
  - SRV-01
  - SRV-03
  - SRV-04
  - SRV-05

# Metrics
duration: 95min
completed: 2026-06-13
---

# Phase 05 Plan 02: Auth Implementation + Server Lifecycle Summary

**AuthService (argon2id, user CRUD, desktop identity) + RusqliteSessionStore + rustls/rcgen TLS bundle + CancellationToken server lifecycle, 21 GREEN integration tests**

## Performance

- **Duration:** ~95 min (across 2 sessions)
- **Started:** 2026-06-13T07:30:00Z
- **Completed:** 2026-06-13T10:38:00Z
- **Tasks:** 2
- **Files modified:** 15

## Accomplishments

- Full `AuthService` with argon2id hashing (OWASP 2024 m=19456/t=2/p=1), user CRUD with optimistic locking, soft-delete, needs_bootstrap, desktop_identity LIMIT-2 rule, get/set_desktop_lock_enabled
- `RusqliteSessionStore` as tower-sessions 0.15 custom `SessionStore` impl: sessions survive store recreation (D-Session-01), expired sessions filtered in SQL, writes via WriterHandle, reads via spawn_blocking+ReaderPool
- `server/tls.rs` with `generate_self_signed` + `load_from_pem` + SHA-256 fingerprint (95-char XX:XX format, D-TLS-01)
- `server/mod.rs` with `start_server(TcpListener)` accept-loop, `biased select!` for priority shutdown, and `start_server_on_addr` convenience wrapper
- `AppCtx` extended with `auth: Arc<AuthService>` and `server_ctl: Arc<Mutex<Option<ServerHandle>>>`
- V019 migration adding `users.is_active` column

## Task Commits

1. **Task 1: AuthService with argon2id, user CRUD, desktop identity** - `b80f731` (feat)
2. **Task 2: RusqliteSessionStore + TLS + server lifecycle + AppCtx wiring** - `2518f0b` (feat)
3. **Rule 1/3 fixes: test fixtures + hardcoded schema version** - `5f39cb8` (fix)

## Files Created/Modified

- `crates/trackly-app/src/services/auth.rs` - AuthService with 11 public methods + 2 free functions
- `crates/trackly-app/src/server/mod.rs` - start_server, start_server_on_addr, ServerHandle
- `crates/trackly-app/src/server/tls.rs` - TlsBundle, generate_self_signed, load_from_pem
- `crates/trackly-app/src/server/rusqlite_session_store.rs` - RusqliteSessionStore (SessionStore impl)
- `migrations/V019__users_is_active.sql` - ADD COLUMN is_active INTEGER NOT NULL DEFAULT 1
- `crates/trackly-app/src/context.rs` - Added auth, server_ctl fields to AppCtx
- `crates/trackly-app/src/lib.rs` - Added pub mod server
- `crates/trackly-app/src/services/mod.rs` - pub mod auth, pub use auth::AuthService
- `crates/trackly-app/Cargo.toml` - Added async-trait, rustls direct deps
- `tests/auth_smoke.rs` - 6 GREEN tests (bootstrap, login, desktop_identity, lock_enabled, hash, rbac)
- `tests/users_crud.rs` - 4 GREEN tests (CRUD, password validation, role enforcement, search)
- `tests/session_survives_restart.rs` - 4 GREEN tests (persist, delete, expired, save_update)
- `tests/tls_server_smoke.rs` - 3 GREEN tests (fingerprint format, PEM round-trip, TCP accept)
- `tests/server_hot_toggle.rs` - 2 GREEN tests (starts_stops_freed, hot_toggle)
- `tests/graceful_shutdown_drain.rs` - 2 GREEN tests (exits_within_timeout, pre-cancel noop)

## Decisions Made

- `start_server(TcpListener)` over `start_server(SocketAddr)` — caller pre-binds to get random port for tests; more flexible API
- Manual `Debug` impl for `RusqliteSessionStore` — WriterHandle/ReaderPool don't derive Debug but tower-sessions requires `Debug + Send + Sync`
- `biased select!` with shutdown as first branch — prevents starvation of shutdown signal when connections arrive rapidly
- Session expiry check in SQL (`WHERE expiry_date > ?`) — avoids loading expired data into memory
- `background_cleanup()` as on-demand function (not timer task) — sufficient for LAN scale; called once at server start

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed hardcoded schema_version 17 in 3 test files**
- **Found during:** Task 2 verification (full test suite run)
- **Issue:** `downgrade_protection.rs`, `health_smoke.rs` used `assert_eq!(..., 17)` — failed after V019 migration bumped version to 19
- **Fix:** Replaced with dynamic `migrations::max_known_version()` call
- **Files modified:** `tests/downgrade_protection.rs`, `tests/health_smoke.rs`
- **Verification:** Both tests pass with `schema_version = 19`
- **Committed in:** `5f39cb8`

**2. [Rule 3 - Blocking] Added auth/server_ctl fields to AppCtx test fixtures**
- **Found during:** Task 2 verification (full test suite run)
- **Issue:** AppCtx struct literals in `http/health.rs`, `tauri_cmds/health.rs`, `tests/specta_roundtrip.rs` were missing newly added `auth` and `server_ctl` fields — compilation error
- **Fix:** Added `auth: Arc::new(AuthService::new(...))` and `server_ctl: Arc::new(Mutex::new(None))` to all three locations
- **Files modified:** `src/http/health.rs`, `src/tauri_cmds/health.rs`, `tests/specta_roundtrip.rs`
- **Verification:** All test files compile and 56 unit tests pass
- **Committed in:** `5f39cb8`

**3. [Rule 3 - Improvement] Refactored start_server to accept TcpListener**
- **Found during:** Task 2 (writing tls_server_smoke tests)
- **Issue:** Original `start_server(SocketAddr)` signature made it impossible to get the random-assigned port in tests before calling the function
- **Fix:** Changed to `start_server(TcpListener)` — caller pre-binds and gets `local_addr()`; added `start_server_on_addr(SocketAddr)` as convenience wrapper for production use
- **Files modified:** `src/server/mod.rs`
- **Verification:** All 9 server/TLS tests pass; no downstream breakage
- **Committed in:** `2518f0b`

---

**Total deviations:** 3 auto-fixed (1 bug fix, 2 blocking/improvement)
**Impact on plan:** All fixes necessary for correctness and testability. No scope creep.

## Issues Encountered

- `trackly-infra::db::migrations.rs` had hardcoded version check (`assert_eq!(max_known_version(), 17)`) that was already fixed in Task 1 commit; downstream test files had same pattern and were caught during full suite run
- tower-sessions 0.15 `SessionStore` trait requires `Debug + Send + Sync` bounds — `WriterHandle` and `ReaderPool` don't derive `Debug`, required manual `impl Debug`

## Known Stubs

None — all methods are fully implemented. No `todo!()` or placeholder values exist in the new code.

The pre-existing RED scaffold `tests/role_endpoint_matrix.rs::employee_cannot_mutate_devices` still has `todo!()` as per Plan 01 design; this test is explicitly marked "Will be GREEN after Plan 02+03" and is outside Plan 02 scope.

## Next Phase Readiness

- Plan 03 can implement the full axum router with session middleware using `RusqliteSessionStore` — all building blocks ready
- `AuthService` is fully wired into `AppCtx` — Tauri commands for login/logout/user management can be added in Plan 03
- `start_server` is ready to be called from `AppCtx::build` or a Tauri command for server toggle
- `ServerHandle` in `AppCtx.server_ctl` provides the hot start/stop mechanism Plan 03 will use

---
*Phase: 05-auth-server-mode*
*Completed: 2026-06-13*

## Self-Check: PASSED

Files verified:
- `crates/trackly-app/src/services/auth.rs` — FOUND
- `crates/trackly-app/src/server/mod.rs` — FOUND
- `crates/trackly-app/src/server/tls.rs` — FOUND
- `crates/trackly-app/src/server/rusqlite_session_store.rs` — FOUND
- `migrations/V019__users_is_active.sql` — FOUND

Commits verified:
- `b80f731` — FOUND (feat: AuthService)
- `2518f0b` — FOUND (feat: server modules + AppCtx wiring)
- `5f39cb8` — FOUND (fix: test fixtures)

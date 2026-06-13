---
phase: 05-auth-server-mode
plan: 01
subsystem: auth-domain
tags: [auth, tdd, domain, dto, migration, scaffolding]
dependency_graph:
  requires: []
  provides:
    - trackly-core::auth (Identity, Role, Action, authorize)
    - crates/trackly-app/src/dto/auth.rs (10 auth/user DTO types)
    - migrations/V018__auth_settings.sql (desktop_lock_enabled seed)
  affects:
    - crates/trackly-core (new auth module)
    - crates/trackly-app (new dto/auth module, new workspace deps)
    - migrations (V018 added)
tech_stack:
  added:
    - argon2 = "0.5" (workspace dep — Phase 5 password hashing)
    - tower-sessions = "0.15" (workspace dep — session middleware)
    - tokio-rustls = "0.26" (workspace dep — TLS for axum server)
    - rcgen = "0.14" (workspace dep — self-signed cert generation)
    - rustls-pemfile = "2" (workspace dep — PEM parsing)
    - tower_governor = "0.8" (workspace dep — rate limiting)
    - rmp-serde = "1" (workspace dep — MessagePack session serialization)
    - sha2 = "0.10" (moved from dev-deps to regular deps)
    - hyper = "1" (workspace dep — low-level HTTP)
    - hyper-util = "0.1" (workspace dep — tokio integration)
  patterns:
    - Role enum with from_str()/as_str() + AppError::Validation on unknown
    - Identity with trusted_admin() constructor (D-Desktop-01 unlocked mode)
    - authorize() pure function with no I/O — hexagonal core invariant maintained
    - DTO pattern: snake_case, #[specta(type = i32)] on i64 fields, no password_hash
key_files:
  created:
    - crates/trackly-core/src/auth.rs
    - crates/trackly-app/src/dto/auth.rs
    - migrations/V018__auth_settings.sql
    - crates/trackly-app/tests/auth_smoke.rs
    - crates/trackly-app/tests/users_crud.rs
    - crates/trackly-app/tests/role_endpoint_matrix.rs
    - crates/trackly-app/tests/session_survives_restart.rs
    - crates/trackly-app/tests/tls_server_smoke.rs
    - crates/trackly-app/tests/server_hot_toggle.rs
    - crates/trackly-app/tests/security_headers.rs
    - crates/trackly-app/tests/graceful_shutdown_drain.rs
  modified:
    - crates/trackly-core/src/lib.rs (pub mod auth; pub use auth::{...})
    - crates/trackly-app/src/dto/mod.rs (pub mod auth)
    - Cargo.toml (10 new workspace deps; tower-http set-header feature)
    - crates/trackly-app/Cargo.toml (10 new deps; sha2 moved from dev to regular)
decisions:
  - "Role enum uses as_str()/from_str() over Display/FromStr traits — simpler, consistent with existing error.rs pattern"
  - "authorize() is a free function, not an impl method — makes call sites explicit (T-05-01 mitigation)"
  - "desktop_lock_enabled in V018 uses INSERT OR IGNORE with both created_at_utc and updated_at_utc — matches V016 app_settings pattern"
  - "tower-sessions pinned to 0.15 as specified in PLAN (vs 0.13 in RESEARCH — plan takes precedence)"
metrics:
  duration: "8 min"
  completed_date: "2026-06-13"
  tasks_completed: 3
  files_created: 11
  files_modified: 4
---

# Phase 05 Plan 01: Auth Domain Contracts, DTOs, Migration, RED Test Scaffolds Summary

Auth domain contracts (Identity/Role/Action/authorize), 10 DTO types, V018 migration with desktop_lock_enabled, and 8 RED integration test stubs — establishes all type contracts Phase 5 Plans 02-05 depend on.

## What Was Built

### Task 1: trackly-core::auth (RED→GREEN)

Created `crates/trackly-core/src/auth.rs` with pure domain types and logic:

- `Role` enum (Admin/Manager/Employee) with `from_str()` → `AppError::Validation` on unknown, `as_str()` roundtrip
- `Identity` struct with `trusted_admin()` constructor (D-Desktop-01 unlocked mode: `user_id: None, role: Admin`)
- `Action` enum covering all 7 permission categories
- `authorize()` pure function implementing the 3-tier permission matrix
- 12 inline unit tests (11 behavior cases from plan + 1 as_str roundtrip)
- Zero I/O dependencies — `no_io_deps` gate confirmed green

Updated `lib.rs` to export `pub mod auth` and re-export `Action, Identity, Role, authorize`.

### Task 2: Auth DTOs + V018 migration + workspace dependencies

Created `crates/trackly-app/src/dto/auth.rs` with 10 DTO types:
- `LoginRequest`, `UserDto` (no `password_hash` — T-05-02 mitigation), `UserNew`, `UserPatch`
- `ChangePasswordRequest`, `AuthStatusDto` (with `desktop_lock_enabled: bool` — D-Desktop-02)
- `NetworkSettingsDto` (with `desktop_lock_enabled: bool`), `ServerStatusDto`
- `UserFilter`, `UserListResponse`

All types: `Debug, Clone, Serialize, Deserialize, Type`; snake_case JSON; `i64` fields annotated `#[specta(type = i32)]`.

Created `migrations/V018__auth_settings.sql`:
- `INSERT OR IGNORE INTO app_settings ('desktop_lock_enabled', '0', ...)` — idempotent seed
- `PRAGMA user_version = 18`

Added 10 workspace dependencies to `Cargo.toml` (argon2, tower-sessions, tokio-rustls, rcgen, rustls-pemfile, tower_governor, rmp-serde, sha2, hyper, hyper-util).
Added `"set-header"` to tower-http features.
Moved sha2 from `trackly-app` dev-deps to regular deps.

### Task 3: 8 RED integration test scaffolds

Created 8 test files in `crates/trackly-app/tests/`:
- Each uses `#[allow(dead_code, unused_imports)]`
- Each has `#[tokio::test(flavor = "multi_thread", worker_threads = 4)]`
- Each wraps body in `tokio::time::timeout(Duration::from_secs(30), ...)`
- Each panics with `todo!("RED: ... not yet implemented — Plan 0N fills this")`

All 8 compile and fail with `todo!()` panic — not compile errors. Existing 13+ green tests remain green.

## Verification Results

1. `cargo test -p trackly-core --lib auth::tests` — 12 passed, 0 failed
2. `cargo test -p trackly-core --test no_io_deps` — 1 passed (gate green)
3. `cargo build -p trackly-app` — `Finished dev profile` (clean)
4. `grep -c "desktop_lock_enabled" migrations/V018__auth_settings.sql` — 2
5. `grep -c "desktop_lock_enabled" crates/trackly-app/src/dto/auth.rs` — 10 (AuthStatusDto + NetworkSettingsDto + comments/tests)
6. `grep -rn "password_hash" crates/trackly-app/src/dto/auth.rs` — only in comments and test assertions, never in struct fields
7. Each of 8 RED tests: `FAILED. 0 passed; 1 failed` (panicked at todo!)

## Deviations from Plan

### Auto-noted adjustments

**1. [Minor] V018 migration uses both created_at_utc and updated_at_utc columns**
- **Found during:** Task 2
- **Issue:** Plan showed only `created_at_utc, updated_at_utc` in VALUES but the `app_settings` DDL (V016) has both columns as NOT NULL
- **Fix:** Added both `created_at_utc` and `updated_at_utc` to INSERT to match the V016 schema
- **Files modified:** `migrations/V018__auth_settings.sql`

**2. [Minor] tower-sessions version: 0.15 (plan) vs 0.13 (RESEARCH)**
- Plan explicitly specifies `tower-sessions = "0.15"` — followed the plan as authoritative

None — plan executed as written otherwise.

## Threat Surface Scan

No new network endpoints, auth paths, or schema trust boundaries introduced in this plan. V018 only adds an `app_settings` row (no new tables). Auth types are pure domain/DTO with no I/O.

## Known Stubs

None — this plan establishes type contracts and RED test scaffolds only. No UI or data-flow wiring.

## Self-Check

- [x] `crates/trackly-core/src/auth.rs` exists
- [x] `crates/trackly-app/src/dto/auth.rs` exists
- [x] `migrations/V018__auth_settings.sql` exists
- [x] All 8 RED test files exist
- [x] 3 task commits present (88012c1, e474b51, 2ac5e76)
- [x] no_io_deps gate green
- [x] cargo build succeeds

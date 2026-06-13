---
phase: 05-auth-server-mode
plan: "03"
subsystem: auth-http
tags: [axum, tower-sessions, tower-governor, session-fixation, security-headers, tauri-commands, specta, rate-limit]

# Dependency graph
requires:
  - phase: 05-02
    provides: "AuthService, RusqliteSessionStore, TlsBundle, ServerHandle, AppCtx.auth + server_ctl"
provides:
  - "http/auth.rs: public_router (login+status) + protected_router (logout+me); session fixation prevention T-05-SF"
  - "http/users.rs: POST /api/v1/users_* CRUD via session identity"
  - "http/settings.rs: server_toggle, server_status, desktop_set_lock, settings_get_network"
  - "http/mod.rs: build_router() with SessionManagerLayer + security headers + GovernorLayer rate limit"
  - "tauri_cmds/auth.rs: auth_login, auth_logout, auth_status, auth_me, server_toggle, server_status, desktop_set_lock"
  - "tauri_cmds/users.rs: users_list/create/update/delete/change_password via resolve_tauri_identity() (D-Desktop-01/02)"
  - "specta_export.rs: 14 Phase 5 commands registered"
  - "main.rs: axum HTTPS server auto-start when config.server.enabled"
  - "tests/security_headers.rs: 2 GREEN tests"
affects: [05-04, 05-05]

# Tech tracking
tech-stack:
  added:
    - "tower-http fs feature — ServeDir for Svelte SPA fallback"
    - "SessionIdentity DTO (local) — serde wrapper for Identity in tower-sessions store"
  patterns:
    - "build_router() applies SessionManagerLayer globally — Session extractor requires layer on all routes"
    - "GovernorLayer via route_layer() on /auth_login only — not whole router (avoids 500 on unit tests)"
    - "session.flush() BEFORE session.insert() in build_auth_login — T-05-SF session fixation prevention"
    - "resolve_tauri_identity(): lock OFF → trusted_admin, lock ON → desktop_identity (D-Desktop-01/02)"
    - "SessionIdentity as serde-serializable session DTO (trackly-core Identity lacks Serialize/Deserialize)"

key-files:
  created:
    - "crates/trackly-app/src/http/auth.rs"
    - "crates/trackly-app/src/http/users.rs"
    - "crates/trackly-app/src/http/settings.rs"
    - "crates/trackly-app/src/tauri_cmds/auth.rs"
    - "crates/trackly-app/src/tauri_cmds/users.rs"
  modified:
    - "crates/trackly-app/src/http/mod.rs"
    - "crates/trackly-app/src/main.rs"
    - "crates/trackly-app/src/tauri_cmds/mod.rs"
    - "crates/trackly-app/src/specta_export.rs"
    - "crates/trackly-app/tests/security_headers.rs"
    - "Cargo.toml"

key-decisions:
  - "SessionManagerLayer applied to all routes (not just protected) — Session extractor requires the layer or returns 500"
  - "GovernorLayer via route_layer() on login route only — whole-router GovernorLayer caused 500 on routes without peer IP in unit tests"
  - "SessionIdentity local DTO for session storage — trackly-core Identity is pure domain without Serialize/Deserialize (no_io_deps invariant)"
  - "resolve_tauri_identity() as single identity resolution point in users Tauri commands (T-05-DL)"

# Metrics
duration: 24min
completed: 2026-06-13
---

# Phase 05 Plan 03: HTTP auth/users/settings routers + Tauri commands + security headers Summary

**HTTP auth/users/settings handlers + 14 Tauri commands + build_router() with session middleware + security headers + rate limit + axum server auto-start**

## Performance

- **Duration:** ~24 min
- **Started:** 2026-06-13T10:43:42Z
- **Completed:** 2026-06-13T11:08:00Z
- **Tasks:** 2
- **Files modified:** 11

## Accomplishments

- `http/auth.rs`: `public_router()` (login + status) + `protected_router()` (logout + me); session fixation prevention: `session.flush()` BEFORE `session.insert()` (T-05-SF)
- `http/users.rs`: POST /api/v1/users_* CRUD; identity extracted via `session_identity()` from session
- `http/settings.rs`: server_toggle, server_status, desktop_set_lock, settings_get_network
- `http/mod.rs`: `build_router()` with `SessionManagerLayer` (all routes), `GovernorLayer` rate limit via `route_layer()` on `/auth_login` only, security headers (`x-frame-options: DENY`, `x-content-type-options: nosniff`, `content-security-policy`), `ServeDir` Svelte SPA fallback
- `tauri_cmds/auth.rs`: auth_login, auth_logout, auth_status, auth_me, server_toggle, server_status, desktop_set_lock
- `tauri_cmds/users.rs`: users_list/create/update/delete/change_password via `resolve_tauri_identity()` (D-Desktop-01/02 — lock OFF → trusted_admin, lock ON → desktop_identity)
- `specta_export.rs`: 14 Phase 5 commands registered; export_bindings test GREEN
- `main.rs`: axum HTTPS server auto-start when `config.server.enabled`; TLS bundle generation; cert PEM saved to exe_dir; child CancellationToken per ServerHandle (D-Server-01)
- `tests/security_headers.rs`: 2 GREEN tests (headers present + rate limit active)

## Task Commits

1. **Task 1: Auth/users/settings HTTP + Tauri commands + specta** — `3f05e65` (feat)
2. **Task 2: build_router() + main.rs server start + security_headers GREEN** — `8184ce2` (feat)

## Files Created/Modified

- `crates/trackly-app/src/http/auth.rs` — auth handlers, SessionIdentity DTO, session fixation fix
- `crates/trackly-app/src/http/users.rs` — user CRUD HTTP handlers
- `crates/trackly-app/src/http/settings.rs` — server toggle + network settings HTTP handlers
- `crates/trackly-app/src/http/mod.rs` — build_router() with all middleware
- `crates/trackly-app/src/tauri_cmds/auth.rs` — auth Tauri commands
- `crates/trackly-app/src/tauri_cmds/users.rs` — users Tauri commands with resolve_tauri_identity
- `crates/trackly-app/src/tauri_cmds/mod.rs` — added pub mod auth, users
- `crates/trackly-app/src/specta_export.rs` — 14 Phase 5 commands registered
- `crates/trackly-app/src/main.rs` — server auto-start on boot
- `crates/trackly-app/tests/security_headers.rs` — 2 GREEN integration tests
- `Cargo.toml` — tower-http fs feature added

## Decisions Made

- **SessionManagerLayer on all routes**: Session extractor (`tower_sessions::Session`) returns 500 if `SessionManagerLayer` not in request stack. Applied to all routes; auth enforcement done in handlers via `session_identity()`.
- **GovernorLayer via `route_layer()`**: Applying GovernorLayer to whole Router caused 500 on all routes in unit tests (no real TCP peer IP). Using `route_layer()` on `/auth_login` route only — correctly targets rate limit without affecting other routes.
- **SessionIdentity local DTO**: `trackly-core::auth::Identity` is pure domain without `Serialize/Deserialize` (no_io_deps invariant). Created `SessionIdentity { user_id, role: String }` in `http/auth.rs` as serde wrapper for session storage.
- **resolve_tauri_identity()**: Single D-Desktop-02 implementation — all Tauri users commands resolve identity through this function. Prevents direct `trusted_admin` hardcoding in mutation handlers (T-05-DL mitigated).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] tower-http missing `fs` feature for ServeDir**
- **Found during:** Task 1 (cargo build)
- **Issue:** `tower_http::services::ServeDir` requires `fs` feature; Cargo.toml had only `trace`, `cors`, `set-header`
- **Fix:** Added `fs` to `tower-http` features in workspace `Cargo.toml`
- **Files modified:** `Cargo.toml`
- **Commit:** `3f05e65`

**2. [Rule 3 - Blocking] Session extractor returns 500 without SessionManagerLayer**
- **Found during:** Task 2 (security_headers test RED)
- **Issue:** `Session` extractor in `auth_status` handler returned 500 because `SessionManagerLayer` was only on protected routes, but `auth_status` is in public_router without session layer
- **Fix:** Applied `SessionManagerLayer` to all routes in `build_router()`; auth enforcement done in handlers
- **Files modified:** `crates/trackly-app/src/http/mod.rs`
- **Commit:** `8184ce2`

**3. [Rule 3 - Blocking] GovernorLayer caused 500 on all routes in unit tests**
- **Found during:** Task 2 (security_headers test RED, rate_limit_on_login)
- **Issue:** Applying `GovernorLayer` to whole Router caused 500 "Unable To Extract Key!" on all routes in Tower oneshot tests (no TCP peer IP available)
- **Fix:** Used `route_layer()` to apply GovernorLayer only to `/api/v1/auth_login` route specifically
- **Files modified:** `crates/trackly-app/src/http/mod.rs`
- **Commit:** `8184ce2`

## Known Stubs

None — all implemented handlers are functional. `settings_get_network` returns current config state (no server URL fingerprint tracking in server_ctl — fingerprint returned as None when checking running status). This is a minor limitation that does not affect plan goals.

## Threat Flags

No new security-relevant surface beyond plan's threat model. All T-05-* mitigations implemented as planned:
- T-05-SF: session fixation — `flush()` before `insert()` in login
- T-05-14: security headers — SetResponseHeaderLayer globally in build_router()
- T-05-10: rate limit — GovernorLayer burst=5/per_second=1 on /auth_login
- T-05-DL: desktop lock bypass — resolve_tauri_identity() in all users Tauri commands

## Self-Check: PASSED

Files verified:
- `crates/trackly-app/src/http/auth.rs` — FOUND
- `crates/trackly-app/src/http/users.rs` — FOUND
- `crates/trackly-app/src/http/settings.rs` — FOUND
- `crates/trackly-app/src/http/mod.rs` — FOUND (build_router exported)
- `crates/trackly-app/src/tauri_cmds/auth.rs` — FOUND
- `crates/trackly-app/src/tauri_cmds/users.rs` — FOUND

Commits verified:
- `3f05e65` — FOUND (feat: Task 1)
- `8184ce2` — FOUND (feat: Task 2)

Tests GREEN: security_headers (2/2), export_bindings (1/1)
Tests expected RED: role_endpoint_matrix (Plan 04 scope — unchanged)

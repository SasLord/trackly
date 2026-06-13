---
phase: 05-auth-server-mode
verified: 2026-06-13T16:45:00Z
status: gaps_found
score: 12/14
overrides_applied: 0
gaps:
  - truth: "settings_set_network Tauri command registered in specta_export (plan 03 must_have listed it as command #14 of 14)"
    status: failed
    reason: "settings_set_network Tauri command does not exist. The HTTP route POST /api/v1/settings_set_network is absent from the settings router (router() in http/settings.rs registers only get_network, server_toggle, server_status, desktop_set_lock). No tauri_cmds/settings.rs module exists. specta_export.rs registers 13 Phase 5 commands, not 14. The UI NetworkSettings.svelte calls apiCall('settings_set_network', {...}) in saveSettings(), which will fail at runtime in both Tauri and browser modes with 'command not found'. ROADMAP success criterion #2 requires configuring port and bind-address — the server toggle works but persisting port/bind-address changes does not."
    artifacts:
      - path: "crates/trackly-app/src/http/settings.rs"
        issue: "settings_set_network route missing from router(); present only as doc-comment TODO on line 5"
      - path: "crates/trackly-app/src/specta_export.rs"
        issue: "Only 13 Phase 5 commands registered; settings_set_network absent"
      - path: "ui/src/features/settings/NetworkSettings.svelte"
        issue: "saveSettings() calls apiCall('settings_set_network') at line 64 — will throw at runtime"
    missing:
      - "Create build_settings_set_network() helper in http/settings.rs and register POST /api/v1/settings_set_network route"
      - "Create Tauri command settings_set_network in tauri_cmds/ and register in specta_export.rs"

  - truth: "trackly-infra migration test suite fully green after V018/V019 addition"
    status: failed
    reason: "crates/trackly-infra/tests/migration_idempotency.rs still asserts applied_count==17 and schema_version==17 at lines 22-23. The cross-crate fix commit 04579bd only updated crates/trackly-infra/src/test_support/test_db.rs (user_version assertion bumped to 19) but missed the separate integration test file. cargo test -p trackly-infra fails with 'left: 19 right: 17'. This is a Phase 5 regression since V018 and V019 were added by this phase."
    artifacts:
      - path: "crates/trackly-infra/tests/migration_idempotency.rs"
        issue: "Lines 17, 22-23, 28, 43 assert applied_count==17 and schema_version==17; actual values are 19 since V018+V019"
    missing:
      - "Update migration_idempotency.rs assertions to expect 19 migrations / schema_version=19"

human_verification:
  - test: "BOOTSTRAP flow on fresh DB"
    expected: "App shows 'Добро пожаловать в Trackly' wizard (FirstRunWizard) on first launch with empty DB. Create admin (login: admin, full_name: Администратор, password: password123). App auto-logs in and navigates to main screen."
    why_human: "Requires running pnpm tauri dev against a fresh DB; cannot verify UI flow programmatically"

  - test: "Users CRUD page (admin)"
    expected: "Navigate to /users. Create second user with role 'Сотрудник'. Verify role label 'Сотрудник' displayed. Edit user — change full_name. Delete user."
    why_human: "Requires visual inspection of the Users CRUD UI and live Tauri invocation"

  - test: "Sidebar role filtering"
    expected: "When logged in as manager: 'Пользователи' and 'Настройки' sidebar items are NOT visible. When logged in as admin: both visible."
    why_human: "Requires login as different roles and visual sidebar inspection"

  - test: "Network Settings — server toggle with HTTPS URL and fingerprint"
    expected: "Settings page shows Network tab. Clicking 'Запустить сервер' starts HTTPS server, displays URL https://127.0.0.1:8443 and certificate fingerprint in XX:XX:XX... colon-hex format."
    why_human: "Requires visual confirmation that the fingerprint block appears and the URL is correct"

  - test: "Browser HTTPS access to LAN server"
    expected: "Navigate to https://127.0.0.1:8443 in Chrome/Firefox. Accept self-signed cert warning. Login page loads. Admin login succeeds. App loads in browser with correct sidebar."
    why_human: "Requires real browser interaction with self-signed certificate acceptance"

  - test: "Employee role access restrictions in browser"
    expected: "Create employee user in desktop. In browser, login as employee. Only allowed sections visible in sidebar. Attempting to navigate to /users shows empty or access-denied state."
    why_human: "Requires browser session with employee role and visual verification"

  - test: "Stop server — connection refused"
    expected: "Clicking 'Остановить сервер' in Settings makes browser access to https://127.0.0.1:8443 fail with connection refused within a few seconds."
    why_human: "Requires live server lifecycle observation in browser"

  - test: "D-Desktop-02: Desktop lock toggle end-to-end"
    expected: "In Settings → Сеть, the 'Требовать вход в десктопе' toggle is visible and enabled/disabled correctly. Enable it → restart desktop app → login screen shown on startup. Disable it → restart → app goes directly to main screen (no login)."
    why_human: "Requires app restart to observe boot-time behavior change driven by desktop_lock_enabled DB flag. This is the most critical manual test — D-Desktop-02 is a locked architectural decision."
---

# Phase 5: Авторизация, локальные пользователи и серверный режим — Verification Report

**Phase Goal:** Включить локальную аутентификацию (argon2id), три роли (Admin/Manager/Employee), HTTPS-сервер axum для доступа из браузера в LAN, единый authorize() для обоих транспортов (Tauri invoke + HTTP); десктоп остаётся unlocked-by-default с опциональным локом (D-Desktop-02).
**Verified:** 2026-06-13T16:45:00Z
**Status:** gaps_found
**Re-verification:** No — initial verification

## Context

Phase 5 spans 5 plans (01-05), a post-execution code review (05-REVIEW.md finding 5 critical + 7 warning issues), and a fix pass (05-REVIEW-FIX.md) that closed 10 findings (CR-01..CR-05, WR-01..WR-03, WR-05, WR-07) and deferred 2 (WR-04 performance, WR-06 bootstrap redesign). A cross-crate regression in trackly-infra was partially fixed. The 8-step manual desktop UAT in Plan 05-05 Task 3 was auto-approved under auto-mode and has NOT been manually executed — these appear as human_verification items below.

## Goal Achievement

### Observable Truths

| #  | Truth | Status | Evidence |
|----|-------|--------|----------|
| 1  | trackly-core::auth compiles with Identity/Role/Action/authorize() — zero I/O deps invariant passes | VERIFIED | `crates/trackly-core/src/auth.rs` exists; no_io_deps gate passes (1/1 test green); 33 unit tests pass |
| 2  | All 10 DTO types defined with correct specta types; AuthStatusDto has desktop_lock_enabled: bool (D-Desktop-02); UserDto has no password_hash field | VERIFIED | `crates/trackly-app/src/dto/auth.rs` — all 10 types present; grep for password_hash confirms it is comment-only, never in struct fields |
| 3  | V018 migration applies with desktop_lock_enabled seed at user_version=18; V019 adds users.is_active | VERIFIED | Both migration files exist; applied in integration tests (migration log shows V18 and V19 applied successfully) |
| 4  | AuthService implements argon2id (m=19456/t=2/p=1) hash/verify in spawn_blocking; needs_bootstrap; login; user CRUD; desktop_identity; get/set_desktop_lock_enabled | VERIFIED | `services/auth.rs` lines 42-49 confirm argon2id params; 6+4+4 tests GREEN in auth_smoke, users_crud, auth_smoke suites |
| 5  | RusqliteSessionStore create/save/load/delete work against sessions table; sessions survive store recreation; expired sessions filtered | VERIFIED | `server/rusqlite_session_store.rs` impl SessionStore with all 4 methods; session_survives_restart (4 tests GREEN) |
| 6  | TLS: generate_self_signed() produces TlsBundle with non-empty colon-hex fingerprint; start_server() exits on CancellationToken cancel | VERIFIED | `server/tls.rs` + `server/mod.rs` exist with expected signatures; tls_server_smoke (3 GREEN), server_hot_toggle (2 GREEN), graceful_shutdown_drain (2 GREEN) |
| 7  | AppCtx has auth: Arc<AuthService> and server_ctl: Arc<Mutex<Option<ServerHandle>>> fields | VERIFIED | `context.rs` lines 80, 84 — both fields present and initialized in build() at line 223 |
| 8  | POST /api/v1/auth_login calls session.flush() BEFORE session.insert() (session fixation prevention); POST /api/v1/auth_logout calls session.flush() | VERIFIED | `http/auth.rs` lines 105-106 (login flush), 127-128 (logout flush) |
| 9  | SessionManagerLayer applied to all routes; security headers x-frame-options: DENY + x-content-type-options: nosniff in all responses; rate limit on /auth_login (burst=5/per_second=1) | VERIFIED | `http/mod.rs` lines 44-67; security_headers (2 GREEN tests); WR-07 fix: script-src 'self' (no unsafe-inline) |
| 10 | POST /api/v1/devices_create/acts_create/cartridges_create with employee session → 403; manager session → 200; no session → 401 (role×endpoint matrix) | VERIFIED | role_endpoint_matrix (1 test with 9 assertions GREEN); authorize() present in devices.rs, acts.rs, cartridges.rs |
| 11 | Tauri devices/acts/cartridges mutation commands use resolve_tauri_identity() (D-Desktop-01/02); no hardcoded trusted_admin in mutation paths | VERIFIED | devices.rs, acts.rs, cartridges.rs tauri_cmds each import and call resolve_tauri_identity; desktop_set_lock (CR-01 fix) also uses it |
| 12 | Auth store (authStore), App.svelte bootstrap guard (D-Desktop-01/02 aware), LoginPage, FirstRunWizard, sidebar role filtering, UsersPage CRUD, NetworkSettings with desktop lock toggle — all exist with 0 svelte-check errors | VERIFIED | All UI files present; pnpm svelte-check: 0 errors, 30 warnings (pre-existing in Phase 4 code) |
| 13 | settings_set_network Tauri command registered in specta_export (plan 03 must_have listed it as command #14 of 14) | FAILED | Command absent from specta_export.rs; HTTP route absent from settings router; UI calls it but will fail at runtime |
| 14 | trackly-infra migration test suite fully green after V018/V019 addition | FAILED | migration_idempotency.rs still asserts applied_count==17; cargo test -p trackly-infra fails with left:19 right:17 |

**Score:** 12/14 truths verified

### Deferred Items

None. Both failed truths are actionable Phase 5 gaps, not items scheduled for a later phase.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/trackly-core/src/auth.rs` | Identity, Role, Action, authorize() — pure domain | VERIFIED | Present; no I/O deps |
| `crates/trackly-app/src/dto/auth.rs` | 10 DTO types with specta, no password_hash in UserDto | VERIFIED | Present; all 10 types confirmed |
| `migrations/V018__auth_settings.sql` | desktop_lock_enabled seed at user_version=18 | VERIFIED | Present |
| `migrations/V019__users_is_active.sql` | users.is_active column | VERIFIED | Present |
| `crates/trackly-app/src/services/auth.rs` | AuthService with 14 methods + 2 free functions | VERIFIED | Present; argon2id params m=19456/t=2/p=1 confirmed |
| `crates/trackly-app/src/server/rusqlite_session_store.rs` | SessionStore impl (create/save/load/delete) | VERIFIED | Present; WR-05 fix: corrupt decode returns Ok(None) |
| `crates/trackly-app/src/server/tls.rs` | TlsBundle, generate_self_signed, load_from_pem, resolve_key_path, load_from_files | VERIFIED | Present; WR-01 fix: explicit key_path validation |
| `crates/trackly-app/src/server/mod.rs` | start_server(TcpListener), ServerHandle, start_server_on_addr | VERIFIED | Present; biased select! for shutdown priority |
| `crates/trackly-app/src/context.rs` | AppCtx with auth + server_ctl fields | VERIFIED | Lines 80, 84 confirmed |
| `crates/trackly-app/src/http/auth.rs` | public_router(), protected_router(), session fixation fix | VERIFIED | Present; flush() before insert() at lines 105, 127 |
| `crates/trackly-app/src/http/users.rs` | users CRUD router; CR-02 fix (session-derived user_id); CR-03 fix (authorize on list) | VERIFIED | Present; ChangePasswordPayload has no user_id; build_users_list calls authorize(ManageUsers) |
| `crates/trackly-app/src/http/settings.rs` | server_toggle, server_status, settings_get_network, desktop_set_lock | VERIFIED (partial) | Present; fingerprint: None in get_network response (not persisted in server_ctl) is a minor stub; settings_set_network MISSING |
| `crates/trackly-app/src/http/mod.rs` | build_router() with SessionManagerLayer, security headers, rate limit | VERIFIED | Present; WR-07: script-src 'self' without unsafe-inline |
| `crates/trackly-app/src/tauri_cmds/auth.rs` | auth_login, auth_logout, auth_status, auth_me, server_toggle, server_status, desktop_set_lock | VERIFIED | Present; CR-01 fix: desktop_set_lock uses resolve_tauri_identity |
| `crates/trackly-app/src/tauri_cmds/users.rs` | users_list/create/update/delete/change_password via resolve_tauri_identity | VERIFIED | Present; CR-02 fix in Tauri path also |
| `crates/trackly-app/src/specta_export.rs` | 14 Phase 5 commands registered | FAILED | Only 13 commands registered; settings_set_network absent |
| `ui/src/lib/stores/auth.svelte.ts` | authStore Svelte 5 $state singleton | VERIFIED | Present |
| `ui/src/features/auth/LoginPage.svelte` | Login form with auth_login call | VERIFIED | Present |
| `ui/src/features/auth/FirstRunWizard.svelte` | First admin creation with admin role | VERIFIED | Present |
| `ui/src/features/users/UsersPage.svelte` | Users CRUD | VERIFIED | Present |
| `ui/src/features/settings/NetworkSettings.svelte` | Server toggle + desktop lock toggle (D-Desktop-02) | VERIFIED (partial) | Present; saveSettings() calls missing settings_set_network command |
| `ui/src/App.svelte` | Bootstrap guard: FirstRunWizard or LoginPage or Layout | VERIFIED | Present; desktop_lock_enabled-aware logic at lines 33-44 |
| `crates/trackly-infra/tests/migration_idempotency.rs` | Updated assertions for V018+V019 | FAILED | Still asserts 17; fails cargo test -p trackly-infra |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `http/auth.rs` | `services/auth.rs` | session.flush() before session.insert("identity") | VERIFIED | Lines 105-128 in http/auth.rs |
| `http/mod.rs` | `server/rusqlite_session_store.rs` | SessionManagerLayer::new(RusqliteSessionStore) | VERIFIED | Line 44 in http/mod.rs |
| `tauri_cmds/users.rs` | `tauri_cmds/users.rs::resolve_tauri_identity` | All mutation build_* call it | VERIFIED | devices/acts/cartridges tauri_cmds all import and call resolve_tauri_identity |
| `tauri_cmds/auth.rs::desktop_set_lock` | `resolve_tauri_identity` | CR-01 fix: not hardcoded | VERIFIED | Line 159 in tauri_cmds/auth.rs |
| `http/devices.rs` | `trackly_core::auth::authorize` | authorize(&identity, &Action::MutateDevices) | VERIFIED | 1 match; pattern present |
| `main.rs` | `server/mod.rs::start_server_on_addr` | if config.server.enabled | VERIFIED | Lines 17, 160 in main.rs |
| `App.svelte` | `authStore` | auth_status → bootstrap guard → isTauri && desktop_lock_enabled | VERIFIED | Lines 33-44 in App.svelte |
| `client.ts` | `authStore.user = null` | 401 response → clear + redirect #/login | VERIFIED | Lines 34-36 in client.ts |
| `NetworkSettings.svelte` | `settings_set_network` Tauri command | saveSettings() calls apiCall | NOT_WIRED | Command does not exist in specta_export or HTTP router |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|--------------|--------|-------------------|--------|
| `http/auth.rs:login handler` | UserDto | AuthService::login → argon2 verify → DB query | Yes — real DB row | FLOWING |
| `server/rusqlite_session_store.rs:load` | Record | SELECT FROM sessions WHERE expiry_date > NOW | Yes — real DB row | FLOWING |
| `App.svelte:onMount` | status (AuthStatusDto) | auth_status Tauri command → AuthService | Yes — DB-backed | FLOWING |
| `NetworkSettings.svelte:saveSettings` | void | settings_set_network Tauri command | No — command missing | DISCONNECTED |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| role_endpoint_matrix (9-case CI test) | `cargo test -p trackly-app --test role_endpoint_matrix` | 1 test, 9 assertions passed | PASS |
| Auth unit tests | `cargo test -p trackly-app --test auth_smoke` | 6 tests passed | PASS |
| Users CRUD tests | `cargo test -p trackly-app --test users_crud` | 4 tests passed | PASS |
| Security headers + rate limit | `cargo test -p trackly-app --test security_headers` | 2 tests passed | PASS |
| Session persistence across restart | `cargo test -p trackly-app --test session_survives_restart` | 4 tests passed | PASS |
| TLS fingerprint + server lifecycle | `cargo test -p trackly-app --test tls_server_smoke --test server_hot_toggle --test graceful_shutdown_drain` | 7 tests passed | PASS |
| trackly-infra migration idempotency | `cargo test -p trackly-infra --test migration_idempotency` | FAILED: left:19 right:17 | FAIL |
| svelte-check | `pnpm svelte-check` (in ui/) | 0 errors, 30 warnings (pre-existing in Phase 4 files) | PASS |

### Probe Execution

No probe scripts defined for Phase 5.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| USR-01 | 05-01, 05-02, 05-03 | CRUD пользователей с argon2id | SATISFIED | AuthService with 14 methods; users CRUD endpoint matrix GREEN |
| USR-02 | 05-03, 05-04 | Три роли: Admin/Manager/Employee | SATISFIED | authorize() enforced on all mutation paths; role_endpoint_matrix GREEN |
| USR-03 | 05-02, 05-03 | Cookie sessions via tower-sessions + rusqlite-store | SATISFIED | RusqliteSessionStore; session_survives_restart GREEN |
| USR-04 | 05-02, 05-03 | Десктоп unlocked-by-default; optional lock (D-Desktop-01/02) | SATISFIED | trusted_admin() + desktop_lock_enabled flag; App.svelte bootstrap guard; get/set_desktop_lock_enabled |
| USR-05 | 05-03 | Logout и login под другим пользователем в веб-режиме | SATISFIED | session.flush() in logout handler; /auth_logout route |
| USR-06 | 05-04 | Авторизация на API-слое (нельзя обойти роль через HTTP) | SATISFIED | authorize() in build_* helpers for both transports; role_endpoint_matrix CI gate |
| USR-07 | 05-02, 05-03 | HTTPS в server mode; rcgen self-signed; configurable cert path | SATISFIED | generate_self_signed + load_from_files in tls.rs; WR-01: explicit key_path |
| SRV-01 | 05-03 | Переключатель сервера в Настройки → Сеть | SATISFIED | server_toggle Tauri command + NetworkSettings.svelte |
| SRV-02 | 05-03 | CSRF-защита, security headers, rate limiting | SATISFIED | SameSite=Strict; x-frame-options/x-content-type-options; GovernorLayer burst=5; WR-07: no unsafe-inline |
| SRV-03 | 05-02, 05-04 | Tauri и axum используют ОДИН набор сервисов через AppCtx | SATISFIED | AppCtx.auth shared; build_* helpers called by both transports |
| SRV-04 | 05-02, 05-03 | HTTPS обязателен в server mode | SATISFIED | TLS via tokio-rustls; no plain HTTP listener in start_server |
| SRV-05 | 05-02 | Корректное завершение axum-сервера | SATISFIED | CancellationToken child_token; biased select!; graceful_shutdown_drain GREEN |
| SET-08 | 05-03, 05-05 | Настройки сетевого доступа: порт, bind-адрес, toggle | PARTIAL | Toggle (server_toggle) works; GET network settings works. PORT/BIND-ADDRESS SAVE broken: settings_set_network command missing |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `crates/trackly-app/src/http/settings.rs` | 5 | `// TODO: Phase 5+` (settings_set_network) | BLOCKER | Route listed as TODO in doc comment AND absent from router — UI calls missing endpoint |
| `crates/trackly-app/src/http/settings.rs` | 74 | `fingerprint: None // TODO: store fingerprint in server_ctl` | WARNING | fingerprint: None returned from settings_get_network (but IS returned correctly from server_toggle response) |
| `crates/trackly-infra/tests/migration_idempotency.rs` | 17, 22-23, 28, 43 | Hardcoded assertion `== 17` | BLOCKER | cargo test -p trackly-infra FAILS; Phase 5 migrations (V018, V019) were added by this phase |

### Human Verification Required

The following 8 items require manual desktop UAT (pnpm tauri dev). They were auto-approved under auto-mode during Plan 05-05 Task 3 (checkpoint:human-verify) and have NOT been executed by a human.

**Note: Item #8 (Desktop Lock) is the most security-critical manual test.** D-Desktop-02 (desktop lock toggle) is a locked architectural decision whose end-to-end behavior — DB-flag read at boot drives login screen appearance — can only be verified by restarting the application.

### 1. Bootstrap Flow

**Test:** Launch with fresh DB (`mv trackly.db trackly.db.bak` if exists). Run `pnpm tauri dev`.
**Expected:** 'Добро пожаловать в Trackly' wizard appears. Create admin (login: admin, full_name: Администратор, password: password123). App auto-logs in and navigates to main screen.
**Why human:** Requires running Tauri desktop app; UI flow cannot be verified programmatically.

### 2. Users CRUD (Admin)

**Test:** Navigate sidebar to 'Пользователи'. Create a second user with role 'Сотрудник'. Verify role label in table. Edit full_name. Delete the user.
**Expected:** All operations succeed; role label shows 'Сотрудник' (not raw 'employee').
**Why human:** Requires visual inspection of Users table and live Tauri invocations.

### 3. Sidebar Role Filtering

**Test:** Login as a manager-role user. Observe sidebar.
**Expected:** 'Пользователи' and 'Настройки' sidebar items NOT visible for manager. Re-login as admin — both visible.
**Why human:** Requires UI inspection with different role sessions.

### 4. Network Settings — Server Toggle

**Test:** Navigate Settings → (Network tab). Click 'Запустить сервер'.
**Expected:** After a few seconds, UI displays https://127.0.0.1:8443 URL and certificate fingerprint in XX:XX:XX:... colon-hex format. D-Server-04 instruction text visible.
**Why human:** Requires live server start and visual confirmation of fingerprint display.

### 5. Browser HTTPS Access

**Test:** While server is running, open Chrome/Firefox and navigate to https://127.0.0.1:8443. Accept self-signed certificate warning.
**Expected:** Login page loads in browser. Admin login with admin/password123 succeeds. App renders in browser with correct sidebar.
**Why human:** Requires real browser interaction and self-signed cert acceptance.

### 6. Employee Role in Browser

**Test:** From admin desktop, create an employee user. In browser, logout and login as employee.
**Expected:** Browser sidebar shows restricted sections. Employee cannot see 'Пользователи' or 'Настройки'.
**Why human:** Requires browser session with employee role and visual sidebar check.

### 7. Stop Server

**Test:** In desktop Settings, click 'Остановить сервер'. Then try to access https://127.0.0.1:8443 in browser.
**Expected:** Browser shows connection refused within a few seconds.
**Why human:** Requires live server lifecycle observation.

### 8. Desktop Lock Toggle (D-Desktop-02) — CRITICAL

**Test:** In Settings → Сеть, locate the 'Требовать вход в десктопе' toggle. Enable it. Close and reopen the desktop app (or run `pnpm tauri dev` again).
**Expected:** App shows login screen on startup (not direct main screen). Login with admin credentials. Return to Settings → disable lock. Restart again — app goes directly to main screen without login prompt.
**Why human:** Requires app restart to observe boot-time behavior. The DB flag `desktop_lock_enabled` must be read correctly at startup by App.svelte `onMount` → `auth_status` → `desktop_lock_enabled` → bootstrap guard decision. This is the end-to-end test of D-Desktop-02 which is a locked architectural decision.

## Gaps Summary

Two gaps block the phase from passing:

**Gap 1 (BLOCKER): `settings_set_network` missing.** The plan's must_have (Plan 03) explicitly listed this as the 14th Tauri command and required it in specta_export. The HTTP route was also planned (settings router). Neither exists. The UI's 'Сохранить настройки' button in NetworkSettings.svelte calls `apiCall('settings_set_network', {...})` which will throw at runtime. ROADMAP success criterion #2 covers port/bind-address configuration — the server toggle itself works (server_toggle command exists) but users cannot save changed port or bind-address. Both the HTTP route and Tauri command need to be implemented.

**Gap 2 (BLOCKER): `trackly-infra/tests/migration_idempotency.rs` stale assertions.** The Phase 5 partial fix (commit 04579bd) updated `test_support/test_db.rs` but missed `tests/migration_idempotency.rs` which asserts 17 migrations and schema_version==17. Since Phase 5 added V018 and V019, this test now fails. A one-line fix is required per assertion line (17, 22-23, 28, 43 → update to 19).

**Human UAT:** 8 desktop test steps from Plan 05-05 checkpoint:human-verify were auto-approved and are outstanding. Item #8 (Desktop Lock) is security-critical. Items #1-7 cover core UI flows.

---

_Verified: 2026-06-13T16:45:00Z_
_Verifier: Claude (gsd-verifier)_

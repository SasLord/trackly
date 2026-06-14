---
phase: 05-auth-server-mode
verified: 2026-06-14T00:00:00Z
status: passed
score: 14/14
overrides_applied: 0
uat: passed (11/11, 05-UAT.md)
security: secured (30/30 threats closed, 05-SECURITY.md)
re_verification:
  previous_status: gaps_found
  previous_score: 12/14
  verified_at: 2026-06-14T00:00:00Z
  gaps_closed:
    - "settings_set_network Tauri command registered in specta_export (plan 03 must_have listed it as command #14 of 14)"
    - "trackly-infra migration test suite fully green after V018/V019 addition"
  gaps_remaining: []
  regressions: []
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

  - test: "Network Settings — Save network settings (settings_set_network)"
    expected: "On Network Settings tab, change port to e.g. 8444. Click 'Сохранить настройки'. No error thrown. Reopen settings — server_port persisted in app_settings, value shows 8444."
    why_human: "Requires live Tauri/browser invocation of the newly implemented settings_set_network command and DB inspection"

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
**Verified:** 2026-06-13T16:45:00Z (initial); **Re-verified:** 2026-06-14
**Status:** human_needed
**Re-verification:** Yes — after gap closure (Plan 05-06, commit 2a88cd9)

## Re-verification Summary (2026-06-14)

Plan 05-06 closed both BLOCKER gaps from the initial 12/14 report:

- **Gap 1 closed:** `settings_set_network` fully implemented on both transports. `build_settings_set_network()` added to `http/settings.rs` with `authorize(&caller, &Action::ManageSettings)` gate and port validation; `build_settings_set_network_tauri()` added to `tauri_cmds/auth.rs` using `resolve_tauri_identity()`; command registered as the 14th Phase 5 command in `specta_export.rs` line 99; `POST /api/v1/settings_set_network` route live in `router()` line 329; `ui/src/bindings.ts` line 552 exports the TypeScript wrapper; `NetworkSettings.svelte` `saveSettings()` wires to it at line 64.
- **Gap 2 closed:** `migration_idempotency.rs` assertions all read `== 19` (lines 22, 23, 28, 40, 43). `cargo test -p trackly-infra --test migration_idempotency` exits 0 (1 test, 0 failed) — confirmed independently by verifier.

Score advances to **14/14**. Status remains `human_needed` because the 8 (now 9, with the new settings_set_network UAT item) manual desktop UAT items are still outstanding.

## Context

Phase 5 spans 5 plans (01-05), a post-execution code review (05-REVIEW.md finding 5 critical + 7 warning issues), a fix pass (05-REVIEW-FIX.md) that closed 10 findings (CR-01..CR-05, WR-01..WR-03, WR-05, WR-07), and a gap-closure plan (05-06) that closed the 2 remaining BLOCKER gaps. The 8-step manual desktop UAT in Plan 05-05 Task 3 was auto-approved under auto-mode and has NOT been manually executed — these appear as human_verification items below, plus one new item for settings_set_network end-to-end.

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
| 13 | settings_set_network on both transports: HTTP route POST /api/v1/settings_set_network registered with authorize(ManageSettings); Tauri command registered as 14th Phase 5 command in specta_export; UI payload matches; server_host/server_port/server_cert_path persisted in app_settings | VERIFIED (re-verified 2026-06-14) | `http/settings.rs` line 329: route registered; line 109: `authorize(&caller, &Action::ManageSettings)`; `tauri_cmds/auth.rs` lines 279-285: `#[tauri::command] #[specta::specta] pub async fn settings_set_network`; `specta_export.rs` line 99: registered as 14th Phase 5 command; `ui/src/bindings.ts` line 552: TypeScript wrapper exported; `NetworkSettings.svelte` line 64: `apiCall('settings_set_network', { patch: { host, port, cert_path } })` |
| 14 | trackly-infra migration test suite fully green after V018/V019 addition | VERIFIED (re-verified 2026-06-14) | `migration_idempotency.rs` lines 22, 23, 28, 40, 43 all assert == 19 (first-run applied_count=19, schema_version=19; no-op runs applied_count=0); `cargo test -p trackly-infra --test migration_idempotency` → 1 passed, 0 failed (confirmed by independent verifier run) |

**Score:** 14/14 truths verified

### Deferred Items

None.

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
| `crates/trackly-app/src/http/settings.rs` | server_toggle, server_status, settings_get_network, desktop_set_lock, settings_set_network | VERIFIED | All five routes present; NetworkPatch + SetNetworkPayload + build_settings_set_network + handler_set_network added by Plan 05-06; TODO comment removed; fingerprint: None in get_network response is a minor known stub (fingerprint IS returned from server_toggle response) |
| `crates/trackly-app/src/http/mod.rs` | build_router() with SessionManagerLayer, security headers, rate limit | VERIFIED | Present; WR-07: script-src 'self' without unsafe-inline |
| `crates/trackly-app/src/tauri_cmds/auth.rs` | auth_login, auth_logout, auth_status, auth_me, server_toggle, server_status, desktop_set_lock, settings_set_network | VERIFIED | Present; CR-01 fix: desktop_set_lock uses resolve_tauri_identity; settings_set_network added by Plan 05-06 with correct attribute order |
| `crates/trackly-app/src/tauri_cmds/users.rs` | users_list/create/update/delete/change_password via resolve_tauri_identity | VERIFIED | Present; CR-02 fix in Tauri path also |
| `crates/trackly-app/src/specta_export.rs` | 14 Phase 5 commands registered | VERIFIED | Line 99: `crate::tauri_cmds::auth::settings_set_network` registered as 14th Phase 5 command |
| `ui/src/lib/stores/auth.svelte.ts` | authStore Svelte 5 $state singleton | VERIFIED | Present |
| `ui/src/features/auth/LoginPage.svelte` | Login form with auth_login call | VERIFIED | Present |
| `ui/src/features/auth/FirstRunWizard.svelte` | First admin creation with admin role | VERIFIED | Present |
| `ui/src/features/users/UsersPage.svelte` | Users CRUD | VERIFIED | Present |
| `ui/src/features/settings/NetworkSettings.svelte` | Server toggle + desktop lock toggle (D-Desktop-02) + save network settings | VERIFIED | Present; saveSettings() at line 64 calls `apiCall('settings_set_network', { patch: { host, port, cert_path } })` — now wired |
| `ui/src/App.svelte` | Bootstrap guard: FirstRunWizard or LoginPage or Layout | VERIFIED | Present; desktop_lock_enabled-aware logic at lines 33-44 |
| `crates/trackly-infra/tests/migration_idempotency.rs` | Updated assertions for V018+V019 | VERIFIED | All assertions use 19; no stale == 17 present; test green |

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
| `NetworkSettings.svelte saveSettings()` | `tauri_cmds/auth::settings_set_network` (Tauri) / `POST /api/v1/settings_set_network` (HTTP) | `apiCall('settings_set_network', { patch: { host, port, cert_path } })` | VERIFIED (re-verified 2026-06-14) | NetworkSettings.svelte line 64; command registered in specta_export.rs line 99; HTTP route in settings.rs router() line 329 |
| `http/settings.rs build_settings_set_network` | `trackly_core::auth::authorize` | `authorize(&caller, &Action::ManageSettings)` | VERIFIED (re-verified 2026-06-14) | settings.rs line 109 |
| `tauri_cmds/auth.rs build_settings_set_network_tauri` | `resolve_tauri_identity` | `crate::tauri_cmds::users::resolve_tauri_identity(ctx).await?` | VERIFIED (re-verified 2026-06-14) | tauri_cmds/auth.rs line 242 |
| `http/settings.rs build_settings_set_network` | app_settings table | upsert server_host / server_port / server_cert_path | VERIFIED (re-verified 2026-06-14) | settings.rs lines 130-140: three separate upserts |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|--------------|--------|-------------------|--------|
| `http/auth.rs:login handler` | UserDto | AuthService::login → argon2 verify → DB query | Yes — real DB row | FLOWING |
| `server/rusqlite_session_store.rs:load` | Record | SELECT FROM sessions WHERE expiry_date > NOW | Yes — real DB row | FLOWING |
| `App.svelte:onMount` | status (AuthStatusDto) | auth_status Tauri command → AuthService | Yes — DB-backed | FLOWING |
| `NetworkSettings.svelte:saveSettings` | void (upsert) | `settings_set_network` → three upserts in app_settings | Yes — writes server_host/server_port/server_cert_path | FLOWING (re-verified 2026-06-14) |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| role_endpoint_matrix (9-case CI test) | `cargo test -p trackly-app --test role_endpoint_matrix` | 1 test, 9 assertions passed | PASS |
| Auth unit tests | `cargo test -p trackly-app --test auth_smoke` | 6 tests passed | PASS |
| Users CRUD tests | `cargo test -p trackly-app --test users_crud` | 6 tests passed (updated count) | PASS |
| Security headers + rate limit | `cargo test -p trackly-app --test security_headers` | 2 tests passed | PASS |
| Session persistence across restart | `cargo test -p trackly-app --test session_survives_restart` | 4 tests passed | PASS |
| TLS fingerprint + server lifecycle | `cargo test -p trackly-app --test tls_server_smoke --test server_hot_toggle --test graceful_shutdown_drain` | 7 tests passed | PASS |
| trackly-infra migration idempotency | `cargo test -p trackly-infra --test migration_idempotency` | 1 passed, 0 failed (re-verified 2026-06-14) | PASS |
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
| SET-08 | 05-03, 05-05, 05-06 | Настройки сетевого доступа: порт, bind-адрес, toggle | SATISFIED (re-verified 2026-06-14) | Toggle (server_toggle) works; GET network settings works; SET network settings works: settings_set_network persists server_host/server_port/server_cert_path to app_settings |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `crates/trackly-app/src/http/settings.rs` | 93 | `fingerprint: None // TODO: store fingerprint in server_ctl` | WARNING | fingerprint: None returned from settings_get_network (but IS returned correctly from server_toggle response). Cosmetic — not a blocker. |

Note: The original BLOCKER anti-patterns (TODO comment on settings_set_network, stale migration_idempotency assertions) were both resolved by Plan 05-06. The remaining WARNING (fingerprint: None in get_network) predates this phase.

### Human Verification Required

The following items require manual desktop UAT (pnpm tauri dev). 8 items from Plan 05-05 checkpoint:human-verify were auto-approved under auto-mode and have NOT been executed by a human. One additional item (Network Settings save) is new from Plan 05-06.

**Note: Item #9 (Desktop Lock) is the most security-critical manual test.** D-Desktop-02 (desktop lock toggle) is a locked architectural decision whose end-to-end behavior — DB-flag read at boot drives login screen appearance — can only be verified by restarting the application.

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

### 5. Network Settings — Save Network Settings (Plan 05-06)

**Test:** On Network Settings tab, change port to e.g. 8444. Click 'Сохранить настройки'.
**Expected:** No error thrown. Inspect DB (`SELECT value FROM app_settings WHERE key='server_port'`) — value shows '8444'. Optional: verify NetworkSettings reloads the saved value on next open.
**Why human:** Requires live Tauri invocation of the new settings_set_network command and DB inspection to confirm persistence.

### 6. Browser HTTPS Access

**Test:** While server is running, open Chrome/Firefox and navigate to https://127.0.0.1:8443. Accept self-signed certificate warning.
**Expected:** Login page loads in browser. Admin login with admin/password123 succeeds. App renders in browser with correct sidebar.
**Why human:** Requires real browser interaction and self-signed cert acceptance.

### 7. Employee Role in Browser

**Test:** From admin desktop, create an employee user. In browser, logout and login as employee.
**Expected:** Browser sidebar shows restricted sections. Employee cannot see 'Пользователи' or 'Настройки'.
**Why human:** Requires browser session with employee role and visual sidebar check.

### 8. Stop Server

**Test:** In desktop Settings, click 'Остановить сервер'. Then try to access https://127.0.0.1:8443 in browser.
**Expected:** Browser shows connection refused within a few seconds.
**Why human:** Requires live server lifecycle observation.

### 9. Desktop Lock Toggle (D-Desktop-02) — CRITICAL

**Test:** In Settings → Сеть, locate the 'Требовать вход в десктопе' toggle. Enable it. Close and reopen the desktop app (or run `pnpm tauri dev` again).
**Expected:** App shows login screen on startup (not direct main screen). Login with admin credentials. Return to Settings → disable lock. Restart again — app goes directly to main screen without login prompt.
**Why human:** Requires app restart to observe boot-time behavior. The DB flag `desktop_lock_enabled` must be read correctly at startup by App.svelte `onMount` → `auth_status` → `desktop_lock_enabled` → bootstrap guard decision. This is the end-to-end test of D-Desktop-02 which is a locked architectural decision.

## Gaps Summary

No automated gaps remain. Both BLOCKER gaps from the initial verification are confirmed closed:

- **Gap 1 CLOSED (2026-06-14):** `settings_set_network` fully implemented on HTTP (`POST /api/v1/settings_set_network` in router(), `build_settings_set_network()` with `authorize(ManageSettings)`, three upserts to app_settings) and Tauri (`settings_set_network` command in tauri_cmds/auth.rs with `resolve_tauri_identity()` + `authorize(ManageSettings)`, registered as 14th Phase 5 command in specta_export.rs). TypeScript binding generated in ui/src/bindings.ts line 552. NetworkSettings.svelte saveSettings() is now fully wired.
- **Gap 2 CLOSED (confirmed 2026-06-14, fixed by prior commit 7c26288):** `migration_idempotency.rs` all assertions use 19. `cargo test -p trackly-infra --test migration_idempotency` exits 0.

**Automated score: 14/14.** Outstanding: 9 manual desktop UAT items (item #9, Desktop Lock, is the most security-critical).

---

_Initial verified: 2026-06-13T16:45:00Z_
_Re-verified: 2026-06-14_
_Verifier: Claude (gsd-verifier)_

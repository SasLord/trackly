---
phase: 05-auth-server-mode
plan: "05"
subsystem: auth-ui
tags: [svelte5, runes, auth-store, login-page, first-run-wizard, sidebar-rbac, users-crud, network-settings, desktop-lock]

# Dependency graph
requires:
  - phase: 05-03
    provides: "Tauri commands: auth_login, auth_status, users_*, server_toggle, desktop_set_lock, settings_get_network"
  - phase: 05-04
    provides: "RBAC enforcement in all mutation endpoints (authorize() in build_* helpers)"

provides:
  - "authStore: Svelte 5 $state singleton with UserRole, CurrentUser, isAuthenticated()"
  - "App.svelte: bootstrap guard — auth_status → FirstRunWizard | LoginPage (desktop-lock-aware) | Layout"
  - "LoginPage.svelte: login/password form with inline validation and server error"
  - "FirstRunWizard.svelte: first admin creation form with auto-login"
  - "sidebar-config.ts: getVisibleItems(role) filters SIDEBAR_ITEMS by roles[] field"
  - "Sidebar.svelte: uses getVisibleItems(authStore.user?.role) for role-filtered navigation"
  - "client.ts: 401 intercept → authStore.clear() + redirect #/login"
  - "UsersPage.svelte: CRUD with UsersList + UserListRow + UserFormModal"
  - "NetworkSettings.svelte: server toggle + URL/fingerprint display + desktop lock toggle (D-Desktop-02)"

affects:
  - "All authenticated pages — sidebar now filtered by role"
  - "App entry — bootstrap guard replaces direct Layout render"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "authStore = $state({user: null}) module-level — matches toast.svelte.ts pattern exactly"
    - "App.svelte onMount: auth_status → isTauri && !desktop_lock_enabled → trusted-admin sentinel (D-Desktop-01)"
    - "client.ts: res.status === 401 → authStore.user = null + window.location.hash = '#/login'"
    - "getVisibleItems(role): filter SIDEBAR_ITEMS by entry.roles (undefined = visible to all)"
    - "UserFormModal: $effect reinitializes form on open (same pattern as DeviceFormBody {#key})"

key-files:
  created:
    - "ui/src/lib/stores/auth.svelte.ts"
    - "ui/src/features/auth/LoginPage.svelte"
    - "ui/src/features/auth/FirstRunWizard.svelte"
    - "ui/src/features/users/UsersPage.svelte"
    - "ui/src/features/users/UsersList.svelte"
    - "ui/src/features/users/UserListRow.svelte"
    - "ui/src/features/users/UserFormModal.svelte"
    - "ui/src/features/settings/NetworkSettings.svelte"
  modified:
    - "ui/src/lib/api/client.ts"
    - "ui/src/features/layout/sidebar-config.ts"
    - "ui/src/features/layout/Sidebar.svelte"
    - "ui/src/routes.ts"
    - "ui/src/App.svelte"
    - "ui/src/pages/UsersPage.svelte"
    - "ui/src/pages/SettingsPage.svelte"
    - "crates/trackly-app/src/tauri_cmds/users.rs"
    - "crates/trackly-app/src/http/users.rs"
    - "crates/trackly-app/tests/role_endpoint_matrix.rs"

key-decisions:
  - "UserRole type in auth.svelte.ts, not bindings.ts — avoids coupling to generated file"
  - "Desktop lock sentinel: id=0, login='desktop', role='admin' — local UI state only, all API calls still authorized server-side"
  - "getVisibleItems() preserves dividers (layout handles consecutive dividers cosmetically)"
  - "NetworkSettingsDto declared locally in NetworkSettings.svelte — not yet in bindings.ts (specta regenerates bindings.ts on every cargo test)"
  - "users_create parameter renamed 'new' → 'user_new' to avoid TypeScript reserved keyword collision"

# Metrics
duration: 17min
completed: 2026-06-13
---

# Phase 05 Plan 05: Auth UI (auth store + login + first-run wizard + users CRUD + network settings) Summary

**Svelte 5 runes auth store, App.svelte bootstrap guard (D-Desktop-02 aware), LoginPage, FirstRunWizard, sidebar role filtering, UsersPage CRUD, NetworkSettings with server toggle and desktop lock toggle**

## Performance

- **Duration:** ~17 min
- **Started:** 2026-06-13T11:32:00Z
- **Completed:** 2026-06-13T11:49:17Z
- **Tasks:** 2 (+ 1 checkpoint reached)
- **Files modified:** 18

## Accomplishments

- `auth.svelte.ts`: `$state` singleton matching `toast.svelte.ts` pattern; `UserRole`, `CurrentUser` types; `isAuthenticated()` helper
- `client.ts`: 401 intercept clears `authStore.user` and redirects to `#/login`; 403 throws without redirect
- `LoginPage.svelte`: standalone centered card; login/password fields with inline validation; server error display; calls `auth_login` + sets authStore + navigates to `/`
- `FirstRunWizard.svelte`: creates first admin via `users_create` then auto-logs in via `auth_login`; title "Добро пожаловать в Trackly"
- `sidebar-config.ts`: added `roles?: UserRole[]` to SidebarItem; `getVisibleItems(role)` — Пользователи and Настройки hidden for non-admin; employee sees all items except those two
- `Sidebar.svelte`: uses `$derived(getVisibleItems(...))` for reactive role-filtered navigation
- `routes.ts`: added `/login` route
- `App.svelte`: bootstrap guard — `auth_status` call on mount; `needs_bootstrap` → `FirstRunWizard`; `isTauri && !desktop_lock_enabled` → trusted-admin sentinel; else `LoginPage`
- `UsersPage.svelte` (feature): CRUD with refresh/handleSave/handleDelete pattern from DevicesPage
- `UsersList.svelte`: table Логин|ФИО|Роль|Email|Статус|Действия; Russian role labels; empty state
- `UserListRow.svelte`: row with inline delete confirmation (Да/Нет)
- `UserFormModal.svelte`: create/edit modal; login readonly in edit; password optional in edit; role dropdown
- `NetworkSettings.svelte`: server toggle with URL+fingerprint display (D-Server-04 instruction); bind-address dropdown; desktop lock toggle calling `desktop_set_lock` (D-Desktop-02 mandatory)
- `pages/UsersPage.svelte`, `pages/SettingsPage.svelte`: replaced placeholders with feature components

## Task Commits

1. **Task 1: Auth store + App bootstrap guard + LoginPage + FirstRunWizard + sidebar + client 401** — `ba23f4b` (feat)
2. **Task 2: UsersPage CRUD + SettingsPage NetworkSettings with desktop lock** — `4cc1180` (feat)
3. **Fix: rename users_create param 'new' → 'user_new'** — `0ac6cb5` (fix)

## Files Created/Modified

**Created:**
- `ui/src/lib/stores/auth.svelte.ts`
- `ui/src/features/auth/LoginPage.svelte`
- `ui/src/features/auth/FirstRunWizard.svelte`
- `ui/src/features/users/UsersPage.svelte`
- `ui/src/features/users/UsersList.svelte`
- `ui/src/features/users/UserListRow.svelte`
- `ui/src/features/users/UserFormModal.svelte`
- `ui/src/features/settings/NetworkSettings.svelte`

**Modified:**
- `ui/src/lib/api/client.ts` — 401 intercept
- `ui/src/features/layout/sidebar-config.ts` — roles field + getVisibleItems()
- `ui/src/features/layout/Sidebar.svelte` — uses getVisibleItems($derived)
- `ui/src/routes.ts` — /login route
- `ui/src/App.svelte` — bootstrap guard
- `ui/src/pages/UsersPage.svelte` — replaced placeholder
- `ui/src/pages/SettingsPage.svelte` — replaced placeholder
- `crates/trackly-app/src/tauri_cmds/users.rs` — renamed param
- `crates/trackly-app/src/http/users.rs` — renamed CreatePayload field
- `crates/trackly-app/tests/role_endpoint_matrix.rs` — updated test payload

## Decisions Made

- **authStore pattern**: module-level `$state({ user: null as CurrentUser | null })` — matches `toast.svelte.ts` exactly. No class, no wrapping object, direct mutation.
- **Desktop lock branch in App.svelte**: `isTauri && !status.desktop_lock_enabled` sets trusted-admin sentinel (id=0, login='desktop', role='admin'). This is local UI state only — all API calls still authorized server-side.
- **NetworkSettingsDto local interface**: `specta` regenerates `bindings.ts` every `cargo test` — type must be declared locally in component until bindings include it.
- **users_create 'new' → 'user_new'**: TypeScript reserves `new` as keyword — Tauri command parameter names flow directly to bindings.ts. Fixed in Rust Tauri command, HTTP payload struct, and test.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] bindings.ts: `users_create` parameter named `new` — TypeScript reserved keyword**
- **Found during:** Task 1 (pnpm svelte-check)
- **Issue:** tauri-specta generates `async usersCreate(new: UserNew)` — `new` is a reserved keyword in TypeScript, causing 3 parse errors in `bindings.ts`
- **Fix:**
  1. Renamed parameter in `crates/trackly-app/src/tauri_cmds/users.rs`: `new: UserNew` → `user_new: UserNew`
  2. Updated `CreatePayload.new` → `CreatePayload.user_new` in `crates/trackly-app/src/http/users.rs` for payload consistency
  3. Updated `role_endpoint_matrix.rs` test payload key
  4. Updated `FirstRunWizard.svelte` and `UsersPage.svelte` call sites
- **Files modified:** `tauri_cmds/users.rs`, `http/users.rs`, `role_endpoint_matrix.rs`, `FirstRunWizard.svelte`, `UsersPage.svelte`
- **Commit:** `0ac6cb5`

**2. [Rule 2 - Missing critical] NetworkSettingsDto not exported from bindings.ts**
- **Found during:** Task 2 (pnpm svelte-check)
- **Issue:** `specta_export.rs` exports Tauri command signatures but `NetworkSettingsDto` was not inlined into bindings.ts as a standalone type
- **Fix:** Declared `NetworkSettingsDto` as a local interface in `NetworkSettings.svelte` matching the Rust struct shape
- **Files modified:** `ui/src/features/settings/NetworkSettings.svelte`
- **Commit:** `4cc1180`

### Scope Notes

- `bindings.ts` is gitignored (regenerated by `cargo test --test export_bindings`) — the reserved keyword fix was applied in the Rust source so all future regenerations produce valid TypeScript

## Known Stubs

None — all UI components are wired to real backend commands. NetworkSettings calls real Tauri/HTTP endpoints. The server URL/fingerprint display shows live data from `server_toggle` response.

## Checkpoint Reached

**Task 3: checkpoint:human-verify** — human verification of the complete auth flow (bootstrap wizard, login page, sidebar role filtering, server toggle, desktop lock toggle) is required to close Phase 5.

## Threat Flags

No new security-relevant surface beyond the plan's threat model. All T-05-2x mitigations implemented:
- T-05-20: password field type="password" — implemented in LoginPage and UserFormModal
- T-05-23: bootstrap check on every load — implemented in App.svelte onMount
- T-05-24: fingerprint displayed intentionally — implemented in NetworkSettings

## Self-Check: PASSED

Files verified:
- `ui/src/lib/stores/auth.svelte.ts` — FOUND
- `ui/src/features/auth/LoginPage.svelte` — FOUND
- `ui/src/features/auth/FirstRunWizard.svelte` — FOUND
- `ui/src/features/users/UsersPage.svelte` — FOUND
- `ui/src/features/settings/NetworkSettings.svelte` — FOUND
- `ui/src/App.svelte` — FOUND (bootstrap guard with desktop_lock_enabled logic)

Commits verified:
- `ba23f4b` — FOUND (feat: auth store + App bootstrap + LoginPage + FirstRunWizard + sidebar)
- `4cc1180` — FOUND (feat: UsersPage CRUD + NetworkSettings)
- `0ac6cb5` — FOUND (fix: rename users_create param)

Checks:
- `pnpm svelte-check`: 0 errors
- `cargo test -p trackly-app -p trackly-core`: all tests GREEN (0 FAILED)
- `grep -c "desktop_lock_enabled"` ui/src/App.svelte: 4 (>= 2)
- `grep -c "desktop_set_lock\|toggleDesktopLock\|Требовать вход"` ui/src/features/settings/NetworkSettings.svelte: 4 (>= 3)

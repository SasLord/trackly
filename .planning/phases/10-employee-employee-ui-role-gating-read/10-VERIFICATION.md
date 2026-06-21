---
phase: 10-employee-employee-ui-role-gating-read
verified: 2026-06-21T11:10:00Z
status: human_needed
score: 8/8 must-haves verified
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 6/8
  gaps_closed:
    - "Backend закрывает read-эндпоинты devices/acts/cartridges/printers/reports/users от employee (D-GATE-01/D-GATE-02) — devices_export_csv now gated"
    - "Backend закрывает read-эндпоинты devices/acts/cartridges/printers/reports/users от employee (D-GATE-01/D-GATE-02) — dashboard_get_consumption_chart now gated"
  gaps_remaining: []
  regressions: []
human_verification:
  - test: "Log in as a user with role=employee in a LAN browser session against the built ui/dist, and visually confirm the EmployeeLayout shell renders (header with brand 'Trackly', user name, 'Сотрудник' label, ThemeSwitcher, 'Выйти' button) instead of the full admin/manager Sidebar+Layout shell."
    expected: "Employee sees only the minimal header shell with no sidebar navigation to other sections; the landing view is the Requests page showing a 'Мои заявки' StatWidget card with real counts."
    why_human: "No frontend test runner exists in this repo by design (confirmed in 10-04-PLAN.md and 10-04-SUMMARY.md); rendering/visual behavior of EmployeeLayout.svelte cannot be verified by static analysis alone, only that the component and the App.svelte role branch exist and reference each other correctly in source."
  - test: "While logged in as employee, directly navigate via hash to each of the 8 forbidden routes (#/devices, #/acts, #/printers, #/cartridges, #/reports, #/users, #/settings, #/map) and confirm AccessDenied.svelte renders for every one (not the admin/manager target page, not a generic 404)."
    expected: "Every forbidden hash resolves to the 'Нет доступа' screen with the 'К заявкам' button returning to #/requests; attempting a 403-triggering action (e.g. a stale fetch to a gated endpoint) shows a toast 'Недостаточно прав для этого действия' without crashing the app or logging the user out."
    why_human: "employeeRoutes' catch-all '*' -> AccessDenied is confirmed at the source level (routes.ts), and client.ts's 403 toast branch is confirmed at the source level, but actual browser navigation behavior and toast rendering require a live session — no test runner exists to assert this programmatically."
---

# Phase 10: Ограничение роли employee — Verification Report

**Phase Goal:** Роль employee по-настоящему ограничена — отдельный минимальный employee-UI (лендинг на «Заявки», без доступа к навигации на другие разделы), backend закрывает read-эндпоинты devices/acts/cartridges/printers/reports/users от employee, дашборд employee показывает только его собственные заявки, заявки employee видны только свои (server-side override, не клиентский фильтр), экран «Нет доступа» при прямой навигации на запрещённый роут, CI-матрица role×endpoint расширена на read-пути.

**Verified:** 2026-06-21T11:10:00Z
**Status:** human_needed
**Re-verification:** Yes — after gap closure (commit `50aa64d`)

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `Action::ReadData` is Admin\|Manager only, not always-true (D-GATE-01) | VERIFIED | `crates/trackly-core/src/auth.rs` — `Action::ReadData` in the `Admin \| Manager` match arm; tests `authorize_employee_read_data_forbidden` / `authorize_manager_read_data_ok` present. Unchanged since prior verification; re-confirmed via the full green workspace test run. |
| 2 | All read endpoints for devices/acts/cartridges/printers/reports/users gate Employee on both transports (D-GATE-02) | VERIFIED (gap closed) | All plan-enumerated endpoints (list/get/search/list_grouped/status_counts/list_by_ids/state_hints/autocomplete and per-domain equivalents) confirmed gated, as before. **Additionally now closed**: `devices_export_csv` and `dashboard_get_consumption_chart` — both org-wide-data routes — are now gated. `build_devices_export_csv` (tauri_cmds/devices.rs:341-348) takes `caller: &Identity` and calls `authorize(caller, &Action::ReadData)?` first; `handler_export_csv` (http/devices.rs:361-374) binds `let identity` (not `_identity`) and threads it through. `build_dashboard_get_consumption_chart` (tauri_cmds/dashboard.rs:24-31) takes `caller` and calls `authorize(caller, &Action::ReadData)?`; `handler_get_consumption_chart` (http/dashboard.rs:52-65) binds `let identity` and threads it through. Confirmed by direct source read, not by trusting SUMMARY/commit-message claims. |
| 3 | `request_service.list` force-overrides `requested_by_user_id` for Employee server-side, not client filter (D-REQ-01) | VERIFIED | Unchanged since prior verification; CI Case 20 still proves this at the body level. |
| 4 | Employee cannot fetch another user's request via get/get_history — BOLA closed both transports | VERIFIED | Unchanged; CI Cases 21-22 (403) and Case 24 (Manager retains access) confirm. |
| 5 | Employee dashboard shows only own requests; org-wide fields are a structurally separate query path (D-GATE-03) | VERIFIED | Unchanged; `get_employee_widgets` confirmed structurally separate. CI Case 23/24 confirm. |
| 6 | Separate minimal employee-UI shell; no navigation to other sections (D-UI-01) | VERIFIED (code) / HUMAN NEEDED (render) | Unchanged. `EmployeeLayout.svelte` standalone, `App.svelte` role branch confirmed at source level; actual rendering requires a live browser session. |
| 7 | "Нет доступа" screen on direct navigation to forbidden route (D-DENY-01) | VERIFIED (code) / HUMAN NEEDED (render) | Unchanged. `AccessDenied.svelte` + `employeeRoutes['*']` + `client.ts` 403 branch confirmed at source level; rendering requires a live browser session. |
| 8 | CI matrix role×endpoint extended to cover read paths (D-TEST-01) | VERIFIED (gap closed) | `role_endpoint_matrix.rs` now has 28 cases (was 24): Cases 25-26 (`devices_export_csv` Employee→403, Manager→not-403) and Cases 27-28 (`dashboard_get_consumption_chart` Employee→403, Manager→not-403), added in commit `50aa64d`. Independently re-run: `TRACKLY_AD_MOCK=1 cargo test --test role_endpoint_matrix -- --test-threads=1` → `test result: ok. 1 passed; 0 failed`. |

**Score:** 8/8 truths verified. The 2 previously-PARTIAL truths (Truth 2, Truth 8) are now fully VERIFIED — the gap-closure commit was independently confirmed against source, not taken on the commit message's word. Truths 6 and 7 remain code-verified with rendering deferred to human verification, as explicitly instructed (no frontend test runner exists in this repository).

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/trackly-core/src/auth.rs` | `Action::ReadData` in Admin\|Manager arm | VERIFIED | Unchanged since prior verification. |
| `crates/trackly-app/src/services/request_service.rs` | Server-side ownership override + BOLA guard | VERIFIED | Unchanged. |
| `crates/trackly-app/src/services/dashboard_service.rs` | Separate employee query path | VERIFIED | Unchanged. |
| `crates/trackly-app/src/tauri_cmds/devices.rs` | `build_devices_export_csv(caller, ...)` calls `authorize(caller, &Action::ReadData)?` | VERIFIED (gap closed) | Read directly: lines 341-348. `caller: &Identity` parameter present; `authorize` call is the first statement in the function body. |
| `crates/trackly-app/src/http/devices.rs` | `handler_export_csv` threads real identity, not `_identity` | VERIFIED (gap closed) | Read directly: lines 361-374. `let identity = session_identity(&session)...` (no underscore prefix); `build_devices_export_csv(&ctx, &identity, payload.filter)` passes it through. |
| `crates/trackly-app/src/tauri_cmds/dashboard.rs` | `build_dashboard_get_consumption_chart(caller, ...)` calls `authorize(caller, &Action::ReadData)?` | VERIFIED (gap closed) | Read directly: lines 24-31. Confirmed. |
| `crates/trackly-app/src/http/dashboard.rs` | `handler_get_consumption_chart` threads real identity | VERIFIED (gap closed) | Read directly: lines 52-65. `let identity = ...` (no underscore); threaded into `build_dashboard_get_consumption_chart(&ctx, &identity, p.window_months)`. |
| `crates/trackly-app/tests/role_endpoint_matrix.rs` | CI matrix covering read paths incl. the 2 previously-ungated routes | VERIFIED (28 cases, all green) | Cases 25-28 added (lines 992-1068); independently re-run, `ok`. |
| `ui/src/features/layout/EmployeeLayout.svelte` | Standalone shell, "employee-brand" present | VERIFIED | Unchanged since prior verification. |
| `ui/src/pages/AccessDenied.svelte` | "Нет доступа" copy | VERIFIED | Unchanged. |
| `ui/src/App.svelte` | Role branch selecting EmployeeLayout+employeeRoutes | VERIFIED | Unchanged. |
| `ui/src/routes.ts` | `employeeRoutes` additive export | VERIFIED | Unchanged. |
| `ui/src/features/requests/RequestsPage.svelte` | "Мои заявки" StatWidget wired to employee dashboard branch | VERIFIED | Unchanged. |
| `ui/src/lib/api/client.ts` | Symmetric 403 toast handling, both transports | VERIFIED | Unchanged. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `auth.rs::authorize` | All gated read service/build_* functions (devices/acts/cartridges/printers/reports/dashboard) | `authorize(caller, &Action::ReadData)?` as first statement | WIRED (gap closed) | Now confirmed for ALL read endpoints, including `build_devices_export_csv` and `build_dashboard_get_consumption_chart`, which were previously the two exceptions. |
| `http/*.rs` handlers | Service layer | `let identity = session_identity(&session)` threaded into `build_*` calls | WIRED (gap closed) | `handler_export_csv` and `handler_get_consumption_chart` now bind `identity` (not `_identity`) and thread it through. The remaining `let _identity =` in `handler_import_csv_preview` (http/devices.rs:336) is unchanged from prior verification and was already classified as non-blocking (no existing-data read; only echoes uploaded bytes — see Anti-Patterns below). |
| `tauri_cmds/*.rs` wrappers | Service layer | `resolve_tauri_identity(state.inner()).await?` | WIRED | Unchanged; `devices_export_csv` and `dashboard_get_consumption_chart` Tauri wrappers both resolve identity and pass it to their `build_*` counterparts. |
| `request_service.list` | `RequestRepository` | `filter.requested_by_user_id` override before `spawn_blocking` query | WIRED | Unchanged. |
| `EmployeeLayout.svelte` | `App.svelte` | role==='employee' conditional render | WIRED | Unchanged. |
| `routes.ts::employeeRoutes['*']` | `AccessDenied.svelte` | svelte-spa-router catch-all | WIRED | Unchanged; rendering behavior remains the human_verification item. |
| `client.ts` 403 branch | `pushToast` | toast on Forbidden, no authStore mutation | WIRED | Unchanged. |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|---------------------|--------|
| `device_service::export_csv` (now gated) | full device export | Direct repo query, gated by `authorize(caller, &Action::ReadData)?` before query executes | Real org-wide data, now correctly restricted to Admin\|Manager only | FLOWING (correctly gated — gap closed) |
| `dashboard_service::get_consumption_chart` (now gated) | org-wide consumption analytics | `audit_log`+`cartridges`+`cartridge_models` join, gated by `authorize(caller, &Action::ReadData)?` before query executes | Real org-wide data, now correctly restricted to Admin\|Manager only | FLOWING (correctly gated — gap closed) |
| `RequestsPage.svelte` StatWidget | `dashboardWidget` | `apiCall('dashboard_get_all_widgets', {period: null})` -> `dashboard_service::get_employee_widgets` | Yes — real scoped `requests` count query | FLOWING (unchanged) |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| `role_endpoint_matrix` (28 cases, incl. new Cases 25-28 for gap closure) | `TRACKLY_AD_MOCK=1 cargo test --test role_endpoint_matrix -- --test-threads=1` | `test result: ok. 1 passed; 0 failed` | PASS |
| Full workspace test suite (all 84 test binaries) | `TRACKLY_AD_MOCK=1 cargo test --workspace -- --test-threads=1` | All binaries report `0 failed`; zero `FAILED`/`error[` lines in output | PASS |
| `devices_export_csv` reachable by Employee, no role check (regression check of previously-confirmed FAIL) | direct source read of `tauri_cmds/devices.rs:341-348` + `http/devices.rs:361-374` | `authorize(caller, &Action::ReadData)?` present in `build_devices_export_csv`; `let identity` (not `_identity`) threaded in `handler_export_csv` | PASS (gap closed) |
| `dashboard_get_consumption_chart` reachable by Employee, no role check (regression check) | direct source read of `tauri_cmds/dashboard.rs:24-31` + `http/dashboard.rs:52-65` | `authorize(caller, &Action::ReadData)?` present; `let identity` threaded through | PASS (gap closed) |
| Frontend type-check (no regression from gap-closure commit) | `pnpm --dir ui exec svelte-check` | `239 FILES 0 ERRORS 36 WARNINGS` — same baseline as prior verification | PASS |
| Commit `50aa64d` touches only the claimed files | `git show --stat 50aa64d` | 5 files changed: `http/dashboard.rs`, `http/devices.rs`, `tauri_cmds/dashboard.rs`, `tauri_cmds/devices.rs`, `tests/role_endpoint_matrix.rs` — exactly matches the claimed scope | PASS |

### Probe Execution

No `scripts/*/tests/probe-*.sh` files exist in this repository and none were declared by the Phase 10 plans/summaries. Step 7c: SKIPPED (no probes declared or found).

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|--------------|------------|--------------|--------|----------|
| D-UI-01 | 10-04 | Separate minimal employee-UI shell | SATISFIED (code) / NEEDS HUMAN (render) | Unchanged. |
| D-GATE-01 | 10-01 | `Action::ReadData` Admin\|Manager only | SATISFIED | Unchanged. |
| D-GATE-02 | 10-02 | All read endpoints gated both transports | SATISFIED (gap closed) | The 2 previously-ungated org-wide routes are now gated and covered by CI. |
| D-GATE-03 | 10-03 | Employee dashboard structurally separate, org fields zeroed | SATISFIED | Unchanged. |
| D-REQ-01 | 10-03 | Server-side ownership override, not client filter | SATISFIED | Unchanged. |
| D-DENY-01 | 10-04 | "Нет доступа" screen on forbidden direct navigation | SATISFIED (code) / NEEDS HUMAN (render) | Unchanged. |
| D-TEST-01 | 10-01/02/03 | CI matrix extended to read paths | SATISFIED (gap closed) | 28 cases all green, now including the 2 previously-missing routes. |
| USR-02 (REQUIREMENTS.md) | Phase 5 (pre-existing) | Three roles, Employee = create-requests only | Re-affirmed | Unchanged. |
| USR-06 (REQUIREMENTS.md) | Phase 5 (pre-existing) | Authorization enforced at API layer, cannot bypass via direct HTTP | Re-affirmed, gap closed | The residual bypass identified in the initial verification (`devices_export_csv`, `dashboard_get_consumption_chart`) is now closed; no known direct-HTTP bypass remains. |

No orphaned requirements found.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `crates/trackly-app/src/http/devices.rs` | 336 (`handler_import_csv_preview`) | `let _identity = session_identity(&session)...` — identity resolved then discarded, no authorize() call downstream | Warning (unchanged, not blocking) | Lower severity: only echoes caller-uploaded bytes back to the caller, no existing-DB-row read, so no data-exposure risk. Inconsistent with the rest of the gated devices read-surface but was correctly out of scope for the two blocking gaps closed in this re-verification. Not part of `must_haves` for Phase 10 — noted as residual `missing` item, not a gap. |
| No `TBD`/`FIXME`/`XXX` debt markers found in any file touched by the gap-closure commit (`50aa64d`). | — | — | — | — |

The two previously-flagged Blocker anti-patterns (`handler_export_csv` and `handler_get_consumption_chart` discarding identity with no `authorize()` call) are CONFIRMED RESOLVED by direct source read — both now bind `identity` (no underscore) and the call chain contains `authorize(caller, &Action::ReadData)?`.

### Human Verification Required

### 1. Employee shell visual rendering (D-UI-01)

**Test:** Log in as an employee-role user in a LAN browser session against the built `ui/dist` (after `pnpm --dir ui build`), and visually confirm the page shows the `EmployeeLayout` header shell (brand "Trackly", user name + "Сотрудник" label, theme switcher, "Выйти" button) and not the admin/manager Sidebar+Layout shell. Confirm the landing view is the Requests page with a "Мои заявки" StatWidget showing real counts.
**Expected:** Minimal shell with no sidebar/nav to other sections; StatWidget shows non-placeholder counts that match the employee's actual request history.
**Why human:** No frontend test runner exists in this repository by design (confirmed in 10-04-PLAN.md/10-04-SUMMARY.md). Source-level wiring (`App.svelte` role branch, `EmployeeLayout.svelte` standalone structure) is confirmed by static reading, but actual DOM rendering and visual correctness cannot be asserted without a live browser session.

### 2. Forbidden-route navigation and 403 toast behavior (D-DENY-01)

**Test:** While logged in as employee, directly type/navigate to each forbidden hash (`#/devices`, `#/acts`, `#/printers`, `#/cartridges`, `#/reports`, `#/users`, `#/settings`, `#/map`) and confirm `AccessDenied.svelte` ("Нет доступа") renders for every one. Also trigger a 403 from the backend (e.g., a stale client attempting a now-gated read) and confirm a toast appears ("Недостаточно прав для этого действия") without crashing the app or forcing a logout.
**Expected:** Every forbidden hash shows the AccessDenied screen with a working "К заявкам" button; 403 responses show a toast only, the user remains logged in.
**Why human:** Same reasoning as #1 — `employeeRoutes['*'] -> AccessDenied` and the `client.ts` 403 branch are confirmed at the source level, but route-resolution and toast-rendering behavior require a live session to observe.

### Gaps Summary

No gaps remain. Both blocking items from the initial verification are confirmed closed by independent source inspection (not by trusting SUMMARY.md or the commit message):

1. `POST /api/v1/devices_export_csv` — `build_devices_export_csv` now requires `caller: &Identity` and calls `authorize(caller, &Action::ReadData)?` before querying; `handler_export_csv` threads a real (non-discarded) identity through both transports.
2. `POST /api/v1/dashboard_get_consumption_chart` — `build_dashboard_get_consumption_chart` now requires `caller` and calls `authorize(caller, &Action::ReadData)?`; `handler_get_consumption_chart` threads a real identity through.

Both are covered by new CI Cases 25-28 in `role_endpoint_matrix.rs` (Employee→403, Manager→not-403 for each), independently re-run green (`TRACKLY_AD_MOCK=1 cargo test --test role_endpoint_matrix -- --test-threads=1`). The full workspace suite (84 test binaries) shows zero failures under `TRACKLY_AD_MOCK=1`. The two AD-dependent test failures that occur WITHOUT that env var (`restore_request_visibility_http`, `settings_ad`) are a pre-existing, documented dev-environment constraint (no AD reachable from this macOS box) — unrelated to Phase 10 and not counted as a regression.

Status is `human_needed` rather than `passed` because D-UI-01 and D-DENY-01 still require a live-browser rendering check — this was true in the initial verification and remains true now; it is not a regression, it is the expected terminal state for frontend-only behaviors in a repository with no frontend test runner. All 8 must-haves are otherwise fully and substantively verified against the actual codebase.

---

*Verified: 2026-06-21T11:10:00Z*
*Verifier: Claude (gsd-verifier)*

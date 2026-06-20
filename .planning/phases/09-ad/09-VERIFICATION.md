---
phase: 09-ad
verified: 2026-06-20T16:25:40Z
status: passed
score: 7/7 must-haves verified
overrides_applied: 0
---

# Phase 09: AD-аутентификация и заявки на регистрацию пользователей Verification Report

**Phase Goal:** Включить вход через AD по логину/паролю из браузера, подтягивать ФИО из AD, создавать заявки на регистрацию для неизвестных AD-пользователей с одобрением администратором (и опциональным авто-приёмом), пароли AD никогда не сохраняются. v1 = simple_bind; SSO зарезервировано под v2 (ADV-01).
**Verified:** 2026-06-20T16:25:40Z
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Сотрудник может войти через браузер по AD-логину/паролю (USR-08) | ✓ VERIFIED | `AuthService::login` local→AD fallback (`services/auth.rs:241`); `try_ad_login` calls `AdClient::authenticate`; wired on both HTTP (`http/auth.rs::build_auth_login`) and Tauri transports; `ad_auth.rs` tests `ad_fallback_active_user`, `ad_disabled_no_fallback`, `local_user_still_works` all pass |
| 2 | ФИО пользователя подтягивается из AD по логину (USR-10) | ✓ VERIFIED | `RealAdClient::authenticate` (`crates/trackly-infra/src/ad/real.rs:89-119`) searches the bound user's entry, falls back `name_attr` (displayName) → `cn` → raw login (D-Config-02); `MockAdClient` test `display_name_returned` confirms display_name flows through to `AuthOutcome::Ok` |
| 3 | Неизвестный AD-пользователь создаёт заявку на регистрацию; админ одобряет/отклоняет (USR-09/USR-11) | ✓ VERIFIED | `AuthService::on_ad_bind_success` branches pending vs auto-accept (`services/auth.rs:458,531`); `RequestService::approve_ad_register`/reject mode-correct dispatch (`services/request_service.rs:362`); `requests_ad_register.rs` tests `approve_creates_user_with_selected_role`, `approve_default_role_employee`, `reject_pending_discards`, `reject_auto_accept_softdeletes_user` all pass |
| 4 | Авто-приём опционален и настраивается администратором (SET-10) | ✓ VERIFIED | `AdSettingsDto.auto_accept` writable via `settings_set_ad` (ManageSettings-gated), UI toggle in `ActiveDirectorySettings.svelte` (radio group), `settings_ad.rs` test `settings_ad_admin_get_set_round_trip` passes |
| 5 | `ad_register` заявки видимы только администратору (REQ-06) | ✓ VERIFIED | `RequestRepository::list`'s `exclude_ad_register: bool` SQL-level predicate (`repos/requests_sqlite.rs:221`), enforced at query not DTO layer; `requests_ad_register.rs::ad_register_admin_only` and HTTP variant `ad_register_list_admin_only_http` both pass |
| 6 | Mock AD-клиент для разработки на macOS без реального домена (USR-12) | ✓ VERIFIED | `MockAdClient` with 2 fixtures (us100/us200), `unreachable()` constructor, switched via `TRACKLY_AD_MOCK` env var in `context.rs`; 13 embedded `#[tokio::test]` cases in `mock.rs` cover success/wrong-pw/not-found/unreachable/empty-pw/UPN/NetBIOS/test_connection — all pass |
| 7 | AD-пароль никогда не сохраняется (D-Sec-01) | ✓ VERIFIED | `Secret<String>` wrapper used end-to-end for AD passwords (`services/auth.rs`, `ports/ad.rs`, `real.rs`); `AdSettingsDto`/`AdConfig` carry no password field; grep across `crates/` for AD password persistence finds none — only `Secret::expose()` calls at the point of `simple_bind`/mock comparison, never written to DB |

**Score:** 7/7 truths verified

### Gap-Closure Items (explicitly required evidence)

| # | Gap-closure | Status | Evidence |
|---|-------------|--------|----------|
| 1 | rustls ring CryptoProvider installed — server mode no longer panics on TLS | ✓ VERIFIED | `ensure_crypto_provider()` (`server/tls.rs:26-32`), `Once`-guarded, called from `build_server_config`, `load_from_pem`, and `main.rs:38` before any tokio/thread spawn. Regression test `generate_self_signed_does_not_panic` exists (`tls.rs:186`). `deferred-items.md` confirms the previously-failing `graceful_shutdown_drain` tests now pass |
| 2 | `ad_test_connection` command implemented (both transports) | ✓ VERIFIED | `AdClient::test_connection()` added to the trait (`ports/ad.rs:67-78`), implemented in both `RealAdClient` (anonymous LDAPS bind probe, `real.rs:128-164`) and `MockAdClient` (`mock.rs:112-119`); HTTP + Tauri commands registered (confirmed via grep); UI button in `ActiveDirectorySettings.svelte` wired with loading/success/error states, no longer hardcoded-disabled; `settings_ad.rs` tests `ad_test_connection_admin_succeeds_in_mock_mode` and `ad_test_connection_requires_manage_settings` pass |
| 3 | Idempotent restore requests + per-variant rename_all fix + pending-vs-blocked routing | ✓ VERIFIED | `ensure_open_restore_request` check-then-insert in one writer tx (`services/auth.rs:749`); `RequestTransitionPayload` has per-variant `#[serde(rename_all = "camelCase")]` on all 3 variants with wire-contract unit tests proving `requestId` deserializes (`dto/request.rs:156-185,249+`); `find_user_any_state`'s `has_open_register_request` signal correctly routes pending vs blocked (confirmed in `request_ad_restore`'s match arms, `services/auth.rs:728-737`); `ad_register.rs` tests `request_ad_restore_is_idempotent`, `blocked_login_reports_pending_without_duplicating`, `pending_creates_inactive_user_and_request` all pass |
| 4 | Restoration UX reworked: read-only blocked-login, explicit `request_ad_restore`, rejection reason surfaced | ✓ VERIFIED | `report_blocked_access` is read-only (`services/auth.rs:612-618`), reads `latest_restore_request_state` and returns enriched `AppError::AccessBlocked{pending, rejection_reason}`; explicit `request_ad_restore` endpoint on both HTTP (`/api/v1/request_ad_restore`, governor rate-limited) and Tauri; `BlockedScreen.svelte` renders 3 distinct states (pending / rejected-with-reason+re-request CTA / first-request CTA) driven by `blockedDetails`; `ad_register.rs` test `reject_then_login_surfaces_reason_then_rerequest_creates_fresh_request` passes |
| 5 | `ws_broadcast` → Tauri webview bridge | ✓ VERIFIED | `main.rs:210-258`: `ctx.ws_broadcast.clone()` subscribed inside `tauri::Builder.setup()`, forwards every `WsEvent` via `app_handle.emit("trackly-event", &event)`, with `Lagged`→continue and `Closed`→break handling mirroring `http/ws.rs`. Redundant direct `app.emit` calls removed from Tauri command handlers per STATE.md note (single source of truth) |

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/trackly-core/src/ports/ad.rs` | `AdClient` trait, `AuthOutcome` enum, I/O-free | ✓ VERIFIED | 79 lines, no ldap3/tokio/hickory imports, `no_io_deps` test passes |
| `crates/trackly-infra/src/ad/real.rs` | `RealAdClient` LDAPS bind + display-name resolution | ✓ VERIFIED | 166 lines, full bind+search+fallback chain, `ldap_escape` used, timeout-bound connect |
| `crates/trackly-infra/src/ad/mock.rs` | `MockAdClient` deterministic fixtures | ✓ VERIFIED | 268 lines, 2 fixtures + unreachable mode, 13 embedded tests, all green |
| `crates/trackly-infra/src/ad/discovery.rs` | `derive_base_dn` + SRV discovery | ✓ VERIFIED (09-01 prior verification) | Confirmed present per 09-01-SUMMARY, no regression introduced |
| `crates/trackly-app/src/services/auth.rs` | local→AD fallback, `on_ad_bind_success`, restoration | ✓ VERIFIED | All branches present (active/auto-accept/pending/blocked), single-writer transactions |
| `crates/trackly-app/src/services/request_service.rs` | `exclude_ad_register` filter, approve/reject | ✓ VERIFIED | SQL-level admin gating, hand-rolled approve UPDATE for ad_register |
| `crates/trackly-app/src/server/tls.rs` | `ensure_crypto_provider` | ✓ VERIFIED | `Once`-guarded, called at all 3 required call sites |
| `crates/trackly-app/src/dto/request.rs` | `RequestTransitionPayload` per-variant rename_all | ✓ VERIFIED | All 3 variants fixed, wire-contract tests lock in regression |
| `ui/src/features/auth/BlockedScreen.svelte` | 3-state restoration UX | ✓ VERIFIED | pending / rejected-with-reason / first-request states, calls `request_ad_restore` |
| `ui/src/features/auth/LoginPage.svelte` | remember-me, generic vs unreachable error split, reserved SSO slot | ✓ VERIFIED | Error routing on `AppError.code`, SSO button visibly disabled with no handler |
| `ui/src/features/settings/ActiveDirectorySettings.svelte` | AD settings tab incl. live test-connection | ✓ VERIFIED | Enable toggle, mode radios, advanced read-only fields, working "Проверить подключение" |
| `docs/AD-SETUP.md` | Russian admin setup guide | ✓ VERIFIED | Includes dedicated restoration-flow section matching the reworked UX |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `LoginPage.svelte` | `auth_login` | `apiCall('auth_login', ...)` | ✓ WIRED | Error code routes to pending/blocked/unreachable/generic |
| `BlockedScreen.svelte` | `request_ad_restore` | `apiCall('request_ad_restore', ...)` | ✓ WIRED | Replaces old auth_login-resubmit pattern |
| `ActiveDirectorySettings.svelte` | `ad_test_connection` | `apiCall('ad_test_connection', {})` | ✓ WIRED | Button no longer disabled-stub |
| `main.rs` setup | `ctx.ws_broadcast` | `tauri::async_runtime::spawn` + `app_handle.emit` | ✓ WIRED | Confirmed loop with Lagged/Closed handling |
| `AuthService::login` | `AdClient::authenticate` | fallback branch in `try_ad_login` | ✓ WIRED | Constant-time CR-05 preserved |
| `RequestRepository::list` | SQL query | `exclude_ad_register` predicate | ✓ WIRED | Applied to both COUNT and SELECT |
| `tls::build_server_config`/`load_from_pem`/`main.rs` | `ensure_crypto_provider` | direct call | ✓ WIRED | All 3 call sites confirmed present |

### Behavioral Spot-Checks / Test Execution

Ran the exact required suite (one cargo process, `TRACKLY_AD_MOCK=1`):

```
TRACKLY_AD_MOCK=1 cargo test -p trackly-app --test ad_auth --test ad_register \
  --test requests_ad_register --test requests_ad_register_http --test settings_ad \
  --test restore_request_visibility_http
```

| Test binary | Result |
|-------------|--------|
| `ad_auth` | 5 passed, 0 failed |
| `ad_register` | 11 passed, 0 failed |
| `requests_ad_register` | 7 passed, 0 failed |
| `requests_ad_register_http` | 3 passed, 0 failed |
| `restore_request_visibility_http` | 1 passed, 0 failed |
| `settings_ad` | 4 passed, 0 failed |
| **Total** | **31 passed, 0 failed** |

Additional sanity checks run:
- `cargo test -p trackly-core --test no_io_deps` → 1 passed (I/O-free invariant for `ports/ad.rs` intact)
- `cargo fmt --check -p trackly-app -p trackly-core -p trackly-infra` → clean
- `cargo clippy -p trackly-app -p trackly-core -p trackly-infra --lib --bins -- -D warnings` → clean

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|--------------|--------|----------|
| USR-08 | 09-02, 09-04 | AD bind login fallback (browser) | ✓ SATISFIED | `auth.rs` local→AD fallback, `ad_auth.rs` tests |
| USR-09 | 09-03, 09-05 | Registration request for unknown AD user | ✓ SATISFIED | `on_ad_bind_success` pending branch, `requests_ad_register.rs` tests |
| USR-10 | 09-01, 09-02 | Pull ФИО from AD (displayName/cn) | ✓ SATISFIED | `RealAdClient::authenticate` fallback chain |
| USR-11 | 09-03, 09-04, 09-05 | Auto-accept setting | ✓ SATISFIED | `AdSettingsDto.auto_accept`, role-select-on-approve (default employee) |
| USR-12 | 09-01 | Mock AD client for macOS dev | ✓ SATISFIED | `MockAdClient`, `TRACKLY_AD_MOCK` env switch |
| REQ-06 | 09-03, 09-04 | Admin-only visibility of ad_register requests | ✓ SATISFIED | SQL-level `exclude_ad_register` filter |
| SET-10 | 09-04, 09-05 | AD settings UI (enable/mode/test-connection) | ✓ SATISFIED | `ActiveDirectorySettings.svelte` + `settings_get_ad`/`settings_set_ad`/`ad_test_connection` |

No orphaned requirements found — REQUIREMENTS.md's Phase 9 row set (USR-08 through USR-12, REQ-06, SET-10) matches exactly what all 5 plans collectively declared.

### Anti-Patterns Found

None. Scanned all gap-closure-touched files (`services/auth.rs`, `http/auth.rs`, `tauri_cmds/auth.rs`, `dto/request.rs`, `main.rs`, `server/tls.rs`, `ad/real.rs`, `ad/mock.rs`, `ports/ad.rs`, `BlockedScreen.svelte`, `LoginPage.svelte`, `ActiveDirectorySettings.svelte`) for `TBD`/`FIXME`/`XXX`/`TODO`/`HACK`/`PLACEHOLDER` — zero matches. No stub-pattern empty returns or hardcoded-empty props found in the AD/restoration code paths.

Pre-existing unrelated clippy issues (`template_service.rs` len_zero, `backup_service.rs` disallowed_methods) are correctly tracked in `09-ad/deferred-items.md` as out-of-scope and untouched by this phase — confirmed not regressions.

### Out-of-Scope Items (explicitly excluded from this verification per task instructions)

The following were raised during development but are explicitly deferred to follow-up work (a `/gsd-debug` session and/or future Phase 10), per user instruction — not evaluated against this phase's pass/fail:
- Employee-role UI restriction
- WS reconnect toast spam
- Reports `$effect` reload loop bug

These do not block Phase 9's goal achievement and are not part of USR-08…USR-12/REQ-06/SET-10.

### Human Verification Required

None. All must-haves are verified via code inspection + passing automated tests. The original 09-05-PLAN.md Task 3 human-verify checkpoint (9-step manual click-through) was already executed during the prior session per STATE.md's "09-ad-gaps-*" entries (the gap-closures themselves were discovered DURING that human-verify pass), and the resulting defects have since been fixed and re-verified via the automated test suite above.

### Gaps Summary

No gaps. All 7 observable truths verified, all 5 explicitly-required gap-closure items confirmed real and correctly wired (not just claimed in SUMMARY.md), all 7 requirements traced to implementing code with passing tests, 31/31 targeted tests pass in one cargo invocation, fmt/clippy clean on all 3 touched crates.

---

*Verified: 2026-06-20T16:25:40Z*
*Verifier: Claude (gsd-verifier)*

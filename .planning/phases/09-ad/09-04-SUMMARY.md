---
phase: 09-ad
plan: 04
subsystem: auth
tags: [axum, tauri, tower-sessions, specta, rbac, ad]

requires:
  - phase: 09-ad (Plan 02/03)
    provides: AuthService AD settings get/set (ad_enabled/ad_auto_accept), RequestService.approve_ad_register, admin-only ad_register list filtering, automatic restoration-request creation on AD bind for blocked/soft-deleted users
provides:
  - LoginRequest.remember (D-UX-02) + per-session cookie expiry policy in build_auth_login
  - AdSettingsDto (snake_case, no secret fields) mirroring NetworkSettingsDto's TOML/DB split
  - settings_get_ad / settings_set_ad on both axum HTTP and Tauri transports, ManageSettings-gated
  - HTTP-transport regression coverage for ad_register admin-only listing and requests_approve_ad_register (REQ-06 end-to-end)
  - ui/src/bindings-phase9.ts hand-written TS types for the new Phase 9 wire shapes
affects: [09-ad plan 05 (frontend AD settings UI + login remember checkbox + approve UI)]

tech-stack:
  added: []
  patterns:
    - "Per-session cookie expiry override via tower_sessions::Session::set_expiry(Some(Expiry)), called AFTER session.insert() (not before flush) so it survives the T-05-SF flush-before-insert sequence"
    - "Read-only TOML bootstrap config + live DB-backed toggle split (AdSettingsDto mirrors NetworkSettingsDto): connection fields (host/port/domain/base_dn/name_attr/no_tls_verify) come from ctx.config.ad and are NOT settable via settings_set_ad; only enabled/auto_accept persist to app_settings"
    - "HTTP-transport test harness: programmatic session creation via RusqliteSessionStore::create() bypasses GovernorLayer entirely for routes that don't need real-TCP peer-IP testing (settings_ad.rs, requests_ad_register_http.rs); real axum::serve + into_make_service_with_connect_info() only needed for GovernorLayer-protected /auth_login (auth_remember_cookie.rs)"

key-files:
  created:
    - crates/trackly-app/tests/auth_remember_cookie.rs
    - crates/trackly-app/tests/settings_ad.rs
    - crates/trackly-app/tests/requests_ad_register_http.rs
    - ui/src/bindings-phase9.ts
  modified:
    - crates/trackly-app/src/dto/auth.rs
    - crates/trackly-app/src/http/auth.rs
    - crates/trackly-app/src/tauri_cmds/auth.rs
    - crates/trackly-app/src/specta_export.rs
    - crates/trackly-app/tests/ad_auth.rs
    - crates/trackly-app/tests/ad_register.rs
    - crates/trackly-app/tests/auth_smoke.rs
    - crates/trackly-app/tests/users_crud.rs

key-decisions:
  - "remember=true sets Expiry::OnInactivity(30 days) (persistent, sliding); remember=false/absent sets Expiry::OnSessionEnd (cleared on browser close) — set_expiry called after insert() so it isn't wiped by the preceding flush()"
  - "AdSettingsDto deliberately has zero AD-password fields (T-09-17/D-Sec-01) — connection settings (host/port/domain/base_dn/name_attr/no_tls_verify) are read-only TOML bootstrap config, not editable via settings_set_ad; only enabled/auto_accept are live DB-backed and writable"
  - "Restoration-request creation needed no new endpoint — it already happens server-side inside AuthService::login's blocked/soft-deleted branch (built in Plan 03); Plan 04's stated scope item was already complete"
  - "ApproveAdRegisterDto and requests_approve_ad_register (both transports) were already fully built in Plan 03 — Plan 04 only needed to add the missing HTTP-transport regression tests, not new production code"
  - "ui/src/bindings-phase9.ts placed at ui/src/ (not ui/src/lib/ as the plan frontmatter's files_modified path stated) — matches the actual, already-established bindings-phase6.ts convention and its real import sites (e.g. '../../bindings-phase6' from ui/src/features/*); the plan's lib/ path was a stale/incorrect reference"

requirements-completed: [USR-08, USR-11, SET-10, REQ-06]

duration: ~50min
completed: 2026-06-20
---

# Phase 09 Plan 04: AD Transport Layer Summary

**`settings_get_ad`/`settings_set_ad` on axum + Tauri, "Запомнить меня" cookie-expiry policy, and HTTP-transport regression tests proving REQ-06's admin-only `ad_register` visibility already works end-to-end.**

## Performance

- **Duration:** ~50 min (continuation from prior session's compaction point — Task 1 implementation/testing happened before this segment; this segment covered Task 1 commit + all of Task 2)
- **Completed:** 2026-06-20
- **Tasks:** 2/2
- **Files modified:** 8 modified, 4 created

## Accomplishments
- `build_auth_login` now applies the D-UX-02 "Запомнить меня" cookie policy (persistent 30-day sliding vs session-only), verified end-to-end over a real TCP HTTP request (required to satisfy `tower_governor`'s `PeerIpKeyExtractor` on `/auth_login`).
- `AdSettingsDto` added — snake_case wire shape mirroring `NetworkSettingsDto`'s TOML-bootstrap/DB-live split, with zero AD-password fields (grep-gated).
- `settings_get_ad`/`settings_set_ad` now reachable on both axum HTTP (`protected_router`, ManageSettings-gated) and Tauri (`resolve_tauri_identity` + same authorize check) — true "one DTO, two transports" thin adapters.
- Discovered and closed a test-coverage gap: Plan 03 had already built `ApproveAdRegisterDto`, the `requests_approve_ad_register` route on both transports, and admin-only `ad_register` list filtering in the service layer — but no HTTP-transport regression test existed. Added `requests_ad_register_http.rs` to prove REQ-06 holds over the wire, not just at the service layer.
- `ui/src/bindings-phase9.ts` hand-written (matching the established `bindings-phase6.ts` convention) covering `LoginRequest.remember`, `AdSettingsDto`, `SetAdPayload`, `ApproveAdRegisterDto`, and a doc note on `RequestDto.adSubtype`.

## Task Commits

Each task was committed atomically:

1. **Task 1: LoginRequest.remember + cookie policy + AdSettingsDto + approve DTO** - `6665c3d` (feat)
2. **Task 2: axum + Tauri endpoints (AD settings) + bindings regen** - `f715055` (feat)
3. **Task 2 (extension): HTTP-transport coverage for ad_register list/approve** - `55c94aa` (test)

**Plan metadata:** _(final docs commit, see below)_

## Files Created/Modified
- `crates/trackly-app/src/dto/auth.rs` - `LoginRequest.remember` (`#[serde(default)]`), `AdSettingsDto` (no secret fields)
- `crates/trackly-app/src/http/auth.rs` - `build_auth_login` cookie-expiry policy; `SetAdPayload`, `build_settings_get_ad`/`build_settings_set_ad`, handlers, routes registered in `protected_router`
- `crates/trackly-app/src/tauri_cmds/auth.rs` - `settings_get_ad`/`settings_set_ad` Tauri commands mirroring the HTTP helpers
- `crates/trackly-app/src/specta_export.rs` - registered the two new Tauri commands
- `crates/trackly-app/tests/auth_remember_cookie.rs` - real-TCP HTTP test for the remember cookie policy (`Set-Cookie` Max-Age/Expires presence vs absence)
- `crates/trackly-app/tests/settings_ad.rs` - 403-for-non-admin (get+set), 401-for-no-session, admin get/set/re-get round-trip, no-password-in-response check
- `crates/trackly-app/tests/requests_ad_register_http.rs` - admin-only `ad_register` visibility and `requests_approve_ad_register` over real HTTP
- `crates/trackly-app/tests/{ad_auth,ad_register,auth_smoke,users_crud}.rs` - mechanical `remember: false` field additions to existing `LoginRequest` struct literals (compile fix; struct literals don't pick up serde defaults)
- `ui/src/bindings-phase9.ts` - hand-written TS types for the new Phase 9 DTOs

## Decisions Made
See `key-decisions` in frontmatter. Most significant: the plan's stated Task 2 scope (AD settings, restoration-request create, approve-with-role, on both transports) was substantially already complete from Plan 03 — only `settings_get_ad`/`settings_set_ad` were genuinely new production code. The remaining work was closing a test-coverage gap (HTTP-transport tests for already-built `approve_ad_register`/admin-only-list behavior) rather than writing new handlers.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `oneshot()` returns 500 on `/auth_login` due to `GovernorLayer`'s `ConnectInfo` requirement**
- **Found during:** Task 1 (writing `auth_remember_cookie.rs`)
- **Issue:** `tower_governor::GovernorLayer`'s `PeerIpKeyExtractor` requires `axum::extract::ConnectInfo<SocketAddr>` in request extensions to compute the per-IP rate-limit bucket on `/auth_login`. Plain `axum::Router::oneshot()` (no real TCP socket) and even a bare `axum::serve(listener, router)` (no `ConnectInfo` layer) both leave this extension absent, returning HTTP 500 "Unable To Extract Key!" — this is documented, pre-existing behavior (see `security_headers.rs`'s `rate_limit_on_login` test, which explicitly avoids asserting 200 for this exact reason).
- **Fix:** Used `axum::serve(listener, router.into_make_service_with_connect_info::<SocketAddr>())`, mirroring the manual `ConnectInfo` injection already present in `server/mod.rs`'s production TLS accept-loop.
- **Files modified:** `crates/trackly-app/tests/auth_remember_cookie.rs`
- **Verification:** Test passes, returns real `Set-Cookie` headers for inspection.
- **Committed in:** `6665c3d`

**2. [Rule 3 - Blocking] `cargo build` failures in 4 pre-existing test files after adding `LoginRequest.remember`**
- **Found during:** Task 1
- **Issue:** Adding a new field to `LoginRequest` (even with `#[serde(default)]`) does not help existing Rust struct literals in `ad_auth.rs`, `ad_register.rs`, `auth_smoke.rs`, `users_crud.rs` — `#[serde(default)]` only affects deserialization, not struct-literal construction, so these test files failed to compile.
- **Fix:** Added `remember: false,` to each affected struct literal (mechanical, no behavior change).
- **Files modified:** `crates/trackly-app/tests/{ad_auth,ad_register,auth_smoke,users_crud}.rs`
- **Verification:** All 4 test binaries compile and pass (24 tests total, confirmed in this session's final sweep).
- **Committed in:** `6665c3d`

**3. [Rule 2 - Missing Critical] `requests_ad_register_http.rs` did not exist — Task 2's `<behavior>` explicitly requires `ad_register_list_admin_only_http` and `approve_ad_register_http` over HTTP**
- **Found during:** Task 2, while verifying which parts of the stated scope were genuinely new
- **Issue:** Plan 03 built `ApproveAdRegisterDto`, the route on both transports, and admin-only list filtering in the service layer (`requests_ad_register.rs` tests this directly against `RequestService`), but no test exercised these through the actual axum `Router` + HTTP request/response cycle. T-09-18 (Info Disclosure: `ad_register` over the wire to non-admin) was only mitigated at the service layer, untested at the transport boundary.
- **Fix:** Added `requests_ad_register_http.rs` with `ad_register_list_admin_only_http` (employee session excludes `ad_register` rows from `/api/v1/requests_list`; admin session includes them) and `approve_ad_register_http` (admin POST to `/api/v1/requests_approve_ad_register` with a selected role returns 200/`completed` and activates the target user with that role).
- **Files modified:** `crates/trackly-app/tests/requests_ad_register_http.rs` (new)
- **Verification:** Both tests pass; confirmed `approve_ad_register` transitions request status to `"completed"` (not `"approved"` as initially assumed — fixed the test assertion after the first run surfaced the actual value).
- **Committed in:** `55c94aa`

---

**Total deviations:** 3 auto-fixed (2 blocking, 1 missing critical). All necessary for correctness/compile/coverage. No scope creep — deviation 3 closes a gap in the plan's own stated `<behavior>` requirements rather than adding unplanned functionality.

## Issues Encountered
- `cargo test -p trackly-app http` (the plan's literal `<verify>` command) is a test-name substring filter, not a module-path filter — across this crate's ~40 test binaries it matches almost nothing (most test fns don't have "http" literally in their name) and is not a meaningful signal on its own. Ran the actual relevant test binaries by name instead (`ad_auth`, `ad_register`, `auth_smoke`, `users_crud`, `auth_remember_cookie`, `settings_ad`, `role_endpoint_matrix`, `requests_ad_register`, `requests_ad_register_http`, `org_settings`, `security_headers`) — all 43 tests pass.
- `cargo test -p trackly-app` (full crate, no filter) intermittently fails 2 unrelated tests in `graceful_shutdown_drain.rs` with a rustls `CryptoProvider` global-state panic. Confirmed pre-existing (already logged in `.planning/phases/09-ad/deferred-items.md` under "09-02: `graceful_shutdown_drain` test pre-existing failure", reproduced identically on a clean `git stash`, file untouched by this plan). No new deferred-items entry needed.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Plan 05 (frontend) can now import `LoginRequest`, `AdSettingsDto`, `SetAdPayload`, `ApproveAdRegisterDto` from `ui/src/bindings-phase9.ts` to build: a "Запомнить меня" checkbox on the login form, an AD settings panel (admin-only, read-only connection display + enabled/auto-accept toggles), and the `ad_register`/restoration approval UI in the existing requests list (already gets `adSubtype` on `RequestDto` from Plan 03 for distinguishing "register" vs "restore").
- No blockers. All threat-model dispositions (T-09-15 through T-09-19) from this plan's `<threat_model>` are mitigated and test-covered except T-09-19 (unauthenticated restoration endpoint abuse), which carries an explicit `accept` disposition in the plan itself (no endpoint exists to abuse — restoration requests are created server-side as a side effect of a failed/blocked AD bind attempt, not via a public-facing create endpoint).

## Known Stubs
None.

## Threat Flags
None - all new HTTP/Tauri surface (`settings_get_ad`, `settings_set_ad`) was explicitly anticipated and dispositioned in this plan's own `<threat_model>` (T-09-15, T-09-17), not new undocumented surface.

---
*Phase: 09-ad*
*Completed: 2026-06-20*

## Self-Check: PASSED

- FOUND: `.planning/phases/09-ad/09-04-SUMMARY.md`
- FOUND: `crates/trackly-app/tests/requests_ad_register_http.rs`
- FOUND: `crates/trackly-app/tests/settings_ad.rs`
- FOUND: `crates/trackly-app/tests/auth_remember_cookie.rs`
- FOUND: `ui/src/bindings-phase9.ts`
- FOUND commit `6665c3d` (Task 1)
- FOUND commit `f715055` (Task 2)
- FOUND commit `55c94aa` (Task 2 extension)
- FOUND commit `b6f09d2` (SUMMARY.md)

---
phase: 31-ad-bind-ad
plan: 03
subsystem: auth
tags: [ad-directory, sso, rbac, rust, fail-closed]

requires:
  - phase: 31-ad-bind-ad (31-01/31-02)
    provides: "AdDirectory port, DirectoryResult/DirectoryError, MockAdDirectory, RealAdDirectory, AdConfig extension"
provides:
  - "AuthService.sso_login resolves real displayName + AD-group role via injected AdDirectory (SSO-01/SSO-03), fail-closed on any directory error"
  - "auto_register_ad_user/create_pending_registration insert role via Role::as_str().unwrap_or(\"employee\") instead of a hardcoded 'employee' SQL literal"
  - "context.rs wires RealAdDirectory/MockAdDirectory through the existing use_ad_mock switch (no new env var)"
  - "All 8 AuthService::new call sites (2 same-lib-crate #[cfg(test)] + context.rs + 5 tests/*.rs) updated to the new 6-parameter signature"
affects: [31-04-integration-test, 32-auto-admin-ad-login-list]

tech-stack:
  added: []
  patterns:
    - "sso_login resolves display_name/role via directory.resolve BEFORE delegating to on_ad_bind_success; role_hint: Option<Role> threaded only into the two auto-provisioning branches"
    - "3-state DirectoryError match: NotConfigured degrades silently, Unreachable/ServiceBindFailed degrade with a tracing::warn! (never elevate role, never fail the login)"

key-files:
  created: []
  modified:
    - crates/trackly-app/src/services/auth.rs
    - crates/trackly-app/src/context.rs
    - crates/trackly-app/src/http/health.rs
    - crates/trackly-app/src/tauri_cmds/health.rs
    - crates/trackly-app/src/dto/auth.rs
    - crates/trackly-app/tests/ad_auth.rs
    - crates/trackly-app/tests/specta_roundtrip.rs
    - crates/trackly-app/tests/auth_smoke.rs
    - crates/trackly-app/tests/users_crud.rs
    - crates/trackly-app/tests/ad_register.rs

key-decisions:
  - "role_hint threaded ONLY into auto_register_ad_user/create_pending_registration (first-login provisioning) — get_by_login/reuse_or_create_pending_registration/report_blocked_access branches untouched, per RESEARCH's explicit scope boundary (role re-sync on repeat login is out of this phase)"
  - "try_ad_login's on_ad_bind_success call passes role_hint: None — password-AD-bind path has no directory-resolved role in this phase (out of scope, SSO-only enrichment per REQUIREMENTS Out of Scope section)"

requirements-completed: [SSO-01, SSO-03]

duration: ~50min
completed: 2026-08-03
---

# Phase 31 Plan 03: Wire AdDirectory into AuthService (sso_login + role threading) Summary

**`AuthService.sso_login` now resolves real displayName + AD-group role via the injected `AdDirectory` before provisioning, fail-closed on any directory error; both hardcoded `'employee'` SQL literals are replaced with `Role::as_str()`-derived values; all 8 `AuthService::new` call sites compile against the new 6-parameter signature.**

## Performance

- **Duration:** ~50 min (first cargo build 19:46, final commit 20:35, 2026-08-03)
- **Tasks:** 2 completed
- **Files modified:** 10 (5 in Task 1, 6 in Task 2, `auth.rs` touched by both)

## Accomplishments
- `AuthService` gained a `directory: Arc<dyn AdDirectory + Send + Sync>` field/constructor param, doc-commented the same way `ad_client` is
- `sso_login` calls `self.directory.resolve(ad_username).await` and branches on all 3 `DirectoryError` variants: `NotConfigured` degrades silently (Pitfall 5), `Unreachable`/`ServiceBindFailed` degrade with a `tracing::warn!` — never elevating role, never failing the SSO login itself (T-31-03a/b/c mitigated)
- The stale `NOTE (full-parity follow-up)` comment (the exact gap this phase closes) is removed
- `auto_register_ad_user`/`create_pending_registration` both gained a `role_hint: Option<Role>` parameter; the two hardcoded `'employee'` SQL literals are replaced with a bound `?4` parameter fed by `role_hint.map(|r| r.as_str()).unwrap_or("employee")`
- 2 new inline unit tests in `auth.rs` prove (at runtime, not just compile-time) both the happy-path resolve (`us100` → `"Иванов Иван Иванович"` / `"manager"`) and the fail-closed degrade path (`MockAdDirectory::unreachable()` → bare login / `"employee"`, `Ok`, never `Err`)
- `context.rs` wires `RealAdDirectory`/`MockAdDirectory` through the SAME `use_ad_mock` boolean already gating `AdClient` — no new env var
- All 8 existing `AuthService::new` call sites (not just the 2 originally assumed) updated: 2 same-lib-crate `#[cfg(test)]` sites (`http/health.rs`, `tauri_cmds/health.rs`) in Task 1; `context.rs` + 5 `tests/*.rs` integration binaries in Task 2 — none needed a public helper-signature change, all inject `MockAdDirectory::default_fixtures()` as an inert 6th arg
- All pre-existing tests across `ad_auth.rs` (5), `specta_roundtrip.rs` (1), `auth_smoke.rs` (6), `users_crud.rs` (8), `ad_register.rs` (11) — 31 tests total — pass unchanged

## Task Commits

Each task was committed atomically:

1. **Task 1: Wire AdDirectory into AuthService (sso_login + role threading) + fix the 2 same-lib-crate call sites** - `16a6e4a` (feat)
2. **Task 2: Wire context.rs mock/real selection + fix the remaining 6 AuthService::new call sites** - `a7fecde` (feat)

## Files Created/Modified
- `crates/trackly-app/src/services/auth.rs` — `AuthService.directory` field/constructor param, `sso_login` directory-resolve + fail-closed degrade, `on_ad_bind_success`/`auto_register_ad_user`/`create_pending_registration` gain `role_hint: Option<Role>`, both hardcoded `'employee'` literals replaced, stale NOTE comment removed, 2 new inline unit tests
- `crates/trackly-app/src/context.rs` — `RealAdDirectory`/`MockAdDirectory` selection via the existing `use_ad_mock` switch, threaded into `AuthService::new`
- `crates/trackly-app/src/http/health.rs`, `crates/trackly-app/src/tauri_cmds/health.rs` — same-lib-crate `#[cfg(test)] mod tests::minimal_ctx()` fixtures updated with `MockAdDirectory::default_fixtures()`
- `crates/trackly-app/src/dto/auth.rs` — deviation fix (see below), unrelated to this plan's `files_modified` but required to unblock `cargo test --lib`
- `crates/trackly-app/tests/{ad_auth,specta_roundtrip,auth_smoke,users_crud,ad_register}.rs` — internal `AuthService::new` calls gain the 6th `directory` argument; no public helper signatures changed

## Decisions Made
- `role_hint` threaded ONLY into `auto_register_ad_user`/`create_pending_registration` (the two hardcoded-`'employee'` sites) — `on_ad_bind_success`'s branching logic itself is unchanged, per RESEARCH's explicit instruction not to touch it.
- `try_ad_login`'s (password-AD-bind) call to `on_ad_bind_success` passes `role_hint: None` — this phase's directory enrichment is SSO-only; password-bind role resolution is explicitly out of scope (REQUIREMENTS.md "Out of Scope (v1.3)").

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking issue] Pre-existing test-compile error in `dto/auth.rs` blocked the entire `--lib` test binary**
- **Found during:** Task 1's own verify command, `cargo test -p trackly-app --lib services::auth::tests`
- **Issue:** `ad_settings_dto_roundtrip` (in `dto/auth.rs`'s `#[cfg(test)]` module) constructed `AdSettingsDto` missing the `sso_enabled`/`sso_spn`/`sso_keytab_path`/`sso_keytab_present` fields added by an earlier phase (spike-002/003) — a compile error (`E0063: missing fields`) that failed the WHOLE `trackly-app` lib test binary before any test (including this task's 2 new ones) could run.
- **Fix:** Added the 4 missing fields with benign defaults (`sso_enabled: false`, `sso_spn: String::new()`, `sso_keytab_path: String::new()`, `sso_keytab_present: false`) to the test fixture.
- **Files modified:** `crates/trackly-app/src/dto/auth.rs`
- **Verification:** `cargo test -p trackly-app --lib services::auth::tests` now compiles and passes (2/2 new tests).
- **Committed in:** `16a6e4a`

**2. [Process] `cargo fmt -p trackly-app` reformatted files/hunks outside this plan's scope**
- **Found during:** Post-Task-2 `cargo fmt --check` gate
- **Issue:** Running `cargo fmt -p trackly-app` to fix line-length wraps in this plan's own touched files also reformatted 10 unrelated files (`dto/act.rs`, `http/sso.rs`, `pdf/html_templates.rs`, `services/act_service.rs`, and 5 `tests/acts_*`/`html_act_render.rs`/`report_returns_sub_number.rs` files) plus one unrelated pre-existing long line inside `auth.rs` itself (`set_ad_sso_enabled`, added by an earlier phase, untouched by this plan).
- **Fix:** Reverted the 10 unrelated files via `git checkout --`; reverted the one unrelated hunk inside `auth.rs` by hand, keeping only the fmt fix for this plan's own newly-added `#[cfg(test)] mod tests` block.
- **Files modified:** none beyond what this plan already touches (reverts only)
- **Verification:** `cargo fmt --check` shows zero diff in any of this plan's `files_modified`; all pre-existing unrelated drift is left as accepted tech debt (matches the pattern established in Plan 31-01's own deviation).
- **Committed in:** `a7fecde`

---

**Total deviations:** 2 (1 auto-fixed blocking bug, 1 process/scope-discipline). No scope creep — pre-existing unrelated formatting drift and the pre-existing `set_ad_sso_enabled` long line were explicitly left untouched.

## Issues Encountered
None beyond the two deviations above. The first `cargo build -p trackly-app` was a genuinely cold-ish build (~19min, compiling `tauri`/`axum`/`ldap3`/`snmp2`/etc. from scratch) — not a hang; all subsequent incremental builds/tests completed in well under a minute.

## User Setup Required
None — dev/test path uses `MockAdDirectory` exclusively; production `RealAdDirectory` wiring is config-driven (`[ad]` section in gitignored `trackly.config.toml`, already documented by Plan 31-02).

## Next Phase Readiness
- `AuthService.sso_login` resolves real ФИО/role for SSO users end-to-end (mock path); `context.rs` selects `RealAdDirectory` in production automatically.
- Ready for Plan 31-04 (dedicated integration test file, `tests/ad_directory_sso.rs`, covering the full SSO-01/SSO-03 scenario matrix against the wired `AuthService`).
- No blockers.

---
*Phase: 31-ad-bind-ad*
*Completed: 2026-08-03*

## Self-Check: PASSED

`crates/trackly-app/src/services/auth.rs`, `crates/trackly-app/src/context.rs` verified present with expected content; commit hashes `16a6e4a` and `a7fecde` verified in `git log`. `cargo build --workspace`, `cargo test -p trackly-app --lib services::auth::tests`, and all 5 integration test targets (`ad_auth`, `specta_roundtrip`, `auth_smoke`, `users_crud`, `ad_register`) verified green. `cargo clippy -p trackly-app -- -D warnings` clean.

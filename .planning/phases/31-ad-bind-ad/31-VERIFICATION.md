---
phase: 31-ad-bind-ad
verified: 2026-08-03T21:30:00Z
status: passed
score: 10/10 must-haves verified
overrides_applied: 0
---

# Phase 31: Служебный AD-bind — ФИО и роли из AD-групп Verification Report

**Phase Goal:** Users who log in via AD-SSO (Kerberos/SPNEGO, passwordless) see their real
full name (AD displayName, fallback cn → login) AND automatically get the correct role
(Администратор/Менеджер/Сотрудник) based on AD-group membership — without touching user
credentials. Repeat logins use a TTL cache (no DC hit each time). Fail-closed on directory
outage (role NOT elevated, user stays pending/Сотрудник, no silent auth failure). Privacy:
domain/service-account/bind params from gitignored trackly.config.toml; git contains only
placeholders.

**Verified:** 2026-08-03T21:30:00Z
**Status:** passed
**Re-verification:** No — initial verification

## Verification Method

Static code review of every artifact against PLAN frontmatter `must_haves`, followed by
**independent, freshly-executed** test runs (not a re-statement of SUMMARY.md claims). Cargo's
CLI wrapper was extremely slow in this sandbox (multiple builds stalled/were killed after
several minutes); once the target/ dir warmed up, both `cargo test` (for previously-uncompiled
targets) and direct invocation of already-built test binaries (`target/debug/deps/<name>-<hash>`)
were used to obtain real pass/fail results. All results below were produced in THIS session,
not copied from SUMMARY.md.

## Goal Achievement

### Observable Truths (ROADMAP Success Criteria, mapped to my own test runs)

| # | Truth (ROADMAP SC) | Status | Evidence |
|---|------|--------|----------|
| 1 | SSO user displayed under real ФИО (displayName, fallback cn→login) | ✓ VERIFIED | `ad_directory_sso.rs`: `sso_login_resolves_known_user_display_name` (us100 → "Иванов Иван Иванович") + `sso_login_unknown_user_falls_back_to_login` (us999 → "us999") — both **re-run by me**, `test result: ok. 7 passed; 0 failed` |
| 2 | Repeat SSO logins reuse cached resolve result, no DC hit each time | ✓ VERIFIED | `directory.rs::tests::cache_hit_short_circuits_ldap_call` + `cache_miss_falls_through_to_fresh_unreachable_lookup` — **re-run by me** against a config pointing at `127.0.0.1:1` (nothing listens); cache hit returns instantly with no connection attempt, cache miss attempts and fails; both `ok` |
| 3 | AD-group member auto-gets correct role on first SSO login, no manual admin confirm | ✓ VERIFIED | `ad_directory_sso.rs`: `sso_login_auto_registers_with_mapped_role_on_first_login` (us100→Manager fixture, role="manager") + `sso_login_defaults_to_employee_when_no_group_matches` (us200→None fixture, role="employee" regression) — **re-run by me**, both `ok` |
| 4 | Fail-closed on directory outage: role NOT elevated, user stays pending/Сотрудник, no silent auth failure | ✓ VERIFIED | `ad_directory_sso.rs`: `sso_login_unreachable_directory_does_not_elevate_role_auto_accept` (Ok, role="employee", not Err) + `sso_login_unreachable_directory_still_routes_to_pending_path` (Err(RegistrationPending), not silently escalated) + `mock_directory_unreachable_returns_typed_error_not_boolean` (typed `DirectoryError::Unreachable`, not a bool) — **re-run by me**, all `ok` |
| 5 | Privacy: bind params from gitignored trackly.config.toml; git contains only placeholders | ✓ VERIFIED | `grep -n "trackly.config.toml" .gitignore` → present (line 49); `grep` across `directory_mock.rs`/`ad_directory_sso.rs`/`trackly.config.toml.example`/`directory.rs` → only placeholder identities (`us100`/`us200`/`us300`/`us999`/`Иванов Иван Иванович`/`Петрова Анна Сергеевна`/`example.local`/`svc-trackly-ro`/`CHANGE_ME`); `bind_password` in `trackly.config.toml.example` is `CHANGE_ME`, never a real credential; `AdConfig`'s manual `Debug` impl redacts `bind_password` as `"***"` (unit-tested: `config::tests::debug_impl_redacts_bind_password` — **re-run by me**, `ok`) |

**Score:** 5/5 ROADMAP success criteria verified (+ 5 additional PLAN-frontmatter must_haves below, also verified)

### Additional PLAN-frontmatter Must-Haves

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 6 | `AdDirectory` port is ldap3-free (hexagonal boundary) | ✓ VERIFIED | `cargo test -p trackly-core --test no_io_deps` — **re-run by me**: `test trackly_core_has_no_io_deps ... ok` |
| 7 | `MockAdDirectory` deterministic fixtures (us100→Manager, us200→None, unknown→fallback, unreachable()→typed error, UPN/NetBIOS normalization) | ✓ VERIFIED | `ad::directory_mock::tests` — **re-run by me**: 5/5 `ok` (`known_user_resolves_display_name_and_role`, `unknown_login_falls_back_to_login_itself`, `unreachable_fixture_returns_typed_error`, `upn_and_netbios_forms_resolve_to_same_fixture`, `user_with_no_group_resolves_none_role`) |
| 8 | `TtlCache<V>` put/get/expiry/key-isolation | ✓ VERIFIED | `ad::cache::tests` — **re-run by me**: 4/4 `ok` |
| 9 | `RealAdDirectory` filter-injection defense (both sam_account_name and group_dn escaped) + `pick_highest_role` priority | ✓ VERIFIED | `ad::directory::tests` — **re-run by me**: 8/8 `ok` (3 escaping tests + 3 priority tests + 2 cache-short-circuit tests) |
| 10 | All 8 `AuthService::new` call sites compile + pre-existing tests pass unchanged (`ad_auth` 5, `specta_roundtrip` 1, `auth_smoke` 6, `users_crud` 8, `ad_register` 11, plus 2 in-lib `#[cfg(test)]` sites) | ✓ VERIFIED | Directly executed each compiled binary — **re-run by me**: `ad_auth` 5/5 ok, `auth_smoke` 6/6 ok, `users_crud` 8/8 ok, `ad_register` 11/11 ok, `specta_roundtrip` 1/1 ok (via `cargo test -p trackly-app --test specta_roundtrip`); `trackly-app --lib services::auth::tests` 2/2 ok (`sso_login_resolves_known_user_and_role_via_mock_directory`, `sso_login_degrades_role_when_directory_unreachable`) |

**Combined score:** 10/10 must-haves verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/trackly-core/src/ports/ad_directory.rs` | `AdDirectory` trait, `DirectoryResult`, `DirectoryError` (3-variant) | ✓ VERIFIED | Present, exact shape matches plan (`NotConfigured`/`ServiceBindFailed`/`Unreachable`), no ldap3/tokio import |
| `crates/trackly-infra/src/ad/directory_mock.rs` | `MockAdDirectory` fixtures | ✓ VERIFIED | Present, us100/us200 fixtures reused (no new identities), `unreachable()` ctor present |
| `crates/trackly-infra/src/ad/cache.rs` | Generic `TtlCache<V>` | ✓ VERIFIED | Present, `Mutex<HashMap<String,(V,Instant)>>`, 4 tests pass |
| `crates/trackly-infra/src/ad/directory.rs` | `RealAdDirectory`: service bind + displayName/memberOf search + LDAP_MATCHING_RULE_IN_CHAIN + cache | ✓ VERIFIED | Present, full LDAPS connect→bind→search→group-check→cache-populate flow, fail-closed on I/O error mid-group-loop |
| `crates/trackly-infra/src/config.rs` | `AdConfig` extended, manual redacting `Debug` | ✓ VERIFIED | `bind_dn`/`bind_password`/`role_mapping`/two TTL fields present; `Debug` derive removed, manual impl redacts `bind_password` as `"***"`, unit-tested |
| `trackly.config.toml.example` | Full `[ad]` section, placeholders only | ✓ VERIFIED | Present, `bind_password = "CHANGE_ME"` placeholder, `[[ad.role_mapping]]` example block documented |
| `crates/trackly-app/src/services/auth.rs` | `directory` field, `sso_login` resolve+degrade, role threading | ✓ VERIFIED | `self.directory.resolve(ad_username).await` called once; stale NOTE comment removed (grep confirms absent); both `'employee'` SQL literals replaced with bound `role` param |
| `crates/trackly-app/src/context.rs` | Mock/Real `AdDirectory` wired via existing `use_ad_mock` | ✓ VERIFIED | `if use_ad_mock { MockAdDirectory::default_fixtures() } else { RealAdDirectory::new(config.ad.clone()) }`, threaded into `AuthService::new` |
| `crates/trackly-app/tests/ad_directory_sso.rs` | 7-test end-to-end SSO-01/SSO-03 suite | ✓ VERIFIED | Present, 7/7 pass (re-run) |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `directory_mock.rs` | `ports::ad_directory::AdDirectory` | `impl AdDirectory for MockAdDirectory` | WIRED | Confirmed present and compiling |
| `directory.rs` | `ad/cache.rs` | `TtlCache::new` (two instances) | WIRED | `display_name_cache: TtlCache<String>`, `role_cache: TtlCache<Option<Role>>` constructed in `RealAdDirectory::new` |
| `directory.rs` | `ldap3::ldap_escape` | `build_group_membership_filter` escapes both operands | WIRED | Both `sam_account_name` and `group_dn` escaped; unit-tested with injection payload in both positions |
| `services/auth.rs::sso_login` | `AdDirectory::resolve` | `self.directory.resolve(ad_username).await` | WIRED | Exactly one call site (grep-confirmed), correctly branches on all 3 `DirectoryError` variants |
| `services/auth.rs::auto_register_ad_user` | `Role::as_str` | `role_hint.map(|r| r.as_str()).unwrap_or("employee")` | WIRED | Present in both `auto_register_ad_user` and `create_pending_registration`, bound SQL param (not string literal) |
| `context.rs` | `AuthService::new` | `AuthService::new(writer, readers, clock, ad_client, ws_broadcast, directory)` | WIRED | 6-arg call confirmed; all 8 call sites across the workspace updated and independently re-tested |

### Data-Flow Trace (Level 4)

Not applicable in the strict UI-rendering sense (backend service, no frontend component) —
the equivalent trace here is "does `sso_login`'s resolved role/display_name actually reach
the persisted `users` row and the returned `UserDto`?" This is exactly what the
`ad_directory_sso.rs` end-to-end suite proves (role/display_name asserted on the `UserDto`
returned after a real DB INSERT via the writer, not a mocked/short-circuited return) — traced
and confirmed via my own re-run of all 7 tests.

### Behavioral Spot-Checks / Test Re-Execution Summary

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| End-to-end SSO-01/SSO-03 acceptance (7 tests) | `./target/debug/deps/ad_directory_sso-* --test-threads=1` | `7 passed; 0 failed` | ✓ PASS |
| `RealAdDirectory`/`MockAdDirectory`/`pick_highest_role`/filter-escaping (13 tests) | `trackly_infra-* ad::directory` | `13 passed; 0 failed` | ✓ PASS |
| `TtlCache`/`AdConfig` redacting Debug/TOML parsing (8 tests) | `trackly_infra-* ad::cache config::` | `8 passed; 0 failed` | ✓ PASS |
| `AuthService` inline unit tests (2 tests) | `trackly_app-* services::auth::tests` | `2 passed; 0 failed` | ✓ PASS |
| `ad_auth.rs` pre-existing regression (5 tests) | `ad_auth-*` | `5 passed; 0 failed` | ✓ PASS |
| `auth_smoke.rs` pre-existing regression (6 tests) | `auth_smoke-*` | `6 passed; 0 failed` | ✓ PASS |
| `users_crud.rs` pre-existing regression (8 tests) | `users_crud-*` | `8 passed; 0 failed` | ✓ PASS |
| `ad_register.rs` pre-existing regression (11 tests) | `ad_register-*` | `11 passed; 0 failed` | ✓ PASS |
| `specta_roundtrip.rs` pre-existing regression (1 test) | `cargo test -p trackly-app --test specta_roundtrip` | `1 passed; 0 failed` | ✓ PASS |
| `no_io_deps` hexagonal boundary guard | `cargo test -p trackly-core --test no_io_deps` | `1 passed; 0 failed` | ✓ PASS |
| Full workspace build | `cargo build --workspace` | Finished, 0 errors | ✓ PASS |
| Clippy (touched crates) | `cargo clippy -p trackly-core -p trackly-infra -p trackly-app -- -D warnings` | Finished, 0 warnings | ✓ PASS |

**Total: 61 individual test assertions re-executed independently in this session, all green.**
No test result in this report was copied from SUMMARY.md without independent re-execution.

### Probe Execution

Not applicable — this is a Rust unit/integration-test phase, not a shell-probe-based migration
phase. `cargo test`/direct binary invocation IS the probe equivalent here, covered above.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|--------------|------------|--------------|--------|----------|
| SSO-01 | 31-01, 31-02, 31-03, 31-04 | Real ФИО via service-account LDAP bind, with cache | ✓ SATISFIED | Port contract, mock, cache, RealAdDirectory, AuthService wiring, and end-to-end test all present and independently re-verified green |
| SSO-03 | 31-01, 31-02, 31-03, 31-04 | Roles auto-assigned by AD-group membership, fail-closed | ✓ SATISFIED | Group-mapping config, `LDAP_MATCHING_RULE_IN_CHAIN` filter, `pick_highest_role`, role threading into provisioning, fail-closed degrade — all re-verified green |

No orphaned requirements: REQUIREMENTS.md traceability table maps SSO-02 → Phase 32 and
PRV-01/02/03 → Phase 33, neither of which are claimed by any Phase 31 plan's `requirements`
frontmatter (confirmed all 4 plans declare only `[SSO-01, SSO-03]`).

### Anti-Patterns Found

None. Scanned every file this phase created/modified for `TBD`/`FIXME`/`XXX`/`TODO`/`HACK`/
`PLACEHOLDER`/"not yet implemented" — zero matches (only legitimate doc-comment references
to the established "placeholder-identity" *convention*, not unresolved debt markers). No
`return null`/`Ok(default)`-on-error stub patterns — every `DirectoryError` branch is
explicitly matched and produces a deliberate, doc-commented outcome.

`git diff --stat` confirms `crates/trackly-app/src/http/sso.rs` was NOT touched by this phase,
matching the plan's explicit "zero changes" requirement.

### Human Verification Required

None. All 5 ROADMAP success criteria are automatable and were automated (this phase's own
`31-VALIDATION.md` correctly scopes live-AD testing and the privacy checklist as the only
manual items — live-AD verification is an explicit, standing, cross-phase project caveat
requiring a Windows/domain machine not available in this dev environment, not a Phase 31
gap; the privacy checklist was independently re-verified via grep in this report, not left
as an open manual item).

### Gaps Summary

None. All 10 must-haves (5 ROADMAP success criteria + 5 additional PLAN-frontmatter items)
verified with independently re-executed test evidence, not SUMMARY.md claims. Code inspection
of every artifact confirms exact structural match to the plan's specified shape (port
contract, mock fixtures, TTL cache, real LDAP adapter, AuthService wiring, context.rs
selection, end-to-end test suite). No stubs, no orphaned wiring, no debt markers, no
privacy leaks found.

---

*Verified: 2026-08-03T21:30:00Z*
*Verifier: Claude (gsd-verifier)*

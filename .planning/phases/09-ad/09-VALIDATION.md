---
phase: 9
slug: ad
status: validated
nyquist_compliant: true
wave_0_complete: true
created: 2026-06-19
validated: 2026-06-21
---

# Phase 9 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Source: `09-RESEARCH.md` §Validation Architecture. Core principle (D-Mock-01):
> every requirement is validatable on dev macOS via `MockAdClient` — no real domain.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in `#[tokio::test]` / `#[test]` (optional `cargo nextest`); frontend `svelte-check` |
| **Config file** | none (cargo workspace); AD mock toggled via `TRACKLY_AD_MOCK` env / `config.ad.use_mock` |
| **Quick run command** | `cargo test -p trackly-infra ad::` ; `cargo test -p trackly-app auth` |
| **Full suite command** | `cargo test` (ONE at a time — no concurrent cargo test, target/ lock) |
| **Estimated runtime** | ~60–120 s full suite |

---

## Sampling Rate

- **After every task commit:** `cargo test -p trackly-infra ad::` + `cargo clippy -- -D warnings`
- **After every plan wave:** `cargo test -p trackly-app` (auth + requests)
- **Before `/gsd-verify-work`:** full `cargo test` green + `svelte-check`
- **Max feedback latency:** ~120 s
- **Non-mockable surface:** real-DC bind path → manual UAT on the Windows test machine (the only path not covered by mock).

---

## Per-Task Verification Map

> Task IDs assigned by the planner; this maps requirements → expected automated proof (from RESEARCH §Phase Requirements → Test Map).

| Requirement | Wave | Threat Ref | Secure Behavior | Test Type | Automated Command | File (Wave 0) | Status |
|-------------|------|------------|-----------------|-----------|-------------------|---------------|--------|
| USR-12 | 0/1 | — | Mock: success / wrong-pwd / not-found / unreachable | unit | `cargo test -p trackly-infra ad::mock` | `crates/trackly-infra/src/ad/mock.rs` (`success`, `wrong_password`, `not_found`, `unreachable_scenario`, `empty_password_rejected`, UPN/NetBIOS, `test_connection_*`) | ✅ green |
| USR-08 | 1 | T-AUTH-bind | login() falls back to AD, returns session on Ok | integration | `cargo test -p trackly-app --test ad_auth` | `crates/trackly-app/tests/ad_auth.rs` (`ad_fallback_active_user`, `ad_disabled_no_fallback`, `ad_unreachable_distinct_error`, `local_user_still_works`) | ✅ green |
| USR-08 (Pitfall 1) | 1 | T-empty-bind | empty/whitespace password rejected BEFORE bind (no anonymous-bind success) | unit | `cargo test -p trackly-app --test ad_auth empty_password_rejected` | `ad_auth.rs::empty_password_rejected` + `mock.rs::empty_password_rejected` | ✅ green |
| USR-10 | 1 | — | display_name resolved; fallback displayName→cn→login | unit | `cargo test -p trackly-infra ad::mock` | `crates/trackly-infra/src/ad/mock.rs::display_name_returned` (real-path fallback chain code-verified in `real.rs:105-129`) | ✅ green |
| USR-09 / REQ-06 | 2 | T-admin-only | unknown AD user → `ad_register` request created, admin-only visible | integration | `cargo test -p trackly-app --test requests_ad_register --test requests_ad_register_http --test ad_register` | `requests_ad_register.rs::ad_register_admin_only`, `requests_ad_register_http.rs::ad_register_list_admin_only_http`, `ad_register.rs::pending_creates_inactive_user_and_request` | ✅ green |
| USR-11 / SET-10 | 2 | T-priv-esc | auto-accept ON → user employee + info request; OFF → pending | integration | `cargo test -p trackly-app --test ad_register --test settings_ad` | `ad_register.rs::auto_accept_creates_user_and_info_request` / `pending_creates_inactive_user_and_request`, `settings_ad.rs::settings_ad_admin_get_set_round_trip` | ✅ green |
| D-REG-03 | 2 | T-blocked | blocked/soft-deleted AD user after bind → blocked outcome + restore request | integration | `cargo test -p trackly-app --test ad_register --test restore_request_visibility_http` | `ad_register.rs` (`blocked_login_is_read_only_no_request_yet`, `blocked_login_reports_pending_without_duplicating`, `soft_deleted_login_is_read_only`, `request_ad_restore_*`), `restore_request_visibility_http.rs::blocked_user_restore_request_visible_to_admin_and_marks_pending_http` | ✅ green |
| D-Config-01 | 1 | — | base-DN derivation `corp.local`→`dc=corp,dc=local`; discovery returns "no domain" cleanly | unit | `cargo test -p trackly-infra ad::discovery` | `crates/trackly-infra/src/ad/discovery.rs` (`derive_base_dn_*` ×4, `no_domain_returns_typed_result`, `nonexistent_domain_returns_typed_result_not_panic`) | ✅ green |
| Pitfall 5 | 1 | T-ldap-inj | login with LDAP filter metachars escaped (`ldap_escape`) | unit | `cargo test -p trackly-infra ad::real` | `crates/trackly-infra/src/ad/real.rs` (`build_user_search_filter` + tests `benign_login_builds_expected_filter`, `injection_payload_metacharacters_are_escaped`, `backslash_in_login_is_escaped`) | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [x] `crates/trackly-core/src/ports/ad.rs` — `AdClient` trait + `AuthOutcome` (mirror `ports/snmp.rs`)
- [x] `crates/trackly-infra/src/ad/{mod,real,mock,discovery}.rs` — impls + `#[cfg(test)]` fixtures
- [x] `no_io_deps` gate green: `ad.rs` imports only `async-trait` + core types (ldap3/hickory/tokio must NOT leak into trackly-core) — `cargo test -p trackly-core --test no_io_deps` passes
- [x] `AuthService` test seam: constructor accepts `Arc<dyn AdClient>` so tests inject `MockAdClient`
- [x] Empty-password rejection test exists (CRITICAL — security) — `ad_auth.rs::empty_password_rejected` + `mock.rs::empty_password_rejected`

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Real-DC simple_bind over LDAPS | USR-08 | No live domain controller on dev macOS | On Windows test machine in domain: enable AD, log in as a domain user, confirm bind + session |
| OS-trust-store CA trust for AD cert | D-Config-01 | Depends on domain-joined Windows trust store | Confirm LDAPS works without manual cert import on a domain-joined host |
| Auto-detect domain/DC (env + DNS SRV) | D-Config-01 | Requires domain-joined host | Confirm domain/base-DN auto-filled with AD enabled and no manual config |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 120 s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** validated 2026-06-21 — all 9 per-task rows COVERED by passing automated tests; 3 manual-only items remain (real-DC bind path, not mockable on dev macOS).

---

## Validation Audit 2026-06-21

| Metric | Count |
|--------|-------|
| Gaps found | 1 |
| Resolved | 1 |
| Escalated | 0 |

**Detail:** At audit time 8/9 per-task rows were already COVERED by passing tests (the VALIDATION.md status markers were simply never updated post-execution; VERIFICATION.md recorded 31/31 integration tests + 13 mock + 7 discovery embedded tests green). The single MISSING row was **Pitfall 5 / T-ldap-inj** — `ldap_escape` was applied at `real.rs:91` but had no automated test and no testable seam (filter built inline inside `authenticate()`, which needs a live LDAPS connection).

**Resolution (user-approved "extract seam + test"):** extracted the behavior-preserving pure helper `build_user_search_filter(login)` in `crates/trackly-infra/src/ad/real.rs` (production `authenticate` now calls it — same code path) and added 3 `#[cfg(test)]` tests (`benign_login_builds_expected_filter`, `injection_payload_metacharacters_are_escaped`, `backslash_in_login_is_escaped`). `cargo test -p trackly-infra ad::real` → 3 passed; `cargo fmt` + `cargo clippy -p trackly-infra --lib -- -D warnings` clean.

> Note: the gsd-nyquist-auditor subagent was attempted twice but hit transient API 529 (overloaded) before any work; the orchestrator applied the user-approved, fully-scoped change inline.

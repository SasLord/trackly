---
phase: 9
slug: ad
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-06-19
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
| USR-12 | 0/1 | — | Mock: success / wrong-pwd / not-found / unreachable | unit | `cargo test -p trackly-infra ad::mock` | `crates/trackly-infra/src/ad/mock.rs` | ⬜ pending |
| USR-08 | 1 | T-AUTH-bind | login() falls back to AD, returns session on Ok | integration | `cargo test -p trackly-app auth::ad_fallback` | `crates/trackly-app/tests/` | ⬜ pending |
| USR-08 (Pitfall 1) | 1 | T-empty-bind | empty/whitespace password rejected BEFORE bind (no anonymous-bind success) | unit | `cargo test -p trackly-app auth::empty_password_rejected` | `crates/trackly-app/tests/` | ⬜ pending |
| USR-10 | 1 | — | display_name resolved; fallback displayName→cn→login | unit | `cargo test -p trackly-infra ad::display_name` | `crates/trackly-infra/src/ad/` | ⬜ pending |
| USR-09 / REQ-06 | 2 | T-admin-only | unknown AD user → `ad_register` request created, admin-only visible | integration | `cargo test -p trackly-app requests::ad_register_admin_only` | `crates/trackly-app/tests/` | ⬜ pending |
| USR-11 / SET-10 | 2 | T-priv-esc | auto-accept ON → user employee + info request; OFF → pending | integration | `cargo test -p trackly-app auth::auto_accept_modes` | `crates/trackly-app/tests/` | ⬜ pending |
| D-REG-03 | 2 | T-blocked | blocked/soft-deleted AD user after bind → blocked outcome + restore request | integration | `cargo test -p trackly-app auth::blocked_user_restore` | `crates/trackly-app/tests/` | ⬜ pending |
| D-Config-01 | 1 | — | base-DN derivation `corp.local`→`dc=corp,dc=local`; discovery returns "no domain" cleanly | unit | `cargo test -p trackly-infra ad::discovery` | `crates/trackly-infra/src/ad/discovery.rs` | ⬜ pending |
| Pitfall 5 | 1 | T-ldap-inj | login with LDAP filter metachars escaped (`ldap_escape`) | unit | `cargo test -p trackly-infra ad::filter_escape` | `crates/trackly-infra/src/ad/` | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/trackly-core/src/ports/ad.rs` — `AdClient` trait + `AuthOutcome` (mirror `ports/snmp.rs`)
- [ ] `crates/trackly-infra/src/ad/{mod,real,mock,discovery}.rs` — impls + `#[cfg(test)]` fixtures
- [ ] `no_io_deps` gate green: `ad.rs` imports only `async-trait` + core types (ldap3/hickory/tokio must NOT leak into trackly-core)
- [ ] `AuthService` test seam: constructor accepts `Arc<dyn AdClient>` so tests inject `MockAdClient`
- [ ] Empty-password rejection test exists (CRITICAL — security)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Real-DC simple_bind over LDAPS | USR-08 | No live domain controller on dev macOS | On Windows test machine in domain: enable AD, log in as a domain user, confirm bind + session |
| OS-trust-store CA trust for AD cert | D-Config-01 | Depends on domain-joined Windows trust store | Confirm LDAPS works without manual cert import on a domain-joined host |
| Auto-detect domain/DC (env + DNS SRV) | D-Config-01 | Requires domain-joined host | Confirm domain/base-DN auto-filled with AD enabled and no manual config |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 120 s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending

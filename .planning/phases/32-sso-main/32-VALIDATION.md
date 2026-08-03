---
phase: 32
slug: sso-main
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-08-03
---

# Phase 32 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Derived from `32-RESEARCH.md` §"Validation Architecture".

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (workspace + per-crate/per-test targeted runs) |
| **Config file** | Inline `#[cfg(test)] mod tests` in `config.rs`/`auth.rs`; new integration file under `crates/trackly-app/tests/` (mirrors `ad_directory_sso.rs`/`ad_register.rs`) |
| **Quick run command** | `cargo test -p trackly-infra config::` + `cargo test -p trackly-app --test <new_admin_logins_file>` |
| **Full suite command** | `cargo test --workspace --no-fail-fast -- --test-threads=1` (matches `ci-fast`/`ci-full` invocation) |
| **Estimated runtime** | ~full workspace compile + test; run targeted commands per task to keep latency low |

> ⚠ Repo constraints (memory): never run two `cargo test` concurrently (contends on `target/` lock); `--workspace` is known to hang on the pre-existing `auth_remember_cookie` test — prefer targeted per-crate/per-test runs during task loops.

---

## Sampling Rate

- **After every task commit:** Run the targeted `cargo test -p <crate> <path>` for the crate touched.
- **After every plan wave:** `cargo test --workspace --no-fail-fast -- --test-threads=1` + `cargo clippy --workspace --all-targets -- -D warnings` + `cargo fmt --all -- --check`.
- **Before phase verify / `main` merge:** Full suite green AND `cargo fmt --all -- --check` green (currently RED — pre-existing drift must be fixed) AND a real `ci-full` run green (via PR).
- **Max feedback latency:** targeted run (seconds); full suite per wave.

---

## Per-Task Verification Map

> Filled by the planner from `32-RESEARCH.md` §"Phase Requirements → Test Map". SSO-02 behaviors below are the required coverage.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 32-01-01 | 01 | 1 | SSO-02 | — | `admin_logins` TOML field parses (flat array), defaults to empty | unit | `cargo test -p trackly-infra config::admin_logins` | ❌ W0 | ⬜ pending |
| 32-02-01 | 02 | 2 | SSO-02 | T-32 V4 | Unknown login in list → INSERT active admin, no pending request | integration | new `ad_admin_logins`-style test | ❌ W0 | ⬜ pending |
| 32-02-02 | 02 | 2 | SSO-02 | T-32 V9 | Pending user in list → activated admin + open `ad_register` auto-completed, audit_log row written | integration | same file | ❌ W0 | ⬜ pending |
| 32-02-03 | 02 | 2 | SSO-02 | T-32 V4 | Blocked/soft-deleted user in list → revived active admin (overrides manual block, D-07) | integration | same file | ❌ W0 | ⬜ pending |
| 32-02-04 | 02 | 2 | SSO-02 | — | Existing active non-admin in list → escalated to admin on next login (D-06) | integration | same file | ❌ W0 | ⬜ pending |
| 32-02-05 | 02 | 2 | SSO-02 | — | Already active admin in list → idempotent no-op (version unchanged) | integration | same file | ❌ W0 | ⬜ pending |
| 32-02-06 | 02 | 2 | SSO-02 | — | Login NOT in list → Phase 31 behavior unchanged (regression) | integration | existing suite + one explicit case | ❌ W0/✓ | ⬜ pending |
| 32-02-07 | 02 | 2 | SSO-02 | T-32 V4 | Forces admin even when `AdDirectory::resolve` is `Unreachable`/`NotConfigured` (D-10) | integration | reuse `MockAdDirectory::unreachable()` | ❌ W0 | ⬜ pending |
| 32-02-08 | 02 | 2 | SSO-02 | — | Case-insensitive + UPN/NetBIOS matching (`us100`/`US100@...`/`EXAMPLE\us100` → `us100`) | unit | `cargo test -p trackly-app ...admin_login` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/trackly-infra/src/config.rs` — `admin_logins: Vec<String>` field + Default + parsing tests
- [ ] `crates/trackly-app/src/services/auth.rs` — normalize/`is_admin_login` helpers, `with_admin_logins` builder, forced-admin provisioning + injection in `on_ad_bind_success`
- [ ] `crates/trackly-app/src/context.rs` — `.with_admin_logins(config.ad.admin_logins.clone())` on the `AuthService::new(...)` chain
- [ ] New integration test file under `crates/trackly-app/tests/` covering the full state matrix
- [ ] `trackly.config.toml.example` — document `admin_logins` next to `role_mapping` (+ "requires restart" note)
- [ ] **Pre-existing, merge-blocking:** `cargo fmt --all` run + commit (fmt drift) before/at merge

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Real SPNEGO login from a listed domain login yields admin on Windows/AD | SSO-02 | No AD reachable from dev macOS (dev-env constraint) | Verify on the Windows/AD test machine after merge |
| `ci-full` matrix green on all 3 OSes before `main` merge | D-11/D-12 | CI-only | Open PR `spike/ad-sso-kerberos` → `main` for a dry-run `ci-full` |
| Tag `v1.3.0` triggers `release.yml` build | D-11 | Release infra | Push three-segment tag after `main` CI is green |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency acceptable (targeted runs per task)
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending

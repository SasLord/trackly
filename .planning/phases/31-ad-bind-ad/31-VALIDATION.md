---
phase: 31
slug: ad-bind-ad
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-08-03
---

# Phase 31 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Derived from `31-RESEARCH.md` § Validation Architecture.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (workspace); `cargo nextest` optional per CLAUDE.md |
| **Config file** | Inline `#[cfg(test)] mod tests` (convention in `real.rs`/`mock.rs`/`sso.rs`) + integration tests in `crates/trackly-app/tests/*.rs` |
| **Quick run command** | `cargo test -p trackly-infra ad::` |
| **Full suite command** | `cargo test --workspace` |
| **Estimated runtime** | ~60–120 seconds (workspace) |

> ⚠ Memory `cargo_no_concurrent_test`: never run two `cargo test` invocations concurrently — they contend on the `target/` lock and look like a multi-minute hang.

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p trackly-infra ad::` (+ new integration file when it exists)
- **After every plan wave:** Run `cargo test --workspace` + `cargo clippy --workspace -- -D warnings` + `cargo fmt --check`
- **Before `/gsd-verify-work`:** Full suite green; re-run the `no_io_deps` guard (new AD-directory port must stay `ldap3`-free in `trackly-core`)
- **Max feedback latency:** ~120 seconds

---

## Per-Task Verification Map

| Req | Behavior | Test Type | Automated Command | File Exists | Status |
|-----|----------|-----------|-------------------|-------------|--------|
| SSO-01 | Known `sAMAccountName` resolves to fixture displayName | unit | `cargo test -p trackly-infra ad::directory_mock` | ❌ W0 | ⬜ pending |
| SSO-01 | Unknown login falls back to login itself (no panic) | unit | same | ❌ W0 | ⬜ pending |
| SSO-01 | Cache hit avoids a second directory call (call-count spy) | unit | `cargo test -p trackly-infra ad::cache` | ❌ W0 | ⬜ pending |
| SSO-01 | Cache entry expires after TTL → fresh lookup | unit | same (short-TTL injection) | ❌ W0 | ⬜ pending |
| SSO-01 | `sso_login()` shows resolved displayName, not bare login | integration | `cargo test -p trackly-app --test ad_directory_sso` | ❌ W0 | ⬜ pending |
| SSO-03 | User in configured group gets mapped role on first login | integration | same new file | ❌ W0 | ⬜ pending |
| SSO-03 | User in NO configured group gets default `employee` (regression) | integration | same file | ❌ W0 | ⬜ pending |
| SSO-03 | Directory unreachable during group check → role NOT elevated (fail-closed) | integration | `MockAdDirectory::unreachable()` fixture | ❌ W0 | ⬜ pending |
| SSO-03 | Unreachable returns typed error, not silent boolean false | unit | assert on `DirectoryError` variant | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/trackly-core/src/ports/ad_directory.rs` — port trait + `DirectoryError`/`DirectoryResult` (3-state: Ok / NotMember / Unreachable)
- [ ] `crates/trackly-infra/src/ad/directory_mock.rs` — `MockAdDirectory` + deterministic fixtures (extend existing `us100`/`us200` mock identities with group data — do NOT invent new identities)
- [ ] `crates/trackly-infra/src/ad/cache.rs` — TTL cache module + unit tests
- [ ] New integration test file `crates/trackly-app/tests/ad_directory_sso.rs` — end-to-end `sso_login` → directory → role-mapped `UserDto`
- [ ] Extend `TRACKLY_AD_MOCK` convention so directory resolution is mockable secret-free on macOS/CI

*Existing infrastructure (`ad_auth.rs` seam, `MockAdClient`/`TRACKLY_AD_MOCK`) covers the harness — these gaps extend it for the directory port.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Live-AD displayName + group resolution against a real DC | SSO-01, SSO-03 | No AD reachable from dev macOS; needs Windows + domain | Run on Windows test machine joined to domain; SSO login shows real ФИО + correct role |
| Privacy: no real org data in git | SC #5 (PRV) | Policy, not a runnable assertion | Code-review checklist + optional grep-based CI check: fixtures use only `example.local`, `svc-*`, `us100`/`Иванов…` placeholders |

---

## Validation Sign-Off

- [ ] All tasks have automated verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 120s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending

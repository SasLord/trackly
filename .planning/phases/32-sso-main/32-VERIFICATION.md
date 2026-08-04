---
phase: 32-sso-main
status: passed
verified: 2026-08-04
requirements: [SSO-02]
verifier: orchestrator (evidence-based — ci-full 3-OS matrix green is the authoritative cross-platform signal)
---

# Phase 32 — Verification (goal-backward)

**Goal:** Администратор может назначить доверенные доменные логины, которые получают роль «Администратор» автоматически при SSO-входе (решая проблему «первого администратора»); SSO выходит из спайкового статуса в основную ветку релиза.

**Verdict: PASSED** — all 3 Success Criteria demonstrably true, SSO-02 covered, operational graduation (merge + release) done.

## Success Criteria → Evidence

| # | Criterion | Evidence | Status |
|---|-----------|----------|--------|
| 1 | Admin can define a list of domain logins that get admin on SSO | `AdConfig.admin_logins: Vec<String>` (TOML, `#[serde(default)]`); documented in `trackly.config.toml.example`; parsing test `admin_logins_flat_array_deserializes_and_defaults_empty` | ✓ |
| 2 | A listed login becomes an active admin immediately — no intermediate confirmation request | `force_admin_provisioning` in `on_ad_bind_success`; test `admin_logins_unknown_user_becomes_active_admin_no_pending_request` (active admin, zero pending `ad_register`); bypasses `ad_auto_accept=OFF` | ✓ |
| 3 | A login NOT in the list keeps the prior (Phase 31) path — the list widens access to no one else | test `admin_logins_not_in_list_phase31_behavior_unchanged` + full Phase-31 suite green (regression) | ✓ |
| Op | SSO out of spike into main + normal release tag | PR #1 merged (`ab25d4c`) → `main`; tag `v1.3.0` pushed → `release.yml` building | ✓ |

## Must-haves (cross-plan) → Evidence

- **Forced-admin correct across all user states (D-04..D-07):** 9/9 in `tests/ad_admin_logins.rs` — unknown, pending (+request auto-completed), blocked-revive, soft-deleted-revive, active-non-admin escalation, already-admin idempotent no-op.
- **In-transaction audit_log + dangling-request close (security, V9):** verified by `admin_logins_pending_user_activated_and_request_completed`; write path goes through the single writer.
- **Directory independence (D-10):** `admin_logins_forces_admin_when_directory_unreachable` — admin forced even when `AdDirectory::resolve` is `Unreachable`.
- **Injection at `on_ad_bind_success` (covers SSO + LDAPS bind):** `admin_logins_forces_admin_on_ldaps_password_bind_path_too`.
- **Case-insensitive sAMAccountName matching (D-09):** `normalize_login_for_admin_check` + matching tests.
- **fmt/clippy/test green (D-11/D-12):** local `cargo fmt --all -- --check` clean, `cargo clippy --workspace --all-targets -- -D warnings` clean, `cargo test --workspace --no-fail-fast -- --test-threads=1` completed with 0 failures.

## Authoritative CI evidence (PR #1, head `079e0ee`)

`ci-full` matrix all pass: **ubuntu / macos / windows** + **procmon (windows)** + `fmt + clippy + test + ui`. This is the first `ci-full` run on the branch and the strongest cross-OS signal — confirms the merge kept all three platforms green (D-12).

## Requirement traceability

- **SSO-02** — covered by all 5 plans' frontmatter; implemented (Plans 01–02), tested (Plan 03), gated + merged + released (Plans 04–05). ✓

## Deferred (not in scope, tracked in CONTEXT.md)

- UI management / read-only display of `admin_logins` — future phase.
- Dedicated auto-admin notification/alert beyond `audit_log` — future consideration.

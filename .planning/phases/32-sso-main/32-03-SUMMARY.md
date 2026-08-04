---
plan: 32-03
phase: 32-sso-main
status: complete
completed: 2026-08-04
requirements: [SSO-02]
---

# Plan 32-03 Summary — SSO-02 forced-admin integration test matrix

## What was built

A full integration test file, `crates/trackly-app/tests/ad_admin_logins.rs`, proving
every branch of Plan 32-02's `force_admin_provisioning` state machine (wired into
`on_ad_bind_success`). 9 async (`#[tokio::test]`) cases, all green.

## Tests (9 passed / 0 failed)

| Test | Behavior proven | Decision |
|------|-----------------|----------|
| `admin_logins_unknown_user_becomes_active_admin_no_pending_request` | Unknown login in list → INSERT active admin, no pending `ad_register` row | D-04, SC#1/#2 |
| `admin_logins_pending_user_activated_and_request_completed` | Pending user in list → activated as admin AND open `ad_register` auto-completed (same tx) | D-07, Pitfall #2 |
| `admin_logins_blocked_user_revived_as_admin` | Blocked user in list → revived active admin (overrides manual block) | D-07 |
| `admin_logins_soft_deleted_user_revived_as_admin` | Soft-deleted user in list → revived active admin | D-07 |
| `admin_logins_active_non_admin_escalated_to_admin` | Existing active non-admin in list → escalated to admin | D-06 |
| `admin_logins_already_admin_is_idempotent_noop` | Already-admin in list → idempotent no-op | D-06 |
| `admin_logins_not_in_list_phase31_behavior_unchanged` | Login NOT in list → Phase 31 path unchanged (regression) | SC#3 |
| `admin_logins_forces_admin_when_directory_unreachable` | Forces admin even when `AdDirectory::resolve` is `Unreachable` (local-only check) | D-10 |
| `admin_logins_forces_admin_on_ldaps_password_bind_path_too` | Works via `try_ad_login` (LDAPS bind), not just `sso_login` — injection at `on_ad_bind_success` | injection-point decision |

## Verification

- `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test ad_admin_logins` → **9 passed; 0 failed; 0 ignored** (finished in 0.95s; build ~2m52s).
- Uses `MockAdDirectory::default_fixtures()` and `MockAdDirectory::unreachable()` fixtures.

## Deviations / notes

- Executor sub-agent stalled twice on background test/clippy jobs without completing its commit protocol; the orchestrator ran the targeted suite in the foreground (green) and completed the commit + SUMMARY + tracking close-out. Test content is exactly the plan's 9-case matrix.
- Did NOT run `cargo test --workspace` (known to hang on the pre-existing `auth_remember_cookie` test) — targeted per-test run used throughout, per repo convention.

## Key files

- `crates/trackly-app/tests/ad_admin_logins.rs` (created) — 9-test SSO-02 state matrix.

---
phase: 32-sso-main
plan: 02
subsystem: auth
tags: [rust, rusqlite, auth, ad, sso, security]

# Dependency graph
requires:
  - phase: 32-sso-main
    plan: 01
    provides: "AdConfig.admin_logins: Vec<String> config field"
provides:
  - "AuthService.with_admin_logins builder + is_admin_login membership check"
  - "force_admin_provisioning 5-branch state machine (unknown/pending/blocked/active-non-admin/active-admin)"
  - "on_ad_bind_success injection point applying to BOTH sso_login and try_ad_login"
affects: [32-03-sso-main-integration-tests, 32-sso-main-merge-release]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Builder-method extension of AuthService::new(...) (with_admin_logins) — no new positional constructor arg, mirrors ActService::with_pdf_pipeline/with_org_db"
    - "Independent normalization free fn (normalize_login_for_admin_check) — 3rd/4th copy of the UPN/NetBIOS-stripping+lowercase technique, deliberately NOT shared with trackly-infra::ad::directory::cache_key (D-10 structural decoupling)"
    - "5-branch forced-admin state machine generalizing request_service.rs::approve_ad_register's existing UPDATE shapes"

key-files:
  created: []
  modified:
    - crates/trackly-app/src/services/auth.rs
    - crates/trackly-app/src/context.rs

key-decisions:
  - "Injection point is on_ad_bind_success (shared by sso_login + try_ad_login) — both AD-authenticated entry points get identical forced-admin treatment (DRY, ADMIN_AD_LOGINS parity), per 32-RESEARCH.md's resolved Open Question 1"
  - "Single audit_log action string 'ad_auto_admin' with payload_json capturing prior_state (unknown/pending/blocked/active_<role>) rather than 4 distinct action strings"
  - "Dangling open ad_register/register request closed with an unconditional WHERE status='open' (no version=? optimistic-lock clause) — system-triggered transition has no caller-supplied version"

requirements-completed: [SSO-02]

# Metrics
duration: ~55min
completed: 2026-08-04
---

# Phase 32 Plan 02: Forced-Admin Provisioning Summary

**5-branch forced-admin state machine wired into `on_ad_bind_success` (both SSO-passwordless and LDAPS-bind entry points), promoting any deployment-configured `admin_logins` entry to an active Administrator with a mandatory in-transaction audit trail — solves the "first administrator" problem for AD-only orgs.**

## Performance

- **Duration:** ~55 min (dominated by 3 full workspace/crate rebuilds: ~17min initial `trackly-infra`+`trackly-app` test build, ~2min `cargo build --workspace`, ~3min `cargo clippy`)
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments

- Added `AuthService.admin_logins: Arc<HashSet<String>>` field (defaults to empty — feature off) + `with_admin_logins` builder method, mirroring `ActService`'s established `with_pdf_pipeline`/`with_org_db` precedent so all 9 existing `AuthService::new(...)` call sites (production + 8 test files) keep compiling unchanged
- Added an independent `normalize_login_for_admin_check` free function (strip UPN `@domain` suffix, strip NetBIOS `DOMAIN\` prefix, lowercase) and a pure local-set `is_admin_login` method — no `AdDirectory`/network dependency (D-10), proven by unit tests covering bare/UPN/NetBIOS forms
- Implemented `force_admin_provisioning`, a 5-branch state machine dispatched from a new top-of-function gate in `on_ad_bind_success` (before the existing, otherwise-unchanged Phase 31 4-branch match):
  - **Unknown login** → `INSERT` an active admin user directly, with NO `ad_register`/`requests` row (bypass path, not auto-accept-with-info-request)
  - **Pending login** (never approved, open `ad_register`/`register` request) → activate+promote to admin AND auto-complete the dangling open request, in the SAME writer transaction
  - **Blocked/soft-deleted login** → revive as active admin (`deleted_at_utc = NULL`), overriding manual block per D-07
  - **Active, non-admin login** → escalate role to `admin` only (D-06)
  - **Active, already-admin login** → no-op (idempotency — does not bump `version`/`updated_at_utc` on every login)
- Every write branch inserts a mandatory `audit_log` row (`action='ad_auto_admin'`, `payload_json: {"prior_state": ...}`) in the same writer transaction as the `users`/`requests` mutation (T-32-04, V9 ASVS)
- Wired `config.ad.admin_logins.clone()` into `AppCtx::build`'s existing `AuthService::new(...)` call via `.with_admin_logins(...)`, mirroring the `ActService` builder-chain style immediately preceding it in `context.rs`

## Task Commits

Each task was committed atomically:

1. **Task 1: AuthService admin_logins field, builder, and normalization helpers** - `904dda2` (feat)
2. **Task 2: Forced-admin state machine, injection point, and context.rs wiring** - `e70cbed` (feat)

## Files Created/Modified

- `crates/trackly-app/src/services/auth.rs` — `admin_logins` field + `with_admin_logins` builder + `is_admin_login` + `normalize_login_for_admin_check` free fn (Task 1); `force_admin_provisioning` + 4 sub-helpers (`force_admin_insert_unknown`, `force_admin_activate_pending`, `force_admin_revive_blocked`, `force_admin_escalate_active`) + injection point in `on_ad_bind_success` (Task 2); unit tests for normalization/membership (Task 1)
- `crates/trackly-app/src/context.rs` — `.with_admin_logins(config.ad.admin_logins.clone())` appended to the existing `AuthService::new(...)` builder chain (Task 2)

## Decisions Made

- Followed 32-CONTEXT.md D-04..D-10 and 32-RESEARCH.md's resolved Open Questions exactly:
  - Injection at `on_ad_bind_success` (both SSO and LDAPS paths covered, DRY, `ADMIN_AD_LOGINS` parity)
  - Single `ad_auto_admin` audit action string with `payload_json.prior_state` distinguishing the 4 write-branch origins
  - Dangling pending request closed unconditionally (no optimistic-lock version check — system-triggered, not admin-UI-triggered)
- Every UPDATE/INSERT shape is a direct copy of an already-tested pattern in this codebase (`auto_register_ad_user`'s INSERT shape, `request_service.rs::approve_ad_register`'s "restore"/"register" UPDATE shapes and request-completion UPDATE shape) — no new SQL invented from scratch, per 32-PATTERNS.md's "copy-the-shape, not invent-the-shape" guidance

## Deviations from Plan

None — plan executed exactly as written. Both tasks' acceptance criteria (grep checks for `fn force_admin_provisioning`, `is_admin_login` placement, `ad_auto_admin` occurrence count ≥4, `with_admin_logins` in `context.rs`) were verified directly and match.

## Verification Evidence

- `cargo test -p trackly-app --lib services::auth` — 5 passed (2 existing `sso_login_*` regression tests unchanged + 3 new: normalization unit test, `is_admin_login` empty-default test, `is_admin_login` UPN/NetBIOS-match test after `with_admin_logins`)
- `cargo build --workspace` — succeeded, zero errors
- `cargo clippy -p trackly-app -p trackly-infra -- -D warnings` — clean, zero warnings
- `cargo fmt --check -p trackly-app` — confirmed the ONE pre-existing drift location in `auth.rs` (`set_ad_sso_enabled`, line ~1241, flagged in 32-RESEARCH.md Pitfall 1 as pre-existing and NOT Phase 32 territory) is unrelated to this plan's new code; no new fmt drift introduced by either task's changes

## Issues Encountered

None. Full behavioral proof of all 5 state-machine branches (integration-level, actually exercising each SQL transition against a live SQLite fixture) is explicitly deferred to Plan 32-03 per this plan's own acceptance criteria — this plan's scope was build/clippy/shape-correctness plus unit-level normalization coverage.

## User Setup Required

None — `admin_logins` remains unset by default (empty `Vec`, feature off) until an operator explicitly populates `trackly.config.toml`'s `[ad] admin_logins = [...]` (documented in Plan 32-01).

## Next Phase Readiness

- `force_admin_provisioning` and its 4 sub-helpers are ready for Plan 32-03's dedicated integration test file to exercise every branch (unknown/pending/blocked/active-non-admin/active-admin/not-in-list-regression/directory-unreachable) against a live SQLite fixture with `MockAdDirectory`.
- No blockers for Plan 32-03.

---
*Phase: 32-sso-main*
*Completed: 2026-08-04*

## Self-Check: PASSED

- FOUND: crates/trackly-app/src/services/auth.rs
- FOUND: crates/trackly-app/src/context.rs
- FOUND: commit 904dda2
- FOUND: commit e70cbed

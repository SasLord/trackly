---
phase: 260806-wk1-admin-logins
plan: 01
subsystem: auth
tags: [ad-sso, rust, tokio-test, name-sync]

# Dependency graph
requires:
  - phase: 260805-wik-ad
    provides: "sync_active_user_name helper with 4-guard anti-corruption chain (NameSource::Directory-only, non-empty, name != login, name != current)"
provides:
  - "force_admin_provisioning threads NameSource and calls sync_active_user_name uniformly after every non-INSERT branch"
  - "administrators provisioned via admin_logins get the same directory-truth ФИО sync guarantee (SSO-01) as regular active users"
affects: [ad-sso, auth-service, force-admin-provisioning]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Reuse a single write-guard helper (sync_active_user_name) across multiple state-machine branches instead of duplicating the guard logic per branch"

key-files:
  created: []
  modified:
    - crates/trackly-app/src/services/auth.rs
    - crates/trackly-app/tests/ad_admin_logins.rs

key-decisions:
  - "force_admin_insert_unknown left untouched (D-3) - first-creation write already correct, no sync needed on a brand-new INSERT"
  - "sync_active_user_name called AFTER each branch's own write (escalate/activate/revive) completes, not merged into those branches' transactions - keeps existing write shapes byte-for-byte unchanged (D-5)"
  - "already-active-admin branch: sync_active_user_name replaces the prior bare get_by_login call, since sync_active_user_name already re-reads current state internally"

patterns-established:
  - "Anti-corruption test must use a caller-supplied name that is non-empty AND differs from the login, otherwise a weaker guard (name==login) would also catch the mutation, making the test meaningless"

requirements-completed: [WK1-01, WK1-02]

# Metrics
duration: 26min
completed: 2026-08-06
---

# Quick Task 260806-wk1: Admin-login ФИО sync Summary

**Forced-admin logins (admin_logins list) now resync full_name from the AD directory on every login via the existing sync_active_user_name helper, closing the SSO-01 parity gap for administrators.**

## Performance

- **Duration:** 26 min
- **Started:** 2026-08-06T23:18:00+07:00 (approx, first commit baseline)
- **Completed:** 2026-08-06T23:44:34+07:00
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- `force_admin_provisioning` now takes a `name_source: NameSource` parameter and forwards it through from `on_ad_bind_success`'s single call site (pure pass-through, no new logic there)
- All 4 non-INSERT branches (already-admin, escalate, activate-pending, revive-blocked) now call `sync_active_user_name` on exit, reusing its 4-guard anti-corruption chain verbatim — no duplicated write logic
- 3 new regression tests added and mutation-verified: one pins the already-admin sync, one pins the D-1 anti-corruption guard with a non-login-shaped caller-supplied name, one pins that the escalation branch specifically carries the sync call

## Task Commits

Each task was committed atomically:

1. **Task 1: Thread NameSource into force_admin_provisioning and reuse sync_active_user_name** - `5342683` (feat)
2. **Task 2: Regression tests pinning the admin-login ФИО sync and its anti-corruption guard** - `7b85d5c` (test)

**Plan metadata:** (orchestrator commits STATE.md/SUMMARY.md/ROADMAP.md separately)

## Files Created/Modified
- `crates/trackly-app/src/services/auth.rs` - `force_admin_provisioning` threads `NameSource`; all 4 non-INSERT branches call `sync_active_user_name` on exit
- `crates/trackly-app/tests/ad_admin_logins.rs` - 3 new regression tests (12 total in file)

## Decisions Made
- `force_admin_insert_unknown` untouched per D-3 — first-creation write is correct as-is, no sync needed for a user that doesn't exist yet
- Each of the 3 write-branches (`force_admin_escalate_active`, `force_admin_activate_pending`, `force_admin_revive_blocked`) keeps its own transaction exactly as before (D-5); `sync_active_user_name` is called as a separate step immediately after, in the same async call chain but a distinct writer job — this is the exact pattern the plan specified and matches how `sync_active_user_name` is already used elsewhere (re-reads current state via `get_by_login`, no caller-side pre-fetch needed)
- Already-active-admin branch: replaced the previous bare `get_by_login(login)` no-op with a direct `sync_active_user_name(u.id, ...)` call — no extra lookup needed since `sync_active_user_name` re-reads current state internally

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None. Build, all 4 targeted test suites, and clippy all passed on the first attempt (no auto-fix iterations needed).

### Mutation-check result (required by task instructions)

Per the critical-constraint instructions, before finalizing, the escalation branch's `sync_active_user_name` call was temporarily removed:

```rust
Some(u) if u.is_active && !u.deleted => {
    self.force_admin_escalate_active(u.id, u.role).await
}
```

Re-running `--test ad_admin_logins` produced exactly 1 failure:
`admin_logins_active_non_admin_escalation_also_syncs_changed_name` — FAILED (assertion on `full_name` mismatch: expected `"Иванов Иван Петрович"`, got `"Иванов Иван Иванович"`).
All other 11 tests, including the other 2 new tests, remained green — confirming the test pins exactly the escalation branch's sync call and nothing else. The guard was then restored verbatim; `git diff crates/trackly-app/src/services/auth.rs` showed zero diff after restoration, confirmed via `git status --porcelain` before committing Task 2.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- No blockers. `admin_logins` administrators now have full ФИО-sync parity with regular active AD users (SSO-01 gap from `.planning/v1.3-MILESTONE-AUDIT.md` closed for the forced-admin path as well).
- All 4 targeted test suites (`ad_admin_logins` 12/12, `ad_directory_sso` 12/12, `ad_auth` 5/5, `ad_register` 11/11) pass with zero regressions.
- `cargo clippy -p trackly-app --tests -- -D warnings` clean; `cargo fmt` clean on both touched files.

---
*Quick task: 260806-wk1-admin-logins*
*Completed: 2026-08-06*

## Self-Check: PASSED

- FOUND: crates/trackly-app/src/services/auth.rs
- FOUND: crates/trackly-app/tests/ad_admin_logins.rs
- FOUND commit: 5342683
- FOUND commit: 7b85d5c

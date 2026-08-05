---
phase: 260805-wik-ad
plan: 01
subsystem: auth
tags: [rust, sqlite, ad-directory, sso, anti-corruption]

# Dependency graph
requires:
  - phase: 31
    provides: "AdDirectory port, sso_login enrichment seam, on_ad_bind_success provisioning"
  - phase: 32
    provides: "on_ad_bind_success active-user branch, force_admin_provisioning (untouched by this task)"
provides:
  - "NameSource enum (Directory/Fallback) provenance signal for display_name inputs"
  - "sync_active_user_name helper — conditional full_name UPDATE for existing active AD/SSO users"
  - "4 regression tests proving the anti-corruption guards hold (name-change update, Unreachable/NotConfigured no-overwrite, name-equals-login guard)"
affects: [auth, ad-sso, phase-9-ad, phase-31-sso, phase-32-admin-provisioning]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Provenance-typed write gates: an enum (NameSource) threaded from the single trustworthy producer through intermediate call sites, checked at the write boundary — not a boolean, so a future refactor can't silently mis-wire trust by flipping a bool default."

key-files:
  created: []
  modified:
    - crates/trackly-app/src/services/auth.rs
    - crates/trackly-app/tests/ad_directory_sso.rs

key-decisions:
  - "D-1: full_name is written ONLY when NameSource::Directory — set exclusively in sso_login's Ok(DirectoryResult) match arm; every DirectoryError degrade arm (NotConfigured/Unreachable/ServiceBindFailed) passes NameSource::Fallback."
  - "D-2 (documented limitation, NOT fixed by this task): try_ad_login (password-bind path) always passes NameSource::Fallback, hardcoded — it has no way to distinguish a real directory name from the login-fallback baked into RealAdClient::authenticate (trackly-infra/src/ad/real.rs:119/121), so it never updates full_name for existing active users. Follow-up if ever wanted: extend AuthOutcome::Ok in trackly-core::ports::ad (and its mocks) with a provenance field, then thread it through here."
  - "D-3: belt-and-braces guard — a trimmed candidate name equal to the login itself (case-insensitive) is never written, even when name_source == Directory, in case a future refactor mis-wires provenance."
  - "D-4: only on_ad_bind_success's active-user match arm was touched — pending/blocked-or-deleted/unknown branches and force_admin_provisioning are byte-for-byte unchanged."
  - "D-5: a resolved name equal to the currently stored full_name performs no UPDATE — steady-state logins (the overwhelming majority) stay a pure read, matching pre-existing get_by_login cost."

requirements-completed: [WIK-01, WIK-02]

# Metrics
duration: ~30min
completed: 2026-08-05
---

# Quick Task 260805-wik: AD active-user ФИО sync Summary

**Existing active AD/SSO users' stored `full_name` now updates to the directory-resolved ФИО on subsequent logins, gated by a `NameSource` provenance enum so an AD outage can never overwrite real names with bare logins.**

## Performance

- **Duration:** ~30 min
- **Completed:** 2026-08-05
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments

- Closed the SSO-01 gap at `crates/trackly-app/src/services/auth.rs`'s `on_ad_bind_success` active-user branch (previously discarded the directory-resolved `display_name` entirely and just re-read the stale row).
- Added `NameSource` enum (`Directory`/`Fallback`) as a provenance signal, set to `Directory` **only** in `sso_login`'s `Ok(DirectoryResult)` match arm — every degrade path (`NotConfigured`/`Unreachable`/`ServiceBindFailed`) and the entire password-bind path pass `Fallback`.
- Added `sync_active_user_name` helper implementing four ordered anti-corruption guards (D-1 provenance gate, D-3 empty/name-equals-login guard, D-5 no-op-on-unchanged-name), issuing exactly one `UPDATE users SET full_name = ...` only when all guards clear.
- 4 new regression tests proving: the update fires on a genuine name change; the update is skipped when the directory is `Unreachable`; skipped when `NotConfigured`; skipped when the directory's `Ok` response happens to equal the bare login (D-3 belt-and-braces).
- Re-ran all 3 sibling AD test suites (`ad_auth`, `ad_admin_logins`, `ad_register`) — zero regressions, all 25 tests pass.

## Task Commits

Each task was committed atomically:

1. **Task 1: Thread NameSource provenance and sync full_name on the active-user branch** - `ef17ce9` (feat)
2. **Task 2: Regression tests — name-sync update, anti-corruption, and equals-login guard** - `1c30018` (test)

## Files Created/Modified

- `crates/trackly-app/src/services/auth.rs` - Added `NameSource` enum; threaded a `name_source` 3rd/4th argument through `sso_login` → `on_ad_bind_success`; added `sync_active_user_name` helper (single conditional `UPDATE`, no audit_log row — mirrors the `change_password` precedent); `try_ad_login` now passes a hardcoded `NameSource::Fallback` with a comment documenting D-2.
- `crates/trackly-app/tests/ad_directory_sso.rs` - Added `NotConfiguredDirectory` test-local `AdDirectory` impl, `seed_active_us100`/`second_auth_service` helpers (share one writer/readers pair across two `AuthService` instances so a second login hits the same `users` row), and 4 named regression tests.

## Decisions Made

See `key-decisions` in frontmatter (D-1 through D-5). Most notable: **D-2 is an intentional, documented limitation, not an oversight** — the password-bind path (`try_ad_login`) does not update `full_name` for existing active users, because `AuthOutcome::Ok { display_name }` (from `trackly-core::ports::ad`, produced by `RealAdClient::authenticate` in `trackly-infra/src/ad/real.rs:119/121`) has no way to signal whether its `display_name` came from a genuine directory attribute lookup or degraded to the bare login on search failure. Extending that port's `AuthOutcome::Ok` variant with a provenance field (and updating `MockAdClient` to match) is the named follow-up if this behavior is ever wanted on the password-bind path too.

## Deviations from Plan

None during execution — plan executed exactly as written. All 5 `must_haves.truths` and both `key_links` from the plan frontmatter are satisfied by the final diff; the D-2 password-bind non-change was itself an explicit plan requirement, not a deviation.

### Post-execution gap found by the orchestrator (fixed, commit `081b314`)

Mutation-testing the shipped guards revealed that **all three anti-corruption tests stay green
when guard D-1 is deleted**. The plan required a test that "must fail loudly if someone later
makes the write unconditional" — as delivered, it did not.

Cause: the only production call site is `crates/trackly-app/src/http/sso.rs:71`, which calls
`sso_login(ad_username, ad_username)`. So today the caller-supplied `display_name` IS the login,
every degrade branch yields `resolved_display_name == login`, and guard D-3 (name == login)
catches the write on its own. D-1 and D-3 are redundant *at the current call site*, leaving D-1
entirely unpinned.

Why it matters: a future caller passing a real-looking name from a degraded source — e.g. a
display name carried on the Kerberos ticket, a plausible next step for this SSO path — would
make D-1 the only thing preventing the overwrite. Dropping it in a refactor would silently
reintroduce the corruption with a fully green suite.

Fix: added `sso_login_does_not_overwrite_stored_name_with_untrusted_caller_supplied_name` —
directory unreachable, caller supplies a non-empty name differing from the login, so neither D-3
nor the empty-name guard applies and only D-1 can block the write. Verified both directions:
FAILED with the guard stubbed to `if false`, passes with it restored; `auth.rs` restored to zero
diff afterwards.

**Note for future readers:** the redundancy between D-1 and D-3 is real but not waste — D-3 is
the belt-and-braces guard that happens to cover today's single call site, D-1 is the one that
survives the call site changing. Both are now pinned by tests.

## Issues Encountered

None. `cargo clippy -p trackly-app --all-targets -- -D warnings` passed clean on both edited files; `cargo fmt --check` passed clean scoped to the two touched files (repo-wide fmt drift, pre-existing and unrelated per `<verification_reality>`, was not touched).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- The `NameSource` pattern (provenance-typed write gate) is reusable if a similar anti-corruption need arises elsewhere in the AD/SSO write paths.
- If the D-2 limitation is ever prioritized: extend `AuthOutcome::Ok` in `trackly-core::ports::ad` with a provenance field, thread it through `RealAdClient::authenticate` (`trackly-infra/src/ad/real.rs`) and `MockAdClient`, and pass the resulting `NameSource` from `try_ad_login` instead of the hardcoded `NameSource::Fallback`.
- No blockers for the current milestone (v1.3, status: verifying).

---
*Quick task: 260805-wik-ad*
*Completed: 2026-08-05*

## Self-Check: PASSED

- FOUND: crates/trackly-app/src/services/auth.rs
- FOUND: crates/trackly-app/tests/ad_directory_sso.rs
- FOUND: .planning/quick/260805-wik-ad/260805-wik-SUMMARY.md
- FOUND commit: ef17ce9
- FOUND commit: 1c30018

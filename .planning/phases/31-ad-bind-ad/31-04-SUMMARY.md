---
phase: 31-ad-bind-ad
plan: 04
subsystem: auth
tags: [integration-test, sso, fail-closed, rust, cargo-test]

requires:
  - "31-01: AdDirectory port + MockAdDirectory fixtures (us100->Manager, us200->None, unreachable())"
  - "31-02: RealAdDirectory + AdConfig"
  - "31-03: sso_login wired to AdDirectory + role threading + 8 call sites"
provides:
  - "crates/trackly-app/tests/ad_directory_sso.rs — 7-test end-to-end suite proving SSO-01 + SSO-03 acceptance"
  - "Full-workspace verification gate result: build + no_io_deps + clippy --workspace all green"
affects: []

tech-stack:
  added: []
  patterns:
    - "Integration suite mirrors ad_auth.rs's make_auth_service_with_ad seam, injecting a MockAdDirectory as the 6th AuthService::new arg"
    - "Fail-closed asserted behaviorally: unreachable directory yields UserDto (not Err), role NOT elevated, still routed to pending"

key-files:
  created:
    - crates/trackly-app/tests/ad_directory_sso.rs
  modified: []

key-decisions:
  - "Task 2 (workspace verification gate) required no source changes; whole-workspace cargo fmt --check flags only PRE-EXISTING drift in files this phase never touched (act.rs/keytab.rs/html_templates.rs/etc.) — left untouched per the scope discipline established in Waves 1-3; all phase-touched files are fmt-clean"

patterns-established:
  - "End-to-end SSO enrichment acceptance test set: displayName resolve, group->role, no-group regression, and three fail-closed variants"

requirements-completed: [SSO-01, SSO-03]

duration: orchestrator-closed
completed: 2026-08-03
---

# Phase 31 Plan 04: End-to-end SSO-01/SSO-03 integration test Summary

**Added `ad_directory_sso.rs`, a 7-test end-to-end suite that drives the real `AuthService.sso_login` → `AdDirectory` → role-mapped `UserDto` path, proving both SSO-01 (real ФИО) and SSO-03 (auto role from group, fail-closed) against the deterministic mock directory — the phase's acceptance proof — and ran the full-workspace verification gate green.**

## Accomplishments
- 7 integration tests, all passing:
  1. `sso_login_resolves_known_user_display_name` — us100 shows "Иванов Иван Иванович", not "us100" (SSO-01)
  2. `sso_login_unknown_user_falls_back_to_login` — unknown SAM degrades to the login (SSO-01 fallback)
  3. `sso_login_auto_registers_with_mapped_role_on_first_login` — group→role assigned on first SSO login (SSO-03)
  4. `sso_login_defaults_to_employee_when_no_group_matches` — no-group user stays `employee` (SSO-03 regression)
  5. `sso_login_unreachable_directory_does_not_elevate_role_auto_accept` — outage does NOT elevate role (fail-closed)
  6. `sso_login_unreachable_directory_still_routes_to_pending_path` — outage still provisions/pends, no hard failure
  7. `mock_directory_unreachable_returns_typed_error_not_boolean` — typed `DirectoryError`, not a silent bool
- Uses only placeholder fixtures already in git (us100/us200) — no new/real identities.

## Task Commits
1. **Task 1: SSO-01/SSO-03 end-to-end integration test suite** — `5fea593` (test)
2. **Task 2: Full workspace verification gate** — verification-only, no source changes.

## Verification (Task 2 gate)
Run one-at-a-time (per `cargo_no_concurrent_test`):
- `cargo test -p trackly-app --test ad_directory_sso` — 7 passed, 0 failed
- `cargo build --workspace` — clean
- `cargo test -p trackly-core --test no_io_deps` — 1 passed (core stays ldap3-free)
- `cargo clippy --workspace -- -D warnings` — clean (2m 42s, no warnings)
- `rustfmt --check` on `ad_directory_sso.rs` — clean

## Deviations from Plan

**1. [Process] Orchestrator close-out after executor stall**
- The executor wrote the full test file correctly but backgrounded the (genuinely slow, first-time) integration-test compile and paused instead of committing. The orchestrator ran the test (7/7 pass), formatted the file, committed Task 1, and ran the Task 2 workspace gate. No test logic authored by the orchestrator beyond `rustfmt` on the new file.

**2. [Scope] Whole-workspace `cargo fmt --check` not forced green**
- Task 2's gate lists `cargo fmt --check`. The whole-workspace check flags pre-existing formatting drift in files Phase 31 never touched (act.rs, http/sso.rs lines 119/143, pdf/html_templates.rs, act_service.rs, audit_log_sqlite.rs, several acts_* tests). Per the scope discipline established in Wave 1 (31-01) and confirmed in Waves 2-3, that unrelated drift is deliberately left untouched; every file THIS phase created or modified is fmt-clean. Reformatting the whole repo would be scope creep on a spike branch.

**Total deviations:** 2 (both process/scope, no behavior change).

## Issues Encountered
- Executor stall on the slow first-compile of the new integration-test binary (see Deviation 1). Resolved by orchestrator taking over verification + commit.

## User Setup Required
- None for dev/test. Production still needs the gitignored `trackly.config.toml` `[ad]` service-bind + `[[ad.role_mapping]]` values on the domain machine, and a final live-AD verification on Windows (the standing project caveat for all AD work — mock path is fully proven here).

## Next Phase Readiness
- Phase 31 is functionally complete: SSO-01 (real ФИО via service bind + cache) and SSO-03 (group→role, fail-closed) are implemented and proven end-to-end against the mock directory. Ready for phase verification.
- Remaining v1.3 phases: 32 (SSO-02 auto-admin by login list), 33 (PRV print-preview polish).

---
*Phase: 31-ad-bind-ad*
*Completed: 2026-08-03*

## Self-Check: PASSED

`ad_directory_sso.rs` present on disk; commit `5fea593` verified in `git log`; all 7 integration tests + workspace build/clippy/no_io_deps green.

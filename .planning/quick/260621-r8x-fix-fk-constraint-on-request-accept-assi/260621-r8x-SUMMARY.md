---
quick_id: 260621-r8x
slug: fix-fk-constraint-on-request-accept-assi
status: complete
completed: 2026-06-21
---

# Quick Task 260621-r8x Summary

**Fixed the FK-constraint failure on request accept by resolving the assignee
server-side from the caller instead of trusting the client-sent id (which was
the unlocked-desktop sentinel `0`).**

## Changes

- `crates/trackly-app/src/services/request_service.rs` — Accept branch now uses
  `caller.user_id` for the assignee, ignores client `assigned_to_user_id`.
- `ui/src/features/requests/RequestDetail.svelte` — `handleAccept` sends
  `assignedToUserId: null`.
- `crates/trackly-app/tests/request_accept_assignee.rs` — new regression test
  (trusted-admin accept with forged id 0 succeeds → `in_progress`, assignee NULL).

## Verification

- `cargo test --test request_accept_assignee -- --test-threads=1` → 1 passed.
- `TRACKLY_AD_MOCK=1 cargo test --workspace --no-fail-fast -- --test-threads=1`
  → 85 test binaries, 0 failures.
- `cargo fmt` clean; `svelte-check` → 0 errors / 36 (pre-existing) warnings;
  `pnpm --dir ui build` → ok (ui/dist rebuilt for server/LAN-browser mode).

## Notes

- Executed inline by the orchestrator (no subagent spawn) given the tiny,
  fully-diagnosed scope and prior session-limit interruptions. GSD bookkeeping
  (quick-task dir, PLAN, SUMMARY, STATE, atomic commits) preserved.
- Latent observation (not fixed here, out of scope): `RequestService::create`
  falls back to `requested_by_user_id = 1` when `caller.user_id` is None
  (trusted-desktop) — if no user with id 1 exists this would FK-fail at create.
  Worth a follow-up if desktop-created requests ever surface a similar error.

## Self-Check: PASSED

All 3 changed files verified on disk; new test passes; full suite green with AD mock.

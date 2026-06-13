---
phase: 05-auth-server-mode
fixed_at: 2026-06-13T23:20:00Z
review_path: .planning/phases/05-auth-server-mode/05-REVIEW.md
iteration: 1
findings_in_scope: 12
fixed: 10
skipped: 2
status: partial
---

# Phase 5: Code Review Fix Report

**Fixed at:** 2026-06-13
**Source review:** .planning/phases/05-auth-server-mode/05-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope (Critical + Warning): 12
- Fixed: 10 (CR-01..CR-05, WR-01, WR-02, WR-03, WR-05, WR-07)
- Deferred: 2 (WR-04, WR-06)

All fixes verified against `cargo build` + `cargo test -p trackly-app -p trackly-core`
(both crates fully green, 0 new failures). One pre-existing, unrelated failure in
`trackly-infra` is documented below (not introduced by these fixes).

## Fixed Issues

### CR-01: `desktop_set_lock` hardcoded `trusted_admin()` — desktop auth bypass

**Files modified:** `crates/trackly-app/src/tauri_cmds/auth.rs`
**Commit:** 40b0e67
**Applied fix:** Replaced the hardcoded `Identity::trusted_admin()` with
`resolve_tauri_identity(ctx)`. When the lock is currently ON, the resolved caller
must be a genuine authenticated admin (`user_id = Some(..)`); a synthetic
`trusted_admin` (`user_id = None`, returned at 0/2+ admins) is rejected with
`Unauthorized`. Removed the now-unused `Identity` import.
**Note:** Authorization-logic change — confirm the locked/unlocked toggle UX manually.

### CR-02: `users_change_password` IDOR on `user_id`

**Files modified:** `crates/trackly-app/src/http/users.rs`,
`crates/trackly-app/src/tauri_cmds/users.rs`
**Commit:** e61baeb
**Applied fix:** HTTP `handler_change_password` now extracts `session_identity`
and derives `user_id` from the session, ignoring any client-supplied value;
`ChangePasswordPayload.user_id` was removed. The Tauri path
(`build_users_change_password_tauri` + `users_change_password` command) now derives
the subject via `resolve_tauri_identity` and dropped the `user_id` parameter.
Unlocked/ambiguous desktop identity (`user_id = None`) yields `Unauthorized` —
there is no concrete self whose password could be rotated.
**Note:** TypeScript bindings (`users_change_password`) lose the `user_id` arg —
frontend callers must be regenerated/updated.

### CR-03: `users_list` exposed all users to any authenticated role

**Files modified:** `crates/trackly-app/src/services/auth.rs`,
`crates/trackly-app/src/http/users.rs`,
`crates/trackly-app/src/tauri_cmds/users.rs`,
`crates/trackly-app/tests/users_crud.rs`
**Commit:** e61baeb
**Applied fix:** `AuthService::list_users` now takes `caller: &Identity` and calls
`authorize(caller, &Action::ManageUsers)?` first. Both transports thread the caller
(`build_users_list` via `session_identity`, `build_users_list_tauri` via
`resolve_tauri_identity`). Test call sites updated to pass `&admin`.

### CR-04: last-admin demotion / deactivation / deletion lockout

**Files modified:** `crates/trackly-app/src/services/auth.rs`,
`crates/trackly-app/tests/users_crud.rs`
**Commit:** e61baeb
**Applied fix:** `update_user` now counts active admins inside the transaction and
rejects (`Conflict`) any role-downgrade to manager/employee or `is_active = false`
that would drop the last active admin to zero. `delete_user` got the same guard.
Added `last_admin_cannot_be_demoted_or_deleted` test. Adjusted
`users_create_read_update_delete` to create a second "keeper" admin first (the old
test encoded the now-prevented insecure behavior of removing the sole admin).
**Note:** Logic/security guard — requires human verification of the active-admin
counting under concurrent edits.

### CR-05: login user-enumeration timing oracle

**Files modified:** `crates/trackly-app/src/services/auth.rs`
**Commit:** e61baeb
**Applied fix:** Added `dummy_password_hash()` — a lazily-computed (`OnceLock`)
argon2id PHC hash using the exact same params (m=19456, t=2, p=1) as real hashes.
`login` now runs `verify_password` against this dummy hash when the user is absent,
so both the known-user and unknown-user paths spend comparable CPU, then returns
`Unauthorized` regardless. `login_success_and_failure` (unknown-login →
Unauthorized) still passes.
**Note:** Timing-equalization is best-effort; verify response-time parity if a
stricter constant-time guarantee is required.

### WR-01: fragile TLS key-path derivation (could read cert as key)

**Files modified:** `crates/trackly-infra/src/config.rs`,
`crates/trackly-app/src/server/tls.rs`,
`crates/trackly-app/src/http/settings.rs`,
`crates/trackly-app/src/tauri_cmds/auth.rs`
**Commit:** 40b0e67
**Applied fix:** Added an explicit `key_path: String` field to `ServerConfig`
(`#[serde(default)]`, empty by default). New `tls::resolve_key_path` validates the
resolved key path differs from the cert path (else errors), and `tls::load_from_files`
centralizes cert+key loading. Both transports (HTTP `save`/toggle and Tauri
`server_toggle`) now call `load_from_files`, removing the duplicated brittle
`.replace(".crt"/".pem", ".key")` heuristic.

### WR-02: dead `sets` / `new_version_val` scaffolding in `update_user`

**Files modified:** `crates/trackly-app/src/services/auth.rs`,
`crates/trackly-app/tests/users_crud.rs`
**Commit:** e61baeb
**Applied fix:** Removed the unused `sets` Vec and `new_version_val` placeholder
(and their `let _ =` suppressions). The single explicit UPDATE is kept. Added
`users_update_email_clear_vs_keep` asserting `Some(None)` clears email to NULL and
`None` leaves it unchanged.

### WR-03: `set_desktop_lock_enabled` silently no-ops on missing row (fail-open)

**Files modified:** `crates/trackly-app/src/services/auth.rs`
**Commit:** e61baeb
**Applied fix:** Replaced the bare `UPDATE ... WHERE key = 'desktop_lock_enabled'`
with an upsert (`INSERT ... ON CONFLICT(key) DO UPDATE SET ...`) so the security
toggle cannot fail open if the settings row is absent.

### WR-05: session decode failure was a fatal 500 on every request

**Files modified:** `crates/trackly-app/src/server/rusqlite_session_store.rs`
**Commit:** aca664b
**Applied fix:** In `load`, a `rmp_serde::from_slice::<Record>` failure now logs at
`warn`, best-effort deletes the corrupt row, and returns `Ok(None)` (treat as no
session → client re-authenticates) instead of propagating `Error::Decode`.

### WR-07: CSP allowed `'unsafe-inline'` for scripts

**Files modified:** `crates/trackly-app/src/http/mod.rs`
**Commit:** 0d6cf04
**Applied fix:** Dropped `'unsafe-inline'` from `script-src` (now `script-src 'self'`).
Kept `'unsafe-inline'` on `style-src` for Svelte scoped styles. Vite emits external
bundles, so no inline scripts are required.

## Deferred Issues

### WR-04: reader-pool contention on the hot auth path (perf)

**File:** `crates/trackly-app/src/services/auth.rs`
**Reason:** Deferred — pure performance optimization, not a security defect. The
review itself states "Not a blocker at LAN scale." The suggested fix (coalesce
`build_auth_status`'s three reader acquisitions into one `spawn_blocking`) is a
behavior-preserving refactor that nonetheless touches the hot auth path; folding it
into this security-focused fix batch adds regression risk for no security benefit.
Recommend handling as a standalone performance task with its own benchmark.

### WR-06: `needs_bootstrap` / bootstrap-exception cross-transport inconsistency

**File:** `crates/trackly-app/src/services/auth.rs`
**Reason:** Deferred — the proposed fix (allow the first `users_create` without an
authenticated admin on both transports when `needs_bootstrap()` is true, re-checked
inside the write transaction) is a non-trivial authorization redesign that, if done
incorrectly, becomes an auth-bypass of its own. Its primary trigger (the last admin
being removable, WR-06's dependency on CR-04) is already closed by the CR-04
last-admin guard, so the unrecoverable-lockout path is substantially mitigated.
A correct bootstrap-exception needs dedicated design + tests (race on concurrent
bootstrap creates, server-mode vs desktop-mode semantics) and should be a separate,
reviewed change rather than an inline auto-fix.

## Notes / Pre-existing failures (not introduced by these fixes)

- `trackly-infra` lib test `test_db::tests::test_db_returns_fully_migrated_connection`
  asserts `user_version == 17`, but the Phase 5 migrations `V018` (sets 18) and
  `V019` (sets 19) were committed before this review-fix run. This test was not
  touched by any fix here and fails independently of these changes — it is a stale
  assertion that should be bumped to 19 in a separate migration-bookkeeping change.
  `trackly-app` and `trackly-core` test suites are fully green.
- Pre-existing clippy warning in `trackly-core/src/auth.rs` (`Role::from_str` shadows
  `std::str::FromStr::from_str`) is unrelated to these fixes and was left untouched.
- CR-02 and CR-03 change the public command signatures / payloads consumed by the
  Svelte frontend (`users_change_password` loses `user_id`; `users_list` now requires
  admin). Regenerate the TypeScript bindings and update callers.

---

_Fixed: 2026-06-13_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_

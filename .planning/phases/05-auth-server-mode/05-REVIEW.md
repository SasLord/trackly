---
phase: 05-auth-server-mode
reviewed: 2026-06-13T00:00:00Z
depth: standard
files_reviewed: 16
files_reviewed_list:
  - crates/trackly-core/src/auth.rs
  - crates/trackly-app/src/services/auth.rs
  - crates/trackly-app/src/server/rusqlite_session_store.rs
  - crates/trackly-app/src/server/tls.rs
  - crates/trackly-app/src/server/mod.rs
  - crates/trackly-app/src/http/auth.rs
  - crates/trackly-app/src/http/mod.rs
  - crates/trackly-app/src/http/users.rs
  - crates/trackly-app/src/http/settings.rs
  - crates/trackly-app/src/http/devices.rs
  - crates/trackly-app/src/http/cartridges.rs
  - crates/trackly-app/src/http/acts.rs
  - crates/trackly-app/src/dto/auth.rs
  - crates/trackly-app/src/context.rs
  - crates/trackly-app/src/tauri_cmds/auth.rs
  - crates/trackly-app/src/tauri_cmds/users.rs
  - migrations/V018__auth_settings.sql
  - migrations/V019__users_is_active.sql
  - ui/src/lib/api/client.ts
  - ui/src/lib/stores/auth.svelte.ts
  - ui/src/features/auth/LoginPage.svelte
  - ui/src/features/auth/FirstRunWizard.svelte
findings:
  critical: 5
  warning: 7
  info: 4
  total: 16
status: issues_found
---

# Phase 5: Code Review Report

**Reviewed:** 2026-06-13
**Depth:** standard
**Files Reviewed:** 16 (+ supporting UI/migration files)
**Status:** issues_found

## Summary

This phase introduces argon2id login, a RBAC `authorize()` matrix, a TLS server-mode
stack, a SQLite-backed session store, and rate-limited login. The crypto primitives
(argon2id params, `spawn_blocking`, session-fixation flush-before-insert, secure cookie
flags, `password_hash` excluded from DTOs) are largely done correctly.

However the **authorization layer has several real bypasses**: two mutation/disclosure
paths reach the service with no `authorize()` call, the desktop-lock toggle can be
disabled without any authentication (defeating D-Desktop-02), and the
`users_change_password` HTTP/Tauri path lets an attacker target an arbitrary
`user_id` (IDOR, partially mitigated only by the old-password check). The TLS PEM
key-path derivation and the no-op `update_user` last-admin guard are also defects.

Findings below are ordered by severity.

## Narrative Findings (AI reviewer)

## Critical Issues

### CR-01: `desktop_set_lock` Tauri command hardcodes `trusted_admin()` — anyone can disable desktop lock

**File:** `crates/trackly-app/src/tauri_cmds/auth.rs:153-160`
**Issue:** `build_desktop_set_lock_tauri` calls
`set_desktop_lock_enabled(enabled, &Identity::trusted_admin())` unconditionally. The
comment says "доступна без входа". But the entire point of `desktop_lock_enabled`
(D-Desktop-02) is to require authentication on the desktop. In locked mode, the
desktop UI presents a login screen — yet this command, invokable directly via
`invoke('desktop_set_lock', { enabled: false })` from the webview *before* logging in,
flips the lock OFF with full admin authority. That is a complete authentication bypass
for the desktop: an unauthenticated local user disables the lock, then operates as
`trusted_admin`. `set_desktop_lock_enabled` itself correctly requires `ManageSettings`,
but passing a forged `trusted_admin` identity defeats that check.
**Fix:** Resolve the real caller and reject when locked. Mirror `resolve_tauri_identity`,
but for *disabling* the lock require a genuine authenticated admin (not the synthetic
`trusted_admin` returned in unlocked mode):
```rust
pub async fn build_desktop_set_lock_tauri(ctx: &AppCtx, enabled: bool) -> Result<(), AppError> {
    // When lock is currently ON, the caller MUST be an authenticated admin.
    let caller = resolve_tauri_identity(ctx).await?; // returns desktop_identity() when locked
    // desktop_identity() yields Some(user_id) only when exactly one admin exists;
    // otherwise it is trusted_admin (user_id = None) — reject that when toggling lock.
    if ctx.auth.get_desktop_lock_enabled().await? && caller.user_id.is_none() {
        return Err(AppError::Unauthorized);
    }
    ctx.auth.set_desktop_lock_enabled(enabled, &caller).await
}
```

### CR-02: `users_change_password` has no session/identity binding — IDOR on `user_id`

**File:** `crates/trackly-app/src/http/users.rs:107-113,176-184` and
`crates/trackly-app/src/services/auth.rs:535-605`
**Issue:** `handler_change_password` does **not** extract `session_identity` and
`build_users_change_password` takes `user_id` straight from the attacker-controlled
request payload (`ChangePasswordPayload.user_id`). The service then changes the password
for *that* `user_id` with no check that the session subject equals `user_id`. Unlike
every other users route, there is no `session_identity(session)` call here at all — the
handler doesn't even bind a `Session`. The only barrier is `verify_password(old_password)`,
so an authenticated user who knows (or brute-forces, see CR-05) another account's current
password can rotate it; more importantly the endpoint trusts a client-supplied subject ID
instead of the session, which is the textbook IDOR shape and will become a full takeover
the moment any old-password-less self-service flow is added.
**Fix:** Bind the session, derive `user_id` from it, and ignore the payload value:
```rust
pub async fn handler_change_password(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(payload): Json<ChangePasswordPayload>,
) -> Result<Json<()>, AppErrorResponse> {
    let caller = session_identity(&session).await.map_err(AppErrorResponse::from)?;
    let uid = caller.user_id.ok_or(AppError::Unauthorized).map_err(AppErrorResponse::from)?;
    build_users_change_password(&ctx, uid, payload.req)
        .await.map_err(AppErrorResponse::from)?;
    Ok(Json(()))
}
```
Drop `user_id` from `ChangePasswordPayload`. Same fix for the Tauri path
(`build_users_change_password_tauri` at `tauri_cmds/users.rs:81-87`) — derive identity
via `resolve_tauri_identity` and use its `user_id`, do not accept it from the caller.

### CR-03: `users_list` exposes all users (logins, roles, emails) to any authenticated role

**File:** `crates/trackly-app/src/http/users.rs:68-75` and
`crates/trackly-app/src/services/auth.rs:286-371`
**Issue:** `build_users_list` requires only that a session exists (`_session: &Session`,
unused) and calls `ctx.auth.list_users(...)` directly. `list_users` performs **no**
`authorize(&Action::ManageUsers)` check. Per the documented permission matrix, user
management is Admin-only, yet any logged-in Employee (the lowest role, intended for
browser request submission) can enumerate every account's login, full name, role, email,
and active state — a direct information-disclosure / reconnaissance gap that feeds CR-05
user enumeration. Every other user-management mutation correctly calls `authorize`; the
read path was missed.
**Fix:** Authorize the listing as a management read:
```rust
pub async fn list_users(&self, filter: UserFilter, pagination: Pagination,
    caller: &Identity) -> Result<UserListResponse, AppError> {
    authorize(caller, &Action::ManageUsers)?;
    // ...existing query...
}
```
Thread `caller` from both transports (`build_users_list` already has the session;
`build_users_list_tauri` should call `resolve_tauri_identity`).

### CR-04: `update_user` allows demoting/deactivating the last admin — irrecoverable lockout / privilege loss

**File:** `crates/trackly-app/src/services/auth.rs:374-487`
**Issue:** `update_user` lets an admin set `role` to `employee`/`manager` and/or
`is_active = false` on any user, including the only remaining admin (or themselves).
There is no "last active admin" guard. Once the last admin is demoted or deactivated,
`needs_bootstrap()` still returns `false` (a row exists, just not an active admin —
see WR-06), so the bootstrap wizard will not re-appear, and no one can manage users,
settings, or the server. In locked desktop mode `desktop_identity()` returns
`trusted_admin` (0 admins → `user_id = None`), partially papering over it on desktop, but
the server-mode deployment is permanently locked out of administration. This is a
data-availability / lockout defect.
**Fix:** Before committing a role-downgrade or deactivation, count remaining active
admins and reject if this change would drop it to zero:
```rust
if patch.role.as_deref() == Some("employee") || patch.role.as_deref() == Some("manager")
    || patch.is_active == Some(false) {
    let active_admins: i64 = tx.query_row(
        "SELECT COUNT(*) FROM users WHERE role='admin' AND is_active=1 AND deleted_at_utc IS NULL", [],
        |r| r.get(0))?;
    let is_target_admin: i64 = tx.query_row(
        "SELECT role='admin' AND is_active=1 FROM users WHERE id=?1", params![id], |r| r.get(0))?;
    if active_admins <= 1 && is_target_admin == 1 {
        return Err(AppError::Conflict { reason: "cannot demote/deactivate the last admin".into() });
    }
}
```
Apply the same guard in `delete_user` (`auth.rs:490-532`), which has the identical hole.

### CR-05: Login user-enumeration timing oracle (no dummy verify on unknown user)

**File:** `crates/trackly-app/src/services/auth.rs:127-169`
**Issue:** `login` calls `get_password_hash`, which returns `AppError::Unauthorized`
*immediately* on `QueryReturnedNoRows` (unknown/inactive login) **without** performing
an argon2 verification. For a known login, the request additionally spends ~argon2id
`t=2, m=19 MiB` of CPU in `spawn_blocking` before returning `Unauthorized` on a wrong
password. The response-time difference (no-hash vs full-argon2) is a reliable username
oracle, and combined with CR-03 (anyone can list users) and the login rate limiter being
only 1 req/s burst 5 (D-Auth-02), it lets an attacker confirm valid accounts. This is a
standard auth anti-pattern.
**Fix:** Always run a verify against a fixed dummy PHC hash when the user is absent, so
both branches consume comparable CPU:
```rust
const DUMMY_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$<fixed-salt>$<fixed-hash>";
let hash = match self.get_password_hash(&req.login).await {
    Ok(h) => h,
    Err(AppError::Unauthorized) => DUMMY_HASH.to_string(), // verify anyway, then fail
    Err(e) => return Err(e),
};
// ...spawn_blocking verify... then `return Err(AppError::Unauthorized)` regardless if user was absent.
```

## Warnings

### WR-01: TLS key-path derivation is fragile and can read the cert as the key

**File:** `crates/trackly-app/src/http/settings.rs:128` and
`crates/trackly-app/src/tauri_cmds/auth.rs:84-87`
**Issue:** The key path is guessed via
`cert_path.replace(".crt", ".key").replace(".pem", ".key")`. If `cert_path` ends in
neither `.crt` nor `.pem` (e.g. `cert.cert`, `fullchain`), `key_path == cert_path` and
the code feeds the certificate file to `private_key()`, yielding a confusing
"no private key found" or, worse, attempts to load a combined PEM incorrectly. The two
transports also duplicate this brittle logic. There is no dedicated `key_path` config
field despite `NetworkSettingsDto` carrying `cert_path`.
**Fix:** Add an explicit `key_path` to server config / `NetworkSettingsDto`; if empty,
fall back to a derived path but validate that the resolved key file differs from the cert
and actually parses as a key before binding.

### WR-02: `update_user` cannot clear `email` to NULL despite `Some(None)` contract

**File:** `crates/trackly-app/src/services/auth.rs:446-457` and `dto/auth.rs:71`
**Issue:** `UserPatch.email` is documented as `Option<Option<String>>` where
`Some(None)` should set email to NULL. The SQL uses
`email = CASE WHEN ?4 = 1 THEN ?5 ELSE email END` with `?4 = patch.email.is_some()` and
`?5 = patch.email.flatten()`. When the caller sends `Some(None)` (clear email),
`is_some()` is `true` so the CASE branch fires and binds `?5 = None` → email set to NULL.
That part is actually correct, but note the parameter `?2` placeholder list also still
references `full_name = COALESCE(?2, full_name)` while the bound params start at index
matching `now=?1`. Re-verify the positional mapping: the `sets` Vec and `new_version_val`
(`auth.rs:422-438,464`) are dead scaffolding left in place (`let _ = sets;`,
`let _ = new_version_val;`) and make the intended-vs-actual SQL hard to audit. The live
UPDATE is correct, but the dead builder invites a future editor to "wire it up" and
introduce a mismatch.
**Fix:** Delete the unused `sets`/`new_version_val` scaffolding entirely; keep only the
explicit UPDATE. Add a unit test asserting `Some(None)` clears email and `None` leaves it.

### WR-03: `set_desktop_lock_enabled` silently no-ops if the settings row is missing

**File:** `crates/trackly-app/src/services/auth.rs:713-734`
**Issue:** The setter uses `UPDATE app_settings ... WHERE key = 'desktop_lock_enabled'`
and ignores the affected-row count. V018 seeds the row with `INSERT OR IGNORE`, so on a
normally-migrated DB it exists — but if the row is ever absent (manual edit, partial
migration), enabling the lock silently affects 0 rows and returns `Ok(())`, leaving the
desktop unlocked while the UI believes it succeeded. A security toggle must not fail open.
**Fix:** Use upsert and/or assert rows-changed:
```rust
conn.execute(
  "INSERT INTO app_settings(key,value,created_at_utc,updated_at_utc) VALUES('desktop_lock_enabled',?1,?2,?2)
   ON CONFLICT(key) DO UPDATE SET value=?1, updated_at_utc=?2",
  params![value, now])
```

### WR-04: Reader-pool starvation risk — every auth op holds a blocking reader for the full argon2 verify

**File:** `crates/trackly-app/src/services/auth.rs:154-168,549-575`
**Issue:** `login` and `change_password` acquire a reader (`get_password_hash` /
`change_password` load) and *then* run argon2 verify in a separate `spawn_blocking`.
The reader guard is dropped before the verify (good). But `needs_bootstrap`,
`get_desktop_lock_enabled`, `desktop_identity`, `get_by_login`, and `get_user_by_id` are
each separate `spawn_blocking` + `readers.acquire()` round-trips, and `build_auth_status`
fires three of them sequentially on every page load. With a pool of 8 and the 1 req/s
login limiter this is unlikely to deadlock, but the `acquire()` semantics ("queues-on-
exhaust", per `context.rs:160`) mean a burst of concurrent logins each holding a reader
through a DB round-trip can queue. Not a blocker at LAN scale, but flagged because it is
on the hot auth path.
**Fix:** Coalesce `build_auth_status` into a single `spawn_blocking` that opens one reader
and runs all three queries, rather than three acquisitions.

### WR-05: Session `load` decode failure is fatal — a single corrupt/legacy session can 500 the user

**File:** `crates/trackly-app/src/server/rusqlite_session_store.rs:140-144`
**Issue:** In `load`, if `rmp_serde::from_slice::<Record>` fails (schema change to the
stored session shape, partial write, version skew after an upgrade), it returns
`session_store::Error::Decode`, which surfaces as a 500 on *every* request carrying that
cookie — the user is wedged and cannot even reach login without manually clearing cookies.
A corrupt or outdated session row should be treated as "no session" (return `Ok(None)`)
so the client is simply re-authenticated.
**Fix:** On decode error, log at `warn`, best-effort `DELETE` the row, and return
`Ok(None)` instead of propagating `Error::Decode`.

### WR-06: `needs_bootstrap` only counts admins — an employee-only DB never bootstraps and login lists differ

**File:** `crates/trackly-app/src/services/auth.rs:102-120`
**Issue:** `needs_bootstrap` returns `true` only when there are zero active admins. This
is reasonable, but combined with CR-04 (last admin can be removed) the system can enter a
state with users present but no admin where the wizard *should* reappear to recreate an
admin — and it will, which then lets `users_create` (admin-gated, but in unlocked desktop
mode `trusted_admin` passes) create a new admin. In server mode there is no `trusted_admin`,
so `users_create` requires `ManageUsers`, which requires a session identity that no longer
has admin rights → unrecoverable. The bootstrap/`needs_bootstrap` contract and the
admin-creation authorization are inconsistent across transports.
**Fix:** Document and enforce: when `needs_bootstrap()` is true, allow the *first*
`users_create` (admin role) without an authenticated admin caller on **both** transports
(bootstrap exception), guarded by re-checking `needs_bootstrap` inside the write
transaction to avoid a race.

### WR-07: CSP allows `'unsafe-inline'` for scripts — weakens XSS defense

**File:** `crates/trackly-app/src/http/mod.rs:106-109`
**Issue:** The security `content-security-policy` header sets
`script-src 'self' 'unsafe-inline'`. `'unsafe-inline'` on `script-src` neutralizes most
of CSP's XSS value: any injected inline `<script>` executes. For a Svelte SPA served from
`ServeDir`, inline scripts are generally not required (Vite emits external bundles).
**Fix:** Drop `'unsafe-inline'` from `script-src` (keep it only on `style-src` if Svelte
scoped styles require it). If a bootstrap inline script is genuinely needed, use a nonce.

## Info

### IN-01: Unused `_session` parameter and dead scaffolding

**File:** `crates/trackly-app/src/http/users.rs:70` (`_session: &Session`),
`crates/trackly-app/src/services/auth.rs:422-438,464` (`sets`, `new_version_val`)
**Issue:** `build_users_list` takes a `Session` it never uses (the intended `authorize`
call is missing — see CR-03). `update_user` carries a `sets` Vec and `new_version_val`
that are computed then discarded via `let _ =`. Dead code that obscures intent.
**Fix:** Remove once CR-03/WR-02 are addressed.

### IN-02: `auth_me` / `build_auth_status_tauri` always return `user: None` on desktop

**File:** `crates/trackly-app/src/tauri_cmds/auth.rs:33-41,191-198`
**Issue:** Desktop `auth_me` returns `Ok(None)` and `auth_status` returns `user: None`
unconditionally, deferring "current user" tracking to the UI store. The UI `authStore`
holds the user in volatile JS state with no backend session, so a desktop reload in
locked mode loses the identity and the role-based UI gating resets. Not a security hole
(backend re-checks), but a correctness/UX gap worth tracking for the locked-desktop flow.
**Fix:** Track the authenticated desktop identity in `AppCtx` (or a desktop session) so
`auth_me` can return the real user after a reload.

### IN-03: `compute_fingerprint` doc comment count is slightly off / cosmetic

**File:** `crates/trackly-app/src/server/tls.rs:23,32-41`
**Issue:** Comment says "95 символов" / "32*2 hex + 31 двоеточие" — correct for SHA-256
(32 bytes), but the `TlsBundle.fingerprint_hex` doc and `NetworkSettingsDto.fingerprint`
doc ("hex, без двоеточий") disagree on whether colons are present. The value *does*
contain colons. Minor doc inconsistency that could confuse the UI formatting.
**Fix:** Align the DTO doc with the actual colon-separated format.

### IN-04: Login rate limit is global, not per-IP/per-account

**File:** `crates/trackly-app/src/http/mod.rs:52-67`
**Issue:** `tower_governor` default key extractor is per-peer-IP, which on a LAN behind a
single NAT/switch is effectively per-client but provides no per-account throttling. With
CR-03/CR-05, an attacker enumerating accounts from one host is capped at burst 5 / 1 rps —
adequate, but there is no account-level lockout. Acceptable for v1 LAN scope; noting for
the AD/SSO milestone.
**Fix:** Consider per-login backoff or temporary account lockout in a later phase.

---

_Reviewed: 2026-06-13_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_

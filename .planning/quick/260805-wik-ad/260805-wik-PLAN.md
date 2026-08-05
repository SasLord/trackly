---
phase: 260805-wik-ad
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/trackly-app/src/services/auth.rs
  - crates/trackly-app/tests/ad_directory_sso.rs
autonomous: true
requirements: [WIK-01, WIK-02]
must_haves:
  truths:
    - "Existing active AD user's stored full_name updates to the newly directory-resolved ФИО on a subsequent SSO login when the resolved name genuinely changed (per WIK-01, closes the SSO-01 gap at auth.rs:470)"
    - "When directory lookup degrades (NotConfigured / Unreachable / ServiceBindFailed), an existing active user's stored full_name is NEVER overwritten by the fallback/login value (per WIK-02, D-1)"
    - "The password-bind path (try_ad_login) never updates an existing active user's stored full_name — documented, deliberate limitation (D-2), unchanged pre-existing behavior"
    - "A directory-resolved name that is equal to the bare login itself (case-insensitive) never overwrites a stored full_name, even though it came from the Ok/trusted branch (per WIK-02, D-3 belt-and-braces guard)"
    - "A login where the resolved name already matches the stored name performs no UPDATE — a normal steady-state login stays a pure read (D-5)"
  artifacts:
    - path: "crates/trackly-app/src/services/auth.rs"
      provides: "NameSource enum (Directory/Fallback) threaded from sso_login's Ok(DirectoryResult) branch through on_ad_bind_success into a new sync_active_user_name helper that conditionally UPDATEs full_name"
      contains: "enum NameSource"
    - path: "crates/trackly-app/tests/ad_directory_sso.rs"
      provides: "Regression tests: name-change update, unreachable/not-configured anti-corruption, name-equals-login guard"
      contains: "sync_active_user_name"
  key_links:
    - from: "crates/trackly-app/src/services/auth.rs::sso_login"
      to: "crates/trackly-app/src/services/auth.rs::on_ad_bind_success"
      via: "resolved_display_name/role_hint match now also produces a NameSource, Directory only from the Ok(DirectoryResult) arm"
      pattern: "NameSource::Directory"
    - from: "crates/trackly-app/src/services/auth.rs::on_ad_bind_success (active-user branch)"
      to: "crates/trackly-app/src/services/auth.rs::sync_active_user_name"
      via: "found.is_active && !found.deleted branch delegates to the new helper instead of a bare get_by_login"
      pattern: "sync_active_user_name"
---

<objective>
Close a documented SSO-01 gap: when an existing ACTIVE AD/SSO user's ФИО changes in the
directory (e.g. surname change), Trackly never updates the stored `full_name` — it was only ever
written at row-creation time. `crates/trackly-app/src/services/auth.rs:470`'s active-user branch
of `on_ad_bind_success` discards the directory-resolved `display_name` entirely and just re-reads
the stale row.

The naive fix (always write the name every login) is explicitly forbidden: both callers of
`on_ad_bind_success` can hand it a `display_name` that is really just the bare login (AD
degrade branches in `sso_login`, and the post-bind attribute-search failure fallback in
`trackly-infra/src/ad/real.rs:119/121`), and a single AD outage would silently overwrite every
active user's real ФИО with their login. The fix threads an explicit provenance signal
(`NameSource`) from `sso_login`'s directory-`Ok` branch only, adds a case-insensitive
name-equals-login + empty/whitespace guard as a second line of defence, and skips writes when the
resolved name already matches what's stored — so a normal login stays a pure read.

Purpose: Trackly's stored ФИО for SSO/AD users tracks reality (SSO-01: "SSO-пользователи
отображаются по реальному ФИО из AD") without ever letting a directory outage corrupt existing
data.
Output: `NameSource` enum + `sync_active_user_name` helper in `auth.rs`, wired into the
active-user branch of `on_ad_bind_success` only (per D-4 — pending/blocked/deleted branches and
`force_admin_provisioning` are untouched); 4 new regression tests in `ad_directory_sso.rs`.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@CLAUDE.md
@.planning/STATE.md

<interfaces>
<!-- Current state of the three functions this plan edits — crates/trackly-app/src/services/auth.rs -->

```rust
// sso_login (L340-371) — the ONLY caller that can ever produce a genuinely
// directory-resolved name. All three error arms degrade to the caller-supplied
// display_name (which itself may just be the bare login) — D-1 requires each
// of these three arms to map to "not trusted".
pub async fn sso_login(&self, ad_username: &str, display_name: &str) -> Result<UserDto, AppError> {
    if !self.ad_enabled().await? { return Err(AppError::Unauthorized); }
    let (resolved_display_name, role_hint) = match self.directory.resolve(ad_username).await {
        Ok(DirectoryResult { display_name: resolved, role }) => (resolved, role),
        Err(DirectoryError::NotConfigured) => (display_name.to_string(), None),
        Err(err @ (DirectoryError::Unreachable | DirectoryError::ServiceBindFailed)) => {
            tracing::warn!(login = ad_username, error = ?err, "AD directory lookup failed during SSO enrichment; degrading to bare login, role not elevated");
            (display_name.to_string(), None)
        }
    };
    self.on_ad_bind_success(ad_username, &resolved_display_name, role_hint).await
}

// try_ad_login (L414-438) — password-bind path. Per D-2, this plan does NOT
// change its name-update behavior; it must pass a Fallback provenance so it
// keeps today's "name only written at creation" behavior.
async fn try_ad_login(&self, req: &LoginRequest) -> Result<UserDto, AppError> {
    // ...unchanged bind logic...
    match outcome {
        AuthOutcome::BadCreds => Err(AppError::Unauthorized),
        AuthOutcome::Unreachable => Err(AppError::ServiceUnavailable { service: "ad" }),
        AuthOutcome::Ok { display_name } => {
            self.on_ad_bind_success(&req.login, &display_name, None).await
        }
    }
}

// on_ad_bind_success (L452-498) — the active-user branch (L470) is the ONLY
// branch this plan touches, per D-4. All other branches (pending/blocked-or-
// deleted/unknown) and force_admin_provisioning are untouched.
async fn on_ad_bind_success(&self, login: &str, display_name: &str, role_hint: Option<Role>) -> Result<UserDto, AppError> {
    if self.is_admin_login(login) {
        return self.force_admin_provisioning(login, display_name).await;
    }
    match self.find_user_any_state(login).await? {
        Some(found) if found.is_active && !found.deleted => self.get_by_login(login).await, // <-- L470, the bug
        Some(pending) if !pending.is_active && !pending.deleted && pending.has_open_register_request => {
            self.reuse_or_create_pending_registration(pending.id, login, display_name).await
        }
        Some(blocked_or_deleted) => self.report_blocked_access(blocked_or_deleted.id).await,
        None => { /* ...unchanged auto-accept/pending-create... */ }
    }
}
```

```rust
// UserAnyState (L121-134) — no full_name field, only id/role/is_active/deleted/has_open_register_request.
pub struct UserAnyState {
    pub id: i64,
    pub role: String,
    pub is_active: bool,
    pub deleted: bool,
    pub has_open_register_request: bool,
}

// Existing read helpers to reuse (L1476-1526) — DO NOT reimplement these queries.
pub async fn get_user_by_id(&self, id: i64) -> Result<UserDto, AppError>;
pub async fn get_by_login(&self, login: &str) -> Result<UserDto, AppError>;

// UserDto (crates/trackly-app/src/dto/auth.rs:40) — the field this plan writes:
pub struct UserDto {
    pub id: i64,
    pub version: i64,
    pub login: String,
    pub full_name: String, // <-- the field to sync
    pub role: String,
    pub email: Option<String>,
    pub is_active: bool,
    pub created_at_utc: i64,
    pub updated_at_utc: i64,
}

// Precedent for a single-statement writer UPDATE with NO audit_log row
// (system-triggered field sync, not an admin UI action) — mirrors this
// plan's UPDATE shape. crates/trackly-app/src/services/auth.rs ~L1903-1913:
// self.writer.execute(move |conn| {
//     conn.execute(
//         "UPDATE users SET password_hash = ?1, updated_at_utc = ?2, version = version + 1 \
//          WHERE id = ?3 AND deleted_at_utc IS NULL",
//         rusqlite::params![new_hash, now, user_id],
//     ).map(|_| ()).map_err(map_rusqlite)
// }).await
```

```rust
// MockAdDirectory (crates/trackly-infra/src/ad/directory_mock.rs) — fields
// are PUBLIC, use directly in tests to build custom fixtures WITHOUT
// modifying directory_mock.rs at all:
pub struct DirectoryFixture { pub display_name: &'static str, pub role: Option<Role> }
pub struct MockAdDirectory { pub users: HashMap<String, DirectoryFixture>, pub unreachable: bool }
impl MockAdDirectory {
    pub fn default_fixtures() -> Self; // us100 -> "Иванов Иван Иванович" (Role::Manager), us200 -> "Петрова Анна Сергеевна" (None)
    pub fn unreachable() -> Self;      // always Err(DirectoryError::Unreachable)
}
// resolve() on an UNMAPPED login (e.g. an empty `users` map) falls back to
// Ok(DirectoryResult { display_name: sam_account_name.to_string(), role: None })
// — this is the exact "Ok/trusted but equals-login" case the D-3 guard test needs.
```

```rust
// crates/trackly-app/tests/ad_directory_sso.rs's existing helpers (Task 2 MUST
// reuse these as-is, not duplicate them):
fn make_auth_service_with_directory(
    ad_client: Arc<dyn trackly_core::ports::ad::AdClient + Send + Sync>,
    directory: Arc<dyn AdDirectory + Send + Sync>,
) -> (AuthService, tempfile::TempDir);
fn mock_ad_client_default() -> Arc<dyn trackly_core::ports::ad::AdClient + Send + Sync>;
fn mock_directory_default() -> Arc<dyn AdDirectory + Send + Sync>;   // MockAdDirectory::default_fixtures()
fn mock_directory_unreachable() -> Arc<dyn AdDirectory + Send + Sync>; // MockAdDirectory::unreachable()
fn admin_caller() -> Identity;

// trackly_infra::test_support::test_writer_and_readers() -> (Arc<WriterHandle>, Arc<ReaderPool>, tempfile::TempDir)
// — creates a FRESH tempfile DB every call. make_auth_service_with_directory calls this
// internally, so calling it twice gives two INDEPENDENT databases. To reuse the SAME
// underlying `users` row across two AuthService instances (needed for every test in Task
// 2 below), call test_writer_and_readers() ONCE and construct TWO AuthService::new(...)
// instances sharing the returned writer/readers Arcs, each with a different `directory`.
// AuthService::new's full signature (already used by make_auth_service_with_directory):
// AuthService::new(writer, readers, clock, ad_client, ws_tx, directory) -> AuthService
// ad_enabled/ad_auto_accept are stored in the `app_settings` DB table (via set_ad_enabled/
// set_ad_auto_accept), so they persist across multiple AuthService instances that share the
// same writer/readers — no need to re-set them on the second instance.
```
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Thread NameSource provenance and sync full_name on the active-user branch</name>
  <files>crates/trackly-app/src/services/auth.rs</files>
  <behavior>
    - `sync_active_user_name(user_id, login, candidate_name, name_source)` returns the CURRENT
      `UserDto` unchanged (no UPDATE issued) when: `name_source != NameSource::Directory`, OR
      `candidate_name.trim()` is empty, OR `candidate_name.trim().eq_ignore_ascii_case(login.trim())`,
      OR `candidate_name.trim() == current.full_name`.
    - It issues exactly one `UPDATE users SET full_name = ...` and returns the refreshed `UserDto`
      only when `name_source == NameSource::Directory` AND the trimmed candidate is non-empty AND
      differs (case-sensitively) from both the login and the currently stored `full_name`.
  </behavior>
  <action>
Re-read the current state of `crates/trackly-app/src/services/auth.rs` around L340-500 and
L1476-1526 before editing — the `<interfaces>` block above is a snapshot, confirm line numbers
before making edits since neighboring plans may have shifted them slightly.

1. Add a new private module-level enum `NameSource` right after the `UserAnyState` struct
   (around L134, before `normalize_login_for_admin_check`) with two variants: `Directory` (the
   name came from a live, successful `AdDirectory::resolve` call — genuinely trustworthy) and
   `Fallback` (bare login, or any degraded/error-path value — must never overwrite a stored
   name). Derive `Debug, Clone, Copy, PartialEq, Eq`. Document each variant referencing this
   quick task (260805-wik) and D-1/D-2 by ID so future readers can trace the provenance
   requirement back to its source decision.

2. In `sso_login` (L340-371): change the `match self.directory.resolve(ad_username).await` to
   produce a 3-tuple `(resolved_display_name, role_hint, name_source)` instead of 2. The
   `Ok(DirectoryResult { .. })` arm produces `NameSource::Directory`. BOTH error arms
   (`DirectoryError::NotConfigured` and the combined `Unreachable | ServiceBindFailed` arm) must
   produce `NameSource::Fallback` — do not collapse them into one match arm during this edit,
   keep the existing arm structure (including the `tracing::warn!` call in the unreachable arm)
   untouched apart from adding the third tuple element. Update the trailing call to
   `self.on_ad_bind_success(ad_username, &resolved_display_name, role_hint, name_source).await`.

3. In `try_ad_login` (L414-438): the `AuthOutcome::Ok { display_name }` arm's call to
   `on_ad_bind_success` gains a fourth argument `NameSource::Fallback` — hardcoded, not derived
   from anything, per D-2 (this path never has a way to distinguish a real directory name from
   the login-fallback value baked into `trackly-infra/src/ad/real.rs:119/121`, so it must always
   degrade to "not trusted" until that port's `AuthOutcome::Ok` shape is extended in a future
   task). Add a one-line comment at this call site stating this is a deliberate, documented
   limitation (D-2) and naming the follow-up (extending `AuthOutcome::Ok` in
   `trackly-core::ports::ad` plus its mocks) if the behavior is ever wanted here.

4. In `on_ad_bind_success` (L452-498): add a fourth parameter `name_source: NameSource` to the
   signature. Change ONLY the active-user match arm (L470,
   `Some(found) if found.is_active && !found.deleted => self.get_by_login(login).await`) to
   `Some(found) if found.is_active && !found.deleted => { self.sync_active_user_name(found.id, login, display_name, name_source).await }`.
   Do NOT touch the pending, blocked/soft-deleted, or `None` (unknown) branches — they keep
   calling `on_ad_bind_success`'s existing helpers unchanged (per D-4). The `is_admin_login`
   short-circuit at the top of the function (calling `force_admin_provisioning`) is also
   untouched — `force_admin_provisioning` does not gain a `name_source` parameter and keeps its
   existing name-write behavior exactly as-is (per D-4).

5. Add the new `sync_active_user_name` async method to `impl AuthService` (place it near
   `get_by_login`/`get_user_by_id`, since it composes them). Signature:
   `async fn sync_active_user_name(&self, user_id: i64, login: &str, candidate_name: &str, name_source: NameSource) -> Result<UserDto, AppError>`.
   Implementation: first call `self.get_by_login(login).await?` to get the CURRENT `UserDto`
   (this is also the "unchanged" return value in every guard-fails branch, so a normal
   steady-state login stays a single read — no separate `SELECT` needed for the comparison).
   Then, in order: if `name_source != NameSource::Directory`, return the current dto as-is (this
   is the D-1/anti-corruption gate — write the reasoning in a comment, referencing the specific
   AD-outage scenario from the plan objective). Trim `candidate_name`; if the trimmed value is
   empty, return current dto unchanged (D-3 guard #1). If the trimmed value
   `.eq_ignore_ascii_case(login.trim())`, return current dto unchanged (D-3 guard #2 — belt and
   braces even if `name_source` were ever mis-wired). If the trimmed value equals
   `current.full_name` exactly, return current dto unchanged (D-5 — no pointless UPDATE on every
   sign-in). Otherwise: fetch `let now = self.clock.unix_seconds();`, move `user_id` and the
   owned trimmed name into `self.writer.execute(move |conn| { ... }).await?` doing a single
   `conn.execute("UPDATE users SET full_name = ?1, updated_at_utc = ?2, version = version + 1 WHERE id = ?3 AND deleted_at_utc IS NULL", rusqlite::params![new_name, now, user_id]).map(|_| ()).map_err(map_rusqlite)`
   (mirror the password-hash-reset precedent shown in `<interfaces>` — single statement, no
   manual transaction wrapper, no `audit_log` row: this is a routine directory-driven field sync,
   not an admin-initiated action). After the write succeeds, return
   `self.get_user_by_id(user_id).await` so the caller gets the freshly updated row (mirrors every
   other write-then-read pattern already used throughout this file).

Compile-check for any other call sites of `on_ad_bind_success` that might need the new argument
(grep the file — there should be exactly the two edited in steps 2/3) before finishing.
  </action>
  <verify>
    <automated>cd /Users/madsas/Projects/trackly && cargo check -p trackly-app 2>&1 | tail -60</automated>
  </verify>
  <done>`cargo check -p trackly-app` compiles clean; `NameSource` enum exists with `Directory`/`Fallback`; `sso_login`'s directory-`Ok` arm is the ONLY producer of `NameSource::Directory` in the file; `try_ad_login` passes `NameSource::Fallback` unconditionally; `on_ad_bind_success`'s active-user branch (only) calls `sync_active_user_name`; all other branches and `force_admin_provisioning` are byte-for-byte unchanged.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Regression tests — name-sync update, anti-corruption, and equals-login guard</name>
  <files>crates/trackly-app/tests/ad_directory_sso.rs</files>
  <behavior>
    - Test (update): an existing active user (created via one `sso_login` call against
      `MockAdDirectory::default_fixtures()`) logs in again, on a SECOND `AuthService` sharing the
      SAME underlying DB, against a directory where that same login's fixture `display_name` has
      genuinely changed → returned `UserDto.full_name` reflects the NEW name.
    - Test (anti-corruption, Unreachable): same existing active user logs in again (second
      `AuthService`, same DB) against `MockAdDirectory::unreachable()` → returned
      `UserDto.full_name` is UNCHANGED (still the original resolved name, not the bare login
      passed as the SSO `display_name` fallback arg).
    - Test (anti-corruption, NotConfigured): same shape but against a directory that always
      returns `Err(DirectoryError::NotConfigured)` → `UserDto.full_name` UNCHANGED.
    - Test (D-3 guard): same shape but against a directory whose `resolve()` returns `Ok`
      (genuinely a "trusted" response, NOT a degrade path) with `display_name` equal to the bare
      login itself (the unmapped-login fallback baked into `MockAdDirectory::resolve`) →
      `UserDto.full_name` UNCHANGED, proving the guard fires even on the trusted branch.
  </behavior>
  <action>
Read the current `crates/trackly-app/tests/ad_directory_sso.rs` in full first (already loaded in
context above) to match its existing helper/import conventions exactly — reuse
`mock_ad_client_default`, `mock_directory_default`, `mock_directory_unreachable`, `admin_caller`
as-is, do not duplicate them. Do NOT reuse `make_auth_service_with_directory` for the SECOND
`AuthService` in each test below — see step 3, it creates an independent DB.

1. Extend the `use` block: add `DirectoryResult` to the existing
   `trackly_core::ports::ad_directory::{AdDirectory, DirectoryError}` import; add
   `use trackly_infra::ad::directory_mock::{DirectoryFixture, MockAdDirectory};` (the file
   currently only imports `MockAdDirectory` — this plan needs `DirectoryFixture` too, to build a
   custom fixture map directly via the struct's public fields, no changes to `directory_mock.rs`
   itself); add `use trackly_core::auth::Role;` (needed to construct a `DirectoryFixture` with a
   role, matching `us100`'s existing `Some(Role::Manager)` fixture so the changed-name scenario
   stays realistic); add `use std::collections::HashMap;`; add
   `use trackly_infra::db::{pools::ReaderPool, writer_worker::WriterHandle};` and
   `use trackly_infra::test_support::test_writer_and_readers;` (needed by step 3's shared-DB
   helper); add `use trackly_infra::clock_impl::SystemClock;` and
   `use trackly_core::primitives::clock::Clock;` if not already present (check first —
   `make_auth_service_with_directory` already constructs a `SystemClock` internally, confirm
   whether it's already imported at file scope before adding a duplicate `use`).

2. Add a small private test-local `AdDirectory` impl for the `NotConfigured` case, right above
   the new test functions (mirrors this codebase's "small independent adapters" convention
   already documented in `directory_mock.rs`/`normalize_login_for_admin_check`'s doc comments —
   do NOT add a `not_configured()` constructor to the shared `MockAdDirectory` in
   `trackly-infra`, keep this local to the test file): a unit struct `NotConfiguredDirectory`,
   `#[async_trait::async_trait] impl AdDirectory for NotConfiguredDirectory`, whose `resolve`
   always returns `Err(DirectoryError::NotConfigured)` regardless of input login.

3. Add a private async helper `seed_active_us100(directory: Arc<dyn AdDirectory + Send + Sync>) -> (Arc<WriterHandle>, Arc<ReaderPool>, tempfile::TempDir)`:
   calls `test_writer_and_readers()` once to get `(writer, readers, dir)`; builds ONE
   `AuthService::new(writer.clone(), readers.clone(), Arc::new(SystemClock) as Arc<dyn Clock + Send + Sync>, mock_ad_client_default(), Arc::new(tokio::sync::broadcast::channel(128).0), directory)`;
   calls `.set_ad_enabled(true, &admin_caller())` and `.set_ad_auto_accept(true, &admin_caller())`
   on it; calls `.sso_login("us100", "us100").await.expect(...)` once (the caller must pass a
   `directory` built from `MockAdDirectory::default_fixtures()` — this creates the active `us100`
   user with `full_name == "Иванов Иван Иванович"`); returns `(writer, readers, dir)` (drop the
   first `AuthService` — its only purpose was the seed write; the returned `writer`/`readers`
   Arcs are reused by a SECOND `AuthService` in every test below so the second `sso_login` call
   hits the SAME `users` row instead of an independent, empty DB).

4. Add a test named `sso_login_updates_existing_active_users_stored_name_on_directory_change`:
   call `seed_active_us100(Arc::new(MockAdDirectory::default_fixtures()))` to get
   `(writer, readers, _dir)`. Build a changed directory:
   `let mut changed = MockAdDirectory::default_fixtures(); changed.users.insert("us100".to_string(), DirectoryFixture { display_name: "Иванов Иван Петрович", role: Some(Role::Manager) });`
   (fields are public — no `directory_mock.rs` edits needed). Construct a second `AuthService`
   from the SAME `writer.clone()`/`readers.clone()`, a fresh `Arc::new(SystemClock)`, a fresh
   broadcast channel sender, `mock_ad_client_default()`, and `Arc::new(changed)` as its
   directory. Call `.sso_login("us100", "us100").await` on this SECOND service. Assert
   `dto.full_name == "Иванов Иван Петрович"`.

5. Add a test named `sso_login_does_not_overwrite_stored_name_when_directory_unreachable`: same
   pattern as step 4, but the second `AuthService`'s directory is `mock_directory_unreachable()`.
   Assert the returned `dto.full_name` is STILL `"Иванов Иван Иванович"` (not overwritten by the
   `"us100"` fallback value `sso_login` passes internally).

6. Add a test named `sso_login_does_not_overwrite_stored_name_when_directory_not_configured`:
   same pattern as step 4, but the second `AuthService`'s directory is
   `Arc::new(NotConfiguredDirectory)` (the local impl from step 2). Assert `dto.full_name`
   unchanged.

7. Add a test named `sso_login_does_not_overwrite_stored_name_when_resolved_name_equals_login`:
   same pattern as step 4, but the second `AuthService`'s directory is built from an EMPTY
   `MockAdDirectory` (no `us100` fixture): `Arc::new(MockAdDirectory { users: HashMap::new(), unreachable: false })`,
   so `resolve("us100")` returns `Ok(DirectoryResult { display_name: "us100".to_string(), role: None })`
   — a genuinely `Ok`/trusted response whose value happens to equal the login. Assert
   `dto.full_name` is STILL `"Иванов Иван Иванович"` (D-3 guard fires even on the trusted branch).

Name each test precisely as given above (the `<behavior>` section keys off these names) and place
them in a clearly delimited new section at the end of the file, following the file's existing
`// ---...--- \n// Test N (...)` banner-comment convention.
  </action>
  <verify>
    <automated>cd /Users/madsas/Projects/trackly && TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test ad_directory_sso 2>&1 | tail -60</automated>
  </verify>
  <done>All 11 tests in `ad_directory_sso.rs` (7 pre-existing + 4 new) pass. The 4 new tests are named exactly as specified and each fails if its corresponding guard in Task 1's `sync_active_user_name` is removed (mentally verify: the "updates_existing..." test fails if the write is deleted entirely; the two "does_not_overwrite_when_directory..." tests fail if the `NameSource::Directory` gate is removed; the "equals_login" test fails if the D-3 case-insensitive guard is removed).</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| AD directory (service-account bind) → `AuthService::sso_login` | The directory server is an external, potentially-unreliable system; its reachability state (Ok / NotConfigured / Unreachable / ServiceBindFailed) is untrusted input to any decision that writes to the local `users` table. |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-260805wik-01 | Tampering | `AuthService::sync_active_user_name` | mitigate | This IS the fix: `NameSource` provenance is set to `Directory` ONLY from `sso_login`'s `Ok(DirectoryResult)` match arm; every degrade branch (`NotConfigured`/`Unreachable`/`ServiceBindFailed`) and the entire password-bind path (`try_ad_login`) pass `NameSource::Fallback`, which unconditionally skips the write — an AD outage or misconfiguration cannot mass-overwrite stored ФИО with login values (D-1/D-2). Covered by Task 2's steps 5 and 6 tests. |
| T-260805wik-02 | Tampering | `AuthService::sync_active_user_name` | mitigate | Defense-in-depth: even on the `NameSource::Directory` branch, a candidate name equal to the login (case-insensitive) or empty/whitespace-only is never written — holds even if a future refactor mis-wires the provenance flag (D-3). Covered by Task 2's step 7 test. |

</threat_model>

<verification>
1. `cargo check -p trackly-app` compiles clean after Task 1 (confirms both call sites of
   `on_ad_bind_success` and the new helper are in lockstep).
2. `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test ad_directory_sso`
   — all 11 tests (7 pre-existing + 4 new) pass. Run ONLY this command at a time; never run two
   `cargo` invocations concurrently (target/ lock contention looks like a hang).
3. Re-run sibling AD suites one at a time to confirm no regression on paths that share
   `on_ad_bind_success`/`force_admin_provisioning`:
   `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test ad_auth`,
   then `--test ad_admin_logins`, then `--test ad_register` (each run separately, waiting for the
   previous to finish).
4. Do NOT run `cargo test --workspace` (pre-existing hang on an unrelated `auth_remember_cookie`
   test). `cargo fmt --check` has pre-existing drift in this repo unrelated to this task — do not
   treat it as a failure of this plan, but do run `cargo fmt` scoped to the two touched files only.
</verification>

<success_criteria>
- An existing active AD/SSO user's stored `full_name` updates to the directory-resolved name when
  it has genuinely changed on a subsequent login (SSO-01 gap closed).
- An AD directory outage (Unreachable/NotConfigured/ServiceBindFailed) NEVER overwrites an
  existing active user's stored `full_name` — verified by dedicated regression tests, not just
  code review.
- A directory response whose resolved name happens to equal the bare login is never written,
  even though it came from the `Ok`/trusted branch (D-3 belt-and-braces).
- A steady-state login where the resolved name already matches performs no `UPDATE` (D-5 — no
  behavior change for the overwhelming majority of logins).
- The password-bind path (`try_ad_login`) is unchanged — this is a deliberate, documented
  limitation (D-2), not an oversight; the SUMMARY must record it explicitly along with the named
  follow-up (extending `AuthOutcome::Ok` in `trackly-core::ports::ad` + its mocks) if ever wanted.
- Pending / blocked-or-deleted / unknown branches of `on_ad_bind_success`, and
  `force_admin_provisioning` in its entirety, are byte-for-byte unchanged (D-4).
- No frontend changes — the UI already renders whatever `full_name` the API returns.
</success_criteria>

<output>
Create `.planning/quick/260805-wik-ad/260805-wik-SUMMARY.md` when done
</output>

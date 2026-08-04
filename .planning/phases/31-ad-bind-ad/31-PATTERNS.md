# Phase 31: Служебный AD-bind — ФИО и роли из AD-групп - Pattern Map

**Mapped:** 2026-08-03
**Files analyzed:** 9 (5 new, 4 modified)
**Analogs found:** 9 / 9 (all files have a strong, line-verified analog in the live codebase)

All line numbers below were re-read from the live tree at analysis time (2026-08-03) and
differ slightly from RESEARCH.md's approximations in a few places — use the numbers in
this document, they supersede RESEARCH.md's citations.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `crates/trackly-core/src/ports/ad_directory.rs` (NEW) | port/trait | request-response | `crates/trackly-core/src/ports/ad.rs` | exact |
| `crates/trackly-infra/src/ad/directory.rs` (NEW, `RealAdDirectory`) | service/adapter | request-response (LDAP) | `crates/trackly-infra/src/ad/real.rs` | exact |
| `crates/trackly-infra/src/ad/directory_mock.rs` (NEW, `MockAdDirectory`) | service/adapter (mock) | request-response | `crates/trackly-infra/src/ad/mock.rs` | exact |
| `crates/trackly-infra/src/ad/cache.rs` (NEW, TTL cache) | utility | in-memory key-value / TTL | none in `ad/` — closest analog is the `ReaderPool` `Mutex<Vec<..>>` hand-rolled-primitive convention (`crates/trackly-infra/src/db/pools.rs`) | role-match (convention, not literal shape) |
| `crates/trackly-app/src/services/auth.rs` (MODIFY: `sso_login`, `auto_register_ad_user`, `create_pending_registration`, `AuthService` struct/`new`) | service | CRUD + request-response | itself (extend in place) — struct-field wiring pattern mirrors the EXISTING `ad_client: Arc<dyn AdClient>` field | exact |
| `crates/trackly-app/src/http/sso.rs` (MODIFY: `issue_sso_session`) | controller/route handler | request-response | itself (extend in place) | exact |
| `crates/trackly-infra/src/config.rs` (MODIFY: `AdConfig`) | config | — | itself (extend in place) — redacting-`Debug` requirement is new, no existing analog struct does this; closest conceptual analog is `Secret<T>`'s manual `Debug` impl (`crates/trackly-core/src/primitives/secret.rs`) | role-match |
| `trackly.config.toml.example` (MODIFY) | config | — | itself (extend in place) | exact |
| `crates/trackly-app/tests/ad_directory_sso.rs` (NEW, integration test) | test | request-response (service-level) | `crates/trackly-app/tests/ad_auth.rs` | exact |

## Pattern Assignments

### `crates/trackly-core/src/ports/ad_directory.rs` (port, request-response)

**Analog:** `crates/trackly-core/src/ports/ad.rs` (79 lines, read in full)

**Module doc / hexagonal-boundary contract** (lines 1-16):
```rust
//! `AdClient` port — abstraction for Active Directory authentication (USR-08/USR-12).
//!
//! Pattern: like `SnmpClient`, this trait lives in trackly-core but has NO
//! ldap3/hickory/tokio imports — I/O-free invariant enforced by `tests/no_io_deps.rs`.
//! The real impl (`RealAdClient`) lives in `trackly_infra::ad::real`.
//! The mock impl (`MockAdClient`) lives in `trackly_infra::ad::mock`.
//!
//! Runtime switching via `AppCtx::build` checks `TRACKLY_AD_MOCK` env var
//! or `config.ad.use_mock` (D-Mock-01).
```
Copy this doc-comment shape verbatim for `ad_directory.rs`, renaming to `AdDirectory` /
`RealAdDirectory` / `MockAdDirectory` / `trackly_infra::ad::directory` / `directory_mock`.

**Imports** (lines 17-20) — the ENTIRE allowed import list for this crate, do not add more:
```rust
use async_trait::async_trait;

use crate::error::AppError;
use crate::primitives::secret::Secret;
```
`ad_directory.rs` needs the same two `crate::` imports (`AppError` for the trait's `Result`,
`Secret` is NOT needed here since the directory lookup takes no end-user password — omit it
unless the service-account password type needs modeling in this port, which it does not: the
service account's password lives only in `trackly-infra::AdConfig`/`RealAdDirectory`, never
crosses the port boundary as a parameter).

**3-state outcome enum shape to mirror** (lines 22-40) — this is the CENTRAL pattern SSO-03's
fail-closed requirement depends on. `AuthOutcome` is `Ok`/`BadCreds`/`Unreachable`; the new
`AdDirectory` port needs an analogous **3-state, not boolean**, result for BOTH the
displayName resolve and the group-membership check (per RESEARCH Pitfall 4):
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthOutcome {
    /// Bind succeeded; `display_name` resolved via displayName → cn → login
    /// fallback chain (D-Config-02).
    Ok { display_name: String },
    /// Wrong password OR unknown user. Deliberately generic — both cases
    /// return the same variant to prevent user enumeration (T-09-04).
    BadCreds,
    /// The AD server could not be reached (network/TLS/timeout failure),
    /// distinct from `BadCreds` so the caller can surface a different
    /// message ("AD недоступен" vs "неверный логин или пароль").
    Unreachable,
}
```
For `ad_directory.rs`, model a comparable `DirectoryOutcome` (or two separate result types —
one for displayName resolve, one for the role/group lookup — RESEARCH's skeleton names the
error type `DirectoryError`, follow that naming). Reuse the SAME never-collapse-to-bool
philosophy: `Result<DirectoryResult, DirectoryError>` where `DirectoryError` has (at minimum)
a variant distinguishing "not configured" (silent, Pitfall 5) from "unreachable" (loggable,
Pitfall 4) — do not merge these the way `AuthOutcome` merges bad-creds+not-found (that
merge is intentional there for anti-enumeration; it is NOT appropriate here, these are
operationally distinct outcomes an admin needs to tell apart).

**Trait shape to mirror** (lines 48-79):
```rust
#[async_trait]
pub trait AdClient: Send + Sync {
    async fn authenticate(
        &self,
        login: &str,
        password: &Secret<String>,
    ) -> Result<AuthOutcome, AppError>;

    async fn test_connection(&self) -> Result<AuthOutcome, AppError>;
}
```
`AdDirectory` should follow the same `#[async_trait] pub trait X: Send + Sync` shape with a
single primary method, e.g. `async fn resolve(&self, sam_account_name: &str) -> Result<DirectoryResult, AppError>`
(or split into `resolve_display_name` + `resolve_role`/`check_group_membership` if the planner
wants two round-trip-shaped methods instead of one combined call — RESEARCH's architecture
diagram implies ONE combined `directory.resolve(ad_username)` call, prefer that shape to
minimize LDAP round trips per Pattern 1/2).

**Required doc note to carry over** — the SAME hexagonal-boundary CRITICAL comment must be
copied onto the new trait (word for word except renaming AdClient→AdDirectory), because this
is exactly what `tests/no_io_deps.rs` enforces workspace-wide, not per-port:
```rust
/// CRITICAL: This trait MUST NOT import tokio, ldap3, or hickory-resolver —
/// those are infra-layer deps. `async_trait` + `crate::error::AppError` +
/// `crate::primitives::secret::Secret` are the only allowed dependencies
/// here (pure-data crate, enforced by `tests/no_io_deps.rs`).
```

---

### `crates/trackly-infra/src/ad/directory.rs` (service/adapter, request-response — LDAP)

**Analog:** `crates/trackly-infra/src/ad/real.rs` (254 lines, read in full)

**Module doc + imports** (lines 1-19):
```rust
//! Real AD client adapter using `ldap3::LdapConnAsync` (D-AD-01, D-Mock-01).
//!
//! CRITICAL: This module is the ONLY place in the codebase that imports `ldap3`.
//! `trackly-core::ports::ad::AdClient` trait must remain ldap3-free.
//!
//! Always wraps the connect call's outcome and the bind result-code into
//! `AuthOutcome` (never `Err`) — DC down / TLS failure / wrong creds are all
//! normal authentication outcomes, not infrastructure errors (mirrors
//! `RealSnmpClient`'s `Ok(None)`-for-unreachable philosophy).

use std::time::Duration;

use async_trait::async_trait;
use ldap3::{ldap_escape, LdapConnAsync, LdapConnSettings, Scope, SearchEntry};
use trackly_core::error::AppError;
use trackly_core::ports::ad::{AdClient, AuthOutcome};
use trackly_core::primitives::secret::Secret;

use crate::config::AdConfig;

const CONN_TIMEOUT: Duration = Duration::from_secs(5);
```
**IMPORTANT correction to RESEARCH.md's module-doc claim:** the module doc says "the ONLY
place in the codebase that imports `ldap3`" — this is an INVARIANT the new `directory.rs`
would violate literally if the doc-comment isn't updated. Either update this comment in
`real.rs` to say "one of two places" (the other being `directory.rs`), or — cleaner — keep the
new module's own doc as the authority and soften `real.rs`'s wording. Flag this as a required
one-line comment edit in `real.rs`, not just an addition in `directory.rs`.

**Connect + bind pattern to copy (service-bind variant)** (lines 71-97 — the `authenticate`
connect/bind prologue, MINUS the end-user-password-specific empty-check at lines 66-70 which
does not apply to a service account with a config-supplied fixed password):
```rust
let settings = LdapConnSettings::new()
    .set_conn_timeout(CONN_TIMEOUT)
    .set_no_tls_verify(self.cfg.no_tls_verify);
let url = format!("ldaps://{}:{}", self.cfg.host, self.cfg.port);

let (conn, mut ldap) = match LdapConnAsync::with_settings(settings, &url).await {
    Ok(v) => v,
    // Connect/TLS handshake failure → DC unreachable, not an error.
    Err(_) => return Ok(AuthOutcome::Unreachable),
};
// Pitfall 7: the connection driver task MUST be driven, or operations hang.
ldap3::drive!(conn);

let bind_result = match ldap.simple_bind(&bind_name, password.expose()).await {
    Ok(res) => res,
    Err(_) => return Ok(AuthOutcome::Unreachable), // protocol/IO error mid-bind
};

if bind_result.success().is_err() {
    let _ = ldap.unbind().await;
    return Ok(AuthOutcome::BadCreds); // for the service bind: map to DirectoryError::ServiceBindFailed instead
}
```
For the service-account variant, `bind_name`/`password` come from `cfg.bind_dn`/`cfg.bind_password`
(new `AdConfig` fields, see below) instead of the end-user's own credentials — this is
RESEARCH's Pattern 1, already correctly shaped; the excerpt above is the VERIFIED live source
it was adapted from (RESEARCH's code sample is a faithful translation, no corrections needed).

**Search + fallback-chain pattern to copy** (lines 99-129 — `displayName`→`cn`→login):
```rust
let filter = build_user_search_filter(login);
let attrs = vec![self.cfg.name_attr.as_str(), "cn"];

let display_name = match ldap
    .search(&self.cfg.base_dn, Scope::Subtree, &filter, attrs)
    .await
    .and_then(|search_result| search_result.success())
{
    Ok((entries, _res)) => entries
        .into_iter()
        .next()
        .map(SearchEntry::construct)
        .and_then(|entry| {
            entry
                .attrs
                .get(&self.cfg.name_attr)
                .and_then(|values| values.first().cloned())
                .or_else(|| {
                    entry
                        .attrs
                        .get("cn")
                        .and_then(|values| values.first().cloned())
                })
        })
        .unwrap_or_else(|| login.to_string()), // D-Config-02 fallback chain
    Err(_) => login.to_string(),
};

let _ = ldap.unbind().await;
```
`directory.rs` should request `memberOf` as a THIRD attribute in the SAME `search()` call
(RESEARCH Pattern 1's code sample already does this — `vec!["displayName", "cn", "memberOf"]`)
so the displayName lookup and group-membership data come back in ONE round trip, then run the
`LDAP_MATCHING_RULE_IN_CHAIN` filter as a SEPARATE, second `search()` call (RESEARCH Pattern 2)
only if role resolution needs a fresh per-group query (the plain `memberOf` attribute returned
in the first search gives DIRECT membership only — nested-group expansion still needs the
`LDAP_MATCHING_RULE_IN_CHAIN` filter as its own query per configured group DN, checked in
priority order per Assumption/Open-Question #3's highest-privilege-wins recommendation).

**Filter-escaping helper to copy verbatim (extend, don't reinvent)** (lines 49-57):
```rust
fn build_user_search_filter(login: &str) -> String {
    let safe_login = ldap_escape(login);
    format!("(|(sAMAccountName={safe_login})(userPrincipalName={safe_login}))")
}
```
Add a sibling `build_group_membership_filter` per RESEARCH Pattern 2, using the SAME
`ldap3::ldap_escape` call on BOTH the `sam_account_name` and the `group_dn` operand — the
existing injection-defense test convention (below) must be replicated for it.

**Bind-name normalization to reuse (or extract to shared fn)** (lines 36-46):
```rust
fn normalize_bind_name(&self, login: &str) -> String {
    if login.contains('@') || login.contains('\\') {
        login.to_string()
    } else {
        format!("{login}@{}", self.cfg.domain)
    }
}
```
This is `RealAdClient`'s private method. Since the service account's OWN bind DN may also need
this normalization (RESEARCH's Don't-Hand-Roll table, row 3), either (a) duplicate this ~6-line
method onto `RealAdDirectory`, or (b) extract it to a shared free function in a small
`ad/bind_name.rs` (or keep in `real.rs` as `pub(crate) fn normalize_bind_name`) and call from
both. Prefer (b) if the planner wants zero duplication; (a) is acceptable and matches the
existing codebase's general preference for small independent adapters over premature sharing
(see `mock.rs`'s own independent `lookup_key` rather than importing from `real.rs`).

**Injection-defense test pattern to replicate** (lines 177-253, full `#[cfg(test)] mod tests`
block — 3 tests: `benign_login_builds_expected_filter`, `injection_payload_metacharacters_are_escaped`,
`backslash_in_login_is_escaped`). Copy this EXACT test shape for `build_group_membership_filter`,
asserting no raw `*`/`(`/`)`/`\` survives from either the `sam_account_name` OR `group_dn` inputs.

---

### `crates/trackly-infra/src/ad/directory_mock.rs` (service/adapter mock)

**Analog:** `crates/trackly-infra/src/ad/mock.rs` (268 lines, read in full)

**Module doc + fixture shape** (lines 1-23):
```rust
//! Mock AD client — deterministic fixtures for dev macOS (D-Mock-01, USR-12).
//!
//! Used when `TRACKLY_AD_MOCK` env var is set or `config.ad.use_mock = true`.
//! Returns preset bind outcomes keyed by AD login (sAMAccountName-style),
//! mirroring `MockSnmpClient::default_fixtures` (`crates/trackly-infra/src/snmp/mock.rs`).
//!
//! 2 fixtures (per plan must_haves):
//!   us100 / Passw0rd! — Иванов Иван Иванович
//!   us200 / Secret123 — Петрова Анна Сергеевна

#[derive(Clone)]
pub struct AdFixture {
    pub password: &'static str,
    pub display_name: &'static str,
}
```
Per RESEARCH's Wave-0 gap note, **extend these SAME `us100`/`us200` fixture identities** with
group-membership data rather than inventing new fixture names — privacy-placeholder
discipline (already-in-git names). E.g. `us100` → member of a placeholder "Managers" group DN,
`us200` → no group (falls to default `employee`). Add a third fixture only if a distinct
"in an Admin-mapped group" scenario is needed for the highest-privilege-wins test (Open
Question #3) — keep the SAME naming convention (`us1NN`/`Фамилия Имя Отчество` placeholder
style already established).

**`unreachable()` constructor pattern to copy** (lines 60-67):
```rust
pub fn unreachable() -> Self {
    Self {
        users: HashMap::new(),
        unreachable: true,
    }
}
```
`MockAdDirectory` needs the SAME `unreachable()` fixture constructor — this is the exact seam
RESEARCH's fail-closed integration test (`MockAdDirectory::unreachable()`-style fixture) needs.

**Lookup-key normalization to copy verbatim (Pitfall 3 dependency)** (lines 69-78):
```rust
fn lookup_key(login: &str) -> &str {
    let without_upn_suffix = login.split('@').next().unwrap_or(login);
    without_upn_suffix
        .rsplit('\\')
        .next()
        .unwrap_or(without_upn_suffix)
}
```
RESEARCH Pitfall 3 explicitly requires the NEW cache key normalization to match THIS exact
logic — copy it verbatim into `directory_mock.rs` (or better: extract to a shared
`ad::normalize_login_key` free function used by BOTH `mock.rs` and `directory_mock.rs`/`cache.rs`,
since Pitfall 3's own fix recommendation is "reuse `MockAdClient::lookup_key`'s exact logic").

**`authenticate`-shape pattern for `resolve()`** (lines 81-108 — the structure to mirror,
substituting `resolve(sam_account_name)` for `authenticate(login, password)`):
```rust
async fn authenticate(
    &self,
    login: &str,
    password: &Secret<String>,
) -> Result<AuthOutcome, AppError> {
    if password.expose().trim().is_empty() {
        return Ok(AuthOutcome::BadCreds);
    }
    if self.unreachable {
        return Ok(AuthOutcome::Unreachable);
    }
    let key = Self::lookup_key(login);
    match self.users.get(key) {
        Some(fixture) if fixture.password == password.expose() => Ok(AuthOutcome::Ok {
            display_name: fixture.display_name.to_string(),
        }),
        Some(_) => Ok(AuthOutcome::BadCreds),
        None => Ok(AuthOutcome::BadCreds),
    }
}
```
`MockAdDirectory::resolve` drops the password-empty-check entirely (no end-user password
involved) but keeps the `if self.unreachable { return Err(DirectoryError::Unreachable) }`
early-return shape, then looks up `key` in a fixture map keyed the same way.

**Full test-module pattern to replicate** (lines 122-267 — 12 `#[tokio::test]` fns covering
success/wrong-password/not-found/unreachable/empty-password/whitespace-password/UPN-format/
NetBIOS-format/test_connection variants). Use this exact per-scenario test-naming and
one-assertion-per-test granularity for `directory_mock.rs`'s own test module (known-user
resolves / unknown-user falls back to login / cache-adjacent unreachable / UPN+NetBIOS forms
resolve to same fixture per Pitfall 3).

---

### `crates/trackly-infra/src/ad/cache.rs` (utility, TTL cache — NO direct analog, RESEARCH skeleton is authoritative)

**Analog:** No existing TTL-cache file in this codebase. Closest STRUCTURAL analog (same
"hand-rolled `Mutex`-guarded primitive, no external crate" convention) is `ReaderPool` in
`crates/trackly-infra/src/db/pools.rs` (simple `Mutex<Vec<Connection>>` LIFO pool per
STATE.md Phase 1 decisions, cited in RESEARCH). This is a CONVENTION match, not a literal
code-shape match — do not copy `ReaderPool`'s pool-checkout API shape, only its
"small hand-rolled Mutex-guarded struct, no new crate" spirit.

**Use RESEARCH.md's own code skeleton verbatim** (already correct, cites `Instant`/`Mutex`/
`HashMap` — the RESEARCH document's "TTL cache skeleton" code block is the primary source for
this file; no further verification needed, it does not reference any live codebase line
numbers that could have drifted). Key structural requirements carried over from RESEARCH:
- `entries: Mutex<HashMap<String, DirectoryCacheEntry>>` — single map, no eviction/LRU
  (Assumption A2 — acceptable at LAN/~20-user scale).
- `expires_at: Instant` (monotonic, process-local) — NOT `time::OffsetDateTime` (that crate/type
  is reserved for wall-clock DB persistence elsewhere in this codebase; this cache is ephemeral
  in-process state, `std::time::Instant` is correct here per RESEARCH's note).
- Inject an artificially-short TTL (e.g. `Duration::from_millis(10)`) for tests that assert
  expiry — do not sleep-loop against a production-scale TTL (RESEARCH's own recommendation,
  matches the "seconds-scale test" convention already used elsewhere in this codebase's async
  tests).
- Cache key MUST be the normalized login (Pitfall 3) — reuse the SAME normalization function
  as `directory_mock.rs`'s `lookup_key`/`RealAdDirectory`'s bind-name normalization (see above).
- Two independently configurable TTLs per RESEARCH Open Question #2 (displayName TTL longer,
  e.g. 30 min; group/role TTL shorter, e.g. 5 min) — either two `DirectoryCache` instances with
  different `ttl: Duration` values, or one cache storing both fields with per-field expiry
  timestamps; the simpler two-instance approach matches the "small hand-rolled primitive, no
  cleverness" philosophy better.

---

### `crates/trackly-app/src/services/auth.rs` (service, CRUD + request-response — MODIFY)

**Analog:** itself — extend the existing `AuthService` struct/methods in place (this file
already contains the exact seam this phase extends; there is no better external analog).

**`AuthService` struct + constructor — field to add** (verified live, lines 143-172):
```rust
#[derive(Clone)]
pub struct AuthService {
    pub writer: Arc<WriterHandle>,
    pub readers: Arc<ReaderPool>,
    pub(crate) clock: Arc<dyn Clock + Send + Sync>,
    /// AD client — `RealAdClient` in prod, `MockAdClient` on dev macOS
    /// (D-Mock-01). Used by `login()`'s local→AD fallback (USR-08).
    pub(crate) ad_client: Arc<dyn AdClient + Send + Sync>,
    pub(crate) ws_tx: Arc<tokio::sync::broadcast::Sender<WsEvent>>,
}

impl AuthService {
    pub fn new(
        writer: Arc<WriterHandle>,
        readers: Arc<ReaderPool>,
        clock: Arc<dyn Clock + Send + Sync>,
        ad_client: Arc<dyn AdClient + Send + Sync>,
        ws_tx: Arc<tokio::sync::broadcast::Sender<WsEvent>>,
    ) -> Self {
        Self { writer, readers, clock, ad_client, ws_tx }
    }
    ...
```
Add a new field the SAME way `ad_client` was added: `pub(crate) directory: Arc<dyn AdDirectory + Send + Sync>`,
with an accompanying doc-comment in the same style ("`RealAdDirectory` in prod, `MockAdDirectory`
on dev macOS (D-Mock-01). Used by `sso_login`'s displayName/role enrichment (SSO-01/SSO-03).")
and a new trailing constructor parameter. **Breaking-change ripple:** `AuthService::new` is a
POSITIONAL constructor with 5 params today; adding a 6th param breaks EVERY call site.
**CORRECTED (plan-checker BLOCKER):** there are **8** call sites to update, not 2 — re-verified
via `grep -rn "AuthService::new" crates/`. Two of the 8 live INSIDE the `trackly-app` lib crate's
own `#[cfg(test)]` modules (same compilation unit as `auth.rs`), so leaving them broken makes
`auth.rs`'s own `--lib` test target fail to compile. Full inventory:
- `crates/trackly-app/src/context.rs:308` — production wiring (mock/real `use_ad_mock` switch)
- `crates/trackly-app/src/http/health.rs:75` — `#[cfg(test)] mod tests::minimal_ctx` (same lib crate)
- `crates/trackly-app/src/tauri_cmds/health.rs:91` — `#[cfg(test)] mod tests::minimal_ctx` (same lib crate)
- `crates/trackly-app/tests/ad_auth.rs:29` — `make_auth_service_with_ad` helper
- `crates/trackly-app/tests/specta_roundtrip.rs:63` — inline construction
- `crates/trackly-app/tests/auth_smoke.rs:24` — `make_auth_service` helper
- `crates/trackly-app/tests/users_crud.rs:22` — `make_auth_service` helper
- `crates/trackly-app/tests/ad_register.rs:33` — `make_auth_service_with_ad` helper

Every site EXCEPT `context.rs` injects the 6th arg as `Arc::new(MockAdDirectory::default_fixtures())`
(none exercise the fail-closed path). Plan 31-03 owns all 8 (Task 1: the two same-lib-crate
`health.rs` sites; Task 2: `context.rs` + the 5 `tests/*.rs` binaries).

**`sso_login` — exact current code to modify** (verified live, lines 266-289):
```rust
/// Passwordless AD SSO login (spike-002 / Kerberos-SPNEGO).
/// ...
/// NOTE (full-parity follow-up): `display_name` currently falls back to the SAM login
/// because SSO has no bind to search from. A service-account displayName lookup (as in
/// the adwebapp reference) is deferred to the AD-SSO milestone.
pub async fn sso_login(
    &self,
    ad_username: &str,
    display_name: &str,
) -> Result<UserDto, AppError> {
    if !self.ad_enabled().await? {
        return Err(AppError::Unauthorized);
    }
    self.on_ad_bind_success(ad_username, display_name).await
}
```
This is THE primary hook point. Per RESEARCH's architecture diagram, insert the directory
resolve call BEFORE `on_ad_bind_success`, replacing the `display_name` parameter's blind
pass-through with a resolved value (falling back to `ad_username` on any directory
error/not-configured, per Pitfall 5) and threading a resolved `role_hint: Option<Role>`
through to `on_ad_bind_success` (which currently takes only `login`/`display_name`, see below
— its signature needs extending too). Delete the stale `NOTE (full-parity follow-up)` doc
comment once this is wired (it was written by the prior phase specifically flagging this gap).

**`on_ad_bind_success` — exact current signature to extend** (verified live, lines 365-369):
```rust
async fn on_ad_bind_success(
    &self,
    login: &str,
    display_name: &str,
) -> Result<UserDto, AppError> {
```
Per RESEARCH's explicit instruction ("Do not touch `on_ad_bind_success`'s branching logic
itself — only feed it better inputs"), add a THIRD parameter (`role_hint: Option<Role>` or
similar) threaded through unchanged branching, then passed down into
`auto_register_ad_user`/`create_pending_registration` (both currently take only
`login`/`display_name`, see next).

**`auto_register_ad_user` — exact hardcoded-role INSERT to modify** (verified live,
lines 489-510, hardcode at line 507):
```rust
async fn auto_register_ad_user(
    &self,
    login: &str,
    display_name: &str,
) -> Result<UserDto, AppError> {
    ...
    tx.execute(
        "INSERT INTO users \
         (login, full_name, password_hash, role, ad_user, is_active, \
          created_at_utc, updated_at_utc, version) \
         VALUES (?1, ?2, NULL, 'employee', 1, 1, ?3, ?3, 1)",
        rusqlite::params![login_owned, display_name_owned, now],
    )
    .map_err(map_rusqlite)?;
```
Replace the literal `'employee'` SQL string with a bound parameter (`?4`) fed by
`role_hint.map(|r| r.as_str()).unwrap_or("employee")` (using the EXISTING `Role::as_str()`
method, `crates/trackly-core/src/auth.rs:47-53` — do not hand-roll a role→string mapping,
reuse this). Add `role` as a new function parameter, threaded from `on_ad_bind_success`.

**`create_pending_registration` — same hardcoded-role INSERT pattern** (verified live,
lines 560-581, hardcode at line 578):
```rust
async fn create_pending_registration(
    &self,
    login: &str,
    display_name: &str,
) -> Result<UserDto, AppError> {
    ...
    tx.execute(
        "INSERT INTO users \
         (login, full_name, password_hash, role, ad_user, is_active, \
          created_at_utc, updated_at_utc, version) \
         VALUES (?1, ?2, NULL, 'employee', 1, 0, ?3, ?3, 1)",
        rusqlite::params![login_owned, display_name_owned, now],
    )
```
IDENTICAL modification to `auto_register_ad_user` above — same pattern, same fix.

**Imports to extend** (verified live, lines 13-34 — add `AdDirectory`/`DirectoryError` etc.
alongside the existing `AdClient`/`AuthOutcome` import on line 24):
```rust
use trackly_core::ports::ad::{AdClient, AuthOutcome};
```
becomes (illustrative):
```rust
use trackly_core::ports::ad::{AdClient, AuthOutcome};
use trackly_core::ports::ad_directory::{AdDirectory, DirectoryError, DirectoryResult};
```

---

### `crates/trackly-app/src/http/sso.rs` (controller/route handler — MODIFY)

**Analog:** itself — the exact call site to change is already isolated in a small function.

**Current code (verified live, lines 63-71):**
```rust
/// After a successful accept, resolve the AD account to a Trackly user and issue the same
/// session cookie as password login (T-05-SF: flush before insert). Returns the display
/// name for the JSON body. Mirrors `build_auth_login`'s session handling.
async fn issue_sso_session(
    ctx: &AppCtx,
    session: &Session,
    ad_username: &str,
) -> Result<String, AppError> {
    let user = ctx.auth.sso_login(ad_username, ad_username).await?;
```
**RESEARCH.md's line citation is very slightly off** (it says "line 71" for the `sso_login`
call — the call IS on line 71, verified correct; but `issue_sso_session`'s own `fn` signature
starts at line 66, not line 66-95 as RESEARCH's Sources section states — the function body
ends at line 95, matching). No functional discrepancy, just a minor citation-range nuance —
noted for planner precision.

**Required change:** Since `AuthService::sso_login` now does the directory resolve internally
(per the `auth.rs` pattern above), this call site likely needs NO change at all if
`sso_login`'s SECOND parameter (`display_name`) is simply ignored/superseded inside
`sso_login` once directory resolution is wired — OR, if the planner prefers `issue_sso_session`
to pass `ad_username` as a hint only and let `sso_login`'s new internal resolve win, the
existing `ctx.auth.sso_login(ad_username, ad_username)` call can remain textually unchanged
(the SAME bare login passed twice, exactly as today) since the enrichment now happens
INSIDE `sso_login`, not at this call site. **Recommendation: keep `sso.rs` unchanged**; all
the real work is inside `AuthService::sso_login`/`on_ad_bind_success` per the Architecture
Patterns diagram in RESEARCH.md (step 3 happens INSIDE `sso_login`, not in the HTTP handler).
Flag this file as "likely zero-diff" rather than "modify" once the planner confirms the
call-site contract.

---

### `crates/trackly-infra/src/config.rs` (config — MODIFY `AdConfig`)

**Analog:** itself — extend the existing struct; the redacting-`Debug` requirement has no
existing struct-level analog to copy verbatim, but `Secret<T>`'s hand-written `Debug` impl
(below) is the CONVENTION to reuse.

**Current `AdConfig` struct (verified live, lines 130-174) — note it derives `Debug` directly:**
```rust
#[derive(Debug, Deserialize, Clone)]
pub struct AdConfig {
    pub enabled: bool,
    pub use_mock: bool,
    pub host: String,
    pub port: u16,
    pub domain: String,
    pub base_dn: String,
    pub name_attr: String,
    pub no_tls_verify: bool,
    #[serde(default)]
    pub sso_enabled: bool,
    #[serde(default)]
    pub spn: String,
    #[serde(default)]
    pub keytab_path: String,
}
```
**Correction to RESEARCH.md's Pitfall 1:** RESEARCH says "verify [`AppConfig` deriving Debug]
stays true, or apply the same redaction there too" as an open question — **VERIFIED: `AppConfig`
(the parent struct, `crates/trackly-infra/src/config.rs:23`) DOES derive `Debug`**
(`#[derive(Debug, Deserialize, Clone, Default)]`), contradicting RESEARCH's speculative "does
NOT currently derive Debug" framing. This does **not** require extra redaction work beyond
`AdConfig` itself, though: Rust's derived `Debug` for a struct simply calls each field's own
`Debug::fmt` — if `AdConfig` gets a hand-written, non-derived `Debug` impl that redacts
`bind_password`, `AppConfig`'s DERIVED `Debug` impl will correctly call into that redacted
impl when formatting the `ad: AdConfig` field. **No propagation gap** — just don't add
`#[derive(Debug)]` to `AdConfig` once the manual impl exists (remove `Debug` from the derive
list on `AdConfig`'s line 130, replace with a hand-written `impl fmt::Debug for AdConfig`).

**Secret redaction convention to reuse (structurally, not verbatim — `Secret<T>` itself
cannot `#[derive(Deserialize)]`, RESEARCH Pitfall 1 correctly identifies this)**, from
`crates/trackly-core/src/primitives/secret.rs` lines 41-45:
```rust
impl<T: Zeroize + Clone> fmt::Debug for Secret<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("***")
    }
}
```
Apply the SAME "***"-redaction convention in a hand-written `impl fmt::Debug for AdConfig`
that prints every field normally EXCEPT `bind_password`, which prints as `"***"` — this
satisfies RESEARCH Pitfall 1's option (a) without needing `Secret<T>` itself (option (a) is
simpler than (b)'s shadow-struct approach and is the recommended path).

**Default impl pattern to extend (verified live, lines 176-192):**
```rust
impl Default for AdConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            use_mock: false,
            host: String::new(),
            port: 636,
            domain: String::new(),
            base_dn: String::new(),
            name_attr: "displayName".to_string(),
            no_tls_verify: false,
            sso_enabled: false,
            spn: String::new(),
            keytab_path: String::new(),
        }
    }
}
```
New fields (`bind_dn: String`, `bind_password: String`, group→role mapping table, cache TTLs)
all need `#[serde(default)]` (matching the existing `sso_enabled`/`spn`/`keytab_path` pattern,
lines 161-173 — "old configs without them parse, feature defaults off") and a corresponding
empty/zero default in `impl Default for AdConfig`.

---

### `trackly.config.toml.example` (config template — MODIFY)

**Analog:** itself (verified live, full file, 16 lines) — the file is currently STALE, it does
not even show `[ad]`/`[server]` sections that already exist in code (confirms RESEARCH's
claim exactly):
```toml
# Trackly — шаблон конфигурации portable-режима
# Переименуйте этот файл в trackly.config.toml и раскомментируйте нужные поля.
# Файл должен находиться рядом с trackly.exe.

# [storage]
# Путь к файлу базы данных.
# ...

# [server]
# Порт серверного режима (HTTPS). По умолчанию: 8443.
# port = 8443
# Адрес привязки. По умолчанию: 0.0.0.0 (все сетевые интерфейсы).
# bind = "0.0.0.0"
```
Follow the EXISTING commented-out-by-default style (`# key = value` with a one/two-line
Russian comment above each field) for the new `[ad]` section fields: `bind_dn`, `bind_password`
(comment MUST say this is a placeholder/service-account credential, never a real one, and that
the real file is gitignored — mirrors the module-doc language already in `config.rs`'s AD
section, lines 124-129), group→role mapping table entries, and the two cache TTLs (displayName
TTL, group/role TTL). Since this phase is also the first to add ANY `[ad]` section to this
example file, bring the WHOLE existing `[ad]` surface up to date here too (host/port/domain/
base_dn/name_attr/no_tls_verify/sso_enabled/spn/keytab_path) — not just the phase-31-new fields
— per RESEARCH's explicit instruction ("this phase should bring it up to date, not just add
new fields to a stale example").

---

### `crates/trackly-app/tests/ad_directory_sso.rs` (NEW integration test)

**Analog:** `crates/trackly-app/tests/ad_auth.rs` (243 lines, read in full)

**Test-seam helper to extend (verified live, lines 22-31) — REQUIRES a signature change
once `AuthService::new` gains the `directory` param:**
```rust
fn make_auth_service_with_ad(
    ad_client: Arc<dyn trackly_core::ports::ad::AdClient + Send + Sync>,
) -> (AuthService, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let (ws_tx, _) = tokio::sync::broadcast::channel(128);
    let svc = AuthService::new(writer, readers, clock, ad_client, Arc::new(ws_tx));
    (svc, dir)
}
```
**This exact helper in `ad_auth.rs` will fail to compile once `AuthService::new` gains a 6th
parameter** — it MUST be updated in the SAME change (add a `directory` parameter to
`make_auth_service_with_ad`, defaulting existing callers in `ad_auth.rs` to a benign
`MockAdDirectory` fixture, e.g. `MockAdDirectory::default_fixtures()`, so the 5 EXISTING
tests in that file keep passing unchanged). The NEW `ad_directory_sso.rs` file should copy
this exact helper shape but accept BOTH `ad_client` and `directory` as parameters (or add a
second, directory-specific helper alongside it — either is fine, follow whichever the planner
finds cleaner. Given only ONE test file currently constructs `AuthService`, updating the
shared helper signature is the lower-duplication choice).

**Fixture-seeding helper to copy verbatim (adapt role literal)** (lines 41-65):
```rust
async fn seed_ad_user(svc: &AuthService, login: &str, full_name: &str, role: &str) {
    let now = SystemClock.unix_seconds();
    let login = login.to_string();
    let full_name = full_name.to_string();
    let role = role.to_string();
    svc.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO users (login, full_name, password_hash, role, ad_user, \
                 is_active, created_at_utc, updated_at_utc, version) \
                 VALUES (?1, ?2, NULL, ?3, 1, 1, ?4, ?4, 1)",
                params![login, full_name, role, now],
            )
            .map_err(|e| AppError::Internal {
                source_chain: format!("seed_ad_user: {e}"),
            })?;
            Ok(())
        })
        .await
        .expect("seed AD user");
}
```
Reuse this UNCHANGED in `ad_directory_sso.rs` — the new tests need it for the "existing local
user, unaffected by directory enrichment" regression case (RESEARCH's SSO-03 "no configured
group → unchanged 'employee' default" test row).

**Per-scenario test-naming/structure to mirror** (lines 71-242 — one `#[tokio::test]` per named
scenario, `admin_caller()`/`mock_default()`/`mock_unreachable()` helper-fn convention at
lines 33-39, 67-69). Write the new SSO-01/SSO-03 tests in the SAME one-behavior-per-test,
Russian-comment-banner style (`// --- Test N: ... ---` section dividers, lines 71-74 etc.):
- SSO-01: `MockAdDirectory` resolves known `sAMAccountName` → real `full_name` shows in `UserDto`
  after `sso_login` (not the bare login).
- SSO-01: unknown `sAMAccountName` → `full_name` falls back to login (no panic/error surfaced).
- SSO-03: user in a configured (mock-fixture) group → mapped role on FIRST auto-register login.
- SSO-03: user in no configured group → default `'employee'` (regression against
  `auto_register_ad_user`'s existing behavior, using `seed_ad_user`'s existing-user path AND a
  fresh not-yet-seeded login for the auto-register path).
- SSO-03: `MockAdDirectory::unreachable()` fixture during group check → role NOT elevated,
  still lands on `RegistrationPending`/`employee` path (fail-closed) — mirrors `ad_auth.rs`'s
  `ad_unreachable_distinct_error` test shape (lines 173-192) but asserts on the ROLE/pending
  outcome rather than `ServiceUnavailable` (SSO enrichment degrades gracefully per Pitfall 5,
  it does NOT hard-fail the whole login the way password-AD-bind unreachability does).

## Shared Patterns

### 3-state (never boolean) outcome modeling for AD-adjacent I/O
**Source:** `crates/trackly-core/src/ports/ad.rs` lines 22-40 (`AuthOutcome::Ok/BadCreds/Unreachable`)
**Apply to:** `ad_directory.rs`'s new `DirectoryOutcome`/`DirectoryError` types — every AD-adjacent
result in this codebase is modeled as a 3+-variant enum/Result, never a `bool`, specifically so
"unreachable"/"not configured" are distinguishable from "checked, negative result" in logs and
in the caller's branching (RESEARCH Pitfall 4 makes this explicit for the group-membership check).

### Mock/Real split gated by `TRACKLY_AD_MOCK` env var + `config.ad.use_mock`
**Source:** `crates/trackly-app/src/context.rs` lines 285-297
```rust
let use_ad_mock = config.ad.use_mock || std::env::var("TRACKLY_AD_MOCK").is_ok();
tracing::info!(
    ad_mode = if use_ad_mock { "mock" } else { "real" },
    "AD client selected"
);
let ad_client: Arc<dyn trackly_core::ports::ad::AdClient + Send + Sync> = if use_ad_mock {
    Arc::new(MockAdClient::default_fixtures())
} else {
    Arc::new(RealAdClient::new(config.ad.clone()))
};
```
**Apply to:** the new `directory` field's wiring — add an IDENTICAL `if use_ad_mock { ... } else { ... }`
block for `Arc<dyn AdDirectory + Send + Sync>` right after this existing block (same
`use_ad_mock` boolean, no new env var needed), then thread `directory` into the
`AuthService::new(...)` call at line 308-314 as the new trailing argument.

### LDAP filter injection defense — `ldap3::ldap_escape()` on every interpolated value
**Source:** `crates/trackly-infra/src/ad/real.rs` lines 49-57 (`build_user_search_filter`) +
its test module, lines 177-253
**Apply to:** BOTH the service-bind user-search filter AND the new group-membership filter's
`sam_account_name`/`group_dn` operands in `directory.rs` — this codebase already has a
documented, tested vulnerability class here (see the module's own test names); the new module
needs the identical escape-then-test treatment, no exceptions for "the input is already
Kerberos-authenticated" (defense in depth, per RESEARCH's own Don't-Hand-Roll table).

### Redacted `Debug` for secret-adjacent config fields
**Source:** `crates/trackly-core/src/primitives/secret.rs` lines 41-45 (`impl fmt::Debug for Secret<T>` → `"***"`)
**Apply to:** `AdConfig`'s new `bind_password` field — since `Secret<T>` cannot itself derive
`Deserialize` (by design, `secret.rs` lines 7-9), `AdConfig` needs its OWN hand-written
`impl fmt::Debug` (not a derive) that redacts just this one field the same way, rather than
using `Secret<T>` directly on a `#[derive(Deserialize)]` struct field.

### `Role::as_str()` / `Role::from_str()` for role↔string conversion
**Source:** `crates/trackly-core/src/auth.rs` lines 27-54
```rust
pub fn from_str(s: &str) -> Result<Self, AppError> { ... }
pub fn as_str(&self) -> &'static str {
    match self {
        Self::Admin => "admin",
        Self::Manager => "manager",
        Self::Employee => "employee",
    }
}
```
**Apply to:** the group→role config mapping (parse config strings into `Role` via `from_str`)
and the `auto_register_ad_user`/`create_pending_registration` INSERT statements (use
`role.as_str()` instead of a hand-rolled string match) — never reinvent this conversion.

## No Analog Found

None — every file in the Wave 0 gap list has at least a role-match analog (the TTL cache in
`cache.rs` has no LITERAL analog but RESEARCH's own code skeleton is complete and verified
consistent with the codebase's conventions, so it is not treated as a gap requiring
planner-invented structure).

## Corrections to RESEARCH.md (verify against these, not the original citations)

| RESEARCH.md claim | Verified reality | Impact |
|---|---|---|
| "AppConfig does NOT currently derive Debug — verify this stays true" (Pitfall 1) | `AppConfig` DOES derive `Debug` (`config.rs:23`, `#[derive(Debug, Deserialize, Clone, Default)]`) | No extra work needed — a hand-written non-derived `Debug` for `AdConfig` alone is sufficient; `AppConfig`'s derived `Debug` calls into it correctly. Just don't re-add `#[derive(Debug)]` to `AdConfig` once it has a manual impl. |
| `real.rs` module doc: "the ONLY place ... that imports `ldap3`" | Still true TODAY, but will become FALSE once `directory.rs` is added | Plan a one-line doc-comment edit in `real.rs` (or scope the invariant to "the only places in `trackly-infra::ad`" rather than a single file) alongside adding `directory.rs`. |
| `AdConfig` struct "line ~130" | Confirmed: struct starts at `config.rs:131`, `#[derive(..)]` at `config.rs:130` | Matches, no correction needed. |
| `sso.rs::issue_sso_session`, "line 66-95" | Function starts at `sso.rs:66`, ends `sso.rs:95` — confirmed correct | Matches, no correction needed. |
| `auth.rs:507`/`578` hardcoded `'employee'` | Confirmed: `auth.rs:507` (`auto_register_ad_user`) and `auth.rs:578` (`create_pending_registration`) | Matches exactly. |
| `auth.rs:365` `on_ad_bind_success` | Confirmed: `async fn on_ad_bind_success` starts at `auth.rs:365` | Matches exactly. |
| `auth.rs:276-279` NOTE comment | Confirmed: NOTE comment spans `auth.rs:277-279`, `sso_login` fn starts `auth.rs:280` | Matches (off-by-one on the exact NOTE start line, immaterial). |

## Metadata

**Analog search scope:** `crates/trackly-core/src/ports/`, `crates/trackly-infra/src/ad/`,
`crates/trackly-infra/src/config.rs`, `crates/trackly-infra/src/db/pools.rs` (cache convention
check), `crates/trackly-app/src/services/auth.rs`, `crates/trackly-app/src/http/sso.rs`,
`crates/trackly-app/src/context.rs`, `crates/trackly-app/tests/ad_auth.rs`,
`crates/trackly-core/src/auth.rs`, `crates/trackly-core/src/primitives/secret.rs`,
`crates/trackly-core/tests/no_io_deps.rs`, `trackly.config.toml.example`.
**Files scanned:** 14 (all read in full except `auth.rs`/`config.rs`/`context.rs`, which were
targeted-range read via `grep`-located line numbers per the large-file protocol).
**Pattern extraction date:** 2026-08-03

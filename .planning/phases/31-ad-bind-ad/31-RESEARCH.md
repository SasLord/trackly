# Phase 31: Служебный AD-bind — ФИО и роли из AD-групп - Research

**Researched:** 2026-08-03
**Domain:** LDAP/Active Directory service-account bind (Rust, `ldap3` 0.12), TTL caching, fail-closed authorization
**Confidence:** HIGH (existing codebase patterns are directly extensible; the target `ldap3` API surface is already proven in `real.rs`) / MEDIUM (group-membership query itself is `[ASSUMED]` from adwebapp reference + `ldap3` docs, not yet live-tested against a real DC in Rust)

## Summary

Phase 31 does NOT start from zero — it extends an already-working, well-tested AD subsystem.
`crates/trackly-infra/src/ad/real.rs` already performs an LDAPS bind + `sAMAccountName` search
+ `displayName`→`cn`→login fallback for the **password-login** path. `ldap3 0.12.1` with
`tls-rustls-ring` is already a pinned dependency (rustls-based, no OpenSSL — satisfies
CLAUDE.md). The **SSO path** (`crates/trackly-app/src/http/sso.rs::issue_sso_session`,
line 71) currently calls `ctx.auth.sso_login(ad_username, ad_username)` — passing the bare
Kerberos-authenticated login as BOTH the username and the display name, with an explicit
`NOTE (full-parity follow-up)` comment in `services/auth.rs` (lines 276-279) marking exactly
this gap for "the AD-SSO milestone" — i.e., this phase.

The work is: (1) add a **service-account bind** capability (distinct from the existing
user-bind path — SSO users have no password to bind with), (2) reuse the existing
`build_user_search_filter`/`ldap_escape` pattern to look up the SSO-authenticated
`sAMAccountName` via the service account, (3) add a **group-membership check** via
`LDAP_MATCHING_RULE_IN_CHAIN` (OID `1.2.840.113556.1.4.1941`) — the same technique
`adwebapp`'s reference `ldap.go` uses — mapped through a configurable
group→role table, (4) wrap both lookups in a small **TTL cache** keyed by
`sAMAccountName`, and (5) wire the result into the *existing* `on_ad_bind_success`
provisioning seam so role assignment happens exactly where local/AD-password login
already resolves accounts.

**Primary recommendation:** Add a new `AdDirectory` port (mirrors the existing `AdClient`
port pattern exactly: trait in `trackly-core::ports`, `RealAdDirectory`/`MockAdDirectory`
impls in `trackly-infra::ad`, `ldap3`-free core). Hook it into `AuthService::sso_login`
BEFORE calling `on_ad_bind_success`, so `display_name` and the auto-assigned `role` are
both resolved from the directory before the existing provisioning branches run. Do not
touch `on_ad_bind_success`'s branching logic itself — only feed it better inputs (already
matches the ROADMAP's stated Phase 32 dependency: "тот же провижининг-путь
`on_ad_bind_success`, расширяется этой фазой").

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| SPNEGO/Kerberos ticket validation | API/Backend (`trackly-infra::ad::sso`) | — | Already implemented; out of scope for this phase (no changes needed) |
| Service-account LDAP bind + user search (displayName) | API/Backend (`trackly-infra::ad`, new module) | — | Pure server-side I/O; must never reach the browser/webview tier |
| Group-membership check (`memberOf`/`LDAP_MATCHING_RULE_IN_CHAIN`) | API/Backend (same new module) | — | Same LDAP connection/bind as displayName lookup — one round trip ideally |
| Group→role mapping table | API/Backend (config: `trackly.config.toml`) | — | Static, admin-authored, not runtime-mutable via UI in this phase (see Open Questions) |
| TTL cache (sAMAccountName → {display_name, role, expires_at}) | API/Backend (in-process, `AuthService` or sibling service) | — | Single-process server mode only (CLAUDE.md: "never use in multi-process scenarios" — n/a here, Trackly server mode is one process) |
| Fail-closed role gating | API/Backend (`AuthService::on_ad_bind_success` call site) | — | Security-relevant decision; must never be delegated to the frontend |
| Display of resolved ФИО | Browser/Client (Svelte) | — | Pure rendering — `UserDto.full_name` already flows end-to-end |
| AD settings read-only display (host/domain/etc.) | Frontend surface only | API/Backend (source of truth) | Existing pattern in `ActiveDirectorySettings.svelte` — extend, don't replace |

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| SSO-01 | SSO users show real ФИО (`displayName`→`cn`→login) via service-account bind, cached | Service-bind + search pattern already proven in `real.rs`'s `authenticate()` (lines 99-129); this phase extracts an equivalent lookup that binds with a FIXED service account instead of the end user's own bind, then adds a TTL cache layer (new) |
| SSO-03 | Roles auto-assigned from AD group membership via `memberOf`/`LDAP_MATCHING_RULE_IN_CHAIN`, fail-closed | `adwebapp`'s `ldap.go::queryGroupMembership` (read-only reference, lines 235-267) is the exact algorithm to port to `ldap3`; fail-closed semantics must be enforced at the `AuthService` call site, not inside the LDAP client (mirrors existing `AuthOutcome::Unreachable` pattern in `trackly-core::ports::ad`) |

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `ldap3` | `0.12.1` (already pinned, `default-features = false`, `features = ["tls-rustls-ring"]`) `[VERIFIED: cargo search + crates/trackly-infra/Cargo.toml:32]` | LDAP client — bind, search, filter escaping | Already the sole LDAP dependency in the codebase (`real.rs` module doc: "the ONLY place ... that imports `ldap3`" — Phase 31 must preserve this invariant, put the new service-bind code in the same module family, not a second LDAP crate) |
| `async-trait` | workspace-pinned `[VERIFIED: crates/trackly-infra/Cargo.toml]` | Trait objects for the new `AdDirectory` port (mirrors `AdClient`) | Already used for `AdClient`/`SnmpClient` ports |
| `tokio` | workspace-pinned | Drives `ldap3::LdapConnAsync` (the `ldap3::drive!(conn)` macro requirement — Pitfall 7 in existing code, applies identically to the new service-bind connection) | Already the async runtime |
| `thiserror` | workspace-pinned | Typed errors for the new directory/group-lookup port, mirrors `SsoError` in `ad/sso.rs` | Project convention |

**No new external crates are required for this phase.** The service-account bind, user
search, and group-membership query are all expressible with the `ldap3` API surface already
exercised in `real.rs` — same `LdapConnAsync::with_settings` → `simple_bind` → `search` shape,
just with a fixed service DN instead of the caller's own credentials, and one additional
filter clause (`memberOf:1.2.840.113556.1.4.1941:=<group-DN>`). For the TTL cache: **hand-roll
it** (see Don't Hand-Roll below for the one thing you should NOT hand-roll — the cache itself
is fine to hand-roll, it's ~40 LoC and matches the codebase's existing "hand-roll simple pool"
convention, e.g. `ReaderPool: simple std::sync::Mutex<Vec<Connection>> LIFO` per STATE.md
Phase 1 decisions). Do not add `moka` or `cached` — neither is in the workspace, and CLAUDE.md's
whole "Alternatives Considered" philosophy for this project favors small hand-rolled
primitives over new dependencies when the data structure is this simple (single map, single
TTL, single-process, no eviction-policy sophistication needed at LAN/20-user scale).

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `zeroize` | workspace-pinned | Already a dep; use for the new service-account password (see Common Pitfalls — config Debug leak) | Any new secret field on `AdConfig` |
| `hickory-resolver` | `0.26.1` (existing, `ad/discovery.rs`) | NOT needed for this phase directly, but `derive_base_dn()` in `discovery.rs` is directly reusable if the group DN needs deriving from a bare group name in the same base DN | Reuse only — no new usage of the DNS-SRV discovery part |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Hand-rolled `Mutex<HashMap<..>>` TTL cache | `moka::sync::Cache` (with `time_to_live`) | `moka` gives you built-in eviction, size limits, async support — but it's a new dependency for what is, at LAN scale (a few dozen distinct SSO users), a trivially small map. Not worth the dependency-audit and slopcheck overhead for this phase's scope. Revisit only if cache correctness bugs surface in practice. |
| `memberOf` + `LDAP_MATCHING_RULE_IN_CHAIN` (server-side nested-group expansion) | Manual recursive `memberOf` walk (fetch group's own `memberOf`, repeat) | Manual walk requires N extra round trips per nested level and duplicate-detection logic to avoid cycles (AD groups CAN be circularly nested by misconfiguration). `LDAP_MATCHING_RULE_IN_CHAIN` does this server-side in Microsoft's AD implementation in one query. This is also exactly what `adwebapp`'s reference does — don't regress to something worse. |
| Service-account **simple bind** over LDAPS | Anonymous bind for the read-only lookup | Many AD deployments disable anonymous bind entirely (confirmed by the existing `test_connection()` fallback logic in `real.rs` lines 162-169, which already anticipates "server rejected anonymous bind"). A dedicated read-only service account (`svc-trackly-ro` style, least-privilege, read-only ACL in AD) is the standard, safer approach — matches `adwebapp`'s `bindDN`/`bindPassword` config shape exactly. |

**Installation:** No `cargo add` needed — `ldap3` is already a dependency. If a new
`trackly-infra::ad::directory` (or similar) module is added, no `Cargo.toml` change is required.

**Version verification:** `ldap3 = "0.12.1"` confirmed current via `cargo search ldap3`
(no newer 0.12.x/0.13 release found at research time) `[VERIFIED: cargo search ldap3, 2026-08-03]`.

## Package Legitimacy Audit

**No new external packages are introduced by this phase.** `ldap3 0.12.1` is an existing,
already-audited workspace dependency (introduced in Phase 9 for the password-AD-login path;
its legitimacy was already established then — it is the canonical pure-Rust LDAP client,
`github.com/inejge/ldap3`, actively maintained). No `slopcheck`/registry-verification gate
applies here since nothing new is being installed.

If the planner decides a TTL-cache crate (e.g. `moka`) IS worth adding despite the
recommendation above, run the full Package Legitimacy Gate on it before use.

**Packages removed due to slopcheck [SLOP] verdict:** none (no new packages)
**Packages flagged as suspicious [SUS]:** none (no new packages)

## Architecture Patterns

### System Architecture Diagram

```
Browser (domain-joined, Kerberos ticket in OS credential cache)
   │  GET /api/v1/auth_ad_sso  (Authorization: Negotiate <token>)
   ▼
axum handler_ad_sso  (crates/trackly-app/src/http/sso.rs:98)
   │  1. accept_spnego() → validates ticket against service keytab (OFFLINE, no KDC/LDAP)
   │     → Ok(Authenticated { username: "us100", .. })
   ▼
issue_sso_session()  (sso.rs:66)
   │  2. ctx.auth.sso_login(ad_username, ad_username)   ◄── TODAY: display_name == ad_username
   │                                                          (THIS PHASE changes this call)
   ▼
AuthService::sso_login()  (services/auth.rs:280)
   │  3. NEW: resolve via AdDirectory (service-account bind)
   │        directory.resolve(ad_username)
   │          -> cache hit?  return cached {display_name, role_hint}
   │          -> cache miss: bind as svc account -> search sAMAccountName
   │                          -> displayName / cn / login fallback
   │                          -> group-membership query (LDAP_MATCHING_RULE_IN_CHAIN)
   │                          -> map matched group(s) -> role via config table
   │                          -> cache the result (TTL)
   │        DIRECTORY UNREACHABLE during ANY step of this resolve
   │          -> display_name: degrade to `ad_username` (non-fatal, same as adwebapp)
   │          -> role: DO NOT ELEVATE — proceed as if no group matched (fail-closed)
   ▼
on_ad_bind_success(login, display_name, role_hint)   (services/auth.rs:365, EXTENDED)
   │  4. EXISTING branching (active/pending/blocked/unknown) — UNCHANGED
   │     but auto_register_ad_user / (new) role-sync-on-existing-user paths
   │     now use role_hint instead of hardcoded 'employee'
   ▼
UserDto { full_name: <real ФИО>, role: <mapped or fallback> }
   │  5. session cookie issued (tower_sessions) — UNCHANGED
   ▼
Svelte SPA — Dashboard, real ФИО shown in header (UNCHANGED — already reads UserDto.full_name)
```

### Recommended Project Structure

```
crates/trackly-core/src/ports/
├── ad.rs                  # EXISTING — AdClient (bind+search for password login) — untouched
└── ad_directory.rs        # NEW — AdDirectory port: resolve(login) -> DirectoryResult
                            #   { display_name: String, role: Option<Role> }
                            #   ldap3-free, enforced by the SAME no_io_deps.rs test

crates/trackly-infra/src/ad/
├── real.rs                # EXISTING — user-bind path, untouched
├── mock.rs                # EXISTING — user-bind mock, untouched
├── directory.rs           # NEW — RealAdDirectory: service-account bind + search + group check
├── directory_mock.rs       # NEW — MockAdDirectory: deterministic fixtures (mirrors mock.rs)
├── cache.rs                # NEW — small TTL cache: Mutex<HashMap<String, CacheEntry>>
├── keytab.rs               # EXISTING — untouched
├── sso.rs                  # EXISTING — untouched (SPNEGO accept only)
└── discovery.rs             # EXISTING — untouched (derive_base_dn reusable if group DN needs deriving)

crates/trackly-app/src/services/auth.rs
└── AuthService                # gains `directory: Arc<dyn AdDirectory + Send + Sync>` field
                                #   mirrors existing `ad_client: Arc<dyn AdClient + Send + Sync>`
                                #   wired via the same TRACKLY_AD_MOCK env-var switch in context.rs
```

### Pattern 1: Service-account bind + search (adapted from `real.rs`'s user-bind path)

**What:** Bind with a FIXED service DN/password (not the end user's), then search for the
target `sAMAccountName`.
**When to use:** Any lookup where the acting principal is the service account, not the
logged-in user (SSO has no user password to bind with — this is the ONLY option).
**Example (translation of the existing `real.rs` shape — placeholders only):**

```rust
// Source: adapted from crates/trackly-infra/src/ad/real.rs:59-133 (existing user-bind
// pattern) + adwebapp/internal/auth/ldap.go:108-144 (service-bind reference, read-only,
// translated to Rust/ldap3 — NOT a verbatim port, ldap3's API shape differs from go-ldap).
use ldap3::{ldap_escape, LdapConnAsync, LdapConnSettings, Scope, SearchEntry};

async fn resolve_display_name_and_groups(
    cfg: &AdConfig,           // extended with bind_dn / bind_password / group mappings
    sam_account_name: &str,
) -> Result<Option<DirectoryEntry>, DirectoryError> {
    let settings = LdapConnSettings::new()
        .set_conn_timeout(CONN_TIMEOUT)
        .set_no_tls_verify(cfg.no_tls_verify);
    let url = format!("ldaps://{}:{}", cfg.host, cfg.port);

    let (conn, mut ldap) = LdapConnAsync::with_settings(settings, &url)
        .await
        .map_err(|_| DirectoryError::Unreachable)?;
    ldap3::drive!(conn); // Pitfall 7 — MUST drive the connection task

    // Service-account bind — fixed DN, e.g. "svc-trackly-ro@example.local" or
    // "cn=svc-trackly-ro,ou=Service Accounts,dc=example,dc=local".
    let bind_result = ldap
        .simple_bind(&cfg.bind_dn, &cfg.bind_password)
        .await
        .map_err(|_| DirectoryError::Unreachable)?;
    if bind_result.success().is_err() {
        return Err(DirectoryError::ServiceBindFailed); // config error, not user error
    }

    let filter = format!(
        "(&(objectClass=user)(sAMAccountName={}))",
        ldap_escape(sam_account_name)
    );
    let (entries, _res) = ldap
        .search(&cfg.base_dn, Scope::Subtree, &filter, vec!["displayName", "cn", "memberOf"])
        .await
        .map_err(|_| DirectoryError::Unreachable)?
        .success()
        .map_err(|_| DirectoryError::Unreachable)?;

    let _ = ldap.unbind().await;
    Ok(entries.into_iter().next().map(SearchEntry::construct).map(DirectoryEntry::from))
}
```

### Pattern 2: Group membership via `LDAP_MATCHING_RULE_IN_CHAIN`

**What:** One filter clause that asks the DC to expand nested group membership server-side.
**When to use:** ANY AD group-membership check where nested groups matter (they almost
always do in real orgs — direct-member-only checks silently under-grant/miss).
**Example (adapted from `adwebapp`'s `queryGroupMembership`, `ldap.go:235-267`):**

```rust
// Source: translated from adwebapp/internal/auth/ldap.go (read-only reference,
// placeholders only — no real domain/group names).
fn build_group_membership_filter(sam_account_name: &str, group_dn: &str) -> String {
    format!(
        "(&(objectClass=user)(sAMAccountName={})(memberOf:1.2.840.113556.1.4.1941:={}))",
        ldap3::ldap_escape(sam_account_name),
        ldap3::ldap_escape(group_dn),
    )
}
// A non-empty result set == member (including nested). Empty == not a member.
// Resolving a bare group name (not yet a DN) to its DN first requires one extra
// search: (&(objectClass=group)(sAMAccountName=<group>)) -> read `distinguishedName`.
// Prefer configuring the FULL group DN directly in trackly.config.toml to skip this
// extra round trip on every cache-miss (adwebapp supports both; Trackly can require
// DN-only for v1 simplicity and document the tradeoff).
```

### Anti-Patterns to Avoid

- **Recursive manual `memberOf` walk:** Don't fetch a user's direct `memberOf` list and then
  separately query each group's own `memberOf` to simulate nesting. `LDAP_MATCHING_RULE_IN_CHAIN`
  does this in one round trip and is what the reference implementation already validated.
- **Treating group-lookup failure as "no groups → default role is fine":** SSO-03's fail-closed
  requirement is explicit: on directory unreachable, the user must land on the SAME
  pending/Сотрудник path as if they had no elevated group — never treat "I couldn't check" as
  "they're not a member, but let's give them Manager anyway by some other heuristic," and never
  silently swallow the error into an `Ok(false)` that looks identical to "checked, not a member"
  in logs (log it as an explicit directory-unreachable event, distinguishable from a genuine
  non-member result, so an admin/observability signal exists — same spirit as the existing
  `AuthOutcome::Unreachable` vs `BadCreds` distinction).
- **Reusing the SAME LDAP connection object across the cache-refill AND the accept_spnego path:**
  keep these fully separate — `accept_spnego` (sso.rs) does an OFFLINE keytab decrypt with NO
  network client at all (`OfflineNetworkClient` errors loudly on any network attempt, by
  design — see `ad/sso.rs` module doc). The new service-bind directory lookup is a SEPARATE,
  intentional network LDAP connection. Do not let these two code paths share state or
  accidentally trigger a KDC call from the offline acceptor.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Nested AD group expansion | Custom recursive group-walk with cycle detection | `memberOf:1.2.840.113556.1.4.1941:=<DN>` filter (AD-side, one query) | AD implements this natively; a hand-rolled walk duplicates DC-side logic, is slower, and risks infinite loops on circularly-nested groups (a real, if rare, AD misconfiguration) |
| LDAP filter string construction | Manual string interpolation of user input into a filter | `ldap3::ldap_escape()` — already used in `real.rs::build_user_search_filter` | LDAP filter injection (RFC 4515 metacharacters `( ) * \ NUL`) is a real, tested vulnerability class in THIS codebase already (see `real.rs` tests `injection_payload_metacharacters_are_escaped`) — the new service-bind search filter needs the identical treatment, do not skip it just because the input now comes from an already-Kerberos-authenticated username rather than raw user input (defense in depth — a compromised/malformed keytab-validated username is still attacker-influenced in the worst case) |
| Bind-name normalization (UPN vs `DOMAIN\user` vs bare) | New ad-hoc parsing | Reuse `RealAdClient::normalize_bind_name` pattern (or extract to a shared free function) for the SERVICE account's own bind DN normalization if the config allows a bare service account name | Consistency; the existing function already encodes the Pitfall-6 lesson |

**Key insight:** Everything genuinely new in this phase (TTL cache, group→role config mapping,
fail-closed wiring at the `AuthService` call site) is thin, single-purpose glue code that
SHOULD be hand-rolled — small, obviously-testable, no external-dependency payoff. Everything
that touches AD protocol semantics (filter escaping, nested-group resolution) has a well-known
correct answer that the codebase (or the reference project) has ALREADY implemented — reuse
it exactly, don't reinvent it.

## Common Pitfalls

### Pitfall 1: `AdConfig`'s `#[derive(Debug)]` will leak a new service-account password

**What goes wrong:** `AdConfig` currently derives `Debug` (`crates/trackly-infra/src/config.rs`,
`#[derive(Debug, Deserialize, Clone)]` on the struct, line ~130). If a new
`bind_password: String` field is added naively, ANY future `tracing::debug!("{:?}", config)`
or accidental `{config:?}` in an error message will print the service-account password in
plaintext to logs (which, per CLAUDE.md, land in `./logs/` next to the executable — files
that could be shared, screenshotted, or attached to a support ticket).
**Why it happens:** `derive(Debug)` is field-blind; it has no concept of "this one is sensitive."
**How to avoid:** Either (a) implement `Debug` for `AdConfig` manually (redact `bind_password`
as `"***"`, matching the existing `Secret<T>` convention's OUTPUT even though `Secret<T>`
itself can't be used directly here — it forbids `Deserialize`, see `secret.rs` lines 7-9 and
53-55), or (b) parse into a private `#[derive(Deserialize)]` shadow struct and immediately
move the password into a wrapper type with a manual redacting `Debug`, never storing the raw
`String` in a struct that derives `Debug`. Also audit whether `AppConfig` (the parent struct)
derives `Debug` and would transitively leak through it (`AppConfig` does NOT currently derive
Debug — verify this stays true, or apply the same redaction there too).
**Warning signs:** Any `{:?}` format specifier touching `ctx.config` or `AdConfig` anywhere in
new code; grep for `config:?` before merging.

### Pitfall 2: Group-DN resolution requires an extra query (if group is configured by name, not DN)

**What goes wrong:** If the group→role config table stores a bare group name (e.g. `"IT-Admins"`)
instead of a full DN, every group-membership check needs a PRIOR lookup to resolve that name
to its `distinguishedName` (as `adwebapp`'s `resolveGroupDN` does) before the actual membership
filter can be built — doubling LDAP round trips per cache-miss.
**Why it happens:** `LDAP_MATCHING_RULE_IN_CHAIN` requires a DN, not a bare `sAMAccountName`,
on the right-hand side of the filter.
**How to avoid:** For v1 simplicity, REQUIRE the config to specify the full group DN directly
(document the format clearly: `CN=IT-Admins,OU=Groups,DC=example,DC=local` — placeholder only).
This avoids the extra query entirely and matches how admins typically copy a DN straight from
`ADUC`/PowerShell (`Get-ADGroup`). If the planner wants to support bare names too (nicer UX),
budget an extra cached lookup for group-DN resolution, itself with its own (longer) TTL since
group DNs essentially never change.

### Pitfall 3: Cache must be keyed by normalized login, not the raw Kerberos principal

**What goes wrong:** `sspi`'s `query_context_names()` (in `ad/sso.rs::accept_spnego`, line
152-158) may return the username in a different case or format (UPN-like) than the
`sAMAccountName` the service-bind search expects. If the cache key doesn't match how
`RealAdClient::normalize_bind_name`/`MockAdClient::lookup_key` already normalize logins
elsewhere in the codebase, you get cache misses on every request (functionally correct but
defeats the whole point of caching) OR, worse, two cache entries for what should be one user.
**Why it happens:** AD is famously case-insensitive and multi-format (`sAMAccountName`,
UPN, NetBIOS `DOMAIN\user`) for the "same" identity.
**How to avoid:** Normalize (lowercase, strip `@domain`/`DOMAIN\` prefix — reuse
`MockAdClient::lookup_key`'s exact logic, lines 69-78 of `mock.rs`) BEFORE using the login as
a cache key or as the search filter value. Write a test that asserts `us100`, `us100@example.local`,
and `EXAMPLE\us100` all hit the SAME cache entry.

### Pitfall 4: Fail-closed must distinguish "directory unreachable" from "user has zero matching groups"

**What goes wrong:** If the group-check function returns `Ok(false)` for BOTH "the DC could
not be reached" and "the DC was reached and confirmed the user is in no configured group,"
the caller cannot log/alert on directory outages distinctly from normal "employee, no special
group" outcomes — an ops-visibility regression, and it also makes it easy to accidentally
special-case "false → no error → proceed as before" in a way that doesn't actually route
through the fail-closed pending/Сотрудник path if a future refactor changes the call site.
**Why it happens:** Collapsing a 3-state outcome (member / not-member / unreachable) into a
2-state `bool` return type.
**How to avoid:** Return a proper `Result<bool, DirectoryError>` (or a 3-variant enum,
mirroring `AuthOutcome`'s `Ok`/`BadCreds`/`Unreachable` shape already established in
`trackly-core::ports::ad`). The CALLER (`AuthService`) then explicitly matches
`Err(DirectoryError::Unreachable) => do not elevate role, proceed as pending/Сотрудник` —
this is exactly the `adwebapp` reference's own documented distinction (`ldap.go` module
comment, lines 8-13: "ошибка LDAP здесь НЕ откатывается на 'тихое разрешение'").

### Pitfall 5: Don't gate on `ad.sso_enabled` twice with different semantics

**What goes wrong:** `sso.rs::handler_ad_sso` already gates on `ad_sso_enabled()` (an
`app_settings`-backed live toggle) AND `ad.spn`/`ad.keytab_path` non-empty (bootstrap TOML).
The NEW service-account directory lookup needs its OWN "is this configured" gate (bind DN +
password + base DN present) — but this must degrade GRACEFULLY (display_name falls back to
login, exactly as documented in the existing `sso_login` NOTE) rather than making the whole
SSO login fail with a 503 just because the OPTIONAL directory-enrichment feature isn't
configured. Only the KERBEROS gate should hard-503; the directory-enrichment gate should
soft-degrade.
**How to avoid:** Treat "directory bind not configured" as a THIRD outcome distinct from
"directory unreachable" — both degrade `display_name` to login, but only genuine
network/bind failures during an ATTEMPTED lookup should be logged as an operational warning;
"not configured" is an expected, silent, zero-log state (mirrors `ldap.go`'s `InitLDAP`
early-return-with-log-once, not per-request logging).

## Code Examples

### TTL cache skeleton (hand-rolled, mirrors `ReaderPool`'s `Mutex<Vec<..>>` convention)

```rust
// Source: pattern adapted from adwebapp/internal/auth/ldap.go's cache (Mutex<HashMap>,
// lines 38-48 + 92-106) — same shape, translated to Rust idioms; no new crate needed.
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub struct DirectoryCacheEntry {
    pub display_name: String,
    pub role: Option<trackly_core::auth::Role>,
    expires_at: Instant,
}

pub struct DirectoryCache {
    entries: Mutex<HashMap<String, DirectoryCacheEntry>>,
    ttl: Duration,
}

impl DirectoryCache {
    pub fn new(ttl: Duration) -> Self {
        Self { entries: Mutex::new(HashMap::new()), ttl }
    }

    pub fn get(&self, key: &str) -> Option<(String, Option<trackly_core::auth::Role>)> {
        let map = self.entries.lock().expect("cache mutex poisoned");
        map.get(key)
            .filter(|e| e.expires_at > Instant::now())
            .map(|e| (e.display_name.clone(), e.role.clone()))
    }

    pub fn put(&self, key: String, display_name: String, role: Option<trackly_core::auth::Role>) {
        let mut map = self.entries.lock().expect("cache mutex poisoned");
        map.insert(key, DirectoryCacheEntry {
            display_name,
            role,
            expires_at: Instant::now() + self.ttl,
        });
    }
}
```

Note: `Instant` (monotonic, process-local) is correct here — NOT `time::OffsetDateTime`/UTC
timestamps used elsewhere in the DB layer (CLAUDE.md picks `time` crate for wall-clock
persistence; this cache is purely in-memory/ephemeral, so `std::time::Instant` is the right
tool and does not need to go through the `Clock` trait abstraction used for testable
wall-clock DB timestamps elsewhere in the codebase — BUT this means tests that want to
assert TTL EXPIRY will need to either sleep (flaky) or inject a fake clock. Recommend adding
an injectable `now: impl Fn() -> Instant` seam, OR structuring `DirectoryCache` generically
enough that tests can construct it with an artificially-short TTL (e.g. 10ms) and a real
short sleep — simpler and matches the "seconds-scale test" convention already used
elsewhere in this codebase's async tests.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|---------------|--------|
| `sso_login(ad_username, ad_username)` — display_name == login | `sso_login` resolves display_name (and role) via service-bind directory lookup | This phase | `UserDto.full_name` for SSO users becomes a real ФИО; no frontend changes needed — `full_name` already flows through `UserDto`/session/`build_auth_status` unchanged |
| Role always hardcoded `'employee'` on auto-register (`auto_register_ad_user`/`create_pending_registration`, `auth.rs:507`/`578`) | Role determined by group→role mapping BEFORE insert, falling back to `'employee'` when no group matches or directory unreachable | This phase | The two INSERT statements at `auth.rs:503-509` and `auth.rs:574-580` need their hardcoded `'employee'` literal replaced with a resolved role value — this is the primary code-change surface for SSO-03 |

**Deprecated/outdated:** none — this is additive to a live, current subsystem, not a
replacement of an outdated one.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `ldap3 0.12.1`'s async `search()` API accepts a `memberOf:1.2.840.113556.1.4.1941:=<DN>` filter clause exactly as a raw filter string, the same way `real.rs`'s existing search does — no special API support needed beyond what's already used | Architecture Patterns / Pattern 2 | Low — `ldap3` treats filters as opaque RFC 4515 strings passed to the DC; this is a protocol-level feature the DC (not the client library) implements, so any client capable of sending an arbitrary filter string (which `real.rs` already proves) supports it. Still `[ASSUMED]` because it has not been tested against a live Rust+`ldap3`+real-AD group query in THIS codebase yet (only the adwebapp Go reference has been live-verified for this specific query shape) |
| A2 | A hand-rolled `Mutex<HashMap<..>>` TTL cache is sufficient at Trackly's LAN/~20-concurrent-user scale, no eviction/size-limit logic needed | Standard Stack | Low-medium — if the org has hundreds of distinct AD accounts hitting SSO, the map grows unbounded (no LRU eviction). Given CLAUDE.md's stated ~20-concurrent-user LAN scale, this is very unlikely to matter in practice, but flag it as a known simplification, not a silent limitation |
| A3 | Group→role mapping should live in `trackly.config.toml` (bootstrap config, admin-edited on disk), NOT in `app_settings` (DB-backed, admin-edited via UI) | Architectural Responsibility Map / Open Questions | Medium — if the user/planner actually wants a Settings-UI-editable mapping table (like `ad_auto_accept` is UI-editable while `host`/`domain`/etc. are TOML-only-read-only-displayed), this assumption steers the whole config-schema and UI-surface decision. See Open Questions #1. |
| A4 | The group DN should be configured as a FULL distinguished name in config (not a bare group name requiring an extra resolve-to-DN LDAP query) | Common Pitfalls #2 | Low-medium — if the planner/user prefers bare group names for UX (matching how `adwebapp` supports both), an extra cached DN-resolution step must be added; not a big change but changes the config schema shape (`group_dn` vs `group_name`) |
| A5 | The service account should use **simple bind over LDAPS** (same TLS approach as the existing password-login path), not GSSAPI/Kerberos for the service account itself | Alternatives Considered | Low — this mirrors the EXISTING `real.rs` pattern exactly and is explicitly the CLAUDE.md-documented approach (`ldap3` simple bind for LDAPS; GSSAPI/NTLM features are deferred/gated separately). Using a service KEYTAB for the LDAP bind itself (rather than a bind DN+password) would be a bigger architectural change and is not what "служебная учётная запись" (service account with bind params in config) implies in the phase's own success criteria wording |

**If this table is empty:** N/A — see entries above; the two HIGH-risk items for the planner
to explicitly confirm/lock as decisions before or during planning are A3 (config vs DB for
role mapping) and A4 (DN vs bare-name for group config).

## Open Questions (RESOLVED)

> All three open questions were resolved by adopting the research recommendations below; the
> Phase 31 plans (31-01..31-04) faithfully implement each. No `/gsd-discuss-phase 31` override
> was requested, so the recommendations stand as the locked decisions.

1. **Should the group→role mapping be editable via the Settings UI, or TOML-only?**
   **RESOLVED:** TOML-only. Plans implement `[[ad.role_mapping]]` in `trackly.config.toml`
   (bootstrap-only, matching the existing `host`/`base_dn` read-only-displayed precedent); no
   CRUD-mapping UI is built in this phase.
   - What we know: `ActiveDirectorySettings.svelte`'s own code comment explicitly documents
     the existing split — `enabled`/`auto_accept` are DB-backed/UI-writable via
     `settings_set_ad`; `host`/`port`/`domain`/`base_dn`/`name_attr`/`no_tls_verify`/SSO
     fields are **TOML-only, read-only-displayed** in that same UI. The phase's own success
     criteria (ROADMAP.md Phase 31, criterion 3) say "настраиваемый маппинг" (configurable
     mapping) but nowhere requires an admin UI form for it — and SSO-02 (Phase 32) is the
     phase that explicitly says "Администратор может задать список" (implying a UI). SSO-03
     has no equivalent "admin can set via UI" wording.
   - What's unclear: whether "configurable" here means "editable in `trackly.config.toml` by
     whoever manages the portable install" (matches existing host/domain/base_dn pattern) or
     "editable by an in-app admin without touching the filesystem."
   - Recommendation: Follow the EXISTING split precedent — group→role mapping goes in
     `trackly.config.toml` (bootstrap-only, like `host`/`base_dn`), read-only-displayed in the
     Settings UI for visibility (consistent with the current `ActiveDirectorySettings.svelte`
     pattern for all other AD connection parameters). This keeps the phase scoped to backend
     + read-only UI surface, avoiding a full CRUD-mapping-table UI feature that isn't in the
     stated success criteria. If the user wants UI-editable mapping, that should be an explicit
     `/gsd-discuss-phase 31` decision before planning, not an assumption baked into the plan.

2. **Does the cache need to be invalidated/bypassed by an admin action (e.g., "user was just
   removed from the AD group, but they're still cached as Manager for up to TTL minutes")?**
   - What we know: `adwebapp`'s reference uses a SHORTER TTL (5 min) for group-membership
     checks specifically BECAUSE "доступ ... может понадобиться отозвать оперативно" (access
     may need prompt revocation) — vs. a longer 30-min TTL for the cosmetic displayName cache.
     Trackly's phase description doesn't specify a TTL value.
   - What's unclear: what TTL Trackly should use, and whether there's a manual
     "clear AD cache" admin action needed (adwebapp's reference has none — it just waits out
     the TTL).
   - Recommendation: Mirror `adwebapp`'s split — separate (or independently configurable) TTLs
     for displayName (longer, e.g. 30 min, low security stakes) vs. group/role (shorter, e.g.
     5 min, direct authorization impact). No manual-invalidation admin action needed for v1 —
     defer to a future phase if operationally requested. Make BOTH TTLs configurable in
     `trackly.config.toml` (not hardcoded) so an admin can tune them without a rebuild.
   **RESOLVED:** Split, independently-configurable TTLs — `display_name_cache_ttl_secs`
   (default 1800s) and `group_cache_ttl_secs` (default 300s) in `trackly.config.toml`. No
   manual cache-invalidation admin action in v1.

3. **What role does a user get if they match MULTIPLE mapped groups (e.g., member of both
   "Managers" and "Admins")?**
   - What we know: Neither `adwebapp`'s reference (which only checks ONE group at a time for
     the Каталог access-gate use case) nor Trackly's phase description addresses multi-group
     precedence.
   - What's unclear: highest-privilege-wins vs. first-match-in-config-order vs. error-on-ambiguity.
   - Recommendation: Highest-privilege-wins (Admin > Manager > Employee) is the safest,
     least-surprising default and requires checking groups in a fixed priority order, short-
     circuiting on the first (highest) match — this also minimizes LDAP round trips (check
     the Admin-mapped group first; only check Manager-mapped group if Admin check is negative).
     Confirm with the user during `/gsd-discuss-phase 31` if this matters to them operationally.
   **RESOLVED:** Highest-privilege-wins (Admin > Manager > Employee), implemented as a
   pure/unit-tested `pick_highest_role` helper in Plan 31-02.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Real AD / Domain Controller | Live service-bind + group query testing | ✗ (dev macOS, per memory: "No AD or target printers reachable from dev macOS") | — | `MockAdDirectory` (new, mirrors existing `MockAdClient`/`MockSnmpClient` pattern) gated by `TRACKLY_AD_MOCK` env var, already the established dev/CI convention |
| `ldap3` crate | Compile-time dependency | ✓ | `0.12.1` (pinned) | — |
| Windows test machine (per user's separate-machine AD/SNMP testing convention) | Final live verification of group-membership query against real nested AD groups | Not verified from this research session — user-owned resource per memory `dev_environment_constraints` | — | Live verification must happen there before this phase can be closed as fully proven, same caveat already flagged on the existing SPNEGO code ("BUILD-VERIFIED, NOT LIVE-VERIFIED") |

**Missing dependencies with no fallback:** none — the mock path is a complete, already-proven
substitute for all unit/integration-test purposes; only FINAL live-AD verification has no
local fallback (expected and already the established project pattern for all AD work).

**Missing dependencies with fallback:** Real AD/DC — `MockAdDirectory` fixtures.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | `cargo test` (workspace), `cargo nextest` optionally per CLAUDE.md dev-tools table |
| Config file | `crates/trackly-app/tests/*.rs` (integration tests), unit tests inline in `#[cfg(test)] mod tests` blocks per module (existing convention in `real.rs`, `mock.rs`, `sso.rs`) |
| Quick run command | `cargo test -p trackly-infra ad::` (module-scoped) / `cargo test -p trackly-app --test ad_auth` (existing file — extend, or add a new `ad_directory.rs` integration test file) |
| Full suite command | `cargo test --workspace` (per project convention — note memory `cargo_no_concurrent_test`: never run two `cargo test` invocations concurrently, they contend on the `target/` lock) |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| SSO-01 | `MockAdDirectory` resolves a known `sAMAccountName` to its fixture displayName | unit | `cargo test -p trackly-infra ad::directory_mock::tests` | ❌ Wave 0 |
| SSO-01 | Unknown `sAMAccountName` falls back to the login itself (no crash, no panic) | unit | same as above | ❌ Wave 0 |
| SSO-01 | Cache hit avoids a second directory call (assert call-count via a spy/mock) | unit | `cargo test -p trackly-infra ad::cache::tests` | ❌ Wave 0 |
| SSO-01 | Cache entry expires after TTL and triggers a fresh lookup | unit | same as above (short-TTL injection) | ❌ Wave 0 |
| SSO-01 | `sso_login()` end-to-end: SSO login now shows resolved displayName, not bare login | integration | `cargo test -p trackly-app --test ad_directory_sso` (new file) | ❌ Wave 0 |
| SSO-03 | User in configured group gets the mapped role on FIRST (auto-register) login | integration | same new integration file | ❌ Wave 0 |
| SSO-03 | User in NO configured group gets default `'employee'` (unchanged behavior) | integration | same file — regression test against EXISTING `auto_register_ad_user` behavior | ❌ Wave 0 (extend existing `ad_register.rs`/`requests_ad_register.rs`?) |
| SSO-03 | Directory unreachable during group check → role NOT elevated, user still lands on pending/Сотрудник path (fail-closed) | integration | `MockAdDirectory::unreachable()`-style fixture (mirrors `MockAdClient::unreachable()`) | ❌ Wave 0 |
| SSO-03 | Directory unreachable does NOT surface as a silent auth failure — error is typed/loggable, not swallowed | unit | assert on the `DirectoryError` variant returned, not just a boolean | ❌ Wave 0 |
| Privacy (SC #5) | Fixtures/tests use ONLY placeholder domains/logins/names (e.g. `example.local`, `svc-trackly-ro`, `us100`/`Иванов Иван Иванович` — reuse EXISTING mock fixture names already in git) | manual review | grep-based CI check (optional) or code-review checklist item | N/A — policy, not a runnable test |

### Sampling Rate

- **Per task commit:** `cargo test -p trackly-infra ad::` + `cargo test -p trackly-app --test ad_directory_sso` (whichever integration file name is chosen)
- **Per wave merge:** `cargo test --workspace` + `cargo clippy --workspace -- -D warnings` + `cargo fmt --check`
- **Phase gate:** Full suite green, plus the `no_io_deps.rs` test re-run (new `AdDirectory` port must stay `ldap3`-free in `trackly-core`) before `/gsd-verify-work`

### Wave 0 Gaps

- [ ] `crates/trackly-core/src/ports/ad_directory.rs` — new port trait + `DirectoryError`/`DirectoryResult` types (mirrors `ports/ad.rs`)
- [ ] `crates/trackly-infra/src/ad/directory.rs` — `RealAdDirectory` impl
- [ ] `crates/trackly-infra/src/ad/directory_mock.rs` — `MockAdDirectory` impl + deterministic fixtures (extend `mock.rs`'s existing `us100`/`us200` fixtures with group-membership data rather than inventing new fixture identities — keeps privacy-placeholder discipline consistent with the ALREADY-in-git fixture names)
- [ ] `crates/trackly-infra/src/ad/cache.rs` — TTL cache module + unit tests
- [ ] New integration test file in `crates/trackly-app/tests/` covering the `sso_login` → directory → `on_ad_bind_success` → role-mapped `UserDto` path end-to-end (mirrors `ad_auth.rs`'s `make_auth_service_with_ad`/`seed_ad_user` test-seam pattern, extended with a directory mock)
- [ ] `trackly.config.toml.example` — add placeholder `[ad]` section fields for `bind_dn`, `bind_password` (or a clearly-named placeholder), `role_mapping` table, and cache TTLs — current example file (`trackly.config.toml.example`, lines 1-16) doesn't even show the `[ad]`/`[server]` sections that already exist in code; this phase should bring it up to date, not just add new fields to a stale example

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-------------------|
| V2 Authentication | yes (indirectly) | No new authentication mechanism — SPNEGO/Kerberos auth is out of scope/already built; this phase only ENRICHES an already-authenticated identity. No password handling changes. |
| V3 Session Management | no (unchanged) | `tower-sessions` cookie issuance unchanged; only the DATA fed into the session (via `UserDto`) changes |
| V4 Access Control | yes | THE core of SSO-03 — group→role mapping directly determines RBAC (`trackly_core::auth::{Role, authorize}`). Standard control: fail-closed on any resolution failure (never fail-open to an elevated role); least-privilege service account (read-only AD bind, no write permissions needed or requested) |
| V5 Input Validation | yes | LDAP filter injection defense — reuse `ldap3::ldap_escape()` for BOTH the `sAMAccountName` search filter AND the group-DN value interpolated into the `memberOf:1.2.840.113556.1.4.1941:=` clause (do not assume the Kerberos-validated username is "safe" — escape it anyway, defense in depth, matches existing `real.rs` test coverage) |
| V6 Cryptography | no direct change | Service-account bind reuses the EXISTING LDAPS/rustls transport config (`no_tls_verify` flag, `tls-rustls-ring` ldap3 feature) — no new crypto surface |
| V9 Data Protection (secrets at rest/in logs) | yes | New service-account PASSWORD in config — must never leak via `Debug`/logs (see Common Pitfalls #1); must live only in the gitignored `trackly.config.toml`, never in git (SC #5 of this phase is explicitly this requirement) |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|----------------------|
| LDAP filter injection via a crafted `sAMAccountName`/group-DN value | Tampering | `ldap3::ldap_escape()` on every interpolated value — already proven pattern in `real.rs`, extend identically |
| Config-file secret leakage via Debug/logs | Information Disclosure | Manual redacting `Debug` impl on `AdConfig` (see Pitfall 1); never log the whole config struct |
| Fail-open privilege escalation on directory outage | Elevation of Privilege | Explicit typed `DirectoryError::Unreachable` variant, matched at the `AuthService` call site to force the non-elevated path — never a boolean collapse (Pitfall 4) |
| Service account over-privileged (write access, or bound as Domain Admin for convenience) | Elevation of Privilege | Document in the config example/comments that the service account MUST be a dedicated, read-only, least-privilege AD account — this is an operational/AD-admin-side control the code cannot enforce, but the config example and any setup docs should say so explicitly (mirrors `adwebapp`'s own `bindDN`/`bindPassword` comment: `"CORP\\svc-readonly"`) |
| Cache poisoning via a race between two concurrent SSO logins for the same user during a cache-miss refill | Tampering (low severity) | `Mutex`-guarded single-writer-per-key semantics are enough at this scale — no need for anything more sophisticated; note that a duplicate concurrent LDAP lookup (both requests miss cache, both query AD) is a performance nit, not a correctness bug (idempotent read), so this is a documented non-issue, not a required fix |

## Sources

### Primary (HIGH confidence)
- `crates/trackly-infra/src/ad/real.rs` (this repo) — existing service-bind-adjacent pattern (user bind + search + fallback chain), lines 1-254
- `crates/trackly-infra/src/ad/mock.rs` (this repo) — mock pattern to mirror for `MockAdDirectory`, lines 1-268
- `crates/trackly-app/src/http/sso.rs` (this repo) — exact SSO hook point (`issue_sso_session`, line 66-95; `sso_login` call, line 71)
- `crates/trackly-app/src/services/auth.rs` (this repo) — `sso_login`/`on_ad_bind_success`/`auto_register_ad_user`/`create_pending_registration`, lines 265-620 — the provisioning seam this phase extends
- `crates/trackly-core/src/ports/ad.rs` (this repo) — `AdClient`/`AuthOutcome` port shape to mirror for the new `AdDirectory` port
- `crates/trackly-infra/src/config.rs` (this repo) — `AdConfig` struct to extend, lines 126-186
- `crates/trackly-infra/src/ad/discovery.rs` (this repo) — `derive_base_dn` reusable helper, lines 25-35
- `crates/trackly-core/src/primitives/secret.rs` (this repo) — `Secret<T>` contract explaining why it can't be derived-Deserialize'd directly onto config, lines 1-56
- `crates/trackly-core/tests/no_io_deps.rs` (this repo) — the enforced hexagonal-boundary test the new `AdDirectory` port must also satisfy
- `/Users/madsas/Projects/llm-projects/adwebapp/internal/auth/ldap.go` (reference project, READ-ONLY, no secrets copied) — canonical service-bind + `LDAP_MATCHING_RULE_IN_CHAIN` group-check algorithm to translate to Rust
- `cargo search ldap3` (run 2026-08-03) — confirms `0.12.1` is current, no newer release found

### Secondary (MEDIUM confidence)
- LDAP `LDAP_MATCHING_RULE_IN_CHAIN` OID `1.2.840.113556.1.4.1941` — well-known Microsoft AD extended matching rule; behavior confirmed by BOTH the adwebapp reference implementation AND general LDAP/AD documentation knowledge, but not independently re-verified via Context7/official Microsoft docs in this research session `[CITED via training knowledge + reference implementation cross-check, not independently re-fetched]`

### Tertiary (LOW confidence)
- None flagged — all claims above trace to either this codebase, the reference project, or a directly-runnable verification command (`cargo search`)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new dependencies, `ldap3` API surface already proven in this exact codebase
- Architecture: HIGH — hook points (`sso_login`, `on_ad_bind_success`) are concrete, cited, and already flagged by an in-code TODO comment written by the previous phase's author
- Pitfalls: MEDIUM-HIGH — most pitfalls are directly derived from patterns already defended against elsewhere in this codebase (filter injection, config Debug leaks are a NEW risk this phase introduces, not yet defended); the LDAP-protocol-level group-query behavior itself is MEDIUM confidence (well-documented technique, but not yet live-fired from Rust/`ldap3` against a real AD in this project — same caveat the existing SPNEGO code already carries)

**Research date:** 2026-08-03
**Valid until:** 30 days (stable protocol/library surface; re-check `ldap3` version currency if planning is delayed past that window)

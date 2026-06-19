# Phase 9: AD-аутентификация и заявки на регистрацию пользователей — Research

**Researched:** 2026-06-19
**Domain:** LDAP/AD authentication (ldap3 simple_bind over LDAPS), user-registration request lifecycle, mock adapter for macOS dev
**Confidence:** HIGH (stack + integration points verified against codebase and ldap3 0.12.1 docs); MEDIUM on AD auto-detect details (no real domain reachable from dev)

<user_constraints>
## User Constraints (from 09-CONTEXT.md)

### Locked Decisions (research HOW, not WHETHER)
- **D-AD-01:** v1 = `ldap3 simple_bind` ONLY. Auto-SSO (Kerberos/NTLM Negotiate) DEFERRED to v2 (ADV-01). `trait AdClient` leaves room for a future SSO adapter — do NOT enable `gssapi`/`ntlm` Cargo features now.
- **D-UX-01:** single login form; server tries local argon2id first, falls back to AD `simple_bind` when AD enabled. One block serves local AND domain users.
- **D-UX-02:** «Запомнить меня» — persistent (sliding 30d) vs session cookie. (Session infra exists from Phase 5; this is UI/cookie-policy wiring.)
- **D-UX-03:** «Войти как \<display name\>» button — v2 only (reserve UI space, no logic).
- **D-REG-01:** two registration modes toggled in Settings (SET-10/USR-11): **auto-register** (create user immediately, role=employee, password_hash=NULL, + informational `ad_register` request whose Reject = soft-delete the user) vs **pending-approval** (`ad_register` request, user sees pending screen, not admitted until approve).
- **D-REG-02:** approval in «Заявки» section; `ad_register` visible to admin only (REQ-06); approve modal selects role, default employee. Reuses Phase 6 lifecycle.
- **D-REG-03:** restoration-of-access flow IS in scope (blocked/deleted AD user → message + «Запрос на восстановление доступа» + «Войти под другим»). Open question to answer below: reuse `ad_register` with sub-flag vs new `request_type`.
- **D-Config-01:** AD connection = **auto-detect-first**. NO manual LDAP config in the happy path. Auto-detect domain + DC + base DN from a domain-joined Windows host. Manual override under «Расширенные». Deliverable: short AD setup doc.
- **D-Config-02:** ФИО attribute `displayName` → `cn` → login; attribute name configurable (default `displayName`).
- **D-Mock-01:** `MockAdClient` mirrors `MockSnmpClient` with fixtures + error scenarios (success / wrong password / not found / server unreachable). Switch via `config.ad.use_mock || TRACKLY_AD_MOCK`.
- **D-Sec-01:** AD password NEVER stored — `Secret<T>`, bind-only, `password_hash=NULL` for AD users (V002 already supports `ad_user` + nullable `password_hash`).
- **Scope:** AD login is **web-only** (desktop stays trusted-admin / local login as Phase 5). MVP vertical slice.

### Claude's Discretion
- Exact AD settings set + auto-detect mechanism (DNS SRV / env / manual override).
- Restoration: sub-flag on `ad_register` vs new `request_type` (D-REG-03) — **recommendation below**.
- Login/pending/blocked screen copy + layout (within UI-SPEC patterns).
- Format + location of the AD setup doc (`docs/AD-SETUP.md` vs README section).
- LDAPS vs LDAP default + self-signed AD cert handling in LAN — **recommendation below**.

### Deferred Ideas (OUT OF SCOPE)
- Auto-SSO (one-click «Войти как …», passwordless) — v2 ADV-01 (Kerberos/NTLM Negotiate).
- AD login in desktop mode — v2.
- SMTP/email notifications for registration requests — v2 (NTF-02); in-app REQ-04 is enough.
- AD-group → role mapping — potential v2.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| USR-08 | Доменный пользователь входит через браузер по AD-логину + паролю; bind через `ldap3 0.12 simple_bind`; пароль не хранится | §Standard Stack (ldap3 0.12.1, tls-rustls), §Pattern 1 (simple_bind), §Pitfall 1 (empty-password trap), §Code Examples |
| USR-09 | Незарегистрированный AD-юзер → заявка на регистрацию (REQ-06); админ подтверждает + назначает роль | §Data Model, reuse `requests` (V006 `ad_register` already in CHECK), §Pattern 4 (AuthService fallback) |
| USR-10 | Подтягивание ФИО из AD (`displayName` → `cn` → login) | §Pattern 2 (search after bind), §Code Examples (search) |
| USR-11 | Настройка «Автоприём заявок» → авто-создание юзера role=employee | §Data Model (auto-register branch), §Settings persistence (app_settings) |
| USR-12 | Mock AD-клиент для macOS-дева | §Pattern 3 (AdClient trait + MockAdClient mirrors MockSnmpClient), §Validation Architecture |
| REQ-06 | Заявки на регистрацию — отдельный подтип, видим только админу | §Data Model (admin-only filter on `request_type='ad_register'`), reuse Phase 6 request lifecycle |
| SET-10 | Настройка автоприёма (см. USR-11) | §Settings persistence (`app_settings` key/value upsert, like `desktop_lock_enabled`) |
</phase_requirements>

## Summary

This phase adds Active Directory authentication to Trackly's existing web login path and a registration-request workflow for unknown domain users. The technical core is small and well-bounded: an async LDAP `simple_bind` against a domain controller over LDAPS, an optional attribute search to pull the user's display name, and a runtime-switchable mock so the whole flow is testable on the dev macOS box with no real domain.

The codebase already contains every seam needed. The `users` table (V002) has `ad_user` and a nullable `password_hash` — no migration needed for AD users. The `requests` table (V006) already lists `ad_register` in its `request_type` CHECK — REQ-06 backing is in place. The mock pattern is fully exemplified by `SnmpClient`/`MockSnmpClient` (trait in `trackly-core/src/ports`, Real+Mock in `trackly-infra`, runtime switch in `AppCtx::build` via env var). `AuthService::login` (auth.rs:180) is the single integration hook — it already runs a constant-time anti-enumeration dummy-hash path that the AD branch must preserve. Settings persistence has a proven key/value upsert pattern (`desktop_lock_enabled` in auth.rs:823 + `low_stock_threshold` in V016).

The one stack decision that needs care: **ldap3's default feature is `tls` = native-tls (SChannel on Windows, OpenSSL elsewhere)**, which violates the project's "no OpenSSL/native-tls" portable discipline. We must build ldap3 with `default-features = false, features = ["tls-rustls-ring"]` (or `tls-rustls-aws-lc-rs`) to stay pure-Rust. `tls-rustls` pulls `rustls-native-certs`, which reads the **OS trust store** — meaning a corporate/enterprise CA already installed on a domain-joined Windows machine is trusted automatically, which is exactly the LAN deployment reality. That makes self-signed/corporate-CA AD certs "just work" without manual cert config in the common case.

**Primary recommendation:** Add `ldap3 0.12.1` with `default-features = false, features = ["tls-rustls-ring"]` + `hickory-resolver 0.26` for DNS SRV. Define `AdClient` trait in `trackly-core/src/ports/ad.rs` (mirror `snmp.rs`), `RealAdClient`/`MockAdClient` in `trackly-infra/src/ad/`, switch in `AppCtx::build` via `config.ad.use_mock || TRACKLY_AD_MOCK`. Integrate into `AuthService::login` as a local-fail → AD-bind fallback that preserves constant-time behavior. Reject empty passwords BEFORE bind (anonymous-bind trap). Store AD config + auto-accept flag in `app_settings`. For restoration (D-REG-03): **reuse `ad_register` with a status/sub-flag rather than a new request_type** (no migration, simpler).

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| AD bind (network LDAPS I/O) | API/Backend (`trackly-app` AuthService + `trackly-infra` RealAdClient) | — | Password must never leave the server; bind is async network I/O; web-only per D-AD-01 |
| AD attribute search (displayName) | API/Backend (RealAdClient, same bound session) | — | Same LDAP session as bind; no client involvement |
| Auto-detect domain/DC/base DN | API/Backend (`trackly-infra`, env + DNS SRV) | — | Reads server-host env vars (USERDNSDOMAIN) + DNS; runs where the server process lives |
| Local→AD fallback decision | API/Backend (`AuthService::login`) | — | Single shared business logic; both transports route through it (but only web exercises AD) |
| User auto-create / restoration | API/Backend (single-writer task) | — | All writes funnel through the writer worker (CLAUDE.md single-writer invariant) |
| Registration request lifecycle | API/Backend (`RequestService`, reuse Phase 6) | — | Reuses existing optimistic-lock + audit lifecycle |
| AD settings toggle + auto-accept flag | API/Backend (`app_settings` upsert) | Frontend Server (Settings UI tab) | Config lives in DB (portable); UI is a thin editor |
| Login form / pending / blocked screens | Browser/Client (Svelte SPA) | — | UX only; all auth decisions server-side (USR-06) |

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `ldap3` | `0.12.1` | Async LDAP client: `simple_bind` + `search` | `[CITED: CLAUDE.md "What NOT to Use"/stack table]` + `[VERIFIED: crates.io — cargo search → 0.12.1]`. Canonical pure-Rust LDAP for Tokio; matches MSRV ≥1.82 (1.85 if NTLM, which we do NOT enable). |
| `hickory-resolver` | `0.26.1` | DNS SRV lookup for DC discovery (`_ldap._tcp.dc._msdcs.<domain>`) | `[VERIFIED: crates.io — cargo search → 0.26.1]`. `trust-dns-resolver` was RENAMED to `hickory-resolver` (trust-dns 0.23.2 is the last legacy name) — use hickory, not trust-dns. Async, tokio-native, supports `srv_lookup`. |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `async-trait` | workspace (already present) | `#[async_trait]` on `AdClient` trait in core | Required — core is I/O-free; `async-trait` is the only allowed external dep in `trackly-core` ports (same as `SnmpClient`). `[VERIFIED: codebase — core/Cargo.toml line 24]` |
| `rustls` | `0.23.x` (already in stack) | TLS backend for ldap3 via `tls-rustls-ring` | Pure-Rust, no OpenSSL DLL (portable discipline). `[CITED: CLAUDE.md]` |
| `tokio` | `1.x` (already present) | async runtime for bind/search/DNS | Always |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `ldap3` tls-rustls-**ring** | `tls-rustls-aws-lc-rs` | aws-lc-rs needs a C/asm toolchain at build time (cmake/nasm on Windows) — more CI friction than `ring`. Pick `ring` unless a FIPS requirement appears (none here). `[CITED: github.com/inejge/ldap3/Cargo.toml features]` |
| `ldap3` default (`tls`=native-tls) | — | **REJECTED.** Default pulls native-tls → OpenSSL/SChannel; violates CLAUDE.md "no native-tls/OpenSSL, use rustls". MUST set `default-features = false`. |
| `hickory-resolver` | std-only / shelling to `nslookup` | std has no SRV record API; shelling out is fragile + packaging surface. hickory is the canonical async SRV resolver. |
| DNS SRV discovery | hardcode `<domain>:636` from `USERDNSDOMAIN` only | Env-only is a valid minimal fallback (DC is usually reachable at the domain name on a joined host), but SRV is the correct AD discovery mechanism. Do both: SRV first, env-domain fallback, manual override last. |

**Installation (`crates/trackly-infra/Cargo.toml`):**
```toml
ldap3 = { version = "0.12.1", default-features = false, features = ["tls-rustls-ring"] }
hickory-resolver = "0.26.1"
# async-trait already in workspace; add to trackly-core if AdClient lives there (it does)
```

> ⚠️ `default-features = false` is mandatory — it drops the `sync` and native-`tls` defaults. The async API needs `tokio/rt`, which `tls-rustls` already enables transitively.

**Version verification performed:**
- `cargo search ldap3` → `ldap3 = "0.12.1"` (matches CLAUDE.md pin) `[VERIFIED]`
- `cargo search hickory-resolver` → `0.26.1` `[VERIFIED]`
- `cargo search rustls` → `0.24.0-dev.0` is dev; stable line is `0.23.x` (already in project) `[VERIFIED]`
- ldap3 `[features]` block fetched from GitHub Cargo.toml `[CITED: github.com/inejge/ldap3]`

## Package Legitimacy Audit

> slopcheck not available in this research session (no network pip install attempted under sandbox); registry existence verified via `cargo search`. Both packages are long-established, high-trust crates referenced by CLAUDE.md (ldap3) or the canonical successor of a well-known crate (hickory = renamed trust-dns). Planner SHOULD still gate the first `cargo add` behind a quick human glance, but risk is low.

| Package | Registry | Age | Downloads | Source Repo | slopcheck | Disposition |
|---------|----------|-----|-----------|-------------|-----------|-------------|
| `ldap3` | crates.io | mature (0.x since ~2017) | high | github.com/inejge/ldap3 | n/a (unavailable) | Approved — pinned by CLAUDE.md `[ASSUMED→low risk]` |
| `hickory-resolver` | crates.io | mature (trust-dns lineage since 2015) | very high | github.com/hickory-dns/hickory-dns | n/a (unavailable) | Approved `[ASSUMED→low risk]` |

**Packages removed due to slopcheck [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

*slopcheck was unavailable; per protocol both packages are tagged `[ASSUMED]`. Given ldap3 is an explicit CLAUDE.md stack pin and hickory is the canonical trust-dns rename, the planner may treat the human-verify checkpoint as a 1-line confirmation rather than a deep audit.*

## Architecture Patterns

### System Architecture Diagram

```
Browser (LAN)  ──POST /api/v1/auth_login {req:{login,password}}──►  axum public_router (http/auth.rs)
                                                                          │
                                                                          ▼
                                                              AuthService::login(req)   ◄── single shared business logic
                                                                          │
                          ┌───────────────────────────────────────────────┤
                          ▼ (1) local lookup                                │
                 get_password_hash(login)                                   │
                   │ found → argon2id verify (spawn_blocking)               │
                   │ not found → dummy-hash verify (constant time)          │
                          │                                                  │
              local OK ◄──┤                                                  │
                          │ local FAIL                                       │
                          ▼ (2) AD enabled? (app_settings ad_enabled)        │
                   reject empty password  ◄── CRITICAL (anonymous-bind trap) │
                          │                                                  │
                          ▼                                                  │
                 Arc<dyn AdClient>.authenticate(login, Secret<pwd>)  ───────►│ TRACKLY_AD_MOCK?
                          │                                              MockAdClient (fixtures)
                          │                                              RealAdClient (ldap3):
                          │                                                LdapConnAsync::new("ldaps://<dc>")
                          │                                                simple_bind(upn, pwd).success()
                          │                                                search(baseDN, displayName/cn)
                          ▼ AuthOutcome { Ok{display_name}, BadCreds, NotFound?, Unreachable }
              ┌───────────┴───────────────┐
              ▼ known AD user?             ▼ unknown AD user
       user row by login            registration mode (app_settings ad_auto_accept)?
       │ active → session            │ auto-accept → WRITER: create user(employee,NULL hash) + info ad_register
       │ blocked/soft-deleted →      │ pending     → WRITER: create ad_register request (user waits)
       │   BlockedScreen response         │
       │   (+ restore request)            ▼
       ▼                            admin reviews in «Заявки» (admin-only filter)
   tower-sessions cookie                   │ Accept(role) → WRITER: create/activate user
   (persistent vs session per «Запомнить»)  │ Reject       → soft-delete user (auto) / discard (pending)
```

### Recommended Project Structure
```
crates/trackly-core/src/ports/
└── ad.rs                  # AdClient trait + AuthOutcome enum (mirrors snmp.rs; async-trait, NO ldap3/tokio import)

crates/trackly-infra/src/ad/
├── mod.rs                 # doc + `pub mod real; pub mod mock;` (mirrors snmp/mod.rs)
├── real.rs                # RealAdClient — ldap3 LdapConnAsync + simple_bind + search
├── mock.rs                # MockAdClient — fixtures + error scenarios (mirrors snmp/mock.rs)
└── discovery.rs           # auto-detect: env (USERDNSDOMAIN/LOGONSERVER) + hickory SRV + base-DN derive

crates/trackly-infra/src/config.rs   # add AdConfig section (enabled, use_mock, host, base_dn, name_attr, ldaps, no_tls_verify)
crates/trackly-app/src/services/auth.rs  # AuthService::login — add local→AD fallback (preserve constant-time)
crates/trackly-app/src/context.rs        # AppCtx::build — Arc<dyn AdClient> switch, inject into AuthService
docs/AD-SETUP.md           # admin setup instruction (deliverable)
```

### Pattern 1: Async simple_bind over LDAPS with result-code interpretation
**What:** Connect to a DC over LDAPS, bind with the user's UPN/credentials, classify the outcome.
**When to use:** The core of `RealAdClient::authenticate`.
**Example:**
```rust
// Source: https://docs.rs/ldap3/latest/ldap3/ (LdapConnAsync, simple_bind, .success())
use ldap3::{LdapConnAsync, LdapConnSettings};
use std::time::Duration;

// no_tls_verify is an opt-in escape hatch surfaced in «Расширенные» (D-Config-01).
let settings = LdapConnSettings::new()
    .set_conn_timeout(Duration::from_secs(5))
    .set_no_tls_verify(cfg.no_tls_verify); // default false — verify against OS trust store

let url = format!("ldaps://{host}:636");
let (conn, mut ldap) = LdapConnAsync::with_settings(settings, &url).await
    .map_err(/* → AuthOutcome::Unreachable (network/TLS failure) */)?;
ldap3::drive!(conn); // spawns the connection driver task

// AD accepts user@domain.tld or DOMAIN\user as the bind DN for simple_bind.
let bind_name = format!("{login}@{domain}"); // or pass-through if login already contains @ or \
let res = ldap.simple_bind(&bind_name, password.expose()).await
    .map_err(/* protocol/io error → Unreachable */)?;

// .success() converts non-zero rc into Err; rc==0 → Ok.
match res.success() {
    Ok(_) => { /* bound OK → proceed to search */ }
    Err(_) => { /* rc==49 invalidCredentials → AuthOutcome::BadCreds */ }
}
```
**Result-code interpretation (`LdapResult.rc`):** `[CITED: docs.rs/ldap3 + ldap.com bind reference]`
- `rc == 0` → success.
- `rc == 49` (invalidCredentials) → wrong password OR account issue. AD encodes the specific reason in the error `data` field (e.g. `data 52e` wrong password, `data 533` account disabled, `data 532` password expired, `data 701` account expired, `data 775` locked out). For v1, treat all `49` as `BadCreds` with a generic Russian message ("Неверный логин или пароль") — do NOT leak account-state to the user (enumeration). Optionally parse the `data` sub-code for an admin-facing log line only.
- Connection/TLS/timeout error before a result → `Unreachable` (distinct from `BadCreds`).

### Pattern 2: Search the bound user's entry for displayName (USR-10)
**What:** After a successful bind, the same `Ldap` handle is an authenticated session — use it to search for `displayName`/`cn`.
**When to use:** Immediately after `simple_bind().success()` in `RealAdClient::authenticate`.
**Example:**
```rust
// Source: https://docs.rs/ldap3/latest/ldap3/ (Ldap::search, SearchEntry::construct)
use ldap3::{Scope, SearchEntry};

// Filter by sAMAccountName (the short login like "us100") or userPrincipalName.
let filter = format!("(|(sAMAccountName={login})(userPrincipalName={login}))");
let attrs = vec![&cfg.name_attr[..], "cn"]; // name_attr default "displayName"

let (rs, _res) = ldap
    .search(&cfg.base_dn, Scope::Subtree, &filter, attrs)
    .await?
    .success()?;

let display_name = rs.into_iter().next().map(SearchEntry::construct).and_then(|e| {
    e.attrs.get(&cfg.name_attr).and_then(|v| v.first().cloned())   // displayName
        .or_else(|| e.attrs.get("cn").and_then(|v| v.first().cloned())) // → cn
}).unwrap_or_else(|| login.to_string()); // → login (D-Config-02 fallback chain)

ldap.unbind().await.ok(); // close session
```
**Note:** A `simple_bind` to AD yields a session with the user's own read rights, which always include reading their own `displayName`/`cn`. No service account needed for the v1 happy path. Escape the `login` in the filter to avoid LDAP filter injection (see Pitfall 5).

### Pattern 3: AdClient trait + Real/Mock + runtime switch (mirror SnmpClient exactly)
**What:** Port trait in core, two impls in infra, env/config switch in AppCtx.
**Trait surface (recommended):**
```rust
// crates/trackly-core/src/ports/ad.rs  — NO ldap3/tokio import (no_io_deps gate)
use async_trait::async_trait;
use crate::error::AppError;
use crate::primitives::secret::Secret;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthOutcome {
    Ok { display_name: String },  // bound + (best-effort) display name resolved
    BadCreds,                     // rc 49 — wrong login/password (generic, no enumeration)
    Unreachable,                  // DC down / TLS / timeout — distinct so UI can say "AD недоступен"
}

#[async_trait]
pub trait AdClient: Send + Sync {
    /// Bind as `login` with `password`; on success resolve display name.
    /// Implementations MUST reject an empty password as BadCreds WITHOUT binding
    /// (anonymous-bind trap — see RESEARCH Pitfall 1).
    async fn authenticate(&self, login: &str, password: &Secret<String>)
        -> Result<AuthOutcome, AppError>;
}
```
- `AppError` reserved for genuine infrastructure faults (config parse, etc.); `BadCreds`/`Unreachable` are normal outcomes, not errors — same philosophy as `SnmpClient` returning `Ok(None)` for unreachable.
- **`Secret<T>` in a core port:** `Secret` already lives in `trackly-core/src/primitives/secret.rs` and is I/O-free, so the trait can take it directly. `[VERIFIED: codebase]`

**Runtime switch (mirror context.rs:284):**
```rust
let use_mock = config.ad.use_mock || std::env::var("TRACKLY_AD_MOCK").is_ok();
let ad_client: Arc<dyn AdClient + Send + Sync> = if use_mock {
    Arc::new(MockAdClient::default_fixtures())
} else {
    Arc::new(RealAdClient::new(config.ad.clone()))
};
// inject into AuthService (extend AuthService::new with ad_client + ad-config reader)
```

### Pattern 4: AuthService::login local→AD fallback preserving constant-time
**What:** Add the AD branch after the existing local verify, without breaking the anti-enumeration timing.
**When to use:** `AuthService::login` (auth.rs:180).
**Recommended shape:**
```rust
pub async fn login(&self, req: LoginRequest) -> Result<UserDto, AppError> {
    // (1) EXISTING local path — keep verbatim incl. dummy-hash constant-time.
    let local = self.try_local_login(&req).await; // refactor lines 186-204 into helper
    if let Ok(user) = local { return Ok(user); }

    // (2) AD fallback — only when AD enabled.
    if !self.ad_enabled().await? { return Err(AppError::Unauthorized); }

    // CRITICAL: reject empty password BEFORE bind (Pitfall 1).
    if req.password.is_empty() { return Err(AppError::Unauthorized); }

    let pwd = Secret::new(req.password.clone());
    match self.ad_client.authenticate(&req.login, &pwd).await? {
        AuthOutcome::BadCreds   => Err(AppError::Unauthorized),
        AuthOutcome::Unreachable => Err(AppError::ServiceUnavailable /* or Internal */),
        AuthOutcome::Ok { display_name } => self.on_ad_bind_success(&req.login, &display_name).await,
    }
}
```
**Constant-time note:** The existing dummy-hash exists so a *local-only* attacker can't distinguish "user exists" from "user doesn't" by timing. The AD fallback adds a network round-trip whose latency dwarfs argon2, so timing-side-channel concerns shift; the important invariant to preserve is that **local lookup always runs the dummy-hash even when it will fall through to AD** — i.e. don't short-circuit the local branch on "user not found" in a way that skips the constant-time verify. Keep the existing local helper intact; only ADD the AD branch after it. The user-facing error for both BadCreds and unknown-local is the same generic `Unauthorized`.

### Pattern 5: AD connection auto-detect (D-Config-01)
**What:** Derive domain, DC host, base DN with zero manual config on a domain-joined Windows host.
**Order of resolution (each step falls back to the next):**
1. **Manual override** (if admin filled «Расширенные»): use `config.ad.host` / `base_dn` verbatim.
2. **DNS SRV** (canonical AD mechanism): query `_ldap._tcp.dc._msdcs.<domain>` via hickory `srv_lookup`; pick lowest-priority target → DC hostname. `<domain>` from env (step 3).
3. **Environment** (Windows, domain-joined): `USERDNSDOMAIN` (e.g. `CORP.LOCAL`) → derive base DN `dc=corp,dc=local`. `LOGONSERVER` (e.g. `\\DC01`) gives a concrete DC name if SRV fails. `USERDOMAIN` is the NetBIOS short name.
4. **Last resort:** ask admin to fill «Расширенные» (host:port, base DN).
- **Base-DN derivation:** split domain on `.`, map each label to `dc=<label>`, join with `,` → `corp.local` ⇒ `dc=corp,dc=local`. Pure string transform, unit-testable.
- **macOS dev has no domain:** `USERDNSDOMAIN` is unset and SRV fails → auto-detect returns "not a domain member". This is fine: dev always runs `TRACKLY_AD_MOCK=1`, so `RealAdClient`/discovery is never exercised on macOS. Make discovery return a typed "no domain detected" rather than panicking. `[ASSUMED — cannot verify env var presence without a domain-joined Windows host; behavior on Windows confirmed by Microsoft docs convention]`

### Anti-Patterns to Avoid
- **Storing the AD password anywhere** (DB, log, session). Bind-only, `Secret<T>`, drop. `password_hash` stays NULL. `[CITED: CLAUDE.md "Безопасность"; D-Sec-01]`
- **Using ldap3 default features** (native-tls). Always `default-features = false` + `tls-rustls-ring`.
- **Binding with an empty password.** AD/OpenLDAP may return success (anonymous/unauthenticated bind) → false-positive login. Reject before bind. (Pitfall 1)
- **String-concatenating the login into the LDAP filter** without escaping (filter injection). (Pitfall 5)
- **Distinguishing "wrong password" from "account disabled" to the end user.** Generic message only (enumeration). Sub-code → admin log only.
- **Doing user-creation outside the writer task.** Auto-register and approve must go through `WriterHandle::execute` (single-writer invariant). `[CITED: CLAUDE.md single-writer]`
- **A new `request_type` for restoration** when a sub-flag suffices (avoid migration churn). See Data Model.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| LDAP wire protocol / BER encoding | custom socket + ASN.1 | `ldap3` | Protocol is large; ldap3 handles bind/search/referrals/TLS |
| TLS to the DC | manual rustls handshake | `ldap3` tls-rustls feature (+ rustls-native-certs) | ldap3 wires rustls + OS trust store for you |
| DC discovery | parse `nslookup` output | `hickory-resolver` `srv_lookup` | Typed SRV records, async, no shell-out |
| Trusting corporate/self-signed AD CA | bundle CA in code / skip-verify by default | rustls-native-certs (OS trust store, auto via tls-rustls) | Domain-joined Windows already trusts the enterprise CA in its store |
| Password hashing for AD users | any | NONE — AD users have `password_hash=NULL` | Bind-only; no local hash exists |
| Request lifecycle / optimistic lock | new state machine | reuse `RequestService` + V006 `requests` | `ad_register` already in CHECK; Phase 6 lifecycle is proven |
| Constant-time anti-enumeration | new timing logic | reuse existing dummy-hash path in `login` | Already implemented + tested (auth.rs:64, CR-05) |

**Key insight:** Almost everything is reuse. The genuinely new code is ~1 trait, ~2 impls (real+mock), ~1 discovery module, the `login` fallback branch, an `AdConfig` section, and the registration/restoration write paths — all modeled on existing patterns.

## Runtime State Inventory

> Not a rename/refactor phase, but the phase touches auth + creates DB rows, so a light inventory:

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | New `users` rows for AD users (`ad_user=1`, `password_hash=NULL`); new `requests` rows (`request_type='ad_register'`); audit_log entries | Code (writer-task INSERTs) — no migration for users/requests |
| Live service config | AD connection settings + `ad_enabled` + `ad_auto_accept` flags | Stored in `app_settings` (DB, portable) — see Settings persistence |
| OS-registered state | None — no Task Scheduler / service registration in this phase | None — verified: phase is auth/web only |
| Secrets/env vars | `TRACKLY_AD_MOCK` (dev only, mirrors `TRACKLY_SNMP_MOCK`); AD password is transient `Secret<T>`, never persisted | None to store; document `TRACKLY_AD_MOCK` for dev |
| Build artifacts | New crate deps (ldap3, hickory) added to `trackly-infra/Cargo.toml` → Cargo.lock changes | `cargo build` regenerates lock; CI picks up |

## Common Pitfalls

### Pitfall 1: Empty-password anonymous/unauthenticated bind (CRITICAL)
**What goes wrong:** A `simple_bind(dn, "")` with a non-empty DN and zero-length password is, per RFC 4513 §5.1.2, an *unauthenticated* bind. Many servers (OpenLDAP default, some AD configs) return `rc=0` (success) — establishing an anonymous authorization state — which your code reads as "login succeeded." An attacker types any valid username + blank password and gets in.
**Why it happens:** RFC allows it for legacy trace-logging use; servers vary in whether they reject it.
**How to avoid:** Reject empty password in `AuthService::login` (and defensively in `RealAdClient::authenticate`) BEFORE calling `simple_bind`. Also reject whitespace-only.
**Warning signs:** Login succeeds in a test with a blank password field.
`[CITED: RFC 4513 §5.1.2 — datatracker.ietf.org/doc/html/rfc4513; blog.lithnet.io AD unauthenticated binds]`

### Pitfall 2: ldap3 default features pull native-tls/OpenSSL
**What goes wrong:** `cargo add ldap3` enables `default = ["sync","tls"]` → native-tls → OpenSSL (non-Windows) / SChannel (Windows). Portable Windows build then wants an OpenSSL DLL or behaves differently from the rustls-everywhere stack.
**How to avoid:** `ldap3 = { version="0.12.1", default-features=false, features=["tls-rustls-ring"] }`.
**Warning signs:** `openssl-sys` appears in `cargo tree`; build fails on a box without OpenSSL.
`[CITED: github.com/inejge/ldap3/Cargo.toml [features]; CLAUDE.md "What NOT to Use"]`

### Pitfall 3: Self-signed / corporate-CA AD certificate over LDAPS
**What goes wrong:** AD's LDAPS cert is often issued by an internal enterprise CA (or self-signed). A fresh rustls client with no custom roots rejects it → every bind fails with a TLS error.
**Why it (usually) doesn't happen on real deployments:** `tls-rustls` enables `rustls-native-certs`, which loads the **OS trust store**. A domain-joined Windows machine already has the enterprise root CA in its store (pushed by GPO), so verification succeeds automatically — no manual cert config. This is the common LAN case and aligns with D-Config-01 "one button."
**How to avoid (edge cases):** Surface `set_no_tls_verify(true)` as an explicit, off-by-default «Расширенные» opt-in (documented as insecure, for non-domain-joined hosts or broken cert chains). Document in `docs/AD-SETUP.md`: "if AD login fails with a certificate error, either install the AD root CA in the host trust store, or enable the (insecure) skip-verify option."
**Recommendation:** Default LDAPS + verify-via-OS-store; skip-verify as documented opt-in. Do NOT default to plain LDAP (sends password in cleartext).
`[CITED: github.com/inejge/ldap3 features (rustls-native-certs); docs.rs LdapConnSettings.set_no_tls_verify]`

### Pitfall 4: requests.requested_by_user_id is NOT NULL (FK to users)
**What goes wrong:** `requests.requested_by_user_id INTEGER NOT NULL REFERENCES users(id)` (V006). For **pending-approval** mode the AD user does NOT exist yet, so there's no user id to attribute the `ad_register` request to.
**How to avoid:** Two viable options — (a) in pending mode, still create the `users` row first (inactive: add an `is_active=0` AD user) then the request references it, and approve flips `is_active=1`; or (b) attribute the request to a system/bootstrap user id. Option (a) is cleaner and matches the existing `is_active` column (V019) + the "revive soft-deleted user" pattern already in `create_user` (auth.rs:243). **Recommendation: create an inactive AD user row in pending mode**, store the requested-display-name on it, and have approve set role + `is_active=1`. This also makes restoration uniform.
**Warning signs:** FK constraint violation inserting `ad_register` for an unknown user.
`[VERIFIED: codebase — V006 schema, V019 is_active, auth.rs create_user revive path]`

### Pitfall 5: LDAP filter injection
**What goes wrong:** `format!("(sAMAccountName={login})")` with `login = "*)(uid=*"` alters the filter.
**How to avoid:** Escape filter special chars (`\`, `*`, `(`, `)`, NUL) per RFC 4515 before interpolation. ldap3 exposes `ldap3::ldap_escape` for this. Use it on `login` before building the filter.
**Warning signs:** Unusual login strings returning unexpected entries.
`[CITED: docs.rs/ldap3 — ldap_escape helper; RFC 4515]`

### Pitfall 6: Bind-name format for AD simple_bind
**What goes wrong:** AD `simple_bind` accepts `user@domain.tld` (UPN) or `DOMAIN\user`, but NOT a bare short login in many configs.
**How to avoid:** Normalize input: if `login` already contains `@` or `\`, pass through; else build `login@<domain>` from the detected domain. Document accepted formats in the login UI hint.
`[CITED: ldap.com bind reference; D-Config-01 examples us100/user@domain/DOMAIN\\user]`

### Pitfall 7: `LdapConnAsync` driver task must be driven
**What goes wrong:** Forgetting `ldap3::drive!(conn)` (or awaiting the conn future) leaves the connection un-pumped; operations hang.
**How to avoid:** Always `ldap3::drive!(conn)` immediately after `LdapConnAsync::new/with_settings`. Pair with `set_conn_timeout` so a dead DC fails fast → `Unreachable`.
`[CITED: docs.rs/ldap3 quick-start]`

### Pitfall 8: Windows MSVC build with tls-rustls-ring
**What goes wrong:** Generally fine (ring builds on MSVC), but the rustls crypto provider must be installed before first use in some rustls 0.23 setups, or you get "no process-level CryptoProvider" panics.
**How to avoid:** `tls-rustls-ring` sets the `rustls-provider` feature, which makes ldap3 install the ring provider for its connections. If a separate process-level provider is needed elsewhere, call `rustls::crypto::ring::default_provider().install_default()` once at startup. Verify on the real Windows runner (STATE.md already flags "validation against real Windows Server 2022 with channel binding" as a Phase-8/9 spike, ~½ day).
`[ASSUMED — rustls provider install behavior; verify on Windows CI runner per STATE.md blocker note]`

## Code Examples

### RealAdClient::authenticate skeleton
```rust
// Source: composed from docs.rs/ldap3 patterns (LdapConnAsync, simple_bind, search, drive!)
use ldap3::{LdapConnAsync, LdapConnSettings, Scope, SearchEntry, ldap_escape};
use std::time::Duration;
use trackly_core::ports::ad::{AdClient, AuthOutcome};
use trackly_core::primitives::secret::Secret;
use trackly_core::error::AppError;
use async_trait::async_trait;

pub struct RealAdClient { cfg: AdConfig }

#[async_trait]
impl AdClient for RealAdClient {
    async fn authenticate(&self, login: &str, password: &Secret<String>)
        -> Result<AuthOutcome, AppError>
    {
        if password.expose().trim().is_empty() {
            return Ok(AuthOutcome::BadCreds); // Pitfall 1 — never bind with empty pwd
        }
        let settings = LdapConnSettings::new()
            .set_conn_timeout(Duration::from_secs(5))
            .set_no_tls_verify(self.cfg.no_tls_verify);
        let url = format!("ldaps://{}:{}", self.cfg.host, self.cfg.port);

        let (conn, mut ldap) = match LdapConnAsync::with_settings(settings, &url).await {
            Ok(v) => v,
            Err(_) => return Ok(AuthOutcome::Unreachable), // DC down / TLS handshake fail
        };
        ldap3::drive!(conn);

        let bind_name = if login.contains('@') || login.contains('\\') {
            login.to_string()
        } else {
            format!("{login}@{}", self.cfg.domain)
        };

        match ldap.simple_bind(&bind_name, password.expose()).await {
            Ok(res) => match res.success() {
                Ok(_) => {} // bound
                Err(_) => { let _ = ldap.unbind().await; return Ok(AuthOutcome::BadCreds); }
            },
            Err(_) => return Ok(AuthOutcome::Unreachable),
        }

        let safe = ldap_escape(login);
        let filter = format!("(|(sAMAccountName={safe})(userPrincipalName={safe}))");
        let display_name = match ldap
            .search(&self.cfg.base_dn, Scope::Subtree, &filter, vec![&self.cfg.name_attr[..], "cn"])
            .await.and_then(|r| r.success())
        {
            Ok((rs, _)) => rs.into_iter().next().map(SearchEntry::construct).and_then(|e| {
                e.attrs.get(&self.cfg.name_attr).and_then(|v| v.first().cloned())
                    .or_else(|| e.attrs.get("cn").and_then(|v| v.first().cloned()))
            }).unwrap_or_else(|| login.to_string()),
            Err(_) => login.to_string(), // search failed → fall back to login (D-Config-02)
        };
        let _ = ldap.unbind().await;
        Ok(AuthOutcome::Ok { display_name })
    }
}
```

### MockAdClient (mirror MockSnmpClient::default_fixtures)
```rust
// Source: codebase pattern — crates/trackly-infra/src/snmp/mock.rs
use std::collections::HashMap;

pub struct AdFixture { pub password: &'static str, pub display_name: &'static str }

pub struct MockAdClient { pub users: HashMap<String, AdFixture>, pub unreachable: bool }

impl MockAdClient {
    pub fn default_fixtures() -> Self {
        let mut users = HashMap::new();
        users.insert("us100".into(), AdFixture { password: "Passw0rd!", display_name: "Иванов Иван Иванович" });
        users.insert("us200".into(), AdFixture { password: "Secret123", display_name: "Петрова Анна Сергеевна" });
        Self { users, unreachable: false }
    }
}

#[async_trait]
impl AdClient for MockAdClient {
    async fn authenticate(&self, login: &str, password: &Secret<String>)
        -> Result<AuthOutcome, AppError>
    {
        if password.expose().is_empty() { return Ok(AuthOutcome::BadCreds); } // Pitfall 1
        if self.unreachable { return Ok(AuthOutcome::Unreachable); }
        // strip @domain / DOMAIN\ for lookup
        let key = login.split('@').next().unwrap_or(login)
                       .rsplit('\\').next().unwrap_or(login);
        match self.users.get(key) {
            Some(f) if f.password == password.expose() =>
                Ok(AuthOutcome::Ok { display_name: f.display_name.to_string() }),
            Some(_) => Ok(AuthOutcome::BadCreds),  // wrong password scenario
            None    => Ok(AuthOutcome::BadCreds),  // not found → generic (no enumeration)
        }
    }
}
```
> Note: "not found" returns `BadCreds` (generic) so the mock matches the no-enumeration contract. The "user unknown to Trackly but valid in AD" registration trigger is decided in `AuthService` AFTER a successful `Ok{}` outcome, by checking the local `users` table — NOT by the mock returning a special "unknown" variant. Add a `MockAdClient::unreachable()` constructor for the server-down scenario test.

### AdConfig section (config.rs, mirror ServerConfig)
```rust
#[derive(Debug, Deserialize, Clone)]
pub struct AdConfig {
    pub enabled: bool,        // default false (DB app_settings is the live source; toml = bootstrap default)
    pub use_mock: bool,       // default false; OR TRACKLY_AD_MOCK env
    pub host: String,         // empty → auto-detect
    pub port: u16,            // default 636 (LDAPS)
    pub domain: String,       // e.g. "corp.local"; empty → auto-detect (USERDNSDOMAIN)
    pub base_dn: String,      // empty → derive from domain (dc=corp,dc=local)
    pub name_attr: String,    // default "displayName" (D-Config-02)
    pub no_tls_verify: bool,  // default false — «Расширенные» opt-in only (Pitfall 3)
}
impl Default for AdConfig { /* enabled:false, port:636, name_attr:"displayName", rest empty/false */ }
```
> **Source-of-truth decision (recommend):** runtime AD settings the admin edits live in `app_settings` (DB, portable, editable without restart — matches `desktop_lock_enabled`/`low_stock_threshold` pattern). `AdConfig` in `trackly.config.toml` provides bootstrap defaults / `use_mock` only. See Settings persistence.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `trust-dns-resolver` | `hickory-resolver` (same project, renamed) | 2024 | Use `hickory-resolver 0.26`; `trust-dns-*` is the legacy name (last 0.23.2) |
| ldap3 native-tls default | rustls feature via `tls-rustls-ring`/`-aws-lc-rs` | ldap3 added rustls features in 0.11/0.12 | Pure-Rust TLS now first-class; matches project no-OpenSSL discipline |
| Hand-rolled SSO ambition (memory `phase8_split_ad_sso`) | v1 = simple_bind, SSO → v2 (D-AD-01) | 2026-06-19 (this phase's discuss) | Memory note `phase8_split_ad_sso` is now superseded — planner should not pursue Kerberos/NTLM in v1 |

**Deprecated/outdated:**
- `trust-dns-resolver` crate name — replaced by `hickory-resolver`.
- ldap3 `gssapi`/`ntlm` features — NOT for v1 (would raise MSRV to 1.85 and need Kerberos/SSPI build deps; macOS dev can't build gssapi). Deferred per D-AD-01.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | On domain-joined Windows, `USERDNSDOMAIN`/`LOGONSERVER` are set and the enterprise root CA is in the OS trust store (so rustls-native-certs trusts LDAPS automatically) | Pattern 5, Pitfall 3 | If CA not in store, LDAPS fails until admin installs CA or enables skip-verify — mitigated by documented «Расширенные» opt-in + AD-SETUP.md |
| A2 | A user's `simple_bind` session can read their own `displayName`/`cn` without a service account | Pattern 2 | If AD ACLs block self-read (rare), display name falls back to login (D-Config-02) — degraded, not broken |
| A3 | rustls ring provider is installed by ldap3's `rustls-provider` feature; no separate process-level install needed for ldap3 connections | Pitfall 8 | Possible "no CryptoProvider" panic on Windows — mitigated by one-line `install_default()` at startup; verify on Windows CI |
| A4 | AD returns rc=49 (with data sub-codes) for all credential/account failures | Pattern 1 | If a config returns a different rc, treat any non-zero as BadCreds (already the design) — safe default |
| A5 | hickory-resolver `srv_lookup` resolves `_ldap._tcp.dc._msdcs.<domain>` on a joined host | Pattern 5 | If SRV blocked, env-domain + manual override fallback covers it |
| A6 | ldap3 0.12.1 exposes `ldap_escape`, `LdapConnAsync::with_settings`, `LdapConnSettings::set_no_tls_verify/set_conn_timeout` | Pattern 1/2/5, Code Examples | Verified via docs.rs/ldap3 + GitHub Cargo.toml; if an exact name differs, a quick docs check during implementation resolves it |

## Open Questions (RESOLVED)

> All three resolved during planning and implemented by the plans: Q1 → V028 `ad_subtype` (09-02), Q2 → inactive-user attribution (09-03), Q3 → `find_user_any_state` (09-02).

1. **Restoration request: sub-flag vs new request_type (D-REG-03)**
   - What we know: `requests.request_type` CHECK = `('cartridge_replace','free_form','ad_register')`; changing a CHECK requires a table rebuild migration in SQLite.
   - What's unclear: whether restoration deserves its own type for reporting.
   - **RESOLVED: REUSE `ad_register` with a discriminator, no new type.** Cleanest options, in order: (a) reuse the **status/`resolution_notes`** or a `description` marker; or (b) add a single nullable column `ad_subtype TEXT NULL` (values `register`/`restore`) via simple `ALTER TABLE ADD COLUMN` (no CHECK rebuild). Distinguish "register" (no/inactive user) from "restore" (soft-deleted/blocked user exists) by inspecting the `users` row state at creation time. This avoids a CHECK-rebuild migration and keeps the admin «Заявки» filter as `request_type='ad_register'`. The approve handler branches on whether the target user is soft-deleted (revive) vs new (create).

2. **Pending-mode user attribution (Pitfall 4)**
   - What we know: `requested_by_user_id` is NOT NULL.
   - **RESOLVED: create an inactive (`is_active=0`) AD user row in pending mode**, request references it, approve flips active + sets role. Reuses V019 `is_active` + the existing revive path. Alternative (attribute to system user) is messier.

3. **Blocked vs unknown distinction in login path**
   - What we know: soft-delete = `deleted_at_utc IS NOT NULL`; blocked = `is_active=0`. Current `get_password_hash`/`get_by_login` filter BOTH out (`deleted_at_utc IS NULL AND is_active=1`), so a blocked/deleted AD user currently looks "unknown."
   - What's unclear: the login path must, after a successful AD bind, look up the user **without** the active/non-deleted filter to detect blocked/soft-deleted and show the BlockedScreen (D-REG-03) instead of re-registering.
   - **RESOLVED:** add an internal lookup `find_user_any_state(login)` returning `{id, role, is_active, deleted}` so post-bind logic can branch: active→session; inactive/deleted→BlockedScreen + restore-request; none→registration mode. This is the key new query; plan it explicitly.

## Environment Availability

| Dependency | Required By | Available (dev macOS) | Version | Fallback |
|------------|------------|-----------------------|---------|----------|
| Active Directory / DC | RealAdClient bind (USR-08/10) | ✗ (no domain reachable from dev) | — | `TRACKLY_AD_MOCK=1` + MockAdClient (D-Mock-01) — full flow testable |
| Domain-joined Windows host | auto-detect env vars + SRV | ✗ on dev; ✓ on target Win10 x64 (Phase 8 release enables this) | — | Mock path on dev; real validation on Windows test machine (STATE.md spike, ½ day) |
| `ldap3` crate | all AD I/O | ✓ (builds pure-Rust on macOS with tls-rustls-ring) | 0.12.1 | — |
| `hickory-resolver` | DC SRV discovery | ✓ (builds on macOS) | 0.26.1 | env-domain + manual override |
| rustls ring provider | LDAPS TLS | ✓ | 0.23 | — |

**Missing dependencies with no fallback:** none (mock covers the only unreachable dependency).
**Missing dependencies with fallback:** real AD/domain — fully substituted by MockAdClient on dev; real validation deferred to the Windows test machine (intended by the Phase 8-before-9 split).

## Validation Architecture

> nyquist_validation: config not inspected as explicitly false → treated as ENABLED. The whole point of D-Mock-01 is that every requirement is validatable on dev macOS with no real domain.

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[tokio::test]` / `#[test]` (+ optional `cargo nextest`); frontend `svelte-check` |
| Config file | none (cargo workspace); mock toggled via `TRACKLY_AD_MOCK` env / `config.ad.use_mock` |
| Quick run command | `cargo test -p trackly-infra ad::` and `cargo test -p trackly-app auth` |
| Full suite command | `cargo test` (one at a time — MEMORY: no concurrent cargo test) |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| USR-12 | Mock: success / wrong-pwd / not-found / unreachable | unit | `cargo test -p trackly-infra ad::mock` | ❌ Wave 0 (`crates/trackly-infra/src/ad/mock.rs` #[cfg(test)]) |
| USR-08 | login() falls back to AD, returns session on Ok | integration | `cargo test -p trackly-app auth::ad_fallback` | ❌ Wave 0 |
| USR-08/Pitfall1 | empty password rejected BEFORE bind (no anonymous-bind success) | unit | `cargo test -p trackly-app auth::empty_password_rejected` | ❌ Wave 0 (CRITICAL — must exist) |
| USR-10 | display_name resolved from fixture; fallback chain displayName→cn→login | unit | `cargo test -p trackly-infra ad::display_name` | ❌ Wave 0 |
| USR-09/REQ-06 | unknown AD user → ad_register request created, admin-only visible | integration | `cargo test -p trackly-app requests::ad_register_admin_only` | ❌ Wave 0 |
| USR-11/SET-10 | auto-accept ON → user created employee + info request; OFF → pending | integration | `cargo test -p trackly-app auth::auto_accept_modes` | ❌ Wave 0 |
| D-REG-03 | blocked/soft-deleted AD user after bind → BlockedScreen outcome, restore request | integration | `cargo test -p trackly-app auth::blocked_user_restore` | ❌ Wave 0 |
| D-Config-01 | base-DN derivation `corp.local`→`dc=corp,dc=local`; discovery returns "no domain" cleanly | unit | `cargo test -p trackly-infra ad::discovery` | ❌ Wave 0 |
| Pitfall 5 | login with filter metachars is escaped (ldap_escape) | unit | `cargo test -p trackly-infra ad::filter_escape` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p trackly-infra ad::` + `cargo clippy -- -D warnings`
- **Per wave merge:** `cargo test -p trackly-app` (auth + requests)
- **Phase gate:** full `cargo test` green + `svelte-check` + manual UAT on Windows test machine for the real-DC path (the only non-mockable surface).

### Wave 0 Gaps
- [ ] `crates/trackly-core/src/ports/ad.rs` — `AdClient` trait + `AuthOutcome` (mirror `ports/snmp.rs`)
- [ ] `crates/trackly-infra/src/ad/{mod,real,mock,discovery}.rs` — impls + `#[cfg(test)]` fixtures
- [ ] `no_io_deps` gate: confirm `ad.rs` imports only `async-trait` + core types (ldap3/hickory/tokio must NOT leak into core)
- [ ] `AuthService` test seam: constructor must accept `Arc<dyn AdClient>` so tests inject `MockAdClient`
- [ ] AD-SETUP.md (deliverable) — not a test, but a phase exit criterion

## Security Domain

> security_enforcement treated as ENABLED (not set false). This phase is squarely an auth phase.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | yes | ldap3 simple_bind over LDAPS; reject empty password; generic failure message (no enumeration); reuse argon2id for local; AD password never stored (`Secret<T>`, NULL hash) |
| V3 Session Management | yes | Reuse Phase 5 `tower-sessions` + session-fixation flush-before-insert (http/auth.rs `build_auth_login`); «Запомнить меня» = persistent vs session cookie (D-UX-02) |
| V4 Access Control | yes | `authorize()` enforced server-side (USR-06); `ad_register` admin-only filter (REQ-06); approve sets role (default employee); last-admin guard already in AuthService |
| V5 Input Validation | yes | `ldap_escape` on login before LDAP filter (Pitfall 5); normalize bind-name format; validate role on approve via `Role::from_str` |
| V6 Cryptography | yes | rustls (tls-rustls-ring) for LDAPS — never hand-roll; LDAPS default, plain LDAP discouraged; argon2id unchanged for local users |
| V7 Data Protection / Secrets | yes | `Secret<T>` for AD password (zeroize-on-drop, no Debug/Serialize leak); password never written to DB or logs |

### Known Threat Patterns for ldap3/AD auth

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Empty-password anonymous bind → auth bypass | Spoofing / Elevation | Reject empty/whitespace password before `simple_bind` (Pitfall 1) |
| LDAP filter injection | Tampering | `ldap3::ldap_escape` per RFC 4515 (Pitfall 5) |
| Cleartext password on the wire (plain LDAP) | Information Disclosure | Default LDAPS (636); rustls verification via OS trust store; plain LDAP only via explicit opt-in |
| MITM with skip-verify left on | Spoofing / Info Disclosure | `no_tls_verify` off by default; «Расширенные»-only, documented as insecure |
| Username enumeration via differing errors/timing | Information Disclosure | Generic «Неверный логин или пароль»; account-state sub-code → admin log only; keep local constant-time dummy-hash path |
| AD password persisted/logged | Information Disclosure | `Secret<T>` (no Debug/Serialize, zeroize); `password_hash=NULL`; bind-only |
| Privilege escalation via self-registration | Elevation | Auto-register hard-codes role=employee; role change requires admin approve (`ManageUsers`/`TransitionRequests`) |

## Sources

### Primary (HIGH confidence)
- `crates/trackly-app/src/services/auth.rs` — `login()`, dummy-hash constant-time, `app_settings` upsert (`desktop_lock_enabled`), `create_user` revive path `[VERIFIED: codebase]`
- `crates/trackly-infra/src/snmp/{mod,mock}.rs`, `crates/trackly-core/src/ports/snmp.rs` — mock pattern to mirror `[VERIFIED: codebase]`
- `crates/trackly-core/src/{auth,primitives/secret}.rs` — Identity/Role/authorize, Secret<T> `[VERIFIED: codebase]`
- `migrations/V002,V006,V016,V018,V019,V024` — users(ad_user,password_hash NULL,is_active), requests(ad_register CHECK), app_settings `[VERIFIED: codebase]`
- `crates/trackly-app/src/context.rs:284` — `TRACKLY_SNMP_MOCK` runtime switch `[VERIFIED: codebase]`
- docs.rs/ldap3 (latest) — LdapConnAsync, simple_bind, .success(), search, LdapConnSettings `[CITED]`
- github.com/inejge/ldap3 Cargo.toml — `[features]` (default=sync+tls; tls-rustls-ring/-aws-lc-rs) `[CITED]`
- RFC 4513 §5.1.2 (datatracker.ietf.org) — unauthenticated/empty-password bind `[CITED]`
- `cargo search` — ldap3 0.12.1, hickory-resolver 0.26.1, rustls 0.23 stable `[VERIFIED]`

### Secondary (MEDIUM confidence)
- ldap.com LDAPv3 bind wire reference — bind-name formats, rc=49 `[CITED]`
- blog.lithnet.io — AD/LDS unauthenticated binds in practice `[CITED]`
- CLAUDE.md — ldap3 0.12 pin, rustls-not-native-tls, single-writer, Secret<T> `[CITED]`

### Tertiary (LOW confidence — validate on Windows)
- Windows env-var presence (USERDNSDOMAIN/LOGONSERVER) + OS-trust-store CA behavior — convention, unverified without a domain-joined host `[ASSUMED]`
- rustls ring process-provider install nuance on MSVC `[ASSUMED]`

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — ldap3/hickory/rustls versions + feature flags verified via cargo search + GitHub Cargo.toml; matches CLAUDE.md pins.
- Architecture/integration: HIGH — every seam (mock pattern, login hook, app_settings, requests reuse, single-writer) verified directly in the codebase.
- AD runtime behavior (auto-detect, cert trust, rc codes): MEDIUM — verified against docs/RFC/Microsoft convention but NOT against a live DC; intentionally deferred to the Windows test machine per the Phase 8→9 split.
- Pitfalls: HIGH for empty-password (RFC), feature flags (Cargo.toml), filter injection (RFC 4515); MEDIUM for rustls-provider-on-Windows.

**Research date:** 2026-06-19
**Valid until:** 2026-07-19 (stable stack; ldap3/hickory move slowly). Re-check ldap3 version if planning slips past a minor release.

# Phase 32: Авто-админ по списку логинов + релиз SSO в main - Research

**Researched:** 2026-08-03
**Domain:** Auth provisioning-seam extension (Rust, existing `AuthService`), TOML config plumbing, git merge/release mechanics (GitHub Actions)
**Confidence:** HIGH — this phase extends a fully-built, fully-tested seam (Phase 31's `on_ad_bind_success`) with a config-driven override; no new external dependency, no new protocol surface, no new I/O boundary. The merge/release mechanics were independently verified in this session (not assumed).

## Summary

Phase 32 is small and additive by construction: it does not touch AD protocol code (`ldap3`,
`sspi`, SPNEGO) at all — it only adds a **local, config-driven override** in front of the
already-built provisioning seam `AuthService::on_ad_bind_success` (Phase 31). The mechanism is
a `Vec<String>` (`admin_logins`) on `AdConfig`, loaded from `trackly.config.toml` exactly like
the existing `role_mapping` field, normalized and compared case-insensitively against the
AD login at the top of the provisioning decision. When a match is found, the existing
branch logic (active / pending / blocked+deleted / unknown) is short-circuited into a single
"force active admin" outcome instead of its normal branch — this requires enumerating **five
distinct pre-existing user states** and writing the correct SQL transition for each (detailed
below), not just one `INSERT`.

The second half of the phase — merging `spike/ad-sso-kerberos` into `main` and cutting `v1.3.0`
— was investigated directly in this session rather than assumed. Two concrete, verified findings
change the shape of that work: (1) **`cargo fmt --all -- --check` currently FAILS on this branch**
in ~15 files, none of which are Phase 31/32 territory (`act.rs`, `act_service.rs`, `html_templates.rs`,
several `acts_*` test files, plus `ad/sso.rs`/`ad/keytab.rs`/`audit_log_sqlite.rs`) — this is a
**hard merge blocker** because `ci-fast`/`ci-full` both gate on `cargo fmt --all -- --check` and
run on every push, including push to `main`; this must be fixed (a single `cargo fmt --all` +
commit) before or as part of the merge, not discovered after a failed `main` CI run. (2) The
premise in CONTEXT.md D-12 that SSO/Kerberos sits behind a Windows-gated Cargo feature
(`gssapi`) does **not match the actual code**: the live implementation uses the pure-Rust `sspi`
crate with **no Cargo feature flag at all**, unconditionally compiled on every platform, and
`ci-full.yml`'s matrix already runs the full mock-backed test suite green on
`ubuntu-latest`/`macos-latest`/`windows-latest` (this is already how Phase 31 passed CI). D-12's
verification task therefore reduces to "confirm fmt/clippy/test are green on all three OSes
before merging" (fmt currently is NOT), not "add Windows feature-gating" (there is none to add;
none is needed).

**Primary recommendation:** Implement the override as one new `AdConfig.admin_logins: Vec<String>`
field (mirrors `role_mapping` exactly, `[serde(default)]`, empty = feature off per D-03) threaded
into `AuthService` via a **builder method** (`AuthService::new(...).with_admin_logins(...)`,
mirroring `ActService`'s established `with_pdf_pipeline`/`with_org_db` pattern) rather than a new
positional constructor argument — this avoids touching the ~9 existing `AuthService::new(...)`
call sites (production `context.rs` + 8 test files). Inject the membership check at the very top
of `on_ad_bind_success`, so both `sso_login` (passwordless SSO) and `try_ad_login` (LDAPS
password bind) get identical, DRY treatment — recommended over injecting only in `sso_login`,
see Open Questions #1. Fix the pre-existing `cargo fmt` drift as an explicit task before the
`main` merge step.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| `admin_logins` config storage/parsing | API/Backend (`trackly-infra::config::AdConfig`) | — | Deployment-time TOML, same tier as `role_mapping` (D-01) |
| Membership check (login ∈ admin_logins) | API/Backend (`trackly-app::services::auth::AuthService`) | — | Pure local set lookup, no I/O, no directory dependency (D-10) |
| Forced role/state transition (INSERT or UPDATE `users`, close pending request) | API/Backend (`AuthService`, single writer tx) | Database/Storage (`users`/`requests`/`audit_log` tables) | Security-relevant write path; must go through the single-writer seam like every other mutation in this codebase |
| Audit trail of the override | API/Backend (`audit_log` INSERT in the same tx) | Database/Storage | D-07 explicitly flags this as security-significant — needs a durable trail, not just a log line |
| Merge/CI-gate verification (fmt/clippy/test on 3 OSes) | CI/Backend (GitHub Actions) | — | Not a runtime capability, but a required phase deliverable (D-11/D-12) |
| Release tag → build → draft release | CI/Backend (`release.yml`) | — | Mechanical; already fully built, verified to trigger correctly on `v*.*.*` |
| UI display of admin_logins | *(none — explicitly out of scope, D-01/D-03/deferred)* | — | No UI capability in this phase at all |

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| SSO-02 | Admin can configure a trusted login list; listed logins get Admin role immediately on SSO login, bypassing manual approval | `AdConfig.admin_logins: Vec<String>` (mirrors `role_mapping` exactly) + a new decision branch at the top of `on_ad_bind_success` that overrides all 5 pre-existing user states (unknown/pending/blocked-or-deleted/active-non-admin/active-admin) — full state matrix below in Architecture Patterns |

## Standard Stack

### Core

No new external crates. This phase is 100% additive glue code inside an already-dependency-complete
crate graph (`serde`/`toml` for config — already present; `rusqlite` writer tx — already present).

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `serde` | workspace-pinned (already a dep) | `#[derive(Deserialize)]` on the new `admin_logins: Vec<String>` field | Identical mechanism to `role_mapping: Vec<RoleMappingEntry>` (`config.rs:233`), just `Vec<String>` instead of `Vec<struct>` — even simpler, TOML natively supports `admin_logins = ["us100", "us200"]` array-of-strings syntax (no `[[ad.admin_logins]]` table-array needed) |
| `rusqlite` | workspace-pinned (already a dep) | Writer transaction for the forced role/state UPDATE/INSERT + audit_log | Same `self.writer.execute(move |conn| { let tx = conn.transaction()...})` pattern used by every other mutation in `auth.rs`/`request_service.rs` |

**No `cargo add` needed.** No `Cargo.toml` change at all for the SSO-02 code path.

### Package Legitimacy Audit

**No new external packages are introduced by this phase.** Both `serde` and `rusqlite` are
existing, already-audited workspace dependencies. No `slopcheck`/registry-verification gate
applies — nothing new is being installed for either the SSO-02 feature or the merge/release
operational work.

**Packages removed due to slopcheck verdict:** none (no new packages)
**Packages flagged as suspicious:** none (no new packages)

## Architecture Patterns

### System Architecture Diagram

```
trackly.config.toml (gitignored, deployment-time)
   [ad]
   admin_logins = ["us100", "us777"]          ◄── NEW (D-01, mirrors role_mapping)
   │
   ▼
AppConfig::load_or_default()  (config.rs — unchanged mechanism, new field only)
   │
   ▼
AppCtx::build()  (context.rs)
   │  AuthService::new(writer, readers, clock, ad_client, ws_tx, directory)
   │    .with_admin_logins(config.ad.admin_logins.clone())   ◄── NEW builder call
   │    (mirrors ActService::new(...).with_pdf_pipeline(...).with_org_db(...))
   ▼
AuthService { ..., admin_logins: Arc<HashSet<String>> }   ◄── normalized once at construction
   │
   │  ── LOGIN TIME (either entry point) ──
   │
   ├─ sso_login(ad_username, display_name)          [passwordless SSO, Phase 31 entry]
   │     directory.resolve(ad_username) → (resolved_display_name, role_hint)
   │     (role_hint used ONLY if login is NOT in admin_logins — see below)
   │
   └─ try_ad_login(req)                              [LDAPS password bind, Phase 9 entry]
         ad_client.authenticate(login, password) → display_name
         (role_hint always None on this path today — unchanged)
   │
   ▼
on_ad_bind_success(login, display_name, role_hint)   ◄── SINGLE INJECTION POINT (recommended)
   │
   │  NEW: is_admin_login(login)?  (case-insensitive, local set check, D-09/D-10 —
   │       does NOT call directory.resolve, does NOT depend on AD reachability)
   │
   ├─ NO  → EXISTING Phase 31 branching, UNCHANGED (D-08):
   │         active → session; pending → RegistrationPending; blocked/deleted →
   │         AccessBlocked; unknown → auto_register_ad_user/create_pending_registration
   │
   └─ YES → NEW forced-admin state machine (D-04..D-07), one writer tx per case:
             ┌─────────────────────────────┬────────────────────────────────────────┐
             │ find_user_any_state(login)  │ Action                                 │
             ├─────────────────────────────┼────────────────────────────────────────┤
             │ None (unknown)              │ INSERT active admin user (NO pending   │
             │                             │ request row at all — bypass, not       │
             │                             │ auto-accept-with-info-request)          │
             │ Some(pending, is_active=0,  │ UPDATE role='admin', is_active=1  +    │
             │  has_open_register_request) │ auto-complete the open 'ad_register'/  │
             │                             │ 'register' request (else it dangles    │
             │                             │ in the Requests inbox for an already-   │
             │                             │ active admin — see Pitfall 2)           │
             │ Some(blocked/deleted)       │ UPDATE role='admin', is_active=1,      │
             │                             │ deleted_at_utc=NULL (revive) — D-07     │
             │                             │ explicit override of manual block       │
             │ Some(active, role≠admin)    │ UPDATE role='admin' only (D-06         │
             │                             │ escalation of existing user)            │
             │ Some(active, role=admin)    │ NO-OP — skip the write entirely         │
             │                             │ (idempotency: don't bump `version`/     │
             │                             │ `updated_at_utc` on every login)         │
             └─────────────────────────────┴────────────────────────────────────────┘
             + audit_log INSERT in the SAME tx (action e.g. 'ad_auto_admin',
               payload_json capturing prior {role, is_active, deleted} for traceability)
   ▼
UserDto { role: "admin", is_active: true, ... }  → session issued exactly as today
```

### Pattern 1: Config field mirrors `role_mapping` exactly (simpler — flat `Vec<String>`)

**What:** Add `admin_logins: Vec<String>` to `AdConfig`, `#[serde(default)]`, safe to `Debug`
(no secrets — just login names, same as `role_mapping`'s `group_dn`/`role` strings).
**Where:** `crates/trackly-infra/src/config.rs`, next to `role_mapping` (line ~233).
**Example (mirrors the existing `role_mapping` test pattern exactly):**

```rust
// AdConfig struct addition:
/// Список доменных логинов (sAMAccountName), которые получают роль admin
/// сразу при AD-bind/SSO-входе, в обход ad_auto_accept и pending-заявки
/// (Phase 32, SSO-02). Матчинг case-insensitive, чисто локальный (без
/// обращения к каталогу) — см. AuthService::is_admin_login.
/// Пустой список (или отсутствие поля) = фича выключена (D-03).
#[serde(default)]
pub admin_logins: Vec<String>,

// Default impl addition:
admin_logins: Vec::new(),

// Debug impl: safe to include as-is (no secrets) — add to the manual
// impl alongside role_mapping, OR note explicitly that admin_logins is
// intentionally the one AdConfig field safe to print unredacted.
```

TOML shape (simpler than `role_mapping` — flat array, no `[[ad.admin_logins]]` table-array
needed):
```toml
[ad]
admin_logins = ["us100", "us777"]
```

### Pattern 2: Threading via builder method, not constructor arg

**What:** `AuthService::new(...)` currently has 9 call sites across the workspace (`context.rs`
production + 8 test files: `ad_auth.rs`, `ad_directory_sso.rs`, `ad_register.rs`, `auth_smoke.rs`,
`specta_roundtrip.rs`, `users_crud.rs`, `health.rs` [http], `health.rs` [tauri_cmds], plus one
inline `#[cfg(test)]` site in `auth.rs` itself). Adding a new REQUIRED positional argument means
touching all 9. This codebase has an established precedent for exactly this problem:
`ActService::new(writer, readers, clock)` is a 3-arg constructor with **two builder methods**
(`with_pdf_pipeline`, `with_org_db`) added in later phases specifically so pre-existing call
sites (including tests) keep compiling unchanged (`act_service.rs:83-90`'s own doc comment:
*"так, чтобы не ломать существующие call sites"*).

**Recommendation:** Add `admin_logins: Arc<std::collections::HashSet<String>>` as a new
`AuthService` field, defaulted to an empty set inside `new()` (feature off, matches D-03), with
a `pub fn with_admin_logins(mut self, logins: Vec<String>) -> Self` builder that normalizes each
entry (see Pattern 3) into the set. Only `context.rs` (production) calls
`.with_admin_logins(config.ad.admin_logins.clone())`; all 8 test call sites are UNAFFECTED
(they get the empty-set default, i.e. admin_logins feature is off in every existing test unless
a new test explicitly opts in via the builder — exactly the isolation you want for new,
security-sensitive test scenarios).

```rust
// AuthService struct addition:
pub(crate) admin_logins: Arc<std::collections::HashSet<String>>,

// new() addition:
admin_logins: Arc::new(std::collections::HashSet::new()),

// New builder method:
/// Builder: настроить список доверенных доменных логинов, получающих
/// принудительную роль admin при AD-bind (Phase 32, SSO-02). Пустой список
/// (дефолт из `new()`) = фича выключена — существующие call sites/тесты не
/// затронуты. Каждый логин нормализуется (lowercase, без UPN/NetBIOS
/// аффиксов) при построении множества, ОДИН раз, не на каждый логин
/// (Pattern 3).
pub fn with_admin_logins(mut self, logins: Vec<String>) -> Self {
    self.admin_logins = Arc::new(
        logins.iter().map(|l| normalize_login_for_admin_check(l)).collect()
    );
    self
}
```

### Pattern 3: Independent normalization helper (do NOT reuse `directory.rs::cache_key` — it's private to another crate)

**What:** `RealAdDirectory`'s `cache_key()` (`directory.rs:69-79`) already implements the exact
normalization needed (strip `@domain` UPN suffix, strip `DOMAIN\` NetBIOS prefix, lowercase) —
but it is a private free function in `trackly-infra::ad::directory`, and more importantly, the
whole point of D-10 is that the admin_logins check must be **structurally independent of the
directory adapter** (works even when `DirectoryError::Unreachable`/`NotConfigured`). Following
this codebase's own established convention ("small independent adapters" — explicitly the
rationale `directory.rs` itself gives at line 51-54 for NOT importing `mock.rs`'s
`normalize_bind_name`), write a small independent copy directly in `auth.rs`.

```rust
/// Normalize a login for admin_logins matching (Phase 32, SSO-02, D-09):
/// strip @domain (UPN) suffix, strip DOMAIN\ (NetBIOS) prefix, lowercase.
/// Independent copy of the same technique used by
/// `RealAdDirectory::cache_key`/`MockAdDirectory::lookup_key` — NOT shared,
/// per this codebase's established "small independent adapters" convention
/// (see `directory.rs` module doc) AND because this check must remain
/// structurally decoupled from the AdDirectory port (D-10: works even when
/// the directory is unreachable/unconfigured).
fn normalize_login_for_admin_check(login: &str) -> String {
    let without_upn = login.split('@').next().unwrap_or(login);
    let without_netbios = without_upn.rsplit('\\').next().unwrap_or(without_upn);
    without_netbios.to_lowercase()
}

impl AuthService {
    /// Локальная set-проверка (D-10 — БЕЗ обращения к каталогу). Требует
    /// правку кода нормализации ОБЕИХ сторон одинаково (config-запись И
    /// login на входе) — иначе `us100` в конфиге не сматчится с
    /// `us100@example.local`/`EXAMPLE\us100`, приходящими с SSO.
    fn is_admin_login(&self, login: &str) -> bool {
        self.admin_logins.contains(&normalize_login_for_admin_check(login))
    }
}
```

### Pattern 4: Reuse the EXISTING "revive with role change" SQL shape

**What:** The blocked/soft-deleted → forced-active-admin transition is NOT new SQL to invent —
`request_service.rs::approve_ad_register`'s `"restore"` branch (lines 782-788) already does
exactly this shape for the manual-approval path:

```sql
-- Source: crates/trackly-app/src/services/request_service.rs:783-788 (existing,
-- proven pattern for the manual-approval "restore" branch — mirror for the
-- forced admin_logins override, same shape, different trigger).
UPDATE users SET role = ?1, is_active = 1, deleted_at_utc = NULL,
     updated_at_utc = ?2, version = version + 1 WHERE id = ?3
```

For the "pending" and "active-non-admin" cases, the same file's `"register"` branch and a
role-only variant apply respectively. **Do not hand-roll new UPDATE shapes** — this exact
4-branch state matrix (unknown/pending/blocked/active) is a straight generalization of code
that already exists and is already tested in this codebase; the only genuinely new piece is the
`is_admin_login` gate and the "close the dangling pending request" step (Pitfall 2 below).

### Anti-Patterns to Avoid

- **Treating admin_logins as "just another `role_hint`" fed into the existing branches
  unchanged.** `role_hint: Option<Role>` from Phase 31 only ever affects the `INSERT` path for
  *unknown* users (`auto_register_ad_user`/`create_pending_registration`) — it does nothing for
  already-existing pending/blocked/active users. D-04/D-06/D-07 require admin_logins to override
  ALL FIVE states, not just the unknown-user INSERT path. Don't just widen `role_hint`'s scope by
  accident and miss the escalation/revival cases — this was the single most important nuance in
  this research (see the state-matrix diagram above).
- **Bumping `version`/`updated_at_utc` on every single login for an already-admin user.** Always
  check current state first; skip the write when the user is already active admin (see the
  no-op row in the state matrix) — every SSO/AD login for that user would otherwise increment
  the optimistic-lock version, which is silently wasteful and pollutes `updated_at_utc` history
  for no behavioral reason.
- **Leaving a dangling open `ad_register`/`register` request for a user forced into admin.** If
  the matched login was in the "pending" state, it has an open `requests` row
  (`request_type='ad_register', ad_subtype='register', status='open'`). If the forced-admin write
  only updates `users` and ignores that row, an admin later opens the Requests screen and sees an
  open registration request for a user who is ALREADY an active admin — confusing, and it can be
  "approved" a second time by an admin unaware of what happened. Close it in the SAME writer tx
  (mirror `request_service.rs`'s manual `UPDATE requests SET status='completed', ...` shown in
  Pattern 4) or explicitly document why it's deliberately left open — do not silently ignore it
  (Common Pitfall #2 below expands on this).
- **Assuming D-12's "gssapi Cargo feature" exists.** It does not (verified this session — grep
  across `Cargo.toml`s finds no `gssapi`/`ntlm` feature anywhere; `ldap3` is pinned
  `default-features = false, features = ["tls-rustls-ring"]` only; the SPNEGO acceptor uses the
  pure-Rust `sspi` crate, unconditionally compiled, no `cfg(windows)` gate). Do not plan work to
  "add Windows feature-gating" — there is none needed. What DOES need planner attention: the
  fmt drift (see Common Pitfalls #1) which is a REAL, verified merge blocker.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Case-insensitive AD login matching | New ad-hoc string comparison | The exact normalize-then-lowercase technique already proven 3x in this codebase (`MockAdClient::lookup_key`, `MockAdDirectory::lookup_key`, `RealAdDirectory::cache_key`/`normalize_bind_name`) — write a 4th small independent copy (Pattern 3), don't invent a different algorithm | Consistency: `us100`, `us100@example.local`, `EXAMPLE\us100` must all resolve to the SAME identity everywhere in the codebase, including this new check |
| "Revive blocked/pending user with new role" SQL | New UPDATE statement invented from scratch | `request_service.rs::approve_ad_register`'s existing "restore"/"register" branch UPDATE shapes (Pattern 4) | Already correct, already tested (11 tests in `ad_register.rs`), handles `version`/`updated_at_utc`/`deleted_at_utc` correctly — copy the shape, don't reinvent |
| Version bump / optimistic lock bookkeeping | Custom guard logic | Existing `version = version + 1` convention + the "skip write if already in target state" idempotency check | This is the established D-Schema-03/04 convention throughout the codebase (see `migrations/V002` comment) |

**Key insight:** Genuinely everything about the DATA-LAYER half of this phase (config field,
writer tx shape, revival SQL, audit_log insert) is a direct mechanical extension of patterns
ALREADY implemented and tested in this exact codebase. The only new *logic* is: (1) the
case-insensitive local-set membership check, and (2) correctly enumerating which of the 5
pre-existing states needs which SQL shape, and closing the dangling pending request. Everything
else is copy-the-shape, not invent-the-shape.

## Common Pitfalls

### Pitfall 1: `cargo fmt --all -- --check` currently FAILS on this branch — verified merge blocker

**What goes wrong:** Both `ci-fast.yml` (`push: branches: ['**']`, i.e. every push) and
`ci-full.yml` (`push: branches: [main]` + every PR) run `cargo fmt --all -- --check` as a
blocking step before `cargo clippy`/`cargo test`. **Independently re-run in this research
session** (not copied from any SUMMARY.md): `cargo fmt --all -- --check` fails with diffs in
~15 files: `crates/trackly-app/src/dto/act.rs`, `crates/trackly-app/src/services/act_service.rs`
(6 locations), `crates/trackly-app/src/pdf/html_templates.rs`, `crates/trackly-app/src/http/sso.rs`,
`crates/trackly-app/src/services/auth.rs` (1 pre-existing location, NOT Phase 32's future edits),
`crates/trackly-infra/src/ad/sso.rs` (5 locations), `crates/trackly-infra/src/ad/keytab.rs`,
`crates/trackly-infra/src/repos/audit_log_sqlite.rs`, and 6 files under `crates/trackly-app/tests/`
(`acts_archived_at.rs`, `acts_date_source.rs`, `acts_update.rs`, `acts_update_return.rs`,
`html_act_render.rs`, `report_returns_sub_number.rs`). This was confirmed identical under BOTH
the pinned `rust-toolchain.toml` version (1.92.0, `rustfmt 1.8.0-stable`) and the CI-pinned
version (1.88, ALSO `rustfmt 1.8.0-stable`, different build date) — this is genuine drift, not a
toolchain-version artifact.
**Why it happens:** Phase 31's own summary (`31-04-SUMMARY.md`, Deviation 2) already flagged this
exact drift as "pre-existing, files this phase never touched" and deliberately left it — a
reasonable per-phase scope decision, but it means the drift has been accumulating un-fixed across
multiple phases and will surface the moment `main`'s CI gate runs it (which it hasn't yet, since
`ci-full` only triggers on PR/push-to-main, and this branch has only had `ci-fast` runs, which
ALSO fails on this — visible in the actual `ci-fast` run history: run `30794811853`,
2026-08-03, status `failure`, fmt-check step).
**How to avoid:** Run `cargo fmt --all` (no `--check`) once, review the diff is purely
whitespace/wrapping (no semantic change — `rustfmt` never changes behavior), commit as its own
`chore` commit BEFORE merging to `main`. This should be an explicit Phase 32 task, not an
afterthought discovered when `ci-full` goes red on `main` after the merge.
**Warning signs:** `cargo fmt --all -- --check` exit code non-zero; `ci-fast` run history showing
`failure` status with `fmt-check`-adjacent step names on THIS branch already (not hypothetical —
already observed, run ID above).

### Pitfall 2: Forced-admin escalation must resolve dangling open `ad_register` requests, not just the `users` row

**What goes wrong:** If a login was previously in the "pending" state (unknown user hit
`ad_auto_accept=OFF` under Phase 31's logic, got an inactive user row + an open
`ad_register`/`register` request) and is LATER added to `admin_logins`, the forced-admin path
must both (a) activate+promote the `users` row and (b) resolve the now-stale open request —
otherwise an admin sees a live "pending registration" request for a user who is already an
active Administrator, and could act on it (approve/reject) with confusing/undefined
consequences for a state the request-service state machine was never designed to reach.
**Why it happens:** The two tables (`users`, `requests`) are updated by two different code paths
today (`on_ad_bind_success` vs `RequestService::approve_ad_register`) — it's easy to update one
and forget the other exists.
**How to avoid:** In the SAME writer transaction that promotes the `users` row, also
`UPDATE requests SET status='completed', ... WHERE ... AND status='open'` for that user's
open `ad_register`/`register` request (mirror the exact SQL already in
`request_service.rs:820-826`), plus an `audit_log` entry noting the auto-resolution. Write a
test asserting the request's `status` becomes `'completed'` (or explicitly document + test that
it's deliberately left `'open'` if the planner decides otherwise — but don't leave this
unaddressed/untested either way).

### Pitfall 3: Config change requires a process restart — no live reload

**What goes wrong:** `admin_logins` is read once at `AppCtx::build()` time (like every other
TOML-only AD field) and baked into the in-memory `AuthService.admin_logins` set. If an admin
edits `trackly.config.toml` to add a login while the app/server is already running, nothing
happens until the process restarts — this matches EVERY other TOML-only AD setting (`host`,
`base_dn`, `role_mapping`, `bind_dn`, etc.), so it's not a NEW inconsistency, but it IS a common
first-time-user support question ("I added myself to admin_logins and it's still not working").
**How to avoid:** Document this explicitly in `trackly.config.toml.example`'s comment for the
new field (mirror the existing "требует перезапуска" convention if one exists elsewhere in that
file, or add one) and in the plan's verification notes — this is a documentation/comment task,
not a code task.

### Pitfall 4: Injection-point choice affects whether the LDAPS password-bind path also gets auto-admin

**What goes wrong:** ROADMAP/REQUIREMENTS wording says "при SSO-входе" (at SSO login), and
Success Criteria 1-2 both say "SSO-вход" specifically — but the CONTEXT.md code-context notes
explicitly leave open WHERE the check sits ("в начале `on_ad_bind_success` (или в
`sso_login`/`try_ad_login`) — важно лишь итоговое поведение"). `on_ad_bind_success` is the ONE
function shared by BOTH `sso_login` (passwordless) and `try_ad_login` (LDAPS username+password
fallback, Phase 9). Injecting at `on_ad_bind_success` (top) makes admin_logins apply to BOTH
entry points — full `ADMIN_AD_LOGINS` parity (the adwebapp reference this phase is modeled on
applies its own equivalent list regardless of auth mechanism). Injecting only inside `sso_login`
(before its call to `on_ad_bind_success`) would make it SSO-only, matching the literal
requirement text but creating an inconsistency where the SAME AD account gets different
treatment depending on which login mechanism they happen to use that day.
**How to avoid:** This is a real design decision, not a pure implementation detail — see Open
Questions #1. This research recommends `on_ad_bind_success` (shared, DRY, matches the reference
project's actual semantics) but flags it explicitly for planner/user confirmation given the
literal requirement wording says "SSO".

## Code Examples

### Forced-admin state-machine skeleton (illustrative, not final — planner refines)

```rust
// New branch at the TOP of on_ad_bind_success, before the existing match:
async fn on_ad_bind_success(
    &self,
    login: &str,
    display_name: &str,
    role_hint: Option<Role>,
) -> Result<UserDto, AppError> {
    if self.is_admin_login(login) {
        return self.force_admin_provisioning(login, display_name).await;
    }
    // ... EXISTING match, UNCHANGED (D-08) ...
}

async fn force_admin_provisioning(
    &self,
    login: &str,
    display_name: &str,
) -> Result<UserDto, AppError> {
    match self.find_user_any_state(login).await? {
        None => self.force_admin_insert(login, display_name).await,
        Some(u) if u.is_active && !u.deleted && u.role == "admin" => {
            // Already admin, active — no-op write, just return the session.
            self.get_by_login(login).await
        }
        Some(u) if u.is_active && !u.deleted => {
            self.force_admin_promote_active(u.id, login).await
        }
        Some(u) if !u.is_active && !u.deleted && u.has_open_register_request => {
            self.force_admin_activate_pending(u.id, login).await
        }
        Some(u) => self.force_admin_revive_blocked(u.id, login).await, // blocked/deleted
    }
}
```

### Config field + test (mirrors `role_mapping`'s own test style exactly)

```rust
// config.rs — new test, same file/style as role_mapping_array_of_tables_deserializes:
#[test]
fn admin_logins_flat_array_deserializes_and_defaults_empty() {
    let empty: AppConfig = toml::from_str("").expect("empty config parses");
    assert_eq!(empty.ad.admin_logins, Vec::<String>::new());

    let toml_str = "[ad]\n\
         enabled = true\n use_mock = false\n host = \"dc1.example.local\"\n \
         port = 636\n domain = \"example.local\"\n base_dn = \"dc=example,dc=local\"\n \
         name_attr = \"displayName\"\n no_tls_verify = false\n \
         admin_logins = [\"us100\", \"us777\"]\n";
    let cfg: AppConfig = toml::from_str(toml_str).expect("admin_logins parses");
    assert_eq!(cfg.ad.admin_logins, vec!["us100".to_string(), "us777".to_string()]);
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|---------------|--------|
| `on_ad_bind_success` has exactly 4 branches (active/pending/blocked-or-deleted/unknown), all deriving role from `role_hint` (Phase 31) or hardcoded `'employee'` | A 5th, PRIORITY-0 branch (`is_admin_login`) short-circuits all 4 into a single forced-admin outcome BEFORE the existing match runs | This phase | The existing 4-branch match itself is untouched (D-08) — the new branch wraps it, doesn't rewrite it |
| First-administrator bootstrap requires either an existing admin approving a pending request, OR `needs_bootstrap()`'s local-account first-run wizard | AD-only orgs get a THIRD bootstrap path: config-listed logins become admin on first SSO/AD login, no existing admin or local-wizard needed | This phase | Solves the "first administrator" problem for organizations that never create a local admin account and rely on AD/SSO exclusively |

**Deprecated/outdated:** none — additive only.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | The injection point should be `on_ad_bind_success` (shared by SSO + LDAPS password bind), not `sso_login` alone | Architecture Patterns / Common Pitfalls #4 | Medium — if the user/planner wants admin_logins to apply ONLY to the passwordless SSO path (matching the literal requirement wording) and NOT to the LDAPS username+password fallback, the injection point moves to `sso_login`'s pre-call to `on_ad_bind_success`, and a second, explicit test must prove `try_ad_login` is UNCHANGED. This is the single highest-value decision for the planner/discuss step to lock explicitly — recommend confirming with the user if not already implicit in "SSO-02" naming. |
| A2 | A dangling pending `ad_register` request should be auto-completed (not left open) when its user is force-promoted to admin | Common Pitfalls #2 | Low-medium — if left open, no functional bug occurs immediately, but an admin could later "approve" or "reject" a request for an already-active admin, hitting an edge case the `RequestService` state machine wasn't designed for (not verified what happens — untested territory either way, so closing it defensively is the safer default) |
| A3 | `cargo fmt --all` output (once run) will be purely whitespace/wrapping with zero semantic diffs | Common Pitfalls #1 | Low — this is `rustfmt`'s documented guarantee (never changes program behavior, AST-preserving), but the planner should still run the full test suite after the fmt commit as a sanity check, not just eyeball the diff |
| A4 | No Cargo feature (`gssapi`/`ntlm`/etc.) needs to be added or verified for the merge — the actual SSO implementation (`sspi` crate) has no feature gate and already builds/tests cross-platform | Summary / Common Pitfalls (Anti-Patterns) | Low — directly verified via `grep` across all `Cargo.toml` files in this session (no matches for `gssapi`/`ntlm`/`cfg(windows)` anywhere near AD/SSO code); if this is somehow wrong, the fallback is simply "ci-full already tests this on windows-latest today via the mock path," which the planner can re-confirm by triggering `ci-full` on the branch before merging |

**If this table is empty:** N/A — see A1 above as the one item that should ideally get an
explicit user/discuss-phase confirmation before the planner locks the design, since it changes
which functions get modified and which regression tests are required.

## Open Questions

1. **Does admin_logins apply to BOTH `sso_login` and `try_ad_login` (LDAPS password fallback), or SSO only?**
   - What we know: CONTEXT.md's own "Claude's Discretion" section explicitly leaves the exact
     injection point open ("важно лишь итоговое поведение из D-04..D-08"). The requirement ID
     is literally named "SSO-02" and ROADMAP Success Criteria 1-2 both say "при... SSO-входе".
     But `on_ad_bind_success` is the ONE function both paths share, and the reference project
     (`adwebapp`'s `ADMIN_AD_LOGINS`) this phase is explicitly modeled on applies its list
     regardless of auth mechanism.
   - What's unclear: whether "SSO" in the requirement text is being used loosely (as shorthand
     for "any AD-authenticated login," since that's the ONLY passwordless mechanism this phase's
     milestone is about) or precisely (only the Kerberos/SPNEGO entry point, deliberately
     excluding the older LDAPS-bind fallback).
   - Recommendation: Inject at `on_ad_bind_success` (both paths covered) for DRY-ness and
     `ADMIN_AD_LOGINS` parity — this is the research recommendation — but flag this explicitly
     for the planner to either confirm via a quick user check or make an explicit, documented
     locked decision in the PLAN.md itself, since it determines whether `try_ad_login`'s
     existing tests need a NEW admin_logins regression case too (recommended either way, low
     cost, high value for later maintainers).

2. **Should the audit_log action name be `ad_auto_admin`, or something more specific per-transition (e.g. distinguishing "first-time grant" from "escalation" from "revival")?**
   - What we know: CONTEXT.md's discretion section suggests `ad_auto_admin` as one option. The
     existing convention uses distinct action strings per INSERT/UPDATE shape elsewhere
     (`ad_auto_register` vs `ad_pending_register` vs `ad_register_approve` are all different
     strings for conceptually related but distinct transitions).
   - What's unclear: whether a single `ad_auto_admin` action string (with the prior state
     captured in `payload_json`) is sufficient audit granularity, or whether the 4 distinct
     forced-admin transitions (insert/promote/activate-pending/revive) deserve 4 distinct action
     strings for easier audit-log querying later.
   - Recommendation: One action string (`ad_auto_admin`) with `payload_json` capturing
     `{"prior_state": "unknown"|"pending"|"blocked"|"active_employee"|"active_manager", ...}` —
     matches this codebase's general preference for a small, stable action-string vocabulary
     plus a JSON payload for detail (see `ad_register_approve`'s own `payload_json: {"role": ...}`
     pattern). Low-risk either way; planner's call.

3. **Does a full `ci-full` dry-run (via `workflow_dispatch` or a throwaway PR) need to be triggered on this branch BEFORE the real merge, to catch anything beyond the fmt drift already found?**
   - What we know: `ci-full.yml` only runs on `pull_request` or `push: branches: [main]` — this
     branch has never had `ci-full` run against it (only `ci-fast`, which is ubuntu-only and
     already shows the fmt failure). `ci-full` additionally runs the ProcMon Windows portable-mode
     check and the full 3-OS matrix — genuinely untested on this branch so far.
   - What's unclear: whether opening a real PR from `spike/ad-sso-kerberos` → `main` (which
     would trigger `ci-full` for free, as a dry run, before actually merging) is acceptable
     process, or whether the user prefers a direct merge+push with manual verification.
   - Recommendation: Open the PR (even if squash-merged or fast-forwarded at the end) specifically
     BECAUSE it gives a free, real `ci-full` run (all 3 OSes + ProcMon) as a pre-merge gate —
     cheaper than discovering a Windows-only failure after `main` is already red. This is a
     process recommendation, not a code change.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Local rustfmt/clippy toolchain | Verifying fmt/clippy state before merge | ✓ | rustfmt 1.8.0-stable (both 1.88 and 1.92.0 toolchains produce identical fmt diffs — confirmed, not a version-skew artifact) | — |
| `cargo clippy --workspace --all-targets -- -D warnings` full re-run | Merge-readiness gate | Attempted this session; did not complete within the session's time budget (large Tauri workspace, multi-minute compile) — NOT independently re-verified fresh in this session | — | Phase 31's own `31-VERIFICATION.md` (2026-08-03T21:30Z, same day) independently re-ran `cargo clippy -p trackly-core -p trackly-app -p trackly-infra -- -D warnings` and got 0 warnings; the planner should re-run the FULL `--workspace --all-targets` variant once during Phase 32 execution as part of the merge-readiness gate, budgeting several minutes |
| `ci-full.yml` actual run on THIS branch | Confirming the 3-OS matrix + ProcMon are green before merge (D-12) | ✗ (never triggered on this branch — only `ci-fast`, which already shows fmt failure) | — | Trigger via a PR from `spike/ad-sso-kerberos` → `main` (see Open Question #3), or `workflow_dispatch` if `ci-full` supported it (it currently does not have a `workflow_dispatch` trigger — PR is the only free path) |
| Real AD/Domain Controller | Live verification of the forced-admin path end-to-end on a real domain | ✗ (dev macOS, per standing project constraint) | — | `MockAdDirectory`/`MockAdClient` fixtures (already established, `TRACKLY_AD_MOCK`) — extend with a 3rd fixture identity if a distinct admin_logins test identity is wanted, or reuse `us200`/`us300` etc. |

**Missing dependencies with no fallback:** none — every gap above has an established fallback
(mock path for AD, PR-trigger for `ci-full`, Phase 31's recent clippy result as interim evidence).

**Missing dependencies with fallback:** see table above.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | `cargo test` (workspace), per-crate/per-test targeted runs (memory: never run 2 `cargo test` concurrently — contends on `target/` lock) |
| Config file | Inline `#[cfg(test)] mod tests` in `config.rs`/`auth.rs`; new integration test file under `crates/trackly-app/tests/` (mirrors `ad_directory_sso.rs`/`ad_register.rs` convention) |
| Quick run command | `cargo test -p trackly-infra config::` (config parsing) + `cargo test -p trackly-app --test ad_admin_logins` (new file, name TBD by planner) |
| Full suite command | `cargo test --workspace --no-fail-fast -- --test-threads=1` (matches `ci-fast`/`ci-full`'s own invocation exactly — includes `--test-threads=1`, required per this codebase's own documented deadlock/contention history on the ubuntu runner) |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| SSO-02 | `admin_logins` TOML field parses (flat array), defaults to empty | unit | `cargo test -p trackly-infra config::admin_logins` | ❌ Wave 0 |
| SSO-02 | Unknown login in admin_logins → INSERT active admin, no pending request row created | integration | new `ad_admin_logins.rs`-style file | ❌ Wave 0 |
| SSO-02 | Pending user's login added to admin_logins → activated as admin AND their open `ad_register` request is auto-completed (or explicitly asserted otherwise per Open Q #2) | integration | same file | ❌ Wave 0 |
| SSO-02 | Blocked/soft-deleted user's login in admin_logins → revived as active admin (overrides manual block, D-07) | integration | same file | ❌ Wave 0 |
| SSO-02 | Existing active non-admin user's login added to admin_logins → escalated to admin on next login (D-06) | integration | same file | ❌ Wave 0 |
| SSO-02 | Login already active admin, still in admin_logins → idempotent no-op (no version bump — assert `version` unchanged across two logins) | integration | same file | ❌ Wave 0 |
| SSO-02 | Login NOT in admin_logins → Phase 31 behavior fully unchanged (regression) | integration | re-run existing `ad_directory_sso.rs`/`ad_register.rs` UNCHANGED + one new explicit "not-in-list, admin_logins non-empty" case | ❌ Wave 0 (new case) / ✓ (existing suite) |
| SSO-02 | admin_logins forces admin even when `AdDirectory::resolve` returns `Unreachable`/`NotConfigured` (D-10 — independent of directory) | integration | same file, reuse `MockAdDirectory::unreachable()` | ❌ Wave 0 |
| SSO-02 | Case-insensitive + UPN/NetBIOS-form matching (`us100`, `US100@example.local`, `EXAMPLE\us100` all match config entry `us100`) | unit | `cargo test -p trackly-app services::auth::tests::admin_login` (or wherever `normalize_login_for_admin_check` lives) | ❌ Wave 0 |
| SSO-02 | Empty/absent admin_logins → feature fully off (regression baseline) | integration | reuse ALL existing Phase 31 tests unmodified with the new `with_admin_logins` builder simply never called (default empty) | ✓ (already true by construction) |
| Operational (D-11/D-12) | `cargo fmt --all -- --check` passes workspace-wide | ci gate | `cargo fmt --all -- --check` | ❌ currently FAILS — fix required |
| Operational (D-11/D-12) | `cargo clippy --workspace --all-targets -- -D warnings` passes | ci gate | same command | ⚠️ not re-verified fresh this session (see Environment Availability) — planner must re-run |
| Operational (D-11/D-12) | `ci-full.yml` matrix green on all 3 OSes + ProcMon | ci gate | trigger via PR (Open Question #3) | ❌ never run on this branch |

### Sampling Rate

- **Per task commit:** `cargo test -p trackly-infra config::` and/or `cargo test -p trackly-app --test <new_file>` (whichever the task touches)
- **Per wave merge:** `cargo test --workspace --no-fail-fast -- --test-threads=1` + `cargo clippy --workspace --all-targets -- -D warnings` + `cargo fmt --all -- --check`
- **Phase gate:** Full suite green + `cargo fmt --all -- --check` green (currently NOT — must be fixed) + a real `ci-full` run green (via PR) before the `main` merge step; tag `v1.3.0` only after `main`'s own `ci-full` run (triggered by the merge push) is green.

### Wave 0 Gaps

- [ ] `crates/trackly-infra/src/config.rs` — `admin_logins: Vec<String>` field + Default + Debug (safe to include, no secret) + parsing tests
- [ ] `crates/trackly-app/src/services/auth.rs` — `admin_logins: Arc<HashSet<String>>` field, `with_admin_logins` builder, `normalize_login_for_admin_check` free fn, `is_admin_login` method, `force_admin_provisioning` + 4 sub-helpers, injection point in `on_ad_bind_success`
- [ ] `crates/trackly-app/src/context.rs` — one new line: `.with_admin_logins(config.ad.admin_logins.clone())` on the existing `AuthService::new(...)` builder chain
- [ ] New integration test file under `crates/trackly-app/tests/` covering the full state matrix (name TBD, e.g. `ad_admin_logins.rs`)
- [ ] `trackly.config.toml.example` — document `admin_logins` next to the existing `role_mapping` block, including the "requires restart" note (Pitfall 3)
- [ ] **Pre-existing, out-of-phase-scope but merge-blocking:** `cargo fmt --all` run + commit (Pitfall 1) — must land on this branch before/at merge time, whether as its own Phase 32 task or a preceding `chore` commit
- [ ] A PR from `spike/ad-sso-kerberos` → `main` (or equivalent) to get a real, free `ci-full` run before the actual merge (Open Question #3)

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-------------------|
| V4 Access Control | **yes — the core of this phase** | This is an explicit, config-authored PRIVILEGE ESCALATION path that overrides manual admin blocking (D-07). Standard control here is NOT "prevent escalation" (that's the feature's whole purpose) but "make it auditable, deterministic, and impossible to trigger except via deployment-time config" — audit_log entry per Common Pitfall discussion, no runtime/UI mutation path (D-01/D-03), local-only set check (D-10) so it can't be influenced by a compromised/unreachable directory |
| V9 Data Protection (audit trail) | yes | Every forced-admin transition MUST write an `audit_log` row (entity_type='user', action='ad_auto_admin' or similar) capturing the PRIOR state — this is the only durable record that a human didn't approve this specific elevation; treat its absence as a phase-blocking gap, not a nice-to-have |
| V1 Architecture (config-as-authority) | yes | D-01/D-02/D-03 make `trackly.config.toml` the sole source of truth for this list — whoever controls deployment of that file (gitignored, filesystem access to the exe's directory) has an implicit "can create an admin" capability. This is a real, already-accepted trust boundary (matches `ADMIN_AD_LOGINS`'s own reference-project threat model) — document it explicitly in the config example's comment (mirrors the existing `bind_password` "never commit real values" warning already in that file) rather than leaving it implicit |
| V5 Input Validation | low | `admin_logins` entries are simple strings compared via exact (normalized) match, not interpolated into any LDAP filter or SQL string concatenation (`rusqlite` bound params throughout, per established convention) — no injection surface here, unlike the LDAP filter-escaping concerns in Phase 31 |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|----------------------|
| Config-file tampering grants unauthorized admin access | Elevation of Privilege | This is an ACCEPTED, intentional trust boundary (whoever can edit `trackly.config.toml` on the server/portable-install filesystem already has broader control than the app itself can defend against — same trust level as being able to swap the `.exe`) — document, don't attempt to defend against it in-app |
| Forgetting to close the dangling pending request lets an admin action later "undo" or duplicate-approve an auto-granted admin | Tampering / Repudiation | Auto-complete the open request in the SAME writer tx (Common Pitfall #2) |
| Silent privilege escalation with no audit trail | Repudiation | Mandatory `audit_log` INSERT in every forced-admin write path, in the same transaction (never best-effort/fire-and-forget for this specific action) |
| Race between two concurrent logins for the same newly-added admin_logins entry | Tampering (low severity) | Single-writer serialization already guarantees this is safe — both requests funnel through the same `WriterHandle`/`spawn_blocking` worker, no new race surface introduced |

## Sources

### Primary (HIGH confidence — all verified directly in this session)

- `crates/trackly-app/src/services/auth.rs` (this repo) — `on_ad_bind_success` (line 404),
  `auto_register_ad_user` (531), `create_pending_registration` (604), `find_user_any_state`
  (1049), `AuthService` struct/constructor (144-179) — read in full for this research
- `crates/trackly-app/src/services/request_service.rs` (this repo) — `approve_ad_register`
  (747-865), the "restore"/"register" UPDATE shapes to mirror (782-796), the request-completion
  UPDATE shape to mirror for closing dangling requests (819-826)
- `crates/trackly-infra/src/config.rs` (this repo) — `AdConfig`/`RoleMappingEntry` (124-288),
  `AppConfig::load_or_default` (290-320), existing test conventions (322-419)
- `crates/trackly-infra/src/ad/directory.rs` (this repo) — `cache_key`/`normalize_bind_name`
  normalization technique to mirror independently (38-79)
- `crates/trackly-infra/src/ad/directory_mock.rs` (this repo) — `MockAdDirectory` fixture/lookup
  pattern to extend for new test fixtures if needed
- `crates/trackly-app/src/context.rs` (this repo) — `AuthService::new(...)` construction site
  (302-332), `ActService::new(...).with_pdf_pipeline(...).with_org_db(...)` builder precedent
  (275-279)
- `crates/trackly-app/src/services/act_service.rs` (this repo) — `with_pdf_pipeline`/`with_org_db`
  builder pattern (63-109), explicitly documented rationale for NOT breaking existing call sites
- `crates/trackly-core/src/auth.rs` (this repo) — `Role` enum, `from_str`/`as_str` (18-54)
- `migrations/V002__core_entities.sql` (this repo) — `users` table schema, confirms `role`/
  `is_active`/`deleted_at_utc`/`version` columns and semantics
- `.github/workflows/release.yml` (this repo) — confirmed `v*.*.*` tag trigger only (lines
  9-11), confirmed version is patched transiently at BUILD time via `perl -0pi` (lines 142-173),
  NOT via a persistent `Cargo.toml` edit — `git show main:Cargo.toml` confirms `main` itself
  stays at `version = "0.1.0"` permanently
- `.github/workflows/ci-fast.yml` / `.github/workflows/ci-full.yml` (this repo) — confirmed
  trigger scopes (`ci-fast`: every push; `ci-full`: PR + push-to-main only), confirmed BOTH gate
  on `cargo fmt --all -- --check` before clippy/test
- **Directly executed in this session:** `cargo fmt --all -- --check` (both rustfmt 1.92.0 and
  1.88 toolchains) — confirmed ~15 files with drift, none in Phase 32's planned scope
- **Directly executed in this session:** `gh run list --branch spike/ad-sso-kerberos` — confirmed
  the most recent `ci-fast` run (30794811853, 2026-08-03) failed, and its log (`gh run view
  --log-failed`) shows the fmt-check step failing with the SAME file set
- **Directly executed in this session:** `grep -rn "gssapi\|cfg(target_os = \"windows\")"` across
  all `Cargo.toml`/AD source files — zero matches, confirming D-12's Cargo-feature premise does
  not match the actual code
- `.planning/phases/31-ad-bind-ad/31-RESEARCH.md`, `31-04-SUMMARY.md`, `31-VERIFICATION.md` (this
  repo) — Phase 31's provisioning seam, its own clean clippy re-verification (2026-08-03T21:30Z),
  and its own explicit note about the pre-existing fmt drift (Deviation 2)

### Secondary (MEDIUM confidence)

- None — this phase required no external documentation lookups; every claim traces to this
  codebase or a directly-run command in this session.

### Tertiary (LOW confidence)

- None flagged.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new dependencies, pure extension of existing config/service patterns
- Architecture: HIGH — every pattern cited traces to an existing, already-tested code path in
  this exact codebase (state-matrix generalization of `request_service.rs`'s existing branches)
- Pitfalls: HIGH — the fmt-drift finding and the gssapi-premise correction were both independently
  verified via direct command execution in this session, not inferred or assumed
- Merge/release mechanics: HIGH — `release.yml`/`ci-fast.yml`/`ci-full.yml` triggers and version-
  patching mechanism read and confirmed directly; actual failing CI run inspected via `gh run view`

**Research date:** 2026-08-03
**Valid until:** 30 days (stable, additive extension of an already-locked architecture; re-check
sooner only if `main`'s CI workflows change, or if the fmt drift is independently fixed by another
change before Phase 32 execution begins — re-run `cargo fmt --all -- --check` at planning time to
confirm the drift list is still current)

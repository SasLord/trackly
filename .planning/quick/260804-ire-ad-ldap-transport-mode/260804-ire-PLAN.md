---
quick_id: 260804-ire
slug: ad-ldap-transport-mode
title: AD LDAP transport mode (plaintext/StartTLS opt-in)
created: 2026-08-04
mode: quick
---

# Quick Task 260804-ire: AD LDAP transport mode (plaintext/StartTLS opt-in)

## Problem

`RealAdDirectory::resolve()` (`crates/trackly-infra/src/ad/directory.rs:142`) and
`RealAdClient::authenticate`/`test_connection` (`crates/trackly-infra/src/ad/real.rs:76,143`)
all hardcode `format!("ldaps://{host}:{port}")`. The user's live DC
(`srvdc1.cmy.local`) serves ONLY plaintext LDAP on :389 — port 636 is
TCP-open but the TLS handshake is forcibly closed, so there is no working
LDAPS path. `no_tls_verify = true` does not help because the URL scheme
itself is still `ldaps://`. Result: `directory.resolve()` returns
`DirectoryError::Unreachable`, SSO degrades to bare login, ФИО shows as the
raw UPN, and group→role mapping never runs.

## Approach

Add a typed `[ad] ldap_tls_mode` config toggle (`"ldaps"` default | `"plain"`
| `"starttls"`), factor URL/`LdapConnSettings` construction into ONE shared
helper (`crates/trackly-infra/src/ad/transport.rs`), and wire all three call
sites through it. `port` becomes `Option<u16>` (`#[serde(default)]`) so
"unset" is distinguishable from "explicitly pinned" — an explicit `port`
always wins; otherwise the port is derived from the mode (636 for `ldaps`,
389 for `plain`/`starttls`). `plain` mode logs a one-time `tracing::warn!` at
config-load time (not per bind attempt) since `AppConfig::load_or_default`
is called exactly once per process (`main.rs:47`).

ldap3 0.12 API note (verified against the vendored crate source at
`~/.cargo/registry/.../ldap3-0.12.1/src/conn.rs`): StartTLS is triggered by
connecting to an `ldap://` URL with `LdapConnSettings::set_starttls(true)` —
there is no separate `starttls://` scheme to construct manually.

Both new config fields (`port`, `ldap_tls_mode`) use `#[serde(default)]`,
mirroring how `bind_dn`/`role_mapping`/`admin_logins` were added as optional
in phases 31/32 — this keeps existing `trackly.config.toml` files parsing
unchanged (backward compat is non-negotiable per the task constraints).

**Security note:** `plain` mode sends the service-account bind password and
user credentials in cleartext on the LAN. This is an explicit, documented
opt-in (default stays `ldaps`), gated by a startup warning — no additional
mitigation is in scope (the DC's own lack of a working LDAPS listener is the
threat surface owner's decision, not this codebase's to silently work
around).

## Tasks

### Task 1 — Config layer: `LdapTlsMode` + optional `port` + one-time warning

**File:** `crates/trackly-infra/src/config.rs`

- Add `pub enum LdapTlsMode { Ldaps, Plain, StartTls }` with
  `#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Default)]`,
  `#[serde(rename_all = "lowercase")]`, `#[default]` on `Ldaps` (serializes
  to `"ldaps"`/`"plain"`/`"starttls"` — matches the required TOML values
  exactly). Add `impl LdapTlsMode { pub fn default_port(&self) -> u16 }` →
  636 for `Ldaps`, 389 for `Plain`/`StartTls`.
- Change `AdConfig::port` from `pub port: u16` to
  `#[serde(default)] pub port: Option<u16>`. Add
  `impl AdConfig { pub fn resolved_port(&self) -> u16 { self.port.unwrap_or_else(|| self.ldap_tls_mode.default_port()) } }`.
- Add `#[serde(default)] pub ldap_tls_mode: LdapTlsMode` field to `AdConfig`,
  doc-commented with the three modes and the plaintext caveat.
- Update `impl Default for AdConfig`: `port: None`, `ldap_tls_mode: LdapTlsMode::Ldaps`.
- Update the manual `impl std::fmt::Debug for AdConfig` to add
  `.field("ldap_tls_mode", &self.ldap_tls_mode)` (not a secret — print as-is,
  `port` field already prints via existing `.field("port", &self.port)`,
  which still compiles unchanged against `Option<u16>`).
- In `AppConfig::load_or_default`: change the trailing
  `toml::from_str::<Self>(&contents).map_err(...)` expression into a
  `let parsed: Self = toml::from_str(&contents).map_err(...)?;` binding, then
  `if parsed.ad.ldap_tls_mode == LdapTlsMode::Plain { tracing::warn!("..."); }`,
  then `Ok(parsed)`. This fires exactly once per process because
  `load_or_default` is called exactly once at `crates/trackly-app/src/main.rs:47`
  — do not add a `std::sync::Once`/static guard, it would be over-engineering
  for a single call site. Warning text in English (matches existing
  `tracing::warn!` message style in `directory.rs`, e.g. "unparseable role in
  AD role_mapping entry, skipping" — log messages are English, only
  UI/config-comment strings are Russian per this codebase's convention).
- Extend the existing `#[cfg(test)] mod tests` block (continue the
  "Behavior N" numbering already at 5) with:
  - Behavior 6: empty config (no `[ad]` section) → `ldap_tls_mode ==
    LdapTlsMode::Ldaps`, `port == None`, `resolved_port() == 636`.
  - Behavior 7: an `[ad]` section present but omitting BOTH `ldap_tls_mode`
    and `port` still parses (proves both are now optional) and resolves to
    `Ldaps` + 636 — backward compat for existing configs upgrading to a
    newer binary without editing their TOML.
  - Behavior 8: `ldap_tls_mode = "plain"` without an explicit `port` →
    `resolved_port() == 389`. A sibling assertion (or separate test) for
    `"starttls"` → also 389.
  - Behavior 9: `ldap_tls_mode = "plain"` WITH an explicit `port = 636` →
    `resolved_port() == 636` (explicit always wins, even against the "other"
    mode's conventional port).
  - A pure-fn test: `LdapTlsMode::Ldaps.default_port() == 636`,
    `LdapTlsMode::Plain.default_port() == 389`,
    `LdapTlsMode::StartTls.default_port() == 389`.

<verify>
<automated>cargo test -p trackly-infra --lib config::tests::</automated>
</verify>
<done>New enum + optional port + resolved_port() + one-time warning compile; all Behavior 1-9 tests pass; existing Behavior 1-5 tests still pass unmodified (proves backward compat holds).</done>

### Task 2 — Shared transport helper + wire all three LDAP call sites

**Files:** `crates/trackly-infra/src/ad/transport.rs` (new),
`crates/trackly-infra/src/ad/mod.rs`,
`crates/trackly-infra/src/ad/real.rs`,
`crates/trackly-infra/src/ad/directory.rs`

- Create `ad/transport.rs`:
  - `pub(crate) const CONN_TIMEOUT: Duration = Duration::from_secs(5);`
    (moved from the identical private consts currently duplicated in
    `real.rs` and `directory.rs`).
  - `pub(crate) fn build_ldap_conn(cfg: &AdConfig) -> (String, ldap3::LdapConnSettings)`:
    resolves `port = cfg.resolved_port()`, builds
    `LdapConnSettings::new().set_conn_timeout(CONN_TIMEOUT).set_no_tls_verify(cfg.no_tls_verify)`,
    then per `cfg.ldap_tls_mode`: `Ldaps` → url
    `format!("ldaps://{}:{}", cfg.host, port)`, settings unchanged (starttls
    stays false, matches today's behavior exactly); `Plain` → url
    `format!("ldap://{}:{}", cfg.host, port)`, settings unchanged; `StartTls`
    → url `format!("ldap://{}:{}", cfg.host, port)`, settings gains
    `.set_starttls(true)`. Return `(url, settings)`.
  - Unit tests (module-private, no live LDAP connection needed — assert on
    the returned URL string and `LdapConnSettings::starttls()`, the only
    public getter the crate exposes; `no_tls_verify` has no public getter so
    is not independently assertable, note this in a code comment rather than
    skipping coverage silently):
    - `ldaps` mode → `"ldaps://<host>:636"`, `settings.starttls() == false`.
    - `plain` mode → `"ldap://<host>:389"`, `settings.starttls() == false`.
    - `starttls` mode → `"ldap://<host>:389"`, `settings.starttls() == true`.
    - explicit `port` (e.g. `Some(2389)`) on any mode → port appears verbatim
      in the built URL, overriding the mode default.
- `ad/mod.rs`: add `pub mod transport;` (consistent with every other
  submodule in this file already being `pub mod`).
- `real.rs` (both call sites, `authenticate` at line ~76 and
  `test_connection` at line ~143): replace the
  `let settings = LdapConnSettings::new()...; let url = format!("ldaps://...");`
  pair with `let (url, settings) = crate::ad::transport::build_ldap_conn(&self.cfg);`.
  Remove the now-unused local `CONN_TIMEOUT` const and the now-unused
  `LdapConnSettings` import from the `use ldap3::{...}` line (keep
  `ldap_escape, LdapConnAsync, Scope, SearchEntry`).
- `directory.rs` (`resolve`, line ~142): same replacement. Remove the local
  `CONN_TIMEOUT` const and unused `LdapConnSettings` import. Update the
  existing test fixture `unreachable_but_configured_cfg()` (~line 348):
  `port: 1,` → `port: Some(1),` (type changed from `u16` to `Option<u16>`;
  `resolved_port()` still returns `1` since an explicit port always wins,
  preserving the test's "nothing listens here" intent).

<verify>
<automated>cargo test -p trackly-infra --lib ad::transport::tests:: && cargo test -p trackly-infra --lib ad::directory::tests::</automated>
</verify>
<done>build_ldap_conn is the single source of URL/settings construction for all three call sites (grep confirms zero remaining `format!("ldaps://` literals outside transport.rs); directory.rs's cache-short-circuit and unreachable-lookup tests still pass with the `Option<u16>` port field.</done>

### Task 3 — DTO surface, example config docs, cross-crate verification

**Files:** `crates/trackly-app/src/http/auth.rs`,
`crates/trackly-app/src/tauri_cmds/auth.rs`,
`trackly.config.toml.example`

- `http/auth.rs::build_settings_get_ad` (line ~261) and
  `tauri_cmds/auth.rs::build_settings_get_ad_tauri` (line ~385): change
  `port: ad_config.port as i64` to `port: ad_config.resolved_port() as i64`
  in both — `AdSettingsDto.port` (`i64`) must keep reporting the port that
  will actually be dialed, not `None`/a raw `Option` leak. `ldap_tls_mode`
  is intentionally NOT added to `AdSettingsDto` — out of scope per this
  task's required_scope (backend transport toggle only, no Settings-UI
  surface requested).
- `trackly.config.toml.example`: in the `[ad]` block, immediately after the
  existing `# port = 636` comment, add (Russian, matching the file's
  existing comment style):
  - `ldap_tls_mode` description: values `ldaps` (по умолчанию, TLS с
    самого начала) | `plain` (незашифрованный LDAP — пароли идут в
    открытом виде, только для DC без рабочего LDAPS) | `starttls` (LDAP с
    апгрейдом до TLS через StartTLS).
  - Note that `port` now defaults per-mode when left commented out (636 for
    `ldaps`, 389 for `plain`/`starttls`) and that an uncommented `port`
    always overrides the mode default.
  - Explicit security caveat for `plain`: пароль служебной учётной записи и
    учётные данные пользователей передаются в открытом виде по сети.
- Cross-crate verification (run sequentially — never two `cargo test`
  invocations concurrently, per this project's `target/` lock convention):
  - `cargo build -p trackly-app` (compiles the two DTO call-site edits).
  - `cargo test -p trackly-infra --lib` (full trackly-infra unit-test suite —
    confirms nothing else in the crate regressed).
  - `cargo test -p trackly-infra --test config_test` (existing integration
    suite for `AppConfig`, unaffected by this change but confirms no
    collateral breakage).
  - `cargo clippy -p trackly-infra -p trackly-app --all-targets -- -D warnings`.
  - `cargo fmt -p trackly-infra -p trackly-app` (apply, not `--check` — this
    repo has pre-existing `fmt --check` drift elsewhere per prior session
    notes; formatting only the two touched crates avoids surfacing
    unrelated drift as a false failure).

<verify>
<automated>cargo build -p trackly-app && cargo test -p trackly-infra --lib && cargo clippy -p trackly-infra -p trackly-app --all-targets -- -D warnings</automated>
</verify>
<done>trackly-app compiles with `resolved_port()` at both DTO call sites; trackly-infra's full unit-test suite is green; clippy is clean on both touched crates; `trackly.config.toml.example` documents `ldap_tls_mode` with the plaintext caveat in Russian, matching the file's existing style.</done>

## must_haves

- truths:
  - Default behavior (no `ldap_tls_mode` in config) is byte-for-byte
    unchanged: `ldaps://host:636`, TLS from the start — existing
    `trackly.config.toml` files behave identically after this change.
  - Setting `ldap_tls_mode = "plain"` connects to `ldap://host:389` (or an
    explicitly configured port) with no TLS, and logs exactly one
    `tracing::warn!` per process at config load, not per bind attempt.
  - Setting `ldap_tls_mode = "starttls"` connects to `ldap://host:389` (or
    explicit port) and upgrades via `LdapConnSettings::set_starttls(true)`.
  - An explicitly configured `port` always wins over the mode-derived
    default, regardless of mode.
  - All three LDAP call sites (`real.rs::authenticate`,
    `real.rs::test_connection`, `directory.rs::resolve`) build their
    URL/settings through the single `ad::transport::build_ldap_conn` helper
    — no duplicated `format!("ldaps://...")` literals remain.
- artifacts:
  - `crates/trackly-infra/src/config.rs` — `LdapTlsMode` enum,
    `AdConfig::port: Option<u16>`, `AdConfig::resolved_port()`, one-time
    plaintext warning, Behavior 6-9 tests.
  - `crates/trackly-infra/src/ad/transport.rs` — `build_ldap_conn`, moved
    `CONN_TIMEOUT`, per-mode unit tests.
  - `crates/trackly-infra/src/ad/real.rs`,
    `crates/trackly-infra/src/ad/directory.rs` — wired through the shared
    helper, unused imports/consts removed.
  - `crates/trackly-app/src/http/auth.rs`,
    `crates/trackly-app/src/tauri_cmds/auth.rs` — `resolved_port()` at the
    DTO read sites.
  - `trackly.config.toml.example` — `ldap_tls_mode` documented.
- key_links:
  - `AdConfig::resolved_port()` is the ONLY place that resolves "unset" port
    → mode default; both DTO call sites and `build_ldap_conn` route through
    it (no second port-resolution codepath).
  - `AppConfig::load_or_default` → single `tracing::warn!` call site for the
    plaintext opt-in (mirrors why `main.rs:47` calls it exactly once).

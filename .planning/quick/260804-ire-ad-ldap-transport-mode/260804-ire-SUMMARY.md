---
quick_id: 260804-ire
slug: ad-ldap-transport-mode
title: AD LDAP transport mode (plaintext/StartTLS opt-in)
mode: quick
status: complete
completed: 2026-08-04
---

# Quick Task 260804-ire: AD LDAP transport mode (plaintext/StartTLS opt-in) — Summary

**One-liner:** Added a typed `[ad] ldap_tls_mode` config toggle (`ldaps` default | `plain` | `starttls`) with a single shared `build_ldap_conn` helper wiring all three LDAP call sites, so the live DC's plaintext-only LDAP (no working LDAPS listener) can be dialed via an explicit, warned opt-in without touching any of the three duplicated connection-builder call sites individually.

## What Was Built

- **`crates/trackly-infra/src/config.rs`**
  - New `LdapTlsMode` enum (`Ldaps` default | `Plain` | `StartTls`), `#[serde(rename_all = "lowercase")]` so TOML values are exactly `"ldaps"`/`"plain"`/`"starttls"`. `LdapTlsMode::default_port()` → 636 for `Ldaps`, 389 for `Plain`/`StartTls`.
  - `AdConfig::port` changed from `u16` to `#[serde(default)] Option<u16>` — `None` means "not explicitly pinned in TOML".
  - `AdConfig::resolved_port()` — the single place that resolves "unset" port → mode default. An explicit `port` always wins, regardless of mode.
  - `AdConfig::ldap_tls_mode: LdapTlsMode` field (`#[serde(default)]`), documented in the struct doc comment.
  - `AppConfig::load_or_default` now binds the parsed config and fires exactly one `tracing::warn!` when `ldap_tls_mode == Plain` (config is parsed once per process at `main.rs:47`, so no `Once`/static guard needed).
  - Manual `Debug` impl for `AdConfig` extended with `.field("ldap_tls_mode", ...)` (not a secret, prints as-is).
  - Behavior 6-9 tests + a pure-fn `default_port()` test added to the existing `#[cfg(test)] mod tests` block (continuing the "Behavior N" numbering already established in this file).

- **`crates/trackly-infra/src/ad/transport.rs`** (new module)
  - `pub(crate) const CONN_TIMEOUT` moved here from the two duplicated locals in `real.rs`/`directory.rs`.
  - `pub(crate) fn build_ldap_conn(cfg: &AdConfig) -> (String, ldap3::LdapConnSettings)` — resolves the port via `cfg.resolved_port()`, then per `cfg.ldap_tls_mode`:
    - `Ldaps` → `ldaps://host:port`, settings unchanged (byte-for-byte the pre-existing behavior).
    - `Plain` → `ldap://host:port`, settings unchanged.
    - `StartTls` → `ldap://host:port`, settings gains `.set_starttls(true)`.
  - Module-private unit tests assert on the returned URL string and `LdapConnSettings::starttls()` (the only public getter the crate exposes); `no_tls_verify` has no public getter so is not independently assertable — noted in a code comment rather than silently skipped.

- **`crates/trackly-infra/src/ad/mod.rs`** — added `pub mod transport;`.

- **`crates/trackly-infra/src/ad/real.rs`** (`authenticate`, `test_connection`) and **`crates/trackly-infra/src/ad/directory.rs`** (`resolve`) — both call sites now do `let (url, settings) = crate::ad::transport::build_ldap_conn(&self.cfg);` instead of building the URL/settings inline. Removed the now-duplicated local `CONN_TIMEOUT` consts and the now-unused `LdapConnSettings` import from both files (kept `ldap_escape`, `LdapConnAsync`, `Scope`, `SearchEntry`). `directory.rs`'s `unreachable_but_configured_cfg()` test fixture updated: `port: 1` → `port: Some(1)`.

- **`crates/trackly-app/src/http/auth.rs::build_settings_get_ad`** and **`crates/trackly-app/src/tauri_cmds/auth.rs::build_settings_get_ad_tauri`** — `port: ad_config.port as i64` → `port: ad_config.resolved_port() as i64`, so `AdSettingsDto.port` always reports the port that will actually be dialed rather than leaking a raw `Option`. `ldap_tls_mode` intentionally NOT added to `AdSettingsDto` — out of the task's required scope (backend transport toggle only, no Settings-UI surface).

- **`trackly.config.toml.example`** — documented `ldap_tls_mode` (Russian, matching the file's existing comment style) immediately after the existing `# port = 636` comment: the three mode values, the per-mode default-port behavior when `port` is left commented out, and the explicit plaintext security caveat (service-account bind password and user credentials travel in cleartext).

## Verification

- `cargo test -p trackly-infra --lib config::tests::` — 10/10 pass (Behavior 1-9 + pure-fn test).
- `cargo test -p trackly-infra --lib ad::transport::tests::` — 4/4 pass.
- `cargo test -p trackly-infra --lib ad::directory::tests::` — 8/8 pass (cache short-circuit + cache-miss-unreachable tests still hold with the `Option<u16>` port field).
- `cargo build -p trackly-app` — succeeds (compiles the two DTO call-site edits).
- `cargo test -p trackly-infra --lib` — full suite, 123/123 pass, zero regressions.
- `cargo test -p trackly-infra --test config_test` — 6/6 pass (pre-existing `AppConfig` integration suite, unaffected).
- `cargo clippy -p trackly-infra -p trackly-app --all-targets -- -D warnings` — clean, zero warnings.
- `cargo fmt -p trackly-infra -p trackly-app` — applied (whitespace-only diff in `config.rs`, e.g. wrapping the `resolved_port()` method chain; no logic changes).
- `grep -rn 'format!("ldaps://' crates/` — the only remaining match is inside `transport.rs` itself (both the doc comment and the one legitimate `Ldaps` branch); zero duplicated literals remain at the three former call sites.

## Backward Compatibility (non-negotiable constraint)

Proven by test, not just asserted:
- `config::tests::empty_config_defaults_to_ldaps_and_resolved_port_636` — whole `[ad]` section absent → `ldap_tls_mode == Ldaps`, `port == None`, `resolved_port() == 636`.
- `config::tests::partial_ad_section_without_transport_fields_resolves_ldaps_636` — an `[ad]` section present (with all the pre-existing required fields filled in) but omitting both `ldap_tls_mode` and `port` still parses and resolves to `Ldaps` + 636 — an existing `trackly.config.toml` upgrading to this binary needs zero edits.
- `ad::transport::tests::ldaps_mode_builds_ldaps_url_no_starttls` — `build_ldap_conn` on a default-mode config produces `"ldaps://<host>:636"` with `settings.starttls() == false`, i.e. dials exactly as before this change.

## Deviations from Plan

None — plan executed exactly as written. The only mid-execution correction was to two newly-added `config.rs` unit tests (`plain_and_starttls_modes_resolve_to_port_389`, `explicit_port_always_wins_over_mode_default`): my first draft used minimal partial `[ad]` TOML snippets that omitted the pre-existing REQUIRED fields (`enabled`/`use_mock`/`host`/`domain`/`base_dn`/`name_attr`/`no_tls_verify` — these predate this task and are not `#[serde(default)]`), which failed to parse with "missing field `enabled`". Fixed by including the full required-field set, mirroring the existing Behavior 2/4/5 test fixtures in the same file. This is normal test-authoring correction, not a deviation from the plan's design — no Rule 1-4 applies since nothing outside the current task's own new test code was touched.

**Process note (not a code deviation):** the very first `cargo test -p trackly-infra --lib config::tests::` invocation stalled at 0% CPU for several minutes in this sandboxed environment (a known throttling behavior of background compiles here, not specific to this change) and was killed and re-run cleanly in ~16s. No code or test content was affected by this restart.

## Known Stubs

None. No hardcoded empty values, placeholder text, or unwired UI data sources were introduced by this change — this is a backend-only transport-config change with no UI surface.

## Threat Flags

| Flag | File | Description |
|------|------|-------------|
| threat_flag: cleartext-credentials-opt-in | `crates/trackly-infra/src/config.rs`, `crates/trackly-infra/src/ad/transport.rs` | New `ldap_tls_mode = "plain"` config value causes the service-account bind password and end-user credentials to be sent unencrypted over the LAN. This is an explicit, documented, default-off opt-in (default remains `ldaps`), gated by a one-time `tracing::warn!` at config load and a security caveat in `trackly.config.toml.example`. No additional mitigation is in scope per the plan's stated threat-model decision — the DC's own lack of a working LDAPS listener is the threat-surface owner's decision, not this codebase's to silently work around.

## Self-Check: PASSED

- FOUND: `crates/trackly-infra/src/ad/transport.rs`
- FOUND: `resolved_port` in `crates/trackly-infra/src/config.rs`
- FOUND: `ldap_tls_mode` in `trackly.config.toml.example`
- FOUND commit `0d3c932` (Task 1 — config layer)
- FOUND commit `0f1c6b8` (Task 2 — shared transport helper)
- FOUND commit `f6738ce` (Task 3 — DTO surface + docs)

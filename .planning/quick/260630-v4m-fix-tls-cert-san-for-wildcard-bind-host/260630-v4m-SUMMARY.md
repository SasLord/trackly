---
quick_id: 260630-v4m
slug: fix-tls-cert-san-for-wildcard-bind-host
title: Fix TLS cert SAN for wildcard bind host
date: 2026-06-30
status: complete
commit: fb129be
followup_commit: 08eab42
---

# Quick Task 260630-v4m — Summary

## What changed

`crates/trackly-app/src/server/tls.rs` — `generate_self_signed(host)` now builds
its subject-alt-name list via a new `collect_subject_alt_names(host)` helper:

- **Non-wildcard host** → unchanged: `[host, "localhost"]`.
- **Wildcard/unspecified host** (`0.0.0.0`, `::`, empty) → `"localhost"` + every
  non-loopback IPv4/IPv6 address from `if_addrs::get_if_addrs()` (as IP-SANs) +
  the OS hostname (`hostname::get()`, validated as a DNS label). The literal
  wildcard (`0.0.0.0`/`::`) is **never** placed in the SAN.

rcgen 0.14's `generate_simple_self_signed` → `CertificateParams::new`
auto-classifies each SAN string: `IpAddr::from_str` success → `SanType::IpAddress`,
else `SanType::DnsName`. So pushing plain IP strings is sufficient — no manual
`SanType` construction needed.

Helpers added: `is_wildcard_host`, `is_valid_dns_name`, `collect_subject_alt_names`.
The hostname is filtered through `is_valid_dns_name` first so an unusual machine
name can't fail cert generation outright (rcgen rejects invalid DNS names).

## Why

After the bind-host fix (4ec2a9b) the server actually binds on `0.0.0.0`, but the
cert SAN still only held `[ "0.0.0.0", "localhost" ]`. LAN browsers connect via
`https://<LAN-IP>:port`, so they got a **hostname-mismatch** cert error on top of
the expected self-signed-untrusted warning, making the fingerprint-trust UX worse.
Adding the real LAN IPs (and hostname) as SANs removes the mismatch error. This is
a secondary/UX issue, not the connectivity blocker (that was the bind fix).

## Dependencies

- `if-addrs = "0.15"` — cross-platform interface enumeration.
- `hostname = "0.4"` — cross-platform OS hostname.

Both pure-Rust (libc-only transitive deps), no OpenSSL/DLL — portable-clean per
CLAUDE.md. Added to `[workspace.dependencies]` and `crates/trackly-app/Cargo.toml`.

## Call sites

Unchanged — `main.rs:162`, `http/settings.rs:272`, `tauri_cmds/auth.rs:93` all
pass `&host`; the wildcard detection happens inside `generate_self_signed`.

## Verification

- `cargo build -p trackly-app` — clean.
- tls unit tests (4, incl. new `is_wildcard_host_classifies_correctly`,
  `collect_sans_non_wildcard_unchanged`, `collect_sans_wildcard_includes_detected_lan_ip`)
  — green. The wildcard test asserts ≥1 detected LAN IP appears in the SAN list,
  and documents/skips the assertion (via `eprintln!`) if the host has no
  non-loopback interfaces.
- `cargo test -p trackly-app --test tls_server_smoke` (5 tests incl.
  `generate_self_signed_does_not_panic`) — green, no regression.
- `cargo clippy -p trackly-app --lib` — clean.

Note: required seeding/building `ui/dist` (`pnpm --dir ui build`) before the lib
would compile (rust-embed `SpaAssets` folder-existence gotcha) — known per project
memory. Tests run with `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1`.

## Follow-up (commit 08eab42) — live-UAT findings

User tested from a neighbour PC (`https://192.168.1.2:8443`) on a rebuilt
(`cargo tauri dev`) binary and reported "still 0.0.0.0" + untrusted-cert
warning. Diagnosis via clarifying questions:

- **"0.0.0.0"** was the **Settings UI** field "Адрес сервера:
  `https://0.0.0.0:8443`" — i.e. the displayed *bind* address, not the cert
  SAN. Cosmetic but useless to hand to a colleague.
- **Untrusted-cert warning** (`received fatal alert: CertificateUnknown` in
  logs) is **expected** for a self-signed cert (untrusted CA). The SAN fix only
  removes the *name-mismatch* error; the untrusted-CA warning stays by design
  (user clicks through / trusts the fingerprint).

Changes:
- Added `tls::display_host(host)` — for wildcard hosts substitutes the best
  detected LAN IP (private IPv4 preferred via new `detect_lan_ips()` ranking:
  private IPv4 → other IPv4 → IPv6), else `"localhost"`. Wired into the
  displayed `server_url` / `ServerStatusDto.url` in `http/settings.rs` and
  `tauri_cmds/auth.rs`. The actual bind still uses the wildcard host.
- `collect_subject_alt_names` now adds loopback (`127.0.0.1`, `::1`) to the
  wildcard SAN so a local test via `https://127.0.0.1:port` doesn't name-mismatch.
- Refactored IP enumeration into shared `detect_lan_ips()`.
- +2 unit tests (`display_host_substitutes_wildcard`,
  `collect_sans_wildcard_includes_loopback`); tls unit tests now 6, all green;
  `tls_server_smoke` (5) green; clippy + fmt clean.

## Commit

- `fb129be` — fix(server): add LAN IP/hostname SANs to self-signed cert for wildcard bind host
- `08eab42` — fix(server): show real LAN IP (not 0.0.0.0) as server URL + add loopback to wildcard SAN

---
spike: 001
name: sspi-windows-compile
type: standard
validates: "Given Trackly's build, when the sspi crate (server-side Kerberos/Negotiate accept) is added behind the AD adapter layer, then it compiles+links for the Windows MSVC target in CI and cargo check on macOS"
verdict: VALIDATED
related: [002, 003]
tags: [rust, kerberos, sspi, ci, portable]
---

# Spike 001: sspi-windows-compile

## What This Validates

**Given** Trackly's Cargo workspace and portable Windows build,
**when** the `sspi` crate (Devolutions — the only credible pure-Rust path to *server-side*
SPNEGO/Negotiate acceptance) is added to `trackly-infra` and referenced behind the AD
adapter layer,
**then** it compiles and links for the **Windows MSVC** target in CI and passes
`cargo check` / `cargo test` on the macOS dev box — with **no mandatory native C crypto
toolchain** (`default-features = false`, dropping `aws-lc-rs`).

This is the killer risk. If `sspi` (or its crypto backend) will not build for portable
Windows, the whole passwordless-AD-SSO approach must be rethought *before* any endpoint
code is written. Everything in spikes 002/003 rides on this.

## Research

Reference project: `adwebapp` (Go) uses `gokrb5` for mature server-side SPNEGO. Rust has no
`gokrb5` equivalent; the realistic options:

| Approach | Library | Pros | Cons | Status |
|----------|---------|------|------|--------|
| Pure-Rust SSPI | **`sspi` 0.21.3** (Devolutions) | Cross-platform (compiles on macOS dev + Windows), pure-Rust Kerberos crypto, native Windows SSPI available too, **server-side accept implemented** (`src/kerberos/server`, `examples/server.rs`), offline keytab decryption | Server-side SPNEGO less battle-tested than gokrb5; default crypto = `aws-lc-rs` (C) | **CHOSEN** |
| Native Windows SSPI | `windows` crate `AcceptSecurityContext` | No keytab (uses service account/LSA) | Windows-only (can't `cargo check` on mac), changes portable run model | Fallback |
| MIT/Heimdal GSSAPI | `libgssapi` / `cross-krb5` | Standard GSSAPI | Needs native krb5 libs + keytab; painful on portable Windows | Rejected |

Confirmed by reading the crate source (`Devolutions/sspi-rs@master`):
- **Server-side accept exists**: `Kerberos::new_server_from_config(KerberosConfig, ServerProperties)`; accept loop via `accept_security_context()` builder (`examples/server.rs`).
- **Offline validation**: `ServerProperties.ticket_decryption_key: Option<Secret<Vec<u8>>>` and `.additional_service_keys` hold the keytab-derived service key — the AP-REQ ticket is decrypted locally; `KerberosConfig.kdc_url` is only used by the *client*. No KDC round-trip per request. ✔ matches the LAN web-SSO model.
- **Crypto-backend caveat**: `default = ["aws-lc-rs"]`. `aws-lc-rs`/`ring` are only the **rustls TLS crypto provider**, gated behind the `network_client`/`tsssp` (rustls) features — unused for offline accept. Hypothesis: `default-features = false` compiles fine (Kerberos crypto is RustCrypto, not the TLS provider). **This spike tests that hypothesis on Windows MSVC.** (Note: the workspace already uses `ring` via `ldap3` `tls-rustls-ring`, so a `ring` provider is available if some path turns out to require one.)

## How to Run

macOS (fast local loop):
```bash
cargo check -p trackly-infra
cargo test -p trackly-infra ad::sso
```

Windows (the actual proof) — via CI on push, or a release dry-run:
```bash
gh workflow run release.yml --ref spike/ad-sso-kerberos --field version=0.0.0-sso-spike
```

## What to Expect

- macOS: `cargo check` / `cargo test` green; the `ad::sso` probe tests pass.
- Windows CI: the workspace (with `sspi`) builds to completion; no C-toolchain/link errors.
  A downloadable Windows artifact appears on the draft release.

## Investigation Trail

- Chose `sspi` over native-only / GSSAPI after confirming (a) server-side accept exists and
  (b) offline keytab decryption is supported — both non-obvious and both verified in source.
- Added `sspi = { version = "0.21", default-features = false }` to `trackly-infra` and a
  minimal `ad::sso` probe module (`sspi_link_probe`) that references the crate's public
  `KerberosConfig` surface (side-effect-free: only parses the KDC URL). The probe forces the
  whole `sspi` crate — server-accept code + crypto included — to compile/link for the target.
- macOS resolution: `sspi 0.21.3` locked; `default-features = false` accepted without a
  resolver error. Build in progress at time of writing.

## Results

**VALIDATED — builds clean on both macOS and Windows MSVC.**

- **Windows CI: PASS.** `release.yml` dry-run (run `30541316563`, ref `spike/ad-sso-kerberos`,
  `version=0.0.1`) — `build (windows-latest, --bundles nsis)` **success**, plus macOS + Linux +
  checksums all green. This is the authoritative proof: `sspi 0.21.3` server-side
  Kerberos/Negotiate accept + pure-Rust crypto compiles and **links** for the portable Windows
  target in Trackly's real release build — no C-toolchain/link failure.
- **Downloadable Windows artifacts** produced in the draft release (`v0.0.1`):
  `trackly-v0.0.1-windows-x64-portable.zip` + `Trackly_0.0.1_x64-setup.exe`.
- macOS `cargo check`: PASS (see below), pure-Rust crypto tree confirmed.

**Caveat — what this does NOT yet prove:** this build only contains the compile/link *probe*,
not a working SSO endpoint. The live Kerberos handshake against a real DC is exercised only
once spikes 002 (Negotiate endpoint + h2-off) and 003 (frontend auto-login + keytab/SPN config)
land — that is the real-AD test.

### macOS detail

- **macOS `cargo check -p trackly-infra`: PASS (exit 0), clean** — no warnings/errors from
  `ad::sso` or the `sspi` dependency. Confirms the `sspi` public API our probe uses
  (`sspi::KerberosConfig::new` + `kdc_url`, re-exported at `sspi/src/lib.rs:120`) type-checks
  and the whole crate compiles.
- **Portable-crypto hypothesis holds on macOS:** with `default-features = false`, the resolved
  `sspi 0.21.3` build pulls a **pure-Rust crypto tree** — `picky-krb`, `curve25519-dalek`,
  `ed25519-dalek`, `rsa`, `sha1`/`sha2`, `p256`/`p384`/`p521` — and **no `aws-lc-rs`, no
  OpenSSL, no C toolchain**. This is the portable win the spike was testing for.
- **Surprise / note:** even `default-features = false` still pulls `async-dnssd`, `tokio`,
  `futures` (pure-Rust) — a heavier dep tree than expected, but all portable-safe. Build was
  slow locally only due to 3 concurrent `cargo check` runs (rust-analyzer contention), not the
  crate itself.

**Remaining (the actual proof):** Windows MSVC build in CI must go green with `sspi` present.
That is the platform where a C-crypto/link problem would surface — validated via the release
dry-run on this branch. Live Kerberos handshake against a real DC is spike 002/real-AD.

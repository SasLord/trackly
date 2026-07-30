# Spike Manifest

## Idea

Portable passwordless **AD Single Sign-On (Kerberos / SPNEGO / Negotiate)** for Trackly's
server mode, ported in spirit from the working Go reference project `adwebapp`
(`/Users/madsas/Projects/llm-projects/adwebapp`). On a domain-joined Windows machine the
browser should log the user in automatically — no login/password prompt — by presenting a
Kerberos ticket that Trackly's axum server validates. The existing LDAPS `simple_bind`
login/password path stays as the fallback.

Full parity with adwebapp (SSO + service-account bind + AD group/role mapping + silent
frontend auto-login + HTTP/2-off) is a **later milestone**. These spikes only de-risk the
single hardest unknown — **is server-side SPNEGO acceptance feasible in Rust and does it
build for the portable Windows target** — and produce a downloadable Windows build so the
handshake can be tested against a real AD.

## Requirements

Design decisions locked so far (non-negotiable for the real build):

- Server-side Kerberos acceptance must validate the ticket **offline from a keytab/service
  key** — no KDC network round-trip on each request (LAN web-SSO model). `sspi`'s
  `ServerProperties.ticket_decryption_key` / `additional_service_keys` supports this.
- **HTTP/2 must be disabled** (ALPN `http/1.1` only) on the axum TLS listener — SPNEGO is a
  two-step handshake on one connection; h2 multiplexing breaks it (adwebapp: `NextProtos`).
- Existing LDAPS `simple_bind` login/password auth must keep working unchanged.
- Portable constraint: prefer a build with **no mandatory native C crypto** — investigate
  `sspi` `default-features = false` (drop `aws-lc-rs`, which is only needed for the rustls
  TLS provider behind `network_client`, unused for offline accept).
- **Privacy:** zero real org data in git — domain, DC IPs, admin logins, org name all stay
  placeholders; real values live only in gitignored runtime config.

## Spikes

| # | Name | Type | Validates | Verdict | Tags |
|---|------|------|-----------|---------|------|
| 001 | sspi-windows-compile | standard | Given Trackly's build, when `sspi` (server-side Kerberos/Negotiate accept) is added behind the AD adapter layer, then it compiles+links for Windows MSVC in CI and `cargo check` on macOS | PARTIAL (macOS ✓, Windows CI pending) | rust, kerberos, sspi, ci, portable |
| 002 | negotiate-endpoint-h2off | standard | Given the axum server, when a `/auth/ad`-style endpoint returns `401 + WWW-Authenticate: Negotiate` and h2 is disabled, then the handshake shape is correct and existing simple_bind auth still works | PENDING | axum, spnego, http2, tls |
| 003 | frontend-autologin-config | standard | Given an anonymous page, when the silent `fetch` + `ad_skip` cookie fallback + SPN/service-account config (placeholders) are wired, then the SSO flow is reachable end-to-end for live-AD testing | PENDING | frontend, config, sso |

Verdicts: 002/003's *live* handshake verdict is deferred to real-AD testing; today they are
built + compile-verified only.

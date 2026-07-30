---
spike: 002
name: negotiate-endpoint-h2off
type: standard
validates: "Given the axum server, when a /auth/ad-style endpoint returns 401+WWW-Authenticate: Negotiate and h2 is disabled, then the SPNEGO handshake shape is correct and existing simple_bind auth still works"
verdict: BUILD-VERIFIED
related: [001, 003]
tags: [axum, spnego, http2, tls, kerberos, keytab]
---

# Spike 002: negotiate-endpoint-h2off

## What This Validates

Given Trackly's axum server, when a `GET /api/v1/auth_ad_sso` endpoint runs the browser
Negotiate handshake (401 challenge → validate ticket via keytab → issue session) and HTTP/2
is disabled, then the SPNEGO flow is wired end-to-end and the existing login/password +
LDAPS-bind paths are untouched.

## What was built

- **`trackly_infra::ad::keytab`** — MIT keytab (v0x0502) reader; extracts the AES256
  (enctype 18) service key for the configured SPN. Deterministic, **fully unit-tested**
  (parse, SPN select, hole-skip, truncation, version guard).
- **`trackly_infra::ad::sso::accept_spnego`** — server-side SPNEGO/Negotiate acceptor via
  `sspi` (`ServerProperties` + `Negotiate::new_server` + `accept_security_context`), offline
  keytab decrypt (no KDC), returns the authenticated AD account via `query_context_names`.
- **`trackly_app::http::sso`** — `GET /api/v1/auth_ad_sso`: 401 + `WWW-Authenticate: Negotiate`
  challenge → on a valid ticket, `AuthService::sso_login` (reuses `on_ad_bind_success`
  provisioning) → same session cookie as password login. Additive, gated on `ad.sso_enabled`.
- **`AuthService::sso_login`** — passwordless AD login mapping the Kerberos-authenticated
  account to a Trackly user with identical semantics to plain AD login.
- **h2-off** — `server::tls` pins ALPN to `http/1.1` (SPNEGO is a two-step single-connection
  handshake that HTTP/2 multiplexing breaks — adwebapp `NextProtos` parity).
- **config** — `AdConfig { sso_enabled, spn, keytab_path }` (`#[serde(default)]`, off by
  default; real values only in gitignored runtime config).

## How to Run (tomorrow, on Windows/AD)

1. On the DC: `ktpass … /princ HTTP/<fqdn>@REALM /crypto AES256-SHA1 /out server.keytab`
   (mirrors adwebapp `setup-kerberos.ps1`); copy `server.keytab` next to `trackly.exe`.
2. In the runtime config: `ad.sso_enabled = true`, `ad.spn = "HTTP/<fqdn>"`,
   `ad.keytab_path = "server.keytab"`, and AD enabled.
3. From a domain-joined browser (FQDN in the address bar), hit `https://<fqdn>/api/v1/auth_ad_sso`
   or just open the app — a Kerberos ticket should log you in with no prompt.

## Results

**BUILD-VERIFIED — live-AD handshake pending.**

- `cargo check -p trackly-app` green (exit 0); keytab parser unit tests + acceptor guard
  tests green (28 `ad::` tests pass, no regression); `server::tls` tests green (ALPN pin did
  not break cert generation); Windows CI build to be confirmed by the release dry-run.
- **Not yet proven (tomorrow's real-AD test):** the actual Kerberos exchange. Two runtime
  unknowns documented in `ad::sso`: (1) `acquire_credentials_handle` shape for a Negotiate
  *server*; (2) `accept_security_context` must stay offline — `OfflineNetworkClient` errors
  loudly if it reaches for the KDC. Also unproven live: browser ↔ endpoint 401/Negotiate
  round trip and the h2-off effect on a strict corporate browser.

---
spike: 003
name: frontend-autologin-config
type: standard
validates: "Given an anonymous page, when the silent fetch + ad_skip cookie fallback + SPN/keytab config are wired, then the SSO flow is reachable end-to-end for live-AD testing"
verdict: BUILD-VERIFIED
related: [001, 002]
tags: [frontend, config, sso, svelte]
---

# Spike 003: frontend-autologin-config

## What This Validates

Given an anonymous Trackly page in a server-mode/LAN browser, when a silent SSO attempt +
`ad_skip` fallback are wired at app start, then a domain user is logged in transparently and
the flow is reachable end-to-end for the live-AD test — without breaking normal login.

## What was built

- **`ui/src/lib/api/adSso.ts` — `trySilentAdSso()`** — pokes `GET /api/v1/auth_ad_sso` so the
  browser performs the Negotiate handshake. Gated: browser-only (skips Tauri desktop), one
  attempt per browser session via a session-scoped `ad_skip` cookie, never throws. Mirrors
  adwebapp `auth-autoad.js`.
- **`ui/src/App.svelte`** — `onMount` refactored into `loadAuthStatus()`; when unauthenticated
  in a server-mode browser (and not on first-run), tries silent SSO once, then re-loads status
  on success to populate the real user. On 503 (SSO disabled — the default) → `ad_skip` → normal
  `LoginPage`, no reload loop.
- Config side (SPN / keytab_path / sso_enabled) landed with spike 002.

## Results

**BUILD-VERIFIED — live behaviour pending.**

- `svelte-check`: 0 errors (48 pre-existing warnings elsewhere, none in the new code).
- **Not yet proven (tomorrow):** the actual silent login in a domain browser — that the
  transparent Negotiate fetch succeeds and populates the session. The failure path (no ticket
  / SSO off → `ad_skip` → LoginPage) is logically safe but unexercised live.

## Notes / follow-ups for the full-parity milestone

- Explicit "Войти через Active Directory" button (adwebapp has one) — not added; silent
  attempt + normal login cover the test. 
- Service-account displayName lookup so SSO users get their real ФИО (currently falls back to
  the SAM login) — deferred (see `AuthService::sso_login` note).

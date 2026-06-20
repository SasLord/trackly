---
phase: 09
slug: ad
status: verified
threats_open: 0
asvs_level: 1
created: 2026-06-20
---

# Phase 09 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.
> AD-аутентификация и заявки на регистрацию пользователей.

Audit verifies that each declared mitigation in the plan-time threat register exists
in the implemented code. Evidence is a concrete code location, not documentation.
Implementation files were not modified during the audit.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| login string → LDAP filter | untrusted user input interpolated into an LDAP search filter | login (untrusted) |
| AD password → bind | secret crosses the process; must never persist/log | AD password (secret) |
| dev macOS → no DC | RealAdClient must fail closed (Unreachable); mock substitutes | — |
| browser → AuthService::login | untrusted credentials; single shared decision point | credentials (untrusted) |
| post-bind → users table | distinguish blocked/deleted from unknown (no accidental re-admit) | account state |
| self-registration → users table | a bound-but-unknown AD user must not self-elevate | role assignment |
| browser → settings/approve endpoints | privileged admin operations exposed over HTTP | admin actions |
| session cookie issuance | fixation + persistence policy | session identity |

---

## Threat Register

| Threat ID | Category | Component | Disposition | Mitigation (evidence) | Status |
|-----------|----------|-----------|-------------|------------------------|--------|
| T-09-01 | Spoofing/Elevation | empty-password bind | mitigate | Empty/whitespace password → `BadCreds` before bind: `ad/real.rs:58-60`, `ad/mock.rs:90-92`; tests `empty_password_rejected`/`whitespace_password_rejected` (mock.rs:191-217) | closed |
| T-09-02 | Tampering | LDAP filter build | mitigate | `ldap_escape(login)` (RFC 4515) before interpolation: `ad/real.rs:91-92` | closed |
| T-09-03 | Information Disclosure | AD password in `Secret<String>` | mitigate | `Secret<T>` no Debug-leak/Serialize/Deserialize/Clone, zeroize-on-drop: `core/primitives/secret.rs:24-55`; `.expose()` only at bind site (`real.rs:77`, `mock.rs:100`); never persisted | closed |
| T-09-04 | Information Disclosure | not-found vs wrong-password | mitigate | Both return `AuthOutcome::BadCreds`: `real.rs:82-87`, `mock.rs:103-106`; test asserts indistinguishable (mock.rs:170-179) | closed |
| T-09-05 | Information Disclosure | LDAPS cert verification | mitigate | `ldaps://` `real.rs:65`; `tls-rustls-ring` (`infra/Cargo.toml:32`); `no_tls_verify` default `false` (`config.rs:167`), opt-in only | closed |
| T-09-SC | Tampering (supply-chain) | ldap3/hickory installs | mitigate | ldap3 0.12.1 + hickory-resolver 0.26.1 pinned, `default-features=false`, rustls-ring, no native-tls (`Cargo.toml:32-33`); Task 0 human legitimacy checkpoint approved (09-01-SUMMARY.md:81,101) | closed |
| T-09-06 | Spoofing/Elevation | login() empty-password AD fallback | mitigate | Service-layer empty/whitespace reject before bind: `services/auth.rs:312-314`, `:707-709` | closed |
| T-09-07 | Information Disclosure | enumeration via timing/error | mitigate | Constant-time dummy-hash (auth.rs:66-81, 279-291); generic `Unauthorized` for `BadCreds` (auth.rs:320) + unknown-local (auth.rs:258,262) | closed |
| T-09-08 | Information Disclosure | AD password in login() | mitigate | `Secret::new` wrap (auth.rs:316,714); `.expose()` confined (T-09-03); AD users `password_hash = NULL` (auth.rs:482,553) | closed |
| T-09-09 | Elevation | blocked/deleted re-admit after bind | mitigate | `find_user_any_state` (auth.rs:935-973); active-only `get_by_login` happy path (auth.rs:346); blocked → `report_blocked_access` (auth.rs:363,612-618) | closed |
| T-09-10 | Elevation | auto-register role | mitigate | Auto-register hard-codes `role='employee'` (auth.rs:482); pending path also `'employee'` (auth.rs:553) | closed |
| T-09-11 | Information Disclosure | ad_register visible to non-admin | mitigate | `exclude_ad_register = !Admin` (request_service.rs:90) → SQL predicate on COUNT+SELECT (`requests_sqlite.rs:233,253`) | closed |
| T-09-12 | Elevation | approve sets arbitrary role | mitigate | `approve_ad_register` gated `authorize(ManageUsers)` (request_service.rs:381); `Role::from_str(...unwrap_or("employee"))` (request_service.rs:384) | closed |
| T-09-13 | Tampering | writes outside single-writer | mitigate | All user/request mutations via `WriterHandle::execute` (auth.rs:474,544,759,849,914; request_service.rs:405) | closed |
| T-09-14 | Repudiation | unattributed AD admin actions | mitigate | `audit_log` INSERT on auto-register (auth.rs:489), pending (560,578), restore (791), approve (request_service.rs:427,466), reject (506+) | closed |
| T-09-15 | Elevation | settings_set_ad / approve over HTTP | mitigate | Server `authorize(ManageSettings)` both transports: HTTP (http/auth.rs:275) + service (auth.rs:846,911) + Tauri (tauri_cmds/auth.rs:393) | closed |
| T-09-16 | Spoofing | session fixation on login | mitigate | `session.flush()` BEFORE `session.insert("identity")`: `http/auth.rs:139-150` | closed |
| T-09-17 | Information Disclosure | AdSettingsDto leaking secret | mitigate | No password field in `AdSettingsDto` (`dto/auth.rs:165-185`) | closed |
| T-09-18 | Information Disclosure | ad_register over the wire to non-admin | mitigate | Same SQL admin gate as T-09-11 on HTTP list path (request_service.rs:90-95); http test `ad_register_list_admin_only_http` passes | closed |
| T-09-19 | DoS/Tampering | unauthenticated restoration endpoint | accept | Governor rate-limited + admin-reviewed + idempotent insert — see Accepted Risks Log | closed |
| T-09-20 | Information Disclosure | login error copy | mitigate | Single generic `GENERIC_AUTH_ERROR = 'Неверный логин или пароль'`; distinct copy only for AD-unreachable (`LoginPage.svelte:12-14`) | closed |
| T-09-21 | Information Disclosure | ad_register visible to non-admin in UI | mitigate | UI `isAdmin` gate (`RequestDetail.svelte:56`) + server SQL filter (T-09-11) | closed |
| T-09-22 | Elevation | role select on approve | mitigate | UI default `approveRole='employee'` (RequestDetail.svelte:41); server re-validates (request_service.rs:381,384) | closed |
| T-09-23 | Spoofing | reserved SSO button | mitigate | `disabled`, `type="button"`, no click handler, no fabricated display name (`LoginPage.svelte:147-151`) | closed |

*Status: open · closed*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*
*Paths are relative to `crates/trackly-{core,infra,app}/src/` and `ui/src/` as named.*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-09-01 | T-09-19 | Restoration endpoint is unauthenticated by necessity (blocked user has no session) but governor rate-limited (burst 5, 1/s — `http/mod.rs:81-93`), idempotent (`ensure_open_restore_request`, auth.rs:749-819), produces no privileged effect on its own (only inserts an `open` `ad_register`/`restore` request requiring `ManageUsers`-gated admin approval), and preserves anti-enumeration (generic `Unauthorized`, auth.rs:718,729,733,736). Low-value LAN endpoint; residual risk accepted. | Alexander Platov | 2026-06-20 |

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-06-20 | 24 | 24 | 0 | gsd-security-auditor |

*22 mitigated (verified in code) + 1 accepted (T-09-19) + 1 supply-chain (T-09-SC). No unregistered attack surface: every SUMMARY `## Threat Flags` entry maps to an existing verified threat ID. Both transports (axum HTTP + Tauri) verified for every authorization-bearing threat (T-09-12, T-09-15). `.expose()` enumerated across `crates/` — all call sites legitimate crypto/bind points, no exposure to logging/serialization/DB.*

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-06-20

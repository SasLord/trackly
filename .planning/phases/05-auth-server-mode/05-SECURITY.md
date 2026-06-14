---
phase: 5
slug: auth-server-mode
status: complete
threats_open: 0
asvs_level: 1
block_on: high
created: 2026-06-14
---

# Phase 5 — Security: auth-server-mode

**Audit Date:** 2026-06-14
**ASVS Level:** 1 · **block_on:** high
**Status:** SECURED — 30/30 threats closed, 0 open
**Register origin:** authored at plan time (all six 05-0N-PLAN.md carry `<threat_model>` blocks)

> Per-phase security contract: threat register verification, accepted risks, and audit trail.

---

## Threat Verification Table

| Threat ID | Category | Disposition | Status | Evidence |
|-----------|----------|-------------|--------|----------|
| T-05-01 | Tampering | mitigate | CLOSED | `trackly-core/src/auth.rs` 192–241: role×action unit tests; `tests/role_endpoint_matrix.rs`: 9-assertion CI test |
| T-05-02 | Info Disclosure | mitigate | CLOSED | `dto/auth.rs` 33–48: `UserDto` has no `password_hash` field; test asserts field absent in serialized JSON |
| T-05-03 | Info Disclosure | mitigate | CLOSED | `services/auth.rs` 195,224,692,704,741: all argon2 hash/verify in `spawn_blocking` |
| T-05-04 | Tampering | mitigate | CLOSED | `server/rusqlite_session_store.rs` 82: `INSERT OR IGNORE`; tower-sessions 128-bit `OsRng` IDs |
| T-05-05 / T-05-SF | EoP | mitigate | CLOSED | `http/auth.rs` 107: `session.flush()` BEFORE `session.insert()` (117); logout flush at 129 |
| T-05-06 | Info Disclosure | mitigate | CLOSED | `main.rs` 143–145: cert/key written to `exe_dir`; file-permission hardening deferred to Phase 7 (documented accepted risk) |
| T-05-07 | EoP | mitigate | CLOSED | `services/auth.rs` 776: `LIMIT 2` admin query; 790–795 exactly-1-admin check, else `trusted_admin()` fallback |
| T-05-08 | Info Disclosure | mitigate | CLOSED | `server/rusqlite_session_store.rs` 53–64: `background_cleanup()` deletes expired rows (called `main.rs` 150); `load()` filters `expiry_date > ?` |
| T-05-09a | EoP | mitigate | CLOSED | `services/auth.rs` 808–820: `get_desktop_lock_enabled` reads `app_settings` DB table, not config file |
| T-05-09b | Tampering | mitigate | CLOSED | `http/mod.rs` 47: `SessionManagerLayer ... with_same_site(SameSite::Strict)` |
| T-05-10 | DoS | mitigate | CLOSED | `http/mod.rs` 52–67: `per_second(1).burst_size(5)` GovernorLayer via `.route_layer()` on `/api/v1/auth_login` only; `server/mod.rs` 98–101: `ConnectInfo(peer_addr)` injected per-connection so `PeerIpKeyExtractor` works at runtime |
| T-05-11 | Info Disclosure | mitigate | CLOSED | `http/auth.rs` 80–87: `session_identity()` reads only `session.get("identity")`; no body/query path |
| T-05-12 | Spoofing | mitigate | CLOSED | `http/auth.rs` 104 → `services/auth.rs` 195: argon2 verify mandatory; no bypass |
| T-05-13 | EoP | mitigate | CLOSED | `http/mod.rs` 93: `SessionManagerLayer` on all routes; handlers call `session_identity()` → 401 if absent |
| T-05-14 | Info Disclosure | mitigate | CLOSED | `http/mod.rs` 97–113: `x-frame-options: DENY`, `x-content-type-options: nosniff`, CSP `script-src 'self'` (no `unsafe-inline` on scripts, WR-07) |
| T-05-15 | Tampering | mitigate | CLOSED | `main.rs` 143–145: cert/key write path uses `exe_dir.join(...)` only; no user-controlled write path |
| T-05-16 | EoP | mitigate | CLOSED | `tauri_cmds/devices.rs` (MutateDevices), `acts.rs` (MutateActs), `cartridges.rs` (MutateCartridges) — authorize() on every mutation; `tests/role_endpoint_matrix.rs` CI gate |
| T-05-17 / T-05-DL | EoP | mitigate | CLOSED | `tauri_cmds/users.rs` 33–40: `resolve_tauri_identity()` checks DB flag; `tauri_cmds/auth.rs` 162: `desktop_set_lock` uses it, not hardcoded `trusted_admin` |
| T-05-18 | EoP | accept | CLOSED | By design (USR-02): `auth.rs` 135 `ReadData` → `Ok(())` for all roles |
| T-05-19 | Tampering | mitigate | CLOSED | Role stored server-side in SQLite session; cookie holds opaque ID only |
| T-05-20 | Info Disclosure | mitigate | CLOSED | `ui/.../LoginPage.svelte` 78: `type="password"`; no localStorage write of password |
| T-05-21 | EoP | accept | CLOSED | Sidebar filtering is UX-only; server `authorize()` is enforcement |
| T-05-22 | Spoofing | accept | CLOSED | authStore manipulation affects UI only; mutations enforce RBAC server-side |
| T-05-23 | DoS | mitigate | CLOSED | `http/users.rs` 91: `build_users_create` calls `session_identity()` first → 401 with no session; `http/auth.rs` 141–163 `auth_status` returns `needs_bootstrap` |
| T-05-24 | Info Disclosure | accept | CLOSED | Intentional per D-Server-04; fingerprint in `server_toggle` response (`http/settings.rs` 199) |
| T-05-25 | EoP | accept | CLOSED | `desktop_set_lock` HTTP path calls `authorize(ManageSettings)` server-side; lock read from DB at boot |
| T-05-SN-01 | Tampering | mitigate | CLOSED | `http/settings.rs` 111: `session_identity()` → 401; 112: `authorize(ManageSettings)` → 403 |
| T-05-SN-02 | EoP | accept | CLOSED | Unlocked desktop = trusted_admin by D-Desktop-01 design |
| T-05-SN-03 | Tampering | mitigate | CLOSED | `http/settings.rs` 115–119 and `tauri_cmds/auth.rs` 246–250: port range 1..=65535 validated on both transports |
| T-05-SC | Tampering | accept | CLOSED | All crates [APPROVED] in 05-RESEARCH.md; no new deps in Phase 5 |

---

## Open Threats

None. All 30 registered threats are CLOSED (mitigation verified in code or accepted risk documented).

---

## Unregistered Flags (from SUMMARY.md `## Threat Flags`)

All executor-declared threat flags map to registered threat IDs (T-05-SF→05, T-05-DL→17, T-05-10/14/20/23/24, T-05-SN-01/02/03). No unregistered flags.

---

## Advisory Note (non-blocking)

**`main.rs` config-driven auto-start (≈line 136) derives the TLS key path with a brittle `.replace(".crt", ".key").replace(".pem", ".key")` heuristic** instead of `tls::resolve_key_path()` / `tls::load_from_files()` (the WR-01 fix used by the `server_toggle` HTTP + Tauri paths).

- **Impact:** if `cert_path` has an unusual extension (`.cer`, `.cert`), the auto-start path silently reads an unchanged (wrong) key path.
- **Why not a blocker:** `cert_path` comes from the admin-controlled config file, not LAN-user input. T-05-15 covers only the write destination. Not an ASVS L1 vulnerability.
- **Recommendation:** consolidate `main.rs` to `tls::load_from_files(&cert_path, "")`. Track as a standalone fix or in a later phase.

---

## Accepted Risks Log

| Risk Ref | Rationale |
|----------|-----------|
| T-05-06 (key file perms) | Key written with default OS permissions; chmod 600 / Windows ACL deferred to Phase 7. Acceptable at ASVS L1 on single-workstation LAN. |
| T-05-18 | `ReadData` allowed for all authenticated roles — by design (USR-02); employees must read device/cartridge data to file requests. |
| T-05-21 | Sidebar role filtering is UX-only; server `authorize()` is the enforcement point. |
| T-05-22 | authStore is browser-mutable; server `authorize()` enforces RBAC regardless. |
| T-05-24 | TLS fingerprint intentionally displayed (D-Server-04) for self-signed cert verification by LAN users. |
| T-05-25 | `desktop_set_lock` protected server-side via `authorize(ManageSettings)`; UI gating is convenience only. |
| T-05-SN-02 | Unlocked desktop = `trusted_admin` (D-Desktop-01). Accepts physical-access-to-workstation risk. |
| T-05-SC | Supply chain accepted: all crates audited in 05-RESEARCH.md; no new dependencies in Phase 5. |

---

## Audit Trail

### Security Audit 2026-06-14

| Metric | Count |
|--------|-------|
| Threats registered | 30 |
| Closed | 30 |
| Open | 0 |

_Audited by: gsd-security-auditor (Claude). Register authored at plan time; auditor verified each mitigation in implementation._

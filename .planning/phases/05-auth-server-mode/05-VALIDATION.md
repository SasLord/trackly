---
phase: 5
slug: auth-server-mode
status: draft
nyquist_compliant: pending
wave_0_complete: true
created: 2026-06-13
---

# Phase 5 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (Rust) + `cargo nextest` (optional) for backend; `vitest` + `svelte-check` for frontend |
| **Config file** | Cargo workspace tests; existing `tests/` in crates (e.g. `secret_zeroize.rs`) |
| **Quick run command** | `cargo test -p trackly-app -p trackly-core` |
| **Full suite command** | `cargo test --workspace && cargo clippy --workspace -- -D warnings && (cd ui && npm run check)` |
| **Estimated runtime** | ~60–120 seconds (workspace test + clippy) |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p <touched-crate>`
- **After every plan wave:** Run `cargo test --workspace`
- **Before `/gsd-verify-work`:** Full suite must be green (incl. role × endpoint 403 matrix test)
- **Max feedback latency:** ~120 seconds

---

## Per-Task Verification Map

> Filled concretely by the planner per task. The success-criteria-aligned validation
> anchors below are mandatory and must each map to at least one automated test.

| Anchor | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | Status |
|--------|-------------|------------|-----------------|-----------|-------------------|--------|
| argon2id hash/verify | USR-01 | T-05 (credential storage) | Password stored only as argon2id hash; verify rejects wrong password | unit | `cargo test -p trackly-app auth::hash` | ⬜ pending |
| Bootstrap first-admin | USR-04 / D-Bootstrap-01 | — | Empty users table → bootstrap creates exactly one `role='admin'`; no default admin/admin seeded | integration | `cargo test -p trackly-app bootstrap` | ⬜ pending |
| Unified authorize() role×endpoint | USR-06 | T-05 (authz bypass) | Every forbidden role→endpoint pair returns 403; cannot bypass via direct HTTP | integration matrix | `cargo test -p trackly-app authorize_matrix` | ⬜ pending |
| Session survives restart | USR-03 / USR-05 / D-Session-01 | T-05 (session) | Session persisted in `sessions` (V010) survives server stop+start; logout revokes | integration | `cargo test -p trackly-app session_persist` | ⬜ pending |
| HTTPS-only + cert fingerprint | SRV-01 / SRV-03 / USR-07 / SET-08 | T-05 (transport) | Server binds HTTPS via rustls/rcgen; no HTTP listener; fingerprint computed | integration | `cargo test -p trackly-app tls_bind` | ⬜ pending |
| Hot start/stop + graceful drain | SRV-04 / SRV-05 / D-Server-01 | — | Toggle starts/stops axum task via child CancellationToken; in-flight requests drained | integration | `cargo test -p trackly-app server_lifecycle` | ⬜ pending |
| CSRF SameSite+Origin / headers | SRV-02 / D-Session-02 | T-05 (CSRF) | Cookie SameSite=Strict+Secure+HttpOnly; mutation rejects bad Origin; security headers present | integration | `cargo test -p trackly-app csrf_headers` | ⬜ pending |
| Rate-limit /login | SRV-02 / D-Auth-02 | T-05 (brute force) | >N attempts/min on /login throttled/blocked | integration | `cargo test -p trackly-app login_ratelimit` | ⬜ pending |
| RBAC UI gating | USR-02 / D-RBAC-03 | — | SIDEBAR_ITEMS filtered by role (employee→Заявки only; manager no Пользователи/admin Settings) | unit (vitest) | `cd ui && npm run test sidebar` | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] Test harness for in-process axum app (spawn router with rustls or HTTP-test client) to drive role × endpoint matrix and session tests
- [ ] Shared test fixture: temp SQLite DB with migrations applied + seeded users for each role (admin/manager/employee)
- [ ] `cargo nextest` optional install (CI already uses cargo test)
- [ ] vitest setup in `ui/` if not already present for sidebar role-gating unit test

*If existing infrastructure covers a row above, the planner marks it so and skips the Wave 0 stub.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Self-signed cert browser flow (нетех-сотрудник проходит предупреждение) | USR-07 / D-Server-04 | Real browser TLS-warning UX cannot be unit-tested | Start server, open `https://<ip>:<port>` from a LAN browser, follow «Дополнительно → Перейти», confirm login screen loads |
| Desktop unlocked-by-default vs optional lock | USR-04 / D-Desktop-01/02 | Tauri desktop UX behavior | Fresh DB → bootstrap wizard; toggle lock flag in /settings → restart desktop shows login screen |
| Cert fingerprint display + connect instructions | USR-07 / D-Server-04 | UI presentation of `https://…` + fingerprint | After server start, confirm UI shows address, SHA-256 fingerprint, and connect hint |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 120s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending

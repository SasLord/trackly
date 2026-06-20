---
phase: 10
slug: employee-employee-ui-role-gating-read
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-06-21
---

# Phase 10 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in `#[tokio::test]` via `cargo test`; integration-style against the real `axum::Router` with `tower::ServiceExt::oneshot` |
| **Config file** | None — assertions live in `crates/trackly-app/tests/role_endpoint_matrix.rs` |
| **Quick run command** | `cargo test --test role_endpoint_matrix` |
| **Full suite command** | `cargo test --workspace --no-fail-fast -- --test-threads=1` |
| **Estimated runtime** | ~60–120 seconds (workspace build dominated) |

> **Project rule:** never run two `cargo test` invocations concurrently (target/ lock contention looks like a multi-minute hang).

---

## Sampling Rate

- **After every task commit:** Run `cargo test --test role_endpoint_matrix`
- **After every plan wave:** Run `cargo test --workspace --no-fail-fast -- --test-threads=1`
- **Before `/gsd-verify-work`:** Full suite green + manual verification of the two frontend-only behaviors (D-UI-01, D-DENY-01)
- **Max feedback latency:** ~120 seconds

---

## Per-Task Verification Map

| Item | Decision | Wave | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|------|----------|------|------------|-----------------|-----------|-------------------|-------------|--------|
| devices read gate | D-GATE-01/02 | backend | API5 BFLA | Employee → `/api/v1/devices_list` → 403 | integration | `cargo test --test role_endpoint_matrix` | ✅ file / ❌ flip Case 9 (currently asserts 200) | ⬜ pending |
| acts read gate | D-GATE-01/02 | backend | API5 BFLA | Employee → `/api/v1/acts_list` → 403 | integration | `cargo test --test role_endpoint_matrix` | ❌ new case | ⬜ pending |
| cartridges read gate | D-GATE-01/02 | backend | API5 BFLA | Employee → `/api/v1/cartridges_list` → 403 | integration | `cargo test --test role_endpoint_matrix` | ❌ new case | ⬜ pending |
| printers read gate | D-GATE-01/02 | backend | API5 BFLA | Employee → `/api/v1/printers_list` → 403 | integration | `cargo test --test role_endpoint_matrix` | ❌ new case | ⬜ pending |
| reports read gate | D-GATE-01/02 | backend | API5 BFLA | Employee → representative report endpoint → 403 | integration | `cargo test --test role_endpoint_matrix` | ❌ new case | ⬜ pending |
| users read gate | D-GATE-01/02 | backend | API5 BFLA | Employee → `/api/v1/users_list` → 403 (verify `ManageUsers` already gates) | integration | `cargo test --test role_endpoint_matrix` | ✅ likely already enforced; add explicit case | ⬜ pending |
| own requests only | D-REQ-01 | backend | API1 BOLA / tampering | Employee → `requests_list` (no filter) → only own requests in body | integration (assert JSON body) | `cargo test --test role_endpoint_matrix` | ❌ new case + body-aware helper | ⬜ pending |
| request detail ownership | D-REQ-01 / OQ-1 | backend | API1 BOLA | Employee → `requests_get_history`/`get` for another user's id → 403/404 | integration | `cargo test --test role_endpoint_matrix` | ❌ new case | ⬜ pending |
| employee dashboard scoped | D-GATE-03 | backend | API1 mass exposure | Employee → `dashboard_get_all_widgets` → no org device/cartridge/printer fields, only request-derived | integration (assert JSON body) | `cargo test --test role_endpoint_matrix` | ❌ new case + body-aware helper | ⬜ pending |
| retained access (regression) | D-GATE-01 | backend | — | Manager/Admin → all reads → 200; Employee → own `requests_list` → 200 | integration | `cargo test --test role_endpoint_matrix` | ✅ partial / extend | ⬜ pending |
| employee shell | D-UI-01 | frontend | — | Employee sees only Requests-related nav, no other sections | manual | manual-only (no FE test runner) | ❌ checkpoint:human-verify | ⬜ pending |
| access-denied screen | D-DENY-01 | frontend | — | Employee → direct `#/devices` → "Нет доступа" + "К Заявкам"; API 403 handled in client.ts | manual | manual-only (no FE test runner) | ❌ checkpoint:human-verify | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] No new test file — extend `crates/trackly-app/tests/role_endpoint_matrix.rs` in place.
- [ ] Add a body-aware helper variant (e.g. `post_with_cookie_json() -> (StatusCode, serde_json::Value)`) — current `post_with_cookie` returns only `StatusCode`, which blocks D-REQ-01 ("only own requests") and D-GATE-03 ("no org-wide fields") body assertions.
- [ ] No frontend test runner exists (`vitest`/`@testing-library/svelte`/`playwright` absent in `ui/`). Do NOT introduce one — gate D-UI-01 and D-DENY-01 behind `checkpoint:human-verify` tasks.

*Backend infrastructure otherwise covers all gating requirements.*

---

## Manual-Only Verifications

| Behavior | Decision | Why Manual | Test Instructions |
|----------|----------|------------|-------------------|
| Employee shell shows only Requests | D-UI-01 | No frontend test runner in repo | Log in as employee (AD or local) → confirm minimal shell, landing on «Заявки», no nav to other sections |
| Access-denied screen + 403 handling | D-DENY-01 | No frontend test runner in repo | As employee, navigate directly to `#/devices` → see «Нет доступа» + «К Заявкам»; trigger a gated API call → 403 handled (not a crash/blank) |

---

## Validation Sign-Off

- [ ] All backend gating items have automated `cargo test --test role_endpoint_matrix` coverage
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify (frontend pair is the only manual block, gated by human-verify)
- [ ] Wave 0 body-aware helper added before D-REQ-01/D-GATE-03 cases
- [ ] No watch-mode flags; single-threaded test invocation
- [ ] Feedback latency < 120s
- [ ] `nyquist_compliant: true` set in frontmatter after planner wires every item to a task

**Approval:** pending

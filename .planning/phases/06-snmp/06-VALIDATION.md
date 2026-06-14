---
phase: 6
slug: snmp
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-06-14
---

# Phase 6 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (unit/integration); `cargo nextest` recommended |
| **Config file** | Workspace `Cargo.toml` (no separate test config) |
| **Quick run command** | `cargo test -p trackly-app -- printers 2>&1 \| head -50` |
| **Full suite command** | `cargo test --workspace` |
| **Estimated runtime** | ~60–120 seconds (workspace) |

> ⚠️ Per project convention: run only ONE `cargo test` at a time — concurrent runs contend on the `target/` lock and look like a multi-minute hang.

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p trackly-app -- --test-threads=1 2>&1 | tail -20`
- **After every plan wave:** Run `cargo test --workspace`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** ~120 seconds

---

## Per-Task Verification Map

| Req ID | Behavior | Threat Ref | Test Type | Automated Command | File Exists | Status |
|--------|----------|------------|-----------|-------------------|-------------|--------|
| PRN-01 | Discovery: parse sysObjectID → vendor | — | unit | `cargo test -p trackly-app test_vendor_identify` | ❌ W0 | ⬜ pending |
| PRN-02 | Snapshot parsing: level/max → percent | — | unit | `cargo test -p trackly-app test_toner_percent` | ❌ W0 | ⬜ pending |
| PRN-03 | OID profile seed: 5 profiles in DB | — | integration | `cargo test -p trackly-app test_oid_profiles_seeded` | ❌ W0 | ⬜ pending |
| PRN-06 | Alert detection: hrDeviceStatus=down → alert upsert | T-snmp-status | unit | `cargo test -p trackly-app test_alert_detection` | ❌ W0 | ⬜ pending |
| PRN-08 | MockSnmpClient returns fixtures | — | unit | `cargo test -p trackly-infra test_mock_snmp` | ❌ W0 | ⬜ pending |
| REQ-01 | RequestService::create persists to DB | — | integration | `cargo test -p trackly-app test_request_create` | ❌ W0 | ⬜ pending |
| REQ-03 | Lifecycle: invalid transition → error | T-req-lifecycle | unit | `cargo test -p trackly-app test_request_lifecycle` | ❌ W0 | ⬜ pending |
| REQ-04 | WS broadcast: event sent after request create | T-ws-auth | unit | `cargo test -p trackly-app test_ws_event_sent` | ❌ W0 | ⬜ pending |
| REQ-05 | CART-07 link → request status=completed | — | integration | `cargo test -p trackly-app test_req_cart_link` | ❌ W0 | ⬜ pending |
| D-Mock-01 | Runtime switch: env → mock client | — | unit | `cargo test -p trackly-app test_snmp_mock_switch` | ❌ W0 | ⬜ pending |
| D-Retention-01 | prune_old_readings deletes > retention | — | unit | `cargo test -p trackly-app test_readings_prune` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/trackly-app/src/services/printer_service.rs` — unit tests for discovery/poll/alert logic
- [ ] `crates/trackly-app/src/services/request_service.rs` — unit tests for create/lifecycle/CART-07 link
- [ ] `crates/trackly-infra/src/snmp/mock.rs` — deterministic mock fixtures (incl. problem/offline states)
- [ ] `migrations/V020+__*.sql` — migrations (printers, printer_readings, oid_profiles+seed, optional request_categories/printer_alerts)
- [ ] `snmp2` (feature `crypto-rust`) added to `crates/trackly-infra/Cargo.toml`

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Vendor-specific OID correctness on real Pantum BM5100ADN | PRN-02, PRN-03 | Vendor OIDs are community-verified (MEDIUM confidence), not lab-tested; mock cannot prove real-device correctness | On Windows test machine with a real Pantum BM5100ADN: run discovery, confirm toner % and page count match the printer's own panel. Repeat snmpwalk for Canon iR supply order. |
| Live SNMP poll against any reachable network printer | PRN-01, PRN-02 | Dev macOS has no reachable printers; mock covers logic but not real network I/O | On a LAN with a real SNMP printer, run discovery + "Обновить сейчас" and confirm a real `printer_readings` snapshot is written. |
| Browser WebSocket push delivery (LAN) | REQ-04 | End-to-end WS path through a real browser over LAN is outside unit scope | From a LAN browser logged in as specialist/admin, create a request from another session; confirm the in-app notification arrives in real time. |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 120s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending

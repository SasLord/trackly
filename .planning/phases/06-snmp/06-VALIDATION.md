---
phase: 6
slug: snmp
status: draft
nyquist_compliant: true
wave_0_complete: true
wave_0_plan: 06-00-PLAN.md
created: 2026-06-14
revised: 2026-06-14
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

> Per project convention: run only ONE `cargo test` at a time — concurrent runs contend on the `target/` lock and look like a multi-minute hang.

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p trackly-app -- --test-threads=1 2>&1 | tail -20`
- **After every plan wave:** Run `cargo test --workspace`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** ~120 seconds

---

## Per-Task Verification Map

| Req ID | Behavior | Threat Ref | Test Type | Automated Command | Plan/Wave | Status |
|--------|----------|------------|-----------|-------------------|-----------|--------|
| PRN-01 | Discovery: parse sysObjectID → vendor | — | unit | `cargo test -p trackly-app test_vendor_identify` | 06-02 W2 | ⬜ pending |
| PRN-02 | Snapshot parsing: level/max → percent | — | unit | `cargo test -p trackly-app test_toner_percent` | 06-02 W2 | ⬜ pending |
| PRN-03 | OID profile seed: 5 profiles in DB | — | integration | `cargo test -p trackly-infra test_oid_profiles_seeded` | 06-01 W1 | ⬜ pending |
| PRN-04 | USB-only printer: NULL ip_address + usb_host_device_id | — | integration | `cargo test -p trackly-infra test_printer_usb_only` | 06-02 W2 | ⬜ pending |
| PRN-06 | Alert detection: hrDeviceStatus=down → alert upsert | T-snmp-status | unit | `cargo test -p trackly-app test_alert_detection` | 06-02 W2 | ⬜ pending |
| PRN-07 | Current cartridge for printer: FK link via cartridges.current_printer_device_id | D-PRN07-01 | integration | `cargo test -p trackly-app test_current_cartridge_for_printer` | 06-02 W2 | ⬜ pending |
| PRN-08 | MockSnmpClient returns fixtures | — | unit | `cargo test -p trackly-infra test_mock_snmp` | 06-01 W1 | ⬜ pending |
| REQ-01 | RequestService::create persists to DB | — | integration | `cargo test -p trackly-app test_request_create` | 06-02 W2 | ⬜ pending |
| REQ-03 | Lifecycle: invalid transition → error | T-req-lifecycle | unit | `cargo test -p trackly-app test_request_lifecycle` | 06-02 W2 | ⬜ pending |
| REQ-04 | WS broadcast: event sent after request create | T-ws-auth | unit | `cargo test -p trackly-app test_ws_event_sent` | 06-02 W2 | ⬜ pending |
| REQ-05 | CART-07 link → request status=completed | — | integration | `cargo test -p trackly-app test_req_cart_link` | 06-02 W2 | ⬜ pending |
| D-Mock-01 | Runtime switch: env → mock client (in AppCtx::build) | — | unit | `cargo test -p trackly-app test_snmp_mock_switch` | 06-03 W3 | ⬜ pending |
| D-Retention-01 | prune_old_readings deletes > retention | — | unit | `cargo test -p trackly-app test_readings_prune` | 06-02 W2 | ⬜ pending |
| ASVS V2/V4 | GET /api/v1/ws without session → 401 before WS upgrade | T-06-09-E | integration | `cargo test -p trackly-app test_ws_unauth_401` | 06-03 W3 | ⬜ pending |
| CLAUDE.md | Secret<T> Debug does not leak value (prints "***") | T-06-07-I | unit | `cargo test -p trackly-app test_secret_debug` | 06-02 W2 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Scaffold (06-00-PLAN.md)

Wave 0 creates stub test files with all test functions declared as `#[ignore]`. This satisfies Nyquist compliance — tests compile and exist before implementation. Each wave removes the `#[ignore]` and adds the real implementation.

### Stub files created by Wave 0:

| File | Tests (14 total) |
|------|-----------------|
| `crates/trackly-app/tests/phase06_stubs.rs` | test_vendor_identify, test_toner_percent, test_oid_profiles_seeded, test_printer_usb_only, test_alert_detection, test_request_create, test_request_lifecycle, test_ws_event_sent, test_req_cart_link, test_snmp_mock_switch, test_readings_prune, test_current_cartridge_for_printer, test_ws_unauth_401, test_secret_debug |
| `crates/trackly-infra/tests/phase06_stubs.rs` | test_mock_snmp |

### Wave dependency chain (all plans depend on 06-00):
- Wave 0 (06-00): creates stubs
- Wave 1 (06-01): depends_on [06-00]
- Wave 2 (06-02): depends_on [06-01], implements test_vendor_identify, test_toner_percent, test_alert_detection, test_request_create, test_request_lifecycle, test_ws_event_sent, test_req_cart_link, test_readings_prune, test_printer_usb_only, test_current_cartridge_for_printer, test_secret_debug
- Wave 3 (06-03): depends_on [06-02], implements test_snmp_mock_switch, test_ws_unauth_401
- Wave 4 (06-04, 06-05): depends_on [06-03], UI plans
- Wave 5 (06-06): depends_on [06-04, 06-05], final integration

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Vendor-specific OID correctness on real Pantum BM5100ADN | PRN-02, PRN-03 | Vendor OIDs are community-verified (MEDIUM confidence), not lab-tested; mock cannot prove real-device correctness | On Windows test machine with a real Pantum BM5100ADN: run discovery, confirm toner % and page count match the printer's own panel. |
| Live SNMP poll against any reachable network printer | PRN-01, PRN-02 | Dev macOS has no reachable printers; mock covers logic but not real network I/O | On a LAN with a real SNMP printer, run discovery + "Обновить сейчас" and confirm a real `printer_readings` snapshot is written. |
| Browser WebSocket push delivery (LAN) | REQ-04 | End-to-end WS path through a real browser over LAN is outside unit scope | From a LAN browser logged in as specialist/admin, create a request from another session; confirm the in-app notification arrives in real time. |

---

## Validation Sign-Off

- [x] All tests have `<automated>` verify commands
- [x] Wave 0 stubs exist for ALL tests (nyquist_compliant: true)
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] No watch-mode flags
- [x] Feedback latency < 120s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** pending execution

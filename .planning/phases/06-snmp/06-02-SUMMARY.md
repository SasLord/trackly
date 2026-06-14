---
phase: "06-snmp"
plan: "06-02"
subsystem: "printer-service-request-service-dto"
tags: ["printers", "requests", "services", "dto", "websocket", "snmp", "auth"]
dependency_graph:
  requires: ["06-01"]
  provides: ["06-03", "06-04", "06-05", "06-06"]
  affects: ["trackly-app", "trackly-core", "trackly-infra"]
tech_stack:
  added: []
  patterns:
    - "Single-writer WriterHandle::execute() for all mutations"
    - "ReaderPool::acquire() for all reads"
    - "WsEvent broadcast channel for real-time push"
    - "Secret<T> masking community strings in Debug"
    - "community_configured: bool only — never serialize raw community string"
    - "RequestTransitionOp optimistic locking with version field"
key_files:
  created:
    - "crates/trackly-app/src/dto/printer.rs"
    - "crates/trackly-app/src/dto/request.rs"
    - "crates/trackly-app/src/services/printer_service.rs"
    - "crates/trackly-app/src/services/request_service.rs"
  modified:
    - "crates/trackly-app/src/dto/mod.rs"
    - "crates/trackly-app/src/services/mod.rs"
    - "crates/trackly-app/tests/phase06_stubs.rs"
    - "crates/trackly-core/src/auth.rs"
    - "crates/trackly-infra/src/repos/requests_sqlite.rs"
decisions:
  - "WsEvent::RequestStatusChanged (not RequestUpdated) — canonical event name per 06-CONTEXT.md"
  - "community_configured: bool in PrinterDto — raw community string never reaches frontend (T-06-07-I)"
  - "PrinterService helpers parse_toner_level/detect_alert_type/identify_vendor are pub for test access"
  - "RequestService::create uses Action::CreateRequest (all roles can create)"
  - "MissedTickBehavior::Skip in run_poll_task to avoid thundering herd"
  - "Action variants MutatePrinters/TransitionRequests/ReadPrinters/ReadRequests added to auth.rs"
metrics:
  duration: "multi-session (prev session + current)"
  completed_date: "2026-06-14"
  tasks_completed: 2
  tasks_total: 2
  files_created: 4
  files_modified: 5
---

# Phase 06 Plan 02: Services + DTO Layer Summary

**One-liner:** PrinterService and RequestService with WS broadcast, PrinterDto/RequestDto with camelCase serde, and 11 integration tests covering the full lifecycle.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | SQLite repositories (printers + requests) | 635c4e0 | printers_sqlite.rs, requests_sqlite.rs, repos/mod.rs |
| 2 | PrinterService + RequestService + DTO layer | 7a66fe6 | printer.rs, request.rs, printer_service.rs, request_service.rs, auth.rs |

## What Was Built

### DTOs (`trackly-app/src/dto/`)

**printer.rs:**
- `PrinterDto` — enriched view with `community_configured: bool` (never raw community string)
- `WsEvent` — `#[serde(tag = "type", rename_all = "snake_case")]` enum with:
  - `NewRequest { request_id, request_type, requester_name }`
  - `RequestStatusChanged { request_id, new_status }` — canonical name per 06-CONTEXT
  - `PrinterAlert { printer_id, printer_name, alert_type }`
  - `is_visible_to(&Identity)` — role-based WS event filtering (T-06-06-I)
- `PrinterCreateDto`, `DiscoveredPrinterDto`, `PrinterListResponse`

**request.rs:**
- `RequestDto` — full request view with requester_name, assigned_to_name
- `RequestTransitionPayload` — `#[serde(tag = "op", rename_all = "camelCase")]` with Accept/Reject/Complete variants
- `RequestCreateDto`, `RequestListResponse`, `RequestCountsDto`

### Services (`trackly-app/src/services/`)

**printer_service.rs:**
- `PrinterService` with `list`, `get` (enriched), `create_from_device`, `acknowledge_alert`, `prune_old_readings`, `poll_single`, `poll_all`, `discover`
- Module-level helpers (pub): `parse_toner_level(level, max, encoding) -> Option<u8>`, `detect_alert_type(status) -> Option<&str>`, `identify_vendor(sys_object_id) -> Option<&str>`
- `run_poll_task(service, interval_secs)` — tokio interval loop with `MissedTickBehavior::Skip`; prunes old readings on every tick

**request_service.rs:**
- `RequestService` with `get`, `list`, `counts`, `create`, `transition`
- `create()` broadcasts `WsEvent::NewRequest` after successful write
- `transition()` validates domain rule via `RequestTransitionOp::validate_from_status()`, broadcasts `WsEvent::RequestStatusChanged`

### Auth extensions (`trackly-core/src/auth.rs`)

Added `Action` variants:
- `MutatePrinters` — Admin | Manager
- `TransitionRequests` — Admin | Manager
- `ReadPrinters` — Admin | Manager
- `ReadRequests` — all roles

### Integration tests (`phase06_stubs.rs`)

11 tests passing, 3 intentionally `#[ignore]` (deferred to HTTP layer / Wave 3+):

| Test | Coverage |
|------|---------|
| `test_oid_profiles_seeded` | PRN-03 |
| `test_vendor_identify` | PRN-01 |
| `test_toner_percent` | PRN-02 |
| `test_printer_usb_only` | PRN-04 |
| `test_alert_detection` | PRN-06 |
| `test_request_create` | REQ-01 |
| `test_request_lifecycle` | REQ-03 |
| `test_ws_event_sent` | REQ-04 |
| `test_readings_prune` | D-Retention-01 |
| `test_current_cartridge_for_printer` | PRN-07 |
| `test_secret_debug` | T-06-07-I |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `d.location` column not in devices table**
- **Found during:** Task 1 (printers_sqlite.rs)
- **Issue:** Plan assumed `devices.location` text column; actual schema has `location_id FK → locations`
- **Fix:** Added `LEFT JOIN locations l ON l.id = d.location_id` and `l.name AS device_location`
- **Files modified:** `crates/trackly-infra/src/repos/printers_sqlite.rs`
- **Commit:** 635c4e0

**2. [Rule 1 - Bug] `users.display_name` column doesn't exist**
- **Found during:** Task 1 (requests_sqlite.rs)
- **Issue:** Plan assumed `display_name`; V002 creates `full_name`
- **Fix:** `SELECT_REQUESTS` uses `u.full_name AS requester_name`
- **Files modified:** `crates/trackly-infra/src/repos/requests_sqlite.rs`
- **Commit:** 635c4e0

**3. [Rule 1 - Bug] `printer_readings.toner_levels_json` column doesn't exist**
- **Found during:** Task 1 (printers_sqlite.rs)
- **Issue:** V022 creates column named `toner_levels` (not `toner_levels_json`)
- **Fix:** INSERT/SELECT use `toner_levels`; domain struct field name kept as `toner_levels_json`
- **Files modified:** `crates/trackly-infra/src/repos/printers_sqlite.rs`
- **Commit:** 635c4e0

**4. [Rule 1 - Bug] `OidValueKind` type doesn't exist**
- **Found during:** Task 1 (printers_sqlite.rs)
- **Issue:** Actual type in `trackly_core::ports::snmp` is `SnmpValue`
- **Fix:** Used `SnmpValue::Integer(n)` pattern
- **Commit:** 635c4e0

**5. [Rule 1 - Bug] `ProbedDevice.sys_object_id` is `String` not `Option<String>`**
- **Found during:** Task 2 (printer_service.rs)
- **Fix:** `identify_vendor(&probed.sys_object_id)` (removed `.as_deref()`)
- **Commit:** 7a66fe6

**6. [Rule 2 - Missing trait import] `Clock` trait not in scope in tests**
- **Found during:** Task 2 test compilation
- **Fix:** Added `use trackly_core::primitives::clock::Clock;` to 3 async test functions
- **Commit:** 7a66fe6

## Known Stubs

Three tests are `#[ignore]` and deferred to later waves:

| Test | Reason | Plan to resolve |
|------|--------|----------------|
| `test_req_cart_link` | Requires cartridge install flow (Phase 5 act) | 06-05 or 06-06 |
| `test_snmp_mock_switch` | Requires full AppCtx struct (not yet built) | 06-03 |
| `test_ws_unauth_401` | Requires HTTP layer (axum handlers) | 06-04 |

## Threat Surface Scan

No new unplanned threat surface. All surfaces covered by threat model in 06-02-PLAN.md:

| Threat | Mitigation |
|--------|-----------|
| T-06-07-I (community string leak) | `community_configured: bool` only in PrinterDto |
| T-06-04-S (unauthorized transition) | `authorize(caller, &Action::TransitionRequests)?` gate |
| T-06-06-I (WS event leakage) | `WsEvent::is_visible_to(&Identity)` role filter |

## Self-Check: PASSED

Files confirmed present:
- `/Users/madsas/Projects/trackly/crates/trackly-app/src/dto/printer.rs` — FOUND
- `/Users/madsas/Projects/trackly/crates/trackly-app/src/dto/request.rs` — FOUND
- `/Users/madsas/Projects/trackly/crates/trackly-app/src/services/printer_service.rs` — FOUND
- `/Users/madsas/Projects/trackly/crates/trackly-app/src/services/request_service.rs` — FOUND

Commits confirmed:
- `635c4e0` — feat(06-02): SQLite repositories for printers and requests
- `7a66fe6` — feat(06-02): add PrinterService, RequestService, and DTO layer

Tests: 11 passed, 0 failed, 3 ignored (intentional)

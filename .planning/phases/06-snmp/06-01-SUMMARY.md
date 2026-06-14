---
phase: 06-snmp
plan: "01"
subsystem: database-schema + core-domain + snmp-adapters
tags: [migrations, snmp, domain, hexagonal, mock]
dependency_graph:
  requires: [06-00]
  provides: [06-02, 06-03, 06-04, 06-05, 06-06]
  affects: [trackly-core, trackly-infra]
tech_stack:
  added:
    - snmp2 0.5 (crypto-rust + tokio features; workspace dep)
    - async-trait in trackly-core ports/snmp.rs
  patterns:
    - hexagonal port trait in trackly-core without I/O (async_trait only)
    - MockSnmpClient with 3 deterministic fixtures keyed by IP
    - RealSnmpClient always wraps snmp2::AsyncSession in tokio::time::timeout (Pitfall 1)
key_files:
  created:
    - migrations/V020__printers.sql
    - migrations/V021__oid_profiles_seed.sql
    - migrations/V022__printer_readings.sql
    - migrations/V023__printer_alerts.sql
    - migrations/V024__request_categories.sql
    - migrations/V025__cartridge_printer_link.sql
    - crates/trackly-core/src/domain/printers.rs
    - crates/trackly-core/src/domain/requests.rs
    - crates/trackly-core/src/ports/snmp.rs
    - crates/trackly-core/src/ports/printers.rs
    - crates/trackly-core/src/ports/requests.rs
    - crates/trackly-infra/src/snmp/mod.rs
    - crates/trackly-infra/src/snmp/real.rs
    - crates/trackly-infra/src/snmp/mock.rs
  modified:
    - crates/trackly-core/src/domain/mod.rs (pub mod printers; pub mod requests;)
    - crates/trackly-core/src/ports/mod.rs (pub mod printers; pub mod requests; pub mod snmp;)
    - crates/trackly-infra/src/lib.rs (pub mod snmp;)
    - crates/trackly-infra/Cargo.toml (snmp2 = { workspace = true })
    - crates/trackly-infra/src/test_support/test_db.rs (schema_version assert 19→25)
    - crates/trackly-infra/tests/migration_idempotency.rs (migration count 19→25)
    - crates/trackly-infra/tests/phase06_stubs.rs (test_mock_snmp implemented)
    - crates/trackly-app/tests/phase06_stubs.rs (test_oid_profiles_seeded implemented)
    - Cargo.toml (snmp2 workspace dep)
decisions:
  - snmp2 0.5 used instead of plan-specified 0.4 — 0.4 lacks the crypto-rust feature flag that appeared in 0.5
  - snmp2::AsyncSession::get_many() used for multi-OID GET (not individual get() per OID)
  - MockSnmpClient returns None for offline fixture (simulates timeout, not error)
  - RequestTransitionOp placed in domain/printers.rs with re-export from domain/requests.rs (co-location with lifecycle logic)
metrics:
  duration: "22 minutes"
  completed_date: "2026-06-14"
  tasks_completed: 2
  files_changed: 21
---

# Phase 06 Plan 01: DB Schema Foundation + SNMP Hexagonal Layer Summary

Laid the complete Phase 6 foundation: 6 SQL migrations (V020-V025), snmp2 0.5 workspace dep, hexagonal domain/port layer in trackly-core, and real+mock SNMP adapters in trackly-infra.

## Tasks Completed

| Task | Name | Commit | Key Files |
|------|------|--------|-----------|
| 1 | Migrations V020-V025 + snmp2 | 8cd56c5 | migrations/V020-V025, Cargo.toml |
| 2 | Core domain/ports + SNMP adapters | 39a0adb | ports/snmp.rs, domain/printers.rs, snmp/mock.rs |

## What Was Built

**Migrations (V020-V025):**
- `printers` table: FK→devices, SNMP fields, `CHECK(ip IS NOT NULL OR usb IS NOT NULL)`
- `oid_profiles` table with 5 vendor seeds: pantum (percent encoding), kyocera, hp, canon, rfc3805 fallback
- `printer_readings` time-series table with index on `(printer_id, ts_utc DESC)`
- `printer_alerts` table with `UNIQUE(printer_id)` dedup constraint
- `request_categories` lookup + ALTER TABLE requests (category_id, completed_cartridge_id)
- `cartridges.current_printer_device_id` FK→devices (D-PRN07-01)

**Core hexagonal layer:**
- `SnmpClient` async trait in trackly-core with zero I/O deps — `get_oids()` and `probe()`
- `PrinterRepository` with `current_cartridge_for_printer()` method (D-PRN07-01)
- `RequestRepository` read-only port trait
- Domain structs: `PrinterRow`, `PrinterReadingRow`, `PrinterAlertRow`, `OidProfileRow`, `RequestRow`, `RequestTransitionOp` with `validate_from_status/target_status/audit_action`

**SNMP adapters:**
- `RealSnmpClient`: snmp2 `AsyncSession::get_many()`, always wrapped in `tokio::time::timeout`
- `MockSnmpClient`: 3 fixtures (Pantum 45% ok, HP 8% warning, Canon offline)

**Nyquist gates:**
- `test_mock_snmp` (PRN-08): #[ignore] removed, implemented and green
- `test_oid_profiles_seeded` (PRN-03): #[ignore] removed, implemented and green

## Success Criteria Verification

- [x] schema_version = 25 after migrations (test_db_returns_fully_migrated_connection)
- [x] oid_profiles = 5 rows (pantum/kyocera/hp/canon/rfc3805)
- [x] cartridges.current_printer_device_id column exists (V025)
- [x] SnmpClient trait in trackly-core has NO tokio/snmp2 imports (no_io_deps test green)
- [x] PrinterRepository::current_cartridge_for_printer declared in trait
- [x] MockSnmpClient with 3 fixtures including offline for alert testing
- [x] RealSnmpClient wraps all SNMP calls in tokio::time::timeout
- [x] test_mock_snmp green (PRN-08)
- [x] test_oid_profiles_seeded green (PRN-03)
- [x] cargo test --workspace green

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] snmp2 0.4 has no crypto-rust feature**
- **Found during:** Task 1 — cargo resolve failed with "package depends on snmp2 with feature crypto-rust but snmp2 does not have that feature"
- **Fix:** Updated snmp2 version from "0.4" to "0.5" in Cargo.toml; feature `crypto-rust` exists in 0.5.0 as part of default features
- **Files modified:** Cargo.toml (workspace dep)
- **Commit:** 8cd56c5

**2. [Rule 1 - Bug] migration_idempotency.rs hardcoded version 19**
- **Found during:** Task 1 — test expected 19 applied migrations, now 25 exist
- **Fix:** Updated hardcoded counts 19→25 in migration_idempotency.rs
- **Files modified:** crates/trackly-infra/tests/migration_idempotency.rs
- **Commit:** 8cd56c5

**3. [Rule 1 - Adjustment] snmp2 uses get_many() not get(&[oids])**
- **Found during:** Task 2 — plan described `sess.get(&parsed_oids)` with a slice, but snmp2 API has `get(oid: &Oid<'_>)` (single) and `get_many(oids: &[&Oid<'_>])` (multi)
- **Fix:** Used `get_many()` with collected `&Oid` refs
- **Files modified:** crates/trackly-infra/src/snmp/real.rs
- **Commit:** 39a0adb

## Known Stubs

None. All implemented functionality is wired correctly.

## Threat Flags

None. No new network endpoints or auth paths introduced in this plan. SQL migrations use embedded compile-time data only (T-06-01-M: snmp2 verified at crates.io — roboplc/snmp2 v0.5.0).

## Self-Check: PASSED

- migrations/V020__printers.sql: FOUND
- migrations/V025__cartridge_printer_link.sql: FOUND
- crates/trackly-core/src/ports/snmp.rs: FOUND
- crates/trackly-infra/src/snmp/mock.rs: FOUND
- Commit 8cd56c5: FOUND (git log)
- Commit 39a0adb: FOUND (git log)
- test_db schema_version=25: PASSED
- test_mock_snmp: PASSED (not #[ignore])
- test_oid_profiles_seeded: PASSED (not #[ignore])
- no_io_deps: PASSED

---
phase: 06-snmp
plan: "08"
subsystem: printers
tags: [gap-closure, prn-01, prn-04, d-gap-printer-add, d-gap-replace-select]
dependency_graph:
  requires: [06-07]
  provides: [printers_admit_working, printer_manual_create, cartridge_replace_select_fixed]
  affects: [printers, requests, devices]
tech_stack:
  added: []
  patterns:
    - "admit: probe → create device(type_id=2) → create printer row (two-step)"
    - "D-GAP-Replace-Select: devices.list(type_id=2) as printer source in requests"
key_files:
  created:
    - ui/src/features/printers/PrinterCreateModal.svelte
  modified:
    - crates/trackly-app/src/tauri_cmds/printers.rs
    - crates/trackly-app/src/http/printers.rs
    - ui/src/features/printers/api.ts
    - ui/src/features/printers/PrintersPage.svelte
    - ui/src/features/printers/DiscoveryModal.svelte
    - ui/src/features/requests/RequestFormModal.svelte
decisions:
  - admit returns Vec<PrinterDto> (richer than count); frontend uses .length for toast
  - duplicate IP check reuses same spawn_blocking pattern as discover()
  - manual create is two-step: devices.create(type_id=2) then printers.create (no new backend command needed)
metrics:
  duration: "~7 min"
  completed: "2026-06-15T07:07:04Z"
  tasks_completed: 2
  tasks_total: 3
  files_modified: 7
---

# Phase 6 Plan 08: Admit + Manual Printer + Replace-Select Fix Summary

**One-liner:** Discovery admit now creates device(type=Принтер)+printers row per IP; manual «Завести принтер» form covers USB/non-SNMP; cartridge-replace select sources all devices type=Принтер.

## What Was Built

### Task 1: Implement printers_admit (PRN-01 end-to-end)

Replaced the stub `build_printers_admit` with a full implementation:

- For each selected IP: check duplicate by scanning printers table (same `spawn_blocking` pattern as `discover()`); if duplicate, skip.
- Probe SNMP for `sys_name`/`sys_descr` (fallback name: `"Принтер <ip>"`).
- Create device via `ctx.devices.create(DeviceNew { type_id: 2, status_id: 1, name: <probed_name>, ... })`.
- Create printer row via `ctx.printers.create_from_device(PrinterCreateDto { device_id: <new_id>, ip, community, snmp_version: "v2c", ... })`.
- Return `Vec<PrinterDto>` (richer than count; frontend takes `.length` for the toast).
- Added `handler_admit` + `AdmitPayload` to `http/printers.rs` router (`POST /api/v1/printers_admit`).

**Authorization:** `MutatePrinters` (Admin | Manager) — enforced at the top of `build_printers_admit`.

### Task 2: Manual printer form «Завести принтер» (PRN-04)

Created `PrinterCreateModal.svelte`:

- Required field: **Наименование** (device name).
- Optional: **Расположение** (LocationAutocomplete).
- Optional SNMP section: **IP-адрес** + **community** (shown only when IP is filled in).
- Two-step submit: `devices.create(type_id=2, status_id=1)` then `printers.create({ deviceId, ipAddress, communityUpdate, snmpVersion: 'v2c' })`.
- If IP is empty: `ipAddress: null`, `communityUpdate: null` — USB/local printer, no SNMP.

Updated `PrintersPage.svelte`:

- Imported `Button` + `PrinterCreateModal`.
- Added `createOpen` state.
- Added `«Завести принтер»` button in page header (visible for admin or manager roles).
- Mounted `<PrinterCreateModal>` with `onSuccess → refresh()`.

### Task 3: Cartridge replace — switch printer source to devices (D-GAP-Replace-Select)

Updated `RequestFormModal.svelte`:

- Replaced `printers.list(...)` with `devices.list({ type_id: 2, ... }, { offset: 0, limit: 200 })`.
- `availablePrinters` state changed from `PrinterDto[]` to `DeviceDto[]`.
- Select option label: `device.name` (was `p.deviceName ?? p.ipAddress`); value: `device.id` (FK to `requests.printer_device_id` → `devices.id`).
- Includes USB/non-SNMP printers (all `type_id=2` devices), not just SNMP-registered ones.

Updated `printers/api.ts`: `admit` return type changed from `apiCall<number>` to `apiCall<PrinterDto[]>`.

Updated `DiscoveryModal.svelte`: `handleCreate` now uses `admitted.length` for the count toast.

## Verification

- `cargo check --workspace`: green.
- `cargo test -p trackly-app`: 1 passed, 0 failed.
- `cargo test -p trackly-app --test export_bindings`: 1 passed.
- `pnpm svelte-check`: 0 errors, 31 warnings (all pre-existing).

## Deviations from Plan

None — plan executed exactly as written.

The plan noted that admit return type could be `Vec<PrinterDto>` or count — chose `Vec<PrinterDto>` as recommended. Frontend updated to use `.length`.

## Threat Mitigations Applied (from threat model)

| Threat | Status |
|--------|--------|
| T-06-08-01: EoP — admit/create | Mitigated: `authorize(caller, MutatePrinters)` at top of `build_printers_admit` |
| T-06-08-02: Tampering — sys_name as device.name | Mitigated: `DeviceService::validate_new` rejects empty name; SNMP data treated as params (not SQL) |
| T-06-08-03: Info Disclosure — community | Accepted: `PrinterDto` does not expose community (communityConfigured: bool) |
| T-06-08-04: Spoofing — duplicate IP | Mitigated: duplicate check before create, skips existing IPs |

## Checkpoint: PENDING HUMAN VERIFICATION

Task 3 is a `checkpoint:human-verify` gate requiring manual dev-mode verification (see below).

## Known Stubs

None — all implementation is functional.

## Self-Check: PASSED

- `ui/src/features/printers/PrinterCreateModal.svelte` — exists ✓
- commit `a1c147b` (Task 1) — exists ✓
- commit `f3ae33b` (Tasks 2+3) — exists ✓
- `cargo check --workspace` green ✓
- `pnpm svelte-check` 0 errors ✓

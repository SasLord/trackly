---
phase: 02-ui
plan: 03
subsystem: ui
tags: [rust, svelte, sqlite, tauri, axum, specta, devices-crud, audit-log, optimistic-lock]

requires:
  - phase: 02-ui
    provides: "UI scaffold, Component library, AppCtx, WriterHandle/ReaderPool, AppConfig, logging"
  - phase: 01-foundation
    provides: "SQLite WAL DB, migrations V001-V013, AppCtx::build, devices domain + ports stubs"

provides:
  - "DeviceService: create/get/list/update/delete_soft with audit_log in same transaction"
  - "SqliteDeviceRepository: full CRUD with V003 column mapping + optimistic lock"
  - "6 Tauri commands + 6 axum handlers via build_* dual-transport pattern"
  - "Frontend DevicesPage: table list + form modal + context menu + destructive confirm"
  - "bindings.ts regenerated with all Device types (DeviceDto/DeviceNew/DevicePatch/DeviceFilter)"

affects:
  - "02-04 (lookups/search will extend DeviceService and DeviceFormModal)"
  - "02-05 (CSV import adds to DeviceService)"
  - "All plans using AppCtx (ctx.devices now populated)"

tech-stack:
  added: []
  patterns:
    - "build_* helper + thin Tauri command wrapper (dual-transport pattern)"
    - "AppErrorResponse newtype for axum IntoResponse (orphan rule workaround)"
    - "#[specta(type = i32)] on i64 DTO fields (BigInt forbidden in specta-typescript)"
    - "Tauri command param naming: avoid TypeScript reserved words (new → device)"
    - "$effect() for form reset on modal open in Svelte 5"

key-files:
  created:
    - crates/trackly-infra/src/repos/devices_sqlite.rs
    - crates/trackly-app/src/dto/device.rs
    - crates/trackly-app/src/services/device_service.rs
    - crates/trackly-app/src/tauri_cmds/devices.rs
    - crates/trackly-app/src/http/devices.rs
    - crates/trackly-app/capabilities/main.json
    - crates/trackly-app/tests/devices_crud.rs
    - crates/trackly-app/tests/devices_http_smoke.rs
    - ui/src/lib/api/devices.ts
    - ui/src/features/devices/DevicesPage.svelte
    - ui/src/features/devices/DeviceList.svelte
    - ui/src/features/devices/DeviceListRow.svelte
    - ui/src/features/devices/DeviceFormModal.svelte
    - ui/src/features/devices/DeviceContextMenu.svelte
    - ui/src/features/devices/api.ts
  modified:
    - crates/trackly-app/src/main.rs
    - crates/trackly-app/src/specta_export.rs
    - crates/trackly-app/src/error_axum.rs
    - crates/trackly-app/tests/export_bindings.rs
    - ui/src/lib/api/index.ts
    - ui/src/routes.ts

key-decisions:
  - "AppErrorResponse(AppError) newtype in trackly-app to satisfy axum IntoResponse (orphan rule: AppError in trackly-core, IntoResponse in axum)"
  - "#[specta(type = i32/u32)] on all i64/u64 DTO fields — specta-typescript 0.0.x forbids BigInt export; SQLite IDs and timestamps fit in i32 for the app's expected lifetime"
  - "Tauri command params for id/version: i32 (not i64) — avoids BigInt, TS number covers SQLite IDs; cast to i64 before delegation to build_* helpers"
  - "Rename Tauri command param 'new: DeviceNew' → 'device: DeviceNew' — 'new' is a TypeScript reserved keyword, would break bindings.ts type extraction"
  - "DeviceFormModal 'Расположение' field backed by freetext Input (not location_id FK) — Plan 04 wires real location lookup; field name set to 'location' (client-only)"
  - "Placeholder device type and status hardcoded mappings in DeviceListRow/DeviceFormModal — Plan 04 provides Tauri lookup command to replace"

requirements-completed:
  - DEV-01
  - DEV-02
  - DEV-03
  - DEV-04
  - DEV-05
  - DEV-07
  - DEV-10

duration: ~95min
completed: "2026-05-26"
---

# Phase 02 Plan 03: Devices CRUD Vertical Slice Summary

**Full-stack Devices CRUD with SQLite/audit_log backend, 6 Tauri+axum dual-transport commands, and Svelte 5 DevicesPage with form modal, list table, and destructive-confirm context menu**

## Performance

- **Duration:** ~95 min
- **Started:** (continued from previous session)
- **Completed:** 2026-05-26T12:58:49Z
- **Tasks:** 3 of 3 auto tasks complete (Task 4 is checkpoint awaiting human smoke)
- **Files modified:** 22

## Accomplishments

- DeviceService fully implemented: create/get/list/update/delete_soft with per-mutation audit_log in same SQLite transaction, optimistic-lock on update/delete (OptimisticLockMismatch on stale version), state_hints() returning 6 Russian STATE_HINTS strings
- 6 Tauri commands + 6 axum POST handlers via build_* dual-transport pattern; specta_export::builder() updated; capabilities/main.json added; main.rs Step 8 wired with tauri::Builder + plugins
- Frontend Devices feature: DevicesPage (heading + header actions + DeviceList), DeviceFormModal (10 fields, 4 required, state-hints chips, submit validation, optimistic-lock toast), DeviceContextMenu (3-dots + destructive confirm modal), DeviceListRow (8-column table row + status Badge)

## Task Commits

1. **Task 1: DTOs + Repository + DeviceService + audit_log (TDD)** - `1bf10dd` (feat — from prior session)
2. **Task 2: Tauri commands + axum router + specta + main.rs + capabilities** - `e4254ef` (feat)
3. **Task 3: Frontend Devices feature** - `f1de8b1` (feat)

## Files Created/Modified

**Backend — Domain/Infra:**
- `crates/trackly-core/src/domain/devices.rs` — DeviceNew/DevicePatch/DeviceFilter/Pagination/DeviceRow domain types
- `crates/trackly-core/src/ports/devices.rs` — DeviceRepository trait with type Conn
- `crates/trackly-infra/src/repos/devices_sqlite.rs` — Full CRUD impl, V003 column mapping (inventory_number↔inventory_no, condition↔state, complectation↔kit, notes↔specs), _in_tx helpers for transaction use, optimistic lock

**Backend — App layer:**
- `crates/trackly-app/src/dto/device.rs` — DeviceDto/DeviceNew/DevicePatch/DeviceFilter/Pagination/DeviceListResponse with serde + #[specta(type=i32)] on i64 fields
- `crates/trackly-app/src/services/device_service.rs` — DeviceService with writer.execute transactions + audit_log + spawn_blocking reads
- `crates/trackly-app/src/tauri_cmds/devices.rs` — 6 build_* helpers + thin #[tauri::command] wrappers (id/version as i32)
- `crates/trackly-app/src/http/devices.rs` — 6 axum handlers using AppErrorResponse, CreatePayload.device field
- `crates/trackly-app/src/error_axum.rs` — AppErrorResponse newtype for axum IntoResponse
- `crates/trackly-app/src/specta_export.rs` — collect_commands! extended with 6 Phase 2 commands
- `crates/trackly-app/src/main.rs` — Step 8: tauri::Builder with single-instance + dialog + manage(ctx) + invoke_handler
- `crates/trackly-app/capabilities/main.json` — core:default + dialog:default

**Tests:**
- `crates/trackly-app/tests/devices_crud.rs` — 9 integration tests GREEN: create/update/delete/list/optimistic-lock/audit_log/state_hints/filters
- `crates/trackly-app/tests/devices_http_smoke.rs` — Dual-transport equivalence (Tauri build_* + axum oneshot)
- `crates/trackly-app/tests/export_bindings.rs` — Added Device type assertions

**Frontend:**
- `ui/src/lib/api/devices.ts` — Typed wrappers for all 6 device commands
- `ui/src/lib/api/index.ts` — Export devices from api barrel
- `ui/src/features/devices/DevicesPage.svelte` — Route shell with page-header and DeviceList
- `ui/src/features/devices/DeviceList.svelte` — 8-column table + skeleton + empty-state + pagination footer
- `ui/src/features/devices/DeviceListRow.svelte` — Row render + status Badge + DeviceContextMenu trigger
- `ui/src/features/devices/DeviceFormModal.svelte` — 10-field form modal (4 required + 6 optional) + state-hints chips
- `ui/src/features/devices/DeviceContextMenu.svelte` — Kebab 3-dots menu + destructive confirm modal
- `ui/src/features/devices/api.ts` — Feature-scoped re-export
- `ui/src/routes.ts` — DevicesPlaceholder → DevicesPage

## Decisions Made

- **AppErrorResponse newtype** — orphan rule prevents `impl IntoResponse for AppError` in trackly-app (AppError lives in trackly-core, IntoResponse in axum). Solution: `AppErrorResponse(pub AppError)` in trackly-app with From impl.
- **`#[specta(type = i32)]` on i64 DTO fields** — specta-typescript 0.0.x raises BigIntForbidden for i64/u64. i32 covers SQLite IDs and Unix timestamps through 2038. JSON number type is unaffected at runtime.
- **Tauri command params i32 not i64** — i64 function params also trigger BigIntForbidden. Changed to i32 with `as i64` cast in build_* delegation. No practical loss — Tauri IPC deserializes JS numbers (max ~53-bit safe integer).
- **Rename `new: DeviceNew` → `device: DeviceNew`** — `new` is a TypeScript reserved keyword; specta would emit `devicesCreate(new: DeviceNew)` which TypeScript rejects with "Identifier expected".

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] BigInt forbidden in specta-typescript export**
- **Found during:** Task 2 (export_bindings test)
- **Issue:** specta-typescript 0.0.x raises `BigIntForbidden` for i64/u64 fields in DTOs and i64 Tauri command parameters
- **Fix:** Added `#[specta(type = i32)]` / `#[specta(type = u32)]` on all BigInt fields in DeviceDto/DeviceNew/DevicePatch/DeviceFilter/Pagination/DeviceListResponse; changed Tauri command params from i64 to i32 (with `as i64` cast)
- **Files modified:** src/dto/device.rs, src/tauri_cmds/devices.rs
- **Verification:** `cargo test -p trackly-app --test export_bindings` GREEN
- **Committed in:** e4254ef (Task 2 commit)

**2. [Rule 1 - Bug] TypeScript reserved keyword 'new' as Tauri command parameter**
- **Found during:** Task 3 (pnpm svelte-check after export_bindings)
- **Issue:** specta generates `devicesCreate(new: DeviceNew)` — TypeScript parser errors: "Identifier expected. 'new' is a reserved word"
- **Fix:** Renamed Tauri command param `new: DeviceNew` → `device: DeviceNew`; updated axum CreatePayload field `new` → `device`, smoke test JSON body `{ "new": ... }` → `{ "device": ... }`, api wrapper
- **Files modified:** src/tauri_cmds/devices.rs, src/http/devices.rs, tests/devices_http_smoke.rs, ui/src/lib/api/devices.ts
- **Verification:** `pnpm svelte-check` 0 errors, `pnpm lint` clean
- **Committed in:** f1de8b1 (Task 3 commit)

---

**Total deviations:** 2 auto-fixed (2 bugs)
**Impact on plan:** Both fixes required for TypeScript compilation. No scope creep.

## Known Stubs

| Stub | File | Reason |
|------|------|--------|
| Hardcoded DEVICE_TYPES array (9 entries) | DeviceFormModal.svelte | Plan 04 wires Tauri lookup command for device types from DB |
| Hardcoded STATUSES array (4 entries) | DeviceFormModal.svelte | Plan 04 wires status lookup |
| Hardcoded TYPE_LABELS / STATUS_LABELS maps | DeviceListRow.svelte | Plan 04 lookups replace |
| location field as freetext Input (not location_id FK) | DeviceFormModal.svelte | Plan 04 provides location autocomplete; location_id=null sent to backend |

These stubs render correctly in the UI (form works, list shows labels) but use hardcoded data instead of DB-driven lookups. Users can create devices with the correct status and type via dropdown, but adding new types/statuses requires a Plan 04 update.

## Issues Encountered

- None beyond the auto-fixed BigInt and reserved-keyword issues above.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Plan 02-04 (Lookups + FTS search + autocomplete) can use `ctx.devices` DeviceService and extend both DeviceService and DeviceFormModal with real lookup commands
- All 9 integration tests GREEN; clippy clean; pnpm svelte-check 0 errors; pnpm build succeeds
- bindings.ts contains all Device types and 6 device commands
- Awaiting Task 4 manual smoke (checkpoint) to confirm pnpm tauri dev CRUD flow

## Threat Flags

No new security-relevant surfaces beyond those in the plan's threat model.

---
*Phase: 02-ui*
*Completed: 2026-05-26*

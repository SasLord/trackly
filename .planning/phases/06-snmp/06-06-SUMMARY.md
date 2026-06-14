---
phase: 06-snmp
plan: "06"
subsystem: api
tags: [specta, typescript, bindings, rust, tauri, svelte, snmp, websocket]

requires:
  - phase: 06-snmp/06-03
    provides: Tauri commands for printers/requests + WS handler
  - phase: 06-snmp/06-04
    provides: Printers UI (PrintersPage, TonerGauge, discovery)
  - phase: 06-snmp/06-05
    provides: Requests UI (RequestsPage, RequestDetail, OperationModal preFillPrinterId)

provides:
  - bindings.ts regenerated with PrinterDto (currentCartridgeId), RequestDto, Phase 6 types via specta export
  - BigIntForbidden specta errors fixed in dto/printer.rs and dto/request.rs
  - export_bindings test passing; full workspace tests green
  - Sidebar navigation already contains Принтеры + Заявки (from 06-04/06-05)

affects:
  - Phase 6 human verification checkpoint
  - Any future plan that adds i64 fields to DTOs (must use #[specta(type = i32)])

tech-stack:
  added: []
  patterns:
    - "specta i64 convention: all i64/u64 DTO fields require #[specta(type = i32)] or #[specta(type = u32)]"
    - "WsEvent types live in bindings-phase6.ts (not gitignored bindings.ts); specta-first for Tauri commands"

key-files:
  created: []
  modified:
    - crates/trackly-app/src/dto/printer.rs
    - crates/trackly-app/src/dto/request.rs

key-decisions:
  - "specta BigIntForbidden fix: all i64 fields in DTOs must carry #[specta(type = i32)] per project convention"
  - "WsEvent remains in bindings-phase6.ts (manually maintained) — it is a WS message type, not a Tauri command return type"
  - "Pagination duplicate export in bindings.ts is expected specta behavior (each DTO module defines its own Pagination)"

requirements-completed:
  - PRN-01
  - PRN-02
  - PRN-03
  - PRN-04
  - PRN-05
  - PRN-06
  - PRN-07
  - PRN-08
  - REQ-01
  - REQ-02
  - REQ-03
  - REQ-04
  - REQ-05
  - REQ-07

duration: 11min
completed: 2026-06-14
---

# Phase 06 Plan 06: Final Bindings Verification + Compile Gate Summary

**specta export fixed (BigIntForbidden errors removed), bindings.ts regenerated with Phase 6 PrinterDto/RequestDto types, full workspace cargo test green**

## Performance

- **Duration:** ~11 min
- **Started:** 2026-06-14T23:42:00Z
- **Completed:** 2026-06-14T23:53:10Z
- **Tasks:** 2 auto tasks executed (checkpoint task 3 remains for human verification)
- **Files modified:** 2

## Accomplishments

- Fixed `BigIntForbidden` specta errors across `dto/printer.rs` and `dto/request.rs` — 8 fields corrected to use `#[specta(type = i32)]` / `#[specta(type = Option<i32>)]`
- `cargo test -p trackly-app --test export_bindings` passes — `bindings.ts` now generated with `PrinterDto` (including `currentCartridgeId`), `RequestDto`, all Phase 6 Tauri command types
- `cargo check --workspace` exits 0, no errors, no warnings
- All workspace tests green: 36 core tests + 12+ infra tests + 14 phase06 integration tests (including `test_ws_unauth_401` and `test_snmp_mock_switch`)
- `pnpm svelte-check` — 0 errors (33 warnings, acceptable)
- Sidebar navigation already has «Принтеры» and «Заявки» links (verified in `sidebar-config.ts`)

## Task Commits

1. **Task 1: Финальная верификация bindings.ts + nav ссылки** — `7dd8665` (fix: specta BigIntForbidden fixes)
2. **Task 2: Финальная компиляция cargo check + тест мок-поллера** — no changes needed; compile/tests were green after Task 1

## Files Created/Modified

- `crates/trackly-app/src/dto/printer.rs` — fixed `#[specta(type = Option<i32>)]` for `last_seen_utc`, `usb_host_device_id`, `page_count`; added `#[specta(type = i32)]` for `PrinterListResponse.total`
- `crates/trackly-app/src/dto/request.rs` — changed `created_at_utc`, `updated_at_utc` from `i64` to `i32`; `deleted_at_utc` from `Option<i64>` to `Option<i32>`; added `#[specta(type = i32)]` to all `RequestCountsDto` fields and `RequestListResponse.total`

## Decisions Made

- `WsEvent` type is not registered in `specta_export.rs` (it's a WS broadcast, not a Tauri command return type) — it correctly lives in `bindings-phase6.ts` which is checked into git; this separation is intentional
- Duplicate `Pagination` type exports in `bindings.ts` are specta behavior (each crate module with its own `Pagination` struct exports it independently); TypeScript handles this via the `// @ts-nocheck` header on the generated file

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed BigIntForbidden errors in PrinterDto**
- **Found during:** Task 1 (export_bindings verification)
- **Issue:** `last_seen_utc`, `usb_host_device_id` had `#[specta(type = Option<i64>)]` — specta-typescript 0.0.9 forbids i64 (BigIntForbidden); `page_count: Option<i64>` had no specta annotation
- **Fix:** Changed to `Option<i32>` annotations, added missing annotation for `page_count`
- **Files modified:** `crates/trackly-app/src/dto/printer.rs`
- **Verification:** `cargo test -p trackly-app --test export_bindings` passes
- **Committed in:** `7dd8665`

**2. [Rule 1 - Bug] Fixed BigIntForbidden errors in RequestDto and RequestCountsDto**
- **Found during:** Task 1 (iterative specta test runs)
- **Issue:** `created_at_utc`, `updated_at_utc` had `#[specta(type = i64)]`; `deleted_at_utc` had `#[specta(type = Option<i64>)]`; all `RequestCountsDto` fields (`all`, `open`, `in_progress`, `completed`, `rejected`) and `RequestListResponse.total` had no specta annotations
- **Fix:** Changed all to `i32`/`Option<i32>` per project convention
- **Files modified:** `crates/trackly-app/src/dto/request.rs`
- **Verification:** `cargo test -p trackly-app --test export_bindings` passes
- **Committed in:** `7dd8665`

---

**Total deviations:** 2 auto-fixed (both Rule 1 — bugs in specta annotations)
**Impact on plan:** Essential fixes for TypeScript binding generation. No scope creep. All fixes follow the existing project convention (`#[specta(type = i32)]` for all `i64` fields).

## Issues Encountered

None beyond the known failing test documented in the plan prompt (BigIntForbidden errors), which were the primary task to fix.

## Known Stubs

None — this plan is a verification/compile plan; no UI stubs were introduced.

## Threat Flags

None — no new network endpoints, auth paths, file access patterns, or schema changes introduced in this plan.

## Next Phase Readiness

Phase 6 is now fully compiled and tested. Awaiting human checkpoint verification:
- TRACKLY_SNMP_MOCK=1 smoke test (PrintersPage with fixture printers)
- RequestsPage: create request → WS toast → specialist sees it
- OperationModal preFillPrinterId from request

---
*Phase: 06-snmp*
*Completed: 2026-06-14*

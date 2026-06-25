---
phase: 12-cartridge-request-interconnection
plan: 21
subsystem: api
tags: [tauri, axum, svelte5, rusqlite, rbac]

requires:
  - phase: 12-cartridge-request-interconnection
    provides: "Plan 12-20: optional printer selection in cartridge-centric install (D-20/D-21/D-22), PrinterSelect component, previousCartridge block"
provides:
  - "printers_get_by_device_id dual-transport read command (Tauri + axum), resolving by printers.device_id instead of printers.id"
  - "PrinterRepository::get_by_device_id port method + SqlitePrinterRepository impl + PrinterService::get_by_device_id enrichment"
  - "OperationModal.svelte printer lookup effect now resolves printerContext correctly in BOTH install entries (cartridge-centric and request-centric)"
  - "DEC-A: printerContextHint omits printer name when PrinterSelect is visible (cartridge-centric entry); keeps name+IP in request-centric entry"
  - "DEC-B: Расположение auto-fills from printerContext.deviceLocation in the cartridge-centric entry, without overwriting manual operator input"
affects: [cartridges, printers, requests]

tech-stack:
  added: []
  patterns:
    - "Dual-transport read-by-alternate-key: get_by_device_id mirrors get()'s SQL/enrichment 1:1, differing only in the WHERE-clause resolve key — same pattern as printers_get_compatible_models"

key-files:
  created: []
  modified:
    - crates/trackly-core/src/ports/printers.rs
    - crates/trackly-infra/src/repos/printers_sqlite.rs
    - crates/trackly-app/src/services/printer_service.rs
    - crates/trackly-app/src/tauri_cmds/printers.rs
    - crates/trackly-app/src/http/printers.rs
    - crates/trackly-app/src/specta_export.rs
    - crates/trackly-app/tests/role_endpoint_matrix.rs
    - ui/src/features/printers/api.ts
    - ui/src/features/cartridges/OperationModal.svelte

key-decisions:
  - "GAP-12-13 root cause: effectivePrinterId is always a device_id (PrinterSelect emits deviceId; preFillPrinterId is request.printerDeviceId), but printers_get resolves WHERE p.id = ?1 — added a parallel get_by_device_id command instead of changing printers_get's contract (it's used elsewhere, e.g. PrinterDetail, keyed by printers.id)"
  - "DEC-A: hint text branches on isSelectorVisible (op==='install' && cartridge!==null && preFillPrinterId===undefined) — same predicate that gates the PrinterSelect markup, so hint and selector visibility never drift apart"
  - "DEC-B: location auto-fill only fires when preFillPrinterId is undefined (cartridge-centric entry) AND location is still empty — never overwrites manual operator input or the request-centric prefillLocation flow"

requirements-completed: [GAP-12-13, DEC-A, DEC-B]

duration: 35min
completed: 2026-06-25
---

# Phase 12 Plan 21: Round 5 Gap Closure — printer device_id lookup fix Summary

**Fixed GAP-12-13 by adding `printers_get_by_device_id` (resolves by `printers.device_id`, not `printers.id`), which was the actual root cause behind the "Предыдущий картридж" block never rendering in either install entry — plus DEC-A hint branching and DEC-B location auto-fill on top of the corrected lookup.**

## Performance

- **Duration:** ~35 min
- **Tasks:** 2
- **Files modified:** 9

## Accomplishments

- New dual-transport read command `printers_get_by_device_id`: port trait method + SQLite repo impl (`WHERE p.device_id = ?1`) + service enrichment (mirrors `get()` 1:1) + Tauri command + axum route + specta registration + RBAC matrix Case 40 (Employee → 403)
- `printers_get` (resolving `WHERE p.id = ?1`) left completely untouched — verified both `printers_sqlite.rs` occurrences of `WHERE p.id = ?1` (in `fetch_in_tx` and `get()`) are intact
- `OperationModal.svelte`'s printer lookup effect switched from `printers.get(effectivePrinterId)` to `printers.getByDeviceId(effectivePrinterId)` — `printerContext` now resolves to a real printer in both the cartridge-centric (PrinterSelect) and request-centric (preFillPrinterId) install entries, unblocking the previously-unreachable "Предыдущий картридж" block (GAP-12-11/D-16, originally closed in Round 4 but never actually triggerable due to GAP-12-13)
- DEC-A implemented via new `isSelectorVisible` derived value, reusing the exact predicate that gates the `PrinterSelect` markup — hint shows `#id (IP)` when the selector is visible, full `name (IP)` otherwise
- DEC-B implemented inside the lookup effect's `.then()` callback — auto-fills `location` from `printer.deviceLocation` only when `preFillPrinterId === undefined` (cartridge-centric entry) and `location` is still empty

## Task Commits

Each task was committed atomically:

1. **Task 1: Backend — printers_get_by_device_id read command (GAP-12-13)** - `4086f7c` (feat)
2. **Task 2: Frontend — OperationModal.svelte lookup fix + DEC-A/DEC-B** - `9aa478a` (fix)

_Note: Task 1 is "feat" (new command), Task 2 is "fix" (corrects a structural bug in the existing lookup effect)._

## Files Created/Modified

- `crates/trackly-core/src/ports/printers.rs` - Added `get_by_device_id` to the `PrinterRepository` trait
- `crates/trackly-infra/src/repos/printers_sqlite.rs` - `SqlitePrinterRepository::get_by_device_id` impl, reuses `SELECT_PRINTERS` + `map_row_printer`, `WHERE p.device_id = ?1`
- `crates/trackly-app/src/services/printer_service.rs` - `PrinterService::get_by_device_id`, mirrors `get()`'s enrichment body (last reading, active alerts, current cartridge)
- `crates/trackly-app/src/tauri_cmds/printers.rs` - `build_printers_get_by_device_id` helper (gated `Action::ReadData`) + `printers_get_by_device_id` Tauri wrapper
- `crates/trackly-app/src/http/printers.rs` - `GetByDeviceIdPayload`, `handler_get_by_device_id`, route `/api/v1/printers_get_by_device_id`
- `crates/trackly-app/src/specta_export.rs` - Registered `printers_get_by_device_id` for TS binding generation
- `crates/trackly-app/tests/role_endpoint_matrix.rs` - Case 40: Employee → `printers_get_by_device_id` → 403 Forbidden
- `ui/src/features/printers/api.ts` - `printers.getByDeviceId(deviceId)` wrapper
- `ui/src/features/cartridges/OperationModal.svelte` - Lookup effect uses `getByDeviceId`; `isSelectorVisible` derived; `printerContextHint` branches on it (DEC-A); location auto-fill in the lookup effect's `.then()` (DEC-B)

## Decisions Made

- **GAP-12-13 fix shape:** added a parallel `get_by_device_id` command rather than changing `printers_get`'s resolve key, because `printers_get` is consumed elsewhere (PrinterDetail) under the `printers.id` contract — changing it would have been a breaking change disguised as a bug fix.
- **DEC-A predicate reuse:** `isSelectorVisible` uses the identical boolean expression that already gates the `PrinterSelect` `{#if}` block in the markup (`op === 'install' && cartridge !== null && preFillPrinterId === undefined`), so the hint text and the selector's visibility can never disagree even if either is edited independently in the future.
- **DEC-B placement:** the auto-fill check lives inside the existing lookup effect's `.then()` callback (no new effect, no new API call) — it only needs the already-fetched `printer` DTO and the existing `location` state.

## Deviations from Plan

None - plan executed exactly as written. Both tasks matched their `<action>` specs; all `must_haves.artifacts` and `must_haves.key_links` patterns are present in the final code (verified via grep).

Two benign, non-actionable discrepancies between the plan's estimated grep counts and the actual literal-string occurrences were noted during execution (not deviations, just estimation drift in the plan's acceptance-criteria grep commands):
- `grep -c "printers_get_by_device_id" crates/trackly-app/src/http/printers.rs` returns 3, not the plan's estimated ≥4 — the payload struct (`GetByDeviceIdPayload`) and handler fn (`handler_get_by_device_id`) don't contain the exact literal substring `printers_get_by_device_id`, but all 4 conceptual elements (payload, handler, import, route) are present and verified.
- `grep -c "printers\.get(" ui/src/features/cartridges/OperationModal.svelte` returns 1, not 0 — the sole remaining match is inside an explanatory code comment describing the GAP-12-13 root cause (`printers.get() resolves by printers.id, so it...`), not an actual call. No `printers.get(` call remains in executable code.

## Issues Encountered

None. `cargo build --workspace` passed on the first attempt with no errors/warnings. `TRACKLY_AD_MOCK=1 cargo test -p trackly-app --test role_endpoint_matrix` passed on the first attempt (all 40 cases, including the new Case 40). `cargo test -p trackly-app --test export_bindings` regenerated `ui/src/bindings.ts` with `printersGetByDeviceId` present. `pnpm exec svelte-check` reported 0 errors (36 pre-existing warnings in unrelated files, out of scope). `pnpm build` succeeded.

## Threat Flags

None — this plan's `<threat_model>` (T-12-21-01 through T-12-21-05) fully covers the new surface (`printers_get_by_device_id`, same RBAC gate class as `printers_get`/`printers_get_compatible_models`, `AppError::NotFound` on missing device_id, no new dependencies). No additional surface was introduced beyond what's documented there.

## Known Stubs

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

GAP-12-13, DEC-A, and DEC-B are closed. The cartridge-request interconnection feature's Round 5 UAT findings (`12-HUMAN-UAT.md` `open_round5`) are now fully addressed:
- `printerContext` resolves correctly in both install entries (no more silent `null`)
- The "Предыдущий картридж" block (D-16/D-22) is now actually reachable in both entries, not just theoretically wired
- The printer hint text is contextually appropriate per entry (no redundant name display when the selector already shows it)
- Расположение auto-fills sensibly in the cartridge-centric entry without fighting manual operator edits

Recommend a final UAT pass (manual or live) specifically re-testing R4-1/R4-3 scenarios from `12-HUMAN-UAT.md` to confirm the fix resolves the originally observed symptoms end-to-end, then mark Phase 12 complete.

---
*Phase: 12-cartridge-request-interconnection*
*Completed: 2026-06-25*

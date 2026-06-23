---
phase: 12-cartridge-request-interconnection
plan: 05
subsystem: api
tags: [rusqlite, axum, tauri, sqlite, rbac, refinery]

# Dependency graph
requires:
  - phase: 12-cartridge-request-interconnection
    provides: "Cartridge-request interconnection base (install picker, request→install flow) and Plan 12-04 (person autocomplete)"
provides:
  - "printer_cartridge_models junction table (device_id ↔ cartridge_model_id by FK, distinct from free-text cartridge_model_compatibility)"
  - "PrinterRepository.get_compatible_model_ids / get_compatible_device_ids + SqlitePrinterRepository set_compatible_models_in_tx / set_compatible_devices_in_tx"
  - "CartridgeFilter.compatible_with_printer_device_id — narrows cartridge list to compatible models when links exist, pass-through when unconfigured (D-13/D-14)"
  - "Four dual-transport commands: printers_get/set_compatible_models, cartridge_models_get/set_compatible_devices, editable from both printer and model sides (D-12)"
affects: [12-06, 12-07, 12-08, 12-09]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "DELETE+re-INSERT inside a single transaction for link-table replace semantics (mirrors existing upsert_compatibility_in_tx pattern for the free-text table)"
    - "D-13/D-14 single-predicate SQL: AND (?N IS NULL OR NOT EXISTS(...) OR model_id IN (...)) encodes both 'narrow when configured' and 'pass-through when not configured' without an application-layer branch"
    - "Service-level inline authorize() for setter methods (set_compatible_models/set_compatible_devices) instead of double-gating in both build_* helper and service — getters still gate in build_* since the corresponding service getters take no caller param"

key-files:
  created:
    - migrations/V029__printer_cartridge_models.sql
  modified:
    - crates/trackly-core/src/domain/cartridges.rs
    - crates/trackly-core/src/ports/printers.rs
    - crates/trackly-app/src/dto/printer.rs
    - crates/trackly-app/src/dto/cartridge.rs
    - crates/trackly-infra/src/repos/printers_sqlite.rs
    - crates/trackly-infra/src/repos/cartridges_sqlite.rs
    - crates/trackly-app/src/services/printer_service.rs
    - crates/trackly-app/src/services/cartridge_service.rs
    - crates/trackly-app/src/tauri_cmds/printers.rs
    - crates/trackly-app/src/tauri_cmds/cartridges.rs
    - crates/trackly-app/src/http/printers.rs
    - crates/trackly-app/src/http/cartridges.rs
    - crates/trackly-app/src/specta_export.rs
    - crates/trackly-app/tests/cartridges_crud.rs
    - crates/trackly-app/tests/role_endpoint_matrix.rs

key-decisions:
  - "CartridgeService gained a printer_repo: Arc<SqlitePrinterRepository> field (constructed internally as Arc::new(SqlitePrinterRepository)) rather than changing CartridgeService::new()'s signature, since that constructor is called from 11 call sites across tests and src/"
  - "Printer-side compatibility methods live on PrinterService, model-side mirror lives on CartridgeService — both write into the SAME printer_cartridge_models table from opposite directions (D-12)"
  - "Getter build_* helpers (build_printers_get_compatible_models, build_cartridge_models_get_compatible_devices) call authorize(caller, &Action::ReadData) directly; setter build_* helpers rely on the service method's own inline authorize() call instead of double-gating"

requirements-completed: [D-11, D-12, D-13, D-14, D-15, D-15a]

# Metrics
duration: 25min
completed: 2026-06-23
---

# Phase 12 Plan 05: Printer↔Cartridge-Model Compatibility (Backend) Summary

**New `printer_cartridge_models` junction table with dual-transport CRUD commands editable from both printer and cartridge-model cards, plus a `CartridgeFilter.compatible_with_printer_device_id` predicate that narrows the install picker to compatible models only when links are configured (pass-through otherwise).**

## Performance

- **Duration:** ~25 min (commit span; TDD RED→GREEN cycle for Task 2)
- **Started:** 2026-06-23T06:59:21+07:00 (Task 1 commit)
- **Completed:** 2026-06-23T07:17:28+07:00 (Task 3 commit)
- **Tasks:** 3/3 completed
- **Files modified:** 16 (1 created, 15 modified)

## Accomplishments
- New `printer_cartridge_models(device_id, cartridge_model_id)` FK↔FK junction table (V029), explicitly distinct from the existing free-text `cartridge_model_compatibility` (V005) — never touched it.
- `PrinterRepository` trait extended with `get_compatible_model_ids`/`get_compatible_device_ids`; `SqlitePrinterRepository` implements both plus transactional `set_compatible_models_in_tx`/`set_compatible_devices_in_tx` (DELETE+re-INSERT replace semantics).
- `CartridgeFilter` (domain + DTO) gained `compatible_with_printer_device_id: Option<i64>`; `SqliteCartridgeRepository::list()` joins against the junction table with a single SQL predicate that narrows when links exist and passes through unfiltered when the printer has no configured links (D-13/D-14).
- Four new dual-transport commands wired through Tauri + axum + `specta_export.rs`: `printers_get_compatible_models`, `printers_set_compatible_models` (PrinterService), `cartridge_models_get_compatible_devices`, `cartridge_models_set_compatible_devices` (CartridgeService) — link is editable from either side, same underlying table.
- RBAC: reads gated by `Action::ReadData`, writes by `Action::MutatePrinters`/`Action::MutateCartridges` — all Admin|Manager only (management feature, not employee-facing). Three new RBAC matrix cases (33-35) confirm Employee gets 403 on all three gated paths.
- Writes are audited: `audit_repo.insert(entity_type: "printer_compatibility", action: "set_compatible_models"/"set_compatible_devices", ...)` inside the same transaction as the link replace.

## Task Commits

Each task was committed atomically:

1. **Task 1: Migration V029 + domain/DTO contracts** - `9888404` (feat)
2. **Task 2: Repo impls — compatibility CRUD + cartridge list filter join** - `1707ad0` (test, RED) → `3f0efa7` (feat, GREEN)
3. **Task 3: Service methods + dual-transport commands + RBAC tests** - `ca8fb08` (feat)

**Plan metadata:** _(this commit — recorded below in Final Commit section)_

_Note: Task 2 is `tdd="true"` — committed as a RED/GREEN pair per the plan's TDD execution protocol._

## TDD Gate Compliance

Task 2 (`tdd="true"`) followed the full RED→GREEN cycle:
- RED: `1707ad0` `test(12-05): add failing tests for printer-cartridge compatibility` — 3 new tests added to `cartridges_crud.rs` (`printer_compatib_list_narrows_to_linked_model`, `printer_compatib_unconfigured_device_does_not_narrow`, `printer_compatib_round_trip_both_directions`), confirmed failing before implementation.
- GREEN: `3f0efa7` `feat(12-05): implement printer-cartridge-model link CRUD + filter narrowing` — repo implementations added, all 9 tests in `cartridges_crud.rs` (6 pre-existing + 3 new) pass.

Task 1 (`tdd="true"`) is schema/contract scaffolding (migration + trait signatures with no impl) — the plan explicitly designates Task 1+2 as a single RED→GREEN pair spanning two tasks, since trait methods without implementations cannot compile-test in isolation. No gate violation.

Task 3 (`type="auto"`, not `tdd="true"`) wires existing tested repo methods through service/command/HTTP layers — no new behavior requiring a RED phase; verified via the pre-existing RBAC matrix pattern (3 new cases, all passing on first run).

## Files Created/Modified
- `migrations/V029__printer_cartridge_models.sql` - New junction table + unique index (device_id, cartridge_model_id) + reverse-lookup index on cartridge_model_id
- `crates/trackly-core/src/domain/cartridges.rs` - `CartridgeFilter.compatible_with_printer_device_id: Option<i64>`
- `crates/trackly-core/src/ports/printers.rs` - `PrinterRepository::get_compatible_model_ids` / `get_compatible_device_ids` trait methods
- `crates/trackly-app/src/dto/printer.rs` - `PrinterCompatibleModelsDto { device_id, model_ids }`
- `crates/trackly-app/src/dto/cartridge.rs` - `CartridgeFilter.compatible_with_printer_device_id` mirrored into DTO + `into_domain()`; `CartridgeModelCompatibleDevicesDto { model_id, device_ids }`
- `crates/trackly-infra/src/repos/printers_sqlite.rs` - `get_compatible_model_ids`/`get_compatible_device_ids` impls + `set_compatible_models_in_tx`/`set_compatible_devices_in_tx` static methods (DELETE+re-INSERT)
- `crates/trackly-infra/src/repos/cartridges_sqlite.rs` - `list()` COUNT + SELECT queries gain the D-13/D-14 narrowing predicate, params shifted accordingly
- `crates/trackly-app/src/services/printer_service.rs` - `get_compatible_models`/`set_compatible_models` (writer.execute + tx + audit insert)
- `crates/trackly-app/src/services/cartridge_service.rs` - `printer_repo: Arc<SqlitePrinterRepository>` field; `get_compatible_devices`/`set_compatible_devices` mirror methods
- `crates/trackly-app/src/tauri_cmds/printers.rs` - `build_printers_get/set_compatible_models` helpers + thin `#[tauri::command]` wrappers
- `crates/trackly-app/src/tauri_cmds/cartridges.rs` - `build_cartridge_models_get/set_compatible_devices` helpers + thin `#[tauri::command]` wrappers
- `crates/trackly-app/src/http/printers.rs` - `handler_get/set_compatible_models` axum handlers + 2 routes
- `crates/trackly-app/src/http/cartridges.rs` - `handler_models_get/set_compatible_devices` axum handlers + 2 routes
- `crates/trackly-app/src/specta_export.rs` - 4 new commands registered in `collect_commands![...]`
- `crates/trackly-app/tests/cartridges_crud.rs` - 3 new tests (narrowing, pass-through, round-trip both directions)
- `crates/trackly-app/tests/role_endpoint_matrix.rs` - Cases 33-35 (Employee 403 on all 3 gated compatibility paths)

## Decisions Made
- `CartridgeService` gained an internally-constructed `printer_repo: Arc<SqlitePrinterRepository>` field rather than threading it through `CartridgeService::new()`'s parameter list — that constructor has 11 call sites across the test suite and `src/`, and `SqlitePrinterRepository` is a zero-field unit struct, so `Arc::new(SqlitePrinterRepository)` inside `new()` adds the capability with zero call-site churn.
- Setter service methods (`PrinterService::set_compatible_models`, `CartridgeService::set_compatible_devices`) call `authorize()` inline themselves; the corresponding `build_*` Tauri/HTTP helpers do NOT redundantly re-gate. Getter service methods take no `caller` param at all (read-only, no audit trail needed), so their `build_*` helpers are the sole gate point. This mirrors an existing inconsistency already present in the codebase (e.g. `PrinterService::acknowledge_alert` self-gates) rather than introducing a new pattern.
- D-13/D-14 implemented as a single SQL predicate (`?N IS NULL OR NOT EXISTS(...) OR model_id IN (...)`) rather than an application-layer branch — keeps "narrow when configured, pass-through when not" as one indexed query with no extra round-trip.

## Deviations from Plan

None - plan executed exactly as written. All three tasks, the TDD RED/GREEN cycle for Task 2, and all four acceptance-criteria grep checks for Task 3 matched the plan's expectations without requiring auto-fixes, architectural changes, or scope adjustments.

`cargo fmt --all` was run before each commit; it touched two unrelated pre-existing files outside the plan's `files_modified` list (`tests/request_printer_options.rs`, `tests/ws_http_single_broadcast.rs`) due to pre-existing formatting drift unconnected to this plan's changes — those were reverted via `git checkout --` before staging, per the scope-boundary rule (out-of-scope, not fixed).

## Issues Encountered
None.

## User Setup Required

None - no external service configuration required. This plan is backend-only; the new commands are not yet consumed by any UI (frontend wiring is Plan 12-07/12-08 per the gap-closure roadmap).

## Next Phase Readiness
- Backend compatibility CRUD + filter narrowing is complete and tested; ready for Plan 12-07 (printer card UI) and 12-08 (cartridge-model card UI) to consume `printers_get/set_compatible_models` and `cartridge_models_get/set_compatible_devices`.
- The install-picker integration (consuming `CartridgeFilter.compatible_with_printer_device_id` from the request-complete flow) is the manual smoke test deferred to after 12-07 lands, per this plan's `<verification>` section — not yet exercised end-to-end through the UI.
- No blockers.

---
*Phase: 12-cartridge-request-interconnection*
*Completed: 2026-06-23*

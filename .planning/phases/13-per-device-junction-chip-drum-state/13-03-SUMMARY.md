---
phase: 13-per-device-junction-chip-drum-state
plan: 03
subsystem: api
tags: [rust, tauri, axum, specta, rbac, sqlite]

# Dependency graph
requires:
  - phase: 13-per-device-junction-chip-drum-state
    provides: "13-01: cartridges_sqlite.rs compatible_model_aggregates() + domain CompatibleModelAggregate; 13-02: V032 migration + deletion of all 4 V029 junction commands (cartridge_models_get/set_compatible_devices, printers_get/set_compatible_models)"
provides:
  - "printers_get_compatible_aggregates (Tauri command + axum HTTP route) — read-only R4 replacement for the deleted per-device junction commands"
  - "CompatibleModelAggregateDto / PrinterCompatibleAggregatesDto DTOs in dto/printer.rs"
  - "CartridgeService::compatible_aggregates_for_printer service method"
  - "role_endpoint_matrix.rs Case 41 — RBAC coverage for the new endpoint"
affects: [13-06, 13-07]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "R4 aggregate read endpoint follows the existing S-1/S-2 dual-transport pattern (build_* helper shared by Tauri wrapper + axum handler), same authorize(&Action::ReadData) gate as printers_get/printers_get_by_device_id"

key-files:
  created: []
  modified:
    - crates/trackly-app/src/dto/printer.rs
    - crates/trackly-app/src/services/cartridge_service.rs
    - crates/trackly-app/src/tauri_cmds/printers.rs
    - crates/trackly-app/src/http/printers.rs
    - crates/trackly-app/src/specta_export.rs
    - crates/trackly-app/tests/role_endpoint_matrix.rs
    - ui/src/bindings.ts (auto-regenerated, gitignored)

key-decisions:
  - "Task 1 (deletion of the 4 V029 junction commands) was already fully completed in Plan 13-02 as a Rule 3 blocking-issue pull-forward — verified via grep before starting, no remaining references found in tauri_cmds/, http/, or specta_export.rs"
  - "compatible_aggregates_for_printer lives on CartridgeService (not PrinterService) because the underlying query (compatible_model_aggregates) lives in cartridges_sqlite.rs from Plan 13-01; printers.rs's build_* helper calls through ctx.cartridges"
  - "D-07 pass-through semantics NOT applied here — a model with zero compatibility rows for the given printer is simply absent from the aggregates array, not included with zero counts"

requirements-completed: [SPEC-13-R1, SPEC-13-R4]

# Metrics
duration: 25min
completed: 2026-06-26
---

# Phase 13 Plan 03: Printer Compatible-Models Aggregate Read Endpoint Summary

**Added `printers_get_compatible_aggregates` (R4) — a read-only aggregate-by-status endpoint over both Tauri and axum transports, replacing the deleted V029 per-device junction commands; Task 1's deletion scope was already completed by Plan 13-02.**

## Performance

- **Duration:** ~25 min
- **Started:** 2026-06-26T00:00:00Z
- **Completed:** 2026-06-26T00:24:11Z
- **Tasks:** 1 of 2 plan tasks required new work (Task 1 deletion already done by 13-02)
- **Files modified:** 6 (+ 1 auto-regenerated, gitignored)

## Accomplishments
- New `printers_get_compatible_aggregates` command available via both Tauri invoke and `POST /api/v1/printers_get_compatible_aggregates`
- `CompatibleModelAggregateDto` / `PrinterCompatibleAggregatesDto` DTOs added to `dto/printer.rs`, mirroring `trackly_core::domain::cartridges::CompatibleModelAggregate`
- `CartridgeService::compatible_aggregates_for_printer` wires the new command to the pre-existing `compatible_model_aggregates()` repo method from Plan 13-01
- Same `authorize(&Action::ReadData)` gate as `printers_get`/`printers_get_by_device_id` (Admin/Manager only, Employee → 403)
- Registered in `specta_export.rs`; `ui/src/bindings.ts` regenerated with the new command + types
- `role_endpoint_matrix.rs` Case 41 added: Employee session → 403 Forbidden

## Task Commits

Plan 13-03 had two tasks defined; Task 1 (deletion) was already executed by Plan 13-02 (see Deviations below). Only Task 2 required new work in this plan:

1. **Task 1: Delete V029 per-device junction commands** — already completed in Plan 13-02, commit `<see 13-02-SUMMARY.md>` (no new commit needed in this plan)
2. **Task 2: Add `printers_get_compatible_aggregates` (R4) read command** - `199438c` (feat)

**Plan metadata:** _(this commit, made immediately following this summary)_

## Files Created/Modified
- `crates/trackly-app/src/dto/printer.rs` - `CompatibleModelAggregateDto`, `PrinterCompatibleAggregatesDto`, and a `From` impl mapping the domain aggregate to the DTO
- `crates/trackly-app/src/services/cartridge_service.rs` - `compatible_aggregates_for_printer(printer_device_id)` async method, `spawn_blocking` over the reader pool
- `crates/trackly-app/src/tauri_cmds/printers.rs` - `build_printers_get_compatible_aggregates` helper + `#[tauri::command]` wrapper `printers_get_compatible_aggregates`
- `crates/trackly-app/src/http/printers.rs` - `GetCompatibleAggregatesPayload`, `handler_get_compatible_aggregates`, route registration
- `crates/trackly-app/src/specta_export.rs` - registered `printers_get_compatible_aggregates` in the `collect_commands!` macro
- `crates/trackly-app/tests/role_endpoint_matrix.rs` - Case 41 (Employee → 403) + updated header doc comment
- `ui/src/bindings.ts` - auto-regenerated via `cargo test export_bindings` (gitignored, not committed)

## Decisions Made
- Task 1's deletion scope (4 V029 junction commands: `cartridge_models_get_compatible_devices`, `cartridge_models_set_compatible_devices`, `printers_get_compatible_models`, `printers_set_compatible_models`) was already fully removed by Plan 13-02 as a Rule 3 (blocking-issue) pull-forward. Verified via grep across `tauri_cmds/`, `http/`, `specta_export.rs`, and `role_endpoint_matrix.rs` before starting — zero remaining references (only historical doc-comment mentions). No re-deletion work was performed; this plan proceeded directly to Task 2.
- `compatible_aggregates_for_printer` placed on `CartridgeService` rather than `PrinterService`, since the underlying SQL lives in `cartridges_sqlite.rs` (owned by the cartridges repo from Plan 13-01). `printers.rs`'s `build_*` helper calls through `ctx.cartridges` rather than duplicating query logic in the printers domain.
- No pass-through (D-07): a cartridge model with zero compatibility rows for the requested printer is simply absent from the `models` array — not included with zero counts. Admin/Manager with no compatible models still get a 200 with `models: []` (not an error), per the plan's `<behavior>` block.
- Статус «Списано» (Written off) is excluded from `inStock`/`atRefill`/`inUse` counts at the repo layer (pre-existing from Plan 13-01, confirmed unchanged).

## Deviations from Plan

### Plan Adjustment (not a Rule 1-4 deviation — pre-flagged by prior plan)

**1. Task 1 skipped — already completed in Plan 13-02**
- **Found during:** Pre-execution verification (before Task 1)
- **Context:** Plan 13-02's `prior_wave_context` and its own SUMMARY.md explicitly documented that it pulled forward the entire V029 deletion half of this plan's Task 1 as a Rule 3 blocking-issue fix, because the V032 migration (collapsing `printer_brand`+`printer_model` into `printer_name`) made the old junction commands' queries structurally invalid — they had to be removed in the same plan as the schema change to keep the build green.
- **Verification:** `grep -rn "get_compatible_devices\|set_compatible_devices\|get_compatible_models\|set_compatible_models" crates/trackly-app/src/tauri_cmds/ crates/trackly-app/src/http/ crates/trackly-app/src/specta_export.rs` returned zero matches before any work in this plan began.
- **Action:** No re-deletion performed. Proceeded directly to Task 2 (the additive R4 command) as instructed.
- **Committed in:** N/A — no commit needed for this task in this plan; the deletion commit lives in Plan 13-02's history.

### Auto-fixed Issues

**1. [Rule 3 - Blocking] cargo fmt formatting violations**
- **Found during:** Task 2 verification (pre-commit `cargo fmt --check`)
- **Issue:** Three formatting violations surfaced after writing the new code: (a) the `impl From<...> for CompatibleModelAggregateDto` declaration in `dto/printer.rs` exceeded the line-length limit and needed multi-line wrapping; (b) the `use crate::dto::printer::{...}` import block in `http/printers.rs` needed re-wrapping at a different break point; (c) a trailing blank line at the end of `tauri_cmds/printers.rs` needed removal.
- **Fix:** Ran `cargo fmt -p trackly-app`, which auto-applied all three fixes.
- **Files modified:** `crates/trackly-app/src/dto/printer.rs`, `crates/trackly-app/src/http/printers.rs`, `crates/trackly-app/src/tauri_cmds/printers.rs`
- **Verification:** Re-ran `cargo fmt --check -p trackly-app` — exit code 0, no diffs remaining.
- **Committed in:** `199438c` (Task 2 commit — fmt fixes were applied before the single commit for this task, so no separate commit was needed)

---

**Total deviations:** 1 plan adjustment (pre-flagged, no new work skipped unexpectedly) + 1 auto-fixed (1 blocking/formatting)
**Impact on plan:** No scope creep. The plan adjustment was anticipated and explicitly authorized by the prior plan's context; the fmt fix is routine tooling hygiene with zero behavioral impact.

## Issues Encountered
- `cargo test -p trackly-app` (full crate suite, run as a final sanity check beyond the plan's specified verification commands) surfaced one failing test: `blocked_user_restore_request_visible_to_admin_and_marks_pending_http` (expects 403, got 503 `service unavailable: ad`). This is a pre-existing, environment-dependent failure unrelated to this plan's scope — AD is not reachable from the macOS dev box (`ad_mode="real"`), and this same test was already known-failing per the project's documented dev-environment constraints (no AD/SNMP reachable from dev macOS). Confirmed out of scope per the deviation rules' "Scope Boundary" — not caused by this plan's changes, not present in any file this plan touched. Not fixed; not logged to deferred-items.md since it is a long-standing, already-known environment limitation rather than a new discovery.
- All plan-specified verification commands passed cleanly: `cargo build -p trackly-app`, `cargo test -p trackly-app --test role_endpoint_matrix` (includes new Case 41), `cargo clippy -p trackly-app -- -D warnings`, `cargo test export_bindings`, `cargo fmt --check -p trackly-app`.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- The R4 read-only aggregate endpoint is fully wired end-to-end (Tauri + HTTP + RBAC + bindings) and ready for frontend consumption.
- Plans 13-06/13-07 (deferred frontend work per 13-02-SUMMARY.md: `CompatibleDevicesEditor.svelte`, `CompatibleModelsEditor.svelte`, and the printer-card "Совместимые модели картриджей" widget) can now consume `printersGetCompatibleAggregates(deviceId)` from `bindings.ts` directly — no further backend work is needed for the read side.
- No blockers identified for subsequent plans in this phase.

---
*Phase: 13-per-device-junction-chip-drum-state*
*Completed: 2026-06-26*

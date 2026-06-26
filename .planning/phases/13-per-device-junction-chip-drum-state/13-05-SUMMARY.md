---
phase: 13-per-device-junction-chip-drum-state
plan: 05
subsystem: api
tags: [rust, rusqlite, axum, tauri, cartridges, printers, compatibility, autocomplete]

# Dependency graph
requires:
  - phase: 13-per-device-junction-chip-drum-state
    provides: "V032 single-column printer_name compatibility contract + CartridgeModelCreateDto.compatibility: Vec<String> + cascading transport-layer removal of V029 junction-table commands (Plans 13-01/13-02)"
provides:
  - "cartridges_crud.rs printer_compatib_* integration suite fully reconciled with V005 printer_name semantics — no remaining test exercises a removed junction-table repository method"
  - "printer_compatib_case_insensitive_match test proving D-03 (case-insensitive + TRIM) comparison semantics, closing a gap left by 13-02's rewire (which preserved narrowing/pass-through coverage but never exercised the case/whitespace-insensitive matching path)"
  - "suggest_compat_printer (service + Tauri + HTTP) sourced from the real printer roster (devices.name WHERE type_id = 2, D-06) instead of cartridge_model_compatibility's own free-text history; field: String parameter removed from all three transport layers"
affects: [13-06, 13-07, 13-08 (frontend compatibility editor rebuild — suggestCompatPrinter call sites must drop the stale field argument)]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Autocomplete sources must reflect the canonical roster (devices table), not a free-text history column that can accumulate typos/stale entries with no live backing row — applies to any future suggest_* helper backed by a free-text compatibility/tag column"

key-files:
  created: []
  modified:
    - crates/trackly-app/tests/cartridges_crud.rs
    - crates/trackly-app/src/services/cartridge_service.rs
    - crates/trackly-app/src/tauri_cmds/cartridges.rs
    - crates/trackly-app/src/http/cartridges.rs
    - .planning/phases/13-per-device-junction-chip-drum-state/deferred-items.md

key-decisions:
  - "Reconciled with Plan 13-02's prior overlapping work rather than redoing it: 13-02 had already rewired printer_compatib_list_narrows_to_linked_model and printer_compatib_unconfigured_device_does_not_narrow onto the Vec<String> contract and fixed suggest_compat_printer's live SQL column bug. This plan's actual remaining work was narrower than originally scoped: (1) replace the still-present printer_compatib_round_trip_both_directions (which still asserted forward/reverse round-trip semantics on the Vec<String> contract but never tested D-03's case-insensitive+TRIM matching) with printer_compatib_case_insensitive_match, and (2) re-point suggest_compat_printer's data source from cartridge_model_compatibility history to devices.name (D-06), dropping the now-meaningless field parameter."
  - "Test fixture for the case-insensitive match uses leading/trailing whitespace + case difference only (not interior multi-space collapsing) — confirmed against the actual SQL (LOWER(TRIM(...)) on both sides, from Plan 13-01) and D-03's literal text in 13-CONTEXT.md, which scopes normalisation to case-insensitive + TRIM, not interior-whitespace collapse. The plan's <behavior> prose used an interior-multi-space example value; verified empirically that the real implementation does not collapse interior whitespace, so the test was adjusted to assert what the system actually (and correctly, per D-03) does."
  - "Left ui/src/features/cartridges/api.ts's suggestCompatPrinter(field, prefix) and its two ModelFormModal.svelte call sites unmodified — confirmed harmless at the wire level today (axum's SuggestCompatPayload only deserializes prefix, ignoring unknown JSON keys; Tauri drops unrecognized invoke args) and explicitly in scope for the later UI-rebuild plan per the existing 13-02 deferred-items.md entry; added an addendum there documenting the additional field-parameter removal so the later plan has the full picture."

requirements-completed: [SPEC-13-R2]

# Metrics
duration: 20min
completed: 2026-06-26
---

# Phase 13 Plan 05: Printer-Compatibility Test Reconciliation + Roster-Based Autocomplete Summary

**Closed the gap Plan 13-02 left in the V029→V005 compatibility migration: replaced a still-stale round-trip test with one proving D-03's case-insensitive+TRIM matching, and re-pointed `suggest_compat_printer` from a free-text history column to the real `devices.name` printer roster (D-06), dropping its now-meaningless `field` parameter across all three transport layers.**

## Performance

- **Duration:** ~20 min
- **Started:** 2026-06-26 (continuing from Plan 13-04)
- **Completed:** 2026-06-26
- **Tasks:** 2
- **Files modified:** 5 (4 source + 1 deferred-items.md doc update)

## Accomplishments

- `cartridges_crud.rs`'s `printer_compatib_*` suite (3 tests, was already partially rewired by Plan 13-02) is now fully reconciled with V005 `printer_name` semantics — zero remaining references to the deleted V029 junction-table repository methods, confirmed by `grep`.
- New `printer_compatib_case_insensitive_match` test proves the D-03 case-insensitive + TRIM comparison path that the prior round-trip test never exercised.
- `suggest_compat_printer` (service method, Tauri command, axum handler) now suggests from the real printer roster (`SELECT DISTINCT name FROM devices WHERE type_id = 2 AND deleted_at_utc IS NULL AND name LIKE ?1 || '%'`) instead of `cartridge_model_compatibility`'s own previously-entered free-text values — D-06 satisfied.
- The obsolete `field: String` parameter (a pre-V032 two-column `printer_brand`/`printer_model` artifact) is now gone from all three layers: `CartridgeService::suggest_compat_printer`, `build_cartridges_suggest_compat_printer` + the `#[tauri::command]` wrapper, and `SuggestCompatPayload` + `handler_suggest_compat_printer` over HTTP.
- `ui/src/bindings.ts` regenerates cleanly with the new `{ prefix }`-only payload shape (confirmed via `cargo test export_bindings`); the file is gitignored so no diff lands in this commit, but the regenerated shape was inspected directly.

## Task Commits

Each task was committed atomically:

1. **Task 1: Переписать printer_compatib_* тесты на V005 printer_name семантику** - `bea37c8` (test)
2. **Task 2: suggest_compat_printer — автокомплит из devices.name (D-06)** - `7a4f7ca` (feat)

**Plan metadata:** _pending (this commit)_

## Files Created/Modified

- `crates/trackly-app/tests/cartridges_crud.rs` - replaced `printer_compatib_round_trip_both_directions` with `printer_compatib_case_insensitive_match`; updated module/helper doc comments to drop literal references to removed V029 method names (acceptance-criteria grep gate)
- `crates/trackly-app/src/services/cartridge_service.rs` - `suggest_compat_printer(prefix: String)` now queries `devices.name WHERE type_id = 2`; dropped `field` param and the column-whitelist match arm
- `crates/trackly-app/src/tauri_cmds/cartridges.rs` - `build_cartridges_suggest_compat_printer` and the `#[tauri::command] cartridges_suggest_compat_printer` wrapper both dropped `field`
- `crates/trackly-app/src/http/cartridges.rs` - `SuggestCompatPayload` dropped `field`; `handler_suggest_compat_printer` updated call site
- `.planning/phases/13-per-device-junction-chip-drum-state/deferred-items.md` - addendum documenting the additional `field`-parameter removal for the later frontend-rebuild plan to pick up

## Decisions Made

See `key-decisions` in frontmatter above — summarized: (1) reconciled with Plan 13-02's prior overlapping work instead of redoing it, narrowing this plan's actual remaining scope to the case-insensitive test + the roster-based autocomplete rewrite; (2) calibrated the new test's whitespace fixture to what D-03 actually specifies (case + TRIM, not interior-whitespace collapse) rather than the plan's illustrative prose value; (3) deferred the stale frontend `field`-argument call sites to the already-scheduled UI-rebuild plan, documenting the additional detail in `deferred-items.md`.

## Deviations from Plan

None beyond the reconciliation already anticipated by `<prior_wave_context>` — the plan's own frontmatter/tasks already accounted for Plan 13-02 having done partial overlapping work; no Rule 1-4 auto-fixes were needed beyond following the plan's explicit reconciliation instructions.

## Issues Encountered

- The plan's `<behavior>` prose for the case-insensitive test used an interior-multi-space example (`"hp   laserjet m404"`). The actual SQL implementation (`LOWER(TRIM(cmc.printer_name)) = LOWER(TRIM(d.name))`, from Plan 13-01) and D-03's literal definition in `13-CONTEXT.md` both scope normalisation to case-insensitive + TRIM (leading/trailing whitespace), not interior-whitespace collapsing. Ran the test against the real implementation first, observed it correctly fail (0 results, not 1) against an interior-multi-space fixture, and adjusted the fixture to leading/trailing whitespace + case difference only — this is the correct behavior per D-03, not a bug to fix.
- `cargo test -p trackly-app` (full suite) reproduces the single pre-existing, unrelated AD-environment failure already documented in Plan 13-02's summary (`restore_request_visibility_http.rs::blocked_user_restore_request_visible_to_admin_and_marks_pending_http`, expects `403` but gets `503` because no AD server is reachable from this macOS dev box) — confirmed out of scope, not touched by this plan's diff.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Plans 13-06/13-07/13-08 (frontend compatibility editor rebuild) can proceed against a clean, fully-V005 backend: `suggest_compat_printer` now has a stable `(prefix: String) -> Vec<String>` contract across both transports, sourced from the real printer roster.
- `ui/src/features/cartridges/api.ts`'s `suggestCompatPrinter(field, prefix)` and its two call sites in `ModelFormModal.svelte` still pass the now-removed `field` argument — harmless today (ignored by both transports) but should be simplified to `suggestCompatPrinter(prefix)` when the compatibility editor UI is rebuilt; tracked in `deferred-items.md`.
- No new blockers identified for the remaining phase 13 plans.

---
*Phase: 13-per-device-junction-chip-drum-state*
*Completed: 2026-06-26*

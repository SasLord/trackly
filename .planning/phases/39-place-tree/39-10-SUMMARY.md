---
phase: 39-place-tree
plan: 10
subsystem: database
tags: [rust, rusqlite, sqlite, recursive-cte, reports, dto, typescript]

# Dependency graph
requires:
  - phase: 39-place-tree plan 01
    provides: "places table, place_full_paths recursive-CTE view, place_id columns on devices/cartridges/acts — locations table dropped"
  - phase: 39-place-tree plan 03
    provides: "domain-layer field renames on printers.rs/requests.rs (device_place/device_place_id, printer_place) this plan's repo layer wires SQL against"
  - phase: 39-place-tree plan 04
    provides: "SqlitePlaceRepository + the canonical recursive-CTE query shapes (descendant subtree walk, ancestor storage walk) this plan's inline report-query CTEs mirror"
provides:
  - "report_service.rs — all 8 report query functions (acts/devices query+count, cartridges query x2, requests query) read place via place_full_paths instead of locations; D-28 subtree-inclusive place_id filter; D-11.2/D-11.4 is_storage ancestor-walk quick filter, independent of item status (D-11.5)"
  - "request_service.rs::printer_options, requests_sqlite.rs::SELECT_REQUESTS, printers_sqlite.rs::SELECT_PRINTERS migrated onto place_full_paths; PrinterRow gains device_place_id for PlacePicker prefill (Plan 16)"
  - "dto/printer.rs, dto/request.rs, dto/reports.rs — wire-facing DTOs renamed field-for-field to match the domain layer (device_place/device_place_id, printer_place, place, place_id, place_path, is_storage)"
  - "tauri_cmds/reports.rs::columns_for() CSV/PDF column keys renamed location_name -> place_path, matching row_field()'s renamed match arm"
  - "ui/src/bindings-phase6.ts hand-maintained TS mirror updated field-for-field (devicePlace/devicePlaceId/printerPlace/place)"
affects: [39-16 (OperationModal.svelte PlacePicker prefill via device_place_id), 39-18 (Reports UI place filter + is_storage quick filter), 39-22 (existing test-fixture cleanup)]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Per-report-function inline WITH RECURSIVE CTE text (not a shared helper) — kept the literal SQL string contiguous per function so it stays independently auditable/greppable, mirroring the plan's own verification approach (grep over the source file, not over emitted SQL)"
    - "Two independent, optionally-comma-combined recursive CTE members in one WITH RECURSIVE prefix: `subtree` (D-28 descendant walk, place_id filter) and `storage_ids` (D-11.4 ancestor walk, is_storage filter) — each added to the WITH clause only when its corresponding ReportFilter field is Some, combined with a single comma when both are present"
    - "query_requests_inner gained an explicit is_storage: Option<bool> parameter (rather than a full &ReportFilter) since its existing signature only ever destructured individual filter fields at the 4 call sites — kept consistent with that convention instead of introducing a new signature shape"

key-files:
  created: []
  modified:
    - crates/trackly-app/src/services/report_service.rs
    - crates/trackly-app/src/services/request_service.rs
    - crates/trackly-infra/src/repos/requests_sqlite.rs
    - crates/trackly-infra/src/repos/printers_sqlite.rs
    - crates/trackly-app/src/dto/printer.rs
    - crates/trackly-app/src/dto/request.rs
    - crates/trackly-app/src/dto/reports.rs
    - crates/trackly-app/src/tauri_cmds/reports.rs
    - crates/trackly-app/tests/request_printer_options.rs
    - ui/src/bindings-phase6.ts

key-decisions:
  - "Cartridge report functions (query_cartridge_audit, query_cartridge_snapshot) and query_requests_inner get the D-11.2/D-11.4 is_storage filter but NOT the D-28 place_id subtree filter — matching the pre-existing functional scope exactly (these three functions never filtered by location_id before this plan, only displayed it; Task 2's own action text confirmed this, only Tasks touching acts/devices reports add the place_id subtree filter). Their COUNT-variant siblings (count_cartridge_audit_inner, count_cartridge_snapshot, count_requests_inner) also do NOT get is_storage — this mirrors a pre-existing count/query filter-parity gap for cartridges/requests that predates this plan and is out of this plan's stated scope (Task 4 explicitly limits is_storage to 'every report query function Task 1/Task 2 already touch')."
  - "count_acts_inner/count_device_snapshot's vestigial `LEFT JOIN locations l ON ...` (never SELECTed, only present because the query was copy-pasted from its row-returning sibling) was dropped entirely rather than rewritten to `LEFT JOIN place_full_paths` — the join added no value to a COUNT(*)/COUNT(DISTINCT) query and the place_id subtree/is_storage filters reference `a.place_id`/`d.place_id` directly, not through any join alias."
  - "request_printer_options.rs's seed_printer_devices helper (previously INSERT INTO locations + devices.location_id, both physically dropped by V038) was rewritten onto INSERT INTO places (root-level 'zone' kind, arbitrary — only the resolved full_path, which equals the node's own name at root level, is asserted on) + devices.place_id. This file is explicitly listed in the plan's own files_modified (unlike the ~31 other location-vocabulary test files reserved for Plan 22) because it directly seeds via raw SQL against the now-dropped locations table — leaving it unmigrated would make every test in the file fail with 'no such table: locations', not just fail assertions."

requirements-completed: [PLC-04]

# Metrics
duration: ~140min (session interrupted once by a usage-limit reset between Task 1 and Task 2; elapsed wall-clock includes the gap, not just active work)
completed: 2026-08-23
---

# Phase 39 Plan 10: Reports/requests read-path migration onto place_full_paths Summary

**All remaining read-path SQL outside the entity-CRUD paths — 8 report query functions, the create-request printer dropdown, and the requests/printers repository SELECTs — migrated off the dropped `locations` table onto `place_full_paths`, with D-28 subtree-inclusive place filtering and a new D-11.2/D-11.4 `is_storage` geographic quick filter added to every report query this plan touches.**

## Performance

- **Duration:** ~140 min wall-clock (one usage-limit interruption between Task 1 and Task 2; work resumed from the last committed state with no rework)
- **Started:** 2026-08-23T05:01:25+07:00 (Task 1 commit)
- **Completed:** 2026-08-23T07:18:50+07:00 (last commit)
- **Tasks:** 6/6
- **Files modified:** 10

## Accomplishments

- `report_service.rs` — `query_acts_inner`/`count_acts_inner`/`query_device_snapshot`/`count_device_snapshot` (acts + devices reports) replace `LEFT JOIN locations` with `LEFT JOIN place_full_paths`; `query_cartridge_audit`/`query_cartridge_snapshot` (cartridges now have a real `place_id` FK per Plan 09) and `query_requests_inner` (printer's device place) do the same
- D-28 subtree-inclusive place filter: choosing a place in `ReportFilter.place_id` now captures that place AND every place nested under it via a `WITH RECURSIVE subtree(id)` descendant walk — not an exact `place_id` match
- New D-11.2/D-11.4 `ReportFilter.is_storage: Option<bool>` quick filter: `Some(true)`/`Some(false)` filters on ancestor-inclusive storage-place membership (a place counts as storage if it OR any ancestor has `is_storage = 1`) via a `WITH RECURSIVE storage_ids(id)` ancestor walk — a dimension kept structurally independent of item status (D-11.5), added to all 7 report query functions Tasks 1/2 touch
- `combine_printer_and_location` renamed to `combine_printer_and_place` (same behavior, printer name + place path → "Принтер, Место")
- `request_service.rs::printer_options`, `requests_sqlite.rs::SELECT_REQUESTS`, `printers_sqlite.rs::SELECT_PRINTERS` all migrated onto `place_full_paths`; the printer-options sort contract (place-less printers sort last) preserved exactly
- `PrinterRow`/`PrinterDto` gain `device_place_id: Option<i64>` (new field, not a rename) — the raw id `OperationModal.svelte` (Plan 16) needs to prefill `PlacePicker` when a printer is chosen for Install
- `dto/printer.rs`, `dto/request.rs`, `dto/reports.rs` field renames: `device_location→device_place`, `printer_location→printer_place`, `location→place` (RequestPrinterOptionDto), `location_id→place_id`/`location_name→place_path` (ReportFilter/ReportRow)
- `tauri_cmds/reports.rs::columns_for()`'s 5 `"location_name"` CSV/PDF column keys renamed to `"place_path"` — an undeclared second consumer of `row_field()`'s match-arm string key that Task 1 alone would have silently broken (string-key mismatch, not a compile error)
- `ui/src/bindings-phase6.ts` (hand-maintained, not generated) mirrored field-for-field: `devicePlace`/`devicePlaceId`/`printerPlace`/`place`
- `request_printer_options.rs`'s `seed_printer_devices` fixture rewritten off `INSERT INTO locations`/`devices.location_id` (both dropped by V038) onto `INSERT INTO places`/`devices.place_id` — this file seeds directly via raw SQL, so it would fail with "no such table: locations" (not just a stale assertion) if left unmigrated

## Task Commits

Each task was committed atomically:

1. **Task 1: report_service.rs — acts + devices report queries onto place_full_paths** - `d0d62a12` (feat)
2. **Task 2: report_service.rs — cartridges + printer report queries, combine_printer_and_place rename** - `b3586100` (feat)
3. **Task 3: request_service.rs printer_options + requests_sqlite.rs + printers_sqlite.rs + dto/printer.rs + dto/request.rs** - `471a943a` (feat)
4. **Task 4: dto/reports.rs place rename + is_storage quick filter (D-11.2)** - `209d4565` (feat)
5. **Task 5: tauri_cmds/reports.rs — column-key rename location_name → place_path** - `509c1b5a` (feat)
6. **Task 6: ui/src/bindings-phase6.ts — hand-maintained DTO field rename** - `910422cd` (feat)

**Follow-up cleanup:** `2bc18802` (docs) — two stray doc-comment mentions of "location" in `report_service.rs` (RPT-01/04/05 method doc comments) found during final grep sweep, updated to "place" for consistency with the renamed filter field.

## Files Created/Modified

- `crates/trackly-app/src/services/report_service.rs` — 8 report query functions migrated onto `place_full_paths`; D-28 subtree filter + D-11.2/D-11.4 `is_storage` filter added; `combine_printer_and_location` → `combine_printer_and_place`; `row_field()`'s `"location_name"` match arm → `"place_path"`; test fixtures updated
- `crates/trackly-app/src/services/request_service.rs` — `printer_options` SQL + doc comments migrated onto `place_full_paths`
- `crates/trackly-infra/src/repos/requests_sqlite.rs` — `SELECT_REQUESTS` joins `place_full_paths` via `devices.place_id`; `RequestRow.printer_place` mapping
- `crates/trackly-infra/src/repos/printers_sqlite.rs` — `SELECT_PRINTERS` joins `place_full_paths`; adds `d.place_id AS device_place_id`; `PrinterRow.device_place`/`device_place_id` mapping
- `crates/trackly-app/src/dto/printer.rs` — `PrinterDto.device_location` → `device_place` + new `device_place_id: Option<i64>`
- `crates/trackly-app/src/dto/request.rs` — `RequestDto.printer_location` → `printer_place`; `RequestPrinterOptionDto.location` → `place`
- `crates/trackly-app/src/dto/reports.rs` — `ReportFilter.location_id` → `place_id`; `ReportRow.location_name` → `place_path`; new `ReportFilter.is_storage: Option<bool>`
- `crates/trackly-app/src/tauri_cmds/reports.rs` — `columns_for()`'s 5 `"location_name"` keys → `"place_path"`
- `crates/trackly-app/tests/request_printer_options.rs` — fixture + assertions migrated off the dropped `locations` table onto `places`/`devices.place_id`
- `ui/src/bindings-phase6.ts` — `PrinterDto`/`RequestDto`/`RequestPrinterOptionDto` TS mirror updated field-for-field

## Decisions Made

See `key-decisions` in frontmatter for the full rationale on: (1) cartridge/requests report functions and their COUNT siblings getting `is_storage` but not the `place_id` subtree filter, matching pre-existing functional scope exactly; (2) dropping the vestigial `LEFT JOIN locations` in the two COUNT(*) act/device functions rather than rewriting it, since it was never SELECTed; (3) `request_printer_options.rs`'s fixture rewrite being in-scope (raw SQL against a dropped table, not just stale vocabulary).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Stale doc-comment "location" references in report_service.rs method docs**
- **Found during:** Final grep sweep after all 6 tasks landed
- **Issue:** Two `pub async fn` doc comments (`list_device_acts`, `list_device_returns`) still read "filtered by period, type, location" / "filtered by period and location" after the underlying filter field was renamed to `place_id` across Tasks 1 and 4 — stale prose, not a compile error, but misleading to a future reader.
- **Fix:** Updated both doc comments to say "place" instead of "location".
- **Files modified:** `crates/trackly-app/src/services/report_service.rs`
- **Verification:** `grep -n "location" crates/trackly-app/src/services/report_service.rs` returns 0 matches after the fix
- **Committed in:** `2bc18802`

---

**Total deviations:** 1 auto-fixed (Rule 1 — stale documentation text, no functional impact).
**Impact on plan:** No scope creep — every acceptance criterion in the plan is satisfied; the fix is a one-line-per-comment prose correction found during the plan's own final verification sweep.

## Issues Encountered

**`cargo build -p trackly-app` cannot reach `trackly-app` at all — this is the expected, already-documented state from `prior_wave_context`, not a bug introduced by this plan.** Ran `cargo build -p trackly-app` in the foreground after all 6 tasks landed (cold build, ~140s to first error report). `trackly-infra`'s lib crate fails to compile with 21 errors, every single one confined to `acts_sqlite.rs` (4 errors) and `cartridges_sqlite.rs` (17 errors) — Plans 39-07 and 39-09's scope respectively, per the prompt's own prior-wave-context instruction not to touch either file. Verified by grepping the full build-log for every file this plan owns (`report_service.rs`, `request_service.rs`, `requests_sqlite.rs`, `printers_sqlite.rs`, `dto/printer.rs`, `dto/request.rs`, `dto/reports.rs`, `tauri_cmds/reports.rs`) — zero matches. Because `trackly-infra` never finishes compiling, the compiler never reaches `trackly-app`'s source at all (not even a syntax pass), so this plan's own `cargo build -p trackly-app` acceptance criterion (Task 3/4's literal text) cannot produce a real pass/fail signal in isolation, matching the identical situation documented in 39-01/39-04/39-06's summaries for the same reason.

To compensate, syntax correctness of every file this plan touches was independently verified via `rustfmt --check --edition 2021` (parses each file standalone, does not require the crate to compile) — zero syntax errors, only pre-existing cosmetic line-wrapping drift on lines this plan did not modify (left untouched per the scope-boundary rule: "only auto-fix issues directly caused by the current task's changes").

`cargo test -p trackly-app --lib combine_printer_and_place` (Task 2's own verify command) could not run for the same reason — the crate never compiles. The renamed unit tests (`combine_printer_and_place_none_without_printer`, `combine_printer_and_place_appends_place`, `combine_printer_and_place_printer_only_when_place_missing`) are believed correct (identical logic to the pre-rename tests, only names changed) but have not been compiler-verified in this plan.

**Action for whichever plan (07/09) restores `cargo build -p trackly-infra`:** run `cargo build -p trackly-app` and `cargo test -p trackly-app --lib combine_printer_and_place --test request_printer_options -- --skip login_remember_persistent_cookie` at that point for the first real, compiler-verified signal on this plan's work.

## TDD Gate Compliance

No tasks in this plan were flagged `tdd="true"` (project-wide `tdd_mode=false`, confirmed in `.planning/config.json`). All 6 tasks are plain `type="auto"` mechanical SQL/DTO migrations with `feat` commits — no RED/GREEN gate sequence applies.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

Every report query, the create-request printer dropdown, and the requests/printers repository read paths now speak `place_full_paths`/`place_id` exclusively — zero remaining `LEFT JOIN locations`/`location_id`/`location_name` references in any file this plan owns (verified via grep across all 10 modified files, zero matches beyond the two doc-comment fixes already corrected). `PrinterDto.device_place_id` is live and ready for Plan 16's `OperationModal.svelte` `PlacePicker` prefill. `ReportFilter.place_id`/`is_storage` are live and ready for Plan 18's Reports UI place-filter/quick-filter controls.

**Blocker inherited, not introduced, by this plan:** `cargo build -p trackly-infra`/`cargo build -p trackly-app` will keep failing until Plans 39-07 (acts) and 39-09 (cartridges) migrate `acts_sqlite.rs`/`cartridges_sqlite.rs` off the dropped `locations` table. Once either lands enough of that migration for `trackly-infra` to compile, run `cargo build -p trackly-app` and `cargo test -p trackly-app --lib combine_printer_and_place --test request_printer_options -- --skip login_remember_persistent_cookie` for the first real, compiler-verified signal on this plan's work.

---
*Phase: 39-place-tree*
*Completed: 2026-08-23*

## Self-Check: PASSED

All 10 modified/created source files plus this SUMMARY.md confirmed present on disk; all 7 commit hashes (`d0d62a12`, `b3586100`, `471a943a`, `209d4565`, `509c1b5a`, `910422cd`, `2bc18802`) confirmed present in `git log`.

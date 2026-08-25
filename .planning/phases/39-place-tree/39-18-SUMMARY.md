---
phase: 39-place-tree
plan: 18
subsystem: ui
tags: [svelte, svelte5-runes, place-tree, reports, place-picker]

# Dependency graph
requires:
  - phase: 39-place-tree plan 13
    provides: "PlacePicker.svelte — the reusable place-selection control (value/onChange/id/disabled/invalid, default apiCall-backed fetchers) this plan wires into the reports filter"
  - phase: 39-place-tree plan 10
    provides: "report_service.rs's place_full_paths migration, D-28 subtree-inclusive place_id filter (WITH RECURSIVE subtree), D-11.2/D-11.4 is_storage ancestor-walk quick filter, ReportFilter.place_id/is_storage and ReportRow.place_path DTO renames; PrinterDto.devicePlace, RequestPrinterOptionDto.place, DeviceDto.full_path renames this plan's Task 3 consumes"
provides:
  - "ReportsPage.svelte — all 9 location_name report columns renamed to place_path, labels unified to 'Место' (UI-SPEC §12); place filter wired onto place_id/is_storage instead of a locations_autocomplete string list"
  - "ReportFilters.svelte — filtering reactivated for the first time since GAP-R4 stripped it down to Export/Print buttons: PlacePicker-based place filter with the D-28 'Включая вложенные места' hint, plus a separate D-11.2/D-11.5 'Складское место' three-option quick filter (Все/На складе/В эксплуатации)"
  - "ReportTable.svelte — ReportRow.location_name renamed to place_path in the local interface and separatorKey grouping (fixes a silent snapshot-report grouping collapse); D-26 short-path/full-path-in-title cell display for the place_path column"
  - "PrinterDetail.svelte / PrinterSelect.svelte / GroupedPrinterSelect.svelte — last 4 remaining svelte-check errors in the codebase cleared, reading the renamed devicePlace/place/full_path DTO fields from Plans 06/10"
affects: [39-19, 39-20, 39-21 (end-to-end/UAT checkpoint — reports place-filter subtree semantics and D-26 short-path display should be exercised in a real webview)]

tech-stack:
  added: []
  patterns:
    - "D-26's short-path/full-path-in-title cell convention belongs to ReportTable.svelte's formatCellValue-equivalent renderer, not ReportsPage.svelte's Column[] definitions — ReportsPage.svelte only builds { key, label } column metadata; it has no per-cell rendering of its own. Any future plan asked to change how a specific report column displays should look at ReportTable.svelte first, regardless of which file a plan's <action> text names."
    - "ReportFilters.svelte's is_storage quick filter reuses CartridgeFilters.svelte's Dropdown variant=\"select\" flat={true} fixed-option-list pattern (string ids, getGroupId/getGroupName/isGroupSelected, onExpandGroup returning an empty typed array) rather than inventing new markup — this is now the established shape for any small (2-5 option) quick filter in this codebase."

key-files:
  created: []
  modified:
    - ui/src/features/reports/ReportsPage.svelte
    - ui/src/features/reports/ReportFilters.svelte
    - ui/src/features/reports/ReportTable.svelte
    - ui/src/features/printers/PrinterDetail.svelte
    - ui/src/lib/components/PrinterSelect.svelte
    - ui/src/lib/components/GroupedPrinterSelect.svelte

key-decisions:
  - "ReportsPage.svelte's local ReportFilter (filter-state) type drops `location_name` entirely rather than renaming it to `place_path`, diverging from Task 1's literal 'rename every location_name/location_id type field... to place_path/place_id' instruction. Rationale: `location_name` was only ever forwarded as the old `locationName` prop to ReportFilters (now replaced by `placeId`); a `place_path` field on a *filter-parameter* type would be semantically nonsensical (filters key off ids, not display strings) and the backend's own `ReportFilter` DTO (Plan 39-10) has no such field either — only `place_id`/`is_storage`. `location_id` was renamed to `place_id` as instructed."
  - "D-26's short-path/tooltip cell display was implemented in ReportTable.svelte (Task 4's file) rather than ReportsPage.svelte (Task 1's file, where the plan's <action> text placed it) — ReportsPage.svelte has no cell renderer of its own, only Column[] key/label metadata consumed by ReportTable.svelte's generic formatCellValue. Implementing it where the actual rendering happens was necessary to satisfy the plan's own must_haves truth; documented here since it crosses the plan's per-task file boundary (though ReportTable.svelte was already in this plan's overall files_modified list for Task 4)."
  - "RequestDetail.svelte required NO code change for Task 3 — grep confirms zero occurrences of `printerLocation`/`prefillLocation` anywhere in the current file. The plan's read_first citation (line ~730, `prefillLocation={request.printerLocation ?? undefined}`) does not match current code: this surface passes `preFillPrinterId` (a device id) to OperationModal, not a location/place string. Likely stale relative to a prior OperationModal rewrite (Plan 39-16). Verified via full-file grep before concluding no edit was needed, not assumed."

requirements-completed: [PLC-03, PLC-04]

# Metrics
duration: ~55min
completed: 2026-08-25
---

# Phase 39 Plan 18: Reports place-filter + place_path columns Summary

**Wired `PlacePicker` into the Reports filter (D-28 subtree-inclusive, with a separate D-11.2 "Складское место" quick filter) and renamed all 9 `location_name` report columns to `place_path` with the D-26 short-path/tooltip display; also cleared the last 4 remaining `svelte-check` errors in the codebase (printer/device DTO-rename consumers). Runtime behavior is UNVERIFIED (see below).**

## Performance

- **Duration:** ~55 min
- **Started:** 2026-08-25 (est.)
- **Completed:** 2026-08-25
- **Tasks:** 4/4
- **Files modified:** 6

## Accomplishments

- `ReportsPage.svelte`: all 9 `location_name` report columns (acts, returns, device in_use/in_stock, cartridge consumption/refills/in_use/in_stock, requests) renamed to `place_path`; every label unified from the prior "Локация"/"Расположение" mix to a single "Место" term per UI-SPEC §12; `locations_autocomplete` onMount fetch + `filterLocations` state deleted (PlacePicker fetches its own data); `<ReportFilters>` now receives `placeId`/`isStorage` instead of `locationName`/`locations`
- `ReportFilters.svelte`: filtering reactivated for the first time since GAP-R4 stripped it to Export/Print-only — new `<PlacePicker>` place filter with the D-28 "Включая вложенные места" hint (`--tr-text-label`/`--tr-text-secondary`), and a separate, independently-labeled "Складское место" three-option Dropdown (Все/На складе/В эксплуатации) for D-11.2/D-11.4, kept structurally distinct from `statusId` per D-11.5
- `ReportTable.svelte`: `ReportRow.location_name` renamed to `place_path` (interface, both comments, `separatorKey` computation) — fixes a silent bug where the optional field's rename-without-error let every snapshot-report separator collapse to a single `''` group; also implements D-26's short-path (last two ` / `-separated segments) cell display with the full `place_path` always in the cell's `title`
- `PrinterDetail.svelte`/`PrinterSelect.svelte`/`GroupedPrinterSelect.svelte`: `deviceData?.location` → `deviceData?.full_path`, `p.deviceLocation` → `p.devicePlace`, `opt.location` → `opt.place` — the last 4 remaining `svelte-check` errors in the codebase (per 39-17-SUMMARY.md's honest baseline), now 0
- `RequestDetail.svelte`: no change — confirmed via grep that this file never referenced `printerLocation`/`prefillLocation` in its current form

## Task Commits

Each task was committed atomically:

1. **Task 3: RequestDetail/PrinterDetail/PrinterSelect/GroupedPrinterSelect — Plan 06/10 DTO rename consumers** - `c0a8597c` (fix)
2. **Task 4: ReportTable.svelte — place_path rename + D-26 short-path cells** - `3a1ff94a` (feat)
3. **Task 1: ReportsPage.svelte — 9 columns → place_path, D-26 label unification** - `371a26c6` (feat)
4. **Task 2: ReportFilters.svelte — PlacePicker place filter + is_storage quick filter** - `2c165c53` (feat)

**Plan metadata:** `7661a766` (docs: log runtime verification debt to deferred-items.md)

(Tasks were executed and committed in dependency order — Task 3 and Task 4 first since Task 1/2's `ReportsPage.svelte`/`ReportFilters.svelte` changes depend on `ReportTable.svelte`'s renamed `ReportRow` interface staying type-consistent — not the plan's literal 1/2/3/4 listing order. Every commit's own `svelte-check` gate stayed green at each step.)

## Files Created/Modified

- `ui/src/features/reports/ReportsPage.svelte` - 9 `place_path` columns, unified "Место" labels, `place_id`/`is_storage` filter wiring, deleted `locations_autocomplete` fetch
- `ui/src/features/reports/ReportFilters.svelte` - `PlacePicker` place filter + D-28 hint + "Складское место" quick filter (filtering reactivated after GAP-R4)
- `ui/src/features/reports/ReportTable.svelte` - `place_path` rename (interface/comments/separatorKey) + D-26 short-path/title cell display
- `ui/src/features/printers/PrinterDetail.svelte` - `deviceData?.full_path`
- `ui/src/lib/components/PrinterSelect.svelte` - `p.devicePlace` in `printerLabel()`
- `ui/src/lib/components/GroupedPrinterSelect.svelte` - `opt.place` in `groups` derivation

## Decisions Made

See `key-decisions` in frontmatter for full rationale on: (1) dropping `location_name` from `ReportsPage.svelte`'s filter-state type instead of renaming it to a semantically-nonsensical `place_path` filter field; (2) implementing D-26's short-path/tooltip convention in `ReportTable.svelte` (the actual cell renderer) rather than `ReportsPage.svelte` (where the plan's task text placed it, but which only builds column metadata); (3) `RequestDetail.svelte` needing zero changes, verified by grep rather than assumed from the plan's (stale) read_first citation.

## Deviations from Plan

### Judgment calls (not bugs, tracked separately)

**1. D-26 short-path display implemented in ReportTable.svelte instead of ReportsPage.svelte**
- **Found during:** Task 1, while reading `ReportsPage.svelte`'s current structure to implement the D-26 truncation logic the task's `<action>` text describes
- **Issue:** `ReportsPage.svelte` builds `Column[]` metadata (`{ key, label }`) only; it passes `columns` to `<ReportTable>`, which owns the actual per-cell rendering via its own `formatCellValue`. There is no "cell renderer" inside `ReportsPage.svelte` to modify.
- **Resolution:** Implemented `shortPlacePath()`/`formatCellDisplay()`/`formatCellTitle()` in `ReportTable.svelte` (already in this plan's `files_modified` for Task 4) instead. `ReportsPage.svelte`'s own contribution to D-26 is limited to renaming the column keys to `place_path` and unifying labels to "Место" — both done in Task 1 as literally scoped.
- **Verification:** `pnpm --dir ui run svelte-check` — 0 errors after both Task 1 and Task 4 landed; manual code review confirms the truncation/title logic only activates for `colKey === 'place_path'`, leaving every other column's display byte-for-byte unchanged.

**2. `filter.location_name` dropped rather than renamed to `place_path` in ReportsPage.svelte's ReportFilter type**
- **Found during:** Task 1, renaming the local `ReportFilter` (filter-state) interface
- **Issue:** The task's literal instruction ("rename every location_name/location_id type field... to place_path/place_id") would produce a `place_path` field on a filter-parameter type that has no backend counterpart (Plan 39-10's backend `ReportFilter` DTO only has `place_id`/`is_storage`) and was never read anywhere in the file except as the value forwarded to the old `locationName` prop (now replaced by `placeId`, reading `filter.place_id`).
- **Resolution:** Dropped `location_name` from the filter type entirely; renamed `location_id` → `place_id` as instructed. No functional loss — the field had exactly one call site and that call site now reads `place_id`.
- **Verification:** `grep -c "location_name\|location_id"` returns 0 on `ReportsPage.svelte`; `svelte-check` 0 errors.

---

**Total deviations:** 0 auto-fixed bugs (Rules 1-3). Two judgment calls on plan-text/file-boundary ambiguity (documented above), both within this plan's own declared `files_modified` scope — no architectural changes requiring Rule 4.

## Issues Encountered

**Only compile/lint/build gates were run — this is NOT runtime verification** (established project convention). Specifically:
- `pnpm --dir ui run svelte-check` — **0 errors** (down from the 4 pre-existing errors this plan's Task 3 territory accounted for per `39-17-SUMMARY.md`'s honest baseline; 54 warnings, unchanged from the pre-plan baseline, none newly introduced by this plan's files)
- `pnpm exec eslint` on all six touched files — clean
- `node scripts/check-tokens.mjs` / `check-focus-outline.mjs` / `check-contrast.mjs` — all PASS
- `pnpm --dir ui build` — succeeds (647 modules, no new warnings attributable to this plan's files)
- Verified the D-28 subtree-inclusive filtering claim in `ReportFilters.svelte`'s hint text is backed by real backend behavior: `grep -n "WITH RECURSIVE subtree" crates/trackly-app/src/services/report_service.rs` confirms all 4 report-query call sites Plan 39-10 touches already implement descendant-walk `place_id` filtering — no backend gap to flag (unlike the cross-plan-dependency warning the plan's own `<action>` text anticipated as a possibility).

**None of the above catch Svelte 5 rune runtime errors or WKWebView-specific rendering behavior.** Runtime behavior (PlacePicker opening/selecting/clearing inside the reports filter, the "Складское место" dropdown's actual filtering effect, D-26's short-path truncation and tooltip rendering across all 9 report tables, snapshot-report grouping after the `separatorKey` fix, the three printer/device detail surfaces showing resolved place text) is **UNVERIFIED**. A detailed manual-verification checklist has been appended to `.planning/phases/39-place-tree/deferred-items.md` under "Plan 18 — Reports place filter runtime verification NOT performed", to be executed in the batched UAT pass at Plan 20/21's checkpoint or via `/gsd-verify-work`.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

`ui/src/features/reports/` now speaks `place_path`/`place_id`/`is_storage` exclusively — zero remaining `location_name`/`location_id`/`locations_autocomplete` references across `ReportsPage.svelte`/`ReportFilters.svelte`/`ReportTable.svelte` (confirmed via grep). `svelte-check` across the entire `ui/` tree is now at **0 errors** (54 pre-existing warnings only) — this plan closed the last error-producing file set the phase had accumulated (`PrinterDetail.svelte`/`PrinterSelect.svelte`/`GroupedPrinterSelect.svelte`, flagged by 39-17-SUMMARY.md). `PlacePicker`'s injection-prop contract from Plan 39-13 held up unmodified for this new real consumer (no changes to `PlacePicker.svelte` itself were needed, consistent with every other Plan 15-19 consumer). Plans 39-19/39-20/39-21 (remaining wiring + end-to-end UAT checkpoint) are unblocked to proceed; the deferred runtime-verification checklist for this plan (place filter subtree semantics, is_storage quick filter, D-26 display, printer/device detail renames) should be run at least once against a real webview before those checkpoints close.

---
*Phase: 39-place-tree*
*Completed: 2026-08-25*

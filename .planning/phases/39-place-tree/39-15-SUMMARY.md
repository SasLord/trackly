---
phase: 39-place-tree
plan: 15
subsystem: ui
tags: [svelte, svelte5-runes, rust, place-tree, devices, csv-import, place-picker]

# Dependency graph
requires:
  - phase: 39-place-tree plan 12
    provides: "places_* Tauri/HTTP commands + PlaceDto/PlaceNewDto/PlacePathDto bindings.ts types"
  - phase: 39-place-tree plan 13
    provides: "PlacePicker.svelte — value/onChange/id/disabled/invalid props, default apiCall-backed fetchers"
  - phase: 39-place-tree plan 06
    provides: "Devices backend migrated to place_id/full_path (DeviceDto/DeviceNew/DevicePatch/DeviceFilter); locations_autocomplete transport removed; CSV import resolves a place-path column against the tree server-side"
provides:
  - "Device create/edit form (DeviceFormBody.svelte) and printer-creation modal (PrinterCreateModal.svelte) select place via PlacePicker bound to place_id — no freeform location text field remains on the device family"
  - "DeviceAutocompleteField.svelte with the field==='location' special case fully removed (FieldName now name|model|specs|kit|state only)"
  - "CSV import's place column resolved by full path under the unified 'Место' term (frontend mapping value 'place', matching backend's renamed match arm)"
  - "Devices list/filter/export and showcase table all read/write place_id/full_path instead of location/location_id"
  - "Fixed a double-row-prefix bug in the CSV import's place-not-found error message (device_service.rs) — now renders UI-SPEC §12's exact copy instead of a duplicated 'Строка N: Строка N: ...' string"
affects: [39-16, 39-17, 39-18, 39-21 (end-to-end/UAT checkpoint should exercise device form + printer creation + CSV import in a real webview)]

tech-stack:
  added: []
  patterns:
    - "Every real PlacePicker consumer (per Plan 13's contract) omits fetchChildren/fetchSearchResults/fetchOne/createPlace and gets the default apiCall-backed behavior — DeviceFormBody.svelte and PrinterCreateModal.svelte both follow this, passing only value/onChange/id/(invalid)."
    - "CSV import's mapping HashMap<String,String> (CSV header -> device field name) is an internal frontend/backend wire contract with no shared type — renaming a mapping value on one side without the other silently drops the column (unknown keys are ignored per T-02-05-08), so the frontend option-value rename ('location' -> 'place') required a matching backend match-arm rename in the same task, even though device_service.rs wasn't in this plan's declared files_modified list."

key-files:
  created: []
  modified:
    - ui/src/features/devices/DeviceAutocompleteField.svelte
    - ui/src/features/devices/DeviceFormBody.svelte
    - ui/src/features/printers/PrinterCreateModal.svelte
    - ui/src/features/devices/DeviceImportCsvModal.svelte
    - ui/src/features/devices/DevicesPage.svelte
    - ui/src/features/devices/DeviceListRow.svelte
    - ui/src/features/devices/DeviceGroupRow.svelte
    - ui/src/features/devices/DeviceList.svelte
    - ui/src/features/showcase/sections/TableSection.svelte
    - crates/trackly-app/src/services/device_service.rs
    - crates/trackly-app/tests/devices_csv_import.rs

key-decisions:
  - "Renamed the CSV import mapping wire value from 'location' to 'place' on BOTH sides (frontend option + backend match arm in device_service.rs), even though device_service.rs wasn't listed in this plan's files_modified. The plan's Task 2 acceptance criteria explicitly required zero '\"location\"'/'Расположение' occurrences in the frontend file; doing that rename on the frontend alone would have made the mapping key mismatch the backend's literal \"location\" match arm, silently dropping the place column on every CSV import (unknown mapping keys are ignored, T-02-05-08) instead of resolving or erroring per-row. Classified as Rule 1 (auto-fix bug directly caused by this task's rename) rather than Rule 4, since it's a same-day, self-contained wire-contract rename with no schema/architecture change."
  - "Found and fixed a pre-existing double-row-prefix bug in device_service.rs's CSV place-not-found error: the backend baked its own 'Строка N:' prefix into error_message on top of the generic 'Строка {row_index}:' prefix the import modal already prepends for every row, producing 'Строка 12: Строка 12: место «...» не найдено в дереве.' instead of UI-SPEC §12's exact copy. This bug existed since Plan 06 (backend-only) but was only ever exercised end-to-end once Plan 15 wired a real UI to trigger it — fixed under Rule 1, backend now emits just 'место «...» не найдено в дереве.' and relies on the frontend's existing per-row prefix, matching every other RowError message in this function."
  - "DeviceList.svelte's table header (renders the same live 'Место'/'Расположение' column DeviceListRow/DeviceGroupRow's data-field rename targets) and TableSection.svelte's showcase header were both renamed 'Расположение' -> 'Место' for UI-SPEC §12 term unification, even though DeviceList.svelte wasn't in this plan's declared files_modified — leaving it unchanged would have shipped a visibly inconsistent label (the device form/printer modal/CSV import all say 'Место', the actual devices table header would still say 'Расположение')."
  - "Added a new backend regression test (import_commit_unresolved_place_reports_row_error_with_exact_copy in devices_csv_import.rs) locking in both the renamed mapping key and the corrected error-copy composition — no existing test previously exercised the place-not-found path end-to-end."

requirements-completed: [PLC-03, PLC-04]

# Metrics
duration: ~55min
completed: 2026-08-25
---

# Phase 39 Plan 15: Device-family PlacePicker wiring Summary

**Wired `PlacePicker` into the device create/edit form and printer-creation modal, removed `DeviceAutocompleteField`'s dead `location` special case, unified CSV import's place column and error copy under the "Место" term (fixing a real double-row-prefix bug in the backend along the way), and renamed the devices list/filter/showcase surfaces off `location`/`location_id` onto `place_id`/`full_path`.**

## Performance

- **Duration:** ~55 min
- **Started:** 2026-08-25 (est.)
- **Completed:** 2026-08-25T03:04:37Z
- **Tasks:** 3/3
- **Files modified:** 11 (9 UI, 2 backend Rust — see key-decisions for why the backend files weren't in the plan's declared list)

## Accomplishments

- `DeviceAutocompleteField.svelte`: `FieldName` union reduced to `name | model | specs | kit | state`; the `field === 'location'` special case (`allLocationSuggestions` state, parallel `locations_autocomplete` fetch at both call sites, dedicated dropdown render block, now-unused `apiCall` import) fully removed
- `DeviceFormBody.svelte`: device's place field is now `<PlacePicker value={placeId} onChange={(id) => (placeId = id)} />` bound to `place_id`; `canSubmit`, the edit-mode `DevicePatch`, and the create-mode `DeviceNew` payload all use `placeId`/`place_id` instead of the removed `location`/`location_id` fields; label unified to "Место"
- `PrinterCreateModal.svelte`: replaced `LocationAutocomplete` with `PlacePicker` bound to `place_id` (still optional, matching prior UX) for the manual printer-creation flow
- `DeviceImportCsvModal.svelte`: column-mapping option renamed `location`/`Расположение` -> `place`/`Место`; header-guesser recognizes "Место"/"место"/"Place"/"place"; documented inline how the per-row error list composes UI-SPEC §12's exact copy from `err.row_index` + `err.error_message`
- `device_service.rs`: CSV mapping match arm renamed `"location"` -> `"place"` to match the frontend's new wire value (a bug that would otherwise have silently dropped every imported place); fixed a double-row-prefix bug in the place-not-found `RowError` (backend no longer bakes its own "Строка N:" into `error_message`, since the import modal already prepends `row_index` generically for every row)
- `devices_csv_import.rs`: new test `import_commit_unresolved_place_reports_row_error_with_exact_copy` locks in the renamed mapping key and the corrected, non-duplicated error copy (T-39-15-01)
- `DevicesPage.svelte`/`DeviceListRow.svelte`/`DeviceGroupRow.svelte`: `DeviceFilter.location_id` -> `place_id` (filter + CSV export payload); list/group rows read `device.full_path`/`group.repr.full_path` (including `DeviceGroupRow`'s `groupStableKey()` composite key)
- `DeviceList.svelte`/`TableSection.svelte`: table header label unified `"Расположение"` -> `"Место"`; showcase demo data/interface field renamed `location` -> `full_path`

## Task Commits

Each task was committed atomically:

1. **Task 1: DeviceAutocompleteField.svelte + DeviceFormBody.svelte + PrinterCreateModal.svelte** - `c2f45725` (feat)
2. **Task 2: DeviceImportCsvModal.svelte — CSV place column resolved by full path** - `b5b3b877` (feat)
3. **Task 3: DevicesPage.svelte + DeviceListRow.svelte + DeviceGroupRow.svelte + TableSection.svelte — location field/column rename** - `2c7f1e10` (feat)

## Files Created/Modified

- `ui/src/features/devices/DeviceAutocompleteField.svelte` - removed `field==='location'` special case
- `ui/src/features/devices/DeviceFormBody.svelte` - PlacePicker wired to `place_id`
- `ui/src/features/printers/PrinterCreateModal.svelte` - PlacePicker replaces LocationAutocomplete
- `ui/src/features/devices/DeviceImportCsvModal.svelte` - place column mapping + guesser unified to "Место"
- `crates/trackly-app/src/services/device_service.rs` - mapping key rename + double-prefix bug fix
- `crates/trackly-app/tests/devices_csv_import.rs` - new regression test for the not-found path
- `ui/src/features/devices/DevicesPage.svelte` - `DeviceFilter.location_id` -> `place_id`
- `ui/src/features/devices/DeviceListRow.svelte` - `device.location` -> `device.full_path`
- `ui/src/features/devices/DeviceGroupRow.svelte` - `g.repr.location`/`group.repr.location` -> `full_path`
- `ui/src/features/devices/DeviceList.svelte` - table header "Расположение" -> "Место"
- `ui/src/features/showcase/sections/TableSection.svelte` - demo data field + header renamed

## Decisions Made

See `key-decisions` in frontmatter for full rationale on: (1) the cross-file CSV mapping-key rename (frontend `'location'`->`'place'` required a matching backend match-arm rename to avoid silently dropping CSV place data); (2) the double-row-prefix bug fix in `device_service.rs` (pre-existing since Plan 06, only surfaced once a real UI exercised the path); (3) `DeviceList.svelte`/`TableSection.svelte` header-label unification beyond the plan's declared file list, for UI-SPEC §12 term consistency.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] CSV import mapping key rename required a backend match-arm rename to avoid silently dropping place data**
- **Found during:** Task 2, while renaming the frontend's column-mapping option value
- **Issue:** The plan's acceptance criteria required renaming the frontend's mapping `value` from `'location'` to `'place'`, but `device_service.rs`'s `build_device_new_from_row` matched CSV mapping values against the literal string `"location"`. A frontend-only rename would have made every CSV place column map to an unknown key, silently ignored (`T-02-05-08`) — devices would import with `place_id: None` and no error, instead of correctly resolving or reporting "not found".
- **Fix:** Renamed the backend match arm from `"location"` to `"place"` (and its two doc comments), keeping frontend and backend wire-compatible.
- **Files modified:** `crates/trackly-app/src/services/device_service.rs`
- **Verification:** `cargo test -p trackly-app --test devices_csv_import` — 11/11 passed (existing 10 + 1 new)
- **Committed in:** `b5b3b877` (Task 2 commit)

**2. [Rule 1 - Bug] Double row-prefix in CSV place-not-found error message**
- **Found during:** Task 2, while verifying the "Строка N: место «...» не найдено в дереве." copy would render correctly
- **Issue:** `device_service.rs`'s place-not-found `RowError.error_message` baked in its own `"Строка {row_index}: "` prefix, while `DeviceImportCsvModal.svelte`'s error-list template *also* prepends `"Строка {err.row_index}:"` generically for every row (the same pattern used for validation/create errors, which don't self-prefix). The combination rendered `"Строка 12: Строка 12: место «...» не найдено в дереве."` — a duplicated prefix, not UI-SPEC §12's literal expected copy. This bug existed since Plan 06 (backend-only, no UI wired to it yet) and was only caught here because Task 2 required verifying the exact rendered string.
- **Fix:** Removed the backend's self-baked prefix; `error_message` is now just `"место «{text}» не найдено в дереве."`, letting the frontend's existing generic row-prefix compose the exact UI-SPEC §12 string.
- **Files modified:** `crates/trackly-app/src/services/device_service.rs`
- **Verification:** New test `import_commit_unresolved_place_reports_row_error_with_exact_copy` asserts both the raw `error_message` (no self-prefix) and the UI-composed string (`format!("Строка {}: {}", row_index, error_message)`) match UI-SPEC §12 exactly.
- **Committed in:** `b5b3b877` (Task 2 commit)

**3. [Rule 2 - Missing critical functionality] Term-unification header labels in files outside the plan's declared list**
- **Found during:** Task 3, after renaming `DeviceListRow`/`DeviceGroupRow`'s data-field reads
- **Issue:** `DeviceList.svelte` (the actual live devices table, imported by `DevicesPage.svelte`) still rendered a `<th>Расположение</th>` header for the same column whose data field this task renamed to `full_path`. Left unchanged, the real Devices page would show "Расположение" as the column header while every other device-family surface (form, printer modal, CSV import) now says "Место" — violating UI-SPEC §12's explicit term-unification requirement and directly caused by this task's scope.
- **Fix:** Renamed `DeviceList.svelte`'s and `TableSection.svelte`'s (showcase mirror) table header from "Расположение" to "Место".
- **Files modified:** `ui/src/features/devices/DeviceList.svelte`, `ui/src/features/showcase/sections/TableSection.svelte`
- **Verification:** `pnpm --dir ui run svelte-check` — no new errors; `pnpm --dir ui build` succeeds
- **Committed in:** `2c7f1e10` (Task 3 commit)

---

**Total deviations:** 3 auto-fixed (2× Rule 1, 1× Rule 2). All were necessary for correctness (CSV import would otherwise silently drop place data and/or show a malformed error message) or spec compliance (term unification on the one visible surface the plan's file list missed). No scope creep beyond what each bug directly required to fix.

## Issues Encountered

**`cargo build -p trackly-app` piped through `| tail -60` appeared to stall indefinitely** (0% CPU, no output growth for 20+ minutes) — matches a known project failure mode (background cargo + pipe stall). Killed the stuck process and re-ran with output redirected directly to a log file (no pipe) instead; the build then completed normally in ~5 minutes. `cargo clippy -p trackly-app --tests -- -D warnings` (11 min) and both CSV-import test runs completed cleanly with this pattern. No code issue — a harness/pipe-buffering artifact, noted here in case it recurs in later Phase 39 plans.

**Runtime behavior in a real webview is UNVERIFIED** for this plan's changes (project convention: svelte-check/eslint/build are compile/lint gates, not runtime verification). Specifically unverified: PlacePicker's actual open/select/clear interaction inside `DeviceFormBody`/`PrinterCreateModal` in a running desktop or LAN-browser session, and the CSV import modal's end-to-end file-pick -> preview -> map -> commit -> error-list flow with a real file. The backend half of the CSV place-resolution path (mapping key, exact-match resolution, error-copy composition) IS verified — via the new and existing `devices_csv_import.rs` integration tests (11/11 passing) — but the frontend rendering of that data has not been exercised in a live app. Per project convention this should be added to `.planning/phases/39-place-tree/deferred-items.md`'s batched UAT checklist (Plan 20/21).

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

Devices, the device form, and the CSV import path are fully off `location`/`location_id` and onto `place_id`/`full_path` — the only remaining `location` field references in the codebase belong to acts/cartridges/printers/requests (Plans 16-18's territory, already confirmed pre-existing/unchanged by this plan's `svelte-check` diff: 33 baseline errors before this plan, 26 after — all reductions are from files this plan touched, zero new errors introduced elsewhere). `PlacePicker`'s injection-prop contract from Plan 13 held up unmodified for both new real consumers (`DeviceFormBody`, `PrinterCreateModal`) — no changes to `PlacePicker.svelte` itself were needed, which is a positive signal for Plans 16-18's upcoming wiring. The CSV import place-resolution path is now genuinely end-to-end correct and test-covered (previously it was backend-only and had never been exercised past `device_service.rs`'s unit-level integration tests). All work in this plan should be included in Plan 20/21's batched real-webview UAT pass per `deferred-items.md`.

---
*Phase: 39-place-tree*
*Completed: 2026-08-25*

## Self-Check: PASSED

All 11 created/modified source files confirmed present on disk, plus this SUMMARY and
`deferred-items.md`. All three task commit hashes (`c2f45725`, `b5b3b877`, `2c7f1e10`)
confirmed present in `git log`.

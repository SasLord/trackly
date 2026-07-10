---
phase: 18-autocomplete-dropdowns
plan: 01
subsystem: api
tags: [rusqlite, fts5, sqlite, devices, backend-contract]

requires: []
provides:
  - "list_grouped() true-branch (group_by_condition=true) groups by (type_id, name, model) instead of (type_id, name, condition) — D-05"
  - "list_grouped() true-branch sorts by count DESC, name ASC instead of alphabetical — D-04"
  - "list_grouped() true-branch supports a real multi-field text filter (name/inventory_no/serial_no/model) via devices_fts MATCH + build_fts_query sanitizer — AUTO-03"
affects: [18-04, 18-05]

tech-stack:
  added: []
  patterns:
    - "Static SQL branch-per-mode (no format!() with user text) — three constant SQL strings dispatched by filter.group_by_condition + presence of a sanitized text filter"
    - "Shared row-mapping helper (group_row_tuple) to avoid tripling an 18-field query_map closure across SQL branches"

key-files:
  created: []
  modified:
    - crates/trackly-infra/src/repos/devices_sqlite.rs
    - crates/trackly-core/src/domain/devices.rs
    - crates/trackly-app/src/dto/device.rs
    - crates/trackly-app/tests/devices_grouping.rs

key-decisions:
  - "group_row_tuple() extracted as a free function (not an impl-block closure) so all three SQL branches (sql_without_condition, sql_grouped_by_model_no_query, sql_grouped_by_model_with_query) reuse one row-mapping path"
  - "query_text computed once via filter.name_prefix.trim().filter(non-empty); only participates in SQL when group_by_condition=true, mirroring the pre-existing search_fts empty-match-expr → Ok(empty) short-circuit"
  - "condition remains in the true-branch SELECT (MAX(d.condition)) and in COUNT(DISTINCT d.condition) — no longer part of GROUP BY, repurposed as a drill-in signal (D-07) rather than a group-key differentiator"

requirements-completed: [AUTO-03, AUTO-04, AUTO-05]

duration: 20min
completed: 2026-07-10
---

# Phase 18 Plan 01: list_grouped backend contract rewrite Summary

**`list_grouped()` true-branch now groups by (type_id, name, model), sorts by count DESC, and filters text through devices_fts MATCH + the existing build_fts_query sanitizer — false-branch (DevicesPage) left byte-for-byte unchanged.**

## Performance

- **Duration:** ~20 min
- **Started:** 2026-07-09 (session)
- **Completed:** 2026-07-10T00:01:20Z
- **Tasks:** 2/2 completed
- **Files modified:** 4

## Accomplishments

- `list_grouped(group_by_condition=true)` group key changed from `(type_id, name, condition)` to `(type_id, name, model)` (D-05) — two devices with the same name but different model now split into separate groups; two devices with the same name+model but different condition now collapse into one group, with `condition_distinct_count` signalling the mix for frontend drill-in (D-07).
- Sort order for the true-branch changed from alphabetical (`ORDER BY d.name`) to `ORDER BY cnt DESC, d.name ASC` (D-04) — the device picker in the act form now surfaces the highest-stock group first.
- `name_prefix` is no longer a dead field for `group_by_condition=true`: it now drives a real multi-field FTS5 filter (name, inventory_number, serial_number, model) reusing the existing `build_fts_query` sanitizer (T-02-04-01) — closes the AUTO-03 regression where the device picker's text filter had no effect.
- `group_by_condition=false` branch (`sql_without_condition`, DevicesPage) is untouched — confirmed via `git diff` showing zero changes to that SQL string.
- Extracted `group_row_tuple()` helper to avoid tripling the 18-field row-mapping closure across the three SQL branches.

## Task Commits

Each task was committed atomically:

1. **Task 1: Переписать группировку/сортировку/фильтр в list_grouped (D-04, D-05, AUTO-03)** - `c3a5237` (feat)
2. **Task 2: Исправить регрессирующие тесты и добавить покрытие на group-by-model/сортировку/фильтр/injection** - `7207ede` (test)

**Plan metadata:** (pending — this commit)

## Files Created/Modified

- `crates/trackly-infra/src/repos/devices_sqlite.rs` — replaced `sql_with_condition` with two static SQL constants (`sql_grouped_by_model_no_query`, `sql_grouped_by_model_with_query`); added `GroupRowTuple` type alias + `group_row_tuple()` helper; dispatch logic computes sanitized `query_text`/`match_expr` and selects among 3 static SQL branches
- `crates/trackly-core/src/domain/devices.rs` — updated doc comments on `DeviceFilter.name_prefix`/`group_by_condition` and `DeviceGroupRow.condition_distinct_count` to describe the new true-branch semantics
- `crates/trackly-app/src/dto/device.rs` — mirrored the same doc-comment updates on the DTO-layer `DeviceFilter`/`DeviceGroup` (fields unchanged, no wire-format break)
- `crates/trackly-app/tests/devices_grouping.rs` — renamed/rewrote 3 regressed tests, added 5 new tests (23 total, all passing)

## Decisions Made

- `group_row_tuple()` is a free function (matching the file's existing `from_row` convention) rather than an inline closure duplicated 3x — keeps the 18-field mapping in one place.
- `condition` stays in the SELECT list (`MAX(d.condition) AS condition`) for the true-branch even though it's no longer in `GROUP BY` — needed so `repr.state` still has a representative value, consistent with how the false-branch already does `MAX(d.condition)`.
- The empty-after-sanitization short-circuit (`return Ok(Vec::new())`) mirrors the existing `search_fts` behaviour exactly, for consistency across the two FTS5 entry points in this file.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Two additional regressed tests beyond the plan's named pair also asserted the old condition-splits-groups behaviour**
- **Found during:** Task 2 (test rewrite/verification pass)
- **Issue:** The plan's `<read_first>` named `condition_key_splits_groups` and `grouping_act_form_keeps_condition_split` as regressing tests, but `grouping_groups_devices_with_same_name_and_different_condition` (same file, DEF-2B-era test) also asserted `groups.len() == 2` for two same-name-and-model devices with different condition under `group_by_condition=true` — a direct, mechanical consequence of Task 1's D-05 semantic change, not a new discovery outside scope.
- **Fix:** Renamed to `grouping_groups_devices_with_same_name_and_model_ignores_condition`; updated assertions to expect 1 group, `count=2`, `condition_distinct_count=2` (same pattern as the two plan-named renames).
- **Files modified:** `crates/trackly-app/tests/devices_grouping.rs`
- **Verification:** `cargo test -p trackly-app --test devices_grouping` — 23/23 passing.
- **Committed in:** `7207ede` (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 — mechanical test-assertion fix directly caused by Task 1's in-scope semantic change).
**Impact on plan:** No scope creep — this is the same class of fix the plan explicitly called out for the other two tests; the third instance was simply not enumerated by name in `<read_first>`.

## Issues Encountered

None — `cargo build -p trackly-infra`, `cargo build -p trackly-app`, `cargo test -p trackly-app --test devices_grouping` (23/23), and `cargo clippy -p trackly-infra -p trackly-app -- -D warnings` all pass clean on first full pass after the type fix below.

One compile-time type mismatch was caught and fixed during verification (not a deviation from plan intent, just a build error): the new `grouping_true_branch_sorts_by_count_desc` test initially collected `g.count` (DTO type `u64`) into a `Vec<i64>`; changed to `Vec<u64>` to match `DeviceGroup.count`'s actual type.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

The backend contract for `list_grouped(group_by_condition=true)` is now stable and matches D-04/D-05/AUTO-03. Plans 18-04 and 18-05 (frontend device picker with grouping/drill-in) can build against this contract: groups arrive sorted by count DESC, grouped by (type_id, name, model), and text-filterable via `name_prefix`. `condition_distinct_count > 1` is the signal those plans need to trigger a condition-level drill-in view (D-07) — no backend work remains to support that drill-in; it consumes the existing field plus a new endpoint/query if the frontend needs the actual per-condition breakdown (not delivered by this plan, out of its scope per `files_modified`).

---
*Phase: 18-autocomplete-dropdowns*
*Completed: 2026-07-10*

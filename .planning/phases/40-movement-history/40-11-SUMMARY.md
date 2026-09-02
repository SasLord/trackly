---
phase: 40-movement-history
plan: 11
subsystem: database
tags: [rusqlite, sqlite, reports, movement-history, with-recursive]

# Dependency graph
requires:
  - phase: 40-movement-history (plan 01)
    provides: "migration V040 (place_movements table + indexes), MovementSource/MovementEntityKind enums"
  - phase: 40-movement-history (plan 02)
    provides: "place_path_display::compute_place_path_short — single owner of the path-shortening formula, callable with &ReaderPool"
provides:
  - "ReportFilter.from_place_id / to_place_id — two independent subtree-inclusive place filters (D-24), AND semantics when both set"
  - "ReportRow.from_place_path / from_place_path_short / actor_name / reason / entity_type_label / is_deleted — the movements-report row shape"
  - "ReportService::list_movements (13th list_*) + query_movements_inner + movement_reason — query layer ready for the adapter layer"
affects: [40-12]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Two independently-built WITH RECURSIVE CTEs (from_subtree/to_subtree), each only emitted when its filter field is Some, combined by AND in the WHERE clause — avoids an unused CTE on the common unfiltered call"
    - "query_movements_inner takes &ReaderPool alongside &Connection specifically to call place_path_display::compute_place_path_short for both from/to snapshots, rather than re-deriving the variant/separators formula inline (the WR-03/WR-08 duplication anti-pattern Plan 40-02 exists to prevent)"
    - "movement_reason: backend-composed reason string, act_id/act_number take priority over the source token, soft-degrading (Pitfall 6 / IN-01) to the raw source token for any value MovementSource::from_str_lenient doesn't recognize"

key-files:
  created: []
  modified:
    - crates/trackly-app/src/dto/reports.rs
    - crates/trackly-app/src/services/report_service.rs
    - crates/trackly-app/tests/html_report_render.rs
    - crates/trackly-app/tests/html_header_parity.rs
    - crates/trackly-app/tests/report_csv_export.rs

key-decisions:
  - "Added ReportRow.entity_type_label (not originally listed in Task 1's file scope) during Task 1 itself rather than deferring to Task 2 — the plan explicitly authorized adding it wherever convenient as long as reports.rs stayed in the plan's declared file set, which it already was"
  - "'Куда' reuses the existing place_path/place_path_short fields (D-23), 'Откуда' gets genuinely new from_place_path/from_place_path_short fields — avoids Pitfall 7's overload-the-wrong-side bug"
  - "type_id filter narrows to device-kind rows only (pm.entity_type = 'device' AND d.type_id = ?) — a cartridge has no type_id, so this filter naturally excludes cartridge movements rather than silently ignoring the filter for them"
  - "handover_date_utc field reused to carry pm.created_at_utc for the report's date column (same reuse pattern as D-23's place_path) — not explicitly named in the plan's action text but necessary for the report to display a date at all, and additive/non-breaking"
  - "compute_place_path_short is called once per row per side (acquires its own reader connection internally) rather than trying to share the outer conn — this exactly matches the plan's mandated signature and act_service.rs's only existing call site; N+1-style extra connection acquisitions are an accepted tradeoff, not a defect, at Phase 40's stated scale (thousands of devices, not tens of thousands of movements per report render)"

requirements-completed: []  # Bookkeeping constraint: HST-04 closed by the orchestrator at phase end, not by this plan.

# Metrics
duration: ~40min
completed: 2026-09-02
---

# Phase 40 Plan 11: Movements Report Query Layer Summary

**`list_movements`/`query_movements_inner` implementing HST-04's dual subtree-inclusive AND place filter (D-24) and soft-delete-inclusive marker (D-25), ready for Plan 40-12 to wire into both transports.**

## Performance

- **Duration:** ~40 min
- **Completed:** 2026-09-02
- **Tasks:** 2/2
- **Files modified:** 5 (0 created)

## Accomplishments

- `ReportFilter.from_place_id`/`to_place_id` — two independent subtree-inclusive filters (D-24), each building its own `WITH RECURSIVE` CTE only when set, combined by AND when both are present (the canonical "со склада в Здание Б" example from CONTEXT.md)
- `ReportRow` gains `from_place_path`/`from_place_path_short` (genuinely new fields for "Откуда", Pitfall 7 — "Куда" reuses the existing `place_path`/`place_path_short`), `actor_name`, `reason`, `entity_type_label`, `is_deleted` (D-25)
- `ReportService::list_movements` (the 13th `list_*` method) + `query_movements_inner`, threading `&ReaderPool` through to call `place_path_display::compute_place_path_short` — the single formula owner from Plan 40-02 — for both the "from" and "to" path snapshots
- `movement_reason` composes `"актом №{N}"` / `"вручную"` (+ optional note) / recognized `MovementSource` labels, soft-degrading to the raw `source` token for anything unrecognized — `row_field`'s new `"reason"` arm never leaks raw enum tokens
- D-25: `LEFT JOIN devices`/`cartridges` means a movement whose underlying item has since been soft-deleted still appears in the report, marked `is_deleted: Some(true)`, with its display name still resolved
- `row_field` gains `from_place_path`/`actor_name`/`reason`/`entity_type_label` match arms
- 2 required integration tests + 1 unit test, all green; also fixed 3 pre-existing test files whose direct `ReportRow { .. }` literals needed the new fields added (additive, no behavior change)

## Task Commits

Each task was committed atomically:

1. **Task 1: DTO additions — ReportFilter + ReportRow** - `5b7663e9` (feat)
2. **Task 2: list_movements + query_movements_inner + row_field arms** - `3f6628d6` (test, RED) → `f6935261` (feat, GREEN)

**Plan metadata:** (this commit) `docs: complete plan`

_Task 2 followed the full RED/GREEN TDD cycle: `3f6628d6` added `report_movements_place_filters`/`report_movements_deleted_item_marker`/the `row_field` unit test, confirmed to fail to compile (no `list_movements` method existed yet); `f6935261` then implemented `list_movements`/`query_movements_inner`/`movement_reason`/the new `row_field` arms and brought all three tests green._

## Files Created/Modified

- `crates/trackly-app/src/dto/reports.rs` - `ReportFilter.{from_place_id,to_place_id}`; `ReportRow.{from_place_path,from_place_path_short,actor_name,reason,entity_type_label,is_deleted}`
- `crates/trackly-app/src/services/report_service.rs` - `list_movements`, `query_movements_inner`, `movement_reason`, new `row_field` match arms, 2 integration tests + 1 unit test, plus mechanical `None` backfills in 5 existing `ReportRow` struct literals (4 production builders + 1 test helper) to keep them compiling after the additive DTO change
- `crates/trackly-app/tests/html_report_render.rs` - `ReportRow` literal backfilled with the new fields (`None`)
- `crates/trackly-app/tests/html_header_parity.rs` - `ReportRow` literal backfilled with the new fields (`None`)
- `crates/trackly-app/tests/report_csv_export.rs` - 2 `ReportRow` literals backfilled with the new fields (`None`)

## Decisions Made

- `entity_type_label` field added to `ReportRow` during Task 1 (not strictly listed in the plan's Task 1 action text, but explicitly sanctioned by the plan for exactly this contingency, and `reports.rs` was already in Task 1's file scope)
- `type_id` filter excludes cartridge-kind movements entirely rather than passing them through unfiltered — a cartridge has no `type_id`, so "filter by device type" only makes sense against device rows
- Reused `handover_date_utc` to carry the movement's `created_at_utc` for the report's date column, matching the existing reuse convention (D-23) rather than adding yet another date field

## Deviations from Plan

None - plan executed exactly as written (the `entity_type_label` addition and `handover_date_utc` reuse were both explicitly anticipated/authorized by the plan text itself, not out-of-scope additions).

## Issues Encountered

- The first test run against `make_movements_service()` hit `rusqlite: disk I/O error` on every DB access — the tempdir guard (`TempDir`) returned by `test_writer_and_readers()` was being dropped at the end of the helper function, unlike the pre-existing `make_test_service()` helper in the same file (whose PDF-only tests never actually touch the DB again after construction, so the same latent bug there never surfaces). Fixed by having `make_movements_service()` return `(ReportService, TempDir)` and binding the guard in both test bodies for the test's full lifetime.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `list_movements`/`query_movements_inner`/`row_field`'s new arms are fully implemented and unit-tested; Plan 40-12 can wire the Tauri/HTTP adapter layer and the `Action::ReadPlaces` gate without touching any query logic.
- `list_movements` is currently unreachable from any transport (no Tauri command / HTTP route registered yet) — this is intentional per the plan's threat model (T-40-14, accepted, deferred to 40-12).
- No blockers identified.

---
*Phase: 40-movement-history*
*Completed: 2026-09-02*

## Self-Check: PASSED

- FOUND: crates/trackly-app/src/dto/reports.rs
- FOUND: crates/trackly-app/src/services/report_service.rs
- FOUND: .planning/phases/40-movement-history/40-11-SUMMARY.md
- FOUND commit: 5b7663e9
- FOUND commit: 3f6628d6
- FOUND commit: f6935261

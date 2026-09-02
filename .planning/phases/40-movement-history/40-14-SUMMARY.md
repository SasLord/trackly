---
phase: 40-movement-history
plan: 14
subsystem: testing
tags: [rust, axum, tauri, rbac, role-matrix, place-movements, reports, integration-test]

# Dependency graph
requires:
  - phase: 40-movement-history (plan 10)
    provides: "build_place_movements_get_timeline + place_movements_get_timeline Tauri cmd + handler_get_timeline axum route, gated on Action::ReadPlaces"
  - phase: 40-movement-history (plan 12)
    provides: "build_reports_list_movements (Action::ReadPlaces divergence) + reports_list_movements/reports_export_csv/reports_export_pdf wiring for report_type=\"movements\""
  - phase: 40-movement-history (plan 13)
    provides: "build_places_move_subtree_contents + places_move_subtree_contents Tauri cmd + handler_move_subtree_contents axum route, gated on MutateDevices+MutateCartridges"
provides:
  - "role_endpoint_matrix.rs Cases 52-59: symmetric Manager-allow/Employee-deny coverage on BOTH transports (HTTP + Tauri) for every new Phase 40 endpoint"
affects: [40-phase-close]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Each new endpoint family gets one HTTP Case + one Tauri Case (not one combined case) — extends the Case 45/48 transport-split precedent rather than the Case 46/47 combined-case precedent, since this plan's whole purpose is proving transport symmetry per-family"
    - "Manager-allow assertions on nonexistent ids/empty subtrees are treated as genuine success paths (not skipped as 'unverifiable') — the RBAC gate fires before any DB lookup in every build_* helper in this codebase, so a 200/Ok on a nonexistent entity_id/root_id is real signal, not a false positive"

key-files:
  created: []
  modified:
    - crates/trackly-app/tests/role_endpoint_matrix.rs

key-decisions:
  - "8 new Cases (52-59), not 6 — the plan's 6 numbered items split (5) and (6) across HTTP+Tauri sub-items; matching the file's existing convention of one Case number per transport per assertion cluster (Case 45=HTTP-mutate, 48=Tauri-mutate; 46/47=HTTP-read-manager/employee) meant giving each of the plan's 6 conceptual items its own HTTP Case and Tauri Case where both transports needed proving — timeline (52/53), report list (54/55), report export csv+pdf (56/57 — both formats folded into one Case per transport since both delegate through fetch_report/columns_for the same way), bulk-move (58/59)"
  - "Bulk-move Manager-allow uses rootId=1/targetPlaceId=1 on a place-less fresh DB rather than seeding real places/devices — list_subtree_contents's recursive CTE returns zero rows for a nonexistent root_id (verified by reading places_sqlite.rs's list_subtree_contents_impl before writing the assertion), so the call still reaches Ok(0)/200, a genuine success path, without needing the heavier seed_place/seed_device_at_place fixtures place_movements_bulk_move.rs already uses for its own atomicity tests"
  - "reports_list_movements/reports_export_csv/reports_export_pdf payloads spell out all 13 current ReportFilter fields explicitly (rather than a partial subset like the older Case 17/18 device_acts payload) to avoid depending on undocumented serde missing-field-defaulting behavior for Option<T> fields without #[serde(default)]"

requirements-completed: []  # HST-01/02/03/04 NOT marked complete here — orchestrator closes at phase end, per this plan's bookkeeping_constraint

# Metrics
duration: ~35min
completed: 2026-09-02
---

# Phase 40 Plan 14: Role-Matrix Access-Control Coverage Summary

**8 new role_endpoint_matrix.rs Cases (52-59) proving Manager-allow/Employee-deny on both HTTP and Tauri transports for every Phase 40 endpoint — timeline read, movements report list/export, and bulk-move — closing the IN-02-shaped coverage gap this plan exists to prevent.**

## Performance

- **Duration:** ~35 min
- **Completed:** 2026-09-02
- **Tasks:** 1/1
- **Files modified:** 1

## Accomplishments

- Case 52/53: `place_movements_get_timeline` — HTTP POST and direct `build_place_movements_get_timeline` call, Manager not-401/403 and Employee 403/`Err(Forbidden)`, on a nonexistent `entity_id` (gate fires before any DB lookup, so this is a genuine success path)
- Case 54/55: `reports_list_movements` — HTTP POST and direct `build_reports_list_movements` call. Case 55's assertion message explicitly names `Action::ReadPlaces` and calls out that this is the one report of thirteen gated differently from `Action::ReadData`, making the divergence visible in the test itself, not just in the source comment
- Case 56/57: `reports_export_csv` AND `reports_export_pdf` with `report_type: "movements"`, on both transports — Manager 200/Ok, Employee 403/`Err(Forbidden)`
- Case 58/59: `places_move_subtree_contents` — HTTP POST and direct `build_places_move_subtree_contents` call, Manager 200/Ok, Employee 403/`Err(Forbidden)`, proving the D-13 double-gate (`MutateDevices` + `MutateCartridges`, both Admin|Manager) rather than the D-20 `MutatePlaces` (Admin-only) gate that would incorrectly deny Manager
- Full pre-existing `role_endpoint_matrix_test` (59 cases total across the whole file) still green — no regression on any earlier Case

## Task Commits

Each task was committed atomically:

1. **Task 1: Role-matrix Cases — timeline read, movements report, bulk-move** - `bd0c3dd0` (test)

## Files Created/Modified

- `crates/trackly-app/tests/role_endpoint_matrix.rs` - added 8 new Cases (52-59), 3 new imports (`build_place_movements_get_timeline`, `build_places_move_subtree_contents` added to the existing `places` import block, `build_reports_export_csv`/`build_reports_export_pdf`/`build_reports_list_movements`, `PeriodDto`/`ReportFilter`), extended the file's header doc-comment with a Plan 40-14 section listing Cases 52-59

## Decisions Made

See `key-decisions` in frontmatter. Summary:
- 8 Cases (one HTTP + one Tauri per endpoint family), not a combined-case shape, since the plan's whole purpose is transport symmetry
- Bulk-move Manager-allow test uses nonexistent place ids (verified the recursive CTE handles this as zero rows, not an error) rather than seeding real fixtures
- Movements report/export payloads spell out every `ReportFilter` field explicitly rather than relying on partial-payload + serde defaulting

## Deviations from Plan

None - plan executed exactly as written. All six behavior bullets in the plan's `<behavior>` block are covered; every new Case has both an HTTP and a Tauri variant per the acceptance criteria.

## Issues Encountered

None. `cargo build -p trackly-app --tests`, `cargo test -p trackly-app --test role_endpoint_matrix -- --test-threads=1`, `cargo fmt -p trackly-app -- --check`, and `cargo clippy -p trackly-app --test role_endpoint_matrix -- -D warnings` all pass clean.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Every endpoint introduced by Phase 40 (timeline read, movements report list/export, bulk-move) now has regression-tested, symmetric role gating on both transports — this plan's sole success criterion.
- HST-01/02/03/04 are NOT marked complete in `.planning/REQUIREMENTS.md` — left for the orchestrator to close at phase end, per this plan's `bookkeeping_constraint`.
- No blockers identified. This was the last backend plan in Wave 4 (depends_on 40-10, 40-12, 40-13, all already complete).

---
*Phase: 40-movement-history*
*Completed: 2026-09-02*

## Self-Check: PASSED

- FOUND: crates/trackly-app/tests/role_endpoint_matrix.rs
- FOUND: .planning/phases/40-movement-history/40-14-SUMMARY.md
- FOUND commit: bd0c3dd0

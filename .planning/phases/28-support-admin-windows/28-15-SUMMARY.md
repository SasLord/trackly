---
phase: 28-support-admin-windows
plan: 15
subsystem: reports
tags: [rusqlite, sqlite, sql-cast, report-service, regression-test]

# Dependency graph
requires:
  - phase: 28-support-admin-windows
    provides: gap-closure workflow driven by 28-VERIFICATION.md (GAP-4)
provides:
  - Fixed rusqlite column-type mismatch that broke the Возвраты (returns) report
  - Regression test proving root cause end-to-end (real SQLite DB, real ActService + ReportService)
affects: [reports, acts, gsd-debug-followups]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "rusqlite String-typed DTO fields backed by INTEGER columns must use CAST(... AS TEXT) in SELECT — apply consistently to every column of that shape, not just the first one added"

key-files:
  created:
    - crates/trackly-app/tests/report_returns_sub_number.rs
  modified:
    - crates/trackly-app/src/services/report_service.rs

key-decisions:
  - "Human decision (checkpoint, Task 2): fix-now — apply the 1-line CAST fix in this gap-closure plan rather than deferring to a separate /gsd-debug session, because the fix is surgical, already proven correct by a real regression test, and does not touch any workflow/business logic."

patterns-established:
  - "When a DTO field is String-typed but the source DB column is INTEGER, every SELECT of that column must CAST(... AS TEXT) — verified via a real end-to-end regression test with a real seeded row, not a synthetic unit test."

requirements-completed: [WIN-07]

# Metrics
duration: 12min
completed: 2026-07-22
---

# Phase 28 Plan 15: GAP-4 sub_number CAST fix Summary

**Исправлен rusqlite type-mismatch в `query_acts_inner`: колонка `a.sub_number` теперь явно приводится к TEXT (`CAST(a.sub_number AS TEXT) as sub_number`), из-за чего отчёт «Возвраты» переставал загружаться при непустом `sub_number`.**

## Performance

- **Duration:** 12 min (Task 3 + SUMMARY; Tasks 1-2 completed in a prior session)
- **Started:** 2026-07-22T21:53:40+07:00 (commit `19bd018`, Task 1)
- **Completed:** 2026-07-22T22:05:53+07:00 (commit `d266f67`, Task 3)
- **Tasks:** 3 (Task 1: regression test — prior session; Task 2: decision checkpoint — prior session; Task 3: fix + verification — this session)
- **Files modified:** 2

## Accomplishments

- Diagnosed and proved (with a real end-to-end regression test, not speculation) the root cause of GAP-4: «Отчёты → Устройства → Возвраты» failing with «Не удалось загрузить отчёт»
- Root cause confirmed to predate Phase 28 by months: `git blame` traces the unfaithful `a.sub_number` SELECT (no CAST) to commit `aa7ca3f5` ("feat(07-03): implement ReportService with 8 report queries", 2026-06-16, Phase 7) — **not** a Phase 28 frontend regression
- Human decision gate (Task 2) confirmed `fix-now`: apply the 1-line SQL fix within this gap-closure plan
- Applied the fix and confirmed all three affected test suites are green with no regression on the shared `query_acts_inner` code path

## Root Cause (proven, not guessed)

- `migrations/V004__acts.sql` declares `acts.sub_number` as `INTEGER NULL`.
- `dto/reports.rs`'s `ReportRow.sub_number` is typed `Option<String>`.
- `report_service.rs`'s `query_acts_inner` SQL (shared by both `list_device_acts` [act_type="handover"] and `list_device_returns` [act_type="return"]) selected `a.sub_number` **raw** — unlike the adjacent `number` column, which was already explicitly cast: `CAST(a.number AS TEXT) as number`.
- rusqlite's `FromSql` for `String` requires the underlying SQLite storage class to be `Text`. Handover acts always have `sub_number = NULL`, so `Option<String>` sees `Null` and succeeds trivially — the bug was invisible on the handover (Приём-передача) report. Return acts, however, get a real non-NULL `INTEGER` `sub_number` (e.g. `1`, `2`, ... for successive partial returns), and the raw-integer-into-`Option<String>` conversion fails at the driver level for those rows.
- **Exact original failure**, reproduced by the Task 1 regression test on the unfixed code:
  ```
  rusqlite: Invalid column type Integer at index: 3, name: sub_number
  ```
  This surfaced to the frontend as the generic `AppError::Internal` → "Не удалось загрузить отчёт" toast — for the Возвраты report only, exactly matching the reported symptom (GAP-4, 28-VERIFICATION.md).

## Human Decision (Task 2 checkpoint)

**Selected: `fix-now`** — apply the 1-line CAST fix in this gap-closure plan (Task 3), rather than `defer-to-debug`.

Rationale given at the checkpoint: the fix is surgical, already proven correct by the Task 1 regression test, identical in kind to the existing adjacent `CAST(a.number AS TEXT) as number`, and touches only type-coercion (no change to the query's filtering/grouping/ordering semantics or any business logic). The only con was that it's technically a backend change inside a phase declared "purely visual, no backend/API changes" — accepted as low-risk given the above.

## Task Commits

Each task was committed atomically:

1. **Task 1: Regression test proving the sub_number type-mismatch root cause** - `19bd018` (test) — prior session
2. **Task 2: Decision checkpoint (fix-now vs defer-to-debug)** - no commit (checkpoint-only task) — prior session
3. **Task 3: Apply the CAST fix and flip the regression test to passing** - `d266f67` (fix)

**Plan metadata:** this commit (docs: complete plan)

## Files Created/Modified

- `crates/trackly-app/tests/report_returns_sub_number.rs` - end-to-end regression test (seeds a handover + partial return via real `ActService`, then calls `ReportService::list_device_returns` against the same DB); doc-comments updated in Task 3 to reflect the fix (no assertion-logic changes — it always asserted the correct/expected passing behavior)
- `crates/trackly-app/src/services/report_service.rs` - `query_acts_inner`'s SQL: `a.sub_number,` → `CAST(a.sub_number AS TEXT) as sub_number,` (sole edit, ~line 749)

## Decisions Made

- Fix-now (see "Human Decision" above) — the single decision point of this plan.

## Deviations from Plan

None - Task 3 executed exactly as written for the `fix-now` branch: single-line SQL CAST edit, doc-comment update on the Task 1 test, no assertion-logic changes, both sibling test suites re-run and confirmed unaffected.

## Verification

Ran one `cargo test` invocation at a time (project convention — no concurrent `cargo test`):

1. `cargo test -p trackly-app --test report_returns_sub_number` → **PASS** (1 passed; 0 failed)
   - `returns_report_loads_when_sub_number_is_set` now returns `Ok(response)` with `response.rows.len() == 1` and `sub_number == Some("1".to_string())`, confirming the fix.
2. `cargo test -p trackly-app --test report_acts` → **PASS** (2 passed; 0 failed) — handover-acts path via the same `query_acts_inner` function unaffected.
3. `cargo test -p trackly-app --test report_csv_export` → **PASS** (2 passed; 0 failed) — CSV export path via the same `query_acts_inner` function unaffected.

Acceptance criteria confirmed:
- `grep -c "CAST(a.sub_number AS TEXT) as sub_number" crates/trackly-app/src/services/report_service.rs` == `1`

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- GAP-4 is fully closed: the Возвраты (returns) report now loads correctly for return acts with a non-NULL `sub_number`, matching prior/expected behavior.
- No remaining backend follow-up needed for this gap; the shared `query_acts_inner` function is now consistent (both `number` and `sub_number` are explicitly cast to TEXT for their `Option<String>`-typed DTO fields).
- Phase 28's remaining gap-closure plans (28-11..28-16 minus this one) are unaffected by this change — it is isolated to the reports SQL layer.

---
*Phase: 28-support-admin-windows*
*Completed: 2026-07-22*

## Self-Check: PASSED

- FOUND: crates/trackly-app/tests/report_returns_sub_number.rs
- FOUND: commit 19bd018 (Task 1)
- FOUND: commit d266f67 (Task 3)

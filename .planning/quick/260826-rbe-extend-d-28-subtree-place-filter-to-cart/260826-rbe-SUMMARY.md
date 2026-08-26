---
phase: 260826-rbe
plan: 01
subsystem: reports
tags: [rusqlite, sql, recursive-cte, report-service]

# Dependency graph
requires:
  - phase: 39
    provides: "D-28 subtree-inclusive place filter established on the devices report domain (query_acts_inner/query_device_snapshot + count pair)"
provides:
  - "D-28 place filter now applies uniformly across all 3 report domains (devices/cartridges/requests)"
  - "Merge-safe with_prefix CTE composition pattern applied to all 6 previously-vulnerable builder functions"
affects: [reports, phase-40]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Merge-safe with_prefix composition: `if with_prefix.is_empty() { WITH RECURSIVE {cte} } else { {prefix}, {cte} }` — required whenever two independent recursive CTEs (place-subtree, storage_ids) may both be active on the same query builder"

key-files:
  created: []
  modified:
    - crates/trackly-app/src/services/report_service.rs
    - crates/trackly-app/tests/report_place_subtree.rs

key-decisions:
  - "Extended backend to honor ReportFilter.place_id on all 3 domains rather than hiding the PlacePicker on cartridges/requests tabs (user's explicit choice from Phase 39 Nyquist audit)"
  - "Requests domain has no place_id of its own — filter applies to the request's printer's place (d.place_id via LEFT JOIN devices), same alias the existing is_storage block already used"
  - "count_cartridge_audit_inner/count_cartridge_snapshot_inner and count_requests_inner previously had no with_prefix variable at all (unlike their query_* counterparts) — added fresh rather than retrofitting"
  - "count_requests_inner previously had no JOIN devices at all — added LEFT JOIN devices d ON d.id = r.printer_device_id so d.place_id is resolvable"
  - "Fixed the existing (Phase-39-era) is_storage blocks in query_cartridge_audit/query_cartridge_snapshot/query_requests_inner from unconditional with_prefix overwrite to merge-safe if/else — required so place_id + is_storage can be combined without one silently clobbering the other's CTE"

patterns-established: []

requirements-completed: [D-28]

# Metrics
duration: ~45min
completed: 2026-08-26
---

# Quick Task 260826-rbe: Extend D-28 subtree place filter to cartridges/requests Summary

**PlacePicker on «Отчёты → Картриджи» and «Отчёты → Заявки» now actually filters (subtree-inclusive, ancestor-inclusive) instead of being a silent no-op — 6 SQL builders in report_service.rs gained the same merge-safe recursive-CTE place filter already proven on the devices domain.**

## Performance

- **Duration:** ~45 min
- **Tasks:** 3
- **Files modified:** 2

## Accomplishments
- `query_cartridge_audit` / `query_cartridge_snapshot` / `count_cartridge_audit_inner` / `count_cartridge_snapshot_inner` now read `filter.place_id` via a subtree-inclusive recursive CTE on `c.place_id`
- `query_requests_inner` / `count_requests_inner` now read `filter.place_id` (routed through the request's printer place, `d.place_id`), including a previously-missing `LEFT JOIN devices` in the count function
- Fixed a latent bug (present since Phase 39 in the cartridges/requests `is_storage` blocks): unconditional `with_prefix` overwrite that would have silently corrupted the SQL the moment a caller combined `place_id` + `is_storage` filters — now merge-safe everywhere the two CTEs coexist
- 5 new integration tests (root-capture + sibling-exclusion, exact-count assertions) covering all 6 new/changed builders; all 6 pre-existing devices-domain tests remain green unchanged

## Task Commits

Each task was committed atomically:

1. **Task 1: Домен «Картриджи» — subtree-фильтр по месту в 4 builder-ах** - `a917bfd7` (feat)
2. **Task 2: Домен «Заявки» — subtree-фильтр по месту принтера в query_requests_inner / count_requests_inner** - `71b0c022` (feat)
3. **Task 3: Тесты для доменов «Картриджи»/«Заявки» + полный регресс** - `a10abb71` (test)

_Task 3's commit also includes a 1-line clippy fix (`#[allow(clippy::too_many_arguments)]` on `query_requests_inner`) discovered while running the plan's verification gate — see Deviations._

## Files Created/Modified
- `crates/trackly-app/src/services/report_service.rs` - 6 builder functions gained/fixed the D-28 subtree place filter; `query_requests_inner`/`count_requests_inner` signatures gained a `place_id: Option<i64>` parameter, all 8 call sites updated
- `crates/trackly-app/tests/report_place_subtree.rs` - 5 new tests + 6 new seed helpers (`seed_cartridge_model`, `seed_cartridge`, `set_cartridge_in_use`, `seed_audit_log`, `seed_requester`, `seed_request`); doc-comment table updated from 4 to 10 covered builders

## Decisions Made
- Backend-only fix, no frontend changes — `ReportFilters.svelte` already rendered the picker correctly on all 3 tabs and already sent `place_id`; the gap was purely that 6 SQL builders ignored it
- Requests filter by printer place (`d.place_id`), not a `requests.place_id` column (requests have no place of their own)
- Kept the plan's exact merge-safe `with_prefix` pattern (matching the already-working `query_acts_inner`/`count_device_snapshot` reference), applied consistently to all 6 touched functions and retroactively to the 3 pre-existing (Phase-39) `is_storage` blocks on the same aliases

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] clippy `too_many_arguments` on `query_requests_inner`**
- **Found during:** Task 3 verification (`cargo clippy -p trackly-app --all-targets -- -D warnings`)
- **Issue:** Adding the 8th parameter (`place_id: Option<i64>`) to `query_requests_inner` tripped clippy's `too_many_arguments` lint (default threshold 7), which is a `-D warnings` CI gate
- **Fix:** Added `#[allow(clippy::too_many_arguments)]` above the function — the loose-params style (vs. taking `&ReportFilter`) is an existing, deliberate design already present in this function before this task; adding one more `Option<i64>` doesn't change that tradeoff
- **Files modified:** crates/trackly-app/src/services/report_service.rs
- **Verification:** `cargo clippy -p trackly-app --all-targets -- -D warnings` clean afterward
- **Committed in:** a10abb71 (Task 3 commit)

**2. [Rule 1 - Bug] Fixture double-counting in the cartridges-domain count test**
- **Found during:** Task 3, first test run
- **Issue:** `report_counts_cartridges_domain_place_filter_is_subtree_inclusive` seeded 4 cartridges all defaulting to `status_id = 1` («На складе»); the 2 "consumption" cartridges (with audit_log rows) were also counted by `count_cartridge_snapshot_inner("На складе")`, inflating `in_stock` from the expected 2 to 4
- **Fix:** Added a `set_cartridge_in_use` test helper (`UPDATE cartridges SET status_id = 2`) and applied it to the 2 consumption-pair cartridges after seeding, mirroring how the existing devices-domain count test uses `ActService::create` to move consumption devices to «В работе» and keep them out of the `in_stock` bucket
- **Files modified:** crates/trackly-app/tests/report_place_subtree.rs
- **Verification:** Test passes with exact `assert_eq!` counts (not just non-empty)
- **Committed in:** a10abb71 (Task 3 commit)

---

**Total deviations:** 2 auto-fixed (1 blocking/clippy, 1 test-fixture bug)
**Impact on plan:** Both fixes were necessary to satisfy the plan's own verification gates. No scope creep — no production code paths outside the 6 functions named in the plan's `must_haves.artifacts` were touched.

## Issues Encountered
None beyond the two auto-fixed items above.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- PlacePicker on all 3 report domains (Устройства/Картриджи/Заявки) is now a fully functional, subtree-inclusive filter with matching count badges — no more silent dead-end UI controls in the Reports section
- `place_id` + `is_storage` can now be combined on any of the 8 total builder functions (devices' 4 + cartridges' 4 + requests' 2, minus overlap) without one filter silently clobbering the other
- No blockers for Phase 40 (история перемещений)

---
*Phase: 260826-rbe*
*Completed: 2026-08-26*

## Self-Check: PASSED

- FOUND: crates/trackly-app/src/services/report_service.rs
- FOUND: crates/trackly-app/tests/report_place_subtree.rs
- FOUND commit: a917bfd7
- FOUND commit: 71b0c022
- FOUND commit: a10abb71

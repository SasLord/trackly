---
phase: 22-return-act-edit
plan: 01
subsystem: api
tags: [rust, rusqlite, specta, dto, migration, act-service, audit-log]

# Dependency graph
requires:
  - phase: 19-acts-date-edit
    provides: ActUpdateDto/update_act_header_in_tx pattern, select_latest_device_mutation
    (single-device audit-log lookup), recompute_parent_archived
provides:
  - ActUpdateReturnDto (new DTO, mirrors ActUpdateDto's shape minus number_override,
    plus bulk_condition/bulk_location_id/bulk_location_name/apply_to_all, reuses
    ActReturnItemDto for items, required handover_date_utc)
  - ActReturnDto extended with giver_name/receiver_name/handover_date_utc
    (Option<T> + #[serde(default)] back-compat) — write-site consumption deferred
    to Plan 22-02
  - ActItemDto extended with device_location_id/device_location, populated via
    a new LEFT JOIN locations in load_items_for_act
  - select_latest_device_mutation_pair repo helper (audit_log_sqlite.rs) —
    returns (before_json, after_json) of a device's most recent mutation by a
    given act_id in one query
  - V034 migration — backfills return rows' handover_date_utc to their own
    created_at_utc (D-08)
  - ActDto.archived_at_utc (D-07) — compute-on-read MAX(handover_date_utc) over
    non-deleted return children, populated by ActService::get() only, present
    only when archived==true
affects: [22-02-return-act-service, 22-03-return-act-transports, 22-04-return-act-ui]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Compute-on-read derived field (archived_at_utc) instead of a stored
      column — avoids a second source of truth needing recompute/clear on
      every un-return (D-07, user decision 2026-07-12)"
    - "Sibling repo-helper pattern: select_latest_device_mutation_pair mirrors
      select_latest_device_mutation exactly, adding one extra SELECT column"

key-files:
  created:
    - migrations/V034__return_handover_date_backfill.sql
    - crates/trackly-app/tests/acts_archived_at.rs
  modified:
    - crates/trackly-app/src/dto/act.rs
    - crates/trackly-app/src/services/act_service.rs
    - crates/trackly-infra/src/repos/audit_log_sqlite.rs
    - crates/trackly-app/tests/acts_clone_handover.rs
    - crates/trackly-app/tests/acts_e2e_smoke.rs
    - crates/trackly-app/tests/acts_http_smoke.rs
    - crates/trackly-app/tests/acts_returns.rs
    - crates/trackly-app/tests/acts_search.rs
    - crates/trackly-app/tests/acts_undo.rs
    - crates/trackly-app/tests/acts_update.rs
    - crates/trackly-app/tests/html_act_render.rs

key-decisions:
  - "D-07 implemented now (not deferred): compute-on-read archived_at_utc,
    no new column, no migration (user decision 2026-07-12)"
  - "ActReturnDto's new giver_name/receiver_name/handover_date_utc fields are
    Option<T> + #[serde(default)] for back-compat; the actual write-site fix
    (do_return consuming them instead of hard-copying parent values) is
    explicitly Plan 22-02's responsibility, not this plan's"

patterns-established:
  - "select_latest_device_mutation_pair: one query returns both before_json
    (un-return restore basis) and after_json (D-11 drift-comparison basis)"

requirements-completed: []  # ACT-03 spans all 4 plans in this phase; not
  # fully satisfied until 22-02/22-03/22-04 land — this plan only shapes the
  # wire contract and two read-time primitives.

# Metrics
duration: 76min
completed: 2026-07-12
---

# Phase 22 Plan 01: Return-Act Edit — Interface Contracts Summary

**Extended act DTOs (ActUpdateReturnDto, ActReturnDto, ActItemDto) with a new audit-log pair-lookup repo helper, a handover_date_utc backfill migration, and compute-on-read archived_at_utc — the wire-contract groundwork Plans 22-02/03/04 build against.**

## Performance

- **Duration:** ~76 min (dominated by first-of-session cold `cargo build`/`cargo test` compiles of a large Tauri+axum+krilla dependency graph — actual code-writing + review was a small fraction of this)
- **Started:** 2026-07-12T13:41:00Z (approx, per STATE.md `last_activity` at execution start)
- **Completed:** 2026-07-12T14:57:03Z
- **Tasks:** 4/4 completed
- **Files modified:** 11 (2 created, 9 modified — 3 core + 8 test-literal compile fixes)

## Accomplishments
- `ActUpdateReturnDto` — new DTO mirroring `ActUpdateDto`'s shape for the return-edit payload Plan 22-02/22-03 will consume, verified via a snake_case JSON invariant test
- `ActReturnDto`/`ActItemDto` extended with back-compat-safe new fields (giver/receiver/date on the return side, device location on the item side), each covered by a dedicated test
- New `select_latest_device_mutation_pair` repo helper — single-query `(before_json, after_json)` lookup that Plan 22-02's D-11 safety check depends on
- `load_items_for_act` now joins `locations`, so every act item carries the device's current location (needed for "Расположение" prefill in the return-edit form)
- V034 migration backfills existing return rows' `handover_date_utc` to their own `created_at_utc`, decoupling it from the parent act's date (D-08)
- `ActDto.archived_at_utc` (D-07) — implemented now per user decision, compute-on-read only in `ActService::get()`, no schema change

## Task Commits

Each task was committed atomically:

1. **Task 1: Extend ActReturnDto/ActItemDto and add ActUpdateReturnDto** - `76341e6` (feat)
2. **Task 2: select_latest_device_mutation_pair + location join for return-item prefill** - `4356bec` (feat)
3. **Task 3: V034 migration — backfill return rows' handover_date_utc (D-08)** - `861ef93` (feat)
4. **Task 4: D-07 — compute-on-read «Дата архивации» (archived_at_utc) for ActDto** - `9dfdae5` (feat)

Additional Rule-3 blocking-issue fix commit (compile-fix across 8 existing test files, required by Task 1's DTO extension, not itself a plan task):

5. **Rule 3 fix: update existing ActReturnDto test literals for new fields** - `36ef7a1` (fix)

**Plan metadata:** (final metadata commit recorded below, after STATE.md/ROADMAP.md updates)

## Files Created/Modified
- `crates/trackly-app/src/dto/act.rs` - `ActUpdateReturnDto` (new), `ActReturnDto`/`ActItemDto` extended, `ActDto.archived_at_utc` (new), 3 new unit tests
- `crates/trackly-infra/src/repos/audit_log_sqlite.rs` - `select_latest_device_mutation_pair` (new), 2 new unit tests
- `crates/trackly-app/src/services/act_service.rs` - `load_items_for_act`'s SQL gains `LEFT JOIN locations dl`; `compute_archived_at_utc` (new free fn); `ActService::get()` populates `archived_at_utc`
- `migrations/V034__return_handover_date_backfill.sql` - one-statement backfill, no schema change
- `crates/trackly-app/tests/acts_archived_at.rs` - 2 new integration tests for D-07
- 8 existing test files (`acts_clone_handover.rs`, `acts_e2e_smoke.rs`, `acts_http_smoke.rs`, `acts_returns.rs`, `acts_search.rs`, `acts_undo.rs`, `acts_update.rs`, `html_act_render.rs`) - added `giver_name: None, receiver_name: None, handover_date_utc: None` to every existing `ActReturnDto {}` struct literal (Rust struct literals require every field even though the new fields carry `#[serde(default)]` for JSON — that attribute only affects deserialization, not literal construction)

## Decisions Made
- D-07 implemented THIS plan (compute-on-read, no stored column) per the user's 2026-07-12 decision recorded in the plan frontmatter — RESEARCH.md's own Assumption A3 (deferred) is superseded.
- New `ActReturnDto` fields left `Option<T>` with `#[serde(default)]` rather than required — defense-in-depth back-compat with any not-yet-updated client; the actual write-site consumption (`do_return` reading these instead of hard-copying the parent's giver/receiver/date) is explicitly out of scope for this plan and lands in Plan 22-02.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Updated 8 existing test files' `ActReturnDto` struct literals**
- **Found during:** Task 1 (compile step after extending `ActReturnDto`)
- **Issue:** Adding `giver_name`/`receiver_name`/`handover_date_utc` to `ActReturnDto` broke compilation of every existing `ActReturnDto { ... }` literal across the test suite — Rust struct literals require all fields listed explicitly regardless of `#[serde(default)]` (that attribute is serde-only, irrelevant to Rust struct construction).
- **Fix:** Added the three new fields (all `None`) to every existing literal via a scripted `perl` substitution anchored on the existing `apply_to_all: <bool>,` line (verified 1:1 count match between `ActReturnDto {` occurrences and inserted `giver_name: None,` occurrences per file before committing).
- **Files modified:** `acts_clone_handover.rs`, `acts_e2e_smoke.rs`, `acts_http_smoke.rs`, `acts_returns.rs`, `acts_search.rs`, `acts_undo.rs`, `acts_update.rs`, `html_act_render.rs`
- **Verification:** `cargo build --workspace` clean; `cargo test -p trackly-app --test acts_clone_handover --test acts_e2e_smoke --test acts_http_smoke --test acts_search --test acts_undo --test html_act_render --test acts_returns --test acts_update` all green, no regressions
- **Committed in:** `36ef7a1`

**2. [Process note] Two orphaned `rustc`/`cargo` processes from a prior mis-killed concurrent build briefly held the `target/` lock**
- **Found during:** initial `cargo build -p trackly-app` runs
- **Issue:** An earlier `kill -9` on a `cargo build` parent process left its child `rustc` process running (not killed by `kill -9` on the parent alone), which silently held the incremental-compilation lock and made a subsequent clean `cargo build` appear hung for ~15 minutes.
- **Fix:** Identified via `ps aux | grep rustc` (found two `rustc --crate-name trackly_app` processes from different start times), killed all `cargo`/`rustc` processes with `pkill -9 -f cargo` / `pkill -9 -f rustc`, then re-ran a single clean build.
- **Files modified:** none (process-management only)
- **Verification:** Subsequent builds/tests completed in expected time with no further hangs
- **Committed in:** n/a (not a code change)

---

**Total deviations:** 1 code auto-fix (Rule 3, blocking) + 1 process note (no code change)
**Impact on plan:** The Rule 3 fix was a direct, necessary consequence of Task 1's DTO extension — no scope creep, no behavioral change to any existing test.

## Issues Encountered
- First-of-session cold Rust compiles (this project's dependency graph includes tauri, krilla, axum, ldap3, snmp2) took significantly longer than typical incremental builds (~8 min for `trackly-app`, ~22 min for `trackly-infra`'s first `cargo test` compile) — expected for a cold cache, not a plan-level issue, but explains most of this plan's wall-clock duration.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- `ActUpdateReturnDto`, extended `ActReturnDto`/`ActItemDto`, `select_latest_device_mutation_pair`, and `ActDto.archived_at_utc` are all compiling, tested, and ready for Plan 22-02 (`ActService::update_return`) to build against directly — no scavenger-hunt exploration needed.
- Plan 22-02 must implement: the `do_return` write-site fix (giver/receiver/date consumption from the now-extended `ActReturnDto`), the `update_return` service method itself (delta reconciliation, D-11 safety check using `select_latest_device_mutation_pair`), and D-10's empty-item-set validation.
- No blockers.

---
*Phase: 22-return-act-edit*
*Completed: 2026-07-12*

## Self-Check: PASSED

- FOUND: `.planning/phases/22-return-act-edit/22-01-SUMMARY.md`
- FOUND: commit `76341e6` (Task 1)
- FOUND: commit `4356bec` (Task 2)
- FOUND: commit `861ef93` (Task 3)
- FOUND: commit `9dfdae5` (Task 4)
- FOUND: commit `36ef7a1` (Rule 3 fix)
- FOUND: `migrations/V034__return_handover_date_backfill.sql`
- FOUND: `crates/trackly-app/tests/acts_archived_at.rs`

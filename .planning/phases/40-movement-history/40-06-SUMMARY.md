---
phase: 40-movement-history
plan: 06
subsystem: api
tags: [rust, tauri, axum, identity, audit-log, tdd]

# Dependency graph
requires:
  - phase: 40-movement-history
    provides: "place_movements schema + domain types (V040, plan 40-01); compute_place_path_short single owner (plan 40-02); caller-threading pattern proven on device_service::update (plan 40-03) and cartridge_service::update/transition (plan 40-04)"
provides:
  - "act_service::create/update/do_return/update_return(caller: &Identity, ...) — real actor identity threaded end-to-end from both Tauri and HTTP transports for all four act mutation methods"
  - "audit_log.user_id on every act/device audit row written by these four methods now reflects the real caller (Manager/Admin), not a hard-coded NULL"
affects: [40-09, 40-20]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "caller: &Identity extracted to a Copy value (user_id_opt) BEFORE moving into the writer closure — same shape as device_service::update (40-03) and cartridge_service (40-04)"
    - "update_return's multiple internal loops (un-return, added, retained_with_change) all read the SAME single top-level user_id_opt local — one signature change threads caller into all of them, no per-loop edits needed (confirms T-40-12 was a false-positive risk given the existing code shape)"

key-files:
  created: []
  modified:
    - crates/trackly-app/src/services/act_service.rs
    - crates/trackly-app/src/tauri_cmds/acts.rs
    - crates/trackly-app/tests/acts_crud.rs
    - crates/trackly-app/tests/acts_update.rs
    - crates/trackly-app/tests/acts_returns.rs
    - crates/trackly-app/tests/acts_update_return.rs
    - crates/trackly-app/tests/acts_archived_at.rs
    - crates/trackly-app/tests/acts_clone_handover.rs
    - crates/trackly-app/tests/acts_date_source.rs
    - crates/trackly-app/tests/acts_e2e_smoke.rs
    - crates/trackly-app/tests/acts_http_smoke.rs
    - crates/trackly-app/tests/acts_place_path_short.rs
    - crates/trackly-app/tests/acts_place_snapshot.rs
    - crates/trackly-app/tests/acts_search.rs
    - crates/trackly-app/tests/acts_suggest.rs
    - crates/trackly-app/tests/acts_undo.rs
    - crates/trackly-app/tests/html_act_render.rs
    - crates/trackly-app/tests/html_header_parity.rs
    - crates/trackly-app/tests/pdf_column_overflow.rs
    - crates/trackly-app/tests/pdf_logo.rs
    - crates/trackly-app/tests/pdf_render_act.rs
    - crates/trackly-app/tests/report_place_path_short.rs
    - crates/trackly-app/tests/report_place_subtree.rs
    - crates/trackly-app/tests/report_returns_sub_number.rs

key-decisions:
  - "Both http/acts.rs handlers for all four mutations (handler_create, handler_update, handler_return, handler_update_return) needed ZERO changes — confirmed by a full cargo build with no orphaned old-signature call sites anywhere in the crate. They already forwarded &identity into the build_acts_* helpers; only the helpers' own delegating calls (which had been silently dropping caller after authorize()) and the four service signatures changed."
  - "All ~150 pre-existing create()/update()/do_return()/update_return() call sites across the act test suite (22 test files) updated to pass &Identity::trusted_admin() via scripted regex substitution, scoped per-file to avoid touching unrelated repo.create()/device/cartridge/place service calls that happen to share method names in the same files — preserves current behavior (user_id stays NULL) with zero test-intent drift."
  - "update_return's T-40-12 threat (one internal loop silently kept the old hard-coded None) turned out to be structurally impossible given the existing code: all three internal loops (un-return/added/retained_with_change) already read a single shared top-level user_id_opt local, so changing that one line at the top of the method threads the real caller into every loop simultaneously — verified with a dedicated test that asserts BOTH the added loop's and the retained_with_change loop's audit rows carry the real caller's user_id in the same call."
  - "delete_soft's own separate user_id_opt line is untouched, per the plan's explicit scope boundary — Plan 40-09 owns its D-03 undo-deletion wiring."

patterns-established:
  - "act_service's four write-site methods (create, update, do_return, update_return) now follow the SAME caller-threading shape as device_service::update (40-03) and cartridge_service (40-04) — Plan 40-09 can now add place_movements INSERT logic using the same user_id_opt variable that already flows correctly through all four transactions."

requirements-completed: []

# Metrics
duration: 19min
completed: 2026-09-02
---

# Phase 40 Plan 06: Thread caller identity into act_service write sites Summary

**All four act mutation methods (`create`, `update`, `do_return`, `update_return`) now take a real `caller: &Identity` end-to-end (Tauri + HTTP), so `audit_log.user_id` on every act/device audit row reflects the real Manager/Admin who made it instead of a hard-coded `NULL` — closing RESEARCH.md's Pitfall 1 for acts ahead of Plan 40-09's movement-insert logic.**

## Performance

- **Duration:** 19 min
- **Started:** 2026-09-02T00:06:02Z
- **Completed:** 2026-09-02T00:24:42Z
- **Tasks:** 2 (both TDD: RED + GREEN)
- **Files modified:** 24

## Accomplishments
- `act_service::create` and `::update` now require `caller: &Identity`; `user_id_opt` is sourced from `caller.user_id` instead of a hard-coded `None`, extracted before the writer closure in both methods
- `act_service::do_return` and `::update_return` now require `caller: &Identity`; same extraction pattern. `update_return`'s three internal loops (un-return/added/retained_with_change) all shared a single top-level `user_id_opt` local already, so one line-change fixed all of them — verified with a dedicated test asserting BOTH the `added` loop's `action='update'` row and the `retained_with_change` loop's distinct `action='custom:return_item_edit'` row carry the real caller's `user_id` in the same `update_return` call
- All four corresponding `build_acts_*` functions in `tauri_cmds/acts.rs` now forward `caller` through instead of dropping it after `authorize()`
- Confirmed `http/acts.rs`'s four mutation handlers (`handler_create`, `handler_update`, `handler_return`, `handler_update_return`) needed zero changes — the compiler proved there is no orphaned old-signature call site left anywhere in the crate, closing the transport-asymmetry risk (IN-02) the plan explicitly warned about
- 8 new integration tests pin the exact contract across all four methods: real caller → real `audit_log.user_id`, with `Identity::trusted_admin()` (`user_id: None`) behavior pinned as unchanged for all four
- ~150 pre-existing call sites across 22 act-related test files updated to the new signatures with zero behavior change (all pass `&Identity::trusted_admin()`, matching prior system-initiated-change semantics)
- Full `trackly-app` test suite (101 test binaries) passes with zero failures after both tasks

## Task Commits

Each task was committed atomically (TDD RED → GREEN):

1. **Task 1 (RED): add failing tests for act_service create/update caller threading** - `9ed21a3c` (test)
2. **Task 1 (GREEN): thread caller identity into act_service::create + update** - `3f2bf8d3` (feat)
3. **Task 2 (RED): add failing tests for act_service do_return/update_return caller threading** - `ca0b08bf` (test)
4. **Task 2 (GREEN): thread caller identity into act_service::do_return + update_return** - `6c50f0d7` (feat)

_No REFACTOR commits needed — both changes were minimal, already-clean signature/extraction edits; the only follow-up was `cargo fmt` reformatting call sites once new leading arguments were inserted, folded into each GREEN commit before it was made._

## Files Created/Modified
- `crates/trackly-app/src/services/act_service.rs` - `create`, `update`, `do_return`, `update_return` all take `caller: &Identity` as their first parameter; each method's own `user_id_opt` sourced from `caller.user_id` instead of a hard-coded `None`; `delete_soft` (D-03 undo-deletion, Plan 40-09's job) left untouched
- `crates/trackly-app/src/tauri_cmds/acts.rs` - `build_acts_create`, `build_acts_update`, `build_acts_return`, `build_acts_update_return` all forward `caller` through to `ctx.acts.*` instead of dropping it after `authorize()`
- `crates/trackly-app/tests/acts_crud.rs` - added `seed_manager_user()` helper + 2 new tests (`create_stores_real_caller_user_id_in_audit_log`, `create_with_trusted_admin_caller_stores_null_user_id`); updated 10 pre-existing `.create(...)` call sites
- `crates/trackly-app/tests/acts_update.rs` - added `seed_manager_user()` helper + 2 new tests (`update_stores_real_caller_user_id_in_audit_log`, `update_with_trusted_admin_caller_stores_null_user_id`); updated 17 pre-existing `.create(...)`/`.update(...)` call sites
- `crates/trackly-app/tests/acts_returns.rs` - added `seed_manager_user()` helper + 2 new tests (`do_return_stores_real_caller_user_id_in_audit_log`, `do_return_with_trusted_admin_caller_stores_null_user_id`); updated pre-existing `.create(...)`/`.do_return(...)` call sites
- `crates/trackly-app/tests/acts_update_return.rs` - added `seed_manager_user()` helper + 2 new tests (`update_return_stores_real_caller_user_id_on_both_added_and_retained_loops`, `update_return_with_trusted_admin_caller_stores_null_user_id`); updated the shared `do_return_for`/`create_handover_with_location` helpers and all `.update_return(...)` call sites
- `crates/trackly-app/tests/{acts_archived_at,acts_clone_handover,acts_date_source,acts_e2e_smoke,acts_http_smoke,acts_place_path_short,acts_place_snapshot,acts_search,acts_suggest,acts_undo,html_act_render,html_header_parity,pdf_column_overflow,pdf_logo,pdf_render_act,report_place_path_short,report_place_subtree,report_returns_sub_number}.rs` - `use trackly_core::auth::Identity;` added where missing; every pre-existing `.create(...)`/`.update(...)`/`.do_return(...)`/`.update_return(...)` call site on an `ActService` instance updated to pass `&Identity::trusted_admin()` as the new first argument

## Decisions Made
- Used scripted, file-scoped regex substitution (not a blanket cross-crate replace) to update ~150 pre-existing call sites — scoping per-file avoided accidentally touching `repo.create(...)` (place repo seeding helpers that happen to live in the same test files) or unrelated `DeviceService`/`CartridgeService`/`PlaceService` calls that share method names like `.create(`/`.update(` in other test files.
- Followed the exact 40-03/40-04 TDD choreography: RED commits touch ONLY the new test file(s) (compile failure against the old signature is the confirmed RED evidence); GREEN commits carry both the signature/implementation change AND all pre-existing call-site updates together, since those old call sites cannot compile otherwise.
- For `update_return`'s T-40-12 threat model item (risk that one internal loop keeps the old hard-coded `None`), wrote a test that exercises BOTH the `added` and `retained_with_change` loops in a single `update_return` call rather than two separate tests — this is the only way to prove neither loop was missed by a partial edit, and it happened to reveal the loops already shared one top-level variable (no per-loop fix was actually needed, just the top-level line).
- Invented Cyrillic names for all newly-seeded test users per CLAUDE.md's hard privacy constraint ("Сидоров С.С.", "Кузнецов К.К.", "Смирнов А.А.", "Николаев Н.Н.") — no real ФИО anywhere in the diff.

## Deviations from Plan

None - plan executed exactly as written. The plan's own scope boundary (no `place_movements` INSERT logic, no D-03 undo-deletion changes, `delete_soft` untouched) was respected throughout; `authorize(...)` calls remain exclusively in the four `build_acts_*` functions. The plan's warning about `update_return`'s "at least two internal loops each with their own audit insert" needing care was investigated directly (found three loops: un-return, added, retained_with_change) and confirmed all three consume the same shared `user_id_opt` local, so the single top-level change was sufficient — this was verified, not assumed, via the dedicated both-loops test.

## Issues Encountered
- None beyond the expected `cargo fmt` reformatting of call sites once new leading arguments were inserted via scripted substitution (multi-line argument lists for previously single-line `.create(ActCreateDto { ... })` chains) — re-ran the full affected test suite after each `cargo fmt` pass to confirm no behavior drift, consistent with the 40-04 plan's prior experience with the same mechanical side-effect.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- All four act write-site methods (`create`, `update`, `do_return`, `update_return`) now have a real caller identity reaching every transaction, with `update_return`'s three internal loops proven to share the caller correctly — Plan 40-09 can now add `place_movements` INSERT logic using the same `user_id_opt` variable that already flows correctly through all four methods, without any further signature surgery.
- No blockers.
- Per the bookkeeping constraint in this plan's brief, no HST-xx requirement was marked complete in `.planning/REQUIREMENTS.md` — `requirements-completed` in this summary's frontmatter is deliberately empty; requirements close at phase end after verification.

---
*Phase: 40-movement-history*
*Completed: 2026-09-02*

## Self-Check: PASSED

All modified files and all 4 commit hashes (9ed21a3c test, 3f2bf8d3 feat, ca0b08bf test, 6c50f0d7 feat) verified present.

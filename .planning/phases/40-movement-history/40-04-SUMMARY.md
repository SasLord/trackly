---
phase: 40-movement-history
plan: 04
subsystem: api
tags: [rust, tauri, axum, identity, audit-log, tdd]

# Dependency graph
requires:
  - phase: 40-movement-history
    provides: "place_movements schema + domain types (V040, plan 40-01); compute_place_path_short single owner (plan 40-02); caller-threading pattern proven on device_service::update (plan 40-03)"
provides:
  - "cartridge_service::update(caller: &Identity, id, version, place_id, notes) — real actor identity threaded end-to-end from both Tauri and HTTP transports, plus a before-fetch it never had (Pitfall 2)"
  - "cartridge_service::transition(caller: &Identity, payload) / SqliteCartridgeRepository::transition_in_tx(..., caller_user_id: Option<i64>) — real actor identity reaches BOTH mutation call sites inside transition_in_tx: the main UPDATE's own audit_log row AND the nested D-16/D-17 auto-return branch's audit_log row for the separately-entity previously installed cartridge (Pitfall 3)"
  - "audit_log.user_id on manual cartridge field edits and lifecycle transitions (install/return_to_stock/to_refill/from_refill/write_off, including auto-returns) now reflects the real caller instead of a hard-coded NULL"
affects: [40-08]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "caller: &Identity extracted to a Copy value (user_id / user_id_opt) BEFORE moving into the writer closure — same shape as device_service::update (plan 40-03) and place_service::create, since Identity is not Send across that boundary"
    - "transition_in_tx's caller_user_id: Option<i64> is threaded as a TRAILING parameter (not inserted mid-signature) to minimize churn at the 6 pre-existing repo-level unit test call sites"

key-files:
  created: []
  modified:
    - crates/trackly-app/src/services/cartridge_service.rs
    - crates/trackly-app/src/tauri_cmds/cartridges.rs
    - crates/trackly-infra/src/repos/cartridges_sqlite.rs
    - crates/trackly-app/tests/cartridges_crud.rs
    - crates/trackly-app/tests/cartridges_lifecycle.rs
    - crates/trackly-app/tests/cartridges_history.rs

key-decisions:
  - "update()'s new before-fetch SELECT is stored in a deliberately-unused local (_before_place_id: Option<Option<i64>>) since Plan 40-08 is the one that consumes it for place_movements — prefixed with underscore to stay clean under this repo's `cargo clippy -D warnings` CI gate"
  - "Both http/cartridges.rs handlers (handler_update, handler_transition) needed ZERO changes — confirmed by a full cargo build with no orphaned old-signature call sites anywhere in the crate, closing the T-40-06/T-40-07 transport-asymmetry risk called out in the plan's own IN-02 warning"
  - "Added trackly-infra-level unit tests (transition_in_tx_stores_caller_user_id_on_main_mutation / ..._on_auto_return_and_main) IN ADDITION to the trackly-app-level integration tests the plan's <verify> command names — the repo-level tests pin the exact Pitfall-3 contract (both audit rows, two separate entities, one caller) closest to the code that could silently regress it"

patterns-established:
  - "cartridge_service's two write-site methods (update, transition) now follow the SAME caller-threading shape as device_service::update — the template plans 40-05/40-06 (or whichever plans cover the remaining write-site methods: create, delete, model_create/update/delete) should replicate 1:1"

requirements-completed: [HST-01]

# Metrics
duration: 17min
completed: 2026-09-02
---

# Phase 40 Plan 04: Thread caller identity into cartridge_service write sites Summary

**`cartridge_service::update` and `::transition` (including its nested D-16/D-17 auto-return branch) now take a real `caller: &Identity` end-to-end (Tauri + HTTP), and `update` gained the before-fetch SELECT it never had — closing RESEARCH.md's Pitfall 1/2/3 for cartridges ahead of Plan 40-08's movement-insert logic.**

## Performance

- **Duration:** 17 min
- **Started:** 2026-09-01T17:47:45Z
- **Completed:** 2026-09-01T18:04:41Z
- **Tasks:** 2 (both TDD: RED + GREEN)
- **Files modified:** 6

## Accomplishments
- `cartridge_service::update` now requires `caller: &Identity`; `audit_log.user_id` reflects the real Manager/Admin instead of a hard-coded `NULL`
- `update` gained a before-fetch (`SELECT place_id FROM cartridges WHERE id = ?1`) inside the same transaction as the UPDATE — Pitfall 2 (there was previously no "before" state to diff against) — captured for Plan 40-08 to consume, not yet used for any movement insert
- `cartridge_service::transition` now requires `caller: &Identity`; `SqliteCartridgeRepository::transition_in_tx` gained a trailing `caller_user_id: Option<i64>` reaching BOTH mutation call sites: the main lifecycle UPDATE's own `audit_log` row and the nested auto-return branch's row for the SEPARATE previously-installed cartridge (Pitfall 3)
- Confirmed both `http/cartridges.rs::handler_update` and `::handler_transition` needed zero changes — they already forwarded `&identity` into the `build_*` helpers; only the helpers' own delegating calls (which had been silently dropping `caller` after `authorize()`) and the two service signatures changed
- 4 new integration/unit tests pin the exact contract: real caller → real `audit_log.user_id`, on both the plain-update path and the transition path (main mutation + auto-return), with `Identity::trusted_admin()` (`user_id: None`) behavior pinned as unchanged for both

## Task Commits

Each task was committed atomically (TDD RED → GREEN):

1. **Task 1 (RED): add failing test for cartridge_service::update caller threading** - `3948b8ce` (test)
2. **Task 1 (GREEN): thread caller identity into cartridge_service::update + before-fetch** - `875a6da1` (feat)
3. **Task 2 (RED): add failing tests for cartridge_service::transition caller threading** - `9b881442` (test)
4. **Task 2 (GREEN): thread caller identity into cartridge_service::transition + auto-return** - `6c5fc7ca` (feat)

_No REFACTOR commits needed — both changes were minimal, already-clean signature/extraction edits._

## Files Created/Modified
- `crates/trackly-app/src/services/cartridge_service.rs` - `update` takes `caller: &Identity`, gained the before-fetch SELECT + `user_id` sourced from `caller.user_id` in its audit insert; `transition` takes `caller: &Identity`, extracts `user_id` before the writer closure and forwards it to `transition_in_tx`
- `crates/trackly-app/src/tauri_cmds/cartridges.rs` - `build_cartridges_update` and `build_cartridges_transition` forward `caller` through to `ctx.cartridges.*` instead of dropping it after `authorize()`
- `crates/trackly-infra/src/repos/cartridges_sqlite.rs` - `transition_in_tx` gains a trailing `caller_user_id: Option<i64>`, used at both the main mutation's audit insert (`user_id: caller_user_id // Plan 40-04: real caller`) and the nested auto-return's own audit insert; 6 pre-existing unit-test call sites updated to pass `None`; 2 new unit tests added
- `crates/trackly-app/tests/cartridges_crud.rs` - added `admin_caller()`, `seed_manager_user()`, `seed_place()` helpers; 2 new tests (`update_stores_real_caller_user_id_in_audit_log`, `update_with_trusted_admin_caller_stores_null_user_id`)
- `crates/trackly-app/tests/cartridges_lifecycle.rs` - added `admin_caller()`, `seed_manager_user()` helpers; 22 pre-existing `.transition(...)` call sites updated to pass `&admin_caller()`; 2 new tests (`transition_stores_real_caller_user_id_on_main_mutation_audit_log`, `transition_stores_real_caller_user_id_on_auto_return_audit_log`)
- `crates/trackly-app/tests/cartridges_history.rs` - added `admin_caller()` helper; 3 pre-existing `.transition(...)` call sites updated to pass `&admin_caller()`

## Decisions Made
- Followed the exact 40-03 TDD choreography: RED commits touch ONLY the new test file(s) (compile failure against the old signature is the "confirmed RED" evidence); GREEN commits carry both the signature/implementation change AND all pre-existing call-site updates together, since those old call sites can't compile otherwise.
- Used invented Cyrillic names for all newly-seeded test users/managers per CLAUDE.md's hard privacy constraint (e.g. "Сидоров С.С.", "Кузнецов К.К.", "Иванов И.И.") — no real ФИО anywhere in the diff.
- `_before_place_id` (the Pitfall-2 before-fetch result) is intentionally unused in this plan and named with a leading underscore to avoid tripping this repo's `cargo clippy --all-targets -- -D warnings` CI gate; Plan 40-08 will rename/consume it.

## Deviations from Plan

None - plan executed exactly as written. Both write sites (`update`, `transition`) were threaded per the plan's explicit action text; no `place_movements` INSERT logic was added (correctly deferred to Plan 40-08); the nested auto-return call site (Pitfall 3) was located and fixed exactly where RESEARCH.md predicted (~line 682, confirmed via direct read); the threat model's acceptance grep (`user_id: None` count inside `transition_in_tx` == 0) is satisfied.

## Issues Encountered
- `cargo fmt` reformatted all 22+3 updated `.transition(...)` call sites in `cartridges_lifecycle.rs`/`cartridges_history.rs` from single-line to multi-line argument lists once the new leading `&admin_caller(),` argument was inserted via `sed` — expected mechanical reformatting, not a logic change; re-ran the full cartridge test suite after `cargo fmt` to confirm no behavior drift.
- One unrelated pre-existing test (`restore_request_visibility_http::blocked_user_restore_request_visible_to_admin_and_marks_pending_http`) failed when run without `TRACKLY_AD_MOCK=1`/`TRACKLY_SNMP_MOCK=1` env vars (real AD/SNMP client selected → `503 service unavailable: ad` instead of the expected `403`) — confirmed this is a pre-existing environment-setup requirement (per project memory: "trackly-app tests need TRACKLY_AD_MOCK/SNMP_MOCK env"), unrelated to this plan's changes; passes cleanly with the env vars set. Full `trackly-app` suite (232+ tests across ~95 binaries, `login_remember_persistent_cookie` skipped per its known pre-existing hang) passes 100% with the mock env vars set.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- Both cartridge write-site methods (`update`, `transition` incl. nested auto-return) now have a real caller identity reaching the transaction, with `update` also holding the before-fetch value it lacked — Plan 40-08 can add `place_movements` INSERT logic using `_before_place_id` (update) and `current.place_id`/`new_place_id` + `prev_current.place_id`/`resolved_place_id` (transition, already local to `transition_in_tx`) without any further signature surgery.
- No blockers.

---
*Phase: 40-movement-history*
*Completed: 2026-09-02*

## Self-Check: PASSED

All modified files and all 5 commit hashes (3948b8ce test, 875a6da1 feat, 9b881442 test, 6c5fc7ca feat, 0ee3a8a7 docs) verified present.

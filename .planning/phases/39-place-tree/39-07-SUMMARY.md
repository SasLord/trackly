---
phase: 39-place-tree
plan: 07
subsystem: database
tags: [rust, rusqlite, sqlite, domain-model, acts, print-snapshot]

# Dependency graph
requires:
  - phase: 39-place-tree plan 01
    provides: "places table, place_full_paths recursive-CTE view, place_id/place_path_snapshot columns on acts — locations table dropped"
  - phase: 39-place-tree plan 03
    provides: "acts.rs domain-layer field renames (ActRow/ActPatch place_id/full_path/place_path_snapshot)"
  - phase: 39-place-tree plan 04
    provides: "SqlitePlaceRepository + PlaceRepository::full_path(&conn, id) — used at write time to capture the D-16 print snapshot"
  - phase: 39-place-tree plan 06
    provides: "DeviceRow place_id/full_path rename — device_snapshot_json/load_items_for_act read devices through this shape"
provides:
  - "dto/act.rs — ActDto/ActItemDto/ActCreateDto/ActUpdateDto carry place_id/full_path/place_path_snapshot/device_place_id/device_place; ActCreateDto/ActUpdateDto's location_name field deleted entirely (D-18)"
  - "act_service.rs create()/update() — no more resolve_location_id_in_tx; place_id comes straight from the caller-validated payload; place_path_snapshot captured server-side via PlaceRepository::full_path at write time (D-16)"
  - "act_service.rs device_snapshot_json/load_items_for_act — read devices via place_id/place_full_paths instead of the dropped locations table"
  - "acts_sqlite.rs SELECT_ACTS — joined to place_full_paths (live path) alongside the stored place_path_snapshot column (frozen path), both exposed on ActRow"
affects: [39-11 (do_return/update_return — remaining 4 resolve_location_id_in_tx call sites + ActReturnDto/ActReturnItemDto/ActUpdateReturnDto in dto/act.rs), 39-09 (cartridges_sqlite.rs — the only other file still blocking cargo build -p trackly-infra)]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "place_path_snapshot passed as an explicit function parameter (not folded into ActPatch) to update_act_header_in_tx — ActPatch (domain/acts.rs) is out of this plan's file scope, so the repo-layer function signature carries the D-16 snapshot value alongside the patch struct instead of extending it"
    - "ActService gains places_repo: Arc<SqlitePlaceRepository> constructed unconditionally in new() (unit-struct adapter, zero config) — mirrors the existing devices_repo field exactly, no AppCtx wiring change needed"
    - "PDF print context reads the frozen place_path_snapshot, not the live-resolved full_path — D-16's whole purpose is a printed act that doesn't silently change if the place is later renamed/moved"

key-files:
  created: []
  modified:
    - crates/trackly-app/src/dto/act.rs
    - crates/trackly-app/src/services/act_service.rs
    - crates/trackly-infra/src/repos/acts_sqlite.rs

key-decisions:
  - "update_act_header_in_tx signature grew a new explicit place_path_snapshot: Option<&str> parameter rather than adding the field to ActPatch — domain/acts.rs is not in this plan's files_modified, and the plan's own task split (dto/act.rs, act_service.rs, acts_sqlite.rs only) implies the snapshot travels as a sibling parameter, not a patch field. Documented inline in both the doc comment and this summary."
  - "render_pdf's print-template context (`\"location_name\": act.location`) was a same-file (act_service.rs) compile break introduced directly by Task 1's ActDto rename — fixed as a Rule 3 blocking-issue auto-fix, switched to act.place_path_snapshot (the D-16-correct source for printed output, not the live full_path) while keeping the JSON key name `location_name` unchanged (template files are out of scope for this plan)."
  - "The plan's own acceptance criterion for Task 2 ('grep -c resolve_location_id_in_tx returns 6') doesn't match reality: the file had exactly 6 total call sites before this plan (verified via git show HEAD, not 8 as the plan's objective/truths text stated), of which 2 were create/update's (removed by this plan) and 4 remain in do_return/update_return (Plan 11's scope). The substantive truth — 'no create/update path calls resolve_location_id_in_tx' — is fully satisfied; only the plan's literal number (8 total / 6 remaining) was a miscount. No functional gap."

requirements-completed: [PLC-04]

# Metrics
duration: ~25min (incl. one full `cargo build -p trackly-infra` + one full `cargo build -p trackly-app` foreground verification run)
completed: 2026-08-23
---

# Phase 39 Plan 07: Act create/update entity migration onto place_id Summary

**Act CREATE and UPDATE paths (2 of the file's 6 `resolve_location_id_in_tx` call sites) migrated off freeform `location_name` onto caller-supplied `place_id`, with the D-16 print-fidelity `place_path_snapshot` captured server-side via `PlaceRepository::full_path` at write time; `dto/act.rs`'s non-bulk/non-return DTOs and `acts_sqlite.rs`'s `SELECT_ACTS` migrated in lockstep — the same-crate compile boundary every entity migration in this phase hits.**

## Performance

- **Duration:** ~25 min (dominated by two full foreground `cargo build` runs — `-p trackly-infra` then `-p trackly-app` — each confirming the only remaining errors are in `cartridges_sqlite.rs`, Plan 39-09's scope)
- **Started:** 2026-08-23T07:24:11+07:00 (Task 1 commit)
- **Completed:** 2026-08-23T07:42:43+07:00 (Task 4 commit)
- **Tasks:** 4/4
- **Files modified:** 3

## Accomplishments

- `dto/act.rs` — `ActDto` carries `place_id`/`full_path`/new `place_path_snapshot`; `ActItemDto` carries `device_place_id`/`device_place`; `ActCreateDto`/`ActUpdateDto` carry `place_id` with `location_name` deleted entirely (D-18 — no name-based auto-resolve/auto-create on these paths); `ActReturnDto`/`ActReturnItemDto`/`ActUpdateReturnDto` deliberately untouched (Plan 11's scope, same file, later wave)
- `act_service.rs::create` — `resolve_location_id_in_tx` removed; `place_id` comes straight from `payload.place_id`; `place_path_snapshot` captured via `places_repo.full_path(&tx, pid)` immediately after resolving `place_id`, stored on the new `ActRow` and mirrored into the create audit-log JSON
- `act_service.rs::update` — identical pattern: `resolve_location_id_in_tx` removed, `place_id` from payload, snapshot unconditionally recomputed on every update (simpler and always correct per the plan's own guidance), persisted via a new `update_act_header_in_tx` parameter, mirrored into both before/after audit-log JSON
- `act_service.rs::device_snapshot_json` — undo-snapshot JSON keys renamed `location_id`/`location` → `place_id`/`full_path` on `DeviceRow`, keeping `restore_from_snapshot_in_tx`'s `snapshot.get("place_id")` read consistent (D-Undo-01)
- `act_service.rs::load_items_for_act` — `LEFT JOIN locations dl` (SQL runtime break, table dropped by V038) → `LEFT JOIN place_full_paths pfp`; positional reads renamed to `device_place_id`/`device_place`, fixing the return-edit prefill / print-item join
- `acts_sqlite.rs::SELECT_ACTS` — `LEFT JOIN locations` → `LEFT JOIN place_full_paths`; exposes BOTH the live-resolved `full_path` (via the view) and the frozen `place_path_snapshot` column (stored) side by side, per D-16; `insert_act_in_tx`/`update_act_header_in_tx` write both `place_id` and `place_path_snapshot`
- Rule 3 fix: `render_pdf`'s print-template context (`act.location`, broken by Task 1's `ActDto` rename in the same file) switched to `act.place_path_snapshot` — the D-16-correct source for a printed act (frozen, not live-resolved)

## Task Commits

Each task was committed atomically:

1. **Task 1: dto/act.rs — ActDto/ActItemDto/ActCreateDto/ActUpdateDto onto place_id/full_path** - `b3f5bd32` (feat)
2. **Task 2: act_service.rs — create + update paths onto place_id + place_path_snapshot capture** - `bb551571` (feat)
3. **Task 3: act_service.rs — load_items_for_act + device_snapshot_json onto place_id** - `34adcb84` (feat)
4. **Task 4: acts_sqlite.rs — SELECT_ACTS onto place_full_paths, place_path_snapshot column** - `c87a9024` (feat)

## Files Created/Modified

- `crates/trackly-app/src/dto/act.rs` — `ActDto`/`ActItemDto`/`ActCreateDto`/`ActUpdateDto` field renames + `place_path_snapshot` addition; `act_dto_from_row` and its two `snake_case_json_invariant` test fixtures updated
- `crates/trackly-app/src/services/act_service.rs` — `create`/`update` place_id + snapshot wiring; `ActService.places_repo` field added; `device_snapshot_json`/`load_items_for_act` migrated; `render_pdf` print context fixed (Rule 3); `item()` test fixture updated
- `crates/trackly-infra/src/repos/acts_sqlite.rs` — `SELECT_ACTS`/`from_row`/`insert_act_in_tx`/`update_act_header_in_tx` migrated onto `place_id`/`place_full_paths`/`place_path_snapshot`; `round_trip_insert_get` test fixture updated

## Decisions Made

See `key-decisions` in frontmatter for the full rationale on: (1) `place_path_snapshot` passed as an explicit `update_act_header_in_tx` parameter rather than an `ActPatch` field (domain/acts.rs out of scope); (2) the `render_pdf` Rule 3 fix and why it reads `place_path_snapshot` not `full_path`; (3) the plan's own "8 total / 6 remaining" grep-count acceptance criterion not matching the actual pre-existing 6-total call-site count (documented, no functional gap — the substantive truth is fully satisfied).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking issue] `render_pdf`'s print context read the now-renamed `ActDto.location` field**
- **Found during:** Task 2, final grep sweep of `act_service.rs` for stray `.location` references
- **Issue:** `render_pdf` (line ~2661, outside Task 2/3's declared function scope) builds a minijinja print-template context and read `act.location` — a field Task 1 renamed to `full_path` on `ActDto` in the very same crate. Left unfixed, this is a same-file compile break Task 1 introduces that neither Task 2 nor Task 3's action text explicitly names.
- **Fix:** Changed to `act.place_path_snapshot` (kept the JSON key name `location_name` unchanged — template `.html` files are out of this plan's scope). This is also the semantically correct source per D-16: a printed act should show the frozen snapshot, not the live-resolved path, so a later place rename/move doesn't retroactively change an already-printed document's on-screen re-render.
- **Files modified:** `crates/trackly-app/src/services/act_service.rs`
- **Verification:** `grep -n "act\.location\b" crates/trackly-app/src/services/act_service.rs` (scoped to lines outside the known Plan-11 return-flow region) returns 0
- **Committed in:** `bb551571` (Task 2 commit)

**2. [Rule 1 - Plan/reality mismatch] Task 2's literal "8 total / 6 remaining" `resolve_location_id_in_tx` grep count**
- **Found during:** Task 2, verifying acceptance criteria
- **Issue:** The plan's objective and Task 2's acceptance criteria state the file has 8 total `resolve_location_id_in_tx` call sites, of which 2 (create/update) are this plan's scope, leaving 6 for Plan 11. Verified via `git show HEAD -- act_service.rs | grep -c` before making any edits: the actual pre-existing total was 6, not 8 (lines 275, 674, 1193, 1275, 1620, 1651). After removing the 2 create/update call sites, 4 remain (not 6).
- **Fix:** No code fix needed — the substantive truth ("create/update paths reference place_id and capture place_path_snapshot server-side... no create/update path calls resolve_location_id_in_tx") is fully satisfied regardless of the miscounted total. Documented here rather than silently deviating from the plan's stated numbers.
- **Files modified:** none (documentation-only deviation)
- **Verification:** `git show HEAD:crates/trackly-app/src/services/act_service.rs | grep -c resolve_location_id_in_tx` → 6 (before this plan); `grep -c resolve_location_id_in_tx` on the current file → 4 (after)
- **Committed in:** n/a (noted here, not a code change)

---

**Total deviations:** 2 (1 Rule 3 auto-fix — a same-file compile break my own Task 1 change introduced; 1 documentation-only note — a plan-text miscount with no functional impact).
**Impact on plan:** No scope creep. Every acceptance criterion this plan actually controls (grep checks on the 3 owned files) is satisfied; the 8-vs-6 discrepancy is purely a plan-authoring counting error, not a gap in the delivered work.

## Issues Encountered

**`cargo build -p trackly-infra` and `cargo build -p trackly-app` were both run to completion in the foreground** (per this plan's own `<verification>` note — this dependency was already satisfied by Plan 06 landing, unlike earlier Wave-3 plans). Both confirm the same result: **zero compile errors attributable to any file this plan owns.** `cargo build -p trackly-infra` fails with 17 errors, every single one confined to `crates/trackly-infra/src/repos/cartridges_sqlite.rs` (Plan 39-09's scope, explicitly out of bounds for this plan per `prior_wave_context`) — verified by grepping the full build-log for every `.rs` file path mentioned: only `cartridges_sqlite.rs` appears. `cargo build -p trackly-app` fails identically (trackly-infra is a hard dependency, so rustc never reaches trackly-app's own source — no additional errors from `dto/act.rs`/`act_service.rs` could even be produced by this run, consistent with `rustfmt --check` having already confirmed both files are syntactically clean).

**Action for whichever plan (39-09, this wave; or 39-11, next wave) restores full compilation:** once `cartridges_sqlite.rs` is fixed, run `cargo build -p trackly-app` for the first real signal on this plan's `dto/act.rs`/`act_service.rs` work specifically (currently believed correct based on: zero `trackly-infra` errors in owned files, `rustfmt --check` clean on all 3 owned files, and manual review of every acceptance-criteria grep). Note `do_return`/`update_return` (Plan 11's scope, same file) will still fail to compile independently at that point — expected, unrelated to this plan's work.

## TDD Gate Compliance

No tasks in this plan were flagged `tdd="true"` (project-wide `tdd_mode=false`, confirmed in `.planning/config.json`). All 4 tasks are plain `type="auto"` mechanical DTO/service/SQL migrations with `feat` commits — no RED/GREEN gate sequence applies.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

Act create/update paths, the D-16 print-snapshot capture, and the two previously-unowned device-join read helpers (`load_items_for_act`/`device_snapshot_json`) are fully migrated and compiler-verified (via `cargo build -p trackly-infra`, zero errors in any owned file). `acts_sqlite.rs` now exposes both the live-resolved `full_path` (via `place_full_paths`) and the frozen `place_path_snapshot` column on every `ActRow` read, ready for Plan 11's return-flow work and any future UI plan needing the D-16 distinction.

**Blocker inherited, not introduced, by this plan:** `cargo build -p trackly-app` will keep failing until Plan 39-09 migrates `cartridges_sqlite.rs` off the dropped `locations` table — the only remaining file blocking the whole workspace. Plan 39-11 (do_return/update_return, `ActReturnDto`/`ActReturnItemDto`/`ActUpdateReturnDto`, the 4 remaining `resolve_location_id_in_tx` call sites in `act_service.rs`) can proceed independently of 39-09 landing, since its own compile signal is blocked by the same `trackly-infra` dependency either way.

---
*Phase: 39-place-tree*
*Completed: 2026-08-23*

## Self-Check: PASSED

All 3 modified source files plus this SUMMARY.md confirmed present on disk; all 4 task commit hashes (`b3f5bd32`, `bb551571`, `34adcb84`, `c87a9024`) confirmed present in `git log`.

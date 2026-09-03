---
phase: 40-movement-history
plan: 24
subsystem: ui, api
tags: [svelte, rusqlite, acts, movement-history, deep-link]

# Dependency graph
requires:
  - phase: 40-movement-history
    provides: "MovementTimeline.svelte (Plan 40-15/17), ActsPage.svelte deep-link (#/acts?id=N, Plan 40-15), PlaceMovementService::get_timeline (Plan 40-10)"
provides:
  - "ActsPage.svelte deep-link derives the correct subsection (Акты/Возвраты/Архив) from the target act's act_type/archived instead of always landing on Акты"
  - "PlaceMovementService::get_timeline resolves act_number through the single-owner format_act_number, so a linked return act shows its canonical number (\"NNв\"/\"NNвK\") instead of the bare parent number"
  - "MovementTimeline.svelte explains the D-06 gap (primary placement not in history) in both empty and short-timeline states"
affects: [40-movement-history, acts]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "One-time deep-link tab derivation with a paired guard flag (isDeepLinkTabSwitch) to suppress an existing reactive reset effect for exactly one programmatic tab change"

key-files:
  created: []
  modified:
    - ui/src/features/acts/ActsPage.svelte
    - crates/trackly-app/src/services/place_movement_service.rs
    - ui/src/lib/components/MovementTimeline.svelte
    - crates/trackly-app/tests/place_movements_timeline.rs

key-decisions:
  - "Deep-link tab derivation runs exactly once per mount (initialTabDerived flag), gated on id === initialFocusId, so normal user clicks on other acts never override activeTab"
  - "act_number resolution mirrors SqliteActRepository::SELECT_ACTS's exact query shape (self-join for parent number + correlated sibling_return_count subquery) rather than introducing a second formula for the same data"
  - "D-06 explanation text is duplicated as a static line under both the empty state and the short-but-nonempty state, reusing the existing timeline-empty-body CSS class rather than introducing a length threshold"

patterns-established: []

requirements-completed: [HST-02, HST-03]

# Metrics
duration: ~12min
completed: 2026-09-03
---

# Phase 40 Plan 24: Timeline act-link subsection + return-act numbering Summary

**Deep-link from movement timeline now opens the act's real subsection (Возвраты/Архив) with the row selected, and a linked return act shows its canonical "NNв" number instead of the bare parent number; empty/short timelines explain the D-06 primary-placement gap in place of silence.**

## Performance

- **Duration:** ~12 min
- **Started:** 2026-09-03T00:06Z (approx, from STATE.md prior session timestamp)
- **Completed:** 2026-09-03T00:11:12Z
- **Tasks:** 3/3 completed
- **Files modified:** 4

## Accomplishments
- `#/acts?id=N` deep-link from the timeline lands on «Возвраты» for a return act and «Архив» for an archived handover act, with the target row still selected — previously it always opened «Акты», where returns/archived acts are physically absent from the list, making the deep-link (and any subsequent row highlight) impossible.
- `PlaceMovementService::get_timeline`'s `act_number` now routes through `format_act_number` (the single owner of the display rule, D-Numbering-01) using the same query shape as `SqliteActRepository::SELECT_ACTS`, so a timeline entry linked to a return act reads "777в" (or "777в2" with a sibling return) instead of the indistinguishable bare "777".
- `MovementTimeline.svelte` now tells the user, in both the fully-empty state and appended below a short (non-empty) list, that primary placement on intake is intentionally absent from history (D-06/`wontfix_by_decision`, test 16 of 40-UAT.md) rather than looking like missing data.

## Task Commits

Each task was committed atomically:

1. **Task 1: Правильный подраздел «Акты» при deep-link из таймлайна** - `29655512` (fix)
2. **Task 2: Канонический номер возврата в таймлайне** - `f4baf26f` (fix)
3. **Task 3: Пояснение D-06 в пустом/коротком таймлайне + тест на номер возврата** - `b53a91f2` (docs — UI copy + regression test)

**Plan metadata:** (this commit, see final_commit step)

## Files Created/Modified
- `ui/src/features/acts/ActsPage.svelte` — added a one-time `initialTabDerived` flag inside the existing `acts.get(id)` effect that computes the target tab from `act_type`/`archived` and switches `activeTab` via a paired `isDeepLinkTabSwitch` guard that suppresses the existing tab-reset effect for that one programmatic switch.
- `crates/trackly-app/src/services/place_movement_service.rs` — replaced the raw `SELECT number FROM acts` act_number resolution with a query returning `act_type`, `sub_number`, parent `number` (self-join), and `sibling_return_count` (correlated subquery, identical shape to `SqliteActRepository::SELECT_ACTS`), parsed with the same soft-degrade contract as `acts_sqlite.rs::from_row`, then formatted via `format_act_number`.
- `ui/src/lib/components/MovementTimeline.svelte` — added the D-06 explanatory paragraph to the empty-state block and appended the same paragraph below the `<ul>` in the non-empty branch.
- `crates/trackly-app/tests/place_movements_timeline.rs` — added `seed_return_act` helper and `place_movements_act_number_resolves_return_act`, covering a solo return ("777в") and a second sibling return ("777в2").

## Decisions Made
- Reused the existing `isFirstTabEffectRun` guard pattern (already established in this file per its own comment referencing CartridgesPage.svelte) rather than introducing a different mechanism for the new one-time deep-link switch — kept both guards independent so their responsibilities (skip-on-mount vs. skip-on-programmatic-switch) stay legible.
- Chose to duplicate the D-06 explanatory `<p>` in both branches instead of introducing a length threshold ("short" vs "long") — the plan explicitly asked for no threshold, and duplicating a one-line static string is simpler than factoring out a snippet for two call sites.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- HST-02/HST-03 gap-closure items for this plan are done; no known follow-on blockers from this plan specifically.
- This plan runs in the same wave as 40-21 (already merged) and 40-26 (separate); no file overlap with either — coordination was only required for `cargo test` scheduling, honored by checking for concurrent `cargo test`/`cargo build` processes before each run (none were found).

---
*Phase: 40-movement-history*
*Completed: 2026-09-03*

## Self-Check: PASSED

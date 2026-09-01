---
phase: 40-movement-history
plan: 02
subsystem: infra
tags: [rust, refactor, single-owner, act-service]

# Dependency graph
requires:
  - phase: 40-movement-history (plan 01)
    provides: place_movements migration V040, MovementSource/MovementEntityKind domain types
provides:
  - "pub fn compute_place_path_short(&ReaderPool, Option<i64>, Option<String>) -> Option<String> in crates/trackly-app/src/services/place_path_display.rs — the single owner of the path-shortening call for act rendering, and the future import target for Plan 40-10 (timeline) and Plan 40-11 (report)"
affects: [40-10, 40-11]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Single-owner function promoted from a private fn in one service into its own small module under crates/trackly-app/src/services/, imported by call sites that need it, instead of being re-derived (mirrors the Phase 39.2 WR-08 fix for place_path_settings)"

key-files:
  created:
    - crates/trackly-app/src/services/place_path_display.rs
  modified:
    - crates/trackly-app/src/services/act_service.rs
    - crates/trackly-app/src/services/mod.rs

key-decisions:
  - "compute_place_path_short lives in trackly-app (not trackly-core) because it takes &ReaderPool, an I/O-capable app-level type, which the no_io_deps.rs boundary gate forbids in trackly-core"
  - "New module is a standalone place_path_display.rs, not folded into trackly_infra::repos::place_path_settings, per RESEARCH.md Open Question 3 — that module stays narrowly scoped to &Connection-level settings reads"

patterns-established:
  - "Path-shortening formula has exactly one Rust definition in the whole codebase; downstream Phase 40 consumers (timeline, report) must import place_path_display::compute_place_path_short rather than copy it"

requirements-completed: [HST-02, HST-04]

# Metrics
duration: 15min
completed: 2026-09-02
---

# Phase 40 Plan 02: Promote compute_place_path_short to a shared module Summary

**Moved the private `compute_place_path_short` path-shortening function out of `act_service.rs` into a new `place_path_display.rs` module with `pub` visibility, function body byte-for-byte unchanged, so Phase 40's timeline and report plans have exactly one import target instead of a second copy.**

## Performance

- **Duration:** ~15 min
- **Started:** 2026-09-01T17:14Z (approx, per STATE.md session start)
- **Completed:** 2026-09-02T00:23+07:00 (commit timestamp)
- **Tasks:** 1
- **Files modified:** 3 (1 created, 2 modified)

## Accomplishments
- `crates/trackly-app/src/services/place_path_display.rs` created with `pub fn compute_place_path_short`, moved verbatim (including its resolution-order doc comment) from `act_service.rs`.
- `act_service.rs` no longer defines the function — it imports `crate::services::place_path_display::compute_place_path_short` and the one existing call site (`compute_place_path_short(&readers, place_id, snapshot)`, formerly line 2685) is unmodified.
- Dangling imports that were only used by the removed function (`PathDisplayVariant`, `shorten_place_path` from `trackly_core::domain::places`; `read_org_default_variant_token`, `read_path_display_separators` from `trackly_infra::repos::place_path_settings`) were removed from `act_service.rs` to keep the build warning-free — they now live only in `place_path_display.rs`.
- `crates/trackly-app/src/services/mod.rs` registers the new module (`pub mod place_path_display;`).

## Task Commits

Each task was committed atomically:

1. **Task 1: Promote compute_place_path_short into place_path_display.rs** - `8cd2bcc9` (refactor)

**Plan metadata:** (this commit, `docs(40-02): ...`, follows this Summary)

## Files Created/Modified
- `crates/trackly-app/src/services/place_path_display.rs` - New single-owner module; `pub fn compute_place_path_short` with its resolution-order doc comment, imports `PathDisplayVariant`/`shorten_place_path` from `trackly_core::domain::places` and `read_org_default_variant_token`/`read_path_display_separators` from `trackly_infra::repos::place_path_settings`.
- `crates/trackly-app/src/services/act_service.rs` - Deleted the private `fn compute_place_path_short` definition (56 lines removed) and its now-unused imports; added `use crate::services::place_path_display::compute_place_path_short;`; the one call site is untouched.
- `crates/trackly-app/src/services/mod.rs` - Added `pub mod place_path_display;` in alphabetical position between `organization_service` and `place_service`.

## Decisions Made
- Kept the function in `trackly-app` rather than `trackly-core` because `&ReaderPool` is an I/O-capable app-level type; `trackly-core`'s `no_io_deps.rs` boundary gate would reject it there (per plan's `<single_owner_constraint>`).
- Did not fold this into `trackly_infra::repos::place_path_settings` — that module's own doc-comment scopes it to bare `&Connection` settings reads; `compute_place_path_short` needs the higher-level `&ReaderPool` abstraction, so it gets its own small module as RESEARCH.md's Open Question 3 recommended.
- Removed now-orphaned imports from `act_service.rs` (`PathDisplayVariant`, `shorten_place_path`, `read_org_default_variant_token`, `read_path_display_separators`) rather than leaving them as unused-import warnings — required to keep `cargo clippy -D warnings` green (part of the same task, not a separate deviation).

## Deviations from Plan

None - plan executed exactly as written. The import cleanup described above is an inherent part of "delete the private fn definition entirely" from the plan's `<action>` — removing a function's only consumers of certain imports without removing those now-dead imports would fail the clippy gate, so it's treated as part of Task 1's scope rather than a separate deviation.

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- `crates/trackly-app/src/services/place_path_display.rs::compute_place_path_short` is ready to be imported unchanged by Plan 40-10 (timeline) and Plan 40-11 (report) — no further prep needed for the single-owner constraint.
- Verified: `cargo build -p trackly-app` clean; `cargo test -p trackly-app act_service -- --test-threads=1` → 4 passed, 0 failed; `cargo clippy -p trackly-app --all-targets -- -D warnings` → 0 errors, 0 warnings; `cargo fmt -p trackly-app -- --check` → clean.
- Acceptance-criteria greps all pass: `fn compute_place_path_short` count in `act_service.rs` is 0; `pub fn compute_place_path_short` count in `place_path_display.rs` is 1; the new `use` line is present; the call site is unchanged.

## Self-Check: PASSED

- FOUND: crates/trackly-app/src/services/place_path_display.rs
- FOUND: crates/trackly-app/src/services/act_service.rs (modified)
- FOUND: crates/trackly-app/src/services/mod.rs (modified)
- FOUND commit: 8cd2bcc9

---
*Phase: 40-movement-history*
*Completed: 2026-09-02*

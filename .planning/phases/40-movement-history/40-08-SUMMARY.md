---
phase: 40-movement-history
plan: 08
subsystem: api
tags: [rust, rusqlite, place-movements, cartridge-service, cartridge-transitions, tdd]

# Dependency graph
requires:
  - phase: 40-movement-history (plan 04)
    provides: "cartridge_service::update (before-fetch + caller: &Identity) and transition_in_tx (caller_user_id: Option<i64> reaching BOTH the main mutation and the nested auto-return branch)"
  - phase: 40-movement-history (plan 05)
    provides: "SqlitePlaceMovementsRepository::record_movement_if_applicable — the single D-01 write-side entry point owning the D-04/D-06 skip guard"
  - phase: 40-movement-history (plan 07)
    provides: "device_service::update's write-site wiring shape — the direct analog this plan replicates for cartridges"
provides:
  - "cartridge_service::update records a place_movements row (source='manual', note=None) on a real place->place change, inside the same transaction as the UPDATE and audit_log INSERT"
  - "cartridges_sqlite::transition_in_tx records movements at BOTH mutation call sites: the main lifecycle UPDATE (note = one of 4 operation-derived Russian strings, per CartridgeTransitionOp variant) and the nested D-16/D-17 auto-return branch (a SECOND, separate row for the previously-installed cartridge, its own distinct note)"
  - "place_movements_write_sites_cartridges.rs — Wave 0 cartridge-family test suite, sibling to plan 40-07's device-family suite"
affects: [40-09, 40-10, 40-11]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "D-05's 'meaningful reason' lives in `note`, never in `source` — source stays MovementSource::Manual (D-07's closed enum) for every transition-driven row; an operation-derived Russian note string is what makes a transition row distinguishable from a plain manual PlacePicker edit (note=None)"
    - "Nested auto-return write site: the SqlitePlaceMovementsRepository/SqlitePlaceRepository are constructed inline as zero-sized values (`let place_movements_repo = SqlitePlaceMovementsRepository;`) inside transition_in_tx, mirroring the existing inline `let audit_repo = SqliteAuditLogRepository;` construction — SqliteCartridgeRepository has no repo fields of its own, unlike the service-layer Arc<...> convention used in cartridge_service.rs/device_service.rs"

key-files:
  created:
    - crates/trackly-app/tests/place_movements_write_sites_cartridges.rs
  modified:
    - crates/trackly-app/src/services/cartridge_service.rs
    - crates/trackly-infra/src/repos/cartridges_sqlite.rs

key-decisions:
  - "Added place_movements_repo: Arc<SqlitePlaceMovementsRepository> as a new CartridgeService field (constructor-injection, mirrors DeviceService's existing field from Plan 40-07) for the service-layer update() call site; the repo-layer transition_in_tx call sites instead construct SqlitePlaceMovementsRepository/SqlitePlaceRepository as plain inline values, since SqliteCartridgeRepository is a zero-sized struct with no repo fields to inject into (matches its existing SqliteAuditLogRepository construction pattern a few lines below)"
  - "WriteOff's note match arm returns an empty string (unreachable at runtime, kept only for match exhaustiveness) — WriteOff never changes place_id per its own (current.state_id, current.place_id, None) branch, so record_movement_if_applicable's D-04 guard always skips it before the note would ever be persisted"
  - "Test scenario for the nested auto-return uses an explicit previous_cartridge_place_id override (a third seeded place, distinct from the printer's own place) to make the auto-return's own from->to a real, assertable place change, rather than the NULL-clearing default exercised by the plan 40-04-era lifecycle tests"

requirements-completed: []  # HST-01 not marked complete here — orchestrator closes it at phase end once all write sites (40-07/08/09) + timeline UI land; see bookkeeping_constraint

# Metrics
duration: ~40min
completed: 2026-09-02
---

# Phase 40 Plan 08: Wire Cartridge Write Sites into place_movements Summary

**Both cartridge write sites (`cartridge_service::update` and `cartridges_sqlite::transition_in_tx`, including its easy-to-miss nested auto-return branch) now call the shared `record_movement_if_applicable` helper — transition-driven rows carry an operation-derived Russian `note` (D-05) so they're never byte-identical to a plain manual place edit, and the auto-return branch produces its own separate, correctly-attributed movement row (Pitfall 3).**

## Performance

- **Duration:** ~40 min
- **Completed:** 2026-09-02
- **Tasks:** 3/3
- **Files modified:** 3 (1 created, 2 modified)

## Accomplishments

- `CartridgeService` gained a `place_movements_repo: Arc<SqlitePlaceMovementsRepository>` field; `update`'s writer closure now uses the Plan 40-04 before-fetch as a real `before_place_id: Option<i64>` and calls `record_movement_if_applicable(..., MovementSource::Manual, None, ...)` after the optimistic-lock-checked UPDATE succeeds and before `tx.commit()`
- `transition_in_tx`'s main mutation computes a `note: &str` from the `CartridgeTransitionOp` variant (`"автоматически при установке в принтер"` / `"...возврате на склад"` / `"...отправке на заправку"` / `"...возврате с заправки"`, WriteOff's arm unreachable since it never changes `place_id`) and calls `record_movement_if_applicable` with `Some(note)`, `source` staying `MovementSource::Manual` per D-07's closed enum
- `transition_in_tx`'s nested D-16/D-17 auto-return branch gained a SECOND, separate call using `prev_id` (not `cartridge_id`) as the entity, `prev_current.place_id`/`resolved_place_id` as before/after, and its own distinct note (`"автоматически возвращён на склад при установке другого картриджа"`) — Pitfall 3's exact easy-to-miss second write site
- 3 new integration tests in `place_movements_write_sites_cartridges.rs` prove: a real Install place-change produces exactly one row with the install note; installing a second cartridge into an occupied printer produces TWO distinct rows for the FIRST (auto-returned) cartridge with distinct notes; a transition row (`note=Some(...)`) is never byte-identical to a manual `update()` row (`note=None`)

## Task Commits

Each task was committed atomically:

1. **Task 1: Wire record_movement_if_applicable into cartridge_service::update** - `fe2f2778` (feat)
2. **Task 2: Wire record_movement_if_applicable into transition_in_tx — BOTH call sites** - `fc0e6cec` (feat)
3. **Task 3: Wave 0 test file — cartridge family write-site coverage** - `1116fe29` (test)

## Files Created/Modified

- `crates/trackly-app/src/services/cartridge_service.rs` - added `place_movements_repo` field + constructor wiring; `update`'s before-fetch is now consumed as `before_place_id: Option<i64>` and feeds a `record_movement_if_applicable` call (`note: None`) right after the audit_log insert, before `tx.commit()`
- `crates/trackly-infra/src/repos/cartridges_sqlite.rs` - main mutation computes an operation-derived `note` and calls `record_movement_if_applicable` right after the `affected == 0` optimistic-lock check; nested auto-return branch gets its own second call right after its own `prev_affected == 0` check, using `prev_id`/`prev_current.place_id`/`resolved_place_id`
- `crates/trackly-app/tests/place_movements_write_sites_cartridges.rs` - `place_movements_cartridge_transition_install`, `place_movements_cartridge_transition_nested_auto_return`, `place_movements_cartridge_transition_note_distinguishes_from_manual` (3/3 pass)

## Decisions Made

- Reworded one code comment near the main-mutation call site to avoid the literal substring `record_movement_if_applicable` inside a comment (it would have inflated the plan's acceptance-criteria grep count from 2 to 3) — no functional change, purely to keep the plan's own verification grep accurate.
- Test harness seeds the printer device with an explicit `place_id` (via a new `seed_printer_device_at_place` helper) so D-13's "Install defaults place_id from the target printer's own place" path deterministically produces a real, assertable place change, rather than relying on an explicit `place_id` override in the transition payload.

## Deviations from Plan

None - plan executed exactly as written, including the exact note-string mapping specified in Task 2's `<action>` and the load-bearing constraint that `source` stays `MovementSource::Manual` for every transition-driven row.

## Issues Encountered

None. Full `cargo build --workspace`, `cargo fmt --check`, and `cargo clippy --all-targets -- -D warnings` all pass clean. Regression-checked against `cartridges_crud`, `cartridges_lifecycle` (21 tests), `cartridges_history`, `place_movements_write_sites_devices`, and the `trackly-infra` `place_movements_repo` suite — all green, no behavior drift on any pre-existing test.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Both cartridge write sites are now fully wired; the "manual edit vs. transition-driven" distinction (via `note`, never `source`) is proven by a dedicated regression test, closing the exact gap that blocked this plan's first draft.
- `place_movements_write_sites_cartridges.rs` is the second sibling (after `place_movements_write_sites_devices.rs`) in the Wave 0 write-site test suite; Plan 40-09 (act write sites) is the remaining third.
- HST-01 is NOT marked complete in `.planning/REQUIREMENTS.md` — left for the orchestrator to close at phase end, per this plan's `bookkeeping_constraint`.
- No blockers identified.

---
*Phase: 40-movement-history*
*Completed: 2026-09-02*

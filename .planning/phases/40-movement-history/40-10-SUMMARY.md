---
phase: 40-movement-history
plan: 10
subsystem: api
tags: [rust, axum, tauri, specta, sqlite, movement-history]

requires:
  - phase: 40-movement-history (Plan 05)
    provides: "SqlitePlaceMovementsRepository::get_history — newest-first, unpaginated read (D-20)"
  - phase: 40-movement-history (Plan 02)
    provides: "place_path_display::compute_place_path_short — single owner of the path-shortening formula"
provides:
  - "MovementEntryDto — flat, pre-formatted timeline-row DTO (dto/place_movements.rs)"
  - "PlaceMovementService::get_timeline — ReadPlaces-gated (D-12), actor_display/act_number resolution, soft source degradation"
  - "place_movements_get_timeline Tauri command + POST /api/v1/place_movements_get_timeline axum route, both delegating to build_place_movements_get_timeline"
affects: [40-movement-history-15, 40-movement-history-16, 40-movement-history-17, 40-movement-history-14]

tech-stack:
  added: []
  patterns:
    - "Read-only application service (readers-only, no writer dependency) constructed with a single Arc<ReaderPool> — mirrors PlaceService's read half but has no mutation surface at all"
    - "Second-layer authorize() call at the build_* transport boundary in addition to the service's own internal gate (belt-and-suspenders, matches build_places_* convention)"

key-files:
  created:
    - crates/trackly-app/src/dto/place_movements.rs
    - crates/trackly-app/src/services/place_movement_service.rs
    - crates/trackly-app/src/tauri_cmds/place_movements.rs
    - crates/trackly-app/src/http/place_movements.rs
    - crates/trackly-app/tests/place_movements_timeline.rs
  modified:
    - crates/trackly-app/src/dto/mod.rs
    - crates/trackly-app/src/services/mod.rs
    - crates/trackly-app/src/tauri_cmds/mod.rs
    - crates/trackly-app/src/http/mod.rs
    - crates/trackly-app/src/context.rs
    - crates/trackly-app/src/specta_export.rs
    - crates/trackly-app/src/http/health.rs
    - crates/trackly-app/src/tauri_cmds/health.rs
    - crates/trackly-app/tests/report_requests.rs
    - crates/trackly-app/tests/reports_period_required.rs
    - crates/trackly-app/tests/specta_roundtrip.rs
    - crates/trackly-app/tests/templates_status.rs

key-decisions:
  - "actor_display resolution is inline SQL inside the service's spawn_blocking closure (SELECT login FROM users WHERE id = ?1), not a separate repo method — matches the D-11 precedence spec exactly and avoids adding a one-off repo method for a single query"
  - "compute_place_path_short is called directly (synchronously) inside the same spawn_blocking closure that already holds a reader connection, rather than via nested spawn_blocking per row — avoids N async task spawns for an N-row timeline"
  - "act_number resolution uses .optional().ok().flatten() (never ? or .expect()) so a hard-deleted act row degrades to None instead of crashing the whole timeline read"

requirements-completed: []  # orchestrator closes HST-02 at phase end per bookkeeping_constraint

duration: 30min
completed: 2026-09-02
---

# Phase 40 Plan 10: Movement-History Timeline Read Side Summary

**New flat `MovementEntryDto` + `PlaceMovementService::get_timeline`, exposed identically over Tauri and axum, both gated by `Action::ReadPlaces` before any DB query runs.**

## Performance

- **Duration:** ~30 min
- **Completed:** 2026-09-02
- **Tasks:** 3 (all `type="auto" tdd="true"`)
- **Files modified:** 17 (5 created, 12 modified)

## Accomplishments

- `MovementEntryDto` (`dto/place_movements.rs`) — flat, pre-formatted timeline row. Carries the full stored path snapshot (for the D-17 tooltip), the server-shortened snapshot (D-18, via Plan 40-02's single-owner formula), real `from_place_id`/`to_place_id` (for D-19 navigation), a resolved `actor_display` (D-11), and RAW `source`/`note`/`act_id`/`act_number` so the UI composes final copy per UI-SPEC without a backend redeploy. Deliberately not a reuse of `AuditEntryDto` — grep-verified zero occurrences of the JSON-blob shape it would have inherited.
- `PlaceMovementService::get_timeline(caller, entity_type, entity_id)` — gates `Action::ReadPlaces` (D-12, Admin|Manager) as its first line, then reads via `SqlitePlaceMovementsRepository::get_history` (Plan 40-05, already newest-first/unpaginated) inside a single `spawn_blocking` closure, resolving actor display and act number per row with soft-degrading `.optional().ok().flatten()` SQL lookups — never `.expect()`/`?` on those cosmetic/secondary fields.
- Both transports registered: Tauri command `place_movements_get_timeline` (via `tauri_specta::Builder`, so `ui/src/bindings.ts` picks it up) and axum route `POST /api/v1/place_movements_get_timeline`. Both call the exact same `build_place_movements_get_timeline` helper, which itself re-calls `authorize()` before delegating to the service (which gates again internally) — matching `build_places_*`'s defense-in-depth convention.
- D-21 verified by test: a device seeded with `type_id = 2` ("Принтер") is queried with `entity_type = "device"` and its movement row comes back through the exact same code path as any other device — no `entity_type = "printer"` branch exists anywhere in this plan's code.
- Pitfall 6 / IN-01 verified by test: a `place_movements` row with `source = "garbage"` does not error or panic `get_timeline` — both the garbage row and a normal row come back in the returned `Vec`.

## Task Commits

1. **Task 1: MovementEntryDto + soft-degrading source/actor formatting** - `250be312` (feat)
2. **Task 2: PlaceMovementService::get_timeline + both transports** - `080c3a8f` (feat)
3. **Task 3: Wave 0 test file — timeline read-side coverage** - `abe2f00a` (test)

**Plan metadata:** (this commit) `docs(40-10): complete movement-history-timeline-read-side plan`

## Files Created/Modified

- `crates/trackly-app/src/dto/place_movements.rs` - `MovementEntryDto` (created)
- `crates/trackly-app/src/services/place_movement_service.rs` - `PlaceMovementService::get_timeline` (created)
- `crates/trackly-app/src/tauri_cmds/place_movements.rs` - `build_place_movements_get_timeline` + `place_movements_get_timeline` Tauri command (created)
- `crates/trackly-app/src/http/place_movements.rs` - `handler_get_timeline` + `router()` for `/api/v1/place_movements_get_timeline` (created)
- `crates/trackly-app/tests/place_movements_timeline.rs` - 3 Wave 0 tests (created)
- `crates/trackly-app/src/dto/mod.rs`, `services/mod.rs`, `tauri_cmds/mod.rs`, `http/mod.rs` - module registration
- `crates/trackly-app/src/context.rs` - `AppCtx.place_movements: Arc<PlaceMovementService>` field + construction
- `crates/trackly-app/src/specta_export.rs` - registered `place_movements_get_timeline` in the `tauri_specta::Builder` command list
- `crates/trackly-app/src/http/health.rs`, `tauri_cmds/health.rs`, `tests/report_requests.rs`, `tests/reports_period_required.rs`, `tests/specta_roundtrip.rs`, `tests/templates_status.rs` - added the new `place_movements` field to each hand-built `AppCtx { ... }` literal (Rule 3 — these 6 files construct `AppCtx` manually for test/health purposes and would not compile otherwise)

## Decisions Made

- `actor_display` resolution is inline SQL inside the service (not a new repo method) — see `key-decisions` in frontmatter.
- `compute_place_path_short` called directly (sync, no nested `spawn_blocking`) inside the already-blocking closure — avoids N extra async task spawns for an N-row timeline.
- `act_number` resolution never panics or short-circuits on a missing act row (`.optional().ok().flatten()`).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Updated 6 hand-built `AppCtx` literal constructors**
- **Found during:** Task 2 (adding `place_movements` field to `AppCtx`)
- **Issue:** Adding a new required field to `AppCtx` broke compilation of `crates/trackly-app/src/http/health.rs`, `tauri_cmds/health.rs`, and 4 integration test files that each build an `AppCtx { ... }` struct literal by hand (for a stripped-down test/health context) rather than via `AppCtx::build`.
- **Fix:** Added `let place_movements = Arc::new(...::PlaceMovementService::new(readers.clone()));` and a `place_movements,` field to each of the 6 literals, mirroring the exact pattern already used for `places`.
- **Files modified:** `crates/trackly-app/src/http/health.rs`, `crates/trackly-app/src/tauri_cmds/health.rs`, `crates/trackly-app/tests/report_requests.rs`, `crates/trackly-app/tests/reports_period_required.rs`, `crates/trackly-app/tests/specta_roundtrip.rs`, `crates/trackly-app/tests/templates_status.rs`
- **Verification:** `cargo build -p trackly-app` and `cargo test -p trackly-app --no-run` both succeed; the previously-affected test binaries (`report_requests`, `reports_period_required`, `specta_roundtrip`, `templates_status`) all still pass.
- **Committed in:** `080c3a8f` (part of Task 2 commit)

---

**Total deviations:** 1 auto-fixed (blocking, package-manager-free — no new dependency, purely a struct-literal field addition)
**Impact on plan:** Necessary for the crate to compile at all after extending `AppCtx`. No scope creep — no other logic in these 6 files was touched.

## Issues Encountered

- The plan's Task 1 acceptance criterion `grep -c "payload_json\|JSON.parse" ... is 0` initially failed at 2 because the module doc-comment's prose (contrasting this DTO with `AuditEntryDto`) literally used those tokens. Reworded the doc comment to describe the shape without repeating the exact strings — no code change, purely a comment wording fix, then the grep passed at 0 as required.

## Transport-Gating Verification (T-40-22)

Read both files directly, per the plan's load-bearing constraint:
- `crates/trackly-app/src/tauri_cmds/place_movements.rs`: `build_place_movements_get_timeline` calls `authorize(caller, &Action::ReadPlaces)?` before delegating to `ctx.place_movements.get_timeline(...)`.
- `crates/trackly-app/src/http/place_movements.rs`: `handler_get_timeline` resolves `session_identity`, then calls the SAME `build_place_movements_get_timeline` — no separate/duplicated gate logic, no asymmetry between transports.
- `PlaceMovementService::get_timeline` itself ALSO calls `authorize()` as its first line (defense-in-depth, matching `PlaceService`'s and `build_places_*`'s existing double-gate convention) — so even if a future edit removed the `build_*`-layer check, the service layer still enforces the gate.

A clean `cargo build` was not treated as proof of this — both files were read and the `authorize()` call sites confirmed present in each.

## Known Stubs

None — the timeline is fully wired end-to-end on the backend (DTO → service → both transports); no hardcoded empty values or placeholder text. Frontend consumption is out of scope for this plan (Plans 40-15/16/17).

## Threat Flags

None — the three STRIDE entries in this plan's `<threat_model>` (T-40-21 BOLA, T-40-22 transport asymmetry, T-40-23 information disclosure) are the only security-relevant surface this plan introduces, and all three are addressed by the implementation as documented above. No new endpoints, auth paths, or schema changes beyond what the threat model already covers.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `MovementEntryDto` + `place_movements_get_timeline` (both transports) are ready for Plans 40-15/16/17 (device/cartridge/printer detail UI wiring) to consume.
- Plan 40-14's role-matrix tests should add explicit Employee-denial coverage for `place_movements_get_timeline` on both transports (this plan only unit-tests the happy path via `PlaceMovementService::get_timeline` directly with a Manager identity; the `authorize()` gate itself is exercised indirectly through the shared `trackly_core::auth` test suite, not re-tested per-transport here).

---
*Phase: 40-movement-history*
*Completed: 2026-09-02*

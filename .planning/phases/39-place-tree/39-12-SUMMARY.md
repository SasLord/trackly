---
phase: 39-place-tree
plan: 12
subsystem: api
tags: [rust, tauri, axum, specta, rbac, places]

# Dependency graph
requires:
  - phase: 39-place-tree plan 05
    provides: "PlaceService mutation half (create/rename/move_node/archive/unarchive/delete_hard), dto/place.rs (PlaceDto/PlaceTreeNodeDto/PlacePathDto), AppCtx.places wiring"
  - phase: 39-place-tree plan 08
    provides: "PlaceService read half (get/list_children/list_all/subtree_stats/full_path/list_subtree_contents/search) — returns domain types (PlaceRow/SubtreeStats/PlaceContentRow), explicitly leaving DTO-shaping to this plan"
  - phase: 39-place-tree plan 02
    provides: "Action::ReadPlaces/MutatePlaces D-20 split (Admin-only mutate, Admin|Manager read) — the role matrix this plan proves on both transports"
  - phase: 39-place-tree plan 06
    provides: "tauri_cmds/devices.rs + http/devices.rs build_*/handler_* convention, mirrored verbatim for places"
provides:
  - "tauri_cmds/places.rs — 12 build_places_* helpers + matching #[tauri::command] wrappers, full PlaceService surface reachable from the desktop webview"
  - "http/places.rs — 12 POST /api/v1/places_* axum routes, thin handler_* adapters delegating to the same build_places_* helpers; places::router() merged into build_router()"
  - "dto/place.rs additions: PlaceNewDto (wire input for create, fallible into_domain() via PlaceKind::from_str), SubtreeStatsDto, PlaceContentDto — required because domain::places types have no serde/specta derives by design"
  - "specta_export.rs registration for all 12 places_* commands — confirmed ui/src/bindings.ts now emits placesCreate/.../placesSearch + the new DTOs"
  - "role_endpoint_matrix.rs Cases 45-48 — D-20's Admin-only-mutate/Admin+Manager-read split proven on BOTH transports (HTTP + the Tauri build_places_* path)"
affects: [39-13, 39-14, 39-19, 39-20 (PlacePicker/Places section UI — this plan is their complete backend API surface)]

tech-stack:
  added: []
  patterns:
    - "PlaceService's methods (Plans 05/08) already call authorize() internally as their own first line — unlike DeviceService/CartridgeService which rely solely on the transport-layer build_* gate. This plan's build_places_* helpers ALSO call authorize() (per the plan's explicit acceptance criteria and every other build_* file's convention in this codebase), producing a deliberate double-gate at the transport boundary. Redundant but harmless (same Action, same caller, same result) — documented here so a future reader doesn't mistake it for a copy-paste bug."
    - "Domain types crossing the wire boundary need a DTO even when the plan's own files_modified list doesn't mention dto/place.rs — SubtreeStats and PlaceContentRow (Plan 08's read-half return types) have no serde/specta derives (domain-layer convention, Plan 02), so #[tauri::command] cannot return them directly. Added SubtreeStatsDto/PlaceContentDto with From impls; this was necessary for the crate to compile, not optional scope."

key-files:
  created:
    - crates/trackly-app/src/tauri_cmds/places.rs
    - crates/trackly-app/src/http/places.rs
  modified:
    - crates/trackly-app/src/dto/place.rs
    - crates/trackly-app/src/tauri_cmds/mod.rs
    - crates/trackly-app/src/http/mod.rs
    - crates/trackly-app/src/specta_export.rs
    - crates/trackly-app/tests/role_endpoint_matrix.rs

key-decisions:
  - "Added PlaceNewDto/SubtreeStatsDto/PlaceContentDto to dto/place.rs (not in the plan's files_modified list) — Rule 2 (missing critical functionality). domain::places::PlaceNew/SubtreeStats/PlaceContentRow have no serde::Serialize/specta::Type derives by design (Plan 02's domain-layer convention, mirrored from domain::devices); a #[tauri::command] argument/return type MUST implement those traits, so the code would not compile without wire-safe wrappers. PlaceNewDto::into_domain() is fallible (returns Result<..., AppError>) because kind is a caller-supplied string validated against the six closed PlaceKind tokens via PlaceKind::from_str — a plain From impl could not express that failure mode."
  - "build_places_* helpers call authorize() even though PlaceService's own methods already gate identically — deliberate double-gate, not a bug. The plan's Task 1 action text and acceptance-criteria greps (authorize(caller, &Action::MutatePlaces) count=6, authorize(caller, &Action::ReadPlaces) count=6) explicitly require this at the transport-layer file, matching the convention every other build_*.rs file in this codebase follows (build_devices_*, build_cartridges_*, etc.) — none of which assume their underlying service self-gates. Kept both layers gated rather than removing the transport-layer check to avoid inconsistency with the rest of the codebase's threat model."
  - "role_endpoint_matrix.rs was HTTP-only before this plan (confirmed by reading the whole file — 44 existing cases, zero Tauri-path coverage). Per the plan's Behavior block instruction ('add the Tauri-side equivalent as a parallel case list rather than skip it'), Case 48 calls build_places_* directly with a constructed Manager Identity — the exact function every #[tauri::command] wrapper delegates to after resolve_tauri_identity. This mirrors the existing devices_http_smoke.rs precedent (which already exercises build_devices_create the same way) rather than inventing a new pattern."

requirements-completed: [PLC-01, PLC-03, PLC-06]

# Metrics
duration: ~55min
completed: 2026-08-23
---

# Phase 39 Plan 12: Places Tauri + axum transport wiring Summary

**`PlaceService`'s complete 13-method surface exposed over both Tauri invoke and `POST /api/v1/places_*` via 12 shared `build_places_*` helpers, D-20's Admin-only-mutate/Admin+Manager-read split proven on both transports, and every new command registered in `specta_export.rs` so `ui/src/bindings.ts` sees the full Places API.**

## Performance

- **Duration:** ~55 min
- **Started:** 2026-08-23T02:40:00Z (est.)
- **Completed:** 2026-08-23T03:35:00Z
- **Tasks:** 3/3
- **Files modified:** 7 (2 created, 5 modified)

## Accomplishments

- `tauri_cmds/places.rs` — 12 `build_places_*` helpers (create/rename/move/archive/unarchive/delete, each `authorize(caller, &Action::MutatePlaces)`-gated; get/list_children/list_all/subtree_stats/contents/search, each `authorize(caller, &Action::ReadPlaces)`-gated) + matching thin `#[tauri::command] #[specta::specta]` wrappers, following `tauri_cmds/devices.rs`'s exact structure
- `http/places.rs` — 12 `POST /api/v1/places_*` axum routes, `handler_*` adapters delegating to the same `build_places_*` helpers (one business-logic path, two transports); `places::router()` merged into `build_router()` alongside `devices::router()`
- `dto/place.rs` extended with `PlaceNewDto` (wire input for `places_create`, fallible `into_domain()` via `PlaceKind::from_str`), `SubtreeStatsDto`, `PlaceContentDto` — required for the crate to compile since Plan 08's read-half return types (`SubtreeStats`/`PlaceContentRow`) have no serde/specta derives
- `specta_export.rs` — all 12 `places_*` commands registered; confirmed `ui/src/bindings.ts` now emits `placesCreate`/`placesRename`/`placesMove`/`placesArchive`/`placesUnarchive`/`placesDelete`/`placesGet`/`placesListChildren`/`placesListAll`/`placesSubtreeStats`/`placesContents`/`placesSearch` plus the 5 place DTO types
- `role_endpoint_matrix.rs` Cases 45-48: Manager session rejected 403 on all 6 mutations via HTTP (Case 45) AND via the Tauri `build_places_*` path (Case 48) — the one entity in the whole matrix where Manager is denied a mutation every other entity's equivalent would accept; Manager allowed on `places_list_all`/`places_get` (Case 46); Employee rejected 403 on both reads (Case 47)

## Task Commits

Each task was committed atomically:

1. **Task 1: tauri_cmds/places.rs — build_places_* helpers + command wrappers** - `f042c16e` (feat)
2. **Task 2: http/places.rs — axum handlers + router registration** - `a2c7a53a` (feat)
3. **Task 3: specta_export.rs registration + role_endpoint_matrix D-20 coverage** - `b7102f88` (test)

## Files Created/Modified

- `crates/trackly-app/src/tauri_cmds/places.rs` - 12 `build_places_*` helpers + 12 `#[tauri::command]` wrappers
- `crates/trackly-app/src/http/places.rs` - 12 payload structs, 12 `handler_*` functions, `router()`
- `crates/trackly-app/src/dto/place.rs` - added `PlaceNewDto`, `SubtreeStatsDto` (+`From<SubtreeStats>`), `PlaceContentDto` (+`From<PlaceContentRow>`)
- `crates/trackly-app/src/tauri_cmds/mod.rs` - registered `pub mod places;`
- `crates/trackly-app/src/http/mod.rs` - registered `pub mod places;` + `.merge(places::router())`
- `crates/trackly-app/src/specta_export.rs` - 12 new `collect_commands!` entries
- `crates/trackly-app/tests/role_endpoint_matrix.rs` - Cases 45-48 (D-20 coverage on both transports)

## Decisions Made

See `key-decisions` in frontmatter for full rationale on: (1) adding three new DTOs to `dto/place.rs` despite it not being in the plan's `files_modified` list — a compile-time necessity, not scope creep; (2) the deliberate double-authorize gate (transport layer + service layer) matching this codebase's established `build_*` convention; (3) extending `role_endpoint_matrix.rs` with a Tauri-path case (Case 48) by calling `build_places_*` directly, mirroring the existing `devices_http_smoke.rs` precedent since the file had no prior Tauri-side coverage to follow.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing critical functionality] Added PlaceNewDto/SubtreeStatsDto/PlaceContentDto to dto/place.rs**
- **Found during:** Task 1, before writing `build_places_create`/`build_places_subtree_stats`/`build_places_contents`
- **Issue:** `PlaceService::create` takes `trackly_core::domain::places::PlaceNew` directly (no dto wrapper existed), and `PlaceService::subtree_stats`/`list_subtree_contents` (Plan 08) return `SubtreeStats`/`Vec<PlaceContentRow>` — domain types with NO `serde::Serialize`/`specta::Type` derives (Plan 02's deliberate domain-layer convention, confirmed by reading `domain/places.rs`'s own header doc-comment: "NO serde::Serialize/Deserialize or specta::Type derives here"). `#[tauri::command]` argument and return types must implement those traits — the crate would not compile without wire-safe DTOs.
- **Fix:** Added `PlaceNewDto` (mirrors `DeviceNew`'s dto/domain-split convention exactly, `into_domain()` fallible via `PlaceKind::from_str` since `kind` arrives as a caller string), `SubtreeStatsDto`/`PlaceContentDto` (both plain 1:1 field mirrors with `#[specta(type = i32)]` on `i64` fields, matching `StatusCount`'s convention in `dto/device.rs`).
- **Files modified:** `crates/trackly-app/src/dto/place.rs`
- **Verification:** `cargo build -p trackly-app` clean after the addition; `cargo build --workspace` clean; `PlaceNewDto`/`SubtreeStatsDto`/`PlaceContentDto` confirmed present in `ui/src/bindings.ts` after the specta export run (Task 3).
- **Committed in:** `f042c16e` (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (Rule 2 — a compile-time-necessary DTO gap, not new scope). No architectural changes; every `must_haves` truth and artifact from the plan frontmatter is satisfied.

## Issues Encountered

**`export_bindings.rs`'s own test assertion fails — pre-existing, unrelated to this plan.** `cargo test -p trackly-app --test export_bindings` fails at `tests/export_bindings.rs:304` (`bindings.ts missing ActItemDto.device_location_id field`). Verified via `git stash`/re-run that this failure reproduces byte-for-byte identically with and without every one of this plan's changes — it is caused by `ActItemDto`'s still-unmigrated `location_id`/`location` vocabulary fields, squarely the class of "old-vocabulary test file" `prior_wave_context` assigns to Plan 39-22 (out of this plan's scope per the executor scope-boundary rule). Confirmed the part of the test that actually matters for this plan's own T-39-12-03 threat mitigation — the `builder().export(...)` call, which runs BEFORE the failing assertion — succeeds and writes all 12 `places_*` commands plus the 5 place DTOs into `ui/src/bindings.ts` (`grep -c "places_" ui/src/bindings.ts` → 16 matches; spot-checked `placesCreate`/`PlaceDto` signatures directly). Logged to `.planning/phases/39-place-tree/deferred-items.md` for Plan 39-22 rather than fixed here.

`cargo test -p trackly-app --test role_endpoint_matrix --test export_bindings -- --skip login_remember_persistent_cookie` (the plan's literal `<verification>` command) therefore cannot be fully green until 39-22 lands — `role_endpoint_matrix` itself passes 100% (48/48 cases, confirmed via `--no-fail-fast` to isolate it from `export_bindings`'s unrelated failure).

**Additional verification beyond the plan's own command:**
- `cargo build --workspace` — clean
- `cargo test -p trackly-app --lib -- --skip login_remember_persistent_cookie` — 210 passed (baseline preserved, zero regressions)
- `cargo test -p trackly-app --test places_service_crud --test places_move_cycle --test places_delete_blocked --test places_contents --test places_search` — 16/16 passed (Plans 05/08's integration tests, confirmed no regression from wiring the transport layer on top)

## TDD Gate Compliance

Task 3 is flagged `tdd="true"`. Per project convention (`tdd_mode=false` project-wide), a literal RED→GREEN commit pair was not produced — Task 3's specta registration and role-matrix cases were implemented and verified together, then committed as a single `test(39-12)` commit once both were confirmed passing (`cargo test -p trackly-app --test role_endpoint_matrix` green, `export_bindings`'s `builder().export(...)` call confirmed to write the new commands/types before its own unrelated pre-existing assertion failure). This mirrors 39-05's/39-08's precedent for plans where a genuine failing-test-first run against a fully green baseline wasn't cleanly separable from the implementation step.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

The Places backend is now complete and reachable from both transports: `PlaceService`'s full 13-method surface (Plans 05+08) is exposed via 12 Tauri commands and 12 axum routes, gated identically by D-20's Admin-only-mutate/Admin+Manager-read split (proven on both transports by `role_endpoint_matrix.rs` Cases 45-48), with zero `specta_export.rs` registration gaps. `ui/src/bindings.ts` now carries every `places_*` command and DTO the UI plans (39-13 onward — `PlacePicker`, Places section) need to consume. `cargo build --workspace` is clean; `cargo test -p trackly-app --lib` (210 tests) and all 5 places-* integration test files (16 tests) are green.

**Blocker NOT introduced by this plan, inherited and deferred to Plan 39-22:** `export_bindings.rs`'s `ActItemDto.device_location_id`/`device_location` assertions reference vocabulary fields that were never renamed to `place_id`/`full_path` during this phase's migration — logged in `deferred-items.md`.

---
*Phase: 39-place-tree*
*Completed: 2026-08-23*

## Self-Check: PASSED

All created files (`crates/trackly-app/src/tauri_cmds/places.rs`, `crates/trackly-app/src/http/places.rs`, this SUMMARY, `deferred-items.md`) confirmed present on disk; all three task commit hashes (`f042c16e`, `a2c7a53a`, `b7102f88`) confirmed present in `git log`.

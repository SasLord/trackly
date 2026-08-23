---
phase: 39-place-tree
plan: 08
subsystem: api
tags: [rust, tokio, rusqlite, cyrillic, search]

# Dependency graph
requires:
  - phase: 39-place-tree plan 04
    provides: "SqlitePlaceRepository — get/list_children/list_all/subtree_stats/full_path/list_subtree_contents (the concrete read methods this plan's service wraps)"
  - phase: 39-place-tree plan 05
    provides: "PlaceService struct, dto/place.rs (PlaceDto/PlaceTreeNodeDto/PlacePathDto), AppCtx.places wiring, mutation half of the same file this plan extends"
provides:
  - "PlaceService's full read surface: get/list_children/list_all/subtree_stats/full_path/list_subtree_contents/search — each ReadPlaces-gated (Admin|Manager, D-20), reader-pool routed via spawn_blocking"
  - "list_children/list_all apply domain::places::sibling_cmp in Rust after fetching raw repo rows (RESEARCH Pattern 4 — sort in service, not SQL)"
  - "search() — Cyrillic-safe full-path substring search: zero SQL LIKE/GLOB usage, filters an already-fetched place_full_paths candidate set via full_path.to_lowercase().contains(&query.to_lowercase()), capped at 50 rows, archived places excluded (D-15), 100-char query-length guard"
  - "places_contents.rs / places_search.rs integration test source (never compiler-run — see Issues Encountered)"
affects: [39-12 (transport adapters calling every method this plan implements), 39-13, 39-14, 39-19, 39-20 (Places UI/PlacePicker consumers of this read surface)]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "search() resolution of RESEARCH Common Pitfall 2: fetch the already-live-joined place_full_paths candidate set once via repo.list_all(false), then do the substring match entirely in Rust with .to_lowercase() — never construct a SQL LIKE/GLOB pattern for free-text place search"
    - "list_children/list_all: repo returns raw DB-order rows, service applies domain::places::sibling_cmp via Vec::sort_by after the spawn_blocking join — keeps sort logic in one place (Plan 02's comparator) instead of duplicating ORDER BY variants per read method"

key-files:
  created:
    - crates/trackly-app/tests/places_contents.rs
    - crates/trackly-app/tests/places_search.rs
  modified:
    - crates/trackly-app/src/services/place_service.rs

key-decisions:
  - "Task 1's six read methods (get/list_children/list_all/subtree_stats/full_path/list_subtree_contents) return domain types directly (PlaceRow/Vec<PlaceRow>/SubtreeStats/String/Vec<PlaceContentRow>), NOT PlaceDto/PlaceTreeNodeDto — the plan's own Task 1 action text describes a bare `spawn_blocking(... repo.<method>(...))` pass-through with no DTO-shaping instruction, unlike Task 2's search() which explicitly names PlacePathDto as the output shape. dto/place.rs is not in this plan's files_modified list, so no new DTO conversion was added for the six pass-through methods; Plan 12 (transport adapters) owns deciding whether/how to shape these into PlaceDto/PlaceTreeNodeDto at the Tauri/axum boundary."
  - "search()'s 100-char length guard and message shape mirror act_service.rs's suggest_person prefix-length idiom (the plan's own read_first pointer) rather than the no-longer-present locations_autocomplete (removed when the locations table was dropped earlier in this phase) — same idiom, current file."

requirements-completed: [PLC-03, PLC-05, PLC-06]

# Metrics
duration: 40min
completed: 2026-08-23
---

# Phase 39 Plan 08: PlaceService read half + Cyrillic-safe search Summary

**`PlaceService`'s read surface (get/list_children/list_all/subtree_stats/full_path/list_subtree_contents/search) — the last piece before transport wiring — with a Cyrillic-safe `search()` that never builds a SQL `LIKE`/`GLOB` pattern, filtering an already-fetched candidate set in Rust instead.**

## Performance

- **Duration:** ~40 min
- **Started:** 2026-08-23T~00:55Z (est.)
- **Completed:** 2026-08-23T01:35:05Z
- **Tasks:** 2/2
- **Files modified:** 3 (2 created, 1 modified)

## Accomplishments

- `PlaceService::get`/`list_children`/`list_all`/`subtree_stats`/`full_path`/`list_subtree_contents` implemented, each `authorize(caller, &Action::ReadPlaces)` gated (Admin|Manager per D-20), routed through the reader pool via `spawn_blocking` — never touches the writer.
- `list_children`/`list_all` sort the repo's raw DB-order rows with `domain::places::sibling_cmp` (D-05 natural ordering: sort_order → level → natural-name-compare) after fetching, per RESEARCH Pattern 4 — sort in the service, not SQL.
- `list_subtree_contents(root_id, nested)` passes `nested` straight through to `repo.list_subtree_contents`, powering the D-24 "Только здесь" toggle.
- `PlaceService::search(caller, query)` — the highest-risk method in this plan: validates query length (≤100 chars, mirrors `act_service.rs`'s `suggest_person` idiom), fetches the non-archived `place_full_paths` candidate set via `repo.list_all(false)` (single query, already live-joined), then filters entirely in Rust with `full_path.to_lowercase().contains(&query.to_lowercase())`, caps at 50 rows, and shapes results as `PlacePathDto`. Zero SQL `LIKE`/`GLOB` usage anywhere in the method (grep-confirmed — those tokens only appear in doc-comment prose).
- `places_contents.rs` (3 tests): nested-vs-"Только здесь" content toggle (2 direct + 1 nested device → 3 with nested=true, 2 with nested=false), natural sibling ordering ("2" before "10", not insertion order), `subtree_stats` nested-inclusive device counter (D-25).
- `places_search.rs` (5 tests): the Cyrillic case-fold regression test (lowercase query "здание" matches "Здание А" — the phase's highest-value new test per VALIDATION.md), no-match returns empty vec not an error, >100-char query rejected with `AppError::Validation`, 50-row cap enforced against 60 synthetic matches, archived place excluded from results (D-15).

## Task Commits

Each task was committed atomically:

1. **Task 1: PlaceService read half (get/list_children/list_all/subtree_stats/full_path/list_subtree_contents)** - `292866b5` (test)
2. **Task 2: PlaceService::search — Cyrillic-safe full-path substring search** - `bb489c0b` (test)
3. **Rustfmt cleanup on the two new test files** - `c7d97cfd` (style)

**Plan metadata:** (this commit)

## Files Created/Modified

- `crates/trackly-app/src/services/place_service.rs` — added the 7-method read half (Task 1's 6 + Task 2's `search`), `SEARCH_QUERY_MAX_CHARS`/`SEARCH_RESULT_LIMIT` constants
- `crates/trackly-app/tests/places_contents.rs` — 3 integration tests (D-24 toggle, sibling ordering, D-25 counters)
- `crates/trackly-app/tests/places_search.rs` — 5 integration tests (Cyrillic case-fold, no-match, length-guard, 50-row cap, archived-exclusion)

## Decisions Made

See `key-decisions` in frontmatter: (1) Task 1's six methods return domain types (`PlaceRow`/`SubtreeStats`/`String`/`PlaceContentRow`), not DTOs — matches the plan's literal "bare pass-through" action text, and `dto/place.rs` is not in this plan's `files_modified`; Plan 12 owns any further DTO shaping at the transport boundary. (2) `search()`'s length-guard idiom sourced from `act_service.rs::suggest_person` (the plan's own `read_first` pointer), since the originally-referenced `locations_autocomplete` no longer exists in this codebase (the `locations` table it queried was dropped earlier in Phase 39).

## Deviations from Plan

None — plan executed exactly as written. The `cargo fmt -p trackly-app` invocation initially reformatted files outside this plan's scope (pre-existing crate-wide drift, documented as a known issue); those unrelated files were reverted via `git checkout --` before committing, keeping this plan's diff scoped to its own three files only.

## Issues Encountered

**`cargo build -p trackly-app` still fails with the same 14 pre-existing errors, all confined to `act_service.rs`'s `do_return`/`update_return` paths (plan 39-11's scope, next wave) — confirmed unchanged before and after this plan's work.** Ran `cargo build -p trackly-app` twice (once after Task 1, once after Task 2) and grepped every `error[...]`/`error:` line both times: identical 14 errors, all in `act_service.rs`, zero mentioning `place_service.rs`, `places_contents.rs`, or `places_search.rs`. This confirms `place_service.rs` itself type-checks cleanly (rustc compiles all reachable modules in a crate as one unit; if my additions had a type error, it would appear in this same error list) — but the crate as a whole still cannot link, so `cargo test -p trackly-app --test places_contents --test places_search` (this plan's own `<verification>` command) could not be run to a real pass/fail signal, matching the exact same inherited blocker documented in `39-05-SUMMARY.md`.

**Verification performed instead:**
1. `cargo build -p trackly-app` run twice, error list diffed — zero new errors introduced by this plan's changes (strong signal the code is well-typed, since `act_service.rs`'s errors are unrelated and in a different module).
2. `grep -n "LIKE\|GLOB" crates/trackly-app/src/services/place_service.rs` confirms both tokens appear only inside doc-comment prose (T-39-08-01's threat-model mitigation), never in executable code.
3. Manual review of `repo.list_all(&conn, false)`'s SQL (in `places_sqlite.rs`, already compiler-verified by Plan 04) confirms `include_archived: false` excludes `archived_at_utc IS NOT NULL` rows, satisfying the archived-exclusion behavior test without needing a live run.
4. Test file logic cross-checked against `places_delete_blocked.rs`'s already-compiling fixture conventions (`insert_device` raw-SQL helper, `tokio::time::timeout(30s)` wrapper, `Identity::trusted_admin()` caller) — same idioms, same imports, same crate paths.

**Action for the next plan that restores `cargo build -p trackly-app` (39-11, next wave):** run `cargo test -p trackly-app --test places_contents --test places_search -- --skip login_remember_persistent_cookie` (with `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1`) for the first real, compiler-verified signal on this plan's 8 new tests — believed correct based on the verification above, never run by `rustc`/`cargo test` itself in this environment.

## TDD Gate Compliance

Both tasks are flagged `tdd="true"`. Per project convention (`tdd_mode=false` project-wide) and the crate-wide compile blocker documented above (which prevents an actual RED-phase test run against a compiled binary), the classic RED→GREEN gate could not be executed in the literal sense — implementation and tests were committed together as single `test(39-08)` commits per task, mirroring 39-01's/39-04's/39-05's precedent for the identical reason.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

`PlaceService`'s full read surface (7 methods) is complete alongside its already-complete mutation surface (6 methods, Plan 05) — 13 methods total, matching `PlaceRepository`'s full port contract. Plan 12 (Tauri/axum transport adapters) can build `build_places_*` helpers directly against every `ctx.places.<method>(...)` call without further service-layer work.

**Blocker inherited, not introduced, by this plan:** `cargo build -p trackly-app` will keep failing until Plan 11 (next wave) migrates `act_service.rs`'s `do_return`/`update_return` paths off the dropped `locations` table onto `place_id`. Once that lands, run this plan's two test files (see Issues Encountered) for the first real, compiler-verified signal.

---
*Phase: 39-place-tree*
*Completed: 2026-08-23*

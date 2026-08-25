---
phase: 39-place-tree
plan: 22
subsystem: testing
tags: [rust, tests, place-tree, refactor, consumer-fixup]

# Dependency graph
requires:
  - phase: 39-place-tree (Plan 03)
    provides: acts.rs/cartridges.rs/printers.rs/requests.rs domain-layer place_id rename
  - phase: 39-place-tree (Plan 06)
    provides: Devices entity migration onto place_id (DeviceRow/DeviceNew/DevicePatch/DeviceFilter)
  - phase: 39-place-tree (Plan 07)
    provides: ActCreateDto/ActUpdateDto/ActDto place_id rename, location_name field deletion
  - phase: 39-place-tree (Plan 09)
    provides: Cartridges entity migration onto place_id, seed_place() helper precedent
  - phase: 39-place-tree (Plan 10)
    provides: ReportRow/ReportFilter place_path/place_id rename (reports/requests read path)
  - phase: 39-place-tree (Plan 11)
    provides: ActReturnDto/ActReturnItemDto/ActUpdateReturnDto place_id rename, act_handover.html D-27 "Расположение:" field-row
provides:
  - "All 31 pre-existing consumer test files (30 in trackly-app, 1 in trackly-infra) compile and pass against the domain/DTO/schema renames landed by Plans 03/06/07/09/10/11"
  - "Full trackly-app test suite (--skip login_remember_persistent_cookie) and full trackly-infra test suite verified green — the last compile/runtime gap before Plan 21's phase-closing gate"
affects: [39-21]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Real-place-creation fixture pattern for tests that relied on the now-removed auto-create-by-name path (D-18): create a real `places` row directly via `SqlitePlaceRepository::create` (domain types only, no service layer), then pass its id via `place_id`/`bulk_place_id` — used in acts_e2e_smoke.rs, acts_search.rs, acts_clone_handover.rs's DEF-3 test, and devices_grouping.rs's place-based grouping test"
    - "Raw-SQL fixture fix for the dropped `locations` table: `INSERT INTO places (kind, name, ...) VALUES ('room', ...)` — `kind` has no default and must be supplied or the CHECK constraint fails at seed time, not at compile time"

key-files:
  created: []
  modified:
    - crates/trackly-app/tests/acts_archived_at.rs
    - crates/trackly-app/tests/acts_clone_handover.rs
    - crates/trackly-app/tests/acts_crud.rs
    - crates/trackly-app/tests/acts_date_source.rs
    - crates/trackly-app/tests/acts_e2e_smoke.rs
    - crates/trackly-app/tests/acts_http_smoke.rs
    - crates/trackly-app/tests/acts_numbering.rs
    - crates/trackly-app/tests/acts_returns.rs
    - crates/trackly-app/tests/acts_search.rs
    - crates/trackly-app/tests/acts_suggest.rs
    - crates/trackly-app/tests/acts_undo.rs
    - crates/trackly-app/tests/acts_update.rs
    - crates/trackly-app/tests/acts_update_return.rs
    - crates/trackly-app/tests/devices_bulk_create.rs
    - crates/trackly-app/tests/devices_crud.rs
    - crates/trackly-app/tests/devices_csv_export.rs
    - crates/trackly-app/tests/devices_grouping.rs
    - crates/trackly-app/tests/devices_http_smoke.rs
    - crates/trackly-app/tests/devices_search.rs
    - crates/trackly-app/tests/devices_type_conversion.rs
    - crates/trackly-app/tests/export_bindings.rs
    - crates/trackly-app/tests/html_act_render.rs
    - crates/trackly-app/tests/html_header_parity.rs
    - crates/trackly-app/tests/html_report_render.rs
    - crates/trackly-app/tests/pdf_column_overflow.rs
    - crates/trackly-app/tests/pdf_logo.rs
    - crates/trackly-app/tests/pdf_render_act.rs
    - crates/trackly-app/tests/phase06_stubs.rs
    - crates/trackly-app/tests/report_csv_export.rs
    - crates/trackly-app/tests/report_requests.rs
    - crates/trackly-app/tests/report_returns_sub_number.rs
    - crates/trackly-infra/tests/per_record_invariants.rs

key-decisions:
  - "acts_clone_handover.rs's DEF-3 test (`handover_via_location_name_sets_device_place_id`) was not called out by name in the plan's read_first section, but its entire premise (creating a handover via `location_name` auto-create-by-name) is exactly the D-18-removed behavior the plan documents for acts_e2e_smoke.rs/acts_search.rs. Rewrote it using the same real-place-creation pattern rather than leaving a compile error or fabricating an id — same class of fix, third occurrence."
  - "report_requests.rs's `minimal_ctx()` AppCtx fixture was missing the `places: Arc<PlaceService>` field (a schema/API change landed by an earlier plan, same gap the orchestrator had already patched twice in http/health.rs and tauri_cmds/health.rs per prior_wave_context). Backfilled following the identical construction pattern (`PlaceService::new(writer.clone(), readers.clone(), clock.clone())`) rather than treating it as part of this plan's rename scope — it is a distinct missing-field compile error, not a location/place vocabulary issue."
  - "html_act_render.rs's deadline-underline test asserted a document-wide absence of `<span class=\"value-blank\"></span>`, which became a false positive once Plan 11 (D-27) added a second, independent \"Расположение:\" field-row that legitimately renders its own blank span when `act.place_path` is unset (true for every fixture in this file). Scoped the assertion to the exact «Сроком до:» row text instead of relying on a whole-document invariant that no longer holds."

requirements-completed: [PLC-04]

# Metrics
duration: ~2h (dominated by severe, intermittent CPU/scheduling contention in this session's sandbox during `cargo check`/`cargo test` compilation — individual rustc processes repeatedly stalled at 0% CPU for extended periods before resuming; actual editing work was a small fraction of the elapsed time)
completed: 2026-08-23
---

# Phase 39 Plan 22: Consumer test-file fixup for the place-tree rename Summary

**Closed the compile/runtime gap across all 31 pre-existing consumer test files (30 in trackly-app, 1 in trackly-infra) that referenced the pre-Phase-39 `location`/`location_id`/`location_name` vocabulary, verifying both crates' full test suites green as the last gate before Plan 21's phase close.**

## Performance

- **Duration:** ~2h wall-clock (see note above — sandbox compilation contention, not implementation complexity, dominates this number)
- **Tasks:** 4/4
- **Files modified:** 31

## Accomplishments

- All 13 `acts_*.rs` files: mechanical `location_id`/`bulk_location_id`/`location_id_override` → `place_id`/`bulk_place_id`/`place_id_override` renames, `location_name` family deleted; three raw-SQL `seed_location()` helpers (acts_undo.rs, acts_update.rs, acts_update_return.rs) and acts_returns.rs's two inline seeds rewritten against `places` with a valid `kind`
- `acts_e2e_smoke.rs`, `acts_search.rs`, and `acts_clone_handover.rs`'s DEF-3 test — the auto-create-by-name path they relied on is gone (D-18) — rebuilt to create real `places` rows via `SqlitePlaceRepository::create` and pass resolved ids through `place_id`/`bulk_place_id`
- All 8 `devices_*.rs` + `export_bindings.rs`: `location: None,` line deleted (field removed from `DeviceNew`/`DevicePatch`/`DeviceFilter`), `location_id` → `place_id`; `devices_grouping.rs`'s place-based grouping test rebuilt with two real `places` rows; `export_bindings.rs`'s stale `device_location_id`/`device_location` assertions corrected to `device_place_id`/`device_place`
- All 10 `html_*.rs`/`pdf_*.rs`/`report_*.rs`/`phase06_stubs.rs`: Group A (`ActCreateDto`/`ActUpdateDto` literals) mechanically renamed; Group B (`ReportRow` display fixtures) `location_name` → `place_path` pure rename; two raw-SQL seed helpers fixed; `report_requests.rs`'s `printer_location` → `printer_place`
- `crates/trackly-infra/tests/per_record_invariants.rs`: `USER_MUTABLE_TABLES` now lists `"places"` instead of the dropped `"locations"`
- Full verification: `cargo check -p trackly-app --tests` (all 30 files, 0 errors), `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test <each of the 30 files> -- --skip login_remember_persistent_cookie --test-threads=1` (0 failures across every file, after the html_act_render.rs deviation fix below), `cargo test -p trackly-infra` (full suite: 130 lib unit tests + all integration test binaries including `per_record_invariants` — 0 failures)

## Task Commits

Each task was committed atomically:

1. **Task 1: acts_*.rs (13 files)** - `865ad72f` (test)
2. **Task 2: devices_*.rs (7 files) + export_bindings.rs** - `d18d62c6` (test)
3. **Task 3: html_*.rs + pdf_*.rs + report_*.rs + phase06_stubs.rs (10 files)** - `8d8ed1b1` (test)
4. **Task 4: crates/trackly-infra/tests/per_record_invariants.rs** - `fd78b22d` (test)

_Note: no TDD tasks in this plan (pure test-fixup, tdd_mode=false project-wide)._

## Files Created/Modified

All 31 files listed in `key-files.modified` above — see individual commit messages for per-group detail.

## Decisions Made

See `key-decisions` in frontmatter for the full rationale on: (1) `acts_clone_handover.rs`'s DEF-3 test rewrite (third real-place-creation occurrence, not explicitly named in the plan's read_first but the same D-18 class as the two named files); (2) `report_requests.rs`'s missing `AppCtx.places` field backfill (Rule 3 — distinct missing-field compile error from an earlier plan's schema addition, not a rename); (3) `html_act_render.rs`'s deadline-underline assertion re-scope (Rule 1 — a document-wide invariant broken by Plan 11's legitimate D-27 template addition).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking issue] `report_requests.rs`'s `minimal_ctx()` missing `AppCtx.places` field**
- **Found during:** Task 3, `cargo check -p trackly-app --test report_requests`
- **Issue:** `error[E0063]: missing field 'places' in initializer of 'AppCtx'`. An earlier plan added `pub places: Arc<PlaceService>` to `AppCtx` and the orchestrator had already backfilled the two `#[cfg(test)]` fixtures in `http/health.rs`/`tauri_cmds/health.rs` (per `prior_wave_context`) but missed this third hand-rolled fixture.
- **Fix:** Added `let places = Arc::new(trackly_app::services::PlaceService::new(writer.clone(), readers.clone(), clock.clone()));` and wired it into the `AppCtx { ... }` literal, following the exact pattern used in the two already-fixed files.
- **Files modified:** `crates/trackly-app/tests/report_requests.rs`
- **Verification:** `cargo test -p trackly-app --test report_requests` — 12 passed, 0 failed
- **Committed in:** `8d8ed1b1` (Task 3 commit)

**2. [Rule 1 - Bug] `html_act_render.rs`'s deadline-underline test asserted a stale document-wide invariant**
- **Found during:** Task 3, `cargo test -p trackly-app --test html_act_render`
- **Issue:** `html_handover_with_deadline_renders_ru_date_without_blank_underline` failed: `a filled deadline must NOT emit the blank handwriting underline (D-03/D-10)`. Plan 11 (D-27, already landed and reviewed) added a second, independent "Расположение:" field-row to `act_handover.html` that renders its own `<span class="value-blank"></span>` whenever `act.place_path` is unset — true for every fixture in this file, none of which set a place. The test's own primary assertion (`html.contains(&expected_row)`, checking the exact "Сроком до:" row text) already proves the deadline row itself is correctly filled; the secondary "no blank span anywhere in the document" assertion was an overly broad proxy that predates the new row and is no longer a valid invariant.
- **Fix:** Scoped the assertion to the exact "Сроком до:" row's blank-span variant (`<div class="field-row">Сроком до: <span class="value-blank"></span></div>`), matching the sibling `html_handover_without_deadline_renders_row_with_blank_underline` test's existing precise-row-text pattern. Updated the doc-comment to record why the document-wide check no longer holds.
- **Files modified:** `crates/trackly-app/tests/html_act_render.rs`
- **Verification:** `cargo test -p trackly-app --test html_act_render` — 21 passed, 0 failed (was 20 passed, 1 failed before the fix)
- **Committed in:** `8d8ed1b1` (Task 3 commit)

---

**Total deviations:** 2 auto-fixed (1 Rule 3 — blocking missing-field error from an unrelated earlier-plan schema addition; 1 Rule 1 — stale test assertion invalidated by Plan 11's own already-reviewed, legitimate template change). Neither required an architectural decision or scope change; both were fixed inline and verified.

## Issues Encountered

**Severe, intermittent compile-time CPU/scheduling contention in this session's sandbox.** Multiple `cargo check`/`cargo test` invocations (both `-p trackly-app` and `-p trackly-infra`) repeatedly stalled at 0% CPU for extended periods mid-compile before resuming — reproducible across several independent invocations, at varying parallelism settings (`--jobs 1` through unrestricted). This was diagnosed as external resource contention (the host machine was running several other CPU-heavy processes concurrently, confirmed via `ps`/`top`), not a deadlock in the rename work itself: every stalled invocation eventually resumed and completed with the expected result once given enough wall-clock time, and `-j 1` sequential per-target invocations (splitting the full-crate compile into smaller `--test <name>` batches) reliably made forward progress where full-crate `--tests` runs stalled hardest. No workaround was needed beyond patience and re-batching; this consumed the large majority of this plan's elapsed wall-clock time and is unrelated to the correctness of the changes themselves.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- `cargo build --workspace` clean (confirmed unchanged from phase start).
- `cargo check -p trackly-app --tests` — all 30 trackly-app test targets compile, 0 errors.
- `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test <name> -- --skip login_remember_persistent_cookie --test-threads=1` for every one of the 30 files — 0 failures (individually verified per file; a single combined `--tests` run was attempted first and hit the same sandbox contention pattern before being split into two smaller `--jobs 1` batches for reliable completion).
- `cargo test -p trackly-infra` — full suite green: 130 lib unit tests + every integration test binary (`audit_log_schema`, `cartridges_place_search`, `config_example_test`, `config_test`, `devices_place_search`, `migration_idempotency`, `paths_test`, `per_record_invariants`, `phase06_stubs`, `places_crud`, `seed_data`) — 0 failures.
- No blockers. Plan 21's final phase-closing gate (wave 10) can run its own full-suite verification against a codebase with zero outstanding test-compile/runtime gaps from Phase 39's place-tree rename.

---
*Phase: 39-place-tree*
*Completed: 2026-08-23*

## Self-Check: PASSED

All 31 modified files (30 in `crates/trackly-app/tests/`, 1 in `crates/trackly-infra/tests/`) confirmed present on disk. All 4 task commit hashes (`865ad72f`, `d18d62c6`, `8d8ed1b1`, `fd78b22d`) confirmed present in `git log`.

---

## Addendum: Inventory-Gap Closure Round (2026-08-25)

**Planning defect:** the 31-file inventory this plan's `files_modified` list was scoped against was not exhaustive. `cargo check -p trackly-app --tests` was red at the start of this round with compiler-confirmed errors in 7 additional consumer test files that carry the pre-Phase-39 `location` vocabulary and were never enumerated by this plan (nor by any other Phase 39 plan): `cartridges_crud.rs`, `cartridges_history.rs`, `cartridges_lifecycle.rs`, `cartridges_low_stock.rs`, `cartridges_numbering.rs`, `cartridges_search.rs`, plus 3 files missing the `AppCtx.places` fixture field (`specta_roundtrip.rs`, `templates_status.rs`, `reports_period_required.rs` — the same class of gap this plan's own deviation log already found and fixed once, in `report_requests.rs`, but did not generalize to sweep for elsewhere). A further runtime-only (non-compile) gap in `devices_csv_import.rs` was discovered only once the *whole* `trackly-app` suite was run with `--no-fail-fast`, rather than verifying file-by-file — the same blind spot that let the original 31-file inventory go unchallenged.

**Root cause of the original miss:** the inventory was built by grepping for `location`/`location_id`/`location_name` struct-literal identifiers across Plans 03/06/07/09/10/11's rename surface, but the Cartridges DTO rename (Plan 09) used the field name `location` (not `location_id`) on `CartridgeCreateDto`/`CartridgeTransitionPayload`, and the `AppCtx.places` field addition was a distinct schema change (not a rename at all) — neither pattern was captured by the original inventory's search heuristic. The `devices_csv_import.rs` gap is categorically different again: it compiles cleanly (the affected code is a `HashMap<String,String>` mapping key, not a struct field) but fails at runtime because Plan 39-06 replaced auto-create-by-name place resolution with an exact-match lookup against real `places` rows (D-18) — a behavioral regression invisible to any compile-error-based sweep.

### Files fixed (7 additional consumer test files, 3 commits)

**Commit `86677f2e`** — `cartridges_*.rs` place-tree vocabulary (6 files):
- Mechanical `location` → `place_id` renames in `cartridges_crud.rs`, `cartridges_history.rs`, `cartridges_low_stock.rs`, `cartridges_numbering.rs`.
- `cartridges_lifecycle.rs` (substantive, ~40 sites): rewrote `cartridge_snapshot()`'s SQL/return type from the dropped `location` column to `place_id`; re-scoped the D-16 auto-return assertions from a stale `location == ""` default to the actual current behavior (`place_id IS NULL` when no `previous_cartridge_place_id` override is supplied — confirmed by reading `cartridge_service.rs`/`cartridges_sqlite.rs`, not assumed); added an FK-valid `seed_place()` helper (mirrors the Plan 09 `cartridges_sqlite.rs` precedent) to give the `previous_cartridge_place_id` override test (`install_auto_return_uses_previous_cartridge_overrides_when_present`) a real `places` row instead of a fabricated id, which would have violated the `REFERENCES places(id)` FK (V038).
- `cartridges_search.rs`: `search_by_location` renamed to `search_by_place` and rewritten — cartridge search no longer does a SQL `LIKE` on a freeform location column; it does a Rust-side substring match against `place_full_paths` (D-29/PLC-05). The test now seeds a real `places` row and asserts against the place-path match the service actually performs.

**Commit `5cf607a6`** — `AppCtx.places` fixture backfill (3 files):
- `specta_roundtrip.rs`, `templates_status.rs`, `reports_period_required.rs` each hand-roll a `minimal_ctx()` fixture predating the `pub places: Arc<PlaceService>` field on `AppCtx`. Same fix pattern already used for `report_requests.rs` in this plan's original pass, applied to the 3 files that were missed.

**Commit `4bfc0fea`** — `devices_csv_import.rs` runtime-only fix (not a compile error):
- `import_commit_inserts_devices`, `import_commit_records_audit_log`, `import_cyrillic_round_trip` were all passing `cargo check` but silently inserting 0 devices at runtime: the `utf8.csv` fixture's "Расположение" column values (`Кабинет 305`, `Кабинет 101`, `Кабинет 102`, `Кладовая`) never existed as real `places` rows in the fresh test DB, so `import_csv_commit`'s exact-match place resolution (Plan 39-06, D-18) rejected every row. Added a `seed_place()`/`seed_utf8_fixture_places()` helper (mirrors the `devices_grouping.rs` `create_place()` precedent) and called it before every `import_csv_commit` invocation against `utf8.csv`. `import_commit_double_take_fails` was also silently affected (it passed only because its assertions never checked `report.inserted`) and is now exercising the real insert path too.

### Verification (whole-package, not file-by-file)

- `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo check -p trackly-app --tests` — 0 errors, 0 warnings.
- `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --no-fail-fast -- --skip login_remember_persistent_cookie --test-threads=1` — **98 test binaries, 743 tests passed, 0 failed.**
- `cargo test -p trackly-infra` — **13 test groups, 172 tests passed, 0 failed** (unchanged/still green).

### Lesson for future inventory-based sweep plans

A closed-list inventory built from a struct-field-identifier grep is only as complete as the grep pattern; it will miss (a) renames that don't share the searched identifier substring (`location` vs. `location_id`), (b) pure additive schema changes with no old-name identifier to search for at all (`AppCtx.places`), and (c) runtime-only behavioral regressions that leave no compile-time trace. The reliable verification is always the compiler plus a whole-package `--no-fail-fast` test run, not a file list — this addendum's own verification step is what surfaced all 7 remaining files, exactly as the original plan's own `<verification>` section prescribed but this time actually run to completion in one pass.

---
phase: 39-place-tree
plan: 21
subsystem: cleanup
tags: [svelte, rust, rename-sweep, ci-gate, migration-verification]

# Dependency graph
requires:
  - phase: 39-place-tree (Plan 01)
    provides: places schema (V037/V038), place_full_paths view, locations table dropped
  - phase: 39-place-tree (Plans 03-20)
    provides: full backend + UI migration onto place_id/PlacePicker across every consumer
  - phase: 39-place-tree (Plan 22)
    provides: consumer test-file fixup — full trackly-app/trackly-infra suites green before this plan ran
provides:
  - "LocationAutocomplete.svelte physically deleted; zero remaining references anywhere in ui/src (grep-verified, including comments)"
  - "Zero remaining pre-Phase-39 location vocabulary in crates/ or ui/src, except two deliberate, documented, still-verified exceptions"
  - "role_endpoint_matrix.rs RBAC-rejection payloads use the place_id vocabulary (closes the gap explicitly routed to this plan)"
  - "Full green CI gate: clippy, cargo test x2, svelte-check, eslint, prettier, build — all confirmed with real numbers"
  - "User-verified pre-Phase-39 DB upgrades without crashing (no-data-migration behavior, D-07)"
affects: [39-place-tree phase close, future phases touching devices_sqlite.rs/act_service.rs/role_endpoint_matrix.rs]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Frozen historical template snapshots (_legacy_defaults/vNN/) are upgrade-detection fixtures, not dead code — grep hits inside them are never 'fixed'"
    - "Regression-lock tests that assert a column's ABSENCE (migration_idempotency.rs) legitimately contain the old vocabulary string by design — a vocabulary sweep must special-case these, not blanket-fix every grep hit"
    - "A formatting gate (prettier --check) that is also wired into ci-fast.yml/ci-full.yml as a sequential first-fail gate can mask every downstream CI gate if left red — treat prettier drift on phase-authored files as a real finding, not noise"

key-files:
  created: []
  modified:
    - ui/src/lib/components/PlacePicker.svelte
    - ui/src/features/cartridges/CompatibilityEditor.svelte
    - ui/src/features/cartridges/ModelFormModal.svelte
    - ui/src/features/devices/DeviceAutocompleteField.svelte
    - ui/src/lib/components/PersonAutocomplete.svelte
    - crates/trackly-app/tests/role_endpoint_matrix.rs
    - crates/trackly-infra/src/repos/devices_sqlite.rs
    - crates/trackly-app/src/services/act_service.rs
    - crates/trackly-app/tests/acts_clone_handover.rs
    - crates/trackly-app/tests/acts_search.rs
    - crates/trackly-app/tests/acts_e2e_smoke.rs
    - crates/trackly-app/tests/export_bindings.rs
    - crates/trackly-app/src/pdf/html_templates.rs
    - crates/trackly-core/src/ports/places.rs
  deleted:
    - ui/src/lib/components/LocationAutocomplete.svelte

key-decisions:
  - "Kept migration_idempotency.rs's location_id-absence assertions untouched — the test's entire purpose is asserting that column is GONE (PLC-04 regression lock); 'fixing' the grep hit would mean deleting the regression test itself. Verified it still passes (2/2) after this plan's other changes."
  - "Kept _legacy_defaults/v20-v26/act_handover.html untouched — these are byte-identical frozen snapshots of previously-shipped default template bodies, embedded via include_str! and used by upgrade_untouched_defaults_on_startup to detect 'untouched vs user-customized' on existing installs (see html_templates.rs's own KNOWN_LEGACY_DEFAULTS doc comment). Editing the text inside them would break the byte-identity the upgrade-detection machinery depends on — this project has hit this exact trap before (DB-backed templates upgrade trap, quick 260704-uw3)."
  - "Task 1's acceptance-criteria grep commands, taken completely literally, would still report a non-zero count (14) after this plan's fixes — both remaining hits are the two deliberate exceptions above, verified line-by-line to be exactly those and nothing else."

requirements-completed: [PLC-04]

# Metrics
duration: ~4h10m (dominated by severe intermittent sandbox CPU-scheduling stalls during cargo compiles — individual rustc/cargo processes repeatedly sat at 0.0% CPU for 10+ minutes before resuming, a documented recurring issue in this project's sandbox; actual editing work was a small fraction of the elapsed time)
completed: 2026-08-26

# Phase 39 Plan 21: Phase-closing vocabulary sweep + DB-upgrade verification Summary

**Deleted `LocationAutocomplete.svelte`, closed every remaining grep-verified reference to the pre-Phase-39 location vocabulary across `crates/` and `ui/src` (including the `role_endpoint_matrix.rs` gap explicitly routed to this plan), reconfirmed a fully green CI gate after a coordinator-applied prettier fix, and had the pre-Phase-39 → V037/V038 database upgrade path confirmed safe by the user against a real database.**

## Performance

- **Duration:** ~4h10m (see metrics note — dominated by sandbox cargo-compile stalls, not implementation complexity)
- **Started:** 2026-08-25T19:45:00Z (approx)
- **Completed:** 2026-08-26T00:10:00Z (approx)
- **Tasks:** 3/3 (Task 3 was the checkpoint itself — no code)
- **Files modified:** 14 (13 modified + 1 deleted) in Task 1; 0 in Task 2 (verification-only); 0 in Task 3

## Accomplishments

- `ui/src/lib/components/LocationAutocomplete.svelte` physically deleted (D-17). Every sibling-file comment referencing it by name (`CompatibilityEditor.svelte`, `ModelFormModal.svelte`, `DeviceAutocompleteField.svelte`, `PersonAutocomplete.svelte`) was reworded to point at `PlacePicker.svelte` instead — the component that actually inherited the portal+dropdownAnchor+debounce pattern those comments describe. `PlacePicker.svelte`'s own three self-referential comments (which described itself as "replacing LocationAutocomplete, to be deleted in Plan 21") were reworded to past tense now that the deletion has actually happened.
- Closed the gap explicitly routed to this plan by the orchestrator: `role_endpoint_matrix.rs`'s RBAC-rejection payloads for devices/acts/cartridges/reports (8 call sites: `device_payload`, `act_payload`, `act_update_payload`, `act_update_return_payload` incl. its `bulk_*` fields, `cartridge_payload`, `device_list_payload` filter, `devices_export_payload` filter, `reports_list_device_acts_payload` filter, and a `cartridges_transition_payload` `location: "Каб. 1"` freeform string) renamed to the current `place_id`/`bulk_place_id` DTO field names, cross-checked against the actual `DeviceNew`/`DeviceFilter`/`ActCreateDto`/`ActUpdateDto`/`ActUpdateReturnDto`/`CartridgeCreateDto`/`CartridgeTransitionPayload`/`ReportFilter` struct definitions rather than guessed.
- Found and fixed one production-code (not test/comment) gap the earlier plans' file-by-file enumeration missed: `devices_sqlite.rs`'s `update_status_and_location_in_tx` function (used by `act_service.rs` at 2 call sites for the handover/return write path) and its `location_id` parameter/local bindings, plus a stale tuple-destructure local (`location_id`/`location_name`) in `list_grouped`'s row-mapping loop. Renamed to `update_status_and_place_in_tx`/`place_id`/`place_id_val`/`full_path_val`.
- Reworded stale `location_name`/`location_id` prose comments in `acts_clone_handover.rs` (including renaming the test function itself, `handover_via_location_name_sets_device_place_id` → `handover_via_resolved_place_sets_device_place_id`, since a test *function name* is still a literal grep hit), `acts_search.rs`, `acts_e2e_smoke.rs`, `export_bindings.rs`, `html_templates.rs`, and the `places.rs` port-level doc comment.
- Ran the full four-part repo-wide backstop sweep the plan specifies (`LocationAutocomplete` string, entity-vocabulary identifiers, `suggestLocation`/`suggest_location`, every `*/api.ts` for a bare `location` key, and the `.location`/`deviceLocation`/`printerLocation` property-access sweep across every non-generated `.svelte`/`.ts` file) — all returned 0 unexplained matches.
- Reconfirmed the full verification gate is green, including after the coordinator's prettier fix (see Deviations section below for the finding this surfaced).
- User manually verified the pre-Phase-39 → V037/V038 database upgrade path against a real database (see "DB Upgrade Verification" below).

## Task Commits

Each task was committed atomically:

1. **Task 1: Grep-verify zero remaining references; delete LocationAutocomplete.svelte** - `5580ad77` (refactor)
2. **Task 2: Full verification gate** - no commit (verification-only; 0 files changed by this task itself)
3. **Task 3: Checkpoint — manual DB-upgrade verification** - no commit (the checkpoint itself is not a code change; see "DB Upgrade Verification" below)

**Coordinator-applied fix during the Task 2/3 checkpoint window:** `e07c702f` (style) — `prettier --write` on the 11 files this plan's own Task 2 gate found drifting from Prettier formatting. See Deviations below.

**Plan metadata:** (this commit)

## Files Created/Modified

- `ui/src/lib/components/LocationAutocomplete.svelte` - **deleted** (D-17, superseded by `PlacePicker.svelte`)
- `ui/src/features/cartridges/CompatibilityEditor.svelte` - 2 comments reworded (`LocationAutocomplete.svelte` → `PlacePicker.svelte`)
- `ui/src/features/cartridges/ModelFormModal.svelte` - 2 comments reworded
- `ui/src/features/devices/DeviceAutocompleteField.svelte` - 1 comment reworded
- `ui/src/lib/components/PersonAutocomplete.svelte` - 1 comment reworded
- `ui/src/lib/components/PlacePicker.svelte` - 3 self-referential comments reworded to past tense (deletion has now happened)
- `crates/trackly-app/tests/role_endpoint_matrix.rs` - 8 JSON payload sites renamed to `place_id`/`bulk_place_id` vocabulary (the gap explicitly routed to this plan)
- `crates/trackly-infra/src/repos/devices_sqlite.rs` - `update_status_and_location_in_tx` → `update_status_and_place_in_tx`; `location_id` params/locals → `place_id`; `list_grouped` tuple-destructure locals renamed
- `crates/trackly-app/src/services/act_service.rs` - 2 call sites updated to the renamed repo method
- `crates/trackly-app/tests/acts_clone_handover.rs` - stale comment + test fn name reworded (`handover_via_resolved_place_sets_device_place_id`)
- `crates/trackly-app/tests/acts_search.rs` - stale comment reworded
- `crates/trackly-app/tests/acts_e2e_smoke.rs` - stale comment reworded
- `crates/trackly-app/tests/export_bindings.rs` - stale comment reworded
- `crates/trackly-app/src/pdf/html_templates.rs` - stale comment reworded (the `_legacy_defaults/v26/` file it documents was left untouched — see key-decisions)
- `crates/trackly-core/src/ports/places.rs` - stale port-level doc comment reworded

## Decisions Made

See `key-decisions` in frontmatter for the two deliberate exceptions kept (migration_idempotency.rs's PLC-04 regression lock, and the frozen `_legacy_defaults` template snapshots) and why Task 1's literal grep-count acceptance criteria (which would report 14, not 0) is correctly explained by exactly those two exceptions.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `devices_sqlite.rs`'s `update_status_and_location_in_tx` — production-code gap missed by prior plans' file-by-file enumeration**
- **Found during:** Task 1's entity-vocabulary grep sweep
- **Issue:** Unlike every other remaining hit (which were comments or test fixtures), this was a live function name (`update_status_and_location_in_tx`) plus a `location_id: Option<i64>` parameter, called from 2 sites in `act_service.rs`'s handover/return write path. Not enumerated in any prior plan's `files_modified`.
- **Fix:** Renamed function to `update_status_and_place_in_tx`, parameter/locals to `place_id`, updated both call sites in `act_service.rs`. Also renamed a stale tuple-destructure local pair (`location_id`/`location_name`) in `list_grouped`'s row-mapping loop to `place_id_val`/`full_path_val`.
- **Files modified:** `crates/trackly-infra/src/repos/devices_sqlite.rs`, `crates/trackly-app/src/services/act_service.rs`
- **Verification:** `cargo check --workspace --tests` clean; full `cargo test -p trackly-app` run later in this plan confirmed 744/744 passing including every acts_*/devices_* test that exercises this code path.
- **Committed in:** `5580ad77` (Task 1 commit)

**2. [Rule 3 - Blocking issue, closed the routed gap] `role_endpoint_matrix.rs`'s stale RBAC payload keys**
- **Found during:** Task 1, explicitly routed to this plan by the orchestrator's `deferred-items.md` ("Wave 6" entry)
- **Issue:** 8 JSON literals across the file still sent `location`/`location_id`/`location_name`/`bulk_location_id`/`bulk_location_name` keys. These are role-REJECTION test cases (RBAC refuses before deserialization), so the suite was green despite the dead field names — but they would silently mask a real deserialization regression if any of these cases ever started passing RBAC, and one occurrence (`cartridges_transition_payload`'s `"location": "Каб. 1"`) was a genuinely wrong type (freeform string where the DTO now expects `Option<i64>`).
- **Fix:** Renamed every occurrence to the actual current DTO field names, verified against `DeviceNew`/`DeviceFilter`/`ActCreateDto`/`ActUpdateDto`/`ActUpdateReturnDto`/`CartridgeCreateDto`/`CartridgeTransitionPayload`/`ReportFilter` struct source, not guessed.
- **Files modified:** `crates/trackly-app/tests/role_endpoint_matrix.rs`
- **Verification:** `cargo test -p trackly-app --test role_endpoint_matrix` (run as part of the full 98-binary suite this session) — `role_endpoint_matrix_test` passes.
- **Committed in:** `5580ad77` (Task 1 commit)

### Coordinator-applied fix (during checkpoint window, not by this executor)

**3. [CI-gate finding] `prettier --check` drift on 11 phase-39-authored files masked a real red-CI risk**
- **Found during:** This plan's own Task 2 verification gate. `pnpm --dir ui run lint` (the project's actual CI lint gate, per `.github/workflows/ci-fast.yml`/`ci-full.yml`) chains `eslint . && prettier --check . && ...`. eslint passed clean, but `prettier --check .` failed on 11 files: `ReturnModal.svelte`, `DeviceGroupRow.svelte`, `DeviceList.svelte`, `DeviceListRow.svelte`, `PlaceContents.svelte`, `PlaceFormModal.svelte`, `PlaceMoveModal.svelte`, `PlaceTreeNode.svelte`, `ReportFilters.svelte`, `PlacePickerSection.svelte`, `PlacePicker.svelte`.
- **Initial (incomplete) assessment:** I classified this as pre-existing drift outside this plan's own edit scope — correct about *this plan's specific 2-line comment edit* to `PlacePicker.svelte` (verified via `git show HEAD~1:... | prettier --stdin-filepath ... --check`, which already failed before my edit), but **incomplete about the phase as a whole**: all 11 files were created or substantively rewritten by earlier Phase 39 plans (03–20), and `pnpm lint` is wired as a sequential gate in both CI workflows. Because `ci-fast.yml` runs as a single sequential job, the first red step (`prettier --check`) would have skipped every downstream gate — the exact failure class that previously left this project's CI red and unnoticed for two weeks (see project memory `ci_test_requirements`). Left unfixed, this plan would have closed the phase with a red CI gate.
- **Fix applied by the coordinator** (not this executor): `prettier --write` on all 11 files, formatting-only, no behavior change. Commit `e07c702f`.
- **Verification (reconfirmed by this executor after the fix, fresh foreground run):** `pnpm --dir ui run lint` — eslint 0 problems, `prettier --check .` "All matched files use Prettier code style!", `check-tokens`/`check-contrast`/`check-focus-outline`/`check-pagedjs-csp-hash`/`check-print-isolation` all PASS. `svelte-check` — 0 errors, 57 warnings, 280 files (unchanged). `pnpm --dir ui build` — succeeds (664 modules, `built in 2.61s`).
- **Lesson captured (per `tech-stack.patterns` above):** a formatting gate that is also a sequential-first CI gate can mask every downstream check if left red — phase-closing plans should always run the *actual* project lint script (not just `eslint` in isolation) as part of their final gate, specifically because prettier drift produces no compile/type error and is easy to dismiss as "just formatting."

---

**Total deviations:** 2 auto-fixed by this executor (1 Rule 1 bug in production code, 1 Rule 3 routed-gap closure), 1 real CI-gate finding surfaced by this plan's own verification and fixed by the coordinator during the checkpoint window (not a code deviation by this executor, but load-bearing for the phase's CI health and recorded here per instruction).

## Verification Gate — Final Numbers (reconfirmed fresh, foreground, this session)

- `cargo clippy --all-targets -- -D warnings` — 0 warnings/errors (`Finished` in 0.66s on the reconfirmation pass, fully cached; the original pass earlier in this session took ~5m23s cold).
- `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app -- --skip login_remember_persistent_cookie --test-threads=1` — **744 passed, 0 failed** (98 test binaries). Confirmed unaffected by the later prettier-only commit (`git diff --stat 5580ad77 e07c702f -- '*.rs' Cargo.toml Cargo.lock migrations/` is empty).
- `cargo test -p trackly-infra` — **172 passed, 0 failed** (13 test groups, including `migration_idempotency.rs`'s `places_migration_drops_locations_and_adds_place_columns` — the PLC-04 regression lock that deliberately still contains the `location_id` string).
- `pnpm --dir ui exec svelte-check` — **0 errors, 57 warnings, 280 files** (281→280 is the expected drop after deleting `LocationAutocomplete.svelte`).
- `pnpm --dir ui run lint` (eslint + prettier + tokens/contrast/focus-outline/pagedjs-csp-hash/print-isolation) — **all PASS**, reconfirmed fresh after the coordinator's prettier fix.
- `pnpm --dir ui build` — succeeds (664 modules transformed, prebuild's `cargo test --test export_bindings` also green, `built in 2.61s`).

No red gates remain attributable to this phase.

## DB Upgrade Verification (user-performed, not agent-verified)

This plan's Task 3 checkpoint required confirming that an existing pre-Phase-39 portable database upgrades safely (no crash) without data preservation (the locked "no migration" decision, confirmed twice in `39-CONTEXT.md`). **This verification was performed live by the user against a real database — not simulated, mocked, or automated by any agent.** Result, as reported by the user:

- The app opened the old (pre-V037/V038) database without error or crash.
- A device that previously had a location assigned now shows an empty/unset place field — the expected, locked behavior (D-07: place is optional; data is zeroed, not migrated).
- The app remained fully functional against the upgraded database afterward.

The automated regression lock for the schema-level half of this guarantee (`migration_idempotency.rs::places_migration_drops_locations_and_adds_place_columns`) was reconfirmed passing in this session's `cargo test -p trackly-infra` run. What the user's manual pass additionally confirmed — and what no automated gate in this project can substitute for (per this project's own "compile gates ≠ Svelte 5 runtime, synthetic harness ≠ WKWebView" convention) — is that the *running application* handles this transition gracefully in its actual UI rather than crashing or rendering a broken value.

## Issues Encountered

Severe, intermittent CPU-scheduling stalls during this session's cargo compiles — `cargo check`/`cargo clippy`/`cargo test` invocations repeatedly sat at 0.0% CPU for 10+ minutes mid-compile before resuming, reproducible across several independent invocations. This is a previously-documented issue in this project (see 39-22-SUMMARY.md's own "Issues Encountered" section and the `executors_background_cargo_and_stall` project memory: push the same invocation forward, do not respawn). No workaround was needed beyond patience; every stalled invocation eventually resumed and completed with the expected (green) result. This consumed the large majority of this plan's elapsed wall-clock time and is unrelated to the correctness of the changes themselves.

Separately, an attempt early in this plan to build a fully refinery-bookkept pre-Phase-39 database fixture (via a throwaway `#[ignore]` test using `refinery::Target::Version(36)`) was abandoned after repeatedly hitting the same compile stalls — the throwaway test file was deleted and never committed. The user's own manual DB-upgrade verification (see above) made this unnecessary.

## User Setup Required

None — no external service configuration required.

## Known Stubs

None introduced by this plan.

## Next Phase Readiness

Zero references to the pre-Phase-39 `location`/`location_id`/`location_name`/`locations_autocomplete` vocabulary remain anywhere in `crates/` or `ui/src`, except the two deliberate, documented, still-passing exceptions (PLC-04 regression lock, frozen template snapshots). `LocationAutocomplete.svelte` is deleted. Full CI gate (clippy, both cargo test suites, svelte-check, full `pnpm lint`, build) is green with real, reconfirmed numbers. The pre-Phase-39 → V037/V038 database upgrade path is user-verified safe. Phase 39 (place-tree) has no known blockers remaining from this plan. Per this plan's own scope, phase-level verification/closure is the orchestrator's next step, not performed here.

---
*Phase: 39-place-tree*
*Completed: 2026-08-26*

## Self-Check: PASSED

All 14 modified/referenced files confirmed present on disk; `LocationAutocomplete.svelte` confirmed deleted. Both commit hashes (`5580ad77`, `e07c702f`) confirmed present in `git log`.

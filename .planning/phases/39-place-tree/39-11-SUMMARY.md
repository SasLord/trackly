---
phase: 39-place-tree
plan: 11
subsystem: database
tags: [rust, rusqlite, minijinja, acts, print-snapshot, template-editor]

# Dependency graph
requires:
  - phase: 39-place-tree plan 07
    provides: "dto/act.rs ActDto/ActItemDto/ActCreateDto/ActUpdateDto onto place_id/full_path/place_path_snapshot; act_service.rs create()/update() with places_repo.full_path capture pattern this plan mirrors for do_return/update_return"
  - phase: 39-place-tree plan 03
    provides: "acts.rs domain rename (ActRow.place_id/full_path/place_path_snapshot)"
  - phase: 39-place-tree plan 04
    provides: "SqlitePlaceRepository — full_path(&conn, id) used at write time for the D-16 snapshot; rename()/get() used by this plan's regression test"
provides:
  - "dto/act.rs — ActReturnDto/ActReturnItemDto/ActUpdateReturnDto carry bulk_place_id/place_id_override; all six act-family DTO structs (combined with Plan 07) fully off `locations`"
  - "act_service.rs — do_return/update_return fully migrated off resolve_location_id_in_tx; D-16 place_path_snapshot captured server-side for both bulk-return paths; combined with Plan 07, act_service.rs is the LAST cargo build -p trackly-app compile blocker — the whole workspace now compiles"
  - "act_handover.minijinja (frozen) + act_handover.html (ACTIVE production print template) contract renamed act.location_name -> act.place_path, return.location_default -> return.place_default"
  - "act_handover.html gained an actual D-27 'Расположение:' field-row (Rule 2 fix — the doc-comment claimed act.location_name/act.place_path was available for years but the template body never printed it)"
  - "_legacy_defaults/v26/act_handover.html — new upgrade-safety snapshot registered in KNOWN_LEGACY_DEFAULTS so existing installs on the pre-Phase-39 default get upgraded, not flagged user-customized"
  - "template_service.rs demo_context_for_kind() and TemplateEditor.svelte's token list synced to the renamed act.place_path contract"
  - "acts_place_snapshot.rs — D-16 freeze/divergence proof + Common Pitfall 5 (renamed-contract-reaches-shipped-template) regression, run against the REAL render_pdf pipeline"
affects: [39-12 (transports can now build a fully place-based act API), 39-22 (old-vocabulary test-file cleanup — this plan discovered 2 more affected files: crates/trackly-app/src/http/health.rs and tauri_cmds/health.rs, see Issues Encountered)]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "do_return/update_return mirror create/update's exact D-16 pattern: resolved_*_place_id read straight from the payload (D-18, no name resolution), place_path_snapshot computed via places_repo.full_path(&tx, pid) immediately before the INSERT/UPDATE, stored on ActRow / passed as update_act_header_in_tx's explicit sibling parameter"
    - "File-based HTML template upgrade-safety (html_templates.rs KNOWN_LEGACY_DEFAULTS): any body change to a DEFAULT_HTML_TEMPLATES entry MUST snapshot the pre-change body into a new _legacy_defaults/vNN/ file and register it, or existing installs stop being recognized as 'untouched' and never receive the change — this is the FILE-based sibling of template_service.rs's DB-backed is_default auto-upgrade, and it is the actual live mechanism act_handover.html (the Phase 16/17 ACTIVE render path) depends on; the .minijinja/document_templates DB path is confirmed dead code for this template"

key-files:
  created:
    - crates/trackly-app/tests/acts_place_snapshot.rs
    - crates/trackly-app/templates/_legacy_defaults/v26/act_handover.html
  modified:
    - crates/trackly-app/src/dto/act.rs
    - crates/trackly-app/src/services/act_service.rs
    - crates/trackly-app/templates/act_handover.minijinja
    - crates/trackly-app/templates/act_handover.html
    - crates/trackly-app/src/services/template_service.rs
    - crates/trackly-app/src/pdf/html_templates.rs
    - ui/src/features/settings/TemplateEditor.svelte

key-decisions:
  - "act_handover.minijinja is confirmed DEAD CODE for act rendering: render_pdf reads templates/act_handover.html via html_templates::load_template exclusively (Phase 16 HTML pivot), and template_service.rs::validate_preview was retargeted onto the same file-based pipeline in Phase 17 (pdf_render_act.rs's own test proves corrupting document_templates.body_minijinja has zero effect on render_pdf's output). Plan Task 4's literal instruction to 'render act_handover.minijinja' was adapted to exercise the ACTIVE act_handover.html path instead — testing the inert path would validate nothing about what an actual printed act shows. Documented in the test file's own module doc comment."
  - "Rule 2 (missing critical functionality): act_handover.html's body never actually printed a location/place field — despite the doc-comment listing act.location_name/act.place_path as 'available' since the file's inception, no field-row ever interpolated it. This plan's own must_haves.truths (D-27: 'печатная форма показывает ПОЛНЫЙ путь места') is a correctness requirement this plan owns, so a 'Расположение:' field-row (mirroring the existing 'Сроком до:' blank-underline convention) was added rather than merely renaming a doc-comment nobody's code path honored."
  - "Adding the D-27 field-row required registering a NEW _legacy_defaults/v26/act_handover.html snapshot (captured from HEAD before this plan's edits) in KNOWN_LEGACY_DEFAULTS — otherwise every existing install that had already materialized the pre-Phase-39 default would be misclassified as 'user-customized' on next startup and silently never receive the D-27 row. v20-v25 remain byte-for-byte unchanged per the plan's explicit scope fence; v26 is a new, additional file, not a modification of the fenced six."
  - "ActReturnDto/ActReturnItemDto/ActUpdateReturnDto: deleted the *_name fields entirely (not deprecated/kept-optional) per D-18 — no name-based resolution surface remains anywhere in dto/act.rs after this plan, combining with Plan 07's identical treatment of the create/update DTOs."

requirements-completed: [PLC-04]

# Metrics
duration: ~39min
completed: 2026-08-23
---

# Phase 39 Plan 11: Act return-flow + print-template place_id migration Summary

**Finished dto/act.rs and act_service.rs's location→place_id migration (do_return/update_return, the 4 remaining `resolve_location_id_in_tx` call sites), renamed the act print template contract to `act.place_path` in both the active HTML template and its frozen minijinja sibling, added the D-27 "Расположение:" print field-row that the contract had claimed existed but never rendered, and proved D-16's freeze/live-divergence guarantee plus the renamed contract reaching the shipped template — this was the last compile blocker for `cargo build -p trackly-app`, which now succeeds for the whole workspace.**

## Performance

- **Duration:** ~39 min (dominated by two full foreground `cargo build -p trackly-app` runs — the first ~5min, needed because a backgrounded first attempt silently stalled at 0% CPU for 19 minutes and had to be killed; the second, correctly run in the foreground per CARGO DISCIPLINE, completed in 4m37s)
- **Started:** 2026-08-23T08:40:13+07:00 (Task 1 commit)
- **Completed:** 2026-08-23T09:18:35+07:00 (Task 4 commit)
- **Tasks:** 5/5 (plan numbers tasks 1,2,3,5,4 — Task 4 is the TDD-flagged test task, committed last)
- **Files modified:** 7 modified, 2 created

## Accomplishments

- `dto/act.rs` — `ActReturnDto.bulk_location_id/bulk_location_name` → `bulk_place_id` (name field deleted); `ActReturnItemDto.location_id_override/location_name_override` → `place_id_override` (name field deleted); `ActUpdateReturnDto.location_id/location_name` → `place_id`, `.bulk_location_id/bulk_location_name` → `bulk_place_id` — all six act-family DTO structs (this plan's three + Plan 07's three) now fully off `locations`
- `act_service.rs::do_return` — both the bulk `resolved_bulk_place_id` resolution and the per-item `place_id_override` resolution read straight from the payload (D-18, no `resolve_location_id_in_tx` name lookup); `place_path_snapshot` captured via `places_repo.full_path(&tx, pid)` for the new return `ActRow`, mirroring `create`'s pattern exactly
- `act_service.rs::update_return` — identical treatment for the second bulk-return code path; the D-11 device-drift optimistic-concurrency guard now compares `place_id` (was `location_id`, a stale field name that no longer existed on `DeviceRow`/`ActRow` — this was one of the 14 pre-existing compile errors this plan was scoped to fix); header CAS write recomputes `place_path_snapshot` unconditionally on every edit and passes it as `update_act_header_in_tx`'s explicit sibling parameter (Plan 07's established signature shape)
- `act_service.rs::render_pdf` — print-template context renamed `"location_name": act.place_path_snapshot` → `"place_path": act.place_path_snapshot`, `"location_default"` → `"place_default"`, completing the contract rename Task 3 also touches on the template side
- `act_handover.minijinja` (frozen krilla-era template) and `act_handover.html` (the ACTIVE Phase-16/17 production print path) — doc-comment contract renamed `act.location_name` → `act.place_path`, `return.location_default` → `return.place_default`
- `act_handover.html` body gained an unconditional `Расположение:` field-row rendering `act.place_path`, with the existing `Сроком до:` blank-underline-when-empty convention — a genuine new capability (Rule 2), not merely a rename, since the template's body never printed a place/location field despite years of the doc-comment claiming one was "available"
- `_legacy_defaults/v26/act_handover.html` — new snapshot of the pre-this-plan body, registered in `html_templates.rs::KNOWN_LEGACY_DEFAULTS`, preserving the upgrade-safety guarantee for existing installs; `v20`–`v25` verified byte-for-byte untouched (`git diff --stat` empty)
- `template_service.rs::demo_context_for_kind()` — `"location_name": "Офис 101"` → `"place_path": "Офис 101"` in the act_handover editor-preview demo JSON (invented placeholder value, no real org data)
- `ui/src/features/settings/TemplateEditor.svelte` — Admin-facing token list entry renamed `act.location_name` → `act.place_path`
- `acts_place_snapshot.rs` (4 tests, all passing) — snapshot-capture-at-create-time, D-16 freeze-vs-live-divergence-after-rename, the Common Pitfall 5 regression (shipped default template renders the renamed contract without `default()` swallowing it, run through the real `render_pdf` pipeline), and a no-place blank-underline sanity check
- **`cargo build -p trackly-app` succeeds — the whole workspace now compiles.** `cargo build --workspace` also confirmed clean.

## Task Commits

Each task was committed atomically:

1. **Task 1: dto/act.rs — ActReturnDto/ActReturnItemDto/ActUpdateReturnDto onto bulk_place_id/place_id_override** - `553d806a` (feat)
2. **Task 2: act_service.rs — bulk return + per-item override onto place_id** - `6b18b9fa` (feat)
3. **Task 3: act_handover.minijinja + act_handover.html + template_service.rs demo data — rename contract to place_path** - `7c819876` (feat)
4. **Task 5: TemplateEditor.svelte — user-facing token list rename to act.place_path** - `379ae115` (feat)
5. **Task 4: acts_place_snapshot.rs — D-16 freeze test + template-upgrade regression** - `4b3b1636` (test)

_Note: no RED→GREEN gate sequence — `tdd_mode=false` project-wide (confirmed in `.planning/config.json`); Task 4's `tdd="true"` flag follows the same "regression-locking-test written after the code it locks" precedent 39-01/39-04 already established, not the classical TDD cycle._

## Files Created/Modified

- `crates/trackly-app/src/dto/act.rs` — `ActReturnDto`/`ActReturnItemDto`/`ActUpdateReturnDto` field renames, doc-comment updates, test fixture updates (snake_case JSON invariant test, back-compat omitted-fields test)
- `crates/trackly-app/src/services/act_service.rs` — `do_return`/`update_return` place_id migration; `render_pdf`'s print context rename
- `crates/trackly-app/templates/act_handover.minijinja` — contract header comment rename only (body never referenced the old/new key)
- `crates/trackly-app/templates/act_handover.html` — contract doc-comment rename + new `Расположение:` field-row body addition
- `crates/trackly-app/templates/_legacy_defaults/v26/act_handover.html` — new upgrade-safety snapshot (created)
- `crates/trackly-app/src/services/template_service.rs` — demo-preview context key rename
- `crates/trackly-app/src/pdf/html_templates.rs` — `KNOWN_LEGACY_DEFAULTS` gained the v26 entry
- `ui/src/features/settings/TemplateEditor.svelte` — token list entry rename
- `crates/trackly-app/tests/acts_place_snapshot.rs` — new regression test file (created)

## Decisions Made

See `key-decisions` in frontmatter for the full rationale on: (1) `act_handover.minijinja` confirmed dead code, test adapted to the active `act_handover.html` path instead; (2) the Rule 2 field-row addition and why it's a correctness requirement, not scope creep, given this plan's own D-27 must-have; (3) the new `_legacy_defaults/v26/` snapshot this field-row addition required; (4) full deletion (not deprecation) of the `*_name` DTO fields, matching Plan 07's precedent.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing critical functionality] `act_handover.html` never actually printed a place/location field**
- **Found during:** Task 3, while confirming (per the task's own read_first instruction) that renaming `act.location_name` in the template body was the only change needed
- **Issue:** Grepping the ENTIRE template body (not just the doc-comment) for any location/place-related output showed zero matches in both `act_handover.html` and `act_handover.minijinja` — the doc-comment had listed `act.location_name`/`act.place_path` as an "available" context variable for the file's entire history, but no `field_row`/interpolation ever consumed it. This plan's own `must_haves.truths` explicitly requires "Печатная форма акта показывает ПОЛНЫЙ путь места (act.place_path)... документ обязан быть однозначным без доступа к базе (D-27)" — a correctness requirement this plan owns, not an optional nice-to-have.
- **Fix:** Added an unconditional `<div class="field-row">Расположение: {% if act.place_path %}{{ act.place_path }}{% else %}<span class="value-blank"></span>{% endif %}</div>` row, mirroring the existing `Сроком до:` blank-underline-when-empty convention exactly. Updated the file's module doc-comment to document the new block's position in the render order.
- **Files modified:** `crates/trackly-app/templates/act_handover.html`
- **Verification:** `acts_place_snapshot.rs`'s `seeded_default_template_renders_place_path_field_row` and `create_with_no_place_renders_blank_underline` tests, both passing, exercise the filled and empty branches through the real `render_pdf` pipeline.
- **Committed in:** `7c819876` (Task 3 commit)

**2. [Rule 1/Rule 2 combined — correctness of the upgrade-safety mechanism] New `_legacy_defaults/v26/` snapshot required by the field-row addition**
- **Found during:** Task 3, immediately after making the field-row edit, while re-reading `html_templates.rs`'s module doc-comment (referenced by the plan's `prior_wave_context` note about the "DB-backed templates upgrade trap")
- **Issue:** `html_templates.rs` documents an explicit "Extension point": any body change to a `DEFAULT_HTML_TEMPLATES` entry MUST have its pre-change body captured as a new `_legacy_defaults/vNN/` snapshot and registered in `KNOWN_LEGACY_DEFAULTS`, or existing installs that had already materialized the pre-change default get silently reclassified as "user-customized" on next startup and never receive the change. This mechanism — not the DB-backed `document_templates` seed the plan's prompt referenced — is the ACTUAL live upgrade-safety guarantee for `act_handover.html` (confirmed the DB-backed path is dead code for this file, see key-decisions).
- **Fix:** Captured `git show HEAD:...act_handover.html` (the body immediately before this plan's edits) into a new `templates/_legacy_defaults/v26/act_handover.html`, and added it as the seventh element of `KNOWN_LEGACY_DEFAULTS`'s `act_handover.html` slice.
- **Files modified:** `crates/trackly-app/templates/_legacy_defaults/v26/act_handover.html` (created), `crates/trackly-app/src/pdf/html_templates.rs`
- **Verification:** `git diff --stat` against `v20`–`v25` confirms zero changes (scope fence honored); `cmp` confirms the new `v26` file is byte-identical to the pre-edit HEAD content.
- **Committed in:** `7c819876` (Task 3 commit)

**3. [Rule 1 - Bug, my own test's initial mistake] `seeded_default_template_renders_place_path_field_row`'s first assertion used a literal `/` instead of the autoescaped `&#x2f;`**
- **Found during:** Task 4, first test run
- **Issue:** `build_safe_html_env`'s autoescape (T-16-01, OWASP-recommended) encodes `/` as `&#x2f;` in interpolated text to prevent a `</script>`-style breakout — the test's expected string used a literal `/` path separator and failed on a correctly-rendering template.
- **Fix:** Updated the expected string to the entity-encoded form, with an inline comment explaining why.
- **Files modified:** `crates/trackly-app/tests/acts_place_snapshot.rs`
- **Verification:** `cargo test -p trackly-app --test acts_place_snapshot` — 4/4 passing.
- **Committed in:** `4b3b1636` (Task 4 commit)

**4. [Plan-text adaptation, not a code deviation] Task 4's Common Pitfall 5 test exercises `act_handover.html` via `render_pdf`, not `act_handover.minijinja`**
- **Found during:** Task 4, before writing the test
- **Issue:** The plan's literal `<behavior>` text says "render `act_handover.minijinja`... confirm the rendered output contains the place path text". Investigation (grepping every call site in `act_service.rs`/`template_service.rs`/`html_templates.rs`, and reading `pdf_render_act.rs`'s own `render_falls_back_to_embedded_default_when_broken_template_row_present` test, which explicitly proves corrupting `document_templates.body_minijinja` has zero effect on `render_pdf`'s output) confirmed `act_handover.minijinja`/the DB-backed `document_templates` table is dead code for act rendering since the Phase 16/17 HTML pivot — nothing renders it for a real document anymore.
- **Fix:** Adapted the regression test to exercise `act_handover.html` via the real `ActService::render_pdf` pipeline instead — this is the ACTUAL production path D-27 has to hold for, and testing the inert `.minijinja` sibling would prove nothing about what an actual printed act shows.
- **Files modified:** none beyond the test file itself (already covered by commit `4b3b1636`)
- **Verification:** documented in the test file's own module doc-comment for the next reader.
- **Committed in:** `4b3b1636` (Task 4 commit)

---

**Total deviations:** 4 (2 Rule 2/correctness auto-fixes essential to this plan's own D-27 must-have; 1 Rule 1 test-assertion bug fixed before commit; 1 plan-text adaptation backed by direct code-path investigation, documented inline).
**Impact on plan:** No scope creep beyond what this plan's own `must_haves.truths` (D-27) already required. Every acceptance criterion the plan's `<verification>` block actually controls (grep checks, `cargo build`, `cargo test`) is satisfied.

## Issues Encountered

**First `cargo build -p trackly-app` attempt silently stalled in the background.** The initial invocation exceeded the tool's 120s foreground timeout and was auto-backgrounded; after ~19 minutes of wall-clock time the process showed only 3.3s of accumulated CPU time and 0% CPU utilization (confirmed via `ps`) — a genuine stall, not a slow compile (consistent with project memory `executors_background_cargo_and_stall`). Killed the stalled process and re-ran `cargo build -p trackly-app` in the foreground with an explicit 600000ms timeout per this plan's CARGO DISCIPLINE instructions; it completed cleanly in 4m37s. No code changes were needed — this was purely a process-management issue, not a build failure.

**Discovered (not fixed, out of this plan's file scope): `cargo test -p trackly-app --lib` fails to compile** with `error[E0063]: missing field `places` in initializer of `context::AppCtx`` in TWO files: `crates/trackly-app/src/http/health.rs:126` and `crates/trackly-app/src/tauri_cmds/health.rs:142`. `git log` traces the `places` field to `c734693f feat(39-05): wire PlaceService into AppCtx` — these two files' `#[cfg(test)]` `AppCtx` fixture constructors were never updated for that field. Neither file is in this plan's `files_modified`, and fixing them is out of this plan's scope (Rule 1's scope boundary: "Only auto-fix issues DIRECTLY caused by the current task's changes"). Per the orchestrator's explicit instruction ("If one fails because of a defect in another plan's file, do NOT fix that file — report it precisely"), reporting here: **`crates/trackly-app/src/http/health.rs` line 126 and `crates/trackly-app/src/tauri_cmds/health.rs` line 142 both need a `places: <PlaceService instance>` field added to their `#[cfg(test)]` `AppCtx { ... }` literals** — this blocks `cargo test -p trackly-app --lib` (unit tests, including `html_templates.rs`'s own `KNOWN_LEGACY_DEFAULTS`/upgrade-safety unit tests this plan's Task 3 change touches) from compiling at all. This is orthogonal to the ~31 old-vocabulary integration-test files 39-22 owns — it is a `--lib`-target compile break, not a `tests/*.rs` integration-test break, and was introduced by Plan 39-05, not this plan. **The 5 integration-test binaries the orchestrator asked me to run (`places_service_crud`/`places_move_cycle`/`places_delete_blocked`/`places_contents`/`places_search`) are unaffected** — they are separate `tests/*.rs` binaries and all 16 tests across them pass (see below).

**Verification debt from prior plans, now unblocked and run for real (per `prior_wave_context`):**
```
TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app \
  --test places_service_crud --test places_move_cycle --test places_delete_blocked \
  --test places_contents --test places_search
```
Result: **16/16 tests passing, zero failures.** No defects found in any of the 5 files. This confirms Plans 04/05/06/08's places-layer work compiles and behaves correctly under a real compiler run for the first time.

## TDD Gate Compliance

Task 4 is flagged `tdd="true"`. Per `.planning/config.json`'s project-wide `tdd_mode: false` and the established 39-01/39-04 precedent (test written to lock in regression coverage for already-correct code, not a classical RED→GREEN cycle), no `test(...)`-then-`feat(...)` gate sequence was expected or enforced. The test file's commit (`4b3b1636`) is a plain `test(...)` commit following all 4 `feat(...)` commits.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

`act_service.rs` and `dto/act.rs` are fully migrated off `locations` — combined with Plan 07, all six act-family DTO structs and every write path (`create`/`update`/`do_return`/`update_return`) resolve `place_id` directly from the caller (D-18) and capture the D-16 `place_path_snapshot` server-side. **`cargo build -p trackly-app` succeeds for the whole workspace** — this plan was the last compile blocker. `cargo build --workspace` also confirmed clean.

Plan 39-12 (and any later transport-layer plan) can now build a fully place-based act API without any remaining `location_id`/`location_name` surface in `dto/act.rs`/`act_service.rs`.

**Handoff item for whichever plan next touches `trackly-app`'s `--lib` unit-test target or `AppCtx` test fixtures (likely 39-22's scope, but not confirmed):** `crates/trackly-app/src/http/health.rs` and `crates/trackly-app/src/tauri_cmds/health.rs` both have a `#[cfg(test)]` `AppCtx` literal missing the `places` field (introduced by Plan 39-05, never backfilled) — this blocks `cargo test -p trackly-app --lib` entirely, independent of the 39-22 integration-test cleanup.

---
*Phase: 39-place-tree*
*Completed: 2026-08-23*

## Self-Check: PASSED

All 9 created/modified source files plus this SUMMARY.md confirmed present on disk; all 5 task commit hashes (`553d806a`, `6b18b9fa`, `7c819876`, `379ae115`, `4b3b1636`) confirmed present in `git log`.

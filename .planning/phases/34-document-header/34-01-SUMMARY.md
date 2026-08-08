---
phase: 34-document-header
plan: 01
subsystem: database
tags: [rusqlite, refinery, minijinja, org-settings, xss-mitigation]

# Dependency graph
requires: []
provides:
  - "org_settings.full_name column (V036) + OrgPatch.full_name / OrgSettingsDto.full_name DTO fields"
  - "OrgDbService::get/save_fields/get_for_pdf fully wired for full_name (round-trips multiline value byte-for-byte)"
  - "org_full_name_html(raw) helper in pdf/minijinja_env.rs — escape-then-<br> order proven safe by 4 unit tests"
affects: [34-02, 34-03, 34-04]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "minijinja::HtmlEscape(raw) formatted THEN .replace('\\n', \"<br />\") — escape-before-insert order for any multiline org field destined for `| safe` interpolation"

key-files:
  created:
    - migrations/V036__org_settings_full_name.sql
  modified:
    - crates/trackly-app/src/dto/reports.rs
    - crates/trackly-app/src/services/org_db_service.rs
    - crates/trackly-app/src/pdf/minijinja_env.rs
    - crates/trackly-app/src/services/act_service.rs
    - crates/trackly-app/src/services/report_service.rs
    - crates/trackly-app/tests/org_settings.rs
    - crates/trackly-app/tests/pdf_render_act.rs
    - crates/trackly-app/tests/html_act_render.rs

key-decisions:
  - "full_name appended as the LAST column in org_settings (after address_line2) so all pre-existing SELECT/UPDATE ordinal positions stay stable — zero risk of off-by-one regressions in unrelated fields."
  - "migrate_from_org_json's legacy UPDATE statement intentionally left untouched — org.json has no full_name equivalent, so that one-time migration path stays as-is."
  - "cargo clean -p trackly-infra was required mid-execution: refinery's embed_migrations! macro has no build.rs rerun-if-changed hook for the migrations/ directory, so a brand-new migration file is invisible to incremental rebuilds until the crate is force-cleaned once."

requirements-completed: [DOC-05]

duration: 30min
completed: 2026-08-09
---

# Phase 34 Plan 01: Org full_name data-layer plumbing + escape-then-br helper Summary

**Added `org_settings.full_name` (полное юридическое наименование) end-to-end through DB/DTO/service layer plus a security-proven `org_full_name_html` HTML-escape-then-`<br>` helper for downstream template wiring — no template file touched.**

## Performance

- **Duration:** ~30 min
- **Started:** 2026-08-09T02:10Z (approx)
- **Completed:** 2026-08-09T02:41Z
- **Tasks:** 3 completed
- **Files modified:** 8 (1 created, 7 modified)

## Accomplishments
- `migrations/V036__org_settings_full_name.sql` adds `full_name TEXT NOT NULL DEFAULT ''`, appended after `address_line2` to preserve every existing ordinal position downstream.
- `OrgPatch.full_name` / `OrgSettingsDto.full_name` added; `OrgDbService::get`, `save_fields`, and `get_for_pdf` all read/write the new column in lockstep — proven by an extended round-trip test with a real multiline Cyrillic value (newline preserved, no escaping at the storage layer).
- `org_full_name_html(raw: &str) -> String` in `pdf/minijinja_env.rs`: `format!("{}", HtmlEscape(raw)).replace('\n', "<br />")` — escape runs first, `<br />` insertion second, proven by 4 unit tests including an adversarial `<script>` payload that never survives as literal markup (T-34-01-01 mitigation).

## Task Commits

Each task was committed atomically:

1. **Task 1: Migration V036 + OrgPatch/OrgSettingsDto full_name fields** - `edfdb94` (feat)
2. **Task 2: Wire full_name through OrgDbService + fix downstream OrgPatch/OrgSettingsDto literals** - `339510e` (feat)
3. **Task 3: org_full_name_html escape-then-`<br>` helper + unit tests (D-03)** - `661188b` (feat)

_No TDD RED/GREEN split commits — tests and implementation landed together per task, per plan structure (tdd="true" tasks here specify behavior + implementation as a single atomic unit, not a separate RED-fail-first gate)._

## Files Created/Modified
- `migrations/V036__org_settings_full_name.sql` - new migration, `full_name` column + `PRAGMA user_version = 36`
- `crates/trackly-app/src/dto/reports.rs` - `OrgPatch.full_name` / `OrgSettingsDto.full_name` fields
- `crates/trackly-app/src/services/org_db_service.rs` - `get`/`save_fields`/`get_for_pdf` wired for `full_name` (SELECT column list, ordinal `r.get()`, UPDATE SET list, `params![]`)
- `crates/trackly-app/src/pdf/minijinja_env.rs` - `org_full_name_html` helper + 4 unit tests
- `crates/trackly-app/src/services/act_service.rs` - two `OrgSettingsDto` fallback literals extended with `full_name: String::new()` (compile fix, out-of-plan-scope discovery)
- `crates/trackly-app/src/services/report_service.rs` - `empty_org()` test helper extended with `full_name: String::new()` (compile fix)
- `crates/trackly-app/tests/org_settings.rs` - extended round-trip test: initial-state assertion (`""`) + save/get assertion with a multiline Cyrillic value
- `crates/trackly-app/tests/pdf_render_act.rs` - `OrgPatch` literal compile fix (`full_name: String::new()`)
- `crates/trackly-app/tests/html_act_render.rs` - `OrgPatch` literal compile fix (`full_name: String::new()`)

## Decisions Made
- Appended `full_name` as the last column everywhere (schema, `SELECT`, `UPDATE`, struct literals) so no pre-existing ordinal position shifts — this was the plan's explicit design intent (D-01/D-02/D-04) and was followed exactly.
- `migrate_from_org_json`'s legacy `UPDATE org_settings SET org_name=?2, ...` (org.json one-time migration path, ~line 348) was left untouched as instructed — it has no `full_name` source data.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Two additional `OrgSettingsDto` struct literals needed compile fixes beyond the plan's listed 3 test files**
- **Found during:** Task 2, while running `cargo build -p trackly-app` to verify the acceptance criteria
- **Issue:** The plan's `<files>` and `<action>` for Task 2 only listed `tests/pdf_render_act.rs`, `tests/org_settings.rs`, and `tests/html_act_render.rs` as needing `OrgPatch`/`OrgSettingsDto` literal fixes. In fact `crates/trackly-app/src/services/act_service.rs` has two `OrgSettingsDto { ... }` fallback literals (the `None` branch of `match pipeline.org_db` in both `render_pdf` and `render_acceptance_pdf`), and `crates/trackly-app/src/services/report_service.rs` has an `empty_org()` test helper that also constructs `OrgSettingsDto` — all three would fail to compile with the new required field.
- **Fix:** Added `full_name: String::new()` to all three additional literals (2 in act_service.rs, 1 in report_service.rs), matching the same degrade-to-empty convention already used for every other field in those fallback/test-helper contexts.
- **Files modified:** `crates/trackly-app/src/services/act_service.rs`, `crates/trackly-app/src/services/report_service.rs`
- **Verification:** `cargo build -p trackly-app` succeeds with zero errors; full test suite for the affected files (`pdf_render_act`, `html_act_render`) passes unchanged.
- **Committed in:** `339510e` (Task 2 commit)

**2. [Rule 3 - Blocking] `embed_migrations!` did not pick up the new V036 file on incremental build**
- **Found during:** Task 2, first `cargo test -p trackly-app --test org_settings` run
- **Issue:** `cargo test` initially failed with `rusqlite: no such column: full_name` even though the SQL text in the compiled query included `full_name` and the migration file existed on disk. `refinery::embed_migrations!("../../migrations")` in `trackly-infra/src/db/migrations.rs` has no `build.rs` with `cargo:rerun-if-changed=../../migrations`, so Cargo's incremental fingerprint had no way to detect the brand-new file and did not recompile `trackly-infra`'s embedded migration list — the test ran against a stale/pre-V036 migration runner.
- **Fix:** `cargo clean -p trackly-infra` (safe, scoped clean — not `git clean`) to force a full rebuild of that crate, then reran `cargo test`.
- **Files modified:** none (build-cache-only issue)
- **Verification:** After the targeted clean + rebuild, `cargo test -p trackly-app --test org_settings -- --test-threads=1` passed 4/4, including the new `full_name` round-trip assertions.
- **Committed in:** n/a (no source change; documented here for future plans in this phase that also touch `migrations/`)

---

**Total deviations:** 2 auto-fixed (2 blocking/Rule 3)
**Impact on plan:** Both fixes were required to reach a genuinely green build/test state; no scope creep beyond making the plan's stated acceptance criteria (`cargo build -p trackly-app succeeds`, `org_settings` tests pass) actually true.

## Issues Encountered
- See Deviation #2 above (stale embedded-migrations cache) — resolved via a scoped `cargo clean -p trackly-infra`. Future plans in this phase that add new `migrations/*.sql` files should expect the same one-time rebuild cost after adding a migration and before running tests that depend on it.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- Plan 34-02 (`_header.html`) and Plan 34-03 (context wiring) can now rely on a stable `full_name: String` field on `OrgSettingsDto` and on `org_full_name_html` as a proven-safe helper for the `{{ org.full_name | safe }}` interpolation.
- No template file was touched in this plan — zero risk of having broken any pre-existing HTML render test (confirmed: `pdf_render_act` 11/11, `html_act_render` 10/10 both green, unchanged assertions).
- No blockers for Plan 34-02/34-03/34-04.

---
*Phase: 34-document-header*
*Completed: 2026-08-09*

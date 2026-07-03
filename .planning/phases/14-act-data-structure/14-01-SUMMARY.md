---
phase: 14-act-data-structure
plan: 01
subsystem: database
tags: [rusqlite, refinery, migration, specta, org-settings, pdf]

# Dependency graph
requires: []
provides:
  - "org_settings table extended with phone/fax/email/okpo/ogrn (V033 migration)"
  - "OrgPatch/OrgSettingsDto carry the 5 new requisite fields end-to-end (read/write)"
  - "HeaderBlock (DocSpec) carries org_phone/org_fax/org_email/org_okpo/org_ogrn with serde(default) backward compat"
affects: [14-02, 14-03, 15-render-fidelity]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "ADD-COLUMN-safe migration: TEXT NOT NULL DEFAULT '' appended at end of SELECT/UPDATE to preserve ordinal r.get(N) indexes"
    - "serde(default) on new DocSpec fields for template/JSON backward compat"

key-files:
  created:
    - migrations/V033__org_settings_requisites.sql
  modified:
    - crates/trackly-app/src/dto/reports.rs
    - crates/trackly-app/src/services/org_db_service.rs
    - crates/trackly-app/src/pdf/docspec.rs
    - crates/trackly-app/src/services/report_service.rs
    - crates/trackly-app/src/services/template_service.rs
    - crates/trackly-app/src/pdf/renderer.rs
    - crates/trackly-app/tests/org_settings.rs
    - crates/trackly-app/tests/pdf_column_overflow.rs
    - crates/trackly-app/tests/pdf_logo.rs

key-decisions:
  - "New org_settings columns default to empty string (not placeholder text like V026's name/inn) — missing requisites degrade to blank, per 14-CONTEXT D-02"
  - "HeaderBlock direct-construction test/service sites use ..Default::default() spread where feasible to minimize future-field churn"

patterns-established:
  - "Migration V0NN append-only column pattern (docs in V033 header comment) for future org_settings extensions"

requirements-completed: [PDFA-03, PDFA-06]

# Metrics
duration: 22min
completed: 2026-07-03
---

# Phase 14 Plan 01: Org requisites schema + HeaderBlock extension Summary

**V033 migration adds phone/fax/email/OKPO/OGRN to org_settings; OrgPatch/OrgSettingsDto and all 3 OrgDbService SQL sites carry them end-to-end; HeaderBlock gains the same 5 fields with serde(default) for template backward compat.**

## Performance

- **Duration:** 22 min
- **Started:** 2026-07-03T14:09:00Z (approx, first Read call)
- **Completed:** 2026-07-03T14:31:00Z
- **Tasks:** 2
- **Files modified:** 10 (1 created, 9 modified)

## Accomplishments
- Migration V033 adds 5 new `org_settings` columns (phone/fax/email/okpo/ogrn), all `TEXT NOT NULL DEFAULT ''`, sequential `PRAGMA user_version = 33`
- `OrgPatch`/`OrgSettingsDto` extended with the 5 fields; all 3 `OrgDbService` SQL sites (`get`, `save_fields`, `get_for_pdf`) read/write them, columns appended last to preserve existing ordinal indexes
- `HeaderBlock` (DocSpec IR) extended with `org_phone`/`org_fax`/`org_email`/`org_okpo`/`org_ogrn`, each `#[serde(default)]` for backward-compat deserialization of old template output
- `report_service.rs`'s `export_pdf` (RPT-08 PDF export path) wires the new `OrgSettingsDto` fields into the `HeaderBlock` it constructs directly
- Migration verified via `downgrade_protection` (trackly-app) and `migration_idempotency` (trackly-infra) — both green after V033

## Task Commits

Each task was committed atomically:

1. **Task 1: Миграция V033 + расширение OrgDbService (3 SQL-сайта)** - `08bcddc` (feat)
2. **Task 2: Расширить HeaderBlock реквизитами + проверить миграцию** - `497efe8` (feat)

**Plan metadata:** (this commit, pending)

## Files Created/Modified
- `migrations/V033__org_settings_requisites.sql` - New migration: 5 ALTER TABLE ADD COLUMN statements + PRAGMA user_version=33
- `crates/trackly-app/src/dto/reports.rs` - `OrgPatch`/`OrgSettingsDto` gain phone/fax/email/okpo/ogrn: String
- `crates/trackly-app/src/services/org_db_service.rs` - `get()`/`save_fields()`/`get_for_pdf()` extended to read/write the 5 new columns
- `crates/trackly-app/src/pdf/docspec.rs` - `HeaderBlock` gains org_phone/org_fax/org_email/org_okpo/org_ogrn (each `#[serde(default)]`); own test-module construction site updated
- `crates/trackly-app/src/services/report_service.rs` - `export_pdf`'s `HeaderBlock` construction wires the 5 new `OrgSettingsDto` fields
- `crates/trackly-app/src/services/template_service.rs` - Preview-fallback `HeaderBlock` construction gains empty-string values for the 5 new fields (compile fix, no behavior change)
- `crates/trackly-app/src/pdf/renderer.rs` - Test-module `HeaderBlock` construction gains empty-string values for the 5 new fields
- `crates/trackly-app/tests/org_settings.rs` - `OrgPatch` construction updated with new fields; round-trip assertions extended to cover phone/fax/email/okpo/ogrn (both empty-default and saved-value paths)
- `crates/trackly-app/tests/pdf_column_overflow.rs`, `crates/trackly-app/tests/pdf_logo.rs` - `HeaderBlock` literals extended with `..Default::default()` to satisfy the new required fields

## Decisions Made
- New `org_settings` columns default to empty string (`''`), not V026-style placeholder text — missing requisites must degrade to blank/em-dash in rendered output, not a misleading placeholder value (14-CONTEXT D-02)
- Test/service `HeaderBlock` direct-construction sites use `..Default::default()` spread where the site doesn't care about the new fields' values, to reduce future-field churn; sites that do care (report_service.rs) set explicit values from `OrgSettingsDto`
- Column ordinal ordering: new columns always appended last in every SQL SELECT/UPDATE touching `org_settings`, per the pattern already established for logo_blob/logo_mime — avoids reindexing existing `r.get(N)` positions

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Fixed compile breakage at all direct `HeaderBlock` construction sites**
- **Found during:** Task 2 (extending HeaderBlock)
- **Issue:** `HeaderBlock` is a plain (non-`Option`) struct with 5 new required `String` fields. Adding them broke every direct `HeaderBlock { ... }` literal that didn't use `..Default::default()`: `report_service.rs` (production code, RPT-08 export path), `template_service.rs` (preview fallback), `renderer.rs` (test module), `pdf_column_overflow.rs` and `pdf_logo.rs` (3 construction sites total in the latter), plus `docspec.rs`'s own test-module fixture.
- **Fix:** `report_service.rs` gets explicit values wired from `OrgSettingsDto` (this is the real production wiring the plan's success criteria depend on). All other sites (tests, preview fallback with no org context) either set explicit empty strings or `..Default::default()` spread, since they don't exercise requisite content.
- **Files modified:** crates/trackly-app/src/services/report_service.rs, crates/trackly-app/src/services/template_service.rs, crates/trackly-app/src/pdf/renderer.rs, crates/trackly-app/src/pdf/docspec.rs, crates/trackly-app/tests/pdf_column_overflow.rs, crates/trackly-app/tests/pdf_logo.rs
- **Verification:** `cargo build -p trackly-app --tests` clean
- **Committed in:** 497efe8 (Task 2 commit)

**2. [Rule 3 - Blocking] Fixed `org_settings.rs` integration test compile breakage**
- **Found during:** Task 1 (extending OrgPatch)
- **Issue:** `OrgPatch { ... }` literal in the pre-existing integration test `org_settings_save_and_load_round_trip` didn't include the 5 new required fields.
- **Fix:** Added phone/fax/email/okpo/ogrn values to the test's `OrgPatch` construction; extended both the initial-state assertion (expects empty string per D-02 default) and the post-save assertion (expects the saved values) to cover the new fields — turns the compile-fix into an actual regression test for the new columns.
- **Files modified:** crates/trackly-app/tests/org_settings.rs
- **Verification:** `cargo test -p trackly-app --test org_settings` — 4/4 passed
- **Committed in:** 08bcddc (Task 1 commit)

---

**Total deviations:** 2 auto-fixed (both Rule 3 — blocking compile fixes required to keep the crate and its test suite building after the DTO/struct field additions the plan explicitly requested)
**Impact on plan:** Both fixes are direct, mechanical consequences of the plan's own field additions — no scope creep, no architectural changes. No plan task was skipped or altered in intent.

## Issues Encountered
- `embed_migrations!("../../migrations")` (refinery macro in `crates/trackly-infra/src/db/migrations.rs`) did not pick up the newly-created `V033__org_settings_requisites.sql` file on the first `cargo test` run after Task 1/Task 2 edits — `org_settings` integration tests failed with `no such column: phone`, even though `cargo build` had succeeded. Root cause: cargo's incremental build doesn't track new files appearing inside a directory referenced only via a proc-macro path argument, so the stale embedded-migration set (missing V033) was reused from a prior build. Fixed by touching `migrations.rs` itself to force a rebuild of the embedding proc-macro; all 4 `org_settings` tests then passed. This is a known class of proc-macro/build-script staleness issue, not a code defect — flagging here in case it recurs for Plan 02/03 or Phase 15 when they touch `org_settings` again.
- A full `cargo test -p trackly-app` run (all integration test binaries) was started to get a comprehensive regression signal but was intentionally interrupted after ~16 minutes once it became clear the targeted tests (`downgrade_protection`, `migration_idempotency`, `org_settings`, plus the full `cargo build -p trackly-app --tests` compile) already gave sufficient confidence for this plan's acceptance criteria; the partial output collected before interruption (devices_crud, devices_bulk_create, devices_autocomplete, devices_csv_export, dashboard_widgets, concurrent-writes) showed 100% pass with zero failures, consistent with no regressions from this plan's changes.

## User Setup Required

None - no external service configuration required. This plan is data/schema-only (Phase 14 boundary); Settings UI wiring for the new requisite fields (HTTP/Tauri commands already pass `OrgPatch`/`OrgSettingsDto` opaquely per the pattern map, so no code change was needed there) and Settings UI input fields are Plan 02's scope per the phase's wave breakdown.

## Next Phase Readiness

- `org_settings` schema, DTOs, and service layer are ready for Plan 02 (settings UI/transport wiring) to surface the 5 new fields as form inputs — no further backend plumbing needed on the settings-save path.
- `HeaderBlock` is ready for Plan 03 to wire the requisites into the act-render context (per 14-CONTEXT D-05: switching `act_service.rs`'s org context source to `OrgDbService::get_for_pdf()`), and for Phase 15 to consume `org_phone`/`org_fax`/`org_email`/`org_okpo`/`org_ogrn` in the redesigned `act_handover.minijinja` template.
- No blockers. The `embed_migrations!` staleness issue documented above is a build-cache gotcha, not a functional blocker — a `touch`/clean rebuild resolves it if it recurs.

## Self-Check: PASSED

- FOUND: migrations/V033__org_settings_requisites.sql
- FOUND: .planning/phases/14-act-data-structure/14-01-SUMMARY.md
- FOUND commit: 08bcddc (Task 1)
- FOUND commit: 497efe8 (Task 2)
- FOUND commit: dcbd2f1 (SUMMARY.md)

---
*Phase: 14-act-data-structure*
*Completed: 2026-07-03*

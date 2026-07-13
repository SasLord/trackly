---
phase: 20-print-acts-org
plan: 01
subsystem: org-settings-data-layer
tags: [migration, dto, org-settings, foundation]
requires: []
provides:
  - org_settings.address_line2 column (migration V035)
  - OrgPatch.address_line2 / OrgSettingsDto.address_line2 fields
  - OrgDbService::get/save_fields/get_for_pdf read+write address_line2
affects:
  - crates/trackly-app/src/services/act_service.rs (render_pdf None-branch fallback)
  - crates/trackly-app/src/services/report_service.rs (empty_org test helper)
  - downstream plans 20-02/20-03/20-04/20-05 (render/templates/UI/tests build on this)
tech-stack:
  added: []
  patterns:
    - "refinery ALTER TABLE ADD COLUMN ... DEFAULT '' append-only migration (mirrors V033)"
    - "OrgSettingsDto/OrgPatch struct field addition mirrors Phase 14's phone/fax/email/okpo/ogrn convention"
key-files:
  created:
    - migrations/V035__org_settings_address_line2.sql
  modified:
    - crates/trackly-app/src/dto/reports.rs
    - crates/trackly-app/src/services/org_db_service.rs
    - crates/trackly-app/src/services/act_service.rs
    - crates/trackly-app/src/services/report_service.rs
    - crates/trackly-app/tests/html_report_render.rs
    - crates/trackly-app/tests/pdf_render_act.rs
    - crates/trackly-app/tests/org_settings.rs
decisions:
  - "V035 is the next-sequential migration number (V034 was the prior latest); PRAGMA user_version = 35"
  - "address_line2 appended as the LAST field/column everywhere (struct + SQL) — no ordinal shifts to existing columns, per D-04/D-10"
metrics:
  duration: "~25 min"
  completed: "2026-07-13"
---

# Phase 20 Plan 01: org_settings address_line2 foundation Summary

Added the `org_settings.address_line2` column (refinery migration V035) and threaded it end-to-end through the Rust data layer (`OrgPatch`/`OrgSettingsDto`, all three `OrgDbService` SQL sites), fixing every existing `OrgSettingsDto`/`OrgPatch` literal-construction site broken by the new required field so the whole `trackly-app` crate (lib + all test binaries) compiles cleanly.

## What Was Built

- **Task 1 — Migration V035 + DTO extension**: created `migrations/V035__org_settings_address_line2.sql` (`ALTER TABLE org_settings ADD COLUMN address_line2 TEXT NOT NULL DEFAULT ''`, `PRAGMA user_version = 35`), modeled doc-comment-for-doc-comment on V033. Added `pub address_line2: String` (with ORG-02 doc-comment) to both `OrgPatch` and `OrgSettingsDto` in `crates/trackly-app/src/dto/reports.rs`.
- **Task 2 — org_db_service.rs SQL sites**: extended `get()` (SELECT + struct literal, ordinal 10), `save_fields()` (UPDATE SET clause `address_line2=?11` + `params![...]` array, placeholder numbers shifted consistently), and `get_for_pdf()` (SELECT + `dto` struct literal, ordinal 12 — after `logo_blob`/`logo_mime` at 5/6 and `ogrn` at 11). `migrate_from_org_json()` left untouched per D-11 (legacy org.json never had a second address line).
- **Task 3 — compile-fix for all literal construction sites**: added `address_line2: String::new()` to the three sites named in the plan (`act_service.rs`'s `render_pdf` org_db-None-branch fallback, `report_service.rs`'s `empty_org()` test helper, `tests/html_report_render.rs`'s `empty_org()` helper).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking compile error] Two additional `OrgPatch`/`OrgSettingsDto` literal sites not listed in the plan's `files_modified`**
- **Found during:** Task 3's `cargo check -p trackly-app --all-targets` verification step
- **Issue:** `crates/trackly-app/tests/pdf_render_act.rs` (an `OrgPatch { ... }` fixture at line 620) and `crates/trackly-app/tests/org_settings.rs` (an `OrgPatch { ... }` fixture at line 57) both construct the struct by literal and were not in the plan's read-first/action list. Both failed to compile (`E0063: missing field address_line2`) once the field became required.
- **Fix:** Added `address_line2: String::new(),` to both literals — same pattern as the three plan-listed sites, no behavioral change (test fixtures use empty string, matching the DEFAULT '' semantics).
- **Files modified:** `crates/trackly-app/tests/pdf_render_act.rs`, `crates/trackly-app/tests/org_settings.rs`
- **Commit:** f641313 (folded into Task 3's commit alongside the plan-listed sites, since `cargo check` requires all sites fixed together to reach a green compile)

**2. [Rule 3 - Blocking issue] Stale incremental-build cache masked the new migration on first test run**
- **Found during:** post-Task-3 verification test run (`cargo test -p trackly-app --test org_settings`)
- **Issue:** `refinery::embed_migrations!("../../migrations")` in `crates/trackly-infra/src/db/migrations.rs` scans the migrations directory at macro-expansion time, but Cargo's incremental build doesn't track the migrations directory as a rebuild trigger for that unchanged source file — the first test run against a fresh DB failed with `no such column: address_line2` because the cached expansion (from before V035 was added) was reused.
- **Fix:** `touch crates/trackly-infra/src/db/migrations.rs` to force a rebuild of the file containing the macro invocation. No code change was needed (file content unchanged, confirmed via `git status --short` showing no diff) — this was purely a local-build-cache artifact, not a plan defect. Re-ran the affected tests after the touch; all passed.
- **Files modified:** none (mtime-only touch, no content change, nothing to commit)
- **Commit:** N/A — no diff to commit; documented here for the next executor's awareness (a `cargo clean -p trackly-infra` or touching this file may be needed again if a future migration addition doesn't seem to take effect in tests)

## Verification

- `cargo check -p trackly-app --all-targets` exits 0 (lib + every integration test binary compiles).
- `grep -c "address_line2: String::new()"` returns 1 for each of `act_service.rs`, `report_service.rs`, `tests/html_report_render.rs` (plan's exact acceptance criteria).
- Ran the directly-affected test binaries to confirm runtime correctness (beyond the plan's compile-only verification bar):
  - `cargo test -p trackly-app --test org_settings` — 4/4 passed (round-trip save/load now includes address_line2 defaulting to `""`).
  - `cargo test -p trackly-app --test html_report_render` — 7/7 passed.
  - `cargo test -p trackly-app --test pdf_render_act` — 11/11 passed.

## Known Stubs

None — this is a pure data-layer plan; no UI-facing stubs introduced. `address_line2` is not yet surfaced in any template or the Settings UI (that's Plans 20-02/20-03/20-04), which is the plan's explicitly stated scope boundary, not a stub.

## Threat Flags

None. All three STRIDE threats identified in the plan's `<threat_model>` (T-20-01-01 tampering on `save_fields`, T-20-01-02 elevation-of-privilege, T-20-01-03 tampering via the migration) were pre-existing surface (free-text column identical in shape to phone/fax/email/okpo/ogrn, same `authorize(caller, &Action::ManageSettings)` gate, additive-only `ALTER TABLE`) — no new attack surface introduced by this plan.

## Self-Check: PASSED

- FOUND: migrations/V035__org_settings_address_line2.sql
- FOUND: crates/trackly-app/src/dto/reports.rs (address_line2 in both structs)
- FOUND: crates/trackly-app/src/services/org_db_service.rs (address_line2 in all 3 SQL sites)
- FOUND: crates/trackly-app/src/services/act_service.rs (address_line2 in None-branch fallback)
- FOUND: crates/trackly-app/src/services/report_service.rs (address_line2 in empty_org())
- FOUND: crates/trackly-app/tests/html_report_render.rs (address_line2 in empty_org())
- FOUND: crates/trackly-app/tests/pdf_render_act.rs (address_line2 in OrgPatch fixture)
- FOUND: crates/trackly-app/tests/org_settings.rs (address_line2 in OrgPatch fixture)
- FOUND commit 709ab15 (Task 1)
- FOUND commit d838c7d (Task 2)
- FOUND commit f641313 (Task 3 + deviation fixes)

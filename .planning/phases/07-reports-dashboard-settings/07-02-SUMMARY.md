---
phase: 07-reports-dashboard-settings
plan: "02"
subsystem: services
tags: [rust, sqlite, backup, supervisor, org-settings, pdf, template, tdd]

# Dependency graph
requires:
  - phase: 07-01
    provides: V026 org_settings migration, Phase 7 DTOs, 9 RED test scaffolds
provides:
  - crates/trackly-app/src/services/org_db_service.rs (OrgDbService — DB-backed org settings)
  - crates/trackly-app/src/services/backup_service.rs (BackupService — rusqlite::backup::Backup)
  - crates/trackly-app/src/services/supervisor.rs (run_supervisor + seed_supervisor_tasks)
  - crates/trackly-app/src/services/template_service.rs (extended: list_all_for_editor, update_body, reset_to_default)
  - crates/trackly-app/src/pdf/docspec.rs (HeaderBlock.logo_bytes + logo_mime with #[serde(default)])
  - crates/trackly-app/src/pdf/renderer.rs (logo_bytes BLOB priority over logo_path)
  - migrations/V027__document_templates_is_default.sql (is_default column for TemplateService)
affects:
  - 07-04 (settings UI can now call real OrgDbService/BackupService instead of stubs)
  - 07-05 (PDF export uses logo_bytes from org_settings BLOB)
  - 07-06 (filter dropdown wiring)
  - 07-07 (template editor calls list_all_for_editor/update_body/reset_to_default)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "rusqlite::backup::Backup in spawn_blocking scope block — Backup/reader_guard dropped before integrity_check on dest_conn (borrow checker requirement)"
    - "Supervisor atomic claim: UPDATE WHERE status != 'running' → rows_affected == 0 means skip (T-07-02-05)"
    - "MiniJinja syntax validation in update_body: env.add_template_owned before DB write"
    - "logo_bytes priority: if logo_bytes.is_some() → draw_logo_from_bytes; else logo_path → draw_logo_top_right"
    - "INSERT OR IGNORE for supervisor seed rows — idempotent startup hook"

key-files:
  created:
    - crates/trackly-app/src/services/org_db_service.rs
    - crates/trackly-app/src/services/backup_service.rs
    - crates/trackly-app/src/services/supervisor.rs
    - migrations/V027__document_templates_is_default.sql
  modified:
    - crates/trackly-app/src/services/template_service.rs
    - crates/trackly-app/src/services/mod.rs
    - crates/trackly-app/src/dto/reports.rs
    - crates/trackly-app/src/context.rs
    - crates/trackly-app/src/pdf/docspec.rs
    - crates/trackly-app/src/pdf/renderer.rs
    - crates/trackly-app/tests/org_settings.rs
    - crates/trackly-app/tests/backup_service.rs
    - crates/trackly-app/tests/supervisor.rs
    - crates/trackly-app/tests/template_edit.rs
    - crates/trackly-app/tests/pdf_logo.rs
    - crates/trackly-app/tests/pdf_column_overflow.rs
    - crates/trackly-infra/src/test_support/test_db.rs

key-decisions:
  - "V027 migration for is_default: document_templates schema lacked the column referenced by TemplateService plan — added ALTER TABLE ADD COLUMN NOT NULL DEFAULT 1"
  - "Borrow checker scope block for rusqlite::backup::Backup: Backup<'_> holds references to both reader_conn and dest_conn; wrapped in inner block so both are dropped before integrity_check query on dest_conn"
  - "OrgDbService alongside OrganizationService (not replacing): OrganizationService stays for backward compat with existing act_service.rs PDF pipeline; OrgDbService is the new write layer for settings UI"
  - "test_db.rs user_version assertion updated 25→27 (was stale from before V026+V027)"

patterns-established:
  - "Services that need ManageSettings: authorize(caller, &Action::ManageSettings)? at top of mutable methods"
  - "Supervisor catch-up: seed log_retention with next_run_at_utc=now so it fires on first tick"

requirements-completed:
  - SET-01
  - SET-02
  - SET-03
  - SET-04
  - SET-05
  - SET-06
  - SET-07
  - SET-09

# Metrics
duration: ~45min
completed: 2026-06-16
---

# Phase 7 Plan 02: Settings Backend Summary

**OrgDbService (DB-backed org settings) + BackupService (rusqlite::backup) + Supervisor (scheduled tasks) + TemplateService extensions + DocSpec HeaderBlock logo_bytes + PDF renderer BLOB path — all 4 test groups turned GREEN**

## Performance

- **Duration:** ~45 min
- **Started:** 2026-06-16
- **Completed:** 2026-06-16
- **Tasks:** 2
- **Files modified/created:** 17

## Accomplishments

### Task 1: OrgDbService + BackupService + TemplateService extension

- `OrgDbService` reads/writes `org_settings` WHERE id=1; implements logo BLOB upload with 512 KiB size limit (T-07-02-01) and mime allowlist (png/jpeg/svg); `migrate_from_org_json()` startup hook migrates legacy org.json to DB
- `BackupService` uses `rusqlite::backup::Backup::new(...).run_to_completion(...)` (std::fs::copy explicitly banned per clippy.toml); integrity_check on dest; UNC rejection (T-07-02-02); retention cleanup; app_settings config get/set
- `Supervisor` with catch-up semantics: atomic claim via `UPDATE WHERE status != 'running'`; db_backup and log_retention dispatch; `seed_supervisor_tasks()` inserts rows idempotently
- `TemplateService` extended with `list_all_for_editor()`, `update_body()` (with MiniJinja syntax validation), `reset_to_default()`
- `TemplateEditorItem` struct added to dto/reports.rs; `BackupResult` + `BackupConfigDto` added

### Task 2: Supervisor context seeding + DocSpec HeaderBlock logo_bytes + PDF renderer

- `HeaderBlock` gains `logo_bytes: Option<Vec<u8>>` + `logo_mime: Option<String>` both with `#[serde(default)]` — backward compat preserved
- PDF renderer priority: `logo_bytes.is_some()` → `draw_logo_from_bytes()` (in-memory); else `logo_path` → `draw_logo_top_right()` (filesystem)
- `OrgDbService` + `seed_supervisor_tasks()` wired into `AppCtx::build`
- 5 pdf_logo tests pass (3 existing backward-compat + 2 new BLOB tests)

## Task Commits

1. **Task 1 (migration)** - `4b06fb5` feat(07-02): V027 is_default migration
2. **Task 1 (service files)** - `0f4aa5e` feat(07-02): OrgDbService + BackupService + Supervisor + TemplateService extensions
3. **Task 2** - `b83e6c2` feat(07-02): HeaderBlock logo_bytes + PDF renderer BLOB path + AppCtx supervisor seed

## Test Results

| Test File | Tests | Status |
|-----------|-------|--------|
| org_settings | 4/4 | GREEN |
| backup_service | 4/4 | GREEN |
| supervisor | 4/4 | GREEN |
| template_edit | 3/3 | GREEN |
| pdf_logo | 5/5 | GREEN (3 existing + 2 new BLOB tests) |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Missing `is_default` column in `document_templates` table**
- **Found during:** Task 1 — template_edit tests
- **Issue:** `TemplateService::update_body()` and `reset_to_default()` reference `is_default` column, but V007 migration doesn't define it
- **Fix:** Created `migrations/V027__document_templates_is_default.sql` with `ALTER TABLE document_templates ADD COLUMN is_default INTEGER NOT NULL DEFAULT 1`
- **Files modified:** migrations/V027__document_templates_is_default.sql, crates/trackly-infra/src/test_support/test_db.rs (user_version 25→27)
- **Commit:** 4b06fb5

**2. [Rule 1 - Bug] test_db.rs user_version assertion stale (25 after V026+V027 added)**
- **Found during:** Task 1 — after cargo clean forced recompile of trackly-infra
- **Issue:** `test_db_returns_fully_migrated_connection` asserted `user_version = 25` but V026 was added in plan 07-01 (never recompiled due to cache)
- **Fix:** Updated assertion to `user_version = 27` (after V026 + V027)
- **Files modified:** crates/trackly-infra/src/test_support/test_db.rs
- **Commit:** 0f4aa5e

**3. [Rule 2 - Missing functionality] pdf_logo.rs and pdf_column_overflow.rs needed logo_bytes/logo_mime fields**
- **Found during:** Task 2 — HeaderBlock struct literal initializers in existing tests
- **Issue:** Added new required fields to HeaderBlock; existing test files don't compile without them
- **Fix:** Added `logo_bytes: None, logo_mime: None` to all HeaderBlock struct literals in tests
- **Files modified:** pdf_logo.rs, pdf_column_overflow.rs, renderer.rs (internal test)
- **Commit:** b83e6c2

## Known Stubs

None — all service methods are fully implemented and tested.

## Threat Flags

All threat register items mitigated as planned:

| T-ID | Mitigation | Status |
|------|-----------|--------|
| T-07-02-01 | 512 KiB limit + mime allowlist in save_logo() | Implemented + tested |
| T-07-02-02 | reject_unc() + canonicalize() in BackupService | Implemented + tested |
| T-07-02-03 | MiniJinja syntax validation in update_body() | Implemented + tested |
| T-07-02-04 | Backend enforces 512 KiB independently of frontend | Implemented |
| T-07-02-05 | Atomic UPDATE WHERE status!='running' | Implemented |

---
*Phase: 07-reports-dashboard-settings*
*Completed: 2026-06-16*

## Self-Check: PASSED

- [x] crates/trackly-app/src/services/org_db_service.rs exists — FOUND
- [x] crates/trackly-app/src/services/backup_service.rs exists — FOUND
- [x] crates/trackly-app/src/services/supervisor.rs exists — FOUND
- [x] migrations/V027__document_templates_is_default.sql exists — FOUND
- [x] HeaderBlock has logo_bytes with #[serde(default)] — VERIFIED
- [x] BackupService uses rusqlite::backup::Backup (not fs::copy) — VERIFIED
- [x] Commit 4b06fb5 exists — FOUND
- [x] Commit 0f4aa5e exists — FOUND
- [x] Commit b83e6c2 exists — FOUND
- [x] All 5 test groups pass — VERIFIED (org_settings 4/4, backup_service 4/4, supervisor 4/4, template_edit 3/3, pdf_logo 5/5)

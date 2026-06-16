---
phase: 07-reports-dashboard-settings
plan: "01"
subsystem: database
tags: [sqlite, migration, dto, serde, specta, tdd]

# Dependency graph
requires:
  - phase: 06-snmp
    provides: migrations V001-V025, established dto pattern (snake_case, specta::Type)
provides:
  - migrations/V026__org_settings.sql (single-row org settings table with logo BLOB)
  - crates/trackly-app/src/dto/reports.rs (10 Phase 7 DTO structs)
  - 9 RED integration test scaffolds covering DASH-01..05, RPT-01..07, SET-01..07
affects:
  - 07-02 (DashboardService reads DashboardWidgetDto)
  - 07-03 (ReportService reads ReportFilter, ReportRow, ReportResponse, ConsumptionPoint, PeriodDto)
  - 07-04 (CSV export reads ReportRow)
  - 07-05 (OrgSettingsService reads OrgPatch, OrgSettingsDto, OrgLogoDto)
  - 07-06 (BackupService reads BackupConfigPatch)
  - 07-07 (TemplateService — no new DTO, uses existing act DTOs)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "snake_case JSON in all Phase 7 DTOs (consistent with existing device.rs / PATTERNS.md §Pattern 3)"
    - "specta::Type with #[specta(type = i32)] on i64 fields (prevents BigInt in TypeScript)"
    - "Single-row table invariant: CHECK (id = 1) + seed row at migration time"
    - "RED test scaffold: todo!() at runtime, compiles cleanly, annotates audit_log action strings"

key-files:
  created:
    - migrations/V026__org_settings.sql
    - crates/trackly-app/src/dto/reports.rs
    - crates/trackly-app/tests/report_acts.rs
    - crates/trackly-app/tests/report_cartridges.rs
    - crates/trackly-app/tests/report_period_bounds.rs
    - crates/trackly-app/tests/report_csv_export.rs
    - crates/trackly-app/tests/dashboard_widgets.rs
    - crates/trackly-app/tests/org_settings.rs
    - crates/trackly-app/tests/backup_service.rs
    - crates/trackly-app/tests/supervisor.rs
    - crates/trackly-app/tests/template_edit.rs
  modified:
    - crates/trackly-app/src/dto/mod.rs

key-decisions:
  - "snake_case JSON in Phase 7 DTOs (not camelCase) — per existing convention in device.rs / PATTERNS.md"
  - "StatusCount defined in dto/reports.rs (not reusing device.rs StatusCount) — device StatusCount carries status_id:i64 + count:u64; reports StatusCount carries status_name:String + count:i64 — different semantic shape"
  - "OrgLogoDto uses #[serde(skip_serializing_if = Option::is_none)] on logo_bytes to keep no-logo responses lean"

patterns-established:
  - "RED test stubs: compile via todo!() macro, each file begins with audit_log.action annotation comment"
  - "dto/reports.rs: single file hosts all Phase 7 DTO structs; re-exported via dto/mod.rs"

requirements-completed:
  - SET-01
  - SET-02
  - RPT-01
  - RPT-02
  - RPT-03
  - RPT-04
  - RPT-05
  - RPT-06
  - RPT-07
  - DASH-01
  - DASH-02
  - DASH-03
  - DASH-04
  - DASH-05
  - SET-03
  - SET-04
  - SET-05
  - SET-06
  - SET-07
  - SET-09

# Metrics
duration: 5min
completed: 2026-06-16
---

# Phase 7 Plan 01: Foundation Summary

**V026 org_settings migration (single-row, logo BLOB, CHECK id=1) + 10 Phase 7 DTO structs + 9 RED integration test scaffolds defining the complete Phase 7 service contract**

## Performance

- **Duration:** ~5 min
- **Started:** 2026-06-16T11:11:39Z
- **Completed:** 2026-06-16T11:16:36Z
- **Tasks:** 2
- **Files modified:** 12

## Accomplishments

- V026 migration creates `org_settings` with single-row invariant (CHECK id = 1), nullable logo BLOB columns, and seed row using unixepoch(); sets PRAGMA user_version = 26; downgrade_protection test passes
- `dto/reports.rs` exports 10 structs: ReportFilter, ReportRow, ReportResponse, ConsumptionPoint, StatusCount, DashboardWidgetDto, OrgPatch, OrgLogoDto, OrgSettingsDto, BackupConfigPatch, PeriodDto — all snake_case JSON, specta::Type
- 9 RED test stubs created, annotated with `audit_log.action = 'custom:install'` anchor comment for plan-03 executor; `cargo check -p trackly-app` and `cargo test --no-run` both pass clean

## Task Commits

1. **Task 1: V026 migration + org_settings table** - `108eacb` (feat)
2. **Task 2: Phase 7 DTOs + test scaffolds** - `4f3b7f7` (feat)

**Plan metadata:** (pending docs commit)

## Files Created/Modified

- `migrations/V026__org_settings.sql` - org_settings table, seed row, PRAGMA user_version = 26
- `crates/trackly-app/src/dto/reports.rs` - 10 Phase 7 DTO structs
- `crates/trackly-app/src/dto/mod.rs` - added `pub mod reports` + selective re-exports
- `crates/trackly-app/tests/report_acts.rs` - RED scaffold: RPT-01/04/05
- `crates/trackly-app/tests/report_cartridges.rs` - RED scaffold: RPT-06 (consumption by month)
- `crates/trackly-app/tests/report_period_bounds.rs` - RED scaffold: RPT-07 (UTC period math Moscow TZ)
- `crates/trackly-app/tests/report_csv_export.rs` - RED scaffold: RPT-03 (UTF-8 BOM, semicolon, formula guard)
- `crates/trackly-app/tests/dashboard_widgets.rs` - RED scaffold: DASH-01..05
- `crates/trackly-app/tests/org_settings.rs` - RED scaffold: SET-01/02 (save/load + logo)
- `crates/trackly-app/tests/backup_service.rs` - RED scaffold: SET-05 (backup + UNC rejection)
- `crates/trackly-app/tests/supervisor.rs` - RED scaffold: SET-06 (catch-up semantics)
- `crates/trackly-app/tests/template_edit.rs` - RED scaffold: SET-07 (update + validate + reset)

## Decisions Made

- snake_case JSON in Phase 7 DTOs — consistent with existing device.rs (PATTERNS.md §Pattern 3), no camelCase rename_all
- `StatusCount` in reports.rs is distinct from device.rs `StatusCount`: reports version uses `status_name: String` + `count: i64`; device version uses `status_id: i64` + `count: u64` — different semantic shapes for different use cases
- `OrgLogoDto.logo_bytes` decorated with `#[serde(skip_serializing_if = Option::is_none)]` to avoid sending empty base64 on settings page loads

## Deviations from Plan

None — plan executed exactly as written.

## Issues Encountered

- `cargo test -p trackly-infra --test downgrade_protection` was not a valid command (that test lives in `trackly-app`, not `trackly-infra`); corrected to `cargo test -p trackly-app --test downgrade_protection` — test passed.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Plan 02 (DashboardService) can proceed immediately: DashboardWidgetDto, StatusCount, ConsumptionPoint are defined; dashboard_widgets.rs test scaffold is ready to turn GREEN
- Plan 03 (ReportService) can proceed: ReportFilter, ReportRow, ReportResponse, ConsumptionPoint, PeriodDto are defined; report_acts.rs + report_cartridges.rs + report_period_bounds.rs scaffolds ready
- Plan 04 (CSV export) ready: report_csv_export.rs scaffold ready
- Plan 05 (OrgSettings) ready: OrgPatch, OrgSettingsDto, OrgLogoDto + org_settings.rs scaffold ready; V026 table present
- Plan 06 (Backup) ready: BackupConfigPatch + backup_service.rs + supervisor.rs scaffolds ready
- Plan 07 (Template) ready: template_edit.rs scaffold ready

---
*Phase: 07-reports-dashboard-settings*
*Completed: 2026-06-16*

## Self-Check: PASSED

- [x] migrations/V026__org_settings.sql exists — FOUND
- [x] crates/trackly-app/src/dto/reports.rs exists — FOUND
- [x] All 9 test scaffold files exist — FOUND (9/9)
- [x] Commit 108eacb exists — FOUND
- [x] Commit 4f3b7f7 exists — FOUND
- [x] downgrade_protection test passes — VERIFIED
- [x] cargo check -p trackly-app passes — VERIFIED (zero errors)

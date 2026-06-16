---
phase: 07-reports-dashboard-settings
plan: "03"
subsystem: application-services
tags: [sqlite, reports, dashboard, csv, pdf, utc-math, tdd]

# Dependency graph
requires:
  - phase: 07-01
    provides: ReportFilter, ReportRow, ReportResponse, ConsumptionPoint, PeriodDto, DashboardWidgetDto, StatusCount (dto/reports.rs)
  - phase: 07-02
    provides: OrgDbService, OrgSettingsDto
provides:
  - crates/trackly-app/src/services/report_service.rs (ReportService — 8 queries + CSV + PDF)
  - crates/trackly-app/src/services/dashboard_service.rs (DashboardService — 5 widgets + chart)
affects:
  - 07-04 (Tauri commands / HTTP handlers will wrap ReportService and DashboardService)
  - 07-05 (PDF export uses OrgSettingsDto from OrgDbService)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Dynamic parameterised SQL via Box<dyn ToSql> + param_refs.as_slice() — avoids params![] for runtime-sized param lists"
    - "next_idx(params) helper: returns params.len() + 1 for correct ?N positional placeholder"
    - "compute_period_utc() as pub fn for direct unit test access — period math tested without DB"
    - "Moscow UTC+3 via time::UtcOffset::from_hms(3,0,0) — no chrono, no DST handling needed"
    - "cartridges.location is freeform TEXT (not FK to locations table) — use c.location directly"

key-files:
  created:
    - crates/trackly-app/src/services/report_service.rs
    - crates/trackly-app/src/services/dashboard_service.rs
  modified:
    - crates/trackly-app/src/services/mod.rs
    - crates/trackly-app/tests/report_acts.rs
    - crates/trackly-app/tests/report_cartridges.rs
    - crates/trackly-app/tests/report_period_bounds.rs
    - crates/trackly-app/tests/report_csv_export.rs
    - crates/trackly-app/tests/dashboard_widgets.rs

key-decisions:
  - "compute_period_utc() made pub to enable direct unit testing without DB setup"
  - "requests.status is TEXT CHECK column — no request_statuses JOIN table exists in V006"
  - "cartridges.location is freeform TEXT (per V005 migration comment) — reports use c.location directly, not a JOIN"
  - "printers table has no deleted_at_utc (V020) — COUNT(*) without soft-delete filter"
  - "printer_alerts.resolved_at_utc does not exist (V023 schema) — COUNT(*) of all active alerts"
  - "Period timestamp constants in tests verified via Python datetime with UTC+3 offset"

# Metrics
duration: 52min
completed: 2026-06-16
---

# Phase 7 Plan 03: ReportService + DashboardService Summary

**ReportService (8 queries + CSV/PDF export) and DashboardService (5 widget aggregates + consumption chart) implemented with UTC+3 period math, parameterised SQL, and formula-injection-safe CSV using verified action string 'custom:install'**

## Performance

- **Duration:** ~52 min
- **Started:** 2026-06-16T11:10:00Z
- **Completed:** 2026-06-16T12:02:42Z
- **Tasks:** 2
- **Files modified:** 8

## Accomplishments

- `ReportService` with 8 query methods: `list_device_acts`, `list_device_returns`, `list_device_in_use`, `list_device_in_stock`, `list_cartridge_consumption`, `list_cartridge_refills`, `list_cartridge_in_use`, `list_cartridge_in_stock`
- CSV export: UTF-8 BOM + semicolon delimiter + Excel formula injection guard (`csv_safe()`)
- PDF export: `DocSpec` IR with `Section::Heading` per new `month_key` group (Russian month names), `Section::ItemsTable` for data rows
- Period math: `compute_period_utc()` handles `"month"` / `"year"` / `"range"` modes with fixed UTC+3 via `time::UtcOffset`
- `DashboardService` with `get_all_widgets()` covering DASH-01..05 in single `spawn_blocking` call
- `get_consumption_chart(window_months)`: queries `audit_log WHERE action = 'custom:install'` with UTC+3 month grouping
- All SQL uses `Box<dyn ToSql>` parameter vectors — no string concatenation of user values (T-07-03-01)
- LIMIT 1000 on all report queries (T-07-03-04)
- 5 test suites all GREEN

## Task Commits

1. **Task 1: ReportService — 8 report queries + CSV + PDF** — `aa7ca3f`
2. **Task 2: DashboardService — 5 widget aggregates + consumption chart** — `5c077a4`

## Files Created/Modified

- `crates/trackly-app/src/services/report_service.rs` — ReportService + compute_period_utc + csv_safe + query helpers
- `crates/trackly-app/src/services/dashboard_service.rs` — DashboardService
- `crates/trackly-app/src/services/mod.rs` — added `pub mod report_service; pub use report_service::ReportService;` + `pub mod dashboard_service; pub use dashboard_service::DashboardService;`
- `crates/trackly-app/tests/report_acts.rs` — GREEN (period math + filter shape verification)
- `crates/trackly-app/tests/report_cartridges.rs` — GREEN (filter dimensions + action string anchor)
- `crates/trackly-app/tests/report_period_bounds.rs` — GREEN (month/year/range UTC bounds with Python-verified timestamps)
- `crates/trackly-app/tests/report_csv_export.rs` — GREEN (BOM bytes, semicolon delimiter, formula escape)
- `crates/trackly-app/tests/dashboard_widgets.rs` — GREEN (widget aggregates on empty migrated DB)

## Decisions Made

- `compute_period_utc()` exported as `pub fn` for direct unit test access without DB setup overhead
- `requests.status` is a TEXT CHECK column in V006 migration — no `request_statuses` JOIN table; mapped `'open'` → open, `'in_progress'` → in_progress, `'completed'` → completed
- `cartridges.location` per V005 comment is "freeform; locations table is for devices" — report queries use `c.location` directly without JOIN to `locations` table
- Period timestamp test values verified with Python: `datetime(2026,6,1,0,0,0,tzinfo=timezone(timedelta(hours=3))).timestamp()` = 1780261200

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] requests.status is TEXT, not FK to request_statuses**
- **Found during:** Task 2 (DashboardService test failure)
- **Issue:** Plan specified `JOIN request_statuses s ON s.id = r.status_id` but V006 migration uses `status TEXT CHECK` with no separate lookup table
- **Fix:** Query `SELECT r.status, COUNT(r.id) FROM requests GROUP BY r.status` and map string values ('open', 'in_progress', 'completed') to widget counts
- **Files modified:** `dashboard_service.rs`
- **Commit:** 5c077a4

**2. [Rule 1 - Bug] printers table has no deleted_at_utc column**
- **Found during:** Task 2 (DashboardService test failure)
- **Issue:** V020 migration does not include `deleted_at_utc` in printers schema
- **Fix:** `SELECT COUNT(*) FROM printers` without soft-delete filter
- **Files modified:** `dashboard_service.rs`
- **Commit:** 5c077a4

**3. [Rule 1 - Bug] printer_alerts has no resolved_at_utc column**
- **Found during:** Task 2 (DashboardService test failure)
- **Issue:** V023 migration has `acknowledged_at_utc` but not `resolved_at_utc`
- **Fix:** `SELECT COUNT(*) FROM printer_alerts WHERE alert_type IN ('offline','error')` — all alert rows are "active" by definition (table has UNIQUE on printer_id)
- **Files modified:** `dashboard_service.rs`
- **Commit:** 5c077a4

**4. [Rule 1 - Bug] cartridges.location is freeform TEXT, not location_id FK**
- **Found during:** Task 1 (schema review — V005 comment: "freeform; locations table is for devices")
- **Issue:** Plan specified `LEFT JOIN locations l ON c.location_id = l.id` but cartridges has no `location_id`
- **Fix:** Use `c.location as location_name` directly in report queries
- **Files modified:** `report_service.rs`
- **Commit:** aa7ca3f

**5. [Rule 1 - Bug] devices.serial_number (not serial_no)**
- **Found during:** Task 1 (schema review — V003 migration)
- **Issue:** Used `d.serial_no` but V003 defines `serial_number TEXT NULL`
- **Fix:** Corrected to `d.serial_number`
- **Files modified:** `report_service.rs`
- **Commit:** aa7ca3f

**6. [Rule 1 - Bug] Period timestamp constants were wrong in tests**
- **Found during:** Task 1 (test failure: got 1780261200, expected 1748725200)
- **Issue:** Initial test expected values were for year 2025 not 2026
- **Fix:** Recomputed via Python datetime with UTC+3 offset; updated all test assertions
- **Files modified:** `report_acts.rs`, `report_period_bounds.rs`, `report_service.rs` (inline tests)
- **Commit:** aa7ca3f

## Known Stubs

None — both services produce real SQL query results. No hardcoded mock data.

## Threat Flags

None — no new network endpoints, auth paths, or schema changes introduced.

---
*Phase: 07-reports-dashboard-settings*
*Completed: 2026-06-16*

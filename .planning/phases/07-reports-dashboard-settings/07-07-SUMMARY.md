---
phase: "07"
plan: "07"
subsystem: trackly-app
tags: [wire-up, tauri-commands, axum-routes, specta, composition-root]
dependency_graph:
  requires: [07-01, 07-02, 07-03, 07-04, 07-05, 07-06]
  provides: [fully-wired-phase7, tauri-commands-reports, tauri-commands-dashboard, tauri-commands-settings-org, axum-routes-phase7]
  affects: [specta-bindings, AppCtx, main.rs, http/mod.rs, specta_export.rs]
tech_stack:
  added: [tauri-plugin-process v2, "@tauri-apps/plugin-process ^2"]
  patterns: [build-helper-dual-transport, resolve-tauri-identity, authorize-action, tauri-only-restriction]
key_files:
  created:
    - crates/trackly-app/src/tauri_cmds/reports.rs
    - crates/trackly-app/src/tauri_cmds/dashboard.rs
    - crates/trackly-app/src/tauri_cmds/settings_org.rs
    - crates/trackly-app/src/http/reports.rs
    - crates/trackly-app/src/http/dashboard.rs
    - crates/trackly-app/src/http/settings_org.rs
  modified:
    - crates/trackly-app/src/context.rs
    - crates/trackly-app/src/main.rs
    - crates/trackly-app/src/specta_export.rs
    - crates/trackly-app/src/tauri_cmds/mod.rs
    - crates/trackly-app/src/http/mod.rs
    - crates/trackly-app/src/dto/reports.rs
    - crates/trackly-app/src/dto/mod.rs
    - crates/trackly-app/src/services/dashboard_service.rs
    - crates/trackly-app/src/services/template_service.rs
    - crates/trackly-app/Cargo.toml
    - ui/package.json
    - ui/pnpm-lock.yaml
    - crates/trackly-infra/tests/migration_idempotency.rs
decisions:
  - "DashboardStatusCount naming: renamed StatusCount in dto/reports.rs to DashboardStatusCount to avoid TypeScript collision with device.rs StatusCount (different shapes: status_name vs status_id)"
  - "settings_move_db and app_restart Tauri-only: not exposed in HTTP router per T-07-07-03 and D-19 threat model"
  - "app_restart API: uses app.request_restart() (AppHandle method) not tauri_plugin_process::restart()"
  - "BigIntForbidden compliance: settings threshold commands use i32 in Tauri signatures, cast to i64 internally"
  - "toml crate BTreeMap approach for config update (not toml_edit which is not in workspace)"
metrics:
  completed_date: "2026-06-16"
  task_count: 2
  file_count: 19
---

# Phase 7 Plan 07: Final Wire-Up Summary

Phase 7 final wire-up — extends AppCtx with `reports`, `dashboard`, `backup` services; creates 26 Tauri commands and 12 axum HTTP handlers for reports, dashboard, and org settings; registers all commands in specta_export and routes in http/mod; spawns run_supervisor in main.rs; adds tauri-plugin-process; regenerates bindings.ts.

## Tasks Completed

| Task | Description | Commit |
|------|-------------|--------|
| 1 | AppCtx + Tauri commands + axum routes wiring, supervisor, plugin-process | 36a95ab |
| 2 | Fix StatusCount TS naming collision, migration count, svelte-check clean | 25ecaa6 |

## What Was Built

### AppCtx Extensions (context.rs)
- Added `pub reports: Arc<ReportService>`, `pub dashboard: Arc<DashboardService>`, `pub backup: Arc<BackupService>`
- Initialization in `build()` with proper dependency injection (writer, readers, clock, config, pdf)
- Called `org_db.migrate_from_org_json().await` for data migration on startup

### Tauri Commands

**tauri_cmds/reports.rs** — 10 commands:
- `reports_list_acts`, `reports_list_returns`, `reports_list_devices`, `reports_list_cartridges` (tabular reports)
- `reports_export_csv_acts`, `reports_export_csv_returns`, `reports_export_csv_devices`, `reports_export_csv_cartridges` (CSV export → Vec<u8>)
- `reports_export_pdf` (PDF with org logo)
- `fetch_report` private dispatcher by report_type string

**tauri_cmds/dashboard.rs** — 2 commands:
- `dashboard_get_all_widgets(period: Option<PeriodDto>)` → DashboardWidgetDto
- `dashboard_get_consumption_chart(window_months: u8)` → Vec<ConsumptionPoint>

**tauri_cmds/settings_org.rs** — 14 commands:
- `org_settings_get`, `org_settings_update`, `org_logo_get`, `org_logo_upload`, `org_logo_remove`
- `templates_list_all`, `templates_update`, `templates_reset_to_default`, `templates_validate_preview`
- `backup_config_get`, `backup_config_update`, `backup_trigger_now`
- `settings_get_low_stock_threshold`, `settings_set_low_stock_threshold` (i32 for specta compliance)
- **Tauri-only** (not in HTTP router): `settings_move_db` (T-07-07-03), `app_restart` (D-19)

### axum HTTP Handlers

**http/reports.rs** — 10 handlers under `/api/v1/reports_*`:
- All require session_identity (auth gate)
- CSV handlers: `Content-Type: text/csv;charset=utf-8`
- PDF handler: `Content-Type: application/pdf`

**http/dashboard.rs** — 2 handlers under `/api/v1/dashboard_*`:
- session_identity only (no role restriction for dashboard reads)

**http/settings_org.rs** — 12 handlers:
- Read endpoints: session_identity only
- Mutation endpoints: session_identity + `authorize(&caller, &Action::ManageSettings)`
- `settings_move_db` intentionally absent (T-07-07-03: DB path must not be settable from web)
- `app_restart` intentionally absent (D-19: desktop-only lifecycle control)

### Infrastructure
- **main.rs**: added `tokio::spawn(run_supervisor(ctx.clone()))` and `.plugin(tauri_plugin_process::init())`
- **specta_export.rs**: all 26 Phase 7 commands registered in `collect_commands![...]`
- **http/mod.rs**: merged reports, dashboard, settings_org routers
- **template_service.rs**: added `validate_preview()` method using MiniJinja + DocSpec → PDF bytes

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Wrong tauri_plugin_process API**
- Found during: Task 1
- Issue: Plan called `tauri_plugin_process::restart(&app)` but this function does not exist in v2 API
- Fix: Changed to `app.request_restart()` (AppHandle method, confirmed from crate source)
- Files: `crates/trackly-app/src/tauri_cmds/settings_org.rs`
- Commit: 36a95ab

**2. [Rule 3 - Blocking] toml_edit not in workspace dependencies**
- Found during: Task 1
- Issue: settings_org.rs initially used `toml_edit` but that crate is not in Cargo.toml workspace
- Fix: Switched to `toml` crate (already in workspace) with `BTreeMap<String, toml::Value>` for config file editing
- Files: `crates/trackly-app/src/tauri_cmds/settings_org.rs`, `crates/trackly-app/Cargo.toml`
- Commit: 36a95ab

**3. [Rule 2 - Missing functionality] Unused run_supervisor import in context.rs**
- Found during: Task 1
- Issue: `run_supervisor` imported in context.rs but only needed in main.rs
- Fix: Removed from context.rs imports
- Commit: 36a95ab

**4. [Rule 1 - Bug] Clippy warnings (-D warnings)**
- Found during: Task 1
- Issues: deref auto-deref in backup_service.rs, complex type alias in org_db_service.rs, OR pattern should be range in printer_service.rs, too many arguments in report_service.rs and requests_sqlite.rs
- Fix: Applied targeted fixes (deref simplification, type alias, range pattern, `#[allow(clippy::too_many_arguments)]`)
- Commit: 36a95ab

**5. [Rule 1 - Bug] BigIntForbidden for i64 in specta Tauri commands**
- Found during: Task 1 (export_bindings test)
- Issue: `settings_get_low_stock_threshold` returned i64, `settings_set_low_stock_threshold` accepted i64 — specta forbids bare i64
- Fix: Changed Tauri command signatures to i32, cast internally to i64 when calling build_* helpers
- Files: `crates/trackly-app/src/tauri_cmds/settings_org.rs`
- Commit: 36a95ab

**6. [Rule 1 - Bug] StatusCount TypeScript naming collision**
- Found during: Task 2 (svelte-check)
- Issue: `dto/device.rs` and `dto/reports.rs` both define `pub struct StatusCount` with different shapes (`status_id: i64` vs `status_name: String`). Specta exports both as `StatusCount`, last definition wins → DevicesPage.svelte `x.status_id` fails type check
- Fix: Renamed `StatusCount` in `dto/reports.rs` to `DashboardStatusCount`; updated `DashboardWidgetDto` fields, `dashboard_service.rs` imports and struct construction, `dto/mod.rs` re-export
- Files: `dto/reports.rs`, `dto/mod.rs`, `services/dashboard_service.rs`
- Commit: 25ecaa6

**7. [Rule 1 - Bug] migration_idempotency test hardcoded migration count**
- Found during: Task 2 (cargo test)
- Issue: Test asserted `applied_count == 25` but V026 and V027 were added in Phase 7
- Fix: Updated all three count assertions from 25 → 27; updated comment
- Files: `crates/trackly-infra/tests/migration_idempotency.rs`
- Commit: 25ecaa6

## Verification Results

- `cargo build -p trackly-app`: PASSED (no warnings with -D warnings)
- `cargo test --workspace`: PASSED (all test groups, 0 failures)
- `cargo test -p trackly-app --test export_bindings`: PASSED — bindings.ts regenerated with DashboardStatusCount
- `npx svelte-check`: 0 ERRORS, 36 WARNINGS (pre-existing, out of scope)

## Known Stubs

None — all Phase 7 commands delegate to fully-implemented service methods from plans 07-01 through 07-03.

## Threat Flags

No new threat surface introduced. HTTP handlers for settings mutations correctly enforce `authorize(&caller, &Action::ManageSettings)`. Tauri-only restrictions for `settings_move_db` (T-07-07-03) and `app_restart` (D-19) are implemented — both absent from `http/settings_org.rs` router.

## Self-Check: PASSED

- [x] `crates/trackly-app/src/tauri_cmds/reports.rs` — FOUND
- [x] `crates/trackly-app/src/tauri_cmds/dashboard.rs` — FOUND
- [x] `crates/trackly-app/src/tauri_cmds/settings_org.rs` — FOUND
- [x] `crates/trackly-app/src/http/reports.rs` — FOUND
- [x] `crates/trackly-app/src/http/dashboard.rs` — FOUND
- [x] `crates/trackly-app/src/http/settings_org.rs` — FOUND
- [x] Commit 36a95ab — FOUND
- [x] Commit 25ecaa6 — FOUND
- [x] svelte-check: 0 errors
- [x] cargo test --workspace: 0 failures

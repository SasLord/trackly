---
phase: 07-reports-dashboard-settings
plan: "08"
subsystem: backend-services
tags: [gap-closure, dashboard, template-preview, rusqlite, minijinja]
dependency_graph:
  requires: []
  provides: [GAP-D1-fixed, GAP-S6-fixed]
  affects:
    - crates/trackly-app/src/services/dashboard_service.rs
    - crates/trackly-app/src/services/template_service.rs
tech_stack:
  added: []
  patterns:
    - "SQL NULL guard: WHERE col IS NOT NULL before strftime to prevent rusqlite type error"
    - "MiniJinja UndefinedBehavior::Strict requires complete nested demo context"
key_files:
  created: []
  modified:
    - crates/trackly-app/src/services/dashboard_service.rs
    - crates/trackly-app/src/services/template_service.rs
decisions:
  - "Option A chosen for NULL guard: WHERE al.created_at_utc IS NOT NULL (cleaner SQL over type-widening)"
  - "validate_preview demo_ctx uses nested org/act/return structure matching act_handover.minijinja schema"
metrics:
  duration: "12 min"
  completed_date: "2026-06-17"
  tasks_completed: 2
  files_modified: 2
---

# Phase 07 Plan 08: Gap Closure — Dashboard Chart + Template Preview Summary

**One-liner:** NULL-guard for strftime over audit_log.created_at_utc (GAP-D1) and nested demo_ctx matching act_handover.minijinja variable schema (GAP-S6).

## Tasks Completed

| Task | Description | Commit | Files |
|------|-------------|--------|-------|
| 1 | Fix GAP-D1 — NULL guard for created_at_utc in consumption chart SQL | b1cc7e8 | dashboard_service.rs |
| 2 | Fix GAP-S6 — nested demo_ctx in validate_preview + unit test | facb6ae | template_service.rs |

## What Was Built

### Task 1: GAP-D1 — Consumption Chart NULL Guard

**Root cause:** `get_consumption_chart` queried `strftime('%Y-%m', datetime(al.created_at_utc, 'unixepoch', ...))` without guarding against NULL `created_at_utc`. When any `audit_log` row has a NULL timestamp, `strftime` returns NULL, and `r.get::<_, String>(1)?` fails with a rusqlite type error ("expected TEXT, got NULL"), causing the entire query to return `Err`, which surfaces on the Dashboard as "Ошибка загрузки".

**Fix applied (Option A — Query guard):** Added `AND al.created_at_utc IS NOT NULL` to the WHERE clause before the GROUP BY. Rows without a timestamp cannot be meaningfully bucketed by month and are safely excluded.

**Column order verified (sanity check):** SQL SELECT is `model_label` (col 0), `month_key` (col 1), `installs` (col 2); query_map maps `r.get(0)` → `model_label`, `r.get(1)` → `month_key`, `r.get(2)` → `installs` — correct, no change needed.

### Task 2: GAP-S6 — Template Preview Demo Context

**Root cause:** `validate_preview` supplied a flat demo context (`act_number`, `act_date`, `org_name`, etc.) but `act_handover.minijinja` expects nested objects (`org.name`, `act.number`, `act.items[].name`, etc.). `UndefinedBehavior::Strict` in the MiniJinja environment causes any undefined variable access to error immediately with "undefined value (in _preview:N)".

**Fix applied:** Rewrote `demo_ctx` to the nested structure matching the template's variable schema:
- `org.{name, inn, kpp, address, logo_path}`
- `act.{number, suffix, date, date_human, giver_name, receiver_name, location_name, deadline, deadline_human, parent, items[]}`
- `act.items[].{name, inventory_no, serial_no, model, quantity}` (exact field names from template)
- `return.{condition_default, location_default}`

**Unit test added:** `validate_preview_returns_pdf_bytes` in `services::template_service::tests` — constructs `TemplateService`, calls `validate_preview` with the embedded `act_handover` default body, asserts `Ok(bytes)` where `bytes.len() > 0` and `bytes` starts with `%PDF`.

## Verification Results

```
cargo test -p trackly-app --test dashboard_widgets
  test dashboard_widget_counts_match_db_state ... ok
  test dashboard_low_stock_reflects_cartridge_state ... ok
  test result: ok. 2 passed; 0 failed

cargo test -p trackly-app --lib -- validate_preview
  test services::template_service::tests::validate_preview_returns_pdf_bytes ... ok
  test result: ok. 1 passed; 0 failed

cargo clippy -p trackly-app -- -D warnings
  Finished (no warnings, no errors)

grep -c "org.*name\|act.*number\|items" template_service.rs → 8 (>= 3)
```

## Deviations from Plan

None — plan executed exactly as written. Option A (SQL NULL guard) was chosen as the plan preferred.

## Known Stubs

None.

## Threat Flags

None — no new network endpoints, auth paths, file access patterns, or schema changes introduced.

## Self-Check: PASSED

- [x] `crates/trackly-app/src/services/dashboard_service.rs` — modified (NULL guard added)
- [x] `crates/trackly-app/src/services/template_service.rs` — modified (demo_ctx nested + unit test)
- [x] Commit b1cc7e8 exists (Task 1)
- [x] Commit facb6ae exists (Task 2)
- [x] GAP-D1: get_consumption_chart returns Ok([]) for empty DB (dashboard_widgets test)
- [x] GAP-S6: validate_preview with default act_handover returns PDF bytes > 0 (unit test)

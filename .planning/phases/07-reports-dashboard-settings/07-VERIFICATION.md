---
status: gaps_found
phase: 07-reports-dashboard-settings
verified_by: human-verify
verifier: user (cargo tauri dev, macOS desktop)
created: 2026-06-17
updated: 2026-06-17
must_haves_total: 31
must_haves_verified: 19
gaps_open: 12
source: [07-07-PLAN human-verify checkpoint]
---

## Summary

Automated gates all passed (cargo test --workspace, export_bindings, svelte-check 0 errors).
Human verification via `cargo tauri dev` on macOS desktop surfaced **12 functional/UX gaps**
across Dashboard, Reports, and Settings. Backend builds and unit tests are green, but several
runtime command paths error and several UI layouts do not meet the UI-SPEC intent.

Migrations V026/V027 applied cleanly; `migrate_from_org_json` ran successfully; supervisor
log_retention task fired on startup — so the wire-up itself is sound. The gaps are in
individual feature behaviors and presentation.

## Gaps

### GAP-D1 — Dashboard consumption chart fails to load
status: failed
area: dashboard
severity: high
type: backend
symptom: Widget "Динамика расхода картриджей" shows "Ошибка загрузки".
expected: ConsumptionPoint series renders in ChartWidget (DASH consumption chart, action='custom:install').
suspect: `dashboard_get_consumption_chart` command/handler erroring at runtime (query or DTO mismatch). Check console/network error and the SQL in DashboardService::get_consumption_chart.

### GAP-R1 — Reports export (CSV / PDF / Print) all fail
status: failed
area: reports
severity: high
type: backend
symptom: "Ошибка при экспорте CSV. Попробуйте ещё раз", "Ошибка при создании PDF. Попробуйте ещё раз" (both Export PDF and Print).
expected: CSV downloads (UTF-8 BOM, ';' delimiter); PDF generates; Print opens/saves PDF.
suspect: report export Tauri commands (`reports_export_csv` / `reports_export_pdf`) erroring — likely arg/DTO shape mismatch with the UI call, or PDF DocSpec build failing. Verify command names + payloads against bindings.ts and ReportService::export_csv/export_pdf.

### GAP-R2 — Reports switch-bars should share one row
status: failed
area: reports
severity: medium
type: frontend
symptom: "Устройства / Картриджи" sub-nav and the Status sub-nav are stacked on separate rows.
expected: On desktop, both switch-bars on the SAME row — Status switch-bar (Акты/Возвраты/В работе/На складе; Расход/История заправок/В работе/На складе) left-aligned, "Устройства/Картриджи" switch-bar right-aligned (justified to opposite sides). Stack only on mobile/narrow.

### GAP-R3 — Range date inputs broken styling
status: failed
area: reports
severity: medium
type: frontend
symptom: When report period = "Диапазон", the "С" and "По" date inputs have huge margins and break out of the visual style (see screenshot).
expected: Date inputs sized/spaced consistently with the rest of the filter row, no oversized padding.

### GAP-R4 — Remove redundant report filters; move period-selector into filter row
status: failed
area: reports
severity: medium
type: frontend
symptom: Filter row carries redundant controls.
expected:
- Devices reports: remove Локация, Тип, Статус, "Поиск в отчёте" filters.
- Cartridges reports: remove Модель, Статус, Цвет, "Поиск в отчёте" filters.
- In the (now-cleared) filters row, place the period-selector on the LEFT.

### GAP-R5 — Status switch-bar counters only show for selected status
status: needs-decision
area: reports
severity: low
type: frontend
symptom: Count badges appear only on the currently-selected status tab; others hidden. User asked whether this is intentional.
expected (recommended): show count badges for ALL status tabs simultaneously so users see distribution without clicking. Confirm during gap planning.

### GAP-S1 — Settings section blocks have no spacing
status: failed
area: settings
severity: medium
type: frontend
symptom: New settings blocks (Org/Storage/Backup/Threshold/Template) are stuck together with no vertical gap.
expected: consistent vertical spacing between section cards per UI-SPEC.

### GAP-S2 — Settings needs a sub-section switch-bar
status: failed
area: settings
severity: medium
type: frontend
symptom: Settings is one very long scrolling page.
expected: add a sub-nav switch-bar at the top of Settings to split it into subsections (avoid one long page).

### GAP-S3 — Current DB path stuck on "Загрузка…"; "Сменить расположение" non-functional
status: failed
area: settings
severity: high
type: backend
symptom: "Текущее расположение базы данных: Загрузка…" never resolves; changing DB location does nothing.
expected: current DB path loads and displays; "Сменить расположение" opens Tauri save dialog and applies (Tauri-only). Check the command that returns the DB path and its binding.

### GAP-S4 — Auto-backup folder picker fails in desktop app
status: failed
area: settings
severity: high
type: frontend
symptom: Selecting auto-backup folder errors "Выбор папки доступен только в десктоп-приложении" — even inside the Tauri desktop app.
expected: in desktop, folder picker opens (Tauri dialog). The desktop-vs-browser detection is wrong — it treats the desktop app as a browser. Fix the runtime-environment check.

### GAP-S5 — Low-stock threshold not loaded on reopen; input styling wrong
status: failed
area: settings
severity: medium
type: both
symptom: "Порог низкого остатка" saves and persists, but the field is EMPTY when Settings is reopened (value not loaded back). Also the number input is mis-styled (spinner arrows should sit at the edge; large padding from edge).
expected: saved threshold value loads into the field on mount; number input styled per design (spinners at edge, correct padding).

### GAP-S6 — Template preview render error
status: failed
area: settings
severity: high
type: backend
symptom: Preview fails: "Шаблон содержит ошибки: validation [template]: Template render error: undefined value (in _preview:15)".
expected: template preview renders the default act template. The preview render context is missing a variable used at line 15 of the preview wrapper (`_preview`). Supply the missing context variable(s) or guard undefined access in the preview render path.

## Verified (working)

- Migrations V026 (org_settings) + V027 (document_templates is_default) apply cleanly.
- `migrate_from_org_json` migrates org.json → DB on first run.
- Supervisor activates on startup (log_retention task ran).
- Reports cartridge "Расход" tab loads rows (Pantum TL-5120x / C-000004 visible) — read path works.
- Settings Org/Backup/Threshold forms render and (for threshold) save.
- Tauri commands + axum routes registered; `settings_move_db` correctly Tauri-only (no HTTP route).

## Routing

12 gaps → gap-closure planning. Backend gaps (D1, R1, S3, S4, S6) are highest priority
(broken features); frontend gaps (R2, R3, R4, S1, S2, S5) are layout/UX; R5 is a decision.

Next: `/gsd-plan-phase 07 --gaps` → creates gap_closure plans → `/gsd-execute-phase 07 --gaps-only`.

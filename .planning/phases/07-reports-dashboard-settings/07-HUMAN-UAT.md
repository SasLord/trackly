---
status: passed
phase: 07-reports-dashboard-settings
source: [07-VERIFICATION.md]
started: 2026-06-17
updated: 2026-06-17
---

## Current Test

[awaiting human testing — run `cargo tauri dev` on desktop]

## Tests

All 12 gap fixes are code-verified and automated gates pass (cargo build, cargo test, svelte-check).
These behaviors were originally found at runtime and need confirmation in the running desktop app.

### 1. Dashboard consumption chart loads (GAP-D1)
expected: Widget "Динамика расхода картриджей" renders the series — no "Ошибка загрузки".
result: [pending]

### 2. Reports CSV export downloads (GAP-R1)
expected: CSV export downloads a UTF-8 BOM, ';'-delimited file — no error toast.
result: [pending]

### 3. Reports PDF export + Print (GAP-R1)
expected: Export PDF generates a file; Print opens/saves a PDF — no "Ошибка при создании PDF".
result: [pending]

### 4. Reports switch-bars share one row (GAP-R2)
expected: On desktop, Status switch-bar left-aligned and "Устройства/Картриджи" switch-bar right-aligned on the SAME row; stacks only on narrow/mobile.
result: [pending]

### 5. Reports range date inputs styled correctly (GAP-R3)
expected: With period = "Диапазон", "С"/"По" date inputs match the height/spacing of other filter controls — no oversized padding.
result: [pending]

### 6. Reports filter row cleaned up (GAP-R4)
expected: Devices/Cartridges reports no longer show redundant filters (Локация/Тип/Статус/Модель/Цвет/"Поиск в отчёте"); period-selector sits on the LEFT with export buttons on the right.
result: [pending]

### 7. Reports badges on all tabs (GAP-R5)
expected: All report tabs show a badge simultaneously (active tab shows the real count; inactive show "–").
result: [pending]

### 8. Settings DB path loads (GAP-S3)
expected: "Текущее расположение базы данных" shows the real path, not a stuck "Загрузка…".
result: [pending]

### 9. Settings "Сменить расположение" works (GAP-S3)
expected: Button opens the native Tauri save dialog (Tauri desktop only).
result: [pending]

### 10. Settings auto-backup folder picker works (GAP-S4)
expected: Inside the desktop app, "Выбрать папку" opens the native folder picker — no false "только в десктоп-приложении" error.
result: [pending]

### 11. Settings threshold reloads after save (GAP-S5)
expected: Save "Порог низкого остатка", navigate away, reopen Settings — the saved value is loaded into the field (not empty). Number input spinner arrows sit at the edge.
result: [pending]

### 12. Template preview renders (GAP-S6)
expected: Template preview renders the default act template — no "Template render error: undefined value".
result: [pending]

### 13. Settings sub-nav works (GAP-S1/S2)
expected: Settings shows a sub-section switch-bar at the top; section cards have visible vertical spacing; tab-switching shows only the active subsection.
result: [pending]

## Summary

total: 13
passed: 0
issues: 0
pending: 13
skipped: 0
blocked: 0

## Gaps

## Follow-ups (out of scope of the 12 gaps — found during this run)

- **OrgSettings.svelte:105** still uses the Tauri 1 `window.__TAURI__` check for the logo file picker — same bug class as GAP-S3/S4. Logo upload via picker will silently fail in the desktop app. Track for next gap-closure pass.
- **Code review (07-REVIEW.md):** CR-02 — `template_service.rs` `update_body()`/`reset_to_default()` swallow `rows_affected`, returning `Ok(())` on a no-op update (pre-existing, from 07-02). CR-01 — `get_consumption_chart()` hardcodes `+3 hours` instead of reading `config.organization.timezone` (consistent with RU-only fixed UTC+3 for v1, but inconsistent with `get_all_widgets`).

---

## Round 2 re-test (after gap plans 07-12..07-14) — run `cargo tauri dev`

Automated gates for the changed code passed: cargo build ✓, targeted cargo test ✓ (lib 68 + dashboard/reports/org/backup/template/bindings suites, 0 failed), pnpm svelte-check ✓ (0 errors). The following were runtime failures last round — re-confirm in the desktop app:

### R2-1. Организация: logo re-add (G2-1)
expected: Picking a PNG/JPG/SVG adds the logo (detection now uses __TAURI_INTERNALS__; fs:allow-read-file granted) — no "Не удалось связаться с приложением".
result: [passed]

### R2-2. Хранилище: "Открыть папку с базой данных" (G2-2)
expected: Opens the DB's containing folder in the OS file manager (new settings_open_db_folder command, no path arg).
result: [passed]

### R2-3. Хранилище: "Сменить расположение" (G2-2)
expected: Opens the native save dialog; moving the DB then restarts. Confirm end-to-end.
result: [passed]

### R2-4. Бэкапы: folder picker (G2-3)
expected: Picking a backup folder persists it (args now wrapped as { patch: { backup_folder } }) — no error.
result: [passed]

### R2-5. Шаблоны: "Проверить (превью PDF)" (G2-4)
expected: Preview renders the act template to a PDF in the iframe — no "undefined value" / no instant button revert.
result: [passed]

### R2-6. Отчёты: export block alignment + real status counts (G2-5)
expected: Export/print buttons block is flush-right and vertically aligned with the period selector; every status tab shows a REAL numeric count simultaneously (not "–").
result: [passed]

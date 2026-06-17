---
phase: 07-reports-dashboard-settings
verified: 2026-06-17T15:00:00Z
status: human_needed
score: 12/12 gap-closure must-haves verified (code), 12 require runtime confirmation
re_verification:
  previous_status: gaps_found
  previous_score: 19/31
  gaps_closed:
    - "GAP-D1: NULL guard added to get_consumption_chart SQL — unit tests pass"
    - "GAP-S6: validate_preview demo_ctx uses nested org/act schema — unit test passes (PDF bytes > 0)"
    - "GAP-S3: StorageSettings uses apiCall<string> + __TAURI_INTERNALS__ detection"
    - "GAP-S4: BackupSettings uses __TAURI_INTERNALS__ detection"
    - "GAP-S5: ThresholdSettings uses apiCall<number> on mount; padding-right:2px + appearance:auto"
    - "GAP-R1: reportTypeKey() helper added; all 3 export functions pass correct backend key"
    - "GAP-R2: ReportSubNav flex-direction:row with domain-nav left, report-nav right"
    - "GAP-R3: PeriodSelector .period-range align-items:center + :global(.date-picker) height override"
    - "GAP-R4: ReportFilters strips all redundant filter controls; PeriodSelector moved into controls-row"
    - "GAP-R5: Badge renders on all tabs (active: accent+count, inactive: default+'–')"
    - "GAP-S1: .settings-content has display:flex flex-direction:column gap:var(--space-lg)"
    - "GAP-S2: SettingsSubNav.svelte created with 6 tabs; SettingsPage shows only active subsection"
  gaps_remaining: []
  regressions: []
human_verification:
  - test: "Dashboard consumption chart loads without 'Ошибка загрузки'"
    expected: "ChartWidget for 'Динамика расхода картриджей' renders a chart or empty-state message, not an error banner"
    why_human: "NULL guard is verified in code and unit tests pass, but runtime DB state and audit_log content in the running app cannot be inspected statically"
  - test: "Reports CSV export downloads a file"
    expected: "Clicking 'Экспорт CSV' triggers a file-save dialog (Tauri) or download (browser); no 'Ошибка при экспорте' toast"
    why_human: "reportTypeKey() is wired correctly in code, but the actual Tauri command invocation path and CSV generation require a running app to confirm end-to-end"
  - test: "Reports PDF export and Print work"
    expected: "Clicking 'Экспорт PDF' opens save dialog and produces a PDF file; 'Печать' opens the PDF in the system viewer"
    why_human: "Same as CSV — backend PDF generation chain cannot be confirmed without running the app"
  - test: "Reports sub-nav shows both switch-bars on one row (desktop)"
    expected: "On a wide viewport, 'Устройства/Картриджи' appears left-aligned and the report-type tabs appear right-aligned on the same visual row"
    why_human: "flex-direction:row is confirmed in code, but actual rendering depends on viewport width and browser layout engine inside WebView2/WKWebView"
  - test: "Reports date range inputs are same height as other controls"
    expected: "When Period = 'Диапазон', the 'С' and 'По' date inputs are ~28px tall and visually aligned with the export buttons"
    why_human: ":global(.date-picker) override confirmed in code; actual computed height depends on browser's DatePicker rendering"
  - test: "Reports filter row shows only PeriodSelector + export buttons (no Локация/Тип/Статус/Модель/Цвет/Поиск)"
    expected: "The controls row contains only the period selector on the left and export buttons on the right"
    why_human: "Filter removal confirmed in code; visual confirmation required"
  - test: "Reports tabs all show badge indicators simultaneously"
    expected: "All status tabs show a badge: active tab shows numeric count, inactive tabs show '–'"
    why_human: "Badge logic confirmed in code; visual confirmation required that badges render for all tabs at once"
  - test: "Settings DB path loads and displays (not stuck on 'Загрузка…')"
    expected: "StorageSettings shows the actual DB file path string on mount"
    why_human: "apiCall<string> fix confirmed; runtime Tauri command response cannot be confirmed without running the app"
  - test: "Settings 'Сменить расположение' opens Tauri save dialog"
    expected: "Clicking the button inside the desktop app opens a native save file dialog (not an error toast)"
    why_human: "__TAURI_INTERNALS__ detection fix confirmed; actual dialog open requires running the desktop app"
  - test: "Settings backup folder picker opens (no false 'только в десктоп-приложении' error)"
    expected: "Clicking 'Выбрать папку' inside the desktop app opens the native folder picker dialog"
    why_human: "__TAURI_INTERNALS__ detection fix confirmed; actual behavior requires running the desktop app"
  - test: "Settings threshold value loads on reopen"
    expected: "After saving a threshold value and navigating away + back, the field shows the saved number (not blank/0)"
    why_human: "apiCall<number> fix confirmed; runtime state persistence requires running the app"
  - test: "Template preview renders without error"
    expected: "Clicking Preview in TemplateEditor returns a rendered PDF preview; no 'undefined value' error toast"
    why_human: "Unit test validate_preview_returns_pdf_bytes passes with demo_ctx; runtime invocation from TemplateEditor UI requires running the app"
  - test: "Settings sub-nav switch-bar is visible and navigation works"
    expected: "Clicking 'Организация' shows only OrgSettings card; clicking 'Шаблоны' shows only TemplateEditor; gap is visible between sub-nav and card"
    why_human: "Component structure and CSS confirmed; visual rendering requires running the app"
---

# Phase 07: Reports/Dashboard/Settings — Re-Verification Report (Gap Closure)

**Phase Goal:** Поставить отчётный слой (Устройства, Картриджи), виджеты дашборда и раздел Настройки (организация, логотип, порог низкого остатка, путь БД, бэкапы, редактирование шаблонов документов).
**Verified:** 2026-06-17T15:00:00Z
**Status:** human_needed
**Re-verification:** Yes — after gap closure via plans 07-08..07-11

## Summary

This is the re-verification of 12 gaps recorded in the prior human-verify pass. Plans 07-08 through 07-11 executed fixes across Rust services and Svelte components. All 12 fixes are **present and substantively correct in code**. Automated gates confirmed:

- `cargo build --workspace` — 0 errors (0.26s, already compiled)
- `cargo test -p trackly-app --test dashboard_widgets` — 2/2 PASS
- `cargo test -p trackly-app --lib -- validate_preview` — 1/1 PASS (PDF bytes > 0)
- `cargo test --workspace` — all suites pass
- All 7 gap-closure commits verified in git history (b1cc7e8, facb6ae, ee01045, 8910f19, 99d24df, 8ef2287, 712356d)

Because the original 12 gaps were discovered at runtime via `cargo tauri dev` (UI layout, dialog open/close, "Ошибка загрузки" disappearing), code inspection alone cannot close them. All 12 require **human runtime confirmation** — which is the expected outcome given the problem statement.

---

## Gap-Closure Verification

### GAP-D1 — Dashboard consumption chart NULL guard

| Check | Evidence | Status |
|-------|----------|--------|
| `AND al.created_at_utc IS NOT NULL` in SQL | Line 306 in `dashboard_service.rs` | VERIFIED |
| `dashboard_widgets` integration tests pass | `2 passed; 0 failed` | VERIFIED |
| Runtime: widget loads without error | Requires `cargo tauri dev` | NEEDS HUMAN |

**Code status: VERIFIED.** Runtime: NEEDS HUMAN.

---

### GAP-S6 — Template preview nested demo_ctx

| Check | Evidence | Status |
|-------|----------|--------|
| `demo_ctx` is nested JSON with `org`, `act`, `return` keys | Lines 247-289 in `template_service.rs` | VERIFIED |
| `validate_preview_returns_pdf_bytes` unit test passes | `1 passed; 0 failed`; bytes start with `%PDF` | VERIFIED |
| Runtime: TemplateEditor preview renders without "undefined value" error | Requires `cargo tauri dev` | NEEDS HUMAN |

**Code status: VERIFIED.** Runtime: NEEDS HUMAN.

---

### GAP-S3 — StorageSettings DB path + move detection

| Check | Evidence | Status |
|-------|----------|--------|
| `apiCall<string>('settings_get_db_path', {})` direct assignment | Line 15 in `StorageSettings.svelte` | VERIFIED |
| `__TAURI_INTERNALS__` in window detection in `proceedWithMove` | Line 42 in `StorageSettings.svelte` | VERIFIED |
| `__TAURI__` (old Tauri 1 check) removed | `grep -c '__TAURI__'` = 0 | VERIFIED |
| Runtime: DB path loads; move dialog opens | Requires `cargo tauri dev` | NEEDS HUMAN |

**Code status: VERIFIED.** Runtime: NEEDS HUMAN.

---

### GAP-S4 — BackupSettings folder picker detection

| Check | Evidence | Status |
|-------|----------|--------|
| `__TAURI_INTERNALS__` in window in `pickFolder` | Line 64 in `BackupSettings.svelte` | VERIFIED |
| `__TAURI__` removed | `grep -c '__TAURI__'` = 0 | VERIFIED |
| Runtime: folder picker opens in desktop app | Requires `cargo tauri dev` | NEEDS HUMAN |

**Code status: VERIFIED.** Runtime: NEEDS HUMAN.

Note: `OrgSettings.svelte` still uses the old `__TAURI__` check at line 105 (logo file picker). This was **not one of the 12 declared gaps** and was not in scope for plans 07-08..07-11. It is a separate defect — logged below as a new warning.

---

### GAP-S5 — ThresholdSettings load + spinner styling

| Check | Evidence | Status |
|-------|----------|--------|
| `apiCall<number>('settings_get_low_stock_threshold', {})` direct assignment | Line 10 in `ThresholdSettings.svelte` | VERIFIED |
| `result.threshold` destructuring removed | `grep` = 0 matches | VERIFIED |
| `padding: var(--space-xs) 2px var(--space-xs) var(--space-sm)` + `appearance: auto` | Lines 95, 102 in `ThresholdSettings.svelte` | VERIFIED |
| Runtime: saved threshold appears on reopen; spinner at edge | Requires `cargo tauri dev` | NEEDS HUMAN |

**Code status: VERIFIED.** Runtime: NEEDS HUMAN.

---

### GAP-R1 — Export passes correct report_type key

| Check | Evidence | Status |
|-------|----------|--------|
| `reportTypeKey()` function defined | Line 230 in `ReportsPage.svelte` | VERIFIED |
| All 3 export functions (`exportCsv`, `exportPdf`, `printReport`) call `reportTypeKey()` | Lines 301, 325, 381 in `ReportsPage.svelte` (4 total occurrences) | VERIFIED |
| `currentCmd()` no longer used in export calls | `currentCmd()` still present at line 213 (for loadReport only) | VERIFIED |
| Runtime: CSV/PDF/Print work without error toast | Requires `cargo tauri dev` | NEEDS HUMAN |

**Code status: VERIFIED.** Runtime: NEEDS HUMAN.

---

### GAP-R2 — Switch-bars on one row

| Check | Evidence | Status |
|-------|----------|--------|
| `flex-direction: row` on `.report-sub-nav` | Line 111 in `ReportSubNav.svelte` | VERIFIED |
| `.domain-nav` flex-shrink:0 (left), `.report-nav` flex:1 justify-content:flex-end (right) | Lines 119, 127 in `ReportSubNav.svelte` | VERIFIED |
| Runtime: both switch-bars share one row on desktop | Requires `cargo tauri dev` visual check | NEEDS HUMAN |

**Code status: VERIFIED.** Runtime: NEEDS HUMAN.

---

### GAP-R3 — Date range input styling

| Check | Evidence | Status |
|-------|----------|--------|
| `.period-range { align-items: center }` | Line 240 in `PeriodSelector.svelte` | VERIFIED |
| `:global(.date-picker)` height override inside `.range-label` | Lines 268+ in `PeriodSelector.svelte` | VERIFIED |
| Runtime: inputs same height as other controls | Requires visual check | NEEDS HUMAN |

**Code status: VERIFIED.** Runtime: NEEDS HUMAN.

---

### GAP-R4 — Remove redundant filters; period in filter row

| Check | Evidence | Status |
|-------|----------|--------|
| `Локация`, `Модель`, `Поиск`, `search-wrap`, `Статус`, `Тип`, `Цвет` removed from rendered output | `grep` on `ReportFilters.svelte` = 0 matches | VERIFIED |
| `PeriodSelector` placed inside `.controls-row` alongside `ReportFilters` | Lines 455-469 in `ReportsPage.svelte` | VERIFIED |
| `.controls-row` CSS defined | Line 530 in `ReportsPage.svelte` | VERIFIED |
| Runtime: filter row shows only period selector + export buttons | Requires visual check | NEEDS HUMAN |

**Code status: VERIFIED.** Runtime: NEEDS HUMAN.

---

### GAP-R5 — Badges on all tabs

| Check | Evidence | Status |
|-------|----------|--------|
| `Badge` component imported and used 3 times (import + active + inactive) | `grep -c 'Badge'` = 3 in `ReportSubNav.svelte` | VERIFIED |
| Active tab: `variant="accent"` with `{rowCount}` | Line 98 in `ReportSubNav.svelte` | VERIFIED |
| Inactive tabs: `variant="default"` with `–` | Line 100 in `ReportSubNav.svelte` | VERIFIED |
| Runtime: all tabs simultaneously show badge slot | Requires visual check | NEEDS HUMAN |

**Code status: VERIFIED.** Runtime: NEEDS HUMAN.

---

### GAP-S1 — Settings section card gap

| Check | Evidence | Status |
|-------|----------|--------|
| `display: flex; flex-direction: column; gap: var(--space-lg)` on `.settings-content` | Lines 48, 70, 71 in `SettingsPage.svelte` | VERIFIED |
| Runtime: visible vertical gap between sub-nav and active card | Requires visual check | NEEDS HUMAN |

**Code status: VERIFIED.** Runtime: NEEDS HUMAN.

---

### GAP-S2 — Settings sub-section switch-bar

| Check | Evidence | Status |
|-------|----------|--------|
| `SettingsSubNav.svelte` exists | `ls` confirmed | VERIFIED |
| 6 tabs: Сеть, Организация, Хранилище, Бэкапы, Порог остатка, Шаблоны | Lines 5-12 in `SettingsSubNav.svelte` | VERIFIED |
| `role="tablist"` + `role="tab"` + `aria-selected` ARIA attributes | Line 22, 30 in `SettingsSubNav.svelte` | VERIFIED |
| `onSectionChange` prop wired | Line 16, 30 in `SettingsSubNav.svelte` | VERIFIED |
| `SettingsSubNav` imported and used in `SettingsPage.svelte` | `grep -c 'SettingsSubNav'` = 2 | VERIFIED |
| `activeSection = $state('network')` drives conditional rendering | 8 occurrences of `activeSection` in `SettingsPage.svelte` | VERIFIED |
| Runtime: clicking tabs switches visible section | Requires `cargo tauri dev` | NEEDS HUMAN |

**Code status: VERIFIED.** Runtime: NEEDS HUMAN.

---

## Observable Truths (Original Phase Must-Haves)

The original phase had 31 must-haves, of which 19 were VERIFIED in the prior pass. The 12 gap-closure fixes address the remaining 12. All 12 are now code-verified; runtime verification is deferred to human re-check.

| # | Truth | Prior Status | Code Status | Runtime |
|---|-------|-------------|-------------|---------|
| D1 | Consumption chart returns data/empty-state (no error) | FAILED | VERIFIED | NEEDS HUMAN |
| R1 | CSV/PDF/Print export works (correct report_type) | FAILED | VERIFIED | NEEDS HUMAN |
| R2 | Sub-navs share one desktop row | FAILED | VERIFIED | NEEDS HUMAN |
| R3 | Date range inputs same height as other controls | FAILED | VERIFIED | NEEDS HUMAN |
| R4 | Filter row: period selector + export only | FAILED | VERIFIED | NEEDS HUMAN |
| R5 | Badges visible on all tabs simultaneously | needs-decision | VERIFIED | NEEDS HUMAN |
| S1 | Settings section cards have vertical gap | FAILED | VERIFIED | NEEDS HUMAN |
| S2 | Settings has sub-section switch-bar (no endless scroll) | FAILED | VERIFIED | NEEDS HUMAN |
| S3 | DB path loads (not Загрузка…); move opens dialog | FAILED | VERIFIED | NEEDS HUMAN |
| S4 | Backup folder picker works in desktop (no false error) | FAILED | VERIFIED | NEEDS HUMAN |
| S5 | Threshold loads on reopen; spinner at edge | FAILED | VERIFIED | NEEDS HUMAN |
| S6 | Template preview renders without "undefined value" | FAILED | VERIFIED | NEEDS HUMAN |

**Code score: 12/12 gap items code-verified.**

---

## New Finding — OrgSettings.svelte Logo Picker (Out of Scope Warning)

`OrgSettings.svelte` line 105 still uses `window.__TAURI__` (Tauri 1 API) for the logo image file picker. This was not in the original 12 gaps and was not addressed in plans 07-08..07-11. It will cause the same symptom as GAP-S3/S4: the logo file picker silently fails inside the desktop app. This is a new gap to track.

Severity: medium — logo upload is non-critical path (no impact on core device/cartridge tracking), but the picker is non-functional in the desktop app.

---

## Human Verification Required

All 12 items below must be tested via `cargo tauri dev` on macOS (or Windows desktop) by opening the running app. Automated code inspection is exhausted.

### 1. Dashboard consumption chart

**Test:** Open the app, navigate to Dashboard, observe the "Динамика расхода картриджей" widget.
**Expected:** Widget shows a chart (if install events exist) or an empty-state placeholder — no red "Ошибка загрузки" banner.
**Why human:** Runtime DB state and audit_log rows vary; unit test covers empty DB case only.

### 2. Reports CSV export

**Test:** Navigate to Reports, select any report tab, click "Экспорт CSV".
**Expected:** A file save dialog appears (Tauri) or download begins (browser); no "Ошибка при экспорте CSV" toast.
**Why human:** Tauri IPC dispatch and CSV backend path require a running app.

### 3. Reports PDF export and Print

**Test:** Navigate to Reports, click "Экспорт PDF" and then "Печать".
**Expected:** PDF save dialog appears; Print opens a PDF file in the system viewer.
**Why human:** PDF generation pipeline requires running the app.

### 4. Reports sub-nav single row (desktop)

**Test:** Open Reports on a wide viewport (full desktop window). Observe the navigation area above the report table.
**Expected:** "Устройства" / "Картриджи" tabs appear on the LEFT and the report-type tabs (Акты / Возвраты / В работе / На складе) appear on the RIGHT of the same horizontal row — not stacked vertically.
**Why human:** CSS rendering depends on actual viewport inside WebView2/WKWebView.

### 5. Reports date range inputs

**Test:** In Reports, set Period to "Диапазон". Observe the "С" and "По" date inputs.
**Expected:** The date inputs are approximately the same height (28px) as the export buttons next to them; no oversized vertical spacing.
**Why human:** `:global(.date-picker)` override height depends on actual DatePicker component rendering.

### 6. Reports filter row clean

**Test:** In Reports, observe the filter/controls row (the row between the sub-nav and the report table).
**Expected:** Only a period selector (left) and export buttons (right). No "Локация", "Тип", "Статус", "Модель", "Цвет", or "Поиск в отчёте" filter controls visible.
**Why human:** Visual confirmation required.

### 7. Reports badges on all tabs

**Test:** Navigate to Reports (any tab active). Observe all report-type tab buttons.
**Expected:** Every tab shows a small badge indicator — the active tab shows a numeric count; inactive tabs show "–". All badge slots visible simultaneously without switching tabs.
**Why human:** Visual confirmation required.

### 8. Settings DB path load

**Test:** Open Settings, navigate to "Хранилище" subsection (after the sub-nav is visible).
**Expected:** "Текущее расположение базы данных:" shows the actual file path string (e.g., `/path/to/trackly.db`), not "Загрузка…".
**Why human:** Tauri `settings_get_db_path` command response requires a running app.

### 9. Settings move DB dialog

**Test:** In Settings > Хранилище, click "Сменить расположение".
**Expected:** A native save-file dialog opens (no toast error).
**Why human:** __TAURI_INTERNALS__ detection requires the actual Tauri webview context.

### 10. Settings backup folder picker

**Test:** In Settings > Бэкапы, click "Выбрать папку".
**Expected:** A native folder picker dialog opens. No "Выбор папки доступен только в десктоп-приложении." toast.
**Why human:** __TAURI_INTERNALS__ detection requires the actual Tauri webview context.

### 11. Settings threshold loads on reopen

**Test:** In Settings > Порог остатка, set a threshold value (e.g., 5) and save. Navigate to a different section, then back to "Порог остатка".
**Expected:** The input field shows "5" (or the saved value), not blank or 0.
**Why human:** Runtime mount lifecycle and Tauri command result require a running app.

### 12. Template preview renders

**Test:** In Settings > Шаблоны, click the Preview button on the default act_handover template.
**Expected:** A PDF preview renders (either inline or opens in system viewer). No "Шаблон содержит ошибки: undefined value" error.
**Why human:** Unit test covers the demo_ctx path; TemplateEditor UI invocation path requires running the app.

### 13. Settings sub-nav switch-bar visible and functional

**Test:** Open Settings. Observe the top of the settings area.
**Expected:** A switch-bar with tabs "Сеть / Организация / Хранилище / Бэкапы / Порог остатка / Шаблоны" is visible. Clicking each tab switches the visible content. The previously-endless scroll is replaced by single-section views.
**Why human:** Component rendering and click handlers require visual confirmation in the running app.

---

## Anti-Patterns

| File | Line | Pattern | Severity | Note |
|------|------|---------|----------|------|
| `OrgSettings.svelte` | 105 | `window.__TAURI__` (Tauri 1 check) | Warning | Out of scope for this gap-closure run; logo file picker will fail silently in desktop app |

No TBD / FIXME / XXX markers found in the gap-closure files.

---

## Requirements Coverage

All requirement IDs declared for this phase (RPT-01..RPT-08, DASH-01..DASH-05, SET-01..SET-09) were addressed in plans 07-01..07-11. The gap-closure plans specifically close:

| Requirement | Addressed By | Status |
|-------------|-------------|--------|
| DASH-03 | Plan 07-08 (GAP-D1 NULL guard) | Code VERIFIED |
| SET-09 | Plan 07-08 (GAP-S6 template preview) | Code VERIFIED |
| SET-03, SET-04, SET-05, SET-07 | Plan 07-09 (GAP-S3/S4/S5) | Code VERIFIED |
| RPT-07, RPT-08, RPT-04 | Plan 07-10 (GAP-R1..R5) | Code VERIFIED |
| SET-01..SET-07 | Plan 07-11 (GAP-S1/S2) | Code VERIFIED |

---

_Verified: 2026-06-17T15:00:00Z_
_Verifier: Claude (gsd-verifier) — re-verification after gap closure (plans 07-08..07-11)_

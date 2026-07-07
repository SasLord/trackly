---
phase: 17-html-krilla
verified: 2026-07-07T12:23:46Z
status: gaps_found
score: 7/10 must-haves verified
overrides_applied: 0
gaps:
  - truth: "Report table headers show Russian column labels (D-03) supplied by Rust as data, not hardcoded in the template"
    status: failed
    reason: >
      columns_for(report_type) in tauri_cmds/reports.rs returns raw snake_case
      column KEYS ("number", "device_name", "giver_name", "location_name", ...).
      These keys are used both to pull cell values via row_field(row, col)
      (correct) AND passed verbatim into the MiniJinja context as "columns",
      which report.html renders literally as <th>{{ col }}</th>. Every exported
      or printed report therefore shows English snake_case headers
      ("number | device_name | giver_name | location_name") instead of Russian
      labels ("Номер | Устройство | Сдал | Локация"). The ReportsPage.svelte
      UI's own COLUMNS_MAP carries correct Russian labels, but that mapping
      never reaches the backend export path. report.html's own doc-comment
      explicitly (and incorrectly) asserts columns are "Rust-supplied Russian
      column-label strings" — the code does not match this contract. This
      directly violates 17-01-PLAN.md's must-have truth D-03 ("русские подписи
      колонок передаёт Rust как данные... шаблон итерирует их, не хардкодит")
      and is a functional regression against the project's Russian-only v1 UI
      constraint (CLAUDE.md). Confirmed independently by reading
      tauri_cmds/reports.rs, report_service.rs, and templates/report.html —
      not merely inferred from 17-REVIEW.md's CR-01 finding.
    artifacts:
      - path: "crates/trackly-app/src/tauri_cmds/reports.rs"
        issue: "columns_for() returns snake_case keys; no column_labels_for()/label-lookup exists; the same key list is passed both as row_field accessors and as the template's 'columns' header data"
      - path: "crates/trackly-app/src/services/report_service.rs"
        issue: "export_pdf's ctx builds \"columns\": columns directly from the &[&str] param with no key-to-label translation (line ~623)"
      - path: "crates/trackly-app/templates/report.html"
        issue: "thead loop renders {{ col }} verbatim as the header text (lines ~158-160), assuming (incorrectly) that Rust already supplies labels"
    missing:
      - "A column_labels_for(report_type) (or equivalent) function mapping each report_type to its Russian label list, index-aligned with columns_for()'s keys"
      - "export_pdf's ctx['columns'] built from the label list, while row_field(row, col) continues to use the key list for cell lookups"
      - "A regression test asserting a Russian header label (e.g. \"Устройство\") appears in exported HTML output — the existing html_report_render.rs suite only asserts row values and month headings, never header text, so it did not catch this"
human_verification:
  - test: "Open Отчёты, pick any report type with data, click «Экспорт PDF» (desktop/Tauri webview)"
    expected: "Preview modal opens showing HTML with a print dialog available, no 'Ошибка при создании PDF' toast (closes 16-HUMAN-UAT 2b)"
    why_human: "Requires driving the actual Tauri webview UI; cannot be confirmed by static analysis alone even though the code wiring (PdfPreviewModal mode='report', apiCall to reports_export_pdf) is verified correct"
  - test: "Connect from a LAN browser to the server-mode instance, open Отчёты, click «Экспорт PDF» / «Печать»"
    expected: "Same preview+print flow works identically to desktop (closes 16-HUMAN-UAT 2a); confirms text/html Content-Type is consumed correctly by the browser transport"
    why_human: "Requires a second machine/browser session on the LAN; SUMMARY.md for 17-03 explicitly states this manual check was not performed in-session"
  - test: "In Настройки > Шаблоны, edit the report/act_handover/act_acceptance body to introduce a typo'd undefined variable, click «Сохранить», then trigger a real export/print of that document type"
    expected: "Ideally «Сохранить» itself rejects the broken template (WR-01); at minimum, confirm whether the current behavior (save succeeds, later real render fails) is acceptable UX or needs a follow-up fix"
    why_human: "Behavioral/UX judgment call flagged by code review (WR-01) — validate_preview and update_body use different MiniJinja environments (strict vs lenient), a decision on whether to fix now or defer"
---

# Phase 17: Отчёты и Шаблоны через HTML-печать — Verification Report

**Phase Goal:** Экспорт Отчётов и редактор Шаблонов переходят с krilla/DocSpec на HTML-печать по паттерну Phase 16 (акты): `export_pdf` возвращает HTML-строку, печать/сохранение идёт через диалог браузера в превью-модалке (desktop + LAN), редактор Шаблонов правит HTML-файлы в `templates/`, и `krilla`/`DocSpec` полностью выведены из активного пути (заморожены, не удалены).
**Verified:** 2026-07-07T12:23:46Z
**Status:** gaps_found
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `ReportService::export_pdf` returns `Result<String, AppError>` HTML; no `DocSpec`/`render_docspec` in active body (Req-1) | VERIFIED | `report_service.rs` body builds `groups`/`columns` JSON ctx and calls `render_with_timeout(&build_safe_html_env(), "report_html", ...)`; `grep -n "DocSpec\|render_docspec\|HeaderBlock\|Section::" report_service.rs` matches only 2 doc-comments, zero active code |
| 2 | `templates/report.html` present in embedded defaults, materializes at startup, file-first + embedded fallback (Req-2) | VERIFIED | `html_templates.rs:39` — `("report.html", include_str!("../../templates/report.html"))` in `DEFAULT_HTML_TEMPLATES`; `materialize_defaults_on_startup`/`load_template` iterate the const generically (unmodified from Phase 16), so report.html gets the same file-first/fallback treatment as the two act templates |
| 3 | Tauri `reports_export_pdf` and HTTP `handler_export_pdf` return HTML string; HTTP `Content-Type: text/html`; `bindings.ts` regenerated (Req-3) | VERIFIED | `tauri_cmds/reports.rs` `build_reports_export_pdf`/`reports_export_pdf` both declare `Result<String, AppError>`; `http/reports.rs:218` sets `(header::CONTENT_TYPE, "text/html; charset=utf-8")`; `ui/src/bindings.ts:834` shows `reportsExportPdf(...): Promise<Result<string, AppError>>` |
| 4 | Report table headers render Russian column labels supplied by Rust as data (D-03), not raw keys | **FAILED** | `columns_for()` in `tauri_cmds/reports.rs` returns snake_case keys (`"device_name"`, `"giver_name"`, ...); these are passed straight into `ctx["columns"]` in `report_service.rs` and rendered verbatim as `<th>{{ col }}</th>` in `templates/report.html`. Confirmed by direct code read — see Gaps below (matches 17-REVIEW.md CR-01) |
| 5 | «Экспорт PDF» in Отчёты opens preview+print instead of file download, works in desktop + LAN (closes 16-HUMAN-UAT 2a/2b) (Req-4) | UNCERTAIN | Code wiring verified: `ReportsPage.svelte` `exportPdf()` sets `reportModalOpen = true`; old blob/`tauri-plugin-fs`/`printReport()` path fully removed (`grep -c "tauri-plugin-fs\|writeFile\|@tauri-apps/plugin-dialog"` = 0, `printReport` function absent); `PdfPreviewModal` has a `mode === 'report'` branch calling `apiCall<string>('reports_export_pdf', ...)`. **Not manually exercised** — 17-03-SUMMARY.md explicitly states interactive desktop/LAN print verification "was not performed in this session" |
| 6 | Редактор Шаблонов reads/writes/resets HTML files in `templates/`, not `document_templates` DB rows (Req-5) | VERIFIED (see caveat) | `list_all_for_editor`/`update_body`/`reset_to_default` in `template_service.rs` use `resolve_templates_dir`/`load_template`/`tokio::fs::write` against a fixed 3-entry `DEFAULT_HTML_TEMPLATES` allowlist; zero `document_templates` references inside these 3 methods (confirmed by direct read); frozen `seed_defaults_on_startup`/`get_active`/`DEFAULT_TEMPLATES` untouched. Caveat: `update_body`'s syntax check uses a bare lenient `minijinja::Environment::new()` while the real render path uses strict `build_safe_html_env()` — a template can save "successfully" yet fail at real render time (WR-01, warning, not scored as failing this truth since save/load/reset mechanics themselves work) |
| 7 | «Доступные переменные» panel updated per-kind (Req-5) | VERIFIED | `TemplateEditor.svelte` `VARIABLES_BY_KIND` has 3 correctly distinct entries (`act_handover`, `act_acceptance`, `report`), each matching its template's own doc-comment variable list; `currentVariables` derives from `VARIABLES_BY_KIND[selectedKind]` |
| 8 | krilla/DocSpec fully out of active Reports/Templates/health paths; krilla tests `#[ignore]`d (Req-6) | VERIFIED | `grep -rn "render_docspec\|PdfRenderer::new"` across `report_service.rs`/`template_service.rs`/`http/reports.rs`/`http/settings_org.rs`/`tauri_cmds/reports.rs`/`tauri_cmds/settings_org.rs` active bodies returns 0; the only `PdfRenderer::new()` calls left are inside `#[cfg(test)]` fixture builders (`http/health.rs`, `tauri_cmds/health.rs`) that never call `.render_docspec(...)`; `self.pdf` is never invoked in `report_service.rs`/`template_service.rs` bodies; `pdf_determinism.rs`'s 2 krilla tests carry `#[ignore]` |
| 9 | HTML render tests exist for report (1/N rows, month grouping, empty) and template file-backed editor (Req-7) | VERIFIED | `tests/html_report_render.rs` (5 tests: single-row, multi-month grouping, empty-state message, org header, no-krilla-artifact) all pass; `tests/template_edit.rs` (5 tests asserting directly against on-disk `templates/*.html`) all pass; `cargo test -p trackly-app --lib report_service` (9/9) and `--lib template_service` (11/11) pass |
| 10 | `cargo test -p trackly-app`, `clippy -D warnings`, `fmt --check` green (Req-7) | UNCERTAIN (partial) | `cargo fmt --check` and `cargo clippy -p trackly-app -- -D warnings` both ran clean. Full unfiltered `cargo test -p trackly-app` hung indefinitely (5+ min, 0% CPU) inside `devices_csv_import` integration test — a file **not touched by any Phase 17 plan** and unrelated to Reports/Templates. Targeted phase-relevant test binaries (`--lib report_service`, `--lib template_service`, `--test html_report_render`, `--test template_edit`) all pass cleanly. The full-suite hang could not be attributed to this phase's changes but was not root-caused either |

**Score:** 7/10 truths verified (1 failed, 2 uncertain)

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/trackly-app/templates/report.html` | Editable HTML report template, org header + month groups + zebra table | VERIFIED (with defect) | 177 lines, self-contained, registered in `DEFAULT_HTML_TEMPLATES`; renders header cells from raw `columns` values which are English keys, not labels (see gap) |
| `crates/trackly-app/src/pdf/html_templates.rs` | `report.html` registered as 3rd `DEFAULT_HTML_TEMPLATES` entry | VERIFIED | Line 39 |
| `crates/trackly-app/src/services/report_service.rs` | `export_pdf` HTML via `build_safe_html_env` | VERIFIED | Confirmed by direct read |
| `crates/trackly-app/src/services/template_service.rs` | Editor methods retargeted to `templates/*.html` file I/O | VERIFIED | Confirmed by direct read |
| `ui/src/features/acts/PdfPreviewModal.svelte` | `mode='report'` branch, `reportParams` prop | VERIFIED | `renderCall()` line ~103-112 |
| `ui/src/features/settings/TemplateEditor.svelte` | Per-kind variables + HTML srcdoc preview + `report` kind | VERIFIED | `KIND_LABELS.report`, `VARIABLES_BY_KIND.report`, `srcdoc={previewHtml}` all present |
| `crates/trackly-app/tests/html_report_render.rs` | HTML-render regression suite | VERIFIED | 5/5 tests pass |
| `crates/trackly-app/tests/template_edit.rs` | File-backed editor regression suite | VERIFIED | 5/5 tests pass |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `report_service.rs` | `pdf/html_templates.rs` | `load_template(&templates_dir, "report.html", embedded_default)` | WIRED | Confirmed in `export_pdf` |
| `http/reports.rs` | text/html response | `Content-Type` header | WIRED | `header::CONTENT_TYPE, "text/html; charset=utf-8"` at line 218 |
| `template_service.rs` | `pdf/html_templates.rs` | `resolve_templates_dir(&self.organization...)` | WIRED | Confirmed in `templates_dir()` helper |
| `tauri_cmds/settings_org.rs` | `template_service.rs` | `validate_preview(&kind, &body)` | WIRED | `kind` no longer discarded (`_kind` renamed) |
| `ReportsPage.svelte` | `PdfPreviewModal.svelte` | `mode="report" reportParams={{...}}` | WIRED | Confirmed in `ReportsPage.svelte` template markup |
| `TemplateEditor.svelte` | `templates_validate_preview` | `apiCall<string>('templates_validate_preview', {kind, body})` | WIRED | Confirmed |
| `tauri_cmds/reports.rs` (columns_for keys) | `report.html` `<th>` header cells | Direct pass-through, no label translation | **NOT WIRED (defective)** | Keys flow straight into header text instead of Russian labels — this is the CR-01 defect |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|---------------------|--------|
| `report.html` (`{{ cell }}` body) | `groups[].rows[]` | `report_service.rs` month-grouping loop over real `ReportRow` DB-fetched rows via `row_field` | Yes | FLOWING |
| `report.html` (`{{ col }}` header) | `columns` | `tauri_cmds/reports.rs::columns_for()` — static list, but of raw field **keys**, not Russian **labels** | Static/wrong content | ⚠️ STATIC / WRONG DATA — technically "flows" but delivers the wrong value type |
| `report.html` (`{{ org.* }}`) | `org` | `ctx.org_db.get()` DB-backed org settings | Yes | FLOWING |
| `TemplateEditor.svelte` preview | `previewHtml` | `apiCall<string>('templates_validate_preview', ...)` → `TemplateService::validate_preview` → `render_with_timeout` | Yes | FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| `export_pdf` produces HTML with month headings + row data | `cargo test -p trackly-app --lib report_service` | 9/9 passed | PASS |
| `export_pdf` empty report shows "Нет данных..." | included in above | passed | PASS |
| `export_pdf` org header renders org name | included in above | passed | PASS |
| `validate_preview` renders per-kind HTML (act_handover/act_acceptance/report) | `cargo test -p trackly-app --lib template_service` | 11/11 passed | PASS |
| File-backed editor writes/reads/resets `templates/*.html` | `cargo test -p trackly-app --test template_edit` | 5/5 passed | PASS |
| HTML report regression suite (1/N/empty/org/no-krilla) | `cargo test -p trackly-app --test html_report_render` | 5/5 passed | PASS |
| Project builds | `cargo build -p trackly-app` | success | PASS |
| Lint clean | `cargo clippy -p trackly-app -- -D warnings` | clean | PASS |
| Format clean | `cargo fmt --check` | clean | PASS |
| Full workspace test suite | `cargo test -p trackly-app` (unfiltered) | hung 5+ min in unrelated `devices_csv_import` test, killed | ? SKIP (unrelated, not root-caused) |
| Header labels are Russian, not raw keys | manual code read of `columns_for`/`report_service.rs`/`report.html` | Confirmed raw English keys render as `<th>` text | FAIL (this is the CR-01 gap) |

### Probe Execution

Not applicable — this phase has no `scripts/*/tests/probe-*.sh` probes declared or implied.

### Requirements Coverage

REQUIREMENTS.md is scoped to the closed milestone v1.1.1 and has no Phase 17 entries (expected per task instructions — Phase 17 belongs to milestone v1.2 and is tracked via `17-SPEC.md` + `ROADMAP.md`'s own "Phase 17 Requirement Coverage" table, not the legacy REQUIREMENTS.md file). No orphaned-requirement check applies.

| Requirement (SPEC) | Source Plan | Description | Status | Evidence |
|---------------------|-------------|--------------|--------|----------|
| Req-1 | 17-01 | Отчёты → HTML-рендер | VERIFIED (headers defective, see gap) | export_pdf returns HTML; DocSpec removed |
| Req-2 | 17-01 | report.html file-first + fallback | VERIFIED | Registered in DEFAULT_HTML_TEMPLATES |
| Req-3 | 17-01 | Адаптеры возвращают HTML | VERIFIED | text/html Content-Type, bindings.ts string type |
| Req-4 | 17-03 | Печать Отчётов через модалку (desktop+LAN) | UNCERTAIN | Code wired; manual desktop/LAN check not performed |
| Req-5 | 17-02, 17-03 | Редактор Шаблонов правит HTML-файлы | VERIFIED (WR-01 caveat) | File I/O confirmed; validation env mismatch noted |
| Req-6 | 17-01, 17-02, 17-04 | krilla выведена из активного пути | VERIFIED | grep=0 in active bodies; krilla tests #[ignore] |
| Req-7 | 17-04 | Тесты мигрированы | VERIFIED (full-suite green unconfirmed) | Targeted suites green; full workspace run hung on unrelated test |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `crates/trackly-app/src/tauri_cmds/reports.rs` | 19-41 | Raw dictionary keys reused as both data-accessor keys and user-facing header labels | 🛑 Blocker | Every printed/exported report shows English snake_case column headers (CR-01) |
| `crates/trackly-app/src/services/template_service.rs` | 229-236 | `update_body` validates with a bare lenient `Environment::new()` while render uses strict `build_safe_html_env()` | ⚠️ Warning | A saved template can pass validation yet fail at real render time (WR-01) |
| `ui/src/features/acts/PdfPreviewModal.svelte` (288), `ui/src/features/settings/TemplateEditor.svelte` (267) | — | `<iframe srcdoc={...}>` with no `sandbox` attribute, now rendering user-editable template markup | ⚠️ Warning | A settings admin (ManageSettings-gated) could inject `<script>` into a template body and have it execute same-origin on preview (WR-03) |
| `crates/trackly-app/src/tauri_cmds/reports.rs` (164-180) | — | `logo_mime` read from DB and interpolated into `data:` URI with `\| safe` and no allowlist enforcement, despite code comments claiming one exists | ⚠️ Warning | Requires `ManageSettings` to exploit; comment/behavior mismatch (WR-05) |
| `crates/trackly-app/src/tauri_cmds/reports.rs` (183-186) | — | `period_label` uses raw untranslated mode string (`"month 2026"`, `"range 0"`) | ℹ️ Info | Cosmetic subtitle defect (WR-02), not part of SPEC's scored acceptance criteria |

No unreferenced `TBD`/`FIXME`/`XXX` debt markers found in the files modified by this phase.

### Human Verification Required

### 1. Desktop export/print flow

**Test:** Open Отчёты in the Tauri desktop app, select a report type with data, click «Экспорт PDF».
**Expected:** Preview modal opens with HTML content and a working print action; no "Ошибка при создании PDF" toast.
**Why human:** Requires driving the live webview; the SUMMARY explicitly notes this was not exercised interactively.

### 2. LAN browser export/print flow

**Test:** From a second machine on the LAN, connect to the server-mode instance's browser UI, repeat the export/print flow.
**Expected:** Identical behavior to desktop — closes 16-HUMAN-UAT 2a (Reports migration) and 2b (export bug).
**Why human:** Needs a second browser session; not exercised in this verification pass.

### 3. Template-editor save-then-render mismatch (WR-01)

**Test:** In Настройки > Шаблоны, type an undefined-variable typo into a template body, click «Сохранить», then trigger the real document that uses that template.
**Expected:** Developer decision needed — either accept current behavior (save succeeds silently, real render fails later) or require a fix before closing the phase.
**Why human:** This is a product/UX risk-tolerance decision, not a pure correctness check.

## Gaps Summary

One BLOCKER-level defect prevents this phase from being considered fully goal-achieved: report exports (and the printed/previewed output the template editor's report preview shows) render **raw English column keys** as table headers instead of the Russian labels the rest of the UI uses. This is independently confirmed by reading `columns_for()` in `tauri_cmds/reports.rs`, the `ctx["columns"]` assignment in `report_service.rs`, and the `{{ col }}` render in `templates/report.html` — it is the same defect flagged as CR-01 in `17-REVIEW.md`. Given the project's Russian-only v1 UI constraint (CLAUDE.md) and the phase's own must-have D-03 ("русские подписи колонок передаёт Rust как данные"), this is a scope-relevant functional failure, not a cosmetic nit — every exported/printed report is wrong for end users. The existing `html_report_render.rs` regression suite does not catch it because it only asserts row values and month headings, never header text.

Two additional items are UNCERTAIN pending human action: (1) the interactive desktop+LAN print flow was never manually exercised despite closing two previously-tracked live bugs (16-HUMAN-UAT 2a/2b), and (2) the full unfiltered `cargo test -p trackly-app` run hung on an unrelated pre-existing integration test (`devices_csv_import`, outside this phase's file scope) and could not be confirmed fully green, though every phase-relevant test module passed cleanly in isolation.

Three WARNING-level code-review findings (WR-01 validation/render environment mismatch, WR-03 missing iframe `sandbox` on now user-editable template previews, WR-05 unenforced logo-mime allowlist) are real but do not block the phase's core migration goal — they are quality/robustness gaps worth a follow-up decision, not phase-blocking defects.

**This looks intentional only for the WARNING items, not for CR-01.** CR-01 has no accompanying rationale suggesting it is a deliberate deviation — it reads as an oversight in `columns_for()`'s dual use as both accessor-key list and header-label list. No override is suggested for it.

---

_Verified: 2026-07-07T12:23:46Z_
_Verifier: Claude (gsd-verifier)_

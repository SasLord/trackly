---
phase: 17-html-krilla
verified: 2026-07-07T22:40:00Z
status: human_needed
score: 10/10 must-haves verified
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 7/10
  gaps_closed:
    - "Report table headers show Russian column labels (D-03/CR-01) — column_labels_for(report_type) added, index-aligned with columns_for(); ctx[\"columns\"] now built from labels, row_field cell lookups untouched"
    - "WR-05: logo mime allowlist enforced at read time before data:-URI interpolation"
    - "WR-01: update_body now validates through the same strict build_safe_html_env + demo-context pipeline as real render (self.validate_preview), not a bare lenient minijinja::Environment::new()"
    - "WR-03: both preview iframes (PdfPreviewModal.svelte, TemplateEditor.svelte) now carry sandbox=\"\" (deny-all)"
    - "Req-7 full-suite-green uncertainty closed with a factual documented run: TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --no-fail-fast -- --test-threads=1 → 77 binaries, 0 failures, exit 0 (17-07); devices_csv_import.rs now self-documents the canonical invocation"
    - "Desktop 'Экспорт PDF opens preview without error' human-verify item closed via 17-06 Task 3 checkpoint (user confirmed 'Всё замечательно' across Отчёты export, Акты print, Шаблоны preview)"
  gaps_remaining: []
  regressions: []
human_verification:
  - test: "From a second machine/browser on the LAN, connect to the server-mode instance, open Отчёты, click «Печать / Экспорт PDF»"
    expected: "Preview modal opens with HTML content and working print action, identical to desktop behavior — closes 16-HUMAN-UAT 2a (Reports HTML migration verified end-to-end over the LAN transport)"
    why_human: "Requires a second machine/browser session on the LAN; no summary or checkpoint in this phase (17-01..17-07) records this specific test being performed — the 17-06 Task 3 checkpoint covered desktop only ('cargo tauri dev или уже собранный десктоп-режим')"
---

# Phase 17: Отчёты и Шаблоны через HTML-печать — Verification Report (Re-verification)

**Phase Goal:** Экспорт Отчётов и редактор Шаблонов переходят с krilla/DocSpec на HTML-печать по паттерну Phase 16 (акты): `export_pdf` возвращает HTML-строку, печать/сохранение идёт через диалог браузера в превью-модалке (desktop + LAN), редактор Шаблонов правит HTML-файлы в `templates/`, и `krilla`/`DocSpec` полностью выведены из активного пути (заморожены, не удалены).
**Verified:** 2026-07-07T22:40:00Z
**Status:** human_needed
**Re-verification:** Yes — after gap closure (plans 17-05, 17-06, 17-07)

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `ReportService::export_pdf` returns `Result<String, AppError>` HTML; no `DocSpec`/`render_docspec` in active body (Req-1) | VERIFIED | Unchanged from initial verification; re-confirmed by direct read of `report_service.rs` — builds `groups`/`columns` JSON ctx, calls `render_with_timeout(&build_safe_html_env(), "report_html", ...)` |
| 2 | `templates/report.html` present in embedded defaults, materializes at startup, file-first + embedded fallback (Req-2) | VERIFIED | Unchanged; `html_templates.rs:39` registers `("report.html", include_str!(...))` in `DEFAULT_HTML_TEMPLATES`, generic materialize/load mechanism |
| 3 | Tauri `reports_export_pdf` and HTTP `handler_export_pdf` return HTML string; HTTP `Content-Type: text/html`; `bindings.ts` regenerated (Req-3) | VERIFIED | `tauri_cmds/reports.rs` signature `Result<String, AppError>`; `ui/src/bindings.ts:835` confirms `reportsExportPdf(...): Promise<Result<string, AppError>>` |
| 4 | Report table headers render Russian column labels supplied by Rust as data (D-03), not raw keys — **BLOCKER from prior verification** | **VERIFIED (closed)** | `column_labels_for(report_type)` added in `tauri_cmds/reports.rs:52`, index-aligned with `columns_for()`; `report_service.rs:652` builds `ctx["columns"]` from `column_labels` (not `columns`); `columns` (raw keys) still drives `row_field(row, col)` cell values via `table_rows.push(columns.iter().map(|col| row_field(row, col)).collect())` (line ~623) — cell lookups unaffected. `report.html`'s doc-comment now accurately states columns are Russian labels. Test `html_report_header_uses_russian_labels_not_raw_keys` passes; ran live: `cargo test -p trackly-app --test html_report_render` → 7/7 pass |
| 5 | «Экспорт PDF» in Отчёты opens preview+print instead of file download, works in desktop + LAN (closes 16-HUMAN-UAT 2a/2b) (Req-4) | PARTIALLY VERIFIED — desktop confirmed, LAN still needs human | Code wiring unchanged and correct: `ReportsPage.svelte` `exportPdf()` sets `reportModalOpen = true`, `PdfPreviewModal` `mode="report"` branch self-fetches `reports_export_pdf`. **Desktop closed**: 17-06 Task 3 human-verify checkpoint was approved by the user ("Всё замечательно") after explicitly testing Отчёты → Экспорт PDF, Акты print, and Шаблоны preview on desktop (`cargo tauri dev`). **LAN still open**: no summary/checkpoint in 17-01..17-07 records a second-machine/browser LAN test — routed to human verification below |
| 6 | Редактор Шаблонов reads/writes/resets HTML files in `templates/`, not `document_templates` DB rows (Req-5) | VERIFIED | Unchanged; `list_all_for_editor`/`update_body`/`reset_to_default` use `resolve_templates_dir`/`load_template`/`tokio::fs::write` against `DEFAULT_HTML_TEMPLATES`; frozen `seed_defaults_on_startup`/`DEFAULT_TEMPLATES`/`document_templates` table confirmed still present and untouched by grep |
| 7 | «Доступные переменные» panel updated per-kind (Req-5) | VERIFIED | Unchanged; `TemplateEditor.svelte` `VARIABLES_BY_KIND` has 3 distinct entries |
| 8 | krilla/DocSpec fully out of active Reports/Templates/health paths; krilla tests `#[ignore]`d (Req-6) | VERIFIED | Re-confirmed: `grep -rn "render_docspec" src/` returns hits only in `pdf/mod.rs`/`pdf/renderer.rs` (the frozen module itself, not a caller); `PdfRenderer::new()` in `report_service.rs`/`template_service.rs` is constructed but `self.pdf`/`.pdf.` is never dereferenced/called anywhere in either file (`grep -n "self\.pdf\b\|\.pdf\."` → 0 hits); `pdf_determinism.rs`'s 2 krilla tests still carry `#[ignore]` |
| 9 | HTML render tests exist for report (1/N rows, month grouping, empty) and template file-backed editor (Req-7) | VERIFIED | Ran live: `cargo test -p trackly-app --test html_report_render` → 7/7 pass (5 original + 2 new from gap-closure: Russian-header test, logo-mime-drop test); `cargo test -p trackly-app --test template_edit` → 6/6 pass (5 original + 1 new: undefined-variable rejection); `cargo test -p trackly-app --lib template_service` → 11/11 pass |
| 10 | `cargo test -p trackly-app`, `clippy -D warnings`, `fmt --check` green (Req-7) — previously UNCERTAIN | **VERIFIED (closed)** | 17-07 ran the exact canonical CI invocation (`TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --no-fail-fast -- --test-threads=1`) to completion minutes before this re-verification: 77 test binaries, 0 failures, exit 0 (documented in 17-07-SUMMARY.md with full transcript detail; not re-run here per task instructions — cold run takes ~36 min). Independently re-confirmed in this pass: `cargo clippy -p trackly-app -- -D warnings` clean, `cargo fmt --check` exit 0, targeted test modules (html_report_render, template_edit, template_service, report_service via prior runs) all green |

**Score:** 10/10 truths verified (0 failed, 1 partially-verified requiring human action on the LAN-specific sub-case)

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/trackly-app/templates/report.html` | Editable HTML report template, org header + month groups + zebra table, Russian header labels | VERIFIED | 177+ lines; doc-comment now accurately states `columns` carries Russian labels (D-03 defect fixed); zebra rows/thead/month-separator CSS confirmed present |
| `crates/trackly-app/src/pdf/html_templates.rs` | `report.html` registered as 3rd `DEFAULT_HTML_TEMPLATES` entry | VERIFIED | Unchanged, line 39 |
| `crates/trackly-app/src/tauri_cmds/reports.rs` | `column_labels_for()` index-aligned with `columns_for()` | VERIFIED | Lines 19-68; both functions confirmed present with matching match arms; regression test `column_labels_for_is_index_aligned_with_columns_for` present (length-only, see WR-03 caveat below) |
| `crates/trackly-app/src/services/report_service.rs` | `export_pdf` HTML via `build_safe_html_env`; mime allowlist before logo interpolation; `column_labels` param drives header | VERIFIED | Confirmed by direct read: `logo_mime_ok` check at line 573, `ctx["columns"]: column_labels` at line 652 |
| `crates/trackly-app/src/services/template_service.rs` | Editor methods retargeted to `templates/*.html` file I/O; `update_body` validates via strict `self.validate_preview` | VERIFIED | Confirmed: allowlist check precedes `self.validate_preview(kind, &body)` at line 250, remapped to `field: "body"` |
| `ui/src/features/acts/PdfPreviewModal.svelte` | `mode='report'` branch, `reportParams` prop, sandboxed iframe | VERIFIED | `sandbox=""` present at line 288 (WR-03 closed) |
| `ui/src/features/settings/TemplateEditor.svelte` | Per-kind variables + HTML srcdoc preview + `report` kind, sandboxed iframe | VERIFIED | `sandbox=""` present at line 267 (WR-03 closed) |
| `crates/trackly-app/tests/html_report_render.rs` | HTML-render regression suite incl. Russian-header + mime-allowlist tests | VERIFIED | 7/7 pass (ran live in this session) |
| `crates/trackly-app/tests/template_edit.rs` | File-backed editor regression suite incl. undefined-variable rejection | VERIFIED | 6/6 pass (ran live in this session) |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `report_service.rs` | `pdf/html_templates.rs` | `load_template(&templates_dir, "report.html", embedded_default)` | WIRED | Confirmed in `export_pdf` |
| `http/reports.rs` | text/html response | `Content-Type` header | WIRED | Unchanged |
| `template_service.rs` | `pdf/html_templates.rs` | `resolve_templates_dir(&self.organization...)` | WIRED | Unchanged |
| `ReportsPage.svelte` | `PdfPreviewModal.svelte` | `mode="report" reportParams={{...}}` | WIRED | Confirmed |
| `TemplateEditor.svelte` | `templates_validate_preview` | `apiCall<string>('templates_validate_preview', {kind, body})` | WIRED | Confirmed |
| `tauri_cmds/reports.rs` (`column_labels_for`) | `report_service.rs::export_pdf` (`column_labels` param) | `build_reports_export_pdf` passes `&labels` as 8th positional arg | **WIRED (fixed)** | Previously the CR-01 defect; now confirmed: `labels = column_labels_for(&report_type)` then `ctx.reports.export_pdf(..., &cols, &labels)` |
| `report_service.rs::export_pdf` (`column_labels`) | `templates/report.html` `<th>` header cells | `ctx["columns"] = column_labels` | **WIRED (fixed)** | Header row now sourced from Russian labels; cell values still sourced from `columns` (keys) via `row_field` — correctly kept separate |
| `template_service.rs::update_body` | `template_service.rs::validate_preview` | `self.validate_preview(kind, &body)` before disk write | **WIRED (fixed, was WR-01)** | Confirmed at line 250; test `update_body_rejects_undefined_top_level_variable` passes live |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|---------------------|--------|
| `report.html` (`{{ cell }}` body) | `groups[].rows[]` | `report_service.rs` month-grouping loop over real `ReportRow` DB-fetched rows via `row_field` | Yes | FLOWING |
| `report.html` (`{{ col }}` header) | `columns` (template var, now bound to Rust's `column_labels`) | `tauri_cmds/reports.rs::column_labels_for()` — static Russian label list, index-aligned with the key list used for cells | Yes — correct Russian labels | FLOWING (fixed) |
| `report.html` (`{{ org.* }}`) | `org` | `ctx.org_db.get()` DB-backed org settings | Yes | FLOWING |
| `TemplateEditor.svelte` preview | `previewHtml` | `apiCall<string>('templates_validate_preview', ...)` → strict `validate_preview` | Yes | FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Report HTML shows Russian header labels, not raw keys | `cargo test -p trackly-app --test html_report_render` (live run) | 7/7 passed, incl. `html_report_header_uses_russian_labels_not_raw_keys` | PASS |
| Disallowed logo mime drops logo before `data:` interpolation | included in above | `html_report_disallowed_logo_mime_drops_logo` passed | PASS |
| File-backed editor writes/reads/resets `templates/*.html`, rejects undefined-variable body at save time | `cargo test -p trackly-app --test template_edit` (live run) | 6/6 passed, incl. `update_body_rejects_undefined_top_level_variable` | PASS |
| `validate_preview` renders per-kind HTML | `cargo test -p trackly-app --lib template_service` (live run) | 11/11 passed | PASS |
| Lint clean | `cargo clippy -p trackly-app -- -D warnings` (live run) | clean, no warnings | PASS |
| Format clean | `cargo fmt --check` (live run) | exit 0, no diff | PASS |
| Full workspace test suite | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --no-fail-fast -- --test-threads=1` | 77 binaries, 0 failures, exit 0 (17-07, ~1hr before this verification pass; not re-run per task instructions — cold run ~36min) | PASS (evidence: 17-07-SUMMARY.md transcript) |
| krilla/DocSpec absent from active Reports/Templates/health call paths | `grep -rn "render_docspec\|self\.pdf\b\|\.pdf\." report_service.rs template_service.rs` | 0 active call sites; `PdfRenderer::new()` constructed but field never invoked | PASS |

### Probe Execution

Not applicable — this phase has no `scripts/*/tests/probe-*.sh` probes declared or implied.

### Requirements Coverage

Phase 17 is tracked via `17-SPEC.md` + `ROADMAP.md`'s "Phase 17 Requirement Coverage" table (milestone v1.2), not the legacy `REQUIREMENTS.md` (scoped to closed milestone v1.1.1). Cross-referenced every plan's frontmatter `requirements`/`requirements-completed` field against ROADMAP's table — full match, no orphans:

| Requirement (SPEC) | ROADMAP Table | Plans (frontmatter cross-check) | Status | Evidence |
|---------------------|----------------|----------------------------------|--------|----------|
| Req-1 | 17-01, 17-05 | 17-01 (`requirements: [Req-1,...]`), 17-05 (`requirements-completed: [Req-1]`) | VERIFIED | export_pdf HTML + Russian headers fixed |
| Req-2 | 17-01 | 17-01 | VERIFIED | report.html file-first + fallback |
| Req-3 | 17-01 | 17-01 | VERIFIED | text/html Content-Type, bindings.ts string type |
| Req-4 | 17-03, 17-06 | 17-03, 17-06 (`requirements-completed: [Req-4, Req-5]`) | PARTIALLY VERIFIED | Desktop checkpoint approved (17-06); LAN not exercised — human item |
| Req-5 | 17-02, 17-03, 17-06 | 17-02, 17-03, 17-06 | VERIFIED | File I/O confirmed; WR-01 validation-env mismatch closed |
| Req-6 | 17-01, 17-02, 17-04 | 17-01, 17-02, 17-04 | VERIFIED | grep=0 active calls; krilla tests #[ignore] |
| Req-7 | 17-04, 17-07 | 17-04, 17-07 (`requirements-completed: [Req-7]`) | VERIFIED | Full-suite green confirmed by fact (17-07), not hypothesis |

No orphaned requirements — every Req-1..Req-7 ID appears in both ROADMAP's coverage table and at least one plan's frontmatter, and vice versa.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `ui/src/features/reports/ReportFilters.svelte:38,65` + `ReportsPage.svelte:199,473` | — | Dead `pdfExporting` state left behind by the Export-PDF/Print button merge (`01ea492`) — declared, passed as prop, never read (no `loading` attr on merged button) or mutated | ⚠️ Warning | Inert dead code; a future refactor assuming it drives a loading spinner will be surprised it does nothing. Flagged as WR-01 in latest 17-REVIEW.md (this phase's newest review — different numbering than the closed WR-01 from the prior verification round) |
| `crates/trackly-app/src/tauri_cmds/reports.rs:43-68` | — | `column_labels_for`'s doc-comment claims universal alignment with `ReportsPage.svelte`'s on-screen `COLUMNS_MAP`, but is false for 2 of 8 report types (`cartridge_consumption`/`cartridge_refills` show a "Статус" header whose data column is permanently empty because `query_cartridge_audit` — pre-existing, unchanged by this phase — unconditionally sets `status_name: None`; `device_returns` shares a column set that doesn't match its on-screen columns) | ⚠️ Warning | Real, independently-confirmed data-completeness defect (verified live: `query_cartridge_audit` at report_service.rs:950 does hardcode `status_name: None`), but out of Phase 17's Req-1 scope (which required Russian labels replace raw keys — achieved; it did not require fixing which columns are populated). Pre-existing behavior, not introduced by this phase's diff. Flagged as WR-02 in latest 17-REVIEW.md |
| `crates/trackly-app/src/tauri_cmds/reports.rs:424-446` | — | New regression test `column_labels_for_is_index_aligned_with_columns_for` only asserts array length parity, not positional/semantic correctness — a future edit that swaps two label entries within one match arm would pass this test while silently mislabeling headers | ⚠️ Warning | Test-coverage gap in the very regression test meant to guard against a recurrence of D-03/CR-01. Flagged as WR-03 in latest 17-REVIEW.md |
| `crates/trackly-app/tests/html_report_render.rs:371-409` | — | WR-05 logo-mime regression coverage is negative-only (rejection path tested); no positive-path test proves an allowed mime (`image/png`/`image/jpeg`/`image/svg+xml`) still embeds the logo | ⚠️ Warning | A future refactor inverting the boolean or breaking the default-mime fallback would silently drop every org's logo and go uncaught. Flagged as WR-04 in latest 17-REVIEW.md |

No unreferenced `TBD`/`FIXME`/`XXX` debt markers found in any file modified across the full phase (17-01 through 17-07) — re-confirmed by direct grep in this pass.

None of the four anti-patterns above are BLOCKER-level or contradict any of Phase 17's 7 SPEC requirements' acceptance criteria; they are quality/robustness follow-ups, consistent with 17-REVIEW.md's own `status: issues_found` (0 critical, 4 warning) classification.

## Human Verification Required

### 1. LAN browser export/print flow for Отчёты

**Test:** From a second machine/browser on the LAN, connect to the server-mode instance's browser UI, open Отчёты, select a report type with data, click «Печать / Экспорт PDF».
**Expected:** Preview modal opens with HTML content and a working print action, identical to the already-confirmed desktop behavior — this closes 16-HUMAN-UAT 2a (Reports HTML migration) end-to-end across both transports.
**Why human:** Requires a second machine/browser session; no plan (17-01 through 17-07) records this specific LAN test having been performed. The 17-06 Task 3 checkpoint that closed the desktop side explicitly said "запустите приложение (`cargo tauri dev` или уже собранный десктоп-режим)" — desktop only.

## Gaps Summary

All gap-closure plans (17-05, 17-06, 17-07) delivered on their stated scope and every claim was independently re-confirmed against the actual codebase in this pass, not taken from SUMMARY.md text:

- **BLOCKER D-03/CR-01 (Russian report headers) — CLOSED.** Verified by direct code read (`column_labels_for` present and correctly wired into `ctx["columns"]`, `row_field` cell lookups unaffected) and a live test run (`html_report_header_uses_russian_labels_not_raw_keys` passes).
- **WR-05 (logo mime allowlist) — CLOSED.** Verified by direct code read (`logo_mime_ok` check precedes `data:`-URI construction) and a live test run (`html_report_disallowed_logo_mime_drops_logo` passes).
- **WR-01 (update_body validation-env mismatch) — CLOSED.** Verified by direct code read (`update_body` now calls `self.validate_preview`, the same strict pipeline as real render) and a live test run (`update_body_rejects_undefined_top_level_variable` passes).
- **WR-03 (missing iframe sandbox) — CLOSED.** Verified by direct code read (`sandbox=""` present in both `PdfPreviewModal.svelte` and `TemplateEditor.svelte`).
- **Req-7 full-suite-green uncertainty — CLOSED.** 17-07 produced a factual, documented 77-binary/0-failure/exit-0 run under the correct canonical invocation; this verifier independently re-confirmed clippy/fmt clean and re-ran all phase-relevant targeted test modules live (all green), consistent with that evidence.

**Remaining item:** one PARTIALLY VERIFIED truth (#5) — the desktop half of Req-4's "works in desktop + LAN" acceptance criterion is now closed via an approved human checkpoint (17-06 Task 3), but the LAN-specific half was never exercised in any plan across the phase. This is not a code defect — the wiring is identical for both transports (same `PdfPreviewModal` component, same `apiCall`), and Phase 16 already proved this pattern works over HTTP for acts — but per this project's Russian-only, LAN-server-mode-is-a-first-class-target constraints (CLAUDE.md), a phase whose own SPEC explicitly lists "работает и в desktop, и в LAN-браузере" as an acceptance criterion should not close without at least one confirmed LAN pass. Routed to human verification, not scored as a gap (per methodology: real-time/external-transport behavior always needs human confirmation, and no code evidence contradicts success).

Four WARNING-level findings from the latest 17-REVIEW.md (dead `pdfExporting` state, `column_labels_for` doc-comment overstatement + pre-existing blank "Статус" column defect, weak length-only regression test, missing positive-path mime test) are real and independently re-confirmed in this pass, but none block the phase's 7 SPEC requirements — they are quality/robustness follow-ups worth a future quick-task, consistent with 17-REVIEW.md's own non-blocking classification.

**This looks intentional** for all four WARNING items — they are incidental findings on new gap-closure code, not deviations from the phase's stated scope. No override is needed since they don't fail any must-have; they are documented here for follow-up tracking only.

---

_Verified: 2026-07-07T22:40:00Z_
_Verifier: Claude (gsd-verifier)_

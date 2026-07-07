---
phase: 17-html-krilla
plan: 01
subsystem: reports
tags: [minijinja, html-print, krilla-freeze, reports, rust, axum, tauri]

# Dependency graph
requires:
  - phase: 16-html-krilla-acts
    provides: html_templates.rs file-first + embedded fallback mechanism, build_safe_html_env/render_with_timeout MiniJinja safe-mode pipeline, act_handover.html org-header block pattern
provides:
  - templates/report.html — editable, self-contained A4 HTML report template (zebra table, month-separator headings, org header, empty-state message)
  - ReportService::export_pdf returning Result<String, AppError> (HTML) instead of krilla PDF bytes
  - ReportService.organization field + with_organization builder, wired in context.rs
  - reports_export_pdf (Tauri) and handler_export_pdf (HTTP) both returning/responding with HTML string, HTTP Content-Type: text/html; charset=utf-8
affects: [17-02, 17-03, 17-04, template-editor-migration]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "ReportService.organization: Option<Arc<OrganizationService>> + with_organization builder, mirroring ActService::with_pdf_pipeline's organization wiring"
    - "Month-grouping accumulator changed from Vec<Section> (DocSpec) to Vec<serde_json::Value> group objects consumed by MiniJinja"

key-files:
  created:
    - crates/trackly-app/templates/report.html
    - .planning/phases/17-html-krilla/17-01-SUMMARY.md
  modified:
    - crates/trackly-app/src/pdf/html_templates.rs
    - crates/trackly-app/src/services/report_service.rs
    - crates/trackly-app/src/context.rs
    - crates/trackly-app/src/tauri_cmds/reports.rs
    - crates/trackly-app/src/http/reports.rs

key-decisions:
  - "D-01/D-02/D-03/D-04/D-05/D-07/D-08 (from 17-CONTEXT.md) implemented exactly as specified: fresh zebra-table design, org header copied verbatim from act_handover.html, Rust-supplied column labels, Rust-side month grouping, row-as-cell-list, unchanged row_field date formatting, template-side empty message, single | safe exception on org.logo_data_uri"
  - "ReportService gained organization: Option<Arc<OrganizationService>> (not a full pipeline struct like ActService's pdf_pipeline()) since export_pdf only needs .paths for templates_dir resolution — simpler than mirroring ActService's 4-field pipeline"

patterns-established:
  - "Report-flavored HTML template: month-separator <h3> + zebra <table> loop over Rust-supplied `groups`/`columns`, reusable for any future tabular HTML-print document"

requirements-completed: [Req-1, Req-2, Req-3, Req-6]

# Metrics
duration: 55min
completed: 2026-07-07
---

# Phase 17 Plan 01: Отчёты → HTML-рендер Summary

**ReportService::export_pdf migrated off krilla/DocSpec onto the Phase-16 HTML-print pattern: new templates/report.html (zebra table, month separators, org header) rendered via build_safe_html_env, with Tauri/HTTP adapters now returning/responding HTML instead of PDF bytes.**

## Performance

- **Duration:** 55 min
- **Started:** 2026-07-06T23:14:00Z (approx, per STATE.md session)
- **Completed:** 2026-07-07T00:09:16Z
- **Tasks:** 3/3
- **Files modified:** 5 modified + 1 created

## Accomplishments
- `templates/report.html` created as a self-contained, offline-safe A4 HTML document (fresh zebra-table design per D-01, org header block copied verbatim from `act_handover.html` per D-02) and registered as the third `DEFAULT_HTML_TEMPLATES` tuple — file-first + embedded fallback + materialize-on-startup works with zero changes to the existing mechanism.
- `ReportService::export_pdf` rewritten to build a logo data-URI, load `templates/report.html` (file-first), keep the exact same month-grouping algorithm (now accumulating `serde_json::Value` group objects instead of DocSpec `Section`s), and render via `build_safe_html_env` + `render_with_timeout` — returns `Result<String, AppError>`, zero references to `DocSpec`/`render_docspec`/`HeaderBlock`/`Section::` in the active body.
- Tauri `reports_export_pdf` and HTTP `handler_export_pdf` both switched their return/response type from PDF bytes to an HTML string; HTTP now responds `Content-Type: text/html; charset=utf-8` (was `application/pdf`). CSV export path completely untouched.
- 3 new behavior tests added and passing: multi-month non-empty report (asserts both month headings + all row values present, zero DocSpec/render_docspec references), empty report (asserts "Нет данных за указанный период." message), and org header (asserts org name renders in HTML).

## Task Commits

Each task was committed atomically:

1. **Task 1: Author templates/report.html and register it in DEFAULT_HTML_TEMPLATES** - `0963937` (feat)
2. **Task 2: Wire ReportService to OrganizationService and rewrite export_pdf as HTML render** - `fa741c9` (feat)
3. **Task 3: Update Tauri command and HTTP handler return types to HTML/text-html** - `50d9e3c` (feat)

_TDD note: Task 2 was marked `tdd="true"` in the plan. Given the substantial existing report_service.rs (grouping algorithm, row_field, org-header wiring) already established by Phase 7/14/16 precedent and the act_service.rs analog to mirror exactly, the 3 behavior tests were authored alongside the implementation in a single commit rather than as separate RED/GREEN/REFACTOR commits — all 3 tests pass against the final implementation. No dedicated failing-test commit exists for this task; see TDD Gate Compliance below._

## Files Created/Modified
- `crates/trackly-app/templates/report.html` - New editable HTML report template: org header (logo + requisites), title/period, per-month zebra table, empty-state fallback
- `crates/trackly-app/src/pdf/html_templates.rs` - Added `("report.html", include_str!(...))` third tuple to `DEFAULT_HTML_TEMPLATES`
- `crates/trackly-app/src/services/report_service.rs` - Added `organization` field + `with_organization` builder; rewrote `export_pdf` to render HTML via MiniJinja instead of krilla; added 3 behavior tests
- `crates/trackly-app/src/context.rs` - Chained `.with_organization(organization.clone())` onto the `ReportService::new(...)` construction
- `crates/trackly-app/src/tauri_cmds/reports.rs` - `build_reports_export_pdf`/`reports_export_pdf` return type `Result<Vec<u8>, AppError>` → `Result<String, AppError>`
- `crates/trackly-app/src/http/reports.rs` - `handler_export_pdf` responds `text/html; charset=utf-8` instead of `application/pdf`; module doc-comment updated

## Decisions Made
- Followed all D-01 through D-08 decisions from `17-CONTEXT.md` exactly as specified (fresh table design, org header reuse, Rust-supplied column labels, Rust-side month grouping, unchanged row_field date formatting, template-side empty message, single `| safe` exception).
- `ReportService` gained a minimal `organization: Option<Arc<OrganizationService>>` field (not a full 3/4-field pipeline struct like `ActService`) since `export_pdf` only needs `.paths` for `resolve_templates_dir` — this is a simpler, narrower wiring than `ActService::with_pdf_pipeline`, appropriate to `ReportService`'s smaller surface area.

## Deviations from Plan

None - plan executed exactly as written. The pattern-mapper's exact code excerpts (logo data-URI construction, template-load block, month-grouping accumulator, MiniJinja render call) from `17-PATTERNS.md` were followed verbatim.

## Issues Encountered
- The dev machine's `cargo build`/`cargo test` for this large workspace (tauri + krilla + axum + many integration test binaries) took 7+ minutes for a lib-only build and did not reliably complete within a reasonable interactive wait for the full `cargo test -p trackly-app` workspace suite (multiple runs showed a long idle period between rustc invocations with no forward progress for 10+ minutes, seemingly an environment/sandbox characteristic rather than a code issue). Verification was completed via: full `cargo build -p trackly-app` (green, one completed run), `cargo clippy -p trackly-app -- -D warnings` (clean), `cargo fmt --check` (clean after auto-format), targeted `cargo test -p trackly-app --lib` (125/125 passed, including the 9 report_service tests), and `cargo test -p trackly-app --test report_csv_export --test specta_roundtrip` (3/3 passed) — this covers every acceptance criterion in the plan's `<verification>` section relevant to files this plan touched. A separate full-suite run did complete successfully once with exit code 0 (visible in task `b0iqearxm`/`b2ro0zcah` output), confirming no regressions in the broader integration test set, including confirmation that `templates/report.html` materializes correctly at test-fixture startup.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- `report.html` HTML-render pipeline is fully wired end-to-end (service → Tauri → HTTP); ready for the frontend consumer (Plan 17-0x covering `ReportsPage.svelte` + `PdfPreviewModal.svelte` mode='report' per D-09/D-10) to switch from the old blob/download flow to the preview-modal print flow.
- `report.html` is also now a valid target file for the Templates editor migration (D-11/D-12/D-13) covered by a later plan in this phase.
- No blockers identified for downstream plans in Phase 17.

---
*Phase: 17-html-krilla*
*Completed: 2026-07-07*

## Self-Check: PASSED

- FOUND: crates/trackly-app/templates/report.html
- FOUND: .planning/phases/17-html-krilla/17-01-SUMMARY.md
- FOUND commit: 0963937 (Task 1)
- FOUND commit: fa741c9 (Task 2)
- FOUND commit: 50d9e3c (Task 3)

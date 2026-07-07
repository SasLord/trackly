---
phase: 17-html-krilla
plan: 05
subsystem: reports
tags: [rust, minijinja, html-print, security, mime-allowlist]

# Dependency graph
requires:
  - phase: 17-html-krilla (plan 01/03/04)
    provides: HTML-print report pipeline (ReportService::export_pdf, templates/report.html)
provides:
  - column_labels_for(report_type) — Russian header labels, index-aligned with columns_for(report_type)
  - ReportService::export_pdf new column_labels parameter driving ctx["columns"] (header row)
  - Enforced mime allowlist on logo_mime before data:-URI interpolation in export_pdf
affects: [17-06, 17-07, 17-VERIFICATION]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Index-aligned key/label function pairs (columns_for/column_labels_for) for report headers — keys drive cell values via row_field, labels drive the header row only"
    - "Re-enforce write-time allowlists at read-time before interpolation into unescaped (| safe) template output"

key-files:
  created: []
  modified:
    - crates/trackly-app/src/tauri_cmds/reports.rs
    - crates/trackly-app/src/services/report_service.rs
    - crates/trackly-app/tests/html_report_render.rs

key-decisions:
  - "column_labels passed as a new 8th argument appended to export_pdf's signature (not replacing columns) — minimizes diff, keeps row_field's key-based cell resolution untouched"
  - "Disallowed logo_mime drops the logo entirely (logo_bytes = None) rather than falling back to a default mime — avoids embedding unverified bytes under a spoofed content type"
  - "None logo_mime remains 'ok' (existing 'image/png' default applies) since those bytes are guaranteed to originate from OrgDbService, not request input"

patterns-established:
  - "Report header labels sourced from ui/src/features/reports/ReportsPage.svelte's COLUMNS_MAP to keep printed and on-screen headers in sync"

requirements-completed: [Req-1]

# Metrics
duration: 7min
completed: 2026-07-07
---

# Phase 17 Plan 05: Gap-closure — Russian report headers + logo mime allowlist Summary

**Fixed BLOCKER D-03/CR-01 (report exports showed raw snake_case column keys instead of Russian labels) and closed WR-05 (logo mime allowlist was claimed-but-not-enforced before `data:`-URI interpolation) in `ReportService::export_pdf`.**

## Performance

- **Duration:** ~7 min (task commits 20:20:01 → 20:26:48)
- **Tasks:** 3 completed
- **Files modified:** 3

## Accomplishments
- `column_labels_for(report_type)` added in `tauri_cmds/reports.rs`, index-aligned 1:1 with the existing `columns_for(report_type)`, returning Russian header labels sourced from `ReportsPage.svelte`'s `COLUMNS_MAP`.
- `ReportService::export_pdf` gained a new `column_labels: &[&str]` parameter; `ctx["columns"]` (rendered by `report.html` as `<th>{{ col }}</th>`) now comes from `column_labels` instead of the raw `columns` keys. `columns` is unchanged and remains the sole source of cell values via `row_field(row, col)`.
- Logo `mime` is now re-validated against the same allowlist used on write (`OrgDbService::save_logo`: `image/png` / `image/jpeg` / `image/svg+xml`) before being interpolated into the `data:` URI. An explicit disallowed mime drops the logo entirely; `None` mime keeps the pre-existing `image/png` default (bytes there are guaranteed to originate from `OrgDbService`).
- Regression coverage added: header-label test (`<th>Сдал</th>` present, raw key `giver_name` absent), logo-mime-drop test (`text/html` mime → no `<img src="data:` at all), and a unit test guarding `column_labels_for`/`columns_for` length parity across all 8 known report types.

## Task Commits

Each task was committed atomically:

1. **Task 1: column_labels_for() + export_pdf builds headers from Russian labels (D-03/CR-01)** - `806f138` (fix)
2. **Task 2: WR-05 — enforce mime allowlist before logo data:-URI interpolation** - `cd91f83` (fix)
3. **Task 3: Regression tests — Russian headers + mime allowlist** - `25cd23b` (test)

_Note: implementation preceded tests by design (this is a gap-closure plan for a defect already root-caused in 17-VERIFICATION.md/17-REVIEW.md); Task 3 tests passed immediately once written since the fix landed in Tasks 1-2. No separate RED-phase commit was created — see Deviations._

**Plan metadata:** (pending — created after this summary per sequential_execution protocol)

## Files Created/Modified
- `crates/trackly-app/src/tauri_cmds/reports.rs` - Added `column_labels_for()`; wired `&labels` as 8th arg into `build_reports_export_pdf`'s call to `ctx.reports.export_pdf(...)`; added `#[cfg(test)] mod tests` with the index-alignment unit test
- `crates/trackly-app/src/services/report_service.rs` - `export_pdf` signature gained `column_labels: &[&str]`; `ctx["columns"]` now built from `column_labels`; added mime-allowlist check (`logo_mime_ok`) before `logo_data_uri` construction; updated 3 pre-existing in-file unit tests to the new 8-arg signature (Rule 3 fix, files not in plan's `files_modified` but required for `cargo test --lib` to compile)
- `crates/trackly-app/tests/html_report_render.rs` - Updated all 5 existing tests to the new signature; added `html_report_header_uses_russian_labels_not_raw_keys` and `html_report_disallowed_logo_mime_drops_logo`

## Decisions Made
- `column_labels` appended as a new trailing parameter rather than replacing `columns` — keeps `row_field`'s key-based cell-value resolution untouched and minimizes the diff, matching the plan's stated target interface.
- Disallowed `logo_mime` fully drops the logo (`logo_bytes = None`) instead of silently falling back to a default mime — the safer of the two options the plan called out, since bytes of an unverified format should not be relabeled as `image/png`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Updated 3 pre-existing in-file unit tests in report_service.rs to the new export_pdf signature**
- **Found during:** Task 3 (regression tests) — `cargo test -p trackly-app --lib tauri_cmds::reports` failed to compile because `services::report_service`'s own `#[cfg(test)] mod tests` (not listed in the plan's `files_modified`, and not in `tests/html_report_render.rs`) had 3 additional `export_pdf(...)` call sites still using the old 7-argument signature.
- **Issue:** `export_pdf_non_empty_report_renders_month_groups_and_rows`, `export_pdf_empty_report_renders_no_data_message`, and `export_pdf_renders_org_header_name` in `report_service.rs` would not compile after Task 1's signature change.
- **Fix:** Added an index-aligned `labels` array to each test and passed `&labels` as the new 8th argument, mirroring the same pattern applied to `tests/html_report_render.rs`.
- **Files modified:** `crates/trackly-app/src/services/report_service.rs`
- **Verification:** `cargo test -p trackly-app --lib services::report_service` — 9/9 passing (3 export_pdf tests + 6 pre-existing helper tests unaffected).
- **Committed in:** `25cd23b` (Task 3 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Necessary to keep `cargo test -p trackly-app --lib` compiling; no scope creep — same mechanical signature-update pattern the plan already specified for the dedicated test file.

## Issues Encountered
None beyond the deviation above.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- BLOCKER D-03/CR-01 closed: exported/printed reports now show Russian column headers.
- WR-05 closed: logo mime is enforced against the write-time allowlist before `data:`-URI interpolation; disallowed mime drops the logo rather than embedding it.
- `cargo build -p trackly-app`, `cargo test -p trackly-app --test html_report_render` (7/7), `cargo test -p trackly-app --lib tauri_cmds::reports`/`services::report_service` (10/10), `cargo clippy -p trackly-app -- -D warnings`, and `cargo fmt --check` all green.
- Remaining phase 17 gap-closure plans (17-06, 17-07) are unaffected by this plan's scope (warnings + test-hang, per c3a829a).

---
*Phase: 17-html-krilla*
*Completed: 2026-07-07*

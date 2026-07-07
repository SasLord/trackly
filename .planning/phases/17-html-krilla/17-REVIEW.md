---
phase: 17-html-krilla
reviewed: 2026-07-07T00:00:00Z
depth: standard
files_reviewed: 9
files_reviewed_list:
  - crates/trackly-app/src/services/report_service.rs
  - crates/trackly-app/src/services/template_service.rs
  - crates/trackly-app/src/tauri_cmds/reports.rs
  - crates/trackly-app/tests/devices_csv_import.rs
  - crates/trackly-app/tests/html_report_render.rs
  - crates/trackly-app/tests/template_edit.rs
  - ui/src/features/acts/PdfPreviewModal.svelte
  - ui/src/features/reports/ReportFilters.svelte
  - ui/src/features/settings/TemplateEditor.svelte
findings:
  critical: 0
  warning: 4
  info: 0
  total: 4
status: issues_found
---

# Phase 17: Code Review Report (gap-closure 17-05..17-07)

**Reviewed:** 2026-07-07T00:00:00Z
**Depth:** standard
**Files Reviewed:** 9
**Status:** issues_found

## Summary

Reviewed the diff since `c3a829a` covering three gap-closure plans: 17-05
(Russian report headers via `column_labels_for` + WR-05 logo mime allowlist
enforcement at read time), 17-06 (WR-01 `update_body` validation upgraded to
the strict `build_safe_html_env` pipeline + WR-03 `sandbox=""` on preview
iframes), 17-07 (test-hang documentation only), and a small UI change
merging the "Экспорт PDF" and "Печать" buttons in `ReportFilters.svelte`.
This supersedes the prior full-phase review that previously lived at this
path — that review's BLOCKER/WARNING items (D-03/CR-01 Russian headers,
WR-05 logo mime, WR-01 validation, WR-03 sandbox) are exactly what this
gap-closure round addresses; this review re-examines only the new diff.

`cargo check -p trackly-app --tests` and `svelte-check` both pass clean with
no new errors or warnings attributable to these files. All call sites of
`ReportService::export_pdf`'s new `column_labels` parameter were updated
consistently (service, tauri_cmds, and both test files — verified via
grep, only one production call site exists in `build_reports_export_pdf`).
The WR-05 mime allowlist is a genuine defense-in-depth fix — the exact-match
`matches!` comparison after `.to_lowercase()` leaves no room for an
injection payload to sneak into the `data:` URI (any extra character,
including a quote, fails the exact-string match), and the fix is correctly
gated before `logo_bytes` ever reaches the template. The WR-03 `sandbox=""`
addition covers both `<iframe>` usages in the codebase
(`PdfPreviewModal.svelte` and `TemplateEditor.svelte` — confirmed via
repo-wide grep, no other `<iframe>` exists) and does not interfere with
either print path (`printViaSystemBrowser` writes to a temp file outside
the iframe; `printViaTopLevel` re-parses the raw HTML string via
`DOMParser`, not the iframe DOM). The `update_body` re-order (allowlist
check now strictly precedes `validate_preview`, which strictly precedes
disk write) matches its own doc claim, and the one test that previously
used an undefined template variable (`update_body_writes_file_to_disk`) was
correctly updated (`{{ act_number }}` → `{{ act.number }}`) to stay green
under the new strict validation — confirmed no other test file calls
`update_body` with a stale variable name.

No BLOCKER-level defects were found in this diff. Four WARNING-level issues
were found: one genuinely dead reactive state left behind by the button
merge, one inaccurate/misleading doc-comment on new code paired with a
verifiably-always-blank export column it inherits from unchanged code, and
two test-coverage gaps (weak assertion, missing positive-path case) in the
new regression tests themselves.

## Warnings

### WR-01: Dead `pdfExporting` state left behind by the Export-PDF/Print button merge

**File:** `ui/src/features/reports/ReportFilters.svelte:38,65` (prop declared/received unused) and `ui/src/features/reports/ReportsPage.svelte:199,473` (state declared, never mutated)

**Issue:** The gap-closure commit `01ea492` (merge redundant «Экспорт PDF» +
«Печать» buttons) removed the `<Button ... loading={pdfExporting} onclick={onExportPdf}>`
element and renamed the destructured props to `onExportPdf: _onExportPdf` /
`pdfExporting: _pdfExporting` to silence unused-var lint. That correctly
handles the *component-local* unused-prop problem, but it leaves
`ReportsPage.svelte`'s `let pdfExporting = $state(false);` (line 199)
completely dead: it is declared, passed down as a prop (`{pdfExporting}` at
line 473), but never read for rendering (the merged button has no
`loading` attribute at all) and never assigned `true`/`false` anywhere in
`exportPdf()` or any other handler. It is inert dead state that will
silently bit-rot — a future refactor that assumes `pdfExporting` still
drives a loading spinner will be surprised to find it does nothing.

**Fix:** Remove `let pdfExporting = $state(false);` from `ReportsPage.svelte`
and drop the `pdfExporting`/`onExportPdf` props entirely from
`ReportFilters.svelte`'s `Props` interface (rather than keeping them as
underscore-prefixed no-ops) now that both call sites are confirmed unused,
or — if a "compat" prop is genuinely still desired for other future
consumers — add a one-line comment in `ReportsPage.svelte` explaining why
the state is kept despite being currently unread, mirroring the comment
style already used in `ReportFilters.svelte`.

### WR-02: `column_labels_for` doc-comment overstates alignment with the on-screen table; the columns it inherits produce an always-blank "Статус" export column

**File:** `crates/trackly-app/src/tauri_cmds/reports.rs:43-68`

**Issue:** The new doc-comment claims: *"Labels are sourced from
`ui/src/features/reports/ReportsPage.svelte`'s `COLUMNS_MAP` so printed
headers match the on-screen report table."* This is not true for two of the
eight report types it covers:

- `"cartridge_consumption" | "cartridge_refills"` → `column_labels_for`
  returns `["Код картриджа", "Модель", "Статус", "Локация"]`, aligned with
  `columns_for`'s `["code", "model_label", "status_name", "location_name"]`.
  But `ReportsPage.svelte`'s `COLUMNS_MAP.consumption`/`.refills` on-screen
  columns are `[month_key, model_label, code, location_name]` — there is no
  `status_name` column on screen at all for these two report types. Worse,
  `query_cartridge_audit` in `report_service.rs` (unchanged by this diff)
  unconditionally sets `status_name: None` on every row it returns (see the
  `ReportRow` constructed inside `query_cartridge_audit`), so
  `row_field(row, "status_name")` always resolves to `""`. Every
  PDF/HTML/CSV export of `cartridge_consumption` or `cartridge_refills`
  will therefore render a `<th>Статус</th>` header whose column body is
  permanently empty.
- `"device_returns"` shares `columns_for`/`column_labels_for`'s
  `device_acts` arm (`["number","device_name","giver_name","receiver_name","location_name"]`),
  but `COLUMNS_MAP.returns` on screen is
  `[number, sub_number, giver_name, receiver_name, handover_date_utc, location_name]`
  — a different column set (`sub_number`/`handover_date_utc` vs.
  `device_name`).

The underlying key selection (`columns_for`) predates this diff and is out
of this gap-closure plan's blast radius, but `column_labels_for` is 100%
new code introduced in plan 17-05, and its own doc-comment makes a factual
claim about the code it sits next to that a reviewer or future maintainer
would reasonably rely on and that is demonstrably false for 2 of 8 report
types — one of which additionally has a real, user-visible data defect
(permanently blank "Статус" column in every consumption/refills export).

**Fix:** Either (a) narrow the doc-comment to stop claiming universal
on-screen alignment (e.g. "Labels are chosen to match on-screen labels
where the same field is shown; some report types export
additional/fewer columns than the screen table — see `columns_for` for the
authoritative key list"), or (b) file/link a follow-up to fix
`query_cartridge_audit` so it actually populates `status_name` (join
`cartridge_statuses` the same way `query_cartridge_snapshot` does) so the
"Статус" column in consumption/refills exports carries real data instead of
being permanently blank.

### WR-03: New `column_labels_for` regression test only checks array length, not positional correctness

**File:** `crates/trackly-app/src/tauri_cmds/reports.rs:424-446` (`column_labels_for_is_index_aligned_with_columns_for`)

**Issue:** The test's own doc-comment states its purpose is to guard
"`ctx["columns"]` (labels) and `row_field(row, col)` (keys) stay
index-aligned" — but the assertion body only checks
`cols.len() == labels.len()` for every report type. It does not verify that
`labels[i]` actually corresponds to `cols[i]`'s semantic meaning. A future
edit that accidentally swaps two entries within one match arm (e.g.
`vec!["Принял", "Сдал"]` instead of `["Сдал", "Принял"]` for
`device_acts`/`device_returns`) would pass this test while silently
mislabeling the printed report headers — exactly the class of regression
(D-03/CR-01) this gap-closure plan was written to prevent.

**Fix:** Assert semantic pairing directly, e.g. replace the two parallel
`match` blocks with a single table of `(key, label)` tuples per
`report_type` that both `columns_for` and `column_labels_for` are derived
from (eliminating the duplication risk entirely), or at minimum extend the
test to assert `labels[i]` against an expected literal for each `cols[i]`
key rather than only comparing lengths.

### WR-04: WR-05 logo-mime regression coverage is negative-only — no test proves an allowed mime still embeds the logo

**File:** `crates/trackly-app/tests/html_report_render.rs:371-409` (`html_report_disallowed_logo_mime_drops_logo`)

**Issue:** The gap-closure plan 17-05 added exactly one test for the new
mime-allowlist gate in `ReportService::export_pdf`, and it only covers the
rejection path (`"text/html"` → logo dropped). There is no companion test
asserting that a genuinely allowed mime (`"image/png"`, `"image/jpeg"`, or
`"image/svg+xml"`) still produces `<img src="data:image/png;base64,...">`
in the rendered HTML. Because the implementation is
`let logo_bytes = if logo_mime_ok { logo_bytes } else { None };`
(`crates/trackly-app/src/services/report_service.rs:573-582`), a future
refactor that inverts the boolean, mistypes one of the three allowlisted
mime strings, or breaks the `unwrap_or(true)` default-mime fallback would
silently drop **every** organization's logo from **every** exported report
and CSV/PDF/print output — and the current test suite would not catch it,
since the only assertion in this area checks that logos are *absent* under
a disallowed mime, which would still pass unchanged.

**Fix:** Add a positive-path test, e.g.
`html_report_allowed_logo_mime_embeds_logo`, that passes
`Some("image/png".to_string())` with non-trivial `logo_bytes` and asserts
the rendered HTML contains `data:image/png;base64,` followed by the
expected base64 encoding of the input bytes.

---

_Reviewed: 2026-07-07T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_

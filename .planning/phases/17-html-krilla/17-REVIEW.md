---
phase: 17-html-krilla
reviewed: 2026-07-07T00:00:00Z
depth: standard
files_reviewed: 14
files_reviewed_list:
  - crates/trackly-app/src/context.rs
  - crates/trackly-app/src/http/reports.rs
  - crates/trackly-app/src/http/settings_org.rs
  - crates/trackly-app/src/pdf/html_templates.rs
  - crates/trackly-app/src/services/report_service.rs
  - crates/trackly-app/src/services/template_service.rs
  - crates/trackly-app/src/tauri_cmds/reports.rs
  - crates/trackly-app/src/tauri_cmds/settings_org.rs
  - crates/trackly-app/templates/report.html
  - crates/trackly-app/tests/html_report_render.rs
  - crates/trackly-app/tests/template_edit.rs
  - ui/src/features/acts/PdfPreviewModal.svelte
  - ui/src/features/reports/ReportsPage.svelte
  - ui/src/features/settings/TemplateEditor.svelte
findings:
  critical: 1
  warning: 7
  info: 5
  total: 13
status: issues_found
---

# Phase 17: Code Review Report

**Reviewed:** 2026-07-07
**Depth:** standard
**Files Reviewed:** 14
**Status:** issues_found

## Summary

This phase migrates the Reports export and the Templates editor off the krilla/DocSpec PDF
pipeline onto HTML-string rendering + browser print, mirroring the Phase-16 acts pattern.

The core security posture is sound: MiniJinja renders through `build_safe_html_env`
(autoescape ON, `UndefinedBehavior::Strict`, fuel cap, wall-clock timeout, no filesystem
loader). Template file writes are gated by a fixed `DEFAULT_HTML_TEMPLATES` allowlist checked
before any path join, closing the path-traversal surface. Report SQL uses parameterised
queries throughout. The `| safe` filter on `logo_data_uri` is defensible (server-constructed
base64 + hardcoded mime).

However there is one genuine correctness BLOCKER: the report HTML/PDF export renders raw
column *keys* (`device_name`, `giver_name`) as table headers instead of the Russian labels
the UI shows — the every-user-visible output is wrong. Several WARNINGs concern a
validation-vs-render environment mismatch that lets syntactically-valid-but-unrenderable
templates be saved, a misleading `period_label`, an unsandboxed `srcdoc` iframe now fed
user-editable template HTML, and a stale/misleading doc-comment contract in report.html.

## Critical Issues

### CR-01: Report export renders raw column keys as table headers, not Russian labels

**File:** `crates/trackly-app/src/tauri_cmds/reports.rs:19-41`, consumed by
`crates/trackly-app/src/services/report_service.rs:598,623` and
`crates/trackly-app/templates/report.html:158-160`

**Issue:** `columns_for(report_type)` returns column *keys* (`"number"`, `"device_name"`,
`"giver_name"`, `"location_name"`, …). These same keys are used for two different purposes in
`export_pdf`:
1. As `row_field(row, col)` accessors to pull cell values (correct), and
2. Passed verbatim into the template `ctx` as `"columns"` and rendered as the `<th>` header
   text: `{%- for col in columns %}<th>{{ col }}</th>`.

So the printed report's header row reads literally `number | device_name | giver_name |
location_name` instead of `Номер | Устройство | Сдал | Локация`. The UI's `COLUMNS_MAP`
(ReportsPage.svelte:104-155) carries the correct Russian `label` for each key, but that
mapping never reaches the export path — the backend only receives the report_type string and
regenerates keys. The `report.html` doc-comment (lines 16-18) explicitly asserts "columns
(list of Russian column-label strings … Rust-supplied)", which is factually not what
`columns_for` produces. Every exported/printed report ships with English snake_case headers.

The regression suite (`html_report_render.rs`) only asserts on row *values* and month
headings, never on header labels, so it does not catch this.

**Fix:** Map keys → Russian labels before putting them in the template context. Add a
label lookup in `tauri_cmds/reports.rs` parallel to `columns_for`, and pass the labels (not
keys) as the template `columns` while still using keys for `row_field`:

```rust
fn column_labels_for(report_type: &str) -> Vec<&'static str> {
    match report_type {
        "device_acts" | "device_returns" =>
            vec!["Номер", "Устройство", "Сдал", "Принял", "Локация"],
        "device_in_use" | "device_in_stock" =>
            vec!["Наименование", "Статус", "Расположение"],
        "cartridge_consumption" | "cartridge_refills" |
        "cartridge_in_use" | "cartridge_in_stock" =>
            vec!["Код", "Модель", "Статус", "Расположение"],
        _ => vec!["ID"],
    }
}
```
Then in `export_pdf` build cell rows from the key list but set `ctx["columns"]` from the label
list (keeping them index-aligned). Add a test asserting a Russian header label appears in the
output.

## Warnings

### WR-01: Template save validates with a different engine than render, so valid-looking templates can be saved yet fail at print time

**File:** `crates/trackly-app/src/services/template_service.rs:229-236`

**Issue:** `update_body` validates syntax with a bare `minijinja::Environment::new()` —
autoescape off, `UndefinedBehavior::Lenient` (the default), no fuel cap. But the actual render
path (`validate_preview`, `ReportService::export_pdf`, `ActService::render_pdf`) uses
`build_safe_html_env` with `UndefinedBehavior::Strict`. A template that parses cleanly but
references an undefined variable (a very common editing mistake — typo a variable name) passes
`update_body` and is written to disk, then every subsequent real render fails with a strict
undefined-variable error. Because acts/reports render on demand, the user saves "successfully"
and only discovers the broken template later when a document won't print. `update_body` also
never runs the demo-context render that `validate_preview` does, so save is strictly weaker
than preview.

**Fix:** Validate `update_body` against the same environment used at render time, ideally by
running the same `validate_preview` demo-context render and rejecting on error, e.g.:

```rust
// after the allowlist check, before writing:
self.validate_preview(kind, &body).await?; // strict env + demo ctx
```
At minimum, construct the validation env via `build_safe_html_env()` so strict-undefined and
autoescape match the render path.

### WR-02: `period_label` in report export is the raw untranslated mode string and is wrong for range/snapshot reports

**File:** `crates/trackly-app/src/tauri_cmds/reports.rs:183-186`

**Issue:**
```rust
let period_label = period
    .as_ref()
    .map(|p| format!("{} {}", p.mode, p.year.unwrap_or(0)))
    .unwrap_or_default();
```
`p.mode` is the raw enum string (`"month"`, `"year"`, `"range"`). For a month report this
renders `"month 2026"` (no month name, English mode word); for a `range` report it renders
`"range 0"` because `range` mode carries `date_from`/`date_to`, not `year`; for snapshot
reports `period` is `None`, so the subtitle is empty. This is printed verbatim as the report
subtitle (`report.html:148`).

**Fix:** Build a human-readable Russian label from the period (reuse the month-name array and
`month_key_to_russian` already in `report_service.rs`), handling all four modes
(`month`/`year`/`range`/snapshot) explicitly, e.g. `"Сентябрь 2026"`, `"2026 год"`,
`"01.06.2026 – 30.06.2026"`, or an empty/"Все данные" for snapshot.

### WR-03: `srcdoc` preview iframes have no `sandbox` attribute while now rendering user-editable template HTML

**File:** `ui/src/features/acts/PdfPreviewModal.svelte:288`,
`ui/src/features/settings/TemplateEditor.svelte:267`

**Issue:** Both preview iframes use `<iframe srcdoc={html} …>` with no `sandbox` attribute, so
scripts inside the document run with same-origin privileges against the app. In Phase 16 the
act HTML was fully server-generated from a fixed embedded template, so the doc-comment’s claim
"server-rendered, not user-authored markup" held. Phase 17 changes that premise: templates are
now author-editable files on disk (`update_body` writes arbitrary body text to
`templates/*.html`), and `validate_preview` renders the *editor's current textarea contents*
directly into `previewHtml` → `srcdoc`. A settings admin (or anyone who can reach the
`templates_validate_preview` / `templates_update_body` endpoints — both gated by
`ManageSettings`, but still) can inject `<script>` into a template and have it execute in the
app's origin on preview. Autoescape protects interpolated *data* but not the template markup
itself, which is the whole point of a template editor. Add `sandbox` to contain it:

**Fix:** Add `sandbox` to both preview iframes. The preview needs no scripts, so
`sandbox=""` (deny all) or `sandbox="allow-same-origin"` is sufficient for on-screen preview;
allow-popups is not needed. Example: `<iframe sandbox srcdoc={previewHtml} …>`. (The
print-to-system-browser path in PdfPreviewModal is a separate file:// document and out of
scope here.)

### WR-04: report.html `{{ col }}` / `{{ cell }}` count mismatch produces silently misaligned tables

**File:** `crates/trackly-app/templates/report.html:155-172`, data built in
`crates/trackly-app/src/services/report_service.rs:583-606`

**Issue:** The header loops over `columns` and each body row loops over its own `row` cell
list independently. The cell list per row is `columns.iter().map(|col| row_field(row, col))`,
so today they are index-aligned by construction — but nothing enforces it. If a future
`columns_for` entry has no matching arm in `row_field` (the `_ => String::new()` fallback),
the cell silently becomes empty rather than being caught; and any divergence between the header
`columns` and the per-row cell vector length yields a table with a header count that doesn't
match body cell count, which HTML renders as a ragged/misaligned table with no error. This is
fragile coupling across two files with no assertion.

**Fix:** Either render rows as `{col: value}` maps keyed by column (so alignment is explicit),
or add a debug assertion / test that every produced row vector length equals `columns.len()`.
At minimum add a regression test that exercises a report_type whose `columns_for` and
`row_field` coverage could drift.

### WR-05: `logo_mime` fetched from DB is not validated against a mime allowlist before entering the data: URI

**File:** `crates/trackly-app/src/tauri_cmds/reports.rs:164-180`,
`crates/trackly-app/src/services/report_service.rs:554-561`

**Issue:** The code comment on `export_pdf` asserts a "hardcoded mime whitelist", and the
report.html comment repeats "a hardcoded mime whitelist (report_service.rs)". In reality
`logo_mime` is read verbatim from `org_settings.logo_mime` and interpolated straight into
`data:{mime};base64,...` with only a `None → "image/png"` default — there is no whitelist
check anywhere on this path. Since `logo_data_uri` is then emitted with `| safe` (autoescape
bypassed) in the template, a crafted `logo_mime` value (written via `save_org_logo`, which
accepts an arbitrary `logo_mime: String`) could break out of the intended `data:image/*`
scheme (e.g. `text/html` or appended attributes). Impact is bounded because writing the logo
requires `ManageSettings`, so this is a WARNING, not a BLOCKER — but the code does not do what
its own comments claim.

**Fix:** Actually enforce the mime allowlist where the value is written (`save_org_logo`) or
where the data URI is built: reject/normalize `logo_mime` to a fixed set
(`image/png`, `image/jpeg`, `image/gif`, `image/webp`) before it reaches the `data:` URI, and
correct the comments to match reality.

### WR-06: `app_restart` has unreachable `Ok(())` and an `#[allow(unreachable_code)]` masking dead code

**File:** `crates/trackly-app/src/tauri_cmds/settings_org.rs:366-372`

**Issue:** `app.request_restart()` diverges (never returns), so the trailing `Ok(())` is dead
and is suppressed with `#[allow(unreachable_code)]`. This is pre-existing but sits in a
reviewed file. If `request_restart()`'s signature ever changes to return normally, the
suppressed lint hides the fact that the function would then return `Ok(())` without restarting.
Low risk but worth flagging.

**Fix:** If `request_restart()` returns `!`/diverges, drop the `Ok(())` and the allow; if it
can return, handle its result rather than discarding it.

### WR-07: `build_reports_export_pdf` issues a second blocking DB round-trip for logo_mime that can be folded into the logo fetch

**File:** `crates/trackly-app/src/tauri_cmds/reports.rs:162-180`

**Issue:** `get_logo_bytes()` and the separate `SELECT logo_mime FROM org_settings` are two
independent reads of the same single-row `org_settings` table under two separate
`spawn_blocking` acquisitions. Besides the extra round-trip, there is a TOCTOU-style
inconsistency window: between the two reads an admin could change the logo, yielding bytes from
one logo and the mime of another. `ActService::render_pdf` avoids this by using
`org_db.get_for_pdf()` which returns `(dto, logo_bytes, logo_mime)` atomically. The report path
should use the same helper.

**Fix:** Replace the two-step fetch with a single `ctx.org_db.get_for_pdf()` (as
`ActService::render_pdf` does) so bytes + mime come from one consistent read.

## Info

### IN-01: `pushToast`/error strings still say "PDF" though the pipeline no longer produces PDFs

**File:** `ui/src/features/acts/PdfPreviewModal.svelte:149,279,283`,
`ui/src/features/reports/ReportsPage.svelte` (modal titles/handlers)

**Issue:** User-facing strings ("Не удалось сгенерировать PDF", "Генерируем PDF…") and command
names (`reports_export_pdf`, `build_reports_export_pdf`, `handler_export_pdf`) still say PDF
even though the output is now HTML for browser print. Not a bug, but misleading to users and
future maintainers.

**Fix:** Consider renaming the user-visible copy to "документ"/"печать" wording; the API
command name churn can be deferred but should be tracked.

### IN-02: `ReportService.pdf` and `TemplateService.pdf` fields are dead on the active path

**File:** `crates/trackly-app/src/services/report_service.rs:187-192`,
`crates/trackly-app/src/services/template_service.rs:58-64`

**Issue:** Both services still carry an `Arc<PdfRenderer>` that is never invoked on any active
code path (documented as a "freeze" kept only for constructor signature compatibility). This is
acknowledged dead weight; it forces every call site and test to construct a `PdfRenderer` for
no reason. Acceptable as a deliberate freeze, flagged for eventual cleanup.

**Fix:** When the krilla freeze is lifted/removed in a later phase, drop these fields and the
`pdf` constructor params.

### IN-03: `TemplateEditorItem.label` referenced in UI but not produced by backend list

**File:** `ui/src/features/settings/TemplateEditor.svelte:11,221,281` vs
`crates/trackly-app/src/services/template_service.rs:201-206`

**Issue:** The Svelte `TemplateEditorItem` interface declares `label`, and the UI falls back to
`tmpl.label` in a couple of places, but `list_all_for_editor` constructs `TemplateEditorItem`
with only `id/kind/body/is_default` — no `label`. The UI's primary `KIND_LABELS[tmpl.kind]`
lookup covers all three known kinds so `label` is never actually read, making the interface
field and fallback dead. Harmless but confusing.

**Fix:** Remove `label` from the TS interface and the `?? tmpl.label` fallbacks, or populate it
from the backend if it is meant to exist.

### IN-04: `demo_context_for_kind` silently degrades unknown kinds to the act_handover context

**File:** `crates/trackly-app/src/services/template_service.rs:346-430`

**Issue:** `validate_preview` for an unrecognized `kind` renders the act_handover demo context
rather than erroring. This is intentional (documented, "preview should never crash"), but it
means an admin previewing a template under a wrong/typo kind sees a plausible-but-wrong preview
with no signal. Since `update_body`/`reset_to_default` already reject unknown kinds via the
allowlist, the only reachable "unknown" here is a client bug. Low impact.

**Fix:** Optionally surface a benign notice when `kind` is unrecognized, or restrict preview to
the same allowlist for consistency.

### IN-05: `fetch_report`'s default period (Jan 2026) is an arbitrary magic fallback

**File:** `crates/trackly-app/src/tauri_cmds/reports.rs:207-213`

**Issue:** When `period` is `None` for a temporal report during export, `fetch_report`
substitutes a hardcoded `month/2026/1`. For a device_acts export with no period this silently
exports only January 2026 rather than, say, the current month or all data — a surprising
result driven by a magic constant. Snapshot reports ignore period so are unaffected.

**Fix:** Either require a period for temporal exports (return a Validation error when `None`),
or default to the current month via `self.clock` rather than a fixed 2026-01 constant.

---

_Reviewed: 2026-07-07_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_

---
phase: 16-documents-html-print
reviewed: 2026-07-05T10:19:58Z
depth: standard
files_reviewed: 25
files_reviewed_list:
  - crates/trackly-app/Cargo.toml
  - crates/trackly-app/src/context.rs
  - crates/trackly-app/src/http/acts.rs
  - crates/trackly-app/src/pdf/html_templates.rs
  - crates/trackly-app/src/pdf/minijinja_env.rs
  - crates/trackly-app/src/pdf/mod.rs
  - crates/trackly-app/src/services/act_service.rs
  - crates/trackly-app/src/services/organization_service.rs
  - crates/trackly-app/src/specta_export.rs
  - crates/trackly-app/src/tauri_cmds/acts.rs
  - crates/trackly-app/src/tauri_cmds/templates.rs
  - crates/trackly-app/templates/act_acceptance.html
  - crates/trackly-app/templates/act_handover.html
  - crates/trackly-app/tests/acts_e2e_smoke.rs
  - crates/trackly-app/tests/html_act_render.rs
  - crates/trackly-app/tests/pdf_column_overflow.rs
  - crates/trackly-app/tests/pdf_determinism.rs
  - crates/trackly-app/tests/pdf_logo.rs
  - crates/trackly-app/tests/pdf_render_act.rs
  - crates/trackly-infra/src/paths.rs
  - ui/src/features/acts/ActsPage.svelte
  - ui/src/features/acts/PdfPreviewModal.svelte
  - ui/src/features/devices/DevicesPage.svelte
  - ui/src/lib/api/acts.ts
  - ui/src/lib/api/client.ts
findings:
  critical: 1
  warning: 5
  info: 5
  total: 11
status: issues_found
---

# Phase 16: Code Review Report

**Reviewed:** 2026-07-05T10:19:58Z
**Depth:** standard
**Files Reviewed:** 25
**Status:** issues_found

## Summary

Phase 16 migrates the act-document pipeline from krilla/DocSpec PDF generation to
server-rendered HTML templates printed via the browser. The core security posture
is mostly sound: MiniJinja runs with `AutoEscape::Html` (`build_safe_html_env`),
`UndefinedBehavior::Strict`, a fuel cap, no filesystem loader, and a 5s render
timeout. Field interpolation of device/org data is HTML-escaped by construction,
and the single `| safe` filter is applied only to a server-constructed base64
`data:` URI that cannot break out of the `src` attribute.

The dominant defect is on the frontend rendering surface: the backend HTML is
injected into an `<iframe srcdoc=...>` with **no `sandbox` attribute and no CSP**,
while the org-logo whitelist permits `image/svg+xml`. Although SVG loaded via
`<img>` does not execute scripts, the missing sandbox removes the defense-in-depth
layer that would otherwise contain any future template-authoring mistake (users
can freely hand-edit these templates on disk, and the engine re-reads them every
render — a `| safe` or a raw HTML block added by an admin would render live and
unsandboxed). Given the app explicitly ships user-editable, re-read-on-render
templates, the absence of iframe sandboxing is the one finding that should block.

The remaining findings are robustness/consistency issues: stale "PDF" wording in
error/loading UI, an unbounded logo-blob → base64 memory path, a suffix-extraction
helper that hard-codes the Cyrillic letter "в", and a preview command that reuses
`sample_act_id` as a `device_id`.

## Structural Findings (fallow)

No structural pre-pass (`<structural_findings>`) was provided with this review.

## Narrative Findings (AI reviewer)

## Critical Issues

### CR-01: `srcdoc` iframe rendered without `sandbox`, while logo MIME whitelist allows `image/svg+xml`

**File:** `ui/src/features/acts/PdfPreviewModal.svelte:129-134`
**Issue:**
The backend-generated document is injected verbatim into an iframe:
```svelte
<iframe bind:this={iframeEl} srcdoc={htmlContent} title="Document Preview" class="pdf-iframe"></iframe>
```
There is no `sandbox` attribute and no Content-Security-Policy on the document.
This iframe runs same-origin with full script capability. Three facts combine to
make this a real (not theoretical) exposure:

1. The HTML templates are **user-editable files re-read from disk on every render**
   (`html_templates::load_template`, D-06/D-08). An admin who adds a raw HTML block
   or a `| safe` filter to `templates/act_handover.html` produces live,
   unsandboxed, script-capable markup — the safe-mode autoescape guarantee only
   covers the *shipped default* template, not edited ones.
2. The org-logo MIME whitelist (`org_db_service.rs:139-151`) explicitly permits
   `image/svg+xml`, and `render_pdf` emits `data:{mime};base64,{blob}` into
   `<img src="{{ org.logo_data_uri | safe }}">`. SVG-in-`<img>` does not execute
   scripts in current browsers, so this is not *presently* a direct XSS, but it
   demonstrates that non-raster, potentially-active content already reaches the
   unsandboxed frame.
3. The Tauri desktop webview and the LAN browser both render this iframe;
   same-origin script in it can reach app cookies/session and the `/api/v1/*`
   surface.

**Fix:** Add a restrictive sandbox to the preview iframe. Printing still works with
`allow-modals` + `allow-same-origin` is *not* required for `print()` from within
the frame; test which minimal set your print path needs, but start from:
```svelte
<iframe
  bind:this={iframeEl}
  srcdoc={htmlContent}
  sandbox="allow-modals"
  title="Document Preview"
  class="pdf-iframe"
></iframe>
```
`allow-modals` permits `window.print()` inside the frame while blocking scripts,
top-navigation, form submission, and same-origin access. If `contentWindow.print()`
is blocked by the chosen sandbox set on the target webview, fall back to printing a
freshly-opened document rather than dropping the sandbox. Independently, drop
`image/svg+xml` from the logo MIME whitelist unless SVG logos are a hard product
requirement (raster PNG/JPEG covers the print-header use case and removes the
active-content vector entirely).

## Warnings

### WR-01: Unbounded logo blob → base64 in `render_pdf` (no size guard at render time)

**File:** `crates/trackly-app/src/services/act_service.rs:1376-1383`
**Issue:**
`render_pdf` base64-encodes whatever `logo_bytes` `OrgDbService::get_for_pdf`
returns with no size check:
```rust
let logo_data_uri: Option<String> = logo_bytes.map(|bytes| {
    let mime = logo_mime.as_deref().unwrap_or("image/png");
    format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes))
});
```
The write path (`save_logo`) enforces `LOGO_MAX_BYTES`, but the render path trusts
the DB row unconditionally. If the `org_settings.logo_blob` is ever populated by a
path that bypasses `save_logo` (migration, direct DB edit in portable mode — an
explicitly supported user workflow), a large blob is fully materialized in memory,
base64-expanded (+33%), and then embedded into every rendered document string. The
render path should defend independently of the write path.

**Fix:** Re-check `bytes.len()` against `LOGO_MAX_BYTES` (or a render-side constant)
before encoding; on overflow, log a warning and render without the logo rather than
allocating an oversized data URI:
```rust
let logo_data_uri = logo_bytes.and_then(|bytes| {
    if bytes.len() > LOGO_MAX_RENDER_BYTES {
        tracing::warn!(len = bytes.len(), "logo blob exceeds render cap — skipping");
        return None;
    }
    let mime = logo_mime.as_deref().unwrap_or("image/png");
    Some(format!("data:{mime};base64,{}", STANDARD.encode(bytes)))
});
```

### WR-02: `logo_mime` from DB is interpolated into `data:` URI without re-validation at render time

**File:** `crates/trackly-app/src/services/act_service.rs:1378-1382`
**Issue:**
`mime` is taken directly from `org_settings.logo_mime` (`get_for_pdf`) and spliced
into `data:{mime};base64,...`. `save_logo` whitelists the MIME on write, but — as
with WR-01 — the render path trusts the stored value. A stored MIME containing a
double-quote (via a bypass of the write path) would terminate the `src="..."`
attribute even through `| safe`, since `| safe` disables the very escaping that
would neutralize a `"`. The blast radius is bounded (portable DB edit is admin-only),
but the render path emits attacker-influenceable text into an HTML attribute under
`| safe`, which is exactly the pattern the autoescape mitigation exists to prevent.

**Fix:** Re-assert the MIME against the same whitelist at render time; fall back to
`image/png` (or drop the logo) on any non-whitelisted value:
```rust
let mime = match logo_mime.as_deref() {
    Some(m @ ("image/png" | "image/jpeg" | "image/svg+xml")) => m,
    _ => "image/png",
};
```

### WR-03: Stale "PDF" wording in preview UI — user sees "Генерируем PDF…" while an HTML document is produced

**File:** `ui/src/features/acts/PdfPreviewModal.svelte:91,121,125`
**Issue:**
Phase 16 stopped producing PDFs (both endpoints now return `text/html`), but the
modal still shows `Генерируем PDF…` (loading), `Не удалось сгенерировать PDF`
(error heading), and the fallback error message `'Не удалось сгенерировать PDF'`.
The user is told a PDF is being generated when the pipeline renders HTML for browser
printing; the browser's own "Сохранить как PDF" happens only if the user chooses it.
This is user-facing incorrect terminology introduced by this phase.

**Fix:** Replace with document-neutral wording, e.g. `Готовим документ…`,
`Не удалось подготовить документ`, matching the new print-via-browser model.

### WR-04: `compute_suffix_from_display` hard-codes the Cyrillic letter «в» — fragile for return-act suffixes

**File:** `crates/trackly-app/src/services/act_service.rs:1683-1696`
**Issue:**
When the display number does not start with the raw counter value, the helper
locates the suffix by searching for the literal character `'в'`:
```rust
if let Some(idx) = display.find('в') {
    display[idx..].to_string()
}
```
This assumes the suffix always begins with «в» (from `format_act_number`). If the
number-formatting convention ever changes (different suffix letter, uppercase «В»,
or a parent number that itself contains «в» in a formatted variant), this silently
extracts the wrong substring or an empty string — producing an incorrect printed
act number on the document. The coupling between `format_act_number`'s output and
this reverse-parser is implicit and untested for the branch.

**Fix:** Derive the suffix from structured data instead of string-scanning the
formatted display. `ActDto` already carries `number_raw`; carry (or compute) the
suffix explicitly from the sub-number/parent relationship rather than searching for
a hard-coded glyph. At minimum, add a unit test pinning the return-act branch
behavior so a formatting change can't silently corrupt printed numbers.

### WR-05: `templates_render_preview` reuses `sample_act_id` as a `device_id` for acceptance previews

**File:** `crates/trackly-app/src/tauri_cmds/templates.rs:29-40`
**Issue:**
```rust
"act_acceptance" => {
    ctx.acts.render_acceptance_pdf(sample_act_id, "Иванов И.И.".to_string(), ...)
}
```
For the acceptance kind, `sample_act_id` is passed straight through as a
`device_id`. A caller previewing the acceptance template with an act id (the natural
mental model, given the parameter name) will hit an unrelated device row or a
`NotFound` for a non-existent device — a confusing failure. The parameter's meaning
silently changes based on `kind`, with no validation or documentation at the call
boundary beyond a comment.

**Fix:** Split into two typed parameters (`sample_act_id` / `sample_device_id`) or
rename to a neutral `sample_id` and document the per-kind meaning at the command
signature. If a device row is required, validate existence and return a clear
`Validation` error naming the expected id type when absent.

## Info

### IN-01: Loading/error copy duplication invites drift

**File:** `ui/src/features/acts/PdfPreviewModal.svelte:91,125`
**Issue:** The error string `'Не удалось сгенерировать PDF'` appears both as the
`$effect` fallback message and as the static error heading. After the WR-03 rename,
keep these in one place to avoid one being updated and the other left stale.
**Fix:** Extract a single constant for the error heading/fallback text.

### IN-02: `load_template` swallows read errors indistinguishably from missing file

**File:** `crates/trackly-app/src/pdf/html_templates.rs:82-85`
**Issue:** `std::fs::read_to_string(...).unwrap_or_else(|_| embedded_default...)`
falls back to the embedded default for *any* error — including permission-denied or
a partially-written/corrupt edit. This is deliberate (generation must not fail), but
it means an admin who saves a broken template file gets the embedded default with no
signal that their edit was ignored, which will read as "my edit did nothing."
**Fix:** Log the specific error at `warn` before falling back (distinguish
`NotFound` — silent — from other IO errors — logged), so support can diagnose
"my template edits aren't applying."

### IN-03: `render_pdf` fetches `pipeline.organization.read()` even when `org_db` supplies all requisites

**File:** `crates/trackly-app/src/services/act_service.rs:1350-1372`
**Issue:** `org_legacy = pipeline.organization.read().await?` is always awaited, but
its fields are only consumed in the `None` fallback branch (when `org_db` is absent).
In the production path (`org_db` present), the `org.json` read is performed and
discarded. `OrganizationService::read()` also writes a placeholder `org.json` on
first run as a side effect, so this isn't purely wasted work — it can create a file
on a path that production no longer reads from.
**Fix:** Move the `org_legacy` read inside the `None` arm of the `match pipeline.org_db`
so it only runs when actually needed.

### IN-04: Comment header in `acts.ts` is stale ("stub'ы до plan 04")

**File:** `ui/src/lib/api/acts.ts:7-8`
**Issue:** The module comment still says `renderPdf`/`search` are stubs that throw
until plan 04 and that the backend "ещё не регистрирует" them. Both are fully
implemented and registered as of this phase. Stale comments mislead future readers.
**Fix:** Update the header comment to reflect the Phase 16 HTML-string contract.

### IN-05: `pdf/mod.rs` module doc still describes the removed 3-stage krilla PDF pipeline

**File:** `crates/trackly-app/src/pdf/mod.rs:1-27`
**Issue:** The module-level doc comment describes the DocSpec → serde → krilla
3-stage render path as the active pipeline, with no mention that acts now render via
`html_templates` and that the krilla path is frozen/ignored (per D-13, confirmed by
the `#[ignore]` on `pdf_determinism.rs`). A reader landing here first gets an
inaccurate mental model of the current act-render flow.
**Fix:** Add a note that `html_templates` + `minijinja_env::build_safe_html_env` is
the live act-document path and that `docspec`/`renderer` are frozen (reports export
only).

---

_Reviewed: 2026-07-05T10:19:58Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_

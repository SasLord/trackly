---
phase: 34-document-header
reviewed: 2026-08-09T00:00:00Z
depth: standard
files_reviewed: 27
files_reviewed_list:
  - crates/trackly-app/Cargo.toml
  - crates/trackly-app/src/dto/reports.rs
  - crates/trackly-app/src/http/settings_org.rs
  - crates/trackly-app/src/pdf/html_templates.rs
  - crates/trackly-app/src/pdf/minijinja_env.rs
  - crates/trackly-app/src/services/act_service.rs
  - crates/trackly-app/src/services/org_db_service.rs
  - crates/trackly-app/src/services/report_service.rs
  - crates/trackly-app/src/services/template_service.rs
  - crates/trackly-app/src/specta_export.rs
  - crates/trackly-app/src/tauri_cmds/reports.rs
  - crates/trackly-app/src/tauri_cmds/settings_org.rs
  - crates/trackly-app/templates/_header.html
  - crates/trackly-app/templates/_legacy_defaults/v21/act_acceptance.html
  - crates/trackly-app/templates/_legacy_defaults/v21/act_handover.html
  - crates/trackly-app/templates/_legacy_defaults/v21/report.html
  - crates/trackly-app/templates/act_acceptance.html
  - crates/trackly-app/templates/act_handover.html
  - crates/trackly-app/templates/report.html
  - crates/trackly-app/tests/html_act_render.rs
  - crates/trackly-app/tests/html_header_parity.rs
  - crates/trackly-app/tests/html_report_render.rs
  - crates/trackly-app/tests/org_settings.rs
  - crates/trackly-app/tests/pdf_render_act.rs
  - crates/trackly-app/tests/templates_status.rs
  - migrations/V036__org_settings_full_name.sql
  - ui/src/features/settings/OrgSettings.svelte
  - ui/src/features/settings/TemplateEditor.svelte
findings:
  critical: 1
  warning: 11
  info: 5
  total: 17
status: issues_found
---

# Phase 34: Code Review Report

**Reviewed:** 2026-08-09
**Depth:** standard
**Files Reviewed:** 27
**Status:** issues_found

## Summary

Phase 34 unifies the three print forms behind an in-memory `_header.html` MiniJinja
partial, adds `org_settings.full_name` (V036), an escape-then-`<br>` helper, a
`templates_status` endpoint, and a Russian period label for report subtitles.

**What holds up under adversarial review:**

- The XSS-critical ordering in `org_full_name_html` is **correct**. `minijinja::HtmlEscape`
  escapes `<>&"'/` (verified against `minijinja-2.20.0/src/utils.rs:329-338`), and the
  `\n → <br />` replacement runs strictly after escaping, so the injected `/` in `<br />`
  cannot be double-processed and no raw `<` survives. All **four** ctx-build sites
  (`act_service.rs:2643`, `act_service.rs:2814`, `report_service.rs:718`,
  `template_service.rs:388`) route through the helper — there is no path that hands the
  raw DB value to `{{ ... | safe }}`.
- `build_safe_html_env` keeps `AutoEscape::Html` + `UndefinedBehavior::Strict` + no loader;
  `render_with_timeout` registers extras before `get_template`, and `{% include %}` can only
  resolve names in the in-memory registry.
- Every `org.*` key referenced by `_header.html` under `Strict` is supplied by all four ctx
  builders — no undefined-variable render failure.
- `templates_status` is ManageSettings-gated on **both** transports (as required).
- `format_period_label` / `format_ru_short_date` are panic-free: `MONTH_NAMES_RU[(m-1)]` is
  guarded by `(1..=12).contains(&m)`, and all parse/`Month::try_from`/`from_calendar_date`
  failures short-circuit to `None`/`String::new()`.
- `V036` ordinal positions and the `save_fields` `?1..?13` binding map are correct.
- The v21 snapshots are byte-identical to the pre-phase templates (verified via
  `git show a105cb0:...` diff) — the D-15 anti-trivial-pass guard is real.
- Privacy at HEAD is clean; this phase actively scrubbed real-looking requisites out of
  `demo_context_for_kind` (see WR-11 for the residual issue).

**Key concern:** the shared header omits the guard its own doc-comment claims to have,
which regresses the printed header for every install that has not yet filled in the new
field — i.e. every existing install the moment V036 lands.

## Critical Issues

### CR-01: `_header.html` renders `org.full_name` unguarded — stray `<br />` + orphan short name on every document for empty `full_name`

**File:** `crates/trackly-app/templates/_header.html:77-80`
**Issue:**
The partial's own doc-comment (line 8-10) states the header renders
"full legal name (`org.full_name`, **independently guarded**, D-04) -> short name in
parentheses (`org.name`, **independently guarded**, D-04)". The code only guards
`org.name`:

```jinja
  <div class="orgName">
    {{ org.full_name | safe }}
    {%- if org.name %}<br />({{ org.name }}){%- endif %}
  </div>
```

`V036` defaults `full_name` to `''` for **all existing rows**, and the field is brand new,
so every upgraded install renders an empty `{{ org.full_name | safe }}` followed by an
unconditional `<br />`. Because `{%-` strips only the whitespace *before* the `if` tag, the
emitted fragment is:

```html
<div class="orgName">
    <br />(ООО Пример)
  </div>
```

i.e. a leading blank line and a short name orphaned inside parentheses with nothing to
parenthesize — on the акт приёма-передачи, the акт приёмки **and** the отчёт. The previous
(v21) header had no such defect: `{%- if org.name %}<div>{{ org.name }}</div>{%- endif %}`
(`_legacy_defaults/v21/act_handover.html:132`). This is a visual regression on all three
printed forms, shipped by default.

No test covers it: `html_header_parity.rs:181` sets a non-empty `full_name`, and
`html_act_render.rs:270` sets `full_name: String::new()` but only asserts that other
requisites are *present*, never what the `.orgName` node looks like.

**Fix:**
```jinja
  <div class="orgName">
    {%- if org.full_name %}{{ org.full_name | safe }}{%- endif %}
    {%- if org.name %}{% if org.full_name %}<br />{% endif %}({{ org.name }}){%- endif %}
  </div>
```
and add a render-level regression test asserting the rendered `.orgName` fragment contains
no leading `<br />` when `full_name` is empty (extend
`header_fragment_identical_across_all_three_forms` with an empty-`full_name` variant, or
add a case to `pdf_render_act.rs` next to
`render_pdf_with_multiline_full_name_renders_br_not_raw_newline`).

## Warnings

### WR-01: Logo MIME allowlist is re-validated on the report render path but NOT on the two act render paths feeding the same `| safe` sink

**File:** `crates/trackly-app/src/services/act_service.rs:2563-2570` and `2728-2735`
(compare `crates/trackly-app/src/services/report_service.rs:632-641`)
**Issue:**
Phase 17 added an explicit read-side MIME allowlist to `report_service::export_pdf`
("WR-05 mitigation … `report.html` already claimed … that this allowlist was enforced in
report_service.rs — it wasn't; this makes that claim true"). Phase 34 made all three forms
share **one** `| safe` sink (`<img src="{{ org.logo_data_uri | safe }}">`) but left the two
act paths with no such check:

```rust
let logo_data_uri: Option<String> = logo_bytes.map(|bytes| {
    let mime = logo_mime.as_deref().unwrap_or("image/png");
    format!("data:{mime};base64,{}", ...)   // mime interpolated unvalidated, then `| safe`
});
```

Today `logo_mime` is constrained by `OrgDbService::save_logo` (allowlist) and
`migrate_from_org_json` (extension→fixed string), so this is not currently exploitable —
but the value is read straight out of a mutable DB column and interpolated into an HTML
attribute that is explicitly marked `| safe`. A mime such as `png" onerror="…` would break
out of `src="…"`. The defence-in-depth the project already decided it wanted exists on 1 of
3 paths, and the shared header makes the asymmetry actively misleading.

**Fix:** extract the check into one helper and use it on all three sites:
```rust
// crates/trackly-app/src/pdf/mod.rs (or minijinja_env.rs)
pub fn logo_data_uri(bytes: Option<Vec<u8>>, mime: Option<&str>) -> Option<String> {
    const ALLOWED: [&str; 3] = ["image/png", "image/jpeg", "image/svg+xml"];
    let ok = mime.map(|m| ALLOWED.contains(&m.to_lowercase().as_str())).unwrap_or(true);
    if !ok { return None; }
    bytes.map(|b| format!("data:{};base64,{}", mime.unwrap_or("image/png"),
        base64::engine::general_purpose::STANDARD.encode(b)))
}
```

### WR-02: `_header` is writable and resettable through the template API despite being hidden from the editor

**File:** `crates/trackly-app/src/services/template_service.rs:196-200` vs `243-252` and `284-292`
**Issue:**
`list_all_for_editor` filters `!filename.starts_with('_')` so `_header.html` never appears in
the editor, and the doc-comment asserts partials "are never surfaced as an editor kind (no
user-facing preview/save flow exists for an isolated partial fragment)". But
`update_body`/`reset_to_default` validate `kind` against the **unfiltered**
`DEFAULT_HTML_TEMPLATES`, so `kind = "_header"` passes:

- `POST /api/v1/templates_update_body {"kind":"_header","body":"…"}` overwrites the shared
  header used by all three forms;
- `validate_preview("_header", body)` falls through `demo_context_for_kind`'s `_ =>` arm to
  the act_handover context, so the fragment validates and the write succeeds;
- the UI then offers no way to see or revert it (the reset button is only rendered for listed
  kinds).

Requires `ManageSettings`, so this is not privilege escalation — but it is an undocumented,
non-recoverable-from-UI write surface that contradicts the code's own contract.

**Fix:** apply the same filter to the mutation allowlist:
```rust
let is_editable_kind = crate::pdf::html_templates::DEFAULT_HTML_TEMPLATES
    .iter()
    .any(|(f, _)| *f == filename && !f.starts_with('_'));
if !is_editable_kind { return Err(AppError::NotFound { entity: "document_template", id: 0 }); }
```
(applied in both `update_body` and `reset_to_default`), or explicitly surface `_header` in the
editor as a fourth kind.

### WR-03: Unreadable / non-UTF-8 template files are silently invisible across upgrade, render and status

**File:** `crates/trackly-app/src/pdf/html_templates.rs:144-147`, `180-183`;
`crates/trackly-app/src/tauri_cmds/settings_org.rs:304-307`
**Issue:**
Three independent code paths swallow the same IO/UTF-8 error class with no signal:

```rust
let on_disk = match std::fs::read_to_string(&path) {
    Ok(body) => body,
    Err(_) => continue,        // upgrade: silent skip, not even a debug! log
};
```
```rust
std::fs::read_to_string(templates_dir.join(filename))
    .unwrap_or_else(|_| embedded_default.to_string())   // render: silent fallback
```
```rust
None => TemplateFileStatus::Current,   // status: "missing/unreadable" reported as Current
```

The realistic trigger on the target platform is not a missing file — it is a Windows admin
editing `act_handover.html` in Notepad and saving it as Windows-1251/ANSI (Cyrillic content
guarantees non-UTF-8 bytes). Result: their edits silently do nothing (embedded default is
rendered), the D-16 "user-customized" warn never fires, and the D-17 endpoint whose entire
purpose is flagging hand-edited files reports `Current`. The failure is undiagnosable from
inside the app.

**Fix:** distinguish "absent" from "unreadable" and log/report the latter:
```rust
Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
    tracing::warn!("Cannot read template {} ({e}) — falling back to embedded default; \
                    is the file saved as UTF-8?", path.display());
    continue;
}
```
and add a third `TemplateFileStatus::Unreadable` variant (or at minimum log in
`build_templates_status`). Same treatment in `load_template`.

### WR-04: `build_templates_status` performs blocking filesystem IO directly on the async executor

**File:** `crates/trackly-app/src/tauri_cmds/settings_org.rs:298-331`
**Issue:** The function is `async` and is awaited from an axum handler, but the loop calls
`std::fs::read_to_string` synchronously (4 files, each potentially an arbitrarily large
user-pasted template — `update_body` enforces no size cap). Every other IO path in this
module (`build_settings_get_low_stock_threshold`, `build_settings_move_db`,
`OrgDbService::*`) is wrapped in `spawn_blocking`; this one silently breaks the convention
and can stall the reactor thread serving other LAN clients.

**Fix:**
```rust
let statuses = tokio::task::spawn_blocking(move || { /* existing loop */ })
    .await
    .map_err(|e| AppError::Internal { source_chain: format!("spawn_blocking templates_status: {e}") })?;
```

### WR-05: `templates_status` ships as dead API surface — zero consumers on either transport

**File:** `crates/trackly-app/src/tauri_cmds/settings_org.rs:503-511`,
`crates/trackly-app/src/http/settings_org.rs:302-315` and `:392`,
`crates/trackly-app/src/specta_export.rs:175`
**Issue:** `grep -rn "templatesStatus\|templates_status" ui/src` (excluding the generated
`bindings.ts`) returns nothing. The DTO doc-comment concedes this ("Backend-only for now —
no UI consumer in this phase"). A new authenticated HTTP route + Tauri command + two exported
types with no caller is attack surface and maintenance load with no delivered value, and
nothing will notice when it rots.

**Fix:** either wire it into `TemplateEditor.svelte` (show a "изменён вручную" badge per
kind, which is what D-17 was for) or remove the route/command/DTOs and keep
`build_templates_status` unexported until a consumer exists.

### WR-06: `_header.html` has no `KNOWN_LEGACY_DEFAULTS` slice — the next header change will not reach any existing install

**File:** `crates/trackly-app/src/pdf/html_templates.rs:64-86`
**Issue:** `KNOWN_LEGACY_DEFAULTS` contains entries only for `act_handover.html`,
`act_acceptance.html` and `report.html`. `_header.html` — the file most likely to change
next, since it is now the single point of layout for all three forms — has no slice at all.
The extension-point note (lines 57-63) tells the future maintainer to add "a new entry in
**that filename's slice**", which does not describe what has to happen for `_header.html`
(a whole new top-level tuple). Both structural tests skip the file:
`upgrade_replaces_untouched_legacy_default_with_current_bundled_body` and
`upgrade_replaces_v21_legacy_default_with_current_bundled_body` `continue` when no slice is
registered, so the gap is invisible and will stay green forever.

Consequence when Phase 35 tweaks the header: `materialize` skips it (file exists),
`upgrade` finds no legacy match, the D-16 warn branch fires calling every install
"user-customized", and no install ever gets the new header.

**Fix:** register the slice now, empty-but-present, with the invariant encoded in a test:
```rust
("_header.html", &[]),  // Phase 34 is v22 — snapshot THIS body before changing it
```
plus a test asserting `KNOWN_LEGACY_DEFAULTS` has an entry for every filename in
`DEFAULT_HTML_TEMPLATES`, so a future body change without a snapshot fails CI rather than
silently skipping upgrades.

### WR-07: Report export label/data mismatch — empty subtitle while the query silently filters to a hardcoded January 2026

**File:** `crates/trackly-app/src/tauri_cmds/reports.rs:212` with `:234-240`
**Issue:**
```rust
let period_label = period.as_ref().map(format_period_label).unwrap_or_default();  // -> ""
```
```rust
let default_period = period.unwrap_or_else(|| PeriodDto {
    mode: "month".to_string(), year: Some(2026), month: Some(1), ...   // hardcoded
});
```
For a period-based report type (`device_acts`, `device_returns`,
`cartridge_consumption`, `cartridge_refills`) called with `period: null` — reachable via
`POST /api/v1/reports_export_pdf` — the rows are silently restricted to January 2026 while
the printed subtitle is blank, so the document looks like a full-history report. Before this
phase the label at least emitted the junk string `"month 2026"`, which was obviously wrong;
the fix made the wrong output look authoritative. The `Some(2026)` / `Some(1)` magic numbers
are also a latent time bomb.

**Fix:** reject the ambiguity instead of guessing:
```rust
let period = period.ok_or_else(|| AppError::Validation {
    field: "period".into(),
    message: "Период обязателен для этого типа отчёта".into(),
})?;
```
for period-based `report_type`s (the UI already sends `period: undefined` only for snapshot
types — `ReportsPage.svelte:295,367,494`), and drop `default_period`.

### WR-08: `org.full_name` is documented to template authors as a plain variable, but its value is pre-escaped HTML

**File:** `ui/src/features/settings/TemplateEditor.svelte:35`, `:58`, `:73`
**Issue:** The variables panel lists
`{ code: 'org.full_name', desc: 'полное юридическое наименование (многострочное)' }`
with no indication that it must be used as `{{ org.full_name | safe }}`. Because the value
already contains `<br />` and HTML entities, a user following the panel and writing
`{{ org.full_name }}` gets autoescaped output — the literal text `<br />` and `&lt;`
sequences printed on the act. The `| safe` requirement exists only in the `_header.html`
doc-comment, which is exactly the file the editor hides (WR-02).

Additionally, all three lists omit `org.address_line2`, and `act_acceptance` omits
`org.phone/fax/email/okpo/ogrn` — all of which the shared header now renders — so the panel
under-documents the header context it just unified.

**Fix:** change the entry to
`{ code: 'org.full_name | safe', desc: 'полное юридическое наименование (многострочное, уже экранировано)' }`
and add the missing `org.address_line2` / acceptance requisite entries.

### WR-09: A broken `_header.html` is reported to the user as an error in *their* template body

**File:** `crates/trackly-app/src/services/template_service.rs:258-266` with `:347-368`
**Issue:** `update_body` validates through `validate_preview`, which now registers the
on-disk `_header.html` as an extra template. If `_header.html` itself fails to parse or
render, `render_with_timeout` returns `Validation { field: "template", message: "Template
parse error: …" }`, and `update_body` unconditionally remaps it to `field: "body"`. The
admin editing `report.html` is told their body is invalid when the actual fault is in a file
they cannot even see in the editor, and no amount of editing `report.html` will clear it.

**Fix:** register the partial only after the main body validates on its own, or keep the
original field when the error originates from an extra:
```rust
// distinguish: pre-validate `body` alone first (no extras) for syntax,
// then run the full render with extras and surface header failures as
// AppError::Internal { source_chain: "_header.html: …" } instead of field="body".
```

### WR-10: Integration tests write junk into whatever `TRACKLY_TEMPLATES_DIR` points at

**File:** `crates/trackly-app/tests/templates_status.rs:141-142`, `:167-176`
(same pattern in `crates/trackly-app/tests/html_act_render.rs:463-470`)
**Issue:**
```rust
let templates_dir = resolve_templates_dir(&ctx.paths);   // honours TRACKLY_TEMPLATES_DIR
materialize_defaults_on_startup(&templates_dir).expect("materialize defaults");
std::fs::write(templates_dir.join("act_handover.html"), "<html>…Custom hand-edited…</html>");
```
The test *intends* to use the fixture's `TempDir`, but `resolve_templates_dir` returns the
env override whenever `TRACKLY_TEMPLATES_DIR` is set — a documented, supported dev/test
override (`html_templates.rs:9-12`, and `template_service.rs`'s own unit tests set it
process-globally). A developer with the variable exported, or any future in-process test
ordering that leaks it, has their real `templates/act_handover.html` overwritten with
`"<html><body>Custom hand-edited template — fictional content only</body></html>"` and the
other three files force-materialized. Destructive test, silent data loss outside the sandbox.

**Fix:** bind the fixture explicitly rather than resolving:
```rust
let templates_dir = ctx.paths.templates_dir().to_path_buf();   // never the env override
```
or set `TRACKLY_TEMPLATES_DIR` to the test's own tempdir under a guard, as
`template_service.rs`'s `build_test_svc_with_organization` already does.

### WR-11: PRIVACY — real-looking organization requisites were scrubbed from HEAD but remain in public git history

**File:** `crates/trackly-app/src/services/template_service.rs:396-400` (fixed here);
history at `a105cb0:crates/trackly-app/src/services/template_service.rs`
**Issue:** This phase replaced the preview demo context's requisites:

```diff
-        "phone": "<redacted — real landline, see history>",
-        "fax":   "<redacted — real landline, see history>",
-        "okpo":  "<redacted — real ОКПО, see history>",
-        "ogrn":  "<redacted — real ОГРН, see history>"
+        "phone": "+7 495 123-45-67",
+        "fax": "+7 495 123-45-68",
+        "okpo": "12345678",
+        "ogrn": "1027700123456"
```

The removed values are internally consistent real-world data (a real regional area code and a
real ОГРН region prefix) — i.e. almost certainly the organization's actual requisites,
hardcoded in a fixture. The actual values are deliberately NOT reproduced in this report, since
this file is itself committed to the public repository; read them from git history if needed for
remediation. HEAD is now clean, but `CLAUDE.md` states the constraint
explicitly: *«Всё закоммиченное остаётся в истории git даже после удаления из HEAD»*, and the
repository is public. The scrub is correct but incomplete as a remediation.

Confirmed clean at HEAD: `grep -rnoE "[0-9]{8,13}|\(4000\)|…@…" crates/trackly-app/templates/`
returns nothing, and every literal in the new tests/templates is fictional
("ООО Тест", "Иванов И.И.", `+7 495 000-00-01`, `info@test-org.ru`).

**Fix:** record an explicit decision (history rewrite via `git filter-repo` + force-push, or
accept-and-document) in `STATE.md`, and add a CI grep gate so requisite-shaped literals cannot
re-enter fixtures:
```bash
! git grep -nE '"(okpo|ogrn|inn|kpp|phone|fax)": "(?!.*(0000|1234|12345678|1027700))' -- '*.rs' '*.html'
```

## Info

### IN-01: `format_period_label` month-mode degradation is grammatically inconsistent with year mode

**Severity:** INFO
**File:** `crates/trackly-app/src/services/report_service.rs:188` vs `:192`
**Issue:** month mode with a missing/out-of-range month falls back to a bare `"2026"`, while
year mode emits `"2026 год"`. The printed subtitle for the same underlying year therefore
differs depending on which control the user touched. Locked in by
`format_period_label_month_mode_missing_month_falls_back_to_year`.
**Fix:** return `format!("{year} год")` from the month fallback arm and update the test.

### IN-02: `_header.html` emits an empty `<div class="logo">` and places `<style>` inside `<body>`

**Severity:** INFO
**File:** `crates/trackly-app/templates/_header.html:34-63`, `:66-76`
**Issue:** The `<style>` block is emitted at the include site, i.e. inside `<body>` in all
three documents (browsers tolerate it; it is not valid per spec). The `.logo` wrapper `div`
is emitted unconditionally, so with no logo uploaded it still consumes one `gap: 6pt` slot in
the flex column.
**Fix:** move the guard outward — `{%- if org.logo_data_uri %}<div class="logo">…</div>{%- endif %}`
— and consider hoisting the header CSS into each parent template's `<head>` (or a
`{% block head %}`) if strict validity matters for the print path.

### IN-03: No length bound on `org_settings.full_name` on either the UI or the backend

**Severity:** INFO
**File:** `ui/src/features/settings/OrgSettings.svelte:275-283`;
`crates/trackly-app/src/services/org_db_service.rs:87-123`
**Issue:** The new `Textarea` has no `maxlength` and `save_fields` performs no validation, so
an arbitrarily long value is stored and rendered into the header of every printed document
(and into the escaped-HTML helper on every render). Consistent with the other org fields,
which are equally unbounded — noting it because this one is multiline and feeds a
per-character `HtmlEscape` pass.
**Fix:** validate in `save_fields` (e.g. `full_name.chars().count() <= 512`) and mirror with
`maxlength` on the textarea.

### IN-04: CRLF input leaves a stray `\r` before the inserted `<br />`

**Severity:** INFO
**File:** `crates/trackly-app/src/pdf/minijinja_env.rs:37-39`
**Issue:** `.replace('\n', "<br />")` on `"a\r\nb"` yields `"a\r<br />b"`. Browsers collapse
the `\r` as whitespace so this is cosmetic, and the HTML textarea API normalizes to LF — but
the HTTP API accepts raw JSON, so CRLF can reach the column.
**Fix:** `raw.replace("\r\n", "\n").replace('\r', "\n")` before escaping, or
`.replace("\r\n", "<br />").replace('\n', "<br />")` after.

### IN-05: A read-only `templates/` directory hard-fails application startup

**Severity:** INFO
**File:** `crates/trackly-app/src/pdf/html_templates.rs:108-123`, `141-174`;
`crates/trackly-app/src/context.rs:218`, `:224`
**Issue:** Both `materialize_defaults_on_startup` and `upgrade_untouched_defaults_on_startup`
propagate `AppError::Internal` on `create_dir_all`/`write` failure, and `AppCtx::build`
uses `?`. In portable mode on a read-only share or a locked install directory the app refuses
to start, even though `load_template` would happily serve embedded defaults. Pre-existing
(Phase 16/20), surfaced again because Phase 34 adds a fourth file to materialize.
**Fix:** downgrade both to `tracing::warn!` + continue — rendering already degrades cleanly
to embedded defaults.

---

_Reviewed: 2026-08-09_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_

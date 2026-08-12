# Phase 36: Пагинация акта по количеству устройств - Pattern Map

**Mapped:** 2026-08-12
**Files analyzed:** 9
**Analogs found:** 9 / 9 (all in-tree; no analog gaps — this phase modifies/extends existing
mechanisms rather than introducing a new architectural shape)

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|--------------------|------|-----------|-----------------|----------------|
| `crates/trackly-app/templates/act_handover.html` | template (MiniJinja, server-rendered HTML) | transform (context → HTML string) | itself (prior HEAD, post-Phase-35) + `act_acceptance.html` for the table/dash idiom | exact (self) / role-match (table idiom) |
| `crates/trackly-app/templates/_legacy_defaults/v24/act_handover.html` | config (versioned snapshot) | file-I/O (byte snapshot, no logic) | `_legacy_defaults/v23/act_handover.html` (and v20-v22 siblings) | exact |
| `crates/trackly-app/src/pdf/html_templates.rs` | config/registry + test | CRUD-like (registry entry) + batch (startup upgrade scan) | its own `v23` entry + `upgrade_replaces_v23_legacy_default_with_current_bundled_body` test | exact |
| `ui/src/lib/pdfPreview/bootstrapScript.js` | client script (event-driven, runs inside srcdoc iframe) | event-driven (Paged.js lifecycle hooks + postMessage) | itself (prior HEAD) — only existing file of this exact role in the codebase | exact |
| `ui/src/features/acts/PdfPreviewModal.svelte` (`printViaTopLevel`) | component (Svelte, print orchestration) | event-driven (DOM injection + Paged.js `Previewer` + `window.print()`) | itself (prior HEAD, `printViaTopLevel` function) | exact |
| `crates/trackly-app/src/http/mod.rs` (CSP `script-src` sha256 constant) | config (security header) | request-response (HTTP response header, static string) | itself (prior HEAD, the existing `sha256-...` token) | exact |
| `crates/trackly-app/tests/html_act_render.rs` | test (integration, HTML-string assertions) | request-response (render → assert) | itself (prior HEAD) — `extract_first_ul` + the two `html_handover_*` tests it feeds | exact |
| `crates/trackly-app/tests/pdf_render_act.rs` | test (integration) | request-response | itself (prior HEAD) — `render_handover_multi_device_wraps_long_fields`, `render_handover_multi_device_fields_attributable_to_own_device` | exact |
| `crates/trackly-app/tests/acts_e2e_smoke.rs` | test (integration, e2e scenario) | request-response (scenario mid-point render assertion) | itself (prior HEAD) — `handover_pdf_render_within_e2e` (uses 2 seeded devices, so it exercises the new N>1 branch as of this phase) | exact |

## Pattern Assignments

### `crates/trackly-app/templates/act_handover.html` (template, transform)

**Analog:** itself (current HEAD) for the N=1 branch to preserve untouched; `act_acceptance.html`
for the new appendix table's cell idiom.

**Doc-comment header pattern to update (C-02)** (lines 1-41):
```html
<!DOCTYPE html>
{#- Default HTML template for Акт приёма-передачи (Phase 16, D-01/D-02/D-03;
  body reworked Phase 35, D-01..D-12).
  ...
  Context variables (same shape as act_service::render_pdf's ctx, D-04, ...):
    org.name, org.full_name, org.inn, ...
    act.items[].name, act.items[].inventory_no, act.items[].serial_no,
    act.items[].model, act.items[].specs, act.items[].kit,
    act.items[].condition, act.items[].quantity
  ...
-#}
```
The comment already lists `act.items[].quantity` in the context shape — Phase 36 must update the
prose (currently describes "per-item field_rows... no Устройство №N", i.e. a single-level flow)
to describe both branches and note `quantity` is now consumed (D-03), not just present.

**N=1 branch — keep byte-identical** (lines 133, 142-164; the `{%- for item in act.items %}
<div class="device-block">...` loop and everything inside it):
```jinja2
{%- for item in act.items %}
  <div class="device-block">
    <div class="field-row">было получено устройство: {{ item.name }}</div>
    {%- if item.inventory_no %}
    <div class="field-row">Инвентарный номер: {{ item.inventory_no }}</div>
    {%- endif %}
    {%- if item.serial_no %}
    <div class="field-row">Серийный номер: {{ item.serial_no }}</div>
    {%- endif %}
    {%- if item.model %}
    <div class="field-row">Модель: {{ item.model }}</div>
    {%- endif %}
    {%- if item.kit %}
    <div class="field-row">Комплектация: {{ item.kit }}</div>
    {%- endif %}
    {%- if item.specs %}
    <div class="field-row">Технические характеристики: {{ item.specs }}</div>
    {%- endif %}
    {%- if item.condition %}
    <div class="field-row">Состояние: {{ item.condition }}</div>
    {%- endif %}
  </div>
{%- endfor %}
```
Per D-08, this entire block must become conditional on `act.items | length == 1` — it must NOT
render at all when `act.items | length > 1` (currently it renders unconditionally alongside the
`<ul>` summary — that's the "today's double-render" bug the phase closes, per CONTEXT.md's
"Исходное состояние" note).

**Existing N>1 summary block to replace** (lines 133-140):
```jinja2
{%- if act.items | length > 1 %}
<div class="field-row">были получены устройства:</div>
<ul>
  {%- for item in act.items %}
  <li>{{ item.name }}</li>
  {%- endfor %}
</ul>
{%- endif %}
```
D-07 replaces `<ul>`/`<li>` with `<ol>`/`<li>` (numbers must align with the appendix table's `№`
column, via `loop.index`) plus a trailing referral line — see RESEARCH.md Pattern 2 for the
full sketch (`<ol class="device-summary">...<div class="field-row">Полное описание устройств — в
Приложении №1.</div>`).

**Dash-for-empty idiom to copy verbatim (D-02)** — from `act_acceptance.html:124-127`:
```jinja2
<tr><td class="key">Инв.№</td><td>{{ device.inventory_no | default("—", true) }}</td></tr>
<tr><td class="key">Серийный №</td><td>{{ device.serial_no | default("—", true) }}</td></tr>
<tr><td class="key">Модель</td><td>{{ device.model | default("—", true) }}</td></tr>
<tr><td class="key">Состояние</td><td>{{ device.condition | default("—", true) }}</td></tr>
```
Apply the exact same `| default("—", true)` filter chain to the new appendix table's
`item.inventory_no` / `item.serial_no` / `item.model` / `item.condition` cells.

**`page-break-inside: avoid` idiom to copy for D-15's `<tbody>`-per-device grouping** — from
`act_handover.html`'s existing CSS (lines 81-84, already proven working for `.device-block`):
```css
.device-block {
  page-break-inside: avoid;
  margin-bottom: 8pt;
}
```
RESEARCH.md Pattern 4 (verified against `ui/node_modules/pagedjs/src/chunker/layout.js:582-589`)
establishes this exact property must be set on a `<tbody>` wrapping each device's 1-2 `<tr>`s, not
on a bare `<tr>` — the TBODY/THEAD-only special case is why this specific selector shape is
required; use `break-inside: avoid;` (unprefixed, standard CSS Fragmentation property, matching
`.signatures`'s existing usage below).

**Forced-break idiom (D-16)** — no existing `break-before` in this file yet; introduce it fresh on
the new `.appendix` wrapper:
```css
.appendix {
  break-before: page;
}
```

**`@page` block — DO NOT TOUCH** (lines 47-50):
```css
@page {
  size: A4 portrait;
  margin: 20mm 15mm;
}
```
`html_page_parity.rs` requires this block byte-identical across all three templates (C-06);
verify no plan step edits it.

---

### `crates/trackly-app/templates/_legacy_defaults/v24/act_handover.html` (config, file-I/O)

**Analog:** `_legacy_defaults/v23/act_handover.html` (192 lines) — and its 3 older siblings
(v20, v21, v22), all sitting in
`crates/trackly-app/templates/_legacy_defaults/{v20,v21,v22,v23}/`.

**Pattern:** an exact byte-for-byte copy of `act_handover.html` as it stands at the START of this
phase (current HEAD, i.e. the post-Phase-35 body already read above), copied into a brand-new
`_legacy_defaults/v24/` sibling directory BEFORE any pagination edit lands (Pitfall 7 — snapshot
timing is the trap). No transformation of any kind — this is a pure copy, mirroring exactly how
v20→v21→v22→v23 were each created at their respective phase boundaries. `act_acceptance.html` is
untouched this phase (per CONTEXT.md domain note) — do NOT create a `v24/act_acceptance.html`
slice; C-01 explicitly says it is not needed.

---

### `crates/trackly-app/src/pdf/html_templates.rs` (config/registry + test)

**Analog:** the file's own `v23` entry and `upgrade_replaces_v23_legacy_default_with_current_bundled_body` test (this file, lines 75-93 and 536-585 respectively).

**Registry entry pattern to extend** (lines 75-84):
```rust
pub const KNOWN_LEGACY_DEFAULTS: &[(&str, &[&str])] = &[
    (
        "act_handover.html",
        &[
            include_str!("../../templates/_legacy_defaults/v20/act_handover.html"),
            include_str!("../../templates/_legacy_defaults/v21/act_handover.html"),
            include_str!("../../templates/_legacy_defaults/v22/act_handover.html"),
            include_str!("../../templates/_legacy_defaults/v23/act_handover.html"),
            include_str!("../../templates/_legacy_defaults/v24/act_handover.html"), // NEW — add here
        ],
    ),
    // act_acceptance.html entry: DO NOT touch — no v24 slice for it this phase
    ...
];
```

**Test pattern to duplicate for v24** (lines 536-585, `upgrade_replaces_v23_legacy_default_with_current_bundled_body`):
```rust
#[test]
fn upgrade_replaces_v24_legacy_default_with_current_bundled_body() {
    let _guard = ENV_GUARD.lock().unwrap();
    let dir = tempfile::tempdir().expect("tempdir");

    for (filename, current) in DEFAULT_HTML_TEMPLATES.iter() {
        let Some(bodies) = KNOWN_LEGACY_DEFAULTS
            .iter()
            .find(|(name, _)| name == filename)
            .map(|(_, bodies)| *bodies)
        else {
            continue; // e.g. _header.html — no legacy slice registered
        };
        let Some(v24_body) = bodies.get(4) else {
            continue; // filename has no v24 element (e.g. act_acceptance.html, report.html)
        };

        // Precondition guard (Pitfall 7): the v24 snapshot must NOT equal the
        // current bundled body, or this test cannot prove a real upgrade happened.
        assert_ne!(
            v24_body, current,
            "{filename}: v24 legacy snapshot must NOT equal the current bundled \
             default — otherwise the snapshot was taken after the pagination rewrite"
        );

        std::fs::write(dir.path().join(filename), v24_body).expect("write v24 body");
    }

    upgrade_untouched_defaults_on_startup(dir.path()).expect("upgrade ok");

    for (filename, current) in DEFAULT_HTML_TEMPLATES.iter() {
        let has_v24 = KNOWN_LEGACY_DEFAULTS
            .iter()
            .find(|(name, _)| name == filename)
            .map(|(_, bodies)| bodies.len() > 4)
            .unwrap_or(false);
        if !has_v24 {
            continue;
        }
        let contents = std::fs::read_to_string(dir.path().join(filename)).expect("file exists");
        assert_eq!(
            &contents, current,
            "{filename} must be upgraded from its v24 legacy body to the current bundled body"
        );
    }
}
```
Note: index `4` (0-based: v20=0, v21=1, v22=2, v23=3, v24=4) — matches the established
index-per-version convention exactly.

---

### `ui/src/lib/pdfPreview/bootstrapScript.js` (client script, event-driven)

**Analog:** itself (current HEAD, 61 lines) — sole file of this role/shape in the codebase; no
other file registers a Paged.js `Handler` yet.

**File-header constraint to preserve** (lines 1-9) — MUST remain a single static string, zero
per-call interpolation, because its exact bytes are hashed into the CSP `script-src` allow-list:
```js
// This file is a PLAIN standalone browser script (no ES module syntax) because
// its exact text is concatenated raw into an inline <script> tag inside the
// preview iframe's `srcdoc` (see pagedPreviewBootstrap.ts). It must remain a
// single static string with zero per-call interpolation: Plan 33-02 hardcodes
// a SHA-256 hash of this text (combined with the Paged.js library text) into
// the LAN-mode CSP `script-src` allow-list. Any edit to this file's bytes
// requires regenerating that hash — do not templatize it.
```

**Existing `window.PagedModule` access pattern to extend, not replace** (lines 19-34):
```js
(function () {
  var previewer = new window.PagedModule.Previewer();
  var pages = 0;

  previewer.chunker.on('renderedPage', function () {
    pages += 1;
    parent.postMessage({ type: 'trackly-pagedjs-progress', pages: pages }, '*');
  });
  ...
```
Per RESEARCH.md Pitfall 1, the new thead-repeat `Handler` must be registered via
`window.PagedModule.Handler` / `window.PagedModule.registerHandlers` (both confirmed present on
the bundled UMD global by direct grep of `paged.min.js`) BEFORE `new window.PagedModule.Previewer()`
is constructed — mirror the existing single-IIFE structure, do not introduce a second `<script>`
tag or split the file (would break the CSP hash formula, which concatenates library text + this
file's exact bytes).

**postMessage protocol comment to keep in sync** (lines 11-18) — if the handler adds any new
diagnostic messaging, follow the same four-message-type convention already documented there
(`trackly-pagedjs-progress` / `-done` / `-error` / `trackly-theme-update`) rather than inventing a
fifth ad hoc channel.

---

### `ui/src/features/acts/PdfPreviewModal.svelte` — `printViaTopLevel` (component, event-driven)

**Analog:** itself (current HEAD) — the function at lines 355-472.

**Separate ESM Paged.js entry point to mirror the bootstrapScript.js handler into** (lines
464-471):
```ts
const { Previewer } = await import('pagedjs');
const previewer = new Previewer();
await previewer.preview(bodyHtml, [{ 'act-preview.css': cssText }], printRoot);
injectedPolisher = previewer.polisher;

window.focus();
window.print();
```
Per RESEARCH.md's System Architecture Diagram and Pitfall 1/Anti-Pattern 3, this is the SECOND of
the two places the thead-repeat `Handler` must be registered — it does NOT go through
`bootstrapScript.js` at all (separate dynamic `import('pagedjs')`, ESM not UMD). Unlike
`bootstrapScript.js`, this file CAN import from a shared `.ts` module (it already imports other
`ui/src/lib/pdfPreview/*` helpers elsewhere in the file) — RESEARCH.md Open Question 2 leaves
"shared module vs. hand-duplication with a cross-referencing comment" to planner discretion, but
either way both places must register logically-identical `Handler` behavior, or LAN print
visibly diverges from desktop print/preview (violates Success Criteria #3/#4).

**Comment convention to extend** (lines 338-354, the function's own doc-comment) — follow the
existing style of citing the originating phase/decision inline (`Phase 33 (D-06/C-03): ...`);
Phase 36's addition should cite `D-15a` the same way.

---

### `crates/trackly-app/src/http/mod.rs` (config, request-response header)

**Analog:** itself (current HEAD) — the CSP `script-src` sha256 token at line ~219.

**Pattern — mechanical regeneration, not hand-editing the digest** (lines 206-219):
```rust
// PRV-CSP (Phase 33, D-14): sha256 hash-source for the Paged.js preview
// ...
// regenerated (node ui/scripts/check-pagedjs-csp-hash.mjs --print) and this
// constant updated whenever bootstrapScript.js or the pinned pagedjs version
// changes — drift is caught by `pnpm lint` (check-pagedjs-csp-hash.mjs) and by
...
"default-src 'self'; script-src 'self' 'sha256-1nG6ajqUxHpGqTH1xMQEfH1DAoyP3C8xrIMr3PNVhPQ='; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self' wss:; frame-src 'self' blob:; object-src 'self' blob:",
```
After `bootstrapScript.js` is edited, run (from `ui/`):
```
node scripts/check-pagedjs-csp-hash.mjs --print
```
and paste the printed `sha256-...` value in place of the current token (the surrounding
`default-src ...; script-src 'self' '<TOKEN>'; ...` string structure is otherwise unchanged) —
this is Pitfall 2 in RESEARCH.md, a required follow-up whenever `bootstrapScript.js`'s bytes
change, not optional polish.

---

### `crates/trackly-app/tests/html_act_render.rs` (test, request-response)

**Analog:** itself (current HEAD) — `extract_first_ul` (lines 191-201) and the two tests it feeds
(`html_handover_single_device_renders_singular_intro_not_plural_summary`,
`html_handover_multi_device_renders_plural_summary_listing_every_name`, lines 210-296+).

**Helper to rewrite (not just rename)** (lines 189-201):
```rust
/// Returns the substring of `html` strictly between the first `<ul>` and the
/// following `</ul>`, panicking if either marker is missing.
fn extract_first_ul(html: &str) -> &str {
    let start = html
        .find("<ul>")
        .expect("rendered HTML must contain a <ul>")
        + "<ul>".len();
    let end = html[start..]
        .find("</ul>")
        .map(|i| start + i)
        .expect("the <ul> must be closed");
    &html[start..end]
}
```
D-07 replaces `<ul>`/`<li>` with `<ol>`/`<li>` for the N>1 summary — this helper must become
`extract_first_ol` (or similar), searching for `<ol` (attribute-bearing, e.g.
`<ol class="device-summary">`) rather than a bare `<ul>` literal, mirroring the exact
find/expect/map structure.

**N=1 negative-assertion pattern to preserve, retarget the literal** (lines 236-240):
```rust
assert!(
    !html.contains("<ul>"),
    "N=1 must NOT render the plural device-name <ul> summary list (D-02). Body: {:?}",
    html.chars().take(2000).collect::<String>()
);
```
Retarget to whatever exact opening tag the new `<ol class="device-summary">` uses, and ALSO assert
N=1 renders no `.appendix`/`appendix-table` markup at all (new assertion, not in the current file)
— since N=1's "no second sheet" behavior is this phase's core DOC-10 guarantee.

**N>1 per-`<li>` assertion pattern to keep, with `<ol>` swapped in and a new appendix
cross-check appended** (lines 263-292, the existing loop over `device_ids`/`expected` `<li>`
strings) — the structural shape (`format!("<li>{name}</li>")`, `.matches("<li>").count() == 3`)
is directly reusable; add a companion loop asserting the appendix table's `№` column values equal
`loop.index` (D-07's cross-reference requirement) and that `.device-block` no longer appears at
all in the N>1 render (D-08 — the CURRENT test at line ~286-292 asserts the OPPOSITE, that
`.device-block`'s singular label appears 3 times; that specific assertion must be DELETED, not
merely edited, per RESEARCH.md Pitfall 6).

---

### `crates/trackly-app/tests/pdf_render_act.rs` (test, request-response)

**Analog:** itself (current HEAD) — `render_handover_multi_device_wraps_long_fields` (lines
278-341) and `render_handover_multi_device_fields_attributable_to_own_device` (lines 354-460+).

**Long-field/no-truncation assertion pattern — content shape reusable, location moves** (lines
326-337):
```rust
assert!(
    !html.contains('…'),
    "long complectation field must wrap, not truncate with ellipsis. Head: {:?}",
    html.chars().take(800).collect::<String>()
);

assert!(
    html.contains("СЕРЕДИНА-МАРКЕР-ЗНАЧЕНИЯ"),
    "middle-of-value marker missing — long field appears to have been cut off. \
     Head: {:?}",
    html.chars().take(800).collect::<String>()
);
```
This assertion pair (no `…`, middle-of-value marker survives) stays valid verbatim — but the long
`complectation_at_time` value now lands in the appendix table's D-01 sub-row (`colspan="7"`), not
inside a `.device-block`'s field-row. No change to the assertion logic itself, only to what the
surrounding fixture/HTML shape looks like — a good candidate for "keep the assertions, update the
comment describing WHERE the value now renders."

**`.device-block`-splitting pattern — must be REPLACED, not patched** (lines 354, 400-455):
```rust
let parts: Vec<&str> = html.split("<div class=\"device-block\">").collect();
assert_eq!(
    parts.len(), 4,
    "expected 1 preamble part + 3 device-block parts (N=3 items). Head: {:?}",
    ...
);
```
Per D-08, N>1 (this test uses 3 items) no longer emits ANY `.device-block` on the first sheet —
this split-and-count structure will find only 1 part (the whole document) and fail loudly. The
underlying INTENT (D-02a's "each device's own optional fields must be attributable to that exact
device, not bleed into a neighbor") is still valid and must be re-proven — but against the new
appendix table structure: split on the appendix's per-device `<tbody class="device-group">`
marker instead of `.device-block`, and assert each `<tbody>` contains its own device's fields and
NOT a sibling device's fields, mirroring the same per-index "own field present / other devices'
absent" assertion shape already used at lines 413-455.

---

### `crates/trackly-app/tests/acts_e2e_smoke.rs` (test, request-response — e2e scenario)

**Analog:** itself (current HEAD) — `handover_pdf_render_within_e2e` (line 259+), which calls
`seed_devices(&p.writer, 2)` (line 262).

**Pattern — no structural change needed to the call itself, only awareness that its render now
exercises the NEW branch:**
```rust
let ids = seed_devices(&p.writer, 2).await;
```
2 devices means `act.items | length == 2 > 1` — after this phase lands, this act's rendered HTML
will contain the new `<ol>`/appendix-table shape instead of two `.device-block`s. The test's
existing assertions (Cyrillic content present, `html.len() > 1000`, per the file's own doc-comment
at lines 10-11) are almost certainly still satisfiable by the new shape, but MUST be re-run as
part of `files_modified`, not assumed to pass unmodified — this is exactly the "expected drift,
not a regression to chase" class flagged in RESEARCH.md Pitfall 6's last bullet.

---

## Shared Patterns

### Legacy-defaults version-bump ritual (C-01/Pitfall 7)
**Source:** `crates/trackly-app/src/pdf/html_templates.rs` lines 46-93, 536-585 + the
`_legacy_defaults/v20`../`v23` directory precedent.
**Apply to:** `_legacy_defaults/v24/act_handover.html` (new file) + `html_templates.rs`'s
`KNOWN_LEGACY_DEFAULTS` registry entry + new `upgrade_replaces_v24_...` test.
**Rule:** snapshot BEFORE editing the live template; register in the array; add the index-N test
with an `assert_ne!` precondition guard. Order matters — this is the #1 documented trap in this
exact mechanism (hit in Phases 34, 35, and quick `260704-uw3`).

### `| default("—", true)` dash-for-empty idiom (D-02)
**Source:** `crates/trackly-app/templates/act_acceptance.html` lines 124-127.
**Apply to:** every appendix-table cell in `act_handover.html` sourced from an optional
`item.*` field (`inventory_no`, `serial_no`, `model`, `condition`).

### `page-break-inside`/`break-inside: avoid` keep-together idiom
**Source:** `crates/trackly-app/templates/act_handover.html` lines 81-84 (`.device-block`) and
88-91 (`.signatures`), corroborated against
`ui/node_modules/pagedjs/src/chunker/layout.js:582-589` (TBODY/THEAD-only special case).
**Apply to:** the new per-device `<tbody class="device-group">` wrapper in the appendix table —
NOT a bare `<tr>` (RESEARCH.md Pattern 4 explains why the `<tr>`-level property is silently
ignored by Paged.js's overflow-finder).

### Dual-transport parity for anything touching Paged.js lifecycle hooks (D-15a)
**Source:** `ui/src/lib/pdfPreview/bootstrapScript.js` (UMD, shared by desktop print + on-screen
preview) vs. `ui/src/features/acts/PdfPreviewModal.svelte`'s `printViaTopLevel` (separate ESM
`import('pagedjs')`, LAN print only).
**Apply to:** the thead-repeat `Handler` — must be added to BOTH files with logically identical
behavior, or LAN print silently diverges from desktop/preview (breaks Success Criteria #3/#4).
This is the single highest-risk shared pattern in the phase per RESEARCH.md's Open Question 1/2.

### CSP sha256 regeneration after any `bootstrapScript.js` byte change (Pitfall 2)
**Source:** `ui/scripts/check-pagedjs-csp-hash.mjs` (hash formula) +
`crates/trackly-app/src/http/mod.rs` line ~219 (hardcoded token).
**Apply to:** any plan task that edits `bootstrapScript.js` — the LAST step of that task must run
`node scripts/check-pagedjs-csp-hash.mjs --print` (from `ui/`) and paste the result into
`http/mod.rs`. Silent failure mode if skipped: LAN pagination breaks, desktop keeps working,
`pnpm lint` is the only thing that catches it (not `cargo test`).

### `html_page_parity.rs` — must stay green untouched
**Source:** `crates/trackly-app/tests/html_page_parity.rs` lines 1-45 — extracts the first
`@page { ... }` block via regex from all three templates and asserts byte-identity.
**Apply to:** any plan task touching `act_handover.html`'s `<style>` block — a reviewer/planner
checklist item, not a file to modify. No `@page`-scoped counters, no second named `@page`, no
margin-box changes anywhere in `act_handover.html`.

## No Analog Found

None. Every file in scope either already exists (test files, `html_templates.rs`, `http/mod.rs`,
`bootstrapScript.js`, `PdfPreviewModal.svelte`) or has a direct structural sibling in the same
directory (`_legacy_defaults/v2N/`) to copy from.

## Metadata

**Analog search scope:** `crates/trackly-app/templates/`, `crates/trackly-app/templates/_legacy_defaults/`,
`crates/trackly-app/src/pdf/`, `crates/trackly-app/src/http/`, `crates/trackly-app/tests/`,
`ui/src/lib/pdfPreview/`, `ui/src/features/acts/`, `ui/scripts/`,
`ui/node_modules/pagedjs/src/chunker/` (read-only, for verifying Paged.js's actual hook/behavior
surface, not as a codebase pattern source).
**Files scanned:** ~14 (all files listed in File Classification plus their direct analog
siblings: `act_acceptance.html`, `_legacy_defaults/v20-23/act_handover.html`,
`check-pagedjs-csp-hash.mjs`, `html_page_parity.rs`).
**Pattern extraction date:** 2026-08-12

# Phase 36: Пагинация акта по количеству устройств - Research

**Researched:** 2026-08-12
**Domain:** HTML/CSS print pagination (Paged.js 0.4.3) + MiniJinja templating, single-file scope
**Confidence:** MEDIUM (HIGH on codebase facts/mechanisms — verified by direct source read; MEDIUM-LOW
specifically on "repeating `<thead>` across pages" — this is NOT a native Paged.js 0.4.3 feature,
see Pitfall 1)

## Summary

Phase 36 is a single-template change (`act_handover.html`) that branches printed output on
`act.items | length`. All sixteen implementation decisions are already locked in
`36-CONTEXT.md` — this research does not re-litigate them. Its job is to answer *how* to
implement three technically non-trivial pieces the context correctly flags as needing outside
verification: (1) whether Paged.js (bundled version **0.4.3**, confirmed via
`ui/package.json`/`pnpm-lock.yaml`) actually supports the CSS fragmentation behaviors D-15/D-16
assume, (2) how `print-color-adjust: exact` behaves for table-cell zebra backgrounds on both
transports, and (3) the exact legacy-defaults/test-drift mechanics for this specific edit.

The single most important finding: **repeating `<thead>` across pages is NOT a native feature of
the bundled Paged.js 0.4.3.** A direct read of `ui/node_modules/pagedjs/src/chunker/layout.js`
and `chunker.js` shows no thead-cloning logic; this is corroborated by multiple long-open
upstream issues and an unmerged PR (#160) still open on the pagedjs GitHub as of this research.
D-15 ("thead повторяется … Paged.js это поддерживает") is not wrong about the *goal* Paged.js can
achieve, but the mechanism is not automatic — it requires a small custom Paged.js `Handler`
registered against the `afterPageLayout` hook (verified present in the bundled chunker), and that
handler must be added to **two separate code paths** (the shared UMD bootstrap used by
desktop-print + on-screen preview, and the LAN print branch's own dynamic ESM import), or the two
transports will visibly diverge — breaking the phase's own WYSIWYG requirement.

Everything else in scope is comfortably supported: `break-inside: avoid` on a `<tbody>` wrapping
one device's two `<tr>` rows is a verified-working mechanism (Paged.js's overflow-finder
explicitly special-cases `TBODY`/`THEAD` `break-inside` styles), `break-before: page` is standard
CSS Fragmentation the polyfill implements, and MiniJinja (`^2.20`, already in
`crates/trackly-app/Cargo.toml`) supports everything the appendix table needs
(`| length`, `loop.index`, `| default("—", true)`, nested `{% if %}`) with no new dependency.

**Primary recommendation:** Implement the appendix table with one `<tbody break-inside:avoid>`
per device (main row + optional colspan sub-row), add a small custom Paged.js `Handler` for
thead-repeat mirrored identically into `bootstrapScript.js` and `PdfPreviewModal.svelte`'s
`printViaTopLevel`, and budget an explicit spike/verification task for the thead-repeat handler
early in the plan — it is the one piece of this phase with real technical risk.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| N=1 vs N>1 branching, appendix table markup | Backend template (MiniJinja, `act_handover.html`) | — | Pure server-side templating; `act.items \| length` is already in the render context, no Rust change needed (per CONTEXT.md domain note) |
| Page-break placement (`break-before: page`, `break-inside: avoid`) | Browser / Client (Paged.js polyfill) | — | Paged.js runs entirely client-side, in both the on-screen `<iframe srcdoc>` preview and the two print branches; the backend never computes page geometry |
| Repeating table header across pages | Browser / Client (custom Paged.js Handler) | Frontend build (`ui/src/lib/pdfPreview/bootstrapScript.js`, `PdfPreviewModal.svelte`) | Not solvable in the HTML template alone — must run inside the Paged.js chunker's lifecycle hooks, on the client |
| Zebra background survives print | Browser / Client (CSS `print-color-adjust`) | — | Print-time background suppression is a browser default the template's CSS must explicitly override; no backend involvement |
| Legacy-defaults upgrade delivery to installed copies | Backend (`html_templates.rs` `KNOWN_LEGACY_DEFAULTS`) | — | Pure Rust constant-table mechanism, already established (v20–v23 precedent) |
| CSP allow-list for the inline Paged.js bootstrap (only if the handler is added to bootstrapScript.js) | Frontend Server (LAN axum) | Backend (`crates/trackly-app/src/http/mod.rs`) | The hash is computed at build/lint time from `bootstrapScript.js` bytes and hardcoded into the LAN server's CSP header |

## User Constraints

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Форма «Приложения №1» (DOC-11)**
- D-01: Two-level row per device — narrow columns `№ / Наименование / Кол-во / Инв.№ /
  Серийный № / Модель / Состояние`, plus a full-width `colspan` sub-row for
  Комплектация/Тех. характеристики (only when there is something to print). Portrait only, `@page`
  unchanged.
- D-02: Empty value → em dash "—" (same `| default("—", true)` idiom as `act_acceptance.html`).
- D-03: "Кол-во" column always present, value only printed when `quantity > 1`, else "—".
- D-04: Zebra striping (no grid lines), `print-color-adjust: exact` (+ `-webkit-` prefix)
  mandatory, table font size smaller than body (~10pt vs 12pt). No full `border: 1px solid #000`
  grid.
- D-05: Thin light-gray hairline between devices and under the table header, as a fallback if the
  zebra background doesn't print.

**Первый лист при N > 1 (DOC-11)**
- D-06: Signature block prints ONLY on the first sheet; the appendix is a pure table, no
  signatures.
- D-07: Summary list is a numbered `<ol>`, numbers matching the appendix table's "№" column;
  followed by a line referring to "Приложение №1".
- D-08: Phase 35's D-02a per-device singular label ("было получено устройство: ⟨item.name⟩")
  remains ONLY at N=1; at N>1 there are no `.device-block`s on the first sheet at all — the
  appendix table row provides attribution instead.
- D-09: First-sheet tail ("Сроком до", optional parent-act reference) unchanged, in current
  order. No "Всего устройств: N" summary line anywhere.

**Заголовок и связка приложения (DOC-11)**
- D-10: Appendix-sheet mark in the top-right corner, two lines, ~10pt: "Приложение №1" /
  "к акту приема-передачи №{{ act.number }}{{ act.suffix }} от {{ act.date_human }}". Centered
  content heading e.g. "Опись передаваемых устройств".
- D-11: Organization header (`_header.html`) is NOT repeated on the appendix sheet — included
  only on the first sheet.
- D-12: No "Лист N из M" pagination — no `@page` counter-based footer (would break
  `html_page_parity.rs`'s byte-identity gate), no static "Лист 2" text either.

**Пороги и поведение при переливе (DOC-10/DOC-11)**
- D-13: Threshold is strictly `N > 1`. Two devices already trigger the appendix.
- D-14: A single-device act that doesn't fit one sheet flows onto a second sheet naturally
  (Paged.js handles this); DOC-10 means "typical device fits", not "guaranteed one sheet under
  any content volume". No truncation with ellipsis, no forced conversion to appendix form.
- D-15: Long appendix table: `<thead>` repeats on every sheet (Paged.js support — **see Pitfall 1,
  this is the one locked decision whose underlying mechanism required correction during
  research**); the two rows of one device (main + sub-row) stay together via `break-inside: avoid`
  on the group. Appendix mark (D-10) prints only on the appendix's first sheet, never
  "(продолжение)" on subsequent sheets (would need a `@page` counter — same conflict as D-12).
- D-16: Break before the appendix is ALWAYS forced (`break-before: page` on the appendix block) —
  never "if there's room".

### Claude's Discretion
- Exact values: zebra shade, hairline thickness/color, table/mark font size, column width
  percentages, exact wording of the appendix heading and the first-sheet referral line — within
  D-01/D-04/D-05/D-07/D-10.
- Mechanism for "two rows of one device stay together": separate `<tbody>` per device vs.
  `break-inside: avoid` on the `<tr>`s themselves — **Research recommendation: use a `<tbody>` per
  device group with `break-inside: avoid` on the `<tbody>`. This is the mechanism Paged.js's
  overflow-finder explicitly special-cases (verified in `layout.js`); `break-inside: avoid` on a
  bare `<tr>` is not covered by that same special-cased code path.**

### Deferred Ideas (OUT OF SCOPE)
- "Всего устройств: N" summary line — considered and rejected in this phase (D-09). Revisit as a
  standalone quick if lost appendix sheets become a real-world problem.
- "Лист N из M" pagination across all three printed forms — requires rewriting
  `html_page_parity.rs`'s byte-identity gate; separate phase/quick (D-12).
- Landscape orientation for wide printed forms — rejected as risky in combination with a second
  named `@page` + Paged.js + the parity gate (D-01). Revisit only on real need for wide reports.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| DOC-10 | Акт на одно устройство умещается на одном листе вместе с полным описанием этого устройства | N=1 branch is unchanged from current (post-Phase-35) `act_handover.html` — confirmed by reading the current template; no new pagination logic touches this path. `page-break-inside: avoid` already on `.device-block`/`.signatures` (Pattern already proven in production). |
| DOC-11 | Акт на несколько устройств: первый лист — только перечень + отсылка к «Приложению №1»; со второго листа — «Приложение №1» с полной таблицей | Requires: (a) moving `.device-block` loop out of the first sheet at N>1 and replacing with `<ol>` (D-07), (b) new appendix `<table>` after a `break-before: page` block, (c) thead-repeat mechanism (Pitfall 1 — the one piece needing a custom Paged.js Handler), (d) `<tbody>`-per-device grouping for D-15's "keep together" requirement (verified mechanism, see Architecture Patterns) |
</phase_requirements>

## Standard Stack

### Core
No new libraries. This phase is a template + small client-side script change on top of an
already-installed stack.

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `pagedjs` | `0.4.3` [VERIFIED: codebase — `ui/package.json:25`, `ui/pnpm-lock.yaml:1191`] | CSS Paged Media polyfill; already the sole pagination engine (Phase 33) | Locked by Phase 33 D-04/D-06; this phase must work within its actual (not assumed) capabilities |
| `minijinja` | `^2.20` [VERIFIED: codebase — `crates/trackly-app/Cargo.toml:53`], features `builtins, json, fuel, serde, multi_template` | HTML template rendering | Already the sole template engine for all three printed forms (Phase 16) |

### Supporting
None — no supporting libraries are introduced by this phase.

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Custom Paged.js `Handler` for thead-repeat | Accept non-repeating thead (only first appendix sheet gets a header row) | Simpler, zero CSP-hash churn, but directly violates locked decision D-15 — would need to go back to the user, not a planner-level substitution |
| Custom Paged.js `Handler` for thead-repeat | Upgrade `pagedjs` to a newer/beta version that might merge PR #160 | Unverified whether any released version has merged this (PR #160 was still open at research time); a version bump is also its own regression-risk surface across all three templates and out of this phase's stated boundary (Phase 33 owns the print mechanism, not Phase 36) |

**Installation:** none — no `npm install` / `cargo add` needed. This phase edits
`crates/trackly-app/templates/act_handover.html`, `ui/src/lib/pdfPreview/bootstrapScript.js`, and
`ui/src/features/acts/PdfPreviewModal.svelte` only (plus `crates/trackly-app/src/http/mod.rs` for
the CSP hash constant, if the bootstrap script changes — see Pitfall 2).

**Version verification:** confirmed via `grep` against `ui/package.json`/`pnpm-lock.yaml`
(pagedjs 0.4.3) and `crates/trackly-app/Cargo.toml` (minijinja `^2.20`) — both already pinned,
no action needed this phase.

## Package Legitimacy Audit

Not applicable — this phase installs no new packages (Node.js or Rust). No `slopcheck`/registry
verification is required. If a future planning pass decides a `pagedjs` version bump is needed to
chase native thead-repeat support, that decision must re-run this gate against the new pinned
version.

## Architecture Patterns

### System Architecture Diagram

```
                         act_service::render_pdf (Rust, UNCHANGED)
                                     │
                    items_json (already has quantity, 8 fields)
                                     ▼
              ┌──────────────────────────────────────────┐
              │  act_handover.html (MiniJinja, THIS PHASE) │
              │  {% if act.items | length > 1 %}           │
              │    branch A: N=1 — unchanged body           │
              │    branch B: N>1 — <ol> summary +           │
              │      break-before:page + appendix <table>  │
              └──────────────────────────────────────────┘
                                     │  HTML string
                     ┌───────────────┴────────────────┐
                     ▼                                 ▼
        Desktop (Tauri, isTauri)              LAN (browser, !isTauri)
                     │                                 │
     printViaSystemBrowser: writes a self-contained     printViaTopLevel: injects
     temp .html (PAGED_PREVIEW_INLINE_SCRIPT =           bodyHtml/cssText into
     paged.min.js + bootstrapScript.js, UMD),            #act-print-root, then
     opened via tauri-plugin-shell in the default        `import('pagedjs')` (ESM) +
     browser/WebView2                                    `new Previewer().preview(...)`
                     │                                 │
                     └──────────────┬──────────────────┘
                                     ▼
                   Paged.js Previewer/Chunker paginates the DOM
                   (break-before/break-inside honored — verified;
                    thead repeat NOT native — needs custom Handler,
                    must be added to BOTH branches identically)
                                     ▼
                        window.print() / saved PDF
```

The on-screen preview (`<iframe srcdoc>` in `PdfPreviewModal.svelte`) uses the same
`PAGED_PREVIEW_INLINE_SCRIPT` (UMD bundle) as the desktop print branch — so a fix/handler placed
in `bootstrapScript.js` automatically covers preview + desktop print. The **LAN print branch is
the odd one out**: it does not go through `bootstrapScript.js` at all — it dynamically
`import('pagedjs')`s the ESM build directly inside `printViaTopLevel()` and constructs its own
`Previewer`. Any change that must apply identically to all rendering surfaces (like a thead-repeat
handler) needs to be added in **two places**, not one.

### Recommended Project Structure
No new files/folders. Existing layout, phase touches:
```
crates/trackly-app/templates/
├── act_handover.html                     # the only template body edited
├── _legacy_defaults/v24/act_handover.html # NEW snapshot (C-01) — pre-phase-36 body
├── _header.html                          # read-only reference, NOT included on appendix sheet
└── act_acceptance.html                   # read-only reference for table.kv/dash idiom, NOT edited

crates/trackly-app/src/pdf/html_templates.rs  # add v24 entries to KNOWN_LEGACY_DEFAULTS
crates/trackly-app/src/http/mod.rs            # CSP sha256 constant, only if bootstrapScript.js changes

ui/src/lib/pdfPreview/bootstrapScript.js       # thead-repeat Handler, if implemented here
ui/src/features/acts/PdfPreviewModal.svelte    # thead-repeat Handler mirror for printViaTopLevel
```

### Pattern 1: N=1 branch — untouched pass-through
**What:** Keep the exact current single-device flow (`.device-block` loop, unconditional label)
byte-for-byte for `act.items | length == 1`.
**When to use:** Always at N=1 — this satisfies DOC-10/Success Criterion #1 by construction; no
new code path is exercised.
**Example:**
```jinja2
{# Source: crates/trackly-app/templates/act_handover.html:133-164 (current HEAD, N=1 path unchanged) #}
{%- if act.items | length > 1 %}
  {# ... N>1 branch, see Pattern 2 ... #}
{%- else %}
  {%- for item in act.items %}
  <div class="device-block">
    <div class="field-row">было получено устройство: {{ item.name }}</div>
    {# ...existing optional field-rows unchanged... #}
  </div>
  {%- endfor %}
{%- endif %}
```

### Pattern 2: N>1 branch — summary list + forced-break appendix
**What:** Replace the per-device `.device-block` loop with a numbered summary + a hard page break
into an appendix `<table>`.
**When to use:** `act.items | length > 1` (D-13's exact threshold).
**Example:**
```jinja2
{%- if act.items | length > 1 %}
<ol class="device-summary">
  {%- for item in act.items %}
  <li>{{ item.name }}</li>
  {%- endfor %}
</ol>
<div class="field-row">Полное описание устройств — в Приложении №1.</div>
{%- endif %}
{# ... "Сроком до" / parent-act / signatures unchanged, still first sheet (D-06/D-09) ... #}

{%- if act.items | length > 1 %}
<div class="appendix">
  <div class="appendix-mark">Приложение №1<br>к акту приема-передачи №{{ act.number }}{{ act.suffix }} от {{ act.date_human }}</div>
  <div class="appendix-title">Опись передаваемых устройств</div>
  <table class="appendix-table">
    <thead>
      <tr>
        <th>№</th><th>Наименование</th><th>Кол-во</th><th>Инв.№</th>
        <th>Серийный №</th><th>Модель</th><th>Состояние</th>
      </tr>
    </thead>
    {%- for item in act.items %}
    <tbody class="device-group">
      <tr class="{{ loop.cycle('row-even', 'row-odd') }}">
        <td>{{ loop.index }}</td>
        <td>{{ item.name }}</td>
        <td>{% if item.quantity > 1 %}{{ item.quantity }}{% else %}—{% endif %}</td>
        <td>{{ item.inventory_no | default("—", true) }}</td>
        <td>{{ item.serial_no | default("—", true) }}</td>
        <td>{{ item.model | default("—", true) }}</td>
        <td>{{ item.condition | default("—", true) }}</td>
      </tr>
      {%- if item.kit or item.specs %}
      <tr class="{{ loop.cycle('row-even', 'row-odd') }} device-subrow">
        <td colspan="7">
          {%- if item.kit %}Комплектация: {{ item.kit }}{% endif -%}
          {%- if item.kit and item.specs %} {% endif -%}
          {%- if item.specs %}Тех. характеристики: {{ item.specs }}{% endif -%}
        </td>
      </tr>
      {%- endif %}
    </tbody>
    {%- endfor %}
  </table>
</div>
{%- endif %}
```
`loop.cycle(...)` is a standard Jinja2/MiniJinja loop helper — confirmed available (`| length` is
already used elsewhere in the codebase per `report.html:117`, and MiniJinja is built with the
`builtins` feature which ships the standard loop object). Wrapping BOTH rows of a device in one
`class="row-even"`/`row-odd"` value (not alternating within a device) keeps the visual zebra
keyed to *device*, not *table row* — a design decision within Claude's Discretion, not a locked
one.

### Pattern 3: forced break before the appendix (D-16)
```css
/* Source: standard CSS Fragmentation, Paged.js implements break-before */
.appendix {
  break-before: page;
}
```
Paged.js reads standard `break-before`/`break-after`/`break-inside` properties (this is its core
purpose — CSS Paged Media polyfill); `break-before: page` unconditionally forcing a new sheet
before `.appendix` is the standard mechanism, matching D-16 exactly ("always forced, never
by-room"). [CITED: pagedjs is a CSS Fragmentation Module polyfill by design — its own
`README`/docs describe `break-before`/`break-after`/`break-inside` as first-class supported
properties; direct behavioral test not run in this research session, MEDIUM confidence]

### Pattern 4: keep-together for a device's two rows (D-15)
**What:** Wrap each device's 1-2 `<tr>`s in their own `<tbody>` and set `break-inside: avoid` on
that `<tbody>`.
**Why this exact mechanism, not `break-inside: avoid` on the `<tr>`:** [VERIFIED: codebase read,
`ui/node_modules/pagedjs/src/chunker/layout.js:582-589`] — the overflow-finding algorithm that
decides where to cut a table explicitly reads
`window.getComputedStyle(container)["break-inside"]` where `container` is
`tableRow.parentElement` **only when that parent's `nodeName` is `TBODY` or `THEAD`**:
```js
// Source: ui/node_modules/pagedjs/src/chunker/layout.js:582-589 (bundled pagedjs 0.4.3)
let tableRow;
if (node.nodeName === "TR") {
  tableRow = node;
} else {
  tableRow = parentOf(node, "TR", rendered);
}
if (tableRow) {
  // honor break-inside="avoid" in parent tbody/thead
  let container = tableRow.parentElement;
  if (["TBODY", "THEAD"].includes(container.nodeName)) {
    let styles = window.getComputedStyle(container);
    if (styles.getPropertyValue("break-inside") === "avoid") prev = container;
  }
  // ...
}
```
There is no equivalent code path that reads a bare `<tr>`'s own `break-inside` style inside this
same table-specific branch. Using per-device `<tbody>` is therefore the only mechanism confirmed
by direct source inspection to be honored for this exact "keep N rows of a table together"
requirement — matching the pattern already proven working in this codebase for
`.device-block { page-break-inside: avoid }` (a non-table block-level element, a different but
verified-working code path in the same file).

### Anti-Patterns to Avoid
- **Assuming thead repeats "because Paged.js is a paged-media polyfill":** it implements CSS
  Fragmentation (`break-*`) faithfully, but table-header repetition across an automatically split
  table is a *separate*, historically unimplemented feature in this codebase (see Pitfall 1). Do
  not treat CSS `@page`/`table-header-group` semantics as automatically honored — confirm with a
  real multi-page render.
- **Adding a `@page`-scoped counter or footer for "Приложение №1 (продолжение)":** explicitly
  forbidden by D-12/D-15 — breaks `html_page_parity.rs`'s byte-identical `@page` block
  requirement across all three templates.
- **Implementing the thead-repeat handler in only one of the two Paged.js entry points:** desktop
  print + on-screen preview share `bootstrapScript.js`; LAN print does not (see System
  Architecture Diagram). A one-sided fix produces a visible WYSIWYG mismatch between transports —
  directly undermining Success Criterion #3/#4.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Page-break computation / A4 pagination | A manual "does this fit" measurement pass in JS or Rust | Paged.js's existing Chunker/Previewer (already wired, Phase 33) | Phase 33 explicitly rejected a custom block-based paginator; Phase 36 stays inside that decision — it only adds CSS fragmentation hints (`break-before`/`break-inside`), never measures pixels itself |
| Table-header repetition | A from-scratch table-splitting algorithm | A small Paged.js `Handler` hooking `afterPageLayout` (or equivalent hook the chunker already exposes) that clones the source `<thead>` into each new page fragment containing a continuation of the same `<table>` | Paged.js already owns the concept of "a page fragment of a table" — a Handler that observes and augments that lifecycle is far less code and far less fragile than re-detecting table splits independently |
| Legacy-defaults version tracking | A new/ad-hoc mechanism to detect "was this file user-edited" | `KNOWN_LEGACY_DEFAULTS` + `upgrade_untouched_defaults_on_startup` (already exists, `html_templates.rs`) — add one new v24 slice, following the exact v20-v23 precedent | The byte-comparison fail-closed mechanism is already built, tested (5 existing regression tests per version), and battle-tested across 4 prior phases (16, 20, 34, 35) |

**Key insight:** every piece of *pagination logic* in this codebase deliberately lives in Paged.js
and CSS, never in Rust or hand-rolled JS layout math (Phase 33's core decision). Phase 36 must
extend that same boundary — the one place it is tempting to hand-roll something (thead repeat) is
exactly where a small, hook-based addition to the existing polyfill is both correct and consistent
with the project's established pattern, versus writing a bespoke table-splitting pass.

## Common Pitfalls

### Pitfall 1: `<thead>` repetition across pages is not a native Paged.js 0.4.3 feature
**What goes wrong:** A plan that assumes "Paged.js repeats thead automatically" (as D-15's prose
implies) will render only the FIRST appendix sheet with column headers; every continuation sheet
will show bare data rows with no labels.
**Why it happens:** [VERIFIED: codebase read] `ui/node_modules/pagedjs/src/chunker/layout.js` and
`chunker.js` contain no thead-cloning/re-insertion logic. This is corroborated by multiple
long-open upstream issues (pagedjs/pagedjs #84, #133, #236) and an **unmerged** PR #160
("Repeat thead and colgroup when table breaks across page") still open at research time — i.e.
even the latest pagedjs source on GitHub does not have this natively; 0.4.3 (older) certainly
doesn't. [MEDIUM confidence — corroborated by 3+ independent community sources plus a direct read
of the exact bundled file, but the exact behavior was not live-tested in a running preview this
session]
**How to avoid:** Implement a small custom Paged.js `Handler`. The bundled UMD build exposes both
`Handler` and `registerHandlers` at `window.PagedModule.Handler` /
`window.PagedModule.registerHandlers` — [VERIFIED: `grep -oE "e\.(Handler|registerHandlers)="` on
`ui/node_modules/pagedjs/dist/paged.min.js` matches both]. The ESM entry
(`ui/node_modules/pagedjs/src/index.js`) exports the same two symbols, so `printViaTopLevel`'s
`import('pagedjs')` can use the identical pattern. The chunker exposes an `afterPageLayout` hook
(confirmed in `ui/node_modules/pagedjs/src/chunker/chunker.js:111`, triggered with
`(pageElement, page, breakToken, chunker)`) that a `Handler` subclass can bind to detect a table
continuation and clone the original `<thead>` into it.
```js
// Sketch — NOT verified end-to-end in this project; treat as a starting point for a spike task,
// pattern adapted from the community gist (theinvensi/pagedjs-repeat-table-header) and confirmed
// hook name/signature against the bundled source (chunker.js:103, layout.js callers).
class RepeatTableHeadHandler extends Handler {
  afterPageLayout(pageElement /*, page, breakToken, chunker */) {
    pageElement.querySelectorAll('table.appendix-table').forEach((table) => {
      if (table.querySelector('thead')) return; // already has one (first fragment)
      const sourceThead = /* the ORIGINAL table's thead, cloned before pagination */;
      table.insertBefore(sourceThead.cloneNode(true), table.firstChild);
    });
  }
}
registerHandlers(RepeatTableHeadHandler);
```
Because the handler must run identically for on-screen preview + desktop print (via
`bootstrapScript.js`) AND for LAN print (via `printViaTopLevel`'s separate `import('pagedjs')`),
budget an explicit early spike/verification task, and keep the two implementations byte-identical
in behavior (a small shared `.ts` source that both import, or careful duplicate-and-comment, is a
planner decision).
**Warning signs:** A live multi-page appendix preview where sheet 2+ has data rows but no column
labels.

### Pitfall 2: any edit to `bootstrapScript.js` requires regenerating the CSP hash constant
**What goes wrong:** Adding the thead-repeat handler to `bootstrapScript.js` changes its bytes;
`PAGED_PREVIEW_INLINE_SCRIPT` (the exact concatenation `paged.min.js text + ';\n' + bootstrapText`)
changes too, and `crates/trackly-app/src/http/mod.rs`'s hardcoded `'sha256-<digest>'` CSP
`script-src` source goes stale — the inline bootstrap `<script>` is silently blocked **only in LAN
mode** (Tauri's `csp: null` is unaffected), reproducing exactly the class of bug Phase 33 D-14
fixed.
**Why it happens:** [VERIFIED: codebase read] `ui/scripts/check-pagedjs-csp-hash.mjs` computes
this hash from the current file bytes and compares against the hardcoded Rust constant — it is a
`pnpm lint` gate, not an automatic regeneration.
**How to avoid:** After any `bootstrapScript.js` edit, run
`node scripts/check-pagedjs-csp-hash.mjs --print` (from `ui/`) and paste the printed
`sha256-...` value into the `script-src` directive in `crates/trackly-app/src/http/mod.rs`. This
is a mechanical, one-line follow-up — cheap once known, invisible if missed (it will only fail
`pnpm lint`, not `cargo test`).
**Warning signs:** `pnpm --dir ui lint` fails on `check-pagedjs-csp-hash`; or — if that gate is
skipped — the LAN-mode preview modal shows the "Paged.js not loaded" fallback (D-02, Phase 33)
while Tauri desktop works fine.

### Pitfall 3: `print-color-adjust: exact` must be scoped to the actual cells, not just `body`
**What goes wrong:** Relying on inheritance from a body-level `print-color-adjust: exact`
declaration can be defeated by an intervening rule, and Safari specifically does **not** apply
`exact` set on `body` to backgrounds of `body` itself, only descendants
[MEDIUM confidence, WebSearch cross-referenced with MDN/caniuse summaries — not project-specific
tested this session].
**Why it happens:** Browsers (Chromium/WebView2 and Safari alike) suppress background colors and
images by default when printing unless explicitly told not to, to save ink — `print-color-adjust`
(unprefixed, Chromium ≥92) plus `-webkit-print-color-adjust` (older WebKit/Blink) is the standard
override, but per-property, not automatically inherited into every possible print surface.
**How to avoid:** Declare `print-color-adjust: exact; -webkit-print-color-adjust: exact;` directly
on the zebra-striped `td`/`tr` selectors inside `act_handover.html`'s own `<style>` block (the
existing project pattern: styles live inline per-template, not centrally) — same approach D-04
already specifies. This does NOT interact with `check-print-isolation.mjs`'s INV-1d, which only
forces `html`/`body`/`.pagedjs_page` to white — it never touches table-cell-level rules (confirmed
by reading the full script: `hasBodyWhite`/`hasSheetWhite` checks are selector-scoped to
`html`/`body`/`.pagedjs_page` only).
**Warning signs:** Zebra visible on screen preview, missing on printed/PDF output — check the
OS/browser print dialog's own "Background graphics"/"печатать фоновые цвета" setting too; that is
a user-controlled setting outside CSS's ability to force, and is explicitly out of PRV-03's
guarantee scope per Phase 33 D-03 (dialog defaults only).

### Pitfall 4: text-extraction tests cannot see page breaks, repeated headers, or zebra — and no
Rust-side PDF file exists to inspect either
**What goes wrong:** Writing/relying on `#[ignore]` tests that shell out to `qlmanage` on a
Rust-generated `.pdf` file, following the pre-Phase-16 project-memory pattern
(`act-pdf-word-fidelity`).
**Why it happens:** Since Phase 16's `pdf-pivot-to-html-print`, `render_pdf` returns an HTML
STRING — a PDF byte stream is only ever materialized by the browser at `window.print()` time
inside `PdfPreviewModal.svelte`. There is no Rust-side PDF file to run `qlmanage` against anymore.
This exact trap is already documented as **Pitfall 5 in `35-RESEARCH.md`** and applies identically
here — nothing changed about the rendering pipeline between Phase 35 and Phase 36.
**How to avoid:** Verification for pagination/thead-repeat/zebra MUST be a live render: open the
real desktop app (`cargo tauri dev`) and the real LAN browser (after `pnpm --dir ui build` —
memory `dev-browser-testing-needs-ui-build`), create a 1-device and a 3+-device handover act, open
the PDF preview modal for each, and visually inspect. Text-extraction Rust tests remain valid for
DOM structure/label presence (does the `<ol>` exist, does each row have the right cell values) but
prove nothing about the geometry criteria (#1/#3 of the phase).
**Warning signs:** `cargo test` fully green while a live preview shows overlapping content,
missing headers, or a zebra that vanished on print.

### Pitfall 5: the TemplateEditor's built-in preview never exercises the N>1 branch
**What goes wrong:** Using Settings → Шаблоны → `act_handover` → «Предпросмотр» as a stand-in for
"I checked the appendix branch renders."
**Why it happens:** [VERIFIED: codebase read] `template_service.rs`'s `demo_context_for_kind`
supplies exactly **one** item in `act.items` for the `act_handover`/default branch — the N>1
appendix path is structurally unreachable through that preview, regardless of any template edit.
This is pre-existing behavior, not something this phase needs to fix, but it is a trap for
verification: a developer clicking "Предпросмотр" after implementing D-01..D-16 will only ever see
the (unchanged) N=1 path.
**How to avoid:** N>1 verification must go through a REAL act with 2+ devices
(`p.acts.render_pdf(act.id)` in Rust tests for structure, and a real multi-device act created
through the UI for the live preview — see Pitfall 4). Do not rely on the Template Editor preview
for this phase's Success Criteria #2/#3.
**Warning signs:** "I checked it in the template editor" reported as done for the appendix branch.

### Pitfall 6: existing test suite asserts the OLD single-flow shape and will fail loudly, not
silently
**What goes wrong:** Several existing tests assert structure that D-06/D-07/D-08 of this phase
deliberately replace — running the full suite unmodified after the template edit will fail, and
that is *expected*, not a regression to chase.
**Why it happens:** [VERIFIED: codebase read, `crates/trackly-app/tests/*.rs`] Specific,
concretely-identified failure points:
- `html_act_render.rs::extract_first_ul` looks for a literal `<ul>`/`</ul>` pair — D-07 replaces
  the summary list with `<ol>`. This helper function itself needs renaming/rewriting, not just the
  test body.
- `html_act_render.rs::html_handover_multi_device_renders_plural_summary_listing_every_name`
  asserts (a) a `<ul>` with exactly one `<li>` per device, AND (b) that EACH `.device-block`
  independently repeats "было получено устройство: ⟨name⟩" 3 times for a 3-device act (Phase 35's
  D-02a). D-08 of this phase explicitly REMOVES `.device-block` from the first sheet at N>1 — this
  assertion is now testing behavior this phase deliberately deletes.
- `pdf_render_act.rs::render_handover_multi_device_fields_attributable_to_own_device` splits HTML
  on the literal string `<div class="device-block">` and asserts 4 parts (1 preamble + 3 device
  blocks) for a 3-device act — same conflict, N>1 no longer emits `.device-block` at all.
- `pdf_render_act.rs::render_handover_multi_device_wraps_long_fields` asserts a long
  `complectation_at_time` value renders without `'…'` truncation somewhere in the N>1 flow — must
  be re-pointed at the new appendix sub-row markup.
- `acts_e2e_smoke.rs::handover_pdf_render_within_e2e` renders a handover PDF mid-e2e-scenario;
  check whether its fixture uses >1 device (if so, its Cyrillic/byte-count assertions may need to
  tolerate the new appendix shape).
This is the SAME class of drift the project has hit in Phases 15, 34, and 35 (each documented in
their own C-0x notes) — not a new risk, but one that must be enumerated in `files_modified`, not
discovered mid-suite-run.
**How to avoid:** Treat every test above as `files_modified`, not `files_untouched`, from the
first plan draft. Re-run the FULL suite (not just `--lib pdf::`) at least once per wave, per
project memory `workspace_test_hangs_auth_remember_cookie`/`cargo_no_concurrent_test` constraints.
**Warning signs:** A plan that lists `act_handover.html` as the only `files_modified` entry.

### Pitfall 7: the v24 legacy-defaults slice must snapshot the PRE-Phase-36 body, taken BEFORE
the template edit lands
**What goes wrong:** Snapshotting `act_handover.html` into
`_legacy_defaults/v24/act_handover.html` AFTER editing it for pagination makes the "legacy" body
byte-identical to the new default — `upgrade_untouched_defaults_on_startup` would then find every
on-disk copy "already current" or match a legacy body that's indistinguishable from current, and
the upgrade path silently does nothing for real installs. [VERIFIED: this exact failure mode is
explicitly guarded against by existing tests — see `html_templates.rs`'s
`assert_ne!(v22_body, current, ...)`/`assert_ne!(v23_body, current, ...)` precondition checks,
each with a comment describing precisely this snapshot-timing trap]
**Why it happens:** Same trap that has hit this exact mechanism at least 3 times before (Phase 34
D-15, Phase 35 C-01/WR-01, quick `260704-uw3`, per project memory `db_backed_templates_upgrade_trap`
and this file's own doc-comments).
**How to avoid:** Capture the file's content as it stands at the START of this phase (i.e. the
current HEAD post-Phase-35 body, already read in full above) into
`crates/trackly-app/templates/_legacy_defaults/v24/act_handover.html` as the FIRST task of the
phase, before any pagination edits to the live file. `act_acceptance.html` is untouched this
phase (CONTEXT.md domain note) — no v24 slice needed for it. Add a
`v24`-indexed test mirroring the existing `upgrade_replaces_v23_legacy_default_with_current_bundled_body`
pattern (index `4` into the `act_handover.html` slice, `assert_ne!` guard included) — this closes
the same class of regression the project has explicitly built regression tests for at every prior
version bump (v21, v22, v23).
**Warning signs:** `KNOWN_LEGACY_DEFAULTS`'s `act_handover.html` slice grows to 5 elements but no
new `upgrade_replaces_v24_...` test exists to prove the new entry actually drives an upgrade.

## Code Examples

### `print-color-adjust` for zebra (D-04)
```css
/* Add inside act_handover.html's existing <style> block, scoped to the new appendix table */
.appendix-table tr.row-even td,
.appendix-table tr.row-odd td {
  print-color-adjust: exact;
  -webkit-print-color-adjust: exact;
}
.appendix-table tr.row-even td { background: #f2f2f2; }
```

### Legacy-defaults v24 slice registration (C-01, Pitfall 7)
```rust
// Source: crates/trackly-app/src/pdf/html_templates.rs:75-93 (current HEAD) — add ONE new
// include_str! entry, mirroring the existing v20..v23 pattern exactly:
(
    "act_handover.html",
    &[
        include_str!("../../templates/_legacy_defaults/v20/act_handover.html"),
        include_str!("../../templates/_legacy_defaults/v21/act_handover.html"),
        include_str!("../../templates/_legacy_defaults/v22/act_handover.html"),
        include_str!("../../templates/_legacy_defaults/v23/act_handover.html"),
        include_str!("../../templates/_legacy_defaults/v24/act_handover.html"), // NEW
    ],
),
```

### `loop.index`/`| default` already-proven idioms in this codebase
```jinja2
{# Source: crates/trackly-app/templates/act_acceptance.html:124-127 (current HEAD) — the
   `| default("—", true)` idiom D-02 explicitly reuses #}
<tr><td class="key">Инв.№</td><td>{{ device.inventory_no | default("—", true) }}</td></tr>
```
```jinja2
{# Source: crates/trackly-app/templates/report.html:117 (current HEAD) — confirms `| length`
   works with no additional Environment configuration, same `builtins` feature also ships
   `loop.index`/`loop.cycle` per MiniJinja's Jinja2-compatible loop object #}
{%- if groups is not defined or groups | length == 0 %}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|---------------|--------|
| Single-level device flow: every `.device-block` (name + full description) prints inline,
  regardless of device count | Branch on `act.items \| length`: N=1 unchanged; N>1 →
  summary `<ol>` on sheet 1, forced-break appendix table from sheet 2 | This phase (DOC-10/DOC-11) | Requires touching `.device-block`-shaped tests across 3 test files (Pitfall 6) |
| PDF bytes generated server-side (krilla/DocSpec era) | HTML string generated server-side, PDF
  materialized client-side by Paged.js at `window.print()` | Phase 16 (`pdf-pivot-to-html-print`) | No Rust-side PDF file to inspect (Pitfall 4) — unchanged from Phase 35, still true here |
| No table-based content anywhere in `act_handover.html` | First `<table>` introduced into this
  specific template (appendix) | This phase | First time this template needs Paged.js's table-specific fragmentation code paths (Pattern 4) — previously only tested via `act_acceptance.html`'s simpler non-paginated `table.kv` |

**Deprecated/outdated:**
- `qlmanage`-based PDF verification (project memory `act-pdf-word-fidelity`) — confirmed stale
  again for this phase, same reasoning as Phase 35's own Pitfall 5 (no Rust-generated PDF file
  exists post-Phase-16).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | The sketched `RepeatTableHeadHandler` (Pitfall 1) using `afterPageLayout` is a workable starting point — hook name/signature and `Handler`/`registerHandlers` export are verified present in the bundled 0.4.3 source, but the exact clone-and-insert logic has not been executed end-to-end against this project's actual appendix table markup | Common Pitfalls, Pitfall 1 | Medium — if the sketched approach doesn't work as-is, the planner should budget a dedicated spike task rather than assume a mechanical 1-hour implementation; worst case requires a different hook (`onPageLayout`, `finalizePage`) or a `MutationObserver`-based approach |
| A2 | `print-color-adjust: exact` scoped to `td` selectors is sufficient in WebView2 (Chromium-based) without additional vendor handling beyond `-webkit-` prefix | Common Pitfalls, Pitfall 3 | Low — this is well-established, widely-supported CSS (Chromium ≥92 unprefixed, all versions with `-webkit-` prefix); WebView2's Chromium version on the target Windows fleet was not independently confirmed ≥92 this session but is extremely likely given current Windows WebView2 auto-update policy |
| A3 | `loop.cycle('row-even', 'row-odd')` (or equivalent per-device zebra keying) is available in MiniJinja's `builtins` feature the same way it is in Jinja2 | Architecture Patterns, Pattern 2 | Low — MiniJinja documents itself as Jinja2-compatible for loop helpers and this project already uses `| length`/other loop-adjacent filters; not independently unit-tested against `loop.cycle` specifically this session |

## Open Questions

1. **Is the sketched custom Paged.js Handler (Pitfall 1) actually the best implementation shape,
   or should the planner treat "faithful thead-repeat" as a spike-first task with an explicit
   go/no-go checkpoint?**
   - What we know: no native support exists in 0.4.3; the mechanism (Handler + `afterPageLayout`)
     is technically available and matches a documented (if historically fragile across pagedjs
     versions) community pattern.
   - What's unclear: whether the exact clone-and-insert logic will behave correctly against THIS
     project's specific table markup (colspan sub-rows, `<tbody>`-per-device grouping) without
     surprises — community reports (GitHub #236) describe this class of handler "stopping working"
     across pagedjs version bumps, which is a signal of general fragility, not necessarily
     something specific to 0.4.3.
   - Recommendation: give this its own early task with a `checkpoint:human-verify` gate (render a
     10+ device fixture, visually confirm every sheet has a header row) BEFORE building out the
     rest of the appendix styling on top of it — cheaper to discover a fragile mechanism early than
     after the whole appendix is styled around it.

2. **Does the two-place duplication of the thead-repeat handler (bootstrapScript.js UMD text vs.
   printViaTopLevel's ESM import) need a shared source of truth, or is careful duplication
   acceptable given the CSP-hash constraint on the UMD path?**
   - What we know: `bootstrapScript.js` must stay a single static string with zero interpolation
     (CSP hash pinning, per its own file-header comment) — it cannot import a shared module.
     `printViaTopLevel` CAN import from a `.ts` module (it already does, for `pagedPreviewBridge.ts`
     patterns elsewhere in `ui/src/lib/pdfPreview/`).
   - What's unclear: whether the handler logic is small/stable enough that hand-duplicating it
     with a "keep these two in sync" comment is acceptable, versus needing a build-time codegen
     step that stamps the shared logic into both places.
   - Recommendation: start with hand-duplication + a loud cross-referencing comment in both files
     (matching the project's existing convention, e.g. how `PAGED_PREVIEW_INLINE_SCRIPT`'s formula
     comment cross-references the CSP-hash script) — escalate to codegen only if review finds the
     duplication error-prone in practice.

## Environment Availability

Skipped — this phase introduces no new external dependency. All required tooling (`cargo`, `pnpm`,
`node`, the already-installed `pagedjs` package) is the same toolchain every prior phase in this
milestone (33–35) already exercised successfully.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | `cargo test` (workspace), integration targets in `crates/trackly-app/tests/*.rs`; `pnpm --dir ui lint` for the two structural JS gates |
| Config file | none separate — workspace `Cargo.toml`; `ui/package.json`'s `lint` script chains the gates |
| Quick run command | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --lib pdf:: -- --test-threads=1` |
| Full suite command | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app -- --test-threads=1` (requires a real `pnpm --dir ui build` beforehand) + `pnpm --dir ui lint` |

**Hard constraints (project memory, do not re-litigate):**
- Never run two `cargo test` invocations concurrently — `target/` lock contention looks like a
  hang (`cargo_no_concurrent_test`).
- `cargo test --workspace` hangs on the pre-existing `login_remember_persistent_cookie` — use
  targeted `-p trackly-app` commands, `--skip` that test if running the full workspace.

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|--------------------|--------------|
| DOC-10 | N=1 handover unchanged: single `.device-block`, full description on one sheet | integration | `cargo test -p trackly-app --test pdf_render_act render_handover_act_produces_cyrillic_pdf` | ✅ existing, should pass unmodified (N=1 path untouched) |
| DOC-10 | N=1 attribution/structure regression guard | integration | `cargo test -p trackly-app --test html_act_render html_handover_single_device_renders_singular_intro_not_plural_summary` | ✅ existing, should pass unmodified |
| DOC-11 | N>1 first sheet shows only `<ol>` summary + referral line, no `.device-block` | integration | `cargo test -p trackly-app --test html_act_render html_handover_multi_device_renders_plural_summary_listing_every_name` (rename/rewrite — see Pitfall 6) | ✅ existing, requires REWRITE not just update |
| DOC-11 | N>1 appendix table: one row per device, correct field values, dash for empty | integration | new test, e.g. `cargo test -p trackly-app --test html_act_render html_handover_appendix_table_has_one_row_per_device` | ❌ Wave 0 — new test |
| DOC-11 | `quantity` column: blank/"—" at 1, printed at >1 | integration | new test alongside the appendix-table test above | ❌ Wave 0 — new test |
| DOC-11 | `<ol>` numbering matches appendix `№` column | integration | new test, split first `<ol>`...`</ol>` and appendix table `<td>` cells, assert numeric correspondence | ❌ Wave 0 — new test |
| DOC-11 | Forced break before appendix, thead present, sub-row grouping | structural (discretion) + manual | a cheap regex-structural test can confirm `break-before: page` and `break-inside: avoid` are present on the right selectors (byte-presence, not rendering); actual page-break rendering is Manual-Only (see below) | ❌ Wave 0 — optional structural test recommended, following `html_page_parity.rs`'s style |
| DOC-11 (SC #3) | Thead repeats across sheets, appendix mark on first appendix sheet only | manual, live render | see Manual-Only table below | N/A — cannot be automated (Pitfall 1/4) |
| — | `@page`-parity across all three templates unchanged | structural | `cargo test -p trackly-app --test html_page_parity` | ✅ existing, must stay green untouched |
| — | Header partial unaffected (not included on appendix sheet is a NEW divergence from prior phases' "always identical header" assumption — confirm `html_header_parity.rs`'s scope) | structural | `cargo test -p trackly-app --test html_header_parity` | ✅ existing — verify it only asserts the FIRST occurrence of `.header`, not "every sheet has one"; if it asserts sheet-count-independent identity this should still pass since D-11 only removes appendix-sheet repetition, which never existed pre-phase-36 either |
| — | CSP hash matches bootstrapScript.js bytes (only if thead-repeat handler lands there) | structural | `node scripts/check-pagedjs-csp-hash.mjs` (via `pnpm --dir ui lint`) | ✅ existing gate, must be re-run after any bootstrapScript.js edit |
| — | Print isolation invariants (LAN print DOM leakage class of regression) | structural | `node scripts/check-print-isolation.mjs` (via `pnpm --dir ui lint`) | ✅ existing gate, unaffected by table-cell-level CSS additions (verified by reading the script's selector scope) |

### Sampling Rate
- **Per task commit:** `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --lib pdf:: -- --test-threads=1` (~20s)
- **Per wave merge:** full `cargo test -p trackly-app -- --test-threads=1` (after `pnpm --dir ui build`) + `pnpm --dir ui lint`
- **Phase gate:** full suite green + Level-2 manual visual pass (below) on BOTH transports, BEFORE `/gsd-verify-work`

### Wave 0 Gaps
- [ ] `crates/trackly-app/templates/_legacy_defaults/v24/act_handover.html` — snapshot taken
      BEFORE any pagination edit (Pitfall 7); register in `KNOWN_LEGACY_DEFAULTS`.
- [ ] New `upgrade_replaces_v24_legacy_default_with_current_bundled_body` test in
      `html_templates.rs`, mirroring the v21/v22/v23 pattern (index `4`, `assert_ne!` guard).
- [ ] New appendix-table structural tests (device count → row count, quantity column,
      `<ol>` ↔ appendix `№` correspondence) — none exist yet.
- [ ] Rewrite (not just touch) `html_act_render.rs::extract_first_ul` and the tests that rely on
      it, plus the `.device-block`-splitting tests in `pdf_render_act.rs` — see Pitfall 6 for the
      full enumerated list.
- [ ] Decide + implement the thead-repeat Handler (Pitfall 1/Open Question 1) — the one piece of
      genuine implementation risk in this phase; recommend an early spike task with its own
      `checkpoint:human-verify`.
- [ ] If the Handler lands in `bootstrapScript.js`: regenerate the CSP hash constant in
      `crates/trackly-app/src/http/mod.rs` (Pitfall 2).

**Manual-Only (required — same C-04/Pitfall-4 reasoning as Phase 33/35):**

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|-------------|--------------------|
| N=1 act fully on one sheet with full description | DOC-10 (SC #1) | Geometry invisible to text-extraction | Desktop preview + LAN preview (after `pnpm --dir ui build`) of a 1-device handover act; confirm one sheet, full description present |
| N>1 first sheet shows ONLY summary + referral, no descriptions | DOC-11 (SC #2) | Same | Multi-device (2-3) handover act preview; confirm first sheet has no `.device-block`-style full field descriptions |
| Appendix starts on sheet 2, forced break, thead + appendix mark render correctly | DOC-11 (SC #3) | Page-break rendering + thead repetition are exactly what text-extraction cannot see (Pitfall 1/4) | Same preview; scroll through all appendix sheets, confirm break lands where expected and (if a long-enough fixture, e.g. 15+ devices spanning 2+ appendix sheets) the header row repeats on every appendix sheet |
| Zebra + print-color-adjust survives actual print/PDF | DOC-11 (D-04) | Background suppression is a real print-time browser behavior, not visible in the on-screen srcdoc preview alone | Trigger an actual print/"Save as PDF" (not just the modal preview) on both transports, confirm zebra visible in the output; verify OS print dialog's "print background graphics" is at its DEFAULT setting (per Phase 33 D-03 scope) |
| Print isolation / LAN DOM leakage unaffected by new appendix CSS | Phase Success Criterion #4 | Structural gate (`check-print-isolation.mjs`) proves invariants hold in source, not that the rendered result looks right | Live LAN print of a multi-device act; confirm no residual app chrome/typography bleeds into the printed appendix pages |
| Same checks on the second transport | DOC-10/11 (SC #1-3), Phase SC #4 | Desktop (WKWebView/WebView2-family) and LAN (real browser) engines can render fragmentation/hyphenation differently | Repeat every check above on BOTH desktop (`cargo tauri dev`) and LAN browser |
| Real Windows/WebView2 run | All of the above | Dev machine is macOS only (project memory `dev-environment-constraints`) | Flag as a deferred pre-close UAT item for the user, same as prior phases with Windows-only verification needs |

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-------------------|
| V5 Input Validation / Output Encoding | yes | `AutoEscape::Html` in `build_safe_html_env()` already covers every new interpolation site this phase adds (`item.quantity`, `item.name`/etc. inside the appendix table, `act.number`/`act.suffix`/`act.date_human` inside the appendix mark) — all are pre-existing context keys, already interpolated elsewhere in this same template via plain `{{ }}`, no new `\| safe` sink introduced |
| V2/V3/V4/V6 | no | Phase touches only printed-form templates/client-side pagination script; no auth, session, access-control, or cryptography surface |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Stored-XSS via a device field (`name`/`model`/`specs`/`kit`) if the appendix table used `\| safe` | Tampering/Injection (same class as T-16-01, already closed by Phase 16) | Continue using plain `{{ item.field }}` interpolation (autoescape ON) for every new appendix-table cell — do not introduce any new `\| safe` site in this phase; `item.*` fields are exactly the same trust level as the fields already interpolated safely in the N=1 `.device-block` today |
| CSP script-src drift silently disabling the inline Paged.js bootstrap in LAN mode | Tampering (config drift, not an attacker-controlled input) | `check-pagedjs-csp-hash.mjs` gate (Pitfall 2) — must be re-run and the Rust constant updated whenever `bootstrapScript.js` changes, or LAN-mode pagination (and therefore this whole phase's LAN-transport behavior) silently breaks |
| A malicious/malformed custom Paged.js Handler cloning unexpected DOM into the wrong page fragment | Tampering (implementation bug, not attacker-facing — the handler operates only on server-rendered, already-escaped HTML) | Scope the handler's `querySelectorAll` strictly to `table.appendix-table` (a class this phase controls), never operate on arbitrary DOM; this is a correctness concern more than a security one, since the handler never touches attacker-controlled strings directly (it manipulates already-rendered, already-escaped nodes) |

No new `T-36-*` threats beyond what Phase 16's HTML-templating security model already covers — this
phase adds new INTERPOLATION SITES (all already-safe context keys) and new CLIENT-SIDE PAGINATION
LOGIC (operating on trusted, server-rendered HTML, not user input directly), not new trust
boundaries.

## Sources

### Primary (HIGH confidence — read directly from the repository, [VERIFIED: codebase read])
- `crates/trackly-app/templates/act_handover.html` (full, 195 lines, current HEAD post-Phase-35)
- `crates/trackly-app/templates/act_acceptance.html` (full, 152 lines) — reference idiom only
- `crates/trackly-app/templates/_header.html` (full, 119 lines)
- `crates/trackly-app/src/pdf/html_templates.rs` (full) — `DEFAULT_HTML_TEMPLATES`,
  `KNOWN_LEGACY_DEFAULTS`, materialize/upgrade mechanism + all existing regression tests
- `crates/trackly-app/src/pdf/minijinja_env.rs` (full) — `build_safe_html_env`,
  `UndefinedBehavior::Strict`, autoescape invariants
- `crates/trackly-app/src/services/act_service.rs:2560-2665` — render context construction,
  `items_json` (confirms `quantity` already present, all 8 fields)
- `crates/trackly-app/src/services/template_service.rs:330-530` — `validate_preview`,
  `demo_context_for_kind` (found: N>1 branch unreachable via the built-in editor preview, Pitfall 5)
- `crates/trackly-app/tests/pdf_render_act.rs`, `tests/html_act_render.rs`,
  `tests/acts_e2e_smoke.rs` — enumerated the specific test-drift points (Pitfall 6)
- `crates/trackly-app/tests/html_page_parity.rs`, `tests/html_header_parity.rs` — durable gates
  confirmed unaffected by this phase's scope
- `ui/scripts/check-print-isolation.mjs` (full, 519 lines) — confirmed selector scope
  (`html`/`body`/`.pagedjs_page` only), unaffected by table-cell CSS additions
- `ui/scripts/check-pagedjs-csp-hash.mjs` (full) — confirmed hash formula and update procedure
- `ui/src/lib/pdfPreview/bootstrapScript.js`, `pagedPreviewBootstrap.ts` — confirmed shared UMD
  bundle formula, confirmed reused identically by `PdfPreviewModal.svelte`'s
  `printViaSystemBrowser`
- `ui/src/features/acts/PdfPreviewModal.svelte:295-480` — confirmed `printViaTopLevel` uses a
  SEPARATE `import('pagedjs')` ESM path, not `bootstrapScript.js` (System Architecture Diagram)
- `ui/node_modules/pagedjs/package.json` — confirmed pinned version `0.4.3`
- `ui/node_modules/pagedjs/src/chunker/layout.js:520-650` — confirmed `break-inside: avoid`
  TBODY/THEAD special-casing (Pattern 4), confirmed NO thead-repeat logic anywhere in this file
- `ui/node_modules/pagedjs/src/chunker/chunker.js:99-113` — confirmed `afterPageLayout` and
  sibling hooks exist (Pitfall 1)
- `ui/node_modules/pagedjs/src/index.js`, `ui/node_modules/pagedjs/dist/paged.min.js` (grep) —
  confirmed `Handler`/`registerHandlers` exported from both the ESM and bundled UMD builds
- `crates/trackly-app/Cargo.toml:53` — minijinja version + enabled features
- `.planning/phases/36-act-pagination/36-CONTEXT.md`, `.planning/REQUIREMENTS.md`,
  `.planning/phases/35-act-handover-body/35-CONTEXT.md`,
  `.planning/phases/35-act-handover-body/35-RESEARCH.md` (Pitfall 5 in that file directly informed
  Pitfall 4 here), `.planning/phases/33-print-preview-polish/33-CONTEXT.md`
- `.planning/config.json` — confirmed `nyquist_validation: true`, `security_enforcement: true`

### Secondary (MEDIUM confidence — WebSearch, cross-referenced against the codebase read above)
- pagedjs GitHub issues #84, #133, #236 and PR #160 (unmerged at research time) — corroborate the
  "thead repeat is not native" finding from the direct source read
- `theinvensi/pagedjs-repeat-table-header` community gist — informed the Handler sketch's shape
  (hook usage, general approach), NOT copied verbatim, NOT independently executed this session
- MDN/caniuse-derived summary of `print-color-adjust`/`-webkit-print-color-adjust` browser support
  (Chromium unprefixed ≥92, Safari prefix-dependent below 15.4, Safari `body`-background caveat)

### Tertiary (LOW confidence)
- None beyond what's captured above with an explicit confidence caveat inline (A1/A2/A3 in the
  Assumptions Log).

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new dependencies, both existing versions confirmed via direct file
  read, not guessed
- Architecture (branching, forced break, tbody-grouping): HIGH — every mechanism except
  thead-repeat is confirmed by direct source read of the bundled Paged.js code, matching an
  already-proven-working pattern (`.device-block`) in this same codebase
- Thead-repeat specifically: MEDIUM-LOW — confirmed ABSENT natively (high confidence), but the
  proposed fix is sketched, not executed end-to-end; flagged as the phase's one real
  implementation-risk item with an explicit spike recommendation
- Pitfalls/test-drift: HIGH — every enumerated test-drift point was read directly, not inferred
- Legacy-defaults mechanism: HIGH — directly read, matches 4 prior phases' proven precedent

**Research date:** 2026-08-12
**Valid until:** ~30 days for the MiniJinja/legacy-defaults/CSP-hash facts (stable, internal,
unlikely to change); ~7-14 days for the Paged.js thead-repeat upstream status specifically
(actively discussed, PR #160 could merge or a new workaround could surface) — re-check pagedjs
GitHub issue/PR status if this research is used to plan more than ~2 weeks after 2026-08-12.

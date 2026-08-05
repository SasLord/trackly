---
quick_id: 260805-ifj
slug: lan-print-neutralize-app-line-height-leak
phase: 260805-ifj
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - ui/src/features/acts/PdfPreviewModal.svelte
autonomous: true
requirements: [IFJ-01]
must_haves:
  truths:
    - "Printing an act from a LAN browser (printViaTopLevel) uses the UA-default line-height (normal, ~1.2) for body text, matching desktop print (printViaSystemBrowser) and the printed act's own template, which was already confirmed correct — no more stretched, over-spaced act text on LAN printouts"
    - "The line-height reset only applies inside @media print — the app's own on-screen typography (line-height: 1.5 from global.scss) is completely unaffected"
    - "The reset is overridable by a template's own body { line-height: ... } rule if a user-customized template on disk declares one (D-01: templates are user-editable) — the reset must lose the cascade in that case, not win it"
    - "All pre-existing @media print rules — display:none app-chrome, position:static/left:auto reset, and the two background:#fff !important rules — are unchanged"
    - "printViaSystemBrowser (desktop print path) and ui/src/lib/pdfPreview/bootstrapScript.js (CSP-hash-locked) are untouched"
  artifacts:
    - path: "ui/src/features/acts/PdfPreviewModal.svelte"
      provides: "printViaTopLevel's injected printStyle prepends a @media print line-height/letter-spacing/word-spacing reset BEFORE cssText, so it loses the cascade to any template-declared line-height but wins over the app's own global.scss default"
      contains: "line-height: normal;"
  key_links:
    - from: "ui/src/features/acts/PdfPreviewModal.svelte printStyle template literal"
      to: "@media print block, placed before ${cssText}"
      via: "@media print { body { line-height: normal; letter-spacing: normal; word-spacing: normal; } } prepended to printStyle.textContent, ahead of the existing ${cssText} interpolation"
      pattern: "line-height: normal"
---

<objective>
Fix a LAN-only print defect: printing an act from a browser connected to the LAN server (not
the desktop app) shows visibly larger line spacing than desktop print, stretching the content
down the page and no longer matching the on-screen preview the user saw before printing. Live
UAT (build 1.3.1) confirmed this on two physical printouts of the same act, side by side.

Root cause (confirmed by reading the code, not inferred): `printViaTopLevel` renders Paged.js
output into the Trackly app's OWN DOM (`document.body`), so act content inherits the app's own
`body` CSS instead of getting a clean slate. `ui/src/styles/global.scss` sets
`body { line-height: var(--tr-line-height-body); }` (`--tr-line-height-body: 1.5`, per
`_tokens.scss`). All three shipped templates (`act_handover.html`, `act_acceptance.html`,
`report.html`) declare `body { font-family; font-size; color; margin; padding }` but
deliberately do NOT declare `line-height` — they're written as standalone documents and rely on
the UA default (`normal`, ~1.2). Every other `body` property the app sets is ALSO redeclared by
the template's own `<style>` block (injected into `printStyle.textContent` as `${cssText}`,
which appears AFTER the app's stylesheet in cascade order), so the template wins those.
`line-height` is the one property only the app declares — nothing in the template's `body` rule
overrides it — so it's the only one that leaks through. The desktop path
(`printViaSystemBrowser`) writes a fully standalone temp HTML file with no app stylesheets
present at all, so it correctly falls back to the UA default — which is exactly why the two
printouts differed.

Purpose: printed output from the LAN path must match the already-correct desktop path and the
on-screen preview the user approved before printing.

Output: three new declarations (`line-height`, `letter-spacing`, `word-spacing`, all `normal`)
inside a NEW `@media print { body { ... } }` rule, prepended to `printViaTopLevel`'s injected
`printStyle.textContent`, ahead of the existing `${cssText}` interpolation.

Explicitly OUT OF SCOPE for this plan (do not touch): a separate, pre-existing defect in the
same function where `${cssText}` is injected unscoped, so a template's `body { font-family;
font-size; color }` also leaks onto the APP's own on-screen body, and `printStyle` is never
removed after printing (only `printRoot.innerHTML` is cleared on `afterprint`) — so the app's
own typography can change after the first LAN print. Fixing that requires either scoping
`cssText` under `@media print` or dropping the manual injection entirely (Paged.js's Polisher
already consumes `cssText` via its `preview()` argument), both of which carry pagination risk
and deserve their own change with their own UAT. Record it as a known follow-up in the SUMMARY;
do not act on it here.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@CLAUDE.md
@.planning/STATE.md

<interfaces>
<!-- Current printStyle.textContent assignment inside printViaTopLevel — the ONLY block this
     plan touches. ui/src/features/acts/PdfPreviewModal.svelte, function printViaTopLevel,
     around line 394-417 (search for "printStyle.textContent ="). Two prior quick tasks
     (260805-gdz, 260805-har) already added rules to the @media print block inside this same
     literal — every rule below except the new one this plan adds is pre-existing and must
     survive unchanged. -->
```javascript
printStyle.textContent = `
  ${cssText}
  #${PRINT_ROOT_ID} {
    position: absolute;
    left: -100000px;
    top: 0;
  }
  @media print {
    html, body {
      background: #fff !important;
    }
    body > :not(#${PRINT_ROOT_ID}) {
      display: none !important;
    }
    #${PRINT_ROOT_ID} {
      display: block !important;
      position: static;
      left: auto;
    }
    .pagedjs_page {
      background: #fff !important;
    }
  }
`;
```

<!-- Source of the leak — ui/src/styles/global.scss line ~33 (DO NOT EDIT, reference only) -->
```scss
body {
  line-height: var(--tr-line-height-body);
}
```
`--tr-line-height-body` (ui/src/styles/_tokens.scss, ~line 238): `1.5`. This is correct for the
app's own on-screen typography — do not change it there. The override belongs only inside this
plan's `@media print` addition.

<!-- Template body rule — crates/trackly-app/templates/act_handover.html <style>, READ ONLY, do
     not edit. Deliberately does not declare line-height; relies on the UA default (`normal`).
     act_acceptance.html and report.html follow the same pattern. -->
```css
body {
  font-family: ...;
  font-size: ...;
  color: ...;
}
```
No `line-height` declared here — confirms the UA default is the intended typography for all
three templates, and that this is the value the print output must fall back to.
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Prepend a print-only line-height reset ahead of the template's own cssText</name>
  <files>ui/src/features/acts/PdfPreviewModal.svelte</files>
  <action>
In `printViaTopLevel`'s `printStyle.textContent` template literal (see `<interfaces>` above for
the exact current text — search the file for "printStyle.textContent ="), prepend a new block
to the START of the template literal, BEFORE the existing `${cssText}` interpolation:

    @media print {
      body {
        line-height: normal;
        letter-spacing: normal;
        word-spacing: normal;
      }
    }

Both aspects of this placement are load-bearing — preserve them and document them with an
inline code comment directly above the new block (do not just paraphrase in prose elsewhere;
the comment must live at the point of insertion so a future editor sees it before touching this
code):

1. **Inside `@media print`** — the app's own on-screen typography (`global.scss`'s
   `line-height: var(--tr-line-height-body)`, currently `1.5`) must never be affected. Do not
   move this reset outside `@media print` and do not edit `global.scss`/`_tokens.scss`.
2. **BEFORE `${cssText}`, not after, and scoped to `body`, not `#${PRINT_ROOT_ID}`** — this is
   what makes the fix respect user-customized templates (D-01: act templates are user-editable
   files on disk, re-read on every render). Cascade order inside `@media print` then reads:
   the app's `line-height: 1.5` (from `global.scss`, an earlier stylesheet in document order) →
   this plan's `line-height: normal` reset (later in document order, wins over the app default)
   → the template's own `body` rule from `${cssText}` (later still in document order, wins over
   this reset IF that template declares its own `line-height`). Placing the reset AFTER
   `${cssText}` instead, or scoping it to `#${PRINT_ROOT_ID}` instead of `body`, would make it
   win against a template author's deliberate `line-height` choice — do not do either.

Do not add, remove, or reorder anything else in the template literal. The existing rules —
`${cssText}` interpolation, the `#${PRINT_ROOT_ID}` positioning rule, and all four rules already
inside the pre-existing `@media print { ... }` block (`html, body` background, `body > :not(...)`
display:none, `#${PRINT_ROOT_ID}` display/position reset, `.pagedjs_page` background) — must be
byte-for-byte unchanged. This plan adds ONE new `@media print { body { ... } }` block; it does
not merge into or reorder the existing `@media print { ... }` block below it (two separate
`@media print` blocks in the same stylesheet is fine and simpler to reason about than merging).

Do not modify:
- Anything outside `printViaTopLevel` — `printViaSystemBrowser`,
  `ui/src/lib/pdfPreview/bootstrapScript.js` (CSP-hash-locked), `ui/src/styles/global.scss`,
  `ui/src/styles/_tokens.scss`, any file under `crates/trackly-app/templates/`, or
  `ui/src/lib/components/Modal.svelte`.
- The separate `${cssText}`-unscoped-onto-app-body defect described in the objective's "out of
  scope" note — do not attempt to fix it here.
  </action>
  <verify>
    <automated>node -e "const fs=require('fs');const s=fs.readFileSync('/Users/madsas/Projects/trackly/ui/src/features/acts/PdfPreviewModal.svelte','utf8');const m=s.match(/printStyle\.textContent = \`([\s\S]*?)\`;/);if(!m){console.log('FAIL_NO_TEMPLATE_LITERAL');process.exit(1)}const body=m[1];const lhIdx=body.indexOf('line-height: normal');const cssIdx=body.indexOf('\${cssText}');if(lhIdx===-1){console.log('FAIL_NO_LINE_HEIGHT_RESET');process.exit(1)}if(cssIdx===-1){console.log('FAIL_NO_CSSTEXT');process.exit(1)}if(!(lhIdx<cssIdx)){console.log('FAIL_ORDER_LINE_HEIGHT_MUST_PRECEDE_CSSTEXT');process.exit(1)}const mediaBefore=body.slice(0,lhIdx).lastIndexOf('@media print');if(mediaBefore===-1||body.slice(mediaBefore,lhIdx).indexOf('}')!==-1){console.log('FAIL_NOT_INSIDE_MEDIA_PRINT');process.exit(1)}console.log('OK_RESET_PRECEDES_CSSTEXT_INSIDE_MEDIA_PRINT')"
grep -c "background: #fff !important" /Users/madsas/Projects/trackly/ui/src/features/acts/PdfPreviewModal.svelte | grep -qx 2 && echo OK_PRIOR_BACKGROUND_RULES_INTACT || echo FAIL_PRIOR_RULES_CHANGED
grep -q "position: static;" /Users/madsas/Projects/trackly/ui/src/features/acts/PdfPreviewModal.svelte && echo OK_POSITION_RESET_INTACT || echo FAIL_POSITION_RESET_MISSING
grep -q "async function printViaSystemBrowser" /Users/madsas/Projects/trackly/ui/src/features/acts/PdfPreviewModal.svelte && echo OK_SYSTEM_BROWSER_PRESENT || echo FAIL_SYSTEM_BROWSER_MISSING
cd /Users/madsas/Projects/trackly/ui && pnpm svelte-check 2>&1 | tail -20 && pnpm lint 2>&1 | tail -30 && pnpm build 2>&1 | tail -20</automated>
  </verify>
  <done>
`printStyle.textContent` in `printViaTopLevel` begins with a new `@media print { body {
line-height: normal; letter-spacing: normal; word-spacing: normal; } }` block, positioned
BEFORE the existing `${cssText}` interpolation, with an inline comment explaining both the
`@media print` scoping and the before-`cssText` ordering. The pre-existing `${cssText}`
interpolation, the `#${PRINT_ROOT_ID}` position rule, and all rules in the original `@media
print { ... }` block (background resets, display:none app-chrome, position:static/left:auto)
are unchanged. `printViaSystemBrowser` is present and unmodified. `pnpm svelte-check`, `pnpm
lint` (including the CSP-hash gate), and `pnpm build` all pass clean.
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| LAN browser client → axum-served SPA | No new trust boundary crossed — this is a pure CSS/print-styling change scoped to `@media print`, touching no data flow, no new input parsing, no new network surface. |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-ifj-01 | Tampering | `printStyle.textContent` (existing `cssText`/`bodyHtml` injection point, unchanged by this plan) | accept | Out of scope for this change — this plan adds one static, hardcoded CSS block (`line-height: normal; letter-spacing: normal; word-spacing: normal;`) with no interpolation of untrusted input. The pre-existing `cssText`/`bodyHtml` extraction from the backend-rendered act HTML is unchanged and was already accepted risk (same-origin backend-authored content, not user-supplied). |
| T-ifj-SC | Tampering (supply chain) | N/A | accept | No new dependency, no package install — pure template-literal string edit in an existing `.svelte` file. Package Legitimacy Gate not applicable. |
</threat_model>

<verification>
1. `pnpm --dir ui svelte-check` — 0 errors.
2. `pnpm --dir ui lint` — clean, including the CSP-hash gate (`bootstrapScript.js` untouched, so
   the hash cannot drift).
3. `pnpm --dir ui build` — succeeds.
4. Grep/node gate (see Task 1 `<verify>`) — proves the `line-height: normal` reset exists inside
   an `@media print` block and precedes `${cssText}` in source order.
5. Manual/human-check (NOT automatable — no frontend test framework exists, and printed line
   spacing cannot be asserted by any of the above commands; this needs a real browser on
   another machine printing against the axum server, same class of check as the LAN print
   fixes in 260805-edd/260805-gdz/260805-har): from a Windows LAN client, open the act preview
   via the LAN server URL, click Печать, and compare the printed sheet's line spacing against
   a desktop-app printout of the SAME act — they must match. Record the result in the SUMMARY;
   if this plan is executed without LAN/Windows access, flag step 5 as a pending follow-up UAT,
   matching the precedent set by the prior print-fix quick tasks in this same defect chain.
</verification>

<success_criteria>
- LAN-browser print (`printViaTopLevel`) uses UA-default line-height (`normal`), matching
  desktop print (`printViaSystemBrowser`) and the on-screen preview.
- The reset applies only under `@media print` — on-screen app typography is unaffected.
- The reset yields to a template's own `line-height` declaration if one exists (cascade order
  verified: app default → this reset → template rule).
- No regression to any of the pre-existing `@media print` rules (backgrounds, display:none,
  position reset).
- `printViaSystemBrowser` and `bootstrapScript.js` remain byte-for-byte unchanged.
- `pnpm --dir ui svelte-check`, `pnpm --dir ui lint`, and `pnpm --dir ui build` all pass.
</success_criteria>

<output>
Create `.planning/quick/260805-ifj-lan-print-neutralize-app-line-height-lea/260805-ifj-SUMMARY.md` when done
</output>

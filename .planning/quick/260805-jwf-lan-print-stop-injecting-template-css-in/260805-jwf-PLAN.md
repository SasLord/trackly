---
quick_id: 260805-jwf
slug: lan-print-stop-injecting-template-css-fix-pagination-mismatch
phase: 260805-jwf
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - ui/src/features/acts/PdfPreviewModal.svelte
autonomous: true
requirements: [JWF-01, JWF-02]
must_haves:
  truths:
    - "After printing an act from a LAN browser and the print dialog closes, the app's own on-screen font/typography is unchanged — no more DejaVu Sans/Arial fallback surviving until reload (defect A: printStyle no longer injects ${cssText} into the app document, and both printStyle.textContent AND Paged.js's own Polisher-inserted <style data-pagedjs-inserted-styles> elements are removed on afterprint)"
    - "The LAN browser's own print-preview dialog shows the SAME page breaks as the on-screen Paged.js preview the user already approved — no more stretched/squeezed mismatch (defect B: the line-height/letter-spacing/word-spacing reset is in effect BEFORE previewer.preview() measures #act-print-root, not only under @media print)"
    - "A template that legitimately declares its own line-height on a specific element (e.g. Trackly's .header .requisites { line-height: 1.35 }) still wins for that element — the reset only changes the AMBIENT inherited default, it does not use !important and does not target elements a template rule already matches directly"
    - "Load-bearing @media print rules from prior fixes (260805-gdz/har) are unchanged: html/body and .pagedjs_page background: #fff !important (both occurrences), body > :not(#act-print-root) { display: none !important }, and #act-print-root's position: static; left: auto; reset"
    - "printViaSystemBrowser (desktop reference path, confirmed correct on paper) is byte-for-byte unchanged"
    - "ui/src/lib/pdfPreview/bootstrapScript.js is byte-for-byte unchanged (CSP-hash gate still passes)"
  artifacts:
    - path: "ui/src/features/acts/PdfPreviewModal.svelte"
      provides: "printViaTopLevel no longer injects ${cssText} into printStyle.textContent (Paged.js's own Polisher already applies the same stylesheet via previewer.preview()'s stylesheets argument); the line-height/letter-spacing/word-spacing reset moves out of @media print and is scoped to #act-print-root so it applies unconditionally (screen AND print) without ever touching the app's own body; afterprint cleanup now clears printStyle.textContent and calls the captured Previewer's polisher.destroy() to remove Paged.js's own injected <style> elements, so nothing survives a print cycle"
      contains: "#${PRINT_ROOT_ID} {"
  key_links:
    - from: "printViaTopLevel's printStyle.textContent template literal"
      to: "document.head, applied before previewer.preview() runs"
      via: "#${PRINT_ROOT_ID} { line-height: normal; letter-spacing: normal; word-spacing: normal; ... } declared OUTSIDE any @media print block, so it is already in the cascade (and thus reflected in getComputedStyle/getBoundingClientRect) at the moment Paged.js's Chunker measures #act-print-root's content to decide page breaks"
      pattern: "line-height: normal"
    - from: "previewer.preview(bodyHtml, [{ 'act-preview.css': cssText }], printRoot)"
      to: "afterprint cleanup"
      via: "previewer.polisher captured into a closure variable after preview() resolves; cleanup calls injectedPolisher?.destroy() (Polisher.destroy() removes this.styleEl plus every style element in this.inserted — i.e. the base pagedjs styles AND the template cssText it inserted into document.head during preview())"
      pattern: "\\.destroy\\(\\)"
---

<objective>
Close out the LAN-print defect chain (260805-edd -> gdz -> har -> ifj) with two fixes in the SAME
function, `printViaTopLevel` in `ui/src/features/acts/PdfPreviewModal.svelte` — the only file this
plan touches.

**Governing principle** (state explicitly, both defects are instances of it, future edits must
respect it): `printViaTopLevel` renders Paged.js output into the APP's live DOM. Paged.js decides
page breaks by MEASURING that DOM on screen, before `window.print()` is called. Therefore anything
that affects document layout must be in effect at measurement time — not only inside
`@media print`. `@media print` may carry only visibility concerns (what to hide/show) and
paint-only properties (backgrounds). Any layout-affecting rule placed in `@media print` alone
guarantees a mismatch between the pagination the user was shown and what the printer receives.

**Defect A — font leak onto the app UI.** `printStyle.textContent` currently interpolates the
template's raw `${cssText}` unscoped into `document.head`. That CSS contains
`body { font-family: "DejaVu Sans", "Arial", sans-serif; ... }`, which therefore also applies to
the app's OWN `body`. On Windows there is no DejaVu Sans, so the UI falls back to Arial instead of
`--tr-font-family` after the first LAN print of a session, until reload. This manual injection is
redundant: Paged.js already receives the identical CSS as the `stylesheets` argument of
`previewer.preview(bodyHtml, [{ 'act-preview.css': cssText }], printRoot)` — that is how the
correct fonts already reach the printed act today (confirmed against `printViaSystemBrowser`,
which uses no manual style injection at all and prints correctly).

Reading `node_modules/pagedjs/dist/paged.esm.js` (`Previewer.preview` -> `Polisher.setup`/`add` ->
`Polisher.insert`) confirms Paged.js's OWN mechanism for consuming that `stylesheets` argument is
`document.querySelector("head").appendChild(<style data-pagedjs-inserted-styles>...)` — i.e. it is
JUST AS unscoped as the manual injection this plan removes, applying directly to the shared
top-level `document` that `printViaTopLevel` renders into (unlike the isolated opaque-origin
preview iframe in `pagedPreviewBootstrap.ts`, where the same mechanism is harmless because it is a
separate document). `Polisher.destroy()` (`this.styleEl.remove(); this.inserted.forEach(s =>
s.remove())`) is the only way to remove those elements again, and nothing currently calls it. So
removing the manual `${cssText}` duplicate is necessary but NOT sufficient on its own — this plan
also captures the `Previewer`'s `polisher` after `preview()` resolves and calls its `.destroy()`
from the `afterprint` cleanup, alongside clearing `printRoot.innerHTML` and `printStyle.textContent`
(both already load-bearing per governing principle: nothing must survive a print cycle).

**Defect B — pagination mismatch (regression from 260805-ifj).** That prior task added
`@media print { body { line-height: normal; ... } }` to fix stretched print output. But Paged.js
paginates the on-screen DOM, where the app's `line-height: 1.5` (global.scss) was still in effect;
the reset then applied only at print time. Page boxes were computed for 1.5-spaced text and filled
with `normal`-spaced text — a mismatch between the browser's own print-preview dialog and the
on-screen Paged.js preview the user already approved.

Fix: move the reset out of `@media print` (apply unconditionally — screen AND print) and scope it
to `#act-print-root` instead of `body`, so it is in effect at measurement time and never touches
the app's own on-screen typography. Verified via source read (see task 1 action) that an element's
own declared value for an inherited property (like `line-height`) always wins over what it would
otherwise inherit from an ancestor, with NO `!important` needed — so `#act-print-root`'s own
`line-height: normal` overrides the app `body`'s `1.5` for everything inside `#act-print-root`,
while any template rule that targets a MORE SPECIFIC descendant selector directly (grepped: only
`.header .requisites { line-height: 1.35 }` across all three shipped templates — none declare
`body { line-height }`) still wins for that element, because a rule matching an element directly
always beats an inherited value regardless of the ancestor rule's specificity. Chose this
(container-scoped, unconditional, no `!important`) deliberately as the option that fails safe
toward matching `printViaSystemBrowser`'s output (the confirmed-correct desktop reference, which
never has the app's stylesheet loaded at all) — documented here per this plan's own instructions,
not presented as an absolute certainty for a hypothetical future template that declares
`body { line-height }` directly (none currently do).

Purpose: LAN-browser printing must match both the on-screen Paged.js preview the user approved AND
the already-correct desktop print path, without corrupting the app's own on-screen UI.

Output: `printViaTopLevel` no longer duplicates `${cssText}` into the app document; its layout
reset applies unconditionally and is scoped to `#act-print-root`; its `afterprint` cleanup removes
everything a print cycle created (print-root content, printStyle rules, Paged.js's own injected
styles).
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@CLAUDE.md
@.planning/STATE.md

<interfaces>
<!-- Current printViaTopLevel — ui/src/features/acts/PdfPreviewModal.svelte, lines ~338-453
     (search "async function printViaTopLevel"). Everything below this point in that function is
     what this plan touches; nothing above it (parsing bodyHtml/cssText from the backend HTML) or
     outside the function changes. -->

Current tail of the function (load-bearing structural facts, do not lose any of them):
- `printStyle.textContent` currently opens with a `@media print { body { line-height: normal;
  letter-spacing: normal; word-spacing: normal; } }` block (260805-ifj), THEN interpolates
  `${cssText}` raw, THEN `#${PRINT_ROOT_ID} { position: absolute; left: -100000px; top: 0; }`,
  THEN a second `@media print { ... }` block containing (in this order): `html, body { background:
  #fff !important; }`, `body > :not(#${PRINT_ROOT_ID}) { display: none !important; }`,
  `#${PRINT_ROOT_ID} { display: block !important; position: static; left: auto; }`,
  `.pagedjs_page { background: #fff !important; }`.
- `cleanup` (bound to `window`'s `afterprint` event) currently only does `printRoot!.innerHTML =
  ''` and removes its own listener.
- The function ends with: create `Previewer`, `await previewer.preview(bodyHtml, [{
  'act-preview.css': cssText }], printRoot)`, then `window.focus(); window.print();`.

Paged.js internals relied on by this plan (read from
`ui/node_modules/pagedjs/dist/paged.esm.js`, non-minified ESM build — do not edit, reference only):
- `Previewer.preview(content, stylesheets, renderTo)` calls `this.polisher.setup()` then `await
  this.polisher.add(...stylesheets)` BEFORE `this.chunker.flow(content, renderTo)` measures/paginates
  — confirms the stylesheet is already applied to the document before chunking/measurement starts.
- `Polisher.insert(text)`: `document.querySelector("head").appendChild(<style
  data-pagedjs-inserted-styles>text</style>)` — unscoped, same document as the app.
- `Polisher.destroy()`: `this.styleEl.remove(); this.inserted.forEach(s => s.remove());` — removes
  every style element `insert()` ever created for this Polisher instance (base pagedjs styles +
  each stylesheet passed to `add()`, i.e. the template's `cssText`).
- `Previewer`'s constructor exposes `this.polisher` as a plain instance property (no TS types ship
  with `pagedjs`, so `previewer.polisher` type-checks as `any` — no cast needed).
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Stop injecting cssText into the app document; fix the measure-vs-print mismatch; destroy Paged.js's own injected styles on cleanup</name>
  <files>ui/src/features/acts/PdfPreviewModal.svelte</files>
  <action>
Edit ONLY `printViaTopLevel` (see `<interfaces>` for its current structure). Three changes, all in
this one function:

**1. `printStyle.textContent` — remove the old two-block structure, replace with ONE consolidated,
unconditional `#${PRINT_ROOT_ID}` rule plus the untouched second `@media print` block:**

Delete the entire `@media print { body { line-height: normal; letter-spacing: normal;
word-spacing: normal; } }` block added by 260805-ifj (comment and code). Delete the
`${cssText}` interpolation line entirely — do not reference `cssText` anywhere inside
`printStyle.textContent` (it stays a used variable elsewhere — see change 3). Merge the
line-height reset and the existing `#${PRINT_ROOT_ID} { position: absolute; left: -100000px; top:
0; }` rule into ONE rule block for `#${PRINT_ROOT_ID}`, declared OUTSIDE any `@media print` block
(applies unconditionally, screen and print alike):

    #${PRINT_ROOT_ID} {
      line-height: normal;
      letter-spacing: normal;
      word-spacing: normal;
      position: absolute;
      left: -100000px;
      top: 0;
    }

Immediately below it, keep the existing second `@media print { ... }` block EXACTLY as it is today
— byte-for-byte: the `html, body { background: #fff !important; }` rule, the
`body > :not(#${PRINT_ROOT_ID}) { display: none !important; }` rule, the `#${PRINT_ROOT_ID} {
display: block !important; position: static; left: auto; }` rule, and the `.pagedjs_page {
background: #fff !important; }` rule, in that order. Do not reorder, merge, or drop any of these
four rules — `position: static; left: auto;` must still fire AFTER the unconditional `position:
absolute; left: -100000px;` above it (media-query specificity handles that; do not change the
selector or nesting to anything that would break it).

Replace the inline comment(s) above this template literal with new prose (place it directly above
`printStyle.textContent = ` \`` at the point of insertion — do not just describe it in this plan)
that states, in your own words grounded in the objective above: (a) why the reset is scoped to
`#${PRINT_ROOT_ID}` and not `body` (never touch the app's own on-screen typography — that IS
defect A's mechanism if you get this wrong), (b) why it is NOT inside `@media print` (Paged.js
measures on-screen, before `window.print()` — that is defect B), (c) that `${cssText}` is
deliberately NOT interpolated here anymore because Paged.js's own `Previewer.preview()` call below
already applies the identical stylesheet, and (d) a one-line pointer to the `Previewer.polisher`
capture + `afterprint` `.destroy()` call (change 3 below) as the mechanism that prevents Paged.js's
own injected styles from surviving past the print cycle — without it, removing the manual
duplicate here would not actually fix the reported "font leaks until reload" defect, since
Paged.js's `Polisher.insert()` is just as unscoped as the code being removed.

**2. Preserve the off-screen-vs-display:none rationale comment.** The existing large comment
explaining why `#${PRINT_ROOT_ID}` uses `position: absolute; left: -100000px` instead of `display:
none` (so Paged.js's `getBoundingClientRect` calls still see real geometry) is unrelated to this
plan's two defects and must survive unchanged (moved as needed to stay attached to the merged rule
block, but not reworded or removed).

**3. `cleanup` and the `Previewer`/`Polisher` capture — nothing survives a print cycle:**

Declare a new closure variable before `cleanup` is defined, e.g. `let injectedPolisher: { destroy:
() => void } | null = null;` (typed loosely since `pagedjs` ships no `.d.ts`; do not add a runtime
dependency on pagedjs's types). Extend `cleanup` to, in addition to the existing `printRoot!.innerHTML
= ''`: clear `printStyle!.textContent = ''`, call `injectedPolisher?.destroy();`, and reset
`injectedPolisher = null;` — before the existing `window.removeEventListener('afterprint',
cleanup);` line. Immediately after `await previewer.preview(bodyHtml, [{ 'act-preview.css':
cssText }], printRoot);` resolves (still using the SAME `cssText` variable already extracted at the
top of the function — do not remove or rename it), assign `injectedPolisher = previewer.polisher;`
so `cleanup` (which was registered on `window` before `preview()` ran) has a live reference by the
time the browser fires `afterprint`. Do not change where `window.addEventListener('afterprint',
cleanup)` is registered, and do not change `window.focus(); window.print();` at the end of the
function.

Do not modify: `printViaSystemBrowser`, `ui/src/lib/pdfPreview/bootstrapScript.js`,
`ui/src/styles/global.scss`, `ui/src/styles/_tokens.scss`, any file under
`crates/trackly-app/templates/`, `ui/src/lib/components/Modal.svelte`, or anything in
`printViaTopLevel` above the `printStyle` setup (the `DOMParser`/`bodyHtml`/`styleHtml`/`cssText`
extraction and the `printRoot`/`printStyle` element creation stay as they are).
  </action>
  <verify>
    <automated>node -e "
const fs = require('fs');
const path = '/Users/madsas/Projects/trackly/ui/src/features/acts/PdfPreviewModal.svelte';
const s = fs.readFileSync(path, 'utf8');
const fnMatch = s.match(/async function printViaTopLevel\(html[\s\S]*?\n}\n/);
if (!fnMatch) { console.log('FAIL_NO_FUNCTION'); process.exit(1); }
const fn = fnMatch[0];
const styleMatch = fn.match(/printStyle\.textContent = \`([\s\S]*?)\`;/);
if (!styleMatch) { console.log('FAIL_NO_STYLE_LITERAL'); process.exit(1); }
const styleLit = styleMatch[1];
if (styleLit.includes('\${cssText}')) { console.log('FAIL_CSSTEXT_STILL_DUPLICATED'); process.exit(1); }
if (!fn.includes(\"'act-preview.css': cssText\")) { console.log('FAIL_CSSTEXT_NOT_PASSED_TO_PAGEDJS'); process.exit(1); }
const rootIdx = styleLit.indexOf('#\${PRINT_ROOT_ID} {');
if (rootIdx === -1) { console.log('FAIL_NO_ROOT_RULE'); process.exit(1); }
const before = styleLit.slice(0, rootIdx);
const lastMedia = before.lastIndexOf('@media print');
if (lastMedia !== -1 && before.slice(lastMedia).indexOf('}') === -1) { console.log('FAIL_STILL_MEDIA_GATED'); process.exit(1); }
const rootBlock = styleLit.slice(rootIdx, styleLit.indexOf('}', rootIdx));
if (!rootBlock.includes('line-height: normal')) { console.log('FAIL_NO_LINE_HEIGHT'); process.exit(1); }
if (!rootBlock.includes('letter-spacing: normal') || !rootBlock.includes('word-spacing: normal')) { console.log('FAIL_NO_SPACING_RESET'); process.exit(1); }
if (!rootBlock.includes('position: absolute') || !rootBlock.includes('left: -100000px')) { console.log('FAIL_NO_OFFSCREEN_POSITION'); process.exit(1); }
const bgCount = (styleLit.match(/background: #fff !important/g) || []).length;
if (bgCount !== 2) { console.log('FAIL_BACKGROUND_RULES_' + bgCount); process.exit(1); }
if (!styleLit.includes('display: none !important')) { console.log('FAIL_NO_CHROME_HIDE'); process.exit(1); }
if (!styleLit.includes('position: static;') || !styleLit.includes('left: auto;')) { console.log('FAIL_NO_PRINT_POSITION_RESET'); process.exit(1); }
if (!fn.includes(\"printStyle!.textContent = ''\") && !fn.includes('printStyle!.textContent = \"\"')) { console.log('FAIL_PRINTSTYLE_NOT_CLEARED_ON_CLEANUP'); process.exit(1); }
if (!fn.includes('previewer.polisher')) { console.log('FAIL_NO_POLISHER_CAPTURE'); process.exit(1); }
if (!/injectedPolisher\?\.destroy\(\)|injectedPolisher\.destroy\(\)/.test(fn)) { console.log('FAIL_NO_POLISHER_DESTROY_CALL'); process.exit(1); }
console.log('OK_DEFECT_A_AND_B_GATES_PASS');
"
grep -c "async function printViaSystemBrowser" /Users/madsas/Projects/trackly/ui/src/features/acts/PdfPreviewModal.svelte | grep -qx 1 && echo OK_SYSTEM_BROWSER_PRESENT || echo FAIL_SYSTEM_BROWSER_MISSING
cd /Users/madsas/Projects/trackly/ui && pnpm svelte-check 2>&1 | tail -20 && pnpm lint 2>&1 | tail -30 && pnpm build 2>&1 | tail -20</automated>
  </verify>
  <done>
`printStyle.textContent` in `printViaTopLevel` no longer interpolates `${cssText}`; its
`#${PRINT_ROOT_ID}` rule (line-height/letter-spacing/word-spacing reset + off-screen positioning,
merged into one block) is declared unconditionally, outside any `@media print` block; the
pre-existing second `@media print { ... }` block (two background resets, chrome display:none,
position:static/left:auto) is unchanged. `cleanup` clears `printRoot.innerHTML`,
`printStyle.textContent`, and calls the captured `Previewer`'s `polisher.destroy()`. `cssText` is
still passed to `previewer.preview()`. `printViaSystemBrowser` and
`ui/src/lib/pdfPreview/bootstrapScript.js` are unmodified. `pnpm svelte-check`, `pnpm lint`
(including the CSP-hash gate), and `pnpm build` all pass clean.
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| LAN browser client -> axum-served SPA | No new trust boundary crossed — pure CSS/print-lifecycle change scoped to `printViaTopLevel`, touching no data flow, no new input parsing, no new network surface. |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-jwf-01 | Tampering | `printStyle.textContent` / `cssText` (existing extraction from backend-rendered act HTML, unchanged by this plan) | accept | This plan only removes a redundant interpolation of the SAME already-accepted `cssText` value and adds static hardcoded CSS (no new untrusted input). The backend-rendered HTML this is extracted from was already same-origin, backend-authored content — pre-existing accepted risk, not reopened here. |
| T-jwf-02 | Denial of Service (resource leak) | `Previewer`/`Polisher` instances created per print click, previously never `.destroy()`-ed | mitigate | This plan's own fix: `injectedPolisher.destroy()` in `afterprint` cleanup removes the DOM elements Paged.js accumulates in `document.head` on every print, preventing unbounded `<style data-pagedjs-inserted-styles>` growth across repeated LAN prints in one session. |
| T-jwf-SC | Tampering (supply chain) | N/A | accept | No new dependency, no package install — edits an existing `.svelte` file only. Package Legitimacy Gate not applicable. |
</threat_model>

<verification>
1. `pnpm --dir ui svelte-check` — 0 errors.
2. `pnpm --dir ui lint` — clean, including the CSP-hash gate (`bootstrapScript.js` untouched).
3. `pnpm --dir ui build` — succeeds.
4. Node structural gate (Task 1 `<verify>`) — proves: no `${cssText}` duplication inside
   `printStyle.textContent`; `cssText` still reaches Paged.js via `previewer.preview()`; the
   line-height/letter-spacing/word-spacing reset is scoped to `#act-print-root` and NOT nested
   inside `@media print`; all four pre-existing `@media print` rules are intact; `cleanup` clears
   `printStyle.textContent` and calls the captured polisher's `.destroy()`.
5. Manual/human-check (NOT automatable — no frontend test framework exists, and neither printed
   pagination nor on-screen app font after a print cycle can be asserted by any command above;
   same class of gap as every prior task in this chain, 260805-edd/gdz/har/ifj): from a real LAN
   client (ideally Windows, matching the reported environment), open an act preview via the LAN
   server URL and:
   a. Click "Печать", let the on-screen Paged.js preview finish, note its page count/breaks, then
      open the browser's own print-preview dialog and confirm the SAME page breaks appear (defect
      B check).
   b. Complete or cancel the print, close the dialog, and confirm the Trackly app's OWN UI (menus,
      buttons, any text) still renders in its normal font — no fallback to Arial/serif anywhere,
      with no reload needed (defect A check).
   c. Repeat a full print cycle a second time in the same browser session (no reload between) to
      confirm no `<style data-pagedjs-inserted-styles>` accumulation or leftover font leak from the
      first cycle.
   If this plan is executed without LAN/Windows access, flag steps a-c as a pending follow-up UAT
   in the SUMMARY, matching the precedent set by 260805-edd/gdz/har/ifj.
</verification>

<success_criteria>
- LAN-browser print (`printViaTopLevel`) no longer duplicates the template's CSS into the app
  document; Paged.js's own `stylesheets` argument remains the single source for print typography.
- The app's own on-screen typography is never affected by a print cycle, before or after —
  including cleanup removing Paged.js's own `<style data-pagedjs-inserted-styles>` elements.
- The on-screen Paged.js preview and the browser's native print-preview dialog agree on page
  breaks — the reset is in effect at measurement time, not only under `@media print`.
- A template's own explicit `line-height` on a specific selector (verified: none of the three
  shipped templates declare one on `body` itself) still wins for that element.
- No regression to any pre-existing `@media print` rule (backgrounds, chrome hide, position reset).
- `printViaSystemBrowser` and `bootstrapScript.js` remain byte-for-byte unchanged.
- `pnpm --dir ui svelte-check`, `pnpm --dir ui lint`, and `pnpm --dir ui build` all pass.
</success_criteria>

<output>
Create `.planning/quick/260805-jwf-lan-print-stop-injecting-template-css-in/260805-jwf-SUMMARY.md` when done
</output>

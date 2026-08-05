---
quick_id: 260805-har
slug: lan-print-neutralize-app-body-background
phase: 260805-har
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - ui/src/features/acts/PdfPreviewModal.svelte
autonomous: true
requirements: [HAR-01]
must_haves:
  truths:
    - "Printing an act from a LAN browser (printViaTopLevel) produces a white sheet background — no grey app-body (--tr-bg) bleeding into the printed output, matching desktop print (printViaSystemBrowser), which was already confirmed correct"
    - "The printed sheet is white in both light and dark theme (D-08: paper is always white, never theme-dependent)"
    - "The existing load-bearing rules in the same @media print block — body > :not(#act-print-root) { display: none } and #act-print-root's position:static/left:auto reset — are unchanged"
    - "printViaSystemBrowser (desktop print path) and ui/src/lib/pdfPreview/bootstrapScript.js (CSP-hash-locked) are untouched"
  artifacts:
    - path: "ui/src/features/acts/PdfPreviewModal.svelte"
      provides: "printViaTopLevel's injected @media print block neutralizes html/body and .pagedjs_page backgrounds to literal #fff"
      contains: "html, body {"
  key_links:
    - from: "ui/src/features/acts/PdfPreviewModal.svelte printStyle template literal"
      to: "@media print block"
      via: "html, body { background: #fff !important; } and .pagedjs_page { background: #fff !important; } added alongside the existing display:none / position:static rules"
      pattern: "background: #fff !important"
---

<objective>
Fix a LAN-only print defect: printing an act from a browser connected to the LAN server (not
the desktop app) shows a grey background instead of a white sheet. Desktop print
(`printViaSystemBrowser`) already prints white and is the reference behaviour.

Root cause (confirmed by reading the code, not inferred): `printViaTopLevel` renders Paged.js
output into the Trackly app's OWN DOM (unlike the desktop path, which writes a self-contained
temp HTML file with no app stylesheets present). `ui/src/styles/global.scss` sets
`body { background: var(--tr-bg); }` (`--tr-bg: #eef1f6` light / `#0e1218` dark, per
`_tokens.scss`). The injected `@media print` block in `printViaTopLevel` hides
`body > :not(#act-print-root)` but never neutralizes `body`'s own background, so it survives
into print. It also never sets an explicit white background on `.pagedjs_page` — the on-screen
preview iframe already does this via `pagedPreviewBootstrap.ts`'s `buildSrcdoc`
(`.pagedjs_page { background: #fff; }`, per locked decision D-08: "the sheet is ALWAYS white —
it is paper"), but the print path has no equivalent, so the sheet is transparent and the grey
`body` shows through it.

Purpose: printed output from the LAN path must match the already-correct desktop path — a
white sheet, in both light and dark theme, per D-08.

Output: two new declarations inside the existing `@media print` block of `printViaTopLevel`'s
injected `printStyle`.
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
     around line 394-411 (search for "printStyle.textContent ="). -->
```javascript
printStyle.textContent = `
  ${cssText}
  #${PRINT_ROOT_ID} {
    position: absolute;
    left: -100000px;
    top: 0;
  }
  @media print {
    body > :not(#${PRINT_ROOT_ID}) {
      display: none !important;
    }
    #${PRINT_ROOT_ID} {
      display: block !important;
      position: static;
      left: auto;
    }
  }
`;
```

<!-- Source of the grey — ui/src/styles/global.scss line ~29 (DO NOT EDIT, reference only) -->
```scss
body {
  background: var(--tr-bg);
}
```
`--tr-bg` (ui/src/styles/_tokens.scss): `#eef1f6` in light theme (~line 24), `#0e1218` in dark
theme (~line 101) — proves a literal `#fff !important` inside `@media print` is required, not a
`--tr-*` token reference (a token would still resolve to near-black in dark theme).

<!-- Precedent this plan mirrors — ui/src/lib/pdfPreview/pagedPreviewBootstrap.ts's buildSrcdoc,
     READ ONLY, do not edit. Already solves the identical problem for the on-screen preview
     iframe. -->
```javascript
`.pagedjs_page { box-shadow: ${chrome.shadow}; background: #fff; }` +
```
Comment above that line (verbatim from the file) explains why `#fff` is literal there: "this
iframe is opaque-origin and cannot resolve the parent app's tokens" — not the same reason as
this plan's case (this plan's DOM IS the app's own document and COULD resolve tokens), but the
outcome must still be a literal `#fff` per D-08 (paper is theme-independent, not just
iframe-isolation-independent).
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Neutralize app body background and force white sheet in LAN print path</name>
  <files>ui/src/features/acts/PdfPreviewModal.svelte</files>
  <action>
In `printViaTopLevel`'s `printStyle.textContent` template literal (see `<interfaces>` above for
the exact current text — search the file for "printStyle.textContent ="), add two declarations
inside the existing `@media print { ... }` block, without changing anything else in that block:

1. As the FIRST rule inside `@media print { ... }` (before the existing
   `body > :not(#${PRINT_ROOT_ID})` rule): `html, body { background: #fff !important; }` — this
   neutralizes the app's own `--tr-bg` page background (set globally by `global.scss`, see
   `<interfaces>`) so it cannot bleed into the printed page. Ordering relative to the other
   rules does not matter functionally (no cascade conflict — different selectors), but placing
   it first keeps the block read top-to-bottom as "reset background, then hide app chrome, then
   show print root."
2. As a new rule inside the SAME `@media print { ... }` block (after the existing
   `#${PRINT_ROOT_ID}` rule): `.pagedjs_page { background: #fff !important; }` — makes the
   sheet itself explicitly white, mirroring what `pagedPreviewBootstrap.ts`'s `buildSrcdoc`
   already does for the on-screen preview per D-08 ("the sheet is ALWAYS white — it is paper").

Use the literal value `#fff` in both new rules — NOT a `--tr-*` custom property. The app's
`body` background must NOT follow the current theme inside `@media print` (D-08: paper is
white in both light and dark theme; the dark-theme `--tr-bg` token resolves to `#0e1218`, which
would reproduce the exact defect this plan fixes).

Do not modify:
- The existing `body > :not(#${PRINT_ROOT_ID}) { display: none !important; }` rule.
- The existing `#${PRINT_ROOT_ID} { display: block !important; position: static; left: auto; }`
  rule — the `position: static; left: auto` reset is load-bearing (quick task 260805-gdz);
  without it printed output goes off-sheet.
- Anything outside `printViaTopLevel` — `printViaSystemBrowser`,
  `ui/src/lib/pdfPreview/bootstrapScript.js` (CSP-hash-locked), `ui/src/styles/global.scss`,
  `ui/src/styles/_tokens.scss`, templates, or `Modal.svelte`.
  </action>
  <verify>
    <automated>grep -n -A25 "printStyle.textContent = " /Users/madsas/Projects/trackly/ui/src/features/acts/PdfPreviewModal.svelte | grep -c "background: #fff !important" | grep -qx 2 && echo OK_TWO_FFF_RULES || echo FAIL_MISSING_FFF_RULES; grep -c "position: static;" /Users/madsas/Projects/trackly/ui/src/features/acts/PdfPreviewModal.svelte | grep -qx 1 && echo OK_POSITION_RESET_INTACT || echo FAIL_POSITION_RESET_CHANGED; grep -q "async function printViaSystemBrowser" /Users/madsas/Projects/trackly/ui/src/features/acts/PdfPreviewModal.svelte && echo OK_SYSTEM_BROWSER_PRESENT || echo FAIL_SYSTEM_BROWSER_MISSING; cd /Users/madsas/Projects/trackly/ui && pnpm svelte-check 2>&1 | tail -20 && pnpm lint 2>&1 | tail -30 && pnpm build 2>&1 | tail -20</automated>
  </verify>
  <done>
`printStyle.textContent` in `printViaTopLevel` contains exactly two new `background: #fff
!important;` declarations — one on an `html, body` selector, one on `.pagedjs_page` — both
inside the existing `@media print { ... }` block. The pre-existing `body > :not(#act-print-root)
{ display: none !important; }` rule and the `position: static; left: auto` reset on
`#act-print-root` are unchanged. `printViaSystemBrowser` is present and unmodified.
`pnpm svelte-check`, `pnpm lint` (including the CSP-hash gate), and `pnpm build` all pass clean.
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
| T-har-01 | Tampering | `printStyle.textContent` (existing `cssText`/`bodyHtml` injection point, unchanged by this plan) | accept | Out of scope for this change — this plan adds two static, hardcoded CSS declarations (`background: #fff !important`) with no interpolation of untrusted input. The pre-existing `cssText`/`bodyHtml` extraction from the backend-rendered act HTML is unchanged and was already accepted risk (same-origin backend-authored content, not user-supplied). |
| T-har-SC | Tampering (supply chain) | N/A | accept | No new dependency, no package install — pure template-literal string edit in an existing `.svelte` file. Package Legitimacy Gate not applicable. |
</threat_model>

<verification>
1. `pnpm --dir ui svelte-check` — 0 errors.
2. `pnpm --dir ui lint` — clean, including the CSP-hash gate (`bootstrapScript.js` untouched, so
   the hash cannot drift).
3. `pnpm --dir ui build` — succeeds.
4. Manual/human-check (NOT automatable — no frontend test framework exists and printed-output
   colour cannot be asserted by any of the above commands; this needs a real browser on another
   machine printing against the axum server, same class of check as the LAN print fixes in
   260805-edd/260805-gdz): from a Windows LAN client, open the act preview via
   `web.cmy.local:8443` (or equivalent LAN URL), click Печать, and in the print preview/dialog
   confirm the sheet background is white — both in the app's light theme and dark theme. Record
   the result in the SUMMARY; if this plan is executed without LAN/Windows access, flag step 4
   as a pending follow-up UAT, matching the precedent set by the prior two print-fix quick tasks
   in this same defect chain.
</verification>

<success_criteria>
- LAN-browser print (`printViaTopLevel`) produces a white sheet, matching desktop print
  (`printViaSystemBrowser`), in both light and dark theme.
- No regression to the existing `display: none` app-chrome hiding rule or the
  `position: static; left: auto` off-screen-to-on-page reset for `#act-print-root`.
- `printViaSystemBrowser` and `bootstrapScript.js` remain byte-for-byte unchanged.
- `pnpm --dir ui svelte-check`, `pnpm --dir ui lint`, and `pnpm --dir ui build` all pass.
</success_criteria>

<output>
Create `.planning/quick/260805-har-lan-print-neutralize-app-body-background/260805-har-SUMMARY.md` when done
</output>

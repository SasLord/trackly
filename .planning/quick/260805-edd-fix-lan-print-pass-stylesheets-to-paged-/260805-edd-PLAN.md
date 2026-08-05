---
quick_id: 260805-edd
slug: lan-print-stylesheets-object-not-string
phase: 260805-edd
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - ui/src/features/acts/PdfPreviewModal.svelte
autonomous: true
requirements: [EDD-01]
must_haves:
  truths:
    - "Pressing «Печать» in the preview modal from a real LAN browser (non-Tauri) no longer fires a network request whose URL is the CSS text itself (the observed `/%3Cstyle%3E...` failed request)"
    - "Paged.js's Polisher.add() receives the preview stylesheet as an object value (CSS text used directly), not a bare string (which Paged.js treats as a URL to fetch)"
    - "printViaTopLevel no longer retries with a second string-shaped preview() call on failure — a failed preview() now surfaces through handlePrint()'s existing catch/toast instead of masking the real error with an identically-broken fallback"
    - "printViaSystemBrowser (the Tauri desktop print path) is untouched"
  artifacts:
    - path: "ui/src/features/acts/PdfPreviewModal.svelte"
      provides: "printViaTopLevel() passes the preview CSS to Paged.js Previewer.preview() as an object keyed by a synthetic filename, not a string"
      contains: "previewer.preview(bodyHtml, [{ 'act-preview.css': cssText }], printRoot)"
  key_links:
    - from: "printViaTopLevel"
      to: "Paged.js Previewer.preview / Polisher.add"
      via: "object-shaped stylesheets argument (CSS text as the object value, not a string)"
      pattern: "previewer\\.preview\\(bodyHtml, \\[\\{"
---

<objective>
Fix LAN-browser print failure: `printViaTopLevel` in the preview modal passes Paged.js's
`Previewer.preview(content, stylesheets, renderTo)` a STRING for `stylesheets`
(`[styleHtml]`, a `<style>...</style>`-wrapped string built from `.outerHTML`). Paged.js's
`Polisher.add()` (`ui/node_modules/pagedjs/dist/paged.js` ~L27506) branches on
`typeof arguments[i]`: an `object` argument's values are used directly as CSS text; anything
else is pushed into `urls` and fetched over the network via `request(arguments[i])`. A string
is therefore treated as a URL — confirmed by the reported DevTools Network entry
(`https://web.cmy.local:8443/%3Cstyle%3E%20%20@page%20...`), which is the CSS text itself,
URL-encoded, hitting the LAN server as a bogus path and failing. The existing `catch`
fallback retries with `[styleHtml.replace(/<\/?style[^>]*>/gi, '')]` — still a plain string
(only the tag-wrapping changed, not the type), so it fails identically and never actually
recovers.

Purpose: LAN-browser users (the only path this function serves — desktop uses
`printViaSystemBrowser`, untouched) must be able to print/save-as-PDF from the preview modal.

Output: `printViaTopLevel` passes stylesheets to Paged.js as `[{ 'act-preview.css': cssText }]`
(object shape); the redundant same-shape-bug fallback is removed so a genuine future failure
surfaces through the existing `handlePrint()` catch/toast instead of being masked by a second,
equally broken attempt.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@CLAUDE.md
@.planning/STATE.md

<interfaces>
<!-- ui/src/features/acts/PdfPreviewModal.svelte — printViaTopLevel, CURRENT (broken) shape.
     Full function is already in scope for the Edit tool; this excerpt exists only to pin the
     exact before/after text so the edit is unambiguous. -->
```typescript
async function printViaTopLevel(html: string) {
    const parsed = new DOMParser().parseFromString(html, 'text/html');
    const bodyHtml = parsed.body?.innerHTML ?? '';
    const styleHtml = Array.from(parsed.head?.querySelectorAll('style') ?? [])
      .map((el) => el.outerHTML)
      .join('\n');

    // ... printRoot / printStyle DOM setup (UNCHANGED by this plan) ...

    printStyle.textContent = `
      ${styleHtml.replace(/<\/?style[^>]*>/gi, '')}
      @media print { /* ... */ }
      @media screen { /* ... */ }
    `;

    // ... cleanup / afterprint listener (UNCHANGED) ...

    const { Previewer } = await import('pagedjs');
    const previewer = new Previewer();
    try {
      await previewer.preview(bodyHtml, [styleHtml], printRoot);
    } catch {
      printRoot.innerHTML = '';
      await previewer.preview(bodyHtml, [styleHtml.replace(/<\/?style[^>]*>/gi, '')], printRoot);
    }

    window.focus();
    window.print();
  }
```

<!-- Confirmed root cause, ui/node_modules/pagedjs/dist/paged.js ~L27506 (Polisher.add) -->
```javascript
async add() {
  for (var i = 0; i < arguments.length; i++) {
    if (typeof arguments[i] === "object") {
      for (let url in arguments[i]) { /* ... resolve(arguments[i][url]) — used as CSS TEXT ... */ }
    } else {
      urls.push(arguments[i]);
      f = request(arguments[i]).then((r) => r.text());   // STRING is fetched as a URL
    }
  }
}
```
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Pass stylesheets to Paged.js Polisher as an object, not a string</name>
  <files>ui/src/features/acts/PdfPreviewModal.svelte</files>
  <action>
In `printViaTopLevel` (the function documented above — do NOT touch `printViaSystemBrowser`,
`bootstrapScript.js`, `templates/*.html`, or `Modal.svelte`):

1. Immediately after the existing `styleHtml` declaration (the `<style>`-wrapped string built
   from `.map((el) => el.outerHTML).join('\n')` — leave that declaration exactly as-is, it is
   still needed below), add one new line computing the bare CSS text (no wrapping `<style>`
   tags): `const cssText = styleHtml.replace(/<\/?style[^>]*>/gi, '');`. Add a short comment
   noting this is the value Paged.js's Polisher needs when passed as an object, whereas
   `styleHtml` (still tag-wrapped) continues to be used below for `printStyle.textContent`
   (which sets literal CSS text on a real `<style>` element and never goes through Paged.js's
   Polisher at all — that code path is untouched).

2. In the `printStyle.textContent` template literal, replace the inline expression
   `${styleHtml.replace(/<\/?style[^>]*>/gi, '')}` with `${cssText}` — same value, now read
   from the new local rather than recomputed inline. No other change to that template literal.

3. Replace the entire `try { await previewer.preview(bodyHtml, [styleHtml], printRoot); } catch
   { ... }` block with a single, un-try/catch-wrapped call:
   `await previewer.preview(bodyHtml, [{ 'act-preview.css': cssText }], printRoot);`
   — the stylesheets argument is now an object (`Polisher.add()`'s object branch, which uses
   the value directly as CSS text) instead of a string (which gets fetched as a URL — the
   confirmed root cause). Do not keep any fallback retry: the removed `catch` block's fallback
   used a string of the SAME shape as the primary attempt (only the `<style>`-tag-wrapping
   differed), so it was structurally incapable of ever succeeding where the primary call
   failed — it wasn't a real fallback, just a second identical failure mode. Let a genuine
   `preview()` failure now propagate up to `handlePrint()`'s existing
   `catch { pushToast('error', 'Не удалось открыть документ для печати'); }`, which already
   wraps every call site of `printViaTopLevel`.

4. Replace the block comment directly above the `try`/`catch` (the one starting "RESEARCH.md
   Open Question 2: Polisher.add()'s stylesheet-argument shape...") with a short comment
   documenting the now-confirmed root cause and fix: Paged.js's `Polisher.add()`
   (`pagedjs/dist/paged.js`, `async add()`) treats a `typeof "object"` argument's values as CSS
   text directly, and anything else (including a plain string) as a URL to `request(...)` and
   fetch — passing `[styleHtml]` (a string) made Paged.js fetch the CSS text itself as a URL,
   which is the exact failed request observed in DevTools Network
   (`/%3Cstyle%3E...`). Fixed by passing `[{ 'act-preview.css': cssText }]`.

Net effect: `styleHtml` (tag-wrapped) still feeds `printStyle.textContent` unchanged; `cssText`
(bare) is the one new value, used only for the Paged.js `preview()` call; the redundant/broken
fallback is gone. No changes to `printViaSystemBrowser`, imports, or any other function in this
file.
  </action>
  <verify>
    <automated>grep -F "previewer.preview(bodyHtml, [{ 'act-preview.css': cssText }], printRoot)" /Users/madsas/Projects/trackly/ui/src/features/acts/PdfPreviewModal.svelte >/dev/null && echo GREP_OK && pnpm --dir /Users/madsas/Projects/trackly/ui svelte-check && pnpm --dir /Users/madsas/Projects/trackly/ui lint && pnpm --dir /Users/madsas/Projects/trackly/ui build</automated>
  </verify>
  <done>printViaTopLevel calls previewer.preview(bodyHtml, [{ 'act-preview.css': cssText }], printRoot) with no surrounding try/catch fallback; svelte-check, lint (incl. the CSP hash-drift gate, unaffected since bootstrapScript.js is untouched), and build all pass; printViaSystemBrowser is byte-for-byte unchanged.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| Backend-rendered document HTML → `DOMParser`/Paged.js in the top-level document | `html` passed into `printViaTopLevel` originates from the backend's own act/report render endpoints (`acts.renderPdf`, `renderAcceptancePdf`, `reports_export_pdf`) — already-trusted, server-generated markup, not raw user input. This plan changes only HOW its extracted CSS text is handed to Paged.js's Polisher (string vs. object argument shape); it does not change what HTML is parsed, sourced, or injected, so no new trust boundary is crossed. |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-edd-01 | Tampering | `printViaTopLevel`'s `cssText` value fed to `Polisher.add()` | accept | `cssText` is derived from the same already-trusted, backend-rendered `html` this function already parsed and injected into `#act-print-root`/`printStyle` before this fix — switching its argument shape (string → object key/value) changes only how Paged.js consumes it internally, not its provenance. No new external or user-controlled input is introduced. |
| T-edd-02 | Denial of Service | Removed `catch` fallback in `printViaTopLevel` | accept | The prior fallback never actually recovered (same string-shape bug as the primary call), so removing it does not reduce any real resilience — a `preview()` failure now surfaces immediately via `handlePrint()`'s existing top-level `catch { pushToast(...) }`, which was already the effective behavior once both attempts failed identically. |
</threat_model>

<verification>
1. `pnpm --dir ui svelte-check` — no new type errors.
2. `pnpm --dir ui lint` — passes, including the CSP hash-drift gate (untouched file, should be
   unaffected).
3. `pnpm --dir ui build` — production build succeeds.
4. Manual/human-check (NOT automatable — requires a real LAN browser hitting the axum server,
   per `synthetic_harness_not_verification`): from a real browser at
   `https://web.cmy.local:8443` (or equivalent LAN URL), open a document preview, press
   «Печать», confirm (a) DevTools Network shows no request whose URL is CSS text
   (`/%3Cstyle%3E...`), (b) the native print dialog opens, (c) the printed/PDF output has the
   expected `@page` margins and fonts applied (proves the CSS actually reached Paged.js's
   Polisher, not just that no error was thrown). Record the result in the SUMMARY.
</verification>

<success_criteria>
- `printViaTopLevel` passes `[{ 'act-preview.css': cssText }]` (object) to
  `previewer.preview(...)`, never a bare string.
- No fallback retry remains in `printViaTopLevel`; a `preview()` failure surfaces through
  `handlePrint()`'s existing toast.
- `printViaSystemBrowser` and all CSP-hash-locked files are unchanged.
- `svelte-check`, `lint`, and `build` all pass.
- Real LAN-browser print verified manually (see verification step 4) — this is the only
  path that actually proves the fix; automated commands above only prove the code compiles
  and lints clean.
</success_criteria>

<output>
Create `.planning/quick/260805-edd-fix-lan-print-pass-stylesheets-to-paged-/260805-edd-SUMMARY.md` when done
</output>
</content>

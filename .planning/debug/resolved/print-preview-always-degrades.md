---
slug: print-preview-always-degrades
status: resolved
trigger: "Превью печати всегда уходит в деградацию D-02: iframe с Paged.js не монтируется, потому что showLoading скрывает ветку с ним; через 8s срабатывает degrade timeout. Симптомы: нет полей внутри листа, «Разбиваем на страницы…» висит ровно 8 секунд. Файл ui/src/features/acts/PdfPreviewModal.svelte, строки 171/416/434/444, таймаут на строке 109."
created: 2026-08-04
updated: 2026-08-05T01:10:00
phase: 33
---

# Debug: превью печати всегда уходит в деградацию D-02

## Symptoms

**Expected behavior**
Модалка предпросмотра показывает документ как лист(ы) A4 с внутренними полями 20mm/15mm
(из `@page` шаблона), разбитые Paged.js, на сероватой подложке. Пагинация должна занимать
доли секунды на страницу, а счётчик «Страница N…» — реально расти.

**Actual behavior**
Лист рендерится без внутренних полей — содержимое прижато к краям белого листа
(подтверждено скриншотом UAT, акт №14, desktop-режим, тёмная тема). Перед этим ровно
~8 секунд висит текст «Разбиваем на страницы…», счётчик страниц не растёт.
Пользователь интерпретировал эти 8 с как «8 секунд на обработку страницы».

**Error messages**
Явных ошибок в UI нет. Ожидается предупреждение в консоли:
`[PdfPreviewModal] Paged.js pagination timed out — falling back to unpaginated preview (D-02).`
— требует подтверждения в запущенном приложении.

**Timeline**
Появилось сразу с Phase 33 (плана 33-03). Раньше не работало иначе — до Phase 33 превью было
одностраничным без полей by design. То есть фича никогда не работала как задумано; регрессии
из более раннего состояния нет.

**Reproduction**
1. `pnpm --dir ui build && cargo tauri dev`
2. Акты → выбрать любой акт → «Печать»
3. Наблюдать: ~8 с спиннера с текстом «Разбиваем на страницы…», затем документ без полей.
Воспроизводится на любом документе и в любой теме, так как путь кода не зависит от содержимого.

## Current Focus

reasoning_checkpoint_defect6_defect7:
  hypothesis: |
    TWO reported problems (UAT round 5) share ONE root cause, and it is NOT the cause the
    checkpoint report tentatively suggested for defect #7:

    DEFECT #6 (regression, hard border) — `.pdf-iframe` (PdfPreviewModal.svelte, pre-fix)
    declares no `border` property at all. Commit `5846bb0` ("feat(33-03): sheet-stack chrome,
    fit-to-width scale, progress markup") deliberately REMOVED the pre-existing
    `border: 1px solid var(--tr-border);` rule, citing D-09 "без рамки" (no frame/border) —
    but nothing replaced it with an explicit `border: none`. With no author-declared border,
    the browser's own default UA stylesheet rule for `<iframe>` applies:
    `iframe:not([seamless]) { border: 2px inset; }` (WHATWG html.spec.whatwg.org, confirmed via
    web search, not assumed from memory) — a 2px inset border on all four sides, which is
    exactly the "жёсткая граница" (harsh/hard border) visible in the user's screenshot.

    DEFECT #7 (persistent internal vertical scroll) — NOT a height-measurement gap in
    `bootstrapScript.js`. Read `chunker.js`/`previewer.js` (pagedjs 0.4.3 source, not the
    minified bundle) directly: `previewer.preview()` is called with ZERO arguments in
    bootstrapScript.js, so `Previewer.preview()` calls `this.wrapContent()` first (moves the
    ENTIRE existing `<body>` content into a hidden `<template data-ref="pagedjs-content">`,
    which has no layout box / does not contribute to scrollHeight) and then
    `Chunker.setup()` appends EXACTLY ONE element to `<body>`: `this.pagesArea` (class
    `pagedjs_pages`), via `document.querySelector("body").appendChild(this.pagesArea)`.
    Grepped the entire pagedjs source tree for every `body.appendChild`/`querySelector("body")`
    call — confirmed there is no other element ever appended to `<body>` by the library. `body`
    has `margin: 0` (our own injected style) and no other rendered child, and `.pagedjs_pages`
    itself is never given an explicit height (only a `--pagedjs-page-count` custom property) —
    it is a `display:flex; flex-direction:column` container whose own box is fully auto-sized
    to its content (padding + pages + gaps), so `.pagedjs_pages.scrollHeight` (what
    `bootstrapScript.js` already reads) and `document.documentElement.scrollHeight` (what the
    checkpoint report proposed instead) are structurally IDENTICAL in this exact DOM — there is
    no "something outside `.pagedjs_pages`" contributing extra height. The checkpoint's specific
    claim ("if Paged.js or body contributes anything outside `.pagedjs_pages`, the reported
    height is short by exactly that amount") is checked and found NOT TRUE for this codebase —
    `naturalHeightPx` is already an accurate measurement of the iframe document's true content
    height. Switching the height source to `documentElement.scrollHeight` would have been a
    no-op change addressing a gap that does not exist.

    The REAL mechanism for defect #7: `ui/src/styles/global.scss` (read directly) applies
    `*, *::before, *::after { box-sizing: border-box; }` — a document-wide reset that DOES
    apply to `.pdf-iframe` (it's a plain global stylesheet rule, not affected by Svelte's
    per-component style scoping, which only scopes rules written *inside* a component's own
    `<style>` block, not global rules matching from outside). Under `box-sizing: border-box`,
    the `height: {naturalHeightPx}px` declared on `.pdf-iframe` is a BORDER-BOX size — i.e. it
    must include the element's own border. Because of defect #6 (no declared border, so the
    UA default 2px-inset border applies), 4px (2px top + 2px bottom) of that declared height
    was being consumed by the border itself, leaving the iframe's actual rendering viewport
    (the height available to the framed document inside) 4px SHORTER than `naturalHeightPx` —
    even though `naturalHeightPx` itself was already fully correct. That 4px is exactly what
    manifested as "still scrolling" inside the iframe on top of `.pdf-page-frame`'s own outer
    scroll. Defects #6 and #7 are therefore the SAME missing declaration, not two unrelated
    bugs — fixing `border: none` on `.pdf-iframe` removes both the visible hard edge AND the
    height deficit in one change, with NO change needed to `bootstrapScript.js` or its
    CSP hash.
  confirming_evidence:
    - "git log -p on PdfPreviewModal.svelte: commit 5846bb0's diff shows `- border: 1px solid
      var(--tr-border);` removed with commit message text \".pdf-iframe drops its 1px border
      (D-09 «без рамки»)\" and no replacement border declaration added in the same or any later
      commit — confirmed by reading the CURRENT file (pre-this-round) which has no `border`
      property anywhere in `.pdf-iframe`."
    - "Web search confirms the WHATWG default UA stylesheet rule `iframe:not([seamless]) {
      border: 2px inset; }` — not accepted from memory, independently looked up."
    - "Read ui/node_modules/.pnpm/pagedjs@0.4.3/node_modules/pagedjs/src/polyfill/previewer.js
      in full: `preview(content, stylesheets, renderTo)` — when called with no arguments
      (confirmed: bootstrapScript.js:51 calls `previewer.preview()` with zero args), `content`
      is falsy so `this.wrapContent()` runs, which moves body's existing children into a
      `<template>` (inert, no layout box) — the ONLY other thing appended to body is via
      `Chunker.setup()`."
    - "Read ui/node_modules/.pnpm/pagedjs@0.4.3/node_modules/pagedjs/src/chunker/chunker.js
      `setup(renderTo)`: `this.pagesArea = document.createElement('div');
      this.pagesArea.classList.add('pagedjs_pages'); ... document.querySelector('body')
      .appendChild(this.pagesArea)` when `renderTo` is falsy (confirmed falsy: chunker.flow is
      called via `previewer.preview()` with no renderTo argument, per previewer.js:157)."
    - "grep -rn \"body.appendChild\\|querySelector(\\\"body\\\")\\|document.body\\.\" across the
      ENTIRE pagedjs src tree: only the two call sites above (previewer.js's template wrapper,
      chunker.js's pagesArea) — nothing else is ever appended to body by this library version."
    - "grep -rn \"pagesArea\\.\" across pagedjs src: only classList.add, one CSS custom-property
      counter, .remove(), and children insertion INSIDE pagesArea (page.js) — no explicit
      width/height is ever set on `.pagedjs_pages` itself, so its box is fully auto-sized to
      content, meaning its scrollHeight already reflects the true full content height with no
      possible external contribution outside it."
    - "Read ui/src/styles/global.scss lines 9-15: `*, *::before, *::after { box-sizing:
      border-box; }` — a plain global selector, confirmed to apply to `.pdf-iframe` (Svelte
      scoping only adds scoping to selectors written inside a component's own <style> block; it
      does not exempt component elements from matching separately-imported global rules)."
    - "Confirmed no other CSS anywhere in ui/src/styles or Modal.svelte sets border/box-sizing
      specifically for iframe elements (grep for \"iframe\" across ui/src/styles/*.scss and
      Modal.svelte) — the UA default border-box interplay is the only mechanism in play."
  falsification_test: |
    If `.pagedjs_pages` or `document.body` inside the iframe's own document had ANY other
    rendered sibling/child contributing height (a stray element, a visible `<template>`
    fallback, a Paged.js-injected UI chrome bar, etc.), `documentElement.scrollHeight` would
    exceed `.pagedjs_pages.scrollHeight` and the checkpoint's original claim would be correct
    instead of mine. Checked exhaustively via grep across the full pagedjs source tree for
    every DOM-body mutation — none found beyond the inert `<template>` and `.pagedjs_pages`
    itself. If `.pdf-iframe` were NOT actually subject to the global `box-sizing: border-box`
    reset (e.g. if Svelte scoping somehow excluded it, or some other rule overrode box-sizing
    back to content-box for iframes specifically), the border-box mechanism for defect #7 would
    be false and the border would be a purely cosmetic (defect #6-only) fix. Checked: no such
    override exists anywhere in the stylesheet tree (grepped for "box-sizing" and "iframe" in
    ui/src/styles).
  fix_rationale: |
    `border: none;` on `.pdf-iframe` addresses the true root cause of BOTH defects with one
    minimal change: it removes the UA-default 2px-inset border (fixing the visible hard-edge
    regression, defect #6) AND, because of the border-box reset, restores the framed document's
    actual rendering viewport to exactly `naturalHeightPx` (fixing the residual internal
    overflow, defect #7) — without touching the height-measurement source in
    `bootstrapScript.js` at all, since that measurement was independently confirmed correct.
    Additionally changed `.pdf-iframe`'s `background` from `var(--tr-n-0)` (white) to
    `var(--tr-surface-sunken)` (the same backdrop colour already painted by `.pdf-page-frame`
    around it) — this background is only ever visible for the instant before the iframe's own
    document paints its body backdrop over the whole viewport; painting it white first produced
    a visible flash against the near-black dark-theme backdrop, whereas matching the surrounding
    backdrop colour makes that instant blend in instead of flashing.
  blind_spots: |
    NOT run in the live app this round — this is static/code-read + CSS-box-model reasoning
    only, per the hard constraints. Specifically NOT verified by this round: (1) whether the
    4px-border-box deficit was in fact the ONLY source of overflow, or whether a smaller,
    second-order effect (e.g. `scrollHeight`'s own integer rounding of a fractional layout
    height, typically at most ~1px) still leaves an imperceptible residual scrollbar — this
    cannot be proven by static reasoning alone and needs the user's eyes on the real rendered
    iframe; (2) whether the fix looks correct in BOTH themes (dark-theme border-color for an
    `inset` border style depends on currentColor / text-color, so the "harshness" the user saw
    may have looked different in light vs dark — removing the border eliminates this regardless
    of theme, but the visual confirmation is still owed to UAT); (3) MULTI-PAGE remains
    completely unverified end-to-end in this project — this round's reasoning about
    `.pagedjs_pages`'s auto-sized box applies identically for N pages (nothing in the traced
    mechanism is single-page-specific), but no multi-page document has ever actually been run
    through this preview. Given the hard constraint's explicit instruction, `html, body {
    overflow: hidden; }` was deliberately NOT added as a safety net this round — the height
    source itself was not touched, and adding a hard clip on top of an unverified-in-the-browser
    fix risks silently hiding content if any small residual gap remains, which is worse than an
    equally small residual scrollbar.

reasoning_checkpoint_defect5:
  hypothesis: |
    THREE independent, overlapping causes produce nested scrolling + clipped/duplicated
    shadow in the pagination iframe, all confirmed by direct code read (none accepted from
    the checkpoint report on faith):

    CAUSE A — `.pdf-iframe` (PdfPreviewModal.svelte:566-574) hardcodes
    `width: 794px; min-width: 794px; height: 1123px; min-height: 1123px;` while its wrapper
    `.pdf-scale-inner` (line 465) receives a correct DYNAMIC inline `height:
    {naturalHeightPx}px`. The iframe's own box never grows past 1123px regardless of
    `naturalHeightPx`, so once inner content (page + `.pagedjs_pages`'s `padding: 16px 0` top
    and bottom, from pagedPreviewBootstrap.ts:59) exceeds 1123px — which it always does even
    for ONE page (1123 + 32 = 1155px) — the iframe's own replaced-element box overflows and
    gets its own native scrollbar, nested inside `.pdf-page-frame`'s outer scrollbar. Confirmed
    exact CSS text via direct read, not restated from the checkpoint.

    CAUSE B — pagedPreviewBootstrap.ts:59's injected `.pagedjs_pages` rule has
    `padding: 16px 0` (zero horizontal). The page box (`.pagedjs_page`) is exactly the iframe's
    content width (794px, matching @page A4 width from the untouched HTML templates, D-01), so
    its `box-shadow` (defined line 68, `0 3px 10px` dark / `0 2px 6px` light) has zero lateral
    room inside the flex-centered `.pagedjs_pages` container — clipped at both edges, and the
    shadow's own spread pushes horizontal overflow, adding an h-scrollbar that compounds cause
    A's v-scrollbar. Confirmed by direct read of the exact style string, not restated.

    CAUSE C — `.pdf-iframe` (PdfPreviewModal.svelte:571) still carries
    `box-shadow: var(--tr-elev-2);`, a leftover from the pre-Phase-33 single-sheet design
    (comment block above `.pdf-page-frame`, lines 529-535, dated Phase 16/GAP-16-01). D-09
    (33-CONTEXT.md:83-86) locks the shadow to be drawn PER `.pagedjs_page` INSIDE the iframe
    (confirmed already present at pagedPreviewBootstrap.ts:68, added in defect #4's fix). The
    outer `.pdf-iframe` shadow is now a second, redundant shadow outlining the ENTIRE iframe
    box (i.e., the whole page stack once multi-page works) rather than each individual sheet —
    duplicating and structurally conflicting with D-09's per-sheet design, confirmed by direct
    comparison of both shadow declarations in the two files.

    DIRECTION: widen the frame (increase `.pagedjs_pages` horizontal padding + iframe width +
    `.pdf-scale-inner` width + scaleFactor divisor, all to 842px = 794 + 2×24), do NOT move the
    shadow onto the iframe element itself. Independently confirmed correct: a shadow applied to
    the outer iframe element can only ever outline the iframe's single box — for a multi-page
    document (D-04, locked) the iframe box spans the ENTIRE page stack (per Cause-A fix), so an
    iframe-level shadow would draw one shadow around all N pages stacked together, not a
    separate shadow per sheet — directly violating D-09's explicit "per-sheet" requirement.
    Widening the frame is structurally the only option compatible with D-09 for N>1 pages.
  confirming_evidence:
    - "PdfPreviewModal.svelte:566-574 read directly: `.pdf-iframe` CSS class literally contains
      `height: 1123px; min-height: 1123px;` with no dynamic binding — the only dynamic height in
      the DOM chain is on `.pdf-scale-inner`'s inline style (line 465), a sibling wrapper, not
      the iframe element itself."
    - "pagedPreviewBootstrap.ts:59 read directly: `.pagedjs_pages { ...padding: 16px 0; }` —
      horizontal padding is literally 0, confirmed byte-for-byte, not paraphrased."
    - "pagedPreviewBootstrap.ts:68 read directly: `.pagedjs_page { box-shadow: ${chrome.shadow};
      background: #fff; }` — shadow IS already drawn per-page inside the iframe (this is
      defect #4's already-applied fix), confirming D-09's per-sheet placement is correctly
      implemented at the source; the iframe-level box-shadow at PdfPreviewModal.svelte:571 is
      therefore provably redundant, not merely 'old code that might still matter'."
    - "bootstrapScript.js:53-54 read directly: `naturalHeightPx` is populated from
      `pagesEl.scrollHeight` where `pagesEl = document.querySelector('.pagedjs_pages')` —
      `scrollHeight` per the CSS box-model spec includes the element's own padding (top+bottom),
      so the value already correctly reflects the 16px+16px vertical padding without any
      additional fix needed on that axis; only the iframe's own CSS height rule ignores this
      value, per Cause A. This confirms the checkpoint's 'should already be correct' claim
      about scrollHeight rather than accepting it on faith."
    - "ui/src/styles/_tokens.scss:182 read directly: `--tr-space-xl: 24px` in both theme blocks
      — confirms the checkpoint's rationale that 24px is a real, already-approved token (not an
      invented number), and 33-UI-SPEC.md:47 already documents this exact token as the
      inter-sheet vertical gap rationale, giving internal consistency to reusing it as the new
      horizontal gutter."
    - "grep across ui/src confirms `.pdf-iframe` in TemplateEditor.svelte:486 is an unrelated,
      independently-scoped Svelte component class (different file, Svelte scopes styles
      per-component) — not affected by any of these three causes or by this fix."
  falsification_test: |
    If the iframe element in fact inherits (or already receives, via some binding not caught in
    the initial read) a dynamic height equal to naturalHeightPx, Cause A would be false — the
    nested scrollbar would have to come from elsewhere. Checked: grepped the full component for
    every occurrence of `naturalHeightPx` and `794`/`1123` (see Evidence) — the iframe element
    itself has NO inline style attribute at all (only `sandbox`, `{srcdoc}`, `bind:this`,
    `title`, `class`), so its dimensions come ENTIRELY from the static `.pdf-iframe` CSS rule.
    No contradicting evidence found. If `.pagedjs_page`'s actual rendered width were narrower
    than the iframe's content-box width (i.e., some existing horizontal margin already gave the
    shadow room), Cause B would be false — not directly provable via static CSS alone (Paged.js
    computes `.pagedjs_page` box sizing internally from `@page` at runtime), flagged as the one
    genuine blind spot below.
  fix_rationale: |
    Each cause maps to a minimal, targeted change addressing its own root cause, not a shared
    symptom patch:
    - Cause A: bind the iframe's own height to `naturalHeightPx` (same value already driving
      `.pdf-scale-inner`), instead of a static CSS rule frozen at the single-page placeholder
      value. Keeps `min-height: 1123px` as a pre-pagination placeholder only (before the first
      `trackly-pagedjs-done` message, matching `naturalHeightPx`'s own initial `$state(1123)`).
    - Cause B: add horizontal padding to `.pagedjs_pages` (16px 0 → 16px 24px) so the per-page
      shadow (already correctly placed by defect #4's fix) has physical room to paint without
      clipping or forcing horizontal overflow. 24px reuses the existing `--tr-space-xl` token
      value (not injectable as a token reference across the opaque-origin boundary, so the
      literal `24px` mirrors the file's existing literal-hex convention for chrome.backdrop/
      chrome.shadow) — chosen because dark theme's shadow spread is `10px`, so 24px gives
      comfortable clearance plus alignment with the app's own spacing rhythm.
    - Cause C: remove the redundant/conflicting `box-shadow: var(--tr-elev-2)` from
      `.pdf-iframe` — the per-sheet shadow (D-09) is now the ONLY shadow, drawn correctly inside
      the iframe per page; keeping both duplicates the visual and, structurally, an
      iframe-level shadow cannot ever be "per sheet" once N>1 pages exist under Cause A's fix.
    All three fixes together require widening `.pdf-iframe`/`.pdf-scale-inner` width and the
    `scaleFactor` divisor from 794 to 842 (794 + 2×24) so the fit-to-width scale math stays
    correct against the new, wider iframe box — leaving the divisor at 794 would under-scale
    the sheet and reintroduce horizontal overflow at narrow modal widths (verified: scaleFactor
    = min(1, frameWidthPx / divisor); if divisor stays smaller than the iframe's actual
    rendered width, the computed scale is too large for the available frame width).
  blind_spots: |
    Not run in the live app this round (static/code-read verification only, per the hard
    constraints and the checkpoint's own framing). Specifically NOT verified by this round:
    (1) whether the nested scrollbar is actually gone in a real WKWebView/browser render — the
    CSS box-model reasoning is sound but Paged.js's own runtime DOM manipulation of
    `.pagedjs_page` sizing was not traced instruction-by-instruction, only its CSS surface;
    (2) whether the shadow now paints fully un-clipped at both edges in both themes; (3) the
    MULTI-PAGE case specifically — this project has NEVER observed multi-page preview working
    end-to-end (only single-page acts have been UAT'd through round 4). The reasoning that
    Cause A's fix "would have hidden all pages past 1 behind an inner scrollbar" for
    multi-page documents is sound static reasoning about the OLD code, but the NEW code's
    actual multi-page behavior (correct pagination, correct per-page shadow rendering, no
    unexpected reflow) remains completely unverified and must not be presented as confirmed.

reasoning_checkpoint_defect4:
  hypothesis: |
    `ui/src/lib/pdfPreview/pagedPreviewBootstrap.ts:56-61`'s injected `<style>` sets
    `.pagedjs_page { box-shadow: ${chrome.shadow}; }` and nothing else — no `background`
    declaration. D-09 deliberately skips Paged.js's stock `interface.css` (which normally
    paints `.pagedjs_page` white), but nothing replaces that rule locally. The sheet is
    therefore transparent and shows the `body`'s own `background: ${chrome.backdrop}`
    (line 58) straight through, so the "paper" reads as the backdrop colour instead of
    white in both themes — violating D-08's "лист — всегда белый" (33-CONTEXT.md line 78)
    and 33-UI-SPEC.md's Color table row "Sheet (secondary, always paper) ... Never inverts
    with theme ... single most load-bearing color rule in this phase" (line 98).
  confirming_evidence:
    - "Direct read of pagedPreviewBootstrap.ts:56-61: the injected style string has exactly
      three rules — body{margin,background}, .pagedjs_pages{flex layout}, .pagedjs_page{box-
      shadow only}. No background/color rule targets .pagedjs_page or any Paged.js-generated
      sheet element anywhere in this file."
    - "33-CONTEXT.md D-09 (line 83-86) confirms interface.css is NOT loaded: 'Штатный
      interface.css от Paged.js не используется: его цвета и тени не согласованы с
      дизайн-системой'. Paged.js's own stylesheet is what normally paints .pagedjs_page
      white; skipping it without a local replacement leaves the box unstyled for background."
    - "grep 'pdf-page-frame|pdf-iframe|tr-surface-sunken|tr-n-0' PdfPreviewModal.svelte:
      .pdf-page-frame already uses --tr-surface-sunken (line 542/564), .pdf-iframe already
      uses --tr-n-0 (line 572) — these are the OUTER DOM element's own box background, which
      is irrelevant here because the iframe's opaque-origin document paints its own `body`
      background (chrome.backdrop) covering the entire content area from the inside,
      independent of the outer iframe element's CSS background."
    - "chrome.backdrop IS already a literal hex ('#e4e8f0' / '#0a0d12', THEME_CHROME const,
      lines 38-46), confirming the file's established convention of literal hex values for
      anything injected into the opaque-origin iframe (custom properties from the parent
      app are unreachable across that boundary) — the fix must follow the same convention."
  falsification_test: |
    If Paged.js itself injects a default white background onto .pagedjs_page via inline
    style or an internally-generated stylesheet (independent of interface.css), the sheet
    would already render white and the hypothesis would be wrong. Checked: D-09's own
    rationale text explicitly says interface.css owns that visual (colors/shadows), and nothing
    in bootstrapScript.js (read in full during defect #3 investigation) sets any DOM style on
    .pagedjs_page. No contradicting evidence found.
  fix_rationale: |
    Add `background: #fff;` to the `.pagedjs_page` rule in pagedPreviewBootstrap.ts's style
    string. This addresses the root cause (missing background declaration on the page box)
    directly, not a symptom — no other file/layer is responsible for painting the sheet.
    Literal `#fff` (not `var(--tr-n-0)`) because the iframe is opaque-origin and cannot resolve
    the parent app's custom properties, matching the file's own established pattern for
    chrome.backdrop. White in both themes, per D-08 — the THEME_CHROME per-theme dict is not
    used for this rule since the sheet is explicitly theme-invariant.
  blind_spots: |
    Not run in the live app this round — cannot visually confirm the sheet renders correctly
    white in either theme, only that the CSS rule is now structurally present and correctly
    scoped (verified via svelte-check/lint/build, not visual render). Does not re-verify
    defects #1-#3 in the live app beyond what UAT round 3 already reported.

reasoning_checkpoint_defect3:
  hypothesis: |
    `ui/src/lib/pdfPreview/bootstrapScript.js:20` references `window.Paged.Previewer` but the
    concatenated pagedjs bundle (`ui/node_modules/pagedjs/dist/paged.min.js`, the UMD build
    actually imported by `pagedPreviewBootstrap.ts:25`) attaches its exports as
    `globalThis.PagedModule`, not `globalThis.Paged`. `window.Paged` is therefore `undefined`
    at the moment the bootstrap script's first statement runs inside the srcdoc iframe, and
    `new window.Paged.Previewer()` throws a TypeError before any postMessage can be sent —
    so the 8s degrade timeout is the ONLY possible outcome, exactly matching the reported
    console error and the D-02 fallback warning.
  confirming_evidence:
    - "Independently read ui/node_modules/pagedjs/dist/paged.min.js head: UMD wrapper resolves
      to `t((e=...globalThis...).PagedModule={})` — attaches to `PagedModule`, not `Paged`."
    - "Independently read the same file's tail: `e.Chunker=Ce,e.Handler=Cu,e.Polisher=wu,
      e.Previewer=Gm,e.initializeHandlers=Nm,...` — confirms `.Previewer` is a property of the
      `PagedModule` export object, matching the UMD wrapper's `e` parameter."
    - "grep -c \"window.Paged\\b\" across ui/src shows exactly ONE hit in the whole app:
      ui/src/lib/pdfPreview/bootstrapScript.js:1 (line count of matches, i.e. one line). No
      other code sets `window.Paged = window.PagedModule` as an alias anywhere (checked
      pagedPreviewBootstrap.ts in full — it only concatenates library text + bootstrap text,
      no global aliasing)."
    - "grep on ui/node_modules/pagedjs/dist/paged.polyfill.min.js confirms it DOES contain
      `Paged=` — verifies the claim that `window.Paged` is the polyfill build's global name,
      a build this project does not import (pagedPreviewBootstrap.ts:25 imports
      `dist/paged.min.js`, not `dist/paged.polyfill.min.js`)."
    - "package.json pins pagedjs 0.4.3 — matches the license header found in paged.min.js
      ('Paged.js v0.4.3')."
  falsification_test: |
    If `window.Paged` were defined by some other loaded script (alias, shim, or a different
    bundle actually being concatenated at build time), the TypeError would not occur despite
    the property-name mismatch. Independently verified this is not the case: only one file
    in ui/src references `window.Paged` (the buggy line itself), and pagedPreviewBootstrap.ts's
    concatenation formula is confirmed to embed paged.min.js (PagedModule-exporting UMD),
    not the polyfill build.
  fix_rationale: |
    Change `window.Paged.Previewer()` to `window.PagedModule.Previewer()` at
    bootstrapScript.js:20 — this addresses the root cause directly (wrong global property
    name), not a symptom. No other line in bootstrapScript.js references `Paged`/`PagedModule`.
    This is the one sanctioned exception to the D-14 CSP-hash-lock this round; hash must be
    regenerated and the constant in crates/trackly-app/src/http/mod.rs updated in the same
    change, verified by `pnpm --dir ui lint` (check-pagedjs-csp-hash.mjs).
  blind_spots: |
    Not run in a live browser/app this round (static-only verification, per coordinator
    instruction to only reproduce structurally). Does not prove pagination completes
    successfully end-to-end, that margins render, that the page counter grows, or that
    pagination speed is acceptable — needs user's eyes in the running app (UAT round 3).
    Also does not address the separately-reported intermittent `previewer.preview()` hang
    (see Eliminated + open lead below) — explicitly deferred per coordinator instruction.

hypothesis: |
  iframe, внутри которого исполняется бутстрап Paged.js, никогда не монтируется в DOM.
  В `PdfPreviewModal.svelte`:
    - `showLoading` (стр. ~171) истинно, пока `paginationStatus` не станет `done` или `degraded`;
    - `{#if showLoading}` (стр. ~416) — ПЕРВАЯ ветка условной цепочки;
    - iframe с `sandbox="allow-scripts"` и `{srcdoc}` (стр. ~444) — ТРЕТЬЯ ветка той же цепочки.
  Пока статус `pending`, рендерится только спиннер, iframe в DOM отсутствует → инлайн-скрипт
  не выполняется → сообщения `trackly-pagedjs-progress` / `trackly-pagedjs-done` не приходят
  никогда. Через `PAGINATION_TIMEOUT_MS = 8000` (стр. ~109) срабатывает degrade → ветка D-02
  (стр. ~434) с `sandbox=""` и ИСХОДНЫМ `htmlContent` вместо `srcdoc`. В исходном шаблоне
  `body { margin: 0; padding: 0 }`, а поля заданы только в `@page`, который на экране не
  применяется — отсюда отсутствие отступов. Оба симптома объясняются этой одной причиной.

test: |
  1. Подтвердить в запущенном приложении: консольное предупреждение о таймауте присутствует,
     `paginationStatus` доходит до `degraded`, а в DOM нет элемента `.pagedjs_pages`.
  2. Подтвердить, что при принудительном монтировании iframe (например, временно сняв
     условие `showLoading`) приходят `trackly-pagedjs-progress` и появляются `.pagedjs_page`
     с непустым `.pagedjs_area` padding.

expecting: |
  Оба пункта подтверждаются. Если (2) не даёт полей даже при смонтированном iframe — гипотеза
  объясняет только таймаут, а причина отсутствия полей отдельная (тогда копать, как Paged.js
  разбирает `@page` из инлайнового `<style>` документа).

next_action: |
  ГОТОВО К UAT ROUND 6. Дефект №4 (белый лист) — закрыт (round 4). Дефект №5 causes B/C (тень
  листа обрезалась/задваивалась) — ПОДТВЕРЖДЕНЫ исправленными пользователем в round 5, закрыты.
  Дефект №5 cause A (вложенный скролл) — фикс round 5 (динамическая высота iframe) НЕ устранил
  проблему полностью; round 6 (см. reasoning_checkpoint_defect6_defect7) нашёл настоящий
  остаточный механизм и это теперь ПОКРЫТО дефектом №7 ниже (та же причина).

  Дефект №6 (регресс, жёсткая рамка iframe) — независимо перепроверен (git log подтверждает
  1px-рамка была намеренно удалена в 33-03 без замены, WHATWG UA-стиль `iframe { border: 2px
  inset }` подтверждён веб-поиском) и исправлен: `.pdf-iframe` получил `border: none;`.

  Дефект №7 (вложенный вертикальный скролл всё ещё присутствует после round 5's fix) —
  ИЗМЕРЕНО чтением исходников pagedjs (`previewer.js`/`chunker.js`), НЕ принято со слов
  checkpoint-отчёта: `naturalHeightPx` (`pagesEl.scrollHeight` для `.pagedjs_pages`) УЖЕ было
  корректным — ничего кроме `.pagedjs_pages` и инертного `<template>` никогда не добавляется в
  `<body>` iframe-документа, так что `documentElement.scrollHeight` дал бы то же самое значение;
  предложенная в checkpoint-отчёте замена источника высоты была бы no-op. Настоящий механизм:
  `global.scss`'s универсальный `box-sizing: border-box` делает объявленную `height` на
  `.pdf-iframe` border-box размером, а недостающая (дефект №6) рамка означала, что 4px
  (UA-дефолтная `2px inset` рамка сверху+снизу) вычитались из полезной высоты viewport'а внутри
  iframe — именно этот дефицит и проявлялся как «скролл всё ещё есть». `border: none;` (тот же
  фикс, что и для дефекта №6) устраняет и это. bootstrapScript.js НЕ тронут, CSP-хэш НЕ
  пересчитывался — оба подтверждены `pnpm --dir ui lint` (`[check-pagedjs-csp-hash] OK`).

  Дополнительно: `.pdf-iframe`'s `background` изменён с `var(--tr-n-0)` (белый) на
  `var(--tr-surface-sunken)` — убирает белую вспышку перед отрисовкой srcdoc-документа в тёмной
  теме (подложка почти чёрная).

  Все проверочные команды зелёные: svelte-check (0 ошибок, тот же baseline 48 warnings), lint
  (включая check-pagedjs-csp-hash — хэш не затронут, подтверждено прогоном), build.

  ЧЕСТНО НЕ ДОКАЗАНО, требует UAT round 6 в запущенном приложении: (1) что 4px border-box
  дефицит был ЕДИНСТВЕННЫМ источником переполнения — второстепенный эффект (округление
  `scrollHeight` до целого от дробной раскладки, обычно ≤1px) статическими рассуждениями не
  проверяется; (2) визуально в ОБЕИХ темах — граница `inset` зависит от currentColor, могла
  выглядеть по-разному в разных темах, но удаление рамки убирает эффект независимо от темы;
  (3) МНОГОСТРАНИЧНЫЙ случай — ПО-ПРЕЖНЕМУ НИ РАЗУ НЕ НАБЛЮДАЛСЯ end-to-end ни в одном раунде
  UAT (round 4 и round 5 — оба однострочные акты). Рассуждение этого раунда о
  `.pagedjs_pages`'s auto-sized боксе логически одинаково применимо для N страниц, но это
  НЕ является подтверждением реального многостраничного рендера — если у пользователя есть
  многостраничный документ, стоит попробовать именно его в этом раунде.

refined_hypothesis: |
  Первый дефект (недостижимость ветки с iframe пагинации, см. root_cause ниже) был реальным и
  исправлен, но маскировал ВТОРОЙ, независимый дефект: после того как iframe стал монтироваться
  безусловно, инлайн-скрипт внутри srcdoc падает с `SyntaxError: Unexpected EOF` при парсинге —
  то есть до того, как Paged.js вообще успевает стартовать, — и превью всё равно уходит в
  8s-таймаут → D-02. Оба call site, собирающие итоговый HTML/srcdoc через
  `actHtml.replace(/<\/body>/i, \`${injected}</body>\`)` (СТРОКОВЫЙ replacement), уязвимы к
  ES-спеке `String.prototype.replace`: строковый replacement интерпретирует `$\``/`$'`/`$&`/
  `$$`/`$<n>` как спецпаттерны. Минифицированный `paged.min.js`, который целиком встраивается
  внутрь `injected`, содержит ровно одно вхождение буквальной подстроки `` $` `` (регэксп-
  источник шаблонного литерала `` `[${t}]+$` `` непосредственно перед закрывающим бэктиком) —
  движок трактует это как спецпаттерн «всё, что до совпадения», подставляя туда огромный кусок
  `actHtml`, что разрывает шаблонный литерал/минифицированный код бандла и даёт синтаксическую
  ошибку при исполнении `<script>` внутри `srcdoc`.

next_action_prior: |
  Подтвердить гипотезу в запущенном приложении, затем починить:
  (а) монтировать контейнер с iframe сразу, как только есть `srcdoc`, а состояние загрузки
      накладывать оверлеем поверх него. КРИТИЧНО: скрывать iframe только через
      `visibility`/`opacity`/z-order — `display: none` обнуляет измерения и сломает разбиение
      Paged.js;
  (б) переосмыслить таймаут: сейчас это жёсткий потолок на всю пагинацию, из-за чего длинный
      отчёт ушёл бы в деградацию даже при исправно работающем Paged.js. Сбрасывать таймер на
      каждом `trackly-pagedjs-progress` — тогда он ловит зависание, а не длительную работу.

reasoning_checkpoint:
  hypothesis: |
    Iframe пагинации (bind:this={iframeEl}, sandbox="allow-scripts") недостижим в цепочке
    {#if showLoading}/{:else if errorMsg}/{:else if htmlContent}/{:else}, пока
    paginationStatus === 'pending', потому что showLoading истинно ровно в этом состоянии.
    Бутстрап-скрипт Paged.js (исполняется внутри srcdoc этого iframe) поэтому никогда не
    запускается и никогда не шлёт trackly-pagedjs-progress/-done — paginationStatus не может
    сам выйти из 'pending'. Единственный выход — срабатывание 8s degradeTimeoutHandle
    (enterDegraded('timeout')), которое ВСЕГДА ведёт в D-02.
  confirming_evidence:
    - "PdfPreviewModal.svelte:416-459 — единая {#if}/{:else if} цепочка; iframe с
      bind:this={iframeEl} (стр. 444) существует только в третьей ветке, недостижимой пока
      showLoading истинно."
    - "showLoading (стр. 170-173): `loading || (htmlContent !== null && paginationStatus !==
      'done' && paginationStatus !== 'degraded')` — истинно ровно при paginationStatus ===
      'pending', который выставляется сразу после сборки srcdoc (стр. 197), то есть ДО того,
      как iframe вообще мог бы смонтироваться."
    - "attachBridge (pagedPreviewBridge.ts) подключается через $effect, ключ — `iframeEl !==
      null` (PdfPreviewModal.svelte:225-226); раз iframeEl не биндится, бридж не
      активируется — сообщению просто неоткуда прийти."
    - "В файле всего два <iframe>, оба внутри веток, гейтящихся тем же showLoading/branch-ом;
      прогревочного скрытого iframe нет."
  falsification_test: |
    Если временно снять гейт {#if showLoading} вокруг ветки пагинации, iframe смонтируется
    сразу, и trackly-pagedjs-progress/-done должны прийти заметно раньше 8s для обычного
    документа. Гипотеза была бы опровергнута, если бы монтирование iframe НЕ вызвало этих
    сообщений (тишина сохранилась бы до таймаута) — тогда причина не в недостижимости ветки,
    а в чём-то внутри самого bootstrapScript.js/Paged.js.
  fix_rationale: |
    Монтировать iframe пагинации безусловно, как только есть htmlContent/srcdoc, независимо
    от paginationStatus, а UI "пагинация идёт" рисовать оверлеем поверх него
    (opacity/position/z-index, НИКОГДА display:none) вместо конкурирующей верхнеуровневой
    ветки, исключающей iframe из DOM. Это чинит причину (iframe не монтируется), а не симптом
    (длительность таймаута). Дополнительно: 33-UI-SPEC.md прямо определяет таймаут как «8
    секунд от установки srcdoc до ПЕРВОГО trackly-pagedjs-progress ИЛИ trackly-pagedjs-done»,
    а 33-RESEARCH.md Pitfall 1 говорит «if no progress/done/error message arrives within N
    seconds... treat as failure» — то есть таймер обязан сбрасываться и по первому progress,
    а не только по done/error. Сейчас это не так (case 'trackly-pagedjs-progress' не чистит
    degradeTimeoutHandle) — второй, более мелкий дефект того же жизненного цикла, сегодня
    замаскированный первым (progress никогда не приходит вообще).
  blind_spots: |
    Не запускал приложение вживую — не подтверждено визуально: текст console.warn, отсутствие
    .pagedjs_pages в DOM при деградации, корректность полей 20mm/15mm при живой пагинации.
    Не проверено поведение при очень длинных документах: после фикса (б) таймер чистится один
    раз по первому progress и больше не перезапускается — если пагинация зависнет на странице
    2+, это уже не поймать. Это соответствует буквальной формулировке 33-UI-SPEC.md, но
    остаточный риск не устранён архитектурно.

## Constraints

- НЕ менять `ui/src/lib/pdfPreview/bootstrapScript.js` — его точный текст захэширован в CSP
  LAN-сервера (D-14), `pnpm --dir ui lint` падает при дрейфе.
- НЕ трогать `crates/trackly-app/templates/*.html` (D-01) и размеры `.modal-pdf-preview`
  в `ui/src/lib/components/Modal.svelte` (D-12).
- Деградационный путь D-02 должен сохраниться как поведение при реальном сбое Paged.js —
  чинится не он, а то, что в него попадают всегда.
- Проверочные команды: `pnpm --dir ui svelte-check`, `pnpm --dir ui lint` (включает гейт
  CSP-хэша), `pnpm --dir ui build`. Агрегирующего скрипта `check` в проекте НЕТ.
- Визуальную корректность автотесты не докажут — финальная проверка только в запущенном
  приложении (см. `.planning/phases/33-print-preview-polish/33-VALIDATION.md`).

## Evidence

- timestamp: 2026-08-04
  observation: |
    Чтение `ui/src/features/acts/PdfPreviewModal.svelte`: единственные два `<iframe>` в файле —
    стр. 434 (ветка `degraded`, `sandbox=""`, `srcdoc={htmlContent}`) и стр. 444 (ветка
    пагинации, `sandbox="allow-scripts"`, `{srcdoc}`). Обе находятся внутри
    `{:else if htmlContent !== null}`, то есть недостижимы, пока `showLoading` истинно.
    Скрытого iframe для «прогрева» пагинации в файле нет.
- timestamp: 2026-08-04
  observation: |
    `showLoading = loading || (htmlContent !== null && paginationStatus !== 'done' &&
    paginationStatus !== 'degraded')` — при `paginationStatus === 'pending'` истинно
    независимо от значения `loading`.
- timestamp: 2026-08-04
  observation: |
    Скриншот UAT: содержимое акта прижато к краям белого листа, отступов 20mm/15mm нет —
    поведение совпадает с исходным шаблоном (`body { margin: 0; padding: 0 }`), то есть
    с деградационной веткой, а не с выводом Paged.js.
- timestamp: 2026-08-04
  observation: |
    Автоматические гейты Phase 33 (`svelte-check` 0 ошибок, `lint` включая CSP-хэш, `build`,
    `security_headers` 4/4, `html_page_parity` 1/1) — все зелёные. Дефект логики рендера
    ими принципиально не ловится.
- timestamp: 2026-08-04T23:00:00
  observation: |
    ПОСЛЕ UAT первого фикса (iframe теперь монтируется) — новый репорт: счётчик страниц
    по-прежнему не растёт, спиннер "Разбиваем на страницы…" висит ~8с, затем D-02. Консоль:
    `SyntaxError: Unexpected EOF (anonymous function) (about:srcdoc:212)` и предупреждение о
    8s pagination timeout. Второй, независимый дефект того же жизненного цикла, ранее
    замаскированный первым (iframe вообще не монтировался, script никогда не парсился).
- timestamp: 2026-08-04T23:10:00
  observation: |
    Самостоятельная проверка (не принято со слов пользователя): `grep -o '\$\`'
    node_modules/pagedjs/dist/paged.min.js | wc -l` → 1 вхождение; `grep -o "\$'"` → 0;
    `grep -c '</script'` → 0; `grep -o '\$&'` → 0. Контекст найденного вхождения:
    `...replace(new RegExp(\`[${t}]+$\`),"")...` — буквальная подстрока `` $` `` образуется
    на стыке `$` (конец regex-источника) и открывающего/закрывающего бэктика шаблонного
    литерала. Оба call site (`pagedPreviewBootstrap.ts:71`, `PdfPreviewModal.svelte:313`)
    используют `actHtml.replace(/<\/body>/i, \`${injected}</body>\`)` — СТРОКОВЫЙ
    replacement, где движок интерпретирует `$\`` как спецпаттерн «текст до совпадения».
- timestamp: 2026-08-04T23:15:00
  observation: |
    Эмпирическое воспроизведение вне приложения (Node-скрипт, читает реальные
    `node_modules/pagedjs/dist/paged.min.js` + `bootstrapScript.js`, собирает `injected` по
    ТОЙ ЖЕ формуле, что и продакшн-код, прогоняет оба варианта `.replace()`): версия со
    строковым replacement — `oldResult.includes(pagedjs)` === false (бандл БОЛЬШЕ НЕ
    целостен внутри итогового HTML), извлечённое содержимое `<script>` не парсится
    (`new Function(body)` → SyntaxError). Версия с replacer-функцией
    (`() => \`${injected}</body>\``) — `newResult.includes(pagedjs)` === true (бандл
    байт-в-байт целостен), извлечённый `<script>` парсится без ошибок. Подтверждает
    механизм независимо от слов пользователя из checkpoint-ответа.
- timestamp: 2026-08-04T23:35:00
  observation: |
    UAT раунд 2: SyntaxError ИСЧЕЗ (дефект №2 подтверждён исправленным пользователем). Новая,
    другая ошибка: `TypeError: undefined is not an object (evaluating
    'new window.Paged.Previewer')` (about:srcdoc:124) + предупреждение о 8s pagination
    timeout → D-02.
- timestamp: 2026-08-04T23:38:00
  observation: |
    ДЕФЕКТ №3, независимо перепроверен (не принят со слов checkpoint-ответа):
    `head -c 300 ui/node_modules/pagedjs/dist/paged.min.js` →
    `!function(e,t){...}(this,(function(e){...t((e=...globalThis...).PagedModule={})...`
    — UMD-обёртка вешает экспорты на `globalThis.PagedModule`, НЕ на `globalThis.Paged`.
    `tail -c 500` того же файла → `...e.Chunker=Ce,e.Handler=Cu,e.Polisher=wu,
    e.Previewer=Gm,e.initializeHandlers=Nm,...` — `.Previewer` действительно свойство объекта
    `e`, который и есть `PagedModule`. `package.json` пиннит `pagedjs: "0.4.3"` — совпадает с
    лицензионным заголовком файла ("Paged.js v0.4.3").
- timestamp: 2026-08-04T23:39:00
  observation: |
    `grep -c "window.Paged\b" ui/src -r` → единственное совпадение во всём приложении:
    `ui/src/lib/pdfPreview/bootstrapScript.js:1` (сама проблемная строка). Полное чтение
    `pagedPreviewBootstrap.ts` подтверждает: формула сборки — просто конкатенация
    `pagedjsLibraryText + ';\n' + bootstrapText`, никакого алиаса `window.Paged =
    window.PagedModule` нигде не добавляется.
- timestamp: 2026-08-04T23:39:30
  observation: |
    `grep -o "Paged=" ui/node_modules/pagedjs/dist/paged.polyfill.min.js` → совпадение
    найдено — подтверждает, что `window.Paged` действительно глобальное имя ОТДЕЛЬНОЙ сборки
    `paged.polyfill.js`, которую проект не импортирует (`pagedPreviewBootstrap.ts:25`
    импортирует `dist/paged.min.js`, не `dist/paged.polyfill.min.js`).
- timestamp: 2026-08-04T23:45:00
  observation: |
    После правки: `node scripts/check-pagedjs-csp-hash.mjs --print` → новый хэш
    `sha256-1nG6ajqUxHpGqTH1xMQEfH1DAoyP3C8xrIMr3PNVhPQ=`, константа в
    `crates/trackly-app/src/http/mod.rs` обновлена. `pnpm --dir ui lint` → PASS, включая
    `[check-pagedjs-csp-hash] OK`. `pnpm --dir ui svelte-check` → 0 ошибок, 48 warnings (тот
    же baseline, что и раньше — не новые). `pnpm --dir ui build` → успешно, `dist/`
    пересобран. `cargo check -p trackly-app` → успешно (Rust-сторона компилируется после
    правки константы хэша).
- timestamp: 2026-08-04T23:50:00
  observation: |
    UAT round 3 (пользователь, живое приложение): дефект №3 ПОДТВЕРЖДЁН исправленным —
    пагинация быстрая, счётчик страниц показывает «1 страница», нет таймаута, нет TypeError.
    Прерывистое зависание `previewer.preview()` (open lead) НЕ воспроизвелось — ни разу за
    сессию UAT. Вопрос скорости закрыт: Paged.js никогда не был медленным, 8s всегда были
    таймаутом от более ранних (уже исправленных) дефектов №1/№2.
- timestamp: 2026-08-04T23:55:00
  observation: |
    ДЕФЕКТ №4 обнаружен по скриншотам UAT в обеих темах + код-ридом, перепроверен
    независимо (не принят со слов checkpoint-ответа): прямое чтение
    `ui/src/lib/pdfPreview/pagedPreviewBootstrap.ts:56-61` (до правки) — инжектируемый
    `<style>` содержит ровно три правила: `body{margin,background}`,
    `.pagedjs_pages{flex-раскладка}`, `.pagedjs_page{box-shadow}`. Ни у `.pagedjs_page`, ни у
    какого-либо другого селектора нет объявления `background`/`color`. `body`'s
    `background: ${chrome.backdrop}` покрывает весь iframe-документ снизу, поэтому
    прозрачный `.pagedjs_page` показывает именно этот фон — лист рендерится в цвете подложки,
    а не белым, в ОБЕИХ темах (в тёмной — почти чёрный лист с чёрным текстом документа поверх,
    что делает акт нечитаемым, как и описано в отчёте).
- timestamp: 2026-08-04T23:56:00
  observation: |
    Перепроверка заявленных locked-decision ссылок (не принято со слов checkpoint-ответа):
    `33-CONTEXT.md` D-08 (строка 78) — «Лист — всегда белый, это бумага... Отвергнут вариант
    с инверсией самого листа в тёмной теме — ломает PRV-03» — подтверждает точную
    формулировку локed-решения. D-09 (строка 83-86) подтверждает, что `interface.css`
    Paged.js (обычно красящий `.pagedjs_page` в белый) НЕ подключается намеренно —
    подтверждает механизм дефекта (ничего не заменило снятую стандартную покраску).
    `33-UI-SPEC.md` строка 98, таблица Color: «Sheet (secondary, always paper) | #ffffff |
    #ffffff (unchanged) | --tr-n-0 | Never inverts with theme — single most load-bearing
    color rule in this phase» — подтверждает требование буквально.
- timestamp: 2026-08-04T23:57:00
  observation: |
    Проверка области CSP-хэша (не принято со слов checkpoint-ответа): чтение
    `ui/scripts/check-pagedjs-csp-hash.mjs` — хэш считается по формуле
    `libraryText + ';\n' + bootstrapText` (то есть только `PAGED_PREVIEW_INLINE_SCRIPT` —
    библиотека Paged.js + `bootstrapScript.js`), инжектируемая style-строка в эту формулу
    НЕ входит. Дополнительно: `http/mod.rs:219` — `style-src 'self' 'unsafe-inline'` уже
    разрешает инлайн-стили без хэша. Подтверждено: правка style-строки в
    `pagedPreviewBootstrap.ts` не требует пересчёта CSP-хэша и не задевает захэшированный
    `bootstrapScript.js`.
- timestamp: 2026-08-04T23:58:00
  observation: |
    Проверка утверждений об `.pdf-page-frame`/`.pdf-iframe` (не принято со слов
    checkpoint-ответа): `grep -n "pdf-page-frame|pdf-iframe|tr-surface-sunken|tr-n-0"
    PdfPreviewModal.svelte` → `.pdf-page-frame` уже использует `--tr-surface-sunken`
    (строки 542/564), `.pdf-iframe` уже использует `--tr-n-0` (строка 572) — оба
    подтверждены, изменений не требуют. Это фон ВНЕШНЕГO DOM-элемента iframe, который не
    относится к дефекту №4: opaque-origin документ внутри iframe красит собственный `body`
    изнутри, перекрывая любой внешний CSS-фон контейнера.
- timestamp: 2026-08-04T23:59:00
  observation: |
    После правки (добавлен `background: #fff;` в правило `.pagedjs_page`, литерал, не
    `var(--tr-*)` — по тому же паттерну, что и уже существующий литеральный `chrome.backdrop`
    в этом же файле): `pnpm --dir ui svelte-check` → 0 ошибок, 48 warnings (тот же baseline).
    `pnpm --dir ui lint` → PASS, включая `[check-pagedjs-csp-hash] OK` (хэш не затронут, как и
    предполагалось). `pnpm --dir ui build` → успешно, `ui/dist` пересобран.

- timestamp: 2026-08-05T00:05:00
  observation: |
    UAT round 4: ДЕФЕКТ №4 (белый лист) ПОДТВЕРЖДЁН ИСПРАВЛЕННЫМ пользователем в обеих темах
    живьём — «лист теперь белый в светлой и тёмной теме». Закрыт.
- timestamp: 2026-08-05T00:10:00
  observation: |
    ДЕФЕКТ №5, репортован пользователем и перепроверен независимо (не принято со слов
    checkpoint-ответа). Прямое чтение `PdfPreviewModal.svelte:566-574` (до правки) —
    `.pdf-iframe` содержит буквально `width: 794px; min-width: 794px; height: 1123px;
    min-height: 1123px;`, БЕЗ какой-либо динамической привязки. Единственная динамическая
    высота во всей цепочке DOM — инлайновый `style` у `.pdf-scale-inner` (стр. 465,
    `height: {naturalHeightPx}px`), у сестринского элемента-обёртки, НЕ у самого iframe.
    Подтверждает CAUSE A буквально.
- timestamp: 2026-08-05T00:11:00
  observation: |
    Прямое чтение `pagedPreviewBootstrap.ts:59` (до правки) — `.pagedjs_pages { ...
    padding: 16px 0; }`, горизонтальный padding буквально ноль. Подтверждает CAUSE B буквально
    (не парафраз со слов checkpoint-ответа).
- timestamp: 2026-08-05T00:12:00
  observation: |
    Прямое чтение `pagedPreviewBootstrap.ts:68` — `.pagedjs_page { box-shadow: ${chrome.shadow};
    background: #fff; }` — тень уже корректно рисуется НА КАЖДОЙ странице внутри iframe (это
    уже применённый фикс дефекта №4), что подтверждает: D-09 (per-sheet тень) реализовано
    ПРАВИЛЬНО в источнике, а тень на самом `.pdf-iframe` (стр. 571, `box-shadow:
    var(--tr-elev-2)`) — доказуемо избыточна/конфликтна, а не «возможно ещё нужна». Подтверждает
    CAUSE C.
- timestamp: 2026-08-05T00:13:00
  observation: |
    Прямое чтение `bootstrapScript.js:53-54` — `naturalHeightPx` заполняется из
    `pagesEl.scrollHeight`, где `pagesEl = document.querySelector('.pagedjs_pages')`.
    `scrollHeight` по спецификации CSS box model включает СОБСТВЕННЫЙ padding элемента
    (верх+низ) — то есть значение уже корректно учитывает вертикальный padding `.pagedjs_pages`
    без каких-либо доп. правок по этой оси; проблема исключительно в том, что CSS-правило
    `.pdf-iframe` игнорировало это значение (CAUSE A). Подтверждает заявление checkpoint-ответа
    «scrollHeight should already be correct» — проверкой чтением, а не на веру.
- timestamp: 2026-08-05T00:14:00
  observation: |
    `grep -n "tr-elev-2|tr-space-xl|tr-space-md" ui/src/styles/_tokens.scss` →
    `--tr-space-xl: 24px` в обеих темах (строка 182), `--tr-elev-2` light
    `0 2px 6px rgba(16,22,34,.09), 0 1px 2px rgba(16,22,34,.06)` совпадает буквально с
    `THEME_CHROME.light.shadow` в `pagedPreviewBootstrap.ts`, dark `0 3px 10px rgba(0,0,0,.55),
    0 1px 2px rgba(0,0,0,.5)` тоже совпадает буквально. Подтверждает, что 24px — реальный,
    уже одобренный токен (не выдуманное число), и что тень 10px spread (тёмная тема) требует
    заметно меньше 24px горизонтального зазора, то есть 24px — комфортный, а не минимальный
    запас.
- timestamp: 2026-08-05T00:15:00
  observation: |
    `grep -rn "pdf-iframe" ui/src` → второе совпадение в `TemplateEditor.svelte:486` —
    независимый, отдельно скоупленный класс Svelte-компонента (Svelte скоупит стили
    по компоненту), без пагинации Paged.js, к дефекту №5 не относится и этим раундом не
    затронут. Подтверждено, не предположено.
- timestamp: 2026-08-05T00:20:00
  observation: |
    После правки всех трёх причин (`pagedPreviewBootstrap.ts` — `.pagedjs_pages` padding
    `16px 0` → `16px 24px`; `PdfPreviewModal.svelte` — `scaleFactor` делитель 794 → 842,
    `.pdf-scale-inner` inline width 794 → 842, iframe получил inline `style="height:
    {naturalHeightPx}px"`, `.pdf-iframe` CSS: width/min-width 794 → 842, статичный
    `height: 1123px` удалён (оставлен только `min-height: 1123px` как плейсхолдер до первого
    `trackly-pagedjs-done`), `box-shadow: var(--tr-elev-2)` удалён):
    `pnpm --dir ui svelte-check` → 0 ошибок, 269 файлов, 48 warnings (тот же baseline, новых
    нет). `pnpm --dir ui lint` → PASS, включая `[check-pagedjs-csp-hash] OK` (хэш не затронут —
    ни один из трёх файлов правки не входит в формулу хэша `PAGED_PREVIEW_INLINE_SCRIPT`,
    `bootstrapScript.js` байты не менялись). `pnpm --dir ui build` → успешно, `ui/dist`
    пересобран, без новых предупреждений/ошибок.
- timestamp: 2026-08-05T00:22:00
  observation: |
    `33-UI-SPEC.md` перепроверен на предмет актуальности зафиксированных чисел: `--tr-space-md`/
    `--tr-space-xl` таблица (строка 47) документировала 24px ТОЛЬКО как вертикальный
    inter-sheet gap — новое использование того же токена как горизонтального gutter
    добавлено отдельной строкой (не перезаписывает существующую), scoped-правка. Строка 320
    («794px page width fits comfortably...») оставлена без изменений — она описывает ширину
    самого ЛИСТА (`.pagedjs_page`, всё ещё 794px, из @page/D-01), а не ширину iframe-бокса
    (842px), поэтому формулировка остаётся точной и после этого фикса.
- timestamp: 2026-08-05T00:45:00
  observation: |
    UAT round 5 (пользователь, живое приложение): дефект №5 causes B/C (тень обрезалась/
    задваивалась) ПОДТВЕРЖДЕНЫ исправленными — «тень теперь красится корректно». Дефект №5
    cause A (вложенный скролл по высоте) НЕ подтверждён исправленным — «скроллинг ещё
    присутствует по высоте». Обнаружен НОВЫЙ регресс — жёсткая/резкая рамка вокруг iframe,
    пользователь предположил закрасить её цветом фона. Пользователь также поднял вопрос о
    двойном скролле для многостраничных документов (архитектурный принцип: `.pdf-page-frame`
    должен быть ЕДИНСТВЕННЫМ скролл-контейнером).
- timestamp: 2026-08-05T00:50:00
  observation: |
    `git log -p --follow -- ui/src/features/acts/PdfPreviewModal.svelte | grep -B5 "pdf-iframe
    drops its 1px border"` → коммит `5846bb0` ("feat(33-03): sheet-stack chrome, fit-to-width
    scale, progress markup") содержит `- border: 1px solid var(--tr-border);` (удалено) с
    сообщением коммита, явно ссылающимся на D-09 "без рамки". Ни этот, ни последующие коммиты
    НЕ добавили `border: none` взамен. Подтверждено чтением ТЕКУЩЕГО (до правки этого раунда)
    `.pdf-iframe`: никакого `border`-свойства нет вообще — только width/min-width/min-height/
    background/flex-shrink.
- timestamp: 2026-08-05T00:52:00
  observation: |
    Веб-поиск (не из памяти) подтверждает: WHATWG-спека default UA stylesheet для `<iframe>` —
    `iframe:not([seamless]) { border: 2px inset; }` (html.spec.whatwg.org/multipage/
    rendering.html). Это буквально и есть источник «жёсткой рамки», которую видит пользователь
    при отсутствии авторского `border`-объявления.
- timestamp: 2026-08-05T00:55:00
  observation: |
    Прямое чтение `ui/node_modules/.pnpm/pagedjs@0.4.3/node_modules/pagedjs/src/polyfill/
    previewer.js` (реальный src, не минифицированный бандл): `preview(content, stylesheets,
    renderTo)` — если `content` не передан (bootstrapScript.js:51 зовёт `previewer.preview()`
    БЕЗ аргументов), вызывается `this.wrapContent()`, который переносит текущее содержимое
    `<body>` в скрытый `<template data-ref="pagedjs-content">` (не имеет render-бокса, не
    влияет на scrollHeight).
- timestamp: 2026-08-05T00:57:00
  observation: |
    Прямое чтение `.../pagedjs/src/chunker/chunker.js` `setup(renderTo)`: `this.pagesArea =
    document.createElement('div'); this.pagesArea.classList.add('pagedjs_pages'); ...
    document.querySelector('body').appendChild(this.pagesArea)` — когда `renderTo` не передан
    (подтверждено: `chunker.flow(content, renderTo)` вызывается из `previewer.preview()` без
    третьего аргумента). Это ЕДИНСТВЕННЫЙ элемент, который библиотека когда-либо добавляет в
    `<body>` помимо инертного `<template>` выше. `grep -rn "body.appendChild|querySelector(\"
    body\")|document.body\." src` по всему дереву исходников pagedjs подтверждает: других
    точек мутации `<body>` нет нигде в библиотеке.
- timestamp: 2026-08-05T00:58:00
  observation: |
    `grep -rn "pagesArea\." .../pagedjs/src` → только `classList.add`, один CSS custom
    property-счётчик (`--pagedjs-page-count`), `.remove()` и добавление детей ВНУТРЬ pagesArea
    (page.js). Явной ширины/высоты на `.pagedjs_pages` нигде не устанавливается — её бокс
    полностью auto-sized по содержимому, то есть `scrollHeight` этого элемента и
    `document.documentElement.scrollHeight` структурно ИДЕНТИЧНЫ в этом DOM — предложенная в
    checkpoint-отчёте замена источника высоты была бы no-op, не фиксом.
- timestamp: 2026-08-05T01:00:00
  observation: |
    Прямое чтение `ui/src/styles/global.scss` строки 9-15: `*, *::before, *::after {
    box-sizing: border-box; }` — глобальный сброс, применяется и к `.pdf-iframe` (Svelte
    scoping не исключает элементы компонента из глобальных правил вне его `<style>`-блока).
    Под `box-sizing: border-box` объявленная `height` на `.pdf-iframe` — border-box размер,
    то есть ДОЛЖНА включать собственную рамку элемента. Из-за дефекта №6 (рамка не объявлена →
    действует UA-дефолт 2px inset) 4px (2px сверху + 2px снизу) высоты `naturalHeightPx`
    вычитались рамкой, оставляя фактический viewport внутри iframe на 4px короче, чем
    содержимое документа — именно это и проявлялось как «скролл всё ещё есть» (дефект №7),
    хотя сама `naturalHeightPx` (измерение из bootstrapScript.js) была уже полностью корректна.
- timestamp: 2026-08-05T01:02:00
  observation: |
    `grep -rn "iframe" ui/src/styles/*.scss ui/src/lib/components/Modal.svelte` → нет других
    правил, задающих border/box-sizing специально для iframe-элементов — механизм border-box +
    UA-дефолтная рамка является единственным действующим фактором.
- timestamp: 2026-08-05T01:05:00
  observation: |
    После правки (`.pdf-iframe` получил `border: none;`, `background` изменён с
    `var(--tr-n-0)` на `var(--tr-surface-sunken)`): `pnpm --dir ui svelte-check` → 0 ошибок,
    269 файлов, 48 warnings (тот же baseline, новых нет). `pnpm --dir ui lint` → PASS, включая
    `[check-pagedjs-csp-hash] OK` (хэш не затронут — правка не касается bootstrapScript.js).
    `pnpm --dir ui build` → успешно, `ui/dist` пересобран, без новых предупреждений/ошибок.

## Eliminated

- hypothesis: |
    Отложить вызов `previewer.preview()` (без аргументов) за `DOMContentLoaded`, чтобы
    устранить прерывистое зависание (ни resolve, ни reject) в гарнессе координатора.
  evidence: |
    Проверено координатором в отдельном браузерном гарнессе: перенос вызова за
    `DOMContentLoaded` НЕ устранил зависание — оно проявлялось так же прерывисто. Отклонено
    как рефутирующий тест этой гипотезы; причина зависания остаётся неизвестной (см. open
    lead ниже). Эта ветка НЕ является дефектом №3 (тот дефект — детерминированный TypeError
    на первой строке бутстрапа, всегда воспроизводимый, а не прерывистый).
  timestamp: 2026-08-04T23:40:00

## Open leads (не дефект, не чинить в этом раунде)

- lead: |
    Прерывистое зависание `previewer.preview()` (без аргументов): в гарнессе координатора
    иногда не settled ни resolve, ни reject — внешне неотличимо от репортящегося симптома
    (тишина, затем 8s таймаут). Успешно на первой загрузке, зависало на последующих в той же
    сессии гарнесса.
  confounder: |
    Сам гарнесс может быть артефактом (офскрин-iframe'ы / несколько экземпляров Previewer на
    один документ за сессию) — зависший Previewer, похоже, блокировал последующие прогоны,
    что делает более поздние замеры не независимыми друг от друга.
  instruction: |
    Если пользователь снова увидит 8s таймаут БЕЗ вообще какой-либо ошибки в консоли после
    фикса этого раунда — это оно и реально. Если пагинация проходит чисто — гарнесс был
    артефактом. Внести в UAT round 3 чеклист как «понаблюдать», не «починить».
  status: |
    ЗАКРЫТО КАК NOT REPRODUCED / harness artifact. UAT round 3 (живое приложение,
    пользователь): зависание НЕ воспроизвелось ни разу за сессию. Ничего в коде продукта не
    менялось для этого lead — он не «исправлен», он просто не подтвердился как реальная
    проблема; исходный гарнесс координатора признан артефактом (см. Evidence
    2026-08-04T23:50:00).

- lead: |
    Тёмная тема: тень `--tr-elev-2` (чёрная) на почти чёрной подложке (`#0a0d12`) может
    визуально не разделять лист и подложку (33-UI-SPEC.md, Dark-Theme note, строка 107-124;
    Open Items #2, строка 374-378).
  status: |
    MOOT после фикса дефекта №4 в этом раунде. С белым листом (`background: #fff`) на
    тёмной подложке контраст по яркости (~19:1, белый на почти чёрном) сам по себе
    достаточно разделяет лист и подложку без дополнительной рамки — граница/`--tr-border`
    fallback НЕ добавлен проактивно, как и предписано 33-UI-SPEC.md («escalation path, not
    a proactive change»). Пересмотреть только если будущий раунд UAT укажет на
    недостаточную визуальную сепарацию в тёмной теме.

## Resolution

root_cause: |
  Три независимых дефекта одного жизненного цикла, каждый маскировал следующий:

  1. В `PdfPreviewModal.svelte` iframe пагинации (`bind:this={iframeEl}`, единственный источник
  `trackly-pagedjs-progress`/`-done` через `attachBridge`) находился в ветке
  `{:else if htmlContent !== null}`, гейтящейся `{#if showLoading}`. `showLoading` истинно
  ровно тогда, когда `paginationStatus === 'pending'` — то есть сразу после того, как срабатывает
  `srcdoc` (стр. 197) и ДО того, как iframe вообще может смонтироваться. Из-за этого мост
  `postMessage` никогда не активировался, `paginationStatus` не мог сам выйти из `'pending'`,
  и единственным выходом оставался жёсткий 8s `degradeTimeoutHandle`, который всегда уводил в
  D-02. Подтверждено прямым чтением кода (структурное доказательство, не косвенная улика):
  единственные два `<iframe>` файла оба лежат внутри веток, недостижимых пока `showLoading`
  истинно, скрытого/прогревочного iframe нет.

  2. После фикса (1) — итоговая сборка `srcdoc`/temp-HTML использовала СТРОКОВЫЙ
  `.replace()`, интерпретирующий буквальную подстроку `` $` `` внутри встроенного
  `paged.min.js` как спецпаттерн, что разрывало бандл и давало `SyntaxError` при парсинге
  `<script>`.

  3. После фикса (2) — `ui/src/lib/pdfPreview/bootstrapScript.js:20` вызывал
  `new window.Paged.Previewer()`, но UMD-сборка `pagedjs/dist/paged.min.js` (0.4.3),
  реально импортируемая проектом, экспортирует себя как `globalThis.PagedModule`, а не
  `globalThis.Paged` (последнее — имя, используемое только отдельной сборкой
  `paged.polyfill.js`, которую проект не подключает). `window.Paged` был `undefined`,
  бутстрап падал с `TypeError` на первой же исполняемой строке — до того, как что-либо
  успевало отправить `postMessage` — и 8s degrade timeout снова уводил в D-02, уже по
  третьей, независимой причине. Подтверждено независимым чтением UMD-обёртки и хвоста
  экспортов `paged.min.js`, а также grep, показавшим, что `window.Paged` больше нигде в
  `ui/src` не используется и не алиасится.
fix: |
  1. `ui/src/features/acts/PdfPreviewModal.svelte` — разделил верхнеуровневую загрузку
     (`{#if loading}`, ещё нет `htmlContent`) от состояния "пагинация идёт" (`htmlContent`
     уже есть, `paginationStatus === 'pending'`). Iframe пагинации (`bind:this={iframeEl}`)
     теперь монтируется безусловно, как только собран `srcdoc`, независимо от
     `paginationStatus`. Пока пагинация не завершена, поверх него рендерится
     `.pagination-overlay` — тот же спиннер + текст прогресса, что раньше был
     верхнеуровневым состоянием, но теперь оверлеем через `position: absolute` +
     сплошной фон (НЕ `display:none`), не убирающий iframe из DOM.
  2. Ветка `'degraded'` (D-02) не тронута — путь деградации при реальном сбое Paged.js
     сохранён как есть.
  3. Второй, более мелкий дефект того же жизненного цикла: `case 'trackly-pagedjs-progress'`
     не чистил `degradeTimeoutHandle`, только `'done'`/`'error'` это делали.
     33-UI-SPEC.md прямо определяет таймаут как «8s от установки srcdoc до ПЕРВОГО
     trackly-pagedjs-progress ИЛИ trackly-pagedjs-done», а 33-RESEARCH.md Pitfall 1 —
     как детектор полной тишины, а не потолок на всю пагинацию. Добавил
     `clearDegradeTimeout()` в обработчик `'trackly-pagedjs-progress'`. Сегодня это было
     замаскировано первым багом (progress не приходил вообще), но стало бы реальной
     проблемой для многостраничных документов после починки (1) без этого фикса.
  4. ВТОРОЙ ДЕФЕКТ (обнаружен после UAT первого фикса, независимый от него): оба места
     сборки итогового HTML для превью-iframe/temp-файла системного браузера использовали
     `actHtml.replace(/<\/body>/i, \`${injected}</body>\`)` — СТРОКОВЫЙ replacement.
     `injected` встраивает целиком минифицированный `paged.min.js`, который содержит
     буквальную подстроку `` $` `` (стык regex-источника шаблонного литерала `` `[${t}]+$` ``
     и его закрывающего бэктика). Спека `String.prototype.replace` интерпретирует `$\`` в
     СТРОКОВОМ replacement-аргументе как спецпаттерн «текст до совпадения» — движок
     подставлял туда огромный кусок `actHtml`, разрывая шаблонный литерал бандла и давая
     `SyntaxError: Unexpected EOF` при парсинге `<script>` внутри `srcdoc`/temp-файла.
     Пагинация падала ДО того, как Paged.js успевал стартовать, и 8s-таймаут снова уводил в
     D-02 — уже по другой причине, чем первый дефект, который он маскировал.
     Исправлено заменой строкового replacement на replacer-ФУНКЦИЮ (`() => ...`), которая не
     интерпретирует `$`-паттерны — возвращаемое значение вставляется дословно:
       - `ui/src/lib/pdfPreview/pagedPreviewBootstrap.ts:71` (buildSrcdoc, превью-iframe)
       - `ui/src/features/acts/PdfPreviewModal.svelte:313` (printViaSystemBrowser,
         temp-HTML-файл для печати через системный браузер — тот же баг, независимо
         затрагивал desktop-печать, не пойман UAT одной только модалки превью).
     В обоих местах добавлен комментарий у call site, называющий именно эту ловушку с
     `$`-паттернами, чтобы никто не «упростил» обратно в строковый replacement.
     Регрессионный lint-гейт в `ui/scripts/` НЕ добавлен — рассмотрено и отклонено: дефект
     детерминирован формулой сборки бандла (не зависит от содержимого документа), уже
     покрыт structural-комментарием на обоих call site, а добавление отдельного
     фикстур-парсинга в lint-цепочку дублировало бы то, что уже делает `svelte-check`/`tsc`
     (replacer-функция типобезопасна) без реальной доп. защиты — посчитано избыточной
     церемонией для этого конкретного случая.
  5. ТРЕТИЙ ДЕФЕКТ (обнаружен после UAT второго фикса, независимый от обоих предыдущих):
     `ui/src/lib/pdfPreview/bootstrapScript.js:20` — `new window.Paged.Previewer()` заменён
     на `new window.PagedModule.Previewer()`, чтобы совпадать с реальным именем глобала,
     под которым UMD-сборка `pagedjs/dist/paged.min.js` (0.4.3) прикрепляет свои экспорты.
     Добавлен комментарий у вызова, явно называющий разницу между `paged.min.js`
     (`PagedModule`) и `paged.polyfill.js` (`Paged`), чтобы никто не «откатил» обратно.
     Это САНКЦИОНИРОВАННОЕ координатором единственное исключение из D-14 lock на этот
     раунд — после правки байтов файла пересчитан CSP sha256-хэш
     (`node ui/scripts/check-pagedjs-csp-hash.mjs --print` →
     `sha256-1nG6ajqUxHpGqTH1xMQEfH1DAoyP3C8xrIMr3PNVhPQ=`, был
     `sha256-5ZDjul5PEiak1qhxbmi9Rx3W4tYmf4sQbt9wgef8vQY=`) и обновлена соответствующая
     константа в `crates/trackly-app/src/http/mod.rs` (строка `HeaderValue::from_static(...)`
     в CSP-заголовке `script-src`). Не тронуты: `crates/trackly-app/templates/*.html` (D-01),
     `.modal-pdf-preview` в `Modal.svelte` (D-12) — оба не относятся к этому дефекту.
verification: |
  Дефект №1 (монтирование iframe) — самопроверка была только статическая (см. предыдущую
  версию этой секции), ПОДТВЕРЖДЕНА пользователем как необходимая, но недостаточная — второй
  дефект (ниже) маскировался первым и проявился только после починки первого.

  Дефект №2 ($-паттерны в String.replace) — проверено ДВУМЯ независимыми способами, не
  просто принято со слов пользователя:
  1. Прямой grep реального `node_modules/pagedjs/dist/paged.min.js`: ровно одно вхождение
     `` $` ``, ноль `$'`, ноль `$&`, ноль `</script` — совпадает с заявленным.
  2. Эмпирическое воспроизведение вне приложения (Node-скрипт с реальными файлами бандла и
     бутстрапа, та же формула сборки, что в продакшн-коде): версия со строковым replacement
     ломает целостность бандла и не проходит `new Function(body)`-парсинг; версия с
     replacer-функцией сохраняет бандл байт-в-байт и парсится без ошибок.

  Дефект №3 (window.Paged vs window.PagedModule) — проверено независимо, не принято со слов
  checkpoint-ответа:
  1. Прямое чтение `paged.min.js` (голова + хвост): UMD-обёртка прикрепляется к
     `globalThis.PagedModule`, экспорт `.Previewer` — свойство этого же объекта.
  2. `grep -c "window.Paged\b" ui/src -r`: единственное совпадение — сама проблемная строка
     в `bootstrapScript.js`, нигде в приложении нет алиаса `window.Paged = window.PagedModule`.
  3. `grep -o "Paged=" paged.polyfill.min.js`: подтверждает, что `window.Paged` — реальное имя
     ОТДЕЛЬНОЙ сборки, которую проект не импортирует.

  Статические гейты после ВСЕХ ТРЁХ фиксов:
  - `pnpm --dir ui svelte-check` — 0 ошибок (269 файлов, те же 48 ранее существовавших
    предупреждений в других файлах, ни одного нового).
  - `pnpm --dir ui lint` — PASS, включая `check-pagedjs-csp-hash` (после регенерации хэша —
    подтверждает, что новый хэш `bootstrapScript.js` синхронизирован с константой в
    `http/mod.rs`, дрейфа нет).
  - `pnpm --dir ui build` — успешен, `ui/dist` пересобран.
  - `cargo check -p trackly-app` — успешен (Rust-сторона компилируется с новой CSP-константой).

  ДЕФЕКТ №3 (window.Paged vs window.PagedModule) — ПОДТВЕРЖДЁН ИСПРАВЛЕННЫМ пользователем в
  UAT round 3, живое приложение: пагинация быстрая, счётчик страниц показывает «1 страница»,
  таймаута нет, TypeError нет. Прерывистое зависание `previewer.preview()` (open lead) НЕ
  воспроизвелось — закрыто как not reproduced/harness artifact, не как «исправлено» (см.
  Open leads).

  ДЕФЕКТ №4 (белый лист / отсутствующий `background` у `.pagedjs_page`) — статически
  перепроверен независимо (не принят со слов checkpoint-ответа), см. Evidence
  2026-08-04T23:55:00 — 23:58:00: прямое чтение кода до правки подтвердило отсутствие
  background-объявления у `.pagedjs_page`; D-08/D-09 и 33-UI-SPEC.md перепроверены по
  первоисточникам, а не приняты со слов; CSP-хэш формула перепроверена — подтверждено, что
  правка style-строки её не затрагивает; `.pdf-page-frame`/`.pdf-iframe` перепроверены —
  подтверждено, что оба уже корректны и не требуют правок.

  После фикса дефекта №4:
  - `pnpm --dir ui svelte-check` — 0 ошибок (269 файлов, тот же baseline 48 warnings).
  - `pnpm --dir ui lint` — PASS, включая `check-pagedjs-csp-hash` OK (хэш НЕ затронут правкой
    style-строки, как и предполагалось до правки — подтверждено фактическим прогоном, а не
    принято на веру).
  - `pnpm --dir ui build` — успешно, `ui/dist` пересобран.

  ДЕФЕКТ №4 (белый лист) — ПОДТВЕРЖДЁН ИСПРАВЛЕННЫМ пользователем в UAT round 4, живое
  приложение, ОБЕ темы. Закрыт.

  ДЕФЕКТ №5 (вложенный скролл внутри iframe + обрезанная/задвоенная тень листа), три
  независимые причины — каждая перепроверена независимым чтением реального кода, ни одна не
  принята со слов checkpoint-ответа (см. Evidence 2026-08-05T00:10:00 — 00:15:00):

  CAUSE A — `.pdf-iframe` (PdfPreviewModal.svelte) хардкодил `height: 1123px; min-height:
  1123px`, никак не связанный с `naturalHeightPx`, который уже корректно управлял высотой
  соседней обёртки `.pdf-scale-inner`. Контент внутри (страница + вертикальный padding
  `.pagedjs_pages`, 16px сверху и снизу) всегда превышает 1123px даже для ОДНОЙ страницы
  (1123 + 32 = 1155px) → у самого iframe как replaced-элемента появляется СОБСТВЕННЫЙ
  скроллбар, вложенный в скролл `.pdf-page-frame`. Для многостраничных документов это
  критичнее: страницы 2+ были бы скрыты за внутренним скроллом — это доказуемо СЛЕДУЕТ из
  прочитанного CSS, но ни разу не наблюдалось живьём (см. блок «многостраничность» ниже).

  CAUSE B — `pagedPreviewBootstrap.ts`'s инжектируемый `.pagedjs_pages` имел
  `padding: 16px 0` (горизонтальный — ноль). Ширина `.pagedjs_page` равна ширине iframe
  (794px, из @page/D-01), поэтому тени листа (уже корректно рисуемой per-sheet после фикса
  дефекта №4) было физически некуда painting — обрезалась по обоим краям, а её spread
  провоцировал горизонтальный оверфлоу/скролл, усугубляя CAUSE A.

  CAUSE C — `.pdf-iframe` всё ещё нёс `box-shadow: var(--tr-elev-2)` — рудимент дизайна
  ДО Phase 33 (единственный лист без пагинации). После фикса дефекта №4 тень уже правильно
  рисуется НА КАЖДОЙ странице внутри iframe (D-09) — внешняя тень на самом iframe стала
  ВТОРОЙ, избыточной тенью, обводящей ВЕСЬ бокс iframe (то есть весь стек страниц, а не
  каждый лист по отдельности) — структурно конфликтует с per-sheet требованием D-09 для
  N > 1 страниц.

  НАПРАВЛЕНИЕ (расширить фрейм, НЕ переносить тень на сам iframe) — перепроверено независимо,
  подтверждено верным: тень на элементе iframe способна обвести только ОДИН бокс целиком;
  после фикса CAUSE A этот бокс охватывает ВЕСЬ стек страниц (D-04, многостраничность), то
  есть тень на iframe в принципе не может быть «per sheet» для N > 1 — расширение фрейма
  (горизонтальный gutter + ширина iframe/scale-inner + делитель scaleFactor, все → 842px =
  794 + 2×24) — единственный вариант, совместимый с уже зафиксированным D-09.
fix: |
  ДЕФЕКТ №5, три точечные правки, каждая нацелена на свою причину, не на общий симптом:

  1. `ui/src/lib/pdfPreview/pagedPreviewBootstrap.ts` — `.pagedjs_pages` padding
     `16px 0` → `16px 24px` (CAUSE B). 24px переиспользует уже одобренный токен
     `--tr-space-xl` (буквальное значение, не ссылка на custom property — iframe
     opaque-origin не может резолвить токены родительского приложения, тот же паттерн, что
     уже применён для `chrome.backdrop`/`chrome.shadow` в этом файле).
  2. `ui/src/features/acts/PdfPreviewModal.svelte`:
     - `scaleFactor` делитель `794` → `842` (794 + 2×24) — иначе fit-to-width математика
       D-11 недомасштабировала бы лист и возвращала горизонтальный оверфлоу на узких ширинах.
     - `.pdf-scale-inner` инлайновый `width: 794px` → `width: 842px`.
     - `<iframe>` получил инлайновый `style="height: {naturalHeightPx}px"` — та же
       переменная, что уже управляет `.pdf-scale-inner`, вместо статичного CSS-правила
       (CAUSE A). Подтверждено чтением `bootstrapScript.js:53-54`: `naturalHeightPx`
       (`pagesEl.scrollHeight`) уже включает вертикальный padding `.pagedjs_pages`
       по спецификации CSS box model — доп. правки по вертикальной оси не требуется.
     - `.pdf-iframe` CSS: `width`/`min-width` `794px` → `842px`; статичный `height: 1123px`
       удалён (высота теперь только инлайновая); `min-height: 1123px` ОСТАВЛЕН как
       плейсхолдер до первого `trackly-pagedjs-done` (совпадает с начальным `naturalHeightPx
       = $state(1123)`); `box-shadow: var(--tr-elev-2)` УДАЛЁН (CAUSE C) — единственная тень
       теперь per-sheet, внутри iframe, как и требует D-09.
  3. `.planning/phases/33-print-preview-polish/33-UI-SPEC.md` — добавлена вторая строка в
     таблицу Spacing Scale, документирующая новое использование `--tr-space-xl` (24px) как
     горизонтального gutter, со ссылкой на этот debug-раунд; существующая строка про
     вертикальный inter-sheet gap НЕ тронута/не переписана — правка scoped к фактически
     изменившемуся числу.

  Комментарии у всех трёх мест правки в коде явно называют дефект и debug-сессию, чтобы никто
  не «упростил» разъединённые width/height/scaleFactor-значения обратно рассинхронизировав их.
verification: |
  ДЕФЕКТ №4 — ПОДТВЕРЖДЁН исправленным пользователем в UAT round 4 (обе темы, живое
  приложение). Закрыт полностью.

  ДЕФЕКТ №5 — все три причины перепроверены НЕЗАВИСИМЫМ чтением фактического кода ДО правки
  (не приняты со слов checkpoint-ответа, см. Evidence 2026-08-05T00:10:00 — 00:15:00):
  CAUSE A подтверждена буквальным чтением `.pdf-iframe`'s статичной `height`/`min-height` и
  отсутствием какой-либо динамической привязки на самом iframe-элементе; CAUSE B подтверждена
  буквальным чтением нулевого горизонтального padding; CAUSE C подтверждена сопоставлением
  двух объявлений тени (per-sheet внутри iframe — уже корректно от фикса №4; на самом iframe —
  избыточно). Направление фикса (расширить фрейм, не переносить тень на iframe) перепроверено
  на предмет совместимости с D-04 (многостраничность) и D-09 (per-sheet тень) по
  первоисточнику `33-CONTEXT.md`, а не принято на веру.

  После правки:
  - `pnpm --dir ui svelte-check` — 0 ошибок, 269 файлов, 48 warnings (тот же baseline,
    новых нет).
  - `pnpm --dir ui lint` — PASS, включая `[check-pagedjs-csp-hash] OK` — подтверждено
    фактическим прогоном, что CSP-хэш не затронут (ни один из трёх изменённых файлов входит
    в формулу хэша `PAGED_PREVIEW_INLINE_SCRIPT`; `bootstrapScript.js` байты не менялись
    в этом раунде).
  - `pnpm --dir ui build` — успешно, `ui/dist` пересобран, без новых предупреждений/ошибок.

  ЧЕСТНО НЕ ДОКАЗАНО (нужна проверка в запущенном приложении, UAT round 5):
  - что вложенный скролл внутри iframe действительно исчез на реальном рендере (статические
    рассуждения о CSS box model обоснованы, но рантайм-манипуляции Paged.js с размерами
    `.pagedjs_page` не трассировались инструкция-за-инструкцией, только их CSS-поверхность);
  - что тень листа теперь красится полностью, без обрезки, по обоим краям, в ОБЕИХ темах;
  - что печать (`printViaSystemBrowser`/`printViaTopLevel`) по-прежнему работает корректно —
    эти пути не используют `.pdf-iframe`/`.pdf-scale-inner` (свой temp-HTML/top-level DOM),
    формально не затронуты этой правкой, но не перепроверялись в этом раунде;
  - МНОГОСТРАНИЧНЫЙ случай — ЭТОТ ПРОЕКТ НИ РАЗУ НЕ НАБЛЮДАЛ многостраничное превью работающим
    end-to-end (только однострочные акты проходили UAT до сих пор, включая round 4).
    Статические рассуждения о причине A/её фиксе для N>1 страниц логически обоснованы, но
    НЕ являются подтверждением того, что реальная многостраничная пагинация, реальный per-page
    рендер тени и реальное отсутствие реflow работают корректно — это ПОЛНОСТЬЮ не проверено
    и не должно преподноситься как проверенное.

UAT_round_5_result: |
  Пользователь подтвердил вживую: дефект №5 causes B/C (тень листа) ИСПРАВЛЕНЫ — тень теперь
  красится корректно, без обрезки. Дефект №5 cause A (вложенный скролл) НЕ подтверждён
  исправленным — «скроллинг ещё присутствует по высоте» (фикс round 5, динамическая высота
  iframe от naturalHeightPx, оказался НЕДОСТАТОЧНЫМ — см. дефект №7 ниже за настоящую причину
  остатка). Обнаружен НОВЫЙ регресс в этом же раунде — дефект №6, жёсткая рамка iframe.

root_cause: |
  ДЕФЕКТ №6 (регресс, жёсткая рамка iframe) — независимо перепроверено (git log, не принято со
  слов пользователя): коммит `5846bb0` ("feat(33-03): sheet-stack chrome, fit-to-width scale,
  progress markup") намеренно удалил `border: 1px solid var(--tr-border);` с сообщением коммита
  "D-09 «без рамки»", но НЕ добавил взамен `border: none`. Без объявленной рамки действует
  дефолтный UA-стиль браузера для `<iframe>`: `iframe:not([seamless]) { border: 2px inset; }`
  (WHATWG html.spec.whatwg.org — подтверждено веб-поиском, не из памяти) — это и есть «жёсткая
  граница», которую увидел пользователь.

  ДЕФЕКТ №7 (вложенный вертикальный скролл, остаток после round 5's fix) — ИЗМЕРЕНО чтением
  РЕАЛЬНЫХ исходников pagedjs 0.4.3 (`src/polyfill/previewer.js`, `src/chunker/chunker.js`, не
  минифицированного бандла), НЕ принято со слов checkpoint-отчёта: `naturalHeightPx`
  (`pagesEl.scrollHeight` для `.pagedjs_pages`, читается в bootstrapScript.js) уже была
  корректным измерением — `previewer.preview()` вызывается БЕЗ аргументов, из-за чего
  `wrapContent()` прячет исходное содержимое body в инертный `<template>`, а
  `Chunker.setup()` добавляет в body РОВНО ОДИН элемент, `.pagedjs_pages`, без явной
  высоты (auto-sized по содержимому). Grep всего дерева исходников pagedjs подтверждает: другие
  точки мутации `<body>` в библиотеке отсутствуют — то есть `documentElement.scrollHeight`
  дало бы ТО ЖЕ САМОЕ значение, что и `.pagedjs_pages.scrollHeight`; замена источника высоты,
  предложенная в checkpoint-отчёте, была бы no-op, не фиксом. Настоящий механизм: `global.scss`
  применяет универсальный `box-sizing: border-box` (в т.ч. к `.pdf-iframe` — глобальное правило,
  Svelte-scoping его не исключает), поэтому объявленная `height` на `.pdf-iframe` — border-box
  размер, включающий рамку. Из-за дефекта №6 (рамка не объявлена → UA-дефолт 2px inset) 4px
  (2px сверху + 2px снизу) высоты `naturalHeightPx` съедались рамкой, оставляя реальный
  viewport внутри iframe на 4px короче содержимого — именно это проявлялось как «скролл ещё
  есть», хотя сама `naturalHeightPx` была полностью верна. Дефекты №6 и №7 — ОДНА и та же
  недостающая декларация, а не два независимых бага.
fix: |
  Оба дефекта устранены ОДНОЙ правкой в `ui/src/features/acts/PdfPreviewModal.svelte`,
  `.pdf-iframe`:
  1. Добавлен `border: none;` — убирает UA-дефолтную 2px-inset рамку (дефект №6) И, благодаря
     border-box сбросу, возвращает фактический viewport внутри iframe к `naturalHeightPx`
     БЕЗ вычета рамки (дефект №7) — без единой правки в `bootstrapScript.js` (высота там уже
     была верна, подтверждено измерением, не переписывалась).
  2. `background` изменён с `var(--tr-n-0)` (белый) на `var(--tr-surface-sunken)` (та же
     подложка, что уже красит `.pdf-page-frame` вокруг) — убирает белую вспышку в момент перед
     тем, как iframe-документ красит собственный `body` поверх всего viewport'а (особенно
     заметно в тёмной теме на почти-чёрной подложке). НЕ закрашен цветом фона, как предположил
     пользователь в чате про саму рамку — рамка убрана полностью (`none`), а не перекрашена:
     iframe — прозрачный viewport на подложку, и per-sheet тень (D-09) уже разделяет лист и
     подложку; любая рамка на самом iframe вернула бы второй, избыточный контур вокруг ВСЕГО
     стека страниц (та же логика, что и в defect #5 cause C).
  `bootstrapScript.js` НЕ тронут этим раундом (санкционированное round-3 исключение из D-14
  осталось единственным); CSP-хэш НЕ пересчитывался — подтверждено `check-pagedjs-csp-hash`
  прогоном, не на веру.
verification: |
  ДЕФЕКТ №6 — независимо перепроверен чтением `git log -p` (коммит `5846bb0`, удаление рамки
  без замены) и веб-поиском (WHATWG UA-стиль `iframe { border: 2px inset }`), не принят со
  слов checkpoint-отчёта.

  ДЕФЕКТ №7 — измерено чтением РЕАЛЬНЫХ исходников pagedjs (не минифицированного бандла):
  подтверждено, что `.pagedjs_pages` — единственный элемент, добавляемый в body, auto-sized,
  без внешнего вклада в высоту; следовательно `naturalHeightPx` уже было верным измерением, а
  реальный механизм остатка — border-box + отсутствующая рамка (дефект №6). Это ОПРОВЕРГАЕТ
  конкретное предположение checkpoint-отчёта о том, что `documentElement.scrollHeight` дал бы
  другое (более полное) значение — измерение показывает, что оба источника идентичны в этом
  DOM, и что менять источник высоты в `bootstrapScript.js` не требовалось.

  После правки:
  - `pnpm --dir ui svelte-check` — 0 ошибок, 269 файлов, 48 warnings (тот же baseline).
  - `pnpm --dir ui lint` — PASS, включая `[check-pagedjs-csp-hash] OK` (хэш не затронут —
    подтверждено фактическим прогоном; `bootstrapScript.js` байты не менялись).
  - `pnpm --dir ui build` — успешно, `ui/dist` пересобран, без новых предупреждений/ошибок.

  ЧЕСТНО НЕ ДОКАЗАНО (нужен реальный запущенный рендер, UAT round 6):
  - что 4px border-box дефицит был ЕДИНСТВЕННЫМ источником переполнения — второстепенный
    эффект округления `scrollHeight` до целого от дробной раскладки (обычно ≤1px) статическими
    рассуждениями не проверяется и требует визуального подтверждения;
  - визуально в ОБЕИХ темах — рамка `inset` зависит от currentColor, могла выглядеть
    по-разному в разных темах; удаление устраняет эффект независимо от темы, но подтверждение
    остаётся за UAT;
  - `overflow: hidden` НЕ добавлен как safety net в этом раунде — источник высоты не менялся,
    и добавление жёсткого клиппинга поверх непроверенного вживую фикса рискует молча обрезать
    контент, если небольшой остаточный зазор всё же есть — это хуже, чем такой же небольшой
    остаточный скролл;
  - МНОГОСТРАНИЧНЫЙ случай — ПО-ПРЕЖНЕМУ НИ РАЗУ НЕ НАБЛЮДАЛСЯ end-to-end ни в одном раунде UAT
    (round 4 и round 5 — оба однострочные акты). Рассуждение этого раунда о `.pagedjs_pages`'s
    auto-sized боксе логически одинаково применимо для N страниц, но это НЕ подтверждение
    реального многостраничного рендера.
files_changed:
  - ui/src/lib/pdfPreview/bootstrapScript.js
  - crates/trackly-app/src/http/mod.rs
  - ui/src/features/acts/PdfPreviewModal.svelte
  - ui/src/lib/pdfPreview/pagedPreviewBootstrap.ts
  - .planning/phases/33-print-preview-polish/33-UI-SPEC.md

---

## Closure (2026-08-05)

**UAT round 6: подтверждено пользователем — «Всё замечательно».**

Итог: шесть независимых дефектов одного жизненного цикла, каждый маскировал следующий.
Ни один не был пойман автоматикой (`svelte-check`, `lint`, `build`, Rust-тесты были зелёными
на всех шести раундах) — во всех случаях типы и синтаксис были корректны, ломалась логика
рендера, разметка или CSS box model.

| # | Дефект | Файл | Как найден |
|---|--------|------|------------|
| 1 | iframe пагинации не монтировался (`showLoading` скрывал ветку с ним) | PdfPreviewModal.svelte | чтение кода |
| 2 | `$`-паттерны в строковом `.replace()` рвали бандл → `SyntaxError` | pagedPreviewBootstrap.ts + PdfPreviewModal.svelte | консоль UAT + подсчёт байт |
| 3 | `window.Paged` вместо `window.PagedModule` (UMD vs polyfill) | bootstrapScript.js | консоль UAT + UMD-заголовок бандла |
| 4 | у `.pagedjs_page` не задан фон → лист цвета подложки | pagedPreviewBootstrap.ts | скриншот UAT + чтение CSS |
| 5 | жёсткая высота iframe / нет места тени / дубль тени | PdfPreviewModal.svelte + pagedPreviewBootstrap.ts | скриншот UAT + чтение CSS |
| 6 | UA-дефолтная рамка iframe: видимая граница И −4px высоты через `box-sizing: border-box` | PdfPreviewModal.svelte | `git log -p` + чтение global.scss |

**Отвергнутые гипотезы (важны, чтобы к ним не возвращались):**
- Запуск бутстрапа до `DOMContentLoaded` — проверено, не воспроизводит, ОТВЕРГНУТО.
- Занижение высоты в `pagesEl.scrollHeight` vs `document.documentElement.scrollHeight` —
  проверено по исходникам Paged.js, значения структурно совпадают; правка захэшированного
  `bootstrapScript.js` НЕ потребовалась.
- Периодическое подвисание `preview()` — наблюдалось только во внешнем стенде оркестратора,
  в приложении не воспроизвелось; признано артефактом стенда.

**Извлечённый урок для фазы:** все шесть раундов UAT шли на одностраничном акте. Многостраничное
превью на момент закрытия сессии end-to-end не наблюдалось — см. открытый пункт в
`33-VALIDATION.md`.

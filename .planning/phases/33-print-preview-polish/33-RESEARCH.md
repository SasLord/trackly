# Phase 33: Полировка предпросмотра печати - Research

**Researched:** 2026-08-04
**Domain:** Client-side CSS Paged Media pagination (Paged.js) inside a sandboxed `srcdoc` iframe, cross-context printing, CSP interaction
**Confidence:** MEDIUM (core Paged.js mechanics VERIFIED by direct inspection of the installed package source; the CSP-inheritance interaction — the single biggest risk in this phase — is verified against the CSP3 spec/MDN via WebSearch but not against Trackly's exact runtime behavior, since no browser test was run in this session)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**D-01:** Шаблоны `act_handover.html` / `act_acceptance.html` / `report.html` **не
изменяются**. `@page { size: A4 portrait; margin: 20mm 15mm }`, который уже есть во всех трёх,
остаётся **единственным источником полей** и для экрана, и для печати — Paged.js (D-04) читает
именно его и строит для каждой страницы область, отступлённую на эти поля.

**D-02:** Деградация вместо поломки: если Paged.js не отработал (не загрузился, упал,
выбросил), модалка показывает документ как раньше — непагинированный, в iframe — а не пустой
экран.

**D-03:** Граница приёмки PRV-03 — **дефолтные настройки диалога печати** (масштаб 100%, поля
по умолчанию). Ручные правки пользователя в диалоге вне зоны гарантии. Плюс короткая
строка-подсказка в футере модалки.

**D-04:** Пагинация — **Paged.js** (полифилл CSS Paged Media). Отвергнуты: свой пагинатор по
блокам верхнего уровня и визуальные зазоры каждые 1123px.

**D-05:** Рендер остаётся в `<iframe srcdoc>`, но `sandbox` меняется с `""` на
**`"allow-scripts"`** — без `allow-same-origin`. Документ в opaque origin. Отвергнуты: рендер в
top-level DOM (смешение стилей) и `allow-scripts allow-same-origin` (снимает sandbox).

**D-06:** **Печать тоже идёт через Paged.js.** Обе ветки печати — desktop (temp `.html` →
системный браузер) и LAN (инъекция в top-level `#act-print-root`) — печатают уже разбитую
Paged.js разметку, а не исходный поток. Обе ветки переделываются и перепроверяются.

**D-07:** Пока Paged.js пагинирует — спиннер с прогрессом страниц («Страница N…»). Кнопка
«Печать» заблокирована до завершения пагинации.

**D-08:** Подложка — `--tr-surface-sunken` (следует теме приложения). Лист — **всегда белый**.
Отвергнут вариант с инверсией самого листа в тёмной теме.

**D-09:** Оформление листа — тень `--tr-elev-2` + вертикальный зазор между листами, **без
рамки**. Штатный `interface.css` от Paged.js не используется.

**D-10:** Счётчик страниц («3 страницы») — **в шапке/футере модалки, вне документа**.

**D-11:** **Авто-вписывание по ширине** через `transform: scale`, с потолком 100%. Горизонтального
скролла нет никогда. Трансформ чисто экранный, на печать не влияет.

**D-12:** Размеры модалки **не меняются** — `.modal-pdf-preview` остаётся `min(95vw, 1100px) ×
min(90vh, 920px)`. `Modal.svelte` не трогаем.

**D-13:** Структурный автотест: все три шаблона содержат `@page` с одинаковыми `size` и
`margin` **плюс** ручной UAT на реальной печати/PDF.

**Ограничения (выведены, не обсуждались):**
- **C-01:** При opaque origin (D-05) родительское окно **не может** прочитать `scrollHeight`
  iframe. Инжектированный в `srcdoc` скрипт обязан передавать наружу через `postMessage`:
  итоговую высоту, число страниц, прогресс пагинации.
- **C-02:** Бандл Paged.js должен **инлайниться в `srcdoc`** — CDN запрещён (portable +
  self-contained). То же касается temp-файла desktop-ветки печати.
- **C-03:** Desktop-ветка печати сейчас вызывает `window.print()` через `setTimeout` после
  события `load`. С Paged.js авто-печать должна ждать **завершения пагинации**, а не `load`.

### Claude's Discretion

- Точная величина зазора между листами и радиус/параметры тени в пределах токенов.
- Формулировка строки-подсказки про настройки диалога печати (D-03).
- Формат текста счётчика страниц и его точное место в шапке или футере (D-10).
- Механика передачи цвета подложки в `srcdoc` и реакция на переключение темы при открытой
  модалке (D-08).

### Deferred Ideas (OUT OF SCOPE)

None — обсуждение не выходило за границы фазы.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-------------------|
| PRV-01 | Модалка предпросмотра печати показывает документ как лист формата A4 над сероватой подложкой-фоном — визуально как предпросмотр печати в Word | Paged.js `Previewer`/`Chunker` mechanics (Architecture Diagram, Pattern 1); D-08/D-09 sheet-chrome CSS guidance (Anti-Patterns, Pitfall 3 re: not re-declaring `@page`) |
| PRV-02 | Лист предпросмотра имеет внутренние поля (margins) соответствующие полям печати | Confirmed identical `@page{size:A4 portrait; margin:20mm 15mm}` across all 3 templates (Code Examples); Paged.js reads this via `removeStyles()`/`Polisher` (Primary sources) |
| PRV-03 | Печать совпадает с предпросмотром (WYSIWYG) через `@media print`, единый источник стилей | D-06 print-path redesign (Architecture Diagram print branches); CSP inline-script risk (Summary, Pitfall 1) and double-`@page`-margin non-risk (Pitfall 3) are the two biggest threats to this guarantee; Pitfall 4 flags the re-run-determinism assumption needing UAT |
</phase_requirements>

## Summary

Phase 33 replaces the flat, single-page `<iframe sandbox="">` preview in `PdfPreviewModal.svelte`
with real pagination via **Paged.js** (`pagedjs` on npm, MIT, v0.4.3 verified on the registry),
rendered inside the same `srcdoc` iframe but with `sandbox="allow-scripts"` (D-05), and reused
for both print branches so screen and paper share one pagination engine (D-06).

The architecture decomposes cleanly into three independent execution contexts, and each has a
**different** constraint profile — this distinction did not exist before this phase and is easy
to miss during planning:

1. **On-screen preview** — Paged.js must run *inside* the opaque-origin `srcdoc` iframe, which
   means it can only be delivered as **inline `<script>` text** embedded in the `srcdoc` string
   (there is no way to hand a live JS module reference across an opaque-origin frame boundary).
2. **LAN-browser print (`printViaTopLevel`)** — this runs in the **app's own top-level document**,
   which is not sandboxed. Paged.js can and should be imported normally
   (`import { Previewer } from 'pagedjs'`, dynamically code-split) and invoked as a function call
   against `#act-print-root`. No inline script is needed here.
3. **Desktop print (`printViaSystemBrowser`)** — a standalone `file://` document opened in the
   OS default browser, outside any CSP (confirmed: `tauri.conf.json` sets `"security": {"csp":
   null}` for the app itself, and Tauri does not control the *external* browser it hands the file
   to). Inline scripts here are unrestricted; C-02's "no CDN" requirement is about self-containment,
   not CSP.

**The critical finding, and the reason context #1 is risky:** in LAN-browser mode, the app's own
axum-served SPA page carries `Content-Security-Policy: script-src 'self'` (no `'unsafe-inline'` —
verified in `crates/trackly-app/src/http/mod.rs:189-208`, comment `WR-07`). Per the CSP3
specification and confirmed by multiple independent sources, **an `<iframe srcdoc>` document
inherits its creator's CSP regardless of `sandbox`** — sandboxing isolates *origin*, not *CSP
enforcement*. This means the inline Paged.js `<script>` inside the preview iframe will be
**silently blocked by the browser in LAN mode** unless the CSP is loosened for that one script.
`'unsafe-inline'` is a non-starter (already deliberately rejected once, per the WR-07 comment).
The practical fix is a **CSP hash source** (`script-src 'self' 'sha256-<digest>'`) computed once
at build time from the bundled/bootstrap script's exact static text and hardcoded into the axum
security-headers layer, plus a drift-detection check. **This requires a small change to
`crates/trackly-app/src/http/mod.rs` (Rust)** — a file outside the "100% frontend" framing in
33-CONTEXT.md. This is flagged as an Open Question for the planner/user rather than assumed away.

**Primary recommendation:** Ship Paged.js as (a) a raw-text UMD bundle (`pagedjs/dist/paged.min.js`,
imported via Vite's `?raw` suffix) inlined into the preview `srcdoc`, driving pagination through a
small first-party bootstrap script that reports progress/height/errors to the parent via
`postMessage` (targetOrigin `'*'`, validated on receipt via `event.source === iframeEl.contentWindow`
since `event.origin` will be the literal string `"null"`); and (b) a normal ESM `import('pagedjs')`
in the app bundle for the `printViaTopLevel` print path, re-running pagination independently rather
than trying to extract DOM out of the opaque-origin iframe (which is not possible without
`allow-same-origin`). Because Paged.js's `@page` handling operates in absolute physical units (mm),
re-running it in a second DOM context should produce identical page breaks — but this determinism
claim needs a manual UAT check (screen vs. print comparison), which dovetails with D-13's existing
manual-verification requirement.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Pagination of act/report HTML into A4 sheets | Browser / Client | — | Paged.js is a pure client-side CSS-Paged-Media polyfill; no server involvement (D-04). |
| On-screen preview rendering (sheet stack + grey backdrop) | Browser / Client | — | `srcdoc` iframe + `PdfPreviewModal.svelte`; theme color sourced from `_tokens.scss` already resolved client-side. |
| Print output (both branches) | Browser / Client | — | Both `printViaSystemBrowser` (system browser, file://) and `printViaTopLevel` (app top-level document) execute entirely client-side; no new backend PDF generation (D-01/D-06). |
| HTML document source (act/report markup + `@page`) | API / Backend | — | Unchanged (D-01) — `act_service`/`report_service` MiniJinja rendering, out of scope for this phase. |
| CSP header allowing the inline pagination script (LAN mode) | API / Backend | Browser / Client | The policy lives in `crates/trackly-app/src/http/mod.rs` (axum `security_headers` layer) — a **backend** file. This is the one place this "frontend-only" phase likely needs a backend touch. Flagged as Open Question below. |
| Cross-context communication (iframe → parent: height/progress/errors) | Browser / Client | — | `postMessage` bridge (C-01), both ends are client JS. |

## Package Legitimacy Audit

| Package | Registry | Age | Downloads | Source Repo | slopcheck | Disposition |
|---------|----------|-----|-----------|--------------|-----------|-------------|
| `pagedjs` | npm | ~7 yrs (first published 2019, current 0.4.3) | high (used by Vivliostyle-adjacent publishing tools, Coko Foundation projects; exact weekly count not queried) | `github.com/pagedjs/pagedjs` (Coko Foundation / pagedmedia.org) | `[OK]` (verified via `slopcheck install pagedjs --ecosystem npm`, this session) | Approved |

**Packages removed due to slopcheck [SLOP] verdict:** none.
**Packages flagged as suspicious [SUS]:** none.

**Provenance note:** `pagedjs` was discovered via WebSearch/training knowledge, then verified two
ways this session: (1) `npm view pagedjs version license` → `0.4.3`, `MIT` (registry-confirmed),
(2) `slopcheck install pagedjs --ecosystem npm` → `[OK]`. Per the package-name-provenance rule,
this is tagged `[ASSUMED]`-origin-but-registry-and-slopcheck-verified — the planner should still
treat "this is the correct/canonical package for CSS Paged Media polyfilling" as a claim resting on
training knowledge + the package's own self-description, not an official Trackly-external
authority. No Context7 entry was available for `pagedjs` in this session (not checked — see
Sources). Confidence: MEDIUM.

**Housekeeping note:** verifying this package caused `slopcheck`'s `npm install` step to run in the
repository root (not `ui/`), creating a stray root-level `node_modules/`, `package.json`, and
`package-lock.json`. These were removed before finishing this research session; `git status`
confirmed clean afterward. Flagging so the planner does not need to re-investigate if evidence of
this surfaces elsewhere (it should not — it was fully reverted).

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|---------------|
| `pagedjs` | `0.4.3` [VERIFIED: npm registry, `npm view pagedjs version` this session] | CSS Paged Media polyfill — turns flowed HTML+CSS (with `@page`) into discrete page boxes | The de facto standard open-source implementation of the W3C CSS Paged Media / Generated Content for Paged Media specs in the browser; MIT license, no viable alternative with comparable maturity (see Alternatives below). |

### Supporting

No new supporting libraries are required. Everything else (postMessage bridge, CSS scoping,
theme propagation) is hand-written glue code against APIs already used elsewhere in the codebase
(`themeStore` in `ui/src/lib/stores/theme.svelte.ts`, existing `isTauri` branch pattern).

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `pagedjs` (full library, `Previewer` class, programmatic control) | `pagedjs` polyfill auto-run mode (`dist/paged.polyfill.js` + `window.PagedConfig`) | Auto-run polyfill is simpler to drop in but offers less control over *when* pagination starts and makes progress-event wiring (D-07) and error-handling (D-02) awkward, since it self-triggers on `DOMContentLoaded` rather than being invoked explicitly. The plain `dist/paged.js` UMD build (exposes `window.Paged.Previewer`, does not auto-run) is the better fit for this phase's explicit control needs. |
| Client-side pagination via Paged.js | Server-side pre-pagination (e.g., a headless-browser PDF step, or a Rust-side page-break calculator) | Explicitly out of scope — D-01 locks the backend render path as unchanged, and D-04 already rejected a hand-rolled pagination approach for the opposite (client) reason. Mentioned here only as a "why not" note; not a live option per CONTEXT.md. |
| `pagedjs` | `paged-media-polyfill` (Google-affiliated abandoned prototype) / hand-rolled `page-break-inside` detection | Both were considered in prior discussion (D-04) and rejected for correctness reasons (doesn't split long tables; visual-gap approach lies about `page-break-inside: avoid`). Not re-litigated here. |

**Installation:**
```bash
pnpm --dir ui add pagedjs@^0.4.3
```

**Version verification:** `npm view pagedjs version` → `0.4.3` (this session, 2026-08-04).
`npm view pagedjs license` → `MIT`. Package tarball inspected directly (`npm pack pagedjs@0.4.3`)
to confirm bundle contents/size (see Code Examples) — this is stronger than a registry-existence
check alone.

## Architecture Patterns

### System Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│ PdfPreviewModal.svelte (parent — Tauri webview OR LAN browser tab)   │
│                                                                       │
│  htmlContent (act/report HTML string, from backend — unchanged)      │
│         │                                                             │
│         ▼                                                             │
│  build srcdoc = htmlContent                                          │
│               + <style> Trackly sheet chrome (shadow/gap/backdrop)   │
│               + <script> [inline] pagedjs UMD bundle text            │
│               + <script> [inline] bootstrap: new Paged.Previewer()   │
│                 .preview() → postMessage progress/height/error       │
│         │                                                             │
│         ▼                                                             │
│  <iframe sandbox="allow-scripts" srcdoc={srcdoc}>   ◄── opaque origin│
│      │  (no allow-same-origin: parent CANNOT read contentDocument)   │
│      │                                                                │
│      │  Paged.js runs INSIDE this document only:                     │
│      │   1. removeStyles() harvests the doc's own <style> (@page)    │
│      │   2. Polisher strips @page, computes .pagedjs_page boxes,     │
│      │      injects its own base @page{margin:0} (no double-margin) │
│      │   3. Chunker paginates content into N .pagedjs_page divs,     │
│      │      emits "renderedPage" per page (progress), "rendered"     │
│      │      when done (flow.total = page count)                      │
│      │   4. bootstrap script postMessage()s: {type:'progress', n},   │
│      │      {type:'done', total, height}, or {type:'error', ...}     │
│      ▼                                                                │
│  window.addEventListener('message', e => {                           │
│    if (e.source !== iframeEl.contentWindow) return;  // C-01 guard   │
│    // update D-07 spinner text, D-10 page counter, iframe height     │
│  })                                                                   │
│                                                                        │
│  ── Print branches (D-06) — do NOT extract DOM from the iframe ──    │
│                                                                        │
│  isTauri?                                                             │
│   ├─ YES → printViaSystemBrowser(html)                                │
│   │         writes temp .html (tauri-plugin-fs) with pagedjs UMD     │
│   │         bundle + bootstrap inlined (no CSP here — file://) →     │
│   │         opens via tauri-plugin-shell in system browser →         │
│   │         bootstrap calls previewer.preview().then(() =>           │
│   │           window.print())  [C-03: wait for pagination, not load] │
│   │                                                                    │
│   └─ NO  → printViaTopLevel(html)                                     │
│             import('pagedjs') [dynamic ESM, code-split, 'self'-      │
│             hosted — NOT inline, so LAN CSP script-src is fine] →    │
│             new Previewer().preview(bodyMarkup, styleTexts,          │
│               printRootEl)  — re-runs pagination independently in    │
│               the app's own top-level document → window.print()      │
└─────────────────────────────────────────────────────────────────────┘
```

### Recommended Project Structure

No new top-level directories needed. Suggested additions inside `ui/src/features/acts/`:

```
ui/src/features/acts/
├── PdfPreviewModal.svelte     # main file of the phase — iframe build, postMessage handler,
│                               # both print branches rewired
├── pagedPreviewBootstrap.ts   # NEW — small, hand-written string template for the inline
│                               # bootstrap <script> content (kept out of the .svelte file so
│                               # its exact text is stable/diffable for the CSP hash)
└── pagedPreviewBridge.ts      # NEW — parent-side postMessage listener + validation
                                # (event.source check), typed message shapes
```

### Pattern 1: Building the inline-script srcdoc (on-screen preview)

**What:** Inline the UMD Paged.js bundle as raw text (via Vite's `?raw` import, so it's embedded
in the app's own JS bundle at build time — no network fetch, satisfies Phase 16 D-11's
self-contained/no-CDN constraint) followed by a small first-party bootstrap script.

**When to use:** Every time the preview iframe's `srcdoc` is (re)built.

**Example (illustrative; exact stylesheet-argument shape for `preview()` should be spike-verified — see Open Questions):**
```typescript
// Source: pagedjs 0.4.3 dist/paged.js inspected directly this session (npm pack pagedjs@0.4.3)
import pagedjsBundleText from 'pagedjs/dist/paged.min.js?raw'; // Vite raw-text import

function buildBootstrapScript(): string {
  // Kept as a single static string (no per-document interpolation) so its
  // SHA-256 hash is a build-time constant usable in the CSP script-src list.
  return `
    (function () {
      var p = new window.Paged.Previewer();
      var pages = 0;
      p.chunker.on('renderedPage', function () { pages++; parent.postMessage({ type: 'trackly-pagedjs-progress', pages: pages }, '*'); });
      p.preview().then(function (flow) {
        var height = document.querySelector('.pagedjs_pages').scrollHeight;
        parent.postMessage({ type: 'trackly-pagedjs-done', total: flow.total, height: height }, '*');
      }).catch(function (err) {
        parent.postMessage({ type: 'trackly-pagedjs-error', message: String(err) }, '*');
      });
    })();
  `;
}

function buildSrcdoc(actHtml: string, sheetChromeCss: string): string {
  const bootstrap = `<script>${pagedjsBundleText}</` + `script><script>${buildBootstrapScript()}</` + `script>`;
  const chrome = `<style>${sheetChromeCss}</style>`; // D-09 shadow/gap, D-08 backdrop color
  return /<\/body>/i.test(actHtml)
    ? actHtml.replace(/<\/body>/i, `${chrome}${bootstrap}</body>`)
    : `${actHtml}${chrome}${bootstrap}`;
}
```

### Pattern 2: `postMessage` bridge validation (C-01)

**What:** Since `sandbox="allow-scripts"` without `allow-same-origin` gives the iframe an opaque
origin, `event.origin` on messages it sends is the literal string `"null"` — not usable for
authentication. The one thing that *does* reliably identify the sender is object identity of
`event.source` against the iframe's own `contentWindow`.

```typescript
// Source: MDN Window.postMessage + web.dev "Play safely in sandboxed IFrames"
// (https://developer.mozilla.org/en-US/docs/Web/API/Window/postMessage,
//  https://web.dev/articles/sandboxed-iframes) — WebSearch-verified this session, MEDIUM confidence.
function attachBridge(iframeEl: HTMLIFrameElement, onMsg: (data: unknown) => void) {
  function handler(e: MessageEvent) {
    if (e.source !== iframeEl.contentWindow) return; // NOT e.origin — always "null" here
    onMsg(e.data);
  }
  window.addEventListener('message', handler);
  return () => window.removeEventListener('message', handler);
}
```
Sending FROM the parent INTO the opaque-origin iframe (e.g. a live theme-color update) requires
`targetOrigin: '*'` — the iframe has no addressable origin to target more narrowly. Since the
message payload is non-secret (a CSS color string / print trigger) this is an acceptable
trade-off, not a data leak.

### Pattern 3: Fit-to-width `transform: scale()` without breaking scroll (D-11)

**What:** `transform` does not affect an element's layout box — the element keeps its
pre-transform size for scrollHeight/layout purposes, only its *paint* is scaled. To fit-to-width
scale the (already correctly-sized) paginated content without leaving dead scroll space or
clipping, scale an **inner** wrapper and size an **outer** wrapper to the *post-scale* dimensions
explicitly.

```svelte
<!-- Source: standard CSS transform behavior (MDN "transform" — does not affect layout).
     scaleFactor = min(1, frameWidth / naturalPageWidthPx); ceiling of 1 per D-11. -->
<div class="pdf-page-frame" style="height: {naturalHeightPx * scaleFactor}px">
  <div class="pdf-scale-wrap" style="width: {naturalWidthPx}px; height: {naturalHeightPx}px;
      transform: scale({scaleFactor}); transform-origin: top center;">
    <iframe ... />
  </div>
</div>
```
`naturalHeightPx` is not knowable from the parent for an opaque-origin iframe — it must come from
the `postMessage`-reported height (C-01/Pattern 2). Disable the scale entirely
(`scaleFactor = 1`, or drop the wrapper) for the print paths — D-11 states the transform is
screen-only.

### Anti-Patterns to Avoid

- **Trying to read `iframeEl.contentDocument` after sandboxing without `allow-same-origin`:**
  Returns `null` / throws. There is no way around this short of `postMessage`. Do not attempt to
  "extract the paginated DOM" from the iframe for `printViaTopLevel` — re-run Paged.js
  independently there instead (Pattern in Architecture Diagram above).
- **Re-declaring `@page {...}` in the Trackly sheet-chrome `<style>` (D-09):** Paged.js's own
  base stylesheet already neutralizes the browser's native page box (`@page { margin: 0 }`,
  confirmed by direct inspection of `dist/paged.js`, line ~26789 and ~28889). Adding a competing
  `@page` rule in Trackly's own injected CSS risks reintroducing the double-margin problem this
  architecture otherwise avoids for free. Only target `.pagedjs_page` / `.pagedjs_pages` / `body`
  with normal selectors for D-09's shadow/gap/backdrop styling.
- **Reassigning `iframe.srcdoc` on every theme toggle to push a new backdrop color:** this forces
  a full document reload of the iframe (new opaque origin, full re-pagination from scratch) —
  expensive and visibly disruptive if the modal is open while the user flips the theme. Prefer a
  live `postMessage` from parent → iframe telling the bootstrap script to update
  `document.body.style.background` in place (Claude's Discretion per CONTEXT.md D-08 — both
  options are legitimate, but the live-update path is clearly cheaper).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|--------------|-----|
| Splitting long HTML/tables across A4 page boundaries respecting `page-break-inside: avoid` | A custom top-level-block splitter or fixed-pixel visual-gap marker | `pagedjs` | Already explicitly settled by D-04 in CONTEXT.md — both hand-rolled alternatives were tried/considered and rejected (doesn't split long report tables correctly; visual gaps lie about break-avoid). Not re-litigated here, just reaffirmed as still the right call after deeper research. |
| SHA-256 hashing of a static script string for CSP | Ad-hoc runtime hashing / trust-on-first-use | A one-line build-time script (Node `crypto.createHash('sha256')` or `openssl dgst -sha256 -binary \| openssl base64`) run once per dependency/bootstrap-code change, with its output hardcoded into the Rust CSP constant + a CI drift check | This is a well-understood, standard CSP technique (hash-source allow-listing) — no library needed, but it does need a deliberate build-time step and a regression guard so a future `pagedjs` version bump or bootstrap-script edit doesn't silently break LAN-mode pagination via CSP block. |

**Key insight:** the phase does not need a new pagination library beyond Paged.js itself — the
"don't hand-roll" risk here is entirely on the **CSP/security glue** side, not the layout side.

## Common Pitfalls

### Pitfall 1: Inline Paged.js `<script>` silently blocked by CSP in LAN-browser mode

**What goes wrong:** The preview iframe renders with an unpaginated or blank page, or (if D-02's
degradation logic isn't wired to detect this specific failure mode) simply hangs on the D-07
loading spinner forever, because the injected `<script>` never executes at all.

**Why it happens:** `srcdoc` iframes inherit the *creator document's* CSP regardless of the
`sandbox` attribute (CSP3 spec behavior, confirmed via multiple sources this session — see
Sources). Trackly's axum-served SPA sets `script-src 'self'` with no `'unsafe-inline'`
(`crates/trackly-app/src/http/mod.rs:189-208`). This CSP is enforced *inside* the sandboxed iframe
too, blocking the inline bootstrap/library `<script>` tags.

**How to avoid:** Add a CSP hash source (`'sha256-<digest of exact script text>'`) to the axum
`script-src` directive for the specific, static bootstrap-script text (and, if inlined separately,
the library bundle text). Compute the hash at build/CI time from the actual shipped bytes so it
cannot silently drift. This is a Rust-side change (`http/mod.rs`) despite the phase being
described as "100% frontend" — see Open Questions.

**Warning signs:** Works perfectly in the Tauri desktop app (CSP is `null` there — no blocking)
but breaks specifically when accessed from a LAN browser. Since a CSP-blocked `<script>` never
executes at all, **no `postMessage` of any kind arrives at the parent** — there is no JS error
event the parent can catch (a `securitypolicyviolation` event fires only inside the *iframe's own*
document, is not JS-observable there either if the very script registering a listener for it was
the one blocked, and does not propagate to the parent window). The only reliable parent-side
detection is a **timeout**: if no progress/done/error message arrives within N seconds of setting
`srcdoc`, treat it as a pagination failure and fall back per D-02.

### Pitfall 2: Assuming `event.origin` can validate the preview iframe's messages

**What goes wrong:** Code that checks `event.origin === window.location.origin` (or any origin
string) as a security gate will always fail (or, worse, if written permissively, always pass for
*any* opaque-origin sender) once `allow-same-origin` is dropped per D-05.

**Why it happens:** Opaque origins serialize to the literal string `"null"` in `postMessage`
events — `event.origin` carries no distinguishing information.

**How to avoid:** Validate `event.source === iframeEl.contentWindow` instead (object identity,
not string comparison) — see Pattern 2 above.

### Pitfall 3: Double `@page` margin on print

**What goes wrong:** Printed output has the content indented by roughly double the intended
20mm/15mm margin, or misaligned relative to the on-screen preview.

**Why it happens:** In a naive integration, the *original* `@page { margin: 20mm 15mm }` from the
act/report template stays active on the physical print surface *in addition to* Paged.js's
internal per-page margin-box simulation, which already accounts for that spacing via
`.pagedjs_area`/`.pagedjs_margin-*` positioning.

**How to avoid:** Verified this is *not* actually a risk with a correct integration: Paged.js's
own bundled base stylesheet neutralizes this automatically — it strips the source `@page` rule
during `polisher.add()` and injects its own `@page { margin: 0 }` (confirmed directly in
`dist/paged.js`, two occurrences). The pitfall only reappears if Trackly's own D-09 sheet-chrome
CSS re-declares a competing `@page` rule (see Anti-Patterns above) — don't do that.

**Warning signs:** Compare a printed/PDF-saved page against the on-screen preview at identical
zoom; content should start at the same physical offset from the paper edge in both.

### Pitfall 4: Re-running Paged.js twice (iframe preview + top-level print) could theoretically diverge

**What goes wrong:** If the two invocations somehow produce different page-break points, PRV-03's
WYSIWYG guarantee breaks even though "the same engine" is technically used in both places.

**Why it happens:** Paged.js's `@page { size: A4 }` is defined in absolute physical units (mm),
so page-box dimensions should be independent of the surrounding container's viewport width — but
this has not been empirically verified against Trackly's actual three templates in this research
session (no headless-browser rendering was performed).

**How to avoid:** Treat "re-run independently in each context" as the primary approach (simpler,
avoids the impossible DOM-extraction-from-opaque-iframe problem), but budget a manual UAT step
that visually diffs the on-screen page-break locations against the printed/PDF output for all
three document types — this folds naturally into D-13's already-planned manual verification pass,
it just needs an explicit checklist item added.

### Pitfall 5: `srcdoc` reassignment destroys Paged.js's in-progress/completed pagination state

**What goes wrong:** Any Svelte reactivity that recomputes the `srcdoc` string (e.g., a naive
implementation of the D-08 theme-color propagation) causes the iframe to fully reload — new
opaque origin, full re-run of pagination — even though only a CSS custom property changed.

**How to avoid:** See Pattern/Anti-Pattern notes above — prefer a live `postMessage` update over
`srcdoc` reassignment for anything that changes after initial pagination completes.

## Code Examples

### Reading the `@page` block for the D-13 structural test

The three templates already carry byte-identical `@page` blocks (verified by direct read this
session):

```html
<!-- crates/trackly-app/templates/act_handover.html, lines 36-39 -->
<!-- crates/trackly-app/templates/act_acceptance.html, lines 29-32 -->
<!-- crates/trackly-app/templates/report.html, lines 34-37 -->
@page {
  size: A4 portrait;
  margin: 20mm 15mm;
}
```

Two viable homes for the D-13 regression test — the planner should pick one (this is genuinely
open, see Open Questions):

**Option A — Rust integration test** (`crates/trackly-app/tests/`), consistent with the existing
`html_act_render.rs` / `html_report_render.rs` convention of reading these exact template files:
```rust
// Illustrative — no existing precedent test reads RAW template text (all existing
// html_*_render.rs tests go through the full render pipeline with substituted variables).
// This would be a new, narrower test reading the files directly, e.g. via the same
// TRACKLY_TEMPLATES_DIR-aware path resolution already in pdf/html_templates.rs.
#[test]
fn all_three_templates_share_identical_page_block() {
    let page_re = regex::Regex::new(r"(?s)@page\s*\{[^}]*\}").unwrap();
    let extract = |path: &str| -> String {
        let text = std::fs::read_to_string(path).unwrap();
        page_re.find(&text).unwrap().as_str().to_string()
    };
    let a = extract("templates/act_handover.html");
    let b = extract("templates/act_acceptance.html");
    let c = extract("templates/report.html");
    assert_eq!(a, b);
    assert_eq!(b, c);
}
```

**Option B — Node script** (`ui/scripts/check-page-margins.mjs`), matching the zero-dependency
convention of `check-tokens.mjs`/`check-contrast.mjs`/`check-focus-outline.mjs` (all wired into
`pnpm lint`):
```javascript
// Source: pattern mirrored from ui/scripts/check-tokens.mjs (Phase 23) — zero-dependency,
// node:fs/node:path only, exits non-zero on violation.
```
Recommendation: **Option A** fits better because the artifact under test (raw `.html` template
files) is Rust-crate-owned, not part of the `ui/` build graph — a Node script would need a
relative path reaching outside `ui/` into `crates/trackly-app/templates/`, which is unusual for
this codebase's existing script conventions (all current `ui/scripts/*.mjs` only touch `ui/src`).
Flagging as discretion, not a hard requirement.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|---------------|--------|
| Flat single-page `<iframe sandbox="">` preview, one fixed 794×1123px frame, browser handles print pagination invisibly (Phase 16, D-09/D-12) | Paged.js-driven multi-page preview in the same iframe with `sandbox="allow-scripts"`, print driven by the same engine | This phase (33) | Screen preview now shows real page breaks (PRV-01/02); print/preview divergence (only guaranteed by trusting the browser's own print pagination previously) is closed by using one engine for both (PRV-03). |

**Deprecated/outdated:** none — this is additive polish on top of the still-current Phase 16
HTML-print architecture (`srcdoc`, dual `isTauri` branches, self-contained documents). Nothing
from Phase 16 is being removed.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `pagedjs` is the canonical/only viable open-source CSS Paged Media polyfill for this use case (no better-fit alternative exists) | Standard Stack, Alternatives Considered | Low — this was already locked by D-04 in CONTEXT.md before this research; re-verified, not newly asserted. |
| A2 | Re-running Paged.js independently in the iframe (preview) and in `#act-print-root` (LAN print) will produce byte-identical page-break points, because `@page` uses absolute mm units | Summary, Pitfall 4 | Medium — if wrong, PRV-03's WYSIWYG guarantee is violated for `printViaTopLevel` specifically. No headless-browser test was run this session to confirm; needs empirical verification (a real render, per the project's own `act_pdf_word_fidelity` lesson that text-extraction tests can't see this class of bug). |
| A3 | `polisher.add(...stylesheets)` accepts an array of plain CSS-text strings (not only the auto-extracted `{href: text}` object form `removeStyles()` produces internally) | Open Questions | Medium — if wrong, the explicit `printViaTopLevel` invocation (Pattern in Architecture Diagram) needs a different stylesheet-argument shape. Source was inspected but the exact accepted input types for `Polisher.add()` were not fully traced in this session — flagged as a spike item, not asserted as fact. |
| A4 | A CSP `sha256-` hash source is the right fix for the inline-script-blocked-in-LAN-mode problem (vs. e.g. a per-request nonce) | Summary, Common Pitfalls #1, Don't Hand-Roll | Low-Medium — hash-source CSP is a standard, spec-legal mechanism (CSP3), but requires the axum `security_headers` layer to move from a fully static header to one that embeds a build-time-computed constant, plus a drift-detection step. A nonce-based approach was considered less suitable because the axum layer currently sets one static header for *all* responses (`SetResponseHeaderLayer::overriding`), and a meaningfully unpredictable per-request nonce would require restructuring that layer to be per-response, which is more invasive than a compile-time hash constant. |

## Open Questions

1. **Does implementing D-05/D-06 require a backend (Rust) file change, contradicting the phase's "100% frontend" framing?**
   - What we know: The axum CSP (`crates/trackly-app/src/http/mod.rs`) currently sets
     `script-src 'self'` with no `'unsafe-inline'` and no hash/nonce sources. `srcdoc` iframes
     inherit this CSP regardless of `sandbox`.
   - What's unclear: Whether the user/planner considers "add one hash-source token to an existing
     CSP header string, plus a drift-detection check" as within the spirit of "чисто
     фронтендовая" (D-01's framing is specifically about the HTML *templates* and the
     act/report *render path*, not about every Rust file in the repo) — or whether this needs to
     be explicitly called out as a small, deliberate exception.
   - Recommendation: Surface this to the user/planner explicitly before planning tasks. If
     confirmed acceptable, the fix is small (one constant + one CI check) and does not touch
     `act_service`/`html_templates.rs`/the render path at all — consistent with D-01's actual
     boundary, just not with the phrase "100% frontend."

2. **Exact `Polisher.add()` stylesheet-argument shape for the explicit (non-auto) `printViaTopLevel` invocation.**
   - What we know: `removeStyles()` (the auto-extraction path used by the parameterless
     `.preview()` call) produces `{[url]: cssText}` objects for inline `<style>` tags and plain
     href strings for `<link>` tags, then spreads them into `polisher.add(...stylesheets)`.
   - What's unclear: Whether passing a plain array of raw CSS-text strings (bypassing the
     `{href: text}` wrapping) works identically, since `printViaTopLevel` needs to hand Paged.js
     explicit `content`/`stylesheets`/`renderTo` arguments (it's rendering into `#act-print-root`,
     not consuming the whole document body the way the iframe's parameterless call does).
   - Recommendation: A short Wave-0 spike task — render one template through both invocation
     styles and diff output — before committing to the exact `printViaTopLevel` implementation.

3. **Does Paged.js's `loadFonts()` step (part of `Chunker.flow()`) introduce any timing surprise for the D-07 progress spinner or D-06's C-03 "wait for pagination before print"?**
   - What we know: The templates only declare system font stacks (`"DejaVu Sans", "Arial",
     sans-serif`) — no `@font-face`/network font loading — so this should resolve near-instantly.
   - What's unclear: Whether `document.fonts.ready` (or whatever `loadFonts()` awaits internally)
     behaves identically inside a sandboxed opaque-origin iframe vs. a normal document — not
     traced to the exact implementation this session.
   - Recommendation: Low risk given no custom fonts are in play; verify incidentally during Wave-0
     spike/manual UAT rather than as a dedicated investigation.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|--------------|-----------|---------|----------|
| Node.js / pnpm (frontend toolchain) | Adding `pagedjs` to `ui/package.json`, Vite `?raw` import | ✓ (existing project toolchain, unchanged by this phase) | pnpm `10.17.1` pinned (per `package.json` `packageManager`), Vite `^6.0.0` | — |
| A real LAN-browser test target (Chrome/Edge/Firefox against the axum server) | Verifying the CSP hash-source fix actually unblocks the inline script (Pitfall 1) | Not verified in this research session (no browser was launched) | — | Must be manually verified during implementation/UAT — this cannot be confirmed by static analysis alone. |
| A real Tauri desktop webview (WKWebView on macOS dev machine) | Verifying `sandbox="allow-scripts"` behaves as expected inside Tauri's webview (historically surprising — see Phase 16's GAP-16-01 WKWebView print quirks) | Available (macOS dev machine per project constraints) but not launched in this research session | — | Manual UAT required; do not assume WKWebView parity with Chrome for iframe/CSP edge cases given the documented Phase 16 precedent of WKWebView-specific print bugs. |

**Missing dependencies with no fallback:** none — all required libraries are static-analyzable;
what's missing is *runtime browser verification*, which is inherently a manual/UAT step, not a
tooling gap.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework (frontend) | **None.** `ui/` has no Vitest/Jest/Playwright — verified via `ui/package.json` (no test-related devDependency) and a repo-wide search for `*.test.*`/`*vitest*` under `ui/` (zero hits). Frontend "tests" today are: `svelte-check` (types), `eslint`+`prettier` (lint/format), and four zero-dependency Node scripts (`check-tokens.mjs`, `check-contrast.mjs`, `check-focus-outline.mjs`, and by Phase 23/30 precedent) wired into `pnpm lint`. |
| Framework (backend) | `cargo test` (workspace), existing precedent for HTML-template-adjacent tests in `crates/trackly-app/tests/html_act_render.rs`, `html_report_render.rs`, `template_edit.rs`. |
| Config file | none (frontend) / standard `Cargo.toml` workspace (backend) |
| Quick run command | `pnpm --dir ui lint` (frontend gates) / `cargo test -p trackly-app --test <new_test_file>` (backend, if Option A chosen for D-13) |
| Full suite command | `pnpm --dir ui lint && pnpm --dir ui svelte-check` / `cargo test --workspace` (see project memory: **do not** run `cargo test --workspace` blindly — it hangs on a pre-existing unrelated `auth_remember_cookie` test; use targeted per-crate/per-test invocation instead, per recorded project lesson) |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|---------------------|--------------|
| PRV-01 | Preview shows A4 sheet(s) on a grey backdrop | manual-only | — | N/A — this is a visual/rendering outcome; per the project's own recorded lesson (`act_pdf_word_fidelity`), text-extraction-style tests cannot see overlap/overflow/visual layout — a real render + human look is required. |
| PRV-02 | Sheet has visible margins matching print margins | manual-only + D-13's structural test | `cargo test` (new, Option A) or `node scripts/check-page-margins.mjs` (Option B) | ❌ Wave 0 — new file needed either way |
| PRV-03 | WYSIWYG — preview matches print output | manual-only | — | N/A — inherently requires a human comparing a real printed/PDF page against the on-screen preview; this was the explicit reasoning behind D-13's manual UAT requirement, reaffirmed by Pitfall 4 above. |

### Sampling Rate

- **Per task commit:** `pnpm --dir ui lint` (fast, catches lint/format/token/contrast regressions
  — will NOT catch anything Paged.js/CSP-specific, since there is no browser-level frontend test
  harness).
- **Per wave merge:** full `pnpm --dir ui lint && pnpm --dir ui svelte-check`, plus the new D-13
  structural test if it lands in `cargo test` (targeted, not full workspace — see hang caveat
  above).
- **Phase gate:** Manual UAT is **required, not optional**, for PRV-01/02/03 given the total
  absence of a frontend rendering test harness in this project — this should be called out
  explicitly in the plan's verification section, not left implicit.

### Wave 0 Gaps

- [ ] `crates/trackly-app/tests/<new>.rs` (Option A) or `ui/scripts/check-page-margins.mjs`
      (Option B) — D-13's structural `@page`-parity guard. Neither exists yet.
- [ ] A short Wave-0 spike verifying the `Polisher.add()` stylesheet-argument shape (Open
      Question 2) before committing to the `printViaTopLevel` implementation.
- [ ] No frontend test framework exists to add proper automated coverage of the iframe/CSP/
      postMessage machinery — this is a structural gap in the project, not something this phase
      is expected to fix, but the plan should not overstate what automated tests will catch here.

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|----------------|---------|---------------------|
| V5 Input Validation | Marginal | The act/report HTML "template" is user-editable (per Phase 16 D-05/D-03) — already treated as semi-trusted, which is *why* D-05 chose `sandbox="allow-scripts"` without `allow-same-origin` in the first place. This phase does not change that trust boundary, only adds a script execution capability *within* it. |
| V14 Configuration (CSP) | Yes | The relevant control here is the axum `Content-Security-Policy` header (`crates/trackly-app/src/http/mod.rs`). This phase's core security-relevant decision is **whether/how to loosen `script-src`** for exactly one static, first-party script — via a hash source, not `'unsafe-inline'`. This preserves CSP's XSS-mitigation value (an attacker-injected inline script would have different, non-matching text, so its hash would not match the allow-listed value). |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|------------------------|
| A malicious/compromised act or report template (user-edited HTML per Phase 16 D-03) injects a `<script>` attempting to exfiltrate data or pivot into the app | Tampering / Elevation of Privilege | `sandbox="allow-scripts"` **without** `allow-same-origin` (D-05, unchanged by this phase) — the iframe's script can run but has an opaque origin, no `document.cookie`/`localStorage`/parent-window access, and (per this research's CSP finding) is *also* still bound by the inherited CSP in LAN mode. Any injected script beyond the hash-allow-listed first-party bootstrap would be CSP-blocked in LAN mode; in Tauri desktop mode (CSP `null`) the sandbox opacity is the only defense, which is why D-05 explicitly rejected `allow-same-origin`. |
| CSP hash allow-list becomes stale after a `pagedjs` version bump or a hand-edit to the bootstrap script, silently reopening the "add `'unsafe-inline'` instead" temptation as a quick fix | Tampering (weakened defense-in-depth) | Add a CI/build-time check that recomputes the hash from the shipped bytes and fails if it doesn't match the hardcoded constant in `http/mod.rs` — do not resolve a mismatch by loosening to `'unsafe-inline'`. |

## Sources

### Primary (HIGH confidence — direct inspection this session)
- `pagedjs` npm package tarball (`npm pack pagedjs@0.4.3`), extracted and read directly:
  `package/dist/paged.js` (UMD build, exposes `window.Paged.Previewer`, does not auto-run),
  `package/package.json` (version, license, exports map, `type: module`).
  Confirmed: `Previewer.preview(content, stylesheets, renderTo)` signature; event names
  `"page"`, `"rendering"`, `"renderedPage"`, `"rendered"`, `"size"`, `"atpages"` (grepped emit
  sites); `Previewer.chunker` is a public property exposing the finer-grained `"renderedPage"`
  event not re-emitted by `Previewer` itself; `ContentParser` accepts either a DOM node or a raw
  HTML string; base stylesheet includes `@page { size: letter; margin: 0; }` (two occurrences,
  confirming no double-margin risk when Paged.js's own polishing runs).
- `npm view pagedjs version license` — `0.4.3`, `MIT`.
- `slopcheck install pagedjs --ecosystem npm` — `[OK]`.
- `crates/trackly-app/src/http/mod.rs` (lines 179-218) — actual axum CSP header text, confirmed
  `script-src 'self'` with no `'unsafe-inline'`, and the WR-07/GAP-16-01 comments explaining why.
- `crates/trackly-app/tauri.conf.json` — confirmed `"security": {"csp": null}` for the Tauri app.
- `ui/src/lib/stores/theme.svelte.ts` — confirmed theme resolution mechanism (`themeStore.resolved`,
  `document.documentElement.dataset.theme`) for D-08 propagation planning.
- `ui/package.json`, repo-wide search for `*.test.*`/`vitest` under `ui/` — confirmed no frontend
  test framework exists.
- `crates/trackly-app/templates/act_handover.html`, `act_acceptance.html`, `report.html` — read
  directly, confirmed byte-identical `@page { size: A4 portrait; margin: 20mm 15mm; }` blocks.
- `ui/src/features/acts/PdfPreviewModal.svelte`, `ui/src/lib/components/Modal.svelte` — read in
  full, current implementation of both print branches and the modal sizing/focus-trap constraints.

### Secondary (MEDIUM confidence — WebSearch, cross-checked against spec/multiple sources)
- CSP3 spec + [w3c/webappsec-csp#700](https://github.com/w3c/webappsec-csp/issues/700) — `srcdoc`
  iframes inherit the creator document's CSP regardless of `sandbox`; this remains an open
  discussion in the CSP working group (no browser currently offers an opt-out), confirming this
  isn't a Trackly-specific quirk.
- [MDN: HTMLIFrameElement.srcdoc](https://developer.mozilla.org/en-US/docs/Web/API/HTMLIFrameElement/srcdoc),
  [MDN: Window.postMessage()](https://developer.mozilla.org/en-US/docs/Web/API/Window/postMessage) —
  CSP inheritance and postMessage/opaque-origin behavior.
- [web.dev: Play safely in sandboxed IFrames](https://web.dev/articles/sandboxed-iframes) —
  opaque-origin `event.origin` = `"null"`, `event.source`-based validation, `targetOrigin: '*'`
  requirement when posting *into* a sandboxed frame.
- [Paged.js documentation — How Paged.js works](https://pagedjs.org/en/documentation/4-how-paged.js-works/),
  [Getting Started](https://pagedjs.org/en/documentation/2-getting-started-with-paged.js/),
  [Handlers, Hooks and custom javascript](https://pagedjs.org/en/documentation/10-handlers-hooks-and-custom-javascript/) —
  general architecture/hooks confirmation, cross-checked against direct source inspection above
  (source inspection is treated as the higher-confidence source where the two could be compared).
- [pagedjs GitHub repository](https://github.com/pagedjs/pagedjs) — package identity, install
  instructions, `Previewer` usage example matching what was found in source.

### Tertiary (LOW confidence — single source or not independently verified)
- Exact weekly npm download count for `pagedjs` — not queried this session (no `npm view pagedjs
  downloads` or npm-stat lookup performed); "high" usage claim in the Package Legitimacy Audit is
  qualitative (known adoption by Coko Foundation / Editoria / general publishing-tooling
  ecosystem), not a measured number.
- `Polisher.add()`'s accepted stylesheet-argument shape for explicit invocation (Open Question 2)
  — traced partially via source inspection but not conclusively; flagged as needing a Wave-0 spike
  rather than asserted as verified fact.

## Metadata

**Confidence breakdown:**
- Standard stack (Paged.js as the library, its API surface): HIGH — package inspected directly,
  version/license registry-confirmed, slopcheck-clean.
- Architecture (iframe/CSP/postMessage interaction): MEDIUM — the CSP-inheritance claim rests on
  spec text and multiple independent secondary sources (converging), not on an actual browser
  test run against Trackly's own axum server in this session. This is the phase's central risk
  and should be treated as "very likely true, verify empirically in Wave 0" rather than settled
  fact.
- Pitfalls: MEDIUM-HIGH — pitfalls 1-3 and 5 rest on verified mechanics (source inspection +
  spec); pitfall 4 (re-run determinism) is explicitly flagged as needing empirical UAT
  confirmation, not asserted as certain.

**Research date:** 2026-08-04
**Valid until:** ~30 days (stable ecosystem — Paged.js releases infrequently; CSP/browser
sandboxing semantics are long-stable web platform behavior, not fast-moving). Re-verify sooner if
`pagedjs` receives a version bump before implementation begins, since the CSP hash source is tied
to exact bundled bytes.

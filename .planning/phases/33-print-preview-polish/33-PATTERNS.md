# Phase 33: Полировка предпросмотра печати - Pattern Map

**Mapped:** 2026-08-04
**Files analyzed:** 6 (1 modify-heavy frontend, 2 new frontend helpers, 1 modify backend, 1 new backend test, 1 dependency manifest)
**Analogs found:** 6 / 6

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|--------------------|------|-----------|-----------------|----------------|
| `ui/src/features/acts/PdfPreviewModal.svelte` | component (modal) | request-response + event-driven (postMessage) | itself (pre-Phase-33 version, same file) | exact — this is a rework, not a greenfield component |
| `ui/src/features/acts/pagedPreviewBootstrap.ts` (NEW) | utility (string builder) | transform | `ui/src/lib/utils/dropdownAnchor.ts` / `ui/src/lib/utils/date.ts` (module shape, JSDoc style) + `PdfPreviewModal.svelte`'s existing inline-script-string builder (`printViaSystemBrowser`'s `autoPrint` string, lines 181-199) | role-match (module conventions) / exact (string-concatenation pattern to avoid closing `</script>`) |
| `ui/src/features/acts/pagedPreviewBridge.ts` (NEW) | utility (event bridge) | event-driven (postMessage) | none pre-existing in codebase — no prior `postMessage` bridge exists; pattern comes from RESEARCH.md Pattern 2, follow `ui/src/lib/utils/` module conventions for shape/comments | no analog — see "No Analog Found" |
| `crates/trackly-app/src/http/mod.rs` (`build_router`, security_headers layer) | middleware (config) | request-response | itself — modifying the existing `SetResponseHeaderLayer::overriding(... content-security-policy ...)` block (lines ~179-208) | exact — same function, extend existing static header string |
| `crates/trackly-app/tests/html_page_parity.rs` (NEW, suggested name) | test (structural regression) | batch (read 3 files, compare) | `crates/trackly-app/tests/pdf_determinism.rs` (SHA-256 fixture-compare pattern — directly reusable for the CSP-hash drift check) + `crates/trackly-app/src/pdf/html_templates.rs` (`resolve_templates_dir`/`TRACKLY_TEMPLATES_DIR` path convention, if the test needs to resolve template paths robustly) + `crates/trackly-app/tests/security_headers.rs` (CSP-string assertion style, if the CSP hash-source itself also gets an integration-test assertion) | exact (hash-fixture pattern) / role-match (path resolution) |
| `ui/package.json` | config (deps manifest) | — | itself — existing `dependencies` block already lists `pdfjs-dist`, a comparable "heavy client-side rendering library shipped for offline/self-contained use" | exact |

## Pattern Assignments

### `ui/src/features/acts/PdfPreviewModal.svelte` (component, request-response + event-driven)

**Analog:** itself, current implementation (full file read, 361 lines — no re-read needed).

**Imports pattern** (lines 38-43):
```svelte
import Button from '$lib/components/Button.svelte';
import Modal from '$lib/components/Modal.svelte';
import Spinner from '$lib/components/Spinner.svelte';
import { pushToast } from '$lib/stores/toast.svelte';
import { acts } from '$lib/api/acts';
import { apiCall } from '$lib/api/client';
```
Add: `import { isTauri } from '$lib/stores/transport.svelte';` (see Shared Patterns — replaces the current file-local `const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;` at line 45 with the shared store import; the shared version already exists and is used by `DeviceImportCsvModal.svelte`).

**isTauri desktop/LAN branch (existing, to be preserved verbatim as the top-level structure)** (lines 260-271):
```svelte
async function handlePrint() {
  if (!ready || htmlContent === null) return;
  try {
    if (isTauri) {
      await printViaSystemBrowser(htmlContent);
    } else {
      printViaTopLevel(htmlContent);
    }
  } catch {
    pushToast('error', 'Не удалось открыть документ для печати');
  }
}
```
D-06 requires both branches internally reworked to route through Paged.js, but this outer `if (isTauri) / else` shape is the established, correct pattern — do not restructure it.

**Desktop print branch — current structure to extend with pagination-wait (C-03)** (lines 181-199):
```svelte
async function printViaSystemBrowser(html: string) {
  // Build the tag via concatenation so the literal '</scr'+'ipt>' does not
  // prematurely close this component's own <script> block at compile time.
  const autoPrint =
    '<' +
    'script>window.addEventListener("load",function(){setTimeout(function(){window.print()},300)})<' +
    '/script>';
  const htmlWithAutoPrint = /<\/body>/i.test(html)
    ? html.replace(/<\/body>/i, `${autoPrint}</body>`)
    : `${html}${autoPrint}`;

  const { writeTextFile, BaseDirectory } = await import('@tauri-apps/plugin-fs');
  const { open: openPath } = await import('@tauri-apps/plugin-shell');
  const fileName = `trackly-print-${Date.now()}.html`;
  await writeTextFile(fileName, htmlWithAutoPrint, { baseDir: BaseDirectory.Temp });
  const { tempDir, join } = await import('@tauri-apps/api/path');
  const filePath = await join(await tempDir(), fileName);
  await openPath(filePath);
}
```
Per C-03 and D-06: the `window.addEventListener("load", ...)` auto-print trigger must become "wait for Paged.js's `preview().then()` to resolve, then `window.print()`" instead of firing on `load`. The `'<' + 'script>...</' + '/script>'` string-concatenation trick (to avoid the literal `</script>` closing this `.svelte` file's own `<script>` block at compile time) is the exact technique `pagedPreviewBootstrap.ts` must reuse when it builds the inlined bootstrap `<script>` tag for the `srcdoc` string — copy this concatenation idiom verbatim, do not use a template literal with a literal `</script>` substring anywhere in `.svelte` files.

**LAN print branch — current structure, `#act-print-root` injection** (lines 209-258): read in full above; D-06 requires this to invoke Paged.js's `Previewer` against `#act-print-root` (dynamic `import('pagedjs')`, per RESEARCH.md's Architecture Diagram) instead of directly injecting raw body/style HTML. The `printRoot`/`printStyle` element creation, `@media print { body > :not(#...) { display:none } }` visibility-scoping, and `afterprint` cleanup listener are all still correct and should be kept — only the content injected into `printRoot.innerHTML` changes from raw `bodyHtml` to Paged.js's rendered page-box markup.

**Current buggy backdrop/sheet CSS (the exact bug D-08 fixes)** (lines 320-338):
```scss
.pdf-page-frame {
  flex: 1;
  display: flex;
  justify-content: center;
  overflow: auto;
  background: var(--tr-surface);        /* BUG: --tr-surface is #ffffff in light theme — same as sheet */
  border-radius: var(--tr-radius-xs);
  padding: var(--tr-space-md) 0;
}
.pdf-iframe {
  width: 794px;
  min-width: 794px;
  height: 1123px;
  min-height: 1123px;
  border: 1px solid var(--tr-border);
  box-shadow: var(--tr-elev-2);
  background: var(--tr-n-0);
  flex-shrink: 0;
}
```
D-08 fix: change `.pdf-page-frame` background to `var(--tr-surface-sunken)`. D-09 fix: drop the `border: 1px solid var(--tr-border)` from `.pdf-iframe` ("без рамки"), keep `box-shadow: var(--tr-elev-2)`, keep `background: var(--tr-n-0)` (already correct — theme-invariant white). The fixed `794px`/`1123px` sizing becomes the "natural size" basis for D-11's `transform: scale()` fit-to-width wrapper (RESEARCH.md Pattern 3) — an outer frame sized to `naturalHeightPx * scaleFactor` wrapping an inner `794×1123` element that gets `transform: scale(...)`.

**States block (loading/error/empty) — structure to extend, not replace** (lines 276-296):
```svelte
{#if loading}
  <div class="state state-loading">
    <Spinner size="md" />
    <p>Генерируем PDF…</p>
  </div>
{:else if errorMsg !== null}
  <div class="state state-error">
    <p class="error-heading">Не удалось сгенерировать PDF</p>
    <p class="error-detail">{errorMsg}</p>
  </div>
{:else if htmlContent !== null}
  <div class="pdf-page-frame">
    <iframe sandbox="" srcdoc={htmlContent} title="Document Preview" class="pdf-iframe"
    ></iframe>
  </div>
{:else}
  <div class="state state-empty">
    <p>Нет данных для предпросмотра.</p>
  </div>
{/if}
```
Per UI-SPEC: `sandbox=""` → `sandbox="allow-scripts"` (D-05); `srcdoc={htmlContent}` → `srcdoc={builtSrcdoc}` (built via `pagedPreviewBootstrap.ts`); `title="Document Preview"` → `title="Предпросмотр документа"` (RU-only a11y fix); add `aria-live="polite"` to `.state.state-loading` (currently missing — the analog for this exact idiom is `CartridgeDetail.svelte:106`, `<div class="detail-loading" aria-live="polite">`); the D-02 degraded path is a *new conditional branch* inside the `{:else if htmlContent !== null}` case (pagination-timeout → render the exact pre-Phase-33 markup shown above with `sandbox=""`, no chrome), not a new top-level `{#if}` branch.

**Footer snippet — structure to extend with meta block** (lines 298-303):
```svelte
{#snippet footer()}
  <Button variant="secondary" onclick={onClose}>Закрыть</Button>
  <Button variant="primary" onclick={handlePrint} disabled={loading || errorMsg !== null}>
    Печать
  </Button>
{/snippet}
```
Per UI-SPEC: add a new `.pdf-preview-footer-meta` flex item before the two buttons (`flex: 1 1 auto; min-width: 0`), containing the page-counter + hint-line stack, gated by `{#if !loading && errorMsg === null && htmlContent !== null}`. Extend the `disabled` binding on the Печать button to also cover "pagination in progress" per D-07 (not just `loading`/`errorMsg`).

**Error handling pattern (existing, reuse as-is)** (lines 144-150):
```typescript
} catch (e: unknown) {
  if (cancelled) return;
  const msg =
    e && typeof e === 'object' && 'message' in e
      ? String((e as { message: unknown }).message)
      : 'Не удалось сгенерировать PDF';
  errorMsg = msg;
}
```
This existing `renderCall()` error path is untouched by this phase — it only covers backend/network failure (the "Render error" row in UI-SPEC's Error & Empty States table), distinct from the new pagination-timeout degraded path which does NOT set `errorMsg`.

---

### `ui/src/features/acts/pagedPreviewBootstrap.ts` (NEW — utility, transform)

**Analog:** `ui/src/lib/utils/dropdownAnchor.ts` / `ui/src/lib/utils/date.ts` for module shape and Russian-JSDoc comment conventions; `PdfPreviewModal.svelte`'s own `printViaSystemBrowser` `autoPrint` string (lines 184-187) for the `</script>`-avoidance string-concatenation idiom.

**Module shape convention** (from `ui/src/lib/utils/date.ts`, full file):
```typescript
/**
 * Форматирует Unix-timestamp (секунды) в читаемую строку для отображения.
 * Backend хранит UTC; отображение — в локали пользователя.
 */
export function formatUnixSeconds(seconds: number): string {
  return new Date(seconds * 1000).toLocaleString('ru-RU', { ... });
}
```
Top-of-file Russian doc comment stating purpose, plain exported `function`s, no class wrapper. `pagedPreviewBootstrap.ts` should follow this shape: export a `buildSrcdoc(actHtml: string, backdropHex: string): string` (or similar) plus whatever internal helpers it needs, with a module-level comment explaining the CSP-hash-stability constraint (RESEARCH.md: "kept as a single static string... so its SHA-256 hash is a build-time constant").

**`</script>`-avoidance idiom to reuse exactly** (from `PdfPreviewModal.svelte` lines 184-187):
```typescript
const autoPrint =
  '<' +
  'script>window.addEventListener("load",function(){setTimeout(function(){window.print()},300)})<' +
  '/script>';
```
Any function in `pagedPreviewBootstrap.ts` that assembles `<script>...</script>` tag text as a string must use the same `'<' + 'script>'` / `'<' + '/script>'` concatenation split, since a literal `</script>` substring inside a `.ts` (or `.svelte`) source file's own string/template-literal risks being misparsed by tooling — this project has already hit and solved this exact problem once.

**Vite `?raw` import** — no existing analog in this codebase (grepped `ui/src` and `vite.config.*`, zero hits for `?raw`). This will be the first use of Vite's raw-text import suffix in the project. RESEARCH.md's Pattern 1 code example (`import pagedjsBundleText from 'pagedjs/dist/paged.min.js?raw';`) is the reference; no internal precedent exists, cite RESEARCH.md directly for this line.

---

### `ui/src/features/acts/pagedPreviewBridge.ts` (NEW — utility, event-driven)

**No direct analog** — the codebase has zero prior `postMessage`/`MessageEvent` usage (grepped, no hits). Follow `ui/src/lib/utils/` module-shape conventions (Russian JSDoc header, plain exported functions) for the wrapper shape, but the actual listener/validation logic must come from RESEARCH.md Pattern 2 (`event.source === iframeEl.contentWindow` identity check, NOT `event.origin` string comparison — opaque origin serializes to `"null"`).

**Closest structural analog for "attach a DOM listener, return a cleanup function"** — `ui/src/lib/utils/portal.ts`'s and `dropdownAnchor.ts`'s Svelte-action `destroy()` return-cleanup convention, and `PdfPreviewModal.svelte`'s own `afterprint` listener cleanup pattern (lines 250-254):
```typescript
const cleanup = () => {
  printRoot!.innerHTML = '';
  window.removeEventListener('afterprint', cleanup);
};
window.addEventListener('afterprint', cleanup);
```
`pagedPreviewBridge.ts`'s `attachBridge(iframeEl, onMsg)` should mirror this "attach + return teardown closure" shape (see RESEARCH.md Pattern 2's own `attachBridge` sketch, which already follows this exact idiom).

---

### `crates/trackly-app/src/http/mod.rs` (middleware, request-response)

**Analog:** itself — modify the existing `security_headers` `ServiceBuilder` block in `build_router`.

**Current CSP construction (exact text to extend)** (from `http/mod.rs`, `security_headers` block):
```rust
let security_headers = ServiceBuilder::new()
    .layer(SetResponseHeaderLayer::overriding(
        axum::http::header::HeaderName::from_static("x-frame-options"),
        HeaderValue::from_static("DENY"),
    ))
    .layer(SetResponseHeaderLayer::overriding(
        axum::http::header::HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    ))
    .layer(SetResponseHeaderLayer::overriding(
        axum::http::header::HeaderName::from_static("content-security-policy"),
        // WR-07: drop 'unsafe-inline' from script-src ...
        HeaderValue::from_static(
            "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self' wss:; frame-src 'self' blob:; object-src 'self' blob:",
        ),
    ));
```
D-14 requires `script-src 'self'` → `script-src 'self' 'sha256-<digest>'`. Follow the existing comment convention immediately above the `HeaderValue::from_static(...)` call (WR-07 / T-06-12-I / PDF-CSP / GAP-16-01 — each prior addition to this CSP string got a named, dated inline comment explaining *why* that specific source/directive was added). Add a new comment block, e.g. `// PRV-CSP (Phase 33, D-14): sha256 hash-source for the Paged.js bootstrap <script> inlined into PdfPreviewModal's srcdoc — see pagedPreviewBootstrap.ts. Hash MUST be regenerated (and this constant updated) whenever that script's exact text changes; drift is caught by tests/<new csp/hash test>.rs.`

**Hash constant placement:** since the header is built via `HeaderValue::from_static(...)` (a `&'static str` literal, not a `format!`), the hash digest must be a hardcoded literal string in this same file (or a `const` a few lines above it), NOT computed at runtime — matching the "compile-time constant" requirement from RESEARCH.md's Don't-Hand-Roll table and Assumption A4.

---

### `crates/trackly-app/tests/html_page_parity.rs` (NEW — test, batch/structural)

**Analog 1 (hash-fixture drift-check pattern — directly reusable for the CSP hash):** `crates/trackly-app/tests/pdf_determinism.rs`:
```rust
use sha2::{Digest, Sha256};
// ...
let mut hasher = Sha256::new();
hasher.update(&bytes);
let actual = format!("{:x}", hasher.finalize());

let expected = include_str!("fixtures/act_42.sha256").trim();
assert_eq!(
    actual, expected,
    // ...
);
```
This is the exact mechanism D-14's drift-detection guard needs: hash the bootstrap script's exact static text (or read it from `pagedPreviewBootstrap.ts` if a Rust-side test can access it — more likely the Rust test just re-hashes a literal copy of the script text embedded via `include_str!` from a shared fixture, OR the planner may choose to keep the CSP hash-check purely in `ui/scripts/`). `sha2` is already a workspace dependency (`Cargo.toml:57`, and already imported in `trackly-app`'s `Cargo.toml:74` and used in `src/server/tls.rs` and `tests/downgrade_protection.rs`) — no new crate needed.

**Analog 2 (raw-template-text D-13 `@page`-parity test — the actual stated precedent):** `crates/trackly-app/tests/html_act_render.rs` — read in full (lines 1-120 shown; note this test suite renders through the FULL `ActService::render_pdf` pipeline with substituted template variables, it does NOT read raw `.html` template bytes directly). The `@page`-parity test is structurally simpler than anything in this file — no existing test reads raw template text directly. Reuse this file's test-module *conventions* (doc-comment header citing the plan/phase, `#[tokio::test]` only where async DB access is needed — the `@page` test itself needs no DB/async, so it can be a plain `#[test]`), not its pipeline machinery.

**Analog 3 (path resolution for reading `crates/trackly-app/templates/*.html` robustly):** `crates/trackly-app/src/pdf/html_templates.rs`:
```rust
pub fn resolve_templates_dir(paths: &Paths) -> PathBuf {
    match std::env::var("TRACKLY_TEMPLATES_DIR") {
        Ok(v) if !v.is_empty() => PathBuf::from(v),
        _ => paths.templates_dir().to_path_buf(),
    }
}
```
For a static structural test that just needs the *shipped default* templates (not a user-overridden `TRACKLY_TEMPLATES_DIR`), a simpler `include_str!("../templates/act_handover.html")`-style compile-time path (relative to `crates/trackly-app/tests/`, i.e. `../templates/...`) is likely sufficient and avoids the env-var indirection entirely — RESEARCH.md's own illustrative code example (Code Examples section, `all_three_templates_share_identical_page_block`) uses plain `std::fs::read_to_string(path)` with a relative literal path, which is the simpler and more appropriate choice for a test that intentionally targets the on-disk source templates, not a runtime-resolved/overridable directory.

**Analog 4 (CSP-string integration-test assertion style, if the hash-source itself gets a `security_headers.rs`-style test):** `crates/trackly-app/tests/security_headers.rs`, lines 81-98:
```rust
let csp = headers
    .get("content-security-policy")
    .and_then(|v| v.to_str().ok())
    .unwrap_or("");
assert!(
    csp.contains("frame-src") && csp.contains("blob:"),
    "CSP must include frame-src with blob: for PDF preview, got: {csp}"
);
```
If the planner decides the D-14 CSP-hash change also needs a `security_headers.rs`-style live-router assertion (in addition to the static hash-drift check), this `csp.contains("...")` pattern against a real `oneshot()`-built router response is the established idiom — add a new `assert!(csp.contains("sha256-"), ...)` following this exact style, in the same file (`security_headers.rs`) rather than a new file, since it tests the same `build_router` output the other assertions already cover.

---

### `ui/package.json` (config, dependency manifest)

**Analog:** itself — existing `dependencies` block.

**Current shape** (full `dependencies`/`devDependencies` block already includes a comparable heavy client-side library):
```json
"dependencies": {
  "@tauri-apps/api": "^2.11.0",
  "@tauri-apps/plugin-dialog": "^2.7.1",
  "@tauri-apps/plugin-fs": "^2.4.2",
  "@tauri-apps/plugin-process": "^2",
  "@tauri-apps/plugin-shell": "^2.3.1",
  "pdfjs-dist": "^4.10.38",
  "svelte": "^5.55.0",
  "svelte-spa-router": "^5.1.0"
}
```
`pdfjs-dist` is the existing precedent for "a large, self-contained, offline-capable rendering library shipped as a direct `dependencies` entry" (not `devDependencies` — `pagedjs` should go in `dependencies` for the same reason: it must ship in the production bundle, not just be a build-time tool). Add `"pagedjs": "^0.4.3"` alphabetically between `"@tauri-apps/plugin-shell"` and `"pdfjs-dist"`. Install via `pnpm --dir ui add pagedjs@^0.4.3` (per RESEARCH.md's Standard Stack section) to keep the lockfile consistent — do not hand-edit `package.json` without also updating `pnpm-lock.yaml`.

---

## Shared Patterns

### `isTauri` desktop/LAN branch
**Source:** `ui/src/lib/stores/transport.svelte.ts` (canonical, shared version):
```typescript
// Transport detection — evaluated once at module load time.
// isTauri: true when running inside Tauri webview (desktop app),
// false when served to a LAN browser (Phase 5+ server mode).
export const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
```
**Apply to:** `PdfPreviewModal.svelte`'s `handlePrint()` branch. Note: `PdfPreviewModal.svelte` currently has its OWN file-local copy of this constant (line 45, byte-identical logic) instead of importing the shared store — `DeviceImportCsvModal.svelte` already imports the shared version (`import { isTauri } from '$lib/stores/transport.svelte';`). Since this phase already touches this file extensively, switching to the shared import is a small, in-scope cleanup (not required by CONTEXT.md, but consistent with the existing better-practice example elsewhere in the codebase — flag as discretionary, not mandatory).

### `srcdoc` iframe with `sandbox` (opaque-origin preview)
**Source:** `ui/src/features/settings/TemplateEditor.svelte` (a second, independent existing consumer of the exact same pattern):
```svelte
<!-- HTML preview iframe (Plan 17-03, D-11: srcdoc, no blob/PDF object URL) -->
{#if previewHtml}
  <div class="preview-wrapper">
    <iframe sandbox="" srcdoc={previewHtml} title="Превью" class="pdf-iframe"></iframe>
  </div>
{/if}
```
**Apply to:** confirms `sandbox=""` + `srcdoc` is an established, repeated project idiom (not unique to `PdfPreviewModal.svelte`) — `TemplateEditor.svelte` is NOT in this phase's scope (CONTEXT.md doesn't mention it) but is worth knowing about: if the CSP hash-source fix or Paged.js pattern generalizes well, `TemplateEditor.svelte`'s preview iframe is a candidate for a future, out-of-scope follow-up. Do not modify it in this phase.

### Theme resolution
**Source:** `ui/src/lib/stores/theme.svelte.ts`:
```typescript
export const themeStore = $state({
  preference: 'system' as Preference,
  resolved: 'light' as Resolved,
});
```
and `applyResolved()`'s `document.documentElement.dataset.theme = r;`.
**Apply to:** D-08's backdrop-color propagation. Read `document.documentElement.dataset.theme` (or `themeStore.resolved` directly, since it's an exported `$state` object) at `srcdoc`-build time to pick the literal backdrop hex (`#e4e8f0` light / `#0a0d12` dark — from `_tokens.scss`, see below) to inline into the bootstrap `<style>`. For live theme-toggle-while-open (edge case), there is no existing "watch theme and react" example in the codebase to copy from directly — `applyResolved()` is imperative/one-shot, not reactive-subscribable beyond Svelte's own `$state` reactivity, so a `$effect` in `PdfPreviewModal.svelte` reading `themeStore.resolved` and firing the `postMessage` update (per UI-SPEC's Dark-Theme Rendering mechanics) is new but straightforward Svelte 5 rune usage, not a novel pattern.

### Design tokens for D-08/D-09
**Source:** `ui/src/styles/_tokens.scss`:
```scss
/* light */
--tr-surface-sunken: #e4e8f0;
--tr-border: #e1e6ef;
--tr-elev-2: 0 2px 6px rgba(16, 22, 34, 0.09), 0 1px 2px rgba(16, 22, 34, 0.06);
/* dark */
--tr-surface-sunken: #0a0d12;
--tr-border: #272e3a;
--tr-elev-2: 0 3px 10px rgba(0, 0, 0, 0.55), 0 1px 2px rgba(0, 0, 0, 0.5);
```
**Apply to:** `.pdf-page-frame` background (D-08, resolved via CSS `var(--tr-surface-sunken)` in the parent document's own stylesheet — no manual hex needed there, only inside the opaque-origin `srcdoc` where CSS custom properties don't cross the boundary, per UI-SPEC's Dark-Theme Rendering section — the literal hex values above are exactly what must be inlined into the bootstrap `<style>` for the two themes). `--tr-elev-2` for `.pdf-iframe` box-shadow (D-09, unchanged token, already correctly used).

### Progress/loading `aria-live` idiom
**Source:** `CartridgeDetail.svelte:106`:
```svelte
<div class="detail-loading" aria-live="polite">
  <Spinner size="md" />
```
**Apply to:** `PdfPreviewModal.svelte`'s `.state.state-loading` container — add `aria-live="polite"` (currently missing on this specific state div, though the pattern is established elsewhere in the app per UI-SPEC's Accessibility section).

### RU pluralization
**No existing helper found.** Grepped `ui/src` for `pluralize`/`declOfNum`/similar — zero hits. UI-SPEC's Open Item #3 already flags this as unverified; confirmed here: this phase must write a small one-off pluralization helper for "N страниц/страницы/страница" (standard RU 1/2-4/5+ rule) — there is nothing to copy from internally. Recommend placing it in `ui/src/lib/utils/` (new small file, e.g. `pluralize.ts`) rather than inline in `PdfPreviewModal.svelte`, matching the existing `date.ts`/`dropdownAnchor.ts` one-function-per-file utility convention — this makes it reusable if another counter (devices, cartridges) needs RU pluralization later.

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `ui/src/features/acts/pagedPreviewBridge.ts` | utility | event-driven | No prior `postMessage`/`MessageEvent` usage anywhere in `ui/src` (grepped, zero hits) — this is the first cross-context message-passing code in the frontend. Structure it per RESEARCH.md Pattern 2 directly; use `portal.ts`/`afterprint`-cleanup idiom only for the "attach + return teardown" shape, not the message-handling logic itself. |
| Vite `?raw` raw-text import | build config usage | — | Zero prior use of Vite's `?raw` suffix in this codebase (grepped `ui/src`, `ui/vite.config.*`) — first occurrence. No `vite.config.ts` change is expected to be needed (the `?raw` suffix is a built-in Vite feature, not a plugin), but flag this as unverified/first-use so the planner budgets a quick sanity check that it Just Works with this project's exact Vite 6 + `@sveltejs/vite-plugin-svelte` setup. |

## Metadata

**Analog search scope:** `ui/src/` (features/acts, features/settings, features/devices, features/reports, lib/components, lib/stores, lib/utils), `ui/scripts/`, `ui/package.json`, `crates/trackly-app/src/http/mod.rs`, `crates/trackly-app/src/pdf/html_templates.rs`, `crates/trackly-app/templates/*.html`, `crates/trackly-app/tests/` (security_headers.rs, html_act_render.rs, pdf_determinism.rs, downgrade_protection.rs).
**Files scanned:** ~20 read/grepped directly this session (in addition to the 3 phase-artifact files: CONTEXT.md, RESEARCH.md, UI-SPEC.md, which were already exhaustively researched upstream).
**Pattern extraction date:** 2026-08-04

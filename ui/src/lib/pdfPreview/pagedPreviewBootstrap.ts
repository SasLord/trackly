/**
 * Phase 33 (D-04/D-05/D-08/D-09): строит `srcdoc` для превью печати — исходный
 * HTML документа (акт/приёмка/отчёт, не меняется, D-01) + инлайновый CSS
 * "хром" листа/подложки + инлайновый бутстрап-скрипт Paged.js.
 *
 * `PAGED_PREVIEW_INLINE_SCRIPT` собирается по ФИКСИРОВАННОЙ формуле
 * (`pagedjsLibraryText + ';\n' + bootstrapText`) — Plan 33-02 пересчитывает
 * SHA-256 этой же строки для CSP `script-src` LAN-режима (D-14); не менять
 * порядок конкатенации и не добавлять символы между частями.
 */

// Relative filesystem import (not the bare `pagedjs/dist/paged.min.js`
// package specifier): pagedjs's package.json `exports` map only exposes the
// bare `pagedjs` root specifier (condition-gated: import/require/browser/
// polyfill/default) — it declares no `./dist/*` subpath exports at all — so
// Vite/Rollup's strict Node package-exports resolution rejects a deep
// `pagedjs/dist/paged.min.js` import once this module is actually reachable
// from the app's entry graph (it wasn't yet in Plan 33-01, so the bug was
// latent until this plan wires `buildSrcdoc` into PdfPreviewModal.svelte).
// A relative path resolves as a plain filesystem path and bypasses package
// `exports` enforcement entirely; pagedjs is a direct `ui` dependency, so
// pnpm always places it at `ui/node_modules/pagedjs`, three levels up from
// this file (pdfPreview -> lib -> src -> ui). Bytes read are identical
// either way — only the resolution mechanism changed.
import pagedjsLibraryText from '../../../node_modules/pagedjs/dist/paged.min.js?raw';
import bootstrapText from './bootstrapScript.js?raw';

/** Точная формула — Plan 33-02's CSP hash-drift script её воспроизводит. */
export const PAGED_PREVIEW_INLINE_SCRIPT = pagedjsLibraryText + ';\n' + bootstrapText;

/**
 * Литеральные hex-значения подложки/тени листа для обеих тем (D-08/D-09).
 * Экспортируется (не module-private) — Plan 33-03 переиспользует те же
 * значения для live `trackly-theme-update` postMessage при переключении
 * темы, не дублируя их.
 */
export const THEME_CHROME = {
  light: {
    backdrop: '#e4e8f0',
    shadow: '0 2px 6px rgba(16, 22, 34, 0.09), 0 1px 2px rgba(16, 22, 34, 0.06)',
  },
  dark: {
    backdrop: '#0a0d12',
    shadow: '0 3px 10px rgba(0, 0, 0, 0.55), 0 1px 2px rgba(0, 0, 0, 0.5)',
  },
} as const;

/**
 * Строит итоговый `srcdoc` для превью-iframe: исходный HTML документа +
 * инлайновый `<style>` хрома листа/подложки (D-08/D-09, без `@page` — базовый
 * стиль Paged.js уже нейтрализует исходный `@page`, RESEARCH.md
 * Anti-Patterns) + инлайновый `<script>` с бутстрапом Paged.js.
 */
export function buildSrcdoc(actHtml: string, theme: 'light' | 'dark'): string {
  const chrome = THEME_CHROME[theme];
  const style =
    '<style>' +
    `body { margin: 0; background: ${chrome.backdrop}; }` +
    '.pagedjs_pages { display: flex; flex-direction: column; align-items: center; gap: 24px; padding: 16px 0; }' +
    `.pagedjs_page { box-shadow: ${chrome.shadow}; }` +
    '</style>';

  // '<' + 'script>' / '<' + '/script>' concatenation idiom, reused verbatim
  // from PdfPreviewModal.svelte's existing printViaSystemBrowser autoPrint —
  // avoids a literal `</script>` substring inside this .ts file's own source.
  const script = '<' + 'script>' + PAGED_PREVIEW_INLINE_SCRIPT + '<' + '/script>';

  const injected = style + script;

  return /<\/body>/i.test(actHtml)
    ? actHtml.replace(/<\/body>/i, `${injected}</body>`)
    : `${actHtml}${injected}`;
}

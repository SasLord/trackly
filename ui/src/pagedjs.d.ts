/**
 * Phase 33-04 (D-06): minimal ambient module declaration for `pagedjs`.
 *
 * The `pagedjs` package ships no TypeScript types (no `types`/`typings`
 * field in its package.json, no `@types/pagedjs` package exists on npm).
 * `printViaTopLevel` in PdfPreviewModal.svelte dynamically imports it
 * (`await import('pagedjs')`) to re-paginate the LAN print path — this
 * declaration only covers the small surface actually used there.
 *
 * Quick 260805-edd: each `stylesheets` entry can be either a `string` (a URL
 * that Paged.js's internal `Polisher.add()` fetches over the network via
 * `request(...)`) or an object whose values are used directly as CSS text
 * (`pagedjs/dist/paged.js` ~L27506, `async add()`, branches on
 * `typeof arguments[i] === 'object'`). The original `string[]`-only type
 * here under-represented the real runtime API and made the object form a
 * type error even though Paged.js supports (and, per this fix, requires) it
 * for inline CSS text.
 *
 * Quick 260805-jwf: added `polisher`, the `Previewer` constructor's plain
 * instance property (`this.polisher = new Polisher(false)`,
 * `pagedjs/dist/paged.esm.js` ~L33031) exposing `Polisher.destroy()`
 * (`this.styleEl.remove(); this.inserted.forEach(s => s.remove());`) — the
 * only way to remove the `<style data-pagedjs-inserted-styles>` elements
 * `Polisher.insert()` appends to `document.head` on every `preview()` call.
 * Typed to the minimal surface `printViaTopLevel` actually calls, not the
 * full (untyped, upstream) `Polisher` class.
 */
declare module 'pagedjs' {
  export class Previewer {
    constructor();
    preview(
      content?: string | HTMLElement,
      stylesheets?: (string | Record<string, string>)[],
      renderTo?: HTMLElement,
    ): Promise<{ total: number }>;
    polisher: { destroy: () => void };
  }
}

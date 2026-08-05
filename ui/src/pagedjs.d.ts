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
 */
declare module 'pagedjs' {
  export class Previewer {
    constructor();
    preview(
      content?: string | HTMLElement,
      stylesheets?: (string | Record<string, string>)[],
      renderTo?: HTMLElement,
    ): Promise<{ total: number }>;
  }
}

/**
 * Phase 33-04 (D-06): minimal ambient module declaration for `pagedjs`.
 *
 * The `pagedjs` package ships no TypeScript types (no `types`/`typings`
 * field in its package.json, no `@types/pagedjs` package exists on npm).
 * `printViaTopLevel` in PdfPreviewModal.svelte dynamically imports it
 * (`await import('pagedjs')`) to re-paginate the LAN print path — this
 * declaration only covers the small surface actually used there.
 */
declare module 'pagedjs' {
  export class Previewer {
    constructor();
    preview(
      content?: string | HTMLElement,
      stylesheets?: string[],
      renderTo?: HTMLElement,
    ): Promise<{ total: number }>;
  }
}

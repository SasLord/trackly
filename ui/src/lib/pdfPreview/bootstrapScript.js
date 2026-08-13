// Phase 33 (D-04/D-07/C-01): Paged.js bootstrap protocol script.
//
// This file is a PLAIN standalone browser script (no ES module syntax) because
// its exact text is concatenated raw into an inline <script> tag inside the
// preview iframe's `srcdoc` (see pagedPreviewBootstrap.ts). It must remain a
// single static string with zero per-call interpolation: Plan 33-02 hardcodes
// a SHA-256 hash of this text (combined with the Paged.js library text) into
// the LAN-mode CSP `script-src` allow-list. Any edit to this file's bytes
// requires regenerating that hash — do not templatize it.
//
// Runs INSIDE the opaque-origin srcdoc iframe (sandbox="allow-scripts", no
// allow-same-origin, per D-05). Communicates with the parent exclusively via
// postMessage using these four message types (matched by exact string
// elsewhere in the app — do not rename):
//   trackly-pagedjs-progress  { pages: number }             one per rendered page
//   trackly-pagedjs-done      { total: number, height: number }
//   trackly-pagedjs-error     { message: string }
//   trackly-theme-update      { backdrop: string }  (INCOMING, parent -> iframe)
(function () {
  // Paged.js's UMD build (dist/paged.min.js, pinned 0.4.3 — see
  // pagedPreviewBootstrap.ts) attaches its exports to `window.PagedModule`,
  // NOT `window.Paged`. `window.Paged` is the global name used only by the
  // separate `dist/paged.polyfill.js` build, which this project does not
  // import. Do not "fix" this back to `window.Paged` — that global is
  // undefined at runtime and this line was previously the first thing to
  // throw (before any postMessage could fire), which always forced the
  // 8s D-02 degrade timeout.
  // D-15/D-15a (Phase 36): pagedjs 0.4.3 does not natively repeat <thead>
  // when a table.appendix-table (act_handover.html's multi-device appendix)
  // is split across pages — verified by reading
  // ui/node_modules/pagedjs/src/chunker/{chunker,layout}.js: no thead-cloning
  // logic exists there, and the upstream PR (#160) that would add it is
  // unmerged. This Handler clones the ORIGINAL <thead> (captured before
  // pagination starts, in the constructor, from the still-intact source DOM)
  // into every page fragment of table.appendix-table that doesn't already
  // have one (i.e. every continuation fragment after the first). Scoped
  // strictly to table.appendix-table — never touches any other table or DOM
  // on the page (T-36-03 threat register). MUST be kept logically identical
  // to the mirror copy in ui/src/features/acts/PdfPreviewModal.svelte's
  // printViaTopLevel() (D-15a) — that separate ESM `import('pagedjs')` code
  // path does not go through this UMD bootstrap at all, so a one-sided edit
  // here silently breaks only LAN print while desktop/preview keep working.
  //
  // MUST be a native ES6 `class ... extends`, NOT ES5 pseudo-inheritance
  // (borrowing the parent constructor via the Function.prototype invocation
  // helper, then wiring the prototype chain by hand) even though the rest of
  // this file is deliberately ES5 (var/function). Paged.js ships `Handler` as
  // a native ES6 class in its UMD bundle (dist/paged.min.js) — a native class
  // constructor CANNOT be invoked that ES5 way, it throws `TypeError: Cannot
  // call a class constructor <name> without |new|` at Previewer construction
  // time, which the D-02 degrade path then swallows into a silent "no
  // pagination, no page chrome" fallback (regressed once already — fixed
  // after live desktop UAT during Plan 36-04; see
  // scripts/check-pagedjs-csp-hash.mjs's checkHandlerIsNativeClass for the
  // structural regression guard). Native class syntax is safe here despite
  // the "no ES module syntax" note at the top of this file: this text is
  // imported with `?raw` and never passed through Vite/esbuild transpilation,
  // so it reaches the WKWebView/WebView2 runtime verbatim, and both support
  // ES2015 classes natively.
  class RepeatTableHeadHandler extends window.PagedModule.Handler {
    constructor(chunker, polisher, caller) {
      super(chunker, polisher, caller);
      var sourceTable = document.querySelector('table.appendix-table');
      var sourceThead = sourceTable ? sourceTable.querySelector('thead') : null;
      this.savedThead = sourceThead ? sourceThead.cloneNode(true) : null;
    }

    afterPageLayout(pageElement) {
      if (!this.savedThead) return;
      var savedThead = this.savedThead;
      pageElement.querySelectorAll('table.appendix-table').forEach(function (table) {
        if (table.querySelector('thead')) return; // already has one — first fragment
        table.insertBefore(savedThead.cloneNode(true), table.firstChild);
      });
    }
  }
  window.PagedModule.registerHandlers(RepeatTableHeadHandler);

  var previewer = new window.PagedModule.Previewer();
  var pages = 0;

  previewer.chunker.on('renderedPage', function () {
    pages += 1;
    parent.postMessage({ type: 'trackly-pagedjs-progress', pages: pages }, '*');
  });

  // Live theme-toggle propagation (D-08 mechanics): the iframe has no access
  // to the parent's CSS custom properties (opaque origin), so the parent
  // sends the literal backdrop hex on theme change instead of us re-reading
  // srcdoc (which would destroy in-progress/completed pagination state, see
  // RESEARCH.md Pitfall 5). No e.source check here: this direction is
  // parent-to-iframe, the iframe cannot identify "its own parent" any more
  // precisely than "whoever posted", and the payload is a non-secret CSS
  // color string (RESEARCH.md Pattern 2).
  window.addEventListener('message', function (e) {
    if (e.data && e.data.type === 'trackly-theme-update') {
      document.body.style.background = e.data.backdrop;
    }
  });

  previewer
    .preview()
    .then(function (flow) {
      var pagesEl = document.querySelector('.pagedjs_pages');
      var height = pagesEl ? pagesEl.scrollHeight : document.body.scrollHeight;
      parent.postMessage({ type: 'trackly-pagedjs-done', total: flow.total, height: height }, '*');
    })
    .catch(function (err) {
      parent.postMessage({ type: 'trackly-pagedjs-error', message: String(err) }, '*');
    });
})();

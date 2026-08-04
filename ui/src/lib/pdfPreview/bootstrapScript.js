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
  var previewer = new window.Paged.Previewer();
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

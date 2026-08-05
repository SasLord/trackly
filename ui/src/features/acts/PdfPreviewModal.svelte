<script lang="ts">
  // Phase 16: Document print-preview modal.
  //
  // Renders the backend-generated HTML document directly in an <iframe> via
  // `srcdoc` — no blob/object-URL lifecycle, no PDF bytes, for on-screen
  // preview. Printing (and "Save as PDF") happens through the browser's
  // native print dialog.
  //
  // GAP-16-01 fix: neither `iframeEl.contentWindow.print()` nor top-level
  // `window.print()` reliably opens the native print panel inside Tauri's
  // desktop webview (WKWebView on macOS). Both are documented upstream as
  // broken (tauri-apps/tauri#13451 — iframe print is a silent no-op even
  // though window.print() works; tauri-apps/tauri#3066 — window.print()
  // itself can fail on WKWebView because printOperationWithPrintInfo: is not
  // implemented by Wry without host-side integration). It DID work in a real
  // LAN browser (Chrome/Edge/WebView2), which is why the bug only reproduced
  // in the desktop app.
  //
  // Fix (branches on isTauri, matches the existing pattern already used by
  // ReportsPage.svelte's printReport()/exportPdf()):
  //   - Desktop (Tauri): do NOT rely on webview print at all. Write the
  //     rendered HTML to a temp .html file (tauri-plugin-fs) and open it in
  //     the OS default browser (tauri-plugin-shell `open`) — the same native
  //     print dialog that already works for LAN-browser users, just launched
  //     from the desktop app instead of relied upon inside the webview.
  //   - LAN browser: print from the TOP-LEVEL document instead of the
  //     iframe — inject the doc's own inline styles (incl. @page) and body
  //     markup into a hidden #print-root host in the main document, scope
  //     visibility with @media print, call window.print(). This is a more
  //     robust replacement for iframe.contentWindow.print() in real browsers
  //     too (no cross-frame print quirks) and keeps D-09 (works in both
  //     modes) satisfied.
  //
  // Buttons:
  //   - Print → handlePrint() (offers "Save as PDF" natively in both modes).
  //   - Закрыть.

  import Button from '$lib/components/Button.svelte';
  import Modal from '$lib/components/Modal.svelte';
  import Spinner from '$lib/components/Spinner.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { acts } from '$lib/api/acts';
  import { apiCall } from '$lib/api/client';
  import { isTauri } from '$lib/stores/transport.svelte';
  import {
    buildSrcdoc,
    THEME_CHROME,
    PAGED_PREVIEW_INLINE_SCRIPT,
  } from '$lib/pdfPreview/pagedPreviewBootstrap';
  import { attachBridge } from '$lib/pdfPreview/pagedPreviewBridge';
  import { themeStore } from '$lib/stores/theme.svelte';
  import { pluralizeRu } from '$lib/utils/pluralize';

  interface AcceptancePayload {
    deviceId: number;
    giverName: string;
    receiverName: string;
    dateUtc: number;
    deviceName?: string;
  }

  /** Plan 17-03 (D-09): reports_export_pdf self-fetch params for mode='report'. */
  export interface ReportParams {
    reportType: string;
    filter: unknown;
    period?: unknown;
  }

  interface Props {
    open: boolean;
    actId: number | null;
    title: string;
    onClose: () => void;
    /** Plan 03-05: 'handover' → render акта приёма-передачи (default);
     *  'acceptance' → render документа приёма устройства (DEV-14);
     *  'report' → render отчёта (Plan 17-03, D-09). */
    mode?: 'handover' | 'acceptance' | 'report';
    /** Required when mode='acceptance'. */
    acceptancePayload?: AcceptancePayload | null;
    /** Required when mode='report'. */
    reportParams?: ReportParams | null;
  }

  const {
    open,
    actId,
    title,
    onClose,
    mode = 'handover',
    acceptancePayload = null,
    reportParams = null,
  }: Props = $props();

  let htmlContent = $state<string | null>(null);
  let loading = $state(false);
  let errorMsg = $state<string | null>(null);

  /** Phase 33 (D-02/D-07/D-10/D-11): Paged.js on-screen pagination state. */
  let srcdoc = $state<string | null>(null);
  let paginationStatus = $state<'idle' | 'pending' | 'done' | 'degraded'>('idle');
  let pageProgress = $state(0);
  let pageTotal = $state<number | null>(null);
  let naturalHeightPx = $state(1123);
  let iframeEl = $state<HTMLIFrameElement | null>(null);
  let frameWidthPx = $state(0);
  /** D-11: fit-to-width scale, ceiling of 1 — never enlarges beyond natural size.
   *  Divisor is 842 (794px @page width + 24px horizontal gutter on each side,
   *  see pagedPreviewBootstrap.ts's .pagedjs_pages padding), matching
   *  .pdf-iframe/.pdf-scale-inner's actual box width — see debug session
   *  print-preview-always-degrades.md, defect #5. Leaving this at 794 would
   *  under-scale the sheet and reintroduce horizontal overflow at narrow
   *  widths. */
  const scaleFactor = $derived(frameWidthPx > 0 ? Math.min(1, frameWidthPx / 842) : 1);

  const PAGINATION_TIMEOUT_MS = 8000;
  /** Not $state — plain closure-shared handle, not rendered anywhere. */
  let degradeTimeoutHandle: ReturnType<typeof setTimeout> | null = null;

  function clearDegradeTimeout() {
    if (degradeTimeoutHandle !== null) {
      clearTimeout(degradeTimeoutHandle);
      degradeTimeoutHandle = null;
    }
  }

  /** D-02: revert to the pre-Phase-33 unpaginated preview. */
  function enterDegraded(reason: string) {
    paginationStatus = 'degraded';
    console.warn(
      '[PdfPreviewModal] Paged.js pagination ' +
        reason +
        ' — falling back to unpaginated preview (D-02).',
    );
  }

  function renderCall(): Promise<string> {
    if (mode === 'acceptance') {
      if (!acceptancePayload) {
        return Promise.reject(new Error('acceptancePayload required for mode="acceptance"'));
      }
      return acts.renderAcceptancePdf(
        acceptancePayload.deviceId,
        acceptancePayload.giverName,
        acceptancePayload.receiverName,
        acceptancePayload.dateUtc,
      );
    }
    if (mode === 'report') {
      if (!reportParams) {
        return Promise.reject(new Error('reportParams required for mode="report"'));
      }
      return apiCall<string>('reports_export_pdf', {
        reportType: reportParams.reportType,
        filter: reportParams.filter,
        period: reportParams.period,
      });
    }
    if (actId === null) {
      return Promise.reject(new Error('actId required for mode="handover"'));
    }
    return acts.renderPdf(actId);
  }

  const ready = $derived(
    open &&
      (mode === 'acceptance'
        ? acceptancePayload !== null
        : mode === 'report'
          ? reportParams !== null
          : actId !== null),
  );

  /** D-07: keep showing the loading state until pagination has settled one
   *  way or the other (done or degraded), not merely until the HTML string
   *  arrived. */
  const showLoading = $derived(
    loading ||
      (htmlContent !== null && paginationStatus !== 'done' && paginationStatus !== 'degraded'),
  );

  $effect(() => {
    if (!ready) {
      htmlContent = null;
      errorMsg = null;
      srcdoc = null;
      paginationStatus = 'idle';
      return;
    }

    loading = true;
    errorMsg = null;
    let cancelled = false;

    (async () => {
      try {
        const html = await renderCall();
        if (cancelled) return;
        htmlContent = html;
        // Built ONCE, imperatively — never as a $derived, which would re-run
        // (and reload the iframe) on every later themeStore.resolved change
        // and destroy in-progress pagination state (RESEARCH.md Pitfall 5).
        srcdoc = buildSrcdoc(html, themeStore.resolved);
        paginationStatus = 'pending';
        pageProgress = 0;
        pageTotal = null;
        degradeTimeoutHandle = setTimeout(() => {
          if (paginationStatus !== 'done') {
            enterDegraded('timeout');
          }
        }, PAGINATION_TIMEOUT_MS);
      } catch (e: unknown) {
        if (cancelled) return;
        const msg =
          e && typeof e === 'object' && 'message' in e
            ? String((e as { message: unknown }).message)
            : 'Не удалось сгенерировать PDF';
        errorMsg = msg;
      } finally {
        if (!cancelled) loading = false;
      }
    })();

    return () => {
      cancelled = true;
      clearDegradeTimeout();
    };
  });

  // Wires the opaque-origin postMessage bridge to the paginated preview
  // iframe once it is mounted (bind:this={iframeEl} in the template).
  $effect(() => {
    if (iframeEl === null) return;
    return attachBridge(iframeEl, (data) => {
      const msg = data as { type?: string };
      switch (msg.type) {
        case 'trackly-pagedjs-progress': {
          // 33-UI-SPEC.md's timeout definition is "8s from srcdoc being set
          // to the FIRST trackly-pagedjs-progress OR trackly-pagedjs-done
          // message" (see 33-RESEARCH.md Pitfall 1: the timeout only exists
          // to detect total silence — a CSP-blocked/failed bootstrap script
          // — not to cap the full pagination run). Clear it here too, not
          // only on 'done', or a normal multi-page document that is still
          // actively paginating past the 8s mark would incorrectly degrade.
          clearDegradeTimeout();
          pageProgress = (data as { pages: number }).pages;
          break;
        }
        case 'trackly-pagedjs-done': {
          const d = data as { total: number; height: number };
          pageTotal = d.total;
          naturalHeightPx = d.height;
          paginationStatus = 'done';
          clearDegradeTimeout();
          break;
        }
        case 'trackly-pagedjs-error': {
          clearDegradeTimeout();
          enterDegraded('error: ' + (data as { message: string }).message);
          break;
        }
        default:
          break;
      }
    });
  });

  // Live theme propagation into the already-loaded iframe — postMessage
  // ONLY, never reassigns `srcdoc` (that would reload the iframe and lose
  // in-progress pagination, RESEARCH.md Pitfall 5).
  $effect(() => {
    if (iframeEl?.contentWindow && paginationStatus !== 'idle') {
      iframeEl.contentWindow.postMessage(
        { type: 'trackly-theme-update', backdrop: THEME_CHROME[themeStore.resolved].backdrop },
        '*',
      );
    }
  });

  const PRINT_ROOT_ID = 'act-print-root';
  const PRINT_STYLE_ID = 'act-print-style';

  /**
   * GAP-16-01 desktop fix: Tauri's webview (WKWebView/WebView2) print
   * support is unreliable from JS (see script-block comment). Instead of
   * fighting the webview print bridge, write the rendered HTML to a temp
   * file and hand it to the OS default browser via tauri-plugin-shell's
   * `open` — that's a real, fully-capable browser where native print
   * already works (proven by the LAN-browser path). Mirrors the pattern
   * already used by ReportsPage.svelte's printReport()/exportPdf().
   *
   * Phase 33 (D-06/C-02/C-03): auto-print must wait for Paged.js pagination
   * to finish, not the document `load` event — printing before pagination
   * used the browser's own native pagination instead of the same engine the
   * on-screen preview uses. The temp file now embeds the SAME frozen
   * `PAGED_PREVIEW_INLINE_SCRIPT` bundle used by the on-screen preview
   * (Plan 33-01) so it re-paginates the document itself before printing, and
   * a small second script listens for the bootstrap's own
   * `trackly-pagedjs-done` postMessage to trigger `window.print()`. In a
   * top-level `file://` document (not an iframe), `parent === window`, so
   * the bootstrap's `parent.postMessage(...)` call dispatches to `window`
   * itself — a document can postMessage to itself, and this listener
   * (registered on the same `window`) receives it. No CSP applies to this
   * path: `tauri.conf.json` sets `"security": {"csp": null}` for the Tauri
   * app, and Tauri does not control the *external* browser this temp file
   * is opened in. Bundling Paged.js inline (not a CDN) keeps the temp file
   * self-contained per C-02/portable-mode discipline.
   */
  async function printViaSystemBrowser(html: string) {
    // Build both tags via concatenation so the literal '</scr'+'ipt>' does
    // not prematurely close this component's own <script> block at compile
    // time — same idiom already used elsewhere in this file.
    const pagedjsScript = '<' + 'script>' + PAGED_PREVIEW_INLINE_SCRIPT + '<' + '/script>';
    const printTriggerScript =
      '<' +
      'script>window.addEventListener("message",function(e){if(e.source!==window)return;if(e.data&&e.data.type==="trackly-pagedjs-done"){setTimeout(function(){window.print()},100)}})<' +
      '/script>';
    const injected = pagedjsScript + printTriggerScript;
    // MUST use a replacer FUNCTION, not a string — `injected` embeds the full
    // minified Paged.js bundle, which contains a literal `$`` substring (see
    // the matching comment in pagedPreviewBootstrap.ts's buildSrcdoc). A
    // string replacement argument interprets `$`` as a special "portion
    // before the match" substitution pattern and corrupts the bundle; a
    // function return value is inserted verbatim. Do not revert to a plain
    // template-literal string replacement here.
    const htmlWithPagination = /<\/body>/i.test(html)
      ? html.replace(/<\/body>/i, () => `${injected}</body>`)
      : `${html}${injected}`;

    const { writeTextFile, BaseDirectory } = await import('@tauri-apps/plugin-fs');
    const { open: openPath } = await import('@tauri-apps/plugin-shell');
    const fileName = `trackly-print-${Date.now()}.html`;
    await writeTextFile(fileName, htmlWithPagination, { baseDir: BaseDirectory.Temp });
    const { tempDir, join } = await import('@tauri-apps/api/path');
    const filePath = await join(await tempDir(), fileName);
    await openPath(filePath);
  }

  /**
   * LAN-browser fix: prints the document from the TOP-LEVEL window instead
   * of the preview iframe's contentWindow (which is unreliable/no-op cross-
   * frame in some webviews — see script-block comment). Extracts the
   * inline style and body markup from the self-contained backend-rendered
   * HTML string, mounts them into a hidden host in the main document, and
   * calls window.print() on the top-level window.
   *
   * Phase 33 (D-06/C-03): re-runs Paged.js pagination against #act-print-root
   * before printing, via a dynamic `import('pagedjs')` — self-hosted ESM,
   * code-split by Vite, served from the app's own 'self' origin, so it needs
   * no CSP script-src change (unlike the opaque-origin preview iframe's
   * inline script, which does). window.print() now fires only after
   * pagination resolves, not immediately after the synchronous DOM
   * injection — printing must reflect the paginated result, not fire before
   * it exists.
   */
  async function printViaTopLevel(html: string) {
    const parsed = new DOMParser().parseFromString(html, 'text/html');
    const bodyHtml = parsed.body?.innerHTML ?? '';
    const styleHtml = Array.from(parsed.head?.querySelectorAll('style') ?? [])
      .map((el) => el.outerHTML)
      .join('\n');
    const cssText = styleHtml.replace(/<\/?style[^>]*>/gi, '');

    let printRoot = document.getElementById(PRINT_ROOT_ID);
    if (!printRoot) {
      printRoot = document.createElement('div');
      printRoot.id = PRINT_ROOT_ID;
      document.body.appendChild(printRoot);
    }

    let printStyle = document.getElementById(PRINT_STYLE_ID) as HTMLStyleElement | null;
    if (!printStyle) {
      printStyle = document.createElement('style');
      printStyle.id = PRINT_STYLE_ID;
      document.head.appendChild(printStyle);
    }
    // Document's own inline styles (incl. @page) + visibility scoping: hide
    // the rest of the app and show only #print-root while printing. Scoping
    // is `@media print`-only so on-screen app layout is never affected.
    //
    // #act-print-root is hidden off-screen via `position: absolute; left:
    // -100000px` instead of `display: none`. `display: none` zeroes out
    // `getBoundingClientRect` for every box in the hidden subtree, and the
    // `await previewer.preview(...)` call immediately below needs real
    // geometry to paginate #act-print-root's content — Paged.js measures
    // actual DOM layout to decide page breaks. Off-screen positioning keeps
    // the container out of the visual viewport without collapsing its
    // layout box. The `@media print` block resets the position back to
    // `static`/`left: auto` so the printed/saved-as-PDF output is not pushed
    // off the page. This is a defect fix on first principles (found by
    // reading the code), NOT a confirmed fix for the specific LAN
    // print-dialog failure reported in UAT — an earlier attempt to isolate
    // it in a standalone harness was inconclusive (the control case, a
    // visible container, also hung).
    printStyle.textContent = `
      /* 260805-jwf: reset the ambient line-height/letter-spacing/word-spacing
         that #act-print-root would otherwise INHERIT from the app's own
         body { line-height: 1.5 } (global.scss), scoped to #act-print-root
         (never body) so the app's own on-screen typography is never touched
         — that scoping IS the fix for defect A (a prior font-leak
         regression came from a rule that reached the app's body). Declared
         UNCONDITIONALLY, outside any print-only media block, not only at
         print time: Paged.js's Previewer measures/paginates this DOM on
         screen, BEFORE window.print() runs, so a reset that only applied
         inside a print-only media block paginated 1.5-spaced text on screen
         and then printed it normal-spaced — that mismatch was defect B
         (regression from 260805-ifj). No !important: an element's own declared value for an
         inherited property always wins over an ancestor's inherited value
         regardless of the ancestor's specificity, so a template rule that
         targets a specific descendant directly (e.g. .header .requisites,
         line-height: 1.35, a rule none currently declare on body itself)
         still wins for that element.
         The cssText variable (template's own style-tag contents, extracted
         above) is deliberately NOT interpolated into this literal anymore —
         Paged.js's own previewer.preview() call below already applies the
         identical stylesheet via its stylesheets argument, and duplicating
         it here was defect A's actual mechanism (that copy landed in this
         shared top-level document, unscoped). What removes Paged.js's OWN injected
         copy again after each print cycle is the captured Previewer's
         polisher.destroy() call in the cleanup function below — without it,
         Paged.js's Polisher.insert() (just as unscoped as the duplicate
         removed here) would still leak past a single print cycle. */
      #${PRINT_ROOT_ID} {
        line-height: normal;
        letter-spacing: normal;
        word-spacing: normal;
        position: absolute;
        left: -100000px;
        top: 0;
      }
      @media print {
        html, body {
          background: #fff !important;
        }
        body > :not(#${PRINT_ROOT_ID}) {
          display: none !important;
        }
        #${PRINT_ROOT_ID} {
          display: block !important;
          position: static;
          left: auto;
        }
        .pagedjs_page {
          background: #fff !important;
        }
      }
    `;

    // Captured after previewer.preview() resolves (see below) — Paged.js's
    // own Polisher inserts style elements marked data-pagedjs-inserted-
    // styles into this shared document's head (unscoped, same mechanism as
    // the duplicate cssText interpolation removed above). destroy() removes
    // every element it ever inserted; without this, nothing would stop
    // those from surviving past a single print cycle (defect A).
    let injectedPolisher: { destroy: () => void } | null = null;

    const cleanup = () => {
      printRoot!.innerHTML = '';
      printStyle!.textContent = '';
      injectedPolisher?.destroy();
      injectedPolisher = null;
      window.removeEventListener('afterprint', cleanup);
    };
    window.addEventListener('afterprint', cleanup);

    const { Previewer } = await import('pagedjs');
    const previewer = new Previewer();
    await previewer.preview(bodyHtml, [{ 'act-preview.css': cssText }], printRoot);
    injectedPolisher = previewer.polisher;

    window.focus();
    window.print();
  }

  async function handlePrint() {
    if (!ready || htmlContent === null) return;
    const printPath = isTauri ? 'printViaSystemBrowser' : 'printViaTopLevel';
    try {
      if (isTauri) {
        await printViaSystemBrowser(htmlContent);
      } else {
        await printViaTopLevel(htmlContent);
      }
    } catch (err) {
      console.error('[PdfPreviewModal] handlePrint failed', printPath, err);
      pushToast('error', 'Не удалось открыть документ для печати');
    }
  }
</script>

<Modal {open} {title} size="pdf-preview" {onClose}>
  <div class="pdf-preview">
    {#if loading}
      <div class="state state-loading" aria-live="polite">
        <Spinner size="md" />
        <p>Готовим документ…</p>
      </div>
    {:else if errorMsg !== null}
      <div class="state state-error">
        <p class="error-heading">Не удалось сгенерировать PDF</p>
        <p class="error-detail">{errorMsg}</p>
      </div>
    {:else if htmlContent !== null}
      {#if paginationStatus === 'degraded'}
        <div class="pdf-page-frame">
          <iframe sandbox="" srcdoc={htmlContent} title="Предпросмотр документа" class="pdf-iframe"
          ></iframe>
        </div>
      {:else}
        <!-- The pagination iframe must mount as soon as srcdoc exists,
             regardless of paginationStatus — Paged.js's bootstrap script
             (running inside srcdoc) is the ONLY thing that can ever move
             paginationStatus off 'pending', via the trackly-pagedjs-progress/
             -done postMessage bridge (attachBridge, keyed on iframeEl !==
             null). Gating this branch behind showLoading previously made the
             iframe unreachable while pending, so the bridge could never fire
             and every preview fell through the 8s timeout into D-02 (see
             debug session print-preview-always-degrades.md). The "pagination
             in progress" UI is now an overlay layered on TOP of the mounted
             iframe (opacity/position, never display:none — that would zero
             out Paged.js's layout measurements) instead of a competing
             top-level branch that excludes the iframe from the DOM. -->
        <div class="pdf-page-frame" bind:clientWidth={frameWidthPx}>
          <div class="pdf-scale-outer" style="height: {naturalHeightPx * scaleFactor}px">
            <div
              class="pdf-scale-inner"
              style="width: 842px; height: {naturalHeightPx}px; transform: scale({scaleFactor}); transform-origin: top center;"
            >
              <iframe
                sandbox="allow-scripts"
                {srcdoc}
                bind:this={iframeEl}
                title="Предпросмотр документа"
                class="pdf-iframe"
                style="height: {naturalHeightPx}px;"
              ></iframe>
            </div>
          </div>
          {#if showLoading}
            <div class="pagination-overlay" aria-live="polite">
              <Spinner size="md" />
              <div style="display:flex;flex-direction:column;gap:var(--tr-space-xs);">
                <p>Готовим документ…</p>
                <p class="progress-detail">
                  {pageProgress === 0 ? 'Разбиваем на страницы…' : `Страница ${pageProgress}…`}
                </p>
              </div>
            </div>
          {/if}
        </div>
      {/if}
    {:else}
      <div class="state state-empty">
        <p>Нет данных для предпросмотра.</p>
      </div>
    {/if}
  </div>

  {#snippet footer()}
    {#if paginationStatus === 'done' && pageTotal !== null}
      <div class="pdf-preview-footer-meta">
        <p class="pdf-preview-page-count">
          {pageTotal}
          {pluralizeRu(pageTotal, ['страница', 'страницы', 'страниц'])}
        </p>
        <p class="pdf-preview-hint">
          Печать использует масштаб 100% и поля по умолчанию — проверьте эти настройки в диалоге
          печати.
        </p>
      </div>
    {/if}
    <Button variant="secondary" onclick={onClose}>Закрыть</Button>
    <Button
      variant="primary"
      onclick={handlePrint}
      disabled={loading ||
        errorMsg !== null ||
        (htmlContent !== null && paginationStatus !== 'done' && paginationStatus !== 'degraded')}
    >
      Печать
    </Button>
  {/snippet}
</Modal>

<style lang="scss">
  .pdf-preview {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 400px;
  }
  /* GAP-16-01 secondary fix: frame the on-screen preview like an A4 page
     instead of stretching the iframe full-width. A4 at 96dpi is about
     794x1123px; the frame scrolls vertically when the doc overflows one
     page, and the outer wrapper centers + scrolls horizontally on narrow
     modal widths. Print output is unaffected: @page in the doc's own
     inline styles governs print sizing, this is purely a screen-preview
     affordance. */
  .pdf-page-frame {
    position: relative;
    flex: 1;
    display: flex;
    justify-content: center;
    overflow: auto;
    background: var(--tr-surface-sunken);
    border-radius: var(--tr-radius-xs);
    padding: var(--tr-space-md) 0;
  }
  /* Sits on top of the already-mounted pagination iframe while Paged.js is
     still running (paginationStatus === 'pending'). Deliberately opaque
     (same background as .pdf-page-frame) so it visually matches the old
     full-state spinner — but this is a sibling overlay, not a conditional
     that removes the iframe from the DOM, so Paged.js's layout measurements
     inside the iframe are never disturbed (no display:none anywhere in this
     stack, see debug session print-preview-always-degrades.md). */
  .pagination-overlay {
    position: absolute;
    inset: 0;
    z-index: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--tr-space-md);
    text-align: center;
    color: var(--tr-text-secondary);
    background: var(--tr-surface-sunken);
  }
  .pdf-iframe {
    /* 794 (A4 @page width, D-01) + 24px horizontal gutter on each side, so
       Paged.js's per-sheet shadow (D-09, drawn on .pagedjs_page inside the
       iframe, see pagedPreviewBootstrap.ts) has room to paint instead of
       clipping at the iframe edge. Must stay in sync with .pdf-scale-inner's
       inline width and .pagedjs_pages's horizontal padding. */
    width: 842px;
    min-width: 842px;
    /* Height is bound inline (see markup above) to naturalHeightPx — the
       SAME value already driving .pdf-scale-inner's height — so the iframe's
       own box grows with the actual paginated content instead of staying
       frozen at the single-page placeholder. min-height keeps that
       placeholder ONLY as the pre-pagination fallback (naturalHeightPx's own
       initial $state is 1123, matching). A static `height: 1123px` here
       previously overflowed by design (page + .pagedjs_pages's vertical
       padding always exceeds 1123px) and produced a scrollbar nested INSIDE
       the iframe on top of .pdf-page-frame's own outer scrollbar — see debug
       session print-preview-always-degrades.md, defect #5 cause A. */
    min-height: 1123px;
    /* No box-shadow here (removed, was var(--tr-elev-2)): D-09 places the
       shadow PER SHEET, on .pagedjs_page inside the iframe (see
       pagedPreviewBootstrap.ts) — an outer shadow on this element would
       outline the entire iframe box (i.e. the whole multi-page stack once
       D-04 pagination is in play), duplicating and conflicting with the
       per-sheet design instead of complementing it. See defect #5 cause C. */
    /* border: none (defects #6 AND #7, print-preview-always-degrades.md):
       this rule never declared its own `border`, so the browser's default UA
       stylesheet rule `iframe:not([seamless]) { border: 2px inset; }`
       (WHATWG html.spec.whatwg.org/multipage/rendering.html) applied instead
       — the harsh dark inset border the user reported. It is also NOT purely
       cosmetic: global.scss's universal `*, *::before, *::after { box-sizing:
       border-box; }` reset applies to this element (the reset is a plain
       document-wide stylesheet, unaffected by Svelte's per-component style
       scoping), so this element's declared `height`/`width` are BORDER-BOX
       sizes — the 2px top + 2px bottom UA-default border was being
       subtracted from the usable content viewport handed to the framed
       document, leaving it ~4px shorter than `naturalHeightPx` and forcing a
       persistent internal scrollbar even though naturalHeightPx (measured
       from .pagedjs_pages.scrollHeight, confirmed by reading
       chunker.js/previewer.js: nothing else is ever appended to the iframe
       document's <body> besides .pagedjs_pages and an inert hidden
       <template>) was itself already fully correct. Removing the border
       makes content-box height === naturalHeightPx exactly, fixing both the
       visual regression and the leftover nested scroll from the same root
       cause. Do NOT set this to a background colour instead (as tentatively
       suggested in UAT) — the iframe is a transparent viewport onto the
       backdrop and D-09's per-sheet shadow already separates sheet from
       backdrop; any border here would just reintroduce a second, redundant
       edge around the whole page stack. */
    border: none;
    /* var(--tr-surface-sunken), not var(--tr-n-0)/white: this background is
       only ever visible for the brief instant before the iframe's own opaque-
       origin document paints its body background (chrome.backdrop, see
       pagedPreviewBootstrap.ts) over the entire viewport. Painting it white
       first caused a flash against the dark-theme backdrop (near-black);
       matching .pdf-page-frame's own backdrop colour here means that flash
       blends into its surroundings instead of standing out. */
    background: var(--tr-surface-sunken);
    flex-shrink: 0;
  }
  .pdf-scale-outer {
    flex-shrink: 0;
  }
  .pdf-scale-inner {
    flex-shrink: 0;
  }
  .progress-detail {
    font-size: 13px;
    font-weight: 500;
    color: var(--tr-text-tertiary);
    margin: 0;
  }
  .pdf-preview-footer-meta {
    flex: 1 1 auto;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-3xs);
    text-align: left;
  }
  .pdf-preview-page-count {
    margin: 0;
    font-size: 13px;
    font-weight: 500;
    color: var(--tr-text-secondary);
  }
  .pdf-preview-hint {
    margin: 0;
    font-size: 12px;
    font-weight: 500;
    color: var(--tr-text-tertiary);
  }
  .state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    flex: 1;
    gap: var(--tr-space-md);
    text-align: center;
    color: var(--tr-text-secondary);
    min-height: 320px;
  }
  .error-heading {
    margin: 0;
    color: var(--tr-danger);
    font-weight: var(--tr-font-weight-semibold);
  }
  .error-detail {
    margin: 0;
    max-width: 480px;
    color: var(--tr-text-secondary);
  }
</style>

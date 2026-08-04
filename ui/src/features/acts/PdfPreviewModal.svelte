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
  import { buildSrcdoc, THEME_CHROME } from '$lib/pdfPreview/pagedPreviewBootstrap';
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
  /** D-11: fit-to-width scale, ceiling of 1 — never enlarges beyond natural size. */
  const scaleFactor = $derived(frameWidthPx > 0 ? Math.min(1, frameWidthPx / 794) : 1);

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
   * Auto-print: inject a small script that calls window.print() AFTER the
   * document fully loads (so the data:image logo is painted before the print
   * dialog appears), with a short setTimeout fallback. This makes the system
   * browser show the print dialog immediately on open. Injected ONLY here in
   * the desktop branch — the LAN path is untouched. Safe: this runs as a
   * standalone file:// document in the user's own browser, outside the app's
   * CSP; the act HTML is server-rendered, not user-authored markup.
   */
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

  /**
   * LAN-browser fix: prints the document from the TOP-LEVEL window instead
   * of the preview iframe's contentWindow (which is unreliable/no-op cross-
   * frame in some webviews — see script-block comment). Extracts the
   * inline style and body markup from the self-contained backend-rendered
   * HTML string, mounts them into a hidden host in the main document, and
   * calls window.print() on the top-level window.
   */
  function printViaTopLevel(html: string) {
    const parsed = new DOMParser().parseFromString(html, 'text/html');
    const bodyHtml = parsed.body?.innerHTML ?? '';
    const styleHtml = Array.from(parsed.head?.querySelectorAll('style') ?? [])
      .map((el) => el.outerHTML)
      .join('\n');

    let printRoot = document.getElementById(PRINT_ROOT_ID);
    if (!printRoot) {
      printRoot = document.createElement('div');
      printRoot.id = PRINT_ROOT_ID;
      document.body.appendChild(printRoot);
    }
    printRoot.innerHTML = bodyHtml;

    let printStyle = document.getElementById(PRINT_STYLE_ID) as HTMLStyleElement | null;
    if (!printStyle) {
      printStyle = document.createElement('style');
      printStyle.id = PRINT_STYLE_ID;
      document.head.appendChild(printStyle);
    }
    // Document's own inline styles (incl. @page) + visibility scoping: hide
    // the rest of the app and show only #print-root while printing. Scoping
    // is `@media print`-only so on-screen app layout is never affected.
    printStyle.textContent = `
      ${styleHtml.replace(/<\/?style[^>]*>/gi, '')}
      @media print {
        body > :not(#${PRINT_ROOT_ID}) {
          display: none !important;
        }
        #${PRINT_ROOT_ID} {
          display: block !important;
        }
      }
      @media screen {
        #${PRINT_ROOT_ID} {
          display: none !important;
        }
      }
    `;

    const cleanup = () => {
      printRoot!.innerHTML = '';
      window.removeEventListener('afterprint', cleanup);
    };
    window.addEventListener('afterprint', cleanup);

    window.focus();
    window.print();
  }

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
</script>

<Modal {open} {title} size="pdf-preview" {onClose}>
  <div class="pdf-preview">
    {#if showLoading}
      <div class="state state-loading" aria-live="polite">
        <Spinner size="md" />
        <div style="display:flex;flex-direction:column;gap:var(--tr-space-xs);">
          <p>Готовим документ…</p>
          <p class="progress-detail">
            {pageProgress === 0 ? 'Разбиваем на страницы…' : `Страница ${pageProgress}…`}
          </p>
        </div>
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
        <div class="pdf-page-frame" bind:clientWidth={frameWidthPx}>
          <div class="pdf-scale-outer" style="height: {naturalHeightPx * scaleFactor}px">
            <div
              class="pdf-scale-inner"
              style="width: 794px; height: {naturalHeightPx}px; transform: scale({scaleFactor}); transform-origin: top center;"
            >
              <iframe
                sandbox="allow-scripts"
                {srcdoc}
                bind:this={iframeEl}
                title="Предпросмотр документа"
                class="pdf-iframe"
              ></iframe>
            </div>
          </div>
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
    flex: 1;
    display: flex;
    justify-content: center;
    overflow: auto;
    background: var(--tr-surface-sunken);
    border-radius: var(--tr-radius-xs);
    padding: var(--tr-space-md) 0;
  }
  .pdf-iframe {
    width: 794px;
    min-width: 794px;
    height: 1123px;
    min-height: 1123px;
    box-shadow: var(--tr-elev-2);
    background: var(--tr-n-0);
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

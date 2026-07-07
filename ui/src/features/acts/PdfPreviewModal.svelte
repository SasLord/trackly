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

  const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

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

  $effect(() => {
    if (!ready) {
      htmlContent = null;
      errorMsg = null;
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
    };
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
        <iframe srcdoc={htmlContent} title="Document Preview" class="pdf-iframe"></iframe>
      </div>
    {:else}
      <div class="state state-empty">
        <p>Нет данных для предпросмотра.</p>
      </div>
    {/if}
  </div>

  {#snippet footer()}
    <Button variant="secondary" onclick={onClose}>Закрыть</Button>
    <Button variant="primary" onclick={handlePrint} disabled={loading || errorMsg !== null}>
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
    background: var(--color-surface);
    border-radius: var(--radius-sm);
    padding: var(--space-md) 0;
  }
  .pdf-iframe {
    width: 794px;
    min-width: 794px;
    height: 1123px;
    min-height: 1123px;
    border: 1px solid var(--color-border);
    box-shadow: var(--shadow-elev-2);
    background: #fff;
    flex-shrink: 0;
  }
  .state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    flex: 1;
    gap: var(--space-md);
    text-align: center;
    color: var(--color-text-secondary);
    min-height: 320px;
  }
  .error-heading {
    margin: 0;
    color: var(--color-destructive);
    font-weight: var(--font-weight-semibold);
  }
  .error-detail {
    margin: 0;
    max-width: 480px;
    color: var(--color-text-secondary);
  }
</style>

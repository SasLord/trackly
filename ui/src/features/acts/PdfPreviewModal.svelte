<script lang="ts">
  // Phase 16: Document print-preview modal.
  //
  // Renders the backend-generated HTML document directly in an <iframe> via
  // `srcdoc` — no blob/object-URL lifecycle, no PDF bytes. Printing (and
  // "Save as PDF") happens through the browser's native print dialog, which
  // works identically in the desktop webview and any LAN browser (D-09).
  //
  // Buttons:
  //   - Print → iframeEl.contentWindow.print() (offers "Save as PDF" natively).
  //   - Закрыть.

  import Button from '$lib/components/Button.svelte';
  import Modal from '$lib/components/Modal.svelte';
  import Spinner from '$lib/components/Spinner.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { acts } from '$lib/api/acts';

  interface AcceptancePayload {
    deviceId: number;
    giverName: string;
    receiverName: string;
    dateUtc: number;
    deviceName?: string;
  }

  interface Props {
    open: boolean;
    actId: number | null;
    title: string;
    onClose: () => void;
    /** Plan 03-05: 'handover' → render акта приёма-передачи (default);
     *  'acceptance' → render документа приёма устройства (DEV-14). */
    mode?: 'handover' | 'acceptance';
    /** Required when mode='acceptance'. */
    acceptancePayload?: AcceptancePayload | null;
  }

  const { open, actId, title, onClose, mode = 'handover', acceptancePayload = null }: Props =
    $props();

  let htmlContent = $state<string | null>(null);
  let loading = $state(false);
  let errorMsg = $state<string | null>(null);
  // eslint-disable-next-line no-undef
  let iframeEl = $state<HTMLIFrameElement | null>(null);

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
    if (actId === null) {
      return Promise.reject(new Error('actId required for mode="handover"'));
    }
    return acts.renderPdf(actId);
  }

  const ready = $derived(
    open && (mode === 'acceptance' ? acceptancePayload !== null : actId !== null),
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

  function handlePrint() {
    if (!ready) return;
    if (iframeEl?.contentWindow) {
      try {
        iframeEl.contentWindow.focus();
        iframeEl.contentWindow.print();
      } catch {
        pushToast('error', 'Не удалось вызвать диалог печати');
      }
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
      <iframe
        bind:this={iframeEl}
        srcdoc={htmlContent}
        title="Document Preview"
        class="pdf-iframe"
      ></iframe>
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
  .pdf-iframe {
    width: 100%;
    flex: 1;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: #fff;
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

<script lang="ts">
  // Phase 3 Plan 04: PDF preview modal.
  //
  // Renders the PDF in an <iframe> via a `blob:` URL — bypasses the
  // pdfjs-dist worker config issue (Pitfall 8 in 03-RESEARCH.md).
  //
  // Buttons:
  //   - Save as PDF → tauri-plugin-dialog save dialog → tauri-plugin-fs writeFile.
  //   - Open in system viewer → write tmp file → tauri-plugin-shell open.
  //   - Print → iframeEl.contentWindow.print().
  //
  // Blob URL lifecycle:
  //   - $effect fetches PDF bytes when (open && actId) changes → creates Blob URL.
  //   - Cleanup function revokes the URL when component unmounts or modal closes.

  import Button from '$lib/components/Button.svelte';
  import Modal from '$lib/components/Modal.svelte';
  import Spinner from '$lib/components/Spinner.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { acts } from '$lib/api/acts';
  import { fetchPdfBlob, revokePdfUrl } from '$lib/api/pdf';

  interface Props {
    open: boolean;
    actId: number | null;
    title: string;
    actNumberDisplay: string | null;
    actDateUtc: number | null;
    onClose: () => void;
  }

  const { open, actId, title, actNumberDisplay, actDateUtc, onClose }: Props = $props();

  let blobUrl = $state<string | null>(null);
  let pdfBytes = $state<number[] | null>(null);
  let loading = $state(false);
  let errorMsg = $state<string | null>(null);
  // eslint-disable-next-line no-undef
  let iframeEl = $state<HTMLIFrameElement | null>(null);

  function isoDateForFilename(unixSeconds: number | null): string {
    if (unixSeconds === null) return 'без-даты';
    const d = new Date(unixSeconds * 1000);
    const y = d.getUTCFullYear();
    const m = String(d.getUTCMonth() + 1).padStart(2, '0');
    const day = String(d.getUTCDate()).padStart(2, '0');
    return `${y}-${m}-${day}`;
  }

  function suggestedFilename(): string {
    const number = actNumberDisplay ?? 'N';
    const date = isoDateForFilename(actDateUtc);
    return `Акт_приёма-передачи_№${number}_${date}.pdf`;
  }

  $effect(() => {
    if (!open || actId === null) {
      // Cleanup on close
      if (blobUrl !== null) {
        revokePdfUrl(blobUrl);
        blobUrl = null;
      }
      pdfBytes = null;
      errorMsg = null;
      return;
    }

    loading = true;
    errorMsg = null;
    let cancelled = false;
    let createdUrl: string | null = null;

    (async () => {
      try {
        const result = await fetchPdfBlob(acts.renderPdf(actId));
        if (cancelled) {
          revokePdfUrl(result.url);
          return;
        }
        createdUrl = result.url;
        blobUrl = result.url;
        // Stash bytes for save/open without re-rendering.
        // (Re-fetch from blob for save/open is awkward; keep raw bytes available.)
        // We can pull them back from the blob via blob.arrayBuffer() in the handlers
        // — but it's simpler to re-call backend if user saves. For now, attach.
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
      if (createdUrl) {
        revokePdfUrl(createdUrl);
        if (blobUrl === createdUrl) blobUrl = null;
      }
    };
  });

  async function handleSave() {
    if (actId === null) return;
    try {
      const { save } = await import('@tauri-apps/plugin-dialog');
      const path = await save({
        defaultPath: suggestedFilename(),
        filters: [{ name: 'PDF', extensions: ['pdf'] }],
      });
      if (!path) return;
      // Re-fetch bytes (simplest reliable path; backend is fast).
      const bytes = pdfBytes ?? (await acts.renderPdf(actId));
      pdfBytes = bytes;
      const { writeFile } = await import('@tauri-apps/plugin-fs');
      await writeFile(path, new Uint8Array(bytes));
      pushToast('success', `PDF сохранён: ${path}`);
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось сохранить PDF';
      pushToast('error', msg);
    }
  }

  async function handleOpen() {
    if (actId === null) return;
    try {
      const bytes = pdfBytes ?? (await acts.renderPdf(actId));
      pdfBytes = bytes;
      // Write to a temp file via tauri-plugin-fs, then open via shell.
      // Phase 3: simplest path — use the user's temp dir via OS plugin.
      const { writeFile, BaseDirectory } = await import('@tauri-apps/plugin-fs');
      const filename = `trackly-preview-${Date.now()}.pdf`;
      await writeFile(filename, new Uint8Array(bytes), {
        baseDir: BaseDirectory.Temp,
      });
      const { tempDir } = await import('@tauri-apps/api/path');
      const tmp = await tempDir();
      const full = `${tmp}${filename}`;
      const { open: openShell } = await import('@tauri-apps/plugin-shell');
      await openShell(full);
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось открыть PDF в системном просмотрщике';
      pushToast('error', msg);
    }
  }

  function handlePrint() {
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
    {:else if blobUrl !== null}
      <iframe bind:this={iframeEl} src={blobUrl} title="PDF Preview" class="pdf-iframe"></iframe>
    {:else}
      <div class="state state-empty">
        <p>Нет данных для предпросмотра.</p>
      </div>
    {/if}
  </div>

  {#snippet footer()}
    <Button variant="secondary" onclick={onClose}>Закрыть</Button>
    <Button variant="secondary" onclick={handleOpen} disabled={loading || errorMsg !== null}>
      Открыть в системном просмотрщике
    </Button>
    <Button variant="secondary" onclick={handlePrint} disabled={loading || errorMsg !== null}>
      Печать
    </Button>
    <Button variant="primary" onclick={handleSave} disabled={loading || errorMsg !== null}>
      Сохранить как PDF
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

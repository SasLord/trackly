<script lang="ts">
  // Фаза 41.3, план 13 (D-05, D-06, D-11, D-13, D-15, D-16, D-17, D-25):
  // единственная реализация секции «Вложения» для всех карточек.
  // Три способа приёма файлов (кнопка, перетаскивание, вставка) сходятся в
  // одну функцию enqueue(File[]) и последовательную очередь загрузки.
  // Имена файлов и людей выводятся ТОЛЬКО интерполяцией текста.
  import { onDestroy, onMount, untrack } from 'svelte';
  import Modal from '$lib/components/Modal.svelte';
  import Button from '$lib/components/Button.svelte';
  import Spinner from '$lib/components/Spinner.svelte';
  import RowChevron from '$lib/components/icons/RowChevron.svelte';
  import { onWsEvent } from '$lib/api/ws';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { authStore } from '$lib/stores/auth.svelte';
  import { saveFile } from '$lib/utils/saveFile';
  import { formatFileSize } from '$lib/utils/formatFileSize';
  import AttachmentTile from './AttachmentTile.svelte';
  import AttachmentViewer from './AttachmentViewer.svelte';
  import { attachments, fetchAttachmentBytes, uploadAttachment } from './api';
  import type { AttachmentDto, AttachmentListDto } from '../../bindings';

  interface Props {
    parentKind: 'request' | 'device' | 'cartridge';
    parentId: number;
    readonly?: boolean;
  }

  const { parentKind, parentId, readonly = false }: Props = $props();

  const ACCEPT =
    '.jpg,.jpeg,.png,.webp,.gif,.heic,.heif,.pdf,.doc,.docx,.xls,.xlsx,.odt,.ods,.txt,.csv,.log,.zip,.7z';

  let list = $state<AttachmentListDto | null>(null);
  let loading = $state(true);
  let loadError = $state(false);
  let loadToken = 0;
  let destroyed = false;

  let sectionEl = $state<HTMLElement | null>(null);
  let fileInput = $state<HTMLInputElement | null>(null);

  const canAttach = $derived(!readonly && (list?.canAttach ?? false));
  const canManageDeleted = $derived(list?.canManageDeleted ?? false);
  const items = $derived(list?.items ?? []);
  const deletedItems = $derived(list?.deleted ?? []);
  const maxBytes = $derived(list?.maxSizeBytes ?? 0);

  // --- загрузка списка -------------------------------------------------

  async function load(silent: boolean) {
    const token = ++loadToken;
    if (!silent) {
      loading = true;
      loadError = false;
    }
    try {
      const res = await attachments.list(parentKind, parentId);
      if (token !== loadToken || destroyed) return;
      list = res;
      loadError = false;
    } catch {
      if (token !== loadToken || destroyed) return;
      if (!silent || list === null) loadError = true;
    } finally {
      if (token === loadToken) loading = false;
    }
  }

  // Смена родителя: сбросить состояние и очередь, загрузить заново.
  $effect(() => {
    void parentKind;
    void parentId;
    untrack(() => {
      queue.length = 0;
      pending = [];
      errors = [];
      batchTotal = 0;
      batchDone = 0;
      announce = '';
      list = null;
      deletedOpen = false;
      viewerOpen = false;
      void load(false);
    });
  });

  // D-15 / D-25: живое обновление по WebSocket.
  onMount(() => {
    const off = onWsEvent((ev) => {
      if (
        ev.type === 'attachments_changed' &&
        ev.parentKind === parentKind &&
        ev.parentId === parentId
      ) {
        void load(true);
      }
    });
    return off;
  });

  onDestroy(() => {
    destroyed = true;
    queue.length = 0;
  });

  // --- приём файлов ----------------------------------------------------

  interface QueueEntry {
    key: number;
    file: globalThis.File;
  }
  let queue: QueueEntry[] = [];
  let processing = false;
  let nextKey = 1;
  let pending = $state<AttachmentDto[]>([]);
  let errors = $state<string[]>([]);
  let batchTotal = $state(0);
  let batchDone = $state(0);
  let announce = $state('');
  let dragging = $state(false);

  function placeholder(key: number, file: globalThis.File): AttachmentDto {
    return {
      id: -key,
      originalName: file.name,
      mime: file.type,
      sizeBytes: file.size,
      hasThumbnail: false,
      viewable: false,
      missing: false,
      uploadedByName: null,
      uploadedById: null,
      createdAtUtc: 0,
      deletedByName: null,
      deletedAtUtc: null,
    };
  }

  function limitMb(): number {
    return Math.floor(maxBytes / (1024 * 1024));
  }

  function rejectLine(name: string, code: string, size: number): string {
    switch (code) {
      case 'too_large':
        return `«${name}» — больше ${limitMb()} МБ (размер файла ${formatFileSize(size)}).`;
      case 'empty':
        return `«${name}» — файл пустой.`;
      case 'type_not_allowed':
        return `«${name}» — такой файл приложить нельзя. Подходят фото (JPEG, PNG, WebP, GIF, HEIC), PDF, документы Word, Excel и OpenDocument, TXT, CSV, LOG, архивы ZIP и 7z.`;
      case 'heic_failed':
        return `«${name}» — не удалось преобразовать фото HEIC. Сохраните его в JPEG и приложите снова.`;
      case 'forbidden':
        return `«${name}» — нет доступа к карточке или она удалена.`;
      default:
        return `«${name}» — не удалось загрузить. Проверьте подключение и попробуйте ещё раз.`;
    }
  }

  function enqueue(files: globalThis.File[], folderNames: string[] = []) {
    if (destroyed) return;
    const lines: string[] = folderNames.map((n) => `«${n}» — это папка. Приложите файлы из неё.`);
    let accepted = 0;
    for (const file of files) {
      if (file.size === 0) {
        lines.push(rejectLine(file.name, 'empty', 0));
        continue;
      }
      if (maxBytes > 0 && file.size > maxBytes) {
        lines.push(rejectLine(file.name, 'too_large', file.size));
        continue;
      }
      const key = nextKey++;
      queue.push({ key, file });
      pending = [...pending, placeholder(key, file)];
      accepted += 1;
    }
    if (lines.length > 0) errors = [...errors, ...lines];
    if (accepted === 0) return;
    if (!processing) {
      batchTotal = 0;
      batchDone = 0;
    }
    batchTotal += accepted;
    announce = `Загружается файлов: ${batchTotal}`;
    if (!processing) void processQueue();
  }

  async function processQueue() {
    processing = true;
    try {
      while (queue.length > 0 && !destroyed) {
        const entry = queue.shift()!;
        const res = await uploadAttachment(parentKind, parentId, entry.file);
        pending = pending.filter((p) => p.id !== -entry.key);
        if (res.rejectCode !== null || res.attachment === null) {
          errors = [
            ...errors,
            rejectLine(entry.file.name, res.rejectCode ?? 'network', entry.file.size),
          ];
        } else {
          batchDone += 1;
          if (!destroyed) await load(true);
        }
      }
    } finally {
      processing = false;
      if (!destroyed && batchTotal > 0) {
        announce = `Загружено файлов: ${batchDone} из ${batchTotal}`;
        await load(true);
      }
    }
  }

  // Правило «выше хоста нет Modal»: блок в странице (хоста нет, глубина -1) или
  // в Modal. Каждый Modal несёт глубину своей позиции в openStack в атрибуте
  // data-modal-depth; приём файлов разрешён, только если самый верхний открытый
  // Modal и есть хост блока (просмотрщик или подтверждение поверх него — нет).
  function intakeAllowed(): boolean {
    if (!canAttach || !sectionEl) return false;
    const host = sectionEl.closest('[data-modal-depth]');
    const hostDepth = host ? Number(host.getAttribute('data-modal-depth')) : -1;
    let topDepth = -1;
    document.querySelectorAll('[data-modal-depth]').forEach((el) => {
      topDepth = Math.max(topDepth, Number(el.getAttribute('data-modal-depth')));
    });
    return hostDepth === topDepth;
  }

  function pad(n: number): string {
    return String(n).padStart(2, '0');
  }

  function screenshotName(ext: string): string {
    const d = new Date();
    return `Скриншот ${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}-${pad(d.getMinutes())}-${pad(d.getSeconds())}.${ext}`;
  }

  function handlePaste(e: globalThis.ClipboardEvent) {
    const target = e.target as HTMLElement | null;
    if (target) {
      if (target.closest('input, textarea, [contenteditable]') || target.isContentEditable) {
        return;
      }
    }
    if (!intakeAllowed()) return;
    const raw: globalThis.File[] = [];
    for (const it of Array.from(e.clipboardData?.items ?? [])) {
      if (it.kind !== 'file') continue;
      const f = it.getAsFile();
      if (f) raw.push(f);
    }
    if (raw.length === 0) return;
    e.preventDefault();
    const files = raw.map((f) => {
      if (f.type.startsWith('image/') && (f.name === '' || /^image\.[a-z0-9]+$/i.test(f.name))) {
        const ext = f.type.split('/')[1]?.replace('jpeg', 'jpg') || 'png';
        return new globalThis.File([f], screenshotName(ext), { type: f.type });
      }
      return f;
    });
    enqueue(files);
  }

  function hasFiles(e: globalThis.DragEvent): boolean {
    return Array.from(e.dataTransfer?.types ?? []).includes('Files');
  }

  function handleDragOver(e: globalThis.DragEvent) {
    if (!hasFiles(e) || !intakeAllowed()) return;
    e.preventDefault();
    dragging = true;
  }

  function handleDragLeave(e: globalThis.DragEvent) {
    const next = e.relatedTarget as Node | null;
    if (next && sectionEl?.contains(next)) return;
    dragging = false;
  }

  function handleDrop(e: globalThis.DragEvent) {
    if (!hasFiles(e)) return;
    e.preventDefault();
    dragging = false;
    if (!intakeAllowed()) return;
    const files: globalThis.File[] = [];
    const folders: string[] = [];
    for (const it of Array.from(e.dataTransfer?.items ?? [])) {
      if (it.kind !== 'file') continue;
      const entry = typeof it.webkitGetAsEntry === 'function' ? it.webkitGetAsEntry() : null;
      if (entry?.isDirectory) {
        folders.push(entry.name);
        continue;
      }
      const f = it.getAsFile();
      if (f) files.push(f);
    }
    enqueue(files, folders);
  }

  function handlePicked() {
    const input = fileInput;
    if (!input) return;
    const files = Array.from(input.files ?? []);
    input.value = '';
    if (files.length > 0 && canAttach) enqueue(files);
  }

  // --- просмотр и скачивание -------------------------------------------

  let viewerOpen = $state(false);
  let viewerStartId = $state<number | null>(null);
  let viewerGroup = $state<'active' | 'deleted'>('active');

  const viewerItems = $derived(
    (viewerGroup === 'deleted' ? deletedItems : items).filter((i) => i.viewable && !i.missing),
  );

  function openViewer(item: AttachmentDto, group: 'active' | 'deleted') {
    viewerGroup = group;
    viewerStartId = item.id;
    viewerOpen = true;
  }

  async function download(item: AttachmentDto) {
    try {
      const buf = await fetchAttachmentBytes(item.id);
      await saveFile(new Uint8Array(buf), item.originalName, item.mime);
    } catch {
      pushToast('error', 'Не удалось скачать файл. Попробуйте ещё раз.');
    }
  }

  // --- удаление / восстановление ---------------------------------------

  let confirmDelete = $state<AttachmentDto | null>(null);
  let confirmPurge = $state<AttachmentDto | null>(null);
  let busy = $state(false);
  let deletedOpen = $state(false);

  function canDeleteItem(item: AttachmentDto): boolean {
    if (readonly || !list) return false;
    if (list.canDeleteAny) return true;
    return item.uploadedById !== null && item.uploadedById === authStore.user?.id;
  }

  async function doDelete() {
    const item = confirmDelete;
    if (!item || busy) return;
    busy = true;
    try {
      await attachments.remove(item.id);
      confirmDelete = null;
      pushToast('success', `Вложение «${item.originalName}» удалено`);
      await load(true);
    } catch {
      pushToast('error', 'Не удалось удалить вложение. Попробуйте ещё раз.');
    } finally {
      busy = false;
    }
  }

  async function doRestore(item: AttachmentDto) {
    try {
      await attachments.restore(item.id);
      pushToast('success', `Вложение «${item.originalName}» восстановлено`);
      await load(true);
    } catch {
      pushToast('error', 'Не удалось восстановить вложение. Попробуйте ещё раз.');
    }
  }

  async function doPurge() {
    const item = confirmPurge;
    if (!item || busy) return;
    busy = true;
    try {
      await attachments.purge(item.id);
      confirmPurge = null;
      pushToast('success', `Вложение «${item.originalName}» удалено навсегда`);
      await load(true);
    } catch {
      pushToast('error', 'Не удалось удалить вложение навсегда. Попробуйте ещё раз.');
    } finally {
      busy = false;
    }
  }

  const headingId = `attachments-heading-${Math.random().toString(36).slice(2)}`;
</script>

<svelte:window onpaste={canAttach ? handlePaste : undefined} />

<section
  class="attachments"
  class:dragging
  aria-labelledby={headingId}
  bind:this={sectionEl}
  ondragenter={canAttach ? handleDragOver : undefined}
  ondragover={canAttach ? handleDragOver : undefined}
  ondragleave={canAttach ? handleDragLeave : undefined}
  ondrop={canAttach ? handleDrop : undefined}
>
  <div class="section-heading-row">
    <h3 class="section-heading" id={headingId}>
      Вложения{items.length > 0 ? ` (${items.length})` : ''}
    </h3>
    {#if canAttach}
      <Button variant="secondary" size="sm" onclick={() => fileInput?.click()}>
        Приложить файлы
      </Button>
    {/if}
  </div>

  {#if canAttach}
    <input
      bind:this={fileInput}
      class="hidden-input"
      type="file"
      multiple
      accept={ACCEPT}
      tabindex="-1"
      aria-hidden="true"
      onchange={handlePicked}
    />
  {/if}

  {#if dragging}
    <div class="drop-hint" aria-hidden="true">Отпустите, чтобы приложить</div>
  {/if}

  {#if loading && list === null}
    <div class="loading"><Spinner size="sm" /></div>
  {:else if loadError && list === null}
    <div class="load-error">
      <span>Не удалось загрузить вложения.</span>
      <Button variant="link" size="sm" onclick={() => void load(false)}>Повторить</Button>
    </div>
  {:else if items.length === 0 && pending.length === 0}
    {#if canAttach}
      <div class="empty-zone">
        <div class="empty-title">Файлов пока нет</div>
        <div class="empty-hint">
          Перетащите файлы сюда, вставьте скриншот из буфера (Ctrl+V) или нажмите «Приложить файлы».
        </div>
      </div>
    {:else}
      <p class="empty-plain">Файлов нет</p>
    {/if}
  {:else}
    <ul class="attachments-grid">
      {#each items as item (item.id)}
        <AttachmentTile
          {item}
          {readonly}
          canDelete={canDeleteItem(item)}
          onOpen={(i) => openViewer(i, 'active')}
          onDownload={download}
          onDelete={(i) => (confirmDelete = i)}
        />
      {/each}
      {#each pending as item (item.id)}
        <AttachmentTile {item} state="uploading" />
      {/each}
    </ul>
  {/if}

  {#if canManageDeleted && deletedItems.length > 0}
    <div class="deleted-group">
      <div class="deleted-toggle">
        <RowChevron
          expanded={deletedOpen}
          label={`Удалённые (${deletedItems.length})`}
          onclick={() => (deletedOpen = !deletedOpen)}
        />
        <button
          type="button"
          class="deleted-toggle-text"
          tabindex="-1"
          aria-hidden="true"
          onclick={() => (deletedOpen = !deletedOpen)}
        >
          Удалённые ({deletedItems.length})
        </button>
      </div>
      {#if deletedOpen}
        <ul class="attachments-grid">
          {#each deletedItems as item (item.id)}
            <AttachmentTile
              {item}
              state="deleted"
              {readonly}
              {canManageDeleted}
              onOpen={(i) => openViewer(i, 'deleted')}
              onDownload={download}
              onRestore={doRestore}
              onPurge={(i) => (confirmPurge = i)}
            />
          {/each}
        </ul>
      {/if}
    </div>
  {/if}

  {#if errors.length > 0}
    <div class="error-list" role="alert">
      <div class="error-head">
        <span class="error-title">Не приложено файлов: {errors.length}</span>
        <Button variant="link" size="sm" onclick={() => (errors = [])}>Скрыть</Button>
      </div>
      <ul>
        {#each errors as line, i (i)}
          <li title={line}>{line}</li>
        {/each}
      </ul>
    </div>
  {/if}

  <div class="sr-only" aria-live="polite">{announce}</div>
</section>

<AttachmentViewer
  items={viewerItems}
  startId={viewerStartId}
  open={viewerOpen}
  onClose={() => (viewerOpen = false)}
/>

{#if confirmDelete}
  <Modal
    open={true}
    title="Удалить вложение?"
    size="md"
    onClose={() => (confirmDelete = null)}
    initialFocus=".modal-footer .btn-secondary"
  >
    <p class="confirm-text">
      {#if canManageDeleted}
        Файл «{confirmDelete.originalName}» переместится в «Удалённые» этой карточки. Его можно
        будет восстановить или удалить навсегда.
      {:else}
        Файл «{confirmDelete.originalName}» исчезнет из карточки. Вернуть его сможет только
        администратор.
      {/if}
    </p>
    {#snippet footer()}
      <Button variant="secondary" onclick={() => (confirmDelete = null)}>Отмена</Button>
      <Button variant="destructive" loading={busy} onclick={() => void doDelete()}>Удалить</Button>
    {/snippet}
  </Modal>
{/if}

{#if confirmPurge}
  <Modal
    open={true}
    title="Удалить навсегда?"
    size="md"
    onClose={() => (confirmPurge = null)}
    initialFocus=".modal-footer .btn-secondary"
  >
    <p class="confirm-text">
      Вложение «{confirmPurge.originalName}» будет удалено безвозвратно. Восстановить его из
      интерфейса будет нельзя.
    </p>
    {#snippet footer()}
      <Button variant="secondary" onclick={() => (confirmPurge = null)}>Отмена</Button>
      <Button variant="destructive" loading={busy} onclick={() => void doPurge()}>
        Удалить навсегда
      </Button>
    {/snippet}
  </Modal>
{/if}

<style lang="scss">
  .attachments {
    position: relative;
    margin-bottom: var(--tr-space-2xl);
    border: 2px dashed transparent;
    border-radius: var(--tr-radius-md);
  }
  .attachments.dragging {
    border-color: var(--tr-accent);
    background: var(--tr-accent-soft);
  }
  .section-heading-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--tr-space-xs);
    margin-bottom: var(--tr-space-md);
  }
  .section-heading {
    margin: 0;
    font-size: var(--tr-font-size-body);
    font-weight: var(--tr-font-weight-semibold);
    color: var(--tr-text-primary);
  }
  .hidden-input {
    display: none;
  }
  .drop-hint {
    position: absolute;
    inset: 0;
    z-index: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    pointer-events: none;
    font: var(--tr-text-body-strong);
    color: var(--tr-accent-text);
  }
  .loading {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 96px;
  }
  .load-error {
    display: flex;
    align-items: center;
    gap: var(--tr-space-sm);
    font: var(--tr-text-body);
    color: var(--tr-text-secondary);
  }
  .empty-zone {
    padding: 24px var(--tr-space-lg);
    border: 2px dashed var(--tr-border-strong);
    border-radius: var(--tr-radius-md);
    text-align: center;
  }
  .empty-title {
    font: var(--tr-text-body);
    color: var(--tr-text-primary);
  }
  .empty-hint {
    margin-top: var(--tr-space-xs);
    font: var(--tr-text-label);
    color: var(--tr-text-secondary);
  }
  .empty-plain {
    margin: 0;
    font: var(--tr-text-body);
    color: var(--tr-text-tertiary);
  }
  .attachments-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(136px, 1fr));
    gap: var(--tr-space-sm);
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .deleted-group {
    margin-top: var(--tr-space-lg);
  }
  .deleted-toggle {
    display: flex;
    align-items: center;
    gap: var(--tr-space-xs);
    margin-bottom: var(--tr-space-sm);
  }
  .deleted-toggle-text {
    padding: 0;
    border: 0;
    background: transparent;
    cursor: pointer;
    font: var(--tr-text-label);
    color: var(--tr-text-secondary);
  }
  .error-list {
    margin-top: var(--tr-space-md);
  }
  .error-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--tr-space-sm);
  }
  .error-title {
    font: var(--tr-text-label);
    color: var(--tr-danger-text);
  }
  .error-list ul {
    margin: var(--tr-space-xs) 0 0;
    padding: 0;
    list-style: none;
  }
  .error-list li {
    font: var(--tr-text-label);
    color: var(--tr-danger-text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .confirm-text {
    margin: 0;
    font: var(--tr-text-body);
    color: var(--tr-text-primary);
    white-space: normal;
    text-align: left;
  }
  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    margin: -1px;
    padding: 0;
    overflow: hidden;
    clip: rect(0 0 0 0);
    white-space: nowrap;
    border: 0;
  }
</style>

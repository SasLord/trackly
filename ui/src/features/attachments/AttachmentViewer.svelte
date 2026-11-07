<script lang="ts">
  // Фаза 41.3, план 12 (D-12, D-13): попап просмотра фото и PDF.
  // Рендерится блоком вложений как СОСЕДНИЙ Modal верхнего уровня (не внутри
  // чужого Modal-контента). Тосты не выводит: все состояния — внутри тела.
  import { untrack } from 'svelte';
  import Modal from '$lib/components/Modal.svelte';
  import Button from '$lib/components/Button.svelte';
  import Spinner from '$lib/components/Spinner.svelte';
  import { saveFile } from '$lib/utils/saveFile';
  import { formatFileSize } from '$lib/utils/formatFileSize';
  import { formatDateTimeShort } from '$lib/utils/date';
  import { attachmentUrl, fetchAttachmentBytes } from './api';
  import type { AttachmentDto } from '../../bindings';

  interface Props {
    /** Навигационный набор: просматриваемые непотерянные вложения родителя. */
    items: AttachmentDto[];
    startId: number | null;
    open: boolean;
    onClose: () => void;
  }

  const { items, startId, open, onClose }: Props = $props();

  type Status = 'loading' | 'ready' | 'missing' | 'error';

  let currentId = $state<number | null>(null);
  // Снимок последнего известного вложения: нужен, чтобы после исчезновения
  // записи из набора (D-15) показать «Вложение удалено.» с прежним именем.
  let snapshot = $state<AttachmentDto | null>(null);
  let lastIndex = $state(0);
  let status = $state<Status>('loading');
  let imgLoaded = $state(false);
  let zoomed = $state(false);
  let canZoom = $state(false);
  let retryTick = $state(0);
  let downloading = $state(false);
  let probeToken = 0;

  const index = $derived(items.findIndex((i) => i.id === currentId));
  const live = $derived(index >= 0 ? items[index] : null);
  const removed = $derived(open && currentId !== null && live === null);
  const current = $derived(live ?? snapshot);
  const isImage = $derived(current?.mime.toLowerCase().startsWith('image/') ?? false);
  const isPdf = $derived(current?.mime.toLowerCase() === 'application/pdf');
  const showNav = $derived(items.length > 1 || removed);
  const counter = $derived(items.length > 1 && index >= 0 ? `${index + 1} из ${items.length}` : '');
  // Соседи: при удалённом текущем «следующий» занимает его место в наборе.
  const prevItem = $derived(removed ? (items[lastIndex - 1] ?? null) : (items[index - 1] ?? null));
  const nextItem = $derived(removed ? (items[lastIndex] ?? null) : (items[index + 1] ?? null));
  const spinnerVisible = $derived(
    !removed && (status === 'loading' || (status === 'ready' && isImage && !imgLoaded)),
  );
  const canDownload = $derived(!removed && status !== 'missing' && status !== 'loading');
  const fileUrl = $derived(current ? attachmentUrl(current.id, 'file') : '');
  const metaLine = $derived(
    current
      ? [
          formatFileSize(current.sizeBytes),
          current.uploadedByName ?? 'Пользователь удалён',
          formatDateTimeShort(current.createdAtUtc),
        ].join(' · ')
      : '',
  );
  const deletedLine = $derived(
    live && live.deletedAtUtc !== null
      ? `Удалено · ${live.deletedByName ?? 'Пользователь удалён'} · ${formatDateTimeShort(live.deletedAtUtc)}`
      : '',
  );

  // Открытие: встать на стартовое вложение.
  $effect(() => {
    if (!open) return;
    const sid = startId;
    untrack(() => {
      currentId = sid;
    });
  });

  // Снимок и индекс обновляются, пока вложение присутствует в наборе.
  $effect(() => {
    if (live) {
      const found = live;
      const at = index;
      untrack(() => {
        snapshot = found;
        lastIndex = at;
      });
    }
  });

  // Загрузка: проба статуса (404 = потерян), затем прямой URL в img/iframe.
  $effect(() => {
    if (!open || live === null) return;
    const id = live.id;
    const flaggedMissing = live.missing;
    void retryTick;
    untrack(() => {
      imgLoaded = false;
      zoomed = false;
      canZoom = false;
      if (flaggedMissing) {
        status = 'missing';
        return;
      }
      status = 'loading';
      const token = ++probeToken;
      void probe(id, token);
    });
  });

  async function probe(id: number, token: number) {
    try {
      const res = await fetch(attachmentUrl(id, 'file'), {
        credentials: 'same-origin',
        headers: { Range: 'bytes=0-0' },
      });
      void res.body?.cancel().catch(() => undefined);
      if (token !== probeToken) return;
      if (res.status === 404) {
        status = 'missing';
      } else if (res.ok) {
        status = 'ready';
      } else {
        status = 'error';
      }
    } catch {
      if (token === probeToken) status = 'error';
    }
  }

  function go(target: AttachmentDto | null) {
    if (!target) return;
    currentId = target.id;
  }

  function handleKeydown(e: KeyboardEvent) {
    if (!open || e.defaultPrevented) return;
    if (e.key !== 'ArrowLeft' && e.key !== 'ArrowRight') return;
    const active = document.activeElement;
    if (active instanceof HTMLIFrameElement) return;
    e.preventDefault();
    go(e.key === 'ArrowLeft' ? prevItem : nextItem);
  }

  function onImgLoad(e: Event) {
    const img = e.currentTarget as globalThis.HTMLImageElement;
    imgLoaded = true;
    canZoom = img.naturalWidth > img.clientWidth || img.naturalHeight > img.clientHeight;
  }

  async function download() {
    if (!current || !canDownload || downloading) return;
    downloading = true;
    try {
      const buf = await fetchAttachmentBytes(current.id);
      await saveFile(new Uint8Array(buf), current.originalName, current.mime);
    } catch {
      // Без тоста (D-13): состояние ошибки показывает тело при повторном открытии.
    } finally {
      downloading = false;
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<Modal
  {open}
  title={current?.originalName ?? 'Вложение'}
  size="pdf-preview"
  {onClose}
  initialFocus="[data-att-download] button"
>
  {#snippet titleExtra()}
    {#if counter}
      <span class="att-counter">{counter}</span>
    {/if}
  {/snippet}

  <div class="viewer">
    {#if showNav}
      <div class="nav nav-prev">
        <Button
          variant="ghost"
          iconOnly
          ariaLabel="Предыдущее вложение"
          disabled={prevItem === null}
          onclick={() => go(prevItem)}
        >
          ‹
        </Button>
      </div>
      <div class="nav nav-next">
        <Button
          variant="ghost"
          iconOnly
          ariaLabel="Следующее вложение"
          disabled={nextItem === null}
          onclick={() => go(nextItem)}
        >
          ›
        </Button>
      </div>
    {/if}

    {#if deletedLine}
      <p class="deleted-strip">{deletedLine}</p>
    {/if}

    <div class="stage" class:stage-scroll={zoomed}>
      {#if removed}
        <p class="state-text">Вложение удалено.</p>
      {:else if status === 'missing'}
        <div class="state-block">
          <p class="state-title">Файл не найден</p>
          <p class="state-text">
            Файла «{current?.originalName ?? ''}» нет в каталоге вложений. Возможно, каталог
            перенесли без него или файл удалили вручную. Сообщите администратору — файл можно
            вернуть из резервной копии.
          </p>
        </div>
      {:else if status === 'error'}
        <div class="state-block">
          <p class="state-text">Не удалось открыть файл.</p>
          <Button variant="secondary" size="sm" onclick={() => retryTick++}>Повторить</Button>
        </div>
      {/if}

      {#if spinnerVisible}
        <div class="state-block loading">
          <Spinner size="md" />
          <span class="state-text">Загружаем файл…</span>
        </div>
      {/if}

      {#if !removed && status === 'ready' && current}
        {#if isImage}
          <button
            type="button"
            class="zoom-btn"
            class:hidden={!imgLoaded}
            class:zoom-in={canZoom && !zoomed}
            class:zoom-out={canZoom && zoomed}
            aria-label={zoomed ? 'Уменьшить' : 'Увеличить'}
            onclick={() => {
              if (canZoom) zoomed = !zoomed;
            }}
          >
            <img
              class="photo"
              class:photo-full={zoomed}
              src={fileUrl}
              alt={current.originalName}
              onload={onImgLoad}
              onerror={() => (status = 'error')}
            />
          </button>
        {:else if isPdf}
          <iframe class="pdf" src={fileUrl} title={current.originalName}></iframe>
        {/if}
      {/if}
    </div>
  </div>

  {#snippet footer()}
    <span class="meta">{metaLine}</span>
    <Button variant="secondary" onclick={onClose}>Закрыть</Button>
    <span data-att-download>
      <Button variant="primary" disabled={!canDownload} loading={downloading} onclick={download}>
        Скачать
      </Button>
    </span>
  {/snippet}
</Modal>

<style lang="scss">
  .att-counter {
    font-size: var(--tr-font-size-caption);
    color: var(--tr-text-secondary);
    white-space: nowrap;
  }

  .viewer {
    position: relative;
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }

  .nav {
    position: absolute;
    top: 50%;
    transform: translateY(-50%);
    z-index: 1;
    width: var(--touch-target-min);
    height: var(--touch-target-min);

    &.nav-prev {
      left: 0;
    }
    &.nav-next {
      right: 0;
    }
  }

  .deleted-strip {
    margin: 0 0 var(--tr-space-xs);
    font-size: var(--tr-font-size-caption);
    color: var(--tr-text-secondary);
    text-align: center;
  }

  .stage {
    flex: 1;
    min-height: 70vh;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-direction: column;
    background: var(--tr-surface-sunken);
    border-radius: var(--tr-radius-sm);
    overflow: hidden;
    text-align: center;

    &.stage-scroll {
      overflow: auto;
      align-items: flex-start;
      justify-content: flex-start;
    }
  }

  .state-block {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--tr-space-xs);
    padding: var(--tr-space-xl);
    max-width: 560px;

    &.loading {
      flex-direction: row;
    }
  }

  .state-title {
    margin: 0;
    font-size: var(--tr-font-size-body-strong);
    font-weight: var(--tr-font-weight-semibold);
    color: var(--tr-text-primary);
  }

  .state-text {
    margin: 0;
    font-size: var(--tr-font-size-body);
    color: var(--tr-text-secondary);
    white-space: normal;
  }

  .zoom-btn {
    display: block;
    border: none;
    padding: 0;
    margin: 0;
    background: transparent;
    cursor: default;
    max-width: 100%;

    &.zoom-in {
      cursor: zoom-in;
    }
    &.zoom-out {
      cursor: zoom-out;
      max-width: none;
    }
    &.hidden {
      display: none;
    }
  }

  .photo {
    display: block;
    max-width: 100%;
    max-height: 70vh;
    margin: 0 auto;

    &.photo-full {
      max-width: none;
      max-height: none;
    }
  }

  .pdf {
    width: 100%;
    height: 75vh;
    border: none;
    background: var(--tr-surface);
  }

  .meta {
    margin-right: auto;
    align-self: center;
    font-size: var(--tr-font-size-caption);
    color: var(--tr-text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }
</style>

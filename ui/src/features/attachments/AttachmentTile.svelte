<script lang="ts">
  // Фаза 41.3, план 13 (D-11, D-13, D-16, D-17): плитка вложения.
  // Одна кнопка (превью + подпись) выполняет основное действие, ActionMenu —
  // соседний элемент поверх угла превью (вложенных кнопок нет).
  // Имена выводятся только интерполяцией текста.
  import ActionMenu from '$lib/components/ActionMenu.svelte';
  import Badge from '$lib/components/Badge.svelte';
  import Spinner from '$lib/components/Spinner.svelte';
  import { formatFileSize } from '$lib/utils/formatFileSize';
  import { formatDateTimeShort } from '$lib/utils/date';
  import AttachmentTypeIcon, { iconKindForMime } from './AttachmentTypeIcon.svelte';
  import { attachmentUrl } from './api';
  import type { AttachmentDto } from '../../bindings';

  interface Props {
    item: AttachmentDto;
    state?: 'normal' | 'uploading' | 'deleted';
    canDelete?: boolean;
    readonly?: boolean;
    canManageDeleted?: boolean;
    onOpen?: (_item: AttachmentDto) => void;
    onDownload?: (_item: AttachmentDto) => void;
    onDelete?: (_item: AttachmentDto) => void;
    onRestore?: (_item: AttachmentDto) => void;
    onPurge?: (_item: AttachmentDto) => void;
  }

  const {
    item,
    state: tileState = 'normal',
    canDelete = false,
    readonly = false,
    canManageDeleted = false,
    onOpen,
    onDownload,
    onDelete,
    onRestore,
    onPurge,
  }: Props = $props();

  let thumbFailed = $state(false);

  const missing = $derived(item.missing && tileState !== 'uploading');
  const ext = $derived(
    item.originalName.includes('.') ? item.originalName.split('.').pop()!.toUpperCase() : '',
  );
  const showThumb = $derived(
    item.hasThumbnail && !thumbFailed && !missing && tileState !== 'uploading',
  );
  const iconKind = $derived(iconKindForMime(item.mime, item.originalName));
  const who = $derived(item.uploadedByName ?? 'Пользователь удалён');
  const whoWhen = $derived(`${who} · ${formatDateTimeShort(item.createdAtUtc)}`);
  const deletedLine = $derived(
    item.deletedAtUtc !== null
      ? `Удалил: ${item.deletedByName ?? 'Пользователь удалён'} · ${formatDateTimeShort(item.deletedAtUtc)}`
      : '',
  );
  const primary = $derived<'open' | 'download'>(missing || item.viewable ? 'open' : 'download');
  const ariaLabel = $derived(
    missing
      ? `Файл не найден: «${item.originalName}»`
      : primary === 'open'
        ? `Открыть «${item.originalName}»`
        : `Скачать «${item.originalName}»`,
  );

  function activate() {
    if (tileState === 'uploading') return;
    if (primary === 'open') onOpen?.(item);
    else onDownload?.(item);
  }
</script>

<li class="tile" class:tile-missing={missing}>
  {#if tileState === 'uploading'}
    <div class="tile-body" aria-busy="true">
      <div class="preview preview-sunken">
        <Spinner size="sm" />
      </div>
      <div class="caption">
        <span class="name" title={item.originalName}>{item.originalName}</span>
        <span class="meta meta-muted">Загрузка…</span>
      </div>
    </div>
  {:else}
    <button type="button" class="tile-body tile-button" aria-label={ariaLabel} onclick={activate}>
      <span class="preview" class:preview-sunken={!showThumb}>
        {#if showThumb}
          <img
            src={attachmentUrl(item.id, 'thumb')}
            alt=""
            loading="lazy"
            onerror={() => (thumbFailed = true)}
          />
        {:else if missing}
          <span class="missing-label">Файл не найден</span>
        {:else}
          <AttachmentTypeIcon kind={iconKind} />
          {#if ext}
            <span class="ext">{ext}</span>
          {/if}
        {/if}
        {#if tileState === 'deleted'}
          <span class="deleted-badge"><Badge variant="default" size="sm">Удалено</Badge></span>
        {/if}
      </span>
      <span class="caption">
        <span class="name" title={item.originalName}>{item.originalName}</span>
        <span class="meta">{formatFileSize(item.sizeBytes)}</span>
        {#if tileState === 'deleted'}
          <span class="meta" title={deletedLine}>{deletedLine}</span>
        {:else}
          <span class="meta" title={whoWhen}>{whoWhen}</span>
        {/if}
      </span>
    </button>
    <div class="tile-menu">
      <ActionMenu variant="ghost-sm" label="Действия с файлом">
        {#if item.viewable || missing}
          <button type="button" role="menuitem" onclick={() => onOpen?.(item)}>Открыть</button>
        {/if}
        <button type="button" role="menuitem" disabled={missing} onclick={() => onDownload?.(item)}>
          Скачать
        </button>
        {#if tileState === 'deleted' && canManageDeleted && !readonly}
          <!-- svelte-ignore a11y_no_redundant_roles -->
          <hr role="separator" />
          <button type="button" role="menuitem" onclick={() => onRestore?.(item)}>
            Восстановить
          </button>
          <button type="button" role="menuitem" class="menu-danger" onclick={() => onPurge?.(item)}>
            Удалить навсегда
          </button>
        {:else if tileState === 'normal' && canDelete && !readonly}
          <!-- svelte-ignore a11y_no_redundant_roles -->
          <hr role="separator" />
          <button
            type="button"
            role="menuitem"
            class="menu-danger"
            onclick={() => onDelete?.(item)}
          >
            Удалить
          </button>
        {/if}
      </ActionMenu>
    </div>
  {/if}
</li>

<style lang="scss">
  .tile {
    position: relative;
    list-style: none;
    min-width: 0;
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-md);
    background: var(--tr-surface);
    overflow: hidden;
    &:hover {
      border-color: var(--tr-border-strong);
    }
  }
  .tile-body {
    display: flex;
    flex-direction: column;
    width: 100%;
    min-width: 0;
    margin: 0;
    padding: 0;
    border: 0;
    background: transparent;
    text-align: left;
    white-space: normal;
    color: inherit;
    font: inherit;
  }
  .tile-button {
    cursor: pointer;
  }
  .preview {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--tr-space-xs);
    height: 96px;
    background: var(--tr-bg);
    color: var(--tr-text-tertiary);
    overflow: hidden;
    img {
      width: 100%;
      height: 100%;
      object-fit: cover;
    }
  }
  .preview-sunken {
    background: var(--tr-surface-sunken);
  }
  .ext {
    font: var(--tr-text-caption);
    color: var(--tr-text-secondary);
  }
  .missing-label {
    font: var(--tr-text-caption);
    color: var(--tr-warning-muted-text);
  }
  .deleted-badge {
    position: absolute;
    top: var(--tr-space-xs);
    left: var(--tr-space-xs);
  }
  .caption {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    padding: var(--tr-space-sm);
  }
  .name {
    font: var(--tr-text-label);
    color: var(--tr-text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .meta {
    font: var(--tr-text-caption);
    color: var(--tr-text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .meta-muted {
    color: var(--tr-text-tertiary);
  }
  .tile-missing .name {
    color: var(--tr-warning-muted-text);
  }
  .tile-menu {
    position: absolute;
    top: var(--tr-space-xs);
    right: var(--tr-space-xs);
    :global(.action-menu-trigger) {
      background: var(--tr-surface-raised);
      border: 1px solid var(--tr-border);
    }
  }
</style>

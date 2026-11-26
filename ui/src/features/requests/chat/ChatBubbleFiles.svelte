<script lang="ts">
  // Файлы внутри пузыря сообщения (D-15, D-22; UI-SPEC §7.3).
  // Фото — миниатюры, остальное — плитки 48px. AttachmentViewer здесь НЕ рендерится:
  // один экземпляр на RequestChat, клики уходят наверх через onOpen / onDownload.
  import { authStore } from '$lib/stores/auth.svelte';
  import { formatFileSize } from '@trackly/kit/utils/formatFileSize';
  import AttachmentTypeIcon, { iconKindForMime } from '@trackly/attachments/AttachmentTypeIcon.svelte';
  import { attachmentUrl } from '@trackly/attachments/api';
  import type { AttachmentDto } from '../../../bindings';

  interface Props {
    attachments: AttachmentDto[];
    own: boolean;
    onOpen: (_attachment: AttachmentDto) => void;
    onDownload: (_attachment: AttachmentDto) => void;
  }

  const { attachments, own, onOpen, onDownload }: Props = $props();

  const isAdmin = $derived(authStore.user?.role === 'admin');

  // Мягко удалённый файл не-Admin видит как «Файл удалён» (D-22, 41.3 D-16).
  function gone(a: AttachmentDto): boolean {
    return a.deletedAtUtc !== null && !isAdmin;
  }
  function isPhoto(a: AttachmentDto): boolean {
    return a.mime.toLowerCase().startsWith('image/') && a.hasThumbnail && !a.missing;
  }

  const photos = $derived(attachments.filter((a) => !gone(a) && isPhoto(a)));
  const others = $derived(attachments.filter((a) => gone(a) || !isPhoto(a)));
  let failedThumbs = $state<Record<number, boolean>>({});

  function act(a: AttachmentDto) {
    if (a.missing || a.viewable) onOpen(a);
    else onDownload(a);
  }
  function label(a: AttachmentDto): string {
    return a.missing || a.viewable ? `Открыть «${a.originalName}»` : `Скачать «${a.originalName}»`;
  }
</script>

{#if photos.length > 0}
  <div class="photos" class:photos-grid={photos.length > 1}>
    {#each photos as a (a.id)}
      <button
        type="button"
        class="photo"
        class:photo-single={photos.length === 1}
        aria-label={label(a)}
        title={a.deletedAtUtc !== null ? `${a.originalName} (удалён)` : a.originalName}
        onclick={() => act(a)}
      >
        {#if failedThumbs[a.id]}
          <AttachmentTypeIcon kind="image" />
        {:else}
          <img
            src={attachmentUrl(a.id, 'thumb')}
            alt=""
            loading="lazy"
            onerror={() => (failedThumbs[a.id] = true)}
          />
        {/if}
      </button>
    {/each}
  </div>
{/if}

{#if others.length > 0}
  <div class="files" class:files-own={own}>
    {#each others as a (a.id)}
      {#if gone(a)}
        <div class="file file-gone">
          <span class="file-icon file-icon-muted">
            <AttachmentTypeIcon kind={iconKindForMime(a.mime, a.originalName)} />
          </span>
          <span class="file-sub">Файл удалён</span>
        </div>
      {:else}
        <button type="button" class="file" aria-label={label(a)} onclick={() => act(a)}>
          <span class="file-icon">
            <AttachmentTypeIcon kind={iconKindForMime(a.mime, a.originalName)} />
          </span>
          <span class="file-text">
            <span class="file-name" title={a.originalName}>{a.originalName}</span>
            {#if a.missing}
              <span class="file-sub file-warn">Файл не найден</span>
            {:else if a.deletedAtUtc !== null}
              <span class="file-sub">Удалён</span>
            {:else}
              <span class="file-sub">{formatFileSize(a.sizeBytes)}</span>
            {/if}
          </span>
        </button>
      {/if}
    {/each}
  </div>
{/if}

<style lang="scss">
  .photos {
    display: flex;
    margin-bottom: var(--tr-space-xs);
  }
  .photos-grid {
    display: grid;
    grid-template-columns: repeat(2, 112px);
    gap: var(--tr-space-2xs);
  }
  .photo {
    display: block;
    margin: 0;
    padding: 0;
    border: 0;
    border-radius: var(--tr-radius-md);
    background: var(--tr-bg);
    overflow: hidden;
    cursor: pointer;
    width: 112px;
    height: 112px;
    img {
      display: block;
      width: 100%;
      height: 100%;
      object-fit: cover;
    }
  }
  .photo-single {
    width: auto;
    height: auto;
    max-width: 240px;
    max-height: 240px;
    img {
      width: auto;
      height: auto;
      max-width: 240px;
      max-height: 240px;
      object-fit: contain;
    }
  }
  .files {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-xs);
    margin-bottom: var(--tr-space-xs);
  }
  .file {
    display: flex;
    align-items: center;
    gap: var(--tr-space-xs);
    box-sizing: border-box;
    width: 100%;
    min-width: 220px;
    height: 48px;
    margin: 0;
    padding: 0 var(--tr-space-xs);
    border: 0;
    border-radius: var(--tr-radius-md);
    background: var(--tr-surface-sunken);
    color: inherit;
    font: inherit;
    text-align: left;
    white-space: normal;
    cursor: pointer;
  }
  .files-own .file {
    background: var(--tr-surface);
  }
  .file-gone {
    cursor: default;
  }
  .file-icon {
    display: inline-flex;
    flex: 0 0 auto;
    width: 32px;
    height: 32px;
    color: var(--tr-text-secondary);
  }
  .file-icon-muted {
    color: var(--tr-text-tertiary);
  }
  .file-text {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .file-name {
    font: var(--tr-text-label);
    color: var(--tr-text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .file-sub {
    font: var(--tr-text-caption);
    color: var(--tr-text-secondary);
    white-space: nowrap;
  }
  .file-warn {
    color: var(--tr-warning-muted-text);
  }
</style>

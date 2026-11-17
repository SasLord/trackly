<script lang="ts">
  // Событие вложения, приложенного через «Информацию» (D-13; UI-SPEC §9.4).
  // AttachmentViewer здесь НЕ рендерится: один экземпляр на RequestChat.
  import Button from '$lib/components/Button.svelte';
  import { formatDateTimeShort } from '$lib/utils/date';
  import { attachmentActionLabel } from '../../attachments/AttachmentHistory.svelte';
  import { attachmentUrl } from '../../attachments/api';
  import { actorLabel } from '../../attachments/labels';
  import type { AttachmentDto, ChatAttachmentEventDto } from '../../../bindings';
  import ChatEventIcon from './ChatEventIcon.svelte';
  import { timeHHmm } from './chatDates';

  interface Props {
    item: ChatAttachmentEventDto;
    attachment: AttachmentDto | null;
    onOpen: (_attachment: AttachmentDto) => void;
  }

  const { item, attachment, onOpen }: Props = $props();

  const ev = $derived(item.event);
  const parts = $derived(attachmentActionLabel(ev.action, '\u0000').split('\u0000'));
  const before = $derived(parts[0] ?? '');
  const after = $derived(parts[1] ?? '');
  const who = $derived(ev.userName ?? actorLabel(null, null));
  const live = $derived(
    attachment !== null && attachment.deletedAtUtc === null && ev.state === 'active',
  );
  const showThumb = $derived(
    live && attachment !== null && attachment.hasThumbnail && !attachment.missing,
  );
  const time = $derived(timeHHmm(item.createdAtUtc));
  const fullTime = $derived(formatDateTimeShort(item.createdAtUtc));
  let thumbFailed = $state(false);
</script>

<li class="event" aria-label={`${before}${ev.name}${after} · ${who}, ${time}`}>
  <span class="icon"><ChatEventIcon kind="paperclip" /></span>
  <div class="body">
    <p class="line">
      {before}{#if live && attachment}<Button
          variant="link"
          size="sm"
          onclick={() => onOpen(attachment)}>{ev.name}</Button
        >{:else}<span class="plain-name">{ev.name}</span>{/if}{after}<span class="who">
        · {who}</span
      >
    </p>
    {#if showThumb && attachment && !thumbFailed}
      <button
        type="button"
        class="thumb"
        aria-label={`Открыть «${attachment.originalName}»`}
        onclick={() => onOpen(attachment)}
      >
        <img
          src={attachmentUrl(attachment.id, 'thumb')}
          alt=""
          loading="lazy"
          onerror={() => (thumbFailed = true)}
        />
      </button>
    {/if}
  </div>
  <span class="time" title={fullTime}>{time}</span>
</li>

<style lang="scss">
  .event {
    display: flex;
    align-items: flex-start;
    gap: var(--tr-space-xs);
    box-sizing: border-box;
    width: fit-content;
    max-width: 560px;
    margin: var(--tr-space-md) auto;
    padding: var(--tr-space-xs) var(--tr-space-sm);
    list-style: none;
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-md);
    background: var(--tr-surface);
    text-align: left;
    white-space: normal;
  }
  .icon {
    display: inline-flex;
    flex: 0 0 auto;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    border-radius: var(--tr-radius-full);
    background: var(--tr-surface-sunken);
    color: var(--tr-text-secondary);
  }
  .body {
    flex: 1 1 auto;
    min-width: 0;
  }
  p {
    margin: 0;
  }
  .line {
    font: var(--tr-text-label);
    color: var(--tr-text-primary);
    overflow-wrap: anywhere;
  }
  .who {
    color: var(--tr-text-secondary);
  }
  .thumb {
    display: block;
    width: 64px;
    height: 64px;
    margin: var(--tr-space-xs) 0 0;
    padding: 0;
    border: 0;
    border-radius: var(--tr-radius-sm);
    background: var(--tr-bg);
    overflow: hidden;
    cursor: pointer;
    img {
      display: block;
      width: 100%;
      height: 100%;
      object-fit: cover;
    }
  }
  .time {
    flex: 0 0 auto;
    align-self: flex-end;
    font: var(--tr-text-caption);
    color: var(--tr-text-secondary);
    white-space: nowrap;
  }
</style>

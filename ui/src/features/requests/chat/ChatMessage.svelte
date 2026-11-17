<script lang="ts">
  // Пузырь сообщения (D-10, D-09, D-15, D-22, D-25; UI-SPEC §7).
  // Чужие слева с ФИО и ролью, свои справа без ФИО. Меню правки и галочек нет.
  // Текст — только интерполяцией (linkify → <a>/текст), без raw-HTML.
  import { formatDateTimeShort } from '$lib/utils/date';
  import type { AttachmentDto, ChatMessageDto } from '../../../bindings';
  import ChatBubbleFiles from './ChatBubbleFiles.svelte';
  import { timeHHmm } from './chatDates';
  import { linkify } from './linkify';
  import { onExternalLinkClick } from './openExternal';

  interface Props {
    item: ChatMessageDto;
    groupStart: boolean;
    onOpenAttachment: (_attachment: AttachmentDto) => void;
    onDownloadAttachment: (_attachment: AttachmentDto) => void;
  }

  const { item, groupStart, onOpenAttachment, onDownloadAttachment }: Props = $props();

  const ROLES: Record<string, string> = {
    admin: 'Администратор',
    manager: 'Специалист',
    requester: 'Автор заявки',
    participant: 'Участник',
  };

  const deletedAuthor = $derived(item.actorName === null);
  const authorName = $derived(deletedAuthor ? 'Пользователь удалён' : item.actorName);
  const roleLabel = $derived(
    deletedAuthor || item.authorRole === null ? null : (ROLES[item.authorRole] ?? null),
  );
  const time = $derived(timeHHmm(item.createdAtUtc));
  const fullTime = $derived(formatDateTimeShort(item.createdAtUtc));
  const segments = $derived(linkify(item.text));
  const srPrefix = $derived(
    item.isOwn ? `Вы, ${time}:` : `${authorName}${roleLabel ? `, ${roleLabel}` : ''}, ${time}:`,
  );
</script>

<li class="msg" class:own={item.isOwn} class:group-start={groupStart}>
  {#if groupStart && !item.isOwn}
    <p class="author">
      <span class="author-name">{authorName}</span>
      {#if roleLabel}<span class="author-role">{roleLabel}</span>{/if}
    </p>
  {/if}
  <div class="bubble" class:bubble-own={item.isOwn} class:tail={groupStart}>
    <span class="sr-only">{srPrefix}</span>
    {#if item.movedFromRequestId !== null}
      <p class="moved">
        <svg
          width="12"
          height="12"
          viewBox="0 0 16 16"
          fill="none"
          stroke="currentColor"
          stroke-width="1.5"
          stroke-linecap="round"
          stroke-linejoin="round"
          aria-hidden="true"
        >
          <path d="M2.5 3l4 4v6M13.5 3l-4 4M8 7v6" />
        </svg>
        из заявки №{item.movedFromRequestId}
      </p>
    {/if}
    {#if item.attachments.length > 0}
      <ChatBubbleFiles
        attachments={item.attachments}
        own={item.isOwn}
        onOpen={onOpenAttachment}
        onDownload={onDownloadAttachment}
      />
    {/if}
    {#if item.text}
      <span class="text"
        >{#each segments as seg, i (i)}{#if seg.type === 'link'}<a
              href={seg.value}
              target="_blank"
              rel="noopener noreferrer"
              onclick={(e) => onExternalLinkClick(e, seg.value)}>{seg.value}</a
            >{:else}{seg.value}{/if}{/each}</span
      >
    {/if}
    <span class="time" title={fullTime}>{time}</span>
  </div>
</li>

<style lang="scss">
  .msg {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    margin: var(--tr-space-2xs) 0 0;
    list-style: none;
    &.group-start {
      margin-top: var(--tr-space-md);
    }
    &.own {
      align-items: flex-end;
    }
  }
  .author {
    display: flex;
    align-items: baseline;
    gap: var(--tr-space-2xs);
    margin: 0 0 var(--tr-space-2xs);
  }
  .author-name {
    font: var(--tr-text-label);
    color: var(--tr-text-primary);
  }
  .author-role {
    font: var(--tr-text-caption);
    color: var(--tr-text-secondary);
  }
  .bubble {
    position: relative;
    display: flow-root;
    box-sizing: border-box;
    min-width: 0;
    max-width: 72%;
    padding: var(--tr-space-xs) var(--tr-space-sm);
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-lg);
    background: var(--tr-surface);
    color: var(--tr-text-primary);
    font: var(--tr-text-body);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    text-align: left;
  }
  .bubble-own {
    background: var(--tr-accent-soft);
    border-color: var(--tr-accent-soft);
  }
  .tail {
    border-top-left-radius: var(--tr-radius-xs);
  }
  .bubble-own.tail {
    border-top-left-radius: var(--tr-radius-lg);
    border-top-right-radius: var(--tr-radius-xs);
  }
  .moved {
    display: flex;
    align-items: center;
    gap: var(--tr-space-2xs);
    margin: 0 0 var(--tr-space-2xs);
    font: var(--tr-text-caption);
    color: var(--tr-text-secondary);
    white-space: normal;
  }
  .text a {
    color: var(--tr-accent-text);
    text-decoration: underline;
  }
  .time {
    float: right;
    margin-left: var(--tr-space-xs);
    font: var(--tr-text-caption);
    color: var(--tr-text-secondary);
    white-space: nowrap;
  }
  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
  }
</style>

<script lang="ts" module>
  // Фаза 41.3, план 14 (D-14): подписи событий вложений для историй карточек.
  // Сервер (AttachmentService::history) отдаёт action БЕЗ префикса
  // `custom:attachment_`: added / deleted / restored / purged (G5).
  export function attachmentActionLabel(action: string, name: string): string {
    switch (action) {
      case 'added':
        return `Приложен файл «${name}»`;
      case 'deleted':
        return `Удалён файл «${name}»`;
      case 'restored':
        return `Восстановлен файл «${name}»`;
      case 'purged':
        return `Файл «${name}» удалён навсегда`;
      default:
        return action;
    }
  }
</script>

<script lang="ts">
  // Строки событий вложений. Имя файла выводится ТОЛЬКО интерполяцией текста;
  // кнопкой-ссылкой оно становится, пока вложение активно (администратору ещё и
  // пока оно в «Удалённых»), иначе это обычный текст.
  import Button from '$lib/components/Button.svelte';
  import { authStore } from '$lib/stores/auth.svelte';
  import { formatDateTimeShort } from '$lib/utils/date';
  import type { AttachmentHistoryEventDto } from '../../bindings';

  interface Props {
    events: AttachmentHistoryEventDto[];
    onOpen?: (attachmentId: number) => void;
    /** true: только <li> (строки встраиваются в чужой <ul>). */
    bare?: boolean;
  }

  const { events, onOpen, bare = false }: Props = $props();

  const isAdmin = $derived(authStore.user?.role === 'admin');

  function linkable(ev: AttachmentHistoryEventDto): boolean {
    if (!onOpen) return false;
    if (ev.action === 'purged') return false;
    return ev.state === 'active' || (ev.state === 'deleted' && isAdmin);
  }

  function prefix(action: string): string {
    // Часть подписи до имени: «Приложен файл «», разделяем по имени ниже.
    return attachmentActionLabel(action, '\u0000').split('\u0000')[0] ?? '';
  }
  function suffix(action: string): string {
    return attachmentActionLabel(action, '\u0000').split('\u0000')[1] ?? '';
  }
</script>

{#snippet rows()}
  {#each events as ev, i (`${ev.attachmentId}-${ev.action}-${ev.createdAtUtc}-${i}`)}
    <li class="history-row">
      <span class="history-text">
        {formatDateTimeShort(ev.createdAtUtc)} — {prefix(ev.action)}{#if linkable(ev)}<Button
            variant="link"
            size="sm"
            onclick={() => onOpen?.(ev.attachmentId)}>{ev.name}</Button
          >{:else}<span class="plain-name">{ev.name}</span>{/if}{suffix(ev.action)}{ev.userName
          ? `; ${ev.userName}`
          : ''}
      </span>
    </li>
  {/each}
{/snippet}

{#if bare}
  {@render rows()}
{:else}
  <ul class="history-list">
    {@render rows()}
  </ul>
{/if}

<style lang="scss">
  .history-list {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .history-row {
    display: flex;
    align-items: center;
    min-height: var(--row-height-dense, 32px);
    padding: 0;
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-primary);
    border-bottom: 1px solid var(--tr-border);
    white-space: normal;
    text-align: left;

    &:last-child {
      border-bottom: none;
    }
  }
  .history-text {
    flex: 1;
    min-width: 0;
    padding: var(--tr-space-2xs) 0;
    overflow-wrap: anywhere;
  }

  // Имя-ссылка (Button variant=link) по умолчанию не переносится: длинное имя
  // файла давало горизонтальную прокрутку карточки (41.3 G4).
  .history-text :global(button) {
    white-space: normal;
    overflow-wrap: anywhere;
    text-align: left;
  }
  .plain-name {
    color: var(--tr-text-primary);
  }
</style>

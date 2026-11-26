<script lang="ts">
  // Неотправленное сообщение в ленте (фаза 43-10; D-12). Свой пузырь справа; под ним статус:
  // «Отправляется…» или «Не отправлено» с кнопкой «Повторить» (повтор уходит с тем же clientMsgId).
  // Текст — только интерполяцией, без raw-HTML.
  import Button from '@trackly/kit/components/Button.svelte';
  import type { PendingMessage } from './pendingStore.svelte';

  interface Props {
    item: PendingMessage;
    onRetry: (_clientMsgId: string) => void;
  }

  const { item, onRetry }: Props = $props();

  const failed = $derived(item.status === 'failed');
</script>

<li class="msg own">
  <div class="bubble bubble-own">
    <span class="sr-only">Вы: </span>
    {#if item.text}<span class="text">{item.text}</span>{/if}
    {#if item.draftIds.length > 0}
      <p class="files">Вложений: {item.draftIds.length}</p>
    {/if}
  </div>
  <div class="status" class:failed role="status">
    {#if failed}
      <span class="status-text">Не отправлено</span>
      <Button variant="link" size="sm" onclick={() => onRetry(item.clientMsgId)}>Повторить</Button>
    {:else}
      <span class="status-text">Отправляется…</span>
      <Button variant="link" size="sm" disabled>Повторить</Button>
    {/if}
  </div>
</li>

<style lang="scss">
  .msg {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    margin: 0;
    list-style: none;
  }
  .bubble {
    position: relative;
    display: flow-root;
    box-sizing: border-box;
    min-width: 0;
    max-width: 72%;
    padding: var(--tr-space-xs) var(--tr-space-sm);
    border: 1px solid var(--tr-accent-soft);
    border-radius: var(--tr-radius-lg);
    background: var(--tr-accent-soft);
    color: var(--tr-text-primary);
    font: var(--tr-text-body);
    white-space: normal;
    overflow-wrap: anywhere;
    text-align: left;
  }
  .text {
    display: block;
    white-space: pre-wrap;
  }
  .files {
    margin: var(--tr-space-2xs) 0 0;
    font: var(--tr-text-caption);
    color: var(--tr-text-secondary);
    white-space: normal;
  }
  .status {
    display: flex;
    align-items: center;
    gap: var(--tr-space-xs);
    margin-top: var(--tr-space-3xs);
    font: var(--tr-text-caption);
    color: var(--tr-text-secondary);
    white-space: nowrap;
    &.failed {
      color: var(--tr-danger-text);
    }
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

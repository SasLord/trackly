<script lang="ts">
  // Plan 06-05: строка списка заявок.
  // По паттерну CartridgeListRow.svelte.
  import Badge from '$lib/components/Badge.svelte';
  import type { RequestDto } from '../../bindings-phase6';

  interface Props {
    request: RequestDto;
    selected: boolean;
    onclick: () => void;
  }

  const { request, selected, onclick }: Props = $props();

  type BadgeVariant = 'success' | 'accent' | 'warning' | 'default' | 'destructive';

  // Badge variant по статусу заявки (UI-SPEC §Badge-цвета статусов заявки)
  const statusVariant = $derived<BadgeVariant>(
    request.status === 'open'
      ? 'accent'
      : request.status === 'in_progress'
        ? 'warning'
        : request.status === 'completed'
          ? 'success'
          : 'default',
  );

  const statusLabel = $derived(
    request.status === 'open'
      ? 'Создана'
      : request.status === 'in_progress'
        ? 'В работе'
        : request.status === 'completed'
          ? 'Выполнена'
          : request.status === 'cancelled'
            ? 'Отменена'
            : 'Отклонена',
  );

  const typeLabel = $derived(
    request.requestType === 'ad_register'
      ? 'Регистрация AD'
      : request.requestType === 'cartridge_replace'
        ? 'Замена картриджа'
        : 'Свободная форма',
  );

  // D-REG-03: restore variant of ad_register — distinct chip from first-time register.
  const isAdRestore = $derived(
    request.requestType === 'ad_register' && request.adSubtype === 'restore',
  );

  // Краткое описание: ad_register — запрошенное ФИО; cartridge_replace — принтер; free_form — description (truncated)
  const shortDesc = $derived(
    request.requestType === 'ad_register'
      ? (request.description ?? request.requesterName ?? '')
      : request.requestType === 'cartridge_replace'
        ? (request.printerName ?? 'Принтер не указан')
        : (request.description ?? ''),
  );

  // Относительная дата
  function relativeDate(utcSeconds: number): string {
    const now = Math.floor(Date.now() / 1000);
    const diff = now - utcSeconds;
    if (diff < 60) return 'только что';
    if (diff < 3600) return `${Math.floor(diff / 60)} мин. назад`;
    if (diff < 86400) return `${Math.floor(diff / 3600)} ч. назад`;
    const days = Math.floor(diff / 86400);
    if (days < 7) return `${days} дн. назад`;
    const d = new Date(utcSeconds * 1000);
    return `${String(d.getUTCDate()).padStart(2, '0')}.${String(d.getUTCMonth() + 1).padStart(2, '0')}.${d.getUTCFullYear()}`;
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      onclick();
    }
  }
</script>

<div
  class="row"
  class:selected
  role="button"
  tabindex="0"
  aria-pressed={selected}
  {onclick}
  onkeydown={handleKeydown}
>
  <div class="top">
    <span class="type-badge">
      <Badge variant="default" size="sm">{typeLabel}</Badge>
    </span>
    {#if isAdRestore}
      <span class="type-badge">
        <Badge variant="warning" size="sm">Восстановление доступа</Badge>
      </span>
    {/if}
    <span class="desc">{shortDesc}</span>
    <span class="status-badge">
      <Badge variant={statusVariant}>{statusLabel}</Badge>
    </span>
  </div>
  <div class="bottom">
    <span class="author">{request.requesterName ?? '—'}</span>
    <span class="date">{relativeDate(request.createdAtUtc)}</span>
  </div>
</div>

<style lang="scss">
  .row {
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: var(--tr-space-2xs);
    min-height: var(--row-height, 40px);
    padding: var(--tr-space-xs) var(--tr-space-md);
    border-bottom: 1px solid var(--tr-border);
    cursor: pointer;
    border-left: 3px solid transparent;

    &:hover {
      background: var(--tr-surface-sunken);
    }

    &:focus-visible {
      outline: none;
      box-shadow: inset 0 0 0 2px var(--tr-accent);
    }

    &.selected {
      border-left-color: var(--tr-accent);
      background: color-mix(in srgb, var(--tr-accent) 8%, transparent);
    }
  }

  .top {
    display: flex;
    align-items: center;
    gap: var(--tr-space-2xs);
    font-size: var(--font-size-body);
    line-height: 1.2;
  }

  .type-badge {
    flex-shrink: 0;
  }

  .desc {
    color: var(--tr-text-secondary);
    font-size: var(--font-size-label);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
    min-width: 0;
  }

  .status-badge {
    flex-shrink: 0;
    margin-left: auto;
  }

  .bottom {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: var(--font-size-label);
    color: var(--tr-text-secondary);
  }

  .author {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .date {
    flex-shrink: 0;
    color: var(--tr-text-tertiary);
    margin-left: var(--tr-space-xs);
  }
</style>

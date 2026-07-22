<script lang="ts">
  // Plan 06-05: строка списка заявок.
  // По паттерну CartridgeListRow.svelte.
  // Plan 28-01 (D-03): rebuilt on shared TableRow primitive per ActListRow.svelte
  // precedent — bespoke two-line `.row` div replaced with a 4-column <TableRow>
  // (Тип/Описание/Автор/Статус). TableRow does not forward arbitrary attrs
  // (onclick/role/tabindex) to its own <tr> — row click/keyboard-select is wired
  // on the <td> cells we own here (onclick on every cell for full-row mouse click;
  // role="button"+tabindex+onkeydown on the first cell as the single keyboard
  // entry point, mirroring the previous single-div tab-stop).
  import Badge from '$lib/components/Badge.svelte';
  import TableRow from '$lib/components/TableRow.svelte';
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

<TableRow {selected} class="request-row">
  <td
    class="cell cell-type"
    role="button"
    tabindex="0"
    aria-pressed={selected}
    {onclick}
    onkeydown={handleKeydown}
  >
    <Badge variant="default" size="sm">{typeLabel}</Badge>
    {#if isAdRestore}
      <Badge variant="warning" size="sm">Восстановление доступа</Badge>
    {/if}
  </td>
  <td class="cell cell-desc" title={shortDesc} {onclick}>{shortDesc}</td>
  <td class="cell cell-author" {onclick}>
    {request.requesterName ?? '—'}
    <span class="cell-date">{relativeDate(request.createdAtUtc)}</span>
  </td>
  <td class="cell cell-status" {onclick}>
    <Badge variant={statusVariant}>{statusLabel}</Badge>
  </td>
</TableRow>

<style lang="scss">
  // TableRow renders its own <tr> (a DIFFERENT Svelte scope-hash than this file) —
  // caller-supplied class needs `:global()`, and the ancestor part of the selector
  // must stay in THIS file's scope: `.request-row :global(> td)`, never
  // `:global(.request-row > td)` (specificity trap, see TableRow.svelte contract).
  :global(tr.request-row) {
    cursor: pointer;
  }

  .cell {
    font-size: var(--tr-font-size-body);
    color: var(--tr-text-primary);
  }

  .cell-type {
    display: flex;
    align-items: center;
    gap: var(--tr-space-2xs);
    white-space: nowrap;
    max-width: 190px;

    &:focus-visible {
      outline: none;
      box-shadow: inset 0 0 0 2px var(--tr-accent);
    }
  }

  .cell-desc {
    color: var(--tr-text-secondary);
    font-size: var(--tr-font-size-label);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 0; // makes text-overflow work in table cells
  }

  .cell-author {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 0;
  }

  .cell-date {
    display: block;
    color: var(--tr-text-tertiary);
    font-size: var(--tr-font-size-label);
  }

  .cell-status {
    width: 110px;
    white-space: nowrap;
  }
</style>

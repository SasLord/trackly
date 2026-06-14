<script lang="ts">
  // Plan 06-04: строка списка принтеров.
  // Колонки: имя устройства, IP/«USB», статус-badge, alert-dot (has_alert), краткий тонер.
  // По паттерну CartridgeListRow.svelte (06-PATTERNS.md §PrinterListRow.svelte).
  import Badge from '$lib/components/Badge.svelte';
  import type { PrinterDto } from '../../bindings-phase6';

  interface Props {
    printer: PrinterDto;
    selected: boolean;
    onclick: () => void;
  }

  const { printer, selected, onclick }: Props = $props();

  // Badge variant по status (UI-SPEC §Badge-цвета статусов принтера).
  type BadgeVariant = 'success' | 'warning' | 'destructive' | 'default';

  const statusVariant = $derived<BadgeVariant>(
    printer.status === 'ok' || printer.status === 'online'
      ? 'success'
      : printer.status === 'warning'
        ? 'warning'
        : printer.status === 'error'
          ? 'destructive'
          : 'default',
  );

  const statusLabel = $derived<string>(
    printer.status === 'ok' || printer.status === 'online'
      ? 'В сети'
      : printer.status === 'warning'
        ? 'Предупреждение'
        : printer.status === 'error'
          ? 'Ошибка'
          : printer.status === 'offline'
            ? 'Не в сети'
            : 'Нет данных',
  );

  const locationLabel = $derived<string>(
    printer.ipAddress ? printer.ipAddress : printer.usbHostDeviceId ? 'USB' : '—',
  );

  const displayName = $derived<string>(printer.deviceName ?? `Принтер #${printer.id}`);

  // Quick toner summary: find first toner level key.
  const tonerSummary = $derived<string | null>(
    printer.tonerLevels
      ? (() => {
          const entries = Object.entries(printer.tonerLevels!);
          if (entries.length === 0) return null;
          const [key, val] = entries[0];
          return val !== null ? `${key}: ${val}%` : null;
        })()
      : null,
  );

  function handleClick() {
    onclick();
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
  onclick={handleClick}
  onkeydown={handleKeydown}
>
  <div class="top">
    {#if printer.hasAlert}
      <span
        class="alert-dot"
        aria-label="Есть проблема с принтером"
        title="Есть проблема"
      ></span>
    {/if}
    <span class="name">{displayName}</span>
    <span class="badge-wrap">
      <Badge variant={statusVariant}>{statusLabel}</Badge>
    </span>
  </div>
  <div class="bottom">
    <span class="location">{locationLabel}</span>
    {#if tonerSummary}
      <span class="toner-hint">{tonerSummary}</span>
    {/if}
  </div>
</div>

<style lang="scss">
  .row {
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: var(--space-xs);
    min-height: var(--row-height, 40px);
    padding: var(--space-sm) var(--space-md);
    border-bottom: 1px solid var(--color-border);
    cursor: pointer;
    border-left: 3px solid transparent;

    &:hover {
      background: var(--color-surface-sunken);
    }

    &:focus-visible {
      outline: none;
      box-shadow: inset 0 0 0 2px var(--color-accent);
    }

    &.selected {
      border-left-color: var(--color-accent);
      background: color-mix(in srgb, var(--color-accent) 8%, transparent);
    }
  }

  .top {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    font-size: var(--font-size-body);
    line-height: 1.2;
  }

  .alert-dot {
    flex-shrink: 0;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--color-destructive);
  }

  .name {
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }

  .badge-wrap {
    flex-shrink: 0;
    margin-left: auto;
  }

  .bottom {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    font-size: var(--font-size-label);
    color: var(--color-text-secondary);
  }

  .location {
    font-variant-numeric: tabular-nums;
  }

  .toner-hint {
    color: var(--color-text-muted);
    font-size: var(--font-size-label);
  }
</style>

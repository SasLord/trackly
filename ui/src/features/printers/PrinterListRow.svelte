<script lang="ts">
  // Plan 06-04: строка списка принтеров.
  // Колонки: имя устройства, IP/«USB», статус-badge, alert-dot (has_alert), краткий тонер.
  // По паттерну CartridgeListRow.svelte (06-PATTERNS.md §PrinterListRow.svelte).
  // Plan 27-07 (D-03): перестроено на общий TableRow-примитив по образцу ActListRow.svelte —
  // bespoke двухстрочный `.row` заменён на 4-колоночную <TableRow> (имя/IP/статус/тонер).
  // Row-click/keyboard-select навешаны на <td>-ячейки (TableRow не форвардит onclick/role/
  // tabindex в свой <tr>) — тот же паттерн, что ActListRow/DeviceListRow.
  import Badge from '$lib/components/Badge.svelte';
  import TableRow from '$lib/components/TableRow.svelte';
  import TonerGauge from './TonerGauge.svelte';
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

  // FIX F2 (Phase 27 batch F): IP/USB shown only when present — no fallback
  // dash, so the merged name+IP cell can collapse to a single centered line.
  const ipText = $derived<string | null>(
    printer.ipAddress ? printer.ipAddress : printer.usbHostDeviceId ? 'USB' : null,
  );

  const displayName = $derived<string>(printer.deviceName ?? `Принтер #${printer.id}`);

  // Quick toner summary: first toner-level entry (same scope as the previous
  // bespoke row — full breakdown lives in PrinterDetail, not the list row).
  const firstTonerEntry = $derived<[string, number | null] | null>(
    printer.tonerLevels ? (Object.entries(printer.tonerLevels)[0] ?? null) : null,
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

<TableRow {selected} class="printer-row">
  <td
    id="printer-row-{printer.id}"
    class="cell cell-name"
    role="button"
    tabindex="0"
    aria-pressed={selected}
    title={displayName}
    onclick={handleClick}
    onkeydown={handleKeydown}
  >
    <span class="cell-name-inner">
      {#if printer.hasAlert}
        <span class="alert-dot" aria-label="Есть проблема с принтером" title="Есть проблема"></span>
      {/if}
      <span class="name-lines">
        <span class="name-text">{displayName}</span>
        {#if ipText}
          <span class="ip-text tr-mono">{ipText}</span>
        {/if}
      </span>
    </span>
  </td>
  <td class="cell cell-status" onclick={handleClick}>
    <Badge variant={statusVariant}>{statusLabel}</Badge>
  </td>
  <td class="cell cell-toner" onclick={handleClick}>
    {#if firstTonerEntry}
      <TonerGauge label={firstTonerEntry[0]} level={firstTonerEntry[1]} encoding="percent" />
    {:else}
      <span class="toner-empty">—</span>
    {/if}
  </td>
</TableRow>

<style lang="scss">
  // TableRow renders its own <tr> (a DIFFERENT Svelte scope-hash than this
  // file) — caller-supplied class needs `:global()`, ancestor part stays in
  // THIS file's scope per the TableRow contract (see ActListRow.svelte).
  :global(tr.printer-row) {
    cursor: pointer;
  }

  .cell {
    font-size: var(--tr-font-size-body);
    color: var(--tr-text-primary);
  }

  // FIX B3: `display: flex` on the <td> ITSELF overrides `display: table-cell`,
  // pulling the cell out of the table's column model — every column collapses/
  // overlaps. The <td> stays a normal table cell (ellipsis/shrink + cursor +
  // focus ring only); the flex layout lives on the inner span below.
  // FIX F2 (Phase 27 batch F): cell now holds two lines (name + IP) when an
  // IP/USB value exists — `white-space: nowrap` dropped from the <td> itself
  // so the inner `.name-lines` column can wrap onto two rows; each line still
  // truncates individually via its own overflow/ellipsis rule below.
  .cell-name {
    cursor: pointer;
    overflow: hidden;
    max-width: 0; // makes text-overflow work in table cells

    &:focus-visible {
      // ring теперь на уровне строки, см. TableRow.svelte .tr-row:has(:focus-visible) (Gap 4, план 30-05)
      // check-focus-outline: ignore
      outline: none;
    }
  }

  .cell-name-inner {
    display: flex;
    align-items: center;
    gap: var(--tr-space-2xs);
  }

  .alert-dot {
    flex-shrink: 0;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--tr-danger);
  }

  // Column of name (top) + optional IP (bottom, smaller/secondary). Centers
  // vertically within the row when only the name line is present (no IP).
  .name-lines {
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 2px;
    min-width: 0;
  }

  .name-text {
    font-weight: var(--tr-font-weight-semibold);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }

  .ip-text {
    font-size: var(--tr-font-size-caption);
    color: var(--tr-text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }

  .cell-status {
    width: 140px;
  }

  .cell-toner {
    width: 160px;
  }

  .toner-empty {
    color: var(--tr-text-tertiary);
  }
</style>

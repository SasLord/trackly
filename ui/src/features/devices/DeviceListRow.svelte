<script lang="ts">
  import Badge from '$lib/components/Badge.svelte';
  import TableRow from '$lib/components/TableRow.svelte';
  import DeviceRowMenu from './DeviceRowMenu.svelte';
  import { isOwnKeyEvent, isRowOverlayEvent } from './deviceRowMenu';
  import { statusLabel, statusTone } from '$lib/stores/deviceStatuses.svelte';
  import type { DeviceDto } from '../../bindings';

  interface Props {
    device: DeviceDto;
    onEdit: (_d: DeviceDto) => void;
    onDelete: () => void;
    /** When true, renders a stronger bottom border to visually close the group. */
    isLastInGroup?: boolean;
    /** Plan 03-05 (DEV-14): pass-through к DeviceRowMenu. */
    onPrintAcceptance?: (_d: DeviceDto) => void;
    /** ITEM-3: when true, shows the «Статус» column. Hide on filtered status tabs. */
    showStatus?: boolean;
    /** GAP-8 (39-UAT.md): cross-section focus from the Places content-row
     *  «Перейти к устройству» action (`#/devices?id=…`) — this row renders
     *  selected/highlighted + carries a stable DOM id for scrollIntoView
     *  when it matches. */
    highlightId?: number | null;
  }

  const {
    device,
    onEdit,
    onDelete,
    isLastInGroup = false,
    onPrintAcceptance,
    showStatus = true,
    highlightId = null,
  }: Props = $props();

  const isHighlighted = $derived(highlightId !== null && device.id === highlightId);

  // Подпись и тон статуса — из единого справочника (фаза 41.1, D-06/D-09):
  // подпись по id, тон по code. Хардкода «id -> подпись» здесь нет.
  const label = $derived(statusLabel(device.status_id));
  const tone = $derived(statusTone(device.status_code));

  // Меню строки (фаза 41.1, план 17; D-21, D-25, D-26). Кебаб, правый клик и
  // клавиатура открывают ОДНО меню DeviceRowMenu; двойной клик — просмотр.
  let menuRef = $state<ReturnType<typeof DeviceRowMenu> | null>(null);
  let firstCellEl = $state<HTMLElement | null>(null);

  // Родное контекстное меню гасится ТОЛЬКО здесь и одинаково в Tauri и в
  // LAN-браузере (ветвей по транспорту нет). Охрана — первым оператором:
  // внутри открытой модалки меню DeviceRowMenu (она стоит внутри строки)
  // остаётся родное меню браузера — копировать/вставить в поле названия.
  function handleContextMenu(e: MouseEvent) {
    if (isRowOverlayEvent(e.target)) return;
    e.preventDefault();
    if (e.clientX === 0 && e.clientY === 0 && firstCellEl) {
      // Событие от клавиши контекстного меню: координат у него нет.
      void menuRef?.openFromElement(firstCellEl);
    } else {
      void menuRef?.openAt(e.clientX, e.clientY);
    }
  }

  function handleDblClick(e: MouseEvent) {
    if (isRowOverlayEvent(e.target)) return;
    // Двойной клик по кебабу, ссылке или кнопке не открывает просмотр.
    if ((e.target as HTMLElement | null)?.closest?.('button, a')) return;
    window.getSelection()?.removeAllRanges();
    void menuRef?.openView();
  }

  function handleCellKeydown(e: KeyboardEvent) {
    if (!isOwnKeyEvent(e.target, e.currentTarget)) return;
    if (e.key === 'Enter') {
      e.preventDefault();
      void menuRef?.openView();
    } else if (e.key === 'ContextMenu' || (e.key === 'F10' && e.shiftKey)) {
      e.preventDefault();
      if (firstCellEl) void menuRef?.openFromElement(firstCellEl);
    }
  }
</script>

<TableRow
  class={isLastInGroup ? 'group-last-child' : undefined}
  selected={isHighlighted}
  oncontextmenu={handleContextMenu}
  ondblclick={handleDblClick}
>
  <td
    id="device-row-{device.id}"
    class="cell cell-name"
    title={device.name}
    role="button"
    tabindex="0"
    aria-haspopup="menu"
    bind:this={firstCellEl}
    onkeydown={handleCellKeydown}>{device.name}</td
  >
  <td class="cell cell-numeric" title={device.inventory_no ?? ''}
    ><span class="tr-mono">{device.inventory_no ?? '—'}</span></td
  >
  <td class="cell cell-numeric" title={device.serial_no ?? ''}
    ><span class="tr-mono">{device.serial_no ?? '—'}</span></td
  >
  <td class="cell" title={device.model ?? ''}>{device.model ?? '—'}</td>
  <td class="cell" title={device.full_path ?? ''}>{device.place_path_short ?? '—'}</td>
  <td class="cell" title={device.state ?? ''}>{device.state ?? '—'}</td>
  {#if showStatus}
    <td class="cell cell-status">
      <Badge variant={tone.variant} appearance={tone.appearance}>{label}</Badge>
    </td>
  {/if}
  <td class="cell cell-actions">
    <DeviceRowMenu
      bind:this={menuRef}
      {device}
      trigger="button"
      {onEdit}
      onChanged={onDelete}
      {onPrintAcceptance}
    />
  </td>
</TableRow>

<style lang="scss">
  // Visual group-end divider: last child in an expanded group gets a stronger
  // bottom border so the eye clearly sees where the group ends. `group-last-child`
  // is passed through to TableRow's rendered <tr> (a DIFFERENT Svelte scope-hash
  // than this file), so the ancestor part of the selector needs :global(); `.cell`
  // stays local (unwrapped) so it keeps this file's scope hash — combined class
  // count (group-last-child + cell + local-hash = 3) beats TableRow's own base
  // <td> border-bottom rule (.tr-row.hash > td, 2 classes) on specificity.
  :global(tr.group-last-child) > .cell {
    border-bottom: 2px solid var(--tr-border-strong);
  }

  .cell {
    font-size: var(--tr-font-size-body);
    color: var(--tr-text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 0; // makes text-overflow work in table cells
  }

  .cell-name {
    width: 25%;
    max-width: 200px;
  }

  .cell-numeric {
    width: 140px;
    font-variant-numeric: tabular-nums;
  }

  .cell-status {
    width: 120px;
  }

  .cell-actions {
    width: 40px;
    text-align: center;
    overflow: visible;
  }
</style>

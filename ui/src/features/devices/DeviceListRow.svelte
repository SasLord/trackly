<script lang="ts">
  import Badge from '$lib/components/Badge.svelte';
  import TableRow from '$lib/components/TableRow.svelte';
  import RowChevron from '$lib/components/icons/RowChevron.svelte';
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
    /** Якорная строка (фаза 41.1, D-12): состав раскрыт. */
    expanded?: boolean;
    /** Якорная строка: шеврон просит раскрыть/свернуть состав группы. */
    onToggleExpand?: (_groupId: number) => void;
    /** Строка состава якоря: отступ первой ячейки (D-12, UI-SPEC 9.4). */
    indent?: boolean;
  }

  const {
    device,
    onEdit,
    onDelete,
    isLastInGroup = false,
    onPrintAcceptance,
    showStatus = true,
    highlightId = null,
    expanded = false,
    onToggleExpand,
    indent = false,
  }: Props = $props();

  // Якорь (D-12): устройство, на котором держится группа. Строка остаётся полной
  // строкой устройства, к имени добавляются шеврон, бейдж группы и счётчик состава.
  const anchorGroupId = $derived(device.anchor_group_id ?? null);
  const isAnchor = $derived(anchorGroupId !== null);

  // Шеврон и ссылка-бейдж гасят click собственным обработчиком: строка не должна
  // реагировать на клики по вложенным управляющим элементам. Двойной клик по ним
  // отсекает handleDblClick строки (`closest('button, a')`, D-26); отдельной
  // привязки dblclick на вложенных элементах нет сознательно: гейт
  // check-device-row-menu читает любую привязку ondblclick= как привязку строки.
  function stopClick(e: MouseEvent) {
    e.stopPropagation();
  }

  function handleChevronClick(e: MouseEvent) {
    e.stopPropagation();
    if (anchorGroupId !== null) onToggleExpand?.(anchorGroupId);
  }

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
  {indent}
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
    onkeydown={handleCellKeydown}
  >
    {#if isAnchor}
      <span class="anchor-cell">
        <RowChevron {expanded} onclick={handleChevronClick} />
        <span class="anchor-name">{device.name}</span>
        <a
          class="anchor-link"
          href={'#/groups?id=' + anchorGroupId}
          title={'Открыть группу «' + device.anchor_group_name + '»'}
          aria-label={'Открыть группу «' + device.anchor_group_name + '»'}
          onclick={stopClick}
        >
          <Badge variant="accent" appearance="soft" size="sm">{device.anchor_group_name}</Badge>
        </a>
        <Badge variant="accent" appearance="count" size="sm">
          <span class="tr-mono">{device.anchor_member_count ?? 0} шт.</span>
        </Badge>
      </span>
    {:else}
      {device.name}
    {/if}
  </td>
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

  // Якорная строка (D-12): имя 600/14px + бейдж-ссылка на группу + счётчик состава.
  // Фон строки не меняется: --tr-group зарезервирован под свёртку одинаковых.
  .anchor-cell {
    display: flex;
    align-items: center;
    gap: var(--tr-space-xs);
    min-width: 0;
  }

  .anchor-name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    font-weight: var(--tr-font-weight-semibold);
  }

  .anchor-link {
    display: inline-flex;
    flex-shrink: 0;
    text-decoration: none;

    &:hover {
      text-decoration: underline;
    }
  }

  .anchor-cell > :global(.badge-m) {
    flex-shrink: 0;
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

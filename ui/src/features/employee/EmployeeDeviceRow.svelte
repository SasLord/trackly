<script lang="ts">
  // Строка таблицы устройств сотрудника (41.6, D-04, D-05, D-07, D-09, D-18, UI-SPEC §8-§9).
  // Кебаб, правый клик и клавиатура открывают ОДНО меню; двойной клик — карточку.
  // Компонент API не вызывает: о действиях сообщает колбэками.
  import Badge from '@trackly/kit/components/Badge.svelte';
  import TableRow from '@trackly/kit/components/TableRow.svelte';
  import PrinterMark from '../devices/PrinterMark.svelte';
  import { isOwnKeyEvent, isRowOverlayEvent } from '../devices/deviceRowMenu';
  import { statusTone } from '$lib/deviceStatusTone';
  import EmployeeDeviceMenu from './EmployeeDeviceMenu.svelte';
  import RequestLabels from './RequestLabels.svelte';
  import TonerMini from './TonerMini.svelte';
  import type { EmployeeDeviceDto } from '../../bindings';

  interface Props {
    row: EmployeeDeviceDto;
    onOpen: (_row: EmployeeDeviceDto) => void;
    onRepair: (_row: EmployeeDeviceDto) => void;
    onCartridge: (_row: EmployeeDeviceDto) => void;
    onGoto: (_requestId: number) => void;
    onCopy: (_row: EmployeeDeviceDto) => void;
  }

  const { row, onOpen, onRepair, onCartridge, onGoto, onCopy }: Props = $props();

  let menuRef = $state<ReturnType<typeof EmployeeDeviceMenu> | null>(null);
  let firstCellEl = $state<HTMLElement | null>(null);

  const tone = $derived(statusTone(row.statusCode));

  // Происхождение принтера считает сервер (D-05): клиент рисует то, что пришло.
  const origin = $derived.by(() => {
    if (row.printerOrigin === 'usb') {
      return { label: 'USB', accent: true, hint: 'подключён по USB' };
    }
    if (row.printerOrigin === 'link') {
      return { label: 'Ссылка', accent: false, hint: 'добавлен ссылкой' };
    }
    if (row.printerOrigin === 'member') {
      return { label: 'В составе', accent: false, hint: 'в составе группы' };
    }
    return null;
  });
  const armText = $derived(row.armNames.join(', '));
  const armTitle = $derived(origin ? (armText ? armText + ' · ' : '') + origin.hint : armText);

  // Охрана события — первым оператором, до preventDefault: внутри открытой
  // карточки или меню родное контекстное меню браузера остаётся.
  function handleContextMenu(e: MouseEvent) {
    if (isRowOverlayEvent(e.target)) return;
    e.preventDefault();
    if (e.clientX === 0 && e.clientY === 0 && firstCellEl) {
      // Событие от клавиши контекстного меню: координат у него нет.
      menuRef?.openFromElement(firstCellEl);
    } else {
      menuRef?.openAt(e.clientX, e.clientY);
    }
  }

  function handleDblClick(e: MouseEvent) {
    if (isRowOverlayEvent(e.target)) return;
    // Двойной клик по кебабу, ссылке-метке или кнопке карточку не открывает.
    if ((e.target as HTMLElement | null)?.closest?.('button, a')) return;
    window.getSelection()?.removeAllRanges();
    menuRef?.openView();
  }

  function handleCellKeydown(e: KeyboardEvent) {
    if (!isOwnKeyEvent(e.target, e.currentTarget)) return;
    if (e.key === 'Enter') {
      e.preventDefault();
      menuRef?.openView();
    } else if (e.key === 'ContextMenu' || (e.key === 'F10' && e.shiftKey)) {
      e.preventDefault();
      if (firstCellEl) menuRef?.openFromElement(firstCellEl);
    }
  }
</script>

<TableRow oncontextmenu={handleContextMenu} ondblclick={handleDblClick}>
  <td
    class="cell cell-name"
    title={row.name}
    role="button"
    tabindex="0"
    aria-haspopup="menu"
    bind:this={firstCellEl}
    onkeydown={handleCellKeydown}
  >
    <span class="name-cell">
      <PrinterMark isPrinter={row.isPrinter} />
      <span class="name-text">{row.name}</span>
      <RequestLabels myOpenRequests={row.myOpenRequests} otherOpenRequest={row.otherOpenRequest} />
    </span>
  </td>
  <td class="cell cell-numeric" title={row.inventoryNumber ?? ''}
    ><span class="tr-mono">{row.inventoryNumber ?? '—'}</span></td
  >
  <td class="cell cell-numeric" title={row.serialNumber ?? ''}
    ><span class="tr-mono">{row.serialNumber ?? '—'}</span></td
  >
  <td class="cell" title={row.model ?? ''}>{row.model ?? '—'}</td>
  <td class="cell" title={row.placeFull ?? ''}>{row.placeFull ?? '—'}</td>
  <td class="cell" title={armTitle}>
    {#if origin}
      <Badge variant={origin.accent ? 'accent' : 'default'} appearance="soft" size="sm"
        >{origin.label}</Badge
      >
    {/if}
    {armText || (origin ? '' : '—')}
  </td>
  <td class="cell cell-status">
    <Badge variant={tone.variant} appearance={tone.appearance} muted={tone.muted}
      >{row.statusName}</Badge
    >
  </td>
  <td class="cell cell-toner">
    {#if row.isPrinter}<TonerMini toner={row.toner} />{/if}
  </td>
  <td class="cell cell-actions">
    <EmployeeDeviceMenu
      bind:this={menuRef}
      {row}
      trigger="button"
      variant="ghost-sm"
      onOpen={() => onOpen(row)}
      onRepair={() => onRepair(row)}
      onCartridge={() => onCartridge(row)}
      {onGoto}
      onCopy={() => onCopy(row)}
    />
  </td>
</TableRow>

<style lang="scss">
  .cell {
    font-size: var(--tr-font-size-body);
    color: var(--tr-text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 0; // text-overflow в ячейках таблицы
  }

  .cell-name {
    min-width: 240px;
  }

  // Имя сжимается (ellipsis) раньше метки заявки.
  .name-cell {
    display: flex;
    align-items: center;
    gap: var(--tr-space-xs);
    min-width: 0;
  }

  .name-text {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .cell-numeric {
    font-variant-numeric: tabular-nums;
  }

  .cell-status {
    width: 128px;
  }

  .cell-toner {
    width: 80px;
  }

  // Кебаб виден всегда: аудитория раздела часто работает с сенсорных устройств.
  .cell-actions {
    width: 40px;
    text-align: center;
    overflow: visible;
  }
</style>

<script lang="ts">
  // Плитка устройства сотрудника для узкого экрана (41.6, D-21, UI-SPEC §16).
  // Переключение таблица/плитки выполняет страница; медиа-запросов здесь нет.
  // Одинарный клик по телу открывает карточку (двойной тап конфликтует с масштабом).
  import Badge from '@trackly/kit/components/Badge.svelte';
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
  let bodyEl = $state<HTMLElement | null>(null);

  const tone = $derived(statusTone(row.statusCode));
  const typeModel = $derived([row.typeName, row.model].filter((v) => v && v.trim()).join(' · '));
  const inventory = $derived((row.inventoryNumber ?? '').trim());
  const serial = $derived((row.serialNumber ?? '').trim());
  const place = $derived((row.placeFull ?? '').trim());
  const armText = $derived(row.armNames.join(', '));

  // Происхождение принтера считает сервер (D-05): клиент рисует то, что пришло.
  const origin = $derived.by(() => {
    if (row.printerOrigin === 'usb') return { label: 'USB', accent: true };
    if (row.printerOrigin === 'link') return { label: 'Ссылка', accent: false };
    if (row.printerOrigin === 'member') return { label: 'В составе', accent: false };
    return null;
  });

  function handleClick(e: MouseEvent) {
    if (isRowOverlayEvent(e.target)) return;
    // Клик по кебабу и меткам-ссылкам карточку не открывает.
    if ((e.target as HTMLElement | null)?.closest?.('button, a')) return;
    onOpen(row);
  }

  function handleContextMenu(e: MouseEvent) {
    if (isRowOverlayEvent(e.target)) return;
    e.preventDefault();
    if (e.clientX === 0 && e.clientY === 0 && bodyEl) {
      menuRef?.openFromElement(bodyEl);
    } else {
      menuRef?.openAt(e.clientX, e.clientY);
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (!isOwnKeyEvent(e.target, e.currentTarget)) return;
    if (e.key === 'Enter') {
      e.preventDefault();
      onOpen(row);
    } else if (e.key === 'ContextMenu' || (e.key === 'F10' && e.shiftKey)) {
      e.preventDefault();
      if (bodyEl) menuRef?.openFromElement(bodyEl);
    }
  }
</script>

<li class="tile">
  <div
    class="tile-body"
    role="button"
    tabindex="0"
    aria-haspopup="menu"
    aria-label={row.name}
    bind:this={bodyEl}
    onclick={handleClick}
    oncontextmenu={handleContextMenu}
    onkeydown={handleKeydown}
  >
    <div class="tile-top">
      <PrinterMark isPrinter={row.isPrinter} />
      <span class="tile-name">{row.name}</span>
      <EmployeeDeviceMenu
        bind:this={menuRef}
        {row}
        trigger="button"
        variant="ghost-md"
        onOpen={() => onOpen(row)}
        onRepair={() => onRepair(row)}
        onCartridge={() => onCartridge(row)}
        {onGoto}
        onCopy={() => onCopy(row)}
      />
    </div>
    {#if typeModel}<div class="tile-sub">{typeModel}</div>{/if}
    <div class="tile-line">
      <Badge variant={tone.variant} appearance={tone.appearance} muted={tone.muted}
        >{row.statusName}</Badge
      >
      <RequestLabels myOpenRequests={row.myOpenRequests} otherOpenRequest={row.otherOpenRequest} />
      {#if row.isPrinter}<TonerMini toner={row.toner} />{/if}
    </div>
    {#if inventory || serial || place || armText || origin}
      <dl class="tile-fields">
        {#if inventory}
          <dt>Инв. №</dt>
          <dd><span class="tr-mono">{inventory}</span></dd>
        {/if}
        {#if serial}
          <dt>Серийный №</dt>
          <dd><span class="tr-mono">{serial}</span></dd>
        {/if}
        {#if place}
          <dt>Место</dt>
          <dd>{place}</dd>
        {/if}
        {#if armText || origin}
          <dt>АРМ</dt>
          <dd>
            {#if origin}
              <Badge variant={origin.accent ? 'accent' : 'default'} appearance="soft" size="sm"
                >{origin.label}</Badge
              >
            {/if}
            {armText}
          </dd>
        {/if}
      </dl>
    {/if}
  </div>
</li>

<style lang="scss">
  .tile {
    list-style: none;
    padding: var(--tr-space-sm) var(--tr-space-md);
    background: var(--tr-surface-raised);
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-md);
    text-align: left;
    white-space: normal;
    overflow-wrap: anywhere;
  }

  .tile-body {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-xs);
    cursor: pointer;
  }

  .tile-top {
    display: flex;
    align-items: center;
    gap: var(--tr-space-xs);
  }

  .tile-name {
    flex: 1;
    min-width: 0;
    font-size: var(--tr-font-size-body);
    font-weight: var(--tr-font-weight-semibold);
    color: var(--tr-text-primary);
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .tile-sub {
    font-size: var(--tr-font-size-label);
    font-weight: var(--tr-font-weight-medium);
    color: var(--tr-text-secondary);
  }

  .tile-line {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--tr-space-2xs) var(--tr-space-xs);
  }

  .tile-fields {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: var(--tr-space-2xs) var(--tr-space-sm);
    margin: 0;
  }

  dt {
    font-size: var(--tr-font-size-caption);
    color: var(--tr-text-tertiary);
  }

  dd {
    margin: 0;
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-primary);
    min-width: 0;
  }
</style>

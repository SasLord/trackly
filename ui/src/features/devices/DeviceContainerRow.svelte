<script lang="ts">
  // Строка АРМ (контейнерной группы) в разделе «Устройства» (фаза 41.9,
  // GRD-07, D-01..D-03, D-11; UI-SPEC §7). Управления группой в меню нет.
  // Охрана событий: isRowOverlayEvent первым оператором обработчиков
  // contextmenu/dblclick, isOwnKeyEvent в keydown (гейт check-device-row-menu).
  import { push } from 'svelte-spa-router';
  import ActionMenu from '$lib/components/ActionMenu.svelte';
  import Badge from '$lib/components/Badge.svelte';
  import TableRow from '$lib/components/TableRow.svelte';
  import RowChevron from '$lib/components/icons/RowChevron.svelte';
  import GroupLoadRestButton from './GroupLoadRestButton.svelte';
  import { isOwnKeyEvent, isRowOverlayEvent } from './deviceRowMenu';
  import type { ContainerRowDto } from '../../bindings';

  interface LoadRest {
    shown: number;
    total: number;
    loading: boolean;
    onclick: () => void;
  }

  interface Props {
    container: ContainerRowDto;
    expanded: boolean;
    /** Глубина вложенности 0..3 (D-11). */
    depth?: number;
    showStatus?: boolean;
    isLastInGroup?: boolean;
    onToggleExpand: (_groupId: number) => void;
    loadRest?: LoadRest | null;
  }

  const {
    container,
    expanded,
    depth = 0,
    showStatus = true,
    isLastInGroup = false,
    onToggleExpand,
    loadRest = null,
  }: Props = $props();

  let menuRef = $state<ReturnType<typeof ActionMenu> | null>(null);
  let firstCellEl = $state<HTMLElement | null>(null);

  function openGroup() {
    void push('#/groups?id=' + container.group_id);
  }

  function stopClick(e: MouseEvent) {
    e.stopPropagation();
  }

  function handleChevronClick(e: MouseEvent) {
    e.stopPropagation();
    onToggleExpand(container.group_id);
  }

  function openMenuFromCell() {
    if (!firstCellEl) return;
    const rect = firstCellEl.getBoundingClientRect();
    menuRef?.openAt(rect.left, rect.bottom);
  }

  function handleContextMenu(e: MouseEvent) {
    if (isRowOverlayEvent(e.target)) return;
    e.preventDefault();
    if (e.clientX === 0 && e.clientY === 0) {
      openMenuFromCell();
    } else {
      menuRef?.openAt(e.clientX, e.clientY);
    }
  }

  function handleDblClick(e: MouseEvent) {
    if (isRowOverlayEvent(e.target)) return;
    // Двойной клик по шеврону, ссылке, кнопке догрузки или «⋯» группу не открывает.
    if ((e.target as HTMLElement | null)?.closest?.('button, a')) return;
    window.getSelection()?.removeAllRanges();
    openGroup();
  }

  function handleCellKeydown(e: KeyboardEvent) {
    if (!isOwnKeyEvent(e.target, e.currentTarget)) return;
    if (e.key === 'Enter') {
      e.preventDefault();
      openGroup();
    } else if (e.key === 'ContextMenu' || (e.key === 'F10' && e.shiftKey)) {
      e.preventDefault();
      openMenuFromCell();
    }
  }
</script>

<TableRow
  class={isLastInGroup ? 'group-last-child' : undefined}
  {depth}
  oncontextmenu={handleContextMenu}
  ondblclick={handleDblClick}
>
  <td
    id="group-row-{container.group_id}"
    class="cell cell-name"
    title={container.name}
    role="button"
    tabindex="0"
    aria-haspopup="menu"
    bind:this={firstCellEl}
    onkeydown={handleCellKeydown}
  >
    <span class="anchor-cell">
      <RowChevron {expanded} onclick={handleChevronClick} />
      <span class="anchor-name">{container.name}</span>
      <a
        class="anchor-link"
        href={'#/groups?id=' + container.group_id}
        title={'Открыть группу «' + container.name + '»'}
        aria-label={'Открыть группу «' + container.name + '»'}
        onclick={stopClick}
      >
        <Badge variant="accent" appearance="soft" size="sm">Группа</Badge>
      </a>
      <span
        class="anchor-count"
        title={'Устройств в составе, включая вложенные группы: ' + container.device_count}
      >
        <Badge variant="accent" appearance="count" size="sm">
          <span class="tr-mono">{container.device_count} шт.</span>
        </Badge>
      </span>
      {#if loadRest}
        <GroupLoadRestButton
          groupName={container.name}
          shown={loadRest.shown}
          total={loadRest.total}
          loading={loadRest.loading}
          onclick={loadRest.onclick}
        />
      {/if}
    </span>
  </td>
  <td class="cell cell-numeric">—</td>
  <td class="cell cell-numeric">—</td>
  <td class="cell">—</td>
  <td class="cell" title={container.place_path_full ?? ''}>{container.place_path_short ?? '—'}</td>
  <td class="cell">—</td>
  {#if showStatus}
    <td class="cell cell-status">—</td>
  {/if}
  <td class="cell cell-actions">
    <ActionMenu
      bind:this={menuRef}
      variant="ghost-sm"
      label={'Действия с группой «' + container.name + '»'}
    >
      <button type="button" role="menuitem" onclick={openGroup}>Открыть группу</button>
      <button type="button" role="menuitem" onclick={() => onToggleExpand(container.group_id)}>
        {expanded ? 'Свернуть' : 'Раскрыть'}
      </button>
    </ActionMenu>
  </td>
</TableRow>

<style lang="scss">
  :global(tr.group-last-child) > .cell {
    border-bottom: 2px solid var(--tr-border-strong);
  }

  .cell {
    font-size: var(--tr-font-size-body);
    color: var(--tr-text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 0;
  }

  .cell-name {
    width: 25%;
    max-width: 200px;
  }

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

  .anchor-count {
    display: inline-flex;
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

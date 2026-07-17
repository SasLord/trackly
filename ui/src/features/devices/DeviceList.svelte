<script lang="ts">
  import DeviceListRow from './DeviceListRow.svelte';
  import DeviceGroupRow from './DeviceGroupRow.svelte';
  import type { DeviceDto, DeviceGroup } from '../../bindings';

  interface Props {
    items: DeviceDto[];
    groups: DeviceGroup[];
    total: number;
    loading: boolean;
    grouped: boolean;
    searchActive: boolean;
    /** Set of stable group keys that should be rendered expanded. */
    expandedGroups?: Set<string>;
    /** Called when a group's expansion state toggles. */
    onExpandToggle?: (_key: string, _expanded: boolean) => void;
    onEdit: (_d: DeviceDto) => void;
    onDelete: () => void;
    /** Plan 03-05 (DEV-14): pass-through. */
    onPrintAcceptance?: (_d: DeviceDto) => void;
    /** ITEM-3: when true, shows the «Статус» column. Hide on filtered status tabs. */
    showStatus?: boolean;
  }

  const {
    items,
    groups,
    total,
    loading,
    grouped,
    searchActive,
    expandedGroups,
    onExpandToggle,
    onEdit,
    onDelete,
    onPrintAcceptance,
    showStatus = true,
  }: Props = $props();

  const showGroups = $derived(grouped && !searchActive && groups.length > 0);
  const isEmpty = $derived(!loading && (showGroups ? groups.length === 0 : items.length === 0));

  // In grouped mode, singletons (count == 1) render as plain DeviceListRow.
  // Groups with count > 1 render as expandable DeviceGroupRow.
  // This ensures every device is visible in grouped mode — no vanishing singletons.

  const emptyMessage = $derived(
    searchActive ? 'По вашему запросу ничего не найдено' : 'Устройств пока нет',
  );
  const emptySubtext = $derived(
    searchActive
      ? 'Попробуйте изменить поисковый запрос или сбросить фильтр статуса.'
      : 'Создайте первое устройство или импортируйте список из CSV.',
  );
</script>

<div class="device-list-wrapper">
  {#if loading && items.length === 0 && groups.length === 0}
    <!-- Skeleton rows while initial load -->
    <table class="device-table">
      <thead>
        <tr class="header-row">
          <th class="th th-name">Наименование</th>
          <th class="th th-numeric">Инвентарный №</th>
          <th class="th th-numeric">Серийный №</th>
          <th class="th">Модель</th>
          <th class="th">Расположение</th>
          <th class="th th-condition">Состояние</th>
          {#if showStatus}<th class="th th-status">Статус</th>{/if}
          <th class="th th-actions">Действия</th>
        </tr>
      </thead>
      <tbody>
        {#each { length: 5 } as _}
          <tr class="skeleton-row">
            {#each { length: showStatus ? 8 : 7 } as _}
              <td class="skeleton-cell">
                <div class="skeleton-block"></div>
              </td>
            {/each}
          </tr>
        {/each}
      </tbody>
    </table>
  {:else if isEmpty}
    <div class="empty-state">
      <p class="empty-title">{emptyMessage}</p>
      <p class="empty-body">{emptySubtext}</p>
    </div>
  {:else}
    <table class="device-table">
      <thead>
        <tr class="header-row">
          <th class="th th-name">Наименование</th>
          <th class="th th-numeric">Инвентарный №</th>
          <th class="th th-numeric">Серийный №</th>
          <th class="th">Модель</th>
          <th class="th">Расположение</th>
          <th class="th th-condition">Состояние</th>
          {#if showStatus}<th class="th th-status">Статус</th>{/if}
          <th class="th th-actions">Действия</th>
        </tr>
      </thead>
      <tbody>
        {#if showGroups}
          {#each groups as group (group.repr.id)}
            {#if group.count > 1}
              <!-- Multi-device group: expandable row with chevron and count badge -->
              <DeviceGroupRow
                {group}
                {onEdit}
                {onDelete}
                expandedGroups={expandedGroups ?? new Set()}
                {onExpandToggle}
                {onPrintAcceptance}
                {showStatus}
              />
            {:else}
              <!-- Singleton group (count == 1): render as plain row, no chevron -->
              <DeviceListRow
                device={group.repr}
                {onEdit}
                {onDelete}
                {onPrintAcceptance}
                {showStatus}
              />
            {/if}
          {/each}
        {:else}
          {#each items as device (device.id)}
            <DeviceListRow {device} {onEdit} {onDelete} {onPrintAcceptance} {showStatus} />
          {/each}
        {/if}
      </tbody>
    </table>

    <footer class="list-footer">
      <span class="pagination-info">
        {#if showGroups}
          Групп: {groups.length}
        {:else}
          Показано {items.length} из {total}
        {/if}
      </span>
    </footer>
  {/if}
</div>

<style lang="scss">
  .device-list-wrapper {
    width: 100%;
    overflow-x: auto;
  }

  .device-table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--tr-font-size-body);
    table-layout: auto;
  }

  .header-row {
    border-bottom: 2px solid var(--tr-border-strong);
  }

  .th {
    padding: var(--tr-space-2xs) var(--tr-space-xs);
    text-align: left;
    font-size: var(--tr-font-size-label);
    font-weight: var(--tr-font-weight-semibold);
    color: var(--tr-text-secondary);
    white-space: nowrap;
    background: var(--tr-bg);
  }

  .th-name {
    width: 25%;
  }
  .th-numeric {
    width: 140px;
  }
  .th-condition {
    width: 120px;
  }
  .th-status {
    width: 120px;
  }
  .th-actions {
    width: 40px;
  }

  // Empty state
  .empty-state {
    padding: var(--tr-space-2xl) var(--tr-space-xl);
    text-align: center;
  }

  .empty-title {
    margin: 0 0 var(--tr-space-2xs);
    font-size: var(--tr-font-size-h3);
    font-weight: var(--tr-font-weight-semibold);
    color: var(--tr-text-primary);
  }

  .empty-body {
    margin: 0;
    color: var(--tr-text-secondary);
    font-size: var(--tr-font-size-body);
  }

  // Skeleton
  .skeleton-row {
    height: var(--row-height, 40px);
  }

  .skeleton-cell {
    padding: 0 var(--tr-space-xs);
    border-bottom: 1px solid var(--tr-border);
  }

  .skeleton-block {
    height: 16px;
    border-radius: var(--tr-radius-xs);
    background: var(--tr-surface-sunken);
    animation: pulse 1.2s ease-in-out infinite;
  }

  @keyframes pulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.4;
    }
  }

  // Footer
  .list-footer {
    padding: var(--tr-space-xs) var(--tr-space-md);
    border-top: 1px solid var(--tr-border);
    display: flex;
    align-items: center;
    justify-content: flex-start;
  }

  .pagination-info {
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-secondary);
  }
</style>

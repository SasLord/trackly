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
          <th class="th th-status">Статус</th>
          <th class="th th-actions">Действия</th>
        </tr>
      </thead>
      <tbody>
        {#each { length: 5 } as _}
          <tr class="skeleton-row">
            {#each { length: 7 } as _}
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
          <th class="th th-status">Статус</th>
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
              />
            {:else}
              <!-- Singleton group (count == 1): render as plain row, no chevron -->
              <DeviceListRow device={group.repr} {onEdit} {onDelete} />
            {/if}
          {/each}
        {:else}
          {#each items as device (device.id)}
            <DeviceListRow {device} {onEdit} {onDelete} />
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
    font-size: var(--font-size-body);
    table-layout: auto;
  }

  .header-row {
    border-bottom: 2px solid var(--color-border-strong);
  }

  .th {
    padding: var(--space-xs) var(--space-sm);
    text-align: left;
    font-size: var(--font-size-label);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-secondary);
    white-space: nowrap;
    background: var(--color-bg);
  }

  .th-name {
    width: 25%;
  }
  .th-numeric {
    width: 140px;
  }
  .th-status {
    width: 120px;
  }
  .th-actions {
    width: 40px;
  }

  // Empty state
  .empty-state {
    padding: var(--space-xl) var(--space-lg);
    text-align: center;
  }

  .empty-title {
    margin: 0 0 var(--space-xs);
    font-size: var(--font-size-heading);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
  }

  .empty-body {
    margin: 0;
    color: var(--color-text-secondary);
    font-size: var(--font-size-body);
  }

  // Skeleton
  .skeleton-row {
    height: var(--row-height, 40px);
  }

  .skeleton-cell {
    padding: 0 var(--space-sm);
    border-bottom: 1px solid var(--color-border);
  }

  .skeleton-block {
    height: 16px;
    border-radius: var(--radius-sm);
    background: var(--color-surface-sunken);
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
    padding: var(--space-sm) var(--space-md);
    border-top: 1px solid var(--color-border);
    display: flex;
    align-items: center;
    justify-content: flex-start;
  }

  .pagination-info {
    font-size: var(--font-size-label);
    color: var(--color-text-secondary);
  }
</style>

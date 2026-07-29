<script lang="ts">
  import Table from '$lib/components/Table.svelte';
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
  // Mirrors the exact skeleton-branch condition of the pre-migration 3-way
  // if/else — used both as Table's `loading` prop and to gate the footer
  // (footer only rendered in the "real table" branch, matching prior behavior).
  const skeletonLoading = $derived(loading && items.length === 0 && groups.length === 0);

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

{#snippet tableHead()}
  <th class="th-name">Наименование</th>
  <th class="th-numeric">Инвентарный №</th>
  <th class="th-numeric">Серийный №</th>
  <th>Модель</th>
  <th>Расположение</th>
  <th class="th-condition">Состояние</th>
  {#if showStatus}<th class="th-status">Статус</th>{/if}
  <th class="th-actions">Действия</th>
{/snippet}

{#snippet footer()}
  {#if !skeletonLoading && !isEmpty}
    <!-- Content only — Table.svelte already wraps this in <footer class="tr-table-footer">
         with its own border-top + padding. A second wrapping <footer> here produced the
         doubled, over-tall footer with two top borders. -->
    <span class="pagination-info">
      {#if showGroups}
        Групп: {groups.length}
      {:else}
        Показано {items.length} из {total}
      {/if}
    </span>
  {/if}
{/snippet}

<Table
  columns={showStatus ? 8 : 7}
  loading={skeletonLoading}
  empty={isEmpty}
  emptyTitle={emptyMessage}
  emptyBody={emptySubtext}
  head={tableHead}
  {footer}
  minWidth="860px"
  fillHeight
>
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
        <DeviceListRow device={group.repr} {onEdit} {onDelete} {onPrintAcceptance} {showStatus} />
      {/if}
    {/each}
  {:else}
    {#each items as device (device.id)}
      <DeviceListRow {device} {onEdit} {onDelete} {onPrintAcceptance} {showStatus} />
    {/each}
  {/if}
</Table>

<style lang="scss">
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

  .pagination-info {
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-secondary);
  }
</style>

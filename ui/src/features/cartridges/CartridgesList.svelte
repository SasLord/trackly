<script lang="ts">
  // Plan 04-04: master-panel list — рендерит CartridgeListRow × N + footer пагинация
  // + empty/loading states. По образцу ActsList.svelte.
  // Plan 27-04 (D-03): rebuilt on shared Table/TableRow primitives per
  // ActsList.svelte/DeviceList.svelte precedent — bespoke .rows/.loading/.empty/
  // .pagination removed, Table now owns the frame/skeleton/empty-state.
  import Spinner from '$lib/components/Spinner.svelte';
  import Button from '$lib/components/Button.svelte';
  import Table from '$lib/components/Table.svelte';
  import CartridgeListRow from './CartridgeListRow.svelte';
  import type { CartridgeDto } from '../../bindings';

  interface Props {
    items: CartridgeDto[];
    total: number;
    loading: boolean;
    selectedId: number | null;
    hasFilter: boolean;
    /** Список отфильтрован по конкретному статусу — скрыть колонку статуса в строках. */
    statusFiltered?: boolean;
    onSelect: (_id: number) => void;
    onMenuAction: (_op: string, _cartridge: CartridgeDto) => void;
    onCreate: () => void;
  }

  const {
    items,
    total,
    loading,
    selectedId,
    hasFilter,
    statusFiltered = false,
    onSelect,
    onMenuAction,
    onCreate,
  }: Props = $props();

  // Empty state config per UI-SPEC §Пустые состояния.
  const emptyConfig = $derived.by(() => {
    if (hasFilter) {
      return {
        heading: 'Ничего не найдено',
        body: 'Попробуйте изменить фильтры или поисковый запрос.',
        actionLabel: null as string | null,
      };
    }
    return {
      heading: 'Картриджей пока нет',
      body: 'Добавьте первый картридж, чтобы начать отслеживать расходники.',
      actionLabel: '+ Добавить картридж/фотобарабан',
    };
  });

  const skeletonLoading = $derived(loading && items.length === 0);
  const isEmpty = $derived(!loading && items.length === 0);
  const columnCount = $derived(statusFiltered ? 4 : 5);
</script>

{#snippet tableHead()}
  <th class="th-code">Код</th>
  <th>Модель</th>
  <th>Расположение</th>
  {#if !statusFiltered}<th class="th-status">Статус</th>{/if}
  <th class="th-actions">Действия</th>
{/snippet}

{#snippet footer()}
  {#if !skeletonLoading}
    {#if isEmpty && emptyConfig.actionLabel}
      <div class="empty-action">
        <Button variant="primary" onclick={onCreate}>{emptyConfig.actionLabel}</Button>
      </div>
    {:else if !isEmpty}
      <footer class="list-footer">
        <span class="pager-info">
          {items.length === 0 ? '0' : `1–${items.length}`} из {total}
        </span>
        {#if loading}
          <Spinner size="sm" />
        {/if}
      </footer>
    {/if}
  {/if}
{/snippet}

<Table
  columns={columnCount}
  loading={skeletonLoading}
  empty={isEmpty}
  emptyTitle={emptyConfig.heading}
  emptyBody={emptyConfig.body}
  head={tableHead}
  {footer}
  framed={false}
  fillHeight
>
  {#each items as c (c.id)}
    <CartridgeListRow
      cartridge={c}
      selected={c.id === selectedId}
      {statusFiltered}
      onSelect={() => onSelect(c.id)}
      {onMenuAction}
    />
  {/each}
</Table>

<style lang="scss">
  .th-code {
    width: 140px;
  }
  .th-status {
    width: 120px;
  }
  .th-actions {
    width: 40px;
  }

  .empty-action {
    display: flex;
    justify-content: center;
  }

  .list-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .pager-info {
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-secondary);
    font-variant-numeric: tabular-nums;
  }
</style>

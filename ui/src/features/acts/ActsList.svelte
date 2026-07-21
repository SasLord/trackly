<script lang="ts">
  // Plan 03-02: master-panel list — рендерит ActListRow × N + footer пагинация
  // + empty/loading states.
  // Plan 27-02 (D-03): rebuilt on shared Table/TableRow primitives per
  // DeviceList.svelte precedent — bespoke .rows/.loading/.empty/.pagination
  // removed, Table now owns the frame/skeleton/empty-state.
  import Spinner from '$lib/components/Spinner.svelte';
  import Button from '$lib/components/Button.svelte';
  import Table from '$lib/components/Table.svelte';
  import ActListRow from './ActListRow.svelte';
  import type { ActDto } from '../../bindings';

  type TabKey = 'handover' | 'returns' | 'archive';

  interface Props {
    items: ActDto[];
    total: number;
    loading: boolean;
    selectedActId: number | null;
    activeTab: TabKey;
    searchQuery: string;
    onSelect: (_id: number) => void;
    onCreate: () => void;
    onResetSearch: () => void;
  }

  const {
    items,
    total,
    loading,
    selectedActId,
    activeTab,
    searchQuery,
    onSelect,
    onCreate,
    onResetSearch,
  }: Props = $props();

  const searchActive = $derived(searchQuery.trim().length > 0);

  // Mirrors DeviceList's skeleton-branch condition — Table shows skeleton rows
  // only for the initial (items-still-empty) load; a background refresh with
  // items already on screen keeps rendering real rows + a footer spinner.
  const skeletonLoading = $derived(loading && items.length === 0);
  const isEmpty = $derived(!loading && items.length === 0);

  // Empty state per UI-SPEC §ActsList.
  const emptyConfig = $derived.by(() => {
    if (searchActive) {
      return {
        heading: 'Ничего не найдено',
        body: `По запросу «${searchQuery}» ничего не нашлось. Проверьте написание или сбросьте поиск.`,
        actionLabel: 'Сбросить поиск',
        actionKind: 'link' as const,
      };
    }
    if (activeTab === 'returns') {
      return {
        heading: 'Возвратов пока нет',
        body: 'Возвраты появятся, когда какие-то устройства вернутся на склад.',
        actionLabel: null,
        actionKind: null,
      };
    }
    if (activeTab === 'archive') {
      return {
        heading: 'Архив пуст',
        body: 'Сюда попадают акты после полного возврата всех устройств.',
        actionLabel: null,
        actionKind: null,
      };
    }
    return {
      heading: 'Актов пока нет',
      body: 'Создайте первый акт приёма-передачи.',
      actionLabel: '+ Создать акт',
      actionKind: 'primary' as const,
    };
  });

  function handleAction() {
    if (emptyConfig.actionKind === 'link') {
      onResetSearch();
    } else if (emptyConfig.actionKind === 'primary') {
      onCreate();
    }
  }
</script>

{#snippet tableHead()}
  <th class="th-number">№</th>
  <th>Дата</th>
  <th>Получатель</th>
  <th class="th-count">Позиций</th>
{/snippet}

{#snippet footer()}
  {#if !skeletonLoading}
    {#if isEmpty && emptyConfig.actionLabel}
      <div class="empty-action">
        {#if emptyConfig.actionKind === 'primary'}
          <Button variant="primary" onclick={handleAction}>{emptyConfig.actionLabel}</Button>
        {:else}
          <Button variant="link" onclick={handleAction}>{emptyConfig.actionLabel}</Button>
        {/if}
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
  columns={4}
  loading={skeletonLoading}
  empty={isEmpty}
  emptyTitle={emptyConfig.heading}
  emptyBody={emptyConfig.body}
  head={tableHead}
  {footer}
  framed={false}
  fillHeight
>
  {#each items as act (act.id)}
    <ActListRow
      {act}
      selected={act.id === selectedActId}
      showArchivedBadge={activeTab === 'archive'}
      {onSelect}
    />
  {/each}
</Table>

<style lang="scss">
  .th-number {
    width: 72px;
  }
  .th-count {
    width: 90px;
    text-align: right;
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

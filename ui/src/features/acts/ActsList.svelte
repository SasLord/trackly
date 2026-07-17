<script lang="ts">
  // Plan 03-02: master-panel list — рендерит ActListRow × N + footer пагинация
  // + empty/loading states.
  import Spinner from '$lib/components/Spinner.svelte';
  import Button from '$lib/components/Button.svelte';
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

<div class="acts-list">
  {#if loading && items.length === 0}
    <div class="loading">
      <Spinner size="md" />
    </div>
  {:else if items.length === 0}
    <div class="empty">
      <h3 class="empty-heading">{emptyConfig.heading}</h3>
      <p class="empty-body">{emptyConfig.body}</p>
      {#if emptyConfig.actionLabel && emptyConfig.actionKind === 'primary'}
        <Button variant="primary" onclick={handleAction}>{emptyConfig.actionLabel}</Button>
      {:else if emptyConfig.actionLabel && emptyConfig.actionKind === 'link'}
        <Button variant="link" onclick={handleAction}>{emptyConfig.actionLabel}</Button>
      {/if}
    </div>
  {:else}
    <div class="rows">
      {#each items as act (act.id)}
        <ActListRow
          {act}
          selected={act.id === selectedActId}
          showArchivedBadge={activeTab === 'archive'}
          {onSelect}
        />
      {/each}
    </div>
    <footer class="pagination">
      <span class="pager-info">
        {items.length === 0 ? '0' : `1–${items.length}`} из {total}
      </span>
      {#if loading}
        <Spinner size="sm" />
      {/if}
    </footer>
  {/if}
</div>

<style lang="scss">
  .acts-list {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--tr-surface);
  }
  .rows {
    flex: 1;
    overflow: auto;
  }
  .loading,
  .empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--tr-space-xs);
    padding: var(--tr-space-4xl);
    text-align: center;
  }
  .empty-heading {
    margin: 0 0 var(--tr-space-2xs);
    font-size: var(--font-size-heading);
    font-weight: var(--font-weight-semibold);
    color: var(--tr-text-primary);
  }
  .empty-body {
    margin: 0 0 var(--tr-space-md);
    color: var(--tr-text-secondary);
    font-size: var(--font-size-body);
  }

  .pagination {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--tr-space-xs) var(--tr-space-md);
    border-top: 1px solid var(--tr-border);
    font-size: var(--font-size-label);
    color: var(--tr-text-secondary);
    background: var(--tr-surface);
  }
  .pager-info {
    font-variant-numeric: tabular-nums;
  }
</style>

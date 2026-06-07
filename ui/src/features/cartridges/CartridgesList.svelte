<script lang="ts">
  // Plan 04-04: master-panel list — рендерит CartridgeListRow × N + footer пагинация
  // + empty/loading states. По образцу ActsList.svelte.
  import Spinner from '$lib/components/Spinner.svelte';
  import Button from '$lib/components/Button.svelte';
  import CartridgeListRow from './CartridgeListRow.svelte';
  import type { CartridgeDto } from '../../bindings';

  interface Props {
    items: CartridgeDto[];
    total: number;
    loading: boolean;
    selectedId: number | null;
    hasFilter: boolean;
    onSelect: (_id: number) => void;
    onMenuAction: (_op: string, _cartridge: CartridgeDto) => void;
    onCreate: () => void;
  }

  const { items, total, loading, selectedId, hasFilter, onSelect, onMenuAction, onCreate }: Props =
    $props();

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
      actionLabel: '+ Добавить картридж',
    };
  });
</script>

<div class="cartridges-list">
  {#if loading && items.length === 0}
    <div class="loading">
      <Spinner size="md" />
    </div>
  {:else if items.length === 0}
    <div class="empty">
      <h3 class="empty-heading">{emptyConfig.heading}</h3>
      <p class="empty-body">{emptyConfig.body}</p>
      {#if emptyConfig.actionLabel}
        <Button variant="primary" onclick={onCreate}>{emptyConfig.actionLabel}</Button>
      {/if}
    </div>
  {:else}
    <div class="rows">
      {#each items as c (c.id)}
        <CartridgeListRow
          cartridge={c}
          selected={c.id === selectedId}
          onSelect={() => onSelect(c.id)}
          {onMenuAction}
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
  .cartridges-list {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--color-surface);
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
    gap: var(--space-sm);
    padding: var(--space-2xl);
    text-align: center;
  }

  .empty-heading {
    margin: 0 0 var(--space-xs);
    font-size: var(--font-size-heading);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
  }

  .empty-body {
    margin: 0 0 var(--space-md);
    color: var(--color-text-secondary);
    font-size: var(--font-size-body);
  }

  .pagination {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-sm) var(--space-md);
    border-top: 1px solid var(--color-border);
    font-size: var(--font-size-label);
    color: var(--color-text-secondary);
    background: var(--color-surface);
  }

  .pager-info {
    font-variant-numeric: tabular-nums;
  }
</style>

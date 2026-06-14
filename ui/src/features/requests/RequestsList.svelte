<script lang="ts">
  // Plan 06-05: master-panel list — по паттерну CartridgesList.svelte.
  import Spinner from '$lib/components/Spinner.svelte';
  import Button from '$lib/components/Button.svelte';
  import RequestListRow from './RequestListRow.svelte';
  import type { RequestDto } from '../../bindings-phase6';

  interface EmptyConfig {
    heading: string;
    body: string;
    actionLabel: string | null;
    onAction?: () => void;
  }

  interface Props {
    items: RequestDto[];
    loading: boolean;
    selectedId: number | null;
    emptyConfig: EmptyConfig;
    onSelect: (_id: number) => void;
  }

  const { items, loading, selectedId, emptyConfig, onSelect }: Props = $props();
</script>

<div class="requests-list">
  {#if loading && items.length === 0}
    <div class="loading">
      <Spinner size="md" />
    </div>
  {:else if items.length === 0}
    <div class="empty">
      <h3 class="empty-heading">{emptyConfig.heading}</h3>
      <p class="empty-body">{emptyConfig.body}</p>
      {#if emptyConfig.actionLabel && emptyConfig.onAction}
        <Button variant="primary" onclick={emptyConfig.onAction}>{emptyConfig.actionLabel}</Button>
      {/if}
    </div>
  {:else}
    <div class="rows">
      {#each items as r (r.id)}
        <RequestListRow
          request={r}
          selected={r.id === selectedId}
          onclick={() => onSelect(r.id)}
        />
      {/each}
    </div>
    <footer class="pagination">
      <span class="pager-info">{items.length} записей</span>
      {#if loading}
        <Spinner size="sm" />
      {/if}
    </footer>
  {/if}
</div>

<style lang="scss">
  .requests-list {
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

<script lang="ts">
  // Plan 06-04: master-panel list — рендерит PrinterListRow × N + empty/loading states.
  // По паттерну CartridgesList.svelte + emptyConfig паттерн (06-PATTERNS.md §PrintersList).
  import Spinner from '$lib/components/Spinner.svelte';
  import Button from '$lib/components/Button.svelte';
  import PrinterListRow from './PrinterListRow.svelte';
  import type { PrinterDto } from '../../bindings-phase6';

  interface EmptyConfig {
    heading: string;
    body: string;
    actionLabel: string | null;
    onAction?: () => void;
  }

  interface Props {
    items: PrinterDto[];
    loading: boolean;
    selectedId: number | null;
    onSelect: (_id: number) => void;
    emptyConfig: EmptyConfig;
  }

  const { items, loading, selectedId, onSelect, emptyConfig }: Props = $props();
</script>

<div class="printers-list">
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
      {#each items as p (p.id)}
        <PrinterListRow printer={p} selected={p.id === selectedId} onclick={() => onSelect(p.id)} />
      {/each}
    </div>
    <footer class="footer">
      <span class="pager-info" style="font-variant-numeric: tabular-nums">
        {items.length} принт.
      </span>
      {#if loading}
        <Spinner size="sm" />
      {/if}
    </footer>
  {/if}
</div>

<style lang="scss">
  .printers-list {
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

  .footer {
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

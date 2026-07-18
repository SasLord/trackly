<script lang="ts">
  // Table — reusable shell (header row, loading skeleton, empty state) for
  // TableRow-based tables. See .planning/phases/25-dropdown/25-01-PLAN.md <interfaces>.
  import type { Snippet } from 'svelte';

  interface Props {
    /** Total <th>/<td> count — drives skeleton cell count and empty-state colspan. */
    columns: number;
    loading?: boolean;
    empty?: boolean;
    /** Only rendered when `empty` is true. */
    emptyTitle?: string;
    emptyBody?: string;
    skeletonRows?: number;
    /** Renders the <th> cells of the header row. */
    head: Snippet;
    /** Renders <TableRow>-based <tbody> rows; only rendered when !loading && !empty. */
    children?: Snippet;
  }

  const {
    columns,
    loading = false,
    empty = false,
    emptyTitle,
    emptyBody,
    skeletonRows = 5,
    head,
    children,
  }: Props = $props();
</script>

<div class="tr-table-wrapper">
  <table class="tr-table">
    <thead>
      <tr class="tr-thead-row">
        {@render head()}
      </tr>
    </thead>
    {#if loading}
      <tbody>
        {#each { length: skeletonRows } as _}
          <tr class="tr-skeleton-row">
            {#each { length: columns } as _}
              <td class="tr-skeleton-cell">
                <div class="tr-skeleton-block"></div>
              </td>
            {/each}
          </tr>
        {/each}
      </tbody>
    {:else if empty}
      <tbody>
        <tr class="tr-empty-row">
          <td class="tr-empty-cell" colspan={columns}>
            {#if emptyTitle}<p class="tr-empty-title">{emptyTitle}</p>{/if}
            {#if emptyBody}<p class="tr-empty-body">{emptyBody}</p>{/if}
          </td>
        </tr>
      </tbody>
    {:else}
      <tbody>
        {@render children?.()}
      </tbody>
    {/if}
  </table>
</div>

<style lang="scss">
  .tr-table-wrapper {
    width: 100%;
    overflow-x: auto;
  }

  .tr-table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--tr-font-size-body);
    table-layout: auto;
  }

  .tr-thead-row {
    height: 34px;
    border-bottom: 2px solid var(--tr-border-strong);
    background: var(--tr-bg);
  }

  // Caller-supplied <th> cells (rendered by the `head` snippet — a different
  // Svelte scope-hash than this file) need the :global() escape hatch, same
  // reasoning as TableRow's base <td> rule (Plan 25-01 Task 1).
  .tr-thead-row :global(> th) {
    padding: 0 10px;
    text-align: left;
    font-size: var(--tr-font-size-caption);
    font-weight: var(--tr-font-weight-semibold);
    color: var(--tr-text-secondary);
    white-space: nowrap;
  }

  // Table.svelte renders its OWN skeleton <td>s (not TableRow instances), so these
  // are ordinary scoped rules — no :global() needed. Metrics mirror TableRow's base
  // <td> rule (height 40px, padding 0 10px) so the table does not visibly jump when
  // real rows replace the skeleton.
  .tr-skeleton-row {
    height: 40px;
  }

  .tr-skeleton-cell {
    padding: 0 10px;
    border-bottom: 1px solid var(--tr-border);
  }

  .tr-skeleton-block {
    height: 16px;
    border-radius: var(--tr-radius-xs);
    background: var(--tr-surface-sunken);
    animation: tr-table-pulse 1.2s ease-in-out infinite;
  }

  @keyframes tr-table-pulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.4;
    }
  }

  .tr-empty-cell {
    padding: var(--tr-space-2xl) var(--tr-space-xl);
    text-align: center;
  }

  .tr-empty-title {
    margin: 0 0 var(--tr-space-2xs);
    font-size: var(--tr-font-size-h3);
    font-weight: var(--tr-font-weight-semibold);
    color: var(--tr-text-primary);
  }

  .tr-empty-body {
    margin: 0;
    color: var(--tr-text-secondary);
    font-size: var(--tr-font-size-body);
  }
</style>

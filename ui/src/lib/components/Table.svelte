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
    /** Draws border+radius(8px)+shadow frame around the scrollable wrapper. Default true. */
    framed?: boolean;
    /** Optional footer rendered inside the frame, below the scroller. Absent by default. */
    footer?: Snippet;
    /** Scoped min-width for the <table> (e.g. "860px") — forces horizontal scroll
     * on narrow viewports instead of squishing columns. Absent by default so other
     * (narrower) Table consumers are unaffected. */
    minWidth?: string;
    /** Opt-in: makes the table fill its parent's height with a sticky header,
     * an internally-scrolling body, and a footer pinned to the bottom — for use
     * inside fixed-height panels (e.g. master-detail). Default false so other
     * consumers (ActFormItemsTable, витрина) that size to content are unaffected. */
    fillHeight?: boolean;
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
    framed = true,
    footer,
    minWidth,
    fillHeight = false,
  }: Props = $props();
</script>

<div class="tr-table-framed" class:framed class:fill={fillHeight}>
  <div class="tr-table-wrapper">
    <table class="tr-table" style:min-width={minWidth}>
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
  {#if footer}
    <footer class="tr-table-footer">{@render footer()}</footer>
  {/if}
</div>

<style lang="scss">
  .tr-table-framed.framed {
    border: 1px solid var(--tr-border);
    border-radius: 8px;
    overflow: hidden;
    box-shadow: var(--tr-elev-1);
  }

  // Opt-in fillHeight mode (FIX A3): the table stretches to its parent's height
  // instead of sizing to content, so it works inside fixed-height panels
  // (master-detail). Gated behind the `.fill` modifier class so the default path
  // (no fillHeight prop) is byte-identical to before this change.
  .tr-table-framed.fill {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }

  .tr-table-framed.fill .tr-table-wrapper {
    flex: 1 1 auto;
    min-height: 0;
    overflow-y: auto;
  }

  // Sticky header only applies in fillHeight mode — the wrapper is the scroll
  // container, so `position: sticky; top: 0` pins the header row while the body
  // scrolls underneath it. Keeps its existing solid background so body rows
  // don't bleed through while scrolled under the header.
  .tr-table-framed.fill .tr-thead-row {
    position: sticky;
    top: 0;
    z-index: 1;
  }

  .tr-table-footer {
    padding: 9px 14px;
    border-top: 1px solid var(--tr-border);
    font-size: 13px;
    color: var(--tr-text-secondary);
    background: var(--tr-bg);
  }

  .tr-table-wrapper {
    width: 100%;
    overflow-x: auto;
    -webkit-overflow-scrolling: touch;
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

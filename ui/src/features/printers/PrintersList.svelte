<script lang="ts">
  // Plan 06-04: master-panel list — рендерит PrinterListRow × N + empty/loading states.
  // По паттерну CartridgesList.svelte + emptyConfig паттерн (06-PATTERNS.md §PrintersList).
  // Plan 27-07 (D-03): rebuilt on shared Table/TableRow primitives per ActsList.svelte
  // precedent — bespoke .rows/.loading/.empty/.footer removed, Table now owns the
  // frame/skeleton/empty-state.
  import Spinner from '$lib/components/Spinner.svelte';
  import Button from '$lib/components/Button.svelte';
  import Table from '$lib/components/Table.svelte';
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

  // Mirrors ActsList's skeleton-branch condition — Table shows skeleton rows only
  // for the initial (items-still-empty) load; a background refresh with items
  // already on screen keeps rendering real rows + a footer spinner.
  const skeletonLoading = $derived(loading && items.length === 0);
  const isEmpty = $derived(!loading && items.length === 0);

  // GAP-8 (39-UAT.md): scroll a newly-selected row into view — covers both
  // the cross-section focus deep link (PrintersPage resolves `?id=…` into
  // `selectedId` before this list has data) and ordinary row clicks (no-op
  // there since the clicked row is already visible; `block: 'nearest'`
  // avoids an unnecessary jump).
  let scrolledToId = $state<number | null>(null);
  $effect(() => {
    const id = selectedId;
    if (id === null || id === scrolledToId || loading) return;
    const el = document.getElementById(`printer-row-${id}`);
    if (el) {
      el.scrollIntoView({ block: 'nearest' });
      scrolledToId = id;
    }
  });
</script>

{#snippet tableHead()}
  <th class="th-name">Имя</th>
  <th class="th-status">Статус</th>
  <th class="th-toner">Тонер</th>
{/snippet}

{#snippet footer()}
  {#if !skeletonLoading}
    {#if isEmpty && emptyConfig.actionLabel && emptyConfig.onAction}
      <div class="empty-action">
        <Button variant="primary" onclick={emptyConfig.onAction}>{emptyConfig.actionLabel}</Button>
      </div>
    {:else if !isEmpty}
      <footer class="list-footer">
        <span class="pager-info" style="font-variant-numeric: tabular-nums">
          {items.length} принт.
        </span>
        {#if loading}
          <Spinner size="sm" />
        {/if}
      </footer>
    {/if}
  {/if}
{/snippet}

<Table
  columns={3}
  loading={skeletonLoading}
  empty={isEmpty}
  emptyTitle={emptyConfig.heading}
  emptyBody={emptyConfig.body}
  head={tableHead}
  {footer}
  framed={false}
  fillHeight
>
  {#each items as p (p.id)}
    <PrinterListRow printer={p} selected={p.id === selectedId} onclick={() => onSelect(p.id)} />
  {/each}
</Table>

<style lang="scss">
  .th-name {
    width: auto;
  }
  .th-status {
    width: 140px;
  }
  .th-toner {
    width: 160px;
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
  }
</style>

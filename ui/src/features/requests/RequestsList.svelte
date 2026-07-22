<script lang="ts">
  // Plan 06-05: master-panel list — по паттерну CartridgesList.svelte.
  // Plan 28-01 (D-03): rebuilt on shared Table/TableRow primitives per ActsList.svelte
  // precedent — bespoke .rows/.loading/.empty/.pagination removed, Table now owns the
  // frame/skeleton/empty-state. No pagination — this list never had one, footer only
  // shows the record count (+ spinner while a background refresh is loading).
  import Spinner from '$lib/components/Spinner.svelte';
  import Button from '$lib/components/Button.svelte';
  import Table from '$lib/components/Table.svelte';
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

  const skeletonLoading = $derived(loading && items.length === 0);
  const isEmpty = $derived(!loading && items.length === 0);
</script>

{#snippet tableHead()}
  <th>Тип</th>
  <th>Описание</th>
  <th>Автор</th>
  <th class="th-status">Статус</th>
{/snippet}

{#snippet footer()}
  {#if !skeletonLoading}
    {#if isEmpty && emptyConfig.actionLabel && emptyConfig.onAction}
      <div class="empty-action">
        <Button variant="primary" onclick={emptyConfig.onAction}>{emptyConfig.actionLabel}</Button>
      </div>
    {:else if !isEmpty}
      <footer class="list-footer">
        <span class="pager-info">{items.length} записей</span>
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
  {#each items as r (r.id)}
    <RequestListRow request={r} selected={r.id === selectedId} onclick={() => onSelect(r.id)} />
  {/each}
</Table>

<style lang="scss">
  .th-status {
    width: 120px;
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

<script lang="ts">
  // DeviceFilters — FTS search input + status switch-bar + group toggle.
  // Per UI-SPEC §DeviceFilters, D-Search-01, DEV-07.

  import Input from '$lib/components/Input.svelte';
  import Tabs from '$lib/components/Tabs.svelte';
  import Checkbox from '$lib/components/Checkbox.svelte';

  interface Props {
    searchQuery: string;
    statusFilter: number | null;
    grouped: boolean;
    counts: Map<number, number>;
    onSearchChange: (_q: string) => void;
    onStatusChange: (_s: number | null) => void;
    onGroupedChange: (_g: boolean) => void;
  }

  const {
    searchQuery,
    statusFilter,
    grouped,
    counts,
    onSearchChange,
    onStatusChange,
    onGroupedChange,
  }: Props = $props();

  // Internal search input value with debounce.
  let localSearch = $state(searchQuery);
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;

  $effect(() => {
    localSearch = searchQuery;
  });

  function handleSearchInput(v: string) {
    localSearch = v;
    if (debounceTimer !== null) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => {
      onSearchChange(localSearch);
    }, 250);
  }

  const STATUSES = [
    { id: null, label: 'Все' },
    { id: 1, label: 'На складе' },
    { id: 2, label: 'В работе' },
    { id: 3, label: 'На ремонте' },
    { id: 4, label: 'Списано' },
  ] as const;

  // Total count = sum of all status counts.
  const totalCount = $derived(Array.from(counts.values()).reduce((sum, c) => sum + c, 0));

  function getCount(id: number | null): number {
    if (id === null) return totalCount;
    return counts.get(id) ?? 0;
  }

  // Adapter for Tabs' string-keyed contract — STATUSES uses number | null ids.
  // String(null) === 'null', which is the exact inverse mapping used in onchange below.
  const tabItems = $derived(
    STATUSES.map((s) => ({ key: String(s.id), label: s.label, count: getCount(s.id) })),
  );
</script>

<div class="device-filters">
  <!-- FTS search input -->
  <label for="device-search-input" class="visually-hidden">Поиск устройств</label>
  <Input
    id="device-search-input"
    type="search"
    value={localSearch}
    oninput={handleSearchInput}
    placeholder="Поиск по наименованию, инвентарному, серийному, модели"
  >
    {#snippet iconLeft()}
      <svg
        width="16"
        height="16"
        viewBox="0 0 16 16"
        fill="none"
        xmlns="http://www.w3.org/2000/svg"
      >
        <circle cx="6.5" cy="6.5" r="4.5" stroke="currentColor" stroke-width="1.5" />
        <path d="M10 10L14 14" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
      </svg>
    {/snippet}
  </Input>

  <!-- Status switch-bar + group toggle -->
  <div class="filters-row">
    <Tabs
      variant="underline"
      tabs={tabItems}
      active={String(statusFilter)}
      ariaLabel="Фильтр по статусу"
      onchange={(key) => onStatusChange(key === 'null' ? null : Number(key))}
    />

    <Checkbox checked={grouped} onchange={onGroupedChange}>Группировать похожие</Checkbox>
  </div>
</div>

<style lang="scss">
  .device-filters {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding-bottom: 12px;
    border-bottom: 1px solid var(--tr-border);
    margin-bottom: 14px;
  }

  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }

  .filters-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--tr-space-md);
    flex-wrap: wrap;
  }
</style>

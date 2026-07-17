<script lang="ts">
  // DeviceFilters — FTS search input + status switch-bar + group toggle.
  // Per UI-SPEC §DeviceFilters, D-Search-01, DEV-07.

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
</script>

<div class="device-filters">
  <!-- FTS search input -->
  <div class="search-wrapper">
    <span class="search-icon" aria-hidden="true">
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
    </span>
    <input
      type="search"
      class="search-input"
      placeholder="Поиск по наименованию, инвентарному, серийному, модели"
      value={localSearch}
      oninput={(e) => handleSearchInput((e.currentTarget as HTMLInputElement).value)}
      aria-label="Поиск устройств"
    />
  </div>

  <!-- Status switch-bar + group toggle -->
  <div class="filters-row">
    <div class="status-bar" role="tablist" aria-label="Фильтр по статусу">
      {#each STATUSES as s}
        {@const active = statusFilter === s.id}
        {@const count = getCount(s.id)}
        <button
          type="button"
          role="tab"
          class="status-tab"
          class:active
          aria-selected={active}
          onclick={() => onStatusChange(s.id)}
        >
          {s.label}
          <span class="count-badge" class:count-active={active}>{count}</span>
        </button>
      {/each}
    </div>

    <label class="group-toggle">
      <input
        type="checkbox"
        class="group-checkbox"
        checked={grouped}
        onchange={(e) => onGroupedChange((e.currentTarget as HTMLInputElement).checked)}
      />
      <span class="group-label">Группировать похожие</span>
    </label>
  </div>
</div>

<style lang="scss">
  .device-filters {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-xs);
    padding-bottom: var(--tr-space-xs);
    border-bottom: 1px solid var(--tr-border);
    margin-bottom: var(--tr-space-md);
  }

  .search-wrapper {
    position: relative;
    display: flex;
    align-items: center;
  }

  .search-icon {
    position: absolute;
    left: var(--tr-space-xs);
    color: var(--tr-text-tertiary);
    pointer-events: none;
    display: flex;
    align-items: center;
  }

  .search-input {
    width: 100%;
    height: 36px;
    padding: 0 var(--tr-space-md) 0 calc(var(--tr-space-xs) * 2 + 16px);
    background: var(--tr-bg);
    color: var(--tr-text-primary);
    border: 1px solid var(--tr-border);
    border-radius: var(--radius-sm);
    font-family: var(--font-family-base);
    font-size: var(--font-size-body);

    &::placeholder {
      color: var(--tr-text-tertiary);
    }

    &:focus-visible {
      outline: none;
      border-color: var(--tr-accent);
      box-shadow: 0 0 0 3px var(--tr-focus-ring);
    }
  }

  .filters-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--tr-space-md);
    flex-wrap: wrap;
  }

  .status-bar {
    display: flex;
    gap: 2px;
    overflow-x: auto;
  }

  .status-tab {
    display: flex;
    align-items: center;
    gap: var(--tr-space-2xs);
    padding: var(--tr-space-2xs) var(--tr-space-xs);
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    font-family: var(--font-family-base);
    font-size: var(--font-size-body);
    color: var(--tr-text-secondary);
    cursor: pointer;
    white-space: nowrap;
    border-radius: var(--radius-sm) var(--radius-sm) 0 0;

    &:hover {
      background: var(--tr-surface);
      color: var(--tr-text-primary);
    }

    &.active {
      color: var(--tr-accent);
      border-bottom-color: var(--tr-accent);
      font-weight: var(--font-weight-medium);
    }
  }

  .count-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 18px;
    height: 18px;
    padding: 0 4px;
    border-radius: 9px;
    font-size: 11px;
    font-weight: var(--font-weight-medium);
    background: var(--tr-surface-sunken);
    color: var(--tr-text-secondary);
    line-height: 1;

    &.count-active {
      background: color-mix(in srgb, var(--tr-accent) 15%, transparent);
      color: var(--tr-accent);
    }
  }

  .group-toggle {
    display: flex;
    align-items: center;
    gap: var(--tr-space-2xs);
    cursor: pointer;
    white-space: nowrap;
    flex-shrink: 0;
  }

  .group-checkbox {
    width: 16px;
    height: 16px;
    cursor: pointer;
    accent-color: var(--tr-accent);
  }

  .group-label {
    font-size: var(--font-size-body);
    color: var(--tr-text-secondary);
    user-select: none;
  }
</style>

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

  const { searchQuery, statusFilter, grouped, counts, onSearchChange, onStatusChange, onGroupedChange }: Props = $props();

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
  const totalCount = $derived(
    Array.from(counts.values()).reduce((sum, c) => sum + c, 0),
  );

  function getCount(id: number | null): number {
    if (id === null) return totalCount;
    return counts.get(id) ?? 0;
  }
</script>

<div class="device-filters">
  <!-- FTS search input -->
  <div class="search-wrapper">
    <span class="search-icon" aria-hidden="true">
      <svg width="16" height="16" viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg">
        <circle cx="6.5" cy="6.5" r="4.5" stroke="currentColor" stroke-width="1.5"/>
        <path d="M10 10L14 14" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
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
    gap: var(--space-sm);
    padding-bottom: var(--space-sm);
    border-bottom: 1px solid var(--color-border);
    margin-bottom: var(--space-md);
  }

  .search-wrapper {
    position: relative;
    display: flex;
    align-items: center;
  }

  .search-icon {
    position: absolute;
    left: var(--space-sm);
    color: var(--color-text-muted);
    pointer-events: none;
    display: flex;
    align-items: center;
  }

  .search-input {
    width: 100%;
    height: 36px;
    padding: 0 var(--space-md) 0 calc(var(--space-sm) * 2 + 16px);
    background: var(--color-bg);
    color: var(--color-text-primary);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    font-family: var(--font-family-base);
    font-size: var(--font-size-body);

    &::placeholder {
      color: var(--color-text-muted);
    }

    &:focus-visible {
      outline: none;
      border-color: var(--color-accent);
      box-shadow: 0 0 0 3px var(--color-accent-focus);
    }
  }

  .filters-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-md);
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
    gap: var(--space-xs);
    padding: var(--space-xs) var(--space-sm);
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    font-family: var(--font-family-base);
    font-size: var(--font-size-body);
    color: var(--color-text-secondary);
    cursor: pointer;
    white-space: nowrap;
    border-radius: var(--radius-sm) var(--radius-sm) 0 0;

    &:hover {
      background: var(--color-surface);
      color: var(--color-text-primary);
    }

    &.active {
      color: var(--color-accent);
      border-bottom-color: var(--color-accent);
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
    background: var(--color-surface-sunken);
    color: var(--color-text-secondary);
    line-height: 1;

    &.count-active {
      background: color-mix(in srgb, var(--color-accent) 15%, transparent);
      color: var(--color-accent);
    }
  }

  .group-toggle {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    cursor: pointer;
    white-space: nowrap;
    flex-shrink: 0;
  }

  .group-checkbox {
    width: 16px;
    height: 16px;
    cursor: pointer;
    accent-color: var(--color-accent);
  }

  .group-label {
    font-size: var(--font-size-body);
    color: var(--color-text-secondary);
    user-select: none;
  }
</style>

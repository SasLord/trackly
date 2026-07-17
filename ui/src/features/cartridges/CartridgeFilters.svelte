<script lang="ts">
  // Plan 04-04: switch-bar статусов + фильтр типа + фильтр модели.
  // По образцу DeviceFilters.svelte, паттерн из PATTERNS.md §CartridgeFilters.svelte.
  import type { CartridgeCountsDto, CartridgeModelDto } from '../../bindings';

  interface Props {
    statusId: number | null;
    kindId: number | null;
    modelId: number | null;
    counts: CartridgeCountsDto;
    models: CartridgeModelDto[];
    onStatusChange: (_s: number | null) => void;
    onKindChange: (_k: number | null) => void;
    onModelChange: (_m: number | null) => void;
  }

  const {
    statusId,
    kindId,
    modelId,
    counts,
    models,
    onStatusChange,
    onKindChange,
    onModelChange,
  }: Props = $props();

  const STATUSES = [
    { id: null, label: 'Все' },
    { id: 1, label: 'На складе' },
    { id: 2, label: 'В работе' },
    { id: 3, label: 'На заправке' },
    { id: 4, label: 'Списано' },
  ] as const;

  function getCount(id: number | null): number {
    if (id === null) return counts.all;
    if (id === 1) return counts.in_stock;
    if (id === 2) return counts.in_use;
    if (id === 3) return counts.at_refill;
    if (id === 4) return counts.written_off;
    return 0;
  }

  // Модели, соответствующие выбранному типу (kindId). При «Все» — все модели.
  const visibleModels = $derived(
    kindId === null ? models : models.filter((m) => m.kind_id === kindId),
  );
</script>

<div class="cartridge-filters">
  <!-- Status switch-bar -->
  <div class="status-bar" role="tablist" aria-label="Фильтр по статусу">
    {#each STATUSES as s}
      {@const active = statusId === s.id}
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

  <!-- Additional filters row -->
  <div class="extra-filters">
    <label class="filter-label">
      <span class="filter-name">Тип</span>
      <select
        class="filter-select"
        value={kindId ?? ''}
        onchange={(e) => {
          const v = (e.currentTarget as HTMLSelectElement).value;
          onKindChange(v === '' ? null : Number(v));
        }}
      >
        <!-- Числовые value (не строковые): Svelte select_option сравнивает строго,
             а kindId — число; строковые "1"/"2" не матчились → метка пропадала. -->
        <option value="">Все</option>
        <option value={1}>Картридж</option>
        <option value={2}>Фотобарабан</option>
      </select>
    </label>

    <label class="filter-label">
      <span class="filter-name">Модель</span>
      <select
        class="filter-select"
        value={modelId ?? ''}
        onchange={(e) => {
          const v = (e.currentTarget as HTMLSelectElement).value;
          onModelChange(v === '' ? null : Number(v));
        }}
      >
        <option value="">Все</option>
        {#each visibleModels as m (m.id)}
          <option value={m.id}>{m.brand} {m.model}</option>
        {/each}
      </select>
    </label>
  </div>
</div>

<style lang="scss">
  .cartridge-filters {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-xs);
    padding: var(--tr-space-xs) var(--tr-space-xs) var(--tr-space-xs);
    border-bottom: 1px solid var(--tr-border);
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
    border-radius: var(--tr-radius-xs) var(--tr-radius-xs) 0 0;

    &:hover {
      background: var(--tr-surface);
      color: var(--tr-text-primary);
    }

    &:focus-visible {
      outline: none;
      box-shadow: 0 0 0 3px var(--tr-focus-ring);
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

  .extra-filters {
    display: flex;
    gap: var(--tr-space-xs);
    flex-wrap: wrap;
  }

  .filter-label {
    display: flex;
    align-items: center;
    gap: var(--tr-space-2xs);
    flex-shrink: 0;
  }

  .filter-name {
    font-size: var(--font-size-label);
    color: var(--tr-text-secondary);
    white-space: nowrap;
  }

  .filter-select {
    height: 28px;
    padding: 0 var(--tr-space-xs);
    background: var(--tr-bg);
    color: var(--tr-text-primary);
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-xs);
    font-family: var(--font-family-base);
    font-size: var(--font-size-label);
    cursor: pointer;

    &:focus-visible {
      outline: none;
      border-color: var(--tr-accent);
      box-shadow: 0 0 0 3px var(--tr-focus-ring);
    }
  }
</style>

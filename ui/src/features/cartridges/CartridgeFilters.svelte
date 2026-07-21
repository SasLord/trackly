<script lang="ts">
  // Plan 04-04: switch-bar статусов + фильтр типа + фильтр модели.
  // Plan 27-06 (D-05): switch-bar статусов переведён на примитив Tabs (variant="underline"),
  // по образцу DeviceFilters.svelte — счётчики встроены в Tabs, bespoke .status-tab удалён.
  import Select from '$lib/components/Select.svelte';
  import Tabs from '$lib/components/Tabs.svelte';
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

  // Адаптер строкового контракта Tabs (D-05) — STATUSES использует number|null id.
  // String(null) === 'null', обратная маппинг-функция в onchange ниже.
  const tabItems = $derived(
    STATUSES.map((s) => ({ key: String(s.id), label: s.label, count: getCount(s.id) })),
  );
</script>

<div class="cartridge-filters">
  <!-- Status switch-bar (D-05: примитив Tabs, счётчик встроен) -->
  <Tabs
    variant="underline"
    tabs={tabItems}
    active={String(statusId)}
    ariaLabel="Фильтр по статусу"
    onchange={(key) => onStatusChange(key === 'null' ? null : Number(key))}
  />

  <!-- Additional filters row -->
  <div class="extra-filters">
    <label class="filter-label">
      <span class="filter-name">Тип</span>
      <Select
        value={kindId !== null ? String(kindId) : ''}
        onchange={(v) => onKindChange(v === '' ? null : Number(v))}
      >
        <option value="">Все</option>
        <option value="1">Картридж</option>
        <option value="2">Фотобарабан</option>
      </Select>
    </label>

    <label class="filter-label">
      <span class="filter-name">Модель</span>
      <Select
        value={modelId !== null ? String(modelId) : ''}
        onchange={(v) => onModelChange(v === '' ? null : Number(v))}
      >
        <option value="">Все</option>
        {#each visibleModels as m (m.id)}
          <option value={String(m.id)}>{m.brand} {m.model}</option>
        {/each}
      </Select>
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
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-secondary);
    white-space: nowrap;
  }
</style>

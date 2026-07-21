<script lang="ts">
  // Plan 04-04: switch-bar статусов + фильтр типа + фильтр модели.
  // Plan 27-06 (D-05): switch-bar статусов переведён на примитив Tabs (variant="underline"),
  // по образцу DeviceFilters.svelte — счётчики встроены в Tabs, bespoke .status-tab удалён.
  // Plan 27-G1: Select (нативный <select>) заменён на кастомный Dropdown
  // (flat + variant="select") — открывающееся меню больше не нативное OS-меню.
  import Dropdown from '$lib/components/Dropdown.svelte';
  import Tabs from '$lib/components/Tabs.svelte';
  import type { CartridgeCountsDto, CartridgeModelDto } from '../../bindings';

  interface FilterOption {
    id: string;
    label: string;
  }

  // Плоские опции без drill-in — onExpandGroup никогда реально не вызывается
  // (isGroupExpandable всегда false), но Dropdown требует типизированную
  // функцию, чтобы вывести TMember (иначе `() => []` выводит `never[]`).
  function noExpand(): FilterOption[] {
    return [];
  }

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

  const TYPE_OPTIONS: FilterOption[] = [
    { id: '', label: 'Все' },
    { id: '1', label: 'Картридж' },
    { id: '2', label: 'Фотобарабан' },
  ];
  const kindValue = $derived(kindId !== null ? String(kindId) : '');
  const kindLabel = $derived(TYPE_OPTIONS.find((o) => o.id === kindValue)?.label ?? 'Все');

  const modelOptions = $derived<FilterOption[]>([
    { id: '', label: 'Все' },
    ...visibleModels.map((m) => ({ id: String(m.id), label: `${m.brand} ${m.model}` })),
  ]);
  const modelValue = $derived(modelId !== null ? String(modelId) : '');
  const modelLabel = $derived(modelOptions.find((o) => o.id === modelValue)?.label ?? 'Все');

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
      <div class="filter-dropdown">
        <Dropdown
          variant="select"
          flat={true}
          value={kindLabel}
          placeholder="Все"
          searchPlaceholder="Поиск"
          loading={false}
          groups={TYPE_OPTIONS}
          getGroupId={(o) => o.id}
          getGroupName={(o) => o.label}
          getGroupCount={() => 0}
          isGroupExpandable={() => false}
          isGroupSelected={(o) => o.id === kindValue}
          onExpandGroup={noExpand}
          getMemberId={(o) => o.id}
          getMemberName={(o) => o.label}
          onSearch={() => {}}
          onPickGroup={(o) => onKindChange(o.id === '' ? null : Number(o.id))}
          onPickMember={() => {}}
        />
      </div>
    </label>

    <label class="filter-label">
      <span class="filter-name">Модель</span>
      <div class="filter-dropdown">
        <Dropdown
          variant="select"
          flat={true}
          value={modelLabel}
          placeholder="Все"
          searchPlaceholder="Поиск"
          loading={false}
          groups={modelOptions}
          getGroupId={(o) => o.id}
          getGroupName={(o) => o.label}
          getGroupCount={() => 0}
          isGroupExpandable={() => false}
          isGroupSelected={(o) => o.id === modelValue}
          onExpandGroup={noExpand}
          getMemberId={(o) => o.id}
          getMemberName={(o) => o.label}
          onSearch={() => {}}
          onPickGroup={(o) => onModelChange(o.id === '' ? null : Number(o.id))}
          onPickMember={() => {}}
        />
      </div>
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

  // Preserves the fixed-width behaviour of the former Select's
  // `.select-wrapper { width: 100%; }` inside this flex row.
  .filter-dropdown {
    width: 180px;
    max-width: 100%;
  }
</style>

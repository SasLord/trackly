<script lang="ts">
  // Plan 07-06 Task 1: Contextual filter row for Reports page.
  // Devices: location / type / status. Cartridges: model / status / color.
  // Export and print buttons are right-aligned — callbacks passed from parent orchestrator.
  import Input from '$lib/components/Input.svelte';
  import Button from '$lib/components/Button.svelte';

  interface ReportFilter {
    location_name?: string | null;
    status_id?: number | null;
    type_id?: number | null;
    model_id?: number | null;
    color?: string | null;
    search?: string | null;
  }

  interface Props {
    reportDomain: 'devices' | 'cartridges';
    reportType: string;
    locationName: string | null;
    statusId: number | null;
    typeId: number | null;
    modelId: number | null;
    color: string | null;
    search: string;
    locations: string[];
    deviceTypes: Array<{ id: number; name: string }>;
    cartridgeModels: Array<{ id: number; label: string }>;
    cartridgeStatuses: Array<{ id: number; name: string }>;
    cartridgeColors: string[];
    onFilterChange: (_f: Partial<ReportFilter>) => void;
    onExportCsv: () => void;
    onExportPdf: () => void;
    onPrint: () => void;
    csvExporting: boolean;
    pdfExporting: boolean;
  }

  const {
    reportDomain,
    locationName,
    statusId,
    typeId,
    modelId,
    color,
    search,
    locations,
    deviceTypes,
    cartridgeModels,
    cartridgeStatuses,
    cartridgeColors,
    onFilterChange,
    onExportCsv,
    onExportPdf,
    onPrint,
    csvExporting,
    pdfExporting,
  }: Props = $props();

  function onLocationChange(e: Event) {
    const v = (e.currentTarget as HTMLSelectElement).value;
    onFilterChange({ location_name: v === '' ? null : v });
  }

  function onDeviceTypeChange(e: Event) {
    const v = (e.currentTarget as HTMLSelectElement).value;
    onFilterChange({ type_id: v === '' ? null : Number(v) });
  }

  function onDeviceStatusChange(e: Event) {
    const v = (e.currentTarget as HTMLSelectElement).value;
    onFilterChange({ status_id: v === '' ? null : Number(v) });
  }

  function onModelChange(e: Event) {
    const v = (e.currentTarget as HTMLSelectElement).value;
    onFilterChange({ model_id: v === '' ? null : Number(v) });
  }

  function onCartridgeStatusChange(e: Event) {
    const v = (e.currentTarget as HTMLSelectElement).value;
    onFilterChange({ status_id: v === '' ? null : Number(v) });
  }

  function onColorChange(e: Event) {
    const v = (e.currentTarget as HTMLSelectElement).value;
    onFilterChange({ color: v === '' ? null : v });
  }

  function onSearchInput(v: string) {
    onFilterChange({ search: v === '' ? null : v });
  }
</script>

<div class="filters-row">
  {#if reportDomain === 'devices'}
    <!-- D-04: Устройства → локация / тип / статус -->
    <label class="filter-label">
      <span class="filter-name">Локация</span>
      <select class="filter-select" value={locationName ?? ''} onchange={onLocationChange}>
        <option value="">Все</option>
        {#each locations as loc}
          <option value={loc}>{loc}</option>
        {/each}
      </select>
    </label>

    <label class="filter-label">
      <span class="filter-name">Тип</span>
      <select class="filter-select" value={typeId ?? ''} onchange={onDeviceTypeChange}>
        <option value="">Все</option>
        {#each deviceTypes as dt (dt.id)}
          <option value={dt.id}>{dt.name}</option>
        {/each}
      </select>
    </label>

    <label class="filter-label">
      <span class="filter-name">Статус</span>
      <select class="filter-select" value={statusId ?? ''} onchange={onDeviceStatusChange}>
        <option value="">Все</option>
        <option value={1}>В работе</option>
        <option value={2}>На складе</option>
        <option value={3}>Списано</option>
      </select>
    </label>
  {:else}
    <!-- D-04: Картриджи → модель / статус / цвет -->
    <label class="filter-label">
      <span class="filter-name">Модель</span>
      <select class="filter-select" value={modelId ?? ''} onchange={onModelChange}>
        <option value="">Все</option>
        {#each cartridgeModels as m (m.id)}
          <option value={m.id}>{m.label}</option>
        {/each}
      </select>
    </label>

    <label class="filter-label">
      <span class="filter-name">Статус</span>
      <select class="filter-select" value={statusId ?? ''} onchange={onCartridgeStatusChange}>
        <option value="">Все</option>
        {#each cartridgeStatuses as s (s.id)}
          <option value={s.id}>{s.name}</option>
        {/each}
      </select>
    </label>

    <label class="filter-label">
      <span class="filter-name">Цвет</span>
      <select class="filter-select" value={color ?? ''} onchange={onColorChange}>
        <option value="">Все</option>
        {#each cartridgeColors as c}
          <option value={c}>{c}</option>
        {/each}
      </select>
    </label>
  {/if}

  <!-- Search input — both domains -->
  <div class="search-wrap">
    <Input
      type="search"
      value={search}
      placeholder="Поиск в отчёте…"
      oninput={onSearchInput}
    />
  </div>

  <!-- Export buttons — right-aligned via margin-left: auto on print button wrapper -->
  <div class="export-buttons">
    <Button
      variant="secondary"
      size="sm"
      loading={csvExporting}
      onclick={onExportCsv}
    >
      Экспорт CSV
    </Button>
    <Button
      variant="secondary"
      size="sm"
      loading={pdfExporting}
      onclick={onExportPdf}
    >
      Экспорт PDF
    </Button>
    <Button variant="ghost" size="sm" onclick={onPrint}>
      <svg
        width="14"
        height="14"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
        aria-hidden="true"
      >
        <polyline points="6 9 6 2 18 2 18 9"></polyline>
        <path d="M6 18H4a2 2 0 0 1-2-2v-5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v5a2 2 0 0 1-2 2h-2"></path>
        <rect x="6" y="14" width="12" height="8"></rect>
      </svg>
      Печать
    </Button>
  </div>
</div>

<style lang="scss">
  .filters-row {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    flex-wrap: wrap;
    padding: var(--space-sm) 0;
  }

  .filter-label {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    flex-shrink: 0;
  }

  .filter-name {
    font-size: var(--font-size-label);
    color: var(--color-text-secondary);
    white-space: nowrap;
  }

  .filter-select {
    height: 28px;
    padding: 0 var(--space-sm);
    background: var(--color-bg);
    color: var(--color-text-primary);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    font-family: var(--font-family-base);
    font-size: var(--font-size-label);
    cursor: pointer;

    &:focus-visible {
      outline: none;
      border-color: var(--color-accent);
      box-shadow: 0 0 0 3px var(--color-accent-focus);
    }
  }

  .search-wrap {
    min-width: 180px;
    max-width: 260px;
    flex: 1;
  }

  .export-buttons {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    margin-left: auto;
  }
</style>

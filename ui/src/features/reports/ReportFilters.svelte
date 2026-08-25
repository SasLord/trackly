<script lang="ts">
  // Plan 07-06 Task 1: Contextual filter row for Reports page.
  // Plan 07-10 Task 2: GAP-R4 — remove redundant domain-specific filters and search.
  //   Keep only Export/Print buttons. Props interface retained for parent compatibility.
  // Plan 39-18 Task 2: GAP-R4 reactivated — PlacePicker-based place filter (D-28,
  //   whole-subtree semantics per report_service.rs's WITH RECURSIVE subtree walk,
  //   Plan 39-10) + a separate D-11.2/D-11.5 "Складское место" quick filter, kept
  //   visually and logically independent of statusId.
  import Button from '$lib/components/Button.svelte';
  import Dropdown from '$lib/components/Dropdown.svelte';
  import PlacePicker from '$lib/components/PlacePicker.svelte';

  interface ReportFilter {
    place_id?: number | null;
    is_storage?: boolean | null;
    status_id?: number | null;
    type_id?: number | null;
    model_id?: number | null;
    color?: string | null;
    search?: string | null;
  }

  interface StorageOption {
    id: string;
    label: string;
  }

  // Плоские опции без drill-in — onExpandGroup никогда реально не вызывается
  // (isGroupExpandable всегда false), но Dropdown требует типизированную
  // функцию, чтобы вывести TMember (иначе `() => []` выводит `never[]`).
  function noExpand(): StorageOption[] {
    return [];
  }

  // D-11.2/D-11.5: geographic quick filter, independent dimension from
  // statusId (item lifecycle status) — never merged into one control.
  const STORAGE_OPTIONS: StorageOption[] = [
    { id: 'null', label: 'Все' },
    { id: 'true', label: 'На складе' },
    { id: 'false', label: 'В эксплуатации' },
  ];

  interface Props {
    reportDomain: 'devices' | 'cartridges' | 'requests';
    reportType: string;
    placeId?: number | null;
    isStorage?: boolean | null;
    // Props below are retained for parent compatibility but no longer rendered.
    // The parent (ReportsPage.svelte) still passes them; removing would require
    // parent refactor. They are accepted and unused here intentionally.
    statusId?: number | null;
    typeId?: number | null;
    modelId?: number | null;
    color?: string | null;
    search?: string;
    deviceTypes?: Array<{ id: number; name: string }>;
    cartridgeModels?: Array<{ id: number; label: string }>;
    cartridgeStatuses?: Array<{ id: number; name: string }>;
    cartridgeColors?: string[];
    onFilterChange?: (_f: Partial<ReportFilter>) => void;
    onExportCsv: () => void;
    onExportPdf: () => void;
    onPrint: () => void;
    csvExporting: boolean;
    pdfExporting: boolean;
  }

  const {
    // GAP-R4: filter props accepted but not rendered (kept for parent compat)
    reportDomain: _reportDomain,
    reportType: _reportType,
    placeId = null,
    isStorage = null,
    statusId: _statusId,
    typeId: _typeId,
    modelId: _modelId,
    color: _color,
    search: _search,
    deviceTypes: _deviceTypes,
    cartridgeModels: _cartridgeModels,
    cartridgeStatuses: _cartridgeStatuses,
    cartridgeColors: _cartridgeColors,
    onFilterChange,
    onExportCsv,
    // GAP-R4/Phase-17: «Экспорт PDF» and «Печать» now trigger the same
    // preview+print modal, so the two buttons were merged into one
    // («Печать / Экспорт PDF», wired to onPrint). These props are retained for
    // parent compatibility but no longer rendered.
    onExportPdf: _onExportPdf,
    onPrint,
    csvExporting,
    pdfExporting: _pdfExporting,
  }: Props = $props();

  const storageValue = $derived(isStorage === null ? 'null' : String(isStorage));
  const storageLabel = $derived(
    STORAGE_OPTIONS.find((o) => o.id === storageValue)?.label ?? 'Все',
  );

  function handleStorageChange(id: string) {
    onFilterChange?.({ is_storage: id === 'null' ? null : id === 'true' });
  }
</script>

<div class="report-filters">
  <div class="place-filter-group">
    <label class="filter-label" for="report-place-filter">
      <span class="filter-name">Место</span>
    </label>
    <div class="place-filter">
      <PlacePicker
        id="report-place-filter"
        value={placeId}
        onChange={(id) => onFilterChange?.({ place_id: id })}
      />
    </div>
  </div>

  <label class="filter-label">
    <span class="filter-name">Складское место</span>
    <div class="filter-dropdown">
      <Dropdown
        variant="select"
        flat={true}
        searchable={false}
        value={storageLabel}
        placeholder="Все"
        searchPlaceholder="Поиск"
        loading={false}
        groups={STORAGE_OPTIONS}
        getGroupId={(o) => o.id}
        getGroupName={(o) => o.label}
        getGroupCount={() => 0}
        isGroupExpandable={() => false}
        isGroupSelected={(o) => o.id === storageValue}
        onExpandGroup={noExpand}
        getMemberId={(o) => o.id}
        getMemberName={(o) => o.label}
        onSearch={() => {}}
        onPickGroup={(o) => handleStorageChange(o.id)}
        onPickMember={() => {}}
      />
    </div>
  </label>

  <div class="export-buttons">
    <Button variant="secondary" size="sm" loading={csvExporting} onclick={onExportCsv}>
      Экспорт CSV
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
      Печать / Экспорт PDF
    </Button>
  </div>
</div>

<style lang="scss">
  .report-filters {
    display: flex;
    align-items: center;
    gap: var(--tr-space-sm);
    flex-wrap: wrap;
  }

  .place-filter-group {
    display: flex;
    align-items: center;
    gap: var(--tr-space-2xs);
  }

  .place-filter {
    display: flex;
    flex-direction: column;
    width: 220px;
    max-width: 100%;
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

  .filter-dropdown {
    width: 180px;
    max-width: 100%;
  }

  .export-buttons {
    display: flex;
    align-items: center;
    gap: var(--tr-space-2xs);
    margin-left: auto;
  }
</style>

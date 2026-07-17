<script lang="ts">
  // Plan 07-06 Task 1: Contextual filter row for Reports page.
  // Plan 07-10 Task 2: GAP-R4 — remove redundant domain-specific filters and search.
  //   Keep only Export/Print buttons. Props interface retained for parent compatibility.
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
    // Props below are retained for parent compatibility but no longer rendered.
    // The parent (ReportsPage.svelte) still passes them; removing would require
    // parent refactor. They are accepted and unused here intentionally.
    locationName?: string | null;
    statusId?: number | null;
    typeId?: number | null;
    modelId?: number | null;
    color?: string | null;
    search?: string;
    locations?: string[];
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
    locationName: _locationName,
    statusId: _statusId,
    typeId: _typeId,
    modelId: _modelId,
    color: _color,
    search: _search,
    locations: _locations,
    deviceTypes: _deviceTypes,
    cartridgeModels: _cartridgeModels,
    cartridgeStatuses: _cartridgeStatuses,
    cartridgeColors: _cartridgeColors,
    onFilterChange: _onFilterChange,
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
</script>

<!-- GAP-R4: filter row now contains only export buttons -->
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

<style lang="scss">
  .export-buttons {
    display: flex;
    align-items: center;
    gap: var(--tr-space-2xs);
  }
</style>

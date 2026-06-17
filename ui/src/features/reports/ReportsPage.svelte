<script lang="ts">
  // Plan 07-06 Task 2: ReportsPage orchestrator.
  // Assembles sub-nav, period selector, filters, and table.
  // Loads dropdown data on mount; auto-reloads report on state changes.
  import { onMount } from 'svelte';
  import { apiCall } from '$lib/api/client';
  import { pushToast } from '$lib/stores/toast.svelte';
  import type { CartridgeModelDto } from '../../bindings';
  import ReportSubNav from './ReportSubNav.svelte';
  import PeriodSelector from './PeriodSelector.svelte';
  import ReportFilters from './ReportFilters.svelte';
  import ReportTable from './ReportTable.svelte';

  type DomainKey = 'devices' | 'cartridges';

  interface PeriodDto {
    mode: string;
    year?: number | null;
    month?: number | null;
    date_from?: string | null;
    date_to?: string | null;
  }

  interface ReportFilter {
    location_name?: string | null;
    status_id?: number | null;
    type_id?: number | null;
    model_id?: number | null;
    color?: string | null;
    search?: string | null;
    date_from_utc?: number | null;
    date_to_utc?: number | null;
    location_id?: number | null;
    act_type?: string | null;
  }

  interface ReportRow {
    id: number;
    month_key?: string | null;
    number?: string | null;
    sub_number?: string | null;
    giver_name?: string | null;
    receiver_name?: string | null;
    handover_date_utc?: number | null;
    location_name?: string | null;
    act_type?: string | null;
    device_name?: string | null;
    quantity?: number | null;
    code?: string | null;
    model_label?: string | null;
    status_name?: string | null;
    [key: string]: unknown;
  }

  interface ReportResponse {
    rows: ReportRow[];
    total: number;
  }

  interface Column {
    key: string;
    label: string;
  }

  // ---------------------------------------------------------------------------
  // Report type config
  // ---------------------------------------------------------------------------
  const DEVICE_REPORTS = [
    { key: 'acts', label: 'Акты', temporal: true, cmd: 'reports_list_device_acts' },
    { key: 'returns', label: 'Возвраты', temporal: true, cmd: 'reports_list_device_returns' },
    { key: 'in_use', label: 'В работе', temporal: false, cmd: 'reports_list_device_in_use' },
    { key: 'in_stock', label: 'На складе', temporal: false, cmd: 'reports_list_device_in_stock' },
  ] as const;

  const CARTRIDGE_REPORTS = [
    {
      key: 'consumption',
      label: 'Расход',
      temporal: true,
      cmd: 'reports_list_cartridge_consumption',
    },
    {
      key: 'refills',
      label: 'История заправок',
      temporal: true,
      cmd: 'reports_list_cartridge_refills',
    },
    {
      key: 'in_use',
      label: 'В работе',
      temporal: false,
      cmd: 'reports_list_cartridge_in_use',
    },
    {
      key: 'in_stock',
      label: 'На складе',
      temporal: false,
      cmd: 'reports_list_cartridge_in_stock',
    },
  ] as const;

  // Column definitions per report type
  const COLUMNS_MAP: Record<string, Column[]> = {
    acts: [
      { key: 'number', label: 'Номер' },
      { key: 'giver_name', label: 'Сдал' },
      { key: 'receiver_name', label: 'Принял' },
      { key: 'handover_date_utc', label: 'Дата' },
      { key: 'location_name', label: 'Локация' },
      { key: 'device_name', label: 'Устройства' },
    ],
    returns: [
      { key: 'number', label: 'Номер' },
      { key: 'sub_number', label: 'Суб-номер' },
      { key: 'giver_name', label: 'Сдал' },
      { key: 'receiver_name', label: 'Принял' },
      { key: 'handover_date_utc', label: 'Дата' },
      { key: 'location_name', label: 'Локация' },
    ],
    in_use: [
      { key: 'device_name', label: 'Наименование' },
      { key: 'location_name', label: 'Расположение' },
      { key: 'status_name', label: 'Статус' },
    ],
    in_stock: [
      { key: 'device_name', label: 'Наименование' },
      { key: 'location_name', label: 'Расположение' },
      { key: 'status_name', label: 'Статус' },
    ],
    consumption: [
      { key: 'month_key', label: 'Месяц' },
      { key: 'model_label', label: 'Модель' },
      { key: 'code', label: 'Код картриджа' },
      { key: 'location_name', label: 'Локация' },
    ],
    refills: [
      { key: 'month_key', label: 'Месяц' },
      { key: 'model_label', label: 'Модель' },
      { key: 'code', label: 'Код картриджа' },
      { key: 'location_name', label: 'Локация' },
    ],
    cartridge_in_use: [
      { key: 'code', label: 'Код' },
      { key: 'model_label', label: 'Модель' },
      { key: 'location_name', label: 'Расположение' },
      { key: 'status_name', label: 'Статус' },
    ],
    cartridge_in_stock: [
      { key: 'code', label: 'Код' },
      { key: 'model_label', label: 'Модель' },
      { key: 'location_name', label: 'Расположение' },
      { key: 'status_name', label: 'Статус' },
    ],
  };

  // ---------------------------------------------------------------------------
  // Navigation state
  // ---------------------------------------------------------------------------
  let activeDomain = $state<DomainKey>('devices');
  let activeReport = $state('acts');

  // ---------------------------------------------------------------------------
  // Period state
  // ---------------------------------------------------------------------------
  let period = $state<PeriodDto>({
    mode: 'month',
    year: new Date().getFullYear(),
    month: new Date().getMonth() + 1,
  });

  // ---------------------------------------------------------------------------
  // Filter state
  // ---------------------------------------------------------------------------
  let filter = $state<Partial<ReportFilter>>({});

  // ---------------------------------------------------------------------------
  // Report data state
  // ---------------------------------------------------------------------------
  let rows = $state<ReportResponse | null>(null);
  let loading = $state(false);
  let error = $state<string | null>(null);

  // ---------------------------------------------------------------------------
  // Export state
  // ---------------------------------------------------------------------------
  let csvExporting = $state(false);
  let pdfExporting = $state(false);

  // ---------------------------------------------------------------------------
  // Filter dropdown data
  // ---------------------------------------------------------------------------
  let filterLocations = $state<string[]>([]);
  let filterDeviceTypes = $state<Array<{ id: number; name: string }>>([
    { id: 1, name: 'Устройство' },
    { id: 2, name: 'Расходник' },
  ]);
  let filterCartridgeModels = $state<Array<{ id: number; label: string }>>([]);
  let filterCartridgeStatuses = $state<Array<{ id: number; name: string }>>([
    { id: 1, name: 'На складе' },
    { id: 2, name: 'В работе' },
    { id: 3, name: 'На заправке' },
    { id: 4, name: 'Списано' },
  ]);
  let filterCartridgeColors = $state<string[]>([]);

  // ---------------------------------------------------------------------------
  // Helpers
  // ---------------------------------------------------------------------------
  function isSnapshot(): boolean {
    return ['in_use', 'in_stock'].includes(activeReport);
  }

  function currentCmd(): string {
    const allReports = [...DEVICE_REPORTS, ...CARTRIDGE_REPORTS];
    const found = allReports.find((r) => r.key === activeReport);
    // For cartridge domain in_use/in_stock, we need to find correct cmd
    if (activeDomain === 'cartridges') {
      const cartridgeFound = CARTRIDGE_REPORTS.find((r) => r.key === activeReport);
      if (cartridgeFound) return cartridgeFound.cmd;
    }
    if (activeDomain === 'devices') {
      const deviceFound = DEVICE_REPORTS.find((r) => r.key === activeReport);
      if (deviceFound) return deviceFound.cmd;
    }
    return found?.cmd ?? 'reports_list_device_acts';
  }

  // GAP-R1: Maps domain + activeReport → backend report_type key expected by
  // reports_export_csv / reports_export_pdf (NOT the full Tauri command name).
  function reportTypeKey(): string {
    if (activeDomain === 'devices') {
      switch (activeReport) {
        case 'acts': return 'device_acts';
        case 'returns': return 'device_returns';
        case 'in_use': return 'device_in_use';
        case 'in_stock': return 'device_in_stock';
      }
    } else {
      switch (activeReport) {
        case 'consumption': return 'cartridge_consumption';
        case 'refills': return 'cartridge_refills';
        case 'in_use': return 'cartridge_in_use';
        case 'in_stock': return 'cartridge_in_stock';
      }
    }
    return 'device_acts'; // fallback
  }

  function currentColumns(): Column[] {
    // For cartridge domain, use prefixed keys to differentiate from device in_use/in_stock
    if (activeDomain === 'cartridges' && (activeReport === 'in_use' || activeReport === 'in_stock')) {
      return COLUMNS_MAP[`cartridge_${activeReport}`] ?? COLUMNS_MAP[activeReport] ?? [];
    }
    return COLUMNS_MAP[activeReport] ?? [];
  }

  // ---------------------------------------------------------------------------
  // Data loading
  // ---------------------------------------------------------------------------
  function loadReport() {
    loading = true;
    error = null;
    const cmd = currentCmd();
    const filterPayload = { ...filter, search: filter.search ?? null };
    apiCall<ReportResponse>(cmd, {
      filter: filterPayload,
      period: isSnapshot() ? undefined : period,
    })
      .then((r) => {
        rows = r;
      })
      .catch((e: unknown) => {
        const msg =
          e && typeof e === 'object' && 'message' in e
            ? String((e as { message: unknown }).message)
            : 'Не удалось загрузить отчёт';
        error = msg;
        pushToast('error', 'Не удалось загрузить отчёт. Попробуйте ещё раз.');
      })
      .finally(() => {
        loading = false;
      });
  }

  // Auto-reload when domain / report / period / filter changes
  $effect(() => {
    // Track reactive dependencies
    void activeDomain;
    void activeReport;
    void period;
    void filter;
    loadReport();
  });

  // ---------------------------------------------------------------------------
  // Export handlers
  // ---------------------------------------------------------------------------
  function exportCsv() {
    csvExporting = true;
    apiCall<number[]>('reports_export_csv', {
      reportType: reportTypeKey(),
      filter,
      period: isSnapshot() ? undefined : period,
    })
      .then((bytes) => {
        const blob = new Blob([new Uint8Array(bytes)], { type: 'text/csv;charset=utf-8' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = 'отчёт.csv';
        a.click();
        URL.revokeObjectURL(url);
      })
      .catch(() => {
        pushToast('error', 'Ошибка при экспорте CSV. Попробуйте ещё раз.');
      })
      .finally(() => {
        csvExporting = false;
      });
  }

  function exportPdf() {
    pdfExporting = true;
    apiCall<number[]>('reports_export_pdf', {
      reportType: reportTypeKey(),
      filter,
      period: isSnapshot() ? undefined : period,
    })
      .then(async (bytes) => {
        const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
        if (isTauri) {
          try {
            // Save via tauri-plugin-fs + open in system viewer
            const { save } = await import('@tauri-apps/plugin-dialog');
            const { writeFile } = await import('@tauri-apps/plugin-fs');
            const { open: openPath } = await import('@tauri-apps/plugin-shell');
            const filePath = await save({
              title: 'Сохранить PDF',
              defaultPath: 'отчёт.pdf',
              filters: [{ name: 'PDF', extensions: ['pdf'] }],
            });
            if (filePath) {
              await writeFile(filePath, new Uint8Array(bytes));
              await openPath(filePath);
            }
          } catch {
            // Fall back to blob download if tauri plugins unavailable
            const blob = new Blob([new Uint8Array(bytes)], { type: 'application/pdf' });
            const url = URL.createObjectURL(blob);
            const a = document.createElement('a');
            a.href = url;
            a.download = 'отчёт.pdf';
            a.click();
            URL.revokeObjectURL(url);
          }
        } else {
          const blob = new Blob([new Uint8Array(bytes)], { type: 'application/pdf' });
          const url = URL.createObjectURL(blob);
          const a = document.createElement('a');
          a.href = url;
          a.download = 'отчёт.pdf';
          a.click();
          URL.revokeObjectURL(url);
        }
      })
      .catch(() => {
        pushToast('error', 'Ошибка при создании PDF. Попробуйте ещё раз.');
      })
      .finally(() => {
        pdfExporting = false;
      });
  }

  async function printReport() {
    const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
    if (isTauri) {
      // Generate PDF then open in system viewer
      pdfExporting = true;
      try {
        const bytes = await apiCall<number[]>('reports_export_pdf', {
          reportType: reportTypeKey(),
          filter,
          period: isSnapshot() ? undefined : period,
        });
        const { save } = await import('@tauri-apps/plugin-dialog');
        const { writeFile } = await import('@tauri-apps/plugin-fs');
        const { open: openPath } = await import('@tauri-apps/plugin-shell');
        const filePath = await save({
          title: 'Сохранить PDF для печати',
          defaultPath: 'отчёт.pdf',
          filters: [{ name: 'PDF', extensions: ['pdf'] }],
        });
        if (filePath) {
          await writeFile(filePath, new Uint8Array(bytes));
          await openPath(filePath);
        }
      } catch {
        pushToast('error', 'Ошибка при создании PDF. Попробуйте ещё раз.');
      } finally {
        pdfExporting = false;
      }
    } else {
      window.print();
    }
  }

  // ---------------------------------------------------------------------------
  // onMount: load dynamic filter data
  // ---------------------------------------------------------------------------
  onMount(async () => {
    // Load locations — locations_autocomplete returns string[] (location names)
    try {
      const locs = await apiCall<string[]>('locations_autocomplete', { prefix: '' });
      filterLocations = locs;
    } catch {
      // Non-fatal; filter shows empty list
    }

    // Load cartridge models — used for model filter and color derivation
    try {
      const models = await apiCall<CartridgeModelDto[]>('cartridge_models_list', {});
      filterCartridgeModels = models.map((m) => ({
        id: m.id,
        label: m.brand + ' ' + m.model,
      }));
      filterCartridgeColors = [
        ...new Set(models.map((m) => m.color).filter((c): c is string => c != null)),
      ].sort();
    } catch {
      // Non-fatal; filter shows empty list
    }
  });
</script>

<div class="reports-page">
  <header class="page-header">
    <h1 class="page-title">Отчёты</h1>
  </header>

  <div class="reports-content">
    <ReportSubNav
      {activeDomain}
      {activeReport}
      rowCount={rows?.total ?? 0}
      onDomainChange={(d) => {
        activeDomain = d;
        activeReport = d === 'devices' ? 'acts' : 'consumption';
        filter = {};
      }}
      onReportChange={(r) => {
        activeReport = r;
      }}
    />

    <!-- GAP-R4: controls row — PeriodSelector left, export buttons right -->
    <div class="controls-row">
      <PeriodSelector
        {period}
        isSnapshot={isSnapshot()}
        onPeriodChange={(p) => {
          period = p;
        }}
      />
      <ReportFilters
        reportDomain={activeDomain}
        reportType={activeReport}
        locationName={filter.location_name ?? null}
        statusId={filter.status_id ?? null}
        typeId={filter.type_id ?? null}
        modelId={filter.model_id ?? null}
        color={filter.color ?? null}
        search={filter.search ?? ''}
        locations={filterLocations}
        deviceTypes={filterDeviceTypes}
        cartridgeModels={filterCartridgeModels}
        cartridgeStatuses={filterCartridgeStatuses}
        cartridgeColors={filterCartridgeColors}
        onFilterChange={(f) => {
          filter = { ...filter, ...f };
        }}
        onExportCsv={exportCsv}
        onExportPdf={exportPdf}
        onPrint={printReport}
        {csvExporting}
        {pdfExporting}
      />
    </div>

    <ReportTable
      rows={rows?.rows ?? []}
      columns={currentColumns()}
      {loading}
      {error}
      reportType={activeReport}
      isSnapshot={isSnapshot()}
    />
  </div>
</div>

<style lang="scss">
  .reports-page {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .page-header {
    padding: var(--space-lg) var(--space-xl);
    border-bottom: 1px solid var(--color-border);
    flex-shrink: 0;
  }

  .page-title {
    margin: 0;
    font-size: var(--font-size-heading);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
  }

  .reports-content {
    flex: 1;
    overflow: auto;
    padding: 0 var(--space-xl);
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
  }

  // GAP-R4: period selector (left) + export buttons (right) on one row
  .controls-row {
    display: flex;
    align-items: flex-start;
    gap: var(--space-md);
    flex-wrap: wrap;
    padding: var(--space-xs) 0;
  }
</style>

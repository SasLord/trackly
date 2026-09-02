<script lang="ts">
  // Plan 07-06 Task 2: ReportsPage orchestrator.
  // Assembles sub-nav, period selector, filters, and table.
  // Loads dropdown data on mount; auto-reloads report on state changes.
  import { onMount } from 'svelte';
  import { apiCall } from '$lib/api/client';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { saveFile } from '$lib/utils/saveFile';
  import type { CartridgeModelDto } from '../../bindings';
  import PageHeader from '$lib/components/PageHeader.svelte';
  import ReportSubNav from './ReportSubNav.svelte';
  import PeriodSelector from './PeriodSelector.svelte';
  import ReportFilters from './ReportFilters.svelte';
  import RequestCategoryFilter from './RequestCategoryFilter.svelte';
  import ReportTable from './ReportTable.svelte';
  import PdfPreviewModal from '../acts/PdfPreviewModal.svelte';

  // Plan 40-18 (D-22): 'movements' mirrors ReportSubNav.svelte's own DomainKey
  // union — the two files each keep an independent local copy (pre-existing
  // duplication, e.g. REQUEST_REPORTS is likewise duplicated between the two
  // files), so both must be extended together or the two DomainKey types
  // stop being structurally compatible.
  type DomainKey = 'devices' | 'cartridges' | 'requests' | 'movements';

  interface PeriodDto {
    mode: string;
    year?: number | null;
    month?: number | null;
    date_from?: string | null;
    date_to?: string | null;
  }

  interface ReportFilter {
    status_id?: number | null;
    type_id?: number | null;
    model_id?: number | null;
    color?: string | null;
    search?: string | null;
    date_from_utc?: number | null;
    date_to_utc?: number | null;
    place_id?: number | null;
    is_storage?: boolean | null;
    act_type?: string | null;
    request_category_filter?: string[] | null;
    // Plan 40-18 (D-24): movements domain only — two independent
    // subtree-inclusive place filters, AND semantics on the backend.
    from_place_id?: number | null;
    to_place_id?: number | null;
  }

  interface ReportRow {
    id: number;
    month_key?: string | null;
    number?: string | null;
    sub_number?: string | null;
    giver_name?: string | null;
    receiver_name?: string | null;
    handover_date_utc?: number | null;
    place_path?: string | null;
    act_type?: string | null;
    device_name?: string | null;
    quantity?: number | null;
    code?: string | null;
    model_label?: string | null;
    status_name?: string | null;
    request_type_label?: string | null;
    // Plan 40-18 (D-23/D-25) — movements report row fields.
    from_place_path?: string | null;
    from_place_path_short?: string | null;
    actor_name?: string | null;
    reason?: string | null;
    entity_type_label?: string | null;
    is_deleted?: boolean | null;
    [key: string]: unknown;
  }

  interface ReportResponse {
    rows: ReportRow[];
    total: number;
  }

  interface Column {
    key: string;
    label: string;
    // Name of a sibling ReportRow field to prepend to place_path (see
    // ReportTable.svelte's formatPlaceCell) — never shortened by D-26.
    compositeWith?: string;
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

  const REQUEST_REPORTS = [
    { key: 'all', label: 'Все', temporal: true, cmd: 'reports_list_requests_all' },
    { key: 'open', label: 'Открытые', temporal: true, cmd: 'reports_list_requests_open' },
    {
      key: 'in_progress',
      label: 'В работе',
      temporal: true,
      cmd: 'reports_list_requests_in_progress',
    },
    {
      key: 'completed',
      label: 'Выполненные',
      temporal: true,
      cmd: 'reports_list_requests_completed',
    },
  ] as const;

  // Plan 40-18 (D-22): mirrors ReportSubNav.svelte's own MOVEMENT_REPORTS
  // copy (pre-existing per-domain-array duplication between the two files).
  const MOVEMENT_REPORTS = [
    { key: 'all', label: 'Все перемещения', temporal: true, cmd: 'reports_list_movements' },
  ] as const;

  // VAD-02: одинаковый набор колонок для всех 4 вкладок домена «Заявки».
  const REQUEST_COLUMNS: Column[] = [
    { key: 'number', label: '№' },
    { key: 'handover_date_utc', label: 'Дата' },
    { key: 'request_type_label', label: 'Тип' },
    { key: 'status_name', label: 'Статус' },
    { key: 'giver_name', label: 'Заявитель' },
    { key: 'place_path', label: 'Место', compositeWith: 'device_name' },
  ];

  // Column definitions per report type
  const COLUMNS_MAP: Record<string, Column[]> = {
    acts: [
      { key: 'number', label: 'Номер' },
      { key: 'giver_name', label: 'Сдал' },
      { key: 'receiver_name', label: 'Принял' },
      { key: 'handover_date_utc', label: 'Дата' },
      { key: 'place_path', label: 'Место' },
      { key: 'device_name', label: 'Устройства' },
    ],
    returns: [
      { key: 'number', label: 'Номер' },
      { key: 'sub_number', label: 'Суб-номер' },
      { key: 'giver_name', label: 'Сдал' },
      { key: 'receiver_name', label: 'Принял' },
      { key: 'handover_date_utc', label: 'Дата' },
      { key: 'place_path', label: 'Место' },
    ],
    in_use: [
      { key: 'device_name', label: 'Наименование' },
      { key: 'place_path', label: 'Место' },
      { key: 'status_name', label: 'Статус' },
    ],
    in_stock: [
      { key: 'device_name', label: 'Наименование' },
      { key: 'place_path', label: 'Место' },
      { key: 'status_name', label: 'Статус' },
    ],
    consumption: [
      { key: 'month_key', label: 'Месяц' },
      { key: 'model_label', label: 'Модель' },
      { key: 'code', label: 'Код картриджа' },
      { key: 'place_path', label: 'Место' },
    ],
    refills: [
      { key: 'month_key', label: 'Месяц' },
      { key: 'model_label', label: 'Модель' },
      { key: 'code', label: 'Код картриджа' },
      { key: 'place_path', label: 'Место' },
    ],
    cartridge_in_use: [
      { key: 'code', label: 'Код' },
      { key: 'model_label', label: 'Модель' },
      { key: 'place_path', label: 'Место' },
      { key: 'status_name', label: 'Статус' },
    ],
    cartridge_in_stock: [
      { key: 'code', label: 'Код' },
      { key: 'model_label', label: 'Модель' },
      { key: 'place_path', label: 'Место' },
      { key: 'status_name', label: 'Статус' },
    ],
    all: REQUEST_COLUMNS,
    open: REQUEST_COLUMNS,
    in_progress: REQUEST_COLUMNS,
    completed: REQUEST_COLUMNS,
    // Plan 40-18 (D-23) — order matches columns_for("movements") in
    // tauri_cmds/reports.rs exactly (Дата/Предмет/Тип/Откуда/Куда/Кем/Причина).
    // Own key 'movements', not 'all' — the movements domain's single report
    // type also uses key 'all' (see MOVEMENT_REPORTS), which would otherwise
    // collide with REQUEST_COLUMNS above; currentColumns() branches on
    // activeDomain to resolve this key instead of activeReport.
    movements: [
      { key: 'handover_date_utc', label: 'Дата' },
      { key: 'device_name', label: 'Предмет' },
      { key: 'entity_type_label', label: 'Тип' },
      { key: 'from_place_path', label: 'Откуда' },
      { key: 'place_path', label: 'Куда' },
      { key: 'actor_name', label: 'Кем' },
      { key: 'reason', label: 'Причина' },
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
  // Per-tab status counts (G2-5b)
  // ---------------------------------------------------------------------------
  let statusCounts = $state<Record<string, number>>({});
  // NON-reactive overlap guard. MUST NOT be $state: loadStatusCounts() is called
  // synchronously inside the auto-reload $effect, so reading a reactive flag here
  // would subscribe the effect to it; writing it (true→false) would then re-trigger
  // the effect, causing an infinite reload loop (see debug session
  // ui-ws-toast-reports-flicker). A plain let is read/written without reactivity.
  let countsLoading = false;

  // ---------------------------------------------------------------------------
  // Export state
  // ---------------------------------------------------------------------------
  let csvExporting = $state(false);
  let pdfExporting = $state(false);
  // Plan 17-03 (D-09/D-10): Экспорт PDF/Печать теперь открывают модалку
  // предпросмотра+печати (PdfPreviewModal mode="report"), которая сама
  // делает self-fetch reports_export_pdf — старый blob/download-путь удалён.
  let reportModalOpen = $state(false);

  // ---------------------------------------------------------------------------
  // Filter dropdown data
  // ---------------------------------------------------------------------------
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
  // VAD-01: домен «Заявки» — все 4 вкладки period-based (created_at_utc),
  // снимков (snapshot) в этом домене нет. activeReport ∈ {all, open,
  // in_progress, completed} — ни один ключ не совпадает с in_use/in_stock,
  // поэтому isSnapshot() уже корректно возвращает false здесь без изменений.
  function isSnapshot(): boolean {
    return ['in_use', 'in_stock'].includes(activeReport);
  }

  function currentCmd(): string {
    const allReports = [...DEVICE_REPORTS, ...CARTRIDGE_REPORTS, ...REQUEST_REPORTS];
    const found = allReports.find((r) => r.key === activeReport);
    // Movements domain: activeReport is 'all', which would otherwise
    // wrongly match REQUEST_REPORTS's own 'all' entry via `found` below —
    // resolve explicitly before falling through to the generic lookup.
    if (activeDomain === 'movements') {
      const movementFound = MOVEMENT_REPORTS.find((r) => r.key === activeReport);
      if (movementFound) return movementFound.cmd;
    }
    // For cartridge domain in_use/in_stock, we need to find correct cmd
    if (activeDomain === 'cartridges') {
      const cartridgeFound = CARTRIDGE_REPORTS.find((r) => r.key === activeReport);
      if (cartridgeFound) return cartridgeFound.cmd;
    }
    if (activeDomain === 'devices') {
      const deviceFound = DEVICE_REPORTS.find((r) => r.key === activeReport);
      if (deviceFound) return deviceFound.cmd;
    }
    if (activeDomain === 'requests') {
      const requestFound = REQUEST_REPORTS.find((r) => r.key === activeReport);
      if (requestFound) return requestFound.cmd;
    }
    return found?.cmd ?? 'reports_list_device_acts';
  }

  // GAP-R1: Maps domain + activeReport → backend report_type key expected by
  // reports_export_csv / reports_export_pdf (NOT the full Tauri command name).
  function reportTypeKey(): string {
    if (activeDomain === 'devices') {
      switch (activeReport) {
        case 'acts':
          return 'device_acts';
        case 'returns':
          return 'device_returns';
        case 'in_use':
          return 'device_in_use';
        case 'in_stock':
          return 'device_in_stock';
      }
    } else if (activeDomain === 'cartridges') {
      switch (activeReport) {
        case 'consumption':
          return 'cartridge_consumption';
        case 'refills':
          return 'cartridge_refills';
        case 'in_use':
          return 'cartridge_in_use';
        case 'in_stock':
          return 'cartridge_in_stock';
      }
    } else if (activeDomain === 'requests') {
      switch (activeReport) {
        case 'all':
          return 'requests_all';
        case 'open':
          return 'requests_open';
        case 'in_progress':
          return 'requests_in_progress';
        case 'completed':
          return 'requests_completed';
      }
    } else if (activeDomain === 'movements') {
      return 'movements';
    }
    return 'device_acts'; // fallback
  }

  function currentColumns(): Column[] {
    // Movements domain: own COLUMNS_MAP key, not keyed by activeReport (see
    // COLUMNS_MAP.movements comment for why 'all' would collide with Заявки).
    if (activeDomain === 'movements') {
      return COLUMNS_MAP.movements ?? [];
    }
    // For cartridge domain, use prefixed keys to differentiate from device in_use/in_stock
    if (
      activeDomain === 'cartridges' &&
      (activeReport === 'in_use' || activeReport === 'in_stock')
    ) {
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

  // ---------------------------------------------------------------------------
  // Per-tab counts loader (G2-5b)
  // ---------------------------------------------------------------------------

  function loadStatusCounts() {
    if (countsLoading) return;
    countsLoading = true;
    apiCall<{ counts: Array<{ key: string; count: number }> }>('reports_get_report_counts', {
      domain: activeDomain,
      period,
      filter,
    })
      .then((result) => {
        // Convert Vec<ReportCountEntry> array to Record<string,number> for O(1) tab lookup
        const map: Record<string, number> = {};
        for (const entry of result.counts) {
          map[entry.key] = entry.count;
        }
        statusCounts = map;
      })
      .catch(() => {
        // Non-fatal — badges fall back to rowCount / '–'
      })
      .finally(() => {
        countsLoading = false;
      });
  }

  // Auto-reload when domain / report / period / filter changes.
  //
  // IMPORTANT: every reactive read performed *synchronously* inside this effect
  // becomes a dependency. The load helpers below must therefore only read the
  // intended reactive state (activeDomain/activeReport/period/filter) and must
  // NOT read any reactive flag they also write (e.g. a loading flag), or the
  // write re-triggers the effect → infinite reload loop. countsLoading is a
  // plain (non-reactive) let for exactly this reason.
  $effect(() => {
    // Track reactive dependencies
    void activeDomain;
    void activeReport;
    void period;
    void filter;
    loadReport();
    loadStatusCounts();
  });

  // ---------------------------------------------------------------------------
  // Export handlers
  // ---------------------------------------------------------------------------
  // GAP-R1 sibling: builds a machine-readable, collision-resistant filename
  // (report type key + local ISO date) so repeated exports on the same day
  // don't collapse into a single «отчёт.csv» / «отчёт (1).csv».
  //
  // Uses local Y/M/D getters, NOT toISOString() — toISOString() returns the
  // UTC date, which can drift by a day around midnight Moscow time.
  function buildCsvFilename(): string {
    const now = new Date();
    const y = String(now.getFullYear());
    const m = String(now.getMonth() + 1).padStart(2, '0');
    const d = String(now.getDate()).padStart(2, '0');
    return `отчёт-${reportTypeKey()}-${y}-${m}-${d}.csv`;
  }

  async function exportCsv() {
    csvExporting = true;
    try {
      const bytes = await apiCall<number[]>('reports_export_csv', {
        reportType: reportTypeKey(),
        filter,
        period: isSnapshot() ? undefined : period,
      });
      const result = await saveFile(
        new Uint8Array(bytes),
        buildCsvFilename(),
        'text/csv;charset=utf-8',
      );
      if (result === 'saved') {
        pushToast('success', 'CSV-файл сохранён');
      }
      // result === 'cancelled' (user closed the save dialog) is a normal
      // action, not an error — no toast.
    } catch {
      pushToast('error', 'Ошибка при экспорте CSV. Попробуйте ещё раз.');
    } finally {
      csvExporting = false;
    }
  }

  // Plan 17-03 (D-10): both «Экспорт PDF» and «Печать» now open the same
  // preview+print modal (PdfPreviewModal mode="report"), which self-fetches
  // reports_export_pdf (now HTML, Phase 17-01) on open. The old blob/
  // save-dialog download path and the separate printReport() function
  // (which used to shell out to native file-save + open plugins) are gone.
  function exportPdf() {
    reportModalOpen = true;
  }

  // ---------------------------------------------------------------------------
  // onMount: load dynamic filter data
  // ---------------------------------------------------------------------------
  onMount(async () => {
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
  <PageHeader title="Отчёты" />

  <div class="reports-content">
    <ReportSubNav
      {activeDomain}
      {activeReport}
      rowCount={rows?.total ?? 0}
      {statusCounts}
      onDomainChange={(d) => {
        activeDomain = d;
        activeReport = d === 'devices' ? 'acts' : d === 'cartridges' ? 'consumption' : 'all';
        filter = {};
      }}
      onReportChange={(r) => {
        activeReport = r;
      }}
    />

    <!-- GAP-R4: controls row — PeriodSelector left, export buttons right -->
    <div class="controls-row">
      <div class="controls-left">
        <PeriodSelector
          {period}
          isSnapshot={isSnapshot()}
          onPeriodChange={(p) => {
            period = p;
          }}
        />
        {#if activeDomain === 'requests'}
          <RequestCategoryFilter
            selectedKeys={filter.request_category_filter ?? null}
            onChange={(keys) => {
              filter = { ...filter, request_category_filter: keys };
            }}
          />
        {/if}
      </div>
      <ReportFilters
        reportDomain={activeDomain}
        reportType={activeReport}
        placeId={filter.place_id ?? null}
        isStorage={filter.is_storage ?? null}
        fromPlaceId={filter.from_place_id ?? null}
        toPlaceId={filter.to_place_id ?? null}
        statusId={filter.status_id ?? null}
        typeId={filter.type_id ?? null}
        modelId={filter.model_id ?? null}
        color={filter.color ?? null}
        search={filter.search ?? ''}
        deviceTypes={filterDeviceTypes}
        cartridgeModels={filterCartridgeModels}
        cartridgeStatuses={filterCartridgeStatuses}
        cartridgeColors={filterCartridgeColors}
        onFilterChange={(f) => {
          filter = { ...filter, ...f };
        }}
        onExportCsv={exportCsv}
        onExportPdf={exportPdf}
        onPrint={exportPdf}
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

  <PdfPreviewModal
    open={reportModalOpen}
    actId={null}
    mode="report"
    title="Печать отчёта"
    reportParams={{
      reportType: reportTypeKey(),
      filter,
      period: isSnapshot() ? undefined : period,
    }}
    onClose={() => {
      reportModalOpen = false;
    }}
  />
</div>

<style lang="scss">
  .reports-page {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .reports-content {
    flex: 1;
    min-height: 0;
    overflow-x: auto;
    overflow-y: hidden;
    // UAT (Отчёты): the framed table was flush against the window's bottom
    // edge — add a bottom breathing gap so the card doesn't touch the edge,
    // matching the vertical rhythm of the other windows (Акты page-content
    // uses --tr-space-xl). Top stays 0 (the sub-nav hugs the PageHeader as
    // before); only the bottom gap was missing.
    padding: 0 var(--tr-space-2xl) var(--tr-space-xl);
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-xs);
  }

  // GAP-R4: period selector (left) + export buttons (right) on one row
  // G2-5a: space-between puts PeriodSelector flush-left and ReportFilters (export block)
  // flush-right; align-items:center keeps them vertically centered on the same baseline.
  .controls-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--tr-space-md);
    flex-wrap: wrap;
    padding: var(--tr-space-2xs) 0;
  }

  // CATF-01 (260821-w18): keeps RequestCategoryFilter directly adjacent to
  // PeriodSelector on the left, so .controls-row's space-between still puts
  // ReportFilters (export block) flush-right regardless of domain.
  .controls-left {
    display: flex;
    align-items: center;
    gap: var(--tr-space-sm);
  }
</style>

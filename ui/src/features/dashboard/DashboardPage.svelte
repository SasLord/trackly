<script lang="ts">
  // Plan 07-05: DashboardPage — главная страница дашборда.
  // 5 виджетов в 2-колоночной адаптивной сетке (breakpoint 1280px).
  // Параллельная загрузка виджетов с независимыми состояниями ошибок (D-10).
  // API: dashboard_get_all_widgets (DASH-01..05) + dashboard_get_consumption_chart (DASH-03).
  import { apiCall } from '$lib/api/client';
  import PageHeader from '$lib/components/PageHeader.svelte';
  import StatWidget from './StatWidget.svelte';
  import ChartWidget from './ChartWidget.svelte';

  // DTO-типы (snake_case по решению 07-01: snake_case JSON in Phase 7 DTOs)
  interface StatusCount {
    status_name: string;
    count: number;
  }

  interface DashboardWidgetDto {
    devices_total: number;
    devices_by_status: StatusCount[];
    cartridge_by_status: StatusCount[];
    low_stock_count: number;
    low_stock_models: string[];
    request_counts_open: number;
    request_counts_in_progress: number;
    request_counts_completed: number;
    printer_online: number;
    printer_offline: number;
    printer_problematic: number;
  }

  interface ConsumptionPoint {
    month_key: string;
    model_label: string;
    installs: number;
  }

  // Русские названия месяцев (для селектора периода)
  const MONTHS = [
    'Январь',
    'Февраль',
    'Март',
    'Апрель',
    'Май',
    'Июнь',
    'Июль',
    'Август',
    'Сентябрь',
    'Октябрь',
    'Ноябрь',
    'Декабрь',
  ];

  // Диапазон лет для периода (текущий год ± 3)
  const currentYear = new Date().getFullYear();
  const YEARS = Array.from({ length: 7 }, (_, i) => currentYear - 3 + i);

  // Состояние виджетов
  let widgetData = $state<DashboardWidgetDto | null>(null);
  let widgetError = $state<string | null>(null);
  let widgetsLoading = $state(true);

  // Состояние графика
  let chartData = $state<ConsumptionPoint[]>([]);
  let chartError = $state<string | null>(null);
  let chartLoading = $state(true);

  // Период (для period-sensitive виджетов)
  let periodMonth = $state(new Date().getMonth() + 1);
  let periodYear = $state(currentYear);

  // Период для графика
  let windowMonths = $state<3 | 6 | 12>(3);

  // Флаг — первый запуск (чтобы $effect не перезагружал при инициализации)
  let mounted = $state(false);

  async function loadWidgets() {
    widgetsLoading = true;
    widgetError = null;
    try {
      widgetData = await apiCall<DashboardWidgetDto>('dashboard_get_all_widgets', {
        period: { mode: 'month', year: periodYear, month: periodMonth },
      });
    } catch (e: unknown) {
      widgetError =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось загрузить данные виджетов';
      widgetData = null;
    } finally {
      widgetsLoading = false;
    }
  }

  async function loadChart() {
    chartLoading = true;
    chartError = null;
    try {
      chartData = await apiCall<ConsumptionPoint[]>('dashboard_get_consumption_chart', {
        windowMonths,
      });
    } catch (e: unknown) {
      chartError =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось загрузить данные графика';
      chartData = [];
    } finally {
      chartLoading = false;
    }
  }

  // Начальная загрузка при монтировании (параллельно)
  $effect(() => {
    if (mounted) return;
    mounted = true;
    void loadWidgets();
    void loadChart();
  });

  // Перезагрузка графика при смене периода графика
  $effect(() => {
    // Читаем windowMonths, чтобы $effect отслеживал это значение
    void windowMonths;
    if (!mounted) return;
    void loadChart();
  });

  function reloadWidgets() {
    void loadWidgets();
  }

  function handleWindowChange(months: 3 | 6 | 12) {
    windowMonths = months;
  }
</script>

<div class="dashboard-page">
  <PageHeader title="Дашборд">
    {#snippet actions()}
      <!-- Селектор периода для period-sensitive виджетов (D-12) -->
      <div class="period-selector" role="group" aria-label="Период">
        <select
          bind:value={periodMonth}
          onchange={reloadWidgets}
          aria-label="Месяц"
          class="period-select"
        >
          {#each MONTHS as name, i}
            <option value={i + 1}>{name}</option>
          {/each}
        </select>
        <select
          bind:value={periodYear}
          onchange={reloadWidgets}
          aria-label="Год"
          class="period-select"
        >
          {#each YEARS as y}
            <option value={y}>{y}</option>
          {/each}
        </select>
      </div>
    {/snippet}
  </PageHeader>

  <div class="dashboard-grid">
    <!-- Ряд статистик: Устройства, Картриджи, Заявки, Принтеры -->
    <div class="stat-row">
      <StatWidget
        id="devices"
        title="Устройства"
        mainNumber={widgetData?.devices_total ?? null}
        mainLabel="устройств в базе"
        breakdown={widgetData?.devices_by_status.map((s) => ({
          label: s.status_name,
          count: s.count,
        })) ?? []}
        loading={widgetsLoading}
        error={widgetData === null && !widgetsLoading ? widgetError : null}
      />

      <StatWidget
        id="cartridges"
        title="Картриджи"
        mainNumber={widgetData
          ? widgetData.cartridge_by_status.reduce((a, s) => a + s.count, 0)
          : null}
        mainLabel="картриджей"
        breakdown={widgetData?.cartridge_by_status.map((s) => ({
          label: s.status_name,
          count: s.count,
        })) ?? []}
        warningItems={widgetData?.low_stock_models ?? []}
        loading={widgetsLoading}
        error={widgetData === null && !widgetsLoading ? widgetError : null}
      />

      <StatWidget
        id="requests"
        title="Заявки"
        mainNumber={widgetData?.request_counts_open ?? null}
        mainLabel="активных"
        breakdown={widgetData
          ? [
              { label: 'Новые', count: widgetData.request_counts_open },
              { label: 'В работе', count: widgetData.request_counts_in_progress },
              { label: 'Выполнены', count: widgetData.request_counts_completed },
            ]
          : []}
        loading={widgetsLoading}
        error={null}
      />

      <StatWidget
        id="printers"
        title="Принтеры"
        mainNumber={widgetData?.printer_online ?? null}
        mainLabel="онлайн"
        breakdown={widgetData
          ? [
              { label: 'Онлайн', count: widgetData.printer_online },
              { label: 'Офлайн', count: widgetData.printer_offline },
              { label: 'Проблемные', count: widgetData.printer_problematic },
            ]
          : []}
        loading={widgetsLoading}
        error={null}
      />
    </div>

    <ChartWidget
      data={chartData}
      {windowMonths}
      loading={chartLoading}
      error={chartError}
      onWindowChange={handleWindowChange}
    />
  </div>
</div>

<style lang="scss">
  @use '../../styles/_breakpoints' as bp;

  .dashboard-page {
    display: flex;
    flex-direction: column;
    // Anchor the page directly to the viewport (height: 100vh), NOT to the parent
    // chain. `height: 100%` / `flex: 1` both depend on .content (Layout.svelte, the
    // app shell) resolving to a definite height — which it does not reliably do in
    // WKWebView, and which Svelte HMR often fails to hot-apply for the shell anyway.
    // 100vh is viewport-absolute and lands via this leaf component's own HMR, so the
    // page fills exactly one screen and .dashboard-grid scrolls internally instead
    // of the whole app shell overflowing past the bottom (Gap 2, QA-03). min-height:0
    // defeats the flex default `min-height: auto` that would otherwise let the tall
    // (un-virtualised StatWidget + ChartWidget) content expand the box.
    height: 100vh;
    min-height: 0;
  }

  .period-selector {
    display: flex;
    gap: 10px;
  }

  .period-select {
    height: 32px;
    padding: 0 var(--tr-space-xs);
    background: var(--tr-surface);
    border: 1px solid var(--tr-border-strong);
    border-radius: 6px;
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-primary);
    cursor: pointer;

    &:focus-visible {
      outline: none;
      border-color: var(--tr-accent);
      box-shadow: 0 0 0 3px var(--tr-focus-ring);
    }
  }

  .dashboard-grid {
    flex: 1;
    min-height: 0;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 24px;
  }

  .stat-row {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 16px;
  }

  @media (max-width: bp.$bp-xl) {
    .stat-row {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }

  @media (max-width: bp.$bp-sm) {
    .stat-row {
      grid-template-columns: minmax(0, 1fr);
    }
    .dashboard-grid {
      padding: 16px;
    }
  }
</style>

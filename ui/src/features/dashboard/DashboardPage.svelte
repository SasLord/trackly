<script lang="ts">
  // Plan 07-05: DashboardPage — главная страница дашборда.
  // 5 виджетов в 2-колоночной адаптивной сетке (breakpoint 1280px).
  // Параллельная загрузка виджетов с независимыми состояниями ошибок (D-10).
  // API: dashboard_get_all_widgets (DASH-01..05) + dashboard_get_consumption_chart (DASH-03).
  import { apiCall } from '$lib/api/client';
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
  <header class="page-header">
    <h1 class="page-title">Дашборд</h1>
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
  </header>

  <div class="dashboard-grid">
    <!-- Колонка 1 (основная): Устройства, Картриджи, График -->
    <div class="dashboard-col dashboard-col--main">
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

      <ChartWidget
        data={chartData}
        {windowMonths}
        loading={chartLoading}
        error={chartError}
        onWindowChange={handleWindowChange}
      />
    </div>

    <!-- Колонка 2 (боковая): Заявки, Принтеры -->
    <div class="dashboard-col dashboard-col--side">
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
  </div>
</div>

<style lang="scss">
  .dashboard-page {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .page-header {
    padding: var(--tr-space-xl) var(--tr-space-2xl);
    border-bottom: 1px solid var(--tr-border);
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: var(--tr-space-md);
    flex-wrap: wrap;
  }

  .page-title {
    margin: 0;
    font-size: var(--tr-font-size-h3);
    font-weight: var(--tr-font-weight-semibold);
    color: var(--tr-text-primary);
    flex: 1;
  }

  .period-selector {
    display: flex;
    gap: var(--tr-space-xs);
  }

  .period-select {
    height: 32px;
    padding: 0 var(--tr-space-xs);
    background: var(--tr-bg);
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-xs);
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
    overflow: auto;
    display: grid;
    grid-template-columns: 3fr 2fr;
    gap: var(--tr-space-2xl);
    padding: var(--tr-space-2xl);
    align-content: start;
  }

  .dashboard-col {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-xl);
  }

  @media (max-width: 1280px) {
    .dashboard-grid {
      grid-template-columns: 1fr;
    }
  }
</style>

<script lang="ts">
  // Plan 260702-vtf-01: ChartWidget — grouped bar chart с Y-осью, сеткой, value labels и tooltip.
  // DASH-03, D-11: 3/6/12 месяцев, multi-model серии, aria-доступность.
  import Spinner from '$lib/components/Spinner.svelte';
  import PeriodToggle from './PeriodToggle.svelte';

  interface ConsumptionPoint {
    month_key: string;
    model_label: string;
    installs: number;
  }

  interface Props {
    data: ConsumptionPoint[];
    windowMonths: 3 | 6 | 12;
    loading: boolean;
    error: string | null;
    onWindowChange: (months: 3 | 6 | 12) => void;
  }

  const { data, windowMonths, loading, error, onWindowChange }: Props = $props();

  // Цвета для серий (до 3 моделей)
  const COLORS = ['var(--tr-accent)', 'var(--tr-success)', 'var(--tr-warning)'];

  // Русские названия месяцев для подписей осей
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

  // Coordinate system constants
  const LEFT_PAD = 42;
  const RIGHT_PAD = 8;
  const TOP_PAD = 20;
  const BOTTOM_PAD = 28;
  const CHART_TOTAL_W = 500;
  const CHART_TOTAL_H = 220;
  const CHART_W = CHART_TOTAL_W - LEFT_PAD - RIGHT_PAD;
  const CHART_H = CHART_TOTAL_H - TOP_PAD - BOTTOM_PAD;

  // Tooltip state
  interface TooltipState {
    visible: boolean;
    x: number;
    y: number;
    month: string;
    model: string;
    installs: number;
  }
  let tooltip = $state<TooltipState>({
    visible: false,
    x: 0,
    y: 0,
    month: '',
    model: '',
    installs: 0,
  });

  // Форматирует month_key "2026-06" в короткое название "Июн."
  function monthKeyToLabel(key: string): string {
    const parts = key.split('-');
    if (parts.length < 2) return key;
    const monthIdx = parseInt(parts[1], 10) - 1;
    const name = MONTHS[monthIdx] ?? key;
    return name.slice(0, 3) + '.';
  }

  // Производные данные
  const uniqueMonths = $derived(
    (() => {
      const set = new Set<string>();
      data.forEach((p) => set.add(p.month_key));
      return Array.from(set).sort();
    })(),
  );

  const modelKeys = $derived(
    (() => {
      const set = new Set<string>();
      data.forEach((p) => set.add(p.model_label));
      return Array.from(set).slice(0, 3);
    })(),
  );

  // Map: model_label -> массив значений installs по уникальным месяцам
  const seriesData = $derived<Record<string, number[]>>(
    (() => {
      const result: Record<string, number[]> = {};
      for (const model of modelKeys) {
        result[model] = uniqueMonths.map((month) => {
          const point = data.find((p) => p.model_label === model && p.month_key === month);
          return point?.installs ?? 0;
        });
      }
      return result;
    })(),
  );

  const maxVal = $derived(
    (() => {
      let max = 1;
      for (const model of modelKeys) {
        for (const month of uniqueMonths) {
          const point = data.find((p) => p.model_label === model && p.month_key === month);
          if (point && point.installs > max) max = point.installs;
        }
      }
      return max;
    })(),
  );

  // yStep: целый «удобный» шаг оси Y — наименьший из ряда 1/2/5/…,
  // дающий не более 5 интервалов. Целый шаг гарантирует целые деления
  // без пропусков и дублей (installs — всегда целые). T-vtf-01: >= 1.
  const yStep = $derived(
    (() => {
      const raw = Math.max(maxVal, 1);
      const candidates = [1, 2, 5, 10, 20, 50, 100, 200, 500, 1000];
      for (const c of candidates) {
        if (Math.ceil(raw / c) <= 5) return c;
      }
      return candidates[candidates.length - 1];
    })(),
  );

  // niceMax: округлить вверх до кратного yStep, не менее 1
  const niceMax = $derived(Math.max(Math.ceil(maxVal / yStep) * yStep, 1));

  // Y-axis ticks: целые деления от 0 до niceMax с шагом yStep (без пропусков)
  const yTicks = $derived(
    (() => {
      const ticks: { value: number; y: number }[] = [];
      for (let value = 0; value <= niceMax; value += yStep) {
        const y = TOP_PAD + CHART_H - (value / niceMax) * CHART_H;
        ticks.push({ value, y });
      }
      return ticks;
    })(),
  );

  // Bar layout: массив объектов с координатами для каждого bar
  interface BarItem {
    x: number;
    y: number;
    width: number;
    height: number;
    color: string;
    installs: number;
    model: string;
    monthLabel: string;
  }

  const barLayout = $derived<BarItem[]>(
    (() => {
      const N = uniqueMonths.length;
      const M = modelKeys.length;
      if (N === 0 || M === 0) return [];

      const GROUP_W = CHART_W / N;
      const BAR_GAP = 2;
      const BAR_W = Math.max(4, (GROUP_W - BAR_GAP * (M + 1)) / M);

      const bars: BarItem[] = [];
      for (let i = 0; i < N; i++) {
        const month = uniqueMonths[i];
        for (let mi = 0; mi < M; mi++) {
          const model = modelKeys[mi];
          const installs = seriesData[model]?.[i] ?? 0;
          const barH = (installs / niceMax) * CHART_H;
          const x = LEFT_PAD + i * GROUP_W + BAR_GAP * (mi + 1) + mi * BAR_W;
          const y = TOP_PAD + CHART_H - barH;
          bars.push({
            x,
            y,
            width: BAR_W,
            height: barH,
            color: COLORS[mi % COLORS.length],
            installs,
            model,
            monthLabel: monthKeyToLabel(month),
          });
        }
      }
      return bars;
    })(),
  );

  // X-axis label positions (по центру группы)
  const xLabels = $derived(
    (() => {
      const N = uniqueMonths.length;
      if (N === 0) return [];
      const GROUP_W = CHART_W / N;
      return uniqueMonths.map((month, i) => ({
        label: monthKeyToLabel(month),
        x: LEFT_PAD + i * GROUP_W + GROUP_W / 2,
        y: TOP_PAD + CHART_H + 16,
      }));
    })(),
  );
</script>

<section class="chart-widget" aria-labelledby="chart-title">
  <div class="chart-header">
    <h2 class="widget-title" id="chart-title">Динамика расхода картриджей</h2>
    <PeriodToggle {windowMonths} {onWindowChange} />
  </div>

  {#if loading}
    <div class="chart-state">
      <Spinner size="md" />
    </div>
  {:else if error}
    <div class="chart-state chart-error">Ошибка загрузки</div>
  {:else if data.length === 0}
    <div class="chart-state chart-empty">Нет данных о расходе за выбранный период</div>
  {:else}
    <div class="chart-area">
      <svg
        role="img"
        aria-label="График динамики расхода картриджей за {windowMonths} месяцев"
        viewBox="0 0 {CHART_TOTAL_W} {CHART_TOTAL_H}"
        preserveAspectRatio="xMidYMid meet"
        class="chart-svg"
      >
        <!-- Y-axis gridlines и метки -->
        {#each yTicks as tick}
          <line
            x1={LEFT_PAD}
            x2={CHART_TOTAL_W - RIGHT_PAD}
            y1={tick.y}
            y2={tick.y}
            stroke="var(--tr-border)"
            stroke-width="0.5"
          />
          <text
            x={LEFT_PAD - 4}
            y={tick.y + 3}
            text-anchor="end"
            font-size="9"
            fill="var(--tr-text-tertiary)">{tick.value}</text
          >
        {/each}

        <!-- Grouped bars -->
        {#each barLayout as bar}
          <rect
            x={bar.x}
            y={bar.y}
            width={bar.width}
            height={bar.height}
            fill={bar.color}
            rx="2"
            style="cursor: crosshair;"
            onmouseenter={(e) => {
              const target = e.currentTarget as SVGRectElement;
              const svgEl = target.closest('svg') as SVGSVGElement | null;
              const areaEl = svgEl?.parentElement as HTMLElement | null;
              if (!areaEl) return;
              const areaRect = areaEl.getBoundingClientRect();
              tooltip = {
                visible: true,
                x: e.clientX - areaRect.left,
                y: e.clientY - areaRect.top,
                month: bar.monthLabel,
                model: bar.model,
                installs: bar.installs,
              };
            }}
            onmouseleave={() => {
              tooltip.visible = false;
            }}
          />
          <!-- Value label над баром (только если installs > 0) -->
          {#if bar.installs > 0}
            <text
              x={bar.x + bar.width / 2}
              y={bar.y - 3}
              text-anchor="middle"
              font-size="8"
              fill={bar.color}>{bar.installs}</text
            >
          {/if}
        {/each}

        <!-- X-axis labels (месяцы) -->
        {#each xLabels as lbl}
          <text
            x={lbl.x}
            y={lbl.y}
            text-anchor="middle"
            font-size="9"
            fill="var(--tr-text-tertiary)">{lbl.label}</text
          >
        {/each}
      </svg>

      <!-- Hover tooltip -->
      {#if tooltip.visible}
        <div class="chart-tooltip" style="left: {tooltip.x + 10}px; top: {tooltip.y - 28}px;">
          {tooltip.month} · {tooltip.model}: {tooltip.installs}
        </div>
      {/if}
    </div>

    <!-- Визуально скрытая таблица данных для доступности (Screen Reader) -->
    <table class="sr-only" aria-label="Данные графика расхода картриджей">
      <thead>
        <tr>
          <th scope="col">Месяц</th>
          {#each modelKeys as m}
            <th scope="col">{m}</th>
          {/each}
        </tr>
      </thead>
      <tbody>
        {#each uniqueMonths as month}
          <tr>
            <td>{month}</td>
            {#each modelKeys as m}
              <td>{seriesData[m]?.[uniqueMonths.indexOf(month)] ?? 0}</td>
            {/each}
          </tr>
        {/each}
      </tbody>
    </table>

    <!-- Легенда моделей -->
    {#if modelKeys.length > 1}
      <ul class="chart-legend" aria-label="Легенда графика">
        {#each modelKeys as model, mi}
          <li class="legend-item">
            <span class="legend-dot" style="background: {COLORS[mi % COLORS.length]}"></span>
            {model}
          </li>
        {/each}
      </ul>
    {/if}
  {/if}
</section>

<style lang="scss">
  .chart-widget {
    background: var(--tr-surface);
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-md);
    padding: var(--tr-space-xl);
    min-height: 220px;
  }

  .chart-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: var(--tr-space-md);
    gap: var(--tr-space-xs);
    flex-wrap: wrap;
  }

  .widget-title {
    margin: 0;
    font-size: var(--tr-font-size-body);
    font-weight: var(--tr-font-weight-semibold);
    color: var(--tr-text-primary);
  }

  .chart-area {
    position: relative;
  }

  .chart-svg {
    width: 100%;
    height: 180px;
    display: block;
  }

  .chart-tooltip {
    position: absolute;
    pointer-events: none;
    background: var(--tr-surface);
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-xs);
    padding: 4px 8px;
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-primary);
    white-space: nowrap;
    box-shadow: var(--tr-elev-2);
    z-index: 10;
  }

  .chart-state {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 150px;
    color: var(--tr-text-tertiary);
    font-size: var(--tr-font-size-label);
  }

  .chart-error {
    color: var(--tr-text-tertiary);
  }

  .chart-empty {
    color: var(--tr-text-tertiary);
    text-align: center;
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
  }

  .chart-legend {
    display: flex;
    gap: var(--tr-space-md);
    flex-wrap: wrap;
    list-style: none;
    padding: 0;
    margin: var(--tr-space-xs) 0 0;
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-secondary);
  }

  .legend-item {
    display: flex;
    align-items: center;
    gap: var(--tr-space-2xs);
  }

  .legend-dot {
    display: inline-block;
    width: 10px;
    height: 10px;
    border-radius: 50%;
    flex-shrink: 0;
  }
</style>

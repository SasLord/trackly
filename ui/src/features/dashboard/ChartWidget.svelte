<script lang="ts">
  // Plan 07-05: ChartWidget — виджет динамики расхода картриджей.
  // SVG polyline (ручная отрисовка, нет npm-зависимостей).
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

  // Цвета для серий (до 3 моделей; остальные схлопываются)
  const COLORS = ['var(--color-accent)', 'var(--color-success)', 'var(--color-warning)'];

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

  // Вычисляет SVG-координаты точек серии.
  // Одна точка центрируется по X (единственный месяц → маркер по середине графика).
  function toCoords(series: number[], maxVal: number): { x: number; y: number }[] {
    const n = series.length;
    return series.map((v, i) => {
      // При n === 1 ставим точку в центр (x = 200); иначе распределяем по ширине.
      const x = n < 2 ? 200 : (i / (n - 1)) * 380 + 10;
      const y = 190 - (v / (maxVal || 1)) * 170;
      return { x, y };
    });
  }

  // Преобразует координаты в строку points для polyline (нужно >= 2 точек).
  function toPolyline(coords: { x: number; y: number }[]): string {
    if (coords.length < 2) return '';
    return coords.map((p) => `${p.x.toFixed(1)},${p.y.toFixed(1)}`).join(' ');
  }

  // Форматирует month_key "2026-06" в короткое название "Июн."
  function monthKeyToLabel(key: string): string {
    const parts = key.split('-');
    if (parts.length < 2) return key;
    const monthIdx = parseInt(parts[1], 10) - 1;
    const name = MONTHS[monthIdx] ?? key;
    return name.slice(0, 3) + '.';
  }

  // Производные данные для SVG
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
      return Array.from(set).slice(0, 3); // показываем до 3 моделей
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
    <svg
      role="img"
      aria-label="График динамики расхода картриджей за {windowMonths} месяцев"
      viewBox="0 0 400 200"
      preserveAspectRatio="xMidYMid meet"
      class="chart-svg"
    >
      {#each modelKeys as model, mi}
        {@const coords = toCoords(seriesData[model] ?? [], maxVal)}
        <!-- Линия (только при >= 2 месяцах) -->
        <polyline
          points={toPolyline(coords)}
          fill="none"
          stroke={COLORS[mi % COLORS.length]}
          stroke-width="2"
          stroke-linejoin="round"
          stroke-linecap="round"
        />
        <!-- Маркеры точек: гарантируют видимость данных даже при одном месяце -->
        {#each coords as pt}
          <circle cx={pt.x.toFixed(1)} cy={pt.y.toFixed(1)} r="3" fill={COLORS[mi % COLORS.length]} />
        {/each}
      {/each}
      <!-- Подписи месяцев по оси X -->
      {#each uniqueMonths as month, i}
        {@const total = uniqueMonths.length}
        <text
          x={(total < 2 ? 200 : (i / (total - 1)) * 380 + 10).toFixed(1)}
          y="198"
          font-size="9"
          text-anchor="middle"
          fill="var(--color-text-muted)"
        >
          {monthKeyToLabel(month)}
        </text>
      {/each}
    </svg>

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
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    padding: var(--space-lg);
    min-height: 220px;
  }

  .chart-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: var(--space-md);
    gap: var(--space-sm);
    flex-wrap: wrap;
  }

  .widget-title {
    margin: 0;
    font-size: var(--font-size-body);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
  }

  .chart-svg {
    width: 100%;
    height: 180px;
    display: block;
  }

  .chart-state {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 150px;
    color: var(--color-text-muted);
    font-size: var(--font-size-label);
  }

  .chart-error {
    color: var(--color-text-muted);
  }

  .chart-empty {
    color: var(--color-text-muted);
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
    gap: var(--space-md);
    flex-wrap: wrap;
    list-style: none;
    padding: 0;
    margin: var(--space-sm) 0 0;
    font-size: var(--font-size-label);
    color: var(--color-text-secondary);
  }

  .legend-item {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
  }

  .legend-dot {
    display: inline-block;
    width: 10px;
    height: 10px;
    border-radius: 50%;
    flex-shrink: 0;
  }
</style>

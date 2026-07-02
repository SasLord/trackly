---
phase: 260702-vtf-y-tooltip
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - ui/src/features/dashboard/ChartWidget.svelte
autonomous: false
requirements:
  - DASH-03
  - D-11

must_haves:
  truths:
    - "Y-ось отображает числовые метки (~4–5 тиков) с горизонтальными сетчатыми линиями"
    - "График — сгруппированные вертикальные столбцы (bars), по одному на модель в каждом месяце"
    - "Над каждым столбцом напечатано значение installs"
    - "При наведении на столбец показывается tooltip с месяцем, моделью и installs"
    - "Одиночный месяц (1 группа) рендерится корректно — не пустой график"
    - "Состояния loading / error / empty сохранены, sr-only таблица сохранена, легенда сохранена"
    - "svelte-check 0 новых ошибок; pnpm --dir ui build проходит"
  artifacts:
    - path: "ui/src/features/dashboard/ChartWidget.svelte"
      provides: "Полностью переписанный виджет grouped bar chart"
      min_lines: 200
  key_links:
    - from: "ChartWidget.svelte script"
      to: "ChartWidget.svelte SVG template"
      via: "$derived barLayout: вычисляет x/y/width/height для каждого bar"
      pattern: "barLayout"
    - from: "SVG <rect> bar"
      to: "tooltip $state"
      via: "onmouseenter/onmouseleave handlers → $state tooltip object"
      pattern: "tooltip"
---

<objective>
Переписать ChartWidget.svelte: ручной SVG-линейный график → сгруппированная столбчатая диаграмма с осью Y, сеткой, подписями значений и hover-tooltip.

Purpose: Текущий линейный график не читаем — нет оси Y, нет числовых меток, одиночный месяц давал невидимую линию. Пользователь не может понять масштаб расхода.

Output: ChartWidget.svelte полностью переписан (только этот файл). Все состояния (loading/error/empty), sr-only таблица, легенда и PeriodToggle сохранены. Интерфейс Props/ConsumptionPoint не меняется.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@ui/src/features/dashboard/ChartWidget.svelte
</context>

<tasks>

<task type="auto" tdd="false">
  <name>Task 1: Переписать ChartWidget.svelte — grouped bar chart с Y-осью, value labels и tooltip</name>
  <files>ui/src/features/dashboard/ChartWidget.svelte</files>
  <action>
Полностью заменить SVG-реализацию внутри ChartWidget.svelte. Интерфейсы Props и ConsumptionPoint, импорты, MONTHS, modelKeys, uniqueMonths, seriesData, maxVal — оставить как есть (или расширить $derived-переменными). Удалить toCoords() и toPolyline().

**Coordinate system (viewBox):**
- Новый viewBox: `0 0 500 220`
- LEFT_PAD = 42 (место для Y-axis labels: до 4 цифр + отступ)
- RIGHT_PAD = 8
- TOP_PAD = 20 (место для value labels над верхними барами)
- BOTTOM_PAD = 28 (место для X-axis labels месяцев)
- CHART_W = 500 - LEFT_PAD - RIGHT_PAD
- CHART_H = 220 - TOP_PAD - BOTTOM_PAD

**Y-axis + gridlines:**
Вычислить 5 равномерных тиков от 0 до maxVal (округлить maxVal вверх до удобного числа — niceMax: ближайший кратный 5 или 10 выше maxVal, но не менее 1). Для каждого тика:
- горизонтальная линия `<line x1={LEFT_PAD} x2={500 - RIGHT_PAD} y1={y} y2={y} stroke="var(--color-border)" stroke-width="0.5" />`
- текстовый label слева: `<text x={LEFT_PAD - 4} y={y + 3} text-anchor="end" font-size="9" fill="var(--color-text-muted)">{tick}</text>`

**Grouped bars ($derived barLayout):**
Для N месяцев и M моделей (M ≤ 3):
- GROUP_W = CHART_W / N
- BAR_GAP = 2
- BAR_W = (GROUP_W - BAR_GAP * (M + 1)) / M (не менее 4px)
Для каждой пары (monthIdx i, modelIdx mi):
- x = LEFT_PAD + i * GROUP_W + BAR_GAP * (mi + 1) + mi * BAR_W
- barH = (installs / niceMax) * CHART_H (если installs === 0 → barH = 0)
- y = TOP_PAD + CHART_H - barH
- rx = 2 (скруглённые углы)
Сохранить в массив объектов: `{ x, y, width: BAR_W, height: barH, color, installs, model, monthLabel }`

**Value labels (над барами):**
`<text x={bar.x + BAR_W / 2} y={bar.y - 3} text-anchor="middle" font-size="8" fill={bar.color}>{bar.installs}</text>` — только если bar.installs > 0.

**X-axis labels:**
По центру группы: `x = LEFT_PAD + i * GROUP_W + GROUP_W / 2`

**Tooltip (Svelte $state, absolutely-positioned div):**
Причина выбора: SVG `<title>` даёт tooltip только при длительном hover и его нельзя стилизовать — непредсказуемо в разных браузерах. Вместо этого — маленький позиционированный `<div>` вне SVG, управляемый `$state`.

```
// В <script>:
interface TooltipState {
  visible: boolean;
  x: number;      // px относительно .chart-area (clientX)
  y: number;      // px относительно .chart-area (clientY)
  month: string;
  model: string;
  installs: number;
}
let tooltip = $state<TooltipState>({ visible: false, x: 0, y: 0, month: '', model: '', installs: 0 });
```

На каждом `<rect>`:
- `onmouseenter={(e) => { tooltip = { visible: true, x: e.offsetX, y: e.offsetY, month: bar.monthLabel, model: bar.model, installs: bar.installs }; }}`
- `onmouseleave={() => { tooltip.visible = false; }}`

Tooltip div — позиция `left: {tooltip.x + 10}px; top: {tooltip.y - 28}px` относительно `.chart-area` (position: relative). Содержимое: `{tooltip.month} · {tooltip.model}: {tooltip.installs}`.

DashboardPage.svelte менять не нужно — tooltip полностью внутри ChartWidget.

**СОХРАНИТЬ без изменений:**
- Блоки `{#if loading}`, `{#if error}`, `{#if data.length === 0}`
- `<table class="sr-only">` — оставить идентичным текущему
- `<ul class="chart-legend">` — оставить идентичным текущему
- `<PeriodToggle>` в заголовке
- Все существующие CSS-классы `.chart-widget`, `.chart-header`, `.widget-title`, `.chart-svg`, `.chart-state`, `.chart-error`, `.chart-empty`, `.sr-only`, `.chart-legend`, `.legend-item`, `.legend-dot`

**Добавить CSS:**
```scss
.chart-area {
  position: relative;
}
.chart-tooltip {
  position: absolute;
  pointer-events: none;
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  padding: 4px 8px;
  font-size: var(--font-size-label);
  color: var(--color-text-primary);
  white-space: nowrap;
  box-shadow: 0 2px 6px rgba(0,0,0,0.15);
  z-index: 10;
}
```

Обернуть `<svg>` в `<div class="chart-area">` и внутри него же разместить tooltip div: `{#if tooltip.visible}<div class="chart-tooltip" ...>{/if}`.
  </action>
  <verify>
    <automated>cd /Users/madsas/Projects/trackly && pnpm --dir ui exec svelte-check --tsconfig ui/tsconfig.json 2>&1 | tail -5</automated>
    <automated>cd /Users/madsas/Projects/trackly && pnpm --dir ui build 2>&1 | tail -8</automated>
    <human-check>Открыть дашборд (десктоп или LAN-браузер), убедиться: столбцы видны, Y-ось с числами, при наведении — tooltip с названием модели и числом.</human-check>
  </verify>
  <done>svelte-check 0 новых ошибок; pnpm --dir ui build OK (ui/dist обновлён, но НЕ git-add); grouped bars отрисованы, Y-ось с тиками видна, value labels над барами, tooltip появляется при hover.</done>
</task>

<task type="checkpoint:human-verify" gate="blocking">
  <what-built>Grouped bar chart в ChartWidget.svelte: Y-ось с числовыми тиками и сеткой, сгруппированные столбцы по моделям, value labels над каждым столбцом, hover-tooltip (месяц + модель + installs). Сборка ui/dist обновлена.</what-built>
  <how-to-verify>
    1. Запустить десктоп-приложение (cargo tauri dev) или открыть LAN-браузер после `pnpm --dir ui build`.
    2. Перейти на дашборд → виджет «Динамика расхода картриджей».
    3. Проверить: столбцы видны и сгруппированы по месяцам; слева — числовая Y-ось; над столбцами — цифры; при наведении курсора на любой столбец — появляется tooltip с текстом «Месяц · Модель: N».
    4. Переключить период (3/6/12 мес.) — столбцы перестраиваются.
    5. Убедиться: ui/dist НЕ добавлен в git (`git status` не должен показывать ui/dist).
  </how-to-verify>
  <resume-signal>Напечатать "approved" если всё работает, или описать проблему для исправления.</resume-signal>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| API → ChartWidget props | ConsumptionPoint.installs — числовое значение от бэкенда. Уже проходит через типизацию TS; SVG генерируется в браузере. |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-vtf-01 | Tampering | SVG coordinate math (niceMax = 0 edge case) | mitigate | niceMax = Math.max(niceMax, 1) — деление на ноль невозможно |
| T-vtf-02 | Denial of Service | 100+ моделей → modelKeys.slice(0, 3) | accept | Уже ограничено 3 моделями в текущем коде; bar overflow не увеличивает поверхность атаки |
</threat_model>

<verification>
- `pnpm --dir ui exec svelte-check` → 0 новых ошибок (pre-existing warnings в других компонентах допустимы)
- `pnpm --dir ui build` → exit 0, ui/dist обновлён
- `git status` → ui/dist НЕ отслеживается (он в .gitignore)
- Визуальная проверка пользователем (десктоп / LAN-браузер)
</verification>

<success_criteria>
- Grouped bar chart корректно рендерится для 1, 3, 6, 12 месяцев и 1–3 моделей
- Y-ось: 5 тиков с числами и горизонтальными сетчатыми линиями
- Value labels над каждым ненулевым столбцом
- Hover-tooltip: «Месяц · Модель: N»
- Все preserved-элементы на месте (loading/error/empty states, sr-only table, legend, PeriodToggle)
- svelte-check 0 новых ошибок; build проходит
- ui/dist не попадает в git
</success_criteria>

<output>
Create `.planning/quick/260702-vtf-y-tooltip/260702-vtf-01-SUMMARY.md` when done.
</output>

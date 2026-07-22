<script lang="ts">
  // Plan 07-06 Task 1: Period selector — Месяц / Год / Диапазон modes.
  // Snapshot reports disable controls with helper text (T-07-06-03 date range validation).
  // Plan 28-03 Task 2 (D-06): mode switch on Tabs segmented, month/year on Select primitive.
  // Plan 28-13 (GAP-1): Select (нативный <select>) заменён на кастомный
  // Dropdown (flat + variant="select") — также фиксит регрессию, при которой
  // выбранное значение не отображалось (Select's internal bind:value на
  // $bindable prop без двустороннего связывания родителя desync'ится от
  // реактивных апдейтов; Dropdown's явная одностороння controlled-value
  // конвенция этой проблемы не имеет).
  import DatePicker from '$lib/components/DatePicker.svelte';
  import Tabs from '$lib/components/Tabs.svelte';
  import Dropdown from '$lib/components/Dropdown.svelte';

  type PeriodMode = 'month' | 'year' | 'range';

  interface PeriodDto {
    mode: string;
    year?: number | null;
    month?: number | null;
    date_from?: string | null;
    date_to?: string | null;
  }

  interface Props {
    period: PeriodDto;
    isSnapshot: boolean;
    onPeriodChange: (_p: PeriodDto) => void;
  }

  const { period, isSnapshot, onPeriodChange }: Props = $props();

  let mode = $state<PeriodMode>((period.mode as PeriodMode) ?? 'month');
  let selectedMonth = $state<number>(period.month ?? new Date().getMonth() + 1);
  let selectedYear = $state<number>(period.year ?? new Date().getFullYear());
  let dateFrom = $state<string>(period.date_from ?? '');
  let dateTo = $state<string>(period.date_to ?? '');
  let rangeError = $state<string | null>(null);

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

  const currentYear = new Date().getFullYear();
  const years = Array.from({ length: 4 }, (_, i) => currentYear - 3 + i);

  // Plan 28-13 (GAP-1): опции для Dropdown (flat + variant="select").
  const monthOptions = MONTHS.map((name, i) => ({ id: i + 1, label: name }));
  const yearOptions = years.map((y) => ({ id: y, label: String(y) }));
  const selectedMonthLabel = $derived(
    monthOptions.find((o) => o.id === selectedMonth)?.label ?? '',
  );
  const selectedYearLabel = $derived(yearOptions.find((o) => o.id === selectedYear)?.label ?? '');

  // Плоские опции без drill-in — onExpandGroup никогда реально не вызывается
  // (isGroupExpandable всегда false), но Dropdown требует типизированную
  // функцию для вывода TMember (иначе `() => []` выводит `never[]`).
  function noExpandMonth(): { id: number; label: string }[] {
    return [];
  }
  function noExpandYear(): { id: number; label: string }[] {
    return [];
  }

  function setMode(m: PeriodMode) {
    mode = m;
    rangeError = null;
    if (m === 'month') {
      onPeriodChange({ mode: 'month', year: selectedYear, month: selectedMonth });
    } else if (m === 'year') {
      onPeriodChange({ mode: 'year', year: selectedYear });
    } else {
      if (dateFrom && dateTo) {
        onPeriodChange({ mode: 'range', date_from: dateFrom, date_to: dateTo });
      }
    }
  }

  // Plan 28-03: Select's onchange hands back a string value (native <select>
  // removed) — same period-recalculation logic, just adapted to a string input.
  function onMonthChange(v: string) {
    selectedMonth = Number(v);
    onPeriodChange({ mode: 'month', year: selectedYear, month: selectedMonth });
  }

  function onYearChange(v: string) {
    selectedYear = Number(v);
    if (mode === 'month') {
      onPeriodChange({ mode: 'month', year: selectedYear, month: selectedMonth });
    } else {
      onPeriodChange({ mode: 'year', year: selectedYear });
    }
  }

  // T-07-06-03: Watch range fields via $effect — fires whenever dateFrom or dateTo changes
  $effect(() => {
    if (mode !== 'range') return;
    rangeError = null;
    if (!dateFrom || !dateTo) return;
    if (dateFrom > dateTo) {
      rangeError = 'Начало не может быть позже окончания';
      return;
    }
    onPeriodChange({ mode: 'range', date_from: dateFrom, date_to: dateTo });
  });

  const MODES: { key: PeriodMode; label: string }[] = [
    { key: 'month', label: 'Месяц' },
    { key: 'year', label: 'Год' },
    { key: 'range', label: 'Диапазон' },
  ];
</script>

<div class="period-selector" role="group" aria-label="Выбор периода">
  <Tabs
    variant="segmented"
    tabs={MODES.map((m) => ({ key: m.key, label: m.label, disabled: isSnapshot }))}
    active={mode}
    ariaLabel="Режим периода"
    onchange={(key) => setMode(key as PeriodMode)}
  />

  {#if isSnapshot}
    <p class="snapshot-hint">Отчёт отражает текущее состояние</p>
  {:else if mode === 'month'}
    <div class="period-controls">
      <Dropdown
        variant="select"
        flat={true}
        value={selectedMonthLabel}
        placeholder="Месяц"
        searchPlaceholder="Поиск"
        searchable={false}
        loading={false}
        groups={monthOptions}
        getGroupId={(o) => o.id}
        getGroupName={(o) => o.label}
        getGroupCount={() => 0}
        isGroupExpandable={() => false}
        isGroupSelected={(o) => o.id === selectedMonth}
        onExpandGroup={noExpandMonth}
        getMemberId={(o) => o.id}
        getMemberName={(o) => o.label}
        onSearch={() => {}}
        onPickGroup={(o) => onMonthChange(String(o.id))}
        onPickMember={() => {}}
      />
      <Dropdown
        variant="select"
        flat={true}
        value={selectedYearLabel}
        placeholder="Год"
        searchPlaceholder="Поиск"
        searchable={false}
        loading={false}
        groups={yearOptions}
        getGroupId={(o) => o.id}
        getGroupName={(o) => o.label}
        getGroupCount={() => 0}
        isGroupExpandable={() => false}
        isGroupSelected={(o) => o.id === selectedYear}
        onExpandGroup={noExpandYear}
        getMemberId={(o) => o.id}
        getMemberName={(o) => o.label}
        onSearch={() => {}}
        onPickGroup={(o) => onYearChange(String(o.id))}
        onPickMember={() => {}}
      />
    </div>
  {:else if mode === 'year'}
    <div class="period-controls">
      <Dropdown
        variant="select"
        flat={true}
        value={selectedYearLabel}
        placeholder="Год"
        searchPlaceholder="Поиск"
        searchable={false}
        loading={false}
        groups={yearOptions}
        getGroupId={(o) => o.id}
        getGroupName={(o) => o.label}
        getGroupCount={() => 0}
        isGroupExpandable={() => false}
        isGroupSelected={(o) => o.id === selectedYear}
        onExpandGroup={noExpandYear}
        getMemberId={(o) => o.id}
        getMemberName={(o) => o.label}
        onSearch={() => {}}
        onPickGroup={(o) => onYearChange(String(o.id))}
        onPickMember={() => {}}
      />
    </div>
  {:else if mode === 'range'}
    <div class="period-controls period-range">
      <label class="range-label">
        <span class="range-text">С</span>
        <DatePicker bind:value={dateFrom} invalid={rangeError !== null} max={dateTo || undefined} />
      </label>
      <label class="range-label">
        <span class="range-text">По</span>
        <DatePicker bind:value={dateTo} invalid={rangeError !== null} min={dateFrom || undefined} />
      </label>
      <!-- Range validation fires via $effect watching dateFrom / dateTo -->
      {#if rangeError}
        <p class="range-error" role="alert">{rangeError}</p>
      {/if}
    </div>
  {/if}
</div>

<style lang="scss">
  .period-selector {
    display: flex;
    // UAT (Отчёты): controls were "pulled to the top" vs the segmented mode
    // switcher — center-align so the 28px Tabs, month/year Dropdowns, and the
    // export buttons in .controls-row all sit on one visual line. Drop the
    // vertical padding so the block height is exactly the 28px control height
    // (no 4px halo that offset it against the right-side export buttons).
    align-items: center;
    gap: var(--tr-space-xs);
    flex-wrap: wrap;
  }

  .snapshot-hint {
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-tertiary);
    margin: 0;
    align-self: center;
  }

  .period-controls {
    display: flex;
    align-items: center;
    gap: var(--tr-space-2xs);
    flex-wrap: wrap;

    // Plan 28-03: Select defaults to full-width/36px form-field sizing —
    // constrain to content width + the 28px filter-row height (GAP-R3
    // precedent below, same treatment as DatePicker in .range-label).
    // Plan 28-13 (GAP-1): re-targeted at Dropdown's select-variant classes —
    // Dropdown's trigger is a `<button class="tr-dropdown-field-button">`,
    // not an `<input>`, so Select's old `.select-wrapper`/`.select` selectors
    // would silently do nothing here.
    :global(.tr-dropdown) {
      width: auto;
      min-width: 110px;
    }

    :global(.tr-dropdown-field-button) {
      height: 28px;
      font-size: var(--tr-font-size-label);
    }
  }

  // GAP-R3: date inputs in range mode must be same height as other filter controls (28px)
  // GAP-3 partial (28-13): breathing room between the two С/По label+DatePicker
  // groups — previously inherited the tight .period-controls gap and read as
  // cramped/misaligned against the rest of the filter row.
  .period-range {
    align-items: center;
    flex-wrap: wrap;
    gap: var(--tr-space-md);
  }

  .range-label {
    display: flex;
    align-items: center;
    gap: var(--tr-space-2xs);

    // GAP-R3: constrain DatePicker to match other filter-row controls (28px height)
    :global(.date-picker) {
      height: 28px;
      font-size: var(--tr-font-size-label);
      width: auto;
      min-width: 130px;
    }
  }

  .range-text {
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-secondary);
    white-space: nowrap;
  }

  .range-error {
    font-size: var(--tr-font-size-label);
    color: var(--tr-danger);
    margin: 0;
    width: 100%;
    margin-top: var(--tr-space-2xs);
  }
</style>

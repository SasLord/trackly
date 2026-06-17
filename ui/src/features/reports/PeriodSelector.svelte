<script lang="ts">
  // Plan 07-06 Task 1: Period selector — Месяц / Год / Диапазон modes.
  // Snapshot reports disable controls with helper text (T-07-06-03 date range validation).
  import DatePicker from '$lib/components/DatePicker.svelte';

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

  function onMonthChange(e: Event) {
    selectedMonth = Number((e.currentTarget as HTMLSelectElement).value);
    onPeriodChange({ mode: 'month', year: selectedYear, month: selectedMonth });
  }

  function onYearChange(e: Event) {
    selectedYear = Number((e.currentTarget as HTMLSelectElement).value);
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
  <div class="period-buttons">
    {#each MODES as m}
      <button
        type="button"
        class="period-btn"
        class:active={mode === m.key}
        disabled={isSnapshot}
        aria-disabled={isSnapshot ? 'true' : undefined}
        onclick={() => !isSnapshot && setMode(m.key)}
      >
        {m.label}
      </button>
    {/each}
  </div>

  {#if isSnapshot}
    <p class="snapshot-hint">Отчёт отражает текущее состояние</p>
  {:else if mode === 'month'}
    <div class="period-controls">
      <select class="period-select" value={selectedMonth} onchange={onMonthChange}>
        {#each MONTHS as name, i}
          <option value={i + 1}>{name}</option>
        {/each}
      </select>
      <select class="period-select" value={selectedYear} onchange={onYearChange}>
        {#each years as y}
          <option value={y}>{y}</option>
        {/each}
      </select>
    </div>
  {:else if mode === 'year'}
    <div class="period-controls">
      <select class="period-select" value={selectedYear} onchange={onYearChange}>
        {#each years as y}
          <option value={y}>{y}</option>
        {/each}
      </select>
    </div>
  {:else if mode === 'range'}
    <div class="period-controls period-range">
      <label class="range-label">
        <span class="range-text">С</span>
        <DatePicker
          bind:value={dateFrom}
          invalid={rangeError !== null}
          max={dateTo || undefined}
        />
      </label>
      <label class="range-label">
        <span class="range-text">По</span>
        <DatePicker
          bind:value={dateTo}
          invalid={rangeError !== null}
          min={dateFrom || undefined}
        />
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
    align-items: flex-start;
    gap: var(--space-sm);
    flex-wrap: wrap;
    padding: var(--space-xs) 0;
  }

  .period-buttons {
    display: flex;
    gap: 2px;
  }

  .period-btn {
    padding: var(--space-xs) var(--space-sm);
    background: transparent;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    font-family: var(--font-family-base);
    font-size: var(--font-size-label);
    color: var(--color-text-secondary);
    cursor: pointer;
    height: 28px;

    &:first-child {
      border-radius: var(--radius-sm) 0 0 var(--radius-sm);
    }

    &:last-child {
      border-radius: 0 var(--radius-sm) var(--radius-sm) 0;
    }

    &:not(:first-child) {
      margin-left: -1px;
    }

    &:hover:not(:disabled) {
      background: var(--color-surface-sunken);
      color: var(--color-text-primary);
    }

    &:focus-visible {
      outline: none;
      box-shadow: 0 0 0 3px var(--color-accent-focus);
      z-index: 1;
      position: relative;
    }

    &.active {
      background: color-mix(in srgb, var(--color-accent) 10%, transparent);
      border-color: var(--color-accent);
      color: var(--color-accent);
      z-index: 1;
      position: relative;
    }

    &:disabled {
      opacity: 0.5;
      cursor: not-allowed;
    }
  }

  .snapshot-hint {
    font-size: var(--font-size-label);
    color: var(--color-text-muted);
    margin: 0;
    align-self: center;
  }

  .period-controls {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    flex-wrap: wrap;
  }

  // GAP-R3: date inputs in range mode must be same height as other filter controls (28px)
  .period-range {
    align-items: center;
    flex-wrap: wrap;
  }

  .period-select {
    height: 28px;
    padding: 0 var(--space-sm);
    background: var(--color-bg);
    color: var(--color-text-primary);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    font-family: var(--font-family-base);
    font-size: var(--font-size-label);
    cursor: pointer;

    &:focus-visible {
      outline: none;
      border-color: var(--color-accent);
      box-shadow: 0 0 0 3px var(--color-accent-focus);
    }
  }

  .range-label {
    display: flex;
    align-items: center;
    gap: var(--space-xs);

    // GAP-R3: constrain DatePicker to match other filter-row controls (28px height)
    :global(.date-picker) {
      height: 28px;
      font-size: var(--font-size-label);
      width: auto;
      min-width: 130px;
    }
  }

  .range-text {
    font-size: var(--font-size-label);
    color: var(--color-text-secondary);
    white-space: nowrap;
  }

  .range-error {
    font-size: var(--font-size-label);
    color: var(--color-destructive);
    margin: 0;
    width: 100%;
    margin-top: var(--space-xs);
  }
</style>

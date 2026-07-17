<script lang="ts">
  // Plan 07-06 Task 1: Universal report table with month-separator rows.
  // Temporal reports: separators by month_key. Snapshot: separators by location_name.
  // Loading / empty / error states follow ActsList.svelte pattern.
  import Spinner from '$lib/components/Spinner.svelte';

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

  interface Column {
    key: string;
    label: string;
  }

  type SeparatorItem = { type: 'separator'; label: string };
  type RowItem = ReportRow & { type?: never };
  type GroupedItem = SeparatorItem | RowItem;

  interface Props {
    rows: ReportRow[];
    columns: Column[];
    loading: boolean;
    error: string | null;
    reportType: string;
    isSnapshot: boolean;
  }

  const { rows, columns, loading, error, isSnapshot }: Props = $props();

  const MONTH_NAMES = [
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

  function formatMonthKey(key: string): string {
    const [year, month] = key.split('-');
    return MONTH_NAMES[parseInt(month) - 1] + ' ' + year;
  }

  function formatCellValue(row: ReportRow, colKey: string): string {
    const val = row[colKey];
    if (val === null || val === undefined) return '—';
    // Format UTC timestamps as date string
    if (colKey === 'handover_date_utc' && typeof val === 'number') {
      const d = new Date(val * 1000);
      return d.toLocaleDateString('ru-RU');
    }
    return String(val);
  }

  // Build grouped rows: insert separator when month_key (temporal) or location_name (snapshot) changes
  const grouped = $derived.by((): GroupedItem[] => {
    const result: GroupedItem[] = [];
    let lastSeparatorKey = '';

    for (const row of rows) {
      const separatorKey = isSnapshot ? (row.location_name ?? '') : (row.month_key ?? '');

      if (separatorKey !== lastSeparatorKey && separatorKey !== '') {
        const label = isSnapshot
          ? separatorKey
          : row.month_key
            ? formatMonthKey(row.month_key)
            : separatorKey;
        result.push({ type: 'separator', label });
        lastSeparatorKey = separatorKey;
      }

      result.push(row);
    }

    return result;
  });
</script>

<div class="report-table-wrap">
  {#if loading}
    <div class="state state-loading">
      <Spinner size="md" />
    </div>
  {:else if error}
    <div class="state state-error">
      <p class="error-text">Не удалось загрузить отчёт. Попробуйте ещё раз.</p>
    </div>
  {:else if rows.length === 0}
    <div class="state state-empty">
      <p class="empty-heading">Нет данных за выбранный период</p>
      <p class="empty-body">Измените диапазон дат или выберите другой тип отчёта.</p>
    </div>
  {:else}
    <div class="table-scroll">
      <table class="report-table">
        <thead>
          <tr>
            {#each columns as col}
              <th scope="col">{col.label}</th>
            {/each}
          </tr>
        </thead>
        <tbody>
          {#each grouped as item}
            {#if 'type' in item && item.type === 'separator'}
              <tr class="month-separator" aria-hidden="true">
                <td colspan={columns.length}>{item.label}</td>
              </tr>
            {:else}
              {@const row = item as ReportRow}
              <tr>
                {#each columns as col}
                  {@const cellVal = formatCellValue(row, col.key)}
                  <td title={cellVal}>{cellVal}</td>
                {/each}
              </tr>
            {/if}
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>

<style lang="scss">
  .report-table-wrap {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    min-height: 200px;
  }

  .state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: var(--tr-space-2xl);
    flex: 1;
    text-align: center;
  }

  .state-loading {
    gap: var(--tr-space-xs);
  }

  .error-text {
    color: var(--tr-danger);
    font-size: var(--font-size-body);
    margin: 0;
  }

  .empty-heading {
    font-size: var(--font-size-heading);
    font-weight: var(--font-weight-semibold);
    color: var(--tr-text-primary);
    margin: 0 0 var(--tr-space-2xs);
  }

  .empty-body {
    font-size: var(--font-size-body);
    color: var(--tr-text-tertiary);
    margin: 0;
  }

  .table-scroll {
    flex: 1;
    overflow: auto;
  }

  .report-table {
    width: 100%;
    border-collapse: collapse;
    table-layout: auto;

    thead {
      position: sticky;
      top: 0;
      z-index: 1;
      background: var(--tr-bg);
    }

    th {
      padding: 0 var(--tr-space-md);
      height: var(--row-height);
      text-align: left;
      font-size: var(--font-size-label);
      font-weight: var(--font-weight-medium);
      color: var(--tr-text-secondary);
      border-bottom: 1px solid var(--tr-border);
      white-space: nowrap;
    }

    td {
      padding: 0 var(--tr-space-md);
      height: var(--row-height);
      font-size: var(--font-size-body);
      color: var(--tr-text-primary);
      border-bottom: 1px solid var(--tr-border);
      max-width: 240px;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    tbody tr:hover {
      background: var(--tr-surface);
    }
  }

  .month-separator td {
    padding: var(--tr-space-2xs) var(--tr-space-md);
    height: var(--row-height-dense);
    background: var(--tr-surface-sunken);
    font-size: var(--font-size-body);
    font-weight: var(--font-weight-semibold);
    border-top: 1px solid var(--tr-border-strong);
    color: var(--tr-text-primary);
  }
</style>

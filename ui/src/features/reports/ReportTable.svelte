<script lang="ts">
  // Plan 07-06 Task 1: Universal report table with month-separator rows.
  // Temporal reports: separators by month_key. Snapshot: separators by location_name.
  // Plan 28-04 (D-07): rebuilt on shared Table/TableRow primitives — dynamic
  // Column[] rendered via head/children snippets; the month/location separator
  // stays a bare <tr> (NOT TableRow's group-collapse mode — that mode is a
  // collapse contract with groupExpanded/onToggleGroup, the separator here is static).
  // Loading/empty now come from Table's built-in states; error keeps its own
  // sibling branch outside Table (no error-state equivalent in Table's API).
  import Table from '$lib/components/Table.svelte';
  import TableRow from '$lib/components/TableRow.svelte';

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

{#snippet tableHead()}
  {#each columns as col}
    <th>{col.label}</th>
  {/each}
{/snippet}

<div class="report-table-wrap">
  {#if error}
    <div class="state state-error">
      <p class="error-text">Не удалось загрузить отчёт. Попробуйте ещё раз.</p>
    </div>
  {:else}
    <Table
      columns={columns.length}
      {loading}
      empty={rows.length === 0 && !loading}
      emptyTitle="Нет данных за выбранный период"
      emptyBody="Измените диапазон дат или выберите другой тип отчёта."
      head={tableHead}
      fillHeight
    >
      {#each grouped as item}
        {#if 'type' in item && item.type === 'separator'}
          <tr class="report-separator" aria-hidden="true">
            <td colspan={columns.length}>{item.label}</td>
          </tr>
        {:else}
          {@const row = item as ReportRow}
          <TableRow>
            {#each columns as col}
              {@const cellVal = formatCellValue(row, col.key)}
              <td title={cellVal}>{cellVal}</td>
            {/each}
          </TableRow>
        {/if}
      {/each}
    </Table>
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

  .error-text {
    color: var(--tr-danger);
    font-size: var(--tr-font-size-body);
    margin: 0;
  }

  .report-separator td {
    padding: var(--tr-space-2xs) var(--tr-space-md);
    height: var(--row-height-dense);
    background: var(--tr-surface-sunken);
    font-size: var(--tr-font-size-body);
    font-weight: var(--tr-font-weight-semibold);
    border-top: 1px solid var(--tr-border-strong);
    color: var(--tr-text-primary);
  }
</style>

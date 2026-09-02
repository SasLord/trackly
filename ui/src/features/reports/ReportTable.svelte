<script lang="ts">
  // Plan 07-06 Task 1: Universal report table with month-separator rows.
  // Temporal reports: separators by month_key. Snapshot: separators by place_path.
  // Plan 28-04 (D-07): rebuilt on shared Table/TableRow primitives — dynamic
  // Column[] rendered via head/children snippets; the month/location separator
  // stays a bare <tr> (NOT TableRow's group-collapse mode — that mode is a
  // collapse contract with groupExpanded/onToggleGroup, the separator here is static).
  // Loading/empty now come from Table's built-in states; error keeps its own
  // sibling branch outside Table (no error-state equivalent in Table's API).
  import Table from '$lib/components/Table.svelte';
  import TableRow from '$lib/components/TableRow.svelte';
  import Badge from '$lib/components/Badge.svelte';

  interface ReportRow {
    id: number;
    month_key?: string | null;
    number?: string | null;
    sub_number?: string | null;
    giver_name?: string | null;
    receiver_name?: string | null;
    handover_date_utc?: number | null;
    place_path?: string | null;
    place_path_short?: string | null;
    act_type?: string | null;
    device_name?: string | null;
    quantity?: number | null;
    code?: string | null;
    model_label?: string | null;
    status_name?: string | null;
    // HST-04 (D-23/D-25) — movements report row fields (Plan 40-11/40-12).
    from_place_path?: string | null;
    from_place_path_short?: string | null;
    actor_name?: string | null;
    reason?: string | null;
    entity_type_label?: string | null;
    is_deleted?: boolean | null;
    [key: string]: unknown;
  }

  interface Column {
    key: string;
    label: string;
    // Name of a sibling ReportRow field whose value must be PREPENDED
    // (joined with ", ") to the place_path cell's value — and must NEVER
    // participate in the D-26 shortening. Used by the requests report's
    // composite «Место» column (printer name + place path).
    compositeWith?: string;
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

  const { rows, columns, loading, error, reportType, isSnapshot }: Props = $props();

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
    // WSU-01/WSU-02: "дд.мм.гг, чч:мм" — same readable format the backend
    // now emits for CSV/HTML export (report_service.rs::format_handover_date),
    // so the screen table no longer disagrees with either export path. Local
    // (not UTC) Date getters are used deliberately — same W-9 single-tz
    // principle documented in DocumentAcceptanceModal.svelte: the browser's
    // local clock is treated as the organization's timezone, no separate
    // test/AD environment on a different timezone is assumed. Manual
    // padStart (not toLocaleString/Intl) keeps the format deterministic
    // regardless of browser locale.
    if (colKey === 'handover_date_utc' && typeof val === 'number') {
      const d = new Date(val * 1000);
      const day = String(d.getDate()).padStart(2, '0');
      const month = String(d.getMonth() + 1).padStart(2, '0');
      const year = String(d.getFullYear() % 100).padStart(2, '0');
      const hours = String(d.getHours()).padStart(2, '0');
      const minutes = String(d.getMinutes()).padStart(2, '0');
      return `${day}.${month}.${year}, ${hours}:${minutes}`;
    }
    return String(val);
  }

  // Phase 39.1: place_path cells arrive already shortened from the backend
  // (row.place_path_short, per the organization/place display-variant setting —
  // PLC-07/PLC-08) — no local shortening formula on the frontend anymore. The
  // full path always goes in the cell's title attribute regardless of variant,
  // so the complete location is one hover away.

  // Composite place cell: combines an optional sibling field (col.compositeWith,
  // e.g. device_name for the requests report's printer column) with place_path
  // via ", ". transformPath is applied ONLY to the path part — the prefix
  // (printer name) never gets D-26-shortened or otherwise mangled, because it
  // is read as a separate ReportRow field, never parsed out of a joined string.
  function formatPlaceCell(
    row: ReportRow,
    col: Column,
    transformPath: (path: string) => string,
  ): string {
    const rawPath = typeof row.place_path === 'string' ? row.place_path : '';
    const path = rawPath ? transformPath(rawPath) : '';
    const prefix =
      col.compositeWith && typeof row[col.compositeWith] === 'string'
        ? (row[col.compositeWith] as string)
        : '';

    if (prefix && path) return `${prefix}, ${path}`;
    if (prefix) return prefix;
    if (path) return path;
    return formatCellValue(row, col.key);
  }

  // Same place_path/place_path_short + title= convention, cloned for the
  // movements report's genuinely separate "Откуда" column (D-23) — never
  // shortened by D-26's compositeWith prefix logic, which is place_path-only.
  function formatFromPlaceCell(row: ReportRow, full: boolean): string {
    const raw = typeof row.from_place_path === 'string' ? row.from_place_path : '';
    if (!raw) return '—';
    return full ? raw : (row.from_place_path_short ?? raw);
  }

  // D-26: the cell's rendered text and its title attribute diverge only for
  // place_path/from_place_path — every other column keeps formatCellValue's
  // plain title=text convention.
  function formatCellTitle(row: ReportRow, col: Column): string {
    if (col.key === 'place_path') {
      return formatPlaceCell(row, col, (p) => p);
    }
    if (col.key === 'from_place_path') {
      return formatFromPlaceCell(row, true);
    }
    return formatCellValue(row, col.key);
  }

  function formatCellDisplay(row: ReportRow, col: Column): string {
    if (col.key === 'place_path') {
      return formatPlaceCell(row, col, () => row.place_path_short ?? '');
    }
    if (col.key === 'from_place_path') {
      return formatFromPlaceCell(row, false);
    }
    return formatCellValue(row, col.key);
  }

  // D-25: soft-deleted report rows show an «Удалено» badge next to the
  // «Предмет» cell (device_name) rather than silently vanishing — only ever
  // true for the movements report, since is_deleted is null/undefined on
  // every other report row shape.
  function showDeletedBadge(row: ReportRow, col: Column): boolean {
    return reportType === 'movements' && col.key === 'device_name' && row.is_deleted === true;
  }

  // Build grouped rows: insert separator when month_key (temporal) or place_path (snapshot) changes
  const grouped = $derived.by((): GroupedItem[] => {
    const result: GroupedItem[] = [];
    let lastSeparatorKey = '';

    for (const row of rows) {
      const separatorKey = isSnapshot ? (row.place_path ?? '') : (row.month_key ?? '');

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
              <td title={formatCellTitle(row, col)}>
                {formatCellDisplay(row, col)}
                {#if showDeletedBadge(row, col)}
                  <Badge variant="default">Удалено</Badge>
                {/if}
              </td>
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

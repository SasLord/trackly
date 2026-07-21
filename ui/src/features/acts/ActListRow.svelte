<script lang="ts">
  // Plan 03-02: list row for ActsList master panel.
  // Plan 27-02 (D-03): rebuilt on shared TableRow primitive per
  // DeviceListRow.svelte precedent — bespoke two-line `.row` div replaced
  // with a 4-column <TableRow> (№/Дата/Получатель/Позиций); select state
  // now via TableRow's `selected` prop, not bespoke `.row.selected`.
  // NOTE: TableRow.svelte does not forward arbitrary attrs (onclick/role/
  // tabindex) to its own <tr> — it only accepts the documented props. Row
  // click/keyboard-select is therefore wired on the <td> cells we own here
  // (onclick on every cell for full-row mouse click; role="button"+tabindex+
  // onkeydown on the first cell as the single keyboard entry point, mirroring
  // the previous single-div tab-stop).
  import Badge from '$lib/components/Badge.svelte';
  import TableRow from '$lib/components/TableRow.svelte';
  import type { ActDto } from '../../bindings';

  interface Props {
    act: ActDto;
    selected: boolean;
    showArchivedBadge?: boolean;
    onSelect: (_id: number) => void;
  }

  const { act, selected, showArchivedBadge = false, onSelect }: Props = $props();

  // Format unix seconds → «28 мая 2026»
  const MONTHS_RU = [
    'января',
    'февраля',
    'марта',
    'апреля',
    'мая',
    'июня',
    'июля',
    'августа',
    'сентября',
    'октября',
    'ноября',
    'декабря',
  ];

  function formatDate(utcSeconds: number): string {
    const d = new Date(utcSeconds * 1000);
    return `${d.getUTCDate()} ${MONTHS_RU[d.getUTCMonth()]} ${d.getUTCFullYear()}`;
  }

  const dateLabel = $derived(formatDate(act.handover_date_utc));
  const itemsCount = $derived(act.items.length);
  // act.number is already formatted (D-Numbering-01) — e.g. "42" / "42в1".

  function handleClick() {
    onSelect(act.id);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      onSelect(act.id);
    }
  }
</script>

<TableRow {selected} class="act-row">
  <td
    class="cell cell-number"
    role="button"
    tabindex="0"
    aria-pressed={selected}
    onclick={handleClick}
    onkeydown={handleKeydown}
  >
    <span class="tr-mono">№{act.number}</span>
  </td>
  <td class="cell cell-date" onclick={handleClick}>
    {dateLabel}
    {#if showArchivedBadge}
      <span class="badge-wrap"><Badge variant="default">В архиве</Badge></span>
    {/if}
  </td>
  <td class="cell" title={act.receiver_name} onclick={handleClick}>{act.receiver_name}</td>
  <td class="cell cell-count" onclick={handleClick}>{itemsCount}</td>
</TableRow>

<style lang="scss">
  // TableRow renders its own <tr> (a DIFFERENT Svelte scope-hash than this
  // file) — caller-supplied class needs `:global()`, and the ancestor part of
  // the selector must stay in THIS file's scope per the TableRow contract:
  // `.act-row :global(> td)`, never `:global(.act-row > td)` (specificity trap).
  :global(tr.act-row) {
    cursor: pointer;
  }

  .cell {
    font-size: var(--tr-font-size-body);
    color: var(--tr-text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 0; // makes text-overflow work in table cells
  }

  .cell-number {
    width: 72px;
    cursor: pointer;

    &:focus-visible {
      outline: none;
      box-shadow: inset 0 0 0 2px var(--tr-accent);
    }
  }

  .cell-date {
    color: var(--tr-text-secondary);
  }

  .cell-count {
    width: 90px;
    text-align: right;
    font-variant-numeric: tabular-nums;
    color: var(--tr-text-secondary);
  }

  .badge-wrap {
    margin-left: var(--tr-space-2xs);
  }
</style>

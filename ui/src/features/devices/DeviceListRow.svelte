<script lang="ts">
  import Badge from '$lib/components/Badge.svelte';
  import TableRow from '$lib/components/TableRow.svelte';
  import DeviceContextMenu from './DeviceContextMenu.svelte';
  import type { DeviceDto } from '../../bindings';

  interface Props {
    device: DeviceDto;
    onEdit: (_d: DeviceDto) => void;
    onDelete: () => void;
    /** When true, renders a stronger bottom border to visually close the group. */
    isLastInGroup?: boolean;
    /** Plan 03-05 (DEV-14): pass-through к DeviceContextMenu. */
    onPrintAcceptance?: (_d: DeviceDto) => void;
    /** ITEM-3: when true, shows the «Статус» column. Hide on filtered status tabs. */
    showStatus?: boolean;
  }

  const {
    device,
    onEdit,
    onDelete,
    isLastInGroup = false,
    onPrintAcceptance,
    showStatus = true,
  }: Props = $props();

  // ---------------------------------------------------------------------------
  // Placeholder status mapping (Plan 04 wires seeded lookups)
  // ---------------------------------------------------------------------------
  type BadgeVariant = 'default' | 'accent' | 'success' | 'warning' | 'destructive';

  const STATUS_LABELS: Record<number, string> = {
    1: 'На складе',
    2: 'В работе',
    3: 'На ремонте',
    4: 'Списано',
  };

  const STATUS_VARIANTS: Record<number, BadgeVariant> = {
    1: 'default',
    2: 'accent',
    3: 'warning',
    4: 'destructive',
  };

  const statusLabel = $derived(STATUS_LABELS[device.status_id] ?? `Статус ${device.status_id}`);
  const statusVariant = $derived(STATUS_VARIANTS[device.status_id] ?? 'default');
</script>

<TableRow class={isLastInGroup ? 'group-last-child' : undefined}>
  <td class="cell cell-name" title={device.name}>{device.name}</td>
  <td class="cell cell-numeric" title={device.inventory_no ?? ''}
    ><span class="tr-mono">{device.inventory_no ?? '—'}</span></td
  >
  <td class="cell cell-numeric" title={device.serial_no ?? ''}
    ><span class="tr-mono">{device.serial_no ?? '—'}</span></td
  >
  <td class="cell" title={device.model ?? ''}>{device.model ?? '—'}</td>
  <td class="cell" title={device.full_path ?? ''}>{device.full_path ?? '—'}</td>
  <td class="cell" title={device.state ?? ''}>{device.state ?? '—'}</td>
  {#if showStatus}
    <td class="cell cell-status">
      <Badge variant={statusVariant}>{statusLabel}</Badge>
    </td>
  {/if}
  <td class="cell cell-actions">
    <DeviceContextMenu {device} {onEdit} {onDelete} {onPrintAcceptance} />
  </td>
</TableRow>

<style lang="scss">
  // Visual group-end divider: last child in an expanded group gets a stronger
  // bottom border so the eye clearly sees where the group ends. `group-last-child`
  // is passed through to TableRow's rendered <tr> (a DIFFERENT Svelte scope-hash
  // than this file), so the ancestor part of the selector needs :global(); `.cell`
  // stays local (unwrapped) so it keeps this file's scope hash — combined class
  // count (group-last-child + cell + local-hash = 3) beats TableRow's own base
  // <td> border-bottom rule (.tr-row.hash > td, 2 classes) on specificity.
  :global(tr.group-last-child) > .cell {
    border-bottom: 2px solid var(--tr-border-strong);
  }

  .cell {
    font-size: var(--tr-font-size-body);
    color: var(--tr-text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 0; // makes text-overflow work in table cells
  }

  .cell-name {
    width: 25%;
    max-width: 200px;
  }

  .cell-numeric {
    width: 140px;
    font-variant-numeric: tabular-nums;
  }

  .cell-status {
    width: 120px;
  }

  .cell-actions {
    width: 40px;
    text-align: center;
    overflow: visible;
  }
</style>

<script lang="ts">
  import Badge from '$lib/components/Badge.svelte';
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
  }

  const { device, onEdit, onDelete, isLastInGroup = false, onPrintAcceptance }: Props = $props();

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

<tr class="device-row" class:group-last-child={isLastInGroup}>
  <td class="cell cell-name">{device.name}</td>
  <td class="cell cell-numeric">{device.inventory_no ?? '—'}</td>
  <td class="cell cell-numeric">{device.serial_no ?? '—'}</td>
  <td class="cell">{device.model ?? '—'}</td>
  <td class="cell">{device.location ?? '—'}</td>
  <td class="cell cell-status">
    <Badge variant={statusVariant}>{statusLabel}</Badge>
  </td>
  <td class="cell cell-actions">
    <DeviceContextMenu {device} {onEdit} {onDelete} {onPrintAcceptance} />
  </td>
</tr>

<style lang="scss">
  .device-row {
    height: var(--row-height, 40px);

    &:hover {
      background: var(--color-surface);
    }

    // Visual group-end divider: last child in an expanded group gets a stronger
    // bottom border so the eye clearly sees where the group ends.
    &.group-last-child .cell {
      border-bottom: 2px solid var(--color-border-strong);
    }
  }

  .cell {
    padding: 0 var(--space-sm);
    font-size: var(--font-size-body);
    color: var(--color-text-primary);
    vertical-align: middle;
    border-bottom: 1px solid var(--color-border);
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

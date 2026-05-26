<script lang="ts">
  import Badge from '$lib/components/Badge.svelte';
  import DeviceContextMenu from './DeviceContextMenu.svelte';
  import type { DeviceDto } from '../../bindings';

  interface Props {
    device: DeviceDto;
    onEdit: (_d: DeviceDto) => void;
    onDelete: () => void;
  }

  const { device, onEdit, onDelete }: Props = $props();

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

  const TYPE_LABELS: Record<number, string> = {
    1: 'Компьютер',
    2: 'Ноутбук',
    3: 'Монитор',
    4: 'Принтер',
    5: 'МФУ',
    6: 'Сервер',
    7: 'Сеть',
    8: 'Периферия',
    9: 'Прочее',
  };

  const statusLabel = $derived(STATUS_LABELS[device.status_id] ?? `Статус ${device.status_id}`);
  const statusVariant = $derived(STATUS_VARIANTS[device.status_id] ?? 'default');
  const typeLabel = $derived(TYPE_LABELS[device.type_id] ?? `Тип ${device.type_id}`);
</script>

<tr class="device-row">
  <td class="cell cell-type">{typeLabel}</td>
  <td class="cell cell-name">{device.name}</td>
  <td class="cell cell-numeric">{device.inventory_no ?? '—'}</td>
  <td class="cell cell-numeric">{device.serial_no ?? '—'}</td>
  <td class="cell">{device.model ?? '—'}</td>
  <td class="cell">{device.location_id ?? '—'}</td>
  <td class="cell cell-status">
    <Badge variant={statusVariant}>{statusLabel}</Badge>
  </td>
  <td class="cell cell-actions">
    <DeviceContextMenu {device} {onEdit} {onDelete} />
  </td>
</tr>

<style lang="scss">
  .device-row {
    height: var(--row-height, 40px);

    &:hover {
      background: var(--color-surface);
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

  .cell-type {
    width: 100px;
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

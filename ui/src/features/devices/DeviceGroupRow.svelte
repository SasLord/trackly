<script lang="ts">
  // DeviceGroupRow — expandable row for non-unique device group.
  // Per UI-SPEC §DeviceGroupRow, DEV-11.
  // On expand: fetches full DeviceDto list via devices.listByIds().

  import Badge from '$lib/components/Badge.svelte';
  import TableRow from '$lib/components/TableRow.svelte';
  import DeviceListRow from './DeviceListRow.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { devices } from './api';
  import type { DeviceDto, DeviceGroup } from '../../bindings';

  // Compute a stable string key for this group from its grouping dimensions.
  // The key is based on the repr device's displayable fields that form the GROUP BY
  // clause in list_grouped SQL. Using repr.id as the key is fragile (repr changes
  // when the representative is deleted); this hash-based key is stable as long as
  // the group exists with the same attributes.
  function groupStableKey(g: DeviceGroup): string {
    return [
      g.repr.name,
      g.repr.model ?? '',
      g.repr.specs ?? '',
      g.repr.kit ?? '',
      g.repr.state ?? '',
      g.repr.full_path ?? '',
      String(g.repr.status_id),
    ].join('\x00');
  }

  interface Props {
    group: DeviceGroup;
    /** Set of stable keys that should be rendered expanded (from parent DevicesPage). */
    expandedGroups: Set<string>;
    /** Called when this group's expanded state changes. */
    onExpandToggle?: (_key: string, _expanded: boolean) => void;
    onEdit: (_d: DeviceDto) => void;
    onDelete: () => void;
    /** Plan 03-05 (DEV-14): pass-through. */
    onPrintAcceptance?: (_d: DeviceDto) => void;
    /** ITEM-3: when true, shows the «Статус» column. Hide on filtered status tabs. */
    showStatus?: boolean;
  }

  const {
    group,
    expandedGroups,
    onExpandToggle,
    onEdit,
    onDelete,
    onPrintAcceptance,
    showStatus = true,
  }: Props = $props();

  const stableKey = $derived(groupStableKey(group));
  const expanded = $derived(expandedGroups.has(stableKey));

  let children = $state<DeviceDto[] | null>(null);
  let loadingChildren = $state(false);

  // Status label for repr device.
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

  const statusLabel = $derived(
    STATUS_LABELS[group.repr.status_id] ?? `Статус ${group.repr.status_id}`,
  );
  const statusVariant = $derived(STATUS_VARIANTS[group.repr.status_id] ?? 'default');

  // ITEM-1: «разное» для смешанной группы по condition_distinct_count
  const conditionDisplay = $derived(
    group.condition_distinct_count > 1 ? 'разное' : (group.repr.state ?? '—'),
  );

  async function toggleExpand() {
    const willExpand = !expanded;
    // Notify parent to update the expandedGroups Set.
    onExpandToggle?.(stableKey, willExpand);
    if (willExpand && children === null) {
      loadingChildren = true;
      try {
        children = await devices.listByIds(group.ids);
      } catch (e: unknown) {
        const msg =
          e && typeof e === 'object' && 'message' in e
            ? String((e as { message: unknown }).message)
            : 'Не удалось загрузить устройства';
        pushToast('error', msg);
        // Collapse again on error.
        onExpandToggle?.(stableKey, false);
      } finally {
        loadingChildren = false;
      }
    }
  }

  // When this component mounts and the group is already in the expandedGroups set
  // (i.e. the list refreshed after a mutation), auto-load children if not yet cached.
  $effect(() => {
    if (expanded && children === null && !loadingChildren) {
      loadingChildren = true;
      devices
        .listByIds(group.ids)
        .then((rows) => {
          children = rows;
        })
        .catch((e: unknown) => {
          const msg =
            e && typeof e === 'object' && 'message' in e
              ? String((e as { message: unknown }).message)
              : 'Не удалось загрузить устройства';
          pushToast('error', msg);
          onExpandToggle?.(stableKey, false);
        })
        .finally(() => {
          loadingChildren = false;
        });
    }
  });

  // After a mutation (edit/delete) let the parent refresh first (onEdit/onDelete
  // triggers refresh() in DevicesPage), then the component remounts with children=null
  // and the $effect above reloads them if still expanded.
  function handleEdit(d: DeviceDto) {
    children = null;
    onEdit(d);
  }

  function handleDelete() {
    children = null;
    onDelete();
  }
</script>

<TableRow
  group
  groupExpanded={expanded}
  groupName={group.repr.name}
  groupColspan={4}
  onToggleGroup={toggleExpand}
>
  <!-- groupColspan={4} merges Наименование + Инв.№ + Серийный № + Модель columns;
       TableRow's own group-mode chevron + merged name cell replace the hand-rolled
       ones this migration removes. -->
  <td class="cell cell-truncate" title={group.repr.full_path ?? ''}
    >{group.repr.place_path_short ?? '—'}</td
  >
  <td class="cell cell-truncate" title={conditionDisplay}>{conditionDisplay}</td>
  {#if showStatus}
    <td class="cell cell-status">
      <Badge variant={statusVariant}>{statusLabel}</Badge>
    </td>
  {/if}
  <!-- Actions column: count badge for multi-device groups -->
  <td class="cell cell-actions cell-count">
    <Badge variant="accent" appearance="count">{group.count} шт.</Badge>
  </td>
</TableRow>

{#if expanded}
  {#if loadingChildren}
    <tr class="children-loading-row">
      <td colspan={showStatus ? 8 : 7} class="children-loading">Загрузка…</td>
    </tr>
  {:else if children && children.length > 0}
    {#each children as child, i (child.id)}
      <DeviceListRow
        device={child}
        onEdit={handleEdit}
        onDelete={handleDelete}
        isLastInGroup={i === children.length - 1}
        {onPrintAcceptance}
        {showStatus}
      />
    {/each}
  {/if}
{/if}

<style lang="scss">
  .cell {
    font-size: var(--tr-font-size-body);
    color: var(--tr-text-primary);
  }

  // Location + Состояние cells: single line with ellipsis (mirror DeviceListRow .cell).
  // max-width: 0 makes text-overflow work inside a table cell; title= provides the
  // full text on hover (ITEM-2 tooltip). Prevents long condition labels from
  // wrapping and stretching the group row height.
  .cell-truncate {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 0;
  }

  .cell-status {
    width: 120px;
  }

  .cell-actions {
    width: 40px;
    text-align: center;
    overflow: visible;
  }

  .cell-count {
    white-space: nowrap;
  }

  .children-loading-row td {
    background: var(--tr-bg);
  }

  .children-loading {
    padding: var(--tr-space-2xs) var(--tr-space-md);
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-tertiary);
    border-bottom: 1px solid var(--tr-border);
  }
</style>

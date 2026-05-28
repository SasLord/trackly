<script lang="ts">
  // DeviceGroupRow — expandable row for non-unique device group.
  // Per UI-SPEC §DeviceGroupRow, DEV-11.
  // On expand: fetches full DeviceDto list via devices.listByIds().

  import Badge from '$lib/components/Badge.svelte';
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
      g.repr.location ?? '',
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
  }

  const { group, expandedGroups, onExpandToggle, onEdit, onDelete }: Props = $props();

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

<tr class="group-row" onclick={toggleExpand}>
  <!-- colspan="4" merges Наименование + Инв.№ + Серийный № + Модель columns.
       Chevron is inline, followed by the group name — no truncation needed. -->
  <td class="cell cell-name-wide" colspan="4">
    <button
      type="button"
      class="chevron-btn"
      class:expanded
      aria-label={expanded ? 'Свернуть группу' : 'Развернуть группу'}
      onclick={(e) => {
        e.stopPropagation();
        toggleExpand();
      }}
    >
      <svg
        width="14"
        height="14"
        viewBox="0 0 14 14"
        fill="none"
        xmlns="http://www.w3.org/2000/svg"
      >
        <path
          d="M4 5L7 8L10 5"
          stroke="currentColor"
          stroke-width="1.5"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
    </button>
    {group.repr.name}
  </td>
  <td class="cell">{group.repr.location ?? '—'}</td>
  <td class="cell cell-status">
    <Badge variant={statusVariant}>{statusLabel}</Badge>
  </td>
  <!-- Actions column: count badge for multi-device groups -->
  <td class="cell cell-actions cell-count">
    <span class="count-pill">{group.count} шт.</span>
  </td>
</tr>

{#if expanded}
  {#if loadingChildren}
    <tr class="children-loading-row">
      <td colspan="7" class="children-loading">Загрузка…</td>
    </tr>
  {:else if children && children.length > 0}
    {#each children as child, i (child.id)}
      <DeviceListRow
        device={child}
        onEdit={handleEdit}
        onDelete={handleDelete}
        isLastInGroup={i === children.length - 1}
      />
    {/each}
  {/if}
{/if}

<style lang="scss">
  .group-row {
    height: var(--row-height, 40px);
    // Toned-down group header: slightly darker than normal rows but not a strong accent.
    // color-mix blends the surface with a small 6% accent tint — readable but subtle.
    background: color-mix(in srgb, var(--color-surface) 94%, var(--color-accent) 6%);
    cursor: pointer;

    &:hover {
      background: color-mix(in srgb, var(--color-surface) 86%, var(--color-accent) 14%);
    }
  }

  .cell {
    padding: 0 var(--space-sm);
    font-size: var(--font-size-body);
    color: var(--color-text-primary);
    vertical-align: middle;
    border-bottom: 1px solid var(--color-border);
  }

  // Name cell spans Наименование + Инв.№ + Серийный + Модель — no truncation.
  // Using flex inside the td via a wrapper pattern: chevron + name text inline.
  .cell-name-wide {
    white-space: nowrap;
    font-weight: var(--font-weight-medium);
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

  .chevron-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    padding: 0;
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    color: var(--color-text-secondary);
    cursor: pointer;
    flex-shrink: 0;
    transition: transform 0.15s ease;

    &:hover {
      color: var(--color-text-primary);
      background: var(--color-surface);
    }

    &.expanded {
      transform: rotate(180deg);
    }
  }

  .count-pill {
    display: inline-flex;
    align-items: center;
    padding: 2px 8px;
    background: color-mix(in srgb, var(--color-accent) 12%, transparent);
    border: 1px solid color-mix(in srgb, var(--color-accent) 30%, transparent);
    border-radius: 10px;
    font-size: 12px;
    font-weight: var(--font-weight-medium);
    color: var(--color-accent);
  }

  .children-loading-row td {
    background: var(--color-bg);
  }

  .children-loading {
    padding: var(--space-xs) var(--space-md);
    font-size: var(--font-size-label);
    color: var(--color-text-muted);
    border-bottom: 1px solid var(--color-border);
  }
</style>

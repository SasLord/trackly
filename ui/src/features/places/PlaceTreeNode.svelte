<script lang="ts">
  // Phase 39 Plan 14 (PLC-01/PLC-02/PLC-06, 39-UI-SPEC.md §8.2-§8.4): one row of
  // the place tree — 32px, chevron/name/badges/counter/ActionMenu — self-
  // recursive for children (role="group" wrapper) so the DOM matches §8.5's
  // ARIA contract exactly. All keyboard-navigation LOGIC lives in the parent
  // PlaceTree.svelte (operating on its own flattened `visibleNodes` list, not
  // on this component's DOM structure) — this component only renders and wires
  // clicks back up through the shared `actions` object.
  //
  // Drag-n-drop (§8.4/D-21, UAT gap 6): pointer-based drag state and hit-
  // testing live entirely in PlaceTree.svelte's `.tree-body` container
  // (pointerdown/pointermove/pointerup delegation + elementFromPoint), NOT
  // here — a captured pointer keeps delivering events to the element that
  // captured it, so per-row native-DnD-style callbacks would never fire once
  // a drag starts. This component only renders `data-place-id` (for the
  // container's hit-testing) and the drag-derived CSS classes
  // (dragging/drop-valid/drop-invalid), driven by the `draggingId`/
  // `dragOverId`/`isInvalidDropTarget` props it already receives.
  import PlaceTreeNode from './PlaceTreeNode.svelte';
  import ActionMenu from '$lib/components/ActionMenu.svelte';
  import Badge from '$lib/components/Badge.svelte';
  import type { PlaceDto } from '../../bindings';

  interface TreeActions {
    onToggleExpand: (_id: number) => void;
    onSelect: (_node: PlaceDto) => void;
    onFocusRow: (_id: number) => void;
    onRename: (_node: PlaceDto) => void;
    onCreateChild: (_node: PlaceDto) => void;
    onMove: (_node: PlaceDto) => void;
    onArchiveToggle: (_node: PlaceDto) => void;
    onDelete: (_node: PlaceDto) => void;
  }

  interface Props {
    node: PlaceDto;
    depth: number;
    childrenMap: Map<number | null, PlaceDto[]>;
    stats: Record<number, number>;
    expandedIds: number[];
    selectedId: number | null;
    focusedId: number | null;
    isAdmin: boolean;
    draggingId: number | null;
    dragOverId: number | null;
    isInvalidDropTarget: (_id: number) => boolean;
    actions: TreeActions;
  }

  const {
    node,
    depth,
    childrenMap,
    stats,
    expandedIds,
    selectedId,
    focusedId,
    isAdmin,
    draggingId,
    dragOverId,
    isInvalidDropTarget,
    actions,
  }: Props = $props();

  // §17.1: node kind is never shown inline — only in `title` and the (Plan 20)
  // detail-panel header.
  const KIND_LABELS: Record<string, string> = {
    territory: 'территория',
    zone: 'зона',
    building: 'здание',
    floor: 'этаж',
    room: 'помещение',
    outdoor: 'уличный объект',
  };

  const children = $derived(childrenMap.get(node.id) ?? []);
  const hasChildren = $derived(children.length > 0);
  const expanded = $derived(expandedIds.includes(node.id));
  const isSelected = $derived(selectedId === node.id);
  const isArchived = $derived(node.archived_at_utc !== null);
  const isFocused = $derived(focusedId === node.id);
  const isDragging = $derived(draggingId === node.id);
  const isDragOver = $derived(
    dragOverId === node.id && draggingId !== null && draggingId !== node.id,
  );
  const isInvalidTarget = $derived(isDragOver && isInvalidDropTarget(node.id));
  const contentCount = $derived(stats[node.id]);
  const rowTitle = $derived(
    `${node.full_path ?? node.name} — ${KIND_LABELS[node.kind] ?? node.kind}`,
  );

  function handleChevronClick(e: MouseEvent): void {
    e.preventDefault();
    e.stopPropagation();
    if (!hasChildren) return;
    actions.onToggleExpand(node.id);
  }

  function handleRowClick(): void {
    actions.onSelect(node);
  }

  function handleRowKeydown(e: KeyboardEvent): void {
    // Composite-widget defensive pairing (a11y gate requires a keydown handler
    // alongside onclick) — actual keyboard navigation is delegated to the
    // role="tree" container's keydown handler in PlaceTree.svelte.
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      actions.onSelect(node);
    }
  }
</script>

<div
  id={`place-tree-row-${node.id}`}
  data-place-id={node.id}
  class="place-tree-row"
  class:selected={isSelected}
  class:dragging={isDragging}
  class:drop-valid={isDragOver && !isInvalidTarget}
  class:drop-invalid={isInvalidTarget}
  role="treeitem"
  tabindex={isFocused ? 0 : -1}
  aria-level={depth + 1}
  aria-selected={isSelected}
  aria-expanded={hasChildren ? expanded : undefined}
  title={rowTitle}
  style={`padding-left: calc(var(--tr-space-xs) + ${depth} * var(--tr-space-md))`}
  onclick={handleRowClick}
  onkeydown={handleRowKeydown}
  onfocus={() => actions.onFocusRow(node.id)}
>
  <span class="chevron-slot">
    {#if hasChildren}
      <button
        type="button"
        class="chevron"
        class:expanded
        aria-hidden="true"
        tabindex="-1"
        onmousedown={(e) => e.preventDefault()}
        onclick={handleChevronClick}
      >
        <svg width="10" height="10" viewBox="0 0 10 10" fill="none" aria-hidden="true">
          <path
            d="M3 1l4 4-4 4"
            stroke="currentColor"
            stroke-width="1.5"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        </svg>
      </button>
    {/if}
  </span>

  <span class="name" class:selected={isSelected} class:archived={isArchived}>{node.name}</span>

  {#if node.is_storage}
    <Badge variant="accent" appearance="soft" size="sm">Склад</Badge>
  {/if}
  {#if isArchived}
    <Badge variant="default" appearance="soft" size="sm">Архив</Badge>
  {/if}

  {#if contentCount !== undefined && contentCount > 0}
    <!-- UAT gap 6 (2026-08-25): was a bare number crowding its neighbours;
         reuse Badge's existing "count" pill appearance (padding + rounded
         shape from design tokens, no hardcoded colors) instead of hand-rolling
         a new pill. `.tr-mono` and the `title` behaviour are unchanged. -->
    <Badge
      variant="default"
      appearance="count"
      size="sm"
      title={`Всего с вложенными: ${contentCount}`}
    >
      <span class="tr-mono">{contentCount}</span>
    </Badge>
  {/if}

  {#if isAdmin}
    <span class="row-actions">
      <ActionMenu variant="ghost-sm" label={`Действия: ${node.name}`}>
        <button type="button" role="menuitem" onclick={() => actions.onRename(node)}
          >Переименовать</button
        >
        <button type="button" role="menuitem" onclick={() => actions.onCreateChild(node)}>
          Создать вложенное место
        </button>
        <button type="button" role="menuitem" onclick={() => actions.onMove(node)}
          >Переместить в…</button
        >
        <button type="button" role="menuitem" onclick={() => actions.onArchiveToggle(node)}>
          {isArchived ? 'Вернуть из архива' : 'Архивировать'}
        </button>
        <button
          type="button"
          role="menuitem"
          class="menu-danger"
          onclick={() => actions.onDelete(node)}
        >
          Удалить
        </button>
      </ActionMenu>
    </span>
  {/if}
</div>

{#if expanded && hasChildren}
  <div role="group">
    {#each children as child (child.id)}
      <PlaceTreeNode
        node={child}
        depth={depth + 1}
        {childrenMap}
        {stats}
        {expandedIds}
        {selectedId}
        {focusedId}
        {isAdmin}
        {draggingId}
        {dragOverId}
        {isInvalidDropTarget}
        {actions}
      />
    {/each}
  </div>
{/if}

<style lang="scss">
  .place-tree-row {
    display: flex;
    align-items: center;
    gap: var(--tr-space-2xs);
    height: 32px;
    padding-right: var(--tr-space-md);
    cursor: pointer;
    transition: none;
    // Pointer-based drag (UAT gap 6) does its own hit-testing over the row
    // for the whole pointerdown→pointerup span; text selection during that
    // span is pure friction, never useful.
    user-select: none;

    &:hover {
      background: var(--tr-row-hover);
    }
    &.selected {
      background: var(--tr-row-selected);
      box-shadow: inset 2px 0 0 0 var(--tr-accent);
    }
    &:focus-visible {
      outline: none;
      box-shadow: inset 0 0 0 2px var(--tr-focus-ring);
    }
    &.selected:focus-visible {
      box-shadow:
        inset 2px 0 0 0 var(--tr-accent),
        inset 0 0 0 2px var(--tr-focus-ring);
    }
    &.dragging .name {
      color: var(--tr-text-tertiary);
    }
    &.drop-valid {
      background: var(--tr-accent-soft);
      box-shadow: inset 0 0 0 1px var(--tr-accent);
    }
    &.drop-invalid {
      background: var(--tr-danger-soft);
      cursor: not-allowed;
    }
  }

  .chevron-slot {
    flex: none;
    width: 16px;
    height: 16px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .chevron {
    width: 16px;
    height: 16px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: none;
    background: transparent;
    padding: 0;
    color: var(--tr-text-tertiary);
    cursor: pointer;
    transition: none;

    &.expanded {
      transform: rotate(90deg);
    }
  }

  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--tr-text-primary);
    font-size: var(--tr-font-size-body);

    &.selected {
      font-weight: var(--tr-font-weight-body-strong);
    }
    &.archived {
      color: var(--tr-text-tertiary);
    }
  }

  .row-actions {
    flex: none;
    display: inline-flex;
    opacity: 0;

    .place-tree-row:hover &,
    .place-tree-row:focus-within &,
    .place-tree-row.selected & {
      opacity: 1;
    }
  }

  .menu-danger {
    border-top: 1px solid var(--tr-border);
    color: var(--tr-danger-text);
  }
</style>

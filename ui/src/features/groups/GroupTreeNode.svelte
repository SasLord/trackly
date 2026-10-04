<script lang="ts" module>
  import type { GroupDto, GroupTypeDto } from '../../bindings';

  /** Узел дерева «Группы»: корни — типы, внутри — группы (вложенная — только под родителем). */
  export interface TreeItem {
    /** `type-{id}` | `group-{id}` — стабильный ключ узла (раскрытие, фокус, DOM id). */
    key: string;
    kind: 'type' | 'group';
    id: number;
    name: string;
    type: GroupTypeDto | null;
    group: GroupDto | null;
    children: TreeItem[];
  }

  export interface TreeNodeActions {
    onToggleExpand: (_key: string) => void;
    onSelect: (_item: TreeItem) => void;
    onFocusRow: (_key: string) => void;
    onCreateGroup: (_typeId: number) => void;
    onRename: (_item: TreeItem) => void;
    onMove: (_groupId: number) => void;
    onRemoveFromParent: (_groupId: number) => void;
    onDelete: (_item: TreeItem) => void;
  }

  export function rowDomId(key: string): string {
    return `group-tree-row-${key}`;
  }
</script>

<script lang="ts">
  // Phase 41 Plan 19 (GRP-04, 41-UI-SPEC.md §8.2-§8.3): одна строка дерева «Группы»,
  // 32px, анатомия PlaceTreeNode (chevron / имя / бейдж / счётчик / ActionMenu),
  // саморекурсивна для детей (обёртка role="group"). Вся клавиатурная ЛОГИКА живёт
  // в родителе GroupTree.svelte (по его плоскому visibleNodes) — этот компонент
  // только рисует и пробрасывает клики через `actions`.
  //
  // Нет перетаскивания узлов (D-04: вложение — строкой-поиском в составе).
  // Нет пункта создания вложенной группы (тот же D-04: два пути с разной
  // семантикой недопустимы).
  import GroupTreeNode from './GroupTreeNode.svelte';
  import ActionMenu from '$lib/components/ActionMenu.svelte';
  import Badge from '$lib/components/Badge.svelte';

  interface Props {
    item: TreeItem;
    depth: number;
    expandedKeys: string[];
    selectedKey: string | null;
    focusedKey: string | null;
    query: string;
    canEditTypes: boolean;
    canMutate: boolean;
    actions: TreeNodeActions;
  }

  const {
    item,
    depth,
    expandedKeys,
    selectedKey,
    focusedKey,
    query,
    canEditTypes,
    canMutate,
    actions,
  }: Props = $props();

  // §8.2: вид узла (тип/группа) в строке не подписывается — только в `title`.
  const BEHAVIOR_LABELS: Record<string, string> = {
    container: 'Контейнер',
    substitute: 'Замещение',
    teardown: 'Разбор',
  };

  const hasChildren = $derived(item.children.length > 0);
  const expanded = $derived(expandedKeys.includes(item.key));
  const isSelected = $derived(selectedKey === item.key);
  const isFocused = $derived(focusedKey === item.key);

  // Тип: число групп типа; группа: device_count (серверный, ВКЛЮЧАЯ вложенные).
  // Нулевой счётчик не рендерится.
  const count = $derived(
    item.kind === 'type' ? (item.type?.group_count ?? 0) : (item.group?.device_count ?? 0),
  );
  const countTitle = $derived(
    item.kind === 'type' ? `Групп: ${count}` : `Устройств с вложенными: ${count}`,
  );
  const noPlace = $derived(item.kind === 'group' && item.group?.place_id === null);

  const rowTitle = $derived(
    item.kind === 'type'
      ? `тип группы · ${BEHAVIOR_LABELS[item.type?.behavior ?? ''] ?? item.type?.behavior ?? ''}`
      : (item.group?.place_path ?? 'место не задано'),
  );

  // Подсветка найденного фрагмента цветом, БЕЗ фоновой заливки (§8.1).
  const nameParts = $derived.by(() => {
    const q = query.trim().toLowerCase();
    const name = item.name;
    if (q === '') return null;
    const at = name.toLowerCase().indexOf(q);
    if (at < 0 || name.toLowerCase().length !== name.length) return null;
    return {
      before: name.slice(0, at),
      hit: name.slice(at, at + q.length),
      after: name.slice(at + q.length),
    };
  });

  const showTypeMenu = $derived(item.kind === 'type' && canEditTypes);
  const showGroupMenu = $derived(item.kind === 'group' && canMutate);
  const canDeleteType = $derived(
    item.kind === 'type' &&
      item.type !== null &&
      !item.type.is_builtin &&
      item.type.group_count === 0,
  );
  const isRootGroup = $derived(item.kind === 'group' && item.group?.parent_group_id === null);
  const isNestedGroup = $derived(item.kind === 'group' && item.group?.parent_group_id !== null);

  function handleChevronClick(e: MouseEvent): void {
    e.preventDefault();
    e.stopPropagation();
    if (!hasChildren) return;
    actions.onToggleExpand(item.key);
  }

  function handleRowKeydown(e: KeyboardEvent): void {
    // Парный обработчик для a11y-гейта; навигация — в role="tree" контейнере GroupTree.
    // Только своё событие: Enter/Space из ActionMenu внутри строки не должны
    // выбирать узел и гасить открытие меню (preventDefault).
    if (e.target !== e.currentTarget) return;
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      actions.onSelect(item);
    }
  }
</script>

<div
  id={rowDomId(item.key)}
  data-node-key={item.key}
  class="group-tree-row"
  class:selected={isSelected}
  role="treeitem"
  tabindex={isFocused ? 0 : -1}
  aria-level={depth + 1}
  aria-selected={isSelected}
  aria-expanded={hasChildren ? expanded : undefined}
  title={rowTitle}
  style={`padding-left: calc(var(--tr-space-xs) + ${depth} * var(--tr-space-md))`}
  onclick={() => actions.onSelect(item)}
  onkeydown={handleRowKeydown}
  onfocus={() => actions.onFocusRow(item.key)}
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

  <span class="name" class:selected={isSelected}>
    {#if nameParts}{nameParts.before}<span class="hit">{nameParts.hit}</span
      >{nameParts.after}{:else}{item.name}{/if}
  </span>

  {#if noPlace}
    <Badge variant="default" appearance="soft" size="sm">Без места</Badge>
  {/if}

  {#if count > 0}
    <Badge variant="default" appearance="count" size="sm" title={countTitle}>
      <span class="tr-mono">{count}</span>
    </Badge>
  {/if}

  {#if showTypeMenu && item.type}
    <span class="row-actions">
      <ActionMenu variant="ghost-sm" label={`Действия: ${item.name}`}>
        <button type="button" role="menuitem" onclick={() => actions.onCreateGroup(item.id)}>
          Создать группу этого типа
        </button>
        <button type="button" role="menuitem" onclick={() => actions.onRename(item)}>
          Переименовать
        </button>
        {#if canDeleteType}
          <button
            type="button"
            role="menuitem"
            class="menu-danger"
            onclick={() => actions.onDelete(item)}
          >
            Удалить
          </button>
        {/if}
      </ActionMenu>
    </span>
  {:else if showGroupMenu && item.group}
    <span class="row-actions">
      <ActionMenu variant="ghost-sm" label={`Действия: ${item.name}`}>
        <button type="button" role="menuitem" onclick={() => actions.onRename(item)}>
          Переименовать
        </button>
        {#if isRootGroup}
          <button type="button" role="menuitem" onclick={() => actions.onMove(item.id)}>
            Перенести…
          </button>
        {/if}
        {#if isNestedGroup}
          <button type="button" role="menuitem" onclick={() => actions.onRemoveFromParent(item.id)}>
            Вывести из состава
          </button>
        {/if}
        <button
          type="button"
          role="menuitem"
          class="menu-danger"
          onclick={() => actions.onDelete(item)}
        >
          Удалить
        </button>
      </ActionMenu>
    </span>
  {/if}
</div>

{#if expanded && hasChildren}
  <div role="group">
    {#each item.children as child (child.key)}
      <GroupTreeNode
        item={child}
        depth={depth + 1}
        {expandedKeys}
        {selectedKey}
        {focusedKey}
        {query}
        {canEditTypes}
        {canMutate}
        {actions}
      />
    {/each}
  </div>
{/if}

<style lang="scss">
  .group-tree-row {
    display: flex;
    align-items: center;
    gap: var(--tr-space-2xs);
    height: 32px;
    padding-right: var(--tr-space-md);
    cursor: pointer;
    transition: none;
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
  }

  .hit {
    color: var(--tr-accent-text);
    font-weight: var(--tr-font-weight-body-strong);
  }

  .row-actions {
    flex: none;
    display: inline-flex;
    opacity: 0;

    .group-tree-row:hover &,
    .group-tree-row:focus-within &,
    .group-tree-row.selected & {
      opacity: 1;
    }
  }

  .menu-danger {
    border-top: 1px solid var(--tr-border);
    color: var(--tr-danger-text);
  }
</style>

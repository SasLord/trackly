<script lang="ts" module>
  /** Ссылка на узел дерева: тип или группа. Контракт с GroupsPage (план 23). */
  export interface GroupTreeNodeRef {
    kind: 'type' | 'group';
    id: number;
  }
</script>

<script lang="ts">
  // Phase 41 Plan 19 (GRP-04, 41-UI-SPEC.md §7.4/§8): левая панель раздела
  // «Группы» — дерево, где корни — ТИПЫ групп, а внутри — группы (вложенная
  // группа показывается только под родителем, под типом не дублируется).
  //
  // Это НОВЫЙ сиблинг PlaceTree: копируется АНАТОМИЯ (role=tree/treeitem/group,
  // aria-level/expanded/selected, roving tabindex, клавиатура по плоскому
  // visibleNodes, строка 32px), но PlaceTree не обобщается — у него свои
  // перетаскивание, архив и ленивые счётчики, которых здесь нет.
  //
  // Данные: оба списка грузятся целиком (`group_types_list` + `groups_list`),
  // дерево собирается на клиенте. Счётчики (`group_count`, `device_count`)
  // серверные — клиент их не считает.
  //
  // Инвалидация (D-08): перезагрузка по `refreshToken` страницы и по
  // `placeContentEventsStore.seq`. Перезагрузка запускается ВНУТРИ untrack:
  // loadAll пишет $state (types/groupList/loading/…), который сам же читает, —
  // без untrack это effect_update_depth_exceeded (компиляционные гейты слепы к
  // рантайму рун, см. 40-32 UAT3-03a).
  import { untrack } from 'svelte';
  import { groupTypes, groups as groupsApi } from '$lib/api/groups';
  import { authStore } from '$lib/stores/auth.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { placeContentEventsStore } from '$lib/stores/placeContentEvents.svelte';
  import Input from '$lib/components/Input.svelte';
  import Button from '$lib/components/Button.svelte';
  import Spinner from '$lib/components/Spinner.svelte';
  import GroupTreeNode, {
    rowDomId,
    type TreeItem,
    type TreeNodeActions,
  } from './GroupTreeNode.svelte';
  import type { GroupDto, GroupTypeDto } from '../../bindings';

  interface Props {
    initialSelected: GroupTreeNodeRef | null;
    /** Страница бампит после любой мутации — дерево перезагружается. */
    refreshToken?: number;
    /**
     * Переход-фокус (D-19, D-28, D-07): НОВЫЙ объект (с новым token) раскрывает путь
     * до узла, выбирает его, ставит фокус и scrollIntoView({block:'nearest'}).
     */
    focusRequest?: (GroupTreeNodeRef & { token: number }) | null;
    /** admin: создание/переименование/удаление типов. */
    canEditTypes: boolean;
    onSelect: (_node: GroupTreeNodeRef | null) => void;
    onCreateType: () => void;
    onCreateGroup: (_typeId: number | null) => void;
    onRename: (_node: GroupTreeNodeRef) => void;
    /** Вторая точка входа D-18 (первая — кнопка панели группы): тот же обработчик страницы. */
    onMove: (_groupId: number) => void;
    onRemoveFromParent: (_groupId: number) => void;
    onDelete: (_node: GroupTreeNodeRef) => void;
    /** Отдаёт странице свежие данные после каждой успешной загрузки. */
    onLoaded?: (_types: GroupTypeDto[], _groups: GroupDto[]) => void;
  }

  const {
    initialSelected,
    refreshToken = 0,
    focusRequest = null,
    canEditTypes,
    onSelect,
    onCreateType,
    onCreateGroup,
    onRename,
    onMove,
    onRemoveFromParent,
    onDelete,
    onLoaded,
  }: Props = $props();

  // Права на группы — UX-слой (граница безопасности — сервер).
  const canMutate = $derived(
    authStore.user?.role === 'admin' || authStore.user?.role === 'manager',
  );

  const typeKey = (id: number) => `type-${id}`;
  const groupKey = (id: number) => `group-${id}`;

  // --- Раскрытые узлы: localStorage `trackly:groups:expanded` ------------------
  // Выбранный узел хранит СТРАНИЦА (initialSelected) — одно место записи на
  // одно состояние, как PlacesPage/PlaceTree.
  const EXPANDED_STORAGE_KEY = 'trackly:groups:expanded';

  function readPersistedExpanded(): string[] {
    if (typeof window === 'undefined') return [];
    try {
      const raw = localStorage.getItem(EXPANDED_STORAGE_KEY);
      if (!raw) return [];
      const parsed: unknown = JSON.parse(raw);
      if (!Array.isArray(parsed)) return [];
      return parsed.filter((v): v is string => typeof v === 'string');
    } catch {
      return [];
    }
  }

  // --- Данные -------------------------------------------------------------------
  let types = $state<GroupTypeDto[] | null>(null);
  let groupList = $state<GroupDto[]>([]);
  let loading = $state(false);
  let loadError = $state(false);
  // Не $state: счётчик устаревших ответов, не рисуется.
  let loadSeq = 0;

  interface TreeData {
    roots: TreeItem[];
    itemByKey: Map<string, TreeItem>;
    parentOf: Map<string, string | null>;
  }

  const treeData = $derived.by<TreeData>(() => {
    const itemByKey = new Map<string, TreeItem>();
    const parentOf = new Map<string, string | null>();

    const byId = new Map<number, GroupDto>();
    for (const g of groupList) byId.set(g.id, g);

    const nestedByParent = new Map<number, GroupDto[]>();
    const rootsByType = new Map<number, GroupDto[]>();
    for (const g of groupList) {
      // Сирота (родитель не пришёл в списке) показывается корнем своего типа —
      // группа не должна пропадать из дерева.
      if (g.parent_group_id !== null && byId.has(g.parent_group_id)) {
        const arr = nestedByParent.get(g.parent_group_id);
        if (arr) arr.push(g);
        else nestedByParent.set(g.parent_group_id, [g]);
      } else {
        const arr = rootsByType.get(g.type_id);
        if (arr) arr.push(g);
        else rootsByType.set(g.type_id, [g]);
      }
    }
    const bySeq = (a: GroupDto, b: GroupDto) => a.seq - b.seq || a.id - b.id;

    function makeGroup(g: GroupDto, parentKey: string): TreeItem {
      const key = groupKey(g.id);
      const kids = (nestedByParent.get(g.id) ?? []).sort(bySeq);
      const item: TreeItem = {
        key,
        kind: 'group',
        id: g.id,
        name: g.name,
        type: null,
        group: g,
        children: [],
      };
      itemByKey.set(key, item);
      parentOf.set(key, parentKey);
      item.children = kids.map((k) => makeGroup(k, key));
      return item;
    }

    const typeList = [...(types ?? [])].sort((a, b) => a.sort_order - b.sort_order || a.id - b.id);
    const roots: TreeItem[] = typeList.map((t) => {
      const key = typeKey(t.id);
      const item: TreeItem = {
        key,
        kind: 'type',
        id: t.id,
        name: t.name,
        type: t,
        group: null,
        children: [],
      };
      itemByKey.set(key, item);
      parentOf.set(key, null);
      item.children = (rootsByType.get(t.id) ?? []).sort(bySeq).map((g) => makeGroup(g, key));
      return item;
    });
    return { roots, itemByKey, parentOf };
  });

  // --- Поиск (§8.1): клиентская фильтрация по уже загруженным именам ------------
  let query = $state('');
  let liveMessage = $state('');
  // Раскрытие в режиме поиска — отдельный набор: схлопывание вручную работает и
  // во время поиска, а сохранённое раскрытие дерева не затирается.
  let searchExpanded = $state<string[]>([]);
  const isSearchMode = $derived(query.trim() !== '');

  const searchResult = $derived.by<{ roots: TreeItem[]; expand: string[]; count: number }>(() => {
    const q = query.trim().toLowerCase();
    if (q === '') return { roots: treeData.roots, expand: [], count: 0 };
    const expand: string[] = [];

    const selfMatches = (item: TreeItem): boolean =>
      item.name.toLowerCase().includes(q) ||
      (item.group?.type_name ?? '').toLowerCase().includes(q);

    function filter(item: TreeItem): TreeItem | null {
      if (selfMatches(item)) {
        // Совпал сам узел — показываем его целиком вместе с содержимым.
        if (item.kind === 'type' && item.children.length > 0) expand.push(item.key);
        return item;
      }
      const kept = item.children.map(filter).filter((c): c is TreeItem => c !== null);
      if (kept.length === 0) return null;
      expand.push(item.key);
      return { ...item, children: kept };
    }

    const roots = treeData.roots.map(filter).filter((r): r is TreeItem => r !== null);

    let count = 0;
    function countHits(list: TreeItem[]) {
      for (const it of list) {
        if (selfMatches(it)) count += 1;
        countHits(it.children);
      }
    }
    countHits(roots);
    return { roots, expand, count };
  });

  // --- Раскрытие / выбор / roving tabindex -------------------------------------
  let expandedKeys = $state<string[]>(readPersistedExpanded());
  let selectedKey = $state<string | null>(null);
  let activeKey = $state<string | null>(null);

  $effect(() => {
    if (typeof window === 'undefined') return;
    const snapshot = JSON.stringify(expandedKeys);
    try {
      localStorage.setItem(EXPANDED_STORAGE_KEY, snapshot);
    } catch {
      // Хранилище недоступно — раскрытие просто не переживёт навигацию.
    }
  });

  const displayRoots = $derived(isSearchMode ? searchResult.roots : treeData.roots);
  const activeExpanded = $derived(isSearchMode ? searchExpanded : expandedKeys);

  interface VisibleRow {
    item: TreeItem;
    parentKey: string | null;
  }

  const visibleNodes = $derived.by<VisibleRow[]>(() => {
    const rows: VisibleRow[] = [];
    function walk(list: TreeItem[], parentKey: string | null) {
      for (const it of list) {
        rows.push({ item: it, parentKey });
        if (activeExpanded.includes(it.key)) walk(it.children, it.key);
      }
    }
    walk(displayRoots, null);
    return rows;
  });

  // Единственная точка табуляции: активный узел, если он виден, иначе первый.
  const tabbableKey = $derived(
    visibleNodes.some((r) => r.item.key === activeKey)
      ? activeKey
      : (visibleNodes[0]?.item.key ?? null),
  );

  function setExpanded(keys: string[]): void {
    if (isSearchMode) searchExpanded = keys;
    else expandedKeys = keys;
  }

  function expandPathTo(key: string): void {
    const toExpand: string[] = [];
    let cur = treeData.parentOf.get(key) ?? null;
    while (cur !== null) {
      toExpand.push(cur);
      cur = treeData.parentOf.get(cur) ?? null;
    }
    if (toExpand.length > 0) expandedKeys = [...new Set([...expandedKeys, ...toExpand])];
  }

  function focusRow(key: string, scroll = false): void {
    activeKey = key;
    requestAnimationFrame(() => {
      const el = document.getElementById(rowDomId(key));
      el?.focus();
      if (scroll) el?.scrollIntoView({ block: 'nearest' });
    });
  }

  function handleSelectItem(item: TreeItem): void {
    selectedKey = item.key;
    activeKey = item.key;
    onSelect({ kind: item.kind, id: item.id });
  }

  // --- Загрузка -------------------------------------------------------------------
  let firstLoadHandled = false;
  let loaded = false;
  // Не $state: ожидающий переход-фокус, применяется когда данные есть.
  let pendingFocus: (GroupTreeNodeRef & { token: number }) | null = null;
  let pendingRetried = false;

  function applyPendingFocus(): void {
    const req = pendingFocus;
    if (!req) return;
    const key = req.kind === 'type' ? typeKey(req.id) : groupKey(req.id);
    const item = treeData.itemByKey.get(key);
    if (!item) {
      // Данные могли устареть (группа только что создана) — один повтор после перезагрузки.
      if (!pendingRetried) {
        pendingRetried = true;
        void loadAll();
      } else {
        pendingFocus = null;
      }
      return;
    }
    pendingFocus = null;
    pendingRetried = false;
    // Переход ведёт к узлу даже из режима поиска: поиск мешал бы его показать.
    query = '';
    liveMessage = '';
    expandPathTo(key);
    selectedKey = key;
    focusRow(key, true);
    onSelect({ kind: item.kind, id: item.id });
  }

  async function loadAll(): Promise<void> {
    const mySeq = ++loadSeq;
    loading = true;
    loadError = false;
    try {
      const [typeRows, groupRows] = await Promise.all([groupTypes.list(false), groupsApi.list()]);
      if (mySeq !== loadSeq) return; // пришёл более новый запрос
      types = typeRows;
      groupList = groupRows;
      loaded = true;
      onLoaded?.(typeRows, groupRows);
    } catch {
      if (mySeq !== loadSeq) return;
      // Список остаётся прежним; на первой загрузке — пустой, с сообщением ниже.
      if (types === null) types = [];
      loadError = true;
      pushToast('error', 'Не удалось загрузить группы. Проверьте подключение и повторите.');
    } finally {
      if (mySeq === loadSeq) loading = false;
    }
    if (loadError || mySeq !== loadSeq) return;

    // Выбросить устаревшие ключи раскрытия (узел удалён с прошлого визита).
    const alive = expandedKeys.filter((k) => treeData.itemByKey.has(k));
    if (alive.length !== expandedKeys.length) expandedKeys = alive;
    if (isSearchMode) searchExpanded = searchResult.expand;

    if (!firstLoadHandled) {
      firstLoadHandled = true;
      if (initialSelected !== null) {
        const key =
          initialSelected.kind === 'type'
            ? typeKey(initialSelected.id)
            : groupKey(initialSelected.id);
        if (treeData.itemByKey.has(key)) {
          expandPathTo(key);
          selectedKey = key;
          activeKey = key;
        } else {
          onSelect(null); // сохранённый узел больше не существует
        }
      }
    } else if (selectedKey !== null && !treeData.itemByKey.has(selectedKey)) {
      selectedKey = null;
      activeKey = null;
      onSelect(null);
    }

    applyPendingFocus();
  }

  // Загрузка по refreshToken страницы и по инвалидации содержимого мест (D-08).
  // Чтение — только токен и seq; сама перезагрузка (она пишет $state) — в untrack.
  $effect(() => {
    void refreshToken;
    void placeContentEventsStore.seq;
    void placeContentEventsStore.reloadSeq; // Фаза 41.7 (D-17): перезагрузка по событию другого клиента
    untrack(() => {
      void loadAll();
    });
  });

  // Переход-фокус: новый объект focusRequest. Реагируем только на сам проп.
  $effect(() => {
    const req = focusRequest;
    if (!req) return;
    untrack(() => {
      pendingFocus = req;
      pendingRetried = false;
      if (loaded) applyPendingFocus();
    });
  });

  // --- Поиск: ввод ------------------------------------------------------------------
  function handleSearchInput(v: string): void {
    query = v;
    if (v.trim() === '') {
      liveMessage = '';
      return;
    }
    // Derived читается синхронно после смены query — значения свежие.
    searchExpanded = searchResult.expand;
    liveMessage = `Найдено совпадений: ${searchResult.count}`;
  }

  // --- Клавиатура (§8.4) по плоскому visibleNodes, не по DOM ---------------------
  function handleTreeKeydown(e: KeyboardEvent): void {
    // События из ActionMenu внутри строки (стрелки/Enter) — не навигация по дереву.
    if ((e.target as HTMLElement | null)?.closest('.row-actions')) return;
    if (e.key === 'Escape' && query !== '') {
      e.preventDefault();
      query = '';
      liveMessage = '';
      return;
    }
    const rows = visibleNodes;
    if (rows.length === 0) return;
    const idx = rows.findIndex((r) => r.item.key === tabbableKey);

    if (e.key === 'ArrowDown') {
      e.preventDefault();
      focusRow(rows[idx < 0 ? 0 : Math.min(idx + 1, rows.length - 1)].item.key);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      focusRow(rows[idx < 0 ? 0 : Math.max(idx - 1, 0)].item.key);
    } else if (e.key === 'Home') {
      e.preventDefault();
      focusRow(rows[0].item.key);
    } else if (e.key === 'End') {
      e.preventDefault();
      focusRow(rows[rows.length - 1].item.key);
    } else if (e.key === 'ArrowRight') {
      e.preventDefault();
      if (idx < 0) return;
      const { item } = rows[idx];
      if (item.children.length === 0) return;
      if (!activeExpanded.includes(item.key)) {
        setExpanded([...activeExpanded, item.key]);
      } else {
        focusRow(item.children[0].key);
      }
    } else if (e.key === 'ArrowLeft') {
      e.preventDefault();
      if (idx < 0) return;
      const row = rows[idx];
      if (activeExpanded.includes(row.item.key)) {
        setExpanded(activeExpanded.filter((k) => k !== row.item.key));
      } else if (row.parentKey !== null) {
        focusRow(row.parentKey);
      }
    } else if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      if (idx >= 0) handleSelectItem(rows[idx].item);
    }
    // F2 не вводится: переименование — через меню узла (§8.4).
  }

  const nodeActions: TreeNodeActions = {
    onToggleExpand(key) {
      setExpanded(
        activeExpanded.includes(key)
          ? activeExpanded.filter((k) => k !== key)
          : [...activeExpanded, key],
      );
    },
    onSelect: handleSelectItem,
    onFocusRow(key) {
      activeKey = key;
    },
    onCreateGroup: (typeId) => onCreateGroup(typeId),
    onRename: (item) => onRename({ kind: item.kind, id: item.id }),
    onMove: (groupId) => onMove(groupId),
    onRemoveFromParent: (groupId) => onRemoveFromParent(groupId),
    onDelete: (item) => onDelete({ kind: item.kind, id: item.id }),
  };
</script>

<div class="group-tree-shell">
  <div class="toolbar">
    <label class="sr-only" for="group-tree-search">Поиск группы</label>
    <div class="toolbar-search">
      <Input
        id="group-tree-search"
        value={query}
        placeholder="Поиск группы"
        oninput={handleSearchInput}
      />
    </div>
    {#if canEditTypes}
      <Button variant="secondary" onclick={onCreateType}>Создать тип</Button>
    {/if}
    <Button
      variant="ghost"
      iconOnly
      ariaLabel="Обновить"
      title="Обновить"
      onclick={() => void loadAll()}
    >
      <svg width="16" height="16" viewBox="0 0 16 16" fill="none" aria-hidden="true">
        <path
          d="M13.5 8a5.5 5.5 0 1 1-1.6-3.9M13.5 2.5v3h-3"
          stroke="currentColor"
          stroke-width="1.5"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
    </Button>
  </div>

  <div
    class="tree-body"
    role="tree"
    aria-label="Типы и группы"
    tabindex="-1"
    onkeydown={handleTreeKeydown}
  >
    {#if loading && types === null}
      <div class="tree-status"><Spinner size="md" /></div>
    {:else if loadError && (types?.length ?? 0) === 0}
      <div class="tree-status tree-status--error">
        Не удалось загрузить группы. Проверьте подключение и повторите.
      </div>
    {:else if isSearchMode && searchResult.roots.length === 0}
      <div class="tree-status tree-status--empty">
        <p class="empty-title">Ничего не найдено</p>
        <p class="empty-body">Проверьте написание или очистите поиск.</p>
      </div>
    {:else if displayRoots.length === 0}
      <div class="tree-status tree-status--empty">
        <p class="empty-title">Типов групп нет</p>
        <p class="empty-body">
          Типы засеваются при запуске приложения. Нажмите «Обновить» — если ничего не появилось,
          перезапустите приложение.
        </p>
      </div>
    {:else}
      {#each displayRoots as root (root.key)}
        <GroupTreeNode
          item={root}
          depth={0}
          expandedKeys={activeExpanded}
          {selectedKey}
          focusedKey={tabbableKey}
          {query}
          {canEditTypes}
          {canMutate}
          actions={nodeActions}
        />
      {/each}
    {/if}
  </div>

  <div class="sr-only" aria-live="polite">{liveMessage}</div>
</div>

<style lang="scss">
  .group-tree-shell {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }

  .toolbar {
    flex: none;
    display: flex;
    align-items: center;
    gap: var(--tr-space-xs);
    padding: var(--tr-space-sm) var(--tr-space-md);
    background: var(--tr-bg);
    border-bottom: 1px solid var(--tr-border);
  }

  .toolbar-search {
    flex: 1 1 auto;
    min-width: 0;

    :global(.input-wrap) {
      width: 100%;
    }
  }

  .tree-body {
    flex: 1 1 auto;
    min-height: 0;
    overflow-y: auto;
  }

  .tree-status {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--tr-space-2xs);
    padding: var(--tr-space-2xl);
    color: var(--tr-text-secondary);
    text-align: center;
  }

  .tree-status--error {
    color: var(--tr-danger-text);
  }

  .tree-status--empty {
    flex-direction: column;
    gap: var(--tr-space-2xs);
  }

  .empty-title {
    margin: 0;
    font-size: var(--tr-font-size-body);
    font-weight: var(--tr-font-weight-body-strong);
    color: var(--tr-text-primary);
  }

  .empty-body {
    margin: 0;
    color: var(--tr-text-secondary);
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }
</style>

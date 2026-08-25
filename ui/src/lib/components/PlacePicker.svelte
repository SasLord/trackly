<script lang="ts">
  // Phase 39 Plan 13 (D-17): единственный контрол выбора места во всём приложении —
  // заменяет LocationAutocomplete.svelte (D-17/удаляется в Плане 21). Механика
  // (portal + dropdownAnchor, open-on-focus, 200ms debounce, таймер очищается в
  // onDestroy — WR-05) унаследована от LocationAutocomplete построчно; панель — своя,
  // древовидная с ленивой подгрузкой узлов (см. 39-UI-SPEC.md §10).
  //
  // В отличие от Dropdown.svelte (D-01/D-02 25-й фазы, «zero data-fetching»,
  // caller передаёт groups/members через props), PlacePicker сам владеет
  // fetch-логикой (places_list_children/places_search/places_get/places_create) —
  // этого требует произвольная вложенность дерева мест (D-01), которую
  // двухуровневый drill-in Dropdown не выражает (см. UI-SPEC §6.2 "Почему
  // Dropdown.svelte не переиспользуется как есть"). Чтобы витрина компонентов
  // (Task 2) могла демонстрировать дерево/поиск на вымышленных данных без
  // реального API-вызова (проектное правило приватности — никаких сид-данных
  // в БД ради витрины), fetch-функции вынесены в необязательные props с
  // дефолтами на apiCall(...) — единственная architecture-level добавка сверх
  // буквального Props-контракта из текста плана.
  import { onDestroy } from 'svelte';
  import { apiCall } from '$lib/api/client';
  import { portal } from '$lib/utils/portal';
  import { dropdownAnchor } from '$lib/utils/dropdownAnchor';
  import { authStore } from '$lib/stores/auth.svelte';
  import Spinner from '$lib/components/Spinner.svelte';
  import Badge from '$lib/components/Badge.svelte';
  import type { PlaceDto, PlaceNewDto, PlacePathDto } from '../../bindings';

  interface Props {
    /** Выбранный place_id (или null — место не выбрано). */
    value: number | null;
    onChange: (placeId: number | null) => void;
    id?: string;
    disabled?: boolean;
    invalid?: boolean;
    /** Инъекция для витрины/тестов — по умолчанию бьёт в реальный API. */
    fetchChildren?: (parentId: number | null) => Promise<PlaceDto[]>;
    fetchSearchResults?: (query: string) => Promise<PlacePathDto[]>;
    fetchOne?: (placeId: number) => Promise<PlaceDto>;
    createPlace?: (place: PlaceNewDto) => Promise<PlaceDto>;
  }

  async function defaultFetchChildren(parentId: number | null): Promise<PlaceDto[]> {
    return apiCall<PlaceDto[]>('places_list_children', { parentId });
  }
  async function defaultFetchSearchResults(query: string): Promise<PlacePathDto[]> {
    return apiCall<PlacePathDto[]>('places_search', { query });
  }
  async function defaultFetchOne(placeId: number): Promise<PlaceDto> {
    return apiCall<PlaceDto>('places_get', { id: placeId });
  }
  async function defaultCreatePlace(place: PlaceNewDto): Promise<PlaceDto> {
    return apiCall<PlaceDto>('places_create', { place });
  }

  const {
    value,
    onChange,
    id,
    disabled = false,
    invalid = false,
    fetchChildren = defaultFetchChildren,
    fetchSearchResults = defaultFetchSearchResults,
    fetchOne = defaultFetchOne,
    createPlace = defaultCreatePlace,
  }: Props = $props();

  /** Минимальный снимок выбранного места — достаточно для отображения поля и
   *  бейджа «Архив»; полный PlaceDto не нужен (search-результаты дают только
   *  full_path/place_id, D-15-исключение решается отдельным fetchOne). */
  interface SelectedPlace {
    id: number;
    full_path: string;
    archived_at_utc: number | null;
  }

  let mode = $state<'closed' | 'tree' | 'search'>('closed');
  let selectedPlace = $state<SelectedPlace | null>(null);
  let queryText = $state('');

  // --- Дерево (режим по фокусу, §10.2) ---
  let rootChildren = $state<PlaceDto[] | null>(null);
  let rootLoading = $state(false);
  let rootLoadError = $state(false);
  let childrenCache = $state<Record<number, PlaceDto[]>>({});
  let loadingChildrenIds = $state<number[]>([]);
  let expandedIds = $state<number[]>([]);
  /** id активной строки; -1 — зарезервированный сентинел строки «Создать…» (D-18). */
  let activeId = $state<number | null>(null);

  // --- Поиск (режим при наборе, §10.3) ---
  let searchQuery = $state('');
  let searchResults = $state<PlacePathDto[] | null>(null);
  let searchLoading = $state(false);
  /** Узел, выделенный в дереве непосредственно перед началом набора — родитель
   *  для строки создания D-18 («если ничего не выделено — корень»). */
  let preSearchActiveNode = $state<PlaceDto | null>(null);
  let searchDebounceTimer: ReturnType<typeof setTimeout> | null = null;

  let liveMessage = $state('');

  let wrapperEl = $state<HTMLDivElement | null>(null);
  let inputEl = $state<HTMLInputElement | null>(null);
  let panelEl = $state<HTMLDivElement | null>(null);

  const uid = $props.id();
  const panelId = `${uid}-panel`;

  // WR-05: тот же класс бага, что и в LocationAutocomplete — debounce-таймер,
  // не отменённый на unmount, дописывает $state уже мёртвого компонента.
  onDestroy(() => {
    if (searchDebounceTimer !== null) clearTimeout(searchDebounceTimer);
  });

  const isAdmin = $derived(authStore.user?.role === 'admin');

  const displayText = $derived(mode === 'search' ? queryText : (selectedPlace?.full_path ?? ''));

  interface VisibleRow {
    place: PlaceDto;
    depth: number;
  }

  const visibleRows = $derived.by<VisibleRow[]>(() => {
    const rows: VisibleRow[] = [];
    function walk(list: PlaceDto[] | null, depth: number) {
      if (!list) return;
      for (const p of list) {
        rows.push({ place: p, depth });
        if (expandedIds.includes(p.id)) {
          walk(childrenCache[p.id] ?? null, depth + 1);
        }
      }
    }
    walk(rootChildren, 0);
    return rows;
  });

  const createParentPath = $derived(preSearchActiveNode?.full_path ?? null);
  const createLabel = $derived(
    createParentPath
      ? `Создать «${searchQuery}» в «${createParentPath}»`
      : `Создать «${searchQuery}» в корне дерева`,
  );

  function isLeafKnown(placeId: number): boolean {
    const kids = childrenCache[placeId];
    return Array.isArray(kids) && kids.length === 0;
  }

  function findNodeById(placeId: number): PlaceDto | null {
    const inRoot = rootChildren?.find((p) => p.id === placeId);
    if (inRoot) return inRoot;
    for (const kids of Object.values(childrenCache)) {
      const hit = kids.find((p) => p.id === placeId);
      if (hit) return hit;
    }
    return null;
  }

  function findParentId(placeId: number): number | null | undefined {
    if (rootChildren?.some((p) => p.id === placeId)) return null;
    for (const [pid, kids] of Object.entries(childrenCache)) {
      if (kids.some((p) => p.id === placeId)) return Number(pid);
    }
    return undefined;
  }

  async function ensureChildrenLoaded(parentId: number | null): Promise<void> {
    if (parentId === null) {
      if (rootChildren !== null) return;
      rootLoading = true;
      rootLoadError = false;
      try {
        rootChildren = await fetchChildren(null);
      } catch {
        rootChildren = [];
        rootLoadError = true;
      } finally {
        rootLoading = false;
      }
      return;
    }
    if (childrenCache[parentId] !== undefined) return;
    loadingChildrenIds = [...loadingChildrenIds, parentId];
    try {
      const kids = await fetchChildren(parentId);
      childrenCache = { ...childrenCache, [parentId]: kids };
    } catch {
      childrenCache = { ...childrenCache, [parentId]: [] };
    } finally {
      loadingChildrenIds = loadingChildrenIds.filter((x) => x !== parentId);
    }
  }

  function expandNode(placeId: number): void {
    if (!expandedIds.includes(placeId)) expandedIds = [...expandedIds, placeId];
    void ensureChildrenLoaded(placeId);
  }

  function collapseNode(placeId: number): void {
    expandedIds = expandedIds.filter((x) => x !== placeId);
  }

  /** По фокусу — раскрыть корни, а если есть текущее значение, раскрыть ветку
   *  до него (§10.2). Архивное текущее значение сервер не вернёт в списке
   *  детей (D-15) — довешиваем его вручную в список родителя (искл. §10.2). */
  async function expandPathToValue(targetId: number): Promise<void> {
    let target: PlaceDto;
    try {
      target = await fetchOne(targetId);
    } catch {
      return;
    }
    const chain: PlaceDto[] = [target];
    let parentId = target.parent_id;
    while (parentId !== null) {
      let parent: PlaceDto;
      try {
        parent = await fetchOne(parentId);
      } catch {
        break;
      }
      chain.unshift(parent);
      parentId = parent.parent_id;
    }
    await ensureChildrenLoaded(null);
    for (let i = 0; i < chain.length - 1; i++) {
      const node = chain[i];
      if (!expandedIds.includes(node.id)) expandedIds = [...expandedIds, node.id];
      await ensureChildrenLoaded(node.id);
    }
    if (target.archived_at_utc !== null) {
      const pid = target.parent_id;
      if (pid === null) {
        if (rootChildren && !rootChildren.some((p) => p.id === target.id)) {
          rootChildren = [...rootChildren, target];
        }
      } else {
        const kids = childrenCache[pid] ?? [];
        if (!kids.some((p) => p.id === target.id)) {
          childrenCache = { ...childrenCache, [pid]: [...kids, target] };
        }
      }
    }
    activeId = target.id;
  }

  async function openTreeMode(): Promise<void> {
    mode = 'tree';
    activeId = null;
    await ensureChildrenLoaded(null);
    if (value !== null) {
      await expandPathToValue(value);
    }
  }

  function handleFocus(): void {
    if (disabled || mode !== 'closed') return;
    void openTreeMode();
  }

  function selectTreeNode(place: PlaceDto): void {
    onChange(place.id);
    selectedPlace = { id: place.id, full_path: place.full_path ?? '', archived_at_utc: place.archived_at_utc };
    mode = 'closed';
    activeId = null;
    inputEl?.blur();
  }

  function selectSearchResult(row: PlacePathDto): void {
    onChange(row.place_id);
    selectedPlace = { id: row.place_id, full_path: row.full_path, archived_at_utc: null };
    mode = 'closed';
    activeId = null;
    searchQuery = '';
    searchResults = null;
    inputEl?.blur();
  }

  function handleChevronClick(e: MouseEvent, place: PlaceDto): void {
    e.preventDefault();
    e.stopPropagation();
    activeId = place.id;
    if (expandedIds.includes(place.id)) {
      collapseNode(place.id);
    } else {
      expandNode(place.id);
    }
  }

  function handleClear(e: MouseEvent): void {
    e.preventDefault();
    onChange(null);
    selectedPlace = null;
    queryText = '';
    mode = 'closed';
  }

  function scheduleSearch(query: string): void {
    if (searchDebounceTimer !== null) clearTimeout(searchDebounceTimer);
    if (query.trim() === '') {
      searchResults = null;
      mode = 'tree';
      return;
    }
    searchDebounceTimer = setTimeout(async () => {
      searchLoading = true;
      try {
        const results = await fetchSearchResults(query);
        searchResults = results.slice(0, 50);
      } catch {
        searchResults = [];
      } finally {
        searchLoading = false;
        liveMessage = `Найдено совпадений: ${searchResults?.length ?? 0}`;
        activeId = searchResults?.length === 0 && isAdmin ? -1 : null;
      }
    }, 200);
  }

  function handleInput(e: Event): void {
    const text = (e.currentTarget as HTMLInputElement).value;
    if (mode !== 'search') {
      preSearchActiveNode = activeId !== null && activeId !== -1 ? findNodeById(activeId) : null;
    }
    mode = 'search';
    queryText = text;
    searchQuery = text;
    scheduleSearch(text);
  }

  function inferKindForQuickCreate(parent: PlaceDto | null): string {
    if (!parent) return 'room';
    switch (parent.kind) {
      case 'territory':
        return 'zone';
      case 'zone':
        return 'building';
      case 'building':
        return 'floor';
      default:
        return 'room';
    }
  }

  async function handleCreate(): Promise<void> {
    const parentId = preSearchActiveNode?.id ?? null;
    const kind = inferKindForQuickCreate(preSearchActiveNode);
    const place: PlaceNewDto = {
      parent_id: parentId,
      kind,
      name: searchQuery,
      level: null,
      is_storage: false,
      sort_order: null,
      notes: null,
    };
    try {
      const created = await createPlace(place);
      // Инвалидируем кэш родителя, чтобы созданный узел появился в дереве при
      // следующем раскрытии.
      if (parentId === null) {
        rootChildren = null;
      } else {
        const rest = { ...childrenCache };
        delete rest[parentId];
        childrenCache = rest;
      }
      selectTreeNode(created);
    } catch {
      // Серверная ошибка (например дубль имени, D-04) — панель остаётся
      // открытой, пользователь может изменить запрос и повторить.
    }
  }

  function scrollActiveIntoView(): void {
    if (activeId === null) return;
    const domId = activeId === -1 ? `${uid}-opt-create` : `${uid}-opt-${activeId}`;
    document.getElementById(domId)?.scrollIntoView({ block: 'nearest' });
  }

  function activeOptionId(): string | undefined {
    if (activeId === null) return undefined;
    return activeId === -1 ? `${uid}-opt-create` : `${uid}-opt-${activeId}`;
  }

  function handleTreeKeydown(e: KeyboardEvent): void {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      const rows = visibleRows;
      if (rows.length === 0) return;
      const idx = rows.findIndex((r) => r.place.id === activeId);
      const next = idx < 0 ? 0 : Math.min(idx + 1, rows.length - 1);
      activeId = rows[next].place.id;
      scrollActiveIntoView();
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      const rows = visibleRows;
      if (rows.length === 0) return;
      const idx = rows.findIndex((r) => r.place.id === activeId);
      const prev = idx < 0 ? 0 : Math.max(idx - 1, 0);
      activeId = rows[prev].place.id;
      scrollActiveIntoView();
    } else if (e.key === 'ArrowRight') {
      e.preventDefault();
      if (activeId === null || activeId === -1) return;
      if (isLeafKnown(activeId)) return;
      if (!expandedIds.includes(activeId)) {
        expandNode(activeId);
      } else {
        const kids = childrenCache[activeId];
        if (kids && kids.length > 0) activeId = kids[0].id;
      }
    } else if (e.key === 'ArrowLeft') {
      e.preventDefault();
      if (activeId === null || activeId === -1) return;
      if (expandedIds.includes(activeId)) {
        collapseNode(activeId);
      } else {
        const parentId = findParentId(activeId);
        if (parentId !== undefined && parentId !== null) activeId = parentId;
      }
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (activeId !== null && activeId !== -1) {
        const node = findNodeById(activeId);
        if (node) selectTreeNode(node);
      }
    } else if (e.key === 'Tab') {
      if (activeId !== null && activeId !== -1) {
        const node = findNodeById(activeId);
        if (node) selectTreeNode(node);
      }
      mode = 'closed';
    }
  }

  function handleSearchKeydown(e: KeyboardEvent): void {
    const list = searchResults ?? [];
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (list.length === 0) return;
      const idx = list.findIndex((r) => r.place_id === activeId);
      const next = idx < 0 ? 0 : (idx + 1) % list.length;
      activeId = list[next].place_id;
      scrollActiveIntoView();
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      if (list.length === 0) return;
      const idx = list.findIndex((r) => r.place_id === activeId);
      const prev = idx < 0 ? list.length - 1 : (idx - 1 + list.length) % list.length;
      activeId = list[prev].place_id;
      scrollActiveIntoView();
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (activeId === -1) {
        void handleCreate();
      } else if (activeId !== null) {
        const row = list.find((r) => r.place_id === activeId);
        if (row) selectSearchResult(row);
      }
    } else if (e.key === 'Tab') {
      if (activeId === -1) {
        void handleCreate();
      } else if (activeId !== null) {
        const row = list.find((r) => r.place_id === activeId);
        if (row) selectSearchResult(row);
      }
      mode = 'closed';
    }
  }

  function handleKeydown(e: KeyboardEvent): void {
    if (disabled) return;
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      // D-12/§10.3: двухступенчатый Escape — первое нажатие возвращает в
      // режим дерева и очищает запрос, второе закрывает панель.
      if (mode === 'search') {
        mode = 'tree';
        queryText = '';
        searchQuery = '';
        searchResults = null;
      } else if (mode === 'tree') {
        mode = 'closed';
      }
      return;
    }
    if (mode === 'closed') {
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        void openTreeMode();
      }
      return;
    }
    if (mode === 'search') {
      handleSearchKeydown(e);
    } else {
      handleTreeKeydown(e);
    }
  }

  function handleClickOutside(e: MouseEvent): void {
    if (mode === 'closed') return;
    const target = e.target as Node;
    const insideWrapper = wrapperEl?.contains(target) ?? false;
    const insidePanel = panelEl?.contains(target) ?? false;
    if (!insideWrapper && !insidePanel) mode = 'closed';
  }

  $effect(() => {
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  });

  // Синхронизация отображаемого текста поля с внешним `value` (place_id) —
  // это не строка, поэтому полный путь надо резолвить отдельным запросом.
  $effect(() => {
    const v = value;
    if (v === null) {
      selectedPlace = null;
      return;
    }
    if (selectedPlace?.id === v) return;
    void (async () => {
      try {
        const dto = await fetchOne(v);
        selectedPlace = { id: dto.id, full_path: dto.full_path ?? '', archived_at_utc: dto.archived_at_utc };
      } catch {
        selectedPlace = { id: v, full_path: `#${v}`, archived_at_utc: null };
      }
    })();
  });

  interface PathSegment {
    text: string;
    isLast: boolean;
  }
  function splitPath(fullPath: string): PathSegment[] {
    const parts = fullPath.split(' / ');
    return parts.map((text, i) => ({ text, isLast: i === parts.length - 1 }));
  }

  interface HighlightPiece {
    text: string;
    matched: boolean;
  }
  function highlightPieces(text: string, query: string): HighlightPiece[] {
    const q = query.trim().toLowerCase();
    if (!q) return [{ text, matched: false }];
    const lower = text.toLowerCase();
    const idx = lower.indexOf(q);
    if (idx === -1) return [{ text, matched: false }];
    const pieces: HighlightPiece[] = [];
    if (idx > 0) pieces.push({ text: text.slice(0, idx), matched: false });
    pieces.push({ text: text.slice(idx, idx + q.length), matched: true });
    if (idx + q.length < text.length) pieces.push({ text: text.slice(idx + q.length), matched: false });
    return pieces;
  }
</script>

<div class="place-picker-wrapper" bind:this={wrapperEl}>
  <div class="field-wrapper">
    <input
      type="text"
      bind:this={inputEl}
      {id}
      class="place-picker-input"
      class:invalid
      {disabled}
      value={displayText}
      placeholder="Выберите место"
      autocomplete="off"
      role="combobox"
      aria-autocomplete="list"
      aria-expanded={mode !== 'closed'}
      aria-controls={panelId}
      aria-activedescendant={activeOptionId()}
      oninput={handleInput}
      onfocus={handleFocus}
      onkeydown={handleKeydown}
    />
    {#if selectedPlace}
      <button
        type="button"
        class="clear-btn"
        aria-label="Очистить место"
        onmousedown={(e) => e.preventDefault()}
        onclick={handleClear}
      >
        ×
      </button>
    {/if}
  </div>

  {#if mode !== 'closed'}
    <div
      class="dropdown--place"
      id={panelId}
      role="tree"
      aria-label={mode === 'tree' ? 'Дерево мест' : 'Результаты поиска места'}
      use:portal
      use:dropdownAnchor={{ anchorEl: inputEl, maxHeight: 320 }}
      bind:this={panelEl}
    >
      {#if mode === 'tree'}
        {#if rootLoading && rootChildren === null}
          <div class="picker-row picker-row--status"><Spinner size="sm" />Загрузка…</div>
        {:else if rootLoadError && (rootChildren?.length ?? 0) === 0}
          <div class="picker-row picker-row--status picker-row--error">
            Не удалось загрузить места. Проверьте подключение и повторите.
          </div>
        {:else if (rootChildren?.length ?? 0) === 0}
          <div class="picker-row picker-row--status">Ничего не найдено</div>
        {:else}
          {#each visibleRows as row (row.place.id)}
            <div
              id={`${uid}-opt-${row.place.id}`}
              class="picker-row"
              class:active={row.place.id === activeId}
              class:selected={row.place.id === selectedPlace?.id}
              role="treeitem"
              tabindex="-1"
              aria-level={row.depth + 1}
              aria-selected={row.place.id === selectedPlace?.id}
              aria-expanded={isLeafKnown(row.place.id) ? undefined : expandedIds.includes(row.place.id)}
              style={`padding-left: calc(var(--tr-space-xs) + ${row.depth} * var(--tr-space-md))`}
              onmousedown={(e) => e.preventDefault()}
              onclick={() => selectTreeNode(row.place)}
              onkeydown={(e) => {
                // Композитный виджет (WAI-ARIA combobox+tree): реальная клавиатурная
                // навигация обрабатывается на поле ввода через aria-activedescendant
                // (handleKeydown выше); строки не входят в tab-order (tabindex="-1").
                // Enter/Space здесь — подстраховка на случай прямого клика по строке
                // с зажатой клавишей (a11y-гейт требует keydown-пару к onclick).
                if (e.key === 'Enter' || e.key === ' ') {
                  e.preventDefault();
                  selectTreeNode(row.place);
                }
              }}
            >
              <span class="picker-chevron-slot">
                {#if !isLeafKnown(row.place.id)}
                  <button
                    type="button"
                    class="picker-chevron"
                    class:expanded={expandedIds.includes(row.place.id)}
                    aria-hidden="true"
                    tabindex="-1"
                    onmousedown={(e) => e.preventDefault()}
                    onclick={(e) => handleChevronClick(e, row.place)}
                  >
                    ›
                  </button>
                {/if}
              </span>
              <span class="picker-name" class:archived={row.place.archived_at_utc !== null}>
                {row.place.name}
              </span>
              {#if row.place.is_storage}
                <Badge variant="accent" appearance="soft" size="sm">Склад</Badge>
              {/if}
              {#if row.place.archived_at_utc !== null}
                <Badge variant="default" appearance="soft" size="sm">Архив</Badge>
              {/if}
            </div>
            {#if expandedIds.includes(row.place.id) && loadingChildrenIds.includes(row.place.id)}
              <div
                class="picker-row picker-row--status"
                style={`padding-left: calc(var(--tr-space-xs) + ${row.depth + 1} * var(--tr-space-md))`}
              >
                <Spinner size="sm" />Загрузка…
              </div>
            {/if}
          {/each}
        {/if}
      {:else if searchLoading}
        <div class="picker-row picker-row--status"><Spinner size="sm" />Загрузка…</div>
      {:else if searchResults && searchResults.length > 0}
        {#each searchResults as row (row.place_id)}
          <div
            id={`${uid}-opt-${row.place_id}`}
            class="picker-row picker-row--flat"
            class:active={row.place_id === activeId}
            role="treeitem"
            tabindex="-1"
            aria-selected={row.place_id === selectedPlace?.id}
            onmousedown={(e) => e.preventDefault()}
            onclick={() => selectSearchResult(row)}
            onkeydown={(e) => {
              if (e.key === 'Enter' || e.key === ' ') {
                e.preventDefault();
                selectSearchResult(row);
              }
            }}
          >
            <span class="picker-path">
              {#each splitPath(row.full_path) as segment, si (si)}
                <span class="seg" class:seg-secondary={!segment.isLast} class:seg-last={segment.isLast}>
                  {#each highlightPieces(segment.text, searchQuery) as piece, pi (pi)}
                    <span class:matched={piece.matched}>{piece.text}</span>
                  {/each}
                </span>
                {#if !segment.isLast}<span class="sep"> / </span>{/if}
              {/each}
            </span>
          </div>
        {/each}
      {:else if searchResults && searchResults.length === 0}
        {#if isAdmin}
          <button
            type="button"
            id={`${uid}-opt-create`}
            class="picker-row picker-row--create"
            class:active={activeId === -1}
            onmousedown={(e) => e.preventDefault()}
            onclick={() => void handleCreate()}
          >
            <svg class="create-icon" width="16" height="16" viewBox="0 0 16 16" fill="none" aria-hidden="true">
              <path d="M8 3v10M3 8h10" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
            </svg>
            {createLabel}
          </button>
        {:else}
          <div class="picker-row picker-row--empty">
            Ничего не найдено. Уточните запрос или обратитесь к администратору, чтобы добавить
            место.
          </div>
        {/if}
      {/if}
    </div>
  {/if}

  <div class="sr-only" aria-live="polite">{liveMessage}</div>
</div>

<style lang="scss">
  .place-picker-wrapper {
    position: relative;
  }

  .field-wrapper {
    position: relative;
  }

  .place-picker-input {
    display: block;
    width: 100%;
    height: 36px;
    padding: 0 32px 0 var(--tr-space-md);
    background: var(--tr-surface-raised);
    color: var(--tr-text-primary);
    border: 1px solid var(--tr-border-strong);
    border-radius: var(--tr-radius-sm);
    font-family: var(--tr-font-family);
    font-size: var(--tr-font-size-body);
    line-height: var(--tr-line-height-body);

    &::placeholder {
      color: var(--tr-text-tertiary);
    }
    &:focus-visible {
      outline: none;
      border-color: var(--tr-accent);
      box-shadow: 0 0 0 3px var(--tr-focus-ring);
    }
    &.invalid {
      border-color: var(--tr-danger);
      box-shadow: 0 0 0 3px var(--tr-danger-ring);
    }
    &:disabled {
      background: var(--tr-surface-sunken);
      color: var(--tr-text-tertiary);
      cursor: not-allowed;
    }
  }

  .clear-btn {
    position: absolute;
    right: 4px;
    top: 50%;
    transform: translateY(-50%);
    width: 28px;
    height: 28px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: none;
    background: transparent;
    border-radius: var(--tr-radius-xs);
    color: var(--tr-text-tertiary);
    font-size: 16px;
    line-height: 1;
    cursor: pointer;

    &:hover {
      background: var(--tr-row-hover);
      color: var(--tr-text-primary);
    }
    &:focus-visible {
      outline: none;
      box-shadow: 0 0 0 3px var(--tr-focus-ring);
    }
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

  // Портированная в <body> панель (use:portal) — вне scoped-дерева компонента,
  // нужен :global(). Namespaced класс .dropdown--place (WR-03) — НЕ переиспользует
  // .dropdown--location/.tr-dropdown-* других контролов.
  :global(.dropdown--place) {
    position: fixed;
    z-index: 1000;
    background: var(--tr-surface-raised);
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-xs);
    box-shadow: var(--tr-elev-2);
    max-height: 320px;
    overflow-y: auto;
  }

  :global(.dropdown--place .picker-row) {
    display: flex;
    align-items: center;
    gap: var(--tr-space-2xs);
    width: 100%;
    height: 32px;
    padding-right: var(--tr-space-md);
    border: none;
    background: transparent;
    color: var(--tr-text-primary);
    font-family: var(--tr-font-family);
    font-size: var(--tr-font-size-body);
    text-align: left;
    cursor: pointer;
    transition: none;
  }

  :global(.dropdown--place .picker-row:hover),
  :global(.dropdown--place .picker-row.active) {
    background: var(--tr-row-hover);
  }

  :global(.dropdown--place .picker-row.selected .picker-name) {
    font-weight: var(--tr-font-weight-body-strong);
  }

  :global(.dropdown--place .picker-row--flat) {
    padding-left: var(--tr-space-md);
  }

  :global(.dropdown--place .picker-row--status) {
    cursor: default;
    gap: var(--tr-space-2xs);
    padding-left: var(--tr-space-md);
    color: var(--tr-text-secondary);
  }

  :global(.dropdown--place .picker-row--error) {
    color: var(--tr-danger-text);
  }

  :global(.dropdown--place .picker-row--empty) {
    cursor: default;
    white-space: normal;
    padding-left: var(--tr-space-md);
    color: var(--tr-text-secondary);
    height: auto;
    min-height: 32px;
    padding-top: var(--tr-space-2xs);
    padding-bottom: var(--tr-space-2xs);
  }

  :global(.dropdown--place .picker-chevron-slot) {
    flex: none;
    width: 16px;
    height: 16px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  :global(.dropdown--place .picker-chevron) {
    width: 16px;
    height: 16px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: none;
    background: transparent;
    padding: 0;
    color: var(--tr-text-tertiary);
    font-size: 13px;
    line-height: 1;
    cursor: pointer;
    transition: none;
  }

  :global(.dropdown--place .picker-chevron.expanded) {
    transform: rotate(90deg);
  }

  :global(.dropdown--place .picker-name) {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  :global(.dropdown--place .picker-name.archived) {
    color: var(--tr-text-tertiary);
  }

  :global(.dropdown--place .picker-path) {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  :global(.dropdown--place .seg-secondary) {
    color: var(--tr-text-secondary);
  }

  :global(.dropdown--place .seg-last) {
    font-weight: var(--tr-font-weight-body-strong);
    color: var(--tr-text-primary);
  }

  :global(.dropdown--place .matched) {
    color: var(--tr-accent-text);
  }

  :global(.dropdown--place .sep) {
    color: var(--tr-text-secondary);
  }

  :global(.dropdown--place .picker-row--create) {
    border-top: 1px solid var(--tr-border);
    color: var(--tr-accent-text);
    padding-left: var(--tr-space-md);
  }

  :global(.dropdown--place .picker-row--create:hover),
  :global(.dropdown--place .picker-row--create.active) {
    background: var(--tr-row-hover);
  }

  :global(.dropdown--place .create-icon) {
    flex: none;
  }
</style>

<script lang="ts">
  // Phase 39 Plan 14 (PLC-01/PLC-02/PLC-06, 39-UI-SPEC.md §8): the left-panel
  // place tree. Fetches the WHOLE tree in one call (`places_list_all` — the
  // plan's own action text, confirmed cheap at real scale: ~300 rows,
  // T-39-14-02) and builds the nested structure client-side; only the
  // per-node content counter (D-25) is fetched lazily, per VISIBLE node, via
  // `places_subtree_stats` — the tree's own structure needs no further
  // round-trips once loaded, so there is no per-branch "expand" fetch (unlike
  // PlacePicker's genuinely lazy children, which this component does not
  // share code with).
  import { onDestroy } from 'svelte';
  import { apiCall } from '$lib/api/client';
  import { authStore } from '$lib/stores/auth.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import Input from '$lib/components/Input.svelte';
  import Checkbox from '$lib/components/Checkbox.svelte';
  import Button from '$lib/components/Button.svelte';
  import Modal from '$lib/components/Modal.svelte';
  import Spinner from '$lib/components/Spinner.svelte';
  import PlaceTreeNode from './PlaceTreeNode.svelte';
  import PlaceFormModal from './PlaceFormModal.svelte';
  import PlaceMoveModal from './PlaceMoveModal.svelte';
  import type { AppError } from '$lib/api/errors';
  import type { PlaceDto, PlacePathDto, SubtreeStatsDto } from '../../bindings';

  interface Props {
    initialSelectedId: number | null;
    onSelect: (_place: PlaceDto | null) => void;
    /** Bumped by the parent (header "Создать место") to force a reload. */
    refreshToken?: number;
    /**
     * Plan 20: an out-of-band selection request from PlacesPage (breadcrumb
     * "выбирает предка в дереве", §9.1). A NEW object (even with the same
     * `id` as the current selection — see the D-14 same-node edge case
     * below) re-fires the effect that applies it, since `$effect` tracks
     * this prop's own object identity through `$props()`, not `id` alone.
     */
    externalSelect?: { id: number; token: number } | null;
    /**
     * Plan 20 (D-14, §11.5): fired from the delete-blocked callout's
     * "Показать содержимое" action, in ADDITION to the normal `onSelect`
     * call `showDeleteBlockedContents` already makes. `onSelect` alone
     * cannot signal "force nested content back on" when the blocked node is
     * ALREADY the selected node (no id change for the parent to key a
     * remount off of) — this callback exists specifically for that edge
     * case, so PlacesPage can force-reset PlaceContents's `onlyHere` state
     * even when the place id does not change.
     */
    onShowBlockedContents?: (_place: PlaceDto) => void;
  }

  const {
    initialSelectedId,
    onSelect,
    refreshToken = 0,
    externalSelect = null,
    onShowBlockedContents,
  }: Props = $props();

  // --- Sibling ordering (D-05) — ported verbatim from
  // trackly-core::domain::places::sibling_cmp / natural_name_cmp. Client-side
  // only per the plan's action text ("do not call the backend for sorting").
  function naturalNameCmp(a: string, b: string): number {
    let ai = 0;
    let bi = 0;
    const isDigit = (c: string) => c >= '0' && c <= '9';
    while (ai < a.length || bi < b.length) {
      const ca = a[ai];
      const cb = b[bi];
      if (ca === undefined && cb === undefined) return 0;
      if (ca === undefined) return -1;
      if (cb === undefined) return 1;
      if (isDigit(ca) && isDigit(cb)) {
        let na = '';
        while (ai < a.length && isDigit(a[ai])) {
          na += a[ai];
          ai++;
        }
        let nb = '';
        while (bi < b.length && isDigit(b[bi])) {
          nb += b[bi];
          bi++;
        }
        const numA = Number(na);
        const numB = Number(nb);
        if (numA !== numB) return numA < numB ? -1 : 1;
      } else {
        let sa = '';
        while (ai < a.length && !isDigit(a[ai])) {
          sa += a[ai];
          ai++;
        }
        let sb = '';
        while (bi < b.length && !isDigit(b[bi])) {
          sb += b[bi];
          bi++;
        }
        if (sa !== sb) return sa < sb ? -1 : 1;
      }
    }
    return 0;
  }

  // quick 260827-rzq: mirrors the fixed Rust `sibling_cmp`
  // (crates/trackly-core/src/domain/places.rs) — every pair goes through the SAME
  // three-stage chain, and each stage explicitly decides `null`-vs-value instead of
  // skipping the stage when only one side has a value. A node WITH a value at a given
  // stage sorts BEFORE a node without one (D-05: manual order wins if set). The
  // previous JS port shared the same bug as the old Rust version (only compared when
  // BOTH sides were non-null) — `Array.prototype.sort` doesn't throw on an
  // inconsistent comparator, so it silently produced an implementation-defined order
  // instead of a visible error.
  function siblingCmp(a: PlaceDto, b: PlaceDto): number {
    if (a.sort_order !== null || b.sort_order !== null) {
      if (a.sort_order !== null && b.sort_order !== null) {
        if (a.sort_order !== b.sort_order) return a.sort_order - b.sort_order;
      } else {
        return a.sort_order !== null ? -1 : 1;
      }
    }
    if (a.level !== null || b.level !== null) {
      if (a.level !== null && b.level !== null) {
        if (a.level !== b.level) return a.level - b.level;
      } else {
        return a.level !== null ? -1 : 1;
      }
    }
    return naturalNameCmp(a.name, b.name);
  }

  const isAdmin = $derived(authStore.user?.role === 'admin');

  // --- UAT gap 5 (2026-08-25): "Места" forgot expanded nodes + selection on
  // navigating away and back. Follows the existing app-wide localStorage
  // pattern (see $lib/stores/theme.svelte.ts: `trackly:`-prefixed key, plain
  // localStorage.getItem/setItem, no shared store abstraction). Only expanded
  // node ids are persisted HERE — selectedId and the "Только здесь" toggle are
  // owned by PlacesPage (which already owns the hash sync / D-14 reset), so
  // there is exactly one localStorage read/write site per piece of state.
  const EXPANDED_STORAGE_KEY = 'trackly:places:expandedIds';

  function readPersistedExpandedIds(): number[] {
    if (typeof window === 'undefined') return [];
    try {
      const raw = localStorage.getItem(EXPANDED_STORAGE_KEY);
      if (!raw) return [];
      const parsed: unknown = JSON.parse(raw);
      if (!Array.isArray(parsed)) return [];
      return parsed.filter((v): v is number => typeof v === 'number' && Number.isInteger(v));
    } catch {
      // Malformed/inaccessible storage — start from a clean expansion state
      // rather than throwing.
      return [];
    }
  }

  // --- Data ---
  let allPlaces = $state<PlaceDto[] | null>(null);
  let loading = $state(false);
  let loadError = $state(false);
  let showArchived = $state(false);

  const placeById = $derived.by(() => {
    const map = new Map<number, PlaceDto>();
    for (const p of allPlaces ?? []) map.set(p.id, p);
    return map;
  });

  const childrenMap = $derived.by(() => {
    const map = new Map<number | null, PlaceDto[]>();
    for (const p of allPlaces ?? []) {
      const key = p.parent_id;
      const arr = map.get(key);
      if (arr) arr.push(p);
      else map.set(key, [p]);
    }
    for (const arr of map.values()) arr.sort(siblingCmp);
    return map;
  });

  const roots = $derived(childrenMap.get(null) ?? []);

  // --- Expansion / selection / roving tabindex ---
  let expandedIds = $state<number[]>(readPersistedExpandedIds());
  let selectedId = $state<number | null>(null);
  let activeId = $state<number | null>(null);

  // Persist on every change (toggle expand/collapse, expandPathTo, pruning
  // below) — plain effect, same shape as theme.svelte.ts's own writes.
  $effect(() => {
    if (typeof window === 'undefined') return;
    try {
      localStorage.setItem(EXPANDED_STORAGE_KEY, JSON.stringify(expandedIds));
    } catch {
      // Storage unavailable/quota exceeded — expansion state simply won't
      // survive navigation; the tree itself still works.
    }
  });

  interface VisibleRow {
    place: PlaceDto;
    depth: number;
    parentId: number | null;
  }

  const visibleNodes = $derived.by<VisibleRow[]>(() => {
    const rows: VisibleRow[] = [];
    function walk(list: PlaceDto[], depth: number, parentId: number | null) {
      for (const p of list) {
        rows.push({ place: p, depth, parentId });
        if (expandedIds.includes(p.id)) {
          walk(childrenMap.get(p.id) ?? [], depth + 1, p.id);
        }
      }
    }
    walk(roots, 0, null);
    return rows;
  });

  function isDescendantOf(ancestorId: number, nodeId: number): boolean {
    let cur = placeById.get(nodeId)?.parent_id ?? null;
    while (cur !== null) {
      if (cur === ancestorId) return true;
      cur = placeById.get(cur)?.parent_id ?? null;
    }
    return false;
  }

  function expandPathTo(id: number): void {
    const toExpand: number[] = [];
    let cur = placeById.get(id)?.parent_id ?? null;
    while (cur !== null) {
      toExpand.push(cur);
      cur = placeById.get(cur)?.parent_id ?? null;
    }
    if (toExpand.length > 0) {
      expandedIds = [...new Set([...expandedIds, ...toExpand])];
    }
  }

  let liveMessage = $state('');
  let firstLoadHandled = false;

  async function loadTree(): Promise<void> {
    loading = true;
    loadError = false;
    try {
      allPlaces = await apiCall<PlaceDto[]>('places_list_all', { includeArchived: showArchived });
    } catch {
      allPlaces = [];
      loadError = true;
    } finally {
      loading = false;
    }
    // UAT gap 5: prune persisted-but-now-stale ids (place deleted since last
    // visit, or restored from a previous session) so they don't accumulate
    // forever — harmless either way (a stale id in expandedIds simply never
    // matches a real node), but this keeps storage tidy. Skipped on a load
    // error, where `allPlaces` was force-set to `[]` and is not real data.
    if (!loadError) {
      expandedIds = expandedIds.filter((id) => placeById.has(id));
    }
    if (!firstLoadHandled) {
      firstLoadHandled = true;
      if (initialSelectedId !== null && placeById.has(initialSelectedId)) {
        expandPathTo(initialSelectedId);
        selectedId = initialSelectedId;
        activeId = initialSelectedId;
        onSelect(placeById.get(initialSelectedId) ?? null);
      } else if (activeId === null) {
        activeId = roots[0]?.id ?? null;
      }
    } else if (selectedId !== null) {
      // Keep the parent's copy of the selected node fresh across every
      // reload (rename/archive/move/"Обновить"/showArchived toggle) — Plan
      // 20's PlaceContents renders place.name/full_path/is_storage/
      // archived_at_utc directly, so a stale reference would silently show
      // pre-mutation data after e.g. a rename.
      const fresh = placeById.get(selectedId) ?? null;
      onSelect(fresh);
      if (fresh === null) {
        selectedId = null;
        activeId = null;
      }
    }
  }

  $effect(() => {
    void showArchived;
    void refreshToken;
    void loadTree();
  });

  // Plan 20: apply an out-of-band selection request (breadcrumb ancestor
  // click). Re-fires whenever the parent passes a NEW object (see the Props
  // doc-comment above) — a fresh object per click, even for the same id, so
  // re-clicking the same ancestor still re-focuses/re-selects it.
  $effect(() => {
    if (externalSelect && placeById.has(externalSelect.id)) {
      const node = placeById.get(externalSelect.id);
      if (node) {
        expandPathTo(node.id);
        handleSelectNode(node);
        focusRow(node.id);
      }
    }
  });

  // --- Content counters (D-25) — lazy per VISIBLE node, cached, never
  // re-fetched once known. Zero-count nodes render no counter at all (§8.2).
  let statsCache = $state<Record<number, number>>({});
  const statsInFlight = new Set<number>();

  $effect(() => {
    for (const row of visibleNodes) {
      const id = row.place.id;
      if (statsCache[id] !== undefined || statsInFlight.has(id)) continue;
      statsInFlight.add(id);
      apiCall<SubtreeStatsDto>('places_subtree_stats', { rootId: id })
        .then((s) => {
          statsCache = { ...statsCache, [id]: s.device_count + s.cartridge_count };
        })
        .catch(() => {
          // Non-critical enrichment — counter simply stays absent on failure.
        })
        .finally(() => {
          statsInFlight.delete(id);
        });
    }
  });

  // --- Search mode (§8.1/§8.5) ---
  let searchQuery = $state('');
  let searchResults = $state<PlacePathDto[] | null>(null);
  let searchLoading = $state(false);
  let searchDebounceTimer: ReturnType<typeof setTimeout> | null = null;
  const isSearchMode = $derived(searchQuery.trim() !== '');

  onDestroy(() => {
    if (searchDebounceTimer !== null) clearTimeout(searchDebounceTimer);
  });

  function scheduleSearch(query: string): void {
    if (searchDebounceTimer !== null) clearTimeout(searchDebounceTimer);
    if (query.trim() === '') {
      searchResults = null;
      return;
    }
    searchDebounceTimer = setTimeout(async () => {
      searchLoading = true;
      try {
        const results = await apiCall<PlacePathDto[]>('places_search', { query });
        searchResults = results;
        activeId = results[0]?.place_id ?? null;
        liveMessage = `Найдено совпадений: ${results.length}`;
      } catch {
        searchResults = [];
        liveMessage = 'Найдено совпадений: 0';
      } finally {
        searchLoading = false;
      }
    }, 200);
  }

  function handleSearchInput(v: string): void {
    searchQuery = v;
    scheduleSearch(v);
  }

  function focusRow(id: number): void {
    activeId = id;
    requestAnimationFrame(() => {
      document.getElementById(`place-tree-row-${id}`)?.focus();
    });
  }

  function handleSelectNode(node: PlaceDto): void {
    selectedId = node.id;
    activeId = node.id;
    onSelect(node);
  }

  function selectSearchResult(row: PlacePathDto): void {
    const node = placeById.get(row.place_id);
    searchQuery = '';
    searchResults = null;
    if (node) {
      expandPathTo(node.id);
      handleSelectNode(node);
      focusRow(node.id);
    }
  }

  // --- Keyboard map (§8.5) ---
  function handleSearchKeydown(e: KeyboardEvent): void {
    const list = searchResults ?? [];
    if (e.key === 'Escape') {
      e.preventDefault();
      searchQuery = '';
      searchResults = null;
      return;
    }
    if (list.length === 0) return;
    const idx = list.findIndex((r) => r.place_id === activeId);
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      const next = idx < 0 ? 0 : Math.min(idx + 1, list.length - 1);
      activeId = list[next].place_id;
      focusRow(activeId);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      const prev = idx < 0 ? 0 : Math.max(idx - 1, 0);
      activeId = list[prev].place_id;
      focusRow(activeId);
    } else if (e.key === 'Home') {
      e.preventDefault();
      activeId = list[0].place_id;
      focusRow(activeId);
    } else if (e.key === 'End') {
      e.preventDefault();
      activeId = list[list.length - 1].place_id;
      focusRow(activeId);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (idx >= 0) selectSearchResult(list[idx]);
    }
  }

  function handleTreeKeydown(e: KeyboardEvent): void {
    if (e.key === 'Escape' && searchQuery !== '') {
      e.preventDefault();
      searchQuery = '';
      searchResults = null;
      return;
    }
    const rows = visibleNodes;
    if (rows.length === 0) return;
    const idx = rows.findIndex((r) => r.place.id === activeId);
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      const next = idx < 0 ? 0 : Math.min(idx + 1, rows.length - 1);
      focusRow(rows[next].place.id);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      const prev = idx < 0 ? 0 : Math.max(idx - 1, 0);
      focusRow(rows[prev].place.id);
    } else if (e.key === 'Home') {
      e.preventDefault();
      focusRow(rows[0].place.id);
    } else if (e.key === 'End') {
      e.preventDefault();
      focusRow(rows[rows.length - 1].place.id);
    } else if (e.key === 'ArrowRight') {
      e.preventDefault();
      if (idx < 0) return;
      const row = rows[idx];
      const hasKids = (childrenMap.get(row.place.id)?.length ?? 0) > 0;
      if (!hasKids) return;
      if (!expandedIds.includes(row.place.id)) {
        expandedIds = [...expandedIds, row.place.id];
      } else {
        const kids = childrenMap.get(row.place.id) ?? [];
        if (kids.length > 0) focusRow(kids[0].id);
      }
    } else if (e.key === 'ArrowLeft') {
      e.preventDefault();
      if (idx < 0) return;
      const row = rows[idx];
      if (expandedIds.includes(row.place.id)) {
        expandedIds = expandedIds.filter((x) => x !== row.place.id);
      } else if (row.parentId !== null) {
        focusRow(row.parentId);
      }
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (idx >= 0) handleSelectNode(rows[idx].place);
    } else if (e.key === 'F2') {
      if (!isAdmin || idx < 0) return;
      e.preventDefault();
      openRename(rows[idx].place);
    }
  }

  function handleContainerKeydown(e: KeyboardEvent): void {
    if (isSearchMode) {
      handleSearchKeydown(e);
    } else {
      handleTreeKeydown(e);
    }
  }

  // --- Mutation modals (mount contract: {#if}-gated, fresh instance per open —
  // deferred-items.md "PlaceFormModal mount contract") ---
  let formModal = $state<{
    mode: 'create' | 'rename';
    place: PlaceDto | null;
    defaultParentId: number | null;
  } | null>(null);
  let moveModal = $state<{ place: PlaceDto; defaultParentId?: number | null } | null>(null);

  function openRename(node: PlaceDto): void {
    formModal = { mode: 'rename', place: node, defaultParentId: null };
  }
  function openCreateChild(node: PlaceDto): void {
    formModal = { mode: 'create', place: null, defaultParentId: node.id };
  }
  function openMove(node: PlaceDto): void {
    moveModal = { place: node };
  }

  function handleFormSaved(saved: PlaceDto): void {
    const wasCreate = formModal?.mode === 'create';
    formModal = null;
    void loadTree().then(() => {
      if (wasCreate) {
        expandPathTo(saved.id);
        handleSelectNode(saved);
        focusRow(saved.id);
      }
    });
  }

  function parentPathOf(fullPath: string | null): string {
    if (!fullPath) return 'корень дерева';
    const parts = fullPath.split(' / ');
    parts.pop();
    return parts.length > 0 ? parts.join(' / ') : 'корень дерева';
  }

  function handleMoved(moved: PlaceDto): void {
    moveModal = null;
    liveMessage = `Место «${moved.name}» перемещено в «${parentPathOf(moved.full_path)}»`;
    pushToast('success', 'Место перемещено');
    void loadTree();
  }

  // --- Delete confirm (§11.5) — no `mode="delete"` variant exists on
  // PlaceFormModal (Plan 19's key-decision), so this is the "minimal inline
  // confirm" the plan's action text explicitly allows as the fallback.
  interface DeleteState {
    place: PlaceDto;
    saving: boolean;
    blockedMessage: string | null;
    serverErr: string | null;
  }
  let deleteState = $state<DeleteState | null>(null);

  function openDelete(node: PlaceDto): void {
    deleteState = { place: node, saving: false, blockedMessage: null, serverErr: null };
  }

  async function confirmDelete(): Promise<void> {
    const current = deleteState;
    if (!current) return;
    const { place } = current;
    deleteState = { ...current, saving: true, serverErr: null };
    try {
      await apiCall<null>('places_delete', { id: place.id, version: place.version });
      pushToast('success', 'Место удалено');
      liveMessage = `Место «${place.name}» удалено`;
      if (selectedId === place.id) {
        selectedId = null;
        onSelect(null);
      }
      deleteState = null;
      void loadTree();
    } catch (e) {
      const err = e as Partial<AppError> | undefined;
      if (err?.code === 'CONFLICT') {
        deleteState = {
          ...current,
          saving: false,
          blockedMessage: err.message ?? 'Место нельзя удалить.',
        };
      } else {
        deleteState = {
          ...current,
          saving: false,
          serverErr: err?.message ?? 'Не удалось удалить место.',
        };
      }
    }
  }

  function showDeleteBlockedContents(): void {
    if (!deleteState) return;
    const node = deleteState.place;
    handleSelectNode(node);
    onShowBlockedContents?.(node);
    deleteState = null;
  }

  function archiveFromDeleteBlocked(): void {
    if (!deleteState) return;
    const node = deleteState.place;
    deleteState = null;
    openArchiveToggle(node);
  }

  // --- Archive / unarchive confirm (§11.4) ---
  interface ArchiveState {
    place: PlaceDto;
    toArchive: boolean;
    saving: boolean;
    serverErr: string | null;
  }
  let archiveState = $state<ArchiveState | null>(null);

  function openArchiveToggle(node: PlaceDto): void {
    archiveState = {
      place: node,
      toArchive: node.archived_at_utc === null,
      saving: false,
      serverErr: null,
    };
  }

  async function confirmArchiveToggle(): Promise<void> {
    const current = archiveState;
    if (!current) return;
    const { place, toArchive } = current;
    archiveState = { ...current, saving: true, serverErr: null };
    try {
      if (toArchive) {
        await apiCall<null>('places_archive', { id: place.id, version: place.version });
        pushToast('success', 'Место архивировано');
        liveMessage = `Место «${place.name}» архивировано`;
      } else {
        await apiCall<null>('places_unarchive', { id: place.id, version: place.version });
        pushToast('success', 'Место возвращено из архива');
        liveMessage = `Место «${place.name}» возвращено из архива`;
      }
      archiveState = null;
      void loadTree();
    } catch (e) {
      const err = e as Partial<AppError> | undefined;
      archiveState = {
        ...current,
        saving: false,
        serverErr: err?.message ?? 'Не удалось изменить статус архивации.',
      };
    }
  }

  // --- Drag-n-drop (§8.4/D-21) — Admin only, "внутрь узла" only, ALWAYS opens
  // PlaceMoveModal (never a silent move — dropping just picks the pre-filled
  // target, confirmation is the same dialog as "Переместить в…").
  //
  // UAT gap 6 (2026-08-25): the original implementation used native HTML5 DnD
  // (draggable/dragstart/dragover/drop/dataTransfer) which WKWebView (macOS
  // Tauri desktop) supports only partially — drop never fired, targets never
  // highlighted, while the same interaction worked fine in a LAN browser tab.
  // Rebuilt on Pointer Events (pointerdown/pointermove/pointerup +
  // setPointerCapture), which WKWebView, WebView2 and browsers all support
  // identically. All state and hit-testing lives HERE (the container) rather
  // than being delegated back up per-row through TreeActions callbacks, since
  // a captured pointer keeps delivering move/up events to the element that
  // captured it regardless of which row the cursor is visually over — the
  // container has to do its own elementFromPoint hit-testing to know which
  // row (or the root dropzone) the pointer currently sits above.
  let draggingId = $state<number | null>(null);
  // UAT прогон 6: pointer-события, в отличие от нативного HTML5 DnD, не рисуют
  // полупрозрачный «призрак» перетаскиваемой строки — браузер делал это сам.
  // Рисуем его вручную: плавающая копия имени, следующая за курсором.
  let dragGhost = $state<{ label: string; x: number; y: number } | null>(null);
  let dragOverId = $state<number | null>(null);
  let overRootDropzone = $state(false);

  // Movement threshold (px) before a pointerdown becomes a drag — below this,
  // pointerup is treated as a plain click (row selection keeps working).
  const DRAG_START_THRESHOLD_PX = 6;

  let pointerDragOriginId: number | null = null;
  let pointerDragPointerId: number | null = null;
  let pointerDragStartX = 0;
  let pointerDragStartY = 0;
  let pointerDragStarted = false;

  function isInvalidDropTargetFrom(originId: number, targetId: number): boolean {
    if (targetId === originId) return true;
    return isDescendantOf(originId, targetId);
  }

  function isInvalidDropTarget(targetId: number): boolean {
    if (draggingId === null) return false;
    return isInvalidDropTargetFrom(draggingId, targetId);
  }

  function resetPointerDrag(): void {
    pointerDragOriginId = null;
    pointerDragPointerId = null;
    pointerDragStarted = false;
    draggingId = null;
    dragOverId = null;
    overRootDropzone = false;
    dragGhost = null;
  }

  function updateDropHitTest(clientX: number, clientY: number): void {
    const el = document.elementFromPoint(clientX, clientY) as HTMLElement | null;
    const rootZone = el?.closest<HTMLElement>('.root-dropzone');
    if (rootZone) {
      overRootDropzone = true;
      dragOverId = null;
      return;
    }
    overRootDropzone = false;
    const rowEl = el?.closest<HTMLElement>('.place-tree-row');
    const rawId = rowEl?.dataset.placeId;
    const id = rawId !== undefined ? Number(rawId) : NaN;
    dragOverId = Number.isFinite(id) ? id : null;
  }

  function handleTreePointerDown(e: PointerEvent): void {
    if (!isAdmin) return;
    if (e.pointerType === 'mouse' && e.button !== 0) return;
    const target = e.target as HTMLElement;
    // Don't start a drag from the chevron or the row's ActionMenu — those
    // have their own click behavior and must stay reachable.
    if (target.closest('.chevron, .row-actions')) return;
    const rowEl = target.closest<HTMLElement>('.place-tree-row');
    const rawId = rowEl?.dataset.placeId;
    const id = rawId !== undefined ? Number(rawId) : NaN;
    if (!Number.isFinite(id)) return;
    pointerDragOriginId = id;
    pointerDragPointerId = e.pointerId;
    pointerDragStartX = e.clientX;
    pointerDragStartY = e.clientY;
    pointerDragStarted = false;
  }

  function handleTreePointerMove(e: PointerEvent): void {
    if (pointerDragOriginId === null || e.pointerId !== pointerDragPointerId) return;
    if (!pointerDragStarted) {
      const dx = e.clientX - pointerDragStartX;
      const dy = e.clientY - pointerDragStartY;
      if (Math.hypot(dx, dy) < DRAG_START_THRESHOLD_PX) return;
      pointerDragStarted = true;
      draggingId = pointerDragOriginId;
      const dragged = placeById.get(pointerDragOriginId);
      dragGhost = {
        label: dragged?.name ?? '',
        x: e.clientX,
        y: e.clientY,
      };
      (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    }
    e.preventDefault();
    if (dragGhost) dragGhost = { ...dragGhost, x: e.clientX, y: e.clientY };
    updateDropHitTest(e.clientX, e.clientY);
  }

  function handleTreePointerUp(e: PointerEvent): void {
    if (pointerDragOriginId === null || e.pointerId !== pointerDragPointerId) return;
    const originId = pointerDragOriginId;
    const started = pointerDragStarted;
    const targetId = dragOverId;
    const droppedOnRoot = overRootDropzone;
    try {
      (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
    } catch {
      // Capture may already have been released (e.g. pointercancel raced us).
    }
    resetPointerDrag();
    if (!started) return; // plain click — row's own onclick handles selection
    const place = placeById.get(originId);
    if (!place) return;
    if (droppedOnRoot) {
      moveModal = { place, defaultParentId: null };
      return;
    }
    if (targetId === null || isInvalidDropTargetFrom(originId, targetId)) return;
    moveModal = { place, defaultParentId: targetId };
  }

  function handleTreePointerCancel(e: PointerEvent): void {
    if (pointerDragOriginId === null || e.pointerId !== pointerDragPointerId) return;
    try {
      (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
    } catch {
      // Already released.
    }
    resetPointerDrag();
  }

  const treeActions = {
    onToggleExpand(id: number) {
      expandedIds = expandedIds.includes(id)
        ? expandedIds.filter((x) => x !== id)
        : [...expandedIds, id];
    },
    onSelect: handleSelectNode,
    onFocusRow(id: number) {
      activeId = id;
    },
    onRename: openRename,
    onCreateChild: openCreateChild,
    onMove: openMove,
    onArchiveToggle: openArchiveToggle,
    onDelete: openDelete,
  };
</script>

<div class="place-tree-shell">
  <!-- UAT gap 4 (2026-08-25): previously one cramped 36px row (search + checkbox +
       button all inline) that read as a squeezed table header. Split into two
       full-width rows — search on its own line, secondary controls below — each
       with its own comfortable padding, still both OUTSIDE `.tree-body` (the
       actual scrollable tree) at the same shell level as before. -->
  <div class="toolbar-search">
    <Input value={searchQuery} placeholder="Поиск места" oninput={handleSearchInput} />
  </div>
  <div class="toolbar-actions">
    <Checkbox checked={showArchived} onchange={(c) => (showArchived = c)}
      >Показывать архивные</Checkbox
    >
    <Button variant="ghost" size="sm" onclick={() => void loadTree()}>Обновить</Button>
  </div>

  <div
    class="tree-body"
    role="tree"
    aria-label="Дерево мест"
    tabindex="-1"
    onkeydown={handleContainerKeydown}
    onpointerdown={handleTreePointerDown}
    onpointermove={handleTreePointerMove}
    onpointerup={handleTreePointerUp}
    onpointercancel={handleTreePointerCancel}
  >
    {#if loading && allPlaces === null}
      <div class="tree-status"><Spinner size="md" /></div>
    {:else if loadError && (allPlaces?.length ?? 0) === 0}
      <div class="tree-status tree-status--error">
        Не удалось загрузить места. Проверьте подключение и повторите.
      </div>
    {:else if isSearchMode}
      {#if searchLoading}
        <div class="tree-status"><Spinner size="sm" />Загрузка…</div>
      {:else if searchResults && searchResults.length > 0}
        {#each searchResults as row (row.place_id)}
          <div
            id={`place-tree-row-${row.place_id}`}
            class="search-row"
            class:active={row.place_id === activeId}
            role="treeitem"
            tabindex={row.place_id === activeId ? 0 : -1}
            aria-selected={row.place_id === selectedId}
            onclick={() => selectSearchResult(row)}
            onkeydown={(e) => {
              if (e.key === 'Enter' || e.key === ' ') {
                e.preventDefault();
                selectSearchResult(row);
              }
            }}
          >
            {row.full_path}
          </div>
        {/each}
      {:else}
        <div class="tree-status">Ничего не найдено</div>
      {/if}
    {:else if roots.length === 0}
      <div class="tree-status tree-status--empty">
        <p class="empty-title">Дерево мест пустое</p>
        <p class="empty-body">
          {#if isAdmin}
            Создайте первое место — территорию или здание. Вложенные места добавляются внутрь уже
            созданных.
          {:else}
            Места ещё не заведены. Обратитесь к администратору.
          {/if}
        </p>
      </div>
    {:else}
      {#each roots as root (root.id)}
        <PlaceTreeNode
          node={root}
          depth={0}
          {childrenMap}
          stats={statsCache}
          {expandedIds}
          {selectedId}
          focusedId={activeId}
          {isAdmin}
          {draggingId}
          {dragOverId}
          {isInvalidDropTarget}
          actions={treeActions}
        />
      {/each}
      {#if draggingId !== null}
        <div
          class="root-dropzone"
          class:drop-valid={overRootDropzone}
          role="button"
          tabindex="-1"
          aria-label="В корень дерева"
        >
          В корень дерева
        </div>
      {/if}
    {/if}
  </div>

  <div class="sr-only" aria-live="polite">{liveMessage}</div>
</div>

{#if dragGhost}
  <div
    class="drag-ghost"
    aria-hidden="true"
    style="transform: translate3d({dragGhost.x + 12}px, {dragGhost.y + 12}px, 0)"
  >
    {dragGhost.label}
  </div>
{/if}

{#if formModal}
  <PlaceFormModal
    mode={formModal.mode}
    place={formModal.place}
    defaultParentId={formModal.defaultParentId}
    onClose={() => (formModal = null)}
    onSaved={handleFormSaved}
  />
{/if}

{#if moveModal}
  <PlaceMoveModal
    place={moveModal.place}
    defaultParentId={moveModal.defaultParentId}
    onClose={() => (moveModal = null)}
    onMoved={handleMoved}
  />
{/if}

{#if deleteState}
  <Modal open={true} title="Удалить место?" onClose={() => (deleteState = null)}>
    {#if deleteState.blockedMessage}
      <div class="blocked-callout">{deleteState.blockedMessage}</div>
    {:else}
      <p class="modal-body-text">Место «{deleteState.place.name}» будет удалено безвозвратно.</p>
      {#if deleteState.serverErr}
        <div class="server-error">{deleteState.serverErr}</div>
      {/if}
    {/if}
    {#snippet footer()}
      {#if deleteState?.blockedMessage}
        <Button variant="ghost" onclick={showDeleteBlockedContents}>Показать содержимое</Button>
        <Button variant="primary" onclick={archiveFromDeleteBlocked}>Архивировать</Button>
      {:else}
        <Button
          variant="secondary"
          onclick={() => (deleteState = null)}
          disabled={deleteState?.saving}
        >
          Отмена
        </Button>
        <Button variant="destructive" loading={deleteState?.saving} onclick={confirmDelete}>
          Удалить
        </Button>
      {/if}
    {/snippet}
  </Modal>
{/if}

{#if archiveState}
  <Modal
    open={true}
    title={archiveState.toArchive ? 'Архивировать место?' : 'Вернуть из архива?'}
    onClose={() => (archiveState = null)}
  >
    <p class="modal-body-text">
      {#if archiveState.toArchive}
        Место «{archiveState.place.name}» скроется из выбора при заполнении форм, но останется в
        дереве, в карточках, в истории и в уже выписанных актах.
      {:else}
        Место «{archiveState.place.name}» снова появится в списках выбора.
      {/if}
    </p>
    {#if archiveState.serverErr}
      <div class="server-error">{archiveState.serverErr}</div>
    {/if}
    {#snippet footer()}
      <Button
        variant="secondary"
        onclick={() => (archiveState = null)}
        disabled={archiveState?.saving}
      >
        Отмена
      </Button>
      <Button variant="primary" loading={archiveState?.saving} onclick={confirmArchiveToggle}>
        {archiveState?.toArchive ? 'Архивировать' : 'Вернуть'}
      </Button>
    {/snippet}
  </Modal>
{/if}

<style lang="scss">
  .place-tree-shell {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }

  .toolbar-search {
    flex: none;
    padding: var(--tr-space-sm) var(--tr-space-md);
    border-bottom: 1px solid var(--tr-border);

    :global(.input-wrap) {
      width: 100%;
    }
  }

  .toolbar-actions {
    flex: none;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--tr-space-sm);
    padding: var(--tr-space-xs) var(--tr-space-md);
    border-bottom: 1px solid var(--tr-border);
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

  .search-row {
    display: flex;
    align-items: center;
    height: 32px;
    padding: 0 var(--tr-space-md);
    color: var(--tr-text-primary);
    cursor: pointer;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;

    &:hover,
    &.active {
      background: var(--tr-row-hover);
    }
  }

  .root-dropzone {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 32px;
    margin: var(--tr-space-2xs) var(--tr-space-md);
    border: 1px dashed var(--tr-border-strong);
    border-radius: var(--tr-radius-sm);
    color: var(--tr-text-secondary);

    &.drop-valid {
      background: var(--tr-accent-soft);
      border-color: var(--tr-accent);
      color: var(--tr-text-primary);
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

  .modal-body-text {
    margin: 0 0 var(--tr-space-md) 0;
    color: var(--tr-text-primary);
  }

  .blocked-callout {
    background: var(--tr-danger-soft);
    border-left: 3px solid var(--tr-danger);
    border-radius: var(--tr-radius-sm);
    padding: var(--tr-space-sm) var(--tr-space-md);
    color: var(--tr-danger-text);
    font-size: var(--tr-font-size-body);
  }

  .server-error {
    padding: var(--tr-space-xs) var(--tr-space-md);
    background: color-mix(in srgb, var(--tr-danger) 10%, transparent);
    border: 1px solid color-mix(in srgb, var(--tr-danger) 30%, transparent);
    border-radius: var(--tr-radius-xs);
    font-size: var(--tr-font-size-body);
    color: var(--tr-danger);
  }

  // Плавающая копия перетаскиваемой строки (UAT прогон 6). pointer-events: none —
  // иначе призрак попадал бы под document.elementFromPoint и ломал hit-test дропа.
  .drag-ghost {
    position: fixed;
    top: 0;
    left: 0;
    z-index: 1000;
    max-width: 260px;
    padding: var(--tr-space-2xs) var(--tr-space-sm);
    border: 1px solid var(--tr-border-strong);
    border-radius: var(--tr-radius-sm);
    background: var(--tr-surface-raised);
    color: var(--tr-text-primary);
    font-size: var(--tr-font-size-body);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    opacity: 0.75;
    pointer-events: none;
    box-shadow: var(--tr-elev-3);
  }
</style>

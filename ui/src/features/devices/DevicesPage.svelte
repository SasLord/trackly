<script lang="ts">
  import { onMount, tick, untrack } from 'svelte';
  import Button from '$lib/components/Button.svelte';
  import PageHeader from '$lib/components/PageHeader.svelte';
  import ActionMenu from '$lib/components/ActionMenu.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { ensureLoaded as ensureStatusesLoaded } from '$lib/stores/deviceStatuses.svelte';
  import { isTauri } from '$lib/stores/transport.svelte';
  import { notifyPlaceContentChanged } from '$lib/stores/placeContentEvents.svelte';
  import { apiCall } from '$lib/api/client';
  import { parseIdFromHash } from '$lib/utils/hashId';
  import DeviceList from './DeviceList.svelte';
  import DeviceFilters from './DeviceFilters.svelte';
  import DeviceFormModal from './DeviceFormModal.svelte';
  import DeviceImportCsvModal from './DeviceImportCsvModal.svelte';
  import DocumentAcceptanceModal from '../acts/DocumentAcceptanceModal.svelte';
  import PdfPreviewModal from '../acts/PdfPreviewModal.svelte';
  import { devices } from './api';
  import {
    GROUP_MEMBERS_PAGE_MAX,
    PORTION_PROBE,
    portionLimitMax,
    PAGE_SIZE,
    deviceListFlags,
    mergeHeadBlock,
    rangeChunks,
    shouldHideAnchorMembers,
    shouldLoadMore,
    shouldShowContainerRows,
    trimPortion,
    type GroupMembersState,
    type ListModeContext,
  } from './deviceListMode';
  import type {
    ContainerRowDto,
    DeviceDto,
    DeviceEntryDto,
    DeviceFilter,
    DeviceGroup,
  } from '../../bindings';

  // ---------------------------------------------------------------------------
  // State
  // ---------------------------------------------------------------------------
  // Строки верхнего уровня плоского списка и поиска: устройства и строки АРМ (D-04).
  let rows = $state<DeviceEntryDto[]>([]);
  let groups = $state<DeviceGroup[]>([]);
  // Блок якорей и АРМ над свёртками (D-05).
  let headRows = $state<DeviceEntryDto[]>([]);
  let total = $state(0);
  let hasMore = $state(false);
  let loadingMore = $state(false);
  let moreError = $state(false);
  // Сколько серверных строк уже получено (rows.length в плоском, число устройств
  // в поиске, groups.length в свёртке): смещение следующей порции.
  let portionOffset = $state(0);
  let scrollEl = $state<HTMLDivElement | null>(null);
  let loading = $state(false);
  let modalOpen = $state(false);
  let editTarget = $state<DeviceDto | null>(null);
  let csvModalOpen = $state(false);

  // Plan 03-05 (DEV-14): intermediate acceptance modal + preview modal state.
  let acceptanceDevice = $state<DeviceDto | null>(null);
  let acceptancePayload = $state<{
    deviceId: number;
    giverName: string;
    receiverName: string;
    dateUtc: number;
    deviceName: string;
  } | null>(null);

  // GAP-8 (39-UAT.md, Прогон 3): cross-section focus from the Places
  // content-row «Перейти к устройству» action — `#/devices?id=…`. Read once
  // (this component is only ever (re)created when the router navigates to
  // /devices, mirroring PlacesPage.svelte's own `initialSelectedId` read-once
  // precedent). When present, grouping starts OFF — the same override
  // `searchActive` already applies to `showGroups` below — so the targeted
  // row is guaranteed to render as a plain, scrollable/highlightable
  // DeviceListRow instead of being hidden inside a collapsed multi-device
  // group.
  const initialFocusId = parseIdFromHash();
  // Первая загрузка при переходе по `#/devices?id=…` догружает порции, пока строка
  // устройства не появится; после этого флаг ставится и не сбрасывается.
  let focusResolved = initialFocusId === null;

  // Filters state (Plan 04).
  let searchQuery = $state('');
  let statusFilter = $state<number | null>(null);
  let grouped = $state(initialFocusId === null);
  let counts = $state<Map<number, number>>(new Map());
  let highlightId = $state<number | null>(initialFocusId);

  // Persisted expansion state: Set of group stable-key strings.
  // DeviceGroupRow reports its key on toggle; keys survive list refreshes.
  let expandedGroups = $state(new Set<string>());

  const searchActive = $derived(searchQuery.trim().length > 0);

  // Список показывает устройства всех типов, включая принтеры (D-18): тип не задан.
  // Раздел «Принтеры» не затронут. group_by_condition: false — DevicesPage схлопывает
  // разные condition в одну группу (ITEM-1). Без флагов якорных групп: флаги
  // дописывает ТОЛЬКО deviceListFlags (ниже).
  const rawFilter = $derived<DeviceFilter>({
    type_id: null,
    place_id: null,
    status_id: statusFilter,
    state: null,
    name_prefix: null,
    include_deleted: false,
    group_by_condition: false,
  });

  // Контекст режима списка (фаза 41.1, D-10): поиск, вкладка статуса и переход
  // «Перейти к устройству» (focusId = highlightId, W3) отключают скрытие членов
  // якорных групп. highlightId нигде не сбрасывается, поэтому focusId отключает
  // скрытие на всё время жизни страницы (осознанно, см. SUMMARY плана 19).
  const listCtx = $derived({ searchActive, statusFilter, focusId: highlightId });
  const hideActive = $derived(shouldHideAnchorMembers(listCtx));

  // Фильтр основного списка: плоский режим или свёртка одинаковых (якоря исключены
  // из свёртки, D-11). Экспорт CSV (exportCsv) флагов НЕ ставит — выгрузка по всем.
  const baseFilter = $derived<DeviceFilter>(
    grouped
      ? { ...rawFilter, ...deviceListFlags('grouped', listCtx) }
      : { ...rawFilter, ...deviceListFlags('flat', listCtx) },
  );

  // K для футера «(+K в составе групп)» (D-34).
  let hiddenMemberCount = $state(0);

  // D-12: раскрытие состава якоря и АРМ. Живёт здесь (а не в строке), по group_id,
  // и переживает перезагрузку списка. Set переприсваивается — иначе нет реактивности.
  let expandedAnchors = $state(new Set<number>());
  let anchorMembers = $state<Record<number, GroupMembersState<DeviceEntryDto>>>({});
  // Вид раскрытой группы: от него зависит источник состава (D-11).
  const groupKinds = new Map<number, 'anchor' | 'container'>();

  // ---------------------------------------------------------------------------
  // Data loading
  // ---------------------------------------------------------------------------
  // Номер поколения refresh(): «последний пришедший» ответ не равен «последнему
  // запрошенному». refresh() зовут $effect (смена фильтра/свёртки), onMount, поиск,
  // onSaved и onDelete, а режим свёртки удвоил запросы (listGrouped + блок якорей), так что
  // медленный ответ старого запроса мог перезаписать список нового. Устаревший ответ
  // отбрасывается ДО записи состояния. Тот же номер охраняет loadMore().
  let refreshSeq = 0;
  // Номер поколения списка: растёт на каждом успешном refresh() (правка, удаление,
  // действие из меню, смена фильтра, WS-инвалидация). Ключ кеша детей раскрытой
  // свёртки (DeviceGroupRow) содержит этот номер, поэтому правка поля вне ключа
  // группировки перечитывает детей. Пишется только в продолжении после await, после
  // охраны refreshSeq: устаревший ответ не должен поднимать номер.
  let listGen = $state(0);
  // То же для перечитывания состава раскрытых групп (reloadExpandedAnchors).
  let anchorsSeq = 0;
  // T-41.1-G29-01: одиночная загрузка состава (loadAnchorMembers) идёт по той же
  // границе и тоже может вернуться не в порядке запросов — быстрый повтор
  // «раскрыть/свернуть/раскрыть» на одной группе даёт два ответа на один ключ.
  // Поколение на группу: пишет только тот ответ, чьё поколение ещё текущее.
  // reloadExpandedAnchors двигает его для всех перечитанных групп, чтобы
  // отставший одиночный ответ не лёг поверх свежего состава.
  const anchorLoadSeq = new Map<number, number>();

  function bumpAnchorLoad(groupId: number): number {
    const next = (anchorLoadSeq.get(groupId) ?? 0) + 1;
    anchorLoadSeq.set(groupId, next);
    return next;
  }

  /** Снимок режима на момент запроса: порции одного списка не смешивают фильтры. */
  interface Mode {
    searchActive: boolean;
    grouped: boolean;
    query: string;
    base: DeviceFilter;
    raw: DeviceFilter;
    ctx: ListModeContext;
  }

  function snapMode(): Mode {
    return {
      searchActive,
      grouped,
      query: searchQuery,
      base: baseFilter,
      raw: rawFilter,
      ctx: listCtx,
    };
  }

  interface Portion {
    rows: DeviceEntryDto[];
    groups: DeviceGroup[];
    total: number;
    hasMore: boolean;
    hiddenMemberCount: number;
    /** Сколько серверных строк в порции (смещение следующей). */
    step: number;
  }

  interface ListSnapshot extends Portion {
    headRows: DeviceEntryDto[];
    truncated: boolean;
    portionOffset: number;
  }

  const asDevice = (device: DeviceDto): DeviceEntryDto => ({ kind: 'device', device });

  /** Одна порция основного списка по режиму; ничего не записывает. */
  async function fetchPortion(mode: Mode, offset: number, limit: number): Promise<Portion> {
    if (mode.searchActive) {
      // Search mode — FTS5 (overrides grouping).
      const resp = await devices.search(mode.query, { offset, limit });
      return {
        rows: resp.items.map(asDevice),
        groups: [],
        total: resp.total,
        hasMore: offset + resp.items.length < resp.total,
        hiddenMemberCount: 0,
        step: resp.items.length,
      };
    }
    if (mode.grouped) {
      // Grouped mode — show non-unique devices as collapsible groups. Якоря и АРМ не
      // участвуют в свёртке (exclude_anchors) и выводятся блоком над ней (D-05).
      const grouping = await devices.listGrouped(mode.base, {
        offset,
        limit: limit + PORTION_PROBE,
      });
      const cut = trimPortion(grouping, limit);
      return {
        rows: [],
        groups: cut.rows,
        total: 0,
        hasMore: cut.hasMore,
        hiddenMemberCount: 0,
        step: cut.rows.length,
      };
    }
    // Flat list mode: устройства и строки АРМ вперемешку по общему порядку сервера.
    const resp = await devices.listEntries(mode.base, { offset, limit });
    return {
      rows: resp.rows,
      groups: [],
      total: resp.total,
      hasMore: resp.has_more,
      hiddenMemberCount: resp.hidden_member_count ?? 0,
      step: resp.rows.length,
    };
  }

  /** mergeHeadBlock требует `id`: у строки АРМ порядковым ключом служит group_id. */
  function mergeHead(anchors: DeviceDto[], containers: ContainerRowDto[]): DeviceEntryDto[] {
    return mergeHeadBlock(
      anchors,
      containers.map((c) => ({ id: c.group_id, name: c.name, row: c })),
    ).map(
      (r): DeviceEntryDto =>
        r.kind === 'device' ? r : { kind: 'container', container: r.container.row },
    );
  }

  /** Блок над свёртками: все якоря (страницами по 200) и АРМ (D-05). */
  async function fetchHead(
    mode: Mode,
  ): Promise<{ rows: DeviceEntryDto[]; hidden: number; truncated: boolean }> {
    const anchors: DeviceDto[] = [];
    for (;;) {
      const resp = await devices.list(
        { ...mode.raw, ...deviceListFlags('anchors', mode.ctx) },
        { offset: anchors.length, limit: GROUP_MEMBERS_PAGE_MAX },
      );
      anchors.push(...resp.items);
      if (resp.items.length === 0 || anchors.length >= resp.total) break;
    }
    if (!shouldShowContainerRows(mode.ctx)) {
      return { rows: mergeHead(anchors, []), hidden: 0, truncated: false };
    }
    const conts = await devices.listContainers({
      ...mode.raw,
      ...deviceListFlags('containers', mode.ctx),
    });
    return {
      rows: mergeHead(anchors, conts.containers),
      hidden: conts.hidden_member_count,
      truncated: conts.truncated,
    };
  }

  /** АРМ, совпавшие с поисковым запросом по имени (D-14). */
  async function fetchSearchContainers(mode: Mode): Promise<DeviceEntryDto[]> {
    if (!shouldShowContainerRows(mode.ctx)) return [];
    const conts = await devices.listContainers(
      { ...mode.raw, ...deviceListFlags('containers', mode.ctx) },
      mode.query,
    );
    return conts.containers.map((container) => ({ kind: 'container', container }));
  }

  /**
   * Только запросы, без записи состояния: запись делает refresh() после проверки
   * поколения. `count` — сколько серверных строк перечитать (чанками по 200).
   */
  async function fetchList(
    mode: Mode,
    count: number,
    focusDeviceId: number | null,
  ): Promise<ListSnapshot> {
    const acc: ListSnapshot = {
      rows: [],
      groups: [],
      total: 0,
      hasMore: false,
      hiddenMemberCount: 0,
      step: 0,
      headRows: [],
      truncated: false,
      portionOffset: 0,
    };
    const take = (p: Portion) => {
      acc.rows = [...acc.rows, ...p.rows];
      acc.groups = [...acc.groups, ...p.groups];
      acc.total = p.total;
      acc.hasMore = p.hasMore;
      acc.hiddenMemberCount = p.hiddenMemberCount;
      acc.portionOffset += p.step;
    };
    for (const chunk of rangeChunks(count, portionLimitMax(mode.grouped))) {
      take(await fetchPortion(mode, acc.portionOffset, chunk.limit));
      if (!acc.hasMore) break;
    }
    // Переход «Перейти к устройству»: догружаем порции, пока строка не появится.
    if (focusDeviceId !== null) {
      const present = () =>
        acc.rows.some((r) => r.kind === 'device' && r.device.id === focusDeviceId);
      while (!present() && acc.hasMore) {
        take(await fetchPortion(mode, acc.portionOffset, portionLimitMax(mode.grouped)));
      }
    }
    if (mode.searchActive) {
      const found = await fetchSearchContainers(mode);
      acc.rows = [...found, ...acc.rows];
      acc.total += found.length;
    } else if (mode.grouped) {
      const head = await fetchHead(mode);
      acc.headRows = head.rows;
      acc.hiddenMemberCount = head.hidden;
      acc.truncated = head.truncated;
    }
    return acc;
  }

  async function refresh(opts: { reset?: boolean } = {}) {
    const seq = ++refreshSeq;
    const reset = opts.reset === true;
    const count = reset ? PAGE_SIZE : Math.max(PAGE_SIZE, portionOffset);
    const focusId = focusResolved ? null : highlightId;
    loading = true;
    // Порция в полёте принадлежит прошлому поколению: её ответ отбросит охрана.
    loadingMore = false;
    if (reset) moreError = false;
    try {
      const snap = await fetchList(snapMode(), count, focusId);
      if (seq !== refreshSeq) return;
      rows = snap.rows;
      groups = snap.groups;
      headRows = snap.headRows;
      total = snap.total;
      hasMore = snap.hasMore;
      hiddenMemberCount = snap.hiddenMemberCount;
      portionOffset = snap.portionOffset;
      if (focusId !== null) focusResolved = true;
      listGen += 1;
      if (snap.truncated) {
        pushToast('error', 'Показаны не все АРМ: список ограничен 1000 записей.');
      }
      void reloadExpandedAnchors();
      // Сброс прокрутки после отрисовки новой первой порции (без await: охрана
      // поколения в refresh() идёт после последнего await, см. check-device-list-mode).
      if (reset && focusId === null) void tick().then(() => scrollEl?.scrollTo({ top: 0 }));
    } catch (e: unknown) {
      if (seq !== refreshSeq) return;
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось загрузить устройства';
      pushToast('error', msg);
    } finally {
      if (seq === refreshSeq) loading = false;
    }
  }

  /** Следующая порция (D-07): одна в полёте, ошибка без тоста и без автоповторов. */
  async function loadMore() {
    if (!shouldLoadMore({ hasMore, loadingMore, moreError, loading, remainingPx: 0 })) return;
    const seq = refreshSeq;
    const mode = snapMode();
    const offset = portionOffset;
    loadingMore = true;
    try {
      const p = await fetchPortion(mode, offset, PAGE_SIZE);
      if (seq !== refreshSeq) return;
      const extra = mode.searchActive ? rows.filter((r) => r.kind === 'container').length : 0;
      rows = [...rows, ...p.rows];
      groups = [...groups, ...p.groups];
      if (!mode.grouped) total = p.total + extra;
      hasMore = p.hasMore;
      portionOffset = offset + p.step;
    } catch {
      if (seq !== refreshSeq) return;
      moreError = true;
    } finally {
      if (seq === refreshSeq) loadingMore = false;
    }
  }

  function retryMore() {
    moreError = false;
    void loadMore();
  }

  // ---------------------------------------------------------------------------
  // Раскрытие состава группы (D-11, D-12)
  // ---------------------------------------------------------------------------
  /** Страница состава: АРМ — прямые дети, якорная группа — плоский список членов. */
  async function fetchMembers(
    groupId: number,
    kind: 'anchor' | 'container',
    offset: number,
    limit: number,
  ): Promise<{ entries: DeviceEntryDto[]; total: number }> {
    if (kind === 'container') {
      const resp = await devices.groupChildren(groupId, { offset, limit });
      return { entries: resp.rows, total: resp.total };
    }
    const resp = await devices.list(
      {
        type_id: null,
        place_id: null,
        status_id: null,
        state: null,
        name_prefix: null,
        include_deleted: false,
        group_by_condition: false,
        ...deviceListFlags('members', listCtx),
        member_of_group_id: groupId,
      },
      { offset, limit },
    );
    return { entries: resp.items.map(asDevice), total: resp.total };
  }

  async function fetchMembersState(
    groupId: number,
    kind: 'anchor' | 'container',
    loadedAll: boolean,
  ): Promise<GroupMembersState<DeviceEntryDto>> {
    const first = await fetchMembers(groupId, kind, 0, GROUP_MEMBERS_PAGE_MAX);
    const entries = [...first.entries];
    let totalCount = first.total;
    // Перечитывание уже догруженной группы берёт весь состав (D-10).
    while (loadedAll && entries.length < totalCount) {
      const next = await fetchMembers(groupId, kind, entries.length, GROUP_MEMBERS_PAGE_MAX);
      if (next.entries.length === 0) break;
      entries.push(...next.entries);
      totalCount = next.total;
    }
    return { loading: false, entries, total: totalCount, loadedAll, loadingRest: false };
  }

  /** Ошибка загрузки состава: тост и сворачивание строки назад. */
  function failAnchor(groupIds: number[]) {
    pushToast('error', 'Не удалось загрузить состав группы. Раскройте строку ещё раз.');
    const next = new Set(expandedAnchors);
    const cache = { ...anchorMembers };
    for (const id of groupIds) {
      next.delete(id);
      delete cache[id];
    }
    expandedAnchors = next;
    anchorMembers = cache;
  }

  async function loadAnchorMembers(groupId: number, kind: 'anchor' | 'container') {
    // T-41.1-G29-01: поколения снимаются ДО await; ответ пишется, только если
    // с тех пор не стартовали ни новая загрузка этой группы, ни перечитывание
    // всех раскрытых, и группа всё ещё раскрыта.
    const myGen = bumpAnchorLoad(groupId);
    const aSeq = anchorsSeq;
    const current = () =>
      anchorLoadSeq.get(groupId) === myGen && aSeq === anchorsSeq && expandedAnchors.has(groupId);
    anchorMembers = {
      ...anchorMembers,
      [groupId]: { loading: true, entries: [], total: 0, loadedAll: false, loadingRest: false },
    };
    try {
      const state = await fetchMembersState(groupId, kind, false);
      if (!current()) return;
      anchorMembers = { ...anchorMembers, [groupId]: state };
    } catch {
      if (!current()) return;
      failAnchor([groupId]);
    }
  }

  /**
   * После перезагрузки списка состав раскрытых групп перечитывается; кеш свёрнутых
   * отбрасывается (иначе при следующем раскрытии он был бы устаревшим). Старый
   * состав остаётся на экране до прихода нового: строка не мигает «Загрузкой».
   */
  async function reloadExpandedAnchors() {
    const aSeq = ++anchorsSeq;
    const ids = [...expandedAnchors];
    // T-41.1-G29-01: одиночные загрузки этих групп, стартовавшие раньше,
    // становятся устаревшими — их ответы уже не лягут поверх нашего.
    for (const id of ids) bumpAnchorLoad(id);
    const results = await Promise.all(
      ids.map((id) =>
        fetchMembersState(
          id,
          groupKinds.get(id) ?? 'anchor',
          anchorMembers[id]?.loadedAll ?? false,
        ).then(
          (state) => [id, state] as const,
          () => [id, null] as const,
        ),
      ),
    );
    if (aSeq !== anchorsSeq) return;
    const fresh: Record<number, GroupMembersState<DeviceEntryDto>> = {};
    const failed: number[] = [];
    for (const [id, state] of results) {
      if (state) fresh[id] = state;
      else failed.push(id);
    }
    // Группа, раскрытая, пока шла перезагрузка, не теряет свою запись (иначе она
    // осталась бы с «Загрузкой…» навсегда).
    for (const id of expandedAnchors) {
      if (!(id in fresh) && !failed.includes(id) && anchorMembers[id]) {
        fresh[id] = anchorMembers[id];
      }
    }
    anchorMembers = fresh;
    if (failed.length > 0) failAnchor(failed);
  }

  function toggleGroup(groupId: number, kind: 'anchor' | 'container') {
    groupKinds.set(groupId, kind);
    const next = new Set(expandedAnchors);
    if (next.has(groupId)) {
      next.delete(groupId);
      expandedAnchors = next;
      return;
    }
    next.add(groupId);
    expandedAnchors = next;
    if (!anchorMembers[groupId]) void loadAnchorMembers(groupId, kind);
  }

  /** id устройства-якоря группы: нужен, чтобы вернуть фокус на шеврон якорной строки. */
  function anchorDeviceId(groupId: number): number | null {
    const entries = [
      ...rows,
      ...headRows,
      ...Object.values(anchorMembers).flatMap((m) => m.entries),
    ];
    for (const e of entries) {
      if (e.kind === 'device' && e.device.anchor_group_id === groupId) return e.device.id;
    }
    for (const g of groups) if (g.repr.anchor_group_id === groupId) return g.repr.id;
    return null;
  }

  /** Догрузка остатка состава (D-09): страницами по 200 до актуального total. */
  async function loadRest(groupId: number) {
    const st = anchorMembers[groupId];
    if (!st || st.loadingRest) return;
    const kind = groupKinds.get(groupId) ?? 'anchor';
    const myGen = bumpAnchorLoad(groupId);
    const aSeq = anchorsSeq;
    // Раскрытость не проверяется: свёрнутая группа может спокойно принять состав в кеш.
    const current = () => anchorLoadSeq.get(groupId) === myGen && aSeq === anchorsSeq;
    anchorMembers = { ...anchorMembers, [groupId]: { ...st, loadingRest: true } };
    try {
      const entries = [...st.entries];
      let totalCount = st.total;
      while (entries.length < totalCount) {
        const next = await fetchMembers(groupId, kind, entries.length, GROUP_MEMBERS_PAGE_MAX);
        if (!current()) return;
        if (next.entries.length === 0) break;
        entries.push(...next.entries);
        totalCount = next.total;
      }
      anchorMembers = {
        ...anchorMembers,
        [groupId]: {
          loading: false,
          entries,
          total: totalCount,
          loadedAll: true,
          loadingRest: false,
        },
      };
      await tick();
      const rowId =
        kind === 'container' ? `group-row-${groupId}` : `device-row-${anchorDeviceId(groupId)}`;
      document
        .getElementById(rowId)
        ?.closest('tr')
        ?.querySelector<HTMLElement>('button.tr-row-chevron')
        ?.focus({ preventScroll: true });
    } catch {
      if (!current()) return;
      pushToast('error', 'Не удалось загрузить весь состав группы. Повторите попытку.');
      const cur = anchorMembers[groupId] ?? st;
      anchorMembers = { ...anchorMembers, [groupId]: { ...cur, loadingRest: false } };
    }
  }

  async function refreshCounts() {
    try {
      const arr = await devices.statusCounts();
      counts = new Map(arr.map((x) => [x.status_id, Number(x.count)]));
    } catch {
      // Non-fatal — counters stay 0.
    }
  }

  // Re-run refresh when filter/grouping changes. refresh() читает состояние страницы
  // до первого await (portionOffset и др.): без untrack эффект подписался бы на них
  // и перезапускался на каждой порции.
  $effect(() => {
    // Access reactive deps: statusFilter, grouped — triggers re-run.
    // searchQuery is handled separately via debounce in DeviceFilters.
    void statusFilter;
    void grouped;
    untrack(() => {
      void refresh({ reset: true });
      void refreshCounts();
    });
  });

  onMount(() => {
    void ensureStatusesLoaded();
    void refresh({ reset: true });
    void refreshCounts();
  });

  // ---------------------------------------------------------------------------
  // Filter handlers
  // ---------------------------------------------------------------------------
  function handleSearchChange(q: string) {
    searchQuery = q;
    void refresh({ reset: true });
    if (!q) refreshCounts();
  }

  function handleStatusChange(s: number | null) {
    statusFilter = s;
  }

  function handleGroupedChange(g: boolean) {
    grouped = g;
  }

  // ---------------------------------------------------------------------------
  // Modal handlers
  // ---------------------------------------------------------------------------
  function openCreate() {
    editTarget = null;
    modalOpen = true;
  }

  function openEdit(d: DeviceDto) {
    editTarget = d;
    modalOpen = true;
  }

  function onSaved(result?: { typeId: number; placeId: number | null }) {
    // WARNING-1 (audit 2026-09-17, D-14/D-15): third and final write-site —
    // D-14 names both lists literally («Устройства»/«Картриджи»), symmetric
    // to CartridgesPage.handleFormSuccess (Task 1 of this plan). The old
    // place comes from `editTarget` (captured by openEdit BEFORE the form
    // opened, null on create — same role as CartridgesPage's separately
    // named `formModalTarget`); it is not reset by onSaved, only by the
    // next openEdit/openCreate call, so it is still valid here.
    const oldPlaceId = editTarget?.place_id ?? null;
    const newPlaceId = result?.placeId ?? null;
    const changedPlaceIds = Array.from(
      new Set([oldPlaceId, newPlaceId].filter((id): id is number => id !== null)),
    );
    if (changedPlaceIds.length > 0) notifyPlaceContentChanged(changedPlaceIds);

    modalOpen = false;
    refresh();
    refreshCounts();
  }

  // -------------------------------------------------------------------------
  // DEV-14 «Печать документа приёма» flow
  // -------------------------------------------------------------------------
  function handlePrintAcceptance(d: DeviceDto) {
    acceptanceDevice = d;
  }

  function handleAcceptanceSubmit(payload: {
    deviceId: number;
    giverName: string;
    receiverName: string;
    dateUtc: number;
  }) {
    // Запомним имя устройства для подсказки имени файла, затем переключаем модалки.
    const name = acceptanceDevice?.name ?? `dev-${payload.deviceId}`;
    acceptanceDevice = null;
    acceptancePayload = { ...payload, deviceName: name };
  }

  // ---------------------------------------------------------------------------
  // Export CSV handler
  // ---------------------------------------------------------------------------
  async function exportCsv() {
    try {
      const csvContent = await devices.exportCsv({
        type_id: null,
        place_id: null,
        status_id: statusFilter,
        state: null,
        name_prefix: null,
        include_deleted: false,
        group_by_condition: false,
      });

      if (isTauri) {
        const { save: saveDialog } = await import('@tauri-apps/plugin-dialog');
        const today = new Date().toISOString().slice(0, 10);
        const defaultPath = `устройства_${today}.csv`;
        const savePath = await saveDialog({
          defaultPath,
          filters: [{ name: 'CSV', extensions: ['csv'] }],
        });
        if (!savePath) return;

        await apiCall<void>('write_file_bytes', { path: savePath, content: csvContent });

        // Count devices in current response for toast message.
        const count = csvContent.split('\n').filter((l) => l.trim().length > 0).length - 1; // subtract header row
        pushToast('success', `Экспортировано ${Math.max(0, count)} устройств.`);
      } else {
        // Browser fallback: trigger <a download>.
        const blob = new Blob([csvContent], { type: 'text/csv;charset=utf-8' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        const today = new Date().toISOString().slice(0, 10);
        a.href = url;
        a.download = `устройства_${today}.csv`;
        a.click();
        URL.revokeObjectURL(url);
      }
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось экспортировать';
      pushToast('error', msg);
    }
  }
</script>

<div class="devices-page">
  <PageHeader title="Устройства">
    {#snippet actions()}
      <div class="actions-inline">
        <Button variant="secondary" onclick={() => (csvModalOpen = true)}>Импорт CSV</Button>
        <Button variant="secondary" onclick={exportCsv}>Экспорт CSV</Button>
      </div>
      <div class="actions-kebab">
        <ActionMenu label="Импорт и экспорт">
          <button type="button" role="menuitem" onclick={() => (csvModalOpen = true)}
            >Импорт CSV</button
          >
          <button type="button" role="menuitem" onclick={exportCsv}>Экспорт CSV</button>
        </ActionMenu>
      </div>
      <Button variant="primary" onclick={openCreate}>+ Добавить устройство</Button>
    {/snippet}
  </PageHeader>

  <div class="page-content">
    <DeviceFilters
      {searchQuery}
      {statusFilter}
      {grouped}
      {counts}
      onSearchChange={handleSearchChange}
      onStatusChange={handleStatusChange}
      onGroupedChange={handleGroupedChange}
    />

    <DeviceList
      {listGen}
      {rows}
      {groups}
      {total}
      {loading}
      {grouped}
      {searchActive}
      {expandedGroups}
      {highlightId}
      {headRows}
      {loadingMore}
      {moreError}
      onNearEnd={loadMore}
      onRetryMore={retryMore}
      bind:scrollEl
      onLoadRest={loadRest}
      {expandedAnchors}
      {anchorMembers}
      onToggleGroup={toggleGroup}
      {hiddenMemberCount}
      {hideActive}
      onExpandToggle={(key, isExpanded) => {
        if (isExpanded) {
          expandedGroups.add(key);
        } else {
          expandedGroups.delete(key);
        }
        // Trigger reactivity: reassign to a new Set so Svelte detects the change.
        expandedGroups = new Set(expandedGroups);
      }}
      onEdit={openEdit}
      onDelete={() => {
        refresh();
        refreshCounts();
      }}
      onPrintAcceptance={handlePrintAcceptance}
      showStatus={statusFilter === null}
    />
  </div>
</div>

<DeviceFormModal
  open={modalOpen}
  target={editTarget}
  onClose={() => (modalOpen = false)}
  {onSaved}
/>

<DeviceImportCsvModal
  open={csvModalOpen}
  onClose={() => (csvModalOpen = false)}
  onImported={(placeIds: number[]) => {
    if (placeIds.length > 0) notifyPlaceContentChanged(placeIds);
    csvModalOpen = false;
    refresh();
    refreshCounts();
    pushToast('success', 'Импорт завершён');
  }}
/>

<!-- DEV-14 (Plan 03-05) — intermediate modal collects giver/receiver/date,
     then opens PdfPreviewModal in mode='acceptance'. -->
<DocumentAcceptanceModal
  open={acceptanceDevice !== null}
  device={acceptanceDevice}
  onClose={() => (acceptanceDevice = null)}
  onSubmit={handleAcceptanceSubmit}
/>

<PdfPreviewModal
  open={acceptancePayload !== null}
  actId={null}
  title={acceptancePayload
    ? `Печать документа приёма: ${acceptancePayload.deviceName}`
    : 'Печать документа приёма'}
  mode="acceptance"
  {acceptancePayload}
  onClose={() => (acceptancePayload = null)}
/>

<style lang="scss">
  @use '../../styles/_breakpoints' as bp;

  .devices-page {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  // The section itself must NOT scroll — the table scrolls internally (fillHeight),
  // filling to the bottom of the window with a symmetric 16px margin. Matches the
  // other sections' "framed table fills the panel" model.
  .page-content {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-sm);
    overflow: hidden;
    padding: var(--tr-space-md);
  }

  // DeviceList's root is the framed <Table fillHeight> — let it grow to fill the
  // remaining height below the filter bar and scroll its own body.
  .page-content > :global(.tr-table-framed) {
    flex: 1;
    min-height: 0;
  }

  .actions-inline {
    display: flex;
    gap: 8px;
  }

  .actions-kebab {
    display: none;
  }

  @media (max-width: (bp.$bp-md - 1px)) {
    .actions-inline {
      display: none;
    }
    .actions-kebab {
      display: inline-flex;
    }
  }
</style>

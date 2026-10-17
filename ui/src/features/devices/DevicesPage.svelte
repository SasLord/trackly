<script lang="ts">
  import { onMount } from 'svelte';
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
    deviceListFlags,
    shouldHideAnchorMembers,
    type AnchorMembersState,
  } from './deviceListMode';
  import type { DeviceDto, DeviceFilter, DeviceGroup, Pagination } from '../../bindings';

  // ---------------------------------------------------------------------------
  // State
  // ---------------------------------------------------------------------------
  let items = $state<DeviceDto[]>([]);
  let groups = $state<DeviceGroup[]>([]);
  let total = $state(0);
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

  // Filters state (Plan 04).
  let searchQuery = $state('');
  let statusFilter = $state<number | null>(null);
  let grouped = $state(initialFocusId === null);
  let counts = $state<Map<number, number>>(new Map());
  let highlightId = $state<number | null>(initialFocusId);

  // Persisted expansion state: Set of group stable-key strings.
  // DeviceGroupRow reports its key on toggle; keys survive list refreshes.
  let expandedGroups = $state(new Set<string>());

  const pagination = $state<Pagination>({ offset: 0, limit: 50 });

  const searchActive = $derived(searchQuery.trim().length > 0);

  // type_id=1 ("Устройство") hardcoded — /devices section shows only Устройства.
  // group_by_condition: false — DevicesPage схлопывает разные condition в одну группу (ITEM-1).
  // Без флагов якорных групп: флаги дописывает ТОЛЬКО deviceListFlags (ниже).
  const rawFilter = $derived<DeviceFilter>({
    type_id: 1,
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

  // D-11/D-34: якоря отдельным вызовом рядом со свёрткой; K для футера плоского списка.
  let anchorItems = $state<DeviceDto[]>([]);
  let hiddenMemberCount = $state(0);

  // D-12: раскрытие состава якоря. Живёт здесь (а не в строке), по group_id,
  // и переживает перезагрузку списка. Set переприсваивается — иначе нет реактивности.
  let expandedAnchors = $state(new Set<number>());
  let anchorMembers = $state<Record<number, AnchorMembersState>>({});
  const ANCHOR_MEMBERS_LIMIT = 200;

  // ---------------------------------------------------------------------------
  // Data loading
  // ---------------------------------------------------------------------------
  // Номер поколения refresh(): «последний пришедший» ответ не равен «последнему
  // запрошенному». refresh() зовут $effect (смена фильтра/свёртки), onMount, поиск,
  // onSaved и onDelete, а режим свёртки удвоил запросы (listGrouped + anchors), так что
  // медленный ответ старого запроса мог перезаписать список нового. Устаревший ответ
  // отбрасывается ДО записи состояния.
  let refreshSeq = 0;
  // То же для перечитывания состава раскрытых якорей (reloadExpandedAnchors).
  let anchorsSeq = 0;

  interface ListSnapshot {
    items: DeviceDto[];
    groups: typeof groups;
    total: number;
    anchorItems: DeviceDto[];
    hiddenMemberCount: number;
  }

  /** Только запросы, без записи состояния: запись делает refresh() после проверки поколения. */
  async function fetchList(): Promise<ListSnapshot> {
    if (searchActive) {
      // Search mode — FTS5 (overrides grouping).
      const resp = await devices.search(searchQuery, pagination);
      return {
        items: resp.items,
        groups: [],
        total: resp.total,
        anchorItems: [],
        hiddenMemberCount: 0,
      };
    }
    if (grouped) {
      // Grouped mode — show non-unique devices as collapsible groups. Якоря не
      // участвуют в свёртке (exclude_anchors) и выводятся отдельным вызовом (D-11).
      const [grouping, anchors] = await Promise.all([
        devices.listGrouped(baseFilter, pagination),
        devices.list(
          { ...rawFilter, ...deviceListFlags('anchors', listCtx) },
          { offset: 0, limit: ANCHOR_MEMBERS_LIMIT },
        ),
      ]);
      return {
        items: [],
        groups: grouping,
        total: 0,
        anchorItems: anchors.items,
        hiddenMemberCount: 0,
      };
    }
    // Flat list mode.
    const resp = await devices.list(baseFilter, pagination);
    return {
      items: resp.items,
      groups: [],
      total: resp.total,
      anchorItems: [],
      hiddenMemberCount: resp.hidden_member_count ?? 0,
    };
  }

  async function refresh() {
    const seq = ++refreshSeq;
    loading = true;
    try {
      const snap = await fetchList();
      if (seq !== refreshSeq) return;
      items = snap.items;
      groups = snap.groups;
      total = snap.total;
      anchorItems = snap.anchorItems;
      hiddenMemberCount = snap.hiddenMemberCount;
      void reloadExpandedAnchors();
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

  // ---------------------------------------------------------------------------
  // Раскрытие состава якоря (D-12)
  // ---------------------------------------------------------------------------
  async function fetchAnchorMembers(groupId: number): Promise<AnchorMembersState> {
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
      { offset: 0, limit: ANCHOR_MEMBERS_LIMIT },
    );
    return { loading: false, items: resp.items, total: resp.total };
  }

  /** Ошибка загрузки состава: тост и сворачивание строки назад. */
  function failAnchor(groupIds: number[]) {
    pushToast('error', 'Не удалось загрузить состав группы.');
    const next = new Set(expandedAnchors);
    const cache = { ...anchorMembers };
    for (const id of groupIds) {
      next.delete(id);
      delete cache[id];
    }
    expandedAnchors = next;
    anchorMembers = cache;
  }

  async function loadAnchorMembers(groupId: number) {
    anchorMembers = { ...anchorMembers, [groupId]: { loading: true, items: [], total: 0 } };
    try {
      const state = await fetchAnchorMembers(groupId);
      anchorMembers = { ...anchorMembers, [groupId]: state };
    } catch {
      failAnchor([groupId]);
    }
  }

  /**
   * После перезагрузки списка состав раскрытых якорей перечитывается; кеш свёрнутых
   * отбрасывается (иначе при следующем раскрытии он был бы устаревшим). Старый
   * состав остаётся на экране до прихода нового: строка не мигает «Загрузкой».
   */
  async function reloadExpandedAnchors() {
    const aSeq = ++anchorsSeq;
    const ids = [...expandedAnchors];
    const results = await Promise.all(
      ids.map((id) =>
        fetchAnchorMembers(id).then(
          (state) => [id, state] as const,
          () => [id, null] as const,
        ),
      ),
    );
    if (aSeq !== anchorsSeq) return;
    const fresh: Record<number, AnchorMembersState> = {};
    const failed: number[] = [];
    for (const [id, state] of results) {
      if (state) fresh[id] = state;
      else failed.push(id);
    }
    // Якорь, раскрытый, пока шла перезагрузка, не теряет свою запись (иначе он
    // остался бы с «Загрузкой…» навсегда).
    for (const id of expandedAnchors) {
      if (!(id in fresh) && !failed.includes(id) && anchorMembers[id]) {
        fresh[id] = anchorMembers[id];
      }
    }
    anchorMembers = fresh;
    if (failed.length > 0) failAnchor(failed);
  }

  function toggleAnchor(groupId: number) {
    const next = new Set(expandedAnchors);
    if (next.has(groupId)) {
      next.delete(groupId);
      expandedAnchors = next;
      return;
    }
    next.add(groupId);
    expandedAnchors = next;
    if (!anchorMembers[groupId]) void loadAnchorMembers(groupId);
  }

  async function refreshCounts() {
    try {
      const arr = await devices.statusCounts();
      counts = new Map(arr.map((x) => [x.status_id, Number(x.count)]));
    } catch {
      // Non-fatal — counters stay 0.
    }
  }

  // Re-run refresh when filter/grouping changes.
  $effect(() => {
    // Access reactive deps: statusFilter, grouped — triggers re-run.
    // searchQuery is handled separately via debounce in DeviceFilters.
    void statusFilter;
    void grouped;
    refresh();
    refreshCounts();
  });

  onMount(() => {
    void ensureStatusesLoaded();
    refresh();
    refreshCounts();
  });

  // ---------------------------------------------------------------------------
  // Filter handlers
  // ---------------------------------------------------------------------------
  function handleSearchChange(q: string) {
    searchQuery = q;
    refresh();
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
        type_id: 1,
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
      {items}
      {groups}
      {total}
      {loading}
      {grouped}
      {searchActive}
      {expandedGroups}
      {highlightId}
      {anchorItems}
      {expandedAnchors}
      {anchorMembers}
      onToggleAnchor={toggleAnchor}
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

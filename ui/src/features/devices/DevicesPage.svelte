<script lang="ts">
  import { onMount } from 'svelte';
  import Button from '$lib/components/Button.svelte';
  import PageHeader from '$lib/components/PageHeader.svelte';
  import ActionMenu from '$lib/components/ActionMenu.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { isTauri } from '$lib/stores/transport.svelte';
  import { apiCall } from '$lib/api/client';
  import DeviceList from './DeviceList.svelte';
  import DeviceFilters from './DeviceFilters.svelte';
  import DeviceFormModal from './DeviceFormModal.svelte';
  import DeviceImportCsvModal from './DeviceImportCsvModal.svelte';
  import DocumentAcceptanceModal from '../acts/DocumentAcceptanceModal.svelte';
  import PdfPreviewModal from '../acts/PdfPreviewModal.svelte';
  import { devices } from './api';
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

  // Filters state (Plan 04).
  let searchQuery = $state('');
  let statusFilter = $state<number | null>(null);
  let grouped = $state(true);
  let counts = $state<Map<number, number>>(new Map());

  // Persisted expansion state: Set of group stable-key strings.
  // DeviceGroupRow reports its key on toggle; keys survive list refreshes.
  let expandedGroups = $state(new Set<string>());

  // type_id=1 ("Устройство") hardcoded — /devices section shows only Устройства.
  // group_by_condition: false — DevicesPage схлопывает разные condition в одну группу (ITEM-1).
  const baseFilter = $derived<DeviceFilter>({
    type_id: 1,
    location_id: null,
    status_id: statusFilter,
    state: null,
    name_prefix: null,
    include_deleted: false,
    group_by_condition: false,
  });

  const pagination = $state<Pagination>({ offset: 0, limit: 50 });

  const searchActive = $derived(searchQuery.trim().length > 0);

  // ---------------------------------------------------------------------------
  // Data loading
  // ---------------------------------------------------------------------------
  async function refresh() {
    loading = true;
    try {
      if (searchActive) {
        // Search mode — FTS5 (overrides grouping).
        const resp = await devices.search(searchQuery, pagination);
        items = resp.items;
        total = resp.total;
        groups = [];
      } else if (grouped) {
        // Grouped mode — show non-unique devices as collapsible groups.
        groups = await devices.listGrouped(baseFilter, pagination);
        items = [];
        total = 0;
      } else {
        // Flat list mode.
        const resp = await devices.list(baseFilter, pagination);
        items = resp.items;
        total = resp.total;
        groups = [];
      }
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось загрузить устройства';
      pushToast('error', msg);
    } finally {
      loading = false;
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

  function onSaved() {
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
        location_id: null,
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
  onImported={() => {
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

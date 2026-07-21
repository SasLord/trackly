<script lang="ts">
  // Plan 03-02: master-detail page for #/acts.
  // Switch-bar (Акты/Возвраты/Архив) + search + master-detail layout.
  import { onMount } from 'svelte';
  import Button from '$lib/components/Button.svelte';
  import PageHeader from '$lib/components/PageHeader.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import ActsSearchAndTabs from './ActsSearchAndTabs.svelte';
  import ActsMasterDetail from './ActsMasterDetail.svelte';
  import ActsList from './ActsList.svelte';
  import ActDetail from './ActDetail.svelte';
  import ActFormModal from './ActFormModal.svelte';
  import ReturnModal from './ReturnModal.svelte';
  import PdfPreviewModal from './PdfPreviewModal.svelte';
  import { acts } from './api';
  import type {
    ActDto,
    ActFilter,
    ActListResponse,
    ActsCountsDto,
    Pagination,
  } from '../../bindings';

  type TabKey = 'handover' | 'returns' | 'archive';

  let items = $state<ActDto[]>([]);
  let total = $state(0);
  let loading = $state(false);
  let counts = $state<ActsCountsDto>({ handover_active: 0, returns: 0, archived: 0 });
  let activeTab = $state<TabKey>('handover');
  let selectedActId = $state<number | null>(null);
  let selectedAct = $state<ActDto | null>(null);
  let detailLoading = $state(false);
  let createModalOpen = $state(false);
  let editModalOpen = $state(false);
  let editTargetAct = $state<ActDto | null>(null);
  let returnModalOpen = $state(false);
  let returnTargetAct = $state<ActDto | null>(null);
  let returnMode = $state<'create' | 'edit'>('create');
  let returnEditTargetAct = $state<ActDto | null>(null);
  let returnEditParentAct = $state<ActDto | null>(null);
  let pdfModalOpen = $state(false);
  let pdfModalAct = $state<ActDto | null>(null);
  let searchQuery = $state('');
  const pagination = $state<Pagination>({ offset: 0, limit: 50 });

  const baseFilter = $derived<ActFilter>({
    act_type: activeTab === 'returns' ? 'return' : 'handover',
    archived: activeTab === 'archive' ? true : activeTab === 'handover' ? false : null,
    search: searchQuery.trim() ? searchQuery.trim() : null,
    include_deleted: false,
  });

  async function refresh() {
    loading = true;
    try {
      const trimmed = searchQuery.trim();
      const resp: ActListResponse = trimmed
        ? await acts.search(trimmed, baseFilter, pagination)
        : await acts.list(baseFilter, pagination);
      items = resp.items;
      total = resp.total;
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось загрузить акты';
      pushToast('error', msg);
    } finally {
      loading = false;
    }
  }

  async function refreshCounts() {
    try {
      counts = await acts.counts();
    } catch {
      // Non-fatal — counters stay at last good value.
    }
  }

  $effect(() => {
    void activeTab;
    selectedActId = null;
    selectedAct = null;
  });

  $effect(() => {
    void activeTab;
    void searchQuery;
    refresh();
    refreshCounts();
  });

  $effect(() => {
    const id = selectedActId;
    if (id === null) {
      selectedAct = null;
      return;
    }
    detailLoading = true;
    acts
      .get(id)
      .then((a) => {
        selectedAct = a;
      })
      .catch((e: unknown) => {
        const msg =
          e && typeof e === 'object' && 'message' in e
            ? String((e as { message: unknown }).message)
            : 'Не удалось загрузить акт';
        pushToast('error', msg);
        selectedAct = null;
      })
      .finally(() => {
        detailLoading = false;
      });
  });

  onMount(() => {
    refresh();
    refreshCounts();
  });

  function openCreate() {
    createModalOpen = true;
  }
  function handleSaved(act: ActDto) {
    createModalOpen = false;
    selectedActId = act.id;
    refresh();
    refreshCounts();
  }
  function handleSelect(id: number) {
    selectedActId = id;
  }
  function handleResetSearch() {
    searchQuery = '';
  }
  function handleReturn(act: ActDto) {
    returnTargetAct = act;
    returnMode = 'create';
    returnModalOpen = true;
  }

  // Plan 19-05 (ACT-02): reuse the `act` argument directly (no acts.get(act.id)
  // re-fetch) — onEdit is only ever invoked from ActDetail where act === selectedAct,
  // and selectedAct is already guaranteed fresh via the acts.get(id) $effect above
  // (Pitfall 5 — only acts.get(id) populates outstanding_device_ids).
  //
  // Phase 22 (ACT-03): return-act rows branch into the ReturnModal edit path
  // instead — that dialog needs BOTH the return's own items (act) AND the
  // parent's still-outstanding items (addable rows), so the parent is
  // fetched here before opening the modal.
  async function handleEdit(act: ActDto) {
    if (act.act_type === 'return') {
      try {
        const parent = await acts.get(act.parent_act_id!);
        returnEditTargetAct = act;
        returnEditParentAct = parent;
        returnMode = 'edit';
        returnModalOpen = true;
      } catch (e: unknown) {
        const msg =
          e && typeof e === 'object' && 'message' in e
            ? String((e as { message: unknown }).message)
            : 'Не удалось загрузить родительский акт';
        pushToast('error', msg);
      }
      return;
    }
    editTargetAct = act;
    editModalOpen = true;
  }

  function handleEditSaved(act: ActDto) {
    editModalOpen = false;
    editTargetAct = null;
    // D-11: selectedActId = act.id is a no-op when the edited act is already
    // selected (the detail $effect is keyed on selectedActId), leaving the
    // detail card stale. Assign selectedAct directly — act is the fresh full
    // ActDto returned by acts.update() (server self.get) — mirroring how
    // handleReturnSuccess refreshes the detail immediately below.
    selectedActId = act.id;
    selectedAct = act;
    refresh();
    refreshCounts();
  }

  function handlePrint(act: ActDto) {
    pdfModalAct = act;
    pdfModalOpen = true;
  }

  function handleReturnSuccess(returnDto: ActDto, _parentArchived: boolean) {
    const wasEdit = returnMode === 'edit';
    returnModalOpen = false;
    returnTargetAct = null;
    returnEditTargetAct = null;
    returnEditParentAct = null;
    returnMode = 'create';
    // Refresh list + counts; selected act всё ещё может смотреться, обновим его detail.
    refresh();
    refreshCounts();
    if (wasEdit && selectedActId === returnDto.id) {
      // D-11 (Phase 19 pattern, reused): assign the fresh ActDto directly —
      // selectedActId = returnDto.id would be a no-op here (already
      // selected), leaving the detail-view keyed $effect stale. onSuccess
      // already receives the server's fresh response, so no second fetch
      // (and no second click) is needed.
      selectedAct = returnDto;
    } else if (selectedActId !== null) {
      acts
        .get(selectedActId)
        .then((a) => {
          selectedAct = a;
        })
        .catch(() => {});
    }
  }

  async function handleDelete(act: ActDto) {
    const isReturn = act.act_type === 'return';
    const heading = isReturn
      ? `Удалить акт возврата №${act.number}?`
      : `Удалить акт №${act.number}?`;
    const body = isReturn
      ? 'Акт будет помечен как удалённый. Состояние и Расположение устройств вернутся к значениям на момент выдачи. Если parent был в Архиве — выйдет из архива.'
      : 'Акт будет помечен как удалённый. Все устройства из акта вернутся на склад в исходные Состояние и Расположение (на момент выдачи). Связанные возвраты также будут отменены. Действие можно отменить только восстановлением из бэкапа БД.';
    const confirmed = window.confirm(`${heading}\n\n${body}`);
    if (!confirmed) return;
    try {
      await acts.delete(act.id, act.version);
      pushToast(
        'success',
        isReturn
          ? `Акт возврата №${act.number} удалён. Устройства возвращены к состоянию на момент выдачи.`
          : `Акт №${act.number} удалён. Устройства восстановлены.`,
      );
      selectedActId = null;
      refresh();
      refreshCounts();
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось удалить акт';
      pushToast('error', msg);
    }
  }
</script>

<div class="acts-page">
  <PageHeader title="Акты">
    {#snippet actions()}
      <Button variant="primary" onclick={openCreate}>+ Создать акт</Button>
    {/snippet}
  </PageHeader>

  <div class="page-content">
    <ActsSearchAndTabs
      {searchQuery}
      {activeTab}
      {counts}
      onSearchChange={(q) => (searchQuery = q)}
      onTabChange={(t) => (activeTab = t)}
    />

    <ActsMasterDetail>
      {#snippet master()}
        <ActsList
          {items}
          {total}
          {loading}
          {selectedActId}
          {activeTab}
          {searchQuery}
          onSelect={handleSelect}
          onCreate={openCreate}
          onResetSearch={handleResetSearch}
        />
      {/snippet}
      {#snippet detail()}
        <ActDetail
          act={selectedAct}
          loading={detailLoading}
          onCreate={openCreate}
          onDelete={handleDelete}
          onEdit={handleEdit}
          onReturn={handleReturn}
          onPrint={handlePrint}
        />
      {/snippet}
    </ActsMasterDetail>
  </div>
</div>

<ActFormModal
  open={createModalOpen}
  onClose={() => (createModalOpen = false)}
  onSaved={handleSaved}
/>

<ActFormModal
  mode="edit"
  initialAct={editTargetAct}
  open={editModalOpen}
  onClose={() => {
    editModalOpen = false;
    editTargetAct = null;
  }}
  onSaved={handleEditSaved}
/>

<ReturnModal
  open={returnModalOpen}
  act={returnTargetAct}
  mode={returnMode}
  editTarget={returnEditTargetAct}
  parentAct={returnEditParentAct}
  onClose={() => {
    returnModalOpen = false;
    returnTargetAct = null;
    returnEditTargetAct = null;
    returnEditParentAct = null;
    returnMode = 'create';
  }}
  onSuccess={handleReturnSuccess}
/>

<PdfPreviewModal
  open={pdfModalOpen}
  actId={pdfModalAct ? pdfModalAct.id : null}
  title={pdfModalAct ? `Печать акта №${pdfModalAct.number}` : 'Печать акта'}
  onClose={() => {
    pdfModalOpen = false;
    pdfModalAct = null;
  }}
/>

<style lang="scss">
  .acts-page {
    display: flex;
    flex-direction: column;
    height: 100%;
  }
  .page-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    // FIX B1: page-content no longer scrolls itself — MasterDetail fills the
    // remaining height and scrolls its own panels internally. Horizontal
    // overflow is preserved for the existing <1100px fallback.
    overflow-x: auto;
    overflow-y: hidden;
    padding: var(--tr-space-xl) var(--tr-space-2xl);
  }
</style>

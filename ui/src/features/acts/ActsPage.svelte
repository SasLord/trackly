<script lang="ts">
  // Plan 03-02: master-detail page for #/acts.
  // Switch-bar (Акты/Возвраты/Архив) + search + master-detail layout.
  import { onMount } from 'svelte';
  import Button from '$lib/components/Button.svelte';
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
  let returnModalOpen = $state(false);
  let returnTargetAct = $state<ActDto | null>(null);
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
      const resp: ActListResponse = await acts.list(baseFilter, pagination);
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
    returnModalOpen = true;
  }

  function handlePrint(act: ActDto) {
    pdfModalAct = act;
    pdfModalOpen = true;
  }

  function handleReturnSuccess(_returnDto: ActDto, _parentArchived: boolean) {
    returnModalOpen = false;
    returnTargetAct = null;
    // Refresh list + counts; selected act всё ещё может смотреться, обновим его detail.
    refresh();
    refreshCounts();
    if (selectedActId !== null) {
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
  <header class="page-header">
    <h1 class="page-title">Акты</h1>
    <div class="header-actions">
      <Button variant="primary" onclick={openCreate}>+ Создать акт</Button>
    </div>
  </header>

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

<ReturnModal
  open={returnModalOpen}
  act={returnTargetAct}
  onClose={() => {
    returnModalOpen = false;
    returnTargetAct = null;
  }}
  onSuccess={handleReturnSuccess}
/>

<PdfPreviewModal
  open={pdfModalOpen}
  actId={pdfModalAct ? pdfModalAct.id : null}
  title={pdfModalAct ? `Печать акта №${pdfModalAct.number}` : 'Печать акта'}
  actNumberDisplay={pdfModalAct ? pdfModalAct.number : null}
  actDateUtc={pdfModalAct ? pdfModalAct.created_at_utc : null}
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
  .page-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-lg) var(--space-xl);
    border-bottom: 1px solid var(--color-border);
    flex-shrink: 0;
    gap: var(--space-md);
    flex-wrap: wrap;
  }
  .page-title {
    margin: 0;
    font-size: var(--font-size-page-title, var(--font-size-heading));
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
    line-height: var(--line-height-heading);
  }
  .header-actions {
    display: flex;
    gap: var(--space-sm);
  }
  .page-content {
    flex: 1;
    overflow: auto;
    padding: var(--space-lg) var(--space-xl);
  }
</style>

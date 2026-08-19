<script lang="ts">
  // Plan 04-04: корневой компонент раздела «Картриджи».
  // Plan 04-05: wire CartridgeContextMenu + OperationModal + CartridgeFormModal + LowStockBanner.
  // Plan 04-06: финальная интеграция ModelsList + ModelFormModal.
  // Два таба: «Картриджи» (master-detail) / «Модели» (полноширинный CRUD-список).
  import { onMount } from 'svelte';
  import Button from '$lib/components/Button.svelte';
  import Modal from '$lib/components/Modal.svelte';
  import PageHeader from '$lib/components/PageHeader.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import CartridgesSearchAndTabs from './CartridgesSearchAndTabs.svelte';
  import CartridgesMasterDetail from './CartridgesMasterDetail.svelte';
  import CartridgeFilters from './CartridgeFilters.svelte';
  import CartridgesList from './CartridgesList.svelte';
  import CartridgeDetail from './CartridgeDetail.svelte';
  import LowStockBanner from './LowStockBanner.svelte';
  import OperationModal from './OperationModal.svelte';
  import CartridgeFormModal from './CartridgeFormModal.svelte';
  import ModelsList from './ModelsList.svelte';
  import ModelFormModal from './ModelFormModal.svelte';
  import { cartridges } from './api';
  import type {
    AuditEntryDto,
    CartridgeCountsDto,
    CartridgeDto,
    CartridgeFilter,
    CartridgeModelDto,
    LowStockItemDto,
    Pagination,
  } from '../../bindings';

  type TabKey = 'cartridges' | 'models';

  let activeTab = $state<TabKey>('cartridges');
  let selectedCartridgeId = $state<number | null>(null);
  let selectedCartridge = $state<CartridgeDto | null>(null);
  let cartridgeHistory = $state<AuditEntryDto[]>([]);
  let detailLoading = $state(false);

  let items = $state<CartridgeDto[]>([]);
  let total = $state(0);
  let listLoading = $state(false);
  let counts = $state<CartridgeCountsDto>({
    all: 0,
    in_stock: 0,
    in_use: 0,
    at_refill: 0,
    written_off: 0,
  });
  let models = $state<CartridgeModelDto[]>([]);
  let modelsLoading = $state(false);
  let lowStockItems = $state<LowStockItemDto[]>([]);

  let searchQuery = $state('');
  let modelSearchQuery = $state('');
  let statusId = $state<number | null>(null);
  let kindId = $state<number | null>(null);
  let modelId = $state<number | null>(null);

  const pagination = $state<Pagination>({ offset: 0, limit: 50 });

  const activeFilter = $derived<CartridgeFilter>({
    status_id: statusId,
    kind_id: kindId,
    model_id: modelId,
    search: searchQuery.trim() ? searchQuery.trim() : null,
    include_deleted: false,
    compatible_with_printer_device_id: null,
  });

  const hasFilter = $derived(
    statusId !== null || kindId !== null || modelId !== null || searchQuery.trim().length > 0,
  );

  // Клиентский фильтр вкладки «Модели» — над уже загруженным целиком списком
  // (models пагинации не имеет), без сетевого запроса.
  const filteredModels = $derived.by(() => {
    const q = modelSearchQuery.trim().toLowerCase();
    if (!q) return models;
    return models.filter((m) => {
      const haystack = `${m.brand} ${m.model} ${m.notes ?? ''}`.toLowerCase();
      return haystack.includes(q);
    });
  });

  async function refresh() {
    listLoading = true;
    try {
      const trimmed = searchQuery.trim();
      const resp = trimmed
        ? await cartridges.search(trimmed, activeFilter)
        : await cartridges.list(activeFilter, pagination);
      items = resp.items;
      total = resp.total;
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось загрузить картриджи';
      pushToast('error', msg);
    } finally {
      listLoading = false;
    }
  }

  async function refreshCounts() {
    try {
      counts = await cartridges.statusCounts();
    } catch {
      // Non-fatal — counters stay at last good value.
    }
  }

  async function refreshModels() {
    modelsLoading = true;
    try {
      models = await cartridges.modelsList();
    } catch {
      // Non-fatal.
    } finally {
      modelsLoading = false;
    }
  }

  async function refreshLowStock() {
    try {
      lowStockItems = await cartridges.lowStock();
    } catch {
      // Non-fatal.
    }
  }

  // loadAll: единое место для загрузки данных вкладки «Картриджи» (Task 2 §2).
  async function loadAll() {
    await Promise.all([refresh(), refreshCounts(), refreshLowStock()]);
  }

  // Сбрасываем выбранный картридж при смене таба.
  $effect(() => {
    void activeTab;
    selectedCartridgeId = null;
    selectedCartridge = null;
    cartridgeHistory = [];
  });

  // Перезагружаем список при изменении фильтров.
  $effect(() => {
    void activeFilter;
    refresh();
    refreshCounts();
  });

  // Загружаем деталь выбранного картриджа.
  $effect(() => {
    const id = selectedCartridgeId;
    if (id === null) {
      selectedCartridge = null;
      cartridgeHistory = [];
      return;
    }
    detailLoading = true;
    Promise.all([cartridges.get(id), cartridges.getHistory(id)])
      .then(([dto, history]) => {
        selectedCartridge = dto;
        cartridgeHistory = history;
      })
      .catch((e: unknown) => {
        const msg =
          e && typeof e === 'object' && 'message' in e
            ? String((e as { message: unknown }).message)
            : 'Не удалось загрузить данные картриджа';
        pushToast('error', msg);
        selectedCartridge = null;
        cartridgeHistory = [];
      })
      .finally(() => {
        detailLoading = false;
      });
  });

  onMount(() => {
    loadAll();
    refreshModels();
  });

  // --- Modal state (04-05) ---
  type OpType = 'install' | 'return_to_stock' | 'to_refill' | 'from_refill' | 'write_off';

  let operationModalOpen = $state(false);
  let operationModalOp = $state<OpType>('install');
  let operationModalCartridge = $state<CartridgeDto | null>(null);

  let formModalOpen = $state(false);
  let formModalTarget = $state<CartridgeDto | null>(null);

  let confirmDeleteOpen = $state(false);
  let confirmDeleteCartridge = $state<CartridgeDto | null>(null);
  let deleting = $state(false);

  // --- Model modal state (04-06) ---
  let modelFormOpen = $state(false);
  let modelFormTarget = $state<CartridgeModelDto | null>(null);

  let confirmDeleteModelOpen = $state(false);
  let confirmDeleteModel = $state<CartridgeModelDto | null>(null);
  let deletingModel = $state(false);

  function handleSelect(id: number) {
    selectedCartridgeId = id;
  }

  function handleMenuAction(op: string, cartridge: CartridgeDto) {
    if (
      op === 'install' ||
      op === 'return_to_stock' ||
      op === 'to_refill' ||
      op === 'from_refill' ||
      op === 'write_off'
    ) {
      operationModalCartridge = cartridge;
      operationModalOp = op as OpType;
      operationModalOpen = true;
    } else if (op === 'edit') {
      formModalTarget = cartridge;
      formModalOpen = true;
    } else if (op === 'delete') {
      confirmDeleteCartridge = cartridge;
      confirmDeleteOpen = true;
    }
  }

  function openCreate() {
    if (activeTab === 'models') {
      modelFormTarget = null;
      modelFormOpen = true;
    } else {
      formModalTarget = null;
      formModalOpen = true;
    }
  }

  function handleOperationSuccess() {
    // Refresh list + counts + low_stock after lifecycle operation (Task 2 §6).
    loadAll();
    // Re-load selected cartridge detail if relevant.
    if (selectedCartridgeId !== null) {
      const id = selectedCartridgeId;
      Promise.all([cartridges.get(id), cartridges.getHistory(id)])
        .then(([dto, history]) => {
          selectedCartridge = dto;
          cartridgeHistory = history;
        })
        .catch(() => {
          // Non-fatal — list will refresh anyway.
        });
    }
  }

  function handleFormSuccess(cart: CartridgeDto) {
    loadAll();
    refreshModels();
    // Auto-select the created/updated cartridge.
    selectedCartridgeId = cart.id;
    // Force-refresh the detail pane even when the edited cartridge is the one
    // already selected: assigning the same selectedCartridgeId is a no-op, so
    // the detail $effect would not re-run and the pane would stay stale
    // (UAT round 3 №1 — location not updating after edit).
    const id = cart.id;
    selectedCartridge = cart;
    Promise.all([cartridges.get(id), cartridges.getHistory(id)])
      .then(([dto, history]) => {
        if (selectedCartridgeId === id) {
          selectedCartridge = dto;
          cartridgeHistory = history;
        }
      })
      .catch(() => {
        // Non-fatal — list already refreshed.
      });
  }

  async function handleConfirmDelete() {
    if (!confirmDeleteCartridge) return;
    deleting = true;
    try {
      await cartridges.delete(confirmDeleteCartridge.id, confirmDeleteCartridge.version);
      pushToast('success', `Картридж «${confirmDeleteCartridge.code}» удалён.`);
      confirmDeleteOpen = false;
      if (selectedCartridgeId === confirmDeleteCartridge.id) {
        selectedCartridgeId = null;
        selectedCartridge = null;
        cartridgeHistory = [];
      }
      confirmDeleteCartridge = null;
      loadAll();
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось удалить картридж';
      pushToast('error', msg);
    } finally {
      deleting = false;
    }
  }

  // --- Model callbacks (04-06) ---

  function handleCreateModel() {
    modelFormTarget = null;
    modelFormOpen = true;
  }

  function handleEditModel(model: CartridgeModelDto) {
    modelFormTarget = model;
    modelFormOpen = true;
  }

  function handleDeleteModel(model: CartridgeModelDto) {
    confirmDeleteModel = model;
    confirmDeleteModelOpen = true;
  }

  function handleModelFormSuccess(_model: CartridgeModelDto) {
    modelFormOpen = false;
    refreshModels();
    // Low stock may change when model list changes.
    refreshLowStock();
  }

  async function handleConfirmDeleteModel() {
    if (!confirmDeleteModel) return;
    deletingModel = true;
    try {
      await cartridges.modelsDelete(confirmDeleteModel.id, confirmDeleteModel.version);
      pushToast(
        'success',
        `Модель «${confirmDeleteModel.brand} ${confirmDeleteModel.model}» удалена.`,
      );
      confirmDeleteModelOpen = false;
      confirmDeleteModel = null;
      refreshModels();
      refreshLowStock();
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось удалить модель';
      // UI-SPEC §ModelListRow: если используется — показываем Toast (без confirm).
      confirmDeleteModelOpen = false;
      pushToast('error', msg);
    } finally {
      deletingModel = false;
    }
  }
</script>

<div class="cartridges-page">
  <PageHeader title="Картриджи">
    {#snippet actions()}
      {#if activeTab === 'cartridges'}
        <Button variant="primary" onclick={openCreate}>+ Добавить картридж/фотобарабан</Button>
      {:else}
        <Button variant="primary" onclick={openCreate}>+ Добавить модель</Button>
      {/if}
    {/snippet}
  </PageHeader>

  <div class="page-content">
    <CartridgesSearchAndTabs
      {searchQuery}
      {activeTab}
      {counts}
      onSearchChange={(q) => (searchQuery = q)}
      onTabChange={(t) => (activeTab = t)}
      {modelSearchQuery}
      onModelSearchChange={(q) => (modelSearchQuery = q)}
    />

    {#if activeTab === 'cartridges'}
      <!-- Switch-bar статусов + фильтры вынесены на уровень страницы (между -->
      <!-- строкой поиска и списком), полноширинно — не зажаты в узкой колонке. -->
      <CartridgeFilters
        {statusId}
        {kindId}
        {modelId}
        {counts}
        {models}
        onStatusChange={(s: number | null) => (statusId = s)}
        onKindChange={(k: number | null) => {
          kindId = k;
          // Сбрасываем модель на «Все», если она не соответствует новому типу;
          // сохраняем выбор, если соответствует (UAT round 3 №3).
          if (k !== null && modelId !== null) {
            const m = models.find((x) => x.id === modelId);
            if (!m || m.kind_id !== k) modelId = null;
          }
        }}
        onModelChange={(m: number | null) => (modelId = m)}
      />

      <LowStockBanner items={lowStockItems} />

      <CartridgesMasterDetail>
        {#snippet master()}
          <CartridgesList
            {items}
            {total}
            loading={listLoading}
            selectedId={selectedCartridgeId}
            {hasFilter}
            statusFiltered={statusId !== null}
            onSelect={handleSelect}
            onMenuAction={handleMenuAction}
            onCreate={openCreate}
          />
        {/snippet}
        {#snippet detail()}
          <CartridgeDetail
            cartridge={selectedCartridge}
            history={cartridgeHistory}
            loading={detailLoading}
            onCreate={openCreate}
            onMenuAction={handleMenuAction}
          />
        {/snippet}
      </CartridgesMasterDetail>
    {:else}
      <!-- Вкладка «Модели» (04-06) -->
      <ModelsList
        models={filteredModels}
        loading={modelsLoading}
        onCreateModel={handleCreateModel}
        onEditModel={handleEditModel}
        onDeleteModel={handleDeleteModel}
      />
    {/if}
  </div>
</div>

<!-- OperationModal (04-05): lifecycle-операции -->
<OperationModal
  open={operationModalOpen}
  op={operationModalOp}
  cartridge={operationModalCartridge}
  onClose={() => (operationModalOpen = false)}
  onSuccess={handleOperationSuccess}
/>

<!-- CartridgeFormModal (04-05): создание/редактирование -->
<CartridgeFormModal
  open={formModalOpen}
  target={formModalTarget}
  {models}
  onClose={() => (formModalOpen = false)}
  onSuccess={handleFormSuccess}
/>

<!-- ModelFormModal (04-06): создание/редактирование модели -->
<ModelFormModal
  open={modelFormOpen}
  target={modelFormTarget}
  onClose={() => (modelFormOpen = false)}
  onSuccess={handleModelFormSuccess}
/>

<!-- Confirm delete modal (04-05): картриджи -->
<Modal
  open={confirmDeleteOpen}
  title="Удалить картридж?"
  onClose={() => (confirmDeleteOpen = false)}
>
  <p class="confirm-body">
    Картридж «{confirmDeleteCartridge?.code ?? ''}» будет помечен как удалённый. Отменить можно
    только восстановлением из резервной копии БД.
  </p>
  {#snippet footer()}
    <Button variant="secondary" onclick={() => (confirmDeleteOpen = false)}>Отмена</Button>
    <Button variant="destructive" loading={deleting} onclick={handleConfirmDelete}>Удалить</Button>
  {/snippet}
</Modal>

<!-- Confirm delete modal (04-06): модели -->
<Modal
  open={confirmDeleteModelOpen}
  title="Удалить модель?"
  onClose={() => (confirmDeleteModelOpen = false)}
>
  <p class="confirm-body">
    Модель «{confirmDeleteModel?.brand ?? ''}
    {confirmDeleteModel?.model ?? ''}» будет помечена как удалённая.
  </p>
  {#snippet footer()}
    <Button variant="secondary" onclick={() => (confirmDeleteModelOpen = false)}>Отмена</Button>
    <Button variant="destructive" loading={deletingModel} onclick={handleConfirmDeleteModel}
      >Удалить</Button
    >
  {/snippet}
</Modal>

<style lang="scss">
  .cartridges-page {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .page-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    // FIX B1: page-content no longer scrolls itself — MasterDetail (or
    // ModelsList, on the «Модели» tab) fills the remaining height and scrolls
    // its own panels internally. Horizontal overflow is preserved for the
    // existing <1100px fallback.
    overflow-x: auto;
    overflow-y: hidden;
    padding: var(--tr-space-xl) var(--tr-space-2xl);
  }

  // ModelsList (the «Модели» tab) is not wrapped in CartridgesMasterDetail —
  // it sits directly in page-content, so it needs its own flex-fill to reach
  // the bottom now that page-content no longer scrolls (FIX B1).
  .page-content > :global(.models-list) {
    flex: 1 1 auto;
    min-height: 0;
  }

  .confirm-body {
    margin: 0;
    color: var(--tr-text-secondary);
    line-height: var(--tr-line-height-body);
    text-align: center;
    overflow-wrap: anywhere;
    word-break: break-word;
    white-space: normal;
    max-width: 100%;
  }
</style>

<script lang="ts">
  // Plan 04-04: корневой компонент раздела «Картриджи».
  // Два таба: «Картриджи» (master-detail) / «Модели» (полноширинный CRUD-список).
  import { onMount } from 'svelte';
  import Button from '$lib/components/Button.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import CartridgesSearchAndTabs from './CartridgesSearchAndTabs.svelte';
  import CartridgesMasterDetail from './CartridgesMasterDetail.svelte';
  import CartridgeFilters from './CartridgeFilters.svelte';
  import CartridgesList from './CartridgesList.svelte';
  import CartridgeDetail from './CartridgeDetail.svelte';
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
  let lowStockItems = $state<LowStockItemDto[]>([]);

  let searchQuery = $state('');
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
  });

  const hasFilter = $derived(
    statusId !== null || kindId !== null || modelId !== null || searchQuery.trim().length > 0,
  );

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
    try {
      models = await cartridges.modelsList();
    } catch {
      // Non-fatal.
    }
  }

  async function refreshLowStock() {
    try {
      lowStockItems = await cartridges.lowStock();
    } catch {
      // Non-fatal.
    }
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
    refresh();
    refreshCounts();
    refreshModels();
    refreshLowStock();
  });

  function handleSelect(id: number) {
    selectedCartridgeId = id;
  }

  function handleMenuAction(_op: string, _cartridge: CartridgeDto) {
    // CRUD / lifecycle модалки будут реализованы в плане 04-05.
    // Пока — заглушка.
  }

  function openCreate() {
    // Открытие модала создания — реализуется в плане 04-05.
  }
</script>

<div class="cartridges-page">
  <header class="page-header">
    <h1 class="page-title">Картриджи</h1>
    <div class="header-actions">
      {#if activeTab === 'cartridges'}
        <Button variant="primary" onclick={openCreate}>+ Добавить картридж</Button>
      {:else}
        <Button variant="primary" onclick={openCreate}>+ Добавить модель</Button>
      {/if}
    </div>
  </header>

  <div class="page-content">
    <CartridgesSearchAndTabs
      {searchQuery}
      {activeTab}
      {counts}
      onSearchChange={(q) => (searchQuery = q)}
      onTabChange={(t) => (activeTab = t)}
    />

    {#if activeTab === 'cartridges'}
      {#if lowStockItems.length > 0}
        <div class="low-stock-banner">
          <span class="low-stock-icon" aria-hidden="true">
            <svg
              width="16"
              height="16"
              viewBox="0 0 16 16"
              fill="none"
              xmlns="http://www.w3.org/2000/svg"
            >
              <path
                d="M8 1.5L14.5 13H1.5L8 1.5Z"
                stroke="currentColor"
                stroke-width="1.5"
                stroke-linejoin="round"
              />
              <path d="M8 6V9" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
              <circle cx="8" cy="11" r="0.75" fill="currentColor" />
            </svg>
          </span>
          <div class="low-stock-content">
            <strong class="low-stock-title">Низкий остаток картриджей</strong>
            <ul class="low-stock-list">
              {#each lowStockItems as item (item.model_id)}
                <li>
                  {item.brand}
                  {item.model} — {item.count} шт. на складе (порог: {item.threshold})
                </li>
              {/each}
            </ul>
          </div>
        </div>
      {/if}

      <CartridgesMasterDetail>
        {#snippet master()}
          <CartridgeFilters
            {statusId}
            {kindId}
            {modelId}
            {counts}
            {models}
            onStatusChange={(s: number | null) => (statusId = s)}
            onKindChange={(k: number | null) => (kindId = k)}
            onModelChange={(m: number | null) => (modelId = m)}
          />
          <CartridgesList
            {items}
            {total}
            loading={listLoading}
            selectedId={selectedCartridgeId}
            {hasFilter}
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
          />
        {/snippet}
      </CartridgesMasterDetail>
    {:else}
      <!-- ModelsList будет реализован в плане 04-05 -->
      <div class="models-placeholder">
        <p class="models-placeholder-text">Список моделей появится в следующем обновлении.</p>
      </div>
    {/if}
  </div>
</div>

<style lang="scss">
  .cartridges-page {
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

  .low-stock-banner {
    display: flex;
    align-items: flex-start;
    gap: var(--space-sm);
    padding: var(--space-md);
    margin-bottom: var(--space-md);
    background: color-mix(in srgb, var(--color-warning) 10%, transparent);
    border: 1px solid var(--color-warning);
    border-radius: var(--radius-md);
    color: var(--color-text-primary);
  }

  .low-stock-icon {
    color: var(--color-warning);
    flex-shrink: 0;
    margin-top: 2px;
    display: flex;
    align-items: center;
  }

  .low-stock-content {
    flex: 1;
  }

  .low-stock-title {
    display: block;
    font-size: var(--font-size-body);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
    margin-bottom: var(--space-xs);
  }

  .low-stock-list {
    margin: 0;
    padding: 0;
    list-style: none;
    font-size: var(--font-size-label);
    color: var(--color-text-secondary);

    li {
      line-height: 1.6;
    }
  }

  .models-placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 240px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
  }

  .models-placeholder-text {
    color: var(--color-text-muted);
    font-size: var(--font-size-body);
  }
</style>

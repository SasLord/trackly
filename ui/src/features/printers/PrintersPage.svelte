<script lang="ts">
  // Plan 06-04: корневой компонент раздела «Принтеры».
  // По паттерну CartridgesPage.svelte (Svelte 5 runes).
  // WS-интеграция: onMount → connectWs() + onWsEvent(handleWsEvent).
  // PrinterDetail и DiscoveryModal: Task 2b.
  import { onMount } from 'svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { authStore } from '$lib/stores/auth.svelte';
  import { connectWs, onWsEvent } from '$lib/api/ws';
  import PrintersMasterDetail from './PrintersMasterDetail.svelte';
  import PrintersSearchAndTabs from './PrintersSearchAndTabs.svelte';
  import PrintersList from './PrintersList.svelte';
  import PrinterDetail from './PrinterDetail.svelte';
  import DiscoveryModal from './DiscoveryModal.svelte';
  import { printers } from './api';
  import type { PrinterDto, PrinterFilter } from '../../bindings-phase6';
  import type { WsEvent } from '../../bindings-phase6';

  let items = $state<PrinterDto[]>([]);
  let listLoading = $state(false);
  let selectedId = $state<number | null>(null);
  let selectedPrinter = $state<PrinterDto | null>(null);
  let detailLoading = $state(false);
  let discoveryOpen = $state(false);

  let filter = $state<PrinterFilter>({ status: null, search: null });

  // Reload list when filter changes.
  $effect(() => {
    void filter;
    refresh();
  });

  // Load detail when selectedId changes.
  $effect(() => {
    const id = selectedId;
    if (id === null) {
      selectedPrinter = null;
      return;
    }
    detailLoading = true;
    printers
      .get(id)
      .then((dto) => {
        selectedPrinter = dto;
      })
      .catch((e: unknown) => {
        const msg =
          e && typeof e === 'object' && 'message' in e
            ? String((e as { message: unknown }).message)
            : 'Не удалось загрузить данные принтера';
        pushToast('error', msg);
        selectedPrinter = null;
      })
      .finally(() => {
        detailLoading = false;
      });
  });

  async function refresh() {
    listLoading = true;
    try {
      const resp = await printers.list(filter, { offset: 0, limit: 100 });
      items = resp.items;
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось загрузить принтеры';
      pushToast('error', msg);
    } finally {
      listLoading = false;
    }
  }

  function handleWsEvent(event: WsEvent) {
    if (event.type === 'printer_alert') {
      pushToast('warning', `Проблема с принтером: ${event.printerName} — ${event.alertType}`);
      void refresh(); // Reload list to update alert indicators.
    }
  }

  onMount(() => {
    refresh();
    // Connect WS for real-time notifications.
    let unlisten: (() => void) | undefined;
    connectWs().then((fn) => {
      unlisten = fn;
    }).catch(() => {
      // WS connection is non-fatal.
    });
    const unsubscribe = onWsEvent(handleWsEvent);
    return () => {
      unsubscribe();
      unlisten?.();
    };
  });

  const hasFilter = $derived(filter.status !== null || (filter.search?.trim().length ?? 0) > 0);

  const emptyConfig = $derived(
    hasFilter
      ? {
          heading: 'Ничего не найдено',
          body: 'Попробуйте изменить фильтр статуса.',
          actionLabel: null as string | null,
          onAction: undefined as (() => void) | undefined,
        }
      : {
          heading: 'Принтеры ещё не добавлены',
          body: 'Запустите поиск принтеров в сети — система найдёт их по SNMP и заведёт автоматически.',
          actionLabel: authStore.user?.role === 'admin' ? 'Найти принтеры' : null,
          onAction: authStore.user?.role === 'admin' ? () => { discoveryOpen = true; } : undefined,
        },
  );
</script>

<div class="printers-page">
  <header class="page-header">
    <h1 class="page-title">Принтеры</h1>
  </header>

  <div class="page-content">
    <PrintersSearchAndTabs
      {filter}
      onFilterChange={(f) => {
        filter = f;
        selectedId = null;
      }}
      onDiscoveryClick={() => (discoveryOpen = true)}
      identity={authStore.user}
    />

    <PrintersMasterDetail>
      {#snippet master()}
        <PrintersList
          {items}
          loading={listLoading}
          selectedId={selectedId}
          onSelect={(id) => (selectedId = id)}
          emptyConfig={emptyConfig}
        />
      {/snippet}
      {#snippet detail()}
        <PrinterDetail
          printer={selectedPrinter}
          loading={detailLoading}
          onRefresh={() => {
            if (selectedId !== null) {
              printers.refresh(selectedId).then((dto) => {
                selectedPrinter = dto;
                pushToast('success', 'Данные принтера обновлены');
                void refresh();
              }).catch(() => {
                pushToast('error', 'Принтер не отвечает на SNMP. Проверьте доступность и community.');
              });
            }
          }}
        />
      {/snippet}
    </PrintersMasterDetail>
  </div>
</div>

<DiscoveryModal
  open={discoveryOpen}
  onClose={() => (discoveryOpen = false)}
  onSuccess={(_n) => {
    discoveryOpen = false;
    void refresh();
  }}
/>

<style lang="scss">
  .printers-page {
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

  .page-content {
    flex: 1;
    overflow: auto;
    padding: var(--space-lg) var(--space-xl);
  }
</style>

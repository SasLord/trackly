<script lang="ts">
  // Plan 06-04: корневой компонент раздела «Принтеры».
  // По паттерну CartridgesPage.svelte (Svelte 5 runes).
  // WS-интеграция: onMount → connectWs() + onWsEvent(handleWsEvent).
  // PrinterDetail и DiscoveryModal: Task 2b.
  import { onMount } from 'svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { authStore } from '$lib/stores/auth.svelte';
  import { connectWs, onWsEvent } from '$lib/api/ws';
  import Button from '$lib/components/Button.svelte';
  import PageHeader from '$lib/components/PageHeader.svelte';
  import { parseIdFromHash } from '$lib/utils/hashId';
  import PrintersMasterDetail from './PrintersMasterDetail.svelte';
  import PrintersSearchAndTabs from './PrintersSearchAndTabs.svelte';
  import PrintersList from './PrintersList.svelte';
  import PrinterDetail from './PrinterDetail.svelte';
  import DiscoveryModal from './DiscoveryModal.svelte';
  import PrinterCreateModal from './PrinterCreateModal.svelte';
  import { printers } from './api';
  import type { PrinterDto, PrinterFilter } from '../../bindings-phase6';
  import type { WsEvent } from '../../bindings-phase6';

  // GAP-8 (39-UAT.md, Прогон 3): cross-section focus from the Places
  // content-row «Перейти к принтеру» action — `#/devices?id=…`'s
  // `?id=` there is a `devices.id` (places_contents/PLC-06 returns the
  // underlying device row id for kind='printer' rows too — see
  // PlaceEntityViewModal.svelte's file-header comment), NOT a `printers.id`.
  // `printers.getByDeviceId` (already used elsewhere — GAP-12-13) resolves
  // the mapping; no backend change needed.
  const initialFocusDeviceId = parseIdFromHash();

  let items = $state<PrinterDto[]>([]);
  let listLoading = $state(false);
  let selectedId = $state<number | null>(null);
  let selectedPrinter = $state<PrinterDto | null>(null);
  let detailLoading = $state(false);
  let discoveryOpen = $state(false);
  let createOpen = $state(false);

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
    if (initialFocusDeviceId !== null) {
      // Best-effort — a stale/missing id (device deleted, no longer a
      // printer, etc.) just leaves selectedId at its default null: the list
      // loads normally and the detail panel shows its existing "Выберите
      // принтер" empty state (no crash, no broken panel).
      printers
        .getByDeviceId(initialFocusDeviceId)
        .then((dto) => {
          selectedId = dto.id;
        })
        .catch(() => {
          // Ignore — see comment above.
        });
    }
    // Connect WS for real-time notifications.
    let unlisten: (() => void) | undefined;
    connectWs()
      .then((fn) => {
        unlisten = fn;
      })
      .catch(() => {
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
          onAction:
            authStore.user?.role === 'admin'
              ? () => {
                  discoveryOpen = true;
                }
              : undefined,
        },
  );
</script>

<div class="printers-page">
  <PageHeader title="Принтеры">
    {#snippet actions()}
      {#if authStore.user?.role === 'admin' || authStore.user?.role === 'manager'}
        <Button variant="secondary" onclick={() => (createOpen = true)}>Завести принтер</Button>
      {/if}
      {#if authStore.user?.role === 'admin'}
        <Button variant="primary" onclick={() => (discoveryOpen = true)}>Найти принтеры</Button>
      {/if}
    {/snippet}
  </PageHeader>

  <div class="page-content">
    <PrintersSearchAndTabs
      {filter}
      onFilterChange={(f) => {
        filter = f;
        selectedId = null;
      }}
    />

    <PrintersMasterDetail>
      {#snippet master()}
        <PrintersList
          {items}
          loading={listLoading}
          {selectedId}
          onSelect={(id) => (selectedId = id)}
          {emptyConfig}
        />
      {/snippet}
      {#snippet detail()}
        <PrinterDetail
          printer={selectedPrinter}
          loading={detailLoading}
          onRefresh={() => {
            if (selectedId !== null) {
              printers
                .refresh(selectedId)
                .then((dto) => {
                  selectedPrinter = dto;
                  pushToast('success', 'Данные принтера обновлены');
                  void refresh();
                })
                .catch(() => {
                  pushToast(
                    'error',
                    'Принтер не отвечает на SNMP. Проверьте доступность и community.',
                  );
                });
            }
          }}
          onDeviceSaved={(result) => {
            // Quick 260820-rdj (UAT gap-closure round 1, defect 2): separate
            // from onRefresh above — this path never triggers an SNMP poll,
            // it just reconciles the list/detail after a «Данные устройства»
            // save (which may have converted the record's type).
            const PRINTER_TYPE_ID = 2;
            if (result && result.typeId !== PRINTER_TYPE_ID) {
              // No longer a printer — drop the selection (detail panel
              // clears via the existing $effect) and reload the list so the
              // now-stale row disappears.
              selectedId = null;
              void refresh();
              return;
            }
            // Type unchanged (still a printer) — reload the detail + list.
            if (selectedId !== null) {
              printers
                .get(selectedId)
                .then((dto) => {
                  selectedPrinter = dto;
                })
                .catch(() => {
                  // Non-fatal — list refresh below still runs.
                });
            }
            void refresh();
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

<PrinterCreateModal
  open={createOpen}
  onClose={() => (createOpen = false)}
  onSuccess={() => {
    createOpen = false;
    void refresh();
  }}
/>

<style lang="scss">
  .printers-page {
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

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
  import DeviceFormModal from '../devices/DeviceFormModal.svelte';
  import DocumentAcceptanceModal from '../acts/DocumentAcceptanceModal.svelte';
  import PdfPreviewModal from '../acts/PdfPreviewModal.svelte';
  import { notifyPlaceContentChanged } from '$lib/stores/placeContentEvents.svelte';
  import { printers } from './api';
  import type { PrinterDto, PrinterFilter } from '../../bindings-phase6';
  import type { WsEvent } from '../../bindings-phase6';
  import type { DeviceDto } from '../../bindings';

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
  // Фаза 41.1, план 18: цели пунктов меню строки принтера.
  let editTarget = $state<DeviceDto | null>(null);
  let acceptanceDevice = $state<DeviceDto | null>(null);
  let acceptancePayload = $state<{
    deviceId: number;
    giverName: string;
    receiverName: string;
    dateUtc: number;
    deviceName: string;
  } | null>(null);

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

  // «Опросить сейчас»: общее действие кнопки панели справа и пункта меню строки.
  // Обновляет панель только если опрошенный принтер сейчас выбран.
  function pollPrinter(id: number) {
    printers
      .refresh(id)
      .then((dto) => {
        if (selectedId === id) selectedPrinter = dto;
        pushToast('success', 'Данные принтера обновлены');
        void refresh();
      })
      .catch(() => {
        pushToast('error', 'Принтер не отвечает на SNMP. Проверьте доступность и community.');
      });
  }

  // Quick 260820-rdj (UAT gap-closure round 1, defect 2): отдельно от опроса —
  // сохранение «Данных устройства» SNMP не запускает, только сверяет список и
  // панель (тип записи мог смениться). Общая функция панели и модалки правки
  // из меню строки.
  function handleDeviceSaved(result?: { typeId: number }) {
    const PRINTER_TYPE_ID = 2;
    if (result && result.typeId !== PRINTER_TYPE_ID) {
      // Больше не принтер — сбросить выбор (панель очистится через $effect) и
      // перечитать список, чтобы устаревшая строка исчезла.
      selectedId = null;
      void refresh();
      return;
    }
    // Тип не изменился — перечитать панель и список.
    if (selectedId !== null) {
      printers
        .get(selectedId)
        .then((dto) => {
          selectedPrinter = dto;
        })
        .catch(() => {
          // Некритично — перечитывание списка ниже всё равно выполняется.
        });
    }
    void refresh();
  }

  // Удаление и добавление в группу из меню строки: перечитать список; если
  // выбранного принтера в нём больше нет (мягко удалён вместе с устройством,
  // D-37), сбросить выбор.
  async function handleMenuChanged() {
    await refresh();
    if (selectedId !== null && !items.some((p) => p.id === selectedId)) {
      selectedId = null;
    }
  }

  // DEV-14 «Печать документа приёма» из меню строки принтера.
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
          onPoll={pollPrinter}
          onEdit={(d) => (editTarget = d)}
          onChanged={handleMenuChanged}
          onPrintAcceptance={handlePrintAcceptance}
        />
      {/snippet}
      {#snippet detail()}
        <PrinterDetail
          printer={selectedPrinter}
          loading={detailLoading}
          onRefresh={() => {
            if (selectedId !== null) pollPrinter(selectedId);
          }}
          onDeviceSaved={handleDeviceSaved}
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

<DeviceFormModal
  open={createOpen}
  target={null}
  initialTypeId={2}
  onClose={() => (createOpen = false)}
  onSaved={(result) => {
    createOpen = false;
    const newPlaceId = result?.placeId ?? null;
    if (newPlaceId !== null) notifyPlaceContentChanged([newPlaceId]);
    void refresh();
  }}
/>

<!-- Правка устройства из меню строки принтера. -->
<DeviceFormModal
  open={editTarget !== null}
  target={editTarget}
  onClose={() => (editTarget = null)}
  onSaved={(result) => {
    editTarget = null;
    const placeId = result?.placeId ?? null;
    if (placeId !== null) notifyPlaceContentChanged([placeId]);
    handleDeviceSaved(result);
  }}
/>

<!-- DEV-14: промежуточная модалка собирает сдающего/принимающего/дату, затем
     PdfPreviewModal в режиме acceptance. -->
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

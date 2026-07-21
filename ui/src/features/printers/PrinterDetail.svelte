<script lang="ts">
  // Plan 06-04: детальная панель принтера.
  // Секции: header (имя, статус-badge, alert-баннер, «Обновить сейчас»),
  //   «Уровни тонера/чернил» (TonerGauge), «Страничные счётчики», «Установленный картридж» (PRN-07),
  //   «История статусов» (PRN-05), Метаданные.
  // По паттерну CartridgeDetail.svelte.
  // Plan 27-07 (D-01): rebuilt on the shared DetailPanel/DetailSection primitives
  // (extracted in 27-01) per CartridgeDetail.svelte precedent — bespoke
  // container/header/body/section wrapper classes removed; detail surface
  // (former container background) dropped — the PrintersMasterDetail wrapper
  // now owns the panel surface (D-02). Section-internal markup
  // (.counter-row/.compat-agg-row/.meta-row/.reading-row) kept verbatim —
  // only re-clothed inside DetailSection, content/fields not removed (SC #4).
  // Async readings/aggregates data-loading below (unchanged — data, not visual).
  import Button from '$lib/components/Button.svelte';
  import Badge from '$lib/components/Badge.svelte';
  import Spinner from '$lib/components/Spinner.svelte';
  import DetailPanel from '$lib/components/DetailPanel.svelte';
  import DetailSection from '$lib/components/DetailSection.svelte';
  import TonerGauge from './TonerGauge.svelte';
  import PrinterAlertBanner from './PrinterAlertBanner.svelte';
  import { printers } from './api';
  import { devices } from '../devices/api';
  import { cartridges } from '../cartridges/api';
  import DeviceFormModal from '../devices/DeviceFormModal.svelte';
  import type { PrinterDto, PrinterReadingDto } from '../../bindings-phase6';
  import type { CompatibleModelAggregateDto, DeviceDto, CartridgeDto } from '../../bindings';

  interface Props {
    printer: PrinterDto | null;
    loading: boolean;
    onRefresh: () => void;
  }

  const { printer, loading, onRefresh }: Props = $props();

  let refreshing = $state(false);
  let readings = $state<PrinterReadingDto[]>([]);
  let readingsLoading = $state(false);

  // Load readings when printer changes.
  $effect(() => {
    const p = printer;
    if (p === null) {
      readings = [];
      return;
    }
    readingsLoading = true;
    printers
      .getReadings(p.id)
      .then((rows) => {
        readings = rows;
      })
      .catch(() => {
        readings = [];
      })
      .finally(() => {
        readingsLoading = false;
      });
  });

  // R4/D-07: read-only совместимые модели картриджей — агрегаты по статусам
  // (заменяет удалённый V029 per-device чеклист-редактор).
  let compatAggregates = $state<CompatibleModelAggregateDto[]>([]);
  let compatLoading = $state(false);

  $effect(() => {
    const p = printer;
    if (p === null) {
      compatAggregates = [];
      return;
    }
    compatLoading = true;
    printers
      .getCompatibleAggregates(p.deviceId)
      .then((res) => {
        compatAggregates = res.models;
      })
      .catch(() => {
        compatAggregates = [];
      })
      .finally(() => {
        compatLoading = false;
      });
  });

  // R5/D-08/D-09: блок данных устройства + диалог редактирования (DeviceFormModal).
  let deviceData = $state<DeviceDto | null>(null);
  let deviceLoading = $state(false);
  let deviceEditOpen = $state(false);

  $effect(() => {
    const p = printer;
    if (p === null) {
      deviceData = null;
      return;
    }
    deviceLoading = true;
    devices
      .get(p.deviceId)
      .then((d) => {
        deviceData = d;
      })
      .catch(() => {
        deviceData = null;
      })
      .finally(() => {
        deviceLoading = false;
      });
  });

  // R6: установленный картридж по коду + наименованию модели (без числового id).
  let installedCartridge = $state<CartridgeDto | null>(null);

  $effect(() => {
    const p = printer;
    if (p === null || p.currentCartridgeId === null) {
      installedCartridge = null;
      return;
    }
    cartridges
      .get(p.currentCartridgeId)
      .then((c) => {
        installedCartridge = c;
      })
      .catch(() => {
        installedCartridge = null;
      });
  });

  // Badge variant по status.
  type BadgeVariant = 'success' | 'warning' | 'destructive' | 'default';

  const statusVariant = $derived<BadgeVariant>(
    printer?.status === 'ok' || printer?.status === 'online'
      ? 'success'
      : printer?.status === 'warning'
        ? 'warning'
        : printer?.status === 'error'
          ? 'destructive'
          : 'default',
  );

  const statusLabel = $derived<string>(
    printer?.status === 'ok' || printer?.status === 'online'
      ? 'В сети'
      : printer?.status === 'warning'
        ? 'Предупреждение'
        : printer?.status === 'error'
          ? 'Ошибка'
          : printer?.status === 'offline'
            ? 'Не в сети'
            : 'Нет данных',
  );

  const displayName = $derived<string>(
    printer ? (printer.deviceName ?? `Принтер #${printer.id}`) : '',
  );

  // Parse tonerLevels into sorted entries for display.
  const tonerEntries = $derived<[string, number | null][]>(
    printer?.tonerLevels ? Object.entries(printer.tonerLevels) : [],
  );

  // Relative time helper.
  function relativeTime(utcSeconds: number | null): string {
    if (utcSeconds === null) return 'никогда';
    const nowSec = Math.floor(Date.now() / 1000);
    const diff = nowSec - utcSeconds;
    if (diff < 60) return 'только что';
    if (diff < 3600) return `${Math.floor(diff / 60)} мин. назад`;
    if (diff < 86400) return `${Math.floor(diff / 3600)} ч. назад`;
    return `${Math.floor(diff / 86400)} дн. назад`;
  }

  // Format reading timestamp.
  function formatTs(utcSeconds: number): string {
    const d = new Date(utcSeconds * 1000);
    const pad = (n: number) => String(n).padStart(2, '0');
    return `${pad(d.getUTCDate())}.${pad(d.getUTCMonth() + 1)}.${d.getUTCFullYear()} ${pad(d.getUTCHours())}:${pad(d.getUTCMinutes())}`;
  }

  // Map status string to Russian label.
  function statusRu(s: string): string {
    return (
      {
        ok: 'В сети',
        online: 'В сети',
        warning: 'Предупреждение',
        error: 'Ошибка',
        offline: 'Не в сети',
      }[s] ?? s
    );
  }

  async function handleRefresh() {
    refreshing = true;
    try {
      await onRefresh();
    } finally {
      refreshing = false;
    }
  }

  const panelTitle = $derived<string | undefined>(printer ? displayName : undefined);
</script>

{#if loading}
  <div class="detail-loading" aria-live="polite">
    <Spinner size="md" />
    <span>Загружаем данные…</span>
  </div>
{:else}
  <DetailPanel
    title={panelTitle}
    empty={printer === null}
    emptyTitle="Выберите принтер"
    emptyBody="Выберите принтер слева, чтобы увидеть уровни тонера, счётчики и историю."
  >
    {#snippet actions()}
      {#if printer}
        <Button variant="primary" size="sm" loading={refreshing} onclick={handleRefresh}>
          Обновить сейчас
        </Button>
      {/if}
    {/snippet}

    {#if printer}
      <div class="title-badges">
        <Badge variant={statusVariant}>{statusLabel}</Badge>
      </div>
      <PrinterAlertBanner
        hasAlert={printer.hasAlert}
        alertType={printer.alertType as 'offline' | 'error' | null}
        lastSeenUtc={printer.lastSeenUtc}
      />

      <!-- Секция: уровни тонера/чернил -->
      {#if tonerEntries.length > 0}
        <DetailSection heading="Уровни тонера/чернил">
          <div class="toner-list">
            {#each tonerEntries as [label, level] (label)}
              <TonerGauge {label} {level} encoding="percent" />
            {/each}
          </div>
        </DetailSection>
      {/if}

      <!-- Секция: страничные счётчики -->
      <DetailSection heading="Страничные счётчики">
        {#if printer.pageCount !== null}
          <p class="counter-row">
            <span class="counter-label">Всего напечатано</span>
            <span class="counter-value" style="font-variant-numeric: tabular-nums">
              {printer.pageCount}
            </span>
          </p>
        {:else}
          <p class="muted">Данные недоступны</p>
        {/if}
      </DetailSection>

      <!-- Секция: установленный картридж (PRN-07, R6 — код+наименование) -->
      <DetailSection heading="Установленный картридж">
        {#if printer.currentCartridgeId !== null}
          {#if installedCartridge !== null}
            <p class="cartridge-row">
              {installedCartridge.code} — {installedCartridge.model_brand}
              {installedCartridge.model_name}
            </p>
          {:else}
            <p class="cartridge-row">…</p>
          {/if}
        {:else}
          <p class="muted">Картридж не закреплён</p>
        {/if}
      </DetailSection>

      <!-- Секция: совместимые модели картриджей (R4/D-07 — read-only агрегаты) -->
      <DetailSection heading="Совместимые модели картриджей">
        {#if compatLoading}
          <div class="readings-loading"><Spinner size="sm" /></div>
        {:else if compatAggregates.length === 0}
          <p class="muted">Совместимость не настроена</p>
        {:else}
          {#each compatAggregates as agg (agg.modelId)}
            <p class="compat-agg-row">
              {agg.brand}
              {agg.model}: На складе {agg.inStock}, На заправке {agg.atRefill}, В работе {agg.inUse}
            </p>
          {/each}
        {/if}
      </DetailSection>

      <!-- Секция: история статусов (PRN-05) -->
      <DetailSection heading="История статусов">
        {#if readingsLoading}
          <div class="readings-loading"><Spinner size="sm" /></div>
        {:else if readings.length === 0}
          <div class="readings-empty">
            <p class="muted">Опросов ещё не было</p>
            <p class="muted-hint">Данные появятся после первого успешного опроса принтера.</p>
            <Button variant="secondary" size="sm" onclick={handleRefresh}>Обновить сейчас</Button>
          </div>
        {:else}
          <ul class="readings-list">
            {#each readings as row (row.id)}
              <li class="reading-row">
                <span class="reading-ts" style="font-variant-numeric: tabular-nums">
                  {formatTs(row.tsUtc)}
                </span>
                <span class="reading-sep">—</span>
                <span class="reading-status">{statusRu(row.status)}</span>
              </li>
            {/each}
          </ul>
        {/if}
      </DetailSection>

      <!-- Секция: данные устройства (R5, D-08/D-09) -->
      <DetailSection>
        <div class="section-heading-row">
          <h3 class="section-heading">Данные устройства</h3>
          <Button variant="secondary" size="sm" onclick={() => (deviceEditOpen = true)}>
            Редактировать
          </Button>
        </div>
        {#if deviceLoading}
          <div class="readings-loading"><Spinner size="sm" /></div>
        {:else}
          <div class="meta-row">
            <span class="meta-label">Инвентарный №</span>
            <span class="meta-value"
              ><span class="tr-mono">{deviceData?.inventory_no ?? '—'}</span></span
            >
          </div>
          <div class="meta-row">
            <span class="meta-label">Серийный №</span>
            <span class="meta-value"
              ><span class="tr-mono">{deviceData?.serial_no ?? '—'}</span></span
            >
          </div>
          <div class="meta-row">
            <span class="meta-label">Расположение</span>
            <span class="meta-value">{deviceData?.location ?? '—'}</span>
          </div>
          <div class="meta-row">
            <span class="meta-label">Состояние</span>
            <span class="meta-value">{deviceData?.state ?? '—'}</span>
          </div>
        {/if}
      </DetailSection>

      <!-- Метаданные -->
      <DetailSection>
        {#if printer.ipAddress}
          <div class="meta-row">
            <span class="meta-label">IP-адрес</span>
            <span class="meta-value" style="font-variant-numeric: tabular-nums">
              {printer.ipAddress}
            </span>
          </div>
        {:else if printer.usbHostDeviceId}
          <div class="meta-row">
            <span class="meta-label">Подключение</span>
            <span class="meta-value">Подключён по USB к: устройство #{printer.usbHostDeviceId}</span
            >
          </div>
        {/if}
        {#if printer.vendor}
          <div class="meta-row">
            <span class="meta-label">Производитель / Модель</span>
            <span class="meta-value">{printer.vendor}</span>
          </div>
        {/if}
        {#if printer.lastSeenUtc}
          <div class="meta-row">
            <span class="meta-label">Последний опрос</span>
            <span class="meta-value muted">{relativeTime(printer.lastSeenUtc)}</span>
          </div>
        {/if}
      </DetailSection>

      <DeviceFormModal
        open={deviceEditOpen}
        target={deviceData}
        onClose={() => (deviceEditOpen = false)}
        onSaved={() => {
          deviceEditOpen = false;
          if (printer) {
            devices.get(printer.deviceId).then((d) => (deviceData = d));
          }
        }}
      />
    {/if}
  </DetailPanel>
{/if}

<style lang="scss">
  .detail-loading {
    height: 100%;
    overflow: auto;
    padding: var(--tr-space-xl);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--tr-space-md);
    min-height: 320px;
    text-align: center;
    color: var(--tr-text-secondary);
  }

  .title-badges {
    display: flex;
    align-items: center;
    gap: var(--tr-space-xs);
    flex-wrap: wrap;
    margin-bottom: var(--tr-space-md);
  }

  .section-heading {
    margin: 0;
    font-size: var(--tr-font-size-body);
    font-weight: var(--tr-font-weight-semibold);
    color: var(--tr-text-primary);
  }

  .section-heading-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--tr-space-xs);
    margin-bottom: var(--tr-space-md);
  }

  .toner-list {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-xs);
  }

  .counter-row {
    display: flex;
    gap: var(--tr-space-md);
    margin: 0;
    font-size: var(--tr-font-size-body);
    align-items: baseline;
  }

  .counter-label {
    color: var(--tr-text-secondary);
  }

  .counter-value {
    font-weight: var(--tr-font-weight-semibold);
    color: var(--tr-text-primary);
  }

  .cartridge-row {
    margin: 0;
    font-size: var(--tr-font-size-body);
    font-weight: var(--tr-font-weight-semibold);
    color: var(--tr-text-primary);
  }

  .compat-agg-row {
    margin: 0;
    font-size: var(--tr-font-size-body);
    color: var(--tr-text-primary);
  }

  .muted {
    color: var(--tr-text-tertiary);
    font-size: var(--tr-font-size-body);
    margin: 0;
  }

  .muted-hint {
    color: var(--tr-text-tertiary);
    font-size: var(--tr-font-size-label);
    margin: 0;
  }

  .readings-loading {
    display: flex;
    justify-content: center;
    padding: var(--tr-space-md);
  }

  .readings-empty {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-2xs);
  }

  .readings-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .reading-row {
    display: flex;
    gap: var(--tr-space-xs);
    align-items: center;
    height: var(--row-height-dense, 32px);
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-secondary);
    border-bottom: 1px solid var(--tr-border);
    padding: 0 var(--tr-space-2xs);

    &:last-child {
      border-bottom: none;
    }
  }

  .reading-ts {
    color: var(--tr-text-tertiary);
    white-space: nowrap;
    min-width: 120px;
  }

  .reading-sep {
    color: var(--tr-text-tertiary);
  }

  .reading-status {
    color: var(--tr-text-primary);
  }

  .meta-row {
    display: flex;
    gap: var(--tr-space-md);
    font-size: var(--tr-font-size-label);
    align-items: baseline;
  }

  .meta-label {
    color: var(--tr-text-secondary);
    min-width: 160px;
    flex-shrink: 0;
  }

  .meta-value {
    color: var(--tr-text-primary);

    &.muted {
      color: var(--tr-text-tertiary);
    }
  }
</style>

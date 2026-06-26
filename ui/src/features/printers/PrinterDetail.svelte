<script lang="ts">
  // Plan 06-04: детальная панель принтера.
  // Секции: header (имя, статус-badge, alert-баннер, «Обновить сейчас»),
  //   «Уровни тонера/чернил» (TonerGauge), «Страничные счётчики», «Установленный картридж» (PRN-07),
  //   «История статусов» (PRN-05), Метаданные.
  // По паттерну CartridgeDetail.svelte.
  import Button from '$lib/components/Button.svelte';
  import Badge from '$lib/components/Badge.svelte';
  import Spinner from '$lib/components/Spinner.svelte';
  import TonerGauge from './TonerGauge.svelte';
  import PrinterAlertBanner from './PrinterAlertBanner.svelte';
  import { printers } from './api';
  import type { PrinterDto, PrinterReadingDto } from '../../bindings-phase6';
  import type { CompatibleModelAggregateDto } from '../../bindings';

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
      { ok: 'В сети', online: 'В сети', warning: 'Предупреждение', error: 'Ошибка', offline: 'Не в сети' }[s] ?? s
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
</script>

<div class="printer-detail" aria-live="polite">
  {#if loading}
    <div class="loading">
      <Spinner size="md" />
      <span>Загружаем данные…</span>
    </div>
  {:else if printer === null}
    <div class="empty">
      <h2 class="empty-heading">Выберите принтер</h2>
      <p class="empty-body">
        Выберите принтер слева, чтобы увидеть уровни тонера, счётчики и историю.
      </p>
    </div>
  {:else}
    <!-- Header -->
    <header class="detail-header">
      <div class="title-row">
        <h2 class="detail-title">{displayName}</h2>
        <Badge variant={statusVariant}>{statusLabel}</Badge>
        <Button
          variant="primary"
          size="sm"
          loading={refreshing}
          onclick={handleRefresh}
        >
          Обновить сейчас
        </Button>
      </div>
      <PrinterAlertBanner
        hasAlert={printer.hasAlert}
        alertType={printer.alertType as 'offline' | 'error' | null}
        lastSeenUtc={printer.lastSeenUtc}
      />
    </header>

    <div class="detail-body">
      <!-- Секция: уровни тонера/чернил -->
      {#if tonerEntries.length > 0}
        <section class="detail-section">
          <h3 class="section-heading">Уровни тонера/чернил</h3>
          <div class="toner-list">
            {#each tonerEntries as [label, level] (label)}
              <TonerGauge
                {label}
                {level}
                encoding="percent"
              />
            {/each}
          </div>
        </section>
      {/if}

      <!-- Секция: страничные счётчики -->
      <section class="detail-section">
        <h3 class="section-heading">Страничные счётчики</h3>
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
      </section>

      <!-- Секция: установленный картридж (PRN-07, D-PRN07-01) -->
      <section class="detail-section">
        <h3 class="section-heading">Установленный картридж</h3>
        {#if printer.currentCartridgeId !== null}
          <p class="cartridge-row">Картридж #{printer.currentCartridgeId}</p>
        {:else}
          <p class="muted">Картридж не закреплён</p>
        {/if}
      </section>

      <!-- Секция: совместимые модели картриджей (R4/D-07 — read-only агрегаты) -->
      <section class="detail-section">
        <h3 class="section-heading">Совместимые модели картриджей</h3>
        {#if compatLoading}
          <div class="readings-loading"><Spinner size="sm" /></div>
        {:else if compatAggregates.length === 0}
          <p class="muted">Совместимость не настроена</p>
        {:else}
          {#each compatAggregates as agg (agg.modelId)}
            <p class="compat-agg-row">
              {agg.brand} {agg.model}: На складе {agg.inStock}, На заправке {agg.atRefill}, В работе {agg.inUse}
            </p>
          {/each}
        {/if}
      </section>

      <!-- Секция: история статусов (PRN-05) -->
      <section class="detail-section">
        <h3 class="section-heading">История статусов</h3>
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
      </section>

      <!-- Метаданные -->
      <section class="detail-section meta-section">
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
            <span class="meta-value">Подключён по USB к: устройство #{printer.usbHostDeviceId}</span>
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
      </section>
    </div>
  {/if}
</div>

<style lang="scss">
  .printer-detail {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .loading,
  .empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-sm);
    padding: var(--space-2xl);
    text-align: center;
  }

  .empty-heading {
    margin: 0 0 var(--space-xs);
    font-size: var(--font-size-heading);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
  }

  .empty-body {
    margin: 0;
    color: var(--color-text-secondary);
    font-size: var(--font-size-body);
  }

  .detail-header {
    padding: var(--space-lg) var(--space-lg) var(--space-md);
    border-bottom: 1px solid var(--color-border);
    flex-shrink: 0;
  }

  .title-row {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    flex-wrap: wrap;
    margin-bottom: var(--space-sm);
  }

  .detail-title {
    margin: 0;
    font-size: var(--font-size-display);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
    line-height: var(--line-height-display);
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .detail-body {
    flex: 1;
    overflow: auto;
    padding: var(--space-lg);
    display: flex;
    flex-direction: column;
    gap: var(--space-xl);
  }

  .detail-section {
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
  }

  .section-heading {
    margin: 0;
    font-size: var(--font-size-body);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
  }

  .toner-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
  }

  .counter-row {
    display: flex;
    gap: var(--space-md);
    margin: 0;
    font-size: var(--font-size-body);
    align-items: baseline;
  }

  .counter-label {
    color: var(--color-text-secondary);
  }

  .counter-value {
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
  }

  .cartridge-row {
    margin: 0;
    font-size: var(--font-size-body);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
  }

  .compat-agg-row {
    margin: 0;
    font-size: var(--font-size-body);
    color: var(--color-text-primary);
  }

  .muted {
    color: var(--color-text-muted);
    font-size: var(--font-size-body);
    margin: 0;
  }

  .muted-hint {
    color: var(--color-text-muted);
    font-size: var(--font-size-label);
    margin: 0;
  }

  .readings-loading {
    display: flex;
    justify-content: center;
    padding: var(--space-md);
  }

  .readings-empty {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
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
    gap: var(--space-sm);
    align-items: center;
    height: var(--row-height-dense, 32px);
    font-size: var(--font-size-label);
    color: var(--color-text-secondary);
    border-bottom: 1px solid var(--color-border);
    padding: 0 var(--space-xs);

    &:last-child {
      border-bottom: none;
    }
  }

  .reading-ts {
    color: var(--color-text-muted);
    white-space: nowrap;
    min-width: 120px;
  }

  .reading-sep {
    color: var(--color-text-muted);
  }

  .reading-status {
    color: var(--color-text-primary);
  }

  .meta-section {
    gap: var(--space-xs);
  }

  .meta-row {
    display: flex;
    gap: var(--space-md);
    font-size: var(--font-size-label);
    align-items: baseline;
  }

  .meta-label {
    color: var(--color-text-secondary);
    min-width: 160px;
    flex-shrink: 0;
  }

  .meta-value {
    color: var(--color-text-primary);

    &.muted {
      color: var(--color-text-muted);
    }
  }
</style>

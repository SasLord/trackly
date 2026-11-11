<script lang="ts">
  // Фаза 41.4, план 32: попап «История ремонтов» устройства. Сводка, заявки
  // (новые первыми) с заходами и вложениями; печать полного отчёта идёт через
  // общий PdfPreviewModal (mode='repairDevice'). Все значения — интерполяцией
  // текста, без {@html}.
  import { push } from 'svelte-spa-router';
  import Modal from '$lib/components/Modal.svelte';
  import Button from '$lib/components/Button.svelte';
  import Badge from '$lib/components/Badge.svelte';
  import Spinner from '$lib/components/Spinner.svelte';
  import DetailSection from '$lib/components/DetailSection.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import PdfPreviewModal from '../acts/PdfPreviewModal.svelte';
  import AttachmentsBlock from '../attachments/AttachmentsBlock.svelte';
  import { repairs } from './api';
  import { requestStatusLabel, requestStatusVariant } from './repairLocks';
  import type { RepairDeviceHistoryDto } from '../../bindings';

  interface Props {
    open: boolean;
    deviceId: number;
    deviceName: string;
    onClose: () => void;
    /** Родитель закрывается после перехода к заявке. */
    onNavigate: () => void;
  }

  const { open, deviceId, deviceName, onClose, onNavigate }: Props = $props();

  let history = $state<RepairDeviceHistoryDto | null>(null);
  let loading = $state(false);
  let loadFailed = $state(false);
  let reportOpen = $state(false);
  let reportPreparing = $state(false);
  let loadToken = 0;

  $effect(() => {
    if (!open) return;
    const id = deviceId;
    const token = ++loadToken;
    loading = true;
    loadFailed = false;
    history = null;
    repairs
      .deviceHistory(id)
      .then((dto) => {
        if (token !== loadToken) return;
        history = dto;
      })
      .catch(() => {
        if (token !== loadToken) return;
        loadFailed = true;
      })
      .finally(() => {
        if (token === loadToken) loading = false;
      });
    return () => {
      loadToken++;
    };
  });

  const sortedRequests = $derived(
    history ? [...history.requests].sort((a, b) => b.createdAtUtc - a.createdAtUtc) : [],
  );

  function formatDate(utcSeconds: number): string {
    const d = new Date(utcSeconds * 1000);
    return `${String(d.getUTCDate()).padStart(2, '0')}.${String(d.getUTCMonth() + 1).padStart(2, '0')}.${d.getUTCFullYear()}`;
  }

  async function goToRequest(requestId: number) {
    // Сначала переход, затем закрытие — как «Перейти к…» в просмотре.
    await push(`#/requests?id=${requestId}`);
    onClose();
    onNavigate();
  }

  function printReport() {
    if (reportPreparing || !history) return;
    reportPreparing = true;
    reportOpen = true;
  }

  function onReportLoaded() {
    reportPreparing = false;
  }

  function onReportFailed() {
    reportPreparing = false;
    reportOpen = false;
    pushToast('error', 'Не удалось подготовить отчёт. Попробуйте ещё раз.');
  }

  function closeReport() {
    reportPreparing = false;
    reportOpen = false;
  }
</script>

<Modal {open} title="История ремонтов" size="wide" {onClose}>
  <div class="history">
    {#if loading}
      <div class="state"><Spinner size="md" /></div>
    {:else if loadFailed}
      <p class="state-text">
        Не удалось загрузить историю ремонтов. Закройте окно и попробуйте ещё раз.
      </p>
    {:else if history && history.requests.length === 0}
      <p class="state-text">Ремонтов не было</p>
    {:else if history}
      <div class="summary">
        <span>Заходов в сервис: {history.visitCount}</span>
        {#if history.totalCostDisplay}
          <span>Стоимость: {history.totalCostDisplay}</span>
        {/if}
      </div>
      {#each sortedRequests as req (req.requestId)}
        <DetailSection>
          <div class="req">
            <header class="req-head">
              <Button variant="link" size="sm" onclick={() => goToRequest(req.requestId)}>
                Заявка №{req.requestId}
              </Button>
              <Badge variant={requestStatusVariant(req.status)}>
                {requestStatusLabel(req.status)}
              </Badge>
            </header>
            <div class="meta">
              Создана {formatDate(req.createdAtUtc)}{req.requesterName
                ? `, автор: ${req.requesterName}`
                : ''}
            </div>
            {#if req.description}
              <div class="desc">{req.description}</div>
            {/if}
            {#each [...req.visits].sort((a, b) => b.seq - a.seq) as visit (visit.id)}
              <article class="visit" aria-label={`Заход ${visit.seq}`}>
                <div class="visit-title">
                  <span class="visit-name">Заход {visit.seq}</span>
                  <span class="muted">· Акт № {visit.actNumber}</span>
                </div>
                <div class="muted">
                  {visit.servicePlacePath ?? '—'} · ушло {formatDate(visit.sentAtUtc)} · вернулось {visit.returnedAtUtc !==
                  null
                    ? formatDate(visit.returnedAtUtc)
                    : '—'}
                </div>
                {#if visit.faultDescription}
                  <div>Неисправность: {visit.faultDescription}</div>
                {/if}
                {#if visit.worksDone}
                  <div>Выполнено: {visit.worksDone}</div>
                {/if}
                {#if visit.costDisplay}
                  <div>Стоимость: {visit.costDisplay}</div>
                {/if}
                <div class="muted">
                  Скан результатов: {visit.scanCount > 0 ? `есть (${visit.scanCount})` : 'нет'}
                </div>
              </article>
            {/each}
            <AttachmentsBlock parentKind="request" parentId={req.requestId} readonly />
          </div>
        </DetailSection>
      {/each}
    {/if}
  </div>

  {#snippet footer()}
    <Button variant="secondary" onclick={onClose}>Закрыть</Button>
    <Button
      variant="primary"
      disabled={!history || history.requests.length === 0}
      loading={reportPreparing}
      onclick={printReport}
    >
      Печать отчёта
    </Button>
  {/snippet}
</Modal>

<PdfPreviewModal
  open={reportOpen}
  actId={null}
  mode="repairDevice"
  repairDeviceId={deviceId}
  title={`Отчёт о ремонтах: ${deviceName}`}
  onClose={closeReport}
  onLoaded={onReportLoaded}
  onLoadError={onReportFailed}
/>

<style lang="scss">
  // Modal монтируется на месте вызова и может наследовать стили контекста —
  // явно сбрасываем перенос и выравнивание внутри своих блоков.
  .history {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-md);
    white-space: normal;
    text-align: left;
  }
  .state {
    display: flex;
    justify-content: center;
    padding: var(--tr-space-lg);
  }
  .state-text {
    margin: 0;
    color: var(--tr-text-secondary);
  }
  .summary {
    display: flex;
    gap: var(--tr-space-md);
    font-weight: 600;
    color: var(--tr-text-primary);
  }
  .req {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-xs);
  }
  .req-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--tr-space-sm);
  }
  .meta,
  .muted {
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-secondary);
  }
  .desc {
    color: var(--tr-text-primary);
    overflow-wrap: anywhere;
  }
  .visit {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-2xs);
    padding: var(--tr-space-md);
    background: var(--tr-surface);
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-md);
    overflow-wrap: anywhere;
  }
  .visit-name {
    font-weight: 600;
    color: var(--tr-text-primary);
  }
</style>

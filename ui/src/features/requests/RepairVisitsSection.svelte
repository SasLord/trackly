<script lang="ts">
  // Фаза 41.4, план 17 (D-13, D-29, D-27, D-28, D-06): секция «Заходы в сервис».
  // Карточки заходов (новые сверху); печать акта идёт через общий PdfPreviewModal
  // (mode='repair') — тот же предпросмотр и печать в обоих режимах, что у актов
  // приёма-передачи. Права: кнопки только у canManage; сервер авторизует независимо.
  import DetailSection from '$lib/components/DetailSection.svelte';
  import DetailField from '$lib/components/DetailField.svelte';
  import Badge from '$lib/components/Badge.svelte';
  import Button from '$lib/components/Button.svelte';
  import ActionMenu from '$lib/components/ActionMenu.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { formatFileSize } from '$lib/utils/formatFileSize';
  import PdfPreviewModal from '../acts/PdfPreviewModal.svelte';
  import { attachments, uploadAttachment } from '../attachments/api';
  import { repairs } from './api';
  import type { RepairVisitDto } from '../../bindings';

  interface Props {
    visits: RepairVisitDto[];
    requestId: number;
    /** Администратор или Специалист. */
    canManage: boolean;
    onScrollToAttachments: () => void;
    /** Состояние ремонта изменилось (приложен скан) — перечитать. */
    onChanged: () => void;
  }

  const { visits, requestId, canManage, onScrollToAttachments, onChanged }: Props = $props();

  /** Новые сверху. */
  const sorted = $derived([...visits].sort((a, b) => b.seq - a.seq));
  const heading = $derived(
    visits.length >= 2 ? `Заходы в сервис (${visits.length})` : 'Заходы в сервис',
  );

  /** Заход, акт которого открыт в предпросмотре. */
  let previewVisit = $state<RepairVisitDto | null>(null);
  /** Заход, акт которого сейчас готовится (кнопка в состоянии loading). */
  let preparingId = $state<number | null>(null);
  let uploadingId = $state<number | null>(null);
  let scanErrors = $state<Record<number, string>>({});
  let fileInputs = $state<Record<number, HTMLInputElement | null>>({});

  function formatDate(utcSeconds: number): string {
    const d = new Date(utcSeconds * 1000);
    return `${String(d.getUTCDate()).padStart(2, '0')}.${String(d.getUTCMonth() + 1).padStart(2, '0')}.${d.getUTCFullYear()}`;
  }

  function printAct(visit: RepairVisitDto) {
    if (preparingId !== null) return;
    preparingId = visit.id;
    previewVisit = visit;
  }

  function onActLoaded() {
    preparingId = null;
  }

  function onActFailed() {
    preparingId = null;
    previewVisit = null;
    pushToast('error', 'Не удалось подготовить акт. Попробуйте ещё раз.');
  }

  function closePreview() {
    preparingId = null;
    previewVisit = null;
  }

  function scanError(name: string, code: string, size: number, maxBytes: number): string {
    switch (code) {
      case 'too_large':
        return `«${name}» — больше ${Math.round(maxBytes / (1024 * 1024))} МБ (размер файла ${formatFileSize(size)}).`;
      case 'empty':
        return `«${name}» — файл пустой.`;
      case 'type_not_allowed':
        return `«${name}» — такой файл приложить нельзя. Подходят фото, PDF, документы, архивы.`;
      case 'heic_failed':
        return `«${name}» — не удалось преобразовать фото HEIC. Сохраните его в JPEG и приложите снова.`;
      case 'forbidden':
        return `«${name}» — нет доступа к карточке или она удалена.`;
      default:
        return `«${name}» — не удалось загрузить. Проверьте подключение и попробуйте ещё раз.`;
    }
  }

  async function onScanPicked(visit: RepairVisitDto, event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const file = input.files?.[0] ?? null;
    input.value = '';
    if (!file || uploadingId !== null) return;
    uploadingId = visit.id;
    scanErrors = { ...scanErrors, [visit.id]: '' };
    try {
      const res = await uploadAttachment('request', requestId, file);
      if (res.rejectCode !== null || res.attachment === null) {
        const code = res.rejectCode ?? 'network';
        let maxBytes = 0;
        if (code === 'too_large') {
          try {
            maxBytes = (await attachments.getLimit()).maxBytes;
          } catch {
            maxBytes = 0;
          }
        }
        scanErrors = { ...scanErrors, [visit.id]: scanError(file.name, code, file.size, maxBytes) };
        return;
      }
      try {
        await repairs.linkScans(visit.id, [res.attachment.id]);
      } catch {
        scanErrors = {
          ...scanErrors,
          [visit.id]:
            'Файл загружен, но не привязан к заходу. Он доступен в блоке «Вложения» заявки.',
        };
      }
      onChanged();
    } finally {
      uploadingId = null;
    }
  }
</script>

{#if visits.length > 0}
  <DetailSection {heading}>
    <div class="visits">
      {#each sorted as visit (visit.id)}
        {@const returned = visit.returnedAtUtc !== null}
        <article class="visit" aria-label={`Заход ${visit.seq}`}>
          <header class="visit-head">
            <span class="visit-title">
              <span class="visit-name">Заход {visit.seq}</span>
              <span class="sep">·</span>
              <span class="tr-mono act-no">Акт № {visit.actNumber}</span>
            </span>
            <Badge variant={returned ? 'success' : 'warning'}>
              {returned ? 'Возвращён' : 'В сервисе'}
            </Badge>
          </header>
          <div class="visit-meta">
            {visit.servicePlacePath ?? '—'} · ушло {formatDate(visit.sentAtUtc)} · вернулось {visit.returnedAtUtc !==
            null
              ? formatDate(visit.returnedAtUtc)
              : '—'}
          </div>
          <div class="fields">
            <DetailField label="Неисправность" value={visit.faultDescription || null} />
            <DetailField label="Выполнено" value={visit.worksDone} />
            <DetailField label="Стоимость" value={visit.costDisplay} />
            <DetailField label="Скан результатов">
              {#if visit.scanCount > 0}
                <Button variant="link" size="sm" onclick={onScrollToAttachments}>
                  есть ({visit.scanCount})
                </Button>
              {:else}
                нет
              {/if}
            </DetailField>
          </div>
          {#if canManage}
            <div class="visit-actions">
              {#if returned}
                <Button
                  variant="link"
                  size="sm"
                  loading={uploadingId === visit.id}
                  disabled={uploadingId !== null}
                  onclick={() => fileInputs[visit.id]?.click()}
                >
                  Приложить скан
                </Button>
                <input
                  class="file-input"
                  type="file"
                  tabindex="-1"
                  aria-hidden="true"
                  bind:this={fileInputs[visit.id]}
                  onchange={(e) => onScanPicked(visit, e)}
                />
              {/if}
              <span class="spacer"></span>
              <Button
                variant="secondary"
                size="sm"
                loading={preparingId === visit.id}
                disabled={preparingId !== null}
                onclick={() => printAct(visit)}
              >
                Печать акта
              </Button>
              <ActionMenu variant="ghost-sm" label="Действия с заходом">
                <button type="button" role="menuitem" onclick={() => printAct(visit)}>
                  Печать акта
                </button>
              </ActionMenu>
            </div>
          {/if}
          {#if scanErrors[visit.id]}
            <span class="scan-error" role="alert">{scanErrors[visit.id]}</span>
          {/if}
        </article>
      {/each}
    </div>
  </DetailSection>
{/if}

{#if canManage}
  <PdfPreviewModal
    open={previewVisit !== null}
    actId={null}
    mode="repair"
    repairVisitId={previewVisit?.id ?? null}
    title={previewVisit
      ? `Акт передачи в ремонт № ${previewVisit.actNumber}`
      : 'Акт передачи в ремонт'}
    onClose={closePreview}
    onLoaded={onActLoaded}
    onLoadError={onActFailed}
  />
{/if}

<style lang="scss">
  .visits {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-sm);
  }
  .visit {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-xs);
    padding: var(--tr-space-md);
    background: var(--tr-surface);
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-md);
  }
  .visit-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--tr-space-sm);
  }
  .visit-title {
    display: inline-flex;
    align-items: baseline;
    gap: var(--tr-space-xs);
  }
  .visit-name {
    font-weight: 600;
    color: var(--tr-text-primary);
  }
  .sep,
  .act-no {
    color: var(--tr-text-secondary);
  }
  .visit-meta {
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-secondary);
  }
  .fields {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-2xs);
  }
  .visit-actions {
    display: flex;
    align-items: center;
    gap: var(--tr-space-xs);
  }
  .spacer {
    flex: 1;
  }
  .file-input {
    display: none;
  }
  .scan-error {
    font-size: var(--tr-font-size-label);
    color: var(--tr-danger-text);
  }
</style>

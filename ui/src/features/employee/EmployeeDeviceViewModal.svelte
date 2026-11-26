<script lang="ts">
  // Карточка устройства сотрудника, только чтение (фаза 41.6, D-15..D-17, UI-SPEC §13).
  // Данные приходят строкой списка по id; отдельного запроса нет. Модалка рендерится
  // вне <td> (на уровне страницы): Modal наследует стили ячейки, поэтому корень тела
  // явно задаёт выравнивание и перенос.
  import Modal from '@trackly/kit/components/Modal.svelte';
  import Badge from '@trackly/kit/components/Badge.svelte';
  import Button from '@trackly/kit/components/Button.svelte';
  import DetailSection from '@trackly/kit/components/DetailSection.svelte';
  import DetailField from '@trackly/kit/components/DetailField.svelte';
  import TonerGauge from '../printers/TonerGauge.svelte';
  import { statusTone } from '$lib/deviceStatusTone';
  import { formatDateTimeShort } from '@trackly/kit/utils/date';
  import { requestTargetHref } from '../requests/requestTarget';
  import { JOIN_REPAIR_FOOTER_NOTE } from './employeeDeviceMenu';
  import type { EmployeeDeviceDto } from '../../bindings';

  interface Props {
    open: boolean;
    row: EmployeeDeviceDto | null;
    onClose: () => void;
    onRepair: (row: EmployeeDeviceDto) => void;
    onCartridge: (row: EmployeeDeviceDto) => void;
    onGotoRequest: (id: number) => void;
  }

  const { open, row, onClose, onRepair, onCartridge, onGotoRequest }: Props = $props();

  const ORIGIN_LABEL: Record<string, string> = {
    usb: 'USB',
    link: 'Ссылка',
    member: 'В составе',
  };

  const TYPE_LABEL: Record<string, string> = {
    ad_register: 'Регистрация AD',
    cartridge_replace: 'Замена картриджа',
    repair: 'Ремонт',
  };

  const STATUS_LABEL: Record<string, string> = {
    open: 'Создана',
    in_progress: 'В работе',
    in_repair: 'В ремонте',
    completed: 'Выполнена',
    cancelled: 'Отменена',
  };

  function typeLabel(t: string): string {
    return TYPE_LABEL[t] ?? 'Свободная форма';
  }
  function statusLabel(s: string): string {
    return STATUS_LABEL[s] ?? 'Отклонена';
  }

  const tone = $derived(statusTone(row?.statusCode));
  const originLabel = $derived(
    row?.printerOrigin ? (ORIGIN_LABEL[row.printerOrigin] ?? null) : null,
  );
  const armNames = $derived(row?.armNames.join(', ') ?? '');
  const hasTonerData = $derived(
    row?.toner != null && (row.toner.polledAtUtc !== null || row.toner.levels.length > 0),
  );
  const joinNote = $derived(row !== null && row.inRepair && row.myRepairRequestId === null);

  function handleRepair(): void {
    if (!row) return;
    const r = row;
    onClose();
    onRepair(r);
  }
  function handleCartridge(): void {
    if (!row) return;
    const r = row;
    onClose();
    onCartridge(r);
  }
  function handleGoto(id: number): void {
    onClose();
    onGotoRequest(id);
  }
  function handleLinkClick(e: MouseEvent, id: number): void {
    e.preventDefault();
    handleGoto(id);
  }
</script>

{#if row}
  <Modal {open} title={row.name} size="md" {onClose}>
    {#snippet titleExtra()}
      <Badge
        variant={tone.variant}
        appearance={tone.appearance}
        muted={tone.muted}
        title={row.statusName}>{row.statusName}</Badge
      >
    {/snippet}

    <div class="card-body">
      <DetailSection heading="Сведения">
        <div class="fields">
          <DetailField label="Тип" value={row.typeName} />
          <DetailField label="Модель" value={row.model} />
          <DetailField label="Инв. №">
            <span class="tr-mono">{row.inventoryNumber ?? '—'}</span>
          </DetailField>
          <DetailField label="Серийный №">
            <span class="tr-mono">{row.serialNumber ?? '—'}</span>
          </DetailField>
          <DetailField label="Место" value={row.placeFull} />
          <DetailField label="АРМ">
            {#if armNames}
              {#if originLabel}<Badge size="sm">{originLabel}</Badge>{/if}
              {armNames}
            {:else}
              —
            {/if}
          </DetailField>
        </div>
      </DetailSection>

      {#if row.isPrinter}
        <DetailSection heading="Тонер">
          {#if hasTonerData && row.toner}
            <div class="toner-list">
              {#each row.toner.levels as lvl (lvl.label)}
                <TonerGauge label={lvl.label} level={lvl.percent} encoding="percent" />
              {/each}
            </div>
            {#if row.toner.polledAtUtc !== null}
              <p class="note">Опрос: {formatDateTimeShort(row.toner.polledAtUtc)}</p>
            {/if}
          {:else}
            <p class="muted">Принтер ещё не опрашивался</p>
          {/if}
        </DetailSection>
      {/if}

      <DetailSection heading="Мои заявки по устройству">
        {#if row.myRequests.length === 0}
          <p class="muted">Заявок по устройству, где вы участвуете, нет.</p>
        {:else}
          <ul class="req-list">
            {#each row.myRequests as req (req.id)}
              <li class="req-row">
                <span class="req-main">
                  <a
                    class="req-link"
                    href={requestTargetHref(req.id, 'info')}
                    onclick={(e) => handleLinkClick(e, req.id)}>№{req.id}</a
                  >
                  · {typeLabel(req.requestType)} · {statusLabel(req.status)}
                </span>
                <span class="req-date">{formatDateTimeShort(req.createdAtUtc)}</span>
              </li>
            {/each}
          </ul>
        {/if}
        {#if row.otherOpenRequest}
          <p class="other">По устройству уже открыта заявка другого сотрудника.</p>
        {/if}
      </DetailSection>
    </div>

    {#snippet footer()}
      <div class="footer">
        {#if joinNote}
          <span class="join-note">{JOIN_REPAIR_FOOTER_NOTE}</span>
        {/if}
        <div class="footer-actions">
          <Button variant="secondary" onclick={onClose}>Закрыть</Button>
          {#if row.isPrinter}
            <Button variant="secondary" onclick={handleCartridge}>Замена картриджа</Button>
          {/if}
          {#if row.myRepairRequestId !== null}
            {@const repairId = row.myRepairRequestId}
            <Button variant="primary" onclick={() => handleGoto(repairId)}>
              Перейти к заявке №{repairId}
            </Button>
          {:else}
            <Button variant="primary" onclick={handleRepair}>Заявка на ремонт</Button>
          {/if}
        </div>
      </div>
    {/snippet}
  </Modal>
{/if}

<style lang="scss">
  .card-body {
    text-align: left;
    white-space: normal;
    overflow-wrap: anywhere;
  }
  .fields {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--tr-space-md);
  }
  .toner-list {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-xs);
  }
  .note {
    margin: var(--tr-space-xs) 0 0;
    font-size: var(--tr-font-size-caption);
    color: var(--tr-text-tertiary);
  }
  .muted {
    margin: 0;
    font-size: var(--tr-font-size-body);
    color: var(--tr-text-secondary);
  }
  .other {
    margin: var(--tr-space-xs) 0 0;
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-secondary);
  }
  .req-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-xs);
  }
  .req-row {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: var(--tr-space-md);
    font-size: var(--tr-font-size-body);
    color: var(--tr-text-primary);
  }
  .req-link {
    color: var(--tr-accent-text);
    text-decoration: none;
    &:hover {
      text-decoration: underline;
    }
  }
  .req-date {
    flex-shrink: 0;
    font-size: var(--tr-font-size-caption);
    color: var(--tr-text-tertiary);
  }
  .footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--tr-space-md);
    width: 100%;
  }
  .footer-actions {
    display: flex;
    gap: var(--tr-space-xs);
    margin-left: auto;
  }
  .join-note {
    font-size: var(--tr-font-size-caption);
    color: var(--tr-text-secondary);
  }
</style>

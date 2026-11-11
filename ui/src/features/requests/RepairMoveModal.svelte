<script lang="ts">
  // Фаза 41.4, план 16 (D-01, D-18, D-23, D-25): попап «Перемещение устройства».
  // Открывается из «Принять в работу» (mode='accept') и «Забрать на склад»
  // (mode='toStorage'). Серверная валидация авторитетна; клиент только собирает выбор.
  import { untrack } from 'svelte';
  import Modal from '$lib/components/Modal.svelte';
  import Button from '$lib/components/Button.svelte';
  import Badge from '$lib/components/Badge.svelte';
  import DetailField from '$lib/components/DetailField.svelte';
  import PlacePicker from '$lib/components/PlacePicker.svelte';
  import { apiCall } from '$lib/api/client';
  import { pushToast } from '$lib/stores/toast.svelte';
  import {
    ensureLoaded as ensureStatusesLoaded,
    statusByCode,
    statusTone,
  } from '$lib/stores/deviceStatuses.svelte';
  import { repairs } from './api';
  import type {
    PlaceDto,
    RepairStateDto,
    RepairTransitionResultDto,
    RequestDto,
  } from '../../bindings';

  interface Props {
    open: boolean;
    mode: 'accept' | 'toStorage';
    request: RequestDto;
    repairState: RepairStateDto;
    onClose: () => void;
    onSuccess: (_result: RepairTransitionResultDto) => void;
  }

  const { open, mode, request, repairState, onClose, onSuccess }: Props = $props();

  const AWAITING_CODE = 'на_ремонте';
  const MAX_QUICK_PICK = 4;

  let placeId = $state<number | null>(null);
  let submitting = $state(false);
  let storageIds = $state<number[]>([]);
  let storageLabels = $state<Record<number, string>>({});

  // Сброс и загрузка при открытии. Читается только `open`; остальное — через untrack,
  // чтобы запись состояния внутри эффекта не создавала цикл.
  $effect(() => {
    if (!open) return;
    let cancelled = false;
    untrack(() => {
      placeId = null;
      submitting = false;
      storageIds = [];
      storageLabels = {};
    });
    void ensureStatusesLoaded();
    (async () => {
      try {
        const last = await repairs.lastPlaces();
        if (!cancelled && placeId === null) placeId = last.movePlaceId;
      } catch {
        // Предзаполнение — удобство: без него поле остаётся пустым.
      }
      try {
        const ids = (await apiCall<number[]>('cartridge_storage_place_ids', {})).slice(
          0,
          MAX_QUICK_PICK,
        );
        if (cancelled) return;
        storageIds = ids;
        const labels: Record<number, string> = {};
        await Promise.all(
          ids.map(async (id) => {
            try {
              const place = await apiCall<PlaceDto>('places_get', { id });
              labels[id] = place.full_path ?? place.name;
            } catch {
              labels[id] = `#${id}`;
            }
          }),
        );
        if (!cancelled) storageLabels = labels;
      } catch {
        if (!cancelled) storageIds = [];
      }
    })();
    return () => {
      cancelled = true;
    };
  });

  const awaitingLabel = $derived(statusByCode(AWAITING_CODE)?.name ?? 'Ожидает ремонта');
  const awaitingTone = $derived(statusTone(AWAITING_CODE));
  const warnedStatusCode = $derived(
    repairState.deviceStatusCode === 'на_списание' || repairState.deviceStatusCode === 'списано'
      ? repairState.deviceStatusCode
      : null,
  );
  const warnedStatusLabel = $derived(
    warnedStatusCode ? (statusByCode(warnedStatusCode)?.name ?? warnedStatusCode) : '',
  );
  const showAnchorNote = $derived(repairState.isAnchor && repairState.groupBlockSize > 0);
  const canMove = $derived(!submitting && placeId !== null);

  async function run(choice: { kind: 'move'; placeId: number } | { kind: 'stay' }) {
    if (submitting) return;
    submitting = true;
    try {
      const result =
        mode === 'accept'
          ? await repairs.accept(request.id, request.version, choice)
          : await repairs.moveToStorage(
              request.id,
              request.version,
              choice.kind === 'move' ? choice.placeId : (placeId as number),
            );
      if (choice.kind === 'move') {
        const where = result.state.currentPlacePath ?? storageLabels[choice.placeId] ?? '';
        pushToast('success', `Устройство перемещено: ${where}`);
      } else {
        pushToast('success', 'Заявка принята в работу');
      }
      onSuccess(result);
      onClose();
    } catch {
      pushToast('error', 'Не удалось переместить устройство. Попробуйте ещё раз.');
    } finally {
      submitting = false;
    }
  }

  function move() {
    if (placeId === null) return;
    void run({ kind: 'move', placeId });
  }

  function stay() {
    void run({ kind: 'stay' });
  }
</script>

<!-- Начальный фокус на «Отмена»: фокус на PlacePicker раскрывает дерево (handleFocus). -->
<Modal
  {open}
  title="Перемещение устройства"
  size="md"
  {onClose}
  initialFocus=".modal-footer .btn-secondary"
>
  <div class="form">
    <div class="facts">
      <DetailField label="Устройство">
        <span class="device">
          {repairState.deviceName ?? '—'}
          <Badge
            variant={awaitingTone.variant}
            appearance={awaitingTone.appearance}
            muted={awaitingTone.muted}>{awaitingLabel}</Badge
          >
        </span>
      </DetailField>
      <DetailField label="Сейчас" value={repairState.currentPlacePath} />
    </div>

    {#if storageIds.length > 0}
      <div class="field">
        <span class="label" id="rep-move-quick-label">Быстрый выбор склада</span>
        <div class="chips" role="group" aria-label="Быстрый выбор склада">
          {#each storageIds as id (id)}
            <Button
              variant={placeId === id ? 'primary' : 'secondary'}
              size="sm"
              ariaPressed={placeId === id}
              onclick={() => (placeId = id)}
            >
              {storageLabels[id] ?? `#${id}`}
            </Button>
          {/each}
        </div>
      </div>
    {/if}

    <div class="field">
      <label class="label" for="rep-move-place">Куда переместить</label>
      <PlacePicker value={placeId} id="rep-move-place" onChange={(id) => (placeId = id)} />
      {#if showAnchorNote}
        <span class="hint"
          >Вместе с устройством переместятся детали его группы: {repairState.groupBlockSize}.</span
        >
      {/if}
    </div>

    {#if warnedStatusCode}
      <div class="warn" role="status">
        <svg class="warn-icon" width="16" height="16" viewBox="0 0 16 16" aria-hidden="true">
          <path d="M8 1.5 15 14H1L8 1.5Z" fill="none" stroke="currentColor" stroke-width="1.5" />
          <path d="M8 6v4M8 11.5v.5" stroke="currentColor" stroke-width="1.5" />
        </svg>
        <span
          >Устройство сейчас в статусе «{warnedStatusLabel}». При принятии в ремонт статус будет
          заменён на «{awaitingLabel}».</span
        >
      </div>
    {/if}
  </div>

  {#snippet footer()}
    <Button variant="secondary" onclick={onClose}>Отмена</Button>
    <span class="spacer"></span>
    {#if mode === 'accept'}
      <Button variant="secondary" disabled={submitting} onclick={stay}>Без перемещения</Button>
    {/if}
    <Button variant="primary" loading={submitting} disabled={!canMove} onclick={move}>
      Переместить
    </Button>
  {/snippet}
</Modal>

<style lang="scss">
  .form {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-md);
    white-space: normal;
    text-align: left;
  }
  .facts {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-xs);
  }
  .device {
    display: inline-flex;
    align-items: center;
    gap: var(--tr-space-xs);
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-2xs);
  }
  .label {
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-secondary);
  }
  .hint {
    font: var(--tr-text-caption);
    color: var(--tr-text-tertiary);
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: var(--tr-space-2xs);
  }
  .warn {
    display: flex;
    align-items: flex-start;
    gap: var(--tr-space-xs);
    padding: var(--tr-space-xs) var(--tr-space-sm);
    background: var(--tr-surface-sunken);
    border: 1px solid var(--tr-warning-muted-text);
    border-radius: var(--tr-radius-xs);
    color: var(--tr-text-primary);
    font-size: var(--tr-font-size-label);
  }
  .warn-icon {
    flex-shrink: 0;
    margin-top: 1px;
    color: var(--tr-warning-muted-text);
  }
  .spacer {
    flex: 1;
  }
</style>

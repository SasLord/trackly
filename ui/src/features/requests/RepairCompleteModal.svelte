<script lang="ts">
  // Фаза 41.4, план 17 (D-09, D-16, D-10): попап «Завершить заявку».
  // Если устройство перемещалось в ходе ремонта — выбор места возврата с живым превью
  // статуса; если нет — обычное подтверждение. Склад определяет сервер
  // (cartridge_storage_place_ids: само место или любой предок с is_storage), клиент
  // дерево не обходит. Итоговый статус устройства выставляет сервер.
  import { untrack } from 'svelte';
  import Modal from '$lib/components/Modal.svelte';
  import Button from '$lib/components/Button.svelte';
  import Textarea from '$lib/components/Textarea.svelte';
  import PlacePicker from '$lib/components/PlacePicker.svelte';
  import { apiCall } from '$lib/api/client';
  import { parseAppError } from '$lib/api/errors';
  import { pushToast } from '$lib/stores/toast.svelte';
  import {
    ensureLoaded as ensureStatusesLoaded,
    statusByCode,
  } from '$lib/stores/deviceStatuses.svelte';
  import { repairs } from './api';
  import type { RepairStateDto, RepairTransitionResultDto, RequestDto } from '../../bindings';

  interface Props {
    open: boolean;
    request: RequestDto;
    repairState: RepairStateDto;
    onClose: () => void;
    onSuccess: (_result: RepairTransitionResultDto) => void;
  }

  const { open, request, repairState, onClose, onSuccess }: Props = $props();

  const IN_USE_CODE = 'в_работе';
  const IN_STORAGE_CODE = 'на_складе';

  let placeId = $state<number | null>(null);
  let notes = $state('');
  let submitting = $state(false);
  let serverError = $state('');
  /** null — список складов не получен: превью не показываем. */
  let storageIds = $state<number[] | null>(null);

  // Сброс при открытии. Читается только `open`; остальное — через untrack.
  $effect(() => {
    if (!open) return;
    let cancelled = false;
    untrack(() => {
      placeId = repairState.suggestedReturnPlaceId;
      notes = '';
      submitting = false;
      serverError = '';
      storageIds = null;
    });
    void ensureStatusesLoaded();
    (async () => {
      try {
        const ids = await apiCall<number[]>('cartridge_storage_place_ids', {});
        if (!cancelled) storageIds = ids;
      } catch {
        // Превью — удобство: без него поле остаётся без подсказки.
      }
    })();
    return () => {
      cancelled = true;
    };
  });

  const moved = $derived(repairState.moved);
  const previewText = $derived.by(() => {
    if (placeId === null || storageIds === null) return '';
    const onStorage = storageIds.includes(placeId);
    const code = onStorage ? IN_STORAGE_CODE : IN_USE_CODE;
    const label = statusByCode(code)?.name ?? (onStorage ? 'На складе' : 'В работе');
    return `Статус устройства: «${label}» ${onStorage ? 'на складе' : 'на рабочем месте'}.`;
  });
  const canSubmit = $derived(!submitting && (!moved || placeId !== null));

  async function submit() {
    if (!canSubmit) return;
    submitting = true;
    serverError = '';
    try {
      const result = await repairs.complete({
        requestId: request.id,
        version: request.version,
        placeId: moved ? placeId : null,
        notes: notes.trim() || null,
      });
      pushToast('success', 'Заявка завершена');
      onSuccess(result);
      onClose();
    } catch (e) {
      const err = parseAppError(e);
      // Серверный отказ (в т.ч. D-10 «сначала заберите из ремонта») — текст как есть.
      if (err.code === 'VALIDATION' || err.code === 'CONFLICT') {
        serverError = err.message;
      } else {
        pushToast('error', 'Не удалось завершить заявку. Попробуйте ещё раз.');
      }
    } finally {
      submitting = false;
    }
  }
</script>

<Modal
  {open}
  title="Завершить заявку"
  size="md"
  {onClose}
  initialFocus={moved ? '.modal-footer .btn-secondary' : '#rep-complete-notes'}
>
  <div class="form">
    {#if moved}
      <div class="field">
        <label class="label" for="rep-complete-place">Куда вернуть устройство</label>
        <PlacePicker value={placeId} id="rep-complete-place" onChange={(id) => (placeId = id)} />
        {#if previewText}
          <span class="hint">{previewText}</span>
        {/if}
      </div>
    {:else}
      <p class="confirm">Устройство не перемещалось в ходе ремонта. Завершить заявку?</p>
    {/if}

    <div class="field">
      <label class="label" for="rep-complete-notes">Комментарий</label>
      <Textarea id="rep-complete-notes" bind:value={notes} rows={3} />
    </div>

    {#if serverError}
      <span class="error" role="alert">{serverError}</span>
    {/if}
  </div>

  {#snippet footer()}
    <Button variant="secondary" onclick={onClose}>Отмена</Button>
    <Button variant="primary" loading={submitting} disabled={!canSubmit} onclick={submit}>
      Завершить
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
  .confirm {
    margin: 0;
    color: var(--tr-text-primary);
  }
  .error {
    font-size: var(--tr-font-size-label);
    color: var(--tr-danger-text);
  }
</style>

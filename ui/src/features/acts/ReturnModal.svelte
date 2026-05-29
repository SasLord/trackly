<script lang="ts">
  // Plan 03-03: «Возврат по акту» modal — bulk + per-row override.
  //
  // Default: applyToAll=true; bulk-default Состояние/Расположение пустые → user
  // заполняет. Per-row override побеждает bulk; per-row None → bulk fallback
  // (если applyToAll); applyToAll=false → каждая checked-row обязана иметь
  // condition + location (валидация на бэке).
  import Modal from '$lib/components/Modal.svelte';
  import Button from '$lib/components/Button.svelte';
  import Input from '$lib/components/Input.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import DeviceAutocompleteField from '../devices/DeviceAutocompleteField.svelte';
  import ReturnItemsTable, { type ReturnRowState } from './ReturnItemsTable.svelte';
  import { acts } from './api';
  import type { ActDto, ActReturnDto, ActReturnItemDto } from '../../bindings';

  interface Props {
    open: boolean;
    act: ActDto | null;
    onClose: () => void;
    onSuccess: (_returnDto: ActDto, _parentArchived: boolean) => void;
  }

  const { open, act, onClose, onSuccess }: Props = $props();

  // State runes.
  let applyToAll = $state(true);
  let bulkCondition = $state('');
  let bulkLocationName = $state('');
  let rows = $state<ReturnRowState[]>([]);
  let submitting = $state(false);

  // Rebuild rows whenever the act prop changes (i.e. modal reopens с другим actом).
  $effect(() => {
    if (act) {
      rows = act.items.map((it) => ({
        actItemId: it.id,
        deviceId: it.device_id,
        deviceLabel: it.inventory_no
          ? `${it.device_name} (инв. ${it.inventory_no})`
          : it.device_name,
        quantity: it.quantity,
        checked: true,
        conditionOverride: null,
        locationOverrideId: null,
        locationOverrideName: '',
      }));
      applyToAll = true;
      bulkCondition = '';
      bulkLocationName = '';
    } else {
      rows = [];
    }
  });

  // Number predict «42в{N+1}» — sub_number следующего возврата = текущий count + 1.
  const predictedSubNumber = $derived((act?.return_ids.length ?? 0) + 1);
  const parentNumber = $derived(act?.number_raw ?? 0);

  const checkedRows = $derived(rows.filter((r) => r.checked));

  // canSubmit: ≥1 checked. Дополнительная валидация делается backend'ом.
  const canSubmit = $derived(checkedRows.length > 0 && !submitting);

  function handleRowsChange(next: ReturnRowState[]) {
    rows = next;
  }

  async function handleSubmit() {
    if (!act || !canSubmit) return;
    submitting = true;

    const items: ActReturnItemDto[] = checkedRows.map((r) => ({
      act_item_id: r.actItemId,
      device_id: r.deviceId,
      quantity: r.quantity,
      condition_override: r.conditionOverride,
      location_id_override: r.locationOverrideId,
      location_name_override:
        r.locationOverrideName.trim().length > 0 ? r.locationOverrideName.trim() : null,
    }));

    const payload: ActReturnDto = {
      bulk_condition: bulkCondition.trim().length > 0 ? bulkCondition.trim() : null,
      bulk_location_id: null,
      bulk_location_name: bulkLocationName.trim().length > 0 ? bulkLocationName.trim() : null,
      apply_to_all: applyToAll,
      items,
    };

    try {
      const wasArchivedBefore = act.archived;
      const ret = await acts.doReturn(act.id, payload);
      const n = items.length;
      const suffix = ret.number.replace(/^\d+/, ''); // «в» / «в1»
      pushToast(
        'success',
        `Создан акт возврата №${ret.number_raw}${suffix}. ${n} устр. вернулось на склад.`,
      );

      // Узнать про auto-archive — повторно подтянуть parent (он в return_dto не
      // приходит; в нашей DTO archived живёт на акте).
      try {
        const parent = await acts.get(act.id);
        if (parent.archived && !wasArchivedBefore) {
          pushToast('info', `Акт №${parent.number} переехал в Архив (все устройства вернулись).`);
        }
        onSuccess(ret, parent.archived);
      } catch {
        onSuccess(ret, false);
      }
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось оформить возврат';
      pushToast('error', msg);
    } finally {
      submitting = false;
    }
  }
</script>

<Modal {open} title={act ? `Возврат по акту №${act.number}` : 'Возврат'} size="wide" {onClose}>
  {#if act}
    <p class="subheading">
      Создаст акт возврата №{parentNumber}в{predictedSubNumber}
    </p>

    <section class="bulk-section">
      <h3 class="section-heading">Применить ко всем выбранным позициям</h3>
      <label class="apply-toggle">
        <input
          type="checkbox"
          checked={applyToAll}
          onchange={(e) => (applyToAll = (e.currentTarget as HTMLInputElement).checked)}
        />
        <span>Применить ко всем (по умолчанию)</span>
      </label>
      <div class="bulk-grid">
        <div class="bulk-field">
          <span class="label">Состояние</span>
          <Input
            type="text"
            value={bulkCondition}
            placeholder="Хорошее / Б/У / Среднее / Новое"
            disabled={!applyToAll}
            oninput={(v) => (bulkCondition = v)}
          />
        </div>
        <div class="bulk-field">
          <span class="label">Расположение на складе</span>
          <DeviceAutocompleteField
            field="location"
            value={bulkLocationName}
            placeholder="Куда вернуть на склад"
            statusIn={['на_складе']}
            onChange={(v) => (bulkLocationName = v)}
          />
        </div>
      </div>
    </section>

    <section class="items-section">
      <h3 class="section-heading">
        Позиции к возврату ({checkedRows.length})
      </h3>
      <ReturnItemsTable
        items={rows}
        {applyToAll}
        bulkCondition={bulkCondition.trim().length > 0 ? bulkCondition.trim() : null}
        {bulkLocationName}
        onChange={handleRowsChange}
      />
      {#if checkedRows.length === 0}
        <p class="empty-hint">Выберите хотя бы одну позицию для возврата.</p>
      {/if}
    </section>
  {/if}

  {#snippet footer()}
    <Button variant="secondary" onclick={onClose}>Отмена</Button>
    <Button variant="primary" loading={submitting} disabled={!canSubmit} onclick={handleSubmit}>
      {#if submitting}Оформляем возврат…{:else}Оформить возврат{/if}
    </Button>
  {/snippet}
</Modal>

<style lang="scss">
  .subheading {
    margin: 0 0 var(--space-lg);
    color: var(--color-text-secondary);
    font-size: var(--font-size-label);
    font-weight: 500;
  }

  .section-heading {
    margin: 0 0 var(--space-sm);
    font-size: var(--font-size-subheading, var(--font-size-body));
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
  }

  .bulk-section {
    background: var(--color-surface);
    padding: var(--space-md);
    border-radius: var(--radius-sm);
    margin-bottom: var(--space-lg);
  }

  .apply-toggle {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    margin-bottom: var(--space-md);
    color: var(--color-text-primary);
    font-size: var(--font-size-body);
    cursor: pointer;
  }

  .bulk-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-md);
  }
  .bulk-field {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
  }
  .label {
    font-size: var(--font-size-label);
    color: var(--color-text-secondary);
    font-weight: 500;
  }

  .items-section {
    margin-bottom: var(--space-md);
  }

  .empty-hint {
    margin: var(--space-sm) 0 0;
    color: var(--color-destructive, #b91c1c);
    font-size: var(--font-size-label);
  }
</style>

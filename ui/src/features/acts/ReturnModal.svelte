<script lang="ts">
  // Phase 3.1 Plan 03: «Возврат по акту» modal — G-6 + G-10 + G-12 DTO.
  //
  // G-10 (Phase 3.1): rows flatten'утся ПО device_id (используя
  // `item.outstanding_device_ids` от backend); если позиция уже возвращена,
  // её device_id не появится в outstanding и row не отрендерится.
  // G-6 (Phase 3.1): apply_to_all=false → bulk-поля DISABLED, per-row
  // condition И location ОБА enabled (symmetric — было buggy: location был
  // enabled без apply_to_all guard).
  // G-12 (Phase 3.1): payload содержит ActReturnItemDto.device_ids[]
  // через buildReturnItems helper (PER-ROW SPLIT INVARIANT — W-4).
  // PersonAutocomplete swap: «Кто возвращает» = parent.receiver_name (тот
  // кто принимал в handover — теперь возвращает); «Кто принимает» = parent.giver_name.
  import Modal from '$lib/components/Modal.svelte';
  import Button from '$lib/components/Button.svelte';
  import PersonAutocomplete from '$lib/components/PersonAutocomplete.svelte';
  import Input from '$lib/components/Input.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import LocationAutocomplete from '$lib/components/LocationAutocomplete.svelte';
  import ReturnItemsTable, { type ReturnRowState } from './ReturnItemsTable.svelte';
  import { buildReturnItems } from './returnPayload';
  import { acts } from './api';
  import type { ActDto, ActReturnDto } from '../../bindings';

  interface Props {
    open: boolean;
    act: ActDto | null;
    onClose: () => void;
    onSuccess: (_returnDto: ActDto, _parentArchived: boolean) => void;
  }

  const { open, act, onClose, onSuccess }: Props = $props();

  // State runes.
  let giverName = $state('');
  let receiverName = $state('');
  let applyToAll = $state(true);
  let bulkCondition = $state('');
  let bulkLocationName = $state('');
  let rows = $state<ReturnRowState[]>([]);
  let submitting = $state(false);

  // Rebuild rows whenever the act prop changes (модал reopens с другим actом).
  // G-10: flatten на per-device-id; G-6 default-swap для PersonAutocomplete.
  $effect(() => {
    if (act) {
      rows = act.items.flatMap((it) =>
        it.outstanding_device_ids.map((did) => ({
          actItemId: it.id,
          deviceId: did,
          deviceLabel: it.inventory_no
            ? `${it.device_name} (инв. ${it.inventory_no})`
            : `${it.device_name} #${did}`,
          checked: true,
          conditionOverride: null,
          locationOverrideName: '',
        })),
      );
      // PersonAutocomplete swap.
      giverName = act.receiver_name;
      receiverName = act.giver_name;
      applyToAll = true;
      bulkCondition = '';
      bulkLocationName = '';
    } else {
      rows = [];
      giverName = '';
      receiverName = '';
    }
  });

  // Number predict «42в{N+1}» — sub_number следующего возврата = текущий count + 1.
  const predictedSubNumber = $derived((act?.return_ids.length ?? 0) + 1);
  const parentNumber = $derived(act?.number_raw ?? 0);

  const checkedRows = $derived(rows.filter((r) => r.checked));

  // canSubmit: ≥1 checked + appropriate validation.
  // applyToAll=true: bulk_condition + bulk_location_name заполнены ИЛИ user
  //                  явно ничего не передаёт (backend применит null fallback).
  //                  Для UX строго требуем хотя бы condition (location может
  //                  остаться на текущем расположении).
  // applyToAll=false: каждая checked-row должна иметь conditionOverride И
  //                   locationOverrideName non-empty (backend validate
  //                   defence-in-depth).
  const canSubmit = $derived.by(() => {
    if (submitting) return false;
    if (checkedRows.length === 0) return false;
    if (applyToAll) {
      // Минимум: bulk_condition не пустой.
      return bulkCondition.trim().length > 0;
    }
    // per-row mode: каждая checked row должна иметь condition + location.
    return checkedRows.every(
      (r) =>
        (r.conditionOverride ?? '').trim().length > 0 && r.locationOverrideName.trim().length > 0,
    );
  });

  function handleRowsChange(next: ReturnRowState[]) {
    rows = next;
  }

  async function handleSubmit() {
    if (!act || !canSubmit) return;
    submitting = true;

    // PER-ROW SPLIT INVARIANT (W-4): see plan 03.1-03 must_haves; covered by
    // returnPayload.test.ts `splits_per_row_overrides_to_single_device_items` test.
    const items = buildReturnItems(rows, applyToAll);

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
      const n = items.reduce((sum, it) => sum + (it.device_ids?.length ?? 0), 0);
      const suffix = ret.number.replace(/^\d+/, ''); // «в» / «в1»
      pushToast(
        'success',
        `Создан акт возврата №${ret.number_raw}${suffix}. ${n} устр. вернулось на склад.`,
      );

      // Узнать про auto-archive — повторно подтянуть parent.
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
    {#if rows.length === 0}
      <p class="empty-state">Все позиции уже возвращены.</p>
    {:else}
      <p class="subheading">
        Создаст акт возврата №{parentNumber}в{predictedSubNumber}
      </p>

      <section class="persons-section">
        <div class="bulk-grid">
          <div class="bulk-field">
            <span class="label">Кто возвращает</span>
            <PersonAutocomplete
              field="receiver"
              bind:value={giverName}
              placeholder="ФИО возвращающего"
              id="ret-giver"
            />
          </div>
          <div class="bulk-field">
            <span class="label">Кто принимает</span>
            <PersonAutocomplete
              field="giver"
              bind:value={receiverName}
              placeholder="ФИО принимающего"
              id="ret-receiver"
            />
          </div>
        </div>
      </section>

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
            <LocationAutocomplete
              value={bulkLocationName}
              placeholder="Куда вернуть на склад"
              onChange={(v) => (bulkLocationName = v)}
            />
          </div>
        </div>
      </section>

      <section class="items-section">
        <h3 class="section-heading">
          Позиции к возврату ({checkedRows.length} из {rows.length})
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
        {#if !applyToAll && checkedRows.length > 0 && !canSubmit && !submitting}
          <p class="empty-hint">
            Заполните «Состояние» и «Расположение» для каждой выбранной позиции (либо включите
            «Применить ко всем»).
          </p>
        {/if}
        {#if applyToAll && !canSubmit && checkedRows.length > 0 && !submitting}
          <p class="empty-hint">Заполните bulk-поле «Состояние».</p>
        {/if}
      </section>
    {/if}
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

  .empty-state {
    margin: var(--space-lg) 0;
    color: var(--color-text-muted);
    font-size: var(--font-size-body);
    text-align: center;
  }

  .section-heading {
    margin: 0 0 var(--space-sm);
    font-size: var(--font-size-subheading, var(--font-size-body));
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
  }

  .persons-section,
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

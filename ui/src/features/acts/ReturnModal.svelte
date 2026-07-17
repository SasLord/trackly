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
  //
  // Phase 22 (ACT-03): edit mode added. `mode='edit'` reuses this SAME dialog
  // to edit an EXISTING return act (`editTarget`), prefilled with dual-source
  // rows — the return's own saved items (checked) + the parent's still-
  // outstanding items (addable, unchecked, via `parentAct`). ФИО prefill in
  // edit mode is NOT swapped (D-12) — sourced directly from
  // `editTarget.giver_name`/`editTarget.receiver_name`, the return's own
  // saved values (Pitfall 1 fix, Plan 22-02). A «Дата возврата» DatePicker
  // (D-03/D-04) is now present in BOTH modes; the create-mode payload also
  // now sends giver_name/receiver_name/handover_date_utc (closes RESEARCH.md
  // Pitfall 1's frontend half — these were previously collected but silently
  // dropped from the submit payload).
  import Modal from '$lib/components/Modal.svelte';
  import Button from '$lib/components/Button.svelte';
  import PersonAutocomplete from '$lib/components/PersonAutocomplete.svelte';
  import Input from '$lib/components/Input.svelte';
  import DatePicker from '$lib/components/DatePicker.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import LocationAutocomplete from '$lib/components/LocationAutocomplete.svelte';
  import ReturnItemsTable, { type ReturnRowState } from './ReturnItemsTable.svelte';
  import { buildReturnItems } from './returnPayload';
  import { acts } from './api';
  import type { ActDto, ActReturnDto, ActUpdateReturnDto } from '../../bindings';

  interface Props {
    open: boolean;
    act: ActDto | null;
    mode?: 'create' | 'edit';
    editTarget?: ActDto | null;
    parentAct?: ActDto | null;
    onClose: () => void;
    onSuccess: (_returnDto: ActDto, _parentArchived: boolean) => void;
  }

  const {
    open,
    act,
    mode = 'create',
    editTarget = null,
    parentAct = null,
    onClose,
    onSuccess,
  }: Props = $props();

  // ----------------------------------------------------------------------------
  // Date helpers (copied verbatim from ActFormBody.svelte — D-03/D-04).
  // ----------------------------------------------------------------------------
  function todayISO(): string {
    const d = new Date();
    const y = d.getUTCFullYear();
    const m = String(d.getUTCMonth() + 1).padStart(2, '0');
    const day = String(d.getUTCDate()).padStart(2, '0');
    return `${y}-${m}-${day}`;
  }

  function unixToIso(unixSeconds: number | null | undefined): string {
    if (unixSeconds === null || unixSeconds === undefined) return '';
    const d = new Date(unixSeconds * 1000);
    const y = d.getUTCFullYear();
    const m = String(d.getUTCMonth() + 1).padStart(2, '0');
    const day = String(d.getUTCDate()).padStart(2, '0');
    return `${y}-${m}-${day}`;
  }

  function isoToUnix(iso: string): number | null {
    if (!iso) return null;
    const t = Date.parse(iso + 'T00:00:00Z');
    return Number.isFinite(t) ? Math.floor(t / 1000) : null;
  }

  // State runes.
  let giverName = $state('');
  let receiverName = $state('');
  let applyToAll = $state(true);
  let bulkCondition = $state('');
  let bulkLocationName = $state('');
  let returnDateISO = $state(todayISO());
  let rows = $state<ReturnRowState[]>([]);
  let submitting = $state(false);

  // Rebuild rows whenever the relevant props change (модал reopens с другим
  // actом, или переключается между create/edit).
  // G-10: flatten на per-device-id; G-6 default-swap для PersonAutocomplete
  // (create-mode only — edit mode skips the swap, D-12).
  $effect(() => {
    if (mode === 'edit') {
      if (editTarget && parentAct) {
        const editedRows: ReturnRowState[] = editTarget.items.map((it) => ({
          actItemId: it.id,
          deviceId: it.device_id,
          deviceLabel: it.inventory_no
            ? `${it.device_name} (инв. ${it.inventory_no})`
            : `${it.device_name} #${it.device_id}`,
          checked: true,
          conditionOverride: it.condition_at_time,
          locationOverrideName: it.device_location ?? '',
        }));
        const addableRows: ReturnRowState[] = parentAct.items.flatMap((it) =>
          it.outstanding_device_ids.map((did) => ({
            actItemId: it.id,
            deviceId: did,
            deviceLabel: it.inventory_no
              ? `${it.device_name} (инв. ${it.inventory_no})`
              : `${it.device_name} #${did}`,
            checked: false,
            conditionOverride: null,
            locationOverrideName: '',
          })),
        );
        rows = [...editedRows, ...addableRows];
        // D-12: un-swapped — the return's own saved values, not the
        // parent's giver/receiver swapped defaults.
        giverName = editTarget.giver_name;
        receiverName = editTarget.receiver_name;
        returnDateISO = unixToIso(editTarget.handover_date_utc);
        // Rows already carry their own saved per-row condition/location —
        // start in per-row mode so those saved values aren't discarded by
        // a bulk value the user hasn't set yet.
        applyToAll = false;
        bulkCondition = '';
        bulkLocationName = '';
      } else {
        rows = [];
        giverName = '';
        receiverName = '';
        returnDateISO = todayISO();
      }
      return;
    }

    // create mode — unchanged row-seeding behavior.
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
      returnDateISO = todayISO();
    } else {
      rows = [];
      giverName = '';
      receiverName = '';
      returnDateISO = todayISO();
    }
  });

  // Number predict «42в{N+1}» — sub_number следующего возврата = текущий count + 1
  // (create mode only).
  const predictedSubNumber = $derived((act?.return_ids.length ?? 0) + 1);
  const parentNumber = $derived(act?.number_raw ?? 0);

  const displayNumber = $derived(mode === 'edit' ? editTarget?.number : act?.number);
  const modalReady = $derived(
    mode === 'edit' ? editTarget !== null && parentAct !== null : act !== null,
  );

  const checkedRows = $derived(rows.filter((r) => r.checked));

  // canSubmit: ≥1 checked + appropriate validation.
  // applyToAll=true: bulk_condition + bulk_location_name заполнены ИЛИ user
  //                  явно ничего не передаёт (backend применит null fallback).
  //                  Для UX строго требуем хотя бы condition (location может
  //                  остаться на текущем расположении).
  // applyToAll=false: каждая checked-row должна иметь conditionOverride И
  //                   locationOverrideName non-empty (backend validate
  //                   defence-in-depth).
  // D-10 (Phase 22): checkedRows.length === 0 guard already covers the
  // empty-composition block for BOTH create and edit modes — unchecking
  // every row in edit mode naturally drives the count to 0.
  const canSubmit = $derived.by(() => {
    if (submitting) return false;
    if (checkedRows.length === 0) return false;
    if (!returnDateISO) return false;
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
    if (!canSubmit) return;

    if (mode === 'edit') {
      if (!editTarget) return;
      submitting = true;

      const items = buildReturnItems(rows, applyToAll);
      const updatePayload: ActUpdateReturnDto = {
        id: editTarget.id,
        expected_version: editTarget.version,
        giver_name: giverName.trim(),
        receiver_name: receiverName.trim(),
        location_id: null,
        location_name: null,
        notes: null,
        deadline_utc: null,
        handover_date_utc: isoToUnix(returnDateISO)!,
        bulk_condition: bulkCondition.trim().length > 0 ? bulkCondition.trim() : null,
        bulk_location_id: null,
        bulk_location_name: bulkLocationName.trim().length > 0 ? bulkLocationName.trim() : null,
        apply_to_all: applyToAll,
        items,
      };

      try {
        const saved = await acts.updateReturn(updatePayload);
        const n = items.reduce((sum, it) => sum + (it.device_ids?.length ?? 0), 0);
        pushToast('success', `Возврат №${saved.number} обновлён. Позиций: ${n}.`);

        try {
          const parent = await acts.get(saved.parent_act_id!);
          onSuccess(saved, parent.archived);
        } catch {
          onSuccess(saved, false);
        }
      } catch (e: unknown) {
        const msg =
          e && typeof e === 'object' && 'message' in e
            ? String((e as { message: unknown }).message)
            : 'Не удалось сохранить возврат';
        pushToast('error', msg);
      } finally {
        submitting = false;
      }
      return;
    }

    if (!act) return;
    submitting = true;

    // PER-ROW SPLIT INVARIANT (W-4): see plan 03.1-03 must_haves; covered by
    // returnPayload.test.ts `splits_per_row_overrides_to_single_device_items` test.
    const items = buildReturnItems(rows, applyToAll);

    // Phase 22 (Pitfall 1 fix): giver_name/receiver_name/handover_date_utc
    // are now sent — previously collected by this form but silently dropped
    // from the payload, so the backend always fell back to the parent-swap
    // default (Plan 22-02's `do_return` fix + this wiring close the gap
    // end-to-end).
    const payload: ActReturnDto = {
      bulk_condition: bulkCondition.trim().length > 0 ? bulkCondition.trim() : null,
      bulk_location_id: null,
      bulk_location_name: bulkLocationName.trim().length > 0 ? bulkLocationName.trim() : null,
      apply_to_all: applyToAll,
      items,
      giver_name: giverName.trim(),
      receiver_name: receiverName.trim(),
      handover_date_utc: isoToUnix(returnDateISO),
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

<Modal
  {open}
  title={displayNumber ? `Возврат по акту №${displayNumber}` : 'Возврат'}
  size="wide"
  {onClose}
>
  {#if modalReady}
    {#if rows.length === 0}
      <p class="empty-state">
        {mode === 'edit' ? 'Нет позиций для отображения.' : 'Все позиции уже возвращены.'}
      </p>
    {:else}
      {#if mode === 'edit'}
        <p class="subheading">Редактирование акта возврата №{editTarget?.number}</p>
      {:else}
        <p class="subheading">
          Создаст акт возврата №{parentNumber}в{predictedSubNumber}
        </p>
      {/if}

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
        <div class="bulk-field date-field">
          <span class="label">Дата возврата</span>
          <DatePicker id="return-date" bind:value={returnDateISO} required />
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
      {#if submitting}
        {mode === 'edit' ? 'Сохраняем…' : 'Оформляем возврат…'}
      {:else if mode === 'edit'}Сохранить{:else}Оформить возврат{/if}
    </Button>
  {/snippet}
</Modal>

<style lang="scss">
  .subheading {
    margin: 0 0 var(--tr-space-xl);
    color: var(--tr-text-secondary);
    font-size: var(--tr-font-size-label);
    font-weight: 500;
  }

  .empty-state {
    margin: var(--tr-space-xl) 0;
    color: var(--tr-text-tertiary);
    font-size: var(--tr-font-size-body);
    text-align: center;
  }

  .section-heading {
    margin: 0 0 var(--tr-space-xs);
    font-size: var(--tr-font-size-body);
    font-weight: var(--tr-font-weight-semibold);
    color: var(--tr-text-primary);
  }

  .persons-section,
  .bulk-section {
    background: var(--tr-surface);
    padding: var(--tr-space-md);
    border-radius: var(--tr-radius-xs);
    margin-bottom: var(--tr-space-xl);
  }

  .apply-toggle {
    display: flex;
    align-items: center;
    gap: var(--tr-space-xs);
    margin-bottom: var(--tr-space-md);
    color: var(--tr-text-primary);
    font-size: var(--tr-font-size-body);
    cursor: pointer;
  }

  .bulk-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--tr-space-md);
  }
  .bulk-field {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-2xs);
  }
  .date-field {
    margin-top: var(--tr-space-md);
    max-width: 280px;
  }
  .label {
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-secondary);
    font-weight: 500;
  }

  .items-section {
    margin-bottom: var(--tr-space-md);
  }

  .empty-hint {
    margin: var(--tr-space-xs) 0 0;
    color: var(--tr-danger);
    font-size: var(--tr-font-size-label);
  }
</style>

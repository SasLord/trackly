<script lang="ts">
  // Plan 03-02: form body for the create-handover modal.
  // Pattern follows DeviceFormBody — exposes onRegisterSubmit / onLoading /
  // onCanSubmitChange to parent ActFormModal, parent's footer button calls
  // bodySubmitFn() directly.
  import { onMount } from 'svelte';
  import Input from '$lib/components/Input.svelte';
  import PersonAutocomplete from '$lib/components/PersonAutocomplete.svelte';
  import DatePicker from '$lib/components/DatePicker.svelte';
  import LocationAutocomplete from '$lib/components/LocationAutocomplete.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { acts } from './api';
  import ActNumberField from './ActNumberField.svelte';
  import ActFormItemsTable from './ActFormItemsTable.svelte';
  import type { FormItemRow } from './ActFormItemsTable.svelte';
  import type { ActCreateDto, ActDto, ActUpdateDto } from '../../bindings';

  interface Props {
    mode?: 'create' | 'edit';
    initialAct?: ActDto | null;
    onSaved: (_act: ActDto) => void;
    onLoading: (_l: boolean) => void;
    onCanSubmitChange: (_c: boolean) => void;
    onRegisterSubmit: (_fn: () => void) => void;
  }

  const {
    mode = 'create',
    initialAct = null,
    onSaved,
    onLoading,
    onCanSubmitChange,
    onRegisterSubmit,
  }: Props = $props();

  // ----------------------------------------------------------------------------
  // State
  // ----------------------------------------------------------------------------
  // G-2 (Phase 3.1 Plan 04): дата фактической передачи (когда отдали).
  // Default = today UTC. Plan 19-08 (IN-01): UTC accessors match
  // unixToIso()/isoToUnix() below — a single TZ convention across the
  // create-default, edit-prefill and round-trip paths, no day-boundary
  // off-by-one against browser-local calendar accessors.
  function todayISO(): string {
    const d = new Date();
    const y = d.getUTCFullYear();
    const m = String(d.getUTCMonth() + 1).padStart(2, '0');
    const day = String(d.getUTCDate()).padStart(2, '0');
    return `${y}-${m}-${day}`;
  }

  // Plan 19-05 (ACT-02): unix seconds (UTC midnight) -> YYYY-MM-DD, the inverse
  // of isoToUnix below. Used only to prefill DatePicker inputs in edit mode.
  function unixToIso(unixSeconds: number | null | undefined): string {
    if (unixSeconds === null || unixSeconds === undefined) return '';
    const d = new Date(unixSeconds * 1000);
    const y = d.getUTCFullYear();
    const m = String(d.getUTCMonth() + 1).padStart(2, '0');
    const day = String(d.getUTCDate()).padStart(2, '0');
    return `${y}-${m}-${day}`;
  }

  const isEditPrefill = mode === 'edit' && initialAct !== null;

  /** Plan 19-05: prefilled from initialAct.items directly (bypassing the
   *  live на_складе search path) — existing positions are в_работе, not
   *  на_складе, so a live re-search would never find them (RESEARCH.md
   *  "wrinkle"). New rows added during this edit session still go through
   *  the normal on-warehouse picker unchanged. */
  function itemsFromInitialAct(act: ActDto): FormItemRow[] {
    return act.items.map((it) => ({
      device_id: it.device_id,
      quantity: 1,
      device_label: it.device_name,
      query: '',
      picked: true,
      group_ids: [],
      complectation_at_time: it.complectation_at_time,
    }));
  }

  let numberOverride = $state<number | null>(isEditPrefill ? initialAct!.number_raw : null);
  let giverName = $state(isEditPrefill ? initialAct!.giver_name : '');
  let receiverName = $state(isEditPrefill ? initialAct!.receiver_name : '');
  let location = $state(isEditPrefill ? (initialAct!.location ?? '') : '');
  let deadlineISO = $state(isEditPrefill ? unixToIso(initialAct!.deadline_utc) : ''); // YYYY-MM-DD picker value
  let handoverDateISO = $state(
    isEditPrefill ? unixToIso(initialAct!.handover_date_utc) : todayISO(),
  );
  let notes = $state(isEditPrefill ? (initialAct!.notes ?? '') : '');
  let items = $state<FormItemRow[]>(
    isEditPrefill
      ? itemsFromInitialAct(initialAct!)
      : [{ device_id: null, quantity: 1, device_label: '', query: '', picked: false }],
  );

  let loading = $state(false);
  let submitting = $state(false);
  let fieldErrors = $state<Record<string, string>>({});

  const validItemCount = $derived(
    items.filter((it) => it.device_id !== null && it.quantity >= 1).length,
  );

  const canSubmit = $derived(
    giverName.trim() !== '' && receiverName.trim() !== '' && validItemCount >= 1 && !submitting,
  );

  $effect(() => {
    onCanSubmitChange(canSubmit);
  });
  $effect(() => {
    onLoading(loading);
  });

  onMount(() => {
    onRegisterSubmit(handleSubmit);
  });

  function isoToUnix(iso: string): number | null {
    if (!iso) return null;
    const t = Date.parse(iso + 'T00:00:00Z');
    return Number.isFinite(t) ? Math.floor(t / 1000) : null;
  }

  // ----------------------------------------------------------------------------
  // Submit
  // ----------------------------------------------------------------------------
  async function handleSubmit() {
    if (!canSubmit || submitting) return;
    submitting = true;
    loading = true;
    fieldErrors = {};

    try {
      let saved: ActDto;

      if (mode === 'edit') {
        // Plan 19-05 (ACT-02): full-replacement items set — device_id +
        // complectation_at_time travel over the wire (retained/removed/added
        // is diffed server-side). GT2 (260715-gt2): a RETAINED row
        // (complectation_at_time !== undefined) still emits exactly one
        // entry, unchanged. A FRESHLY-ADDED row (complectation_at_time
        // === undefined) with quantity > 1 now expands via group_ids —
        // mirrors the create-branch expansion below (groupIds.slice(0,
        // it.quantity)) — falling back to [device_id] if group_ids is
        // empty/absent (defensive, mirrors serialised single-instance picks).
        // ActUpdateItemDto itself is unchanged (still one device_id +
        // complectation_at_time per entry); multi-qty travels as N entries.
        const updateItems = items
          .filter((it) => it.device_id !== null)
          .flatMap((it) => {
            if (it.complectation_at_time !== undefined) {
              return [
                {
                  device_id: it.device_id as number,
                  complectation_at_time: it.complectation_at_time ?? null,
                },
              ];
            }
            const groupIds = it.group_ids ?? [];
            const deviceIds =
              groupIds.length > 0 ? groupIds.slice(0, it.quantity) : [it.device_id as number];
            return deviceIds.map((deviceId) => ({
              device_id: deviceId,
              complectation_at_time: null,
            }));
          });

        const updatePayload: ActUpdateDto = {
          id: initialAct!.id,
          expected_version: initialAct!.version,
          number_override: numberOverride,
          giver_name: giverName.trim(),
          receiver_name: receiverName.trim(),
          location_id: null,
          location_name: location.trim().length > 0 ? location.trim() : null,
          notes: notes.trim() || null,
          deadline_utc: isoToUnix(deadlineISO),
          handover_date_utc: isoToUnix(handoverDateISO),
          items: updateItems,
        };

        saved = await acts.update(updatePayload);
        pushToast('success', `Акт №${saved.number} обновлён`);
      } else {
        // Build payload — drop any incomplete item rows.
        // UAT Fix #3/#4: device_ids[] = первые `quantity` штук из group_ids
        // (если выбрана группа) — backend использует именно эти devices без
        // клонирования; legacy fallback (group_ids пуст) — старый clone path.
        const payloadItems = items
          .filter((it) => it.device_id !== null && it.quantity >= 1)
          .map((it) => {
            const groupIds = it.group_ids ?? [];
            const deviceIds = groupIds.length > 0 ? groupIds.slice(0, it.quantity) : [];
            return {
              device_id: it.device_id as number,
              device_ids: deviceIds,
              quantity: it.quantity,
            };
          });

        const payload: ActCreateDto = {
          number_override: numberOverride,
          giver_name: giverName.trim(),
          receiver_name: receiverName.trim(),
          // location_id wiring TODO: Phase 2 currently stores `location` as text;
          // sending null means «not picked» — service is tolerant per Plan 02 spec.
          location_id: null,
          location_name: location.trim().length > 0 ? location.trim() : null,
          notes: notes.trim() || null,
          deadline_utc: isoToUnix(deadlineISO),
          handover_date_utc: isoToUnix(handoverDateISO),
          items: payloadItems,
        };

        saved = await acts.create(payload);
        pushToast('success', `Создан акт №${saved.number}`);
      }

      onSaved(saved);
    } catch (e: unknown) {
      if (e && typeof e === 'object') {
        const err = e as {
          code?: string;
          message?: string;
          details?: { field?: string; reason?: string };
        };
        if (err.code === 'Validation' && err.details?.field) {
          fieldErrors = { ...fieldErrors, [err.details.field]: err.message ?? 'Ошибка' };
          pushToast('error', err.message ?? 'Проверьте поля формы');
        } else if (err.code === 'Conflict') {
          fieldErrors = {
            ...fieldErrors,
            number: err.message ?? 'Конфликт номера',
          };
          pushToast('error', err.message ?? 'Конфликт номера акта');
        } else if (err.code === 'OptimisticLockMismatch') {
          pushToast(
            'error',
            'Акт был изменён другим пользователем — обновите страницу и попробуйте снова.',
          );
        } else {
          pushToast(
            'error',
            err.message ??
              (mode === 'edit' ? 'Не удалось сохранить акт' : 'Не удалось создать акт'),
          );
        }
      } else {
        pushToast('error', mode === 'edit' ? 'Не удалось сохранить акт' : 'Не удалось создать акт');
      }
    } finally {
      loading = false;
      submitting = false;
    }
  }
</script>

<form
  class="act-form"
  onsubmit={(e) => {
    e.preventDefault();
    e.stopPropagation();
  }}
>
  <h3 class="section-heading">Шапка акта</h3>

  <!-- Row 1: №, Когда отдали, Сроком до (3 колонки) -->
  <div class="grid-3">
    <div class="field" class:has-error={!!fieldErrors['number']}>
      <label class="label" for="act-number">№ ⃰</label>
      <ActNumberField
        bind:value={numberOverride}
        onChange={(v) => {
          numberOverride = v;
        }}
        invalid={!!fieldErrors['number']}
        errorMessage={fieldErrors['number'] ?? null}
      />
    </div>

    <div class="field">
      <label class="label" for="act-handover-date">Когда отдали <span class="req">*</span></label>
      <DatePicker id="act-handover-date" bind:value={handoverDateISO} required />
    </div>

    <div class="field">
      <label class="label" for="act-deadline">Сроком до</label>
      <DatePicker id="act-deadline" bind:value={deadlineISO} />
    </div>
  </div>

  <!-- Row 2: Сдал, Принял (2 колонки) -->
  <div class="grid-2">
    <div class="field" class:has-error={!!fieldErrors['giver_name']}>
      <label class="label" for="act-giver">Сдал ⃰</label>
      <PersonAutocomplete
        id="act-giver"
        field="giver"
        bind:value={giverName}
        placeholder="Иванов Иван Иванович"
        invalid={!!fieldErrors['giver_name']}
      />
      {#if fieldErrors['giver_name']}<p class="error">{fieldErrors['giver_name']}</p>{/if}
    </div>

    <div class="field" class:has-error={!!fieldErrors['receiver_name']}>
      <label class="label" for="act-receiver">Принял ⃰</label>
      <PersonAutocomplete
        id="act-receiver"
        field="receiver"
        bind:value={receiverName}
        placeholder="Петров Пётр Петрович"
        invalid={!!fieldErrors['receiver_name']}
      />
      {#if fieldErrors['receiver_name']}<p class="error">{fieldErrors['receiver_name']}</p>{/if}
    </div>
  </div>

  <!-- Row 3: Расположение, Заметки (2 колонки) -->
  <div class="grid-2">
    <div class="field">
      <label class="label" for="act-location">Расположение</label>
      <LocationAutocomplete
        id="act-location"
        value={location}
        placeholder="Куда передаются устройства"
        onChange={(v) => (location = v)}
      />
    </div>

    <div class="field">
      <label class="label" for="act-notes">Заметки</label>
      <Input
        id="act-notes"
        type="text"
        value={notes}
        placeholder="Необязательно"
        oninput={(v) => (notes = v)}
      />
    </div>
  </div>

  <h3 class="section-heading">Позиции</h3>
  <ActFormItemsTable {items} {fieldErrors} {mode} onChange={(next) => (items = next)} />
</form>

<style lang="scss">
  .act-form {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-xl);
  }
  .section-heading {
    margin: 0;
    font-size: var(--tr-font-size-body);
    font-weight: var(--tr-font-weight-semibold);
    color: var(--tr-text-primary);
  }
  .grid-2 {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--tr-space-md);
  }
  .grid-3 {
    display: grid;
    grid-template-columns: 1fr 1fr 1fr;
    gap: var(--tr-space-md);
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-2xs);
  }
  .label {
    font-size: var(--tr-font-size-label);
    font-weight: 500;
    color: var(--tr-text-secondary);
  }
  .hint {
    margin: 0;
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-tertiary);
  }
  .req {
    color: var(--tr-danger);
    margin-left: 2px;
  }
  .error {
    margin: 0;
    font-size: var(--tr-font-size-label);
    color: var(--tr-danger);
  }

  @media (max-width: 720px) {
    .grid-2,
    .grid-3 {
      grid-template-columns: 1fr;
    }
  }
</style>

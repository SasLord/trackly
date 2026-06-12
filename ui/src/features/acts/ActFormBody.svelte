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
  import type { ActCreateDto, ActDto } from '../../bindings';

  interface Props {
    onSaved: (_act: ActDto) => void;
    onLoading: (_l: boolean) => void;
    onCanSubmitChange: (_c: boolean) => void;
    onRegisterSubmit: (_fn: () => void) => void;
  }

  const { onSaved, onLoading, onCanSubmitChange, onRegisterSubmit }: Props = $props();

  // ----------------------------------------------------------------------------
  // State
  // ----------------------------------------------------------------------------
  let numberOverride = $state<number | null>(null);
  let giverName = $state('');
  let receiverName = $state('');
  let location = $state('');
  let deadlineISO = $state(''); // YYYY-MM-DD picker value
  // G-2 (Phase 3.1 Plan 04): дата фактической передачи (когда отдали).
  // Default = today UTC (browser-local будет хорошо для пользователя в МСК).
  function todayISO(): string {
    const d = new Date();
    const y = d.getFullYear();
    const m = String(d.getMonth() + 1).padStart(2, '0');
    const day = String(d.getDate()).padStart(2, '0');
    return `${y}-${m}-${day}`;
  }
  let handoverDateISO = $state(todayISO());
  let notes = $state('');
  let items = $state<FormItemRow[]>([
    { device_id: null, quantity: 1, device_label: '', query: '', picked: false },
  ]);

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

      const created = await acts.create(payload);
      pushToast('success', `Создан акт №${created.number}`);
      onSaved(created);
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
        } else {
          pushToast('error', err.message ?? 'Не удалось создать акт');
        }
      } else {
        pushToast('error', 'Не удалось создать акт');
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
  <ActFormItemsTable {items} {fieldErrors} onChange={(next) => (items = next)} />
</form>

<style lang="scss">
  .act-form {
    display: flex;
    flex-direction: column;
    gap: var(--space-lg);
  }
  .section-heading {
    margin: 0;
    font-size: var(--font-size-body);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
  }
  .grid-2 {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-md);
  }
  .grid-3 {
    display: grid;
    grid-template-columns: 1fr 1fr 1fr;
    gap: var(--space-md);
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
  }
  .label {
    font-size: var(--font-size-label);
    font-weight: 500;
    color: var(--color-text-secondary);
  }
  .hint {
    margin: 0;
    font-size: var(--font-size-label);
    color: var(--color-text-muted);
  }
  .error {
    margin: 0;
    font-size: var(--font-size-label);
    color: var(--color-destructive);
  }

  @media (max-width: 720px) {
    .grid-2,
    .grid-3 {
      grid-template-columns: 1fr;
    }
  }
</style>

<script lang="ts">
  // DeviceFormBody — the inner form component for DeviceFormModal.
  //
  // This component is intentionally separate from DeviceFormModal so that
  // {#key openInstanceCounter} in the parent forces a full remount on every
  // modal open. This guarantees all $state variables (name, inventoryNo,
  // serialNo, etc.) are reset to their initial values on each opening —
  // no stale form data carries over between create/edit sessions.
  //
  // Regression 6 fix: serial_no and inventory_no now always reset correctly
  // because the component is re-created from scratch on each open.

  import Input from '$lib/components/Input.svelte';
  import Select from '$lib/components/Select.svelte';
  import DeviceAutocompleteField from './DeviceAutocompleteField.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { devices } from './api';
  import type { DeviceDto, DeviceNew, DevicePatch } from '../../bindings';

  const STATUSES = [
    { id: 1, label: 'На складе' },
    { id: 2, label: 'В работе' },
    { id: 3, label: 'На ремонте' },
    { id: 4, label: 'Списано' },
  ];

  interface Props {
    target: DeviceDto | null;
    stateHints: string[];
    onSaved: () => void;
    /** Expose submit-button state to parent's footer snippet. */
    onLoading: (_loading: boolean) => void;
    onCanSubmitChange: (_can: boolean) => void;
    /** Called when parent's submit button is clicked — triggers submit. */
    submitTrigger: number;
  }

  const { target, stateHints, onSaved, onLoading, onCanSubmitChange, submitTrigger }: Props = $props();

  // ---------------------------------------------------------------------------
  // Form state — all initialised from target (edit) or empty (create).
  // Because this component is re-mounted via {#key} on every modal open,
  // these are always fresh: no stale closures, no missing resets.
  // ---------------------------------------------------------------------------
  let name = $state(target?.name ?? '');
  let location = $state(target?.location ?? '');
  let statusId = $state(target ? String(target.status_id) : '');
  let inventoryNo = $state(target?.inventory_no ?? '');
  let serialNo = $state(target?.serial_no ?? '');
  let model = $state(target?.model ?? '');
  let specs = $state(target?.specs ?? '');
  let kit = $state(target?.kit ?? '');
  let stateField = $state(target?.state ?? '');
  let quantity = $state(1);
  let loading = $state(false);
  let submitting = $state(false);
  let fieldErrors = $state<Record<string, string>>({});
  // Local mutable copy of target.version so we can refresh it after a
  // successful update without requiring the parent to re-mount the form.
  let currentVersion = $state(target?.version ?? 1);

  const isEdit = $derived(target !== null);

  const quantityDisabled = $derived(
    isEdit ||
    inventoryNo.trim() !== '' ||
    serialNo.trim() !== ''
  );

  // canSubmit: all required fields filled AND no in-flight request.
  // submitting guards against double-submit even before loading propagates.
  const canSubmit = $derived(
    name.trim() !== '' && location.trim() !== '' && statusId !== '' && !submitting,
  );

  // Reset quantity to 1 when inv/serial become non-empty.
  $effect(() => {
    if (inventoryNo.trim() !== '' || serialNo.trim() !== '') {
      quantity = 1;
    }
  });

  // Propagate canSubmit to parent for footer button state.
  $effect(() => {
    onCanSubmitChange(canSubmit);
  });

  // Propagate loading to parent.
  $effect(() => {
    onLoading(loading);
  });

  // React to parent's submit trigger (incremented when user clicks the footer button).
  // Guard: ignore if already submitting (double-click / rapid re-trigger).
  $effect(() => {
    if (submitTrigger > 0 && !submitting) {
      handleSubmit();
    }
  });

  // ---------------------------------------------------------------------------
  // Submit
  // ---------------------------------------------------------------------------
  async function handleSubmit() {
    if (!canSubmit) return;
    // In-flight guard: prevent double-submit from rapid clicks or concurrent
    // form-onsubmit + submitTrigger firing.
    if (submitting) return;
    submitting = true;
    loading = true;
    fieldErrors = {};

    try {
      if (isEdit && target) {
        const patch: DevicePatch = {
          type_id: null,
          name: name.trim() || null,
          inventory_no: inventoryNo.trim() || null,
          serial_no: serialNo.trim() || null,
          model: model.trim() || null,
          specs: specs.trim() || null,
          kit: kit.trim() || null,
          state: stateField.trim() || null,
          location: location.trim() || null,
          location_id: null,
          status_id: parseInt(statusId, 10) || null,
        };
        const updated = await devices.update(target.id, currentVersion, patch);
        // Refresh the local version counter so a subsequent edit in the same
        // modal session uses the correct (incremented) version.
        currentVersion = updated.version;
        pushToast('success', 'Устройство сохранено');
      } else {
        const newDevice: DeviceNew = {
          type_id: 1,
          name: name.trim(),
          inventory_no: inventoryNo.trim() || null,
          serial_no: serialNo.trim() || null,
          model: model.trim() || null,
          specs: specs.trim() || null,
          kit: kit.trim() || null,
          state: stateField.trim() || null,
          location: location.trim() || null,
          location_id: null,
          status_id: parseInt(statusId, 10),
        };

        const qty = quantityDisabled ? 1 : Math.max(1, Math.min(100, quantity || 1));
        await devices.bulkCreate(newDevice, qty);

        if (qty === 1) {
          pushToast('success', 'Устройство создано');
        } else {
          pushToast('success', `Создано ${qty} устройств`);
        }
      }
      onSaved();
    } catch (e: unknown) {
      if (e && typeof e === 'object') {
        const err = e as { code?: string; message?: string; details?: { field?: string } };
        if (err.code === 'Validation' && err.details?.field) {
          fieldErrors = { ...fieldErrors, [err.details.field]: err.message ?? 'Ошибка' };
          pushToast('error', err.message ?? 'Ошибка валидации');
        } else if (err.code === 'OptimisticLockMismatch') {
          pushToast(
            'error',
            'Данные были изменены другим пользователем. Обновите страницу и попробуйте снова.',
          );
        } else {
          pushToast('error', err.message ?? 'Не удалось сохранить устройство');
        }
      } else {
        pushToast('error', 'Не удалось сохранить устройство');
      }
    } finally {
      loading = false;
      submitting = false;
    }
  }
</script>

<form
  class="device-form"
  onsubmit={(e) => {
    // Prevent default HTML form submission in all cases.
    // Submission is always triggered via submitTrigger from DeviceFormModal's
    // footer button — never via Enter key or implicit form submit.
    e.preventDefault();
    e.stopPropagation();
  }}
>
  <!-- 1. Required: Наименование (with autocomplete) -->
  <div class="field" class:has-error={!!fieldErrors['name']}>
    <label class="label" for="f-name">
      Наименование <span class="required" aria-hidden="true">*</span>
    </label>
    <DeviceAutocompleteField
      field="name"
      value={name}
      placeholder="Ноутбук Lenovo ThinkPad X1"
      id="f-name"
      invalid={!!fieldErrors['name']}
      onChange={(v) => (name = v)}
    />
    {#if fieldErrors['name']}
      <p class="field-error">{fieldErrors['name']}</p>
    {/if}
  </div>

  <!-- 2–4. Инвентарный № / Серийный № / Количество — один горизонтальный ряд.
       Количество ВСЕГДА отображается, но disabled когда inv/serial заполнен или это edit-режим.
       Это исключает «дёрганье» макета при вводе номеров. -->
  <div class="field-row">
    <div class="field field-row-item">
      <label class="label" for="f-inv">Инвентарный №</label>
      <Input
        id="f-inv"
        value={inventoryNo}
        placeholder="ИНВ-000001"
        oninput={(v) => (inventoryNo = v)}
      />
    </div>
    <div class="field field-row-item">
      <label class="label" for="f-serial">Серийный №</label>
      <Input
        id="f-serial"
        value={serialNo}
        placeholder="SN-XXXXXXXX"
        oninput={(v) => (serialNo = v)}
      />
    </div>
    <div class="field field-row-item">
      <label class="label" for="f-qty">Количество</label>
      <input
        id="f-qty"
        type="number"
        class="input"
        class:input-disabled={quantityDisabled}
        min={1}
        max={100}
        value={quantityDisabled ? 1 : quantity}
        disabled={quantityDisabled}
        oninput={(e) => {
          if (!quantityDisabled) {
            const v = parseInt((e.currentTarget as HTMLInputElement).value, 10);
            quantity = isNaN(v) ? 1 : Math.max(1, Math.min(100, v));
          }
        }}
      />
    </div>
  </div>

  <!-- 5. Optional: Модель (with autocomplete, contextual) -->
  <div class="field">
    <label class="label" for="f-model">Модель</label>
    <DeviceAutocompleteField
      field="model"
      value={model}
      placeholder="ThinkPad X1 Carbon Gen 12"
      id="f-model"
      contextName={name.trim() || undefined}
      onChange={(v) => (model = v)}
    />
  </div>

  <!-- 6. Optional: Технические характеристики (specs) — multiline textarea -->
  <div class="field">
    <label class="label" for="f-specs">Технические характеристики</label>
    <DeviceAutocompleteField
      field="specs"
      value={specs}
      placeholder="i7-1365U, 16 ГБ RAM, 512 ГБ SSD"
      id="f-specs"
      multiline={true}
      contextName={name.trim() || undefined}
      onChange={(v) => (specs = v)}
    />
  </div>

  <!-- 7. Optional: Комплектация (kit) -->
  <div class="field">
    <label class="label" for="f-kit">Комплектация</label>
    <DeviceAutocompleteField
      field="kit"
      value={kit}
      placeholder="Зарядное устройство, мышь"
      id="f-kit"
      contextName={name.trim() || undefined}
      onChange={(v) => (kit = v)}
    />
  </div>

  <!-- 8. Required: Статус -->
  <div class="field" class:has-error={!!fieldErrors['status_id']}>
    <label class="label" for="f-status">
      Статус <span class="required" aria-hidden="true">*</span>
    </label>
    <Select
      id="f-status"
      value={statusId}
      invalid={!!fieldErrors['status_id']}
      onchange={(v) => (statusId = v)}
    >
      <option value="">— выберите статус —</option>
      {#each STATUSES as s}
        <option value={String(s.id)}>{s.label}</option>
      {/each}
    </Select>
    {#if fieldErrors['status_id']}
      <p class="field-error">{fieldErrors['status_id']}</p>
    {/if}
  </div>

  <!-- 9. Required: Расположение (with autocomplete, filtered by status + name context) -->
  <div class="field" class:has-error={!!fieldErrors['location']}>
    <label class="label" for="f-location">
      Расположение <span class="required" aria-hidden="true">*</span>
    </label>
    <DeviceAutocompleteField
      field="location"
      value={location}
      placeholder="Кабинет 305"
      id="f-location"
      invalid={!!fieldErrors['location']}
      contextName={name.trim() || undefined}
      contextStatusId={parseInt(statusId, 10) || null}
      onChange={(v) => (location = v)}
    />
    {#if fieldErrors['location']}
      <p class="field-error">{fieldErrors['location']}</p>
    {/if}
  </div>

  <!-- 10. Optional: Состояние + state-hints chips (with autocomplete) — ПОСЛЕДНЕЕ -->
  <div class="field">
    <label class="label" for="f-state">Состояние</label>
    <DeviceAutocompleteField
      field="state"
      value={stateField}
      placeholder="Хорошее"
      id="f-state"
      contextName={name.trim() || undefined}
      onChange={(v) => (stateField = v)}
    />
    {#if stateHints.length > 0}
      <div class="state-hints">
        <span class="state-hints-label">Быстрый выбор:</span>
        <div class="state-hints-chips">
          {#each stateHints as hint}
            <button
              type="button"
              class="hint-chip"
              class:active={stateField === hint}
              onclick={() => (stateField = hint)}
            >
              {hint}
            </button>
          {/each}
        </div>
      </div>
    {/if}
  </div>
</form>

<style lang="scss">
  .device-form {
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
  }

  // Horizontal row for Инв.№ / Серийный № / Количество.
  // Все три поля всегда присутствуют — макет не «прыгает».
  .field-row {
    display: flex;
    gap: var(--space-md);
    align-items: flex-start;
  }

  .field-row-item {
    flex: 1 1 0;
    min-width: 0; // prevents flex child from overflowing
  }

  .label {
    font-size: var(--font-size-label);
    font-weight: var(--font-weight-medium);
    color: var(--color-text-primary);
  }

  .required {
    color: var(--color-destructive);
    margin-left: 2px;
  }

  .field-error {
    margin: 0;
    font-size: var(--font-size-label);
    color: var(--color-destructive);
  }

  .input {
    display: block;
    width: 100%;
    height: 36px;
    padding: 0 var(--space-md);
    background: var(--color-bg);
    color: var(--color-text-primary);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    font-family: var(--font-family-base);
    font-size: var(--font-size-body);
    line-height: var(--line-height-body);

    &::placeholder {
      color: var(--color-text-muted);
    }

    &:focus-visible {
      outline: none;
      border-color: var(--color-accent);
      box-shadow: 0 0 0 3px var(--color-accent-focus);
    }

    &:disabled,
    &.input-disabled {
      background: var(--color-surface-sunken);
      color: var(--color-text-muted);
      cursor: not-allowed;
    }
  }

  .state-hints {
    margin-top: var(--space-xs);
  }

  .state-hints-label {
    display: block;
    font-size: var(--font-size-label);
    color: var(--color-text-secondary);
    margin-bottom: var(--space-xs);
  }

  .state-hints-chips {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-xs);
  }

  .hint-chip {
    padding: 2px var(--space-sm);
    background: var(--color-surface-sunken);
    border: 1px solid var(--color-border);
    border-radius: 12px;
    font-size: var(--font-size-label);
    color: var(--color-text-secondary);
    cursor: pointer;
    font-family: var(--font-family-base);
    transition: none;

    &:hover {
      background: var(--color-surface);
      color: var(--color-text-primary);
      border-color: var(--color-border-strong);
    }

    &.active {
      background: color-mix(in srgb, var(--color-accent) 15%, transparent);
      border-color: var(--color-accent);
      color: var(--color-accent);
    }

    &:focus-visible {
      outline: none;
      box-shadow: 0 0 0 3px var(--color-accent-focus);
    }
  }
</style>

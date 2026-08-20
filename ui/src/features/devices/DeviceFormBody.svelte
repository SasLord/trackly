<script lang="ts">
  // DeviceFormBody — the inner form component for DeviceFormModal.
  //
  // This component is intentionally separate from DeviceFormModal so that
  // {#key openInstanceCounter} in the parent forces a full remount on every
  // modal open. This guarantees all $state variables (name, inventoryNo,
  // serialNo, etc.) are reset to their initial values on each opening —
  // no stale form data carries over between create/edit sessions.
  //
  // Round 8: submitTrigger side-channel eliminated. The parent now binds to
  // the `submit` prop (exposed via $bindable) and calls it directly from the
  // footer button. No reactive trigger, no ordering race.

  import { onMount } from 'svelte';
  import Input from '$lib/components/Input.svelte';
  import Select from '$lib/components/Select.svelte';
  import Button from '$lib/components/Button.svelte';
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
    /** Выбранный тип устройства (1=Устройство, 2=Принтер) — управляется
     *  ActionMenu в заголовке DeviceFormModal, не этим компонентом. */
    typeId: number;
    onSaved: () => void;
    /** Expose submit-button state to parent's footer snippet. */
    onLoading: (_loading: boolean) => void;
    onCanSubmitChange: (_can: boolean) => void;
    /**
     * Called once on mount with a reference to handleSubmit.
     * The parent stores this function and calls it from the footer button.
     * Because {#key openInstanceCounter} remounts the body on each modal open,
     * a fresh function is provided each time — no stale closures, no side-channel races.
     */
    onRegisterSubmit: (_fn: () => void) => void;
  }

  const {
    target,
    stateHints,
    typeId,
    onSaved,
    onLoading,
    onCanSubmitChange,
    onRegisterSubmit,
  }: Props = $props();

  const DEVICE_TYPE_ID = 1;
  const PRINTER_TYPE_ID = 2;

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

  const quantityDisabled = $derived(isEdit || inventoryNo.trim() !== '' || serialNo.trim() !== '');

  let confirmDowngrade = $state(false);

  // Сбросить inline-подтверждение при любой смене типа (пользователь мог снова
  // переключить меню в заголовке, пока подтверждение было открыто) — иначе
  // подтверждение может «зависнуть» для уже неактуального перехода типа.
  $effect(() => {
    typeId;
    confirmDowngrade = false;
  });

  // canSubmit: all required fields filled AND no in-flight request.
  // submitting guards against double-submit even before loading propagates.
  const canSubmit = $derived(
    name.trim() !== '' &&
      location.trim() !== '' &&
      statusId !== '' &&
      !submitting &&
      !confirmDowngrade,
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

  // Register handleSubmit with the parent once on mount.
  // onMount is used (not $effect) to guarantee exactly one call per component
  // instance — when the parent remounts via {#key}, a fresh instance calls
  // onRegisterSubmit with the new closure.
  onMount(() => {
    onRegisterSubmit(handleSubmit);
  });

  // ---------------------------------------------------------------------------
  // Submit
  // ---------------------------------------------------------------------------
  async function handleSubmit() {
    if (!canSubmit) return;
    // In-flight guard: prevent double-submit from rapid clicks.
    if (submitting) return;

    // RDJ-05: перед сохранением конверсии Принтер→Устройство — подтверждение
    // потери данных мониторинга (показания тонера, активные оповещения).
    // confirmDowngrade сам исключён из canSubmit выше, так что повторный клик
    // по «Сохранить» сюда уже не попадёт — реальное сохранение запускает
    // отдельная кнопка «Да, сохранить» в inline-предупреждении (onclick={performSave}).
    const isDowngrade =
      isEdit && target?.type_id === PRINTER_TYPE_ID && typeId === DEVICE_TYPE_ID;
    if (isDowngrade && !confirmDowngrade) {
      confirmDowngrade = true;
      return;
    }

    await performSave();
  }

  async function performSave() {
    submitting = true;
    loading = true;
    fieldErrors = {};

    try {
      if (isEdit && target) {
        const patch: DevicePatch = {
          type_id: typeId,
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
          type_id: typeId,
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
          pushToast('success', typeId === PRINTER_TYPE_ID ? 'Принтер создан' : 'Устройство создано');
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
      confirmDowngrade = false;
    }
  }
</script>

{#if confirmDowngrade}
  <div class="downgrade-confirm" role="alertdialog" aria-live="polite">
    <p>
      Тип устройства меняется с «Принтер» на «Устройство». История показаний тонера и активные
      оповещения по этому принтеру будут удалены безвозвратно.
    </p>
    <div class="downgrade-confirm-actions">
      <Button variant="secondary" onclick={() => (confirmDowngrade = false)}>Отмена</Button>
      <Button variant="destructive" loading={submitting} onclick={performSave}>
        Да, сохранить
      </Button>
    </div>
  </div>
{:else}
<form
  class="device-form"
  onsubmit={(e) => {
    // Prevent default HTML form submission in all cases.
    // Submission is always triggered via the bound submit function from
    // DeviceFormModal's footer button — never via Enter key or implicit form submit.
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
{/if}

<style lang="scss">
  .device-form {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-md);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-2xs);
  }

  // Horizontal row for Инв.№ / Серийный № / Количество.
  // Все три поля всегда присутствуют — макет не «прыгает».
  .field-row {
    display: flex;
    gap: var(--tr-space-md);
    align-items: flex-start;
  }

  .field-row-item {
    flex: 1 1 0;
    min-width: 0; // prevents flex child from overflowing
  }

  .label {
    font-size: var(--tr-font-size-label);
    font-weight: var(--tr-font-weight-medium);
    color: var(--tr-text-primary);
  }

  .required {
    color: var(--tr-danger);
    margin-left: 2px;
  }

  .field-error {
    margin: 0;
    font-size: var(--tr-font-size-label);
    color: var(--tr-danger);
  }

  .input {
    display: block;
    width: 100%;
    height: 36px;
    padding: 0 var(--tr-space-md);
    background: var(--tr-bg);
    color: var(--tr-text-primary);
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-xs);
    font-family: var(--tr-font-family);
    font-size: var(--tr-font-size-body);
    line-height: var(--tr-line-height-body);

    &::placeholder {
      color: var(--tr-text-tertiary);
    }

    &:focus-visible {
      outline: none;
      border-color: var(--tr-accent);
      box-shadow: 0 0 0 3px var(--tr-focus-ring);
    }

    &:disabled,
    &.input-disabled {
      background: var(--tr-surface-sunken);
      color: var(--tr-text-tertiary);
      cursor: not-allowed;
    }
  }

  .state-hints {
    margin-top: var(--tr-space-2xs);
  }

  .state-hints-label {
    display: block;
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-secondary);
    margin-bottom: var(--tr-space-2xs);
  }

  .state-hints-chips {
    display: flex;
    flex-wrap: wrap;
    gap: var(--tr-space-2xs);
  }

  .hint-chip {
    padding: 2px var(--tr-space-xs);
    background: var(--tr-surface-sunken);
    border: 1px solid var(--tr-border);
    border-radius: 12px;
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-secondary);
    cursor: pointer;
    font-family: var(--tr-font-family);
    transition: none;

    &:hover {
      background: var(--tr-surface);
      color: var(--tr-text-primary);
      border-color: var(--tr-border-strong);
    }

    &.active {
      background: color-mix(in srgb, var(--tr-accent) 15%, transparent);
      border-color: var(--tr-accent);
      color: var(--tr-accent);
    }

    &:focus-visible {
      outline: none;
      box-shadow: 0 0 0 3px var(--tr-focus-ring);
    }
  }

  .downgrade-confirm {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-md);
  }

  .downgrade-confirm-actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--tr-space-xs);
  }
</style>

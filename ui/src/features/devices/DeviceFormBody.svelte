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
  //
  // Quick 260820-rdj (UAT gap-closure round 1, defect 1): this component is
  // intentionally a "dumb" form again — the Принтер→Устройство downgrade
  // confirmation decision now lives in DeviceFormModal (a nested Modal), not
  // here. This body just saves whatever `typeId` it was given; it never
  // second-guesses the parent.
  //
  // GAP-8 (39-UAT.md, Прогон 3): `readonly` — read-only mode for
  // PlaceEntityViewModal.svelte's «Просмотр устройства/принтера» popup.
  // Every field's own `disabled` prop is threaded from this single flag
  // (never a second, forked, non-interactive markup copy — see the gap's
  // "reuse, don't fork" instruction). Two defense-in-depth guards on top of
  // "no submit button is rendered by the caller": `canSubmit` is forced
  // false and `handleSubmit` early-returns, so even a stray Enter-key path
  // could never persist a change from a component that is supposed to be
  // strictly a mirror of the current record.

  import { onMount } from 'svelte';
  import Input from '$lib/components/Input.svelte';
  import Select from '$lib/components/Select.svelte';
  import Checkbox from '$lib/components/Checkbox.svelte';
  import DeviceAutocompleteField from './DeviceAutocompleteField.svelte';
  import PlacePicker from '$lib/components/PlacePicker.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { apiCall } from '$lib/api/client';
  import { devices } from './api';
  import type { DeviceDto, DeviceNew, DevicePatch } from '../../bindings';

  // Seed status ids (see STATUSES below / device_service.rs::resolve_status_id):
  // 1 = На складе, 2 = В работе, 3 = На ремонте, 4 = Списано.
  const STORAGE_STATUS_ID = 1;

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
    /** GAP-8: renders every field disabled and blocks submit — see the
     *  file-header comment above. Defaults to false so every existing
     *  caller (DeviceFormModal) is unaffected. */
    readonly?: boolean;
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
    readonly = false,
    onSaved,
    onLoading,
    onCanSubmitChange,
    onRegisterSubmit,
  }: Props = $props();

  const PRINTER_TYPE_ID = 2;

  // ---------------------------------------------------------------------------
  // Form state — all initialised from target (edit) or empty (create).
  // Because this component is re-mounted via {#key} on every modal open,
  // these are always fresh: no stale closures, no missing resets.
  // ---------------------------------------------------------------------------
  let name = $state(target?.name ?? '');
  let placeId = $state<number | null>(target?.place_id ?? null);
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

  // D-11.3: storage-place status suggestion. `storagePlaceIds` is fetched once
  // per form instance (this component is always freshly mounted per modal
  // open — {#key openInstanceCounter} in DeviceFormModal — so a plain onMount
  // fetch, no `open`-toggle re-fetch guard needed, unlike OperationModal.svelte
  // which reuses one mounted instance across opens). Reuses the same
  // `cartridge_storage_place_ids` Tauri/HTTP command cartridges already call —
  // the underlying query (`PlaceRepo::list_storage_place_ids`, D-11.4 ancestor
  // inheritance) is place-tree-derived and entity-agnostic despite the
  // command's cartridge-era name; it is `Action::ReadData`-gated, not
  // cartridge-specific, so any caller able to open this device form already
  // has permission to call it.
  let storagePlaceIds = $state<Set<number>>(new Set());
  // Default-checked (D-11.3: "включённый по умолчанию"); the user may uncheck
  // it (D-10: no forced status change once unchecked).
  let storageStatusSuggested = $state(true);

  const isEdit = $derived(target !== null);

  const quantityDisabled = $derived(isEdit || inventoryNo.trim() !== '' || serialNo.trim() !== '');

  // canSubmit: all required fields filled AND no in-flight request.
  // submitting guards against double-submit even before loading propagates.
  const canSubmit = $derived(
    !readonly && name.trim() !== '' && placeId !== null && statusId !== '' && !submitting,
  );

  // Reset quantity to 1 when inv/serial become non-empty.
  $effect(() => {
    if (inventoryNo.trim() !== '' || serialNo.trim() !== '') {
      quantity = 1;
    }
  });

  // D-11.3: the selected place (including D-11.4 ancestor inheritance,
  // already resolved server-side into the flat `storagePlaceIds` set) is a
  // storage place.
  const isStoragePlace = $derived(placeId !== null && storagePlaceIds.has(placeId));

  // D-11.3: while a storage place is selected AND the suggestion checkbox is
  // checked, the device status is (re-)set to «На складе» — this has a real
  // payload effect (unlike the cartridge form, cartridges have no
  // status-override field; devices do, via DevicePatch.status_id/
  // DeviceNew.status_id). Unchecking stops the auto-apply; the Статус
  // dropdown above is then fully manual again — no forced change (D-10).
  // GAP-8: skipped entirely in readonly mode — a «Просмотр» popup must
  // mirror the record's ACTUAL saved status, never a suggestion that would
  // never actually be applied (nothing here ever submits).
  $effect(() => {
    if (!readonly && isStoragePlace && storageStatusSuggested) {
      statusId = String(STORAGE_STATUS_ID);
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

    let cancelled = false;
    apiCall<number[]>('cartridge_storage_place_ids', {})
      .then((ids) => {
        if (cancelled) return;
        storagePlaceIds = new Set(ids);
      })
      .catch(() => {
        if (cancelled) return;
        // Fail-safe: a failed lookup just hides the suggestion checkbox —
        // never blocks saving the device itself.
        storagePlaceIds = new Set();
      });
    return () => {
      cancelled = true;
    };
  });

  // ---------------------------------------------------------------------------
  // Submit
  // ---------------------------------------------------------------------------
  async function handleSubmit() {
    // GAP-8 defense-in-depth: readonly instances render no submit button and
    // canSubmit is already forced false above, but this guard makes the
    // no-persist guarantee true even if handleSubmit were ever invoked some
    // other way (e.g. a future caller wiring onRegisterSubmit by mistake).
    if (readonly) return;
    if (!canSubmit) return;
    // In-flight guard: prevent double-submit from rapid clicks.
    if (submitting) return;
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
          place_id: placeId,
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
          place_id: placeId,
          status_id: parseInt(statusId, 10),
        };

        const qty = quantityDisabled ? 1 : Math.max(1, Math.min(100, quantity || 1));
        await devices.bulkCreate(newDevice, qty);

        if (qty === 1) {
          pushToast(
            'success',
            typeId === PRINTER_TYPE_ID ? 'Принтер создан' : 'Устройство создано',
          );
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
      disabled={readonly}
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
        disabled={readonly}
        oninput={(v) => (inventoryNo = v)}
      />
    </div>
    <div class="field field-row-item">
      <label class="label" for="f-serial">Серийный №</label>
      <Input
        id="f-serial"
        value={serialNo}
        placeholder="SN-XXXXXXXX"
        disabled={readonly}
        oninput={(v) => (serialNo = v)}
      />
    </div>
    <div class="field field-row-item">
      <label class="label" for="f-qty">Количество</label>
      <input
        id="f-qty"
        type="number"
        class="input"
        class:input-disabled={quantityDisabled || readonly}
        min={1}
        max={100}
        value={quantityDisabled ? 1 : quantity}
        disabled={quantityDisabled || readonly}
        oninput={(e) => {
          if (!quantityDisabled && !readonly) {
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
      disabled={readonly}
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
      disabled={readonly}
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
      disabled={readonly}
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
      disabled={readonly}
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

  <!-- 9. Required: Место (PlacePicker — единственный контрол выбора места, D-17) -->
  <div class="field" class:has-error={!!fieldErrors['place_id']}>
    <label class="label" for="f-place">
      Место <span class="required" aria-hidden="true">*</span>
    </label>
    <PlacePicker
      value={placeId}
      onChange={(id) => (placeId = id)}
      id="f-place"
      invalid={!!fieldErrors['place_id']}
      disabled={readonly}
    />
    {#if fieldErrors['place_id']}
      <p class="field-error">{fieldErrors['place_id']}</p>
    {/if}
    {#if isStoragePlace && !readonly}
      <Checkbox checked={storageStatusSuggested} onchange={(c) => (storageStatusSuggested = c)}>
        Перевести устройство в статус «На складе»
      </Checkbox>
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
      disabled={readonly}
      onChange={(v) => (stateField = v)}
    />
    {#if stateHints.length > 0 && !readonly}
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
</style>

<script lang="ts">
  // type_id=1 = "Устройство" (V001 seed). The /devices section creates only devices of this
  // internal type. /printers (Phase 6) will hardcode type_id=2. "Тип" is NOT a user-facing
  // field — it is an internal entity-class discriminator set automatically by the UI section.
  import { onMount } from 'svelte';
  import Modal from '$lib/components/Modal.svelte';
  import Button from '$lib/components/Button.svelte';
  import Input from '$lib/components/Input.svelte';
  import Select from '$lib/components/Select.svelte';
  import Textarea from '$lib/components/Textarea.svelte';
  import DeviceAutocompleteField from './DeviceAutocompleteField.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { devices } from './api';
  import type { DeviceDto, DeviceNew, DevicePatch } from '../../bindings';

  // ---------------------------------------------------------------------------
  // Placeholder lookups (Plan 04 wires real Tauri queries)
  // ---------------------------------------------------------------------------
  // NOTE: DEVICE_TYPES removed — "Тип" is an internal discriminator, not user-facing.
  // The /devices section always uses type_id=1 ("Устройство").
  // The /printers section (Phase 6) will use type_id=2 ("Принтер").

  const STATUSES = [
    { id: 1, label: 'На складе' },
    { id: 2, label: 'В работе' },
    { id: 3, label: 'На ремонте' },
    { id: 4, label: 'Списано' },
  ];

  // ---------------------------------------------------------------------------
  // Props
  // ---------------------------------------------------------------------------
  interface Props {
    open: boolean;
    target: DeviceDto | null;
    onClose: () => void;
    onSaved: () => void;
  }

  const { open, target, onClose, onSaved }: Props = $props();

  // ---------------------------------------------------------------------------
  // Form state (reset on open)
  // ---------------------------------------------------------------------------
  // typeId is intentionally NOT in form state — /devices always creates with type_id=1.
  let name = $state('');
  let location = $state('');
  let statusId = $state('');
  let inventoryNo = $state('');
  let serialNo = $state('');
  let model = $state('');
  let specs = $state('');
  let kit = $state('');
  let stateField = $state('');

  // Bulk-create quantity (scope extension 2026-05-26).
  // Only shown in create mode AND when both inv/serial are empty.
  let quantity = $state<number>(1);

  let stateHints = $state<string[]>([]);
  let loading = $state(false);
  let fieldErrors = $state<Record<string, string>>({});

  const isEdit = $derived(target !== null);
  const modalTitle = $derived(isEdit ? 'Редактирование устройства' : 'Новое устройство');
  const submitLabel = $derived(isEdit ? 'Сохранить' : 'Создать');

  const canSubmit = $derived(name.trim() !== '' && location.trim() !== '' && statusId !== '');

  // Show quantity field: create mode only, AND both inv/serial are empty.
  const showQuantity = $derived(
    !isEdit &&
    inventoryNo.trim() === '' &&
    serialNo.trim() === ''
  );

  // Reset form whenever the modal opens.
  $effect(() => {
    if (open) {
      fieldErrors = {};
      quantity = 1;
      if (target) {
        name = target.name;
        location = target.specs ?? ''; // location is freetext until Plan 04 location lookup
        statusId = String(target.status_id);
        inventoryNo = target.inventory_no ?? '';
        serialNo = target.serial_no ?? '';
        model = target.model ?? '';
        specs = target.specs ?? '';
        kit = target.kit ?? '';
        stateField = target.state ?? '';
      } else {
        name = '';
        location = '';
        statusId = '';
        inventoryNo = '';
        serialNo = '';
        model = '';
        specs = '';
        kit = '';
        stateField = '';
      }
    }
  });

  // Reset quantity to 1 when user fills in inv/serial.
  $effect(() => {
    if (inventoryNo.trim() !== '' || serialNo.trim() !== '') {
      quantity = 1;
    }
  });

  onMount(async () => {
    try {
      stateHints = await devices.stateHints();
    } catch {
      // Non-fatal — chips won't appear but form still works
    }
  });

  // ---------------------------------------------------------------------------
  // Submit
  // ---------------------------------------------------------------------------
  async function handleSubmit() {
    if (!canSubmit) return;
    loading = true;
    fieldErrors = {};

    try {
      if (isEdit && target) {
        const patch: DevicePatch = {
          // type_id intentionally omitted — edit never changes the internal type discriminator
          type_id: null,
          name: name.trim() || null,
          inventory_no: inventoryNo.trim() || null,
          serial_no: serialNo.trim() || null,
          model: model.trim() || null,
          specs: specs.trim() || null,
          kit: kit.trim() || null,
          state: stateField.trim() || null,
          location_id: null,
          status_id: parseInt(statusId, 10) || null,
        };
        await devices.update(target.id, target.version, patch);
        pushToast('success', 'Устройство сохранено');
      } else {
        const newDevice: DeviceNew = {
          // type_id=1 hardcoded: /devices section always creates "Устройство" (V001 seed id=1)
          type_id: 1,
          name: name.trim(),
          inventory_no: inventoryNo.trim() || null,
          serial_no: serialNo.trim() || null,
          model: model.trim() || null,
          specs: specs.trim() || null,
          kit: kit.trim() || null,
          state: stateField.trim() || null,
          location_id: null,
          status_id: parseInt(statusId, 10),
        };

        // Use bulk_create for all create operations (count=1 is equivalent to create).
        const qty = showQuantity ? Math.max(1, Math.min(100, quantity || 1)) : 1;
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
    }
  }
</script>

<Modal {open} title={modalTitle} size="md" {onClose}>
  <form
    class="device-form"
    onsubmit={(e) => {
      e.preventDefault();
      handleSubmit();
    }}
  >
    <!-- Required: Наименование (with autocomplete) -->
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

    <!-- Required: Расположение (with autocomplete, contextual) -->
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
        onChange={(v) => (location = v)}
      />
      {#if fieldErrors['location']}
        <p class="field-error">{fieldErrors['location']}</p>
      {/if}
    </div>

    <!-- Required: Статус -->
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

    <!-- Optional: Инвентарный № -->
    <div class="field">
      <label class="label" for="f-inv">Инвентарный №</label>
      <Input
        id="f-inv"
        value={inventoryNo}
        placeholder="ИНВ-000001"
        oninput={(v) => (inventoryNo = v)}
      />
    </div>

    <!-- Optional: Серийный № -->
    <div class="field">
      <label class="label" for="f-serial">Серийный №</label>
      <Input
        id="f-serial"
        value={serialNo}
        placeholder="SN-XXXXXXXX"
        oninput={(v) => (serialNo = v)}
      />
    </div>

    <!-- Quantity (scope extension: bulk create for non-unique devices) -->
    {#if showQuantity}
      <div class="field">
        <label class="label" for="f-qty">Количество</label>
        <input
          id="f-qty"
          type="number"
          class="input"
          min={1}
          max={100}
          value={quantity}
          oninput={(e) => {
            const v = parseInt((e.currentTarget as HTMLInputElement).value, 10);
            quantity = isNaN(v) ? 1 : Math.max(1, Math.min(100, v));
          }}
        />
        <p class="field-help">При создании одинаковых устройств без серийного и инвентарного номеров.</p>
      </div>
    {/if}

    <!-- Optional: Модель (with autocomplete, contextual) -->
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

    <!-- Optional: Технические характеристики -->
    <div class="field">
      <label class="label" for="f-specs">Технические характеристики</label>
      <Textarea
        id="f-specs"
        value={specs}
        placeholder="i7-1365U, 16 ГБ RAM, 512 ГБ SSD"
        rows={2}
        oninput={(v) => (specs = v)}
      />
    </div>

    <!-- Optional: Комплектация -->
    <div class="field">
      <label class="label" for="f-kit">Комплектация</label>
      <Textarea
        id="f-kit"
        value={kit}
        placeholder="Зарядное устройство, мышь"
        rows={2}
        oninput={(v) => (kit = v)}
      />
    </div>

    <!-- Optional: Состояние + state-hints chips (with autocomplete) -->
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

  {#snippet footer()}
    <Button variant="secondary" onclick={onClose}>Отмена</Button>
    <Button variant="primary" {loading} disabled={!canSubmit} onclick={handleSubmit}>
      {#if loading}Сохранение…{:else}{submitLabel}{/if}
    </Button>
  {/snippet}
</Modal>

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

  .field-help {
    margin: 0;
    font-size: var(--font-size-label);
    color: var(--color-text-secondary);
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

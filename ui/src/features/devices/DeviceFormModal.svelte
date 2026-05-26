<script lang="ts">
  import { onMount } from 'svelte';
  import Modal from '$lib/components/Modal.svelte';
  import Button from '$lib/components/Button.svelte';
  import Input from '$lib/components/Input.svelte';
  import Select from '$lib/components/Select.svelte';
  import Textarea from '$lib/components/Textarea.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { devices } from './api';
  import type { DeviceDto, DeviceNew, DevicePatch } from '../../bindings';

  // ---------------------------------------------------------------------------
  // Placeholder lookups (Plan 04 wires real Tauri queries)
  // ---------------------------------------------------------------------------
  const DEVICE_TYPES = [
    { id: 1, label: 'Компьютер' },
    { id: 2, label: 'Ноутбук' },
    { id: 3, label: 'Монитор' },
    { id: 4, label: 'Принтер' },
    { id: 5, label: 'МФУ' },
    { id: 6, label: 'Сервер' },
    { id: 7, label: 'Сетевое оборудование' },
    { id: 8, label: 'Периферия' },
    { id: 9, label: 'Прочее' },
  ];

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
  let typeId = $state('');
  let name = $state('');
  let location = $state('');
  let statusId = $state('');
  let inventoryNo = $state('');
  let serialNo = $state('');
  let model = $state('');
  let specs = $state('');
  let kit = $state('');
  let stateField = $state('');

  let stateHints = $state<string[]>([]);
  let loading = $state(false);
  let fieldErrors = $state<Record<string, string>>({});

  const isEdit = $derived(target !== null);
  const modalTitle = $derived(isEdit ? 'Редактирование устройства' : 'Новое устройство');
  const submitLabel = $derived(isEdit ? 'Сохранить' : 'Создать');

  const canSubmit = $derived(
    typeId !== '' && name.trim() !== '' && location.trim() !== '' && statusId !== '',
  );

  // Reset form whenever the modal opens
  $effect(() => {
    if (open) {
      fieldErrors = {};
      if (target) {
        typeId = String(target.type_id);
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
        typeId = '';
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
          type_id: parseInt(typeId, 10) || null,
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
          type_id: parseInt(typeId, 10),
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
        await devices.create(newDevice);
        pushToast('success', 'Устройство создано');
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
    <!-- Required: Тип -->
    <div class="field" class:has-error={!!fieldErrors['type_id']}>
      <label class="label" for="f-type"
        >Тип <span class="required" aria-hidden="true">*</span></label
      >
      <Select
        id="f-type"
        value={typeId}
        invalid={!!fieldErrors['type_id']}
        onchange={(v) => (typeId = v)}
      >
        <option value="">— выберите тип —</option>
        {#each DEVICE_TYPES as t}
          <option value={String(t.id)}>{t.label}</option>
        {/each}
      </Select>
      {#if fieldErrors['type_id']}
        <p class="field-error">{fieldErrors['type_id']}</p>
      {/if}
    </div>

    <!-- Required: Наименование -->
    <div class="field" class:has-error={!!fieldErrors['name']}>
      <label class="label" for="f-name">
        Наименование <span class="required" aria-hidden="true">*</span>
      </label>
      <Input
        id="f-name"
        value={name}
        placeholder="Ноутбук Lenovo ThinkPad X1"
        invalid={!!fieldErrors['name']}
        oninput={(v) => (name = v)}
      />
      {#if fieldErrors['name']}
        <p class="field-error">{fieldErrors['name']}</p>
      {/if}
    </div>

    <!-- Required: Расположение (freetext until Plan 04) -->
    <div class="field" class:has-error={!!fieldErrors['location']}>
      <label class="label" for="f-location">
        Расположение <span class="required" aria-hidden="true">*</span>
      </label>
      <Input
        id="f-location"
        value={location}
        placeholder="Кабинет 305"
        invalid={!!fieldErrors['location']}
        oninput={(v) => (location = v)}
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

    <!-- Optional: Модель -->
    <div class="field">
      <label class="label" for="f-model">Модель</label>
      <Input
        id="f-model"
        value={model}
        placeholder="ThinkPad X1 Carbon Gen 12"
        oninput={(v) => (model = v)}
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

    <!-- Optional: Состояние + state-hints chips -->
    <div class="field">
      <label class="label" for="f-state">Состояние</label>
      <Input
        id="f-state"
        value={stateField}
        placeholder="Хорошее"
        oninput={(v) => (stateField = v)}
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

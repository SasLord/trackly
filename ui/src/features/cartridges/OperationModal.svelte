<script lang="ts">
  // Plan 04-05: OperationModal — единая параметризованная модалка для 5 lifecycle-операций.
  // По образцу ReturnModal.svelte: $effect reset при open, submitting state, handleSubmit с try/catch + pushToast.
  //
  // op prop: 'install' | 'return_to_stock' | 'to_refill' | 'from_refill' | 'write_off'
  // Заголовки, поля, дефолты — по UI-SPEC §Поля OperationModal + D-Op-Fields-01.
  import Modal from '$lib/components/Modal.svelte';
  import Button from '$lib/components/Button.svelte';
  import DatePicker from '$lib/components/DatePicker.svelte';
  import Select from '$lib/components/Select.svelte';
  import Textarea from '$lib/components/Textarea.svelte';
  import PersonAutocomplete from '$lib/components/PersonAutocomplete.svelte';
  import LocationAutocomplete from '$lib/components/LocationAutocomplete.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { cartridges } from './api';
  import type { CartridgeDto, CartridgeTransitionPayload } from '../../bindings';

  type Op = 'install' | 'return_to_stock' | 'to_refill' | 'from_refill' | 'write_off';

  interface Props {
    open: boolean;
    op: Op;
    cartridge: CartridgeDto | null;
    onClose: () => void;
    onSuccess: () => void;
  }

  const { open, op, cartridge, onClose, onSuccess }: Props = $props();

  // --- Form state ---
  let dateIso = $state(''); // ISO YYYY-MM-DD (DatePicker output)
  let givenByName = $state('');
  let givenToName = $state('');
  let location = $state('');
  let stateId = $state(3); // default: Пустой (D-Op-Fields-01)
  let notes = $state('');
  let submitting = $state(false);

  // Validation errors
  let locationError = $state('');
  let givenByError = $state('');
  let givenToError = $state('');

  // D-Op-Fields-01: from_refill → Полный (1), остальные → Пустой (3)
  // install: поле состояния не показывается (не меняется при install)
  const defaultStateId = $derived(op === 'from_refill' ? 1 : 3);

  // Reset form when modal opens
  $effect(() => {
    if (open) {
      const now = new Date();
      const y = now.getFullYear();
      const m = String(now.getMonth() + 1).padStart(2, '0');
      const d = String(now.getDate()).padStart(2, '0');
      dateIso = `${y}-${m}-${d}`;
      givenByName = '';
      givenToName = '';
      location = '';
      notes = '';
      stateId = defaultStateId;
      locationError = '';
      givenByError = '';
      givenToError = '';
    }
  });

  // Modal titles (UI-SPEC §Заголовки OperationModal)
  const MODAL_TITLES: Record<Op, string> = {
    install: 'Установка в принтер',
    return_to_stock: 'Возврат на склад',
    to_refill: 'Отправка на заправку',
    from_refill: 'Получение с заправки',
    write_off: 'Списание картриджа',
  };

  // Confirm button labels (UI-SPEC §Primary CTA)
  const CONFIRM_LABELS: Record<Op, string> = {
    install: 'Установить',
    return_to_stock: 'Вернуть на склад',
    to_refill: 'Отправить на заправку',
    from_refill: 'Вернуть с заправки',
    write_off: 'Списать',
  };

  // State options for Select (Состояние заряда)
  const STATE_OPTIONS = [
    { value: 1, label: 'Полный' },
    { value: 2, label: 'Частичный' },
    { value: 3, label: 'Пустой' },
  ];

  // Convert ISO date string to unix seconds
  function isoToUnix(iso: string): number {
    if (!iso) return Math.floor(Date.now() / 1000);
    return Math.floor(new Date(iso + 'T00:00:00Z').getTime() / 1000);
  }

  // Build payload from form state
  function buildPayload(): CartridgeTransitionPayload {
    const id = cartridge!.id;
    const version = cartridge!.version;

    if (op === 'install') {
      return {
        op: 'install',
        cartridge_id: id,
        version,
        date_utc: isoToUnix(dateIso),
        given_by_name: givenByName.trim(),
        given_to_name: givenToName.trim(),
        location: location.trim(),
      };
    } else if (op === 'return_to_stock') {
      return {
        op: 'return_to_stock',
        cartridge_id: id,
        version,
        state_id: stateId,
        location: location.trim(),
        notes: notes.trim() || null,
      };
    } else if (op === 'to_refill') {
      return {
        op: 'to_refill',
        cartridge_id: id,
        version,
        date_utc: isoToUnix(dateIso),
        given_by_name: givenByName.trim(),
        given_to_name: givenToName.trim(),
        location: location.trim(),
      };
    } else if (op === 'from_refill') {
      return {
        op: 'from_refill',
        cartridge_id: id,
        version,
        state_id: stateId,
        location: location.trim(),
        notes: notes.trim() || null,
      };
    } else {
      // write_off
      return {
        op: 'write_off',
        cartridge_id: id,
        version,
        date_utc: isoToUnix(dateIso),
        notes: notes.trim() || null,
      };
    }
  }

  function validate(): boolean {
    let valid = true;
    locationError = '';
    givenByError = '';
    givenToError = '';

    if (op === 'install' || op === 'to_refill') {
      if (!givenByName.trim()) {
        givenByError = 'Заполните это поле';
        valid = false;
      }
      if (!givenToName.trim()) {
        givenToError = 'Заполните это поле';
        valid = false;
      }
      if (!location.trim()) {
        locationError = 'Заполните это поле';
        valid = false;
      }
    } else if (op === 'return_to_stock' || op === 'from_refill') {
      if (!location.trim()) {
        locationError = 'Заполните это поле';
        valid = false;
      }
    }
    // write_off: no required fields beyond date (auto-filled)

    return valid;
  }

  async function handleSubmit() {
    if (!cartridge || submitting) return;
    if (!validate()) return;

    submitting = true;
    try {
      await cartridges.transition(buildPayload());
      onSuccess();
      onClose();
      pushToast('success', `Операция выполнена успешно.`);
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось выполнить операцию. Повторите попытку.';
      pushToast('error', msg);
    } finally {
      submitting = false;
    }
  }

  const modalTitle = $derived(MODAL_TITLES[op] ?? 'Операция');
  const confirmLabel = $derived(CONFIRM_LABELS[op] ?? 'Подтвердить');

  // canSubmit — simple check (required validation happens in handleSubmit)
  const canSubmit = $derived(!submitting && !!cartridge);
</script>

<Modal {open} title={modalTitle} size="md" {onClose}>
  <form
    class="form"
    onsubmit={(e) => {
      e.preventDefault();
      handleSubmit();
    }}
  >
    <!-- Поля по op (UI-SPEC §Поля OperationModal) -->

    {#if op === 'install' || op === 'to_refill'}
      <!-- Дата -->
      <div class="field">
        <label class="label" for="op-date">Дата</label>
        <DatePicker bind:value={dateIso} id="op-date" required />
      </div>

      <!-- Кто выдал -->
      <div class="field">
        <label class="label" for="op-given-by">Кто выдал</label>
        <PersonAutocomplete
          field="giver"
          bind:value={givenByName}
          placeholder="ФИО выдавшего"
          id="op-given-by"
          invalid={!!givenByError}
        />
        {#if givenByError}
          <span class="field-error">{givenByError}</span>
        {/if}
      </div>

      <!-- Кому выдал -->
      <div class="field">
        <label class="label" for="op-given-to">Кому выдал</label>
        <PersonAutocomplete
          field="receiver"
          bind:value={givenToName}
          placeholder="ФИО получившего"
          id="op-given-to"
          invalid={!!givenToError}
        />
        {#if givenToError}
          <span class="field-error">{givenToError}</span>
        {/if}
      </div>

      <!-- Расположение -->
      <div class="field">
        <label class="label" for="op-location">Расположение</label>
        <LocationAutocomplete
          value={location}
          placeholder="Расположение"
          id="op-location"
          invalid={!!locationError}
          onChange={(v) => (location = v)}
        />
        {#if locationError}
          <span class="field-error">{locationError}</span>
        {:else if op === 'install'}
          <span class="field-hint">Укажите рабочее место или кабинет (не склад)</span>
        {/if}
      </div>
    {:else if op === 'return_to_stock' || op === 'from_refill'}
      <!-- Состояние заряда -->
      <div class="field">
        <label class="label" for="op-state">Состояние заряда</label>
        <Select value={String(stateId)} id="op-state" onchange={(v) => (stateId = parseInt(v, 10))}>
          {#each STATE_OPTIONS as opt (opt.value)}
            <option value={String(opt.value)}>{opt.label}</option>
          {/each}
        </Select>
      </div>

      <!-- Расположение -->
      <div class="field">
        <label class="label" for="op-location">Расположение</label>
        <LocationAutocomplete
          value={location}
          placeholder="Расположение"
          id="op-location"
          invalid={!!locationError}
          onChange={(v) => (location = v)}
        />
        {#if locationError}
          <span class="field-error">{locationError}</span>
        {:else if op === 'return_to_stock'}
          <span class="field-hint">Укажите склад или место хранения</span>
        {/if}
      </div>

      <!-- Примечание (optional) -->
      <div class="field">
        <label class="label" for="op-notes">Примечание</label>
        <Textarea
          value={notes}
          placeholder="Необязательно"
          id="op-notes"
          oninput={(v) => (notes = v)}
        />
      </div>
    {:else if op === 'write_off'}
      <!-- Дата -->
      <div class="field">
        <label class="label" for="op-date">Дата</label>
        <DatePicker bind:value={dateIso} id="op-date" required />
      </div>

      <!-- Причина / Примечание (optional) -->
      <div class="field">
        <label class="label" for="op-notes">Причина / Примечание</label>
        <Textarea
          value={notes}
          placeholder="Необязательно"
          id="op-notes"
          oninput={(v) => (notes = v)}
        />
      </div>
    {/if}
  </form>

  {#snippet footer()}
    <Button variant="secondary" onclick={onClose}>Отмена</Button>
    <Button variant="primary" loading={submitting} disabled={!canSubmit} onclick={handleSubmit}>
      {confirmLabel}
    </Button>
  {/snippet}
</Modal>

<style lang="scss">
  .form {
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
    color: var(--color-text-secondary);
    font-weight: var(--font-weight-regular);
  }

  .field-hint {
    font-size: var(--font-size-label);
    color: var(--color-text-muted);
  }

  .field-error {
    font-size: var(--font-size-label);
    color: var(--color-destructive);
  }
</style>

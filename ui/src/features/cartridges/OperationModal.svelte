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
  import CartridgeSelect from '$lib/components/CartridgeSelect.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { cartridges } from './api';
  import type { CartridgeDto, CartridgeTransitionPayload } from '../../bindings';

  type Op = 'install' | 'return_to_stock' | 'to_refill' | 'from_refill' | 'write_off';

  interface Props {
    open: boolean;
    op: Op;
    cartridge: CartridgeDto | null;
    /** Pre-fill the «Принтер» context when op='install' is opened from a request (REQ-05). */
    preFillPrinterId?: number;
    /** Filter the request-centric cartridge picker to the request's model (D-02). */
    cartridgeModelId?: number;
    /** Pre-fill «Расположение» from the request's printer location (D-05). */
    prefillLocation?: string;
    /** Pre-fill «Кому отдал» from the requester's name (D-04). */
    prefillGivenToName?: string;
    onClose: () => void;
    /**
     * WR-03: may return a Promise (e.g. the request-centric flow awaits a
     * follow-up `complete` transition). `handleSubmit` awaits this before
     * showing the modal-level success toast, so a rejected follow-up never
     * produces a false-positive "Операция выполнена успешно." alongside the
     * caller's own error toast.
     */
    onSuccess: (_cartridgeId: number) => void | Promise<void>;
  }

  const {
    open,
    op,
    cartridge,
    preFillPrinterId,
    cartridgeModelId,
    prefillLocation,
    prefillGivenToName,
    onClose,
    onSuccess,
  }: Props = $props();

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

  // D-01..D-08 (Phase 12 Plan 03): request-centric install flow. When the
  // caller passes `cartridge={null}` with `op='install'` (RequestDetail),
  // the modal loads a flat picker of installable stock cartridges instead
  // of operating on a pre-selected cartridge. The old cartridge-centric
  // entry (menu → «Установить в принтер», `cartridge` prop set) is
  // unaffected (D-08) — `effectiveCartridge` simply prefers the prop.
  let selectedCartridge = $state<CartridgeDto | null>(null);
  let cartridgeOptions = $state<CartridgeDto[]>([]);
  let cartridgeListLoading = $state(false);

  const effectiveCartridge = $derived(cartridge ?? selectedCartridge);

  // Вид расходника: фотобарабан (kind 2) использует другой набор состояний.
  const isDrum = $derived(effectiveCartridge?.model_kind_id === 2);

  // D-Op-Fields-01: from_refill → Полный (1), остальные → Пустой (3).
  // Для фотобарабана при возврате на склад по умолчанию «Отработанный» (6)
  // (UAT R4 №3). install: поле состояния не показывается.
  const defaultStateId = $derived(isDrum ? 6 : op === 'from_refill' ? 1 : 3);

  // Reset form when modal opens or when `op` changes while modal is already
  // open (WR-03: stateId must track defaultStateId whenever op changes, not
  // only on open→close cycle).
  $effect(() => {
    void op; // explicit dependency: re-run when op changes
    if (open) {
      const now = new Date();
      const y = now.getFullYear();
      const m = String(now.getMonth() + 1).padStart(2, '0');
      const d = String(now.getDate()).padStart(2, '0');
      dateIso = `${y}-${m}-${d}`;
      givenByName = '';
      givenToName = prefillGivenToName ?? '';
      location = prefillLocation ?? '';
      notes = '';
      stateId = defaultStateId;
      selectedCartridge = null;
      locationError = '';
      givenByError = '';
      givenToError = '';
    }
  });

  // REQ-05: preFillPrinterId is accepted as context when the modal is opened
  // from a request (RequestDetail). The install form is cartridge-centric;
  // we show a hint about which printer this cartridge targets when the prop is set.
  const printerContextHint = $derived(
    op === 'install' && preFillPrinterId !== undefined
      ? `Устанавливается в принтер #${preFillPrinterId}`
      : null,
  );

  // WR-02: when the request-centric install flow (cartridge === null) has no
  // cartridge_model_id, the picker below cannot scope the list to a model —
  // it lists every installable cartridge regardless of model/printer fit.
  // Surface an explicit warning so the operator checks compatibility by hand
  // instead of silently trusting an unscoped list.
  const noModelScopeWarning = $derived(
    op === 'install' && cartridge === null && cartridgeModelId === undefined
      ? 'Модель не указана — проверьте совместимость вручную'
      : null,
  );

  // D-01/D-02 (Phase 12 Plan 03): load the installable-stock cartridge list
  // when the modal is opened for the request-centric install flow
  // (cartridge prop === null). The cartridge-centric flow (menu →
  // «Установить в принтер») never triggers this — `cartridge` is non-null
  // there, so no extra network call is made (D-08 regression guard).
  $effect(() => {
    if (!(open && op === 'install' && cartridge === null)) return;
    cartridgeListLoading = true;
    cartridges
      .list(
        {
          status_id: 1,
          installable_only: true,
          model_id: cartridgeModelId ?? null,
          kind_id: null,
          search: null,
          include_deleted: false,
        },
        { offset: 0, limit: 200 },
      )
      .then((res) => {
        cartridgeOptions = res.items;
      })
      .catch((e: unknown) => {
        cartridgeOptions = [];
        const msg =
          e && typeof e === 'object' && 'message' in e
            ? String((e as { message: unknown }).message)
            : 'Не удалось загрузить список картриджей.';
        pushToast('error', msg);
      })
      .finally(() => {
        cartridgeListLoading = false;
      });
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

  // State options for Select — по виду расходника (V017).
  const CARTRIDGE_STATES = [
    { value: 1, label: 'Полный' },
    { value: 2, label: 'Частичный' },
    { value: 3, label: 'Пустой' },
  ];
  const DRUM_STATES = [
    { value: 4, label: 'Новый' },
    { value: 5, label: 'Изношенный' },
    { value: 6, label: 'Отработанный' },
  ];
  const stateOptions = $derived(isDrum ? DRUM_STATES : CARTRIDGE_STATES);
  const stateFieldLabel = $derived(isDrum ? 'Состояние' : 'Состояние заряда');

  // Convert ISO date string to unix seconds
  function isoToUnix(iso: string): number {
    if (!iso) return Math.floor(Date.now() / 1000);
    return Math.floor(new Date(iso + 'T00:00:00Z').getTime() / 1000);
  }

  // Build payload from form state
  function buildPayload(): CartridgeTransitionPayload {
    const id = effectiveCartridge!.id;
    const version = effectiveCartridge!.version;

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
    if (!effectiveCartridge || submitting) return;
    if (!validate()) return;

    submitting = true;
    try {
      await cartridges.transition(buildPayload());
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось выполнить операцию. Повторите попытку.';
      pushToast('error', msg);
      submitting = false;
      return;
    }

    // WR-03: cartridge transition succeeded — now await the caller's
    // onSuccess (e.g. RequestDetail's handleInstallSuccess, which completes
    // the request). Only announce the modal-level success once onSuccess
    // resolves; if it rejects, the caller is responsible for its own
    // error toast (it already owns the more specific failure message), so
    // we just close without adding a duplicate/contradictory toast here.
    try {
      await onSuccess(effectiveCartridge.id);
      onClose();
      pushToast('success', `Операция выполнена успешно.`);
    } catch {
      onClose();
    } finally {
      submitting = false;
    }
  }

  const modalTitle = $derived(MODAL_TITLES[op] ?? 'Операция');
  const confirmLabel = $derived(CONFIRM_LABELS[op] ?? 'Подтвердить');

  // canSubmit — simple check (required validation happens in handleSubmit)
  const canSubmit = $derived(!submitting && !!effectiveCartridge);
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
      {#if op === 'install' && cartridge === null}
        <!-- D-01/D-02/D-03/D-08: request-centric install flow — pick a
             physical cartridge from the installable-stock list. Not shown
             when `cartridge` is already set (old cartridge-centric entry). -->
        <div class="field">
          <label class="label" for="op-cartridge">Картридж</label>
          <CartridgeSelect
            options={cartridgeOptions}
            value={selectedCartridge ? String(selectedCartridge.id) : ''}
            disabled={cartridgeListLoading}
            id="op-cartridge"
            onchange={(v) => {
              selectedCartridge = cartridgeOptions.find((c) => String(c.id) === v) ?? null;
            }}
          />
          {#if noModelScopeWarning}
            <span class="field-warning">{noModelScopeWarning}</span>
          {/if}
        </div>
      {/if}
      {#if printerContextHint}
        <p class="field-hint">{printerContextHint}</p>
      {/if}
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
      <!-- Состояние (заряда — для картриджей; для фотобарабанов — состояние) -->
      <div class="field">
        <label class="label" for="op-state">{stateFieldLabel}</label>
        <Select value={String(stateId)} id="op-state" onchange={(v) => (stateId = parseInt(v, 10))}>
          {#each stateOptions as opt (opt.value)}
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

  .field-warning {
    font-size: var(--font-size-label);
    color: var(--color-warning);
  }
</style>

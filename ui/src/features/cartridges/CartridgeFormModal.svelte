<script lang="ts">
  // Plan 04-05: CartridgeFormModal — CRUD модалка создания/редактирования экземпляра.
  // По образцу DeviceFormModal.svelte + ActFormModal.svelte:
  //   - openInstanceCounter паттерн для сброса формы при каждом открытии.
  //   - isEdit = target !== null.
  //   - Код можно оставить пустым (авто C-XXXXXX) или ввести вручную.
  //
  // Архитектура: форма вынесена в отдельный компонент CartridgeFormBody.svelte
  // для совместимости с {#key openInstanceCounter} паттерном сброса состояния.
  //
  // Phase 40.2 Plan 15 (NUM-06/07/08/09/10/11/12, D-01): the same
  // containing-block reasoning that made DeviceFormModal.svelte/
  // ActFormModal.svelte render the D-01 save-chain popups
  // («Номер занят»/«Не соответствует шаблону»/«Проверьте буквы в номере»)
  // as top-level siblings (Plans 13/14) applies here — CartridgeFormBody
  // OWNS their orchestration (occupied/mismatch/script-mix chain, button
  // behaviour) but reports it up via `onPopupChange`; THIS component
  // renders the actual popups.
  import Modal from '$lib/components/Modal.svelte';
  import Button from '$lib/components/Button.svelte';
  import NumberTakenPopup from '$lib/components/NumberTakenPopup.svelte';
  import NumberMismatchPopup from '$lib/components/NumberMismatchPopup.svelte';
  import NumberScriptWarningPopup from '$lib/components/NumberScriptWarningPopup.svelte';
  import CartridgeFormBody from './CartridgeFormBody.svelte';
  import type { CartridgeNumberPopupState } from './CartridgeFormBody.svelte';
  import type { CartridgeDto, CartridgeModelDto } from '../../bindings';

  interface Props {
    open: boolean;
    target: CartridgeDto | null; // null = создание
    models: CartridgeModelDto[];
    onClose: () => void;
    onSuccess: (_cart: CartridgeDto) => void;
  }

  const { open, target, models, onClose, onSuccess }: Props = $props();

  const isEdit = $derived(target !== null);
  // При редактировании заголовок зависит от вида (картридж/фотобарабан);
  // при создании вид ещё не выбран — общий заголовок.
  const editKindNoun = $derived(target?.model_kind_id === 2 ? 'фотобарабана' : 'картриджа');
  const modalTitle = $derived(
    isEdit ? `Редактирование ${editKindNoun}` : 'Новый картридж/фотобарабан',
  );
  const submitLabel = $derived(isEdit ? 'Сохранить изменения' : 'Добавить');

  // ---------------------------------------------------------------------------
  // Form instance counter — incremented each time the modal opens (false → true).
  // The {#key} block remounts CartridgeFormBody on every open, guaranteeing
  // all form fields reset to their initial values.
  // ---------------------------------------------------------------------------
  let openInstanceCounter = $state(0);
  let _wasOpen = $state(false);

  $effect(() => {
    const isOpen = open;
    if (isOpen && !_wasOpen) {
      openInstanceCounter += 1;
    }
    _wasOpen = isOpen;
  });

  // Footer button state — driven by CartridgeFormBody callbacks.
  let formLoading = $state(false);
  let formCanSubmit = $state(false);
  let bodySubmitFn = $state<(() => void) | null>(null);

  // Phase 40.2 Plan 15 (D-01): the current D-01 save-chain popup (or null)
  // — owned/orchestrated by CartridgeFormBody, rendered here as a
  // TOP-LEVEL sibling of the edit Modal (see this file's header comment).
  let numberPopup = $state<CartridgeNumberPopupState>(null);
</script>

<Modal {open} title={modalTitle} size="md" {onClose}>
  {#key openInstanceCounter}
    <CartridgeFormBody
      {target}
      {models}
      {onClose}
      {onSuccess}
      onLoading={(l) => (formLoading = l)}
      onCanSubmitChange={(can) => (formCanSubmit = can)}
      onRegisterSubmit={(fn) => (bodySubmitFn = fn)}
      onPopupChange={(popup) => (numberPopup = popup)}
    />
  {/key}

  {#snippet footer()}
    <Button variant="secondary" onclick={onClose}>Отмена</Button>
    <Button
      variant="primary"
      loading={formLoading}
      disabled={!formCanSubmit}
      onclick={() => bodySubmitFn?.()}
    >
      {#if formLoading}Сохранение…{:else}{submitLabel}{/if}
    </Button>
  {/snippet}
</Modal>

{#if numberPopup?.kind === 'taken'}
  <NumberTakenPopup
    number={numberPopup.number}
    record={numberPopup.record}
    canTakeNext={numberPopup.canTakeNext}
    loadingTakeNext={numberPopup.loadingTakeNext}
    onTakeNext={numberPopup.onTakeNext}
    onClose={numberPopup.onClose}
  />
{:else if numberPopup?.kind === 'mismatch'}
  <NumberMismatchPopup
    number={numberPopup.number}
    mask={numberPopup.mask}
    contextLabel={numberPopup.contextLabel}
    onFix={numberPopup.onFix}
    onContinue={numberPopup.onContinue}
  />
{:else if numberPopup?.kind === 'scriptWarning'}
  <NumberScriptWarningPopup
    number={numberPopup.number}
    message={numberPopup.message}
    doppelganger={numberPopup.doppelganger}
    onFix={numberPopup.onFix}
    onContinue={numberPopup.onContinue}
  />
{/if}

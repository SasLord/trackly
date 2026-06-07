<script lang="ts">
  // Plan 04-05: CartridgeFormModal — CRUD модалка создания/редактирования экземпляра.
  // По образцу DeviceFormModal.svelte + ActFormModal.svelte:
  //   - openInstanceCounter паттерн для сброса формы при каждом открытии.
  //   - isEdit = target !== null.
  //   - Код можно оставить пустым (авто C-XXXXXX) или ввести вручную.
  //
  // Архитектура: форма вынесена в отдельный компонент CartridgeFormBody.svelte
  // для совместимости с {#key openInstanceCounter} паттерном сброса состояния.
  import Modal from '$lib/components/Modal.svelte';
  import Button from '$lib/components/Button.svelte';
  import CartridgeFormBody from './CartridgeFormBody.svelte';
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
  const modalTitle = $derived(isEdit ? 'Редактирование картриджа' : 'Новый картридж');
  const submitLabel = $derived(isEdit ? 'Сохранить изменения' : 'Добавить картридж');

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

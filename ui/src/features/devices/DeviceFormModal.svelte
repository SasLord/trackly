<script lang="ts">
  // DeviceFormModal — outer shell: Modal + footer buttons + form-instance lifecycle.
  //
  // Form state lives entirely inside DeviceFormBody.svelte.
  // {#key openInstanceCounter} remounts DeviceFormBody on every open, guaranteeing
  // all form fields reset to their initial values — no stale serial/inv data
  // carries over between create sessions (Regression 6 fix).
  //
  // Round 8 refactor: submitTrigger side-channel eliminated.
  // The footer button now calls bodySubmitFn() directly — a function bound from
  // DeviceFormBody via `bind:submit`. No reactive trigger, no race condition.
  import { onMount } from 'svelte';
  import Modal from '$lib/components/Modal.svelte';
  import Button from '$lib/components/Button.svelte';
  import ActionMenu from '$lib/components/ActionMenu.svelte';
  import DeviceFormBody from './DeviceFormBody.svelte';
  import { devices } from './api';
  import type { DeviceDto } from '../../bindings';

  interface Props {
    open: boolean;
    target: DeviceDto | null;
    onClose: () => void;
    onSaved: () => void;
  }

  const { open, target, onClose, onSaved }: Props = $props();

  const DEVICE_TYPE_ID = 1;
  const PRINTER_TYPE_ID = 2;

  const isEdit = $derived(target !== null);
  let typeId = $state(DEVICE_TYPE_ID);
  const modalTitle = $derived.by(() => {
    const isPrinter = typeId === PRINTER_TYPE_ID;
    if (isEdit) return isPrinter ? 'Редактирование принтера' : 'Редактирование устройства';
    return isPrinter ? 'Новый принтер' : 'Новое устройство';
  });
  const submitLabel = $derived(isEdit ? 'Сохранить' : 'Создать');

  // ---------------------------------------------------------------------------
  // Form instance counter — incremented each time the modal opens (false → true).
  // The {#key} block below remounts DeviceFormBody on every increment, ensuring
  // all internal $state is re-initialised from the current `target` prop.
  // ---------------------------------------------------------------------------
  let openInstanceCounter = $state(0);
  let _wasOpen = $state(false);

  $effect(() => {
    const isOpen = open;
    if (isOpen && !_wasOpen) {
      openInstanceCounter += 1;
      typeId = target?.type_id ?? DEVICE_TYPE_ID;
    }
    _wasOpen = isOpen;
  });

  // Footer button state — driven by DeviceFormBody callbacks.
  let formLoading = $state(false);
  let formCanSubmit = $state(false);

  // Registered submit function — DeviceFormBody calls onRegisterSubmit(handleSubmit)
  // from its onMount hook. Each {#key} remount provides a fresh function pointer.
  // The footer button calls this directly — no reactive trigger, no ordering race.
  let bodySubmitFn = $state<(() => void) | null>(null);

  // State hints loaded once on mount (non-fatal if fails).
  let stateHints = $state<string[]>([]);

  onMount(async () => {
    try {
      stateHints = await devices.stateHints();
    } catch {
      // Non-fatal — state chips won't appear but form still works.
    }
  });
</script>

<Modal {open} title={modalTitle} size="md" {onClose}>
  {#key openInstanceCounter}
    <DeviceFormBody
      {target}
      {stateHints}
      {typeId}
      {onSaved}
      onLoading={(l) => (formLoading = l)}
      onCanSubmitChange={(can) => (formCanSubmit = can)}
      onRegisterSubmit={(fn) => (bodySubmitFn = fn)}
    />
  {/key}

  {#snippet titleExtra()}
    <ActionMenu label="Тип устройства" variant="ghost-sm">
      <button type="button" role="menuitem" onclick={() => (typeId = DEVICE_TYPE_ID)}>
        <span class="type-menu-row">
          <span>Устройство</span>
          {#if typeId === DEVICE_TYPE_ID}
            <span class="type-menu-check" aria-hidden="true">✓</span>
          {/if}
        </span>
      </button>
      <button type="button" role="menuitem" onclick={() => (typeId = PRINTER_TYPE_ID)}>
        <span class="type-menu-row">
          <span>Принтер</span>
          {#if typeId === PRINTER_TYPE_ID}
            <span class="type-menu-check" aria-hidden="true">✓</span>
          {/if}
        </span>
      </button>
    </ActionMenu>
  {/snippet}

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

<style lang="scss">
  .type-menu-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    gap: var(--tr-space-xs);
  }
  .type-menu-check {
    color: var(--tr-accent);
    font-weight: var(--tr-font-weight-semibold);
  }
</style>

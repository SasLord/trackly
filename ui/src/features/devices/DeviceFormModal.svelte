<script lang="ts">
  // DeviceFormModal — outer shell: Modal + footer buttons + form-instance lifecycle.
  //
  // Form state lives entirely inside DeviceFormBody.svelte.
  // {#key openInstanceCounter} remounts DeviceFormBody on every open, guaranteeing
  // all form fields reset to their initial values — no stale serial/inv data
  // carries over between create sessions (Regression 6 fix).
  import { onMount } from 'svelte';
  import Modal from '$lib/components/Modal.svelte';
  import Button from '$lib/components/Button.svelte';
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

  const isEdit = $derived(target !== null);
  const modalTitle = $derived(isEdit ? 'Редактирование устройства' : 'Новое устройство');
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
      // CRITICAL: reset submitTrigger to 0 on every modal open.
      //
      // submitTrigger lives OUTSIDE the {#key openInstanceCounter} block, so it
      // persists across remounts of DeviceFormBody. After the first successful
      // submit, submitTrigger is 1. When the modal reopens and DeviceFormBody
      // mounts fresh, its $effect( submitTrigger > 0 → handleSubmit() ) fires
      // immediately on mount — submitting the form before the user has entered
      // anything. Resetting to 0 here prevents that spurious call.
      submitTrigger = 0;
    }
    _wasOpen = isOpen;
  });

  // Footer button state — driven by DeviceFormBody callbacks.
  let formLoading = $state(false);
  let formCanSubmit = $state(false);

  // Submit trigger — incrementing this causes DeviceFormBody to call handleSubmit().
  // Reset to 0 each time the modal opens (see $effect above) to avoid spurious
  // submits when DeviceFormBody remounts and sees a stale trigger value.
  let submitTrigger = $state(0);

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
      {onSaved}
      onLoading={(l) => (formLoading = l)}
      onCanSubmitChange={(can) => (formCanSubmit = can)}
      {submitTrigger}
    />
  {/key}

  {#snippet footer()}
    <Button variant="secondary" onclick={onClose}>Отмена</Button>
    <Button
      variant="primary"
      loading={formLoading}
      disabled={!formCanSubmit}
      onclick={() => (submitTrigger += 1)}
    >
      {#if formLoading}Сохранение…{:else}{submitLabel}{/if}
    </Button>
  {/snippet}
</Modal>

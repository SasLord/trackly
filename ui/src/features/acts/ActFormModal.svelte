<script lang="ts">
  // Plan 03-02: outer shell for «Создать акт» modal.
  // Uses Modal size="xwide" (1000px). Footer button calls bodySubmitFn() directly,
  // same pattern as DeviceFormModal.
  import Modal from '$lib/components/Modal.svelte';
  import Button from '$lib/components/Button.svelte';
  import ActFormBody from './ActFormBody.svelte';
  import type { ActDto } from '../../bindings';

  interface Props {
    open: boolean;
    onClose: () => void;
    onSaved: (_act: ActDto) => void;
  }

  const { open, onClose, onSaved }: Props = $props();

  let openInstanceCounter = $state(0);
  let _wasOpen = $state(false);

  $effect(() => {
    const isOpen = open;
    if (isOpen && !_wasOpen) {
      openInstanceCounter += 1;
    }
    _wasOpen = isOpen;
  });

  let formLoading = $state(false);
  let formCanSubmit = $state(false);
  let bodySubmitFn = $state<(() => void) | null>(null);
</script>

<Modal {open} title="Новый акт" size="xwide" {onClose}>
  {#key openInstanceCounter}
    <ActFormBody
      {onSaved}
      onLoading={(l) => (formLoading = l)}
      onCanSubmitChange={(c) => (formCanSubmit = c)}
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
      {#if formLoading}Создание…{:else}Создать акт{/if}
    </Button>
  {/snippet}
</Modal>

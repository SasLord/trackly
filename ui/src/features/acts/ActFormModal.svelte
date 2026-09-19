<script lang="ts">
  // Plan 03-02: outer shell for «Создать акт» modal.
  // Uses Modal size="xwide" (1000px). Footer button calls bodySubmitFn() directly,
  // same pattern as DeviceFormModal.
  //
  // Phase 40.2 Plan 14 (NUM-06..12, D-01/D-05): the D-01 save-chain popups
  // («Номер занят»/«Не соответствует шаблону»/«Проверьте буквы в номере»)
  // are rendered HERE as top-level `Modal` siblings, not nested inside
  // ActFormBody — the outer create/edit `Modal`'s `.modal-backdrop` has
  // `backdrop-filter: blur(2px)` (a containing-block trap for
  // `position: fixed`), the exact same reasoning DeviceFormModal.svelte
  // documents for its own popups (Plan 13). ActFormBody OWNS the
  // orchestration state and reports it up via `onPopupChange`.
  import Modal from '$lib/components/Modal.svelte';
  import Button from '$lib/components/Button.svelte';
  import NumberTakenPopup from '$lib/components/NumberTakenPopup.svelte';
  import NumberMismatchPopup from '$lib/components/NumberMismatchPopup.svelte';
  import NumberScriptWarningPopup from '$lib/components/NumberScriptWarningPopup.svelte';
  import ActFormBody from './ActFormBody.svelte';
  import type { ActNumberPopupState } from './ActFormBody.svelte';
  import type { ActDto } from '../../bindings';

  interface Props {
    open: boolean;
    mode?: 'create' | 'edit';
    initialAct?: ActDto | null;
    onClose: () => void;
    onSaved: (_act: ActDto) => void;
  }

  const { open, mode = 'create', initialAct = null, onClose, onSaved }: Props = $props();

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

  // Phase 40.2 Plan 14 (D-01/D-05): the current D-01 save-chain popup (or
  // null) — owned/orchestrated by ActFormBody, rendered here as a
  // TOP-LEVEL sibling of the create/edit Modal (see file header comment).
  let numberPopup = $state<ActNumberPopupState>(null);
</script>

<Modal
  {open}
  title={mode === 'edit' ? `Редактировать акт №${initialAct?.number}` : 'Новый акт'}
  size="xwide"
  {onClose}
>
  {#key openInstanceCounter}
    <ActFormBody
      {mode}
      {initialAct}
      {onSaved}
      onLoading={(l) => (formLoading = l)}
      onCanSubmitChange={(c) => (formCanSubmit = c)}
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
      {#if mode === 'edit'}
        {#if formLoading}Сохранение…{:else}Сохранить{/if}
      {:else if formLoading}Создание…{:else}Создать акт{/if}
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
  <!-- Create-mode only (D-05 excludes edit — see ActFormBody.svelte,
       submitEdit() never opens this popup). -->
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
    doppelganger={numberPopup.doppelganger}
    onFix={numberPopup.onFix}
    onContinue={numberPopup.onContinue}
  />
{/if}

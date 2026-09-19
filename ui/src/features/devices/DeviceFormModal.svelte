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
  //
  // Quick 260820-rdj (UAT gap-closure round 1, defect 1): the Принтер→Устройство
  // downgrade confirmation used to render INLINE inside DeviceFormBody's form body
  // — that left the kebab menu, the outer footer (Отмена/Сохранить) AND the inline
  // confirm's own buttons all interactive at once, which is confusing and lets the
  // user re-toggle the type mid-confirmation. It is now a real NESTED <Modal> owned
  // by THIS component (which already knows `typeId` + `target`), rendered as a
  // top-level sibling after the edit Modal — NOT inside DeviceFormBody, whose
  // `.modal-body` ancestor has `backdrop-filter: blur(2px)` (a containing-block
  // trap for `position: fixed`) that would pin a nested backdrop to the wrong box.
  // DeviceFormBody goes back to being a "dumb" form with no knowledge of the
  // confirmation step.
  //
  // Phase 40.2 Plan 13 (NUM-06/07/08/09/10/11/12, D-01/D-05): the same
  // containing-block reasoning applies to the D-01 save-chain popups
  // («Номер занят»/«Не соответствует шаблону»/«Проверьте буквы в номере») —
  // DeviceFormBody OWNS their orchestration (occupied/mismatch/script-mix
  // chain, button behaviour) but reports it up via `onPopupChange`; THIS
  // component renders the actual `<NumberTakenPopup>`/`<NumberMismatchPopup>`/
  // `<NumberScriptWarningPopup>` as top-level siblings, same pattern as the
  // downgrade-confirm Modal above. Fix 40.2-13 (NUM-11): `NumberMismatchPopup`
  // added post-Plan-13 — devices/printers DO check template mismatch in
  // `create_single_with_number_check()` (create-mode only, D-05 excludes
  // edit), contrary to Plan 13's original assumption.
  import { onMount } from 'svelte';
  import Modal from '$lib/components/Modal.svelte';
  import Button from '$lib/components/Button.svelte';
  import ActionMenu from '$lib/components/ActionMenu.svelte';
  import NumberTakenPopup from '$lib/components/NumberTakenPopup.svelte';
  import NumberMismatchPopup from '$lib/components/NumberMismatchPopup.svelte';
  import NumberScriptWarningPopup from '$lib/components/NumberScriptWarningPopup.svelte';
  import DeviceFormBody from './DeviceFormBody.svelte';
  import type { DeviceNumberPopupState } from './DeviceFormBody.svelte';
  import { devices } from './api';
  import type { DeviceDto } from '../../bindings';

  interface Props {
    open: boolean;
    target: DeviceDto | null;
    onClose: () => void;
    /**
     * Called after a successful save. `result.typeId` is the FINAL type_id the
     * record was saved with — callers that need to react to a type conversion
     * (e.g. PrinterDetail/PrintersPage deciding whether the record is still a
     * printer) can read it; existing callers that ignore the argument (e.g.
     * DevicesPage's `onSaved={() => {...}}`) keep working unchanged.
     * `result.placeId` is the FINAL place_id the record was saved with
     * (WARNING-1, audit 2026-09-17, D-14/D-15) — added the same way as
     * `typeId` above: an object with an extra field is assignable wherever
     * the narrower type is expected, so existing callers with fewer-arg
     * handlers stay valid without changes.
     */
    onSaved: (result?: { typeId: number; placeId: number | null }) => void;
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

  // RDJ-05: Принтер→Устройство requires an explicit confirmation (loses
  // toner-reading history + active alerts) before the save actually happens.
  let confirmOpen = $state(false);
  const isDowngrade = $derived(
    isEdit && target?.type_id === PRINTER_TYPE_ID && typeId === DEVICE_TYPE_ID,
  );

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
      confirmOpen = false;
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

  // Phase 40.2 Plan 13 (D-01/D-05): the current D-01 save-chain popup (or
  // null) — owned/orchestrated by DeviceFormBody, but rendered here as a
  // TOP-LEVEL sibling of the edit Modal, mirroring RDJ-05's own
  // downgrade-confirm popup precedent below (see this file's header
  // comment): a `<Modal>` nested inside DeviceFormBody would land its own
  // `position: fixed` backdrop inside the edit Modal's `.modal-backdrop`
  // (which has `backdrop-filter: blur(2px)`, a containing-block trap for
  // `position: fixed`).
  let numberPopup = $state<DeviceNumberPopupState>(null);

  // State hints loaded once on mount (non-fatal if fails).
  let stateHints = $state<string[]>([]);

  onMount(async () => {
    try {
      stateHints = await devices.stateHints();
    } catch {
      // Non-fatal — state chips won't appear but form still works.
    }
  });

  // Footer «Сохранить» click: downgrade conversions open the nested confirm
  // modal instead of saving directly; every other save goes straight through.
  function handleSaveClick() {
    if (isDowngrade) {
      confirmOpen = true;
      return;
    }
    bodySubmitFn?.();
  }

  // Confirm modal «Да, сохранить»: close the confirm popup and trigger the
  // real save. «Отмена» just closes the confirm popup — the edit popup stays
  // open with the form state (and the already-selected type) untouched.
  function handleConfirmDowngrade() {
    confirmOpen = false;
    bodySubmitFn?.();
  }

  // Forwarded to DeviceFormBody instead of the raw `onSaved` prop so the final
  // type_id (owned here, not by the dumb form body) reaches the caller.
  // placeId is forwarded straight through from DeviceFormBody — it already
  // knows the final saved value, no extra request needed (WARNING-1, D-14/D-15).
  function handleBodySaved(placeId: number | null) {
    onSaved({ typeId, placeId });
  }
</script>

<Modal {open} title={modalTitle} size="md" {onClose}>
  {#key openInstanceCounter}
    <DeviceFormBody
      {target}
      {stateHints}
      {typeId}
      onSaved={handleBodySaved}
      onLoading={(l) => (formLoading = l)}
      onCanSubmitChange={(can) => (formCanSubmit = can)}
      onRegisterSubmit={(fn) => (bodySubmitFn = fn)}
      onPopupChange={(popup) => (numberPopup = popup)}
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
      onclick={handleSaveClick}
    >
      {#if formLoading}Сохранение…{:else}{submitLabel}{/if}
    </Button>
  {/snippet}
</Modal>

<Modal
  open={confirmOpen}
  title="Сменить тип на «Устройство»?"
  size="md"
  onClose={() => (confirmOpen = false)}
>
  <p>
    Тип устройства меняется с «Принтер» на «Устройство». История показаний тонера и активные
    оповещения по этому принтеру будут удалены безвозвратно.
  </p>
  {#snippet footer()}
    <Button variant="secondary" onclick={() => (confirmOpen = false)}>Отмена</Button>
    <Button variant="destructive" loading={formLoading} onclick={handleConfirmDowngrade}>
      Да, сохранить
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
  <!-- Fix 40.2-13 (NUM-11): create-mode only (D-05 excludes edit — see
       DeviceFormBody.svelte, submitEdit() never opens this popup). -->
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

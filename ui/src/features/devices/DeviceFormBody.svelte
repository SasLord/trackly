<script module lang="ts">
  // Phase 40.2 Plan 13 (NUM-06/07/08/09/10/11/12): shared shape describing
  // which of the D-01 save-chain popups DeviceFormModal.svelte should render
  // right now (or `null` — none). Declared in a real `<script module>` block
  // (not the instance `<script>`, unlike NumberTakenPopup.svelte's own
  // `export interface`, which Plan 11 left untested for cross-file type
  // import) so `import type { DeviceNumberPopupState } from
  // './DeviceFormBody.svelte'` is guaranteed to resolve for svelte-check —
  // module-level exports are real ES module exports, instance-script
  // exports are not.
  //
  // Ownership split (RDJ-05 precedent, see DeviceFormModal.svelte's own
  // header comment): DeviceFormBody OWNS the orchestration logic (when to
  // open which popup, what each button does) but does NOT render the popups
  // itself — nesting a `<Modal>` inside this component would put its
  // `position: fixed` backdrop inside the OUTER edit-Modal's own
  // `.modal-backdrop` (which has `backdrop-filter: blur(2px)`, a
  // containing-block trap for `position: fixed`, exactly the reason RDJ-05's
  // downgrade-confirm popup was pulled OUT of this component and rendered as
  // a top-level sibling in DeviceFormModal.svelte instead). This component
  // reports popup state up via `onPopupChange`; DeviceFormModal renders the
  // real `<NumberTakenPopup>`/`<NumberScriptWarningPopup>` at its own
  // top level.
  import type { NumberTakenRecordSummary } from '$lib/components/NumberTakenPopup.svelte';

  export interface DeviceScriptWarningDoppelganger {
    number: string;
    record: { kind: string; title: string };
  }

  export type DeviceNumberPopupState =
    | {
        kind: 'taken';
        number: string;
        record: NumberTakenRecordSummary;
        canTakeNext: boolean;
        loadingTakeNext: boolean;
        onTakeNext?: () => void;
        onClose: () => void;
      }
    | {
        // Fix 40.2-13 (NUM-11): create-mode ONLY (D-05 — edit never has an
        // active template, so this variant never fires from submitEdit).
        kind: 'mismatch';
        number: string;
        mask: string;
        contextLabel: string;
        onFix: () => void;
        onContinue: () => void;
      }
    | {
        kind: 'scriptWarning';
        number: string;
        doppelganger: DeviceScriptWarningDoppelganger | null;
        onFix: () => void;
        onContinue: () => void;
      }
    | null;
</script>

<script lang="ts">
  // DeviceFormBody — the inner form component for DeviceFormModal.
  //
  // This component is intentionally separate from DeviceFormModal so that
  // {#key openInstanceCounter} in the parent forces a full remount on every
  // modal open. This guarantees all $state variables (name, inventoryNo,
  // serialNo, etc.) are reset to their initial values on each opening —
  // no stale form data carries over between create/edit sessions.
  //
  // Round 8: submitTrigger side-channel eliminated. The parent now binds to
  // the `submit` prop (exposed via $bindable) and calls it directly from the
  // footer button. No reactive trigger, no ordering race.
  //
  // Quick 260820-rdj (UAT gap-closure round 1, defect 1): this component is
  // intentionally a "dumb" form again — the Принтер→Устройство downgrade
  // confirmation decision now lives in DeviceFormModal (a nested Modal), not
  // here. This body just saves whatever `typeId` it was given; it never
  // second-guesses the parent.
  //
  // GAP-8 (39-UAT.md, Прогон 3): `readonly` — read-only mode for
  // PlaceEntityViewModal.svelte's «Просмотр устройства/принтера» popup.
  // Every field's own `disabled` prop is threaded from this single flag
  // (never a second, forked, non-interactive markup copy — see the gap's
  // "reuse, don't fork" instruction). Two defense-in-depth guards on top of
  // "no submit button is rendered by the caller": `canSubmit` is forced
  // false and `handleSubmit` early-returns, so even a stray Enter-key path
  // could never persist a change from a component that is supposed to be
  // strictly a mirror of the current record.

  import { onMount } from 'svelte';
  import Input from '$lib/components/Input.svelte';
  import Select from '$lib/components/Select.svelte';
  import Checkbox from '$lib/components/Checkbox.svelte';
  import NumberTemplateField from '$lib/components/NumberTemplateField.svelte';
  import DeviceAutocompleteField from './DeviceAutocompleteField.svelte';
  import PlacePicker from '$lib/components/PlacePicker.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import {
    confirmsFor,
    occupyingRecordOrRethrow,
    withConfirm,
    type PendingConfirm,
  } from '$lib/numbering/saveChain';
  import { apiCall } from '$lib/api/client';
  import { authStore } from '$lib/stores/auth.svelte';
  import { devices } from './api';
  import type {
    DeviceDto,
    DeviceNew,
    DevicePatch,
    DeviceSaveOutcome,
    NumberFieldInput,
    NumberWarningDto,
    OccupyingRecordDto,
    TemplateContextDto,
  } from '../../bindings';

  // Seed status ids (see STATUSES below / device_service.rs::resolve_status_id):
  // 1 = На складе, 2 = В работе, 3 = На ремонте, 4 = Списано.
  const STORAGE_STATUS_ID = 1;

  const STATUSES = [
    { id: 1, label: 'На складе' },
    { id: 2, label: 'В работе' },
    { id: 3, label: 'На ремонте' },
    { id: 4, label: 'Списано' },
  ];

  interface Props {
    target: DeviceDto | null;
    stateHints: string[];
    /** Выбранный тип устройства (1=Устройство, 2=Принтер) — управляется
     *  ActionMenu в заголовке DeviceFormModal, не этим компонентом. */
    typeId: number;
    /** GAP-8: renders every field disabled and blocks submit — see the
     *  file-header comment above. Defaults to false so every existing
     *  caller (DeviceFormModal) is unaffected. */
    readonly?: boolean;
    /** WARNING-1 (audit 2026-09-17, D-14/D-15): forwards the just-saved
     *  device's `place_id` to the caller so it can invalidate the «Места»
     *  tree's content counters for the affected place(s) without an extra
     *  request — this form already holds the final value in `placeId`
     *  above (just persisted via `devices.update`/`devices.bulkCreate`). */
    onSaved: (_placeId: number | null) => void;
    /** Expose submit-button state to parent's footer snippet. */
    onLoading: (_loading: boolean) => void;
    onCanSubmitChange: (_can: boolean) => void;
    /**
     * Called once on mount with a reference to handleSubmit.
     * The parent stores this function and calls it from the footer button.
     * Because {#key openInstanceCounter} remounts the body on each modal open,
     * a fresh function is provided each time — no stale closures, no side-channel races.
     */
    onRegisterSubmit: (_fn: () => void) => void;
    /** Phase 40.2 Plan 13 (D-01/D-05): reports which D-01 save-chain popup
     *  DeviceFormModal.svelte should render right now (`null` — none). See
     *  the `<script module>` doc-comment above for why the popups themselves
     *  are NOT rendered from inside this component. Optional with a no-op
     *  default so PlaceEntityViewModal.svelte's `readonly` view instance
     *  (which never submits, hence never opens a popup) doesn't need to pass
     *  it. */
    onPopupChange?: (_popup: DeviceNumberPopupState) => void;
  }

  const {
    target,
    stateHints,
    typeId,
    readonly = false,
    onSaved,
    onLoading,
    onCanSubmitChange,
    onRegisterSubmit,
    onPopupChange = () => {},
  }: Props = $props();

  const PRINTER_TYPE_ID = 2;

  // ---------------------------------------------------------------------------
  // Form state — all initialised from target (edit) or empty (create).
  // Because this component is re-mounted via {#key} on every modal open,
  // these are always fresh: no stale closures, no missing resets.
  // ---------------------------------------------------------------------------
  let name = $state(target?.name ?? '');
  let placeId = $state<number | null>(target?.place_id ?? null);
  let statusId = $state(target ? String(target.status_id) : '');
  let inventoryNo = $state(target?.inventory_no ?? '');
  let serialNo = $state(target?.serial_no ?? '');
  let model = $state(target?.model ?? '');
  let specs = $state(target?.specs ?? '');
  let kit = $state(target?.kit ?? '');
  let stateField = $state(target?.state ?? '');
  let quantity = $state(1);
  let loading = $state(false);
  let submitting = $state(false);
  let fieldErrors = $state<Record<string, string>>({});
  // Local mutable copy of target.version so we can refresh it after a
  // successful update without requiring the parent to re-mount the form.
  let currentVersion = $state(target?.version ?? 1);

  // ---------------------------------------------------------------------------
  // Phase 40.2 Plan 13 — D-01/D-05 save-chain popup orchestration state.
  // Shared between the create branch (Task 2) and the edit branch (Task 3):
  // `activePopup`/`pendingConfirm` are ONE set of variables, not two — an
  // edit session never has a `selectedTemplateId` (D-05: no template concept
  // in edit), so it simply never gets set outside the create branch.
  // ---------------------------------------------------------------------------
  // Fix 40.2-13 (NUM-11): 'mismatch' added — create-mode ONLY (D-05 excludes
  // it from edit; `submitEdit()` never sets `activePopup = 'mismatch'`).
  let activePopup = $state<'taken' | 'mismatch' | 'scriptWarning' | null>(null);
  // FE-CR-01: a confirmation belongs to ONE number (see saveChain.ts) —
  // reset by every «Поправлю»/«Закрыть»/«Взять следующий свободный» and
  // ignored as soon as the number differs from the confirmed one.
  let pendingConfirm = $state<PendingConfirm>(null);
  // NUM-08: which template NumberTemplateField currently has selected — only
  // ever set by the create branch's NumberTemplateField instance (edit mode
  // renders a plain Input, never calls this).
  let selectedTemplateId = $state<number | null>(null);
  // Fix 40.2-13 (NUM-11): the mask of `selectedTemplateId`, needed to render
  // NumberMismatchPopup (which does not have access to NumberTemplateField's
  // internal template list) — kept in lockstep with `selectedTemplateId` via
  // the SAME `onSelectedTemplateChange` callback (see the field's own
  // extended two-argument signature, Fix 40.2-13).
  let selectedTemplateMask = $state<string | null>(null);
  let numberFieldRef: NumberTemplateField | null = $state(null);

  let takenNumber = $state('');
  let takenRecord = $state<NumberTakenRecordSummary | null>(null);
  let takenCanTakeNext = $state(false);
  let takeNextLoading = $state(false);

  let mismatchNumber = $state('');
  let mismatchMask = $state('');
  let mismatchContextLabel = $state('');

  let scriptWarningNumber = $state('');
  let scriptWarningDoppelganger = $state<DeviceScriptWarningDoppelganger | null>(null);

  // D-11.3: storage-place status suggestion. `storagePlaceIds` is fetched once
  // per form instance (this component is always freshly mounted per modal
  // open — {#key openInstanceCounter} in DeviceFormModal — so a plain onMount
  // fetch, no `open`-toggle re-fetch guard needed, unlike OperationModal.svelte
  // which reuses one mounted instance across opens). Reuses the same
  // `cartridge_storage_place_ids` Tauri/HTTP command cartridges already call —
  // the underlying query (`PlaceRepo::list_storage_place_ids`, D-11.4 ancestor
  // inheritance) is place-tree-derived and entity-agnostic despite the
  // command's cartridge-era name; it is `Action::ReadData`-gated, not
  // cartridge-specific, so any caller able to open this device form already
  // has permission to call it.
  let storagePlaceIds = $state<Set<number>>(new Set());
  // Default-checked (D-11.3: "включённый по умолчанию"); the user may uncheck
  // it (D-10: no forced status change once unchecked).
  let storageStatusSuggested = $state(true);

  const isEdit = $derived(target !== null);

  // Phase 40.2 Plan 13 (NUM-06/08): which of the 2 device/printer create
  // contexts NumberTemplateField/the D-01 save chain uses — reactive to
  // `typeId` so switching «Устройство»/«Принтер» via DeviceFormModal's
  // titleExtra ActionMenu (while THIS instance stays mounted, no {#key}
  // remount) immediately re-targets the field's remembered template
  // (NumberTemplateField's own `$effect` already reacts to a `context` prop
  // change — see plan 12's Discretion #8).
  const numberContext = $derived<TemplateContextDto>(
    typeId === PRINTER_TYPE_ID ? 'printer_create' : 'device_create',
  );

  const quantityDisabled = $derived(isEdit || inventoryNo.trim() !== '' || serialNo.trim() !== '');

  // canSubmit: all required fields filled AND no in-flight request.
  // submitting guards against double-submit even before loading propagates.
  const canSubmit = $derived(
    !readonly && name.trim() !== '' && placeId !== null && statusId !== '' && !submitting,
  );

  // Reset quantity to 1 when inv/serial become non-empty.
  $effect(() => {
    if (inventoryNo.trim() !== '' || serialNo.trim() !== '') {
      quantity = 1;
    }
  });

  // D-11.3: the selected place (including D-11.4 ancestor inheritance,
  // already resolved server-side into the flat `storagePlaceIds` set) is a
  // storage place.
  const isStoragePlace = $derived(placeId !== null && storagePlaceIds.has(placeId));

  // D-11.3: while a storage place is selected AND the suggestion checkbox is
  // checked, the device status is (re-)set to «На складе» — this has a real
  // payload effect (unlike the cartridge form, cartridges have no
  // status-override field; devices do, via DevicePatch.status_id/
  // DeviceNew.status_id). Unchecking stops the auto-apply; the Статус
  // dropdown above is then fully manual again — no forced change (D-10).
  // GAP-8: skipped entirely in readonly mode — a «Просмотр» popup must
  // mirror the record's ACTUAL saved status, never a suggestion that would
  // never actually be applied (nothing here ever submits).
  $effect(() => {
    if (!readonly && isStoragePlace && storageStatusSuggested) {
      statusId = String(STORAGE_STATUS_ID);
    }
  });

  // Propagate canSubmit to parent for footer button state.
  $effect(() => {
    onCanSubmitChange(canSubmit);
  });

  // Propagate loading to parent. Phase 40.2 Plan 13 (UI-SPEC §6): "Пока идёт
  // цепочка, существующая основная кнопка попапа создания... в состоянии
  // loading" — an open D-01 popup counts as "chain in progress" even during
  // the gap between two network requests (waiting on the user's
  // Продолжить/Поправлю/Закрыть decision), so `activePopup !== null` is
  // OR'd in here rather than only reflecting the narrower in-flight-request
  // `loading` flag.
  $effect(() => {
    onLoading(loading || activePopup !== null);
  });

  // Phase 40.2 Plan 13 (D-01/D-05): propagate the current save-chain popup
  // (or null) to the parent — see the `<script module>` doc-comment at the
  // top of this file for why DeviceFormModal renders the popup, not this
  // component. Runs whenever any of the underlying $state pieces change
  // (including `takeNextLoading`, so clicking «Взять следующий свободный»
  // re-renders the SAME open popup with its button in a loading state).
  $effect(() => {
    if (activePopup === 'taken' && takenRecord) {
      onPopupChange({
        kind: 'taken',
        number: takenNumber,
        record: takenRecord,
        canTakeNext: takenCanTakeNext,
        loadingTakeNext: takeNextLoading,
        onTakeNext: takenCanTakeNext ? handleTakeNext : undefined,
        onClose: closeTakenPopup,
      });
    } else if (activePopup === 'mismatch') {
      onPopupChange({
        kind: 'mismatch',
        number: mismatchNumber,
        mask: mismatchMask,
        contextLabel: mismatchContextLabel,
        onFix: closeMismatchPopup,
        onContinue: continueMismatch,
      });
    } else if (activePopup === 'scriptWarning') {
      onPopupChange({
        kind: 'scriptWarning',
        number: scriptWarningNumber,
        doppelganger: scriptWarningDoppelganger,
        onFix: closeScriptWarningPopup,
        onContinue: continueScriptWarning,
      });
    } else {
      onPopupChange(null);
    }
    // On teardown (modal closed / {#key} remount) make sure the parent never
    // keeps a stale popup open for a component instance that no longer
    // exists.
    return () => onPopupChange(null);
  });

  // Phase 40.2 Plan 13 (UI-SPEC §6): «Номер уже занят.» stays under the
  // field only «до следующей правки» — clear it the moment the value
  // actually changes (manual edit, template pick, or «Взять следующий
  // свободный»), regardless of which of those caused the change.
  let previousInventoryNo = inventoryNo;
  $effect(() => {
    if (inventoryNo !== previousInventoryNo) {
      previousInventoryNo = inventoryNo;
      if (fieldErrors['inventory_no']) {
        const { inventory_no: _removed, ...rest } = fieldErrors;
        fieldErrors = rest;
      }
    }
  });

  // Register handleSubmit with the parent once on mount.
  // onMount is used (not $effect) to guarantee exactly one call per component
  // instance — when the parent remounts via {#key}, a fresh instance calls
  // onRegisterSubmit with the new closure.
  onMount(() => {
    onRegisterSubmit(handleSubmit);

    let cancelled = false;
    apiCall<number[]>('cartridge_storage_place_ids', {})
      .then((ids) => {
        if (cancelled) return;
        storagePlaceIds = new Set(ids);
      })
      .catch(() => {
        if (cancelled) return;
        // Fail-safe: a failed lookup just hides the suggestion checkbox —
        // never blocks saving the device itself.
        storagePlaceIds = new Set();
      });
    return () => {
      cancelled = true;
    };
  });

  // ---------------------------------------------------------------------------
  // Phase 40.2 Plan 13 (D-01/D-05) — save-chain popup helpers.
  // ---------------------------------------------------------------------------

  // FE-CR-02: only called with a record that `occupyingRecordOrRethrow`
  // (saveChain.ts) confirmed as really occupying the number — a CONFLICT
  // without an occupying record is some other conflict and goes to the
  // generic error handler with the server's own message.
  function toTakenRecordSummary(record: OccupyingRecordDto): NumberTakenRecordSummary {
    return {
      kind: record.kind as NumberTakenRecordSummary['kind'],
      title: record.title,
      subtitle: record.subtitle,
      place: record.place,
      status: record.status,
    };
  }

  function focusNumberField() {
    if (isEdit) {
      document.getElementById('f-inv')?.focus();
    } else {
      numberFieldRef?.focus();
    }
  }

  function showTakenPopup(record: OccupyingRecordDto, canTakeNextFlag: boolean) {
    takenNumber = inventoryNo;
    takenRecord = toTakenRecordSummary(record);
    takenCanTakeNext = canTakeNextFlag;
    takeNextLoading = false;
    activePopup = 'taken';
  }

  function closeTakenPopup() {
    activePopup = null;
    pendingConfirm = null;
    fieldErrors = { ...fieldErrors, inventory_no: 'Номер уже занят.' };
    focusNumberField();
  }

  async function handleTakeNext() {
    // D-05: edit sessions always pass `canTakeNext=false`, so the button
    // (and therefore this handler) never renders/fires from an edit popup.
    // FE-WR-07: the number comes from NumberTemplateField itself, so the
    // field tracks it as its own suggestion (↑/↓ arrow, D-14 silent refresh).
    if (selectedTemplateId === null || numberFieldRef === null) return;
    takeNextLoading = true;
    try {
      const result = await numberFieldRef.takeNext();
      if (result === 'error') {
        pushToast(
          'error',
          'Не удалось получить следующий номер. Введите номер вручную или попробуйте ещё раз.',
        );
        return;
      }
      if (result === 'unavailable') return;
      // 'overflowed': the field is cleared and shows «Шаблон … переполнен»
      // under itself — back to the form so the user can pick/type another.
      activePopup = null;
      pendingConfirm = null;
      focusNumberField();
    } finally {
      takeNextLoading = false;
    }
  }

  // Fix 40.2-13 (NUM-11): "Не соответствует шаблону" — create-mode only
  // (D-05 excludes edit entirely, `submitEdit()` never calls this). `mask`
  // comes from `selectedTemplateMask` (the field's OWN reactive callback,
  // NOT re-derived here — this component has no access to
  // NumberTemplateField's internal template list, per UI-SPEC/D-01: "client
  // renders `message` verbatim" is the SERVER's contract for the toast/log
  // text, but this popup's own props are number/mask/contextLabel per
  // NumberMismatchPopup.svelte's Plan 11 interface).
  function openMismatchPopup() {
    mismatchNumber = inventoryNo;
    mismatchMask = selectedTemplateMask ?? '';
    mismatchContextLabel = typeId === PRINTER_TYPE_ID ? 'Новый принтер' : 'Новое устройство';
    activePopup = 'mismatch';
  }

  function closeMismatchPopup() {
    activePopup = null;
    pendingConfirm = null;
    focusNumberField();
  }

  function continueMismatch() {
    pendingConfirm = withConfirm(pendingConfirm, inventoryNo, 'mismatch');
    activePopup = null;
    void handleSubmit();
  }

  function openScriptWarningPopup(warning: NumberWarningDto) {
    scriptWarningNumber = inventoryNo;
    // Homoglyph doppelganger: the double LOOKS identical to the candidate
    // (that's the definition of a script-mix collision) — the server's
    // `NumberWarningDto.doppelganger` (`OccupyingRecordDto`) intentionally
    // carries no separate "number" field of its own (see
    // `dto/number_template.rs`'s doc-comment), so the displayed
    // "номер-двойник" text is the SAME candidate string the user typed.
    scriptWarningDoppelganger = warning.doppelganger
      ? {
          number: inventoryNo,
          record: { kind: warning.doppelganger.kind, title: warning.doppelganger.title },
        }
      : null;
    activePopup = 'scriptWarning';
  }

  function closeScriptWarningPopup() {
    activePopup = null;
    pendingConfirm = null;
    focusNumberField();
  }

  function continueScriptWarning() {
    pendingConfirm = withConfirm(pendingConfirm, inventoryNo, 'scriptMix');
    activePopup = null;
    void handleSubmit();
  }

  // ---------------------------------------------------------------------------
  // Submit
  // ---------------------------------------------------------------------------

  async function submitSingleCreate() {
    const confirms = confirmsFor(pendingConfirm, inventoryNo);
    const newDevice: DeviceNew = {
      type_id: typeId,
      name: name.trim(),
      inventory_no: null, // resolved server-side from `numberInput` below, not this field
      serial_no: serialNo.trim() || null,
      model: model.trim() || null,
      specs: specs.trim() || null,
      kit: kit.trim() || null,
      state: stateField.trim() || null,
      place_id: placeId,
      status_id: parseInt(statusId, 10),
    };
    const numberInput: NumberFieldInput = {
      value: inventoryNo,
      templateId: selectedTemplateId,
      // Fix 40.2-13 (NUM-11): `create_single_with_number_check()` DOES check
      // template mismatch for devices/printers (D-01 applies to every entity
      // family whenever a template is active in the field — see
      // `device_service.rs`'s corrected doc-comment) — `confirmMismatch` now
      // actually carries the D-01 chain's mismatch confirmation, same as
      // `confirmScriptMix` below.
      confirmMismatch: confirms.mismatch,
      confirmScriptMix: confirms.scriptMix,
    };

    let outcome: DeviceSaveOutcome;
    try {
      outcome = await devices.createSingleWithNumberCheck(newDevice, numberInput, numberContext);
    } catch (e) {
      // FE-CR-02: rethrows anything that is not «number really occupied».
      const record = await occupyingRecordOrRethrow(e, numberContext, inventoryNo, null);
      showTakenPopup(record, selectedTemplateId !== null);
      return;
    }

    if (outcome.outcome === 'created') {
      pendingConfirm = null;
      pushToast('success', typeId === PRINTER_TYPE_ID ? 'Принтер создан' : 'Устройство создано');
      onSaved(placeId);
      return;
    }
    if (outcome.kind === 'mismatch') {
      openMismatchPopup();
      return;
    }
    if (outcome.kind === 'script_mix') {
      openScriptWarningPopup(outcome);
      return;
    }
    pushToast('error', 'Не удалось сохранить');
  }

  async function submitEdit(currentTarget: DeviceDto) {
    const patch: DevicePatch = {
      type_id: typeId,
      name: name.trim() || null,
      number_input: {
        value: inventoryNo.trim(),
        confirm_script_mix: confirmsFor(pendingConfirm, inventoryNo).scriptMix,
      },
      serial_no: serialNo.trim() || null,
      model: model.trim() || null,
      specs: specs.trim() || null,
      kit: kit.trim() || null,
      state: stateField.trim() || null,
      place_id: placeId,
      status_id: parseInt(statusId, 10) || null,
    };

    let outcome: DeviceSaveOutcome;
    try {
      outcome = await devices.update(currentTarget.id, currentVersion, patch);
    } catch (e) {
      const record = await occupyingRecordOrRethrow(
        e,
        numberContext,
        inventoryNo,
        currentTarget.id,
      );
      showTakenPopup(record, false);
      return;
    }

    if (outcome.outcome === 'created') {
      pendingConfirm = null;
      // Refresh the local version counter so a subsequent edit in the same
      // modal session uses the correct (incremented) version.
      currentVersion = outcome.version;
      pushToast('success', 'Устройство сохранено');
      onSaved(placeId);
      return;
    }
    if (outcome.kind === 'script_mix') {
      openScriptWarningPopup(outcome);
      return;
    }
    // D-05: `update()` never calls `check_mismatch` at all — if this ever
    // fires, it's a server bug. Defensive per this plan's own <action> text:
    // generic toast, never `NumberMismatchPopup` (no `contextLabel`/mask
    // makes sense for an edit session).
    pushToast('error', 'Не удалось сохранить');
  }

  async function handleSubmit() {
    // GAP-8 defense-in-depth: readonly instances render no submit button and
    // canSubmit is already forced false above, but this guard makes the
    // no-persist guarantee true even if handleSubmit were ever invoked some
    // other way (e.g. a future caller wiring onRegisterSubmit by mistake).
    if (readonly) return;
    if (!canSubmit) return;
    // In-flight guard: prevent double-submit from rapid clicks.
    if (submitting) return;
    submitting = true;
    loading = true;
    fieldErrors = {};

    try {
      if (isEdit && target) {
        await submitEdit(target);
      } else {
        const qty = quantityDisabled ? 1 : Math.max(1, Math.min(100, quantity || 1));
        if (qty === 1) {
          await submitSingleCreate();
        } else {
          const newDevice: DeviceNew = {
            type_id: typeId,
            name: name.trim(),
            inventory_no: inventoryNo.trim() || null,
            serial_no: serialNo.trim() || null,
            model: model.trim() || null,
            specs: specs.trim() || null,
            kit: kit.trim() || null,
            state: stateField.trim() || null,
            place_id: placeId,
            status_id: parseInt(statusId, 10),
          };
          await devices.bulkCreate(newDevice, qty);
          pushToast('success', `Создано ${qty} устройств`);
          onSaved(placeId);
        }
      }
    } catch (e: unknown) {
      if (e && typeof e === 'object') {
        const err = e as { code?: string; message?: string; details?: { field?: string } };
        if (err.code === 'VALIDATION' && err.details?.field) {
          fieldErrors = { ...fieldErrors, [err.details.field]: err.message ?? 'Ошибка' };
          pushToast('error', err.message ?? 'Ошибка валидации');
        } else if (err.code === 'OPTIMISTIC_LOCK_MISMATCH') {
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
      submitting = false;
    }
  }
</script>

<form
  class="device-form"
  onsubmit={(e) => {
    // Prevent default HTML form submission in all cases.
    // Submission is always triggered via the bound submit function from
    // DeviceFormModal's footer button — never via Enter key or implicit form submit.
    e.preventDefault();
    e.stopPropagation();
  }}
>
  <!-- 1. Required: Наименование (with autocomplete) -->
  <div class="field" class:has-error={!!fieldErrors['name']}>
    <label class="label" for="f-name">
      Наименование <span class="required" aria-hidden="true">*</span>
    </label>
    <DeviceAutocompleteField
      field="name"
      value={name}
      placeholder="Ноутбук Lenovo ThinkPad X1"
      id="f-name"
      invalid={!!fieldErrors['name']}
      disabled={readonly}
      onChange={(v) => (name = v)}
    />
    {#if fieldErrors['name']}
      <p class="field-error">{fieldErrors['name']}</p>
    {/if}
  </div>

  <!-- 2–4. Инвентарный № / Серийный № / Количество — один горизонтальный ряд.
       Количество ВСЕГДА отображается, но disabled когда inv/serial заполнен или это edit-режим.
       Это исключает «дёрганье» макета при вводе номеров. -->
  <div class="field-row">
    <div class="field field-row-item" class:number-field-item={!isEdit}>
      <label class="label" for="f-inv">Инвентарный №</label>
      {#if isEdit}
        <Input
          id="f-inv"
          value={inventoryNo}
          placeholder="ИНВ-000001"
          disabled={readonly}
          oninput={(v) => (inventoryNo = v)}
        />
      {:else}
        <NumberTemplateField
          bind:this={numberFieldRef}
          context={numberContext}
          bind:value={inventoryNo}
          placeholder="ИНВ-000001"
          disabled={readonly || quantity > 1}
          invalid={!!fieldErrors['inventory_no']}
          errorMessage={fieldErrors['inventory_no'] ?? null}
          canManageSettings={authStore.user?.role === 'admin'}
          onSelectedTemplateChange={(id, mask) => {
            selectedTemplateId = id;
            selectedTemplateMask = mask;
          }}
        />
      {/if}
    </div>
    <div class="field field-row-item">
      <label class="label" for="f-serial">Серийный №</label>
      <Input
        id="f-serial"
        value={serialNo}
        placeholder="SN-XXXXXXXX"
        disabled={readonly}
        oninput={(v) => (serialNo = v)}
      />
    </div>
    <div class="field field-row-item">
      <label class="label" for="f-qty">Количество</label>
      <input
        id="f-qty"
        type="number"
        class="input"
        class:input-disabled={quantityDisabled || readonly}
        min={1}
        max={100}
        value={quantityDisabled ? 1 : quantity}
        disabled={quantityDisabled || readonly}
        oninput={(e) => {
          if (!quantityDisabled && !readonly) {
            const v = parseInt((e.currentTarget as HTMLInputElement).value, 10);
            quantity = isNaN(v) ? 1 : Math.max(1, Math.min(100, v));
          }
        }}
      />
    </div>
  </div>

  <!-- 5. Optional: Модель (with autocomplete, contextual) -->
  <div class="field">
    <label class="label" for="f-model">Модель</label>
    <DeviceAutocompleteField
      field="model"
      value={model}
      placeholder="ThinkPad X1 Carbon Gen 12"
      id="f-model"
      contextName={name.trim() || undefined}
      disabled={readonly}
      onChange={(v) => (model = v)}
    />
  </div>

  <!-- 6. Optional: Технические характеристики (specs) — multiline textarea -->
  <div class="field">
    <label class="label" for="f-specs">Технические характеристики</label>
    <DeviceAutocompleteField
      field="specs"
      value={specs}
      placeholder="i7-1365U, 16 ГБ RAM, 512 ГБ SSD"
      id="f-specs"
      multiline={true}
      contextName={name.trim() || undefined}
      disabled={readonly}
      onChange={(v) => (specs = v)}
    />
  </div>

  <!-- 7. Optional: Комплектация (kit) -->
  <div class="field">
    <label class="label" for="f-kit">Комплектация</label>
    <DeviceAutocompleteField
      field="kit"
      value={kit}
      placeholder="Зарядное устройство, мышь"
      id="f-kit"
      contextName={name.trim() || undefined}
      disabled={readonly}
      onChange={(v) => (kit = v)}
    />
  </div>

  <!-- 8. Required: Статус -->
  <div class="field" class:has-error={!!fieldErrors['status_id']}>
    <label class="label" for="f-status">
      Статус <span class="required" aria-hidden="true">*</span>
    </label>
    <Select
      id="f-status"
      value={statusId}
      invalid={!!fieldErrors['status_id']}
      disabled={readonly}
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

  <!-- 9. Required: Место (PlacePicker — единственный контрол выбора места, D-17) -->
  <div class="field" class:has-error={!!fieldErrors['place_id']}>
    <label class="label" for="f-place">
      Место <span class="required" aria-hidden="true">*</span>
    </label>
    <PlacePicker
      value={placeId}
      onChange={(id) => (placeId = id)}
      id="f-place"
      invalid={!!fieldErrors['place_id']}
      disabled={readonly}
    />
    {#if fieldErrors['place_id']}
      <p class="field-error">{fieldErrors['place_id']}</p>
    {/if}
    {#if isStoragePlace && !readonly}
      <Checkbox checked={storageStatusSuggested} onchange={(c) => (storageStatusSuggested = c)}>
        Перевести устройство в статус «На складе»
      </Checkbox>
    {/if}
  </div>

  <!-- 10. Optional: Состояние + state-hints chips (with autocomplete) — ПОСЛЕДНЕЕ -->
  <div class="field">
    <label class="label" for="f-state">Состояние</label>
    <DeviceAutocompleteField
      field="state"
      value={stateField}
      placeholder="Хорошее"
      id="f-state"
      contextName={name.trim() || undefined}
      disabled={readonly}
      onChange={(v) => (stateField = v)}
    />
    {#if stateHints.length > 0 && !readonly}
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

<style lang="scss">
  .device-form {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-md);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-2xs);
  }

  // Horizontal row for Инв.№ / Серийный № / Количество.
  // Все три поля всегда присутствуют — макет не «прыгает».
  .field-row {
    display: flex;
    gap: var(--tr-space-md);
    align-items: flex-start;
  }

  .field-row-item {
    flex: 1 1 0;
    min-width: 0; // prevents flex child from overflowing
  }

  // UI-SPEC Discretion #12: the Инвентарный № slot gets more room than
  // Серийный №/Количество in CREATE mode only (the 36px «Вставка» button +
  // ↑/↓ arrow would otherwise squeeze the Input) — edit mode's plain Input
  // keeps the row's original equal-thirds layout untouched.
  .number-field-item {
    flex: 2 1 0;
  }

  .label {
    font-size: var(--tr-font-size-label);
    font-weight: var(--tr-font-weight-medium);
    color: var(--tr-text-primary);
  }

  .required {
    color: var(--tr-danger);
    margin-left: 2px;
  }

  .field-error {
    margin: 0;
    font-size: var(--tr-font-size-label);
    color: var(--tr-danger);
  }

  .input {
    display: block;
    width: 100%;
    height: 36px;
    padding: 0 var(--tr-space-md);
    background: var(--tr-bg);
    color: var(--tr-text-primary);
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-xs);
    font-family: var(--tr-font-family);
    font-size: var(--tr-font-size-body);
    line-height: var(--tr-line-height-body);

    &::placeholder {
      color: var(--tr-text-tertiary);
    }

    &:focus-visible {
      outline: none;
      border-color: var(--tr-accent);
      box-shadow: 0 0 0 3px var(--tr-focus-ring);
    }

    &:disabled,
    &.input-disabled {
      background: var(--tr-surface-sunken);
      color: var(--tr-text-tertiary);
      cursor: not-allowed;
    }
  }

  .state-hints {
    margin-top: var(--tr-space-2xs);
  }

  .state-hints-label {
    display: block;
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-secondary);
    margin-bottom: var(--tr-space-2xs);
  }

  .state-hints-chips {
    display: flex;
    flex-wrap: wrap;
    gap: var(--tr-space-2xs);
  }

  .hint-chip {
    padding: 2px var(--tr-space-xs);
    background: var(--tr-surface-sunken);
    border: 1px solid var(--tr-border);
    border-radius: 12px;
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-secondary);
    cursor: pointer;
    font-family: var(--tr-font-family);
    transition: none;

    &:hover {
      background: var(--tr-surface);
      color: var(--tr-text-primary);
      border-color: var(--tr-border-strong);
    }

    &.active {
      background: color-mix(in srgb, var(--tr-accent) 15%, transparent);
      border-color: var(--tr-accent);
      color: var(--tr-accent);
    }

    &:focus-visible {
      outline: none;
      box-shadow: 0 0 0 3px var(--tr-focus-ring);
    }
  }
</style>

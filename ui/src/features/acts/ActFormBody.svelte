<script module lang="ts">
  // Phase 40.2 Plan 14 (NUM-06/07/08/09/10/11/12): shared shape describing
  // which of the D-01 save-chain popups ActFormModal.svelte should render
  // right now (or `null` — none). Same ownership split as Plan 13's
  // `DeviceNumberPopupState` (see DeviceFormBody.svelte's own doc-comment):
  // ActFormBody OWNS the orchestration logic (when to open which popup,
  // what each button does) but does NOT render the popups itself — nesting
  // a `<Modal>` inside this component would put its `position: fixed`
  // backdrop inside the OUTER create/edit Modal's own `.modal-backdrop`
  // (which has `backdrop-filter: blur(2px)`, a containing-block trap for
  // `position: fixed`, confirmed identical to DeviceFormModal.svelte's
  // Modal.svelte usage). This component reports popup state up via
  // `onPopupChange`; ActFormModal renders the real
  // `<NumberTakenPopup>`/`<NumberMismatchPopup>`/`<NumberScriptWarningPopup>`
  // at its own top level.
  import type { NumberTakenRecordSummary } from '$lib/components/NumberTakenPopup.svelte';

  export interface ActScriptWarningDoppelganger {
    number: string;
    record: { kind: string; title: string };
  }

  export type ActNumberPopupState =
    | {
        kind: 'taken';
        number: string;
        record: NumberTakenRecordSummary;
        /** false — форма правки (D-05): «Взять следующий свободный» не
         *  показывается. */
        canTakeNext: boolean;
        loadingTakeNext: boolean;
        onTakeNext?: () => void;
        onClose: () => void;
      }
    | {
        // Create-mode ONLY (D-05 — edit never has an active template, so
        // this variant never fires from submitEdit()).
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
        doppelganger: ActScriptWarningDoppelganger | null;
        onFix: () => void;
        onContinue: () => void;
      }
    | null;
</script>

<script lang="ts">
  // Plan 03-02: form body for the create-handover modal.
  // Pattern follows DeviceFormBody — exposes onRegisterSubmit / onLoading /
  // onCanSubmitChange to parent ActFormModal, parent's footer button calls
  // bodySubmitFn() directly.
  //
  // Phase 40.2 Plan 14 (NUM-06..12, D-01/D-05): create-mode «№» is now
  // `NumberTemplateField` + the full occupied→mismatch→script-mix chain
  // (mirrors DeviceFormBody's Plan 13 orchestration exactly, same
  // `activePopup`/`pendingConfirm` state machine); edit mode keeps a plain
  // `Input` (D-05 — no templates in edit) but gets occupied+script-mix on
  // submit (never mismatch — `ActNumberEditInput` has no `template_id`
  // field at all, a structural guarantee from Plan 06).
  import { onMount } from 'svelte';
  import Input from '$lib/components/Input.svelte';
  import NumberTemplateField from '$lib/components/NumberTemplateField.svelte';
  import PersonAutocomplete from '$lib/components/PersonAutocomplete.svelte';
  import DatePicker from '$lib/components/DatePicker.svelte';
  import PlacePicker from '$lib/components/PlacePicker.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { apiCall } from '$lib/api/client';
  import { authStore } from '$lib/stores/auth.svelte';
  import { acts } from './api';
  import ActFormItemsTable from './ActFormItemsTable.svelte';
  import type { FormItemRow } from './ActFormItemsTable.svelte';
  import type {
    ActCreateDto,
    ActDto,
    ActSaveOutcome,
    ActUpdateDto,
    NextNumberDto,
    NumberWarningDto,
    OccupyingRecordDto,
    TemplateContextDto,
  } from '../../bindings';

  interface Props {
    mode?: 'create' | 'edit';
    initialAct?: ActDto | null;
    onSaved: (_act: ActDto) => void;
    onLoading: (_l: boolean) => void;
    onCanSubmitChange: (_c: boolean) => void;
    onRegisterSubmit: (_fn: () => void) => void;
    /** Phase 40.2 Plan 14 (D-01/D-05): reports which D-01 save-chain popup
     *  ActFormModal.svelte should render right now (`null` — none). See the
     *  `<script module>` doc-comment above for why the popups themselves are
     *  NOT rendered from inside this component. */
    onPopupChange?: (_popup: ActNumberPopupState) => void;
  }

  const {
    mode = 'create',
    initialAct = null,
    onSaved,
    onLoading,
    onCanSubmitChange,
    onRegisterSubmit,
    onPopupChange = () => {},
  }: Props = $props();

  // Phase 40.2 Plan 14 (NUM-06): acts have exactly one create context — no
  // device/printer-style type toggle.
  const numberContext: TemplateContextDto = 'act_create';

  // ----------------------------------------------------------------------------
  // State
  // ----------------------------------------------------------------------------
  // G-2 (Phase 3.1 Plan 04): дата фактической передачи (когда отдали).
  // Default = today UTC. Plan 19-08 (IN-01): UTC accessors match
  // unixToIso()/isoToUnix() below — a single TZ convention across the
  // create-default, edit-prefill and round-trip paths, no day-boundary
  // off-by-one against browser-local calendar accessors.
  function todayISO(): string {
    const d = new Date();
    const y = d.getUTCFullYear();
    const m = String(d.getUTCMonth() + 1).padStart(2, '0');
    const day = String(d.getUTCDate()).padStart(2, '0');
    return `${y}-${m}-${day}`;
  }

  // Plan 19-05 (ACT-02): unix seconds (UTC midnight) -> YYYY-MM-DD, the inverse
  // of isoToUnix below. Used only to prefill DatePicker inputs in edit mode.
  function unixToIso(unixSeconds: number | null | undefined): string {
    if (unixSeconds === null || unixSeconds === undefined) return '';
    const d = new Date(unixSeconds * 1000);
    const y = d.getUTCFullYear();
    const m = String(d.getUTCMonth() + 1).padStart(2, '0');
    const day = String(d.getUTCDate()).padStart(2, '0');
    return `${y}-${m}-${day}`;
  }

  const isEditPrefill = mode === 'edit' && initialAct !== null;

  /** Plan 19-05: prefilled from initialAct.items directly (bypassing the
   *  live на_складе search path) — existing positions are в_работе, not
   *  на_складе, so a live re-search would never find them (RESEARCH.md
   *  "wrinkle"). New rows added during this edit session still go through
   *  the normal on-warehouse picker unchanged. */
  function itemsFromInitialAct(act: ActDto): FormItemRow[] {
    return act.items.map((it) => ({
      device_id: it.device_id,
      quantity: 1,
      device_label: it.device_name,
      query: '',
      picked: true,
      group_ids: [],
      complectation_at_time: it.complectation_at_time,
    }));
  }

  // Phase 40.2 Plan 14 (NUM-13/NUM-14): number is a free TEXT value — create
  // mode starts empty (NumberTemplateField autofills the remembered
  // template's next number on mount, NUM-08); edit mode prefills the act's
  // current raw number.
  let numberValue = $state(isEditPrefill ? initialAct!.number_raw : '');
  let giverName = $state(isEditPrefill ? initialAct!.giver_name : '');
  let receiverName = $state(isEditPrefill ? initialAct!.receiver_name : '');
  let placeId = $state<number | null>(isEditPrefill ? (initialAct!.place_id ?? null) : null);
  let deadlineISO = $state(isEditPrefill ? unixToIso(initialAct!.deadline_utc) : ''); // YYYY-MM-DD picker value
  let handoverDateISO = $state(
    isEditPrefill ? unixToIso(initialAct!.handover_date_utc) : todayISO(),
  );
  let notes = $state(isEditPrefill ? (initialAct!.notes ?? '') : '');
  let items = $state<FormItemRow[]>(
    isEditPrefill
      ? itemsFromInitialAct(initialAct!)
      : [{ device_id: null, quantity: 1, device_label: '', query: '', picked: false }],
  );

  let loading = $state(false);
  let submitting = $state(false);
  let fieldErrors = $state<Record<string, string>>({});

  // ---------------------------------------------------------------------------
  // Phase 40.2 Plan 14 — D-01/D-05 save-chain popup orchestration state.
  // Identical structure to DeviceFormBody.svelte's Plan 13 state machine —
  // one shared shape across all three *FormBody components (devices/acts/
  // cartridges), per that plan's own "recommended for future plans" note.
  // ---------------------------------------------------------------------------
  let activePopup = $state<'taken' | 'mismatch' | 'scriptWarning' | null>(null);
  let pendingConfirm = $state<{ mismatch: boolean; scriptMix: boolean }>({
    mismatch: false,
    scriptMix: false,
  });
  // NUM-08: which template NumberTemplateField currently has selected — only
  // ever set by the create branch's NumberTemplateField instance (edit mode
  // renders a plain Input, never calls this).
  let selectedTemplateId = $state<number | null>(null);
  let selectedTemplateMask = $state<string | null>(null);
  let numberFieldRef: NumberTemplateField | null = $state(null);

  let takenNumber = $state('');
  let takenRecord = $state<NumberTakenRecordSummary | null>(null);
  let takenCanTakeNext = $state(false);
  let takeNextLoading = $state(false);

  let mismatchNumber = $state('');
  let mismatchMask = $state('');
  const MISMATCH_CONTEXT_LABEL = 'Новый акт';

  let scriptWarningNumber = $state('');
  let scriptWarningDoppelganger = $state<ActScriptWarningDoppelganger | null>(null);

  const validItemCount = $derived(
    items.filter((it) => it.device_id !== null && it.quantity >= 1).length,
  );

  const canSubmit = $derived(
    giverName.trim() !== '' && receiverName.trim() !== '' && validItemCount >= 1 && !submitting,
  );

  $effect(() => {
    onCanSubmitChange(canSubmit);
  });
  // UI-SPEC §6: while a D-01 popup is open (waiting on the user's
  // Продолжить/Поправлю/Закрыть decision), the footer button stays in a
  // loading-looking state — mirrors DeviceFormBody's identical OR.
  $effect(() => {
    onLoading(loading || activePopup !== null);
  });

  // Phase 40.2 Plan 14 (D-01/D-05): propagate the current save-chain popup
  // (or null) to the parent — see the `<script module>` doc-comment at the
  // top of this file for why ActFormModal renders the popup, not this
  // component.
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
        contextLabel: MISMATCH_CONTEXT_LABEL,
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

  // «Номер уже занят.» stays under the field only «до следующей правки» —
  // clear it the moment the value actually changes (manual edit, template
  // pick, or «Взять следующий свободный»).
  let previousNumberValue = numberValue;
  $effect(() => {
    if (numberValue !== previousNumberValue) {
      previousNumberValue = numberValue;
      if (fieldErrors['number']) {
        const { number: _removed, ...rest } = fieldErrors;
        fieldErrors = rest;
      }
    }
  });

  onMount(() => {
    onRegisterSubmit(handleSubmit);
  });

  function isoToUnix(iso: string): number | null {
    if (!iso) return null;
    const t = Date.parse(iso + 'T00:00:00Z');
    return Number.isFinite(t) ? Math.floor(t / 1000) : null;
  }

  // ---------------------------------------------------------------------------
  // Phase 40.2 Plan 14 (D-01/D-05) — save-chain popup helpers.
  // Mirrors DeviceFormBody.svelte's Plan 13 helpers 1:1.
  // ---------------------------------------------------------------------------

  // `AppError.code()` serialises as SCREAMING_SNAKE_CASE (see
  // `crates/trackly-core/src/error.rs::code()`) — 'CONFLICT', not 'Conflict'.
  function isConflictError(e: unknown): boolean {
    return !!e && typeof e === 'object' && (e as { code?: string }).code === 'CONFLICT';
  }

  // acts.number_input's number space (TemplateType::ActNumber) is occupied
  // ONLY by acts — unlike devices/printers sharing one space, no fallback
  // kind ambiguity here.
  function toTakenRecordSummary(record: OccupyingRecordDto | null): NumberTakenRecordSummary {
    if (record) {
      return {
        kind: record.kind as NumberTakenRecordSummary['kind'],
        title: record.title,
        subtitle: record.subtitle,
        place: record.place,
        status: record.status,
      };
    }
    // Best-effort fallback (race where the occupying record is deleted
    // between the failed save and this follow-up `is_occupied` call).
    return { kind: 'act', title: '—', subtitle: null, place: null, status: null };
  }

  function focusNumberField() {
    if (mode === 'edit') {
      document.getElementById('act-number')?.focus();
    } else {
      numberFieldRef?.focus();
    }
  }

  async function openTakenPopup(excludeId: number | null, canTakeNextFlag: boolean) {
    const candidate = numberValue.trim();
    let record: OccupyingRecordDto | null;
    try {
      record = await apiCall<OccupyingRecordDto | null>('number_templates_is_occupied', {
        context: numberContext,
        candidate,
        excludeId,
      });
    } catch {
      record = null;
    }
    takenNumber = numberValue;
    takenRecord = toTakenRecordSummary(record);
    takenCanTakeNext = canTakeNextFlag;
    takeNextLoading = false;
    activePopup = 'taken';
  }

  function closeTakenPopup() {
    activePopup = null;
    fieldErrors = { ...fieldErrors, number: 'Номер уже занят.' };
    focusNumberField();
  }

  async function handleTakeNext() {
    // D-05: edit sessions always pass `canTakeNext=false`, so the button
    // (and therefore this handler) never renders/fires from an edit popup.
    if (selectedTemplateId === null) return;
    takeNextLoading = true;
    try {
      const dto = await apiCall<NextNumberDto>('number_templates_peek_next', {
        templateId: selectedTemplateId,
        context: numberContext,
      });
      numberValue = dto.rendered;
      activePopup = null;
      focusNumberField();
    } catch {
      pushToast(
        'error',
        'Не удалось получить следующий номер. Введите номер вручную или попробуйте ещё раз.',
      );
    } finally {
      takeNextLoading = false;
    }
  }

  // Create-mode only (D-05 excludes edit entirely — `submitEdit()` never
  // calls this). `mask` comes from `selectedTemplateMask` (the field's own
  // reactive callback) — this component has no access to
  // NumberTemplateField's internal template list.
  function openMismatchPopup() {
    mismatchNumber = numberValue;
    mismatchMask = selectedTemplateMask ?? '';
    activePopup = 'mismatch';
  }

  function closeMismatchPopup() {
    activePopup = null;
    focusNumberField();
  }

  function continueMismatch() {
    pendingConfirm = { ...pendingConfirm, mismatch: true };
    activePopup = null;
    void handleSubmit();
  }

  function openScriptWarningPopup(warning: NumberWarningDto) {
    scriptWarningNumber = numberValue;
    // Homoglyph doppelganger: the double LOOKS identical to the candidate
    // (that's the definition of a script-mix collision) — the server's
    // `NumberWarningDto.doppelganger` intentionally carries no separate
    // "number" field of its own, so the displayed "номер-двойник" text is
    // the SAME candidate string the user typed (same adaptation as
    // DeviceFormBody.svelte, Plan 13).
    scriptWarningDoppelganger = warning.doppelganger
      ? {
          number: numberValue,
          record: { kind: warning.doppelganger.kind, title: warning.doppelganger.title },
        }
      : null;
    activePopup = 'scriptWarning';
  }

  function closeScriptWarningPopup() {
    activePopup = null;
    focusNumberField();
  }

  function continueScriptWarning() {
    pendingConfirm = { ...pendingConfirm, scriptMix: true };
    activePopup = null;
    void handleSubmit();
  }

  // ----------------------------------------------------------------------------
  // Submit
  // ----------------------------------------------------------------------------

  async function submitCreate() {
    // Build payload — drop any incomplete item rows.
    // UAT Fix #3/#4: device_ids[] = первые `quantity` штук из group_ids
    // (если выбрана группа) — backend использует именно эти devices без
    // клонирования; legacy fallback (group_ids пуст) — старый clone path.
    const payloadItems = items
      .filter((it) => it.device_id !== null && it.quantity >= 1)
      .map((it) => {
        const groupIds = it.group_ids ?? [];
        const deviceIds = groupIds.length > 0 ? groupIds.slice(0, it.quantity) : [];
        return {
          device_id: it.device_id as number,
          device_ids: deviceIds,
          quantity: it.quantity,
        };
      });

    const payload: ActCreateDto = {
      number_input: {
        value: numberValue.trim(),
        templateId: selectedTemplateId,
        confirmMismatch: pendingConfirm.mismatch,
        confirmScriptMix: pendingConfirm.scriptMix,
      },
      giver_name: giverName.trim(),
      receiver_name: receiverName.trim(),
      place_id: placeId,
      notes: notes.trim() || null,
      deadline_utc: isoToUnix(deadlineISO),
      handover_date_utc: isoToUnix(handoverDateISO),
      items: payloadItems,
    };

    let outcome: ActSaveOutcome;
    try {
      outcome = await acts.create(payload);
    } catch (e) {
      if (isConflictError(e)) {
        await openTakenPopup(null, selectedTemplateId !== null);
        return;
      }
      throw e;
    }

    if (outcome.outcome === 'created') {
      pendingConfirm = { mismatch: false, scriptMix: false };
      pushToast('success', `Создан акт №${outcome.number}`);
      onSaved(outcome);
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
    pushToast('error', 'Не удалось создать акт');
  }

  async function submitEdit(target: ActDto) {
    // Plan 19-05 (ACT-02): full-replacement items set — device_id +
    // complectation_at_time travel over the wire (retained/removed/added
    // is diffed server-side). GT2 (260715-gt2): a RETAINED row
    // (complectation_at_time !== undefined) still emits exactly one
    // entry, unchanged. A FRESHLY-ADDED row (complectation_at_time
    // === undefined) with quantity > 1 now expands via group_ids —
    // mirrors the create-branch expansion above (groupIds.slice(0,
    // it.quantity)) — falling back to [device_id] if group_ids is
    // empty/absent (defensive, mirrors serialised single-instance picks).
    // ActUpdateItemDto itself is unchanged (still one device_id +
    // complectation_at_time per entry); multi-qty travels as N entries.
    const updateItems = items
      .filter((it) => it.device_id !== null)
      .flatMap((it) => {
        if (it.complectation_at_time !== undefined) {
          return [
            {
              device_id: it.device_id as number,
              complectation_at_time: it.complectation_at_time ?? null,
            },
          ];
        }
        const groupIds = it.group_ids ?? [];
        const deviceIds =
          groupIds.length > 0 ? groupIds.slice(0, it.quantity) : [it.device_id as number];
        return deviceIds.map((deviceId) => ({
          device_id: deviceId,
          complectation_at_time: null,
        }));
      });

    const updatePayload: ActUpdateDto = {
      id: target.id,
      expected_version: target.version,
      number_input: { value: numberValue.trim(), confirm_script_mix: pendingConfirm.scriptMix },
      giver_name: giverName.trim(),
      receiver_name: receiverName.trim(),
      place_id: placeId,
      notes: notes.trim() || null,
      deadline_utc: isoToUnix(deadlineISO),
      handover_date_utc: isoToUnix(handoverDateISO),
      items: updateItems,
    };

    let outcome: ActSaveOutcome;
    try {
      outcome = await acts.update(updatePayload);
    } catch (e) {
      if (isConflictError(e)) {
        await openTakenPopup(target.id, false);
        return;
      }
      throw e;
    }

    if (outcome.outcome === 'created') {
      pendingConfirm = { mismatch: false, scriptMix: false };
      pushToast('success', `Акт №${outcome.number} обновлён`);
      onSaved(outcome);
      return;
    }
    if (outcome.kind === 'script_mix') {
      openScriptWarningPopup(outcome);
      return;
    }
    // D-05: `update()` never calls `check_mismatch` at all — if this ever
    // fires, it's a server bug. Defensive: generic toast, never
    // `NumberMismatchPopup` (no template concept makes sense for an edit
    // session).
    pushToast('error', 'Не удалось сохранить акт');
  }

  async function handleSubmit() {
    if (!canSubmit || submitting) return;
    submitting = true;
    loading = true;
    fieldErrors = {};

    try {
      if (mode === 'edit' && initialAct) {
        await submitEdit(initialAct);
      } else {
        await submitCreate();
      }
    } catch (e: unknown) {
      if (e && typeof e === 'object') {
        const err = e as {
          code?: string;
          message?: string;
          details?: { field?: string; reason?: string };
        };
        if (err.code === 'Validation' && err.details?.field) {
          fieldErrors = { ...fieldErrors, [err.details.field]: err.message ?? 'Ошибка' };
          pushToast('error', err.message ?? 'Проверьте поля формы');
        } else if (err.code === 'OptimisticLockMismatch') {
          pushToast(
            'error',
            'Акт был изменён другим пользователем — обновите страницу и попробуйте снова.',
          );
        } else {
          pushToast(
            'error',
            err.message ??
              (mode === 'edit' ? 'Не удалось сохранить акт' : 'Не удалось создать акт'),
          );
        }
      } else {
        pushToast('error', mode === 'edit' ? 'Не удалось сохранить акт' : 'Не удалось создать акт');
      }
    } finally {
      loading = false;
      submitting = false;
    }
  }
</script>

<form
  class="act-form"
  onsubmit={(e) => {
    e.preventDefault();
    e.stopPropagation();
  }}
>
  <h3 class="section-heading">Шапка акта</h3>

  <!-- Row 1: №, Когда отдали, Сроком до (3 колонки) -->
  <div class="grid-3">
    <div class="field" class:has-error={!!fieldErrors['number']}>
      <label class="label" for="act-number">№ <span class="req">*</span></label>
      {#if mode === 'edit'}
        <Input
          id="act-number"
          type="text"
          value={numberValue}
          invalid={!!fieldErrors['number']}
          oninput={(v) => (numberValue = v)}
        />
      {:else}
        <NumberTemplateField
          bind:this={numberFieldRef}
          context={numberContext}
          bind:value={numberValue}
          invalid={!!fieldErrors['number']}
          errorMessage={fieldErrors['number'] ?? null}
          canManageSettings={authStore.user?.role === 'admin'}
          onSelectedTemplateChange={(id, mask) => {
            selectedTemplateId = id;
            selectedTemplateMask = mask;
          }}
        />
      {/if}
      {#if mode === 'edit' && fieldErrors['number']}
        <p class="error">{fieldErrors['number']}</p>
      {/if}
    </div>

    <div class="field">
      <label class="label" for="act-handover-date">Когда отдали <span class="req">*</span></label>
      <DatePicker id="act-handover-date" bind:value={handoverDateISO} required />
    </div>

    <div class="field">
      <label class="label" for="act-deadline">Сроком до</label>
      <DatePicker id="act-deadline" bind:value={deadlineISO} />
    </div>
  </div>

  <!-- Row 2: Сдал, Принял (2 колонки) -->
  <div class="grid-2">
    <div class="field" class:has-error={!!fieldErrors['giver_name']}>
      <label class="label" for="act-giver">Сдал <span class="req">*</span></label>
      <PersonAutocomplete
        id="act-giver"
        field="giver"
        bind:value={giverName}
        placeholder="Иванов Иван Иванович"
        invalid={!!fieldErrors['giver_name']}
      />
      {#if fieldErrors['giver_name']}<p class="error">{fieldErrors['giver_name']}</p>{/if}
    </div>

    <div class="field" class:has-error={!!fieldErrors['receiver_name']}>
      <label class="label" for="act-receiver">Принял <span class="req">*</span></label>
      <PersonAutocomplete
        id="act-receiver"
        field="receiver"
        bind:value={receiverName}
        placeholder="Петров Пётр Петрович"
        invalid={!!fieldErrors['receiver_name']}
      />
      {#if fieldErrors['receiver_name']}<p class="error">{fieldErrors['receiver_name']}</p>{/if}
    </div>
  </div>

  <!-- Row 3: Место, Заметки (2 колонки) -->
  <div class="grid-2">
    <div class="field">
      <label class="label" for="act-place">Место</label>
      <PlacePicker value={placeId} id="act-place" onChange={(id) => (placeId = id)} />
    </div>

    <div class="field">
      <label class="label" for="act-notes">Заметки</label>
      <Input
        id="act-notes"
        type="text"
        value={notes}
        placeholder="Необязательно"
        oninput={(v) => (notes = v)}
      />
    </div>
  </div>

  <h3 class="section-heading">Позиции</h3>
  <ActFormItemsTable {items} {fieldErrors} {mode} onChange={(next) => (items = next)} />
</form>

<style lang="scss">
  .act-form {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-xl);
  }
  .section-heading {
    margin: 0;
    font-size: var(--tr-font-size-body);
    font-weight: var(--tr-font-weight-semibold);
    color: var(--tr-text-primary);
  }
  .grid-2 {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--tr-space-md);
  }
  .grid-3 {
    display: grid;
    grid-template-columns: 1fr 1fr 1fr;
    gap: var(--tr-space-md);
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-2xs);
  }
  .label {
    font-size: var(--tr-font-size-label);
    font-weight: 500;
    color: var(--tr-text-secondary);
  }
  .hint {
    margin: 0;
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-tertiary);
  }
  .req {
    color: var(--tr-danger);
    margin-left: 2px;
  }
  .error {
    margin: 0;
    font-size: var(--tr-font-size-label);
    color: var(--tr-danger);
  }

  @media (max-width: 720px) {
    .grid-2,
    .grid-3 {
      grid-template-columns: 1fr;
    }
  }
</style>

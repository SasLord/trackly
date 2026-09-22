<script module lang="ts">
  // Phase 40.2 Plan 15 (NUM-06/07/08/09/10/11/12): shared shape describing
  // which of the D-01 save-chain popups CartridgeFormModal.svelte should
  // render right now (or `null` — none). Same ownership split as Plan 13's
  // `DeviceNumberPopupState`/Plan 14's `ActNumberPopupState` (see
  // DeviceFormBody.svelte's own doc-comment): CartridgeFormBody OWNS the
  // orchestration logic (when to open which popup, what each button does)
  // but does NOT render the popups itself — nesting a `<Modal>` inside this
  // component would put its `position: fixed` backdrop inside the OUTER
  // create/edit Modal's own `.modal-backdrop` (which has `backdrop-filter:
  // blur(2px)`, a containing-block trap for `position: fixed`, confirmed
  // identical to DeviceFormModal.svelte/ActFormModal.svelte's `Modal.svelte`
  // usage). This component reports popup state up via `onPopupChange`;
  // CartridgeFormModal renders the real
  // `<NumberTakenPopup>`/`<NumberMismatchPopup>`/`<NumberScriptWarningPopup>`
  // at its own top level.
  import type { NumberTakenRecordSummary } from '$lib/components/NumberTakenPopup.svelte';

  export interface CartridgeScriptWarningDoppelganger {
    number: string;
    record: { kind: string; title: string };
  }

  export type CartridgeNumberPopupState =
    | {
        kind: 'taken';
        number: string;
        record: NumberTakenRecordSummary;
        /** false — форма правки (D-05, out of scope for this plan — код не
         *  редактируется): «Взять следующий свободный» не показывается. */
        canTakeNext: boolean;
        loadingTakeNext: boolean;
        onTakeNext?: () => void;
        onClose: () => void;
      }
    | {
        // Create-mode ONLY — код при редактировании не меняется через эту
        // форму (SPEC boundaries), поэтому mismatch не может сработать вне
        // создания.
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
        doppelganger: CartridgeScriptWarningDoppelganger | null;
        onFix: () => void;
        onContinue: () => void;
      }
    | null;
</script>

<script lang="ts">
  // Plan 04-05: CartridgeFormBody — inner form state component for CartridgeFormModal.
  // Remounted on every {#key openInstanceCounter} — guarantees field reset.
  // Поля (UI-SPEC §CartridgeFormModal): Код (обязателен при создании, NUM-13) +
  // Модель + Состояние заряда + Место (D-12) + Примечания.
  //
  // GAP-8 (39-UAT.md, Прогон 3): `readonly` — read-only mode for
  // PlaceEntityViewModal.svelte's «Просмотр картриджа» popup. Mirrors
  // DeviceFormBody.svelte's identical readonly contract: every field's own
  // `disabled` prop threaded from one flag, `canSubmit` forced false,
  // `handleSubmit` early-returns as a defense-in-depth guard.
  //
  // Phase 40.2 Plan 15 (NUM-06/07/08/09/10/11/12, D-01): create-mode «Код»
  // is now `NumberTemplateField` + the full occupied→mismatch→script-mix
  // chain (mirrors DeviceFormBody's Plan 13 / ActFormBody's Plan 14
  // orchestration exactly, same `activePopup`/`pendingConfirm` state
  // machine). `context` is reactive to `kindId` (Картридж→cartridge_create,
  // Фотобарабан→drum_create) — NumberTemplateField's OWN `$effect`
  // (`contextAlwaysReplaces()`, Plan 12) already replaces the value
  // UNCONDITIONALLY on a cartridge_create<->drum_create context switch (no
  // extra prop/`{#key}` needed here, see plan's own SUMMARY for the
  // verification of this). Edit mode is UNCHANGED — code stays a disabled
  // plain `Input`, no template concept in edit (SPEC boundaries, out of
  // scope for this plan).
  import { onMount } from 'svelte';
  import Input from '$lib/components/Input.svelte';
  // Plan 27-G1: Select (нативный <select>) заменён на кастомный Dropdown
  // (flat + variant="select") — открывающееся меню больше не нативное OS-меню.
  // Dropdown не принимает `id`/`for`, поэтому подпись оборачивает поле
  // (implicit label), а не связывается через `for` (как раньше у Select).
  import Dropdown from '$lib/components/Dropdown.svelte';
  import Textarea from '$lib/components/Textarea.svelte';
  import PlacePicker from '$lib/components/PlacePicker.svelte';
  import NumberTemplateField from '$lib/components/NumberTemplateField.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import {
    confirmsFor,
    occupyingRecordOrRethrow,
    withConfirm,
    type PendingConfirm,
  } from '$lib/numbering/saveChain';
  import { apiCall } from '$lib/api/client';
  import { authStore } from '$lib/stores/auth.svelte';
  import { cartridges } from './api';
  import type {
    CartridgeCreateDto,
    CartridgeDto,
    CartridgeModelDto,
    CartridgeSaveOutcome,
    NextNumberDto,
    NumberWarningDto,
    OccupyingRecordDto,
    TemplateContextDto,
  } from '../../bindings';

  interface Props {
    target: CartridgeDto | null;
    models: CartridgeModelDto[];
    /** GAP-8: renders every field disabled and blocks submit — see the
     *  file-header comment above. Defaults to false so the existing caller
     *  (CartridgeFormModal) is unaffected. */
    readonly?: boolean;
    onClose: () => void;
    onSuccess: (_cart: CartridgeDto) => void;
    onLoading: (_l: boolean) => void;
    onCanSubmitChange: (_can: boolean) => void;
    onRegisterSubmit: (_fn: () => void) => void;
    /** Phase 40.2 Plan 15 (D-01): reports which D-01 save-chain popup
     *  CartridgeFormModal.svelte should render right now (`null` — none).
     *  See the `<script module>` doc-comment above for why the popups
     *  themselves are NOT rendered from inside this component. Optional
     *  with a no-op default so PlaceEntityViewModal.svelte's `readonly` view
     *  instance (which never submits, hence never opens a popup) doesn't
     *  need to pass it. */
    onPopupChange?: (_popup: CartridgeNumberPopupState) => void;
  }

  const {
    target,
    models,
    readonly = false,
    onClose,
    onSuccess,
    onLoading,
    onCanSubmitChange,
    onRegisterSubmit,
    onPopupChange = () => {},
  }: Props = $props();

  const isEdit = $derived(target !== null);

  // Вид расходника: 1 = Картридж, 2 = Фотобарабан. При создании выбирается
  // пользователем (первое поле); при редактировании фиксирован моделью.
  let kindId = $state<number>(target?.model_kind_id ?? 1);

  // Состояния по виду: картридж → «Состояние заряда» (Полный/Частичный/Пустой);
  // фотобарабан → «Состояние» (Новый/Изношенный/Отработанный) (V017).
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
  const stateOptions = $derived(kindId === 2 ? DRUM_STATES : CARTRIDGE_STATES);
  const stateLabel = $derived(kindId === 2 ? 'Состояние' : 'Состояние заряда');
  const codePlaceholder = $derived(kindId === 2 ? 'D-XXXX' : 'C-XXXX');

  // Phase 40.2 Plan 15 (NUM-06/08): which of the 2 cartridge/drum create
  // contexts NumberTemplateField/the D-01 save chain uses — reactive to
  // `kindId` so switching «Картридж»/«Фотобарабан» immediately re-targets
  // the field's remembered template AND unconditionally replaces the value
  // (NumberTemplateField's own `contextAlwaysReplaces()`, Plan 12,
  // Discretion #8 — cartridge_create<->drum_create always replaces, unlike
  // device_create<->printer_create's conditional D-14 replace).
  const numberContext = $derived<TemplateContextDto>(
    kindId === 2 ? 'drum_create' : 'cartridge_create',
  );

  // Form fields — initialised from target (edit) or defaults (create)
  let code = $state(target?.code ?? '');
  let modelId = $state<number | null>(target?.model_id ?? null);
  let stateId = $state<number>(target?.state_id ?? (target?.model_kind_id === 2 ? 4 : 1));
  // Plan 16 (D-12): картридж — своё place_id, как у устройства.
  let placeId = $state<number | null>(target?.place_id ?? null);
  let notes = $state(target?.notes ?? '');

  // Validation errors
  let codeError = $state('');
  let modelError = $state('');
  let loading = $state(false);
  let submitting = $state(false);

  // ---------------------------------------------------------------------------
  // Phase 40.2 Plan 15 — D-01 save-chain popup orchestration state (create
  // only — edit never touches the code field, SPEC boundaries). Identical
  // structure to DeviceFormBody.svelte's Plan 13 / ActFormBody.svelte's
  // Plan 14 state machine.
  // ---------------------------------------------------------------------------
  let activePopup = $state<'taken' | 'mismatch' | 'scriptWarning' | null>(null);
  // FE-CR-01: a confirmation belongs to ONE number (see saveChain.ts) —
  // reset by every «Поправлю»/«Закрыть»/«Взять следующий свободный» and
  // ignored as soon as the number differs from the confirmed one.
  let pendingConfirm = $state<PendingConfirm>(null);
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
  let mismatchContextLabel = $state('');

  let scriptWarningNumber = $state('');
  let scriptWarningDoppelganger = $state<CartridgeScriptWarningDoppelganger | null>(null);

  // Модели, соответствующие выбранному виду.
  const visibleModels = $derived(models.filter((m) => m.kind_id === kindId));

  // Plan 27-G1: опции для Dropdown (flat + variant="select") — «Что добавляем».
  const KIND_OPTIONS = [
    { id: 1, label: 'Картридж' },
    { id: 2, label: 'Фотобарабан' },
  ];
  const kindLabel = $derived(KIND_OPTIONS.find((o) => o.id === kindId)?.label ?? '');

  const modelOptions = $derived(
    visibleModels.map((m) => ({ id: m.id, label: `${m.brand} ${m.model}` })),
  );
  const selectedModelLabel = $derived(modelOptions.find((o) => o.id === modelId)?.label ?? '');

  const selectedStateLabel = $derived(stateOptions.find((o) => o.value === stateId)?.label ?? '');

  // Плоские опции без drill-in — onExpandGroup никогда реально не вызывается
  // (isGroupExpandable всегда false), но Dropdown требует типизированную
  // функцию, чтобы вывести TMember (иначе `() => []` выводит `never[]`).
  function noExpandKind(): { id: number; label: string }[] {
    return [];
  }
  function noExpandModel(): { id: number; label: string }[] {
    return [];
  }
  function noExpandState(): { value: number; label: string }[] {
    return [];
  }

  function handleKindChange(v: string) {
    const k = parseInt(v, 10);
    kindId = k;
    // Сброс модели, если она не соответствует новому виду.
    if (modelId !== null) {
      const m = models.find((x) => x.id === modelId);
      if (!m || m.kind_id !== k) modelId = null;
    }
    // Состояние по умолчанию для нового вида.
    stateId = k === 2 ? 4 : 1;
    modelError = '';
    // Код: НЕ трогаем здесь — `numberContext` (реактивно к `kindId`) уже
    // передаётся в `NumberTemplateField`, чей собственный `$effect`
    // безусловно подставляет первое свободное число нового вида
    // (`contextAlwaysReplaces('cartridge_create'|'drum_create')` === true,
    // Plan 12). Дублировать эту подстановку здесь было бы вторым источником
    // истины для одного и того же значения.
  }

  // canSubmit: Модель обязательна
  const canSubmit = $derived(!readonly && !submitting && modelId !== null);

  // Sync canSubmit upward
  $effect(() => {
    onCanSubmitChange(canSubmit);
  });

  // UI-SPEC §6: while a D-01 popup is open (waiting on the user's
  // Продолжить/Поправлю/Закрыть decision), the footer button stays in a
  // loading-looking state — mirrors DeviceFormBody/ActFormBody's identical OR.
  $effect(() => {
    onLoading(loading || activePopup !== null);
  });

  // Phase 40.2 Plan 15 (D-01): propagate the current save-chain popup (or
  // null) to the parent — see the `<script module>` doc-comment at the top
  // of this file for why CartridgeFormModal renders the popup, not this
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

  // «Код уже занят.» stays under the field only «до следующей правки» —
  // clear it the moment the value actually changes (manual edit, template
  // pick, «Взять следующий свободный», or a kindId-driven context switch).
  let previousCode = code;
  $effect(() => {
    if (code !== previousCode) {
      previousCode = code;
      if (codeError) codeError = '';
    }
  });

  function validate(): boolean {
    let valid = true;
    codeError = '';
    modelError = '';

    if (modelId === null) {
      modelError = 'Выберите модель картриджа';
      valid = false;
    }

    // Phase 40.2 Plan 07 (NUM-13): server-side auto-generation is retired —
    // an empty code is now a hard validation error, not "auto-fill". This
    // mirrors that rule client-side (only on create — an existing
    // cartridge's code is not editable via this form) to avoid a round-trip
    // just to learn the same thing the server already knows.
    if (!isEdit && code.trim() === '') {
      codeError = 'Введите код или выберите шаблон.';
      valid = false;
    }

    return valid;
  }

  // ---------------------------------------------------------------------------
  // Phase 40.2 Plan 15 (D-01) — save-chain popup helpers. Mirrors
  // DeviceFormBody.svelte's Plan 13 / ActFormBody.svelte's Plan 14 helpers
  // 1:1, adapted to cartridges/drums' one-shared-code-space-two-contexts
  // shape (Plan 07).
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
      document.getElementById('cart-code')?.focus();
    } else {
      numberFieldRef?.focus();
    }
  }

  function showTakenPopup(record: OccupyingRecordDto, canTakeNextFlag: boolean) {
    takenNumber = code;
    takenRecord = toTakenRecordSummary(record);
    takenCanTakeNext = canTakeNextFlag;
    takeNextLoading = false;
    activePopup = 'taken';
  }

  function closeTakenPopup() {
    activePopup = null;
    pendingConfirm = null;
    codeError = 'Номер уже занят.';
    focusNumberField();
  }

  async function handleTakeNext() {
    // Edit sessions always pass `canTakeNext=false` (code isn't editable),
    // so the button (and therefore this handler) never renders/fires from
    // an edit popup.
    if (selectedTemplateId === null) return;
    takeNextLoading = true;
    try {
      const dto = await apiCall<NextNumberDto>('number_templates_peek_next', {
        templateId: selectedTemplateId,
        context: numberContext,
      });
      code = dto.rendered;
      activePopup = null;
      pendingConfirm = null;
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

  // Create-mode only (код не редактируется в правке — SPEC boundaries,
  // out of scope for this plan). `mask` comes from `selectedTemplateMask`
  // (the field's own reactive callback) — this component has no access to
  // NumberTemplateField's internal template list. `contextLabel` uses the
  // CURRENT `kindId` at the moment of the save attempt (UI-SPEC/plan's own
  // must_haves truth), not whatever it was when the template was last
  // selected.
  function openMismatchPopup() {
    mismatchNumber = code;
    mismatchMask = selectedTemplateMask ?? '';
    mismatchContextLabel = kindId === 2 ? 'Новый фотобарабан' : 'Новый картридж';
    activePopup = 'mismatch';
  }

  function closeMismatchPopup() {
    activePopup = null;
    pendingConfirm = null;
    focusNumberField();
  }

  function continueMismatch() {
    pendingConfirm = withConfirm(pendingConfirm, code, 'mismatch');
    activePopup = null;
    void handleSubmit();
  }

  function openScriptWarningPopup(warning: NumberWarningDto) {
    scriptWarningNumber = code;
    // Homoglyph doppelganger: the double LOOKS identical to the candidate
    // (that's the definition of a script-mix collision) — the server's
    // `NumberWarningDto.doppelganger` intentionally carries no separate
    // "number" field of its own, so the displayed "номер-двойник" text is
    // the SAME candidate string the user typed (same adaptation as
    // DeviceFormBody.svelte/ActFormBody.svelte, Plans 13/14).
    scriptWarningDoppelganger = warning.doppelganger
      ? {
          number: code,
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
    pendingConfirm = withConfirm(pendingConfirm, code, 'scriptMix');
    activePopup = null;
    void handleSubmit();
  }

  // ---------------------------------------------------------------------------
  // Submit
  // ---------------------------------------------------------------------------

  async function submitCreate() {
    const confirms = confirmsFor(pendingConfirm, code);
    // Phase 40.2 Plan 07 (NUM-13): `code_override` -> `number_input`.
    // `NumberFieldInput` (dto::number_template) has its OWN
    // `#[serde(rename_all = "camelCase")]` — camelCase here is intentional,
    // NOT a slip from the rest of this snake_case DTO.
    const payload: CartridgeCreateDto = {
      model_id: modelId!,
      number_input: {
        value: code.trim(),
        templateId: selectedTemplateId,
        confirmMismatch: confirms.mismatch,
        confirmScriptMix: confirms.scriptMix,
      },
      state_id: stateId,
      place_id: placeId,
      notes: notes.trim() || null,
    };

    let outcome: CartridgeSaveOutcome;
    try {
      outcome = await cartridges.create(payload);
    } catch (e) {
      // FE-CR-02: rethrows anything that is not «number really occupied».
      const record = await occupyingRecordOrRethrow(e, numberContext, code, null);
      showTakenPopup(record, selectedTemplateId !== null);
      return;
    }

    if (outcome.outcome === 'created') {
      pendingConfirm = null;
      onSuccess(outcome);
      onClose();
      pushToast('success', `Картридж «${outcome.code}» добавлен.`);
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
    pushToast('error', 'Не удалось сохранить картридж. Повторите попытку.');
  }

  async function submitEdit(currentTarget: CartridgeDto) {
    // Update: передаём place_id + notes (code не меняется через update,
    // SPEC boundaries — out of scope for this plan).
    const result = await cartridges.update(
      currentTarget.id,
      currentTarget.version,
      placeId,
      notes.trim() || null,
    );
    onSuccess(result);
    onClose();
    pushToast('success', `Картридж «${result.code}» обновлён.`);
  }

  async function handleSubmit() {
    // GAP-8 defense-in-depth — see the readonly comment at the top of this file.
    if (readonly) return;
    if (!validate() || submitting) return;

    submitting = true;
    loading = true;

    try {
      if (isEdit && target) {
        await submitEdit(target);
      } else {
        await submitCreate();
      }
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : '';
      pushToast('error', msg || 'Не удалось сохранить картридж. Повторите попытку.');
    } finally {
      submitting = false;
      loading = false;
    }
  }

  // Register submit function for footer button (no reactive trigger — direct call)
  onMount(() => {
    onRegisterSubmit(handleSubmit);
  });
</script>

<div class="form">
  <!-- Вид расходника (только при создании) — определяет модели, состояние и код -->
  {#if !isEdit}
    <div class="field">
      <label class="label dropdown-label">
        <span class="label-text">Что добавляем</span>
        <Dropdown
          variant="select"
          flat={true}
          value={kindLabel}
          placeholder="Выберите вид"
          searchPlaceholder="Поиск"
          searchable={false}
          loading={false}
          groups={KIND_OPTIONS}
          getGroupId={(o) => o.id}
          getGroupName={(o) => o.label}
          getGroupCount={() => 0}
          isGroupExpandable={() => false}
          isGroupSelected={(o) => o.id === kindId}
          onExpandGroup={noExpandKind}
          getMemberId={(o) => o.id}
          getMemberName={(o) => o.label}
          onSearch={() => {}}
          onPickGroup={(o) => handleKindChange(String(o.id))}
          onPickMember={() => {}}
        />
      </label>
    </div>
  {/if}

  <!-- Код: обязателен при создании (NUM-13, Phase 40.2 Plan 07 — server-side
       auto-generation retired); при редактировании не меняется через эту
       форму (SPEC boundaries). Create mode: NumberTemplateField + full
       occupied→mismatch→script-mix chain (Plan 15). -->
  <div class="field">
    <label class="label" for="cart-code">Код{isEdit ? '' : ' *'}</label>
    {#if isEdit}
      <Input
        value={code}
        placeholder={codePlaceholder}
        id="cart-code"
        invalid={!!codeError}
        disabled={readonly || isEdit}
        aria-describedby={codeError ? 'cart-code-error' : undefined}
        oninput={(v) => {
          code = v;
        }}
      />
    {:else}
      <NumberTemplateField
        bind:this={numberFieldRef}
        context={numberContext}
        bind:value={code}
        placeholder={codePlaceholder}
        disabled={readonly}
        invalid={!!codeError}
        errorMessage={codeError || null}
        canManageSettings={authStore.user?.role === 'admin'}
        onSelectedTemplateChange={(id, mask) => {
          selectedTemplateId = id;
          selectedTemplateMask = mask;
        }}
      />
    {/if}
    {#if codeError && isEdit}
      <span id="cart-code-error" class="field-error">{codeError}</span>
    {/if}
  </div>

  <!-- Модель (required) -->
  <div class="field">
    <label class="label dropdown-label">
      <span class="label-text">Модель</span>
      <Dropdown
        variant="select"
        flat={true}
        value={selectedModelLabel}
        placeholder="— Выберите модель —"
        searchPlaceholder="Поиск модели"
        invalid={!!modelError}
        disabled={readonly}
        loading={false}
        groups={modelOptions}
        getGroupId={(o) => o.id}
        getGroupName={(o) => o.label}
        getGroupCount={() => 0}
        isGroupExpandable={() => false}
        isGroupSelected={(o) => o.id === modelId}
        onExpandGroup={noExpandModel}
        getMemberId={(o) => o.id}
        getMemberName={(o) => o.label}
        onSearch={() => {}}
        onPickGroup={(o) => {
          modelId = Number(o.id);
          modelError = '';
        }}
        onPickMember={() => {}}
      />
    </label>
    {#if modelError}
      <span class="field-error">{modelError}</span>
    {/if}
  </div>

  <!-- Состояние (заряда — для картриджей; для фотобарабанов: Новый/Изношенный/Отработанный) -->
  {#if !isEdit}
    <div class="field">
      <label class="label dropdown-label">
        <span class="label-text">{stateLabel}</span>
        <Dropdown
          variant="select"
          flat={true}
          value={selectedStateLabel}
          placeholder="Выберите состояние"
          searchPlaceholder="Поиск"
          loading={false}
          groups={stateOptions}
          getGroupId={(o) => o.value}
          getGroupName={(o) => o.label}
          getGroupCount={() => 0}
          isGroupExpandable={() => false}
          isGroupSelected={(o) => o.value === stateId}
          onExpandGroup={noExpandState}
          getMemberId={(o) => o.value}
          getMemberName={(o) => o.label}
          onSearch={() => {}}
          onPickGroup={(o) => (stateId = Number(o.value))}
          onPickMember={() => {}}
        />
      </label>
    </div>
  {/if}

  <!-- Место (optional, D-07) -->
  <div class="field">
    <label class="label" for="cart-place">Место</label>
    <PlacePicker
      value={placeId}
      id="cart-place"
      onChange={(id) => (placeId = id)}
      disabled={readonly}
    />
  </div>

  <!-- Примечания (optional) -->
  <div class="field">
    <label class="label" for="cart-notes">Примечания</label>
    <Textarea
      value={notes}
      placeholder="Необязательно"
      id="cart-notes"
      disabled={readonly}
      oninput={(v) => (notes = v)}
    />
  </div>
</div>

<style lang="scss">
  .form {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-md);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-2xs);
  }

  .label {
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-secondary);
    font-weight: var(--tr-font-weight-regular);
  }

  // Plan 27-G1: Dropdown не принимает `id`, поэтому подпись оборачивает поле
  // (implicit label) вместо `for`/`id` association — сохраняет вертикальный
  // макет «подпись сверху, поле снизу», как у остальных .field.
  .dropdown-label {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-2xs);
  }

  .field-error {
    font-size: var(--tr-font-size-label);
    color: var(--tr-danger);
  }
</style>

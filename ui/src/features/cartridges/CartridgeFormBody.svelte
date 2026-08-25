<script lang="ts">
  // Plan 04-05: CartridgeFormBody — inner form state component for CartridgeFormModal.
  // Remounted on every {#key openInstanceCounter} — guarantees field reset.
  // Поля (UI-SPEC §CartridgeFormModal): Код (авто/ручной) + Модель + Состояние заряда + Место (D-12) + Примечания.
  //
  // GAP-8 (39-UAT.md, Прогон 3): `readonly` — read-only mode for
  // PlaceEntityViewModal.svelte's «Просмотр картриджа» popup. Mirrors
  // DeviceFormBody.svelte's identical readonly contract: every field's own
  // `disabled` prop threaded from one flag, `canSubmit` forced false,
  // `handleSubmit` early-returns as a defense-in-depth guard.
  import { onMount } from 'svelte';
  import Input from '$lib/components/Input.svelte';
  // Plan 27-G1: Select (нативный <select>) заменён на кастомный Dropdown
  // (flat + variant="select") — открывающееся меню больше не нативное OS-меню.
  // Dropdown не принимает `id`/`for`, поэтому подпись оборачивает поле
  // (implicit label), а не связывается через `for` (как раньше у Select).
  import Dropdown from '$lib/components/Dropdown.svelte';
  import Textarea from '$lib/components/Textarea.svelte';
  import PlacePicker from '$lib/components/PlacePicker.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { cartridges } from './api';
  import type { CartridgeDto, CartridgeModelDto } from '../../bindings';

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
  let submitting = $state(false);

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
  }

  // canSubmit: Модель обязательна
  const canSubmit = $derived(!readonly && !submitting && modelId !== null);

  // Sync canSubmit upward
  $effect(() => {
    onCanSubmitChange(canSubmit);
  });

  function validate(): boolean {
    let valid = true;
    codeError = '';
    modelError = '';

    if (modelId === null) {
      modelError = 'Выберите модель картриджа';
      valid = false;
    }

    return valid;
  }

  async function handleSubmit() {
    // GAP-8 defense-in-depth — see the readonly comment at the top of this file.
    if (readonly) return;
    if (!validate() || submitting) return;

    submitting = true;
    onLoading(true);
    codeError = '';

    try {
      if (isEdit && target) {
        // Update: передаём place_id + notes (code не меняется через update)
        const result = await cartridges.update(
          target.id,
          target.version,
          placeId,
          notes.trim() || null,
        );
        onSuccess(result);
        onClose();
        pushToast('success', `Картридж «${result.code}» обновлён.`);
      } else {
        // Create
        const result = await cartridges.create({
          model_id: modelId!,
          code_override: code.trim() || null, // пустая строка → авто-код
          state_id: stateId,
          place_id: placeId,
          notes: notes.trim() || null,
        });
        onSuccess(result);
        onClose();
        pushToast('success', `Картридж «${result.code}» добавлен.`);
      }
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : '';

      // Конфликт кода — UI-SPEC §Ошибочные состояния
      if (
        msg.toLowerCase().includes('conflict') ||
        msg.toLowerCase().includes('уже существует') ||
        (e && typeof e === 'object' && 'code' in e && (e as { code: unknown }).code === 'Conflict')
      ) {
        codeError = `Картридж с кодом «${code.trim()}» уже существует. Введите другой код.`;
      } else {
        pushToast('error', msg || 'Не удалось сохранить картридж. Повторите попытку.');
      }
    } finally {
      submitting = false;
      onLoading(false);
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

  <!-- Код (optional — авто, если пусто) -->
  <div class="field">
    <label class="label" for="cart-code">Код</label>
    <Input
      value={code}
      placeholder={codePlaceholder}
      id="cart-code"
      invalid={!!codeError}
      disabled={readonly}
      aria-describedby={codeError ? 'cart-code-error' : 'cart-code-hint'}
      oninput={(v) => {
        code = v;
        codeError = '';
      }}
    />
    {#if codeError}
      <span id="cart-code-error" class="field-error">{codeError}</span>
    {:else}
      <span id="cart-code-hint" class="field-hint"
        >Будет присвоен автоматически. Введите свой код (например, штрих-код) при необходимости.</span
      >
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

  .field-hint {
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-tertiary);
  }

  .field-error {
    font-size: var(--tr-font-size-label);
    color: var(--tr-danger);
  }
</style>

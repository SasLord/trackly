<script lang="ts">
  // Plan 04-05: CartridgeFormBody — inner form state component for CartridgeFormModal.
  // Remounted on every {#key openInstanceCounter} — guarantees field reset.
  // Поля (UI-SPEC §CartridgeFormModal): Код (авто/ручной) + Модель + Состояние заряда + Расположение + Примечания.
  import { onMount } from 'svelte';
  import Input from '$lib/components/Input.svelte';
  import Select from '$lib/components/Select.svelte';
  import Textarea from '$lib/components/Textarea.svelte';
  import LocationAutocomplete from '$lib/components/LocationAutocomplete.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { cartridges } from './api';
  import type { CartridgeDto, CartridgeModelDto } from '../../bindings';

  interface Props {
    target: CartridgeDto | null;
    models: CartridgeModelDto[];
    onClose: () => void;
    onSuccess: (_cart: CartridgeDto) => void;
    onLoading: (_l: boolean) => void;
    onCanSubmitChange: (_can: boolean) => void;
    onRegisterSubmit: (_fn: () => void) => void;
  }

  const {
    target,
    models,
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
  let location = $state(target?.location ?? '');
  let notes = $state(target?.notes ?? '');

  // Validation errors
  let codeError = $state('');
  let modelError = $state('');
  let submitting = $state(false);

  // Модели, соответствующие выбранному виду.
  const visibleModels = $derived(models.filter((m) => m.kind_id === kindId));

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
  const canSubmit = $derived(!submitting && modelId !== null);

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
    if (!validate() || submitting) return;

    submitting = true;
    onLoading(true);
    codeError = '';

    try {
      if (isEdit && target) {
        // Update: передаём location + notes (code не меняется через update)
        const result = await cartridges.update(
          target.id,
          target.version,
          location.trim() || null,
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
          location: location.trim() || null,
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
      <label class="label" for="cart-kind">Что добавляем</label>
      <Select value={String(kindId)} id="cart-kind" onchange={handleKindChange}>
        <option value="1">Картридж</option>
        <option value="2">Фотобарабан</option>
      </Select>
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
    <label class="label" for="cart-model">Модель</label>
    <Select
      value={modelId !== null ? String(modelId) : ''}
      id="cart-model"
      invalid={!!modelError}
      onchange={(v) => {
        modelId = v ? parseInt(v, 10) : null;
        modelError = '';
      }}
    >
      <option value="">— Выберите модель —</option>
      {#each visibleModels as m (m.id)}
        <option value={String(m.id)}>{m.brand} {m.model}</option>
      {/each}
    </Select>
    {#if modelError}
      <span class="field-error">{modelError}</span>
    {/if}
  </div>

  <!-- Состояние (заряда — для картриджей; для фотобарабанов: Новый/Изношенный/Отработанный) -->
  {#if !isEdit}
    <div class="field">
      <label class="label" for="cart-state">{stateLabel}</label>
      <Select value={String(stateId)} id="cart-state" onchange={(v) => (stateId = parseInt(v, 10))}>
        {#each stateOptions as opt (opt.value)}
          <option value={String(opt.value)}>{opt.label}</option>
        {/each}
      </Select>
    </div>
  {/if}

  <!-- Расположение (optional) -->
  <div class="field">
    <label class="label" for="cart-location">Расположение</label>
    <LocationAutocomplete
      value={location}
      placeholder="Расположение (необязательно)"
      id="cart-location"
      onChange={(v) => (location = v)}
    />
  </div>

  <!-- Примечания (optional) -->
  <div class="field">
    <label class="label" for="cart-notes">Примечания</label>
    <Textarea
      value={notes}
      placeholder="Необязательно"
      id="cart-notes"
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

  .field-hint {
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-tertiary);
  }

  .field-error {
    font-size: var(--tr-font-size-label);
    color: var(--tr-danger);
  }
</style>

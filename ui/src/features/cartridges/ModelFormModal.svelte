<script lang="ts">
  // Plan 04-06: CRUD-модалка модели картриджа.
  // По образцу DeviceFormModal.svelte — openInstanceCounter + {#key} remount.
  // size="wide" (960px) из-за CompatibilityEditor.
  // Plan 13-06 (R3): свёрнуто до ОДНОГО блока совместимости — compatibility:
  // string[] (V032/Phase 13 single-column contract), CompatibleDevicesEditor
  // (V029 per-device чеклист) удалён.
  import Modal from '$lib/components/Modal.svelte';
  import Button from '$lib/components/Button.svelte';
  import Select from '$lib/components/Select.svelte';
  import Textarea from '$lib/components/Textarea.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import CompatibilityEditor from './CompatibilityEditor.svelte';
  import { cartridges } from './api';
  import type { CartridgeModelDto } from '../../bindings';

  // Цвета (D-Model-Color-01), UI-SPEC §ModelFormModal.
  const COLOR_OPTIONS = [
    'Чёрный',
    'Голубой',
    'Пурпурный',
    'Жёлтый',
    'Светло-голубой',
    'Светло-пурпурный',
  ];

  interface Props {
    open: boolean;
    target: CartridgeModelDto | null;
    onClose: () => void;
    onSuccess: (_model: CartridgeModelDto) => void;
  }

  const { open, target, onClose, onSuccess }: Props = $props();

  const isEdit = $derived(target !== null);
  const modalTitle = $derived(isEdit ? 'Редактирование модели' : 'Новая модель картриджа');
  const submitLabel = $derived(isEdit ? 'Сохранить изменения' : 'Добавить модель');

  // openInstanceCounter: сброс формы при каждом открытии.
  let openInstanceCounter = $state(0);
  let _wasOpen = $state(false);

  // --- Form state (внутри {#key openInstanceCounter}) ---
  let kindId = $state<number>(target?.kind_id ?? 1);
  let brand = $state(target?.brand ?? '');
  let model = $state(target?.model ?? '');
  let color = $state(target?.color ?? 'Чёрный');
  let notes = $state(target?.notes ?? '');

  // Совместимость: список имён принтеров (V032/Phase 13, прямое присваивание).
  let compatibility = $state<string[]>(target?.compatibility ?? []);

  $effect(() => {
    const isOpen = open;
    if (isOpen && !_wasOpen) {
      openInstanceCounter += 1;
      // Сброс формы при каждом открытии. {#key} ремаунтит только разметку, но
      // эти $state живут на уровне компонента и сами не реинициализируются —
      // поэтому без явного сброса при повторном «Добавить модель» оставались
      // данные ранее созданной модели (UAT round 2, замечание №1).
      kindId = target?.kind_id ?? 1;
      brand = target?.brand ?? '';
      model = target?.model ?? '';
      color = target?.color ?? 'Чёрный';
      notes = target?.notes ?? '';
      compatibility = target?.compatibility ?? [];
      brandError = '';
      modelError = '';
      conflictError = '';
      brandSuggestOpen = false;
      modelSuggestOpen = false;
      brandSuggestions = [];
      modelSuggestions = [];
    }
    _wasOpen = isOpen;
  });

  // Ошибки валидации
  let brandError = $state('');
  let modelError = $state('');
  let conflictError = $state('');

  let submitting = $state(false);

  // Автокомплит бренда и модели
  let brandSuggestions = $state<string[]>([]);
  let brandSuggestOpen = $state(false);
  let brandActiveIndex = $state(-1);
  let brandDebounce: ReturnType<typeof setTimeout> | null = null;
  let brandWrapperEl = $state<HTMLDivElement | null>(null);
  let brandSuppressed = $state(false);

  let modelSuggestions = $state<string[]>([]);
  let modelSuggestOpen = $state(false);
  let modelActiveIndex = $state(-1);
  let modelDebounce: ReturnType<typeof setTimeout> | null = null;
  let modelWrapperEl = $state<HTMLDivElement | null>(null);
  let modelSuppressed = $state(false);

  async function fetchBrandSuggestions(prefix: string) {
    try {
      brandSuggestions = await cartridges.suggestBrand(prefix);
      if (!brandSuppressed) brandSuggestOpen = brandSuggestions.length > 0;
      brandActiveIndex = -1;
    } catch {
      brandSuggestions = [];
      brandSuggestOpen = false;
    }
  }

  function handleBrandFocus() {
    brandSuppressed = false;
    if (brandDebounce !== null) clearTimeout(brandDebounce);
    brandDebounce = setTimeout(() => void fetchBrandSuggestions(brand), 0);
  }

  function handleBrandInput(value: string) {
    brand = value;
    brandError = '';
    conflictError = '';
    brandSuppressed = false;
    if (brandDebounce !== null) clearTimeout(brandDebounce);
    brandDebounce = setTimeout(() => void fetchBrandSuggestions(value), 200);
    brandSuggestOpen = true;
  }

  function selectBrand(value: string) {
    brand = value;
    brandSuggestOpen = false;
    brandSuppressed = true;
    brandSuggestions = [];
    brandActiveIndex = -1;
  }

  function handleBrandKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      brandSuggestOpen = false;
      return;
    }
    if (!brandSuggestOpen || brandSuggestions.length === 0) return;
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      brandActiveIndex = (brandActiveIndex + 1) % brandSuggestions.length;
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      brandActiveIndex = brandActiveIndex <= 0 ? brandSuggestions.length - 1 : brandActiveIndex - 1;
    } else if (e.key === 'Enter') {
      if (brandActiveIndex >= 0) {
        e.preventDefault();
        selectBrand(brandSuggestions[brandActiveIndex]);
      }
    } else if (e.key === 'Tab') {
      if (brandActiveIndex >= 0) selectBrand(brandSuggestions[brandActiveIndex]);
      brandSuggestOpen = false;
    }
  }

  async function fetchModelSuggestions(prefix: string) {
    try {
      modelSuggestions = await cartridges.suggestModel(brand, prefix);
      if (!modelSuppressed) modelSuggestOpen = modelSuggestions.length > 0;
      modelActiveIndex = -1;
    } catch {
      modelSuggestions = [];
      modelSuggestOpen = false;
    }
  }

  function handleModelFocus() {
    modelSuppressed = false;
    if (modelDebounce !== null) clearTimeout(modelDebounce);
    modelDebounce = setTimeout(() => void fetchModelSuggestions(model), 0);
  }

  function handleModelInput(value: string) {
    model = value;
    modelError = '';
    conflictError = '';
    modelSuppressed = false;
    if (modelDebounce !== null) clearTimeout(modelDebounce);
    modelDebounce = setTimeout(() => void fetchModelSuggestions(value), 200);
    modelSuggestOpen = true;
  }

  function selectModel(value: string) {
    model = value;
    modelSuggestOpen = false;
    modelSuppressed = true;
    modelSuggestions = [];
    modelActiveIndex = -1;
  }

  function handleModelKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      modelSuggestOpen = false;
      return;
    }
    if (!modelSuggestOpen || modelSuggestions.length === 0) return;
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      modelActiveIndex = (modelActiveIndex + 1) % modelSuggestions.length;
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      modelActiveIndex = modelActiveIndex <= 0 ? modelSuggestions.length - 1 : modelActiveIndex - 1;
    } else if (e.key === 'Enter') {
      if (modelActiveIndex >= 0) {
        e.preventDefault();
        selectModel(modelSuggestions[modelActiveIndex]);
      }
    } else if (e.key === 'Tab') {
      if (modelActiveIndex >= 0) selectModel(modelSuggestions[modelActiveIndex]);
      modelSuggestOpen = false;
    }
  }

  function handleClickOutside(e: MouseEvent) {
    const target = e.target as Node;
    if (brandWrapperEl && !brandWrapperEl.contains(target)) brandSuggestOpen = false;
    if (modelWrapperEl && !modelWrapperEl.contains(target)) modelSuggestOpen = false;
  }

  $effect(() => {
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  });

  function validate(): boolean {
    let ok = true;
    if (!brand.trim()) {
      brandError = 'Заполните это поле';
      ok = false;
    }
    if (!model.trim()) {
      modelError = 'Заполните это поле';
      ok = false;
    }
    return ok;
  }

  async function handleSubmit() {
    conflictError = '';
    if (!validate()) return;

    // T-04-06-02: фильтруем пустые/дублирующиеся строки перед submit.
    const filteredCompatibility = Array.from(
      new Set(compatibility.map((s) => s.trim()).filter((s) => s.length > 0)),
    );

    submitting = true;
    try {
      let result: CartridgeModelDto;
      if (isEdit && target) {
        result = await cartridges.modelsUpdate({
          id: target.id,
          version: target.version,
          brand: brand.trim(),
          model: model.trim(),
          kind_id: kindId,
          color: kindId !== 2 ? color || null : null,
          notes: notes.trim() || null,
          compatibility: filteredCompatibility,
        });
      } else {
        result = await cartridges.modelsCreate({
          brand: brand.trim(),
          model: model.trim(),
          kind_id: kindId,
          color: kindId !== 2 ? color || null : null,
          notes: notes.trim() || null,
          compatibility: filteredCompatibility,
        });
      }
      pushToast('success', isEdit ? 'Модель обновлена.' : 'Модель создана.');
      onSuccess(result);
      onClose();
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось сохранить модель';
      // T-04-06-01: показываем конфликт «бренд+модель уже существует» инлайн.
      if (msg.toLowerCase().includes('уже') || msg.toLowerCase().includes('exist')) {
        conflictError = `Модель «${brand.trim()} ${model.trim()}» уже создана`;
      } else if (msg.toLowerCase().includes('изменились') || msg.toLowerCase().includes('lock')) {
        conflictError = 'Данные изменились в другом окне. Обновите страницу.';
      } else {
        pushToast('error', msg);
      }
    } finally {
      submitting = false;
    }
  }

  const canSubmit = $derived(brand.trim().length > 0 && model.trim().length > 0 && !submitting);
</script>

<Modal {open} title={modalTitle} size="wide" {onClose}>
  {#key openInstanceCounter}
    <div class="form-grid">
      <!-- Тип расходника -->
      <div class="field field-full">
        <label class="field-label" for="model-kind">Тип расходника</label>
        <Select
          id="model-kind"
          value={String(kindId)}
          onchange={(v) => {
            kindId = Number(v);
          }}
        >
          <option value="1">Картридж</option>
          <option value="2">Фотобарабан</option>
        </Select>
      </div>

      <!-- Бренд -->
      <div class="field">
        <label class="field-label" for="model-brand">Бренд <span class="required">*</span></label>
        <div class="autocomplete-wrapper" bind:this={brandWrapperEl}>
          <input
            id="model-brand"
            type="text"
            class="autocomplete-input"
            class:invalid={!!brandError || !!conflictError}
            value={brand}
            placeholder="Например: Pantum"
            autocomplete="off"
            aria-autocomplete="list"
            oninput={(e) => handleBrandInput((e.currentTarget as HTMLInputElement).value)}
            onfocus={handleBrandFocus}
            onkeydown={handleBrandKeydown}
          />
          {#if brandSuggestOpen && brandSuggestions.length > 0}
            <div class="dropdown" role="listbox">
              {#each brandSuggestions as s, i (s)}
                <button
                  type="button"
                  role="option"
                  class="dropdown-item"
                  class:active={i === brandActiveIndex}
                  aria-selected={i === brandActiveIndex}
                  onmousedown={(e) => {
                    e.preventDefault();
                    selectBrand(s);
                  }}
                >
                  {s}
                </button>
              {/each}
            </div>
          {/if}
        </div>
        {#if brandError}
          <span class="field-error">{brandError}</span>
        {/if}
      </div>

      <!-- Модель -->
      <div class="field">
        <label class="field-label" for="model-model">Модель <span class="required">*</span></label>
        <div class="autocomplete-wrapper" bind:this={modelWrapperEl}>
          <input
            id="model-model"
            type="text"
            class="autocomplete-input"
            class:invalid={!!modelError || !!conflictError}
            value={model}
            placeholder="Например: TL-5120X"
            autocomplete="off"
            aria-autocomplete="list"
            oninput={(e) => handleModelInput((e.currentTarget as HTMLInputElement).value)}
            onfocus={handleModelFocus}
            onkeydown={handleModelKeydown}
          />
          {#if modelSuggestOpen && modelSuggestions.length > 0}
            <div class="dropdown" role="listbox">
              {#each modelSuggestions as s, i (s)}
                <button
                  type="button"
                  role="option"
                  class="dropdown-item"
                  class:active={i === modelActiveIndex}
                  aria-selected={i === modelActiveIndex}
                  onmousedown={(e) => {
                    e.preventDefault();
                    selectModel(s);
                  }}
                >
                  {s}
                </button>
              {/each}
            </div>
          {/if}
        </div>
        {#if modelError}
          <span class="field-error">{modelError}</span>
        {/if}
      </div>

      <!-- Цвет (только для Картриджа, kind_id !== 2) -->
      {#if kindId !== 2}
        <div class="field">
          <label class="field-label" for="model-color">Цвет</label>
          <Select id="model-color" value={color} onchange={(v) => (color = v)}>
            {#each COLOR_OPTIONS as c (c)}
              <option value={c}>{c}</option>
            {/each}
          </Select>
        </div>
      {/if}

      <!-- Примечание -->
      <div class="field field-full">
        <label class="field-label" for="model-notes">Примечание</label>
        <Textarea
          id="model-notes"
          value={notes}
          placeholder="Необязательно"
          oninput={(v) => (notes = v)}
        />
      </div>

      <!-- Конфликт Бренд+Модель -->
      {#if conflictError}
        <div class="field-error conflict-error field-full">{conflictError}</div>
      {/if}

      <!-- CompatibilityEditor — единственный блок совместимости (R3), полная ширина -->
      <div class="field field-full compat-section">
        <h3 class="compat-heading">Совместимые принтеры</h3>
        {#if compatibility.length === 0}
          <p class="compat-empty">
            Совместимость не задана — картриджи этой модели подходят к любому принтеру.
          </p>
        {/if}
        <CompatibilityEditor
          {compatibility}
          onChange={(names) => (compatibility = names)}
          suggestFn={(prefix) => cartridges.suggestCompatPrinter(prefix)}
        />
      </div>
    </div>
  {/key}

  {#snippet footer()}
    <Button variant="secondary" onclick={onClose}>Отмена</Button>
    <Button variant="primary" loading={submitting} disabled={!canSubmit} onclick={handleSubmit}>
      {#if submitting}Сохранение…{:else}{submitLabel}{/if}
    </Button>
  {/snippet}
</Modal>

<style lang="scss">
  .form-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-md);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .field-full {
    grid-column: 1 / -1;
  }

  .field-label {
    font-size: var(--font-size-label);
    color: var(--color-text-secondary);
    line-height: var(--line-height-label);
  }

  .required {
    color: var(--color-destructive);
  }

  .field-error {
    font-size: var(--font-size-label);
    color: var(--color-destructive);
    margin-top: 2px;
  }

  .conflict-error {
    padding: var(--space-sm) var(--space-md);
    background: color-mix(in srgb, var(--color-destructive) 8%, transparent);
    border: 1px solid var(--color-destructive);
    border-radius: var(--radius-sm);
  }

  .autocomplete-wrapper {
    position: relative;
  }

  .autocomplete-input {
    display: block;
    width: 100%;
    height: 36px;
    padding: 0 var(--space-md);
    background: var(--color-bg);
    color: var(--color-text-primary);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    font-family: var(--font-family-base);
    font-size: var(--font-size-body);
    line-height: var(--line-height-body);

    &::placeholder {
      color: var(--color-text-muted);
    }

    &:focus-visible {
      outline: none;
      border-color: var(--color-accent);
      box-shadow: 0 0 0 3px var(--color-accent-focus);
    }

    &.invalid {
      border-color: var(--color-destructive);
      box-shadow: 0 0 0 3px rgba(220, 38, 38, 0.2);
    }
  }

  .dropdown {
    position: absolute;
    top: calc(100% + 2px);
    left: 0;
    right: 0;
    z-index: 50;
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    box-shadow: var(--shadow-md);
    max-height: 200px;
    overflow-y: auto;
  }

  .dropdown-item {
    display: block;
    width: 100%;
    padding: var(--space-sm) var(--space-md);
    background: transparent;
    border: none;
    text-align: left;
    color: var(--color-text-primary);
    font-family: inherit;
    font-size: var(--font-size-body);
    cursor: pointer;

    &:hover,
    &.active {
      background: var(--color-surface-hover);
    }
  }

  .compat-section {
    border-top: 1px solid var(--color-border);
    padding-top: var(--space-md);
    margin-top: var(--space-xs);
  }

  .compat-heading {
    margin: 0 0 var(--space-sm);
    font-size: var(--font-size-body);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
  }

  .compat-empty {
    margin: 0 0 var(--space-sm);
    font-size: var(--font-size-body);
    color: var(--color-text-muted);
  }
</style>

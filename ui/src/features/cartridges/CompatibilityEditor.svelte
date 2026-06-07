<script lang="ts">
  // Plan 04-06: добавляемый список пар (Бренд принтера + Модель принтера) с focus-open autocomplete.
  // Нет аналога — новый паттерн.
  // Пары хранятся как [printer_brand, printer_model][] (совместимо с CartridgeModelCreateDto.compatibility).
  import Button from '$lib/components/Button.svelte';

  interface CompatRow {
    printer_brand: string;
    printer_model: string;
  }

  interface Props {
    compatibility: CompatRow[];
    onChange: (_pairs: CompatRow[]) => void;
    suggestBrandFn: (_prefix: string) => Promise<string[]>;
    suggestModelFn: (_prefix: string) => Promise<string[]>;
  }

  const { compatibility, onChange, suggestBrandFn, suggestModelFn }: Props = $props();

  // Локальная копия строк — инициализируется при монтировании из prop.
  let rows = $state<CompatRow[]>(compatibility.map((r) => ({ ...r })));

  function addRow() {
    rows = [...rows, { printer_brand: '', printer_model: '' }];
    onChange(rows);
  }

  function removeRow(index: number) {
    rows = rows.filter((_, i) => i !== index);
    onChange(rows);
  }

  function updateBrand(index: number, value: string) {
    rows = rows.map((r, i) => (i === index ? { ...r, printer_brand: value } : r));
    onChange(rows);
  }

  function updateModel(index: number, value: string) {
    rows = rows.map((r, i) => (i === index ? { ...r, printer_model: value } : r));
    onChange(rows);
  }

  // --- inline autocomplete state per row field ---
  // Для избежания громоздких массивов объектов используем единое состояние по (index, field).

  let openKey = $state<string | null>(null); // '{i}-brand' | '{i}-model'
  let suggestions = $state<string[]>([]);
  let activeIndex = $state(-1);
  let loadingKey = $state<string | null>(null);

  let debounceTimer: ReturnType<typeof setTimeout> | null = null;
  let wrapperEls: Record<string, HTMLDivElement | null> = {};

  function getKey(index: number, field: 'brand' | 'model') {
    return `${index}-${field}`;
  }

  function closeSuggestions() {
    openKey = null;
    suggestions = [];
    activeIndex = -1;
  }

  async function fetchSuggestions(index: number, field: 'brand' | 'model', prefix: string) {
    const key = getKey(index, field);
    loadingKey = key;
    try {
      const results = field === 'brand' ? await suggestBrandFn(prefix) : await suggestModelFn(prefix);
      if (openKey === key) {
        suggestions = results;
        activeIndex = -1;
      }
    } catch {
      if (openKey === key) suggestions = [];
    } finally {
      if (loadingKey === key) loadingKey = null;
    }
  }

  function handleFocus(index: number, field: 'brand' | 'model') {
    const key = getKey(index, field);
    openKey = key;
    suggestions = [];
    activeIndex = -1;
    if (debounceTimer !== null) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => {
      void fetchSuggestions(
        index,
        field,
        field === 'brand' ? rows[index]?.printer_brand ?? '' : rows[index]?.printer_model ?? '',
      );
    }, 0);
  }

  function handleInput(index: number, field: 'brand' | 'model', value: string) {
    if (field === 'brand') {
      updateBrand(index, value);
    } else {
      updateModel(index, value);
    }
    const key = getKey(index, field);
    openKey = key;
    if (debounceTimer !== null) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => void fetchSuggestions(index, field, value), 200);
  }

  function selectSuggestion(index: number, field: 'brand' | 'model', value: string) {
    if (field === 'brand') {
      updateBrand(index, value);
    } else {
      updateModel(index, value);
    }
    closeSuggestions();
  }

  function handleKeydown(e: KeyboardEvent, index: number, field: 'brand' | 'model') {
    const key = getKey(index, field);
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      closeSuggestions();
      return;
    }
    if (openKey !== key || suggestions.length === 0) return;
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      activeIndex = (activeIndex + 1) % suggestions.length;
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      activeIndex = activeIndex <= 0 ? suggestions.length - 1 : activeIndex - 1;
    } else if (e.key === 'Enter') {
      if (activeIndex >= 0 && activeIndex < suggestions.length) {
        e.preventDefault();
        e.stopPropagation();
        selectSuggestion(index, field, suggestions[activeIndex]);
      }
    } else if (e.key === 'Tab') {
      if (activeIndex >= 0 && activeIndex < suggestions.length) {
        selectSuggestion(index, field, suggestions[activeIndex]);
      }
      closeSuggestions();
    }
  }

  function handleClickOutside(e: MouseEvent) {
    if (!openKey) return;
    const target = e.target as Node;
    // Проверяем все wrapper'ы — если клик вне всех, закрываем.
    for (const el of Object.values(wrapperEls)) {
      if (el && el.contains(target)) return;
    }
    closeSuggestions();
  }

  $effect(() => {
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  });
</script>

<div class="compat-editor">
  {#each rows as row, i (i)}
    <div class="compat-row">
      <!-- Бренд принтера -->
      <div
        class="compat-field"
        bind:this={wrapperEls[`${i}-brand`]}
        role="none"
      >
        <label class="field-label" for="compat-brand-{i}">Бренд принтера</label>
        <div class="autocomplete-wrapper">
          <input
            id="compat-brand-{i}"
            type="text"
            class="autocomplete-input"
            value={row.printer_brand}
            placeholder="Бренд принтера"
            autocomplete="off"
            aria-autocomplete="list"
            aria-activedescendant={openKey === getKey(i, 'brand') && activeIndex >= 0
              ? `compat-brand-item-${i}-${activeIndex}`
              : undefined}
            oninput={(e) => handleInput(i, 'brand', (e.currentTarget as HTMLInputElement).value)}
            onfocus={() => handleFocus(i, 'brand')}
            onkeydown={(e) => handleKeydown(e, i, 'brand')}
          />
          {#if openKey === getKey(i, 'brand')}
            <div class="dropdown" role="listbox">
              {#if loadingKey === getKey(i, 'brand')}
                <div class="dropdown-loading">Загружаем…</div>
              {:else if suggestions.length === 0}
                <div class="dropdown-empty">Нет совпадений</div>
              {:else}
                {#each suggestions as s, si (s)}
                  <button
                    type="button"
                    id="compat-brand-item-{i}-{si}"
                    role="option"
                    class="dropdown-item"
                    class:active={si === activeIndex}
                    aria-selected={si === activeIndex}
                    onmousedown={(e) => {
                      e.preventDefault();
                      selectSuggestion(i, 'brand', s);
                    }}
                  >
                    {s}
                  </button>
                {/each}
              {/if}
            </div>
          {/if}
        </div>
      </div>

      <!-- Модель принтера -->
      <div
        class="compat-field"
        bind:this={wrapperEls[`${i}-model`]}
        role="none"
      >
        <label class="field-label" for="compat-model-{i}">Модель принтера</label>
        <div class="autocomplete-wrapper">
          <input
            id="compat-model-{i}"
            type="text"
            class="autocomplete-input"
            value={row.printer_model}
            placeholder="Модель принтера"
            autocomplete="off"
            aria-autocomplete="list"
            aria-activedescendant={openKey === getKey(i, 'model') && activeIndex >= 0
              ? `compat-model-item-${i}-${activeIndex}`
              : undefined}
            oninput={(e) => handleInput(i, 'model', (e.currentTarget as HTMLInputElement).value)}
            onfocus={() => handleFocus(i, 'model')}
            onkeydown={(e) => handleKeydown(e, i, 'model')}
          />
          {#if openKey === getKey(i, 'model')}
            <div class="dropdown" role="listbox">
              {#if loadingKey === getKey(i, 'model')}
                <div class="dropdown-loading">Загружаем…</div>
              {:else if suggestions.length === 0}
                <div class="dropdown-empty">Нет совпадений</div>
              {:else}
                {#each suggestions as s, si (s)}
                  <button
                    type="button"
                    id="compat-model-item-{i}-{si}"
                    role="option"
                    class="dropdown-item"
                    class:active={si === activeIndex}
                    aria-selected={si === activeIndex}
                    onmousedown={(e) => {
                      e.preventDefault();
                      selectSuggestion(i, 'model', s);
                    }}
                  >
                    {s}
                  </button>
                {/each}
              {/if}
            </div>
          {/if}
        </div>
      </div>

      <!-- Кнопка удаления строки -->
      <button
        type="button"
        class="remove-btn"
        aria-label="Удалить принтер {i + 1}"
        onclick={() => removeRow(i)}
      >
        ✕
      </button>
    </div>
  {/each}

  <Button variant="secondary" size="sm" onclick={addRow}>+ Добавить принтер</Button>
</div>

<style lang="scss">
  .compat-editor {
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
  }

  .compat-row {
    display: grid;
    grid-template-columns: 1fr 1fr 28px;
    gap: var(--space-sm);
    align-items: end;
    margin-bottom: var(--space-sm);
  }

  .compat-field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .field-label {
    font-size: var(--font-size-label);
    color: var(--color-text-secondary);
    line-height: var(--line-height-label);
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

  .dropdown-loading,
  .dropdown-empty {
    padding: var(--space-sm) var(--space-md);
    color: var(--color-text-muted);
    font-size: var(--font-size-label);
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

  .remove-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    background: transparent;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    color: var(--color-text-secondary);
    cursor: pointer;
    font-size: 12px;
    flex-shrink: 0;
    align-self: flex-end;

    &:hover {
      background: var(--color-surface-sunken);
      color: var(--color-destructive);
      border-color: var(--color-destructive);
    }

    &:focus-visible {
      outline: none;
      box-shadow: 0 0 0 3px var(--color-accent-focus);
    }
  }
</style>

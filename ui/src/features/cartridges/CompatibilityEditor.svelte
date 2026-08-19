<script lang="ts">
  // Plan 04-06: добавляемый список строк (имя принтера) с focus-open autocomplete.
  // Plan 13-06 (R3/D-04): свёрнуто с пар (Бренд+Модель) до одного свободного
  // текстового поля «Имя принтера» — V032/Phase 13 single-column contract
  // (CartridgeModelCreateDto.compatibility: string[]).
  import Button from '$lib/components/Button.svelte';
  import { portal } from '$lib/utils/portal';
  import { dropdownAnchor } from '$lib/utils/dropdownAnchor';

  interface Props {
    compatibility: string[];
    onChange: (_names: string[]) => void;
    suggestFn: (_prefix: string) => Promise<string[]>;
  }

  const { compatibility, onChange, suggestFn }: Props = $props();

  // Локальная копия строк — инициализируется при МОНТИРОВАНИИ из prop и
  // далее НЕ ресинхронизируется (WR-06: prop читается один раз by design).
  // Контракт компонента: данные текут наружу через `onChange`; входящие
  // изменения `compatibility` после монтирования игнорируются. Сброс формы
  // обеспечивается ВЫЗЫВАЮЩИМ кодом через ремонтирование — `ModelFormModal`
  // оборачивает редактор в `{#key openInstanceCounter}`, поэтому при
  // переоткрытии/смене target создаётся новый инстанс и `rows` пере-сеется
  // из свежего prop. Любая будущая правка, сбрасывающая `compatibility` без
  // бампа `openInstanceCounter`, ДОЛЖНА сохранить этот ремонт, иначе
  // редактор останется со старым снимком.
  let rows = $state<string[]>([...compatibility]);

  function addRow() {
    rows = [...rows, ''];
    onChange(rows);
  }

  function removeRow(index: number) {
    rows = rows.filter((_, i) => i !== index);
    onChange(rows);
  }

  function updateName(index: number, value: string) {
    rows = rows.map((r, i) => (i === index ? value : r));
    onChange(rows);
  }

  // --- inline autocomplete state per row ---

  let openKey = $state<string | null>(null); // '{i}'
  let suggestions = $state<string[]>([]);
  let activeIndex = $state(-1);
  let loadingKey = $state<string | null>(null);
  let anchorEl = $state<HTMLInputElement | null>(null);
  let dropdownEl = $state<HTMLDivElement | null>(null);

  let debounceTimer: ReturnType<typeof setTimeout> | null = null;

  function getKey(index: number) {
    return String(index);
  }

  function closeSuggestions() {
    openKey = null;
    suggestions = [];
    activeIndex = -1;
  }

  async function fetchSuggestions(index: number, prefix: string) {
    const key = getKey(index);
    loadingKey = key;
    try {
      const results = await suggestFn(prefix);
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

  function handleFocus(index: number, e: FocusEvent) {
    anchorEl = e.currentTarget as HTMLInputElement;
    const key = getKey(index);
    openKey = key;
    suggestions = [];
    activeIndex = -1;
    if (debounceTimer !== null) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => {
      void fetchSuggestions(index, rows[index] ?? '');
    }, 0);
  }

  function handleInput(index: number, value: string) {
    updateName(index, value);
    const key = getKey(index);
    openKey = key;
    if (debounceTimer !== null) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => void fetchSuggestions(index, value), 200);
  }

  function selectSuggestion(index: number, value: string) {
    updateName(index, value);
    closeSuggestions();
  }

  function handleKeydown(e: KeyboardEvent, index: number) {
    const key = getKey(index);
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
        selectSuggestion(index, suggestions[activeIndex]);
      }
    } else if (e.key === 'Tab') {
      if (activeIndex >= 0 && activeIndex < suggestions.length) {
        selectSuggestion(index, suggestions[activeIndex]);
      }
      closeSuggestions();
    }
  }

  function handleClickOutside(e: MouseEvent) {
    if (!openKey) return;
    // Клик внутри любого поля совместимости — не закрываем (focus переключит
    // openKey сам). Клик вне — закрываем. `closest` убирает необходимость в
    // bind:this в нереактивный объект (svelte binding_property_non_reactive).
    const el = e.target as HTMLElement | null;
    if (el && el.closest('.compat-field')) return;
    if (dropdownEl?.contains(e.target as Node)) return;
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
      <!-- Имя принтера -->
      <div class="compat-field" role="none">
        <label class="visually-hidden" for="compat-name-{i}">Имя принтера {i + 1}</label>
        <div class="autocomplete-wrapper">
          <input
            id="compat-name-{i}"
            type="text"
            class="autocomplete-input"
            value={row}
            placeholder="Наименование / тип принтера"
            autocomplete="off"
            aria-autocomplete="list"
            aria-activedescendant={openKey === getKey(i) && activeIndex >= 0
              ? `compat-name-item-${i}-${activeIndex}`
              : undefined}
            oninput={(e) => handleInput(i, (e.currentTarget as HTMLInputElement).value)}
            onfocus={(e) => handleFocus(i, e)}
            onkeydown={(e) => handleKeydown(e, i)}
          />
          {#if openKey === getKey(i)}
            <div
              class="dropdown--compat"
              role="listbox"
              use:portal
              use:dropdownAnchor={{ anchorEl, maxHeight: 200 }}
              bind:this={dropdownEl}
            >
              {#if loadingKey === getKey(i)}
                <div class="dropdown-loading">Загружаем…</div>
              {:else if suggestions.length === 0}
                <div class="dropdown-empty">Нет совпадений — будет сохранено как есть</div>
              {:else}
                {#each suggestions as s, si (s)}
                  <button
                    type="button"
                    id="compat-name-item-{i}-{si}"
                    role="option"
                    class="dropdown-item"
                    class:active={si === activeIndex}
                    aria-selected={si === activeIndex}
                    onmousedown={(e) => {
                      e.preventDefault();
                      selectSuggestion(i, s);
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
    gap: var(--tr-space-xs);
  }

  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }

  .compat-row {
    display: grid;
    grid-template-columns: 1fr 28px;
    gap: var(--tr-space-xs);
    align-items: end;
    margin-bottom: var(--tr-space-xs);
  }

  .compat-field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .autocomplete-wrapper {
    position: relative;
  }

  .autocomplete-input {
    display: block;
    width: 100%;
    height: 36px;
    padding: 0 var(--tr-space-md);
    // Plan 27-G2: unified field surface — was --tr-bg/--tr-border/--tr-radius-xs,
    // now matches Input.svelte/LocationAutocomplete.svelte/PersonAutocomplete.svelte.
    background: var(--tr-surface-raised);
    color: var(--tr-text-primary);
    border: 1px solid var(--tr-border-strong);
    border-radius: var(--tr-radius-sm);
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
  }

  // Plan 260819-thx (Task 3): портирован в <body> через portal-utility +
  // dropdownAnchor-utility (см. PersonAutocomplete.svelte/DeviceAutocompleteField.svelte/
  // LocationAutocomplete.svelte) — панель больше не живёт внутри
  // .autocomplete-wrapper, поэтому стили здесь :global и namespaced
  // (WR-03: без namespace-класса глобальные правила .dropdown/.dropdown-item
  // коллизируют между несколькими портированными компонентами).
  :global(.dropdown--compat) {
    position: fixed;
    z-index: 1000;
    background: var(--tr-surface-raised);
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-xs);
    box-shadow: var(--tr-elev-2);
    max-height: 200px;
    overflow-y: auto;
  }

  :global(.dropdown--compat .dropdown-loading),
  :global(.dropdown--compat .dropdown-empty) {
    padding: var(--tr-space-xs) var(--tr-space-md);
    color: var(--tr-text-tertiary);
    font-size: var(--tr-font-size-label);
  }

  :global(.dropdown--compat .dropdown-item) {
    display: block;
    width: 100%;
    padding: var(--tr-space-xs) var(--tr-space-md);
    background: transparent;
    border: none;
    text-align: left;
    color: var(--tr-text-primary);
    font-family: inherit;
    font-size: var(--tr-font-size-body);
    cursor: pointer;
  }

  :global(.dropdown--compat .dropdown-item:hover),
  :global(.dropdown--compat .dropdown-item.active) {
    background: var(--tr-row-hover);
  }

  .remove-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    background: transparent;
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-xs);
    color: var(--tr-text-secondary);
    cursor: pointer;
    font-size: 12px;
    flex-shrink: 0;
    align-self: flex-end;

    &:hover {
      background: var(--tr-surface-sunken);
      color: var(--tr-danger);
      border-color: var(--tr-danger);
    }

    &:focus-visible {
      outline: none;
      box-shadow: 0 0 0 3px var(--tr-focus-ring);
    }
  }
</style>

<script lang="ts">
  // PersonAutocomplete — shared autocomplete для полей «Кто сдал»/«Кто принял»
  // в актных модалах. G-5 (Phase 3.1 Plan 02).
  //
  // Pattern simpler than DeviceAutocompleteField:
  //   - source — DISTINCT acts.{giver_name|receiver_name} via acts.suggestPerson.
  //   - debounce 200ms.
  //   - dropdown closes on select (G-4 pre-emptive).
  //   - bindable value (Svelte 5 $bindable).
  //
  // Edit-mode pre-fill suppression: при mount если value non-empty, считаем что
  // оно уже выбрано (lastSelected=value, suppressDropdown=true) — dropdown не
  // открывается на programmatic re-render. Идентичный подход c
  // DeviceAutocompleteField.

  import { onMount, onDestroy } from 'svelte';
  import { acts } from '$lib/api/acts';
  import { portal } from '$lib/utils/portal';
  import { dropdownAnchor } from '$lib/utils/dropdownAnchor';

  type FieldName = 'giver' | 'receiver';

  interface Props {
    field: FieldName;
    value: string;
    placeholder?: string;
    id?: string;
    invalid?: boolean;
    disabled?: boolean;
    onSelect?: (_v: string) => void;
    onChange?: (_v: string) => void;
  }

  let {
    field,
    value = $bindable(''),
    placeholder = 'Введите имя',
    id,
    invalid = false,
    disabled = false,
    onSelect,
    onChange,
  }: Props = $props();

  let suggestions = $state<string[]>([]);
  let loading = $state(false);
  let open = $state(false);
  let activeIndex = $state(-1);
  let wrapperEl = $state<HTMLDivElement | null>(null);
  let inputEl = $state<HTMLInputElement | null>(null);
  let dropdownEl = $state<HTMLDivElement | null>(null);

  let debounceTimer: ReturnType<typeof setTimeout> | null = null;
  let lastSelected: string | null = $state<string | null>(null);
  let suppressDropdown = $state(false);

  onMount(() => {
    if (value.length > 0) {
      lastSelected = value;
      suppressDropdown = true;
    }
  });

  // WR-05: и fetch $effect ниже, и handleFocus() планируют debounce-таймер в
  // одну и ту же переменную debounceTimer, но ни один из путей не отменял
  // pending-таймер на unmount — компонент мог размонтироваться (модал
  // закрыт) с ещё не сработавшим таймером, который потом всё равно issue-ил
  // API-запрос и писал в $state уже мёртвого компонента.
  onDestroy(() => {
    if (debounceTimer !== null) clearTimeout(debounceTimer);
  });

  $effect(() => {
    const v = value;
    if (debounceTimer !== null) clearTimeout(debounceTimer);
    if (v.length < 1) {
      suggestions = [];
      open = false;
      suppressDropdown = false;
      lastSelected = null;
      return;
    }
    debounceTimer = setTimeout(async () => {
      try {
        loading = true;
        suggestions = await acts.suggestPerson(field, v);
        if (!suppressDropdown) {
          open = suggestions.length > 0;
        }
        activeIndex = -1;
      } catch {
        suggestions = [];
        open = false;
      } finally {
        loading = false;
      }
    }, 200);
  });

  function select(s: string) {
    lastSelected = s;
    suppressDropdown = true;
    value = s;
    onChange?.(s);
    onSelect?.(s);
    open = false;
    activeIndex = -1;
    suggestions = [];
  }

  function handleInput(e: Event) {
    const newValue = (e.currentTarget as HTMLInputElement).value;
    if (suppressDropdown && newValue !== lastSelected) {
      suppressDropdown = false;
      lastSelected = null;
    }
    value = newValue;
    onChange?.(newValue);
  }

  function handleFocus() {
    // DEF-1 (Phase 03.2): открываем dropdown сразу на focus (empty prefix → top 20).
    // Сбрасываем suppressDropdown чтобы user-initiated focus всегда открывал дропдаун.
    suppressDropdown = false;
    lastSelected = null;
    if (debounceTimer !== null) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(async () => {
      try {
        loading = true;
        suggestions = await acts.suggestPerson(field, value);
        if (!suppressDropdown) {
          open = suggestions.length > 0;
        }
        activeIndex = -1;
      } catch {
        suggestions = [];
        open = false;
      } finally {
        loading = false;
      }
    }, 0);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'ArrowDown' && !open) {
      e.preventDefault();
      suppressDropdown = false;
      if (suggestions.length > 0) {
        open = true;
        activeIndex = 0;
      }
      return;
    }
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      open = false;
      activeIndex = -1;
      return;
    }
    if (e.key === 'Enter') {
      if (open) {
        e.preventDefault();
        e.stopPropagation();
        if (activeIndex >= 0 && activeIndex < suggestions.length) {
          select(suggestions[activeIndex]);
        }
      }
      return;
    }
    if (!open) return;
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      activeIndex = (activeIndex + 1) % suggestions.length;
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      activeIndex = activeIndex <= 0 ? suggestions.length - 1 : activeIndex - 1;
    } else if (e.key === 'Tab') {
      if (activeIndex >= 0 && activeIndex < suggestions.length) {
        select(suggestions[activeIndex]);
      }
      open = false;
    }
  }

  function handleClickOutside(e: MouseEvent) {
    const target = e.target as Node;
    const insideWrapper = wrapperEl?.contains(target) ?? false;
    const insideDropdown = dropdownEl?.contains(target) ?? false;
    if (!insideWrapper && !insideDropdown) open = false;
  }

  $effect(() => {
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  });
</script>

<div class="autocomplete-wrapper" bind:this={wrapperEl}>
  <input
    type="text"
    bind:this={inputEl}
    {id}
    {placeholder}
    {disabled}
    class="autocomplete-input"
    class:invalid
    {value}
    autocomplete="off"
    aria-autocomplete="list"
    aria-activedescendant={activeIndex >= 0 ? `person-autocomplete-item-${activeIndex}` : undefined}
    oninput={handleInput}
    onfocus={handleFocus}
    onkeydown={handleKeydown}
  />

  {#if open}
    <div class="dropdown--person" role="listbox" use:portal use:dropdownAnchor={{ anchorEl: inputEl }} bind:this={dropdownEl}>
      {#if loading}
        <div class="dropdown-loading">Загружаем подсказки…</div>
      {:else if suggestions.length === 0}
        <div class="dropdown-empty">Совпадений нет</div>
      {:else}
        {#each suggestions as s, i (s)}
          <button
            type="button"
            id="person-autocomplete-item-{i}"
            role="option"
            class="dropdown-item"
            class:active={i === activeIndex}
            aria-selected={i === activeIndex}
            onmousedown={(e) => {
              e.preventDefault();
              select(s);
            }}
          >
            {s}
          </button>
        {/each}
      {/if}
    </div>
  {/if}
</div>

<style lang="scss">
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

    &:disabled {
      background: var(--color-surface-muted);
      color: var(--color-text-muted);
      cursor: not-allowed;
    }
  }

  /*
   * Дропдаун перенесён use:portal в <body>, поэтому scoped CSS компонента до
   * него не доходит — нужен :global(). Позиция (position/top/left/width/bottom)
   * управляется JS через use:dropdownAnchor, здесь только визуал (AUTO-01).
   */
  // WR-03: дропдаун портирован в <body> из НЕСКОЛЬКИХ компонентов
  // (PersonAutocomplete/LocationAutocomplete/DeviceAutocompleteField/
  // ActFormItemsTable) — без namespace-класса на корне глобальные правила
  // .dropdown/.dropdown-item/... коллизируют между компонентами (последний
  // подключённый stylesheet выигрывает). Все правила ниже скопированы под
  // :global(.dropdown--person ...).
  :global(.dropdown--person) {
    position: fixed;
    z-index: 1000;
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    box-shadow: var(--shadow-elev-2);
    max-height: 240px;
    overflow-y: auto;
  }

  /* Дочерние элементы дропдауна тоже перенесены в <body> вместе с ним — :global(). */
  :global(.dropdown--person .dropdown-loading),
  :global(.dropdown--person .dropdown-empty) {
    padding: var(--space-sm) var(--space-md);
    color: var(--color-text-muted);
    font-size: var(--font-size-sm);
  }

  :global(.dropdown--person .dropdown-item) {
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
  }
  :global(.dropdown--person .dropdown-item:hover),
  :global(.dropdown--person .dropdown-item.active) {
    background: var(--color-surface-hover);
  }
</style>

<script lang="ts">
  // UAT-fix: dedicated location autocomplete — БЕРЁТ ВСЕ locations из таблицы
  // `locations` (не device-derived), без фильтра по device name/status.
  // Также (UAT-fix #3): dropdown открывается СРАЗУ на focus (даже для пустого
  // префикса показывает первые 20 расположений).

  import { onDestroy } from 'svelte';
  import { apiCall } from '$lib/api/client';
  import { portal } from '$lib/utils/portal';
  import { dropdownAnchor } from '$lib/utils/dropdownAnchor';

  interface Props {
    value: string;
    placeholder?: string;
    id?: string;
    invalid?: boolean;
    disabled?: boolean;
    onChange: (_v: string) => void;
  }

  let {
    value,
    placeholder = 'Расположение',
    id,
    invalid = false,
    disabled = false,
    onChange,
  }: Props = $props();

  let suggestions = $state<string[]>([]);
  let open = $state(false);
  let activeIndex = $state(-1);
  let wrapperEl = $state<HTMLDivElement | null>(null);
  let inputEl = $state<HTMLInputElement | null>(null);
  let dropdownEl = $state<HTMLDivElement | null>(null);
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;
  let suppress = $state(false);

  async function fetchSuggestions(prefix: string) {
    try {
      suggestions = await apiCall<string[]>('locations_autocomplete', { prefix });
    } catch {
      suggestions = [];
    }
  }

  // WR-05: scheduleFetch() (вызывается и из handleInput, и из handleFocus,
  // и из handleKeydown ArrowDown) не отменял pending-таймер на unmount —
  // компонент мог размонтироваться с ещё не сработавшим debounce-таймером,
  // который потом всё равно issue-ил API-запрос и писал в $state уже
  // мёртвого компонента.
  onDestroy(() => {
    if (debounceTimer !== null) clearTimeout(debounceTimer);
  });

  function scheduleFetch(prefix: string, delayMs: number) {
    if (debounceTimer !== null) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(async () => {
      await fetchSuggestions(prefix);
      if (!suppress) open = suggestions.length > 0;
      activeIndex = -1;
    }, delayMs);
  }

  function handleInput(e: Event) {
    const v = (e.currentTarget as HTMLInputElement).value;
    suppress = false;
    onChange(v);
    scheduleFetch(v, 200);
  }

  function handleFocus() {
    // UAT-fix #3: открываем dropdown сразу на focus (empty prefix → top 20).
    suppress = false;
    scheduleFetch(value, 0);
  }

  function select(s: string) {
    onChange(s);
    suppress = true;
    open = false;
    activeIndex = -1;
    inputEl?.blur();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      open = false;
      return;
    }
    if (e.key === 'ArrowDown' && !open) {
      e.preventDefault();
      suppress = false;
      scheduleFetch(value, 0);
      return;
    }
    if (!open) return;
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
        select(suggestions[activeIndex]);
      }
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
    oninput={handleInput}
    onfocus={handleFocus}
    onkeydown={handleKeydown}
  />

  {#if open && suggestions.length > 0}
    <div
      class="dropdown--location"
      role="listbox"
      use:portal
      use:dropdownAnchor={{ anchorEl: inputEl }}
      bind:this={dropdownEl}
    >
      {#each suggestions as s, i (s)}
        <button
          type="button"
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
  // :global(.dropdown--location ...).
  :global(.dropdown--location) {
    position: fixed;
    z-index: 1000;
    background: var(--color-surface-raised);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    box-shadow: var(--shadow-elev-2);
    max-height: 240px;
    overflow-y: auto;
  }
  /* Дочерние элементы дропдауна тоже перенесены в <body> вместе с ним — :global(). */
  :global(.dropdown--location .dropdown-item) {
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
  :global(.dropdown--location .dropdown-item:hover),
  :global(.dropdown--location .dropdown-item.active) {
    background: var(--color-surface-hover);
  }
</style>

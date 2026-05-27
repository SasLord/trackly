<script lang="ts">
  // DeviceAutocompleteField — reusable autocomplete with contextual support.
  // Per UI-SPEC §DeviceAutocompleteField, DEV-08, DEV-09, D-Autocomplete-01.

  import { devices } from './api';

  type FieldName = 'name' | 'model' | 'specs' | 'kit' | 'state' | 'location';

  interface Props {
    field: FieldName;
    value: string;
    contextName?: string;
    contextStatusId?: number | null;
    placeholder?: string;
    id?: string;
    invalid?: boolean;
    onChange: (_v: string) => void;
  }

  const { field, value, contextName, contextStatusId, placeholder, id, invalid = false, onChange }: Props = $props();

  let suggestions = $state<string[]>([]);
  let loading = $state(false);
  let open = $state(false);
  let activeIndex = $state(-1);

  let wrapperEl = $state<HTMLDivElement | null>(null);

  let debounceTimer: ReturnType<typeof setTimeout> | null = null;

  // Track the last value that was chosen via a suggestion click/keyboard.
  // While suppressDropdown is true the dropdown stays closed even if suggestions
  // are returned — it is cleared only when the user types a character that
  // differs from the just-selected value (or clears the field).
  let lastSelected: string | null = $state(null);
  let suppressDropdown = $state(false);

  // Trigger autocomplete when value or context changes (debounced 200ms).
  // The $effect intentionally does NOT open the dropdown — opening is done
  // only inside the oninput handler so that focusing a pre-filled field
  // does NOT re-open the dropdown.
  $effect(() => {
    const v = value;
    // Track context deps so effect re-runs when they change.
    const ctxName = contextName;
    const ctxStatus = contextStatusId;
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
        suggestions = await devices.autocomplete(field, v, ctxName, ctxStatus);
        // Only open if not suppressed (i.e. user is typing, not just focused).
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
    onChange(s);
    open = false;
    activeIndex = -1;
    suggestions = [];
  }

  function handleInput(e: Event) {
    const newValue = (e.currentTarget as HTMLInputElement).value;
    // If the user is typing a value that differs from the last selected suggestion,
    // lift the suppression so the dropdown can open again.
    if (suppressDropdown && newValue !== lastSelected) {
      suppressDropdown = false;
      lastSelected = null;
    }
    onChange(newValue);
  }

  function handleKeydown(e: KeyboardEvent) {
    // ArrowDown on non-empty field explicitly re-opens the dropdown (escape hatch).
    if (e.key === 'ArrowDown' && !open && value.length > 0) {
      e.preventDefault();
      suppressDropdown = false;
      if (suggestions.length > 0) {
        open = true;
        activeIndex = 0;
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
    } else if (e.key === 'Enter') {
      if (activeIndex >= 0 && activeIndex < suggestions.length) {
        e.preventDefault();
        select(suggestions[activeIndex]);
      }
    } else if (e.key === 'Escape') {
      open = false;
      activeIndex = -1;
    } else if (e.key === 'Tab') {
      if (activeIndex >= 0 && activeIndex < suggestions.length) {
        select(suggestions[activeIndex]);
      }
      open = false;
    }
  }

  function handleClickOutside(e: MouseEvent) {
    if (wrapperEl && !wrapperEl.contains(e.target as Node)) {
      open = false;
    }
  }

  $effect(() => {
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  });
</script>

<div class="autocomplete-wrapper" bind:this={wrapperEl}>
  <input
    type="text"
    {id}
    {placeholder}
    class="autocomplete-input"
    class:invalid
    {value}
    autocomplete="off"
    aria-autocomplete="list"
    aria-activedescendant={activeIndex >= 0 ? `autocomplete-item-${activeIndex}` : undefined}
    oninput={handleInput}
    onkeydown={handleKeydown}
  />

  {#if open}
    <div class="dropdown" role="listbox">
      {#if field !== 'name' && suggestions.length > 0 && (contextName || contextStatusId)}
        <header class="dropdown-header">
          {#if contextName && contextStatusId}
            Ранее использовалось с «{contextName}» в статусе #{contextStatusId}:
          {:else if contextName}
            Ранее использовалось с «{contextName}»:
          {:else if contextStatusId}
            Ранее использовалось в статусе #{contextStatusId}:
          {/if}
        </header>
      {/if}
      {#if loading}
        <div class="dropdown-loading">Загружаем подсказки…</div>
      {:else if suggestions.length === 0}
        <div class="dropdown-empty">Начните вводить, чтобы увидеть подсказки</div>
      {:else}
        {#each suggestions as s, i (s)}
          <button
            type="button"
            id="autocomplete-item-{i}"
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
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.12);
    max-height: 200px;
    overflow-y: auto;
  }

  .dropdown-header {
    padding: var(--space-xs) var(--space-sm);
    font-size: var(--font-size-label);
    color: var(--color-text-secondary);
    border-bottom: 1px solid var(--color-border);
    background: var(--color-surface-sunken);
    font-style: italic;
  }

  .dropdown-loading,
  .dropdown-empty {
    padding: var(--space-sm);
    font-size: var(--font-size-label);
    color: var(--color-text-muted);
    text-align: center;
  }

  .dropdown-item {
    display: block;
    width: 100%;
    padding: var(--space-xs) var(--space-sm);
    background: transparent;
    border: none;
    border-radius: 0;
    font-family: var(--font-family-base);
    font-size: var(--font-size-body);
    color: var(--color-text-primary);
    text-align: left;
    cursor: pointer;

    &:hover {
      background: var(--color-surface-sunken);
    }

    &.active {
      background: color-mix(in srgb, var(--color-accent) 12%, transparent);
      color: var(--color-accent);
    }
  }
</style>

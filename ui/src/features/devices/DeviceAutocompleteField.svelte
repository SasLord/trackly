<script lang="ts">
  // DeviceAutocompleteField — reusable autocomplete with contextual support.
  // Per UI-SPEC §DeviceAutocompleteField, DEV-08, DEV-09, D-Autocomplete-01.
  //
  // Round 8 rewrite: state machine simplified from _userTyping flag + watcher
  // $effect to onMount-based suppression seed.
  //
  // STATE MACHINE (simplified):
  //   1. onMount (fires exactly once): if value is non-empty, seed lastSelected=value
  //      and set suppressDropdown=true. This covers the edit-mode pre-fill case.
  //      Subsequent prop changes from parent do NOT trigger re-suppression.
  //   2. Main fetch $effect: debounces on `value` change, calls autocomplete API,
  //      sets open=true only when !suppressDropdown.
  //   3. handleInput: the ONLY place that can lift suppressDropdown. When the user
  //      types a character that differs from lastSelected, suppression is cleared,
  //      and the next fetch $effect run will open the dropdown.
  //   4. select(s): sets lastSelected=s, suppressDropdown=true — user chose a
  //      suggestion, no need to re-open on the same value.
  //
  // LIMITATION: if the parent changes `value` externally WITHOUT a remount (e.g.
  // via a hypothetical "clear" button), suppressDropdown stays as it was. This is
  // intentional and acceptable — no such "clear" button exists in current UI.
  // When DeviceFormModal remounts DeviceFormBody via {#key openInstanceCounter},
  // this component also remounts → onMount fires fresh → correct suppression seeded.

  import { onMount } from 'svelte';
  import { devices } from './api';

  type FieldName = 'name' | 'model' | 'specs' | 'kit' | 'state' | 'location';

  interface Props {
    field: FieldName;
    value: string;
    contextName?: string;
    contextStatusId?: number | null;
    /** Filter results to devices whose status `code` (V014) is one of these,
     *  e.g. `['на_складе']` to restrict the act-form device autocomplete. */
    statusIn?: string[];
    placeholder?: string;
    id?: string;
    invalid?: boolean;
    /** When true, renders a <textarea> instead of <input>.
     *  Autocomplete dropdown still works identically. */
    multiline?: boolean;
    onChange: (_v: string) => void;
  }

  const {
    field,
    value,
    contextName,
    contextStatusId,
    statusIn,
    placeholder,
    id,
    invalid = false,
    multiline = false,
    onChange,
  }: Props = $props();

  let suggestions = $state<string[]>([]);
  let loading = $state(false);
  let open = $state(false);
  let activeIndex = $state(-1);

  let wrapperEl = $state<HTMLDivElement | null>(null);
  // G-4 (Phase 3.1 Plan 06): inputEl reference для explicit blur() в select(),
  // чтобы при click на suggestion dropdown definitively закрылся и не reopen-ил
  // на refocus до явного нового keystroke. Эквивалентно `justSelected` flag
  // pattern из 03.1-CONTEXT.md G-4, но реализовано через suppressDropdown +
  // lastSelected (см. comments на state machine выше).
  let inputEl = $state<HTMLInputElement | HTMLTextAreaElement | null>(null);

  let debounceTimer: ReturnType<typeof setTimeout> | null = null;

  // Track the last value that was chosen via a suggestion click/keyboard.
  // suppressDropdown: when true, the fetch $effect will NOT open the dropdown
  // even if suggestions are returned.
  let lastSelected: string | null = $state<string | null>(null);
  let suppressDropdown = $state(false);

  // Seed suppression exactly once, on mount.
  // If value is non-empty (edit-mode pre-fill), treat it as already-selected:
  // do not open the dropdown on focus or programmatic value updates.
  onMount(() => {
    if (value.length > 0) {
      lastSelected = value;
      suppressDropdown = true;
    }
  });

  // Trigger autocomplete when value or context changes (debounced 200ms).
  // Opens the dropdown only when suppressDropdown is false — i.e. the user
  // is actively typing (handleInput cleared suppression).
  $effect(() => {
    const v = value;
    // Track context deps so effect re-runs when they change.
    const ctxName = contextName;
    const ctxStatus = contextStatusId;
    const sIn = statusIn;
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
        suggestions = await devices.autocomplete(field, v, ctxName, ctxStatus, sIn);
        // Only open if the user is actively typing (suppression was lifted by handleInput).
        // This prevents the dropdown from re-opening on programmatic value changes
        // (e.g. parent re-rendering, edit-mode pre-fill, prop change from outside).
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
    // G-4 fix (03.1 Plan 06): equivalent of `justSelected` guard — see comment
    // на `inputEl` declaration. suppressDropdown=true defers любой dropdown
    // re-open до genuine handleInput keystroke; explicit blur() убирает focus
    // чтобы visual cues совпали (caret/dropdown оба закрыты).
    lastSelected = s;
    suppressDropdown = true;
    onChange(s);
    open = false;
    activeIndex = -1;
    suggestions = [];
    inputEl?.blur();
  }

  function handleInput(e: Event) {
    const newValue = (e.currentTarget as HTMLInputElement).value;
    // Lift suppression when the user types something that differs from the last
    // selected (or mount-seeded) value, or when they clear the field.
    // This is the ONLY place suppressDropdown is set to false for non-empty input.
    if (suppressDropdown && newValue !== lastSelected) {
      suppressDropdown = false;
      lastSelected = null;
    }
    onChange(newValue);
    // Note: no _userTyping flag needed. onMount handles initial seeding exactly
    // once; subsequent prop changes from parent do not re-arm suppression.
    // The fetch $effect reads suppressDropdown AFTER onChange() has propagated
    // the new value back through the prop — by that point suppressDropdown is
    // already false (lifted above), so the dropdown will open correctly.
  }

  function handleFocus() {
    // DEF-1 (Phase 03.2): открываем dropdown сразу на focus (empty prefix → top 20).
    // Bypass v.length < 1 guard из $effect — делаем прямой fetch с delay 0.
    suppressDropdown = false;
    lastSelected = null;
    if (debounceTimer !== null) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(async () => {
      try {
        loading = true;
        suggestions = await devices.autocomplete(field, value, contextName, contextStatusId, statusIn);
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
    // ArrowDown explicitly re-opens the dropdown (escape hatch), works even on empty field.
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
      // Always prevent Escape from bubbling — callers (form, modal) must not
      // receive it while the dropdown is managing keyboard state.
      e.preventDefault();
      e.stopPropagation();
      open = false;
      activeIndex = -1;
      return;
    }
    if (e.key === 'Enter') {
      if (open) {
        // When dropdown is open, Enter either selects the highlighted suggestion
        // or does nothing — but in BOTH cases it must NOT submit the form.
        e.preventDefault();
        e.stopPropagation();
        if (activeIndex >= 0 && activeIndex < suggestions.length) {
          select(suggestions[activeIndex]);
        }
        // activeIndex === -1: no suggestion focused, dropdown stays open — form
        // submit intentionally suppressed (user is still navigating suggestions).
      }
      // When dropdown is closed, Enter propagates naturally → form submit.
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
  {#if multiline}
    <textarea
      bind:this={inputEl}
      {id}
      {placeholder}
      rows={3}
      class="autocomplete-input autocomplete-textarea"
      class:invalid
      {value}
      autocomplete="off"
      aria-autocomplete="list"
      aria-activedescendant={activeIndex >= 0 ? `autocomplete-item-${activeIndex}` : undefined}
      oninput={handleInput}
      onfocus={handleFocus}
      onkeydown={handleKeydown}
    ></textarea>
  {:else}
    <input
      type="text"
      bind:this={inputEl}
      {id}
      {placeholder}
      class="autocomplete-input"
      class:invalid
      {value}
      autocomplete="off"
      aria-autocomplete="list"
      aria-activedescendant={activeIndex >= 0 ? `autocomplete-item-${activeIndex}` : undefined}
      oninput={handleInput}
      onfocus={handleFocus}
      onkeydown={handleKeydown}
    />
  {/if}

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

  // Multiline variant — overrides the fixed single-line height.
  .autocomplete-textarea {
    height: auto;
    min-height: 76px; // ~ 3 rows
    padding: var(--space-sm) var(--space-md);
    resize: vertical;
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

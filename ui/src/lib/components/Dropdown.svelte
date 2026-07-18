<script lang="ts" generics="TGroup, TMember">
  // Plan 25-02 (CMP-07): generic drill-in combobox/select primitive, extracted
  // (not redesigned) from ActFormItemsTable.svelte's per-row device picker
  // (D-01/D-02 of Phase 25 context). This plan implements the full prop
  // contract, the internal drill-in state machine (AUTO-05 auto-flatten,
  // manual drill-in/backToGroups), and the `variant === 'combobox'` field.
  // Plan 25-03 completes the `variant === 'select'` field and the full
  // keyboard/ARIA layer beyond the pre-existing regression floor (Home/End,
  // member-mode arrow navigation, aria-activedescendant, focus management).
  import { onDestroy } from 'svelte';
  import Spinner from '$lib/components/Spinner.svelte';
  import { portal } from '$lib/utils/portal';
  import { dropdownAnchor } from '$lib/utils/dropdownAnchor';

  interface Props {
    /** D-03: 'combobox' — type directly in the field (implemented here).
     *  'select' — field shows the picked value + in-panel search box
     *  (Plan 25-03). */
    variant: 'combobox' | 'select';
    /** SC #3: flat option list (no drill-in, `groups` IS the option list,
     *  checkmark on `isGroupSelected`). Default `false` = grouped/drill-in
     *  variant (SC #4). */
    flat?: boolean;
    /** One-way, controlled-component convention (D-11 precedent) — NOT
     *  `$bindable`. The caller re-renders `value` itself via `onQueryInput`/
     *  pick callbacks. */
    value: string;
    placeholder?: string;
    /** select-variant only, wired in Plan 25-03. */
    searchPlaceholder?: string;
    invalid?: boolean;
    disabled?: boolean;
    /** Caller-controlled fetch-in-flight flag — drives the panel's
     *  "Загрузка…" state (D-13). */
    loading: boolean;
    /** Current, already-filtered group/option list — Dropdown performs zero
     *  data-fetching itself. */
    groups: TGroup[];
    getGroupId: (g: TGroup) => string | number;
    getGroupName: (g: TGroup) => string;
    getGroupMeta?: (g: TGroup) => string | undefined;
    /** e.g. serial/inventory row — rendered `.tr-mono` in flat mode only. */
    getGroupSub?: (g: TGroup) => string | undefined;
    getGroupCount: (g: TGroup) => number;
    isGroupExpandable: (g: TGroup) => boolean;
    /** flat-mode checkmark. */
    isGroupSelected?: (g: TGroup) => boolean;
    /** Drill-in fetch callback — called BOTH on manual click of an
     *  expandable group (showBack = true) AND internally by AUTO-05's
     *  auto-flatten (showBack = false). */
    onExpandGroup: (g: TGroup) => Promise<TMember[]> | TMember[];
    getMemberId: (m: TMember) => string | number;
    getMemberName: (m: TMember) => string;
    getMemberMeta?: (m: TMember) => string | undefined;
    getMemberSub?: (m: TMember) => string | undefined;
    /** Fires after Dropdown's own internal 250ms debounce on typed input,
     *  and IMMEDIATELY (no debounce) on focus (AUTO-02). */
    onSearch: (query: string) => void;
    /** Fires synchronously on every keystroke (combobox variant), BEFORE the
     *  debounced `onSearch` — lets the caller keep its own `value` in sync
     *  with the DOM input (controlled-input pattern). */
    onQueryInput?: (query: string) => void;
    onPickGroup: (g: TGroup) => void;
    onPickMember: (m: TMember) => void;
  }

  const {
    variant,
    flat = false,
    value,
    placeholder,
    searchPlaceholder = 'Поиск',
    invalid = false,
    disabled = false,
    loading,
    groups,
    getGroupId,
    getGroupName,
    getGroupMeta,
    getGroupSub,
    getGroupCount,
    isGroupExpandable,
    isGroupSelected,
    onExpandGroup,
    getMemberId,
    getMemberName,
    getMemberMeta,
    getMemberSub,
    onSearch,
    onQueryInput,
    onPickGroup,
    onPickMember,
  }: Props = $props();

  // Internal state (D-02): part of the component, NOT caller-supplied props —
  // every future consumer inherits AUTO-02/AUTO-05 and the drill-in mechanics
  // for free instead of re-implementing them per-screen.
  let open = $state(false);
  let viewMode = $state<'groups' | 'members'>('groups');
  let activeGroup = $state<TGroup | null>(null);
  let members = $state<TMember[]>([]);
  /** D-06/checkpoint fix #1: title in member-view is ALWAYS shown; the
   *  "← Назад" button is shown ONLY when the user manually drilled in — two
   *  independent conditions, not one boolean (UI-SPEC correction of
   *  ActFormItemsTable.svelte:568-588). */
  let showBack = $state(false);
  /** Keyboard nav index — wired fully in Plan 25-03; declared here because
   *  the view-mode transitions below already need to reset it. */
  let activeIndex = $state(-1);

  // Plan 18-04 precedent (ActFormItemsTable.svelte): Input.svelte has no
  // ref-forwarding, so the combobox field is a raw <input> with bind:this,
  // used as `anchorEl` for use:dropdownAnchor.
  let inputEl = $state<HTMLInputElement | null>(null);
  let panelEl = $state<HTMLUListElement | null>(null);

  let searchDebounce: ReturnType<typeof setTimeout> | undefined;

  // WR-05 precedent (PersonAutocomplete.svelte/ActFormItemsTable.svelte):
  // cancel any pending debounced onSearch call on unmount so it doesn't fire
  // into a dead component.
  onDestroy(() => {
    if (searchDebounce) clearTimeout(searchDebounce);
  });

  // State-machine rule (mirrors ActFormItemsTable.svelte's fetchGroups
  // end-branch, lines 218-229): whenever `groups` changes, a single
  // remaining group auto-flattens into its members (AUTO-05, showBack =
  // false — nowhere to go back to); otherwise the panel resets to the
  // groups view. Flat mode has no drill-in concept at all (`groups` IS the
  // flat option list), so this machine only runs when `flat` is false.
  $effect(() => {
    if (flat) return;
    const list = groups;
    if (list.length === 1) {
      const only = list[0];
      void (async () => {
        const result = await onExpandGroup(only);
        activeGroup = only;
        members = result;
        viewMode = 'members';
        showBack = false;
        activeIndex = -1;
      })();
    } else {
      viewMode = 'groups';
      activeGroup = null;
      members = [];
      showBack = false;
      activeIndex = -1;
    }
  });

  /** Manual drill-in (D-01/D-06): clicking an expandable group replaces the
   *  panel content with its members and shows "← Назад" (showBack = true) —
   *  unlike AUTO-05's auto-flatten, this is a user-initiated navigation. */
  async function drillInto(g: TGroup) {
    const result = await onExpandGroup(g);
    activeGroup = g;
    members = result;
    viewMode = 'members';
    showBack = true;
    activeIndex = -1;
  }

  /** D-06: "← Назад" — returns from the member list to the group list. */
  function backToGroups() {
    viewMode = 'groups';
    activeGroup = null;
    members = [];
    showBack = false;
    activeIndex = -1;
  }

  /** D-01/D-08: click on an option row in the groups/flat panel. Grouped
   *  mode drills into expandable groups; flat mode (and non-expandable
   *  groups) picks directly. */
  function handleOptionClick(g: TGroup) {
    if (!flat && isGroupExpandable(g)) {
      void drillInto(g);
    } else {
      onPickGroup(g);
    }
  }

  /** Member-view header title (UI-SPEC "two independent conditions" rule):
   *  always shows the active group's name (+ optional meta), independent of
   *  whether the "← Назад" button is also shown. */
  const drillTitle = $derived.by(() => {
    if (!activeGroup) return '';
    const meta = getGroupMeta?.(activeGroup);
    return meta ? `${getGroupName(activeGroup)} · ${meta}` : getGroupName(activeGroup);
  });

  function scheduleSearch(query: string) {
    if (searchDebounce) clearTimeout(searchDebounce);
    searchDebounce = setTimeout(() => onSearch(query), 250);
  }

  function handleInput(e: Event) {
    const query = (e.currentTarget as HTMLInputElement).value;
    onQueryInput?.(query);
    scheduleSearch(query);
  }

  /** AUTO-02: panel opens on focus, no typing required — fires onSearch
   *  immediately (delay 0), not through the 250ms debounce. */
  function handleFocus() {
    open = true;
    if (searchDebounce) clearTimeout(searchDebounce);
    onSearch(value);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && open) {
      e.preventDefault();
      e.stopPropagation();
      open = false;
    }
  }

  // Self-contained click-outside close, mirroring PersonAutocomplete.svelte's
  // existing $effect-based mousedown pattern (single instance per usage, not
  // ActFormItemsTable.svelte's row-indexed Record<number, T> pattern, which
  // collapses away once extracted per PATTERNS.md).
  function handleClickOutside(e: MouseEvent) {
    if (!open) return;
    const target = e.target as Node;
    const insideInput = inputEl?.contains(target) ?? false;
    const insideDropdown = panelEl?.contains(target) ?? false;
    if (!insideInput && !insideDropdown) open = false;
  }

  $effect(() => {
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  });

  // `searchPlaceholder` is select-variant only (in-panel search box) — that
  // field is not implemented until Plan 25-03 (see the commented `{:else}`
  // branch below). Referenced here only to satisfy `noUnusedLocals` until
  // then.
  void searchPlaceholder;
</script>

<div class="tr-dropdown">
  {#if variant === 'combobox'}
    <input
      type="text"
      bind:this={inputEl}
      class="tr-dropdown-field"
      class:invalid
      {value}
      {placeholder}
      {disabled}
      autocomplete="off"
      aria-autocomplete="list"
      oninput={handleInput}
      onfocus={handleFocus}
      onkeydown={handleKeydown}
    />
  {:else}
    <!-- TODO Plan 25-03: variant === 'select' field (value display + in-panel
         search box). Left unimplemented in Plan 25-02 per its explicit scope
         boundary — renders nothing rather than throwing. -->
  {/if}

  {#if open}
    <ul
      class="tr-dropdown-panel"
      class:tr-dropdown-panel--flat={flat}
      role="listbox"
      use:portal
      use:dropdownAnchor={{ anchorEl: inputEl, maxHeight: flat ? 240 : 280 }}
      bind:this={panelEl}
    >
      {#if !flat && viewMode === 'members'}
        <!-- D-01/D-06 drill-in header — checkpoint fix #1 (UI-SPEC): title is
             ALWAYS shown in member-view; "← Назад" only on manual drill-in
             (showBack), not on AUTO-05 auto-flatten. Two independent
             conditions, not one boolean. -->
        <li class="tr-dropdown-drill-header">
          {#if showBack}
            <button
              type="button"
              class="tr-dropdown-drill-back"
              onmousedown={(e) => e.preventDefault()}
              onclick={backToGroups}
            >
              ← Назад
            </button>
          {/if}
          <span class="tr-dropdown-drill-title">{drillTitle}</span>
        </li>
        {#if loading}
          <li class="tr-dropdown-loading"><Spinner size="sm" />Загрузка…</li>
        {:else if members.length === 0}
          <li class="tr-dropdown-empty">Ничего не найдено</li>
        {:else}
          {#each members as m (getMemberId(m))}
            <li>
              <button
                type="button"
                class="tr-dropdown-option"
                role="option"
                aria-selected="false"
                onmousedown={(e) => e.preventDefault()}
                onclick={() => onPickMember(m)}
              >
                <span class="tr-dropdown-option-row">
                  <span class="tr-dropdown-option-name">{getMemberName(m)}</span>
                  {#if getMemberMeta?.(m)}
                    <span class="tr-dropdown-option-meta">{getMemberMeta(m)}</span>
                  {/if}
                </span>
                {#if getMemberSub?.(m)}
                  <span class="tr-dropdown-option-sub">{getMemberSub(m)}</span>
                {/if}
              </button>
            </li>
          {/each}
        {/if}
      {:else if loading}
        <li class="tr-dropdown-loading"><Spinner size="sm" />Загрузка…</li>
      {:else if groups.length === 0}
        <li class="tr-dropdown-empty">Ничего не найдено</li>
      {:else}
        {#each groups as g, i (getGroupId(g))}
          <li>
            <button
              type="button"
              class="tr-dropdown-option"
              class:active={i === activeIndex}
              class:selected={flat && !!isGroupSelected?.(g)}
              role="option"
              aria-selected={flat ? !!isGroupSelected?.(g) : i === activeIndex}
              onmousedown={(e) => e.preventDefault()}
              onclick={() => handleOptionClick(g)}
            >
              <span class="tr-dropdown-option-row">
                <span class="tr-dropdown-option-name" class:tr-dropdown-option-name--flat={flat}
                  >{getGroupName(g)}</span
                >
                {#if getGroupMeta?.(g)}
                  <span class="tr-dropdown-option-meta">{getGroupMeta(g)}</span>
                {/if}
                {#if flat}
                  {#if isGroupSelected?.(g)}
                    <span class="tr-dropdown-option-check" aria-hidden="true">✓</span>
                  {/if}
                {:else}
                  <span class="tr-dropdown-option-count">×{getGroupCount(g)}</span>
                  <span class="tr-dropdown-option-chevron" aria-hidden={!isGroupExpandable(g)}
                    >{isGroupExpandable(g) ? '›' : ''}</span
                  >
                {/if}
              </span>
              {#if getGroupSub?.(g)}
                <span class="tr-dropdown-option-sub" class:tr-mono={flat}>{getGroupSub(g)}</span>
              {/if}
            </button>
          </li>
        {/each}
      {/if}
    </ul>
  {/if}
</div>

<style lang="scss">
  .tr-dropdown {
    position: relative;
  }

  .tr-dropdown-field {
    display: block;
    width: 100%;
    height: 36px;
    padding: 0 12px;
    background: var(--tr-surface);
    color: var(--tr-text-primary);
    border: 1px solid var(--tr-border-strong);
    border-radius: var(--tr-radius-sm);
    font-family: var(--tr-font-family);
    font-size: var(--tr-font-size-body);
    line-height: var(--tr-line-height-body);

    &:focus-visible {
      outline: none;
      border-color: var(--tr-accent);
      box-shadow: 0 0 0 3px var(--tr-focus-ring);
    }
    &.invalid {
      border-color: var(--tr-danger);
      box-shadow: 0 0 0 3px var(--tr-danger-ring);
    }
    &:disabled {
      opacity: 0.6;
      cursor: not-allowed;
    }
  }

  // Plan 18-04 (AUTO-01): the panel is moved to <body> by use:portal, so this
  // component's scoped <style> never reaches it — styling goes through a
  // namespaced global class instead (WR-03: un-namespaced .dropdown/-empty
  // classes collide across the 4+ components that already portal to <body>).
  // This DOES work inside a component's own <style lang="scss"> (compiled by
  // the Svelte compiler) — unlike :global() in a plain .scss file, which is
  // Phase 24 Learning #2's trap.
  :global(.tr-dropdown-panel) {
    position: fixed;
    z-index: 1000;
    overflow: auto;
    max-height: 280px;
    margin: 0;
    padding: 0;
    list-style: none;
    background: var(--tr-surface-raised);
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-md);
    box-shadow: var(--tr-elev-2);
  }
  :global(.tr-dropdown-panel.tr-dropdown-panel--flat) {
    max-height: 240px;
  }

  :global(.tr-dropdown-panel .tr-dropdown-option) {
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 2px;
    width: 100%;
    min-height: 46px;
    padding: 8px 12px;
    text-align: left;
    background: transparent;
    border: none;
    border-bottom: 1px solid var(--tr-border);
    cursor: pointer;
    color: var(--tr-text-primary);
    font-family: var(--tr-font-family);
    font-size: var(--tr-font-size-body);
  }
  :global(.tr-dropdown-panel .tr-dropdown-option:hover),
  :global(.tr-dropdown-panel .tr-dropdown-option.active) {
    background: var(--tr-row-hover);
  }
  :global(.tr-dropdown-panel .tr-dropdown-option.selected) {
    background: var(--tr-row-selected);
  }

  :global(.tr-dropdown-panel .tr-dropdown-option-row) {
    display: flex;
    align-items: baseline;
    gap: 8px;
    width: 100%;
  }
  :global(.tr-dropdown-panel .tr-dropdown-option-name) {
    font-size: 14px;
    font-weight: 600;
    color: var(--tr-text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  :global(.tr-dropdown-panel .tr-dropdown-option-name--flat) {
    font-weight: 500;
  }
  :global(.tr-dropdown-panel .tr-dropdown-option-meta) {
    font-size: 13px;
    color: var(--tr-text-tertiary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  :global(.tr-dropdown-panel .tr-dropdown-option-sub) {
    font-size: 12px;
    color: var(--tr-text-tertiary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  :global(.tr-dropdown-panel .tr-dropdown-option-count) {
    margin-left: auto;
    min-width: 34px;
    flex: 0 0 auto;
    text-align: right;
    font-size: 13px;
    font-weight: 600;
    color: var(--tr-accent-text);
    font-variant-numeric: tabular-nums;
  }
  :global(.tr-dropdown-panel .tr-dropdown-option-chevron) {
    flex: 0 0 auto;
    width: 12px;
    text-align: center;
    color: var(--tr-text-secondary);
    font-size: 12px;
  }
  :global(.tr-dropdown-panel .tr-dropdown-option-check) {
    flex: 0 0 auto;
    width: 14px;
    text-align: center;
    color: var(--tr-accent);
    font-size: 14px;
    font-weight: 700;
  }

  // D-01/checkpoint fix #1: sticky drill-in header, opaque background so
  // member rows don't show through while scrolling underneath it.
  :global(.tr-dropdown-panel .tr-dropdown-drill-header) {
    position: sticky;
    top: 0;
    z-index: 1;
    display: flex;
    align-items: center;
    gap: 8px;
    height: 38px;
    padding: 0 12px;
    background: var(--tr-surface-sunken);
    border-bottom: 1px solid var(--tr-border);
    font-size: 13px;
    list-style: none;
  }
  :global(.tr-dropdown-panel .tr-dropdown-drill-back) {
    flex: 0 0 auto;
    background: transparent;
    border: none;
    padding: 0;
    cursor: pointer;
    font-family: var(--tr-font-family);
    font-size: 13px;
    font-weight: 600;
    color: var(--tr-text-primary);
  }
  :global(.tr-dropdown-panel .tr-dropdown-drill-title) {
    flex: 1 1 auto;
    min-width: 0;
    color: var(--tr-text-tertiary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  // D-13 (canonical copy): both rows share the same 46px height as a normal
  // option row so the panel doesn't jump size when state changes.
  :global(.tr-dropdown-panel .tr-dropdown-empty),
  :global(.tr-dropdown-panel .tr-dropdown-loading) {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    min-height: 46px;
    padding: 8px 12px;
    list-style: none;
    color: var(--tr-text-tertiary);
    font-size: 14px;
  }
</style>

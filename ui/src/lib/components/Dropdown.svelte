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
    if (!insideInput) open = false;
  }

  $effect(() => {
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  });

  // Plan 25-02 Task 2 wires these into the portal-rendered panel markup
  // (drill-in header, group/member rows, empty/loading states). Referenced
  // here only to satisfy the project's `noUnusedLocals` gate until that
  // markup lands later in this same plan.
  void loading;
  void searchPlaceholder;
  void getGroupId;
  void getGroupName;
  void getGroupMeta;
  void getGroupSub;
  void getGroupCount;
  void isGroupExpandable;
  void isGroupSelected;
  void getMemberId;
  void getMemberName;
  void getMemberMeta;
  void getMemberSub;
  void onPickGroup;
  void onPickMember;
  void viewMode;
  void activeGroup;
  void members;
  void showBack;
  void activeIndex;
  void drillInto;
  void backToGroups;
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
</style>

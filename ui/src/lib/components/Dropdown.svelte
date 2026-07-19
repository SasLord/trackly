<script lang="ts" generics="TGroup, TMember">
  // Plan 25-02 (CMP-07): generic drill-in combobox/select primitive, extracted
  // (not redesigned) from ActFormItemsTable.svelte's per-row device picker
  // (D-01/D-02 of Phase 25 context). Plan 25-02 built the full prop contract,
  // the internal drill-in state machine (AUTO-05 auto-flatten, manual
  // drill-in/backToGroups), and the `variant === 'combobox'` field.
  // Plan 25-03 Task 1 built the `variant === 'select'` field (value display +
  // in-panel search box) and the flat-list checkmark mode. Task 2 (this
  // task) adds the full keyboard/ARIA layer beyond the pre-existing
  // regression floor (Home/End, member-mode arrow navigation,
  // aria-activedescendant, two-stage Escape, scrollIntoView).
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

  // Stable per-instance id prefix (Svelte 5.20+ rune) — Dropdown is a
  // reusable primitive with potentially many simultaneous instances (unlike
  // PersonAutocomplete's hardcoded `person-autocomplete-item-*` ids, which
  // assume a single instance per screen).
  const uid = $props.id();
  const panelId = `${uid}-panel`;

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
  /** Keyboard nav index into whichever list is currently visible (`groups`
   *  when `flat` or `viewMode === 'groups'`, `members` otherwise). */
  let activeIndex = $state(-1);
  /** D-12 focus management: the `groups` index to restore `activeIndex` to
   *  when `backToGroups()` returns from a manual drill-in — "при возврате —
   *  та группа, из которой вышли". */
  let returnIndex = $state(-1);

  // Plan 18-04 precedent (ActFormItemsTable.svelte): Input.svelte has no
  // ref-forwarding, so the combobox field is a raw <input> with bind:this,
  // used as `anchorEl` for use:dropdownAnchor. The select-variant field is a
  // raw <button> for the same reason.
  let inputEl = $state<HTMLInputElement | null>(null);
  let triggerEl = $state<HTMLButtonElement | null>(null);
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
  /** CR-02 generation token for every async `onExpandGroup` round-trip
   *  (AUTO-05 auto-flatten below AND manual `drillInto`). `onExpandGroup` is a
   *  real IPC call in the Acts form, so a resolve can land after the user has
   *  typed on and `groups` has moved to a different result set. Without this
   *  guard the stale promise force-writes `viewMode = 'members'` + the OLD
   *  group's member list under the NEW query — clicking one then writes a
   *  `device_id` that does not match what was searched for (and, via DEF-2A
   *  dedup against a stale selection snapshot, can claim an id another row
   *  already holds). A plain `let` (not `$state`) on purpose: the effect
   *  writes it, and making it reactive would retrigger the effect.
   *
   *  Both paths share one counter — auto-flatten and manual drill-in are
   *  mutually exclusive navigation intents, so whichever fires last wins. */
  let expandSeq = 0;

  $effect(() => {
    if (flat) return;
    const list = groups;
    if (list.length === 1) {
      const only = list[0];
      const seq = ++expandSeq;
      void (async () => {
        const result = await onExpandGroup(only);
        // Superseded by a newer `groups` change or a manual drill-in — drop
        // the stale result rather than force it onto the panel.
        if (seq !== expandSeq) return;
        activeGroup = only;
        members = result;
        viewMode = 'members';
        showBack = false;
        // D-12 focus management: entering member-view activates the first
        // option (same rule as manual drillInto below).
        activeIndex = result.length > 0 ? 0 : -1;
      })();
    } else {
      // Cancel any in-flight expand — its result no longer describes `groups`.
      expandSeq++;
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
    // CR-02: same generation guard as the AUTO-05 effect above — without it a
    // slow drill-in fetch still forces `viewMode = 'members'` even if `groups`
    // changed (or the panel closed) between the click and the resolve.
    const seq = ++expandSeq;
    const result = await onExpandGroup(g);
    if (seq !== expandSeq) return;
    // D-12 focus management: remember which group we drilled into so
    // backToGroups() can restore activeIndex to it, not reset to -1.
    returnIndex = groups.findIndex((x) => getGroupId(x) === getGroupId(g));
    activeGroup = g;
    members = result;
    viewMode = 'members';
    showBack = true;
    activeIndex = result.length > 0 ? 0 : -1;
  }

  /** D-06: "← Назад" — returns from the member list to the group list. */
  function backToGroups() {
    viewMode = 'groups';
    activeGroup = null;
    members = [];
    showBack = false;
    activeIndex = returnIndex;
  }

  /** D-01/D-08: click on an option row in the groups/flat panel. Grouped
   *  mode drills into expandable groups; flat mode (and non-expandable
   *  groups) picks directly. A direct pick closes the panel (Plan 25-07
   *  fix — see handleMemberClick below for the full rationale); drilling in
   *  does not, since it replaces the panel content with the member list. */
  function handleOptionClick(g: TGroup) {
    if (!flat && isGroupExpandable(g)) {
      void drillInto(g);
    } else {
      onPickGroup(g);
      open = false;
    }
  }

  /** Plan 25-07 fix: member-row pick counterpart to handleOptionClick's
   *  direct-pick branch. Neither mouse click nor keyboard Enter previously
   *  closed the panel after a final pick (only Tab/Escape/click-outside
   *  did) — a latent gap in this primitive's first two plans, invisible
   *  until ActFormItemsTable.svelte (Plan 25-07) became its first real
   *  consumer. ActFormItemsTable's pre-migration pickDevice()/pickGroup()
   *  always closed the dropdown unconditionally on pick (`openByRow[idx] =
   *  false`); this restores that parity for both field variants/list modes. */
  function handleMemberClick(m: TMember) {
    onPickMember(m);
    open = false;
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

  /** CR-01: typing MUST (re)open the panel. Pre-migration
   *  ActFormItemsTable.fetchGroups set `openByRow[idx] = true` on every fetch;
   *  Plan 25-07 added `open = false` on pick without restoring that path, so
   *  after the first pick the panel could never reopen (every option row
   *  preventDefaults mousedown, so the field keeps focus and `handleFocus`
   *  never fires again — only ArrowDown recovered it).
   *
   *  This is the one placement that covers BOTH field variants: the combobox
   *  field and the select-variant in-panel search box share this handler.
   *  It cannot regress 25-07's close-on-pick, because a pick never produces
   *  an `input` event — `value` is re-rendered by the caller as a prop, and a
   *  programmatic value change does not fire `oninput`. */
  function handleInput(e: Event) {
    const query = (e.currentTarget as HTMLInputElement).value;
    open = true;
    activeIndex = -1;
    onQueryInput?.(query);
    scheduleSearch(query);
  }

  /** Opens the panel and fires an immediate (non-debounced) onSearch — shared
   *  by AUTO-02 (combobox focus), the select-variant trigger click, and the
   *  ArrowDown-on-closed-panel regression-floor behavior (D-12). */
  function openPanel(query: string) {
    if (searchDebounce) clearTimeout(searchDebounce);
    open = true;
    activeIndex = -1;
    // WR-02: fully reset the drill-in state machine on every (re)open — a
    // manual drill-in that was left mid-flight (panel closed without a pick)
    // must not resurface as a stale member list once the panel reopens. The
    // increment below comes FIRST (mirrors the AUTO-05 effect's own
    // cancel-in-flight branch above): openPanel() becomes a third
    // participant in that same shared counter, so a still-in-flight
    // drillInto promise from before the panel closed is dropped by the
    // existing guard in drillInto/the AUTO-05 effect instead of
    // force-writing over this reset once it resolves.
    expandSeq++;
    viewMode = 'groups';
    activeGroup = null;
    members = [];
    showBack = false;
    onSearch(query);
  }

  /** AUTO-02: panel opens on focus, no typing required — fires onSearch
   *  immediately (delay 0), not through the 250ms debounce. */
  function handleFocus() {
    openPanel(value);
  }

  /** select-variant field click: toggle. Opening fires onSearch('') — the
   *  select field has no typed query of its own, the in-panel search box
   *  drives filtering instead. */
  function toggleSelectOpen() {
    if (open) {
      open = false;
    } else {
      openPanel('');
    }
  }

  /** D-12 `aria-activedescendant` target: id of the option row at
   *  `activeIndex` in whichever list (`groups`/`members`) is currently
   *  visible. Options are portaled out of this component's own DOM subtree,
   *  so ids (not array position) are the only way to reference them. */
  function activeOptionId(): string | undefined {
    if (activeIndex < 0) return undefined;
    if (flat || viewMode === 'groups') {
      if (activeIndex >= groups.length) return undefined;
      return `${uid}-opt-${getGroupId(groups[activeIndex])}`;
    }
    if (activeIndex >= members.length) return undefined;
    return `${uid}-opt-${getMemberId(members[activeIndex])}`;
  }

  /** D-12: scrolls the newly-active option into view after keyboard nav. */
  function scrollActiveIntoView() {
    const id = activeOptionId();
    if (!id) return;
    document.getElementById(id)?.scrollIntoView({ block: 'nearest' });
  }

  function handleKeydown(e: KeyboardEvent) {
    // Regression floor: ArrowDown on a closed panel opens it.
    if (e.key === 'ArrowDown' && !open) {
      e.preventDefault();
      openPanel(variant === 'combobox' ? value : '');
      return;
    }
    if (!open) return;

    const inGroupsView = flat || viewMode === 'groups';

    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      // D-12: two-stage Escape in member-view — first press returns to the
      // group list, but only when there IS a group list to return to
      // (manual drill-in, showBack=true). AUTO-05's auto-flattened
      // single-group view has nowhere to go back to (showBack=false) and
      // closes immediately, same as groups-view (regression floor).
      if (!inGroupsView && showBack) {
        backToGroups();
      } else {
        open = false;
      }
      return;
    }

    if (inGroupsView) {
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        if (groups.length === 0) return;
        activeIndex = activeIndex < 0 ? 0 : (activeIndex + 1) % groups.length;
        scrollActiveIntoView();
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        if (groups.length === 0) return;
        activeIndex = activeIndex <= 0 ? groups.length - 1 : activeIndex - 1;
        scrollActiveIntoView();
      } else if (e.key === 'Home') {
        e.preventDefault();
        if (groups.length === 0) return;
        activeIndex = 0;
        scrollActiveIntoView();
      } else if (e.key === 'End') {
        e.preventDefault();
        if (groups.length === 0) return;
        activeIndex = groups.length - 1;
        scrollActiveIntoView();
      } else if (e.key === 'Enter') {
        if (activeIndex >= 0 && activeIndex < groups.length) {
          e.preventDefault();
          e.stopPropagation();
          handleOptionClick(groups[activeIndex]);
        }
      } else if (e.key === 'Tab') {
        // WR-01: Tab must never both start an async drillInto AND
        // synchronously close the panel — that silently loses the pick and
        // primes the component to show a stale drilled-in list on next open
        // (WR-02). Commit directly via onPickGroup only for a non-expandable
        // group; an expandable group (or no active option) just closes, same
        // as the groups-view Escape behavior — "closing wins, no partial
        // navigation state left behind." The `g &&` guard (evaluated before
        // isGroupExpandable(g)) preserves the old bounds check: focusing the
        // field runs AUTO-02 -> openPanel() -> activeIndex = -1, so an
        // immediate Tab with no arrow key first would otherwise read
        // groups[-1] === undefined and crash both production consumers'
        // isExpandable/isGroupExpandable calls.
        const g = groups[activeIndex];
        if (g && !(!flat && isGroupExpandable(g))) {
          onPickGroup(g);
        }
        open = false;
      }
      return;
    }

    // Member-view (drill-in) keyboard nav — net-new per D-12 (previously
    // mouse-only navigation, per UI-SPEC's correction of CONTEXT.md).
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (members.length === 0) return;
      activeIndex = activeIndex < 0 ? 0 : (activeIndex + 1) % members.length;
      scrollActiveIntoView();
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      if (members.length === 0) return;
      activeIndex = activeIndex <= 0 ? members.length - 1 : activeIndex - 1;
      scrollActiveIntoView();
    } else if (e.key === 'Home') {
      e.preventDefault();
      if (members.length === 0) return;
      activeIndex = 0;
      scrollActiveIntoView();
    } else if (e.key === 'End') {
      e.preventDefault();
      if (members.length === 0) return;
      activeIndex = members.length - 1;
      scrollActiveIntoView();
    } else if (e.key === 'Enter') {
      // WR-02: Enter must never bubble to a host <form> submit — suppressed
      // unconditionally (the pre-existing regression floor). D-12 adds the
      // pick action on top of that suppression; the suppression itself is
      // not new.
      e.preventDefault();
      e.stopPropagation();
      if (activeIndex >= 0 && activeIndex < members.length) {
        handleMemberClick(members[activeIndex]);
      }
    } else if (e.key === 'Tab') {
      if (activeIndex >= 0 && activeIndex < members.length) {
        onPickMember(members[activeIndex]);
      }
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
    const insideField =
      (inputEl?.contains(target) ?? false) || (triggerEl?.contains(target) ?? false);
    const insideDropdown = panelEl?.contains(target) ?? false;
    if (!insideField && !insideDropdown) open = false;
  }

  $effect(() => {
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  });
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
      role="combobox"
      aria-autocomplete="list"
      aria-expanded={open}
      aria-controls={panelId}
      aria-haspopup="listbox"
      aria-activedescendant={activeOptionId()}
      oninput={handleInput}
      onfocus={handleFocus}
      onkeydown={handleKeydown}
    />
  {:else}
    <!-- D-03/No Analog Found (PATTERNS.md): select-variant field — value
         display + trailing arrow, WAI-ARIA "select-only combobox" pattern
         (button-based trigger, not directly editable). -->
    <button
      type="button"
      bind:this={triggerEl}
      class="tr-dropdown-field-button"
      class:invalid
      {disabled}
      role="combobox"
      aria-expanded={open}
      aria-controls={panelId}
      aria-haspopup="listbox"
      aria-activedescendant={activeOptionId()}
      onclick={toggleSelectOpen}
      onkeydown={handleKeydown}
    >
      <span class="tr-dropdown-field-value" class:placeholder={!value}>{value || placeholder}</span>
      <span class="tr-dropdown-field-arrow" aria-hidden="true">▼</span>
    </button>
  {/if}

  {#if open}
    <ul
      class="tr-dropdown-panel"
      class:tr-dropdown-panel--flat={flat}
      role="listbox"
      id={panelId}
      use:portal
      use:dropdownAnchor={{ anchorEl: inputEl ?? triggerEl, maxHeight: flat ? 240 : 280 }}
      bind:this={panelEl}
    >
      {#if variant === 'select'}
        <!-- D-03/UI-SPEC "Dropdown — две формы": in-panel search box, the
             first child of the panel (before drill-in header or options). -->
        <li class="tr-dropdown-search">
          <span class="tr-dropdown-search-box">
            <span class="tr-dropdown-search-icon" aria-hidden="true">⌕</span>
            <input
              type="text"
              class="tr-dropdown-search-input"
              aria-label="Поиск"
              placeholder={searchPlaceholder}
              aria-activedescendant={activeOptionId()}
              aria-controls={panelId}
              oninput={handleInput}
              onkeydown={handleKeydown}
            />
          </span>
        </li>
      {/if}
      {#if !flat && viewMode === 'members'}
        <!-- D-01/D-06 drill-in header — checkpoint fix #1 (UI-SPEC): title is
             ALWAYS shown in member-view; "← Назад" only on manual drill-in
             (showBack), not on AUTO-05 auto-flatten. Two independent
             conditions, not one boolean. -->
        <li
          class="tr-dropdown-drill-header"
          class:tr-dropdown-drill-header--offset={variant === 'select'}
        >
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
          {#each members as m, i (getMemberId(m))}
            <li>
              <button
                type="button"
                id={`${uid}-opt-${getMemberId(m)}`}
                class="tr-dropdown-option"
                class:active={i === activeIndex}
                role="option"
                aria-selected={i === activeIndex}
                onmousedown={(e) => e.preventDefault()}
                onclick={() => handleMemberClick(m)}
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
              id={`${uid}-opt-${getGroupId(g)}`}
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

  // D-03/No Analog Found: select-variant trigger — value display + arrow,
  // WAI-ARIA "select-only combobox" pattern. Shares field geometry with
  // .tr-dropdown-field (h=36px, same surface/border/radius tokens).
  .tr-dropdown-field-button {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
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
    text-align: left;
    cursor: pointer;

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
  .tr-dropdown-field-value {
    flex: 1 1 auto;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;

    &.placeholder {
      color: var(--tr-text-tertiary);
    }
  }
  .tr-dropdown-field-arrow {
    flex: 0 0 auto;
    font-size: 10px;
    color: var(--tr-text-secondary);
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
    // UI-SPEC Checker Sign-Off recommendation: 600 (not a new 700 weight),
    // keeps the typography scale closed to the existing 4 weights.
    font-weight: var(--tr-font-weight-semibold);
  }

  // D-03/UI-SPEC "Dropdown — две формы": select-variant in-panel search box,
  // sticky so it stays visible while the option list scrolls underneath.
  :global(.tr-dropdown-panel .tr-dropdown-search) {
    position: sticky;
    top: 0;
    z-index: 1;
    display: flex;
    align-items: center;
    padding: 6px 12px;
    background: var(--tr-surface-raised);
    border-bottom: 1px solid var(--tr-border);
    list-style: none;
  }
  :global(.tr-dropdown-panel .tr-dropdown-search-box) {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    height: 30px;
    padding: 0 10px;
    background: var(--tr-surface-sunken);
    border-radius: 5px;
  }
  :global(.tr-dropdown-panel .tr-dropdown-search-icon) {
    flex: 0 0 auto;
    font-size: 13px;
    color: var(--tr-text-tertiary);
  }
  :global(.tr-dropdown-panel .tr-dropdown-search-input) {
    flex: 1 1 auto;
    min-width: 0;
    background: transparent;
    border: none;
    outline: none;
    color: var(--tr-text-primary);
    font-family: var(--tr-font-family);
    font-size: var(--tr-font-size-label);

    &::placeholder {
      color: var(--tr-text-tertiary);
    }
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
  // select variant + grouped (non-flat) drill-in: the in-panel search box
  // (42px, .tr-dropdown-search) is also sticky at top:0 — without this
  // offset the two sticky headers would overlap once scrolled. This combo
  // isn't in the Showcase Contract's two canonical examples but is a valid
  // point in the variant×flat prop matrix, so it must not visually break.
  :global(.tr-dropdown-panel .tr-dropdown-drill-header--offset) {
    top: 42px;
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

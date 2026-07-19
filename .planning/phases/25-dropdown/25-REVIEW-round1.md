---
phase: 25-dropdown
reviewed: 2026-07-19T00:00:00Z
depth: standard
files_reviewed: 11
files_reviewed_list:
  - ui/src/lib/components/Table.svelte
  - ui/src/lib/components/TableRow.svelte
  - ui/src/lib/components/Dropdown.svelte
  - ui/src/styles/_tokens.scss
  - ui/src/features/devices/DeviceList.svelte
  - ui/src/features/devices/DeviceListRow.svelte
  - ui/src/features/devices/DeviceGroupRow.svelte
  - ui/src/features/acts/ActFormItemsTable.svelte
  - ui/src/features/showcase/ShowcasePage.svelte
  - ui/src/features/showcase/sections/TableSection.svelte
  - ui/src/features/showcase/sections/DropdownSection.svelte
findings:
  critical: 2
  warning: 14
  info: 6
  total: 22
status: issues_found
---

# Phase 25: Code Review Report

**Reviewed:** 2026-07-19
**Depth:** standard
**Files Reviewed:** 11
**Status:** issues_found

## Summary

Two design-system primitives (`Table`/`TableRow`, `Dropdown`) plus two production migrations.
The `Table`/`TableRow` work is largely sound — the `:global()` specificity reasoning in
`TableRow.svelte:100` and `DeviceListRow.svelte:80` was verified by hand and is correct
(`.tr-row.hash > td` = 0,2,1 loses to `tr.group-last-child > .cell.hash` = 0,3,1).

`Dropdown.svelte` is where the defects concentrate. The Plan 25-07 "close panel on pick"
patch was applied without a matching "reopen panel on typed input" path, which the code it
replaced (`ActFormItemsTable.fetchGroups` → `openByRow[idx] = true`) had. The result is a
picker that goes permanently dark after the first pick in the Acts form. Separately, the
AUTO-05 auto-flatten `$effect` performs an unguarded async write, which can render a stale
member list under a newer query. Both were confirmed by diffing against `3b44a0f`.

Secondary themes: the combobox ARIA layer (a stated phase requirement) has an invalid
listbox/option DOM structure; the select-variant in-panel search box is outside the keyboard
layer entirely; and several behaviours present pre-migration (drill-in loading indicator,
synchronous view-mode reset on keystroke, group-name tooltip) were dropped silently.

`svelte-check` passes with 0 errors; it flags one dead CSS selector in the phase's scope
(`.hint-warn`, confirmed below).

## Narrative Findings (AI reviewer)

## Critical Issues

### CR-01: Dropdown panel can never reopen after a pick — Acts device picker dies after first selection

**File:** `ui/src/lib/components/Dropdown.svelte:236-240, 245-256, 200-220`
**Issue:** `open` is set to `true` in exactly three places: `handleFocus`, `toggleSelectOpen`,
and the `ArrowDown`-on-closed branch of `handleKeydown`. `handleInput` (typing) never opens
the panel — it only fires `onQueryInput` and schedules the debounced `onSearch`.

Plan 25-07 added `open = false` to `handleOptionClick`'s direct-pick branch (line 205) and
`handleMemberClick` (line 219). Every option button carries `onmousedown={(e) => e.preventDefault()}`
(lines 517, 548), so focus never leaves the field on a pick. Therefore after a pick:

1. `open === false`, input is still focused.
2. The user types to search for the next device → `handleInput` → `onSearch` fires →
   `ActFormItemsTable.fetchGroups` populates `suggestionsByRow[idx]` → **panel stays closed.**
3. No `focus` event will ever fire again (the element is already focused), so `handleFocus`
   cannot recover it.

Only `ArrowDown` reopens the panel. The same dead-end is reachable via `Escape`.

This is a direct regression: the pre-migration code at `3b44a0f:ActFormItemsTable.svelte:208`
set `openByRow[idx] = true` inside `fetchGroups` — on *every* fetch, including the debounced
typed-input path and the error path. The migration dropped that line and did not replace it.
`DeviceList*` is unaffected (no Dropdown); this breaks the Acts form per-row device picker.

**Fix:**
```ts
function handleInput(e: Event) {
  const query = (e.currentTarget as HTMLInputElement).value;
  open = true;              // regain the pre-migration `openByRow[idx] = true` behaviour
  activeIndex = -1;
  onQueryInput?.(query);
  scheduleSearch(query);
}
```
(Equivalently, set `open = true` inside `scheduleSearch`'s timeout callback so it also covers
the select-variant in-panel search input.)

### CR-02: Unguarded async write in the AUTO-05 auto-flatten `$effect` renders a stale member list

**File:** `ui/src/lib/components/Dropdown.svelte:147-169`
**Issue:** The effect starts a fire-and-forget async IIFE that `await`s `onExpandGroup(only)`
and then unconditionally writes `activeGroup`, `members`, `viewMode`, `showBack`, `activeIndex`.
There is no generation token, no abort, and no re-check that `groups` still holds that single
group when the promise resolves.

Sequence that corrupts state (reachable by normal typing in the Acts form, where
`onExpandGroup` performs a real `devices.listByIds` IPC round-trip):

1. Query A narrows to one group → effect fires, `onExpandGroup(A)` in flight.
2. User keeps typing; query B returns 3 groups → effect re-runs, takes the `else` branch,
   resets to `viewMode = 'groups'`.
3. `onExpandGroup(A)` resolves → overwrites with `viewMode = 'members'`, `members = A's devices`,
   `activeGroup = A`.

The panel now shows group A's instances with A's title while the caller's filter is B. Clicking
one calls `onPickMember` → `pickDevice` → writes a `device_id` that does not match what the user
searched for, and (via DEF-2A dedup performed against the *old* selection snapshot) can pick an
id already claimed by another row. This is a wrong-data write, not a cosmetic glitch.

`drillInto` (line 174-184) has the same unguarded shape: if the panel is closed or `groups`
changes between the click and the resolve, it still forces `viewMode = 'members'`.

**Fix:**
```ts
let expandSeq = 0;

$effect(() => {
  if (flat) return;
  const list = groups;
  if (list.length === 1) {
    const only = list[0];
    const seq = ++expandSeq;
    void (async () => {
      const result = await onExpandGroup(only);
      if (seq !== expandSeq) return;   // superseded — drop the stale result
      activeGroup = only;
      members = result;
      viewMode = 'members';
      showBack = false;
      activeIndex = result.length > 0 ? 0 : -1;
    })();
  } else {
    expandSeq++;                        // cancel any in-flight expand
    viewMode = 'groups';
    activeGroup = null;
    members = [];
    showBack = false;
    activeIndex = -1;
  }
});
```
Apply the same `seq` guard in `drillInto`.

## Warnings

### WR-01: `Tab` on an expandable group drills in *and* closes the panel — pick is lost

**File:** `ui/src/lib/components/Dropdown.svelte:344-349`
**Issue:** The `Tab` branch in groups-view calls `handleOptionClick(groups[activeIndex])` and
then unconditionally `open = false`. For an expandable group `handleOptionClick` takes the
`drillInto` branch (line 201-202), which fires a fetch and — once it resolves — sets
`viewMode = 'members'` on an already-closed panel. Net effect of `Tab`: nothing is picked, a
wasted IPC call is made, and the component is left in member-view so the *next* open shows a
stale drill-in (see WR-03).
**Fix:** In the `Tab` branch, only commit non-expandable groups:
```ts
} else if (e.key === 'Tab') {
  const g = groups[activeIndex];
  if (g && !(!flat && isGroupExpandable(g))) onPickGroup(g);
  open = false;
}
```

### WR-02: Closing the panel does not reset drill-in state

**File:** `ui/src/lib/components/Dropdown.svelte:245-250, 301-315, 200-220`
**Issue:** `open = false` (Escape, pick, Tab, click-outside) leaves `viewMode`, `activeGroup`,
`members` and `showBack` untouched, and `openPanel` resets only `activeIndex`. If `groups` has
not changed identity since (the `$effect` at line 147 is the *only* thing that resets view mode),
reopening the panel drops the user straight back into a previously drilled-in member list, with
a `drillTitle` for a group that may no longer be in `groups`.
**Fix:** Reset the drill-in machine inside `openPanel`:
```ts
function openPanel(query: string) {
  if (searchDebounce) clearTimeout(searchDebounce);
  open = true;
  activeIndex = -1;
  viewMode = 'groups';
  activeGroup = null;
  members = [];
  showBack = false;
  onSearch(query);
}
```

### WR-03: Migration loss — view-mode no longer resets synchronously on keystroke

**File:** `ui/src/lib/components/Dropdown.svelte:236-240` vs `3b44a0f:ActFormItemsTable.svelte:159-165`
**Issue:** The pre-migration `handleQueryInput` reset `viewModeByRow/drillGroupByRow/membersByRow/
showBackByRow` **synchronously on every keystroke**, explicitly citing the UI-SPEC rule
«изменение текста фильтра сбрасывает view-mode строки обратно к списку групп». The new Dropdown
resets view mode only as a side effect of `groups` changing, i.e. after the 250 ms debounce plus
the IPC round-trip. For that entire window the drilled-in member list of the *previous* query
stays visible and clickable.
**Fix:** Reset the drill-in state at the top of `handleInput`, not only in the `$effect`.

### WR-04: Migration loss — no loading indicator during drill-in fetch

**File:** `ui/src/features/acts/ActFormItemsTable.svelte:210-218` vs `3b44a0f:ActFormItemsTable.svelte:355-366`
**Issue:** The old `drillInto` wrapped `devices.listByIds(ids)` in `loadingByRow[idx] = true` /
`finally { loadingByRow[idx] = false }`. The new `expandGroup` does neither, and `Dropdown`'s
`loading` prop is caller-driven — so during a manual drill-in the panel shows the previous
content frozen with no feedback. The `{#if loading}` branch at `Dropdown.svelte:503` is now
unreachable for the drill-in path in this consumer.
**Fix:**
```ts
async function expandGroup(idx: number, g: DeviceGroup): Promise<MemberRow[]> {
  const selectedIds = getSelectedIds(idx);
  const ids = g.ids.filter((id) => !selectedIds.has(id));
  loadingByRow[idx] = true;
  try {
    return partitionMembers(await devices.listByIds(ids));
  } catch {
    return [];
  } finally {
    loadingByRow[idx] = false;
  }
}
```

### WR-05: Invalid listbox/option DOM structure breaks `aria-activedescendant`

**File:** `ui/src/lib/components/Dropdown.svelte:457-576`
**Issue:** `<ul role="listbox">` has these children: bare `<li>` wrappers (509, 539) that carry
no role and whose *child* `<button role="option">` holds the option semantics, plus
`<li class="tr-dropdown-search">` (469) and `<li class="tr-dropdown-drill-header">` (486). Per
WAI-ARIA, a `listbox`'s owned elements must be `option` or `group`; a role-less `li` is not a
valid intermediary, and the two chrome `<li>`s are announced as unlabeled list items inside the
listbox. `aria-activedescendant` (428, 447) therefore points at elements a screen reader does not
consider owned options. Accessibility of this layer is a stated phase requirement, not incidental.
**Fix:** Give each wrapper `role="presentation"` and each chrome row `role="presentation"` too
(or drop the `ul`/`li` and render `<div role="listbox">` with `<div role="option">` children):
```svelte
<li role="presentation">
  <button role="option" id={...} ...>
```

### WR-06: Select-variant in-panel search box is outside the keyboard layer

**File:** `ui/src/lib/components/Dropdown.svelte:472-478`
**Issue:** The in-panel `<input class="tr-dropdown-search-input">` has `oninput={handleInput}`
but no `onkeydown={handleKeydown}`. Clicking it moves focus off the trigger button (no
`onmousedown` preventDefault on this row), so from that point `Escape`, `ArrowUp/Down`, `Home/End`,
`Enter` and `Tab` do nothing — the entire D-12 keyboard layer is dead exactly where a select-variant
user does their typing. The panel can then only be dismissed by clicking outside.
**Fix:** Add `onkeydown={handleKeydown}` to the search input (and mirror `aria-activedescendant`/
`aria-controls` onto it, since it now owns focus).

### WR-07: `handleInput` fires `onQueryInput` from the select-variant search box

**File:** `ui/src/lib/components/Dropdown.svelte:236-240, 477`
**Issue:** `onQueryInput` is documented (lines 61-64) as the combobox controlled-input sync hook
— consumers wire it to overwrite their own `value`. Reusing `handleInput` for the select variant's
in-panel search box means a select consumer that supplies `onQueryInput` will have its *displayed
value* clobbered by search keystrokes. Additionally the search input is uncontrolled (no
`bind:value`), so its text survives a close/reopen while `openPanel('')` resets the caller's
filter — displayed query and displayed results disagree.
**Fix:** Split the handlers — a `handleSearchInput` for the panel box that calls only
`scheduleSearch`, and reset its value when the panel opens.

### WR-08: `aria-selected` misused as "active" in grouped mode

**File:** `ui/src/lib/components/Dropdown.svelte:516, 547`
**Issue:** In member mode and non-flat group mode, `aria-selected={i === activeIndex}` reports
keyboard-focus position as selection state. With `aria-activedescendant` already conveying the
active option, this announces "selected" for an option the user has merely arrowed onto. Flat
mode gets it right (`aria-selected={!!isGroupSelected?.(g)}`).
**Fix:** Use `aria-selected={false}` (or omit) for non-flat options and rely on
`aria-activedescendant` + the `.active` class for the visual/AT active state.

### WR-09: `DeviceGroupRow` `$effect` can retry a failing fetch forever

**File:** `ui/src/features/devices/DeviceGroupRow.svelte:109-129`
**Issue:** The effect's guard is `expanded && children === null && !loadingChildren`. On failure
`children` stays `null` and `loadingChildren` flips back to `false` — which is a tracked
dependency — so the effect re-runs and refetches. The only brake is
`onExpandToggle?.(stableKey, false)`, an **optional** prop (line 35). `DevicesPage` supplies it
today, but any other caller (or a parent that debounces/ignores the callback) gets an unbounded
retry loop with one error toast per iteration. Pre-existing code, but it is now in a component
being promoted onto shared primitives and is one optional prop away from a live incident.
**Fix:** Track failure explicitly:
```ts
let childrenError = $state(false);
$effect(() => {
  if (expanded && children === null && !loadingChildren && !childrenError) { ... }
});
// in .catch: childrenError = true;
// reset childrenError in toggleExpand() when the user re-expands
```

### WR-10: Migration loss — group-name tooltip dropped

**File:** `ui/src/features/devices/DeviceGroupRow.svelte:145-151`
**Issue:** The pre-migration group row had `title={group.repr.name}` on the merged name cell
(`3b44a0f:DeviceGroupRow.svelte:147`). `TableRow`'s group mode renders that `<td>` itself
(`TableRow.svelte:56-67`) and exposes no way to set `title`, so long group names now truncate
with no hover text — inconsistent with every other cell on the screen, all of which still carry
`title=`.
**Fix:** Add an optional `groupTitle?: string` prop to `TableRow` and forward it to the name
`<td>`; pass `groupTitle={group.repr.name}` from `DeviceGroupRow`.

### WR-11: `TableRow` silently ignores half its props in group mode

**File:** `ui/src/lib/components/TableRow.svelte:54-74`
**Issue:** When `group` is true, `selected`, `indent` and `last` are accepted, destructured, and
never applied — the group `<tr>` gets neither `class:selected`, `class:indent` nor `class:last`.
A consumer writing `<TableRow group last>` for the final group of a table gets no error and no
effect. Conversely `groupExpanded`/`groupName`/`groupColspan`/`onToggleGroup` are silently inert
in normal mode.
**Fix:** Either apply the shared state classes in both branches, or split the two modes into
separate components / narrow the `Props` type with a discriminated union on `group`.

### WR-12: Duplicate-key risk in the Acts picker option list

**File:** `ui/src/features/acts/ActFormItemsTable.svelte:416` with `Dropdown.svelte:538`
**Issue:** `getGroupId={(g: DeviceGroup) => g.repr.id}` is used as the `{#each ... (key)}` key,
while `fetchGroups` requests `group_by_condition: true` — i.e. one logical device group is split
into several rows by `condition`. If the backend can pick the same representative row id for two
condition-split groups, Svelte raises a runtime duplicate-key error and the panel fails to render.
The old code used no keyed each here, so this is newly load-bearing.
**Fix:** Verify against `list_grouped` SQL; if repr ids are not guaranteed unique across
condition splits, key on a composite:
`getGroupId={(g) => `${g.repr.id}:${g.repr.state ?? ''}`}`.

### WR-13: `openPanel` discards the auto-flatten's initial active index

**File:** `ui/src/lib/components/Dropdown.svelte:245-250` vs `147-169`
**Issue:** The AUTO-05 branch deliberately sets `activeIndex = result.length > 0 ? 0 : -1`
("entering member-view activates the first option", line 158-160). `openPanel` then
unconditionally resets `activeIndex = -1`. Whether the first option is pre-activated depends on
the arrival order of the effect vs the open — a race, not a rule.
**Fix:** Decide one rule and enforce it in a single place; simplest is for `openPanel` to reset
view mode (WR-02) and let the effect re-derive `activeIndex`.

### WR-14: Non-conforming DOM `id`s in `aria-activedescendant`

**File:** `ui/src/lib/components/Dropdown.svelte:512, 542, 277, 280` with `ActFormItemsTable.svelte:201`
**Issue:** Option ids are `${uid}-opt-${getMemberId(m)}`. In the Acts consumer, subgroup member
keys are `sg-${state ?? '_'}` where `state` is free-text DB data («Требует ремонта», etc.), so ids
contain spaces and arbitrary characters. These are fed to `aria-activedescendant` and
`document.getElementById`. `getElementById` tolerates it, but the value is not a valid HTML `id`
and would break any future `querySelector('#…')`/CSS use, and some AT implementations resolve
`aria-activedescendant` via a selector.
**Fix:** Sanitise in `Dropdown`: `const safe = String(id).replace(/[^A-Za-z0-9_-]/g, '_')`, or
have `ActFormItemsTable` key subgroups by index rather than by state text.

## Info

### IN-01: Dead CSS — `.hint-warn`

**File:** `ui/src/features/acts/ActFormItemsTable.svelte:601-605`
**Issue:** Confirmed by `svelte-check`: `Warn: Unused CSS selector ".hint-warn"`. Leftover from
a removed markup block.
**Fix:** Delete the rule.

### IN-02: Dead CSS — `.col-device { position: relative }`

**File:** `ui/src/features/acts/ActFormItemsTable.svelte:515-517`
**Issue:** Was the positioning context for the hand-rolled absolute dropdown removed by this
migration. The panel is now portaled to `<body>` with `position: fixed`, so this no longer does
anything for the Dropdown (`.loading-row` at line 526 still relies on it, so the rule cannot just
be deleted — but the comment/intent is now stale).
**Fix:** Move the positioning context comment onto `.loading-row`'s requirement, or drop
`.loading-row` per IN-03 and then remove this rule.

### IN-03: Duplicated loading indicator in the Acts picker

**File:** `ui/src/features/acts/ActFormItemsTable.svelte:432-434`
**Issue:** While `loadingByRow[idx]` is true, the user sees both `Dropdown`'s own
`«Загрузка…» + Spinner` panel row (`Dropdown.svelte:534`) and this overlay `Spinner` pinned to the
field. The overlay was the pre-Dropdown mechanism and is now redundant.
**Fix:** Remove the `.loading-row` block and its styles; `Dropdown` owns the loading affordance
(D-13).

### IN-04: Dead condition in `DeviceList.isEmpty`

**File:** `ui/src/features/devices/DeviceList.svelte:41-42`
**Issue:** `showGroups` already requires `groups.length > 0`, so the ternary's true-branch
`groups.length === 0` is unreachable — when there are no groups, `showGroups` is false and the
`items.length === 0` branch runs. Reads as a guard that does nothing.
**Fix:** `const isEmpty = $derived(!loading && !showGroups && items.length === 0);`

### IN-05: Showcase misrepresents the production Devices table

**File:** `ui/src/features/showcase/sections/TableSection.svelte:227` vs `DeviceGroupRow.svelte:175-182`
**Issue:** The showcase renders group children with `<TableRow indent …>`, but the real Devices
screen renders them via `DeviceListRow`, which never passes `indent`. `indent` is used *only* in
the showcase (grep confirms 1 call site), so the gallery advertises a row state no production
screen uses and shows a group block that doesn't match the actual screen.
**Fix:** Either pass `indent` through `DeviceListRow` for group children (matching the showcase),
or drop the `indent` demo — but the two should agree.

### IN-06: DropdownSection forces four portaled panels open at once, and its checkmark demo is inert

**File:** `ui/src/features/showcase/sections/DropdownSection.svelte:73-102, 41-45, 149-154`
**Issue:** Demo-only, weighted low. Two points: (a) `onMount` opens all four dropdowns via
synthetic `focus()`/`click()`; nothing closes them, and since each panel is `position: fixed` and
portaled to `<body>`, four panels overlay the page simultaneously and follow their anchors on
scroll — the gallery below them is obscured. (b) `isGroupSelected={(g) => !!g.selected}` reads a
hard-coded flag on the `const flatOptions` array while `onPickGroup` writes `flatValue`, so
picking a different option moves the field text but leaves the checkmark on «В работе» —
the demo displays a self-contradictory state.
**Fix:** (a) Open one block at a time (or gate the sequence behind a "показать открытым" toggle);
(b) drive `selected` from `flatValue`: `isGroupSelected={(g) => g.name === flatValue}`.

## Verified-Correct Notes (no action)

- `TableRow.svelte:100` / `DeviceListRow.svelte:80` specificity claims hold:
  `tr.group-last-child > .cell.svelte-hash` (0,3,1) beats `.tr-row.svelte-hash > td` (0,2,1).
- `DeviceList.svelte:78` `columns={showStatus ? 8 : 7}` matches the `tableHead` snippet's
  conditional `<th>` count and `DeviceGroupRow`'s `colspan={showStatus ? 8 : 7}`.
- `DeviceGroupRow` `groupColspan={4}` + 3/4 trailing `<td>`s totals 7/8 — consistent.
- `--tr-group` added to both light and dark blocks of `_tokens.scss`; consumed only by
  `TableRow.tr-row-group`. `.tr-row.svelte-hash:hover` (3 classes) still beats
  `.tr-row-group.svelte-hash` (2 classes), so group-row hover works.
- `Dropdown`'s `onDestroy` clears `searchDebounce`, and the click-outside `$effect` returns its
  `removeEventListener` cleanup — no listener/timer leak found. `portal`'s `destroy()` removes
  the node from `<body>`, so the `{#if open}` teardown does not orphan the panel.

---

_Reviewed: 2026-07-19_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_

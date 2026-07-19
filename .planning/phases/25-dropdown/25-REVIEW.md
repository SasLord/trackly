---
phase: 25-dropdown
reviewed: 2026-07-19T00:00:00Z
depth: standard
round: 2
files_reviewed: 1
files_reviewed_list:
  - ui/src/lib/components/Dropdown.svelte
findings:
  critical: 1
  warning: 7
  info: 3
  total: 11
status: issues_found
---

# Phase 25: Code Review Report (round 2 — gap closure)

**Reviewed:** 2026-07-19
**Depth:** standard
**Files Reviewed:** 1 (`ui/src/lib/components/Dropdown.svelte`)
**Status:** issues_found

## Summary

Round-2 scope was plan 25-08 (commits `09c3f8c`, `2d48bea`), which claimed to close round-1's
WR-01, WR-02 and WR-06 in `Dropdown.svelte`. Verdict:

- **WR-01 (Tab drills in + closes): fixed correctly.** The `g &&` null guard is ordered before
  `isGroupExpandable(g)`, so the `groups[-1] === undefined` crash path round 1 flagged is
  preserved-safe. Flat mode still commits via `onPickGroup`, matching `handleOptionClick`.
- **WR-06 (search input outside keyboard layer): partially fixed.** `onkeydown` is wired and the
  ARIA attributes are present, but the fix stops at the point where it becomes usable — closing
  the panel from the search input destroys the focused element and drops focus to `<body>`
  (WR-03 below), and the focused element still carries no combobox role (WR-05).
- **WR-02 (drill-in state not reset on close): NOT fixed — only half the reopen surface was
  patched.** `openPanel()` resets the state machine, but `handleInput()` — the *other* path that
  sets `open = true`, added in round 1 to fix CR-01 — does not. The exact defect WR-02 described
  is still reachable, via the most common interaction of all (typing). This is BL-01.

Regression check on round-1 criticals: **CR-01 intact** (`handleInput` still sets `open = true`,
line 276). **CR-02 intact** (generation guards present at lines 172 and 201; `expandSeq` correctly
remains a plain `let`). However the WR-02 patch made `openPanel()` a third writer of `expandSeq`,
which introduces a new way to permanently lose an AUTO-05 auto-flatten (WR-01 below).

Verified against consumers: `ActFormItemsTable.svelte` is the only production consumer
(combobox, non-flat); `DropdownSection.svelte` is showcase-only. `ActFormBody.svelte:260`'s
`onsubmit` unconditionally `preventDefault()`s, which is what keeps WR-02 below out of
BLOCKER territory today.

## Narrative Findings (AI reviewer)

## Critical Issues

### BL-01: WR-02 is not fixed — typing still reopens the panel straight into a stale drilled-in member list

**File:** `ui/src/lib/components/Dropdown.svelte:274-280` (vs the patched `285-304`)
**Issue:** `open` is set to `true` in exactly two code paths: `openPanel()` (line 287) and
`handleInput()` (line 276). Commit `09c3f8c` added the drill-in reset and the `expandSeq++`
cancel to `openPanel()` only. `handleInput()` sets `open = true` and `activeIndex = -1` and
nothing else — `viewMode`, `activeGroup`, `members` and `showBack` are untouched, and the
generation token is not bumped.

Reproduction (combobox variant, Acts per-row device picker):

1. User drills into an expandable group → `viewMode = 'members'`, `showBack = true`,
   `members = [A1, A2, A3]`.
2. User dismisses the panel without picking — click-outside (line 470), `Escape` (line 366),
   or a `Tab` on an expandable group (line 415, the new WR-01 path). All of these set
   `open = false` and leave the drill-in state intact by design.
3. User types to search for something else → `handleInput` → `open = true`.
4. The panel re-renders **immediately** into the member view of the *previous* group: the
   `{#if !flat && viewMode === 'members'}` branch at line 552 is still true, `drillTitle` still
   names the old group, and `members` is still the old list — all clickable.

The stale list stays live for the full 250 ms debounce plus the IPC round-trip, and a click in
that window calls `onPickMember` → `pickMember(idx, m)` → writes a `device_id` from the previous
query. That is the same wrong-data-write class as round-1 CR-02, and it is why WR-02 was filed.
The state only clears once `groups` changes identity and the AUTO-05 `$effect` runs; a consumer
that memoizes or short-circuits an identical query never clears it at all.

Additionally, step 2 does not bump `expandSeq`, so a `drillInto` promise still in flight from
before the dismissal passes the `seq !== expandSeq` guard at line 201 and force-writes
`viewMode = 'members'` onto the reopened panel — the exact scenario the new `openPanel()` comment
(lines 289-297) claims is handled. It is handled for `openPanel()`, not for `handleInput()`.

**Fix:** Factor the reset out and call it from both entry points.

```ts
/** Shared by openPanel() and handleInput() — every path that transitions the
 *  panel to open must land in the groups view with no in-flight expand. */
function resetDrillState() {
  expandSeq++;
  viewMode = 'groups';
  activeGroup = null;
  members = [];
  showBack = false;
}

function handleInput(e: Event) {
  const query = (e.currentTarget as HTMLInputElement).value;
  open = true;
  activeIndex = -1;
  resetDrillState();   // BL-01 — was only in openPanel()
  onQueryInput?.(query);
  scheduleSearch(query);
}

function openPanel(query: string) {
  if (searchDebounce) clearTimeout(searchDebounce);
  open = true;
  activeIndex = -1;
  resetDrillState();
  onSearch(query);
}
```

Resetting on *every* keystroke (not just on the open transition) also closes round-1 WR-03
— UI-SPEC's «изменение текста фильтра сбрасывает view-mode строки обратно к списку групп»,
which the pre-migration `handleQueryInput` did synchronously and this component still does not.

## Warnings

### WR-01: `openPanel()`'s new `expandSeq++` can permanently discard an AUTO-05 auto-flatten

**File:** `ui/src/lib/components/Dropdown.svelte:298-302` with `162-190`
**Issue:** New in this round. `openPanel()` now increments `expandSeq` and force-writes
`viewMode = 'groups'` / `members = []`. The AUTO-05 `$effect` that produces the auto-flatten is
keyed on `groups` (and `flat`) — reopening the panel does not re-run it. So:

- Any AUTO-05 expand in flight when the panel is (re)opened is silently dropped and never retried.
- If `groups` still holds exactly one group at reopen, the panel now shows that group as a
  collapsed row instead of its auto-flattened members, contradicting AUTO-05's stated rule.

`ActFormItemsTable` self-heals because `fetchGroups` (line 161) unconditionally assigns a fresh
array, re-triggering the effect — but the user still sees a members → group-row → members flicker
one IPC round-trip wide on every reopen. A consumer with a static or memoized `groups` (the
showcase pattern, or any client-side-filtered list) never recovers.
**Fix:** Do not have `openPanel()` decide the view mode independently of the state machine that
owns it. Either re-derive after reset:

```ts
function resetDrillState() {
  expandSeq++;
  viewMode = 'groups';
  activeGroup = null;
  members = [];
  showBack = false;
  autoFlattenTick++;      // $state counter also read by the AUTO-05 $effect,
}                         // so reopening re-evaluates the single-group rule
```

…or restrict the reset to manual drill-ins only (`if (showBack) { ... }`), leaving an
auto-flattened view — which by definition matches the current `groups` — in place.

### WR-02: groups-view `Enter` with no active option neither preventDefaults nor stops propagation

**File:** `ui/src/lib/components/Dropdown.svelte:392-397` vs `442-451`
**Issue:** The member-view `Enter` branch calls `e.preventDefault()` / `e.stopPropagation()`
**unconditionally**, and its comment (lines 443-446) states the invariant explicitly: "Enter must
never bubble to a host `<form>` submit — suppressed unconditionally (the pre-existing regression
floor)." The groups-view branch puts both calls *inside* the `activeIndex >= 0` bounds check, so
`Enter` with no active option escapes.

`activeIndex` is `-1` after every `openPanel()` (line 288) **and** after every keystroke
(`handleInput`, line 277) — so "type, then press Enter" is the default state, not an edge case.
Consequences:

- Combobox variant inside a `<form>`: implicit submission. Neutralised today only because
  `ActFormBody.svelte:262` `preventDefault()`s its own `onsubmit`; nothing in this primitive
  guarantees the next consumer does.
- Select variant: the unprevented `Enter` reaches the `<button>`'s default activation → `click`
  → `toggleSelectOpen()` → the panel closes on Enter. Inconsistent with member view.
- Either variant: the event bubbles to any ancestor keydown handler (modal, row handler).

**Fix:** Hoist the suppression out of the bounds check, mirroring the member-view branch:
```ts
} else if (e.key === 'Enter') {
  e.preventDefault();
  e.stopPropagation();
  if (activeIndex >= 0 && activeIndex < groups.length) {
    handleOptionClick(groups[activeIndex]);
  }
}
```

### WR-03: WR-06's fix loses focus — closing the panel from the in-panel search input dumps focus to `<body>`

**File:** `ui/src/lib/components/Dropdown.svelte:536-549` with `355-368`, `398-416`
**Issue:** The search input now handles `Escape` and `Tab` via `handleKeydown`, and both set
`open = false`. The input lives inside `{#if open}` (line 523), so the element that currently
holds focus is destroyed in the same update. Nothing refocuses `triggerEl`. The browser resets
focus to `<body>`, so the next `Tab` restarts from the top of the document and `Escape` strands
the keyboard user with no visible focus ring.

WAI-ARIA's combobox pattern requires `Escape` to return focus to the combobox element. This is
squarely inside WR-06's remit — wiring the keys without focus return makes the select variant
keyboard-reachable but not keyboard-usable.
**Fix:** Return focus on every close initiated from inside the panel:
```ts
function closePanel(restoreFocus = true) {
  open = false;
  if (restoreFocus) (inputEl ?? triggerEl)?.focus();
}
```
Call it from the `Escape` and `Tab` branches (for `Tab`, restore focus to the trigger and let the
browser continue tabbing from there, or call `triggerEl.focus()` before returning so the default
Tab traversal resumes from the field rather than the document root).

### WR-04: select-variant search keystrokes still fire `onQueryInput`

**File:** `ui/src/lib/components/Dropdown.svelte:274-280, 546`
**Issue:** Round-1 WR-07, unaddressed and now more reachable since the search input is a
first-class focus target. `onQueryInput` is documented (lines 61-64) as the *combobox*
controlled-input sync hook — consumers wire it to overwrite their own `value`. The select
variant's in-panel search box shares `handleInput`, so a select consumer that supplies
`onQueryInput` has its displayed field value clobbered by search-box keystrokes.
**Fix:** Split the handler; the panel search box should schedule the search only.
```ts
function handleSearchInput(e: Event) {
  const query = (e.currentTarget as HTMLInputElement).value;
  activeIndex = -1;
  scheduleSearch(query);   // no onQueryInput, no `open = true` (already open)
}
```

### WR-05: the focused element in the select variant carries no combobox role

**File:** `ui/src/lib/components/Dropdown.svelte:504-520, 536-549`
**Issue:** After the fix, `aria-activedescendant` and `aria-controls` are duplicated onto the
search input — but `role="combobox"` and `aria-expanded` remain only on the `<button>` trigger,
which is *not* focused once the user clicks into the search box. A screen reader on the focused
input announces a plain textbox with an active descendant and no expanded-state or popup
relationship, while an unfocused element elsewhere claims to be the combobox. Two elements now
advertise control of the same listbox.
**Fix:** Move the combobox semantics to whichever element owns focus. Simplest correct shape for
the select variant is the WAI-ARIA "combobox with inline listbox and a separate text input":
give the search input `role="combobox"`, `aria-expanded={open}`, `aria-haspopup="listbox"` and
drop `aria-activedescendant`/`aria-controls` from the trigger button while the panel is open.

### WR-06: no rejection handling on either async expand path

**File:** `ui/src/lib/components/Dropdown.svelte:168-180, 195-210`
**Issue:** The AUTO-05 IIFE (`void (async () => { const result = await onExpandGroup(only); ... })()`)
and `drillInto` (invoked as `void drillInto(g)` at line 228) both `await` a caller-supplied
callback with no `try`/`catch`. If `onExpandGroup` rejects: an unhandled promise rejection is
raised, and every state write after the `await` is skipped — the panel is left frozen on the
previous content with `activeIndex` stale and no error affordance. `ActFormItemsTable.expandGroup`
happens to catch internally (line 213-217), so this is latent rather than live — but the
primitive's contract (`Promise<TMember[]> | TMember[]`) invites rejecting implementations and
provides no guarantee.
**Fix:** Wrap both call sites and fall back to the empty state:
```ts
let result: TMember[];
try {
  result = await onExpandGroup(only);
} catch {
  if (seq === expandSeq) { members = []; viewMode = 'members'; activeIndex = -1; }
  return;
}
```

### WR-07: `backToGroups()` restores `returnIndex` without re-validating it

**File:** `ui/src/lib/components/Dropdown.svelte:213-219` with `204`
**Issue:** `returnIndex` is captured inside `drillInto` *after* the await, from the `groups` array
as it stood then. `groups` can shrink or change before the user presses "← Назад" (the AUTO-05
effect only resets `viewMode` when the drill was not manual, and BL-01's dismiss/reopen path
leaves it entirely stale). `backToGroups` assigns it to `activeIndex` unchecked, so the `.active`
highlight can land on an unrelated row or on no row at all. `activeOptionId()` bounds-checks
(line 330) and `Enter` bounds-checks (line 393), so this is a visual/AT inconsistency rather than
a crash — but the highlighted row and the announced active descendant can disagree.
**Fix:** `activeIndex = returnIndex >= 0 && returnIndex < groups.length ? returnIndex : -1;`

## Info

### IN-01: `Tab` on an expandable group discards the user's active option silently

**File:** `ui/src/lib/components/Dropdown.svelte:398-416`
**Issue:** Consequence of the (correct) WR-01 fix. Arrowing onto an expandable group and pressing
`Tab` now closes the panel and commits nothing, with no visual or announced feedback — the user
who thought they were selecting gets an empty field.
**Fix:** Acceptable as "closing wins", but consider drilling in *without* closing instead
(`void drillInto(g)` and `e.preventDefault()` to keep focus), which matches what `Enter` does on
the same row and is what a user pressing Tab on a chevron row most likely intends.

### IN-02: stale finding-id references in code comments

**File:** `ui/src/lib/components/Dropdown.svelte:134, 443`
**Issue:** Line 443 attributes the member-view `Enter` suppression to "WR-02", and line 134
attributes the `onDestroy` debounce cleanup to "WR-05" — neither matches the round-1 report's
numbering (WR-02 = drill-in reset, WR-05 = listbox/`<li>` nesting). Comments citing review IDs
that mean something different in the archived report will mislead the next reader.
**Fix:** Reference the phase/plan (e.g. "Plan 25-08 WR-02") or drop the ID and keep the rationale.

### IN-03: `expandSeq` now has three writers and no single owner

**File:** `ui/src/lib/components/Dropdown.svelte:160, 167, 183, 199, 298`
**Issue:** The generation token is incremented from the `$effect` (both branches), `drillInto`,
and now `openPanel`. Correctness depends on every future `open = true` / view-mode transition
remembering to participate — which BL-01 demonstrates is already not holding. The 13-line comment
at 147-159 is doing work the type system should.
**Fix:** Funnel all increments through the `resetDrillState()` helper proposed in BL-01 and make
`expandSeq++` unreachable outside it.

## Previously Deferred (round 1, user scope decision — re-listed, not re-argued)

These remain present in the code and were explicitly deferred under the "blockers-only" scope call:

- **round-1 WR-05** — `<ul role="listbox">` owning role-less `<li>` wrappers plus two chrome
  `<li>`s (search box line 536, drill header line 557); `aria-activedescendant` targets are not
  ARIA-owned options. Still applies, and WR-05 above compounds it.
- **round-1 WR-03** — view mode still does not reset synchronously on keystroke. Folded into
  BL-01's fix above; listed separately because the UI-SPEC rule is independent of the reopen bug.
- **round-1 WR-08** — `aria-selected={i === activeIndex}` still reports keyboard position as
  selection in non-flat mode (lines 586, 617).
- **round-1 WR-09** (`DeviceGroupRow` retry-forever), **IN-01** (dead `.hint-warn` CSS),
  **IN-06** (showcase force-open) — out of this round's file scope, unchanged.

## Verified-Correct in This Round (no action)

- WR-01's `const g = groups[activeIndex]; if (g && ...)` ordering: the `g &&` short-circuit
  precedes `isGroupExpandable(g)`, so the `activeIndex === -1` → `groups[-1] === undefined`
  crash path is genuinely closed. Flat mode correctly still commits.
- CR-01 (round 1) not regressed: `handleInput` sets `open = true` at line 276.
- CR-02 (round 1) not regressed: `seq !== expandSeq` guards present at lines 172 and 201;
  `expandSeq` correctly a plain `let` so the effect does not self-retrigger.
- `onDestroy` still clears `searchDebounce`; the click-outside `$effect` still returns its
  `removeEventListener`. `handleClickOutside` correctly treats the portaled panel as "inside"
  via `panelEl.contains(target)`, so clicking the new search input does not close the panel.

---

_Reviewed: 2026-07-19_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard (round 2, gap closure)_
</content>
</invoke>

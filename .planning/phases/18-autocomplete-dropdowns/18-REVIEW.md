---
phase: 18-autocomplete-dropdowns
reviewed: 2026-07-11T00:00:00Z
depth: standard
files_reviewed: 13
files_reviewed_list:
  - crates/trackly-app/src/dto/device.rs
  - crates/trackly-app/tests/devices_grouping.rs
  - crates/trackly-core/src/domain/devices.rs
  - crates/trackly-infra/src/repos/devices_sqlite.rs
  - ui/src/features/acts/ActFormItemsTable.svelte
  - ui/src/features/devices/DeviceAutocompleteField.svelte
  - ui/src/lib/components/CartridgeSelect.svelte
  - ui/src/lib/components/GroupedPrinterSelect.svelte
  - ui/src/lib/components/LocationAutocomplete.svelte
  - ui/src/lib/components/PersonAutocomplete.svelte
  - ui/src/lib/components/PrinterSelect.svelte
  - ui/src/lib/components/Select.svelte
  - ui/src/lib/utils/dropdownAnchor.ts
findings:
  critical: 0
  warning: 5
  info: 5
  total: 10
status: issues_found
---

# Phase 18: Code Review Report

**Reviewed:** 2026-07-11
**Depth:** standard
**Files Reviewed:** 13
**Status:** issues_found

## Summary

Reviewed the Phase 18 device-grouping backend rework (`list_grouped` dual-mode with
model key + FTS filter), the shared `dropdownAnchor` portal utility, the migrated
autocomplete/select components, and the drill-in device picker in the act form.

**Security assessment is clean.** The stated risk areas held up:
- **FTS injection:** `build_fts_query` correctly whitelists tokens (splits on whitespace,
  escapes `"` → `""`, strips NUL, wraps each token in quotes + `*`). All three
  `list_grouped` SQL branches are static string constants; the only user value
  (`match_expr`) is bound via `rusqlite::params!` as `?4`. The `T-18-01` sanitizer test
  exercises `(AND OR)`, `NOT foo`, unmatched quotes, `NEAR(...)`, `foo*bar` — no injection
  surface. `autocomplete` derives column names only from the `AutocompleteField` enum, and
  the mixed numbered/anonymous `?` placeholder ordering in the status-IN branch is
  internally consistent with SQLite's auto-numbering rule.
- **Portal listener cleanup:** `dropdownAnchor` removes both `scroll` (capture) and
  `resize` listeners in `destroy()` — no listener leak.

No BLOCKER-class defects found. Five WARNING-class correctness/maintainability issues and
five INFO items are documented below. The most impactful are the per-row transient-state
misalignment on row removal (WR-01) and the `Enter`-key form-submit escape in the drill-in
member view (WR-02).

## Warnings

### WR-01: Per-row dropdown state maps are not reindexed on row removal

**File:** `ui/src/features/acts/ActFormItemsTable.svelte:101-107` (and `:460`)
**Issue:** All transient dropdown state is stored in index-keyed records
(`suggestionsByRow`, `openByRow`, `loadingByRow`, `viewModeByRow`, `drillGroupByRow`,
`membersByRow`, `activeIndexByRow`, `showBackByRow`, `rowInputEls`, `rowDropdownEls`),
while the `{#each items as row, idx (idx)}` loop is keyed by array index. `removeRow(idx)`
deletes only `suggestionsByRow[idx]`, `loadingByRow[idx]`, `openByRow[idx]` and does **not**
shift the remaining keys down. Removing a non-last row therefore leaves every subsequent
row displaying the previous occupant's dropdown/drill state (stale open dropdown, wrong
suggestions, wrong drill-in members, wrong `activeIndex`). The committed `items` data stays
correct (it is filtered), so submitted acts are unaffected, but the picker UI misaligns.
The `viewModeByRow`/`drillGroupByRow`/`membersByRow`/`activeIndexByRow`/`showBackByRow`
entries are also never cleaned up at all, leaking across removals.
**Fix:** Rebuild the index-keyed maps as arrays on mutation, or key the `{#each}` by a
stable per-row id instead of `idx` and store state under that id. Minimal patch for
`removeRow`:
```ts
function removeRow(idx: number) {
  const next = items.filter((_, i) => i !== idx);
  // Reindex ALL transient per-row maps so state follows the surviving rows.
  const shift = <T>(m: Record<number, T>): Record<number, T> => {
    const out: Record<number, T> = {};
    for (const k of Object.keys(m)) {
      const i = Number(k);
      if (i < idx) out[i] = m[i];
      else if (i > idx) out[i - 1] = m[i];
    }
    return out;
  };
  suggestionsByRow = shift(suggestionsByRow);
  loadingByRow = shift(loadingByRow);
  openByRow = shift(openByRow);
  viewModeByRow = shift(viewModeByRow);
  drillGroupByRow = shift(drillGroupByRow);
  membersByRow = shift(membersByRow);
  activeIndexByRow = shift(activeIndexByRow);
  showBackByRow = shift(showBackByRow);
  onChange(next);
}
```

### WR-02: `Enter` in drill-in member view is not suppressed and can submit the act form

**File:** `ui/src/features/acts/ActFormItemsTable.svelte:308-353`
**Issue:** `handleRowKeydown` returns early when `viewModeByRow[idx] === 'members'`
(line 327) *before* the `Enter` handling that lives further down (lines 339-345). In the
groups view, an open dropdown swallows `Enter` via `preventDefault()`/`stopPropagation()`.
In the member (drill-in / auto-flatten) view, an open dropdown does **not** — the keydown
propagates. If this table is rendered inside a `<form>`, pressing `Enter` while the member
list is open triggers native form submission, creating/submitting the act mid-pick.
Contrast `DeviceAutocompleteField.handleKeydown`, which always prevents `Enter` while
`open`.
**Fix:** Suppress `Enter` (and ideally have `Escape` return to the group list) while a
member-view dropdown is open, before the early `return`:
```ts
if (viewModeByRow[idx] === 'members') {
  if (e.key === 'Enter') { e.preventDefault(); e.stopPropagation(); }
  return;
}
```

### WR-03: Global CSS selectors collide across portal-dropdown components

**File:** `ui/src/lib/components/PersonAutocomplete.svelte:281`,
`ui/src/lib/components/LocationAutocomplete.svelte:203`,
`ui/src/features/devices/DeviceAutocompleteField.svelte:424`,
`ui/src/features/acts/ActFormItemsTable.svelte:683`
**Issue:** Because the dropdown is portaled to `<body>`, each component styles it via
un-namespaced `:global(.dropdown)`, `:global(.dropdown-item)`, `:global(.dropdown-empty)`,
`:global(.opt)` etc. These global rules leak process-wide and conflict: `.dropdown`
`background` is `--color-surface` (PersonAutocomplete), `--color-surface-raised`
(LocationAutocomplete / ActFormItemsTable), and `max-height` is `240px` vs `200px`
(DeviceAutocompleteField). `.dropdown-empty` is defined with different padding/alignment in
PersonAutocomplete vs ActFormItemsTable. Whichever component's stylesheet is injected last
wins, so a portaled dropdown can render with another component's styling depending on load
order — a non-deterministic visual defect and a maintenance hazard.
**Fix:** Namespace each component's portal root (e.g. `.dropdown--person`,
`.dropdown--device`, `.dropdown--items`) and scope the `:global()` rules under that class,
or move the shared dropdown chrome into one shared component/stylesheet imported once.

### WR-04: `condition_distinct_count` under-counts because `COUNT(DISTINCT)` ignores NULL

**File:** `crates/trackly-infra/src/repos/devices_sqlite.rs:1004,1038,1074`;
consumed at `ui/src/features/acts/ActFormItemsTable.svelte:188-191`
**Issue:** All three grouped SQL branches compute
`COUNT(DISTINCT d.condition) AS condition_distinct_count`. SQLite's `COUNT(DISTINCT …)`
does not count NULLs. A group whose members have conditions `{NULL, "Новое"}` yields
`condition_distinct_count = 1`, so `isExpandable()` treats the group as condition-uniform,
never offers drill-in, and `pickGroup` clones all members as if identical — silently mixing
a device with no recorded state and a "Новое" device into one act line. The `«разное»` /
drill-in signal (the stated purpose of the field, D-07) is therefore missed whenever a set
condition coexists with an unset one.
**Fix:** Count NULL as a distinct bucket, e.g.
`COUNT(DISTINCT COALESCE(d.condition, '')) AS condition_distinct_count` (or
`COUNT(DISTINCT d.condition) + (MAX(d.condition IS NULL))`), and add a regression test with
mixed NULL + non-NULL conditions asserting `condition_distinct_count == 2`.

### WR-05: Debounce timers and in-flight fetches are not cancelled on unmount

**File:** `ui/src/lib/components/PersonAutocomplete.svelte:64-89,117-133`;
`ui/src/lib/components/LocationAutocomplete.svelte:46-53`;
`ui/src/features/devices/DeviceAutocompleteField.svelte:105-145,184-212`;
`ui/src/features/acts/ActFormItemsTable.svelte:75,124-128`
**Issue:** The fetch `$effect`s and focus handlers schedule `setTimeout` debounce callbacks
but never clear the pending timer on teardown (the effects `return` nothing, or only return
the click-outside cleanup). When a component/row unmounts with a pending timer (e.g. a modal
closes, or `removeRow` runs), the timer still fires, issues the API call, and assigns to
`$state` on a destroyed component. In `ActFormItemsTable` the `debounceTimers` record entry
for a removed row is likewise never cleared, so a stale fetch can write into the reindexed
maps. Low functional impact but a real resource/leak pattern replicated across five files.
**Fix:** Return a cleanup from each fetch `$effect` (`return () => clearTimeout(timer)`),
and in `removeRow` clear/delete `debounceTimers[idx]`.

## Info

### IN-01: `dropdownAnchor` does not reposition on async content-height changes

**File:** `ui/src/lib/utils/dropdownAnchor.ts:42-53`
**Issue:** `reposition` recomputes the flip-up decision from `node.scrollHeight`, but is only
invoked on mount, on `scroll`/`resize`, and on `update(params)`. When dropdown content
changes height *after* mount without an anchor change — e.g. `ActFormItemsTable` toggling
between the group list and the taller drill-in member list, or async suggestions arriving —
`update` is not triggered (the `anchorEl` reference is unchanged), so the up/down flip can
be stale and the list may overflow the viewport. Position (`left`/`width`/`top`) stays
correct; only the flip choice is affected.
**Fix:** Observe content changes (e.g. `ResizeObserver` on `node`) or call `reposition()`
explicitly after the view-mode toggle / suggestion update.

### IN-02: Dead CSS rule `.hint-warn`

**File:** `ui/src/features/acts/ActFormItemsTable.svelte:878-882`
**Issue:** `.hint-warn` is defined but no element uses that class in the template.
**Fix:** Remove the unused rule.

### IN-03: `pickGroup` label drops the serial number when both SN and inventory are present

**File:** `ui/src/features/acts/ActFormItemsTable.svelte:368-372`
**Issue:** For a serial device with both `serial_no` and `inventory_no`, the label renders
only `(инв. …)` and omits the SN, whereas `pickDevice` (line 264-270) shows both. Cosmetic
inconsistency in the picked-device label. (In practice serial groups reach `drillInto`, so
this branch is rarely hit for `ids.length>1`, but singletons with SN+inv do hit it.)
**Fix:** Mirror `pickDevice`'s label composition so SN is always shown when present.

### IN-04: Keyboard-navigation modulo can produce `NaN` if `open` is ever true with an empty list

**File:** `ui/src/lib/components/PersonAutocomplete.svelte:165,168`;
`ui/src/lib/components/LocationAutocomplete.svelte:92,95`
**Issue:** `activeIndex = (activeIndex + 1) % suggestions.length` runs after `if (!open)
return` without re-checking `suggestions.length > 0`. Current control flow only sets
`open = true` when the list is non-empty, so this is not presently reachable, but the guard
is implicit and fragile against future edits. `DeviceAutocompleteField` and
`ActFormItemsTable` already guard with explicit `length === 0` checks.
**Fix:** Add `if (suggestions.length === 0) return;` before the modulo in the Arrow
handlers.

### IN-05: `list_grouped` representative row mixes MIN(id) with MAX() aggregates

**File:** `crates/trackly-infra/src/repos/devices_sqlite.rs:995-1027`
**Issue:** `repr.id = MIN(d.id)` while `model/notes/complectation/condition/location_id/
status_id/version/timestamps/inv_no/serial_no` are `MAX(...)` and the joined location name
comes from `MAX(d2.location_id)`. The representative therefore need not be a single real
row: its displayed status/location/inventory can come from a different member than
`MIN(id)`. This is an intentional aggregate-display trade-off for collapsed groups, but for
`count == 1` (singleton) groups it is a no-op and for `count > 1` the UI hides most of these
columns — so impact is limited. Documented for awareness; verify the singleton path still
surfaces the true row (the `grouping_singleton_includes_inventory_and_serial_no` test
covers inv/serial only).
**Fix:** None required if the display semantics are accepted; otherwise select the repr
columns via a correlated subquery keyed on `MIN(id)` rather than per-column `MAX()`.

---

_Reviewed: 2026-07-11_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_

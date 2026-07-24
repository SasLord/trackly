---
phase: 30-quality-a11y-platform-parity
reviewed: 2026-07-24T18:58:20Z
depth: standard
files_reviewed: 8
files_reviewed_list:
  - ui/src/lib/components/Dropdown.svelte
  - ui/src/lib/components/TableRow.svelte
  - ui/src/features/acts/ActListRow.svelte
  - ui/src/features/cartridges/CartridgeListRow.svelte
  - ui/src/features/printers/PrinterListRow.svelte
  - ui/src/features/requests/RequestListRow.svelte
  - ui/src/features/dashboard/PeriodToggle.svelte
  - ui/src/features/dashboard/DashboardPage.svelte
findings:
  critical: 1
  warning: 2
  info: 1
  total: 4
status: issues_found
---

# Phase 30: Code Review Report

**Reviewed:** 2026-07-24T18:58:20Z
**Depth:** standard
**Files Reviewed:** 8
**Status:** issues_found

## Summary

This diff is a focused accessibility / focus-ring gap-closure batch: (1) a shared
row-wide keyboard focus ring in `TableRow.svelte` (`.tr-row:has(:focus-visible)`)
replacing four duplicated cell-level `box-shadow` rules; (2) two Dropdown keyboard
additions — moving DOM focus into the in-panel search input on open (Gap 3), and an
`ArrowLeft`-to-go-back drill-in exit (Gap 5); (3) small CSS fixes in `PeriodToggle`
(inset focus ring so it isn't clipped by `overflow-x`) and `DashboardPage`
(`min-height: 0` flex-scroll fix).

The two CSS fixes in PeriodToggle/DashboardPage are correct. However, the central
mechanism of the Gap 4 consolidation — drawing the focus ring on the `<tr>` element —
does not render on the project's primary target engines, and the same change removes
the previously-working per-cell rings, producing a net accessibility **regression**
rather than an improvement. The two Dropdown keyboard additions each introduce a
usability defect.

## Critical Issues

### CR-01: Row-wide focus ring is invisible on target engines — `box-shadow` on `<tr>` is not painted in a `border-collapse: collapse` table

**File:** `ui/src/lib/components/TableRow.svelte:100-102`
(with removals in `ActListRow.svelte:104-110`, `CartridgeListRow.svelte:146-152`,
`PrinterListRow.svelte:131-137`, `RequestListRow.svelte:162-168`)

**Issue:**
The new rule draws the focus indicator via `box-shadow` on the `<tr>` element:

```scss
.tr-row:has(:focus-visible) {
  box-shadow: inset 0 0 0 2px var(--tr-accent);
}
```

`.tr-row` is a `<tr>` rendered inside `Table.svelte`'s `.tr-table`, which sets
`border-collapse: collapse` (`Table.svelte:142`). Blink (WebView2 on Windows — the
primary target) and WebKit (WKWebView on macOS) **do not paint `box-shadow` on
`<tr>` / table-row-group boxes when `border-collapse: collapse`** — this is a
long-standing, well-known cross-engine limitation. The rule *matches* (the `:has()`
selector is fine on evergreen engines) but nothing is drawn.

This is not a hypothetical: the codebase itself only ever applies `box-shadow` to
`<td>` cells or `<button>` elements where it *does* render — the selected-row accent
(`.tr-row.selected :global(> td:first-child)`, `TableRow.svelte:133-135`) and the
chevron ring (`.tr-row-chevron:focus-visible`, `TableRow.svelte:167-170`). The new
rule is the only one placed on `<tr>`.

Because the same commit **deletes** the previously-working cell-level rings from all
four list rows (each replaced with bare `outline: none` + a `check-focus-outline:
ignore` marker), the net effect on the primary desktop targets is: keyboard focus on
an Acts / Cartridges / Printers / Requests row now shows **no visible focus indicator
at all**. The `check-focus-outline: ignore` whitelist markers also suppress the very
lint gate (30-01) that would otherwise have flagged the missing indicator, so CI
stays green while the a11y guarantee is silently lost. The "give Devices/Users a
row-wide highlight" goal also does not materialize.

**Fix:** Draw the ring on the cells (proven to render in this exact table — the
`.selected` accent already does), not on the `<tr>`. For example, a full-row ring
composed on the cells:

```scss
// Top + bottom edges on every cell, left edge on first, right edge on last.
.tr-row:has(:focus-visible) :global(> td) {
  box-shadow:
    inset 0 2px 0 var(--tr-accent),
    inset 0 -2px 0 var(--tr-accent);
}
.tr-row:has(:focus-visible) :global(> td:first-child) {
  box-shadow:
    inset 0 2px 0 var(--tr-accent),
    inset 0 -2px 0 var(--tr-accent),
    inset 2px 0 0 var(--tr-accent);
}
.tr-row:has(:focus-visible) :global(> td:last-child) {
  box-shadow:
    inset 0 2px 0 var(--tr-accent),
    inset 0 -2px 0 var(--tr-accent),
    inset -2px 0 0 var(--tr-accent);
}
```

(Note this must be reconciled with the existing `.tr-row.selected > td:first-child`
box-shadow so a selected+focused first cell keeps both the 3px accent edge and the
ring.) Alternatively switch `.tr-table` to `border-collapse: separate; border-spacing: 0;`
so `box-shadow` on `<tr>` paints — but that risks reintroducing the double-border
seams `collapse` was chosen to avoid, so the cell-based approach is safer. Either
way, **verify visually on WebView2 and WKWebView**, not just Firefox/dev Chrome,
before closing Gap 4.

## Warnings

### WR-01: `ArrowLeft` drill-in-exit hijacks text-caret navigation in the combobox field and the select search input

**File:** `ui/src/lib/components/Dropdown.svelte:491-502`

**Issue:**
The new `ArrowLeft` branch runs inside the member-view keyboard block and calls
`e.preventDefault()` + `backToGroups()` whenever `showBack` is true:

```js
} else if (e.key === 'ArrowLeft') {
  if (showBack) {
    e.preventDefault();
    backToGroups();
  }
}
```

But `handleKeydown` is bound to two **text inputs**: the combobox field
(`variant === 'combobox'`, line 576) and the select-variant in-panel search input
(line 628). When a user has drilled into a group (`showBack === true`) and has typed
a query, pressing `ArrowLeft` to move the text caret left within their query instead
triggers `backToGroups()` and `preventDefault()` swallows the normal caret movement.
Text editing is broken while in member-view. This is especially likely now that
Gap 3 (WR/CR context below) auto-focuses the select search input, so the caret lives
in a text field whenever the panel is open.

For the select-variant *button* trigger (non-searchable) there is no caret to hijack,
so `ArrowLeft` is a reasonable "go back" affordance there — the bug is specific to
the text-input contexts.

**Fix:** Gate the `ArrowLeft`-to-go-back behavior to non-text-input contexts, or only
consume it when the caret is already at position 0 of an empty/collapsed selection.
For a text input:

```js
} else if (e.key === 'ArrowLeft') {
  const el = e.currentTarget as HTMLInputElement;
  const atStart =
    el.tagName !== 'INPUT' ||
    (el.selectionStart === 0 && el.selectionEnd === 0);
  if (showBack && atStart) {
    e.preventDefault();
    backToGroups();
  }
}
```

### WR-02: Auto-focusing the search input on open leaves keyboard focus stranded on `<body>` after the panel closes

**File:** `ui/src/lib/components/Dropdown.svelte:549-553` (with `searchInputEl`, lines 141-144/621)

**Issue:**
The new Gap 3 effect moves DOM focus into the portaled search input every time the
select-variant panel opens:

```js
$effect(() => {
  if (open && variant === 'select' && searchable) {
    searchInputEl?.focus();
  }
});
```

This is a genuinely *new* focus location: before this diff, focus stayed on the
trigger `<button>`, so closing the panel (pick / Escape / click-outside / Tab) left
focus on a real, still-mounted element. Now focus lives inside the `<li>` search
input, which is unmounted the moment `open` becomes `false` (pick via
`handleMemberClick`/`handleOptionClick`, or `openPanel`/`toggleSelectOpen` close
paths). When that element is removed, the browser drops focus to `<body>` — a
keyboard user is dumped to the top of the document and screen-reader context is lost
after every selection. The code comment acknowledges "No cleanup/restore-on-close
step ... out of this gap's literal scope," but for a phase whose entire purpose is
keyboard accessibility, silently discarding the focus position is a regression
(WCAG 2.4.3 Focus Order).

**Fix:** Restore focus to the trigger element when the panel closes. Track the close
transition and re-focus `triggerEl` (select variant) or `inputEl` (combobox):

```js
let wasOpen = false;
$effect(() => {
  if (open && variant === 'select' && searchable) {
    searchInputEl?.focus();
  } else if (!open && wasOpen) {
    (triggerEl ?? inputEl)?.focus();
  }
  wasOpen = open;
});
```

(Skip the restore when the close was caused by a `Tab` that intentionally moves focus
forward, to avoid yanking focus back.)

## Info

### IN-01: `check-focus-outline: ignore` markers assume the row ring works — they will mask CR-01 in CI

**File:** `ActListRow.svelte:107-108`, `CartridgeListRow.svelte:149-150`,
`PrinterListRow.svelte:134-135`, `RequestListRow.svelte:165-166`

**Issue:** Each removed cell ring was whitelisted with a `// check-focus-outline:
ignore` marker so the 30-01 lint gate stays green. The markers are only correct if
the consolidated row ring actually renders (CR-01). If CR-01 is fixed by moving the
ring back onto the cells, these markers become misleading (there *is* again a
cell-level indicator) and should be re-evaluated. If CR-01 is fixed at the table
level, the markers are acceptable but the comment should point at a *verified*
rendering, not an assumed one.

**Fix:** After resolving CR-01, revisit whether each `ignore` marker is still
accurate, and reference the verified rendering location rather than an untested
`<tr>` rule.

---

_Reviewed: 2026-07-24T18:58:20Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_

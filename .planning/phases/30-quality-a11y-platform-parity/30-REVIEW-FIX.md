---
phase: 30-quality-a11y-platform-parity
fixed_at: 2026-07-25T00:00:00Z
review_path: .planning/phases/30-quality-a11y-platform-parity/30-REVIEW.md
iteration: 1
findings_in_scope: 3
fixed: 3
skipped: 0
status: all_fixed
---

# Phase 30: Code Review Fix Report

**Fixed at:** 2026-07-25
**Source review:** .planning/phases/30-quality-a11y-platform-parity/30-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 3 (CR-01, WR-01, WR-02)
- Fixed: 3
- Skipped: 0

Info finding IN-01 was intentionally out of scope. It is resolved implicitly by
the CR-01 fix: the ring is once again drawn on the row's `<td>` cells, so the
`check-focus-outline: ignore` markers in the four list-row files now sit above
`outline: none` declarations whose visible indicator (the cell box-shadow) is
supplied by the shared `TableRow` primitive. The lint gate stays green (0
violations) and the markers are accurate rather than masking a missing indicator.

## Fixed Issues

### CR-01: Row-wide focus ring invisible on target engines (box-shadow on `<tr>` under border-collapse)

**Files modified:** `ui/src/lib/components/TableRow.svelte`
**Commit:** c5bfae8
**Applied fix:** Replaced the single `<tr>`-level rule
`.tr-row:has(:focus-visible) { box-shadow: ... }` — which matches but paints
nothing on Blink (WebView2) / WebKit (WKWebView) under `border-collapse:
collapse` — with cell-level rules that compose a continuous full-row ring on the
row's `<td>` children:
- `.tr-row:has(:focus-visible) :global(> td)` — top + bottom edges on every cell
- `... :global(> td:first-child)` — adds the left edge
- `... :global(> td:last-child)` — adds the right edge

These use the same in-scope `.tr-row … :global(> td)` selector shape the file
already relies on (compiles to `.tr-row…svelte-hash > td`, so it keeps
specificity over a consumer's `.cell` class), matching the proven-to-render
`.selected` accent precedent.

Reconciled with the existing `.tr-row.selected :global(> td:first-child)` accent
via a dedicated `.tr-row.selected:has(:focus-visible) :global(> td:first-child)`
rule (specificity 0,5,1) that composes BOTH the 3px selected accent AND the ring
edges in one declaration — box-shadow does not stack across separate rules, and
the plain focus first-child ring (0,4,1) would otherwise tie with, and be
overridden by, the `.selected` accent (0,4,1), dropping one indicator. A
selected+focused row now keeps both.

**Requires human verification:** The reviewer explicitly requires visual
confirmation on WebView2 (Windows) and WKWebView (macOS) — CSS specificity/paint
behavior cannot be verified by syntax checks alone. Recommend a quick keyboard-tab
across an Acts/Cartridges/Printers/Requests row plus a selected+focused row on
both target engines before closing Gap 4.

### WR-01: `ArrowLeft` drill-in-exit hijacks text-caret navigation

**Files modified:** `ui/src/lib/components/Dropdown.svelte`
**Commit:** 8239bc9
**Applied fix:** Gated the `ArrowLeft` → `backToGroups()` branch in
`handleKeydown` so it only fires when there is no caret to move: a non-`INPUT`
`e.currentTarget` (the select button trigger), or an `INPUT` whose caret is
already at position 0 with no selection (`selectionStart === 0 &&
selectionEnd === 0`). In every other case the browser's normal left-caret
movement is preserved (no `preventDefault`). Text editing in the combobox field
and the in-panel search input is no longer broken while drilled into a group.

**Requires human verification:** Behavioral change — recommend a manual keyboard
check that (a) ArrowLeft moves the caret mid-query, and (b) ArrowLeft at caret
position 0 (and on the button trigger) still exits the drill-in.

### WR-02: Auto-focused search input leaves focus stranded on `<body>` after close

**Files modified:** `ui/src/lib/components/Dropdown.svelte`
**Commit:** a523cbd
**Applied fix:** Extended the Gap 3 focus effect to restore focus on the
open→close transition. Added `wasOpen` and `skipFocusRestoreOnClose` tracking:
on close (`!open && wasOpen`) for the select+searchable variant — the only case
Gap 3 moved focus — focus is returned to the still-mounted `triggerEl` button,
so the portaled search `<input>` unmounting no longer dumps focus to `<body>`
(WCAG 2.4.3). The restore is skipped when `Tab` caused the close: both Tab
branches in `handleKeydown` (groups-view and member-view) set
`skipFocusRestoreOnClose = true` before `open = false`, so Tab's intentional
forward focus movement is not yanked back. Scope limited to select+searchable to
avoid changing combobox/other close-path behavior.

**Requires human verification:** Behavioral change — recommend a manual keyboard
check that pick / Escape / click-outside return focus to the trigger, while Tab
advances focus forward without snapping back.

## Verification / Gate Results

- `node ui/scripts/check-focus-outline.mjs` — **PASS** (0 нарушений)
- `pnpm --dir ui svelte-check` — **0 errors** in scope. The 61 errors on first
  run were all `Cannot find module '../../bindings'` — the generated,
  gitignored `ui/src/bindings.ts` (produced by `cargo test export_bindings`) is
  absent in a fresh worktree. After supplying that generated artifact the check
  reports **0 errors, 48 warnings** (all pre-existing `state_referenced_locally`
  warnings in unrelated files; none in `Dropdown.svelte`/`TableRow.svelte`).
- Frontend build (`vite build`) — **PASS** (399 modules, built cleanly; only a
  pre-existing dynamic-import advisory for `toast.svelte.ts`). Note: the full
  `pnpm --dir ui build` script runs a `prebuild` = `cargo test -p trackly-app
  --test export_bindings` that fails on unrelated Rust backend code (3 compile
  errors in `trackly-app` lib, untouched by this fix batch and unrelated to the
  frontend-only changes). The frontend bundle itself compiles.

## Notes

All work was performed in an isolated git worktree on branch
`gsd-reviewfix/30-88851`; the three fix commits were fast-forwarded onto `main`
(35bbeed → a523cbd) and the worktree/temp-branch/recovery-sentinel were cleaned
up transactionally.

---

_Fixed: 2026-07-25_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_

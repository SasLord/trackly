---
phase: 26-windows-with-mockup
reviewed: 2026-07-20T13:22:32Z
depth: standard
files_reviewed: 18
files_reviewed_list:
  - ui/src/features/layout/Layout.svelte
  - ui/src/features/layout/Sidebar.svelte
  - ui/src/features/layout/layout-state.svelte.ts
  - ui/src/lib/components/PageHeader.svelte
  - ui/src/lib/components/ActionMenu.svelte
  - ui/src/lib/components/Table.svelte
  - ui/src/lib/components/TableRow.svelte
  - ui/src/lib/components/Input.svelte
  - ui/src/lib/components/ThemeSwitcher.svelte
  - ui/src/features/devices/DevicesPage.svelte
  - ui/src/features/devices/DeviceFilters.svelte
  - ui/src/features/devices/DeviceList.svelte
  - ui/src/features/dashboard/DashboardPage.svelte
  - ui/src/features/dashboard/StatWidget.svelte
  - ui/src/features/dashboard/ChartWidget.svelte
  - ui/src/features/dashboard/PeriodToggle.svelte
  - ui/src/styles/_breakpoints.scss
  - ui/src/styles/_tokens.scss
  - ui/eslint.config.js
findings:
  critical: 0
  warning: 3
  info: 4
  total: 7
status: issues_found
---

# Phase 26: Code Review Report

**Reviewed:** 2026-07-20T13:22:32Z
**Depth:** standard
**Files Reviewed:** 18 (+ 26-CONTEXT.md, 26-UI-SPEC.md, 26-0N-SUMMARY.md read as required background)
**Status:** issues_found

## Summary

Phase 26 is a visually-scoped restyle (Дашборд + Устройства + shared shell) with no backend
changes, and the bulk of the migration is careful and internally consistent: `D-01/D-03/D-04/D-08`
("макет задаёт форму, приложение задаёт содержание") are honored line-for-line — no fabricated
dashboard panels, no new "+ Создать акт" entry point, `warningItems`/logout/role-nav preserved,
paired error strings identical across `StatWidget`/`ChartWidget`, CRUD/CSV/print handlers in
`DevicesPage.svelte` untouched. Cross-checking `Sidebar.svelte`, `StatWidget.svelte` and most of
`ChartWidget.svelte` against `26-UI-SPEC.md` §3's per-property value table shows near-exact
compliance (padding, radius, font sizes, tokens all match the locked spec).

Two classes of real defects were found:

1. **An accessibility regression in the new mobile drawer/kebab-menu code** (`Layout.svelte`,
   `ActionMenu.svelte`) — both were explicitly claimed in their plan summaries to "mirror
   `Modal.svelte`'s focus-trap-entry pattern," but neither actually implements `Modal.svelte`'s
   `trapTab`/focus-restore mechanics, leaving keyboard focus able to leave the open drawer/menu.
2. **A missed, self-documented spec requirement in `Table.svelte`**: the phase's own
   `26-UI-SPEC.md` §3.14/§6.4 explicitly designates `min-width: 860px` on `<table>` (plus
   `-webkit-overflow-scrolling`/`scrollbar-gutter`) as the *entire* mobile-adaptation strategy for
   the Devices table (chosen specifically because `DeviceListRow`/`DeviceGroupRow` are D-12-frozen
   and can't become responsive cards) — and it was never added to the shared `Table.svelte`. On
   narrow viewports the table will silently squeeze its columns instead of triggering the
   documented horizontal-scroll fallback.

No hardcoded secrets, dangerous functions, or debug artifacts found. No raw hex/rgba literals leak
into `<style>` blocks beyond the one pre-documented, explicitly-accepted `ChartWidget.COLORS`
data-viz exception (verified against `check-tokens.mjs`'s actual Rule 2/4 regexes — it only scans
`<style>` blocks, so the JS-literal exception is real and correctly out of the gate's reach).

## Warnings

### WR-01: Mobile nav drawer is not keyboard-trapped and lets focus reach hidden background content

**File:** `ui/src/features/layout/Layout.svelte:46-60` (focus-management `$effect`), `:87-106`
(backdrop/aside/main markup)

**Issue:** The 26-01 plan summary states this drawer "mirrors `Modal.svelte`'s
focus-trap-entry pattern," but it only replicates the *entry* half (move focus in on open, restore
on close) — it never implements `Modal.svelte`'s `trapTab()` (Tab/Shift-Tab cycling confined to the
open surface). There is also no `role="dialog"`/`aria-modal="true"` anywhere on `<aside>` or the
backdrop (the backdrop is deliberately `role="presentation"`, i.e. decorative). Critically,
`<main id="main" class="content">` (line 104) is **never** given `inert` — only `<aside>` gets
`inert={!isDesktop && !sidebarNav.open}` (line 100), which is the *closed*-state guard, not an
*open*-state guard for the content behind the drawer. Concretely: with the drawer open on a
narrow/LAN-browser viewport, a keyboard user can Tab from the last drawer nav-link straight into
the (visually backdrop-covered, but not `inert`) buttons/links in `<main>`, defeating the modal
illusion the backdrop + `overflow:hidden` body-lock create for pointer users.

**Fix:**
```svelte
<!-- Layout.svelte -->
<main id="main" class="content" inert={sidebarNav.open && !isDesktop}>
  {@render children?.()}
</main>
```
and/or add a `trapTab`-equivalent Tab handler (reuse `Modal.svelte`'s implementation) inside
`handleKeydown`, plus `role="dialog" aria-modal="true"` on `<aside>` while the drawer is open.

### WR-02: ActionMenu kebab popover doesn't restore focus to its trigger, and declares an incomplete ARIA `menu` widget

**File:** `ui/src/lib/components/ActionMenu.svelte:11-33` (no `prevFocus` capture/restore),
`:52` (`role="menu"` with plain `<button>` children, no `role="menuitem"`, no arrow-key/Home/End
navigation)

**Issue:** Closing the panel — via Escape, an outside click, or clicking a menu item — never moves
focus back to `.action-menu-trigger`. If a keyboard user opened the menu and pressed Escape (or
clicked an item) while focus was on a button inside `.action-menu-panel`, that button is removed
from the DOM on the next render (`{#if open}`), and the browser drops focus to `<body>` — the same
class of bug `Modal.svelte` explicitly guards against with its `prevFocus`/`WR-02` comment (see
`Modal.svelte:24-27,67-86`). Separately, `role="menu"` (WAI-ARIA APG "menu" composite widget)
implies Up/Down/Home/End keyboard navigation and `role="menuitem"` children; neither is
implemented, so screen readers announce a menu widget whose actual keyboard model doesn't match.

**Fix:**
```svelte
<script lang="ts">
  let prevFocus: HTMLElement | null = null;
  let triggerEl = $state<HTMLElement | null>(null);

  function close() {
    open = false;
    prevFocus?.focus();
  }
  // set prevFocus = document.activeElement in the trigger's onclick before opening,
  // and call close() instead of `open = false` in onDown/onKey/onClick.
</script>
```
Either implement the minimal APG "menu" keyboard model (`role="menuitem"` on each action +
Up/Down/Home/End handling), or drop `role="menu"`/`aria-haspopup="menu"` for a plainer
non-composite popover semantics that matches what's actually implemented.

### WR-03: `Table.svelte` is missing the phase's own locked `min-width:860px` / scroll-affordance requirement — Devices table won't horizontally-scroll on narrow widths as designed

**File:** `ui/src/lib/components/Table.svelte:101-106` (`.tr-table` rule), `:96-99`
(`.tr-table-wrapper` rule)

**Issue:** `26-UI-SPEC.md` §3.14 ("Рамка таблицы (D-11) и её содержимое") explicitly lists
`min-width:860px` on `<table>` and marks it `**CHG**` (required change, not yet in code), and §6.4
("Во что превращается строка таблицы на узкой ширине") states in bold: *"Ни во что — таблица
остаётся таблицей и едет горизонтально. `min-width: 860px` на `<table>` плюс `overflow-x:auto` на
внутреннем слое рамки — это и есть предписанное макетом поведение,"* explicitly because
`DeviceListRow`/`DeviceGroupRow` are frozen by D-12 and a responsive-card fallback would require
touching them. §6.4 also calls for `-webkit-overflow-scrolling: touch` and `scrollbar-gutter:
stable` on the scroller. None of the three values exist anywhere in the shipped code (verified via
`grep -rn "860\|webkit-overflow-scrolling\|scrollbar-gutter" ui/src` — zero real matches). As
written, `.tr-table { width:100%; table-layout:auto; }` has nothing forcing it wider than its
container, so on a narrow viewport (e.g. the 480px width covered by 26-08's own UAT pass) the table
will squeeze its 7-8 columns to fit instead of triggering `.tr-table-wrapper`'s `overflow-x:auto` —
the opposite of the documented, decided behavior. This also means the same gap will silently repeat
across all Phase 27-28 windows that adopt `Table.svelte`.

**Fix:**
```scss
// Table.svelte
.tr-table {
  width: 100%;
  border-collapse: collapse;
  font-size: var(--tr-font-size-body);
  table-layout: auto;
  min-width: 860px; // spec-mandated — table never shrinks below this, wrapper scrolls instead
}

.tr-table-wrapper {
  width: 100%;
  overflow-x: auto;
  -webkit-overflow-scrolling: touch;
  scrollbar-gutter: stable;
}
```
`Table.svelte` is shared with the showcase and the `ActFormItemsTable` pilot (D-11's own
stated risk) — confirm 860px doesn't visually break those two narrower consumers before landing
unconditionally; gate behind a prop if it does.

## Info

### IN-01: `<th>` row height still 34px, spec calls for 36px

**File:** `ui/src/lib/components/Table.svelte:108-112` (`.tr-thead-row { height: 34px; }`)

**Issue:** `26-UI-SPEC.md` §3.14 lists `<th>` height as `36px` (marked `**CHG**` from the
pre-phase 34px). The value was never updated. 2px, low visual impact, but it's one of the exact
values D-18's "чек-лист значений" verification pass was supposed to catch line-by-line.

**Fix:** `height: 36px;` in `.tr-thead-row`, or update `26-UI-SPEC.md` if 34px is now the accepted
final value (currently the two disagree).

### IN-02: ChartWidget legend `margin-top` doesn't match locked spec value

**File:** `ui/src/features/dashboard/ChartWidget.svelte:434-445` (`.chart-legend { margin: var(--tr-space-xs) 0 0; padding-top: 14px; }`)

**Issue:** `26-UI-SPEC.md` §3.12 specifies `margin-top:16px; padding-top:14px` for `.chart-legend`.
The shipped rule uses `var(--tr-space-xs)` (8px) for margin-top, not the spec'd 16px literal.
`padding-top:14px` is correct.

**Fix:** Change `margin: var(--tr-space-xs) 0 0;` to a literal `margin: 16px 0 0;` (the phase
already accepts off-scale literals per §4's "побеждает макет" rule, so this isn't a token-gate
concern — just an unmatched value).

### IN-03: `Layout.svelte`'s `isDesktop` defaults to `true` before the mount-time `matchMedia` check runs

**File:** `ui/src/features/layout/Layout.svelte:15,19-35`

**Issue:** `let isDesktop = $state(true);` is optimistic; the real value is only known once the
`$effect` on line 19 runs `window.matchMedia('(min-width: 1024px)')`. For the brief window before
that effect fires, on an actual mobile-width LAN-browser session, `<aside>`'s `inert` computes as
`false` (line 100: `inert={!isDesktop && !sidebarNav.open}` — with `isDesktop` still `true`, this is
`false`), i.e. the drawer is briefly *not* marked inert even though it's closed and CSS-transformed
off-screen. Self-corrects on the next microtask and isn't practically exploitable, but is worth
flagging given the phase's explicit "mobile-first correctness" scope (D-15).

**Fix:** Low priority. If addressed, consider seeding `isDesktop` from a synchronous
`window.matchMedia(...).matches` read (guarded for `typeof window === 'undefined'`) instead of a
hardcoded `true` default.

### IN-04: `DevicesPage.svelte` always renders both the desktop action row and the mobile kebab menu, toggled only by CSS

**File:** `ui/src/features/devices/DevicesPage.svelte:233-244`

**Issue:** `.actions-inline` (two `<Button>`s) and `.actions-kebab` (an `<ActionMenu>` wrapping two
plain `<button>`s with duplicate `onclick` handlers) are both always present in the DOM, switched
via `display:none` media queries rather than conditional rendering. Functionally harmless
(`display:none` elements are excluded from the tab order and accessibility tree in all evergreen
browsers), but it means the CSV import/export click handlers are now defined in two separate
places that must be kept in sync by hand, and doubles the always-mounted interactive elements/DOM
nodes for this header.

**Fix:** Not required to fix, but consider deriving a single boolean (reusing the one sanctioned
`matchMedia` pattern already present in `Layout.svelte` for `inert`, or a shared breakpoint
`$derived`) to drive one `{#if}` branch, or factor the two handlers into a shared snippet so a
future edit to one branch can't silently drift from the other.

---

_Reviewed: 2026-07-20T13:22:32Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_

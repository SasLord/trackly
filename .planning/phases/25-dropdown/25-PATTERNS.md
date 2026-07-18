# Phase 25: Таблицы и Dropdown - Pattern Map

**Mapped:** 2026-07-19
**Files analyzed:** 9 (2 new primitives, 2 new showcase sections, 1 showcase wiring edit, 3 pilot edits, 1 token edit)
**Analogs found:** 9 / 9

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|-----------------|---------------|
| `ui/src/lib/components/Table.svelte` | component (table shell) | CRUD (render list) | `ui/src/features/devices/DeviceList.svelte` | role-match (extracts shell, not a 1:1 wrapper) |
| `ui/src/lib/components/TableRow.svelte` | component (table row) | CRUD (render row) | `ui/src/features/devices/DeviceListRow.svelte` + `DeviceGroupRow.svelte` | exact (states) / role-match (group row) |
| `ui/src/lib/components/Dropdown.svelte` | component (combobox/select) | request-response (search) + event-driven (keyboard/focus) | `ui/src/features/acts/ActFormItemsTable.svelte` (per-row picker, lines 75–517, 560–705, 807–937) | exact (drill-in/ARIA contract is the literal spec source) |
| `ui/src/features/showcase/sections/TableSection.svelte` (name at planner's discretion) | component (showcase section) | transform (static demo data) | `ui/src/features/showcase/sections/TabsSection.svelte` | exact (structural template) |
| `ui/src/features/showcase/sections/DropdownSection.svelte` (name at planner's discretion) | component (showcase section) | transform (static demo data) | `ui/src/features/showcase/sections/FieldsSection.svelte` | role-match (state-matrix demo of a field-like component) |
| `ui/src/features/showcase/ShowcasePage.svelte` | provider (page wiring) | transform | itself (existing file, add 2 imports + 2 `<section>` blocks) | exact |
| `ui/src/features/devices/DeviceList.svelte` | component (table wrapper) | CRUD | itself (existing — convert `<table>`/`<thead>`/`<tr>` markup to `Table` slot API) | exact |
| `ui/src/features/devices/DeviceListRow.svelte` | component (table row) | CRUD | itself (existing — convert to consume `TableRow`) | exact |
| `ui/src/features/devices/DeviceGroupRow.svelte` | component (table group row) | CRUD | itself (existing — convert to consume `TableRow` group variant) | exact |
| `ui/src/features/acts/ActFormItemsTable.svelte` | component (form row picker) | request-response + event-driven | itself (existing — device-picker cell replaced by `Dropdown`, drill-in state stays in this file per D-02) | exact |
| `ui/src/styles/_tokens.scss` | config (design tokens) | — | itself (existing — add `--tr-group` to both theme blocks, lines 61–63 light / 137–139 dark) | exact |

## Pattern Assignments

### `ui/src/lib/components/Table.svelte` / `TableRow.svelte` (component, CRUD)

**Analogs:** `ui/src/features/devices/DeviceList.svelte`, `DeviceListRow.svelte`, `DeviceGroupRow.svelte`

**Imports pattern** (`DeviceGroupRow.svelte` lines 1–10):
```svelte
<script lang="ts">
  import Badge from '$lib/components/Badge.svelte';
  import DeviceListRow from './DeviceListRow.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { devices } from './api';
  import type { DeviceDto, DeviceGroup } from '../../bindings';
</script>
```
New primitives live in `ui/src/lib/components/`, so imports are `$lib/components/Badge.svelte` style (no relative `../../`), matching how `DeviceListRow.svelte` imports `Badge` (line 2: `import Badge from '$lib/components/Badge.svelte';`).

**Props pattern (Svelte 5 runes)** (`DeviceListRow.svelte` lines 6–25):
```svelte
interface Props {
  device: DeviceDto;
  onEdit: (_d: DeviceDto) => void;
  onDelete: () => void;
  isLastInGroup?: boolean;
  onPrintAcceptance?: (_d: DeviceDto) => void;
  showStatus?: boolean;
}

const {
  device,
  onEdit,
  onDelete,
  isLastInGroup = false,
  onPrintAcceptance,
  showStatus = true,
}: Props = $props();
```
`TableRow.svelte` should follow this shape: plain (non-bindable) `Props` interface + `$props()` destructure with defaults, since `selected`/`indent`/`last` are one-way display flags, not two-way bound state (per D-11, no bindable — selection mechanics are out of scope).

**Row-state CSS pattern to replace** (`DeviceListRow.svelte` lines 71–96, current implementation — **does not yet match `TableRows.dc` values**, only the `--row-height`/hover/border shape is reusable structurally):
```scss
.device-row {
  height: var(--row-height, 40px);
  &:hover {
    background: var(--tr-surface);
  }
  &.group-last-child .cell {
    border-bottom: 2px solid var(--tr-border-strong);
  }
}
.cell {
  padding: 0 var(--tr-space-xs);
  font-size: var(--tr-font-size-body);
  color: var(--tr-text-primary);
  vertical-align: middle;
  border-bottom: 1px solid var(--tr-border);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 0; // makes text-overflow work in table cells
}
```
**Deviation required by UI-SPEC (D-10, values from `.dc`, not from this file):** hover must use `--tr-row-hover` (not `--tr-surface`), selected must use `--tr-row-selected` + `border-left: 3px solid var(--tr-accent)` with 29px compensated padding, `td` height is 40px with `padding: 0 10px` — this file's `--tr-surface` hover and `--tr-space-xs` (8px) padding are the **old** pre-Phase-25 values and must NOT be copied verbatim into `TableRow`; only the mechanical shape (`height: var(--row-height, 40px)`, `max-width: 0` ellipsis trick, `border-bottom` divider convention) transfers.

**Group-row pattern to replace** (`DeviceGroupRow.svelte` lines 208–219, 261–296 — **current implementation predates `--tr-group` and uses `color-mix()` + inline pill styling, both forbidden by the new token gate**):
```scss
.group-row {
  height: var(--row-height, 40px);
  background: color-mix(in srgb, var(--tr-surface) 94%, var(--tr-accent) 6%);
  cursor: pointer;
  &:hover {
    background: color-mix(in srgb, var(--tr-surface) 86%, var(--tr-accent) 14%);
  }
}
.chevron-btn {
  transition: transform 0.15s ease;
  &.expanded { transform: rotate(180deg); }
}
.count-pill {
  padding: 2px 8px;
  background: color-mix(in srgb, var(--tr-accent) 12%, transparent);
  border: 1px solid color-mix(in srgb, var(--tr-accent) 30%, transparent);
  border-radius: 10px;
  font-size: 12px;
  color: var(--tr-accent);
}
```
**Deviation required (D-09/D-10):** `background: var(--tr-group)` (new token, not `color-mix`), chevron glyph `▸` with `transform: rotate(90deg)` when expanded (not `180deg`) and `.15s` transition (matches, keep as-is — this is the one value the old file already got right), count pill **must be replaced** by `<Badge variant="accent" appearance="count">{N} шт.</Badge>` per UI-SPEC "Переиспользование вместо дублирования" — do not hand-roll `.count-pill` in the new component; the geometry already exists in `Badge.svelte` (`.badge-m-count`, `.badge-m-accent.badge-m-count`, lines 192–212).

**Status badge — already correct, reuse verbatim** (`DeviceListRow.svelte` lines 30–47, 61–64):
```svelte
type BadgeVariant = 'default' | 'accent' | 'success' | 'warning' | 'destructive';
const STATUS_VARIANTS: Record<number, BadgeVariant> = {
  1: 'default', 2: 'accent', 3: 'warning', 4: 'destructive',
};
...
<Badge variant={statusVariant}>{statusLabel}</Badge>
```
This mapping is untouched by Phase 25 — `Badge appearance="soft" size="md"` already matches `TableRows.dc` badge geometry byte-for-byte per UI-SPEC; `TableRow`'s consumer (`DeviceListRow`) keeps calling `Badge` exactly like this.

**Table shell + skeleton + empty state** (`DeviceList.svelte` lines 57–147): the `<table class="device-table">` / `<thead>` / `<tbody>` structure, loading-skeleton branch, and empty-state branch are the shape `Table.svelte` should wrap. `DeviceList.svelte` itself stays a `<table>`-based consumer post-migration (per Integration Points note in CONTEXT.md: `DeviceList` uses real `<table>`, `ActFormItemsTable` uses `role="row"` divs — the two are NOT unified by one shared component in this phase; `Table`/`TableRow` targets the `<table>` case, `ActFormItemsTable`'s div-grid stays a `Dropdown`-only consumer, not a `Table` consumer).

---

### `ui/src/lib/components/Dropdown.svelte` (component, request-response + event-driven)

**Analog:** `ui/src/features/acts/ActFormItemsTable.svelte` — this is the literal behavioral source of truth (D-01/D-02), not just a stylistic analog. Extraction, not redesign.

**Imports pattern** (lines 11–17):
```svelte
import { onDestroy } from 'svelte';
import Button from '$lib/components/Button.svelte';
import Spinner from '$lib/components/Spinner.svelte';
import { devices } from '$lib/api/devices';
import { portal } from '$lib/utils/portal';
import { dropdownAnchor } from '$lib/utils/dropdownAnchor';
import type { DeviceDto, DeviceGroup } from '../../bindings';
```
`Dropdown.svelte` (living in `lib/components/`) must **reuse** `portal`/`dropdownAnchor` exactly like this — do not reimplement positioning (D-02, "не переписывается"). Since `Dropdown` is generic (not device-specific), the API/data-fetch import (`devices` API) and `DeviceGroup`/`DeviceDto` types stay in the **pilot consumer** (`ActFormItemsTable.svelte`), not in the primitive — `Dropdown` should accept generic option/group shapes via props/snippets, per Claude's Discretion in CONTEXT.md.

**Portal + anchor usage on the panel** (lines 560–566):
```svelte
{#if openByRow[idx]}
  <ul
    class="dropdown--items"
    role="listbox"
    use:portal
    use:dropdownAnchor={{ anchorEl: rowInputEls[idx] }}
    bind:this={rowDropdownEls[idx]}
  >
```
Copy this `use:portal` + `use:dropdownAnchor={{ anchorEl }}` pairing verbatim into `Dropdown.svelte`'s panel element. `dropdownAnchor.ts` (`ui/src/lib/utils/dropdownAnchor.ts`, full file, 70 lines) computes `position: fixed` + flip-up-on-overflow; `Dropdown` needs a real input/field DOM ref (`bind:this`) as `anchorEl` — `Input.svelte` has no ref-forwarding (see comment at line 82–84), so the combobox form's `<input>` must be raw, exactly as `ActFormItemsTable` does at lines 544–556.

**Drill-in state machine — the AUTO-04/D-06/D-07/AUTO-05/D-09 contract to extract (lines 102–279)**:
```ts
let viewModeByRow = $state<Record<number, 'groups' | 'members'>>({});
let drillGroupByRow = $state<Record<number, DeviceGroup | null>>({});
let membersByRow = $state<Record<number, DeviceDto[]>>({});
let showBackByRow = $state<Record<number, boolean>>({});

function isExpandable(g: DeviceGroup): boolean {
  if (g.ids.length <= 1) return false;
  return g.condition_distinct_count > 1 || !!g.repr.serial_no || !!g.repr.inventory_no;
}

async function drillInto(idx: number, g: DeviceGroup, showBack: boolean = true) {
  // ... fetch members, then:
  drillGroupByRow[idx] = g;
  viewModeByRow[idx] = 'members';
  showBackByRow[idx] = showBack;
}

function backToGroups(idx: number) {
  viewModeByRow[idx] = 'groups';
  drillGroupByRow[idx] = null;
  membersByRow[idx] = [];
  showBackByRow[idx] = false;
}
```
For a single (non-per-row) `Dropdown` instance the `Record<number, T>` per-row indirection collapses to plain `$state<T>()` — the *state names and transition rules* are what must be preserved (D-02: "часть компонента, не пропы-опции"), not the row-indexed storage shape (that's `ActFormItemsTable`-specific, since it manages N independent pickers in one table).

**AUTO-05 single-group auto-flatten (lines 218–229)** — must be preserved as unconditional behavior, not an option:
```ts
if (filtered.length === 1) {
  await drillInto(idx, filtered[0], false); // showBack=false — sticky header, no back button
} else {
  viewModeByRow[idx] = 'groups';
  ...
}
```

**Drill-in header markup — the two-independent-conditions nuance (lines 568–589)**:
```svelte
<li class="drill-header">
  {#if showBackByRow[idx]}
    <button type="button" class="drill-back" onmousedown={(e) => e.preventDefault()} onclick={() => backToGroups(idx)}>
      ← Назад
    </button>
  {/if}
  <span class="drill-title">{drillGroupByRow[idx]?.repr.name}...</span>
</li>
```
Header (`drill-title`) is **always** shown in member-view; back button is conditional on `showBackByRow`. UI-SPEC explicitly calls this out as "два независимых условия, а не одно" — do not collapse them into one boolean.

**`onmousedown={(e) => e.preventDefault()}` on every option button (lines 578, 601, 634, 666)** — this prevents the input from losing focus (and closing the dropdown via blur) before the click handler fires. Required on every clickable option/back-button in the new `Dropdown`, including the flat-select variant's checkmark row.

**Keyboard handling — the ARIA gate contract (lines 359–422, 501–512)**:
```ts
function handleFocus(idx: number) {
  if (debounceTimers[idx]) clearTimeout(debounceTimers[idx]);
  void fetchGroups(idx, (items[idx]?.query ?? '').trim());
}

function handleRowKeydown(idx: number, e: KeyboardEvent) {
  if (e.key === 'Escape') { e.preventDefault(); e.stopPropagation(); openByRow[idx] = false; return; }
  if (e.key === 'ArrowDown' && !openByRow[idx]) { e.preventDefault(); handleFocus(idx); return; }
  if (!openByRow[idx]) return;
  if (viewModeByRow[idx] === 'members') {
    if (e.key === 'Enter') { e.preventDefault(); e.stopPropagation(); } // WR-02: suppress form submit
    return; // NOTE: member-mode ArrowUp/Down/Enter navigation is NOT YET implemented here —
            // UI-SPEC D-12 requires Dropdown to ADD this (paritet with groups-mode), see below.
  }
  const list = visibleGroups(idx);
  if (e.key === 'ArrowDown') { /* cyclic index++ */ }
  else if (e.key === 'ArrowUp') { /* cyclic index-- */ }
  else if (e.key === 'Enter') { /* pick active, preventDefault+stopPropagation */ }
  else if (e.key === 'Tab') { /* commit active, close */ }
}
```
This is the **regression floor** (UI-SPEC "Обязательный минимум"): `Enter` suppression in member-mode, `ArrowDown` opens on closed panel, `Escape` closes + `stopPropagation` (does not bubble to modal), `Tab` commits active. **New in this phase (D-12, not yet in this file):** `role="combobox"` + `aria-expanded` + `aria-controls` + `aria-haspopup="listbox"` on the input, `aria-activedescendant` (id pattern precedent below), `ArrowUp/Down/Enter` navigation *inside* member-mode (currently mouse-only), `Home`/`End`, two-stage `Escape` in member-mode (first press → `backToGroups()`, second → close), `scrollIntoView({ block: 'nearest' })` on keyboard nav.

**`aria-activedescendant` id pattern — copy from `PersonAutocomplete.svelte`** (lines 210–211, 234–237 — the only existing precedent in the codebase):
```svelte
aria-autocomplete="list"
aria-activedescendant={activeIndex >= 0 ? `person-autocomplete-item-${activeIndex}` : undefined}
...
<li id={`person-autocomplete-item-${i}`} role="option" aria-selected={i === activeIndex}>
```
`Dropdown.svelte` needs a similar stable `id` per option (e.g. `${uid}-option-${i}` via a generated instance id) for `aria-activedescendant` to reference, since options are portaled out of the component's own DOM subtree.

**Global-class-for-portaled-content pattern (`:global()` — the only correct usage; per Phase 24 Learning #2, `:global()` in *plain* `.scss` files does NOT work, but it DOES work inside a Svelte component's own `<style lang="scss">` block, which is exactly what this file does)** (lines 796–937):
```scss
// Plan 18-04 (AUTO-01): дропдаун перенесён use:portal в <body>, поэтому scoped
// CSS компонента до него (и его потомков) не доходит — нужен :global().
:global(.dropdown--items) {
  position: fixed;
  z-index: 1000;
  max-height: 240px;
  overflow: auto;
  background: var(--tr-surface-raised, var(--tr-surface));
  border: 1px solid var(--tr-border);
  border-radius: var(--tr-radius-xs);
  box-shadow: var(--tr-elev-2);
}
:global(.dropdown--items .opt) { ... }
:global(.dropdown--items .opt:hover),
:global(.dropdown--items .opt.active) { background: var(--tr-surface-sunken); }
```
`Dropdown.svelte`'s panel styling must follow this exact `:global(.<namespaced-root-class> ...)` structure inside its own `<style lang="scss">` block (WR-03 note at lines 801–806: a **namespaced** root class is required — un-namespaced `.dropdown`/`.dropdown-empty` collide across the 4+ components that already portal dropdowns to `<body>`). Panel visual values must be updated to UI-SPEC (`max-height: 280px` grouped / `240px` flat, not the uniform 240px this file currently uses; `margin-top: 4px`; option `min-height: 46px` not implicit).

**Empty/loading state text (canonical, already correct)** (lines 590–591, 656):
```svelte
<li class="dropdown-empty">Ничего не найдено</li>
```
Reuse this exact copy string per UI-SPEC Copywriting Contract; add a loading variant (`Загрузка…` + `<Spinner size="sm" />`, not present in this file yet — `Spinner` import already exists at line 13 and is used inline at line 558 for the input-adjacent spinner, same component to reuse for the panel loading row).

**Field styling (combobox form)** (lines 965–987, `.device-input`):
```scss
.device-input {
  display: block;
  width: 100%;
  height: 36px;
  padding: 0 var(--tr-space-md);
  background: var(--tr-bg);
  border: 1px solid var(--tr-border);
  border-radius: var(--tr-radius-xs);
  &:focus-visible {
    outline: none;
    border-color: var(--tr-accent);
    box-shadow: 0 0 0 3px var(--tr-focus-ring);
  }
  &.invalid {
    border-color: var(--tr-danger);
    box-shadow: 0 0 0 3px var(--tr-danger-ring);
  }
}
```
**Deviation required by UI-SPEC:** field background must be `--tr-surface` (not `--tr-bg`), radius `--tr-radius-sm`/6px (not `--tr-radius-xs`), border `--tr-border-strong` at rest → `--tr-accent` on focus (this file already gets the focus/invalid transitions right — keep those mechanics, fix the token values).

---

### Showcase sections (component, transform)

**Analog for `TableSection`:** `ui/src/features/showcase/sections/TabsSection.svelte` (full file, 63 lines) — canonical structure:
```svelte
<section class="tabs-section">
  <h2>Вкладки</h2>
  <div class="variant-block">
    <h3 class="variant-label">Switch-bar (underline)</h3>
    <Tabs variant="underline" tabs={underlineTabs} bind:active={underlineActive} />
  </div>
</section>

<style lang="scss">
  .tabs-section { display: flex; flex-direction: column; gap: var(--tr-space-lg); }
  h2 { font-size: var(--tr-font-size-h2); font-weight: var(--tr-font-weight-semibold); color: var(--tr-text-primary); }
  .variant-block { display: flex; flex-direction: column; align-items: flex-start; gap: var(--tr-space-sm); }
  .variant-label { font-size: var(--tr-font-size-label); font-weight: var(--tr-font-weight-semibold); color: var(--tr-text-secondary); text-transform: uppercase; }
</style>
```
`h2` → `.variant-block` → `.variant-label` is the exact nesting to replicate for both new sections; `TableSection` needs `variant-block`s for: normal/hover/selected row states, collapsed/expanded group row, all 4 badge tones, mono identifiers, last-row-no-border — all listed explicitly in UI-SPEC "Showcase Contract".

**Analog for `DropdownSection` state-matrix layout:** `ui/src/features/showcase/sections/FieldsSection.svelte` (lines 1–60 shown) — `.field-row` → `.state-group` → `.state-cell` with `.state-tag` labels ("Обычное"/"Ошибка"/"Отключено") is the right template for showing combobox-with-groups (expanded, drill-in), flat-select-with-search, empty state, loading state as labeled side-by-side variants.

**Wiring into `ShowcasePage.svelte`** (full file, lines 1–36):
```svelte
<script lang="ts">
  import ButtonsSection from './sections/ButtonsSection.svelte';
  ...
  import ModalSection from './sections/ModalSection.svelte';
</script>
...
<section class="showcase-block">
  <ModalSection />
</section>
```
Add two more `import` lines + two more `<section class="showcase-block">` blocks after the existing `ModalSection` block, same pattern.

---

## Shared Patterns

### Portal + anchor positioning (do not reimplement)
**Source:** `ui/src/lib/utils/portal.ts` (33 lines, full file read), `ui/src/lib/utils/dropdownAnchor.ts` (70 lines, full file read)
**Apply to:** `Dropdown.svelte` panel element only.
```ts
// portal.ts — moves node to <body>, tags it data-tr-portal (existing detection marker)
export function portal(node: HTMLElement, target: HTMLElement | string = 'body') {
  ...
  node.setAttribute('data-tr-portal', '');
  targetEl.appendChild(node);
  return { destroy() { node.parentNode?.removeChild(node); } };
}
```
```ts
// dropdownAnchor.ts — fixed-position anchor with flip-up-on-overflow, capture-phase scroll listener
export function dropdownAnchor(node: HTMLElement, params: DropdownAnchorParams) {
  function reposition() {
    const rect = anchorEl.getBoundingClientRect();
    node.style.position = 'fixed';
    node.style.left = `${rect.left}px`;
    node.style.width = `${rect.width}px`;
    const spaceBelow = window.innerHeight - rect.bottom;
    if (spaceBelow >= neededHeight) { node.style.top = `${rect.bottom + gap}px`; node.style.bottom = 'auto'; }
    else { node.style.bottom = `${window.innerHeight - rect.top + gap}px`; node.style.top = 'auto'; }
  }
  window.addEventListener('scroll', reposition, true); // capture phase — catches modal-internal scroll
  window.addEventListener('resize', reposition);
  ...
}
```
7 existing selectors + 2 context menus already depend on this pair; SC #5 verification specifically targets the modal-nested pilot (`ActFormItemsTable`) because it's the riskiest portal/scroll-container interaction.

### `Badge` reuse for status + count pill (no hand-rolled pills/badges)
**Source:** `ui/src/lib/components/Badge.svelte` (full file, 221 lines)
**Apply to:** `TableRow.svelte` group-row count and status cell.
```svelte
<Badge variant={statusVariant}>{statusLabel}</Badge>            <!-- status, appearance="soft" implied default -->
<Badge variant="accent" appearance="count">{N} шт.</Badge>      <!-- group count pill -->
```
Geometry (`h=22px`/`radius 11px`/`12px 600` for status; `h=20px`/`padding 0 9px`/`11px 600` for count) already matches `TableRows.dc` — UI-SPEC explicitly forbids re-implementing these.

### `:global()` scoping for portaled dropdown content
**Source:** `ui/src/features/acts/ActFormItemsTable.svelte` lines 796–937 (namespaced `:global(.dropdown--items ...)` block)
**Apply to:** `Dropdown.svelte` panel/option/drill-header styles.
Rule: `:global()` **works** inside a component's own `<style lang="scss">` (compiled by the Svelte compiler), but **does not work** in plain `.scss` files like `global.scss` (compiled by sass/Vite directly) — this was Phase 24's Learning #2 trap. `Dropdown.svelte` must scope every portaled-content rule under one namespaced root class (e.g. `:global(.tr-dropdown-panel ...)`), not bare `:global(.opt)`, to avoid collision with the 4 existing portaled dropdowns.

### Token-gate discipline (`check-tokens.mjs`, closed-world Rule 3)
**Source:** `ui/scripts/check-tokens.mjs` (Rules 1–4, grepped)
**Apply to:** `ui/src/styles/_tokens.scss` edit (`--tr-group`) and every new `<style>` block in `Table.svelte`/`TableRow.svelte`/`Dropdown.svelte`/both showcase sections.
- Rule 2 + 4: **zero** hex literals, **zero** `rgba()/rgb()/hsl()/hsla()` calls inside any `.svelte` `<style>` block — this is why `DeviceGroupRow.svelte`'s current `color-mix(in srgb, var(--tr-accent) 12%, transparent)` pattern must NOT be copied into the new components (color-mix with token args is allowed structurally but UI-SPEC explicitly says derive from `*-soft` tokens instead, per Badge reuse above).
- Rule 3: `--tr-group` must be added to **both** `:root, [data-theme='light']` (insert near lines 61–63, next to `--tr-row-hover`/`--tr-row-selected` under the "Table row states" comment) and `[data-theme='dark']` (near lines 137–139) in `ui/src/styles/_tokens.scss` **before** any `.svelte` file references `var(--tr-group)`, or the build fails closed-world validation (this is verbatim the Phase 24 trap noted in CONTEXT D-09).

```scss
// ui/src/styles/_tokens.scss — insert into the "Table row states" block, light theme (~line 63):
--tr-group: #e9edf5;
// ... and dark theme (~line 139):
--tr-group: #1a212b;
```

### Motion — micro-transitions, theme-switch suppression, reduced-motion
**Source:** `ui/src/styles/global.scss` lines 47–53 (`prefers-reduced-motion`), lines 58–66 (theme-switch suppression class)
**Apply to:** All new `.12s`/`.15s` transitions in `TableRow.svelte` (row background), `Dropdown.svelte` (field border/shadow) — these are global `<style>`-block mechanisms already applying to any `transition:` declared in scoped component styles; new components must not add a competing global override, just declare `transition: background-color .12s` etc. locally and let the existing suppression mechanisms handle the rest.

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| Flat-select form of `Dropdown` (field shows selected value + `▼`, search box inside panel) | component (sub-variant) | request-response | No existing component in the codebase does "value display + in-panel search box" — `Select.svelte` wraps a native `<select>` (browser-native popup, no portal needed) and `ActFormItemsTable`/`PersonAutocomplete`/`LocationAutocomplete` are all combobox-form (type-to-search in the field itself). This sub-variant is genuinely new; UI-SPEC's own pixel values (`searchBoxStyle`: h=30px, `--tr-surface-sunken`, radius 5px, icon `⌕`) are the only source of truth — use `Dropdown.dc.html` directly, not codebase precedent. |

## Metadata

**Analog search scope:** `ui/src/lib/components/`, `ui/src/features/devices/`, `ui/src/features/acts/`, `ui/src/features/showcase/`, `ui/src/lib/utils/`, `ui/src/styles/_tokens.scss`, `ui/scripts/check-tokens.mjs`
**Files scanned:** 17 (Badge, Checkbox, Spinner, Select, PersonAutocomplete grep, DeviceList, DeviceListRow, DeviceGroupRow, ActFormItemsTable, portal.ts, dropdownAnchor.ts, TabsSection, FieldsSection, ShowcasePage, _tokens.scss, global.scss grep, check-tokens.mjs grep)
**Pattern extraction date:** 2026-07-19

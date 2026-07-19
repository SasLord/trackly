# Phase 26: Окна с готовым макетом - Pattern Map

**Mapped:** 2026-07-19
**Files analyzed:** 16 (3 new, 13 modified)
**Analogs found:** 16 / 16 (2 with only partial/structural analogs — see §No Analog Found)

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `ui/src/lib/components/PageHeader.svelte` (NEW) | component (layout) | request-response (static props + snippet render) | `ui/src/lib/components/Tabs.svelte` (variant-prop pattern) + `DevicesPage.svelte`/`DashboardPage.svelte` headers (content being extracted) | role-match |
| `ui/src/styles/_breakpoints.scss` (NEW) | config | — | `ui/src/styles/_tokens.scss` (SCSS partial structure/section-comment convention) | partial (no breakpoint file precedent exists) |
| `ui/src/features/layout/layout-state.svelte.ts` (NEW) | store (rune state) | event-driven | `ui/src/lib/stores/theme.svelte.ts` | exact |
| `ui/src/features/layout/Layout.svelte` | provider/layout | request-response | itself (existing file) + `ui/src/lib/components/Modal.svelte` (backdrop/focus/escape pattern for the new drawer) | role-match |
| `ui/src/features/layout/Sidebar.svelte` | component (nav) | request-response | itself (existing file) | exact |
| `ui/src/lib/components/ThemeSwitcher.svelte` | component | request-response | `ui/src/lib/components/Tabs.svelte` (`variant="segmented"` block) | exact (target styling is structurally identical) |
| `ui/src/features/dashboard/DashboardPage.svelte` | component (page) | request-response | itself + `ui/src/features/devices/DevicesPage.svelte` (header consolidation reference) | exact |
| `ui/src/features/dashboard/StatWidget.svelte` | component | request-response | itself (existing file) | exact |
| `ui/src/features/dashboard/ChartWidget.svelte` | component | request-response | itself (existing file) | exact |
| `ui/src/features/dashboard/PeriodToggle.svelte` | component | request-response | itself + `ui/src/lib/components/Tabs.svelte` (`variant="underline"` — role differs: `group`, not `tablist`) | role-match |
| `ui/src/features/devices/DevicesPage.svelte` | component (page) | CRUD (via `devices` service) | itself + `ui/src/features/dashboard/DashboardPage.svelte` | exact |
| `ui/src/features/devices/DeviceFilters.svelte` | component | event-driven (filter callbacks) | `ui/src/lib/components/Input.svelte`, `ui/src/lib/components/Tabs.svelte` (`underline`), `ui/src/lib/components/Checkbox.svelte` | exact |
| `ui/src/lib/components/Table.svelte` | component (shared shell) | CRUD (list rendering) | itself (existing file); `footer` prop mirrors `ui/src/lib/components/Modal.svelte`'s optional `footer?: Snippet` | exact |
| `ui/src/features/devices/DeviceList.svelte` | component | CRUD | itself (existing file) | exact |
| `ui/src/lib/components/Input.svelte` | component | request-response | itself (existing file); `iconLeft` slot precedent: `ui/src/features/devices/DeviceFilters.svelte` current inline `.search-icon` markup (being replaced) | exact |
| `ui/src/styles/_tokens.scss` | config | — | itself (single value edit) | exact |

**Not touched (explicit KEEP per D-09/D-12):** `ui/src/features/layout/sidebar-config.ts`, `ui/src/features/devices/DeviceListRow.svelte`, `ui/src/features/devices/DeviceGroupRow.svelte`, `ui/src/features/layout/EmployeeLayout.svelte`, `ui/src/features/acts/ActFormItemsTable.svelte` (confirmed NOT a `Table` consumer — see UI-SPEC §6.5).

---

## Pattern Assignments

### `ui/src/lib/components/PageHeader.svelte` (NEW — component, request-response)

**No direct analog exists** — this is the first shared page-header primitive. Pattern is assembled from three sources:

**Variant-prop pattern, analog `ui/src/lib/components/Tabs.svelte` (lines 9-24):**
```ts
interface Props {
  variant?: 'underline' | 'segmented';
  tabs: Tab[];
  active: string;
  onchange?: (_key: string) => void;
  ariaLabel?: string;
}

let { variant = 'underline', tabs, active = $bindable(), onchange, ariaLabel }: Props = $props();
```
Copy the `variant?: 'fixed' | 'wrap'` prop shape and the `{#if variant === 'x'}...{:else}...{/if}` root-level branching (or a single class binding driven by `variant`) from this file, not from a page component.

**Content being extracted into the new component** — `DashboardPage.svelte` (lines 137-163) and `DevicesPage.svelte` (lines 228-236) both currently own an inline `<header class="page-header">`. Read both before writing `PageHeader.svelte` — the `wrap` variant must reproduce `DevicesPage.svelte`'s `flex-wrap: wrap` header exactly (UI-SPEC §3.7), the `fixed` variant reproduces `DashboardPage.svelte`'s (UI-SPEC §3.6).

**Snippet-slot pattern for `actions`, analog `ui/src/lib/components/Modal.svelte` (lines 1-13):**
```ts
interface Props {
  open: boolean;
  title: string;
  size?: 'md' | 'wide' | 'xwide' | 'pdf-preview';
  onClose: () => void;
  children?: Snippet;
  footer?: Snippet;
}
```
`Modal`'s optional `footer?: Snippet` (undefined → nothing rendered, same as UI-SPEC §6.1's `actions?: Snippet`) is the established convention for "optional named slot, absent = not rendered."

**Error handling / validation:** none — this is a pure presentational component, no async, no try/catch.

---

### `ui/src/features/layout/layout-state.svelte.ts` (NEW — store, event-driven)

**Analog:** `ui/src/lib/stores/theme.svelte.ts` (full file, 32 lines)

```ts
// .svelte.ts extension REQUIRED — Svelte 5 runes are only processed in .svelte/.svelte.ts files.

type Resolved = 'light' | 'dark';
type Preference = 'light' | 'dark' | 'system';

export const themeStore = $state({
  preference: 'system' as Preference,
  resolved: 'light' as Resolved,
});
```

Copy exactly this shape for `layout-state.svelte.ts`:
```ts
export const sidebarNav = $state({ open: false });
```
No `localStorage` persistence needed (UI-SPEC §6.3 doesn't call for it — drawer state resets per session). Mutator functions (`openNav()`/`closeNav()`) should follow `setTheme()`'s pattern of a plain exported function mutating the `$state` object directly (lines 26-29 of `theme.svelte.ts`).

---

### `ui/src/features/layout/Layout.svelte` (MODIFIED — provider/layout, request-response)

**Current file (full, 61 lines) — self-analog for the parts that don't change:**
```svelte
<div class="app-layout">
  <aside class="sidebar-container">
    <Sidebar />
  </aside>
  <main id="main" class="content">
    {@render children?.()}
  </main>
</div>

<style lang="scss">
  .app-layout {
    display: grid;
    grid-template-columns: var(--sidebar-width) 1fr;
    min-height: 100vh;
  }
  .content {
    padding: var(--tr-space-xl);
    overflow: auto;
    min-height: 100vh;
    background: var(--tr-bg);
  }
</style>
```
**Required changes (D-06/D-07, UI-SPEC §3.8):** remove `padding` and `background` from `.content` — these move to each page's body per PageHeader contract. `.content` keeps `overflow: auto` (renamed responsibility only).

**Drawer/backdrop pattern for `< 1024px` (UI-SPEC §6.3), analog `ui/src/lib/components/Modal.svelte` (lines 130-180, 169-179):**
```svelte
<svelte:window onkeydown={open ? handleKeydown : undefined} />

{#if open}
  <div class="modal-backdrop" onmousedown={...} onmouseup={...} aria-modal="true" role="dialog" tabindex="-1">
    ...
  </div>
{/if}
```
```scss
.modal-backdrop {
  position: fixed;
  inset: 0;
  background: var(--tr-overlay);
  z-index: 500;
}
```
Copy the `position: fixed; inset: 0; background: var(--tr-overlay)` backdrop rule, the `Escape`-key handler wired through `<svelte:window onkeydown>`, and the focus-restore-on-close `$effect` (lines 67-86) — UI-SPEC §6.3 requires the same behaviors (focus moves to first nav link on open, returns to burger on close, `inert` on closed panel). `z-index` for the nav drawer must be lower than Modal's 500 (UI-SPEC specifies 55/60) so a Modal opened from a page while the drawer is open still stacks correctly.

**Error handling:** none — layout has no async/error states.

---

### `ui/src/features/layout/Sidebar.svelte` (MODIFIED — component, request-response)

**Current file (full, 217 lines)** — read above. Key excerpts to change per UI-SPEC §3.1-3.4:

**Imports (lines 1-8, unchanged):**
```ts
import { link } from 'svelte-spa-router';
import active from 'svelte-spa-router/active';
import { getVisibleItems } from './sidebar-config';
import { authStore } from '$lib/stores/auth.svelte';
import type { UserRole } from '$lib/stores/auth.svelte';
import ThemeSwitcher from '$lib/components/ThemeSwitcher.svelte';
import { apiCall } from '$lib/api/client';
```

**Active-nav-link pattern (lines 127-132) — DO NOT "fix" the `:global()`:**
```scss
:global(.nav-link.is-active) {
  border-left-color: var(--tr-accent);
  background: color-mix(in srgb, var(--tr-accent) 10%, transparent);
  color: var(--tr-text-primary);
  font-weight: var(--tr-font-weight-medium);
}
```
Per D-06/UI-SPEC §3.3, this block's declarations change (border-left → `box-shadow: inset 3px 0 0 var(--tr-accent)`, background → `var(--tr-accent-soft)`, color → `var(--tr-accent-text)`, weight → 600) but the `:global(.nav-link.is-active)` selector wrapper stays — it is a scoped-component `:global()`, not a plain-`.scss` one (24-LEARNINGS pitfall does not apply here, confirmed in UI-SPEC §3.3 note).

**Footer/theme-row copy change (lines 73-76):**
```svelte
<div class="theme-row">
  <span class="theme-label">Тема</span>
  <ThemeSwitcher />
</div>
```
→ text "Тема" becomes "Оформление" (D-08); logout button block (lines 62-71) and `sidebar-config.ts` import stay untouched.

**Error handling:** `logout()` (lines 25-38) try/catch/finally pattern is unchanged — not touched by this phase.

---

### `ui/src/lib/components/ThemeSwitcher.svelte` (MODIFIED — component, request-response)

**Analog:** `ui/src/lib/components/Tabs.svelte`, `.tabs-segmented` block (lines 131-165) — this is the styling target UI-SPEC §3.5 describes almost verbatim:
```scss
.tabs-segmented {
  display: inline-flex;
  gap: 3px;
  padding: 3px;
  background: var(--tr-surface-sunken);
  border-radius: 7px;

  .tab {
    ...
    &.active {
      background: var(--tr-surface);
      color: var(--tr-accent-text);
      box-shadow: var(--tr-elev-1);
    }
    &:focus-visible {
      box-shadow: 0 0 0 3px var(--tr-focus-ring);
    }
  }
}
```
Apply the same `padding + gap` on sunken background + per-button `box-shadow: var(--tr-elev-1)` on the active segment idiom to `.theme-switcher`/`.segment` (currently lines 26-70 of `ThemeSwitcher.svelte`, no padding/gap, `border-right` dividers instead of `gap`). Also reorder `options` array (lines 4-8) to Светлая · Системная · Тёмная (D-08/UI-SPEC §3.5 row "порядок кнопок").

**Transition pattern** — currently `transition: none` (line 48); change to `background .12s, color .12s` matching `Tabs.svelte`'s `transition: background 0.12s, box-shadow 0.12s;` (line 80-82) and `Button.svelte`'s `transition: background 0.12s, box-shadow 0.12s;` (line 46-48) — this is the project-wide micro-transition convention from Phase 24.

---

### `ui/src/features/dashboard/DashboardPage.svelte` (MODIFIED — component, request-response)

**Self-analog, current header block (lines 137-163) and grid (lines 165-241, 290-311):** async data-loading (`loadWidgets`/`loadChart`, lines 76-134) is untouched — D-14. Only the `<header class="page-header">` markup is replaced by `<PageHeader title="Дашборд" variant="fixed">{#snippet actions()}...{/snippet}</PageHeader>`, and `.dashboard-grid` changes from `grid-template-columns: 3fr 2fr` (line 294) to `repeat(4,1fr)` stat row + full-width chart below (D-02).

**Error string change (both `StatWidget` and `ChartWidget` must be updated together — UI-SPEC §9):** currently `'Ошибка загрузки'` in both `StatWidget.svelte:41` and `ChartWidget.svelte:225` → `'Не удалось загрузить. Смените период или обновите страницу.'`. This is a copy-only change referencing `reloadWidgets()` (line 128) and the chart's `$effect` on `windowMonths` (lines 121-126), neither of which is modified.

---

### `ui/src/features/dashboard/StatWidget.svelte` (MODIFIED — component, request-response)

**Self-analog (full file, 143 lines).** Key excerpts requiring change per UI-SPEC §3.10-3.11:

**Card shell (lines 66-72):**
```scss
.stat-widget {
  background: var(--tr-surface);
  border: 1px solid var(--tr-border);
  border-radius: var(--tr-radius-md);
  padding: var(--tr-space-xl);
  min-height: 120px;
}
```
→ `padding: 16px` (was 24px), add `box-shadow: var(--tr-elev-1); min-width: 0;` (keep `min-height: 120px` — D-14).

**Breakdown → pill row (lines 95-105):** current `<ul class="breakdown-list">` list markup is replaced by flex-wrap pill row per UI-SPEC §3.10 (`display:flex; flex-wrap:wrap; gap:6px`, each pill `padding:3px 9px; border-radius:11px; background:var(--tr-surface-sunken)`, value in `<strong>`). **Do not reach for `Badge.svelte`** — confirmed in UI-SPEC §3.10 that `Badge` (`ui/src/lib/components/Badge.svelte` lines 90-105, `badge-m-count`) doesn't support the "label + strong value" pair shape; write local markup instead.

**Warning block (lines 120-142, D-04):** keep the `<div class="widget-warning">` structure and copy ("Низкий остаток:" + list), only retone: `background: var(--tr-warning-soft)` (was `color-mix(--tr-warning 10%)`), remove the solid `1px solid var(--tr-warning)` border, header text `color: var(--tr-warning-text)`.

---

### `ui/src/features/dashboard/ChartWidget.svelte` (MODIFIED — component, request-response)

**Self-analog (full file, 453 lines).** Two distinct kinds of change:

1. **Pure restyle** (card shell lines 354-360, header lines 362-369, tooltip/labels) — follow UI-SPEC §3.12 value table line-by-line; no logic touched (`barLayout`, `yTicks`, `seriesData` derivations at lines 80-211 are untouched).
2. **Color literal change (line 24):**
```ts
const COLORS = ['var(--tr-accent)', 'var(--tr-success)', 'var(--tr-warning)'];
```
→ per UI-SPEC §7 "Согласованные литералы", becomes hardcoded hex literals `['#3b6fe0', '#1a9d5f', '#d8820e']` (same in both themes) — this is a documented exception from the `--tr-*` token gate (`check-tokens.mjs` doesn't scan these, they're not tokens).

**Error string** — see DashboardPage entry above; `ChartWidget.svelte:225` is one of the two files that must change together.

---

### `ui/src/features/dashboard/PeriodToggle.svelte` (MODIFIED — component, request-response)

**Self-analog (full file, 65 lines).** Currently tab-like styling (`.toggle-btn` lines 33-63) mirrors the OLD `DeviceFilters` status-tab pattern (comment at line 3 says so explicitly: "Паттерн: status-bar tabs из CartridgeFilters.svelte"). UI-SPEC §3.12 replaces this with the `.dc` `pStyle` values: `padding: 2px 1px 5px`, active `border-bottom-color: var(--tr-accent)` + `color: var(--tr-accent-text)` + weight 600, inactive weight 500 + `color: var(--tr-text-secondary)`. **Role stays `role="group"` / `aria-label="Период графика"` (line 13) — NOT `tablist`.** Do not convert to `ui/src/lib/components/Tabs.svelte`'s underline variant wholesale; only borrow the CSS values, since `Tabs`'s `role="tablist"`/`role="tab"` semantics don't apply here (this is a toggle-group, not tabs).

---

### `ui/src/features/devices/DevicesPage.svelte` (MODIFIED — component, CRUD)

**Self-analog, header block (lines 228-236) and `.page-content` (line 238, 348-352):**
```svelte
<header class="page-header">
  <h1 class="page-title">Устройства</h1>
  <div class="header-actions">
    <Button variant="primary" onclick={openCreate}>+ Создать устройство</Button>
    <Button variant="secondary" onclick={() => (csvModalOpen = true)}>Импорт CSV</Button>
    <Button variant="secondary" onclick={exportCsv}>Экспорт CSV</Button>
  </div>
</header>
```
Migrates to `<PageHeader title="Устройства" variant="wrap">{#snippet actions()}<Button .../>...{/snippet}</PageHeader>`. All three `Button` calls, `openCreate`/`exportCsv` handlers (lines 142-225) are unchanged — only the wrapping markup moves. `.page-content` padding changes `24px 32px` → `20px 24px` (UI-SPEC §3.7).

---

### `ui/src/features/devices/DeviceFilters.svelte` (MODIFIED — component, event-driven)

**Three analogs, one per primitive being adopted (D-10):**

**1. Search input → `ui/src/lib/components/Input.svelte` (full file, 78 lines):**
```svelte
<input
  {type}
  {id}
  {placeholder}
  {disabled}
  class="input"
  class:invalid
  {value}
  oninput={(e) => { const v = (e.currentTarget as HTMLInputElement).value; value = v; oninput?.(v); }}
/>
```
```scss
.input {
  height: 36px;
  padding: 0 var(--tr-space-md);
  background: var(--tr-surface);
  border: 1px solid var(--tr-border-strong);
  border-radius: var(--tr-radius-sm);
}
```
Replaces the hand-rolled `.search-input` (`DeviceFilters.svelte` lines 140-160, currently `background: var(--tr-bg); border: var(--tr-border); radius: var(--tr-radius-xs)`). The debounce logic (`localSearch`, `debounceTimer`, `handleSearchInput`, lines 26-39) is preserved unchanged — only the markup/CSS around it is replaced; `Input`'s `oninput` callback signature (`(_value: string) => void`) already matches `handleSearchInput`'s shape.

**2. Status tabs → `ui/src/lib/components/Tabs.svelte`, `.tabs-underline` block (lines 57-129):**
```svelte
<div class="tabs tabs-underline" role="tablist" aria-label={ariaLabel}>
  {#each tabs as tab (tab.key)}
    <button type="button" class="tab" class:active={tab.key === active} ...>
      {tab.label}
      {#if variant === 'underline' && tab.count != null}<span class="tab-count">{tab.count}</span>{/if}
    </button>
  {/each}
</div>
```
Replaces `.status-bar`/`.status-tab`/`.count-badge` (current `DeviceFilters.svelte` lines 85-101, 176-221). **Hard contract (D-10 risk):** `STATUSES` array (lines 41-47), `onStatusChange` signature, and the "Все · На складе · В работе · На ремонте · Списано" order must be preserved verbatim — only the rendering primitive changes, feed `Tabs` a `tabs: Tab[]` derived from `STATUSES`+`counts`, `active={String(statusFilter)}` (or equivalent), `onchange` calling `onStatusChange`.

**3. Group checkbox → `ui/src/lib/components/Checkbox.svelte` (full file, 120 lines):**
```svelte
<label class="check-row" class:disabled>
  <span class="box-wrap">
    <input type="checkbox" bind:checked {disabled} {id} class="native-input" onchange={() => onchange?.(checked)} />
    <span class="box" class:invalid aria-hidden="true"></span>
  </span>
  {@render children?.()}
</label>
```
Replaces `.group-toggle`/`.group-checkbox` (current lines 103-111, 223-243) — native `16×16` + `accent-color` becomes `Checkbox`'s custom `18×18` box with `::after` checkmark. `onGroupedChange` wiring: `<Checkbox checked={grouped} onchange={onGroupedChange}>Группировать похожие</Checkbox>`.

---

### `ui/src/lib/components/Table.svelte` (MODIFIED — component, CRUD)

**Self-analog (full file, 147 lines).** Adding two props per UI-SPEC §6.5:
```ts
/** Рамка по макету: border + radius 8px + overflow hidden + elev-1. */
framed?: boolean;   // default: true
/** Полоса итога внутри рамки, под скроллером таблицы. */
footer?: Snippet;   // default: undefined — полоса не рендерится
```

**Optional-snippet convention, analog `ui/src/lib/components/Modal.svelte` (lines 9-13, 150-154):**
```ts
footer?: Snippet;
```
```svelte
{#if footer}
  <footer class="modal-footer">
    {@render footer()}
  </footer>
{/if}
```
Copy this exact `{#if footer}...{@render footer()}{/if}` guard — `Table.svelte` already uses the same `{@render head()}`/`{@render children?.()}` idiom internally (lines 37, 63), so the new `footer` slot is consistent with the file's own conventions, reinforced by `Modal`'s prior art for an *optional* footer.

**`framed` wrapper** — new outer `<div class="tr-table-framed">` around the existing `.tr-table-wrapper` (line 33), applying `border: 1px solid var(--tr-border); border-radius: 8px; overflow: hidden; box-shadow: var(--tr-elev-1)`. Per UI-SPEC §3.14, `overflow: hidden` (framed) and `overflow-x: auto` (`.tr-table-wrapper`, line 71-73, unchanged) are two different elements — do not merge them.

---

### `ui/src/features/devices/DeviceList.svelte` (MODIFIED — component, CRUD — LIMITED scope per D-12)

**Self-analog, footer block (lines 107-118, 142-154):**
```svelte
{#if !skeletonLoading && !isEmpty}
  <footer class="list-footer">
    <span class="pagination-info">
      {#if showGroups}
        Групп: {groups.length}
      {:else}
        Показано {items.length} из {total}
      {/if}
    </span>
  </footer>
{/if}
```
**Permitted change:** wrap this exact block in `{#snippet footer()}...{/snippet}`, pass `{footer}` to `<Table>` (line 74-81), delete the outer `.device-list-wrapper` div (lines 73, 120-124 — its `overflow-x: auto` now lives in `Table`'s internal wrapper).
**Forbidden (D-12):** touching `{#if !skeletonLoading && !isEmpty}` condition, the two message strings, `emptyMessage`/`emptySubtext` (lines 52-59), or any `$derived` (lines 41-59). If the planner judges the footer-relocation itself too risky, the documented fallback (UI-SPEC §6.5) is: `Table` gets only `framed`, footer stays outside as-is, and this is logged as an accepted deviation from У:104.

---

### `ui/src/lib/components/Input.svelte` (MODIFIED — component, request-response)

**Self-analog (full file, 78 lines).** Adding one prop per UI-SPEC §6.6:
```ts
/** Иконка слева внутри поля. Задана — поле получает padding-left:34px. */
iconLeft?: Snippet;
```
Positioning excerpt to copy (from `DeviceFilters.svelte`'s current, being-replaced `.search-icon`, lines 131-138):
```scss
.search-icon {
  position: absolute;
  left: var(--tr-space-xs);
  color: var(--tr-text-tertiary);
  pointer-events: none;
  display: flex;
  align-items: center;
}
```
Adjust `left` to `12px` per UI-SPEC §3.13, wrap the `<input>` in a `position: relative` container only when `iconLeft` is passed, conditionally add `padding-left: 34px` to `.input` via a class binding. Must be backward-compatible — no `iconLeft` prop passed on any of `Input`'s other current call sites means zero layout change (UI-SPEC §6.6 explicit constraint).

---

### `ui/src/styles/_tokens.scss` (MODIFIED — config)

**Self-analog, layout-constants section (lines 185-192):**
```scss
// ── Layout constants (unchanged — out of scope for the --tr-* migration, D-02) ───────────────
--sidebar-width: 240px;
--header-height: 56px;
```
Single value edit: `--sidebar-width: 240px` → `236px`. This is NOT a `--tr-*` token so `check-tokens.mjs`'s closed-world gate does not need an allow-list update (confirmed UI-SPEC §7).

---

## Shared Patterns

### Micro-transitions (Phase 24 convention)
**Source:** `ui/src/lib/components/Button.svelte` lines 46-48, `ui/src/lib/components/Tabs.svelte` lines 80-82
**Apply to:** `ThemeSwitcher.svelte` segment transitions, `PeriodToggle.svelte`, any newly restyled interactive element
```scss
transition:
  background 0.12s,
  box-shadow 0.12s;
```

### Optional `Snippet` prop — "absent = not rendered"
**Source:** `ui/src/lib/components/Modal.svelte` lines 9-13, 150-154; also `ui/src/lib/components/Table.svelte` lines 16-18, 62-65 (existing `children?: Snippet`)
**Apply to:** `PageHeader.actions`, `Table.footer`, `Input.iconLeft`
```ts
footer?: Snippet;   // default: undefined
```
```svelte
{#if footer}
  <footer>{@render footer()}</footer>
{/if}
```

### `:global()` in scoped `<style lang="scss">` — allowed; in plain `.scss` — forbidden
**Source:** `ui/src/features/layout/Sidebar.svelte` line 127 (`:global(.nav-link.is-active)`), confirmed working; contrast with 24-LEARNINGS.md prohibition on `:global()` inside `global.scss`.
**Apply to:** any active-state class applied via `use:active` (svelte-spa-router) inside a Svelte component's own `<style>` block — do NOT "fix" these into non-global selectors.

### Focus-trap / backdrop / Escape-to-close
**Source:** `ui/src/lib/components/Modal.svelte` lines 67-127, 130, 169-179
**Apply to:** the new mobile sidebar drawer in `Layout.svelte` (UI-SPEC §6.3) — reuse the `<svelte:window onkeydown>` Escape wiring, the focus-restore `$effect`, and the `position: fixed; inset: 0; background: var(--tr-overlay)` backdrop rule. Z-index must sit below Modal's `500` (use 55/60 per UI-SPEC).

### Rune-based global UI state (`.svelte.ts`)
**Source:** `ui/src/lib/stores/theme.svelte.ts` (full file)
**Apply to:** `ui/src/features/layout/layout-state.svelte.ts`
```ts
// .svelte.ts extension REQUIRED — Svelte 5 runes are only processed in .svelte/.svelte.ts files.
export const sidebarNav = $state({ open: false });
```

### Error-string duplication trap
**Source:** `ui/src/features/dashboard/StatWidget.svelte:41` and `ui/src/features/dashboard/ChartWidget.svelte:225` — same literal string, two files
**Apply to:** planner must assign both files to the same task/plan when changing this copy — asymmetric completion is a visible regression within one screen.

---

## No Analog Found

| File | Role | Data Flow | Reason |
|---|---|---|---|
| `ui/src/styles/_breakpoints.scss` | config | — | First SCSS-variable breakpoints file in the codebase — no prior `$bp-*` convention exists anywhere (`grep` for `@media`/`$bp-` across `ui/src` returned no plain-SCSS-variable breakpoint file). Structural precedent only: `_tokens.scss`'s section-comment convention (`// ── Section name ──`). Values themselves come from UI-SPEC §6.2, not from an existing file. |

---

## Metadata

**Analog search scope:** `ui/src/lib/components/`, `ui/src/features/layout/`, `ui/src/features/dashboard/`, `ui/src/features/devices/`, `ui/src/lib/stores/`, `ui/src/styles/`, `ui/src/features/showcase/sections/` (Table consumer check).
**Files scanned:** ~30 (all components in `ui/src/lib/components/`, all files in the four touched feature directories, both rune-store files consulted, `_tokens.scss` layout-constants section, `TableSection.svelte` for `Table` consumer regression scope).
**Pattern extraction date:** 2026-07-19

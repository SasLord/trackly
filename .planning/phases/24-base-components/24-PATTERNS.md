# Phase 24: Базовые компоненты - Pattern Map

**Mapped:** 2026-07-18
**Files analyzed:** 13 (5 modified primitives + 3 new primitives + showcase page + route + sidebar + theme hook + global.scss)
**Analogs found:** 13 / 13

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `ui/src/lib/components/Checkbox.svelte` (NEW) | component (form input) | request-response (local state, `bind:checked`) | `ui/src/lib/components/Input.svelte` | role-match (runes shape identical, DOM primitive differs: `<input>` text vs hidden `<input type=checkbox>` + styled span) |
| `ui/src/lib/components/Radio.svelte` (NEW) | component (form input) | request-response (local state, `bind:group`) | `ui/src/lib/components/Input.svelte` + native `bind:group` semantics | role-match |
| `ui/src/lib/components/Tabs.svelte` (NEW) | component (navigation/switch) | event-driven (click → active-tab change) | `ui/src/features/requests/RequestsSearchAndTabs.svelte` (interaction shape) + `ui/src/lib/components/ThemeSwitcher.svelte` (segmented variant) + `ui/src/lib/components/Button.svelte` (variant-prop styling convention) | role-match (self-rolled tab bar exists but is feature-local, not a reusable primitive; Tabs.svelte generalizes it) |
| `ui/src/pages/ComponentShowcasePage.svelte` (NEW, thin wrapper) + `ui/src/features/showcase/ComponentShowcasePage.svelte` (NEW, feature body) | route/page | request-response (static demo render, no data fetch) | `ui/src/pages/UsersPage.svelte` (thin wrapper) + `ui/src/features/users/UsersPage.svelte` (feature body shell) | exact (thin-wrapper-imports-feature pattern is the established convention for every route) |
| `ui/src/routes.ts` (MODIFY) | route | request-response | itself — existing `routes` map | exact |
| `ui/src/features/layout/sidebar-config.ts` (MODIFY) | config | request-response | itself — existing `SIDEBAR_ITEMS` + `/users`/`/settings` entries | exact |
| `ui/src/lib/components/Badge.svelte` (MODIFY) | component | request-response | itself — current prop shape | exact |
| `ui/src/lib/components/ThemeSwitcher.svelte` / `ui/src/lib/stores/theme.svelte.ts` (MODIFY) | store/hook | event-driven (theme toggle → DOM mutation) | itself — `applyResolved()` is the single mutation point | exact |
| `ui/src/lib/components/Button.svelte` (MODIFY) | component | request-response | itself — current variant/state CSS | exact |
| `ui/src/lib/components/Input.svelte`, `Select.svelte`, `Textarea.svelte` (MODIFY) | component (form input) | request-response | itself — current `ctrlBase`-equivalent CSS | exact |
| `ui/src/lib/components/Modal.svelte` (MODIFY) | component (overlay) | request-response | itself — current container CSS | exact |

## Pattern Assignments

### `ui/src/lib/components/Checkbox.svelte` (NEW component, request-response)

**Analog:** `ui/src/lib/components/Input.svelte` (runes shape) — no checkbox/radio precedent exists in-repo; DOM structure is synthesized per RESEARCH.md Pattern 2 (hidden native input + styled sibling span), not copied from any file.

**Runes/Props pattern to copy** (from `ui/src/lib/components/Input.svelte:1-23`):
```typescript
interface Props {
  type?: 'text' | 'number' | 'search';
  value: string;
  placeholder?: string;
  disabled?: boolean;
  invalid?: boolean;
  id?: string;
  'aria-describedby'?: string;
  oninput?: (_value: string) => void;
}

const {
  type = 'text',
  value = $bindable(''),
  placeholder,
  disabled = false,
  invalid = false,
  id,
  'aria-describedby': ariaDescribedby,
  oninput,
}: Props = $props();
```
Adapt for Checkbox: replace `value: string` with `checked = $bindable(false)`, drop `type`/`placeholder`, keep `disabled`/`invalid`/`id`, add `children?: Snippet` for the label text (see Select.svelte's `Snippet` usage below), event prop becomes `onchange?: (_checked: boolean) => void`.

**`Snippet` children pattern** (from `ui/src/lib/components/Select.svelte:1,7,15,24-25,40`):
```typescript
import type { Snippet } from 'svelte';
interface Props {
  // ...
  children?: Snippet;
}
const { /* ... */ children }: Props = $props();
```
```svelte
{@render children?.()}
```

**Native-input-driven visual state pattern (synthesized shape, not copied — see RESEARCH.md Pattern 2):**
```svelte
<label class="check-row" class:disabled>
  <span class="box-wrap">
    <input type="checkbox" bind:checked={checked} {disabled} class="native-input" />
    <span class="box" aria-hidden="true"></span>
  </span>
  {@render children?.()}
</label>
```
Drive `.box`'s border/background/checkmark purely via CSS sibling selectors on `.native-input:checked`,
`:focus-visible`, `:disabled` — do not hand-roll `role="checkbox"` + keydown handlers.

**Exact values to transcribe (from RESEARCH.md "Fields" section, sourced from `Fields.dc.html`):**
```
box: width:18px; height:18px; flex:none; display:inline-flex; align-items:center; justify-content:center;
     border:1.5px solid var(--tr-border-strong); background: var(--tr-surface); box-sizing:border-box;
     border-radius: 5px (checkbox)
checked: background: var(--tr-accent); border-color: var(--tr-accent);
focus:   box-shadow: 0 0 0 3px var(--tr-focus-ring); border-color: var(--tr-accent);
disabled: background: var(--tr-surface-sunken); border-color: var(--tr-border);
checkmark (checked): width:10px; height:6px; border-left:2px solid var(--tr-on-accent);
     border-bottom:2px solid var(--tr-on-accent); transform: rotate(-45deg) translate(0,-1px);
row (label wrapper): display:inline-flex; align-items:center; gap:10px; font-size:14px;
     color: var(--tr-text-primary); cursor:pointer;
row disabled: color: var(--tr-text-disabled); cursor:not-allowed;
```

**Error-state handling pattern** (from `ui/src/lib/components/Input.svelte:65-68`):
```scss
&.invalid {
  border-color: var(--tr-danger);
  box-shadow: 0 0 0 3px var(--tr-danger-ring);
}
```
Note: Checkbox/Radio in `Fields.dc.html` don't show an explicit error/invalid box state — if CMP-02 requires
it, reuse this same `--tr-danger`/`--tr-danger-ring` pair on `.box` the same way Input applies it to itself.

**Disabled-state handling pattern** (from `ui/src/lib/components/Input.svelte:70-74`):
```scss
&:disabled {
  background: var(--tr-surface-sunken);
  color: var(--tr-text-tertiary);
  cursor: not-allowed;
}
```

---

### `ui/src/lib/components/Radio.svelte` (NEW component, request-response)

**Analog:** same as Checkbox.svelte (`Input.svelte` for runes shape). Structural difference: circular box
(`border-radius: 50%`) instead of 5px, inner dot instead of checkmark, and native `bind:group` for
mutual-exclusivity instead of a boolean `checked`.

**Exact values (from RESEARCH.md "Fields" section):**
```
box: same base as Checkbox but border-radius: 50%
checked dot: width:8px; height:8px; border-radius:50%; background: var(--tr-on-accent);
```

**Group-binding decision (RESEARCH.md Open Question 2, Recommendation):** expose `group = $bindable()` on
`Radio.svelte`, forwarding to the native `<input type="radio" bind:group={group}>` — do not invent a custom
event-based group-sync mechanism. This mirrors the `value = $bindable('')` convention already used by
`Input.svelte`/`Select.svelte`/`Textarea.svelte`.

---

### `ui/src/lib/components/Tabs.svelte` (NEW component, event-driven)

**Analog 1 (interaction shape — click handler, active-state tracking, `role="tablist"`):**
`ui/src/features/requests/RequestsSearchAndTabs.svelte:21-58` (full tab list + click handler):
```typescript
type StatusTab = null | 'open' | 'in_progress' | 'completed' | 'rejected' | 'cancelled';
interface Tab {
  key: StatusTab;
  label: string;
}
const TABS: Tab[] = [
  { key: null, label: 'Все' },
  { key: 'open', label: 'Созданные' },
  // ...
];
function handleTabClick(key: StatusTab) {
  onFilterChange({ ...filter, status: key });
}
```
```svelte
<div class="tabs" role="tablist" aria-label="Статус заявок">
  {#each TABS as tab (String(tab.key))}
    <button
      class="tab"
      class:active={filter.status === tab.key}
      onclick={() => handleTabClick(tab.key)}
      role="tab"
      aria-selected={filter.status === tab.key}
      type="button"
    >
      <span class="tab-label">{tab.label}</span>
    </button>
  {/each}
</div>
```
This is exactly the shape `Tabs.svelte`'s `underline` variant should generalize into a reusable component:
`tabs: {key, label, count?}[]` prop + `active = $bindable()` + `onchange?: (_key) => void`, replacing this
feature-local hardcoded array with a generic prop. **Do not retrofit `RequestsSearchAndTabs.svelte` itself
in this phase** (D-07 — retrofit is out of scope, phases 26-28).

**Analog 2 (segmented variant, active-state pill, `role="group"`):**
`ui/src/lib/components/ThemeSwitcher.svelte:1-24` (full component):
```svelte
<div class="theme-switcher" role="group" aria-label="Переключение темы">
  {#each options as opt}
    <button
      type="button"
      class="segment"
      class:active={themeStore.preference === opt.key}
      aria-label={opt.ariaLabel}
      aria-pressed={themeStore.preference === opt.key}
      onclick={() => setTheme(opt.key)}
    >
      {opt.label}
    </button>
  {/each}
</div>
```
This is the direct implementation reference for Tabs' `segmented` variant — same each-loop-of-buttons +
`class:active` + click-to-select shape, just needs to move from a fixed 3-option theme picker to a generic
`options` prop.

**Analog 3 (variant-prop styling convention):** `ui/src/lib/components/Button.svelte:5-28` shows the
established pattern for a component with a `variant` union prop driving a CSS class name:
```typescript
interface Props {
  variant?: 'primary' | 'secondary' | 'destructive' | 'ghost' | 'link';
  size?: 'sm' | 'md';
  // ...
}
const { variant = 'primary', size = 'md', /* ... */ }: Props = $props();
```
```svelte
<button {type} class="btn btn-{variant} btn-{size}" class:loading disabled={isDisabled} {onclick}>
```
Apply the same `class="tabs tabs-{variant}"` convention for Tabs' `variant: 'underline' | 'segmented'` prop.

**Exact underline values (from RESEARCH.md "Tabs" section, `Tabs.dc.html`):**
```
tab: display:inline-flex; align-items:center; gap:6px; height:34px; padding:0 12px; background:transparent;
     border:none; border-bottom:2px solid transparent; margin-bottom:-1px; font-size:14px; font-weight:500;
     color: var(--tr-text-secondary); cursor:pointer; white-space:nowrap; border-radius:6px 6px 0 0; outline:none;
active:  color: var(--tr-accent-text) [MISSING TOKEN — must be added, see Token Mismatches below];
         border-bottom-color: var(--tr-accent); font-weight:600;
hover:   color: var(--tr-text-primary); background: var(--tr-surface-sunken);
focus:   color: var(--tr-text-primary); box-shadow: 0 0 0 3px var(--tr-focus-ring);
disabled: color: var(--tr-text-disabled); cursor:not-allowed;

count badge: display:inline-flex; align-items:center; justify-content:center; min-width:18px; height:18px;
     padding:0 5px; border-radius:9px; font-size:11px; font-weight:600; line-height:1;
     background: var(--tr-surface-sunken); color: var(--tr-text-secondary);
badge active: background: var(--tr-accent-soft); color: var(--tr-accent-text); [use canonical accent-soft — see mismatches]
badge disabled: color: var(--tr-text-disabled);
```

**Exact segmented values:**
```
container: display:inline-flex; gap:3px; padding:3px; background: var(--tr-surface-sunken); border-radius:7px;
segment: display:inline-flex; align-items:center; height:28px; padding:0 12px; border-radius:5px;
     font-size:13px; font-weight:600; cursor:pointer;
active: background: var(--tr-surface); color: var(--tr-accent-text);
     box-shadow: 0 1px 2px rgba(16,22,34,.12) [RAW RGBA — blocked by check-tokens.mjs Rule 4;
     RESEARCH.md recommends substituting var(--tr-elev-1) as the pragmatic default — see Open Questions]
inactive: background:transparent; color: var(--tr-text-secondary);
```

**Related (out-of-scope for this phase, D-07) self-rolled tab bars kept for reference only — do not modify:**
`ui/src/features/cartridges/CartridgesSearchAndTabs.svelte`, `ui/src/features/settings/SettingsSubNav.svelte`,
`ui/src/features/acts/ActsSearchAndTabs.svelte`.

---

### `ui/src/pages/ComponentShowcasePage.svelte` (NEW page) + `ui/src/features/showcase/ComponentShowcasePage.svelte` (NEW feature body)

**Analog:** `ui/src/pages/UsersPage.svelte` (thin wrapper, full file):
```svelte
<script lang="ts">
  import UsersPageFeature from '../features/users/UsersPage.svelte';
</script>

<UsersPageFeature />
```
Every route in `ui/src/routes.ts` follows this exact "page = thin re-export of a feature component" shape.
`ComponentShowcasePage.svelte` under `ui/src/pages/` should be a one-line wrapper importing the actual
showcase body from `ui/src/features/showcase/ComponentShowcasePage.svelte` (or split into
section-partials, per CONTEXT.md's Claude's-discretion note on witrina structure).

**Feature body shell pattern** (from `ui/src/features/users/UsersPage.svelte:1-13`, structural shape only —
showcase has no API calls so `apiCall`/`onMount`/`pushToast` are NOT needed):
```typescript
<script lang="ts">
  import Button from '$lib/components/Button.svelte';
  // ... import each primitive being showcased
</script>
```
Showcase sections render static demo permutations (variant×size×state loops) with no `apiCall`, no
`onMount` data fetch — it's the one feature-body file in the app that is pure presentational content.

**Route registration** (`ui/src/routes.ts:1-28`, exact current file):
```typescript
import Dashboard from './pages/Dashboard.svelte';
// ...
import SettingsPage from './pages/SettingsPage.svelte';
import NotFound from './pages/NotFound.svelte';
// ...

export const routes = {
  '/': Dashboard,
  // ...
  '/settings': SettingsPage,
  '*': NotFound,
} as const;
```
Add `import ComponentShowcasePage from './pages/ComponentShowcasePage.svelte';` and one new entry
`'/showcase': ComponentShowcasePage,` to `routes` (NOT `employeeRoutes` — showcase is admin-only, and
`employeeRoutes` is a completely separate, deliberately restrictive map for the employee role — see
`ui/src/routes.ts:30-38`).

**Admin-only sidebar entry** (`ui/src/features/layout/sidebar-config.ts:16-31`, exact current file):
```typescript
export const SIDEBAR_ITEMS: SidebarEntry[] = [
  // ...
  { kind: 'item', route: '/users', label: 'Пользователи', phase: 5, roles: ['admin'] },
  { kind: 'divider' },
  { kind: 'item', route: '/settings', label: 'Настройки', phase: 7, roles: ['admin'] },
];
```
Add `{ kind: 'item', route: '/showcase', label: 'Витрина компонентов', roles: ['admin'] }` following the
exact `roles: ['admin']` shape used by `/users` and `/settings` (not `['admin', 'manager']` — `manager` is a
distinct role per `getVisibleItems()` filtering logic at `sidebar-config.ts:38-45`, and D-02 requires
admin-only).

**Gating mechanism (no separate guard component exists):** `getVisibleItems(role)` (`sidebar-config.ts:38-45`)
is the *only* authorization mechanism for `/users`/`/settings`-class routes — there is no `RequireRole`
wrapper anywhere in the codebase. Do not invent one for the showcase; match this exact precedent.

---

### `ui/src/lib/components/Badge.svelte` (MODIFY, backward-compatible extension)

**Current full file (this file is its own analog for D-08):**
```typescript
interface Props {
  variant?: 'default' | 'accent' | 'success' | 'warning' | 'destructive';
  size?: 'sm' | 'md';
  children?: Snippet;
}
const { variant = 'default', size = 'md', children }: Props = $props();
```
```svelte
<span class="badge badge-{variant} badge-{size}">
  {@render children?.()}
</span>
```
Current CSS uses `color-mix(in srgb, var(--tr-success) 15%, transparent)`-style soft tones — these must be
replaced with the dedicated `--tr-success-soft`/`--tr-warning-soft`/`--tr-danger-soft` tokens (already in
`_tokens.scss`, confirmed present) per the reference's tone table, not `color-mix()`.

**D-08 contract (from CONTEXT.md + RESEARCH.md, verified against 21 real call-sites via
`grep -rn "<Badge" ui/src`):** add `appearance: 'soft' | 'solid' | 'dot' | 'count'` with default `'soft'`.
Internal tone-name mapping required so the 21 existing call-sites (all using `variant`, none passing
`appearance`) keep working unchanged: `default → neutral`, `destructive → danger`; `accent`/`success`/
`warning` pass through. Whether to keep the prop named `variant` (mapped internally to a `tone`) or add a
`tone` alias is Claude's discretion — **zero of the 21 call-sites may be touched.**

**5-tone × 4-appearance value table (from RESEARCH.md "Badge" section, `Badges.dc.html`):**
```
pill base: display:inline-flex; align-items:center; gap:6px; height:22px; padding:0 10px;
     border-radius:11px; font-size:12px; font-weight:600; white-space:nowrap; line-height:1;

neutral: soft bg=--tr-surface-sunken/color=--tr-text-secondary; solid bg=--tr-border-strong/color=--tr-text-primary; dot=--tr-text-tertiary
accent:  soft bg=--tr-accent-soft/color=--tr-accent-text [MISSING TOKEN]; solid bg=--tr-accent/color=--tr-on-accent; dot=--tr-accent
success: soft bg=--tr-success-soft/color=--tr-success-text; solid bg=--tr-success/color=--tr-on-accent; dot=--tr-success
warning: soft bg=--tr-warning-soft/color=--tr-warning-text; solid bg=--tr-warning/color=--tr-on-accent; dot=--tr-warning
danger:  soft bg=--tr-danger-soft/color=--tr-danger-text; solid bg=--tr-danger/color=--tr-on-accent; dot=--tr-danger

dot indicator: width:7px; height:7px; border-radius:50%; flex:none; background: <tone dot color>
count (accent-outlined): {...pill, background:'var(--tr-accent-soft)', color:'var(--tr-accent-text)',
     border:'1px solid var(--tr-accent)', borderRadius:'11px', padding:'0 9px', height:'20px', fontSize:'11px'}
count (neutral compact): {...pill, background:'var(--tr-surface-sunken)', color:'var(--tr-text-secondary)',
     borderRadius:'11px', minWidth:'18px', height:'18px', padding:'0 6px', fontSize:'11px', justifyContent:'center'}
```

---

### `ui/src/lib/components/ThemeSwitcher.svelte` / `ui/src/lib/stores/theme.svelte.ts` (MODIFY, D-09 hook point)

**Analog:** itself — `applyResolved()` in `theme.svelte.ts` is the single, exact hook point (full current file
already shown above). Current version (lines 30-35):
```typescript
function applyResolved(): void {
  const r: Resolved =
    themeStore.preference === 'system' ? (mql?.matches ? 'dark' : 'light') : themeStore.preference;
  themeStore.resolved = r;
  document.documentElement.dataset.theme = r;
}
```
**Required change (RESEARCH.md Pattern 4, exact code):**
```typescript
function applyResolved(): void {
  const r: Resolved =
    themeStore.preference === 'system' ? (mql?.matches ? 'dark' : 'light') : themeStore.preference;
  themeStore.resolved = r;
  document.documentElement.classList.add('theme-switching');
  document.documentElement.dataset.theme = r;
  requestAnimationFrame(() => {
    document.documentElement.classList.remove('theme-switching');
  });
}
```
**Global CSS addition point** — `ui/src/styles/global.scss:45-56` (current reduced-motion block, exact
excerpt, new rule goes near/after this):
```scss
// ── Reduced motion ────────────────────────────────────────────────────────────

@media (prefers-reduced-motion: reduce) {
  *,
  *::before,
  *::after {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
    scroll-behavior: auto !important;
  }
}
```
Add directly after:
```scss
:global(.theme-switching),
:global(.theme-switching) * {
  transition: none !important;
}
```
No conflict with the reduced-motion block above — that one already zeroes all transitions unconditionally
for users with the OS preference set; `.theme-switching` only matters for the normal case.

**`ThemeSwitcher.svelte` itself:** remove the stale `transition: none;` comment at line 48
(`.segment { ... transition: none; ... }`) — D-09 reverses the Phase-23 "no transitions" directive project-wide,
this component's own segment buttons should also regain the `.12s` micro-transition once the suppression
class exists.

---

### `ui/src/lib/components/Button.svelte` (MODIFY, itself is the analog)

**Current base rule** (`Button.svelte:36-60`) — note `transition: none` at line 46 and `opacity: 0.5` at
line 51, both must change per CMP-01:
```scss
.btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: var(--tr-space-2xs);
  border: none;
  border-radius: var(--tr-radius-sm);
  font-family: var(--tr-font-family);
  font-weight: var(--tr-font-weight-semibold);
  cursor: pointer;
  transition: none; // Theme switch: no transitions per UI-SPEC §Motion
  white-space: nowrap;
  text-decoration: none;

  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
    pointer-events: none;
  }
  // ...
}
```
Reference wants: `border: 1px solid transparent` (currently `border: none`), `transition: background .12s,
box-shadow .12s` (currently `none` — remove the stale comment per D-09), disabled `opacity: .45` (currently
`.5`). Current `.btn-secondary` (`Button.svelte:88-99`) has `background: transparent` — reference wants
`background: var(--tr-surface)`. No `:active` (pressed) state exists anywhere in the current file — must be
added per variant per the CMP-01 transcription table in RESEARCH.md.

---

### `ui/src/lib/components/Input.svelte`, `Select.svelte`, `Textarea.svelte` (MODIFY, itself is the analog)

**Current shared drift** (all three, confirmed by direct read): `background: var(--tr-bg)` +
`border: 1px solid var(--tr-border)` (e.g. `Input.svelte:47,49`; `Select.svelte:66,68`;
`Textarea.svelte:44,46`) must become `background: var(--tr-surface)` + `border: 1px solid var(--tr-border-strong)`
to match `ctrlBase()` from `Fields.dc.html`. Focus/disabled states already largely correct (compare
`Input.svelte:59-74` against the `ctrlBase` focus/error/disabled spec in RESEARCH.md's Fields section) —
only the base bg/border token swap and the `--tr-danger-ring` alpha (already canonical at `.2`, no action
needed there) require changes.

---

### `ui/src/lib/components/Modal.svelte` (MODIFY, itself is the analog)

**Current container rule** (`Modal.svelte:91-99`):
```scss
.modal-container {
  background: var(--tr-surface-raised);
  border-radius: var(--tr-radius-md);
  box-shadow: var(--tr-elev-2);
  display: flex;
  flex-direction: column;
  max-height: calc(100vh - 64px);
  animation: modal-in 150ms ease-out;
}
```
Must become `border-radius: var(--tr-radius-lg)` (12px, confirmed present in `_tokens.scss:190`) and
`box-shadow: var(--tr-elev-3)` (confirmed present in `_tokens.scss:80`/`155`). Overlay (`Modal.svelte:80-89`,
`background: var(--tr-overlay); backdrop-filter: blur(2px);`) and header/body/footer padding/border-color
already match the reference — no other structural changes needed. Footer buttons already compose real
`<Button>` via the `footer` snippet prop (see `footer?: Snippet` at `Modal.svelte:10` and
`{@render footer()}` at line 62) — once `Button.svelte` is corrected, Modal's footer inherits the fix with
no Modal-specific button styling to change.

---

## Shared Patterns

### Svelte 5 runes prop shape (applies to all new + modified components)
**Source:** `ui/src/lib/components/Input.svelte:1-23`, `Select.svelte:9-25`
```typescript
interface Props {
  value: string;
  disabled?: boolean;
  invalid?: boolean;
  id?: string;
  onchange?: (_value: string) => void;
  children?: Snippet;
}
const { value = $bindable(''), disabled = false, invalid = false, id, onchange, children }: Props = $props();
```
No Svelte 4 `export let`, no local Svelte stores for component-internal state — every new component
(Checkbox, Radio, Tabs) must use this exact `interface Props` + `$props()` + `$bindable()` shape.

### Variant-driven CSS class naming
**Source:** `ui/src/lib/components/Button.svelte:28` — `class="btn btn-{variant} btn-{size}"`.
**Apply to:** Tabs.svelte (`class="tabs tabs-{variant}"`), any new component with a variant union prop.

### Focus-visible ring
**Source:** `ui/src/styles/global.scss:40-43` (global fallback) + per-component overrides, e.g.
`Input.svelte:59-63`:
```scss
&:focus-visible {
  outline: none;
  border-color: var(--tr-accent);
  box-shadow: 0 0 0 3px var(--tr-focus-ring);
}
```
**Apply to:** Checkbox/Radio `.native-input:focus-visible ~ .box`, Tabs tab/segment focus states.

### Disabled-state convention
**Source:** `ui/src/lib/components/Input.svelte:70-74`:
```scss
&:disabled {
  background: var(--tr-surface-sunken);
  color: var(--tr-text-tertiary);
  cursor: not-allowed;
}
```
**Apply to:** all modified/new form-input components. Note the `.dc` reference uses `--tr-text-disabled`
(not `--tr-text-tertiary`) for disabled text — this is one of the visual corrections CMP-02 brings in,
not a pattern to copy verbatim from the current file.

### Admin-only route gating (sidebar-filter, no separate guard component)
**Source:** `ui/src/features/layout/sidebar-config.ts:38-45` (`getVisibleItems`) — the only authorization
mechanism for `/users`, `/settings`, and now `/showcase`. Do not add a `RequireRole` wrapper or per-page
role check; actual data authorization (irrelevant here — showcase has none) stays server-side.

### Token closed-world gate (`pnpm lint` / `check-tokens.mjs`)
**Source:** `ui/scripts/check-tokens.mjs` (not excerpted here — see RESEARCH.md "Common Pitfalls" for the
4 rules). Applies to every `<style lang="scss">` block touched or created in this phase: no raw hex, no raw
`rgba()`/`rgb()`/`hsl()`/`hsla()`, only `var(--tr-*)` tokens already declared in `ui/src/styles/_tokens.scss`.
Two concrete consequences for this phase:
1. `--tr-accent-text` must be added to `_tokens.scss` (`#2350bd` light / `#8fb0ff` dark, both `[data-theme]`
   blocks) before it can be used in Badge.svelte/Tabs.svelte.
2. Tabs.svelte's segmented-active box-shadow (`rgba(16,22,34,.12)` in the reference) has no exact token —
   substitute `var(--tr-elev-1)` (nearest existing shadow token, `_tokens.scss:78`/`153`) rather than pasting
   the raw rgba value.

## No Analog Found

None — every file in scope has at least a role-match or exact analog in the current codebase (Checkbox/
Radio/Tabs/Showcase have no *direct* prior instance of the same component, but their runes shape, DOM
interaction pattern, and route-registration mechanics are all fully covered by existing analogs listed
above).

## Metadata

**Analog search scope:** `ui/src/lib/components/`, `ui/src/features/{requests,users,layout,showcase}/`,
`ui/src/pages/`, `ui/src/routes.ts`, `ui/src/lib/stores/theme.svelte.ts`, `ui/src/styles/{_tokens,global}.scss`
**Files scanned:** 16 (Button, Badge, Input, Select, Textarea, Modal, ThemeSwitcher, theme.svelte.ts,
RequestsSearchAndTabs, UsersPage×2, sidebar-config.ts, routes.ts, global.scss, _tokens.scss, plus a grep
sweep across 4 other self-rolled tab-bar files confirmed out of scope)
**Pattern extraction date:** 2026-07-18

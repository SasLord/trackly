# Phase 24: Базовые компоненты - Research

**Researched:** 2026-07-18
**Domain:** Svelte 5 component styling (design-system transcription), no new runtime dependencies
**Confidence:** HIGH

## Summary

Phase 24 is a pure frontend transcription task: 5 primitives (Button, Input/Select/Textarea/Checkbox/Radio,
Badge, Tabs, Modal) get their exact visual values copied from 5 Design-Canvas reference files
(`.planning/reference/design-system-v2/*.dc.html`) onto the existing `--tr-*` token layer from Phase 23,
plus a new permanent admin-only showcase route. Three components (Checkbox, Radio, Tabs) do not exist yet;
five (Button, Input, Select, Textarea, Badge, Modal) exist and are already on `--tr-*` tokens but have
value drift from the references (documented below, value-by-value). No new npm packages are required —
routing (`svelte-spa-router`), the role-gated sidebar pattern, and the Svelte 5 runes conventions are all
already established in the codebase and only need to be extended, not introduced.

The single biggest risk is **token drift inside the reference files themselves**: two `--tr-*` values used
in the `.dc.html` demo `:root` blocks (`--tr-accent-text` is entirely undefined in `_tokens.scss`;
`--tr-danger-ring` and `--tr-accent-soft` have different alpha values in different `.dc` files than the
canonical `_tokens.scss`) will trip Phase 23's `check-tokens.mjs` closed-world gate (Rule 3) if copied
verbatim, or silently diverge from the canonical palette if not caught. These are flagged explicitly below
with exact numbers — the planner must resolve them (most likely: add one new token, and use the canonical
`_tokens.scss` value instead of the `.dc` file's local demo value where they disagree).

**Primary recommendation:** Transcribe `styleFor`/`ctrlBase`/`renderVals`/`tabStyle`/`badgeStyle` values from
each `.dc.html` file's embedded `<script type="text/x-dc">` block directly into each component's scoped
`<style lang="scss">`, using CSS classes for variant×state (not JS style objects — the `.dc` format uses JS
style objects only because Design Canvas has no CSS pipeline; Svelte components use real SCSS). Resolve the
`--tr-accent-text` gap and the `--tr-danger-ring`/`--tr-accent-soft` value mismatches before writing any
component CSS (see "Token Mismatches" below) — otherwise `pnpm lint`'s `check-tokens.mjs` Rule 3 will fail
on Badge/Tabs, or the shipped colors will silently disagree with the approved palette.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Button/Input/Select/Textarea/Checkbox/Radio/Badge/Tabs/Modal rendering & styling | Browser/Client | — | Pure presentational Svelte components; no data fetching, no server round-trip. Same components render identically in Tauri webview and LAN browser (SPA, no SSR tier in this app). |
| Showcase route access control (admin-only) | Browser/Client (UI gate) | API/Backend (session role, unchanged) | Sidebar-link visibility + route table membership is a client-side UI gate only (matches existing `/users`, `/settings` pattern — see "Admin-gating reality check" below). Actual authorization for any *data* the showcase touches (none — it's static demo markup) stays server-side, but the showcase itself renders no privileged data, so client-side gating is consistent with existing precedent. |
| Theme switching + transition suppression (D-09) | Browser/Client | — | `theme.svelte.ts` mutates `document.documentElement.dataset.theme`; transition-suppression class toggling is a same-tier DOM operation, no backend involvement. |
| Design token values (`--tr-*`) | Browser/Client (CSS custom properties) | — | Single source of truth `ui/src/styles/_tokens.scss`, loaded once via `global.scss`, consumed by all component `<style>` blocks. |

## Standard Stack

No new packages. This phase installs nothing — it styles existing components and adds Svelte files using
the project's existing toolchain.

### Core (existing, unchanged)
| Library | Version (installed) | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `svelte` | `^5.55.0` [VERIFIED: ui/package.json] | UI framework, runes (`$props`, `$bindable`, `$derived`, `$state`) | Project-locked (CLAUDE.md) |
| `svelte-spa-router` | `^5.1.0` [VERIFIED: ui/package.json] | Hash-based SPA routing, used by both `Layout`/admin routes and `EmployeeLayout`/employee routes | Already wired in `App.svelte` — showcase route is one more entry, not a new integration |
| `sass` | `^1.80.0` [VERIFIED: ui/package.json] | SCSS compilation via `vitePreprocess` | Used by every existing component `<style lang="scss">` block |

### Package Legitimacy Audit

Not applicable — this phase introduces zero new npm packages. Skip the legitimacy gate.

## Architecture Patterns

### Recommended Project Structure
```
ui/src/lib/components/
├── Button.svelte        # MODIFY — transitions restored, active state added, secondary bg fixed
├── Input.svelte         # MODIFY — bg/border token swap
├── Select.svelte        # MODIFY — bg/border token swap
├── Textarea.svelte      # MODIFY — bg/border token swap
├── Checkbox.svelte      # NEW — hidden native <input type=checkbox> + styled box span
├── Radio.svelte         # NEW — hidden native <input type=radio> + styled box span
├── Badge.svelte         # MODIFY — add `appearance` prop, backward-compat default
├── Tabs.svelte          # NEW — variant: 'underline' | 'segmented'
├── Modal.svelte         # MODIFY — elev-3, radius-lg
└── Spinner.svelte       # UNCHANGED — already reused correctly by Button loading state

ui/src/pages/ (or ui/src/features/showcase/)
└── ComponentShowcasePage.svelte   # NEW — permanent admin-only gallery, mirrors 5 .dc galleries

ui/src/routes.ts               # MODIFY — add '/showcase' (or similar) to `routes` map only (NOT employeeRoutes)
ui/src/features/layout/sidebar-config.ts   # MODIFY — add SidebarItem with roles: ['admin']
ui/src/lib/stores/theme.svelte.ts          # MODIFY — D-09 transition-suppression hook in applyResolved()
ui/src/styles/_tokens.scss                 # MODIFY (likely) — add --tr-accent-text (see Token Mismatches)
```

### System Architecture Diagram

```
User clicks sidebar "Витрина" (admin only, filtered by getVisibleItems(role))
        │
        ▼
svelte-spa-router matches '/showcase' in `routes` map (App.svelte's <Router {routes} />)
        │
        ▼
ComponentShowcasePage.svelte mounts
        │
        ├─▶ renders <Button> in all variant×size×state permutations (static demo data, no API calls)
        ├─▶ renders <Input>/<Select>/<Textarea>/<Checkbox>/<Radio> in all states
        ├─▶ renders <Badge> in all tone×appearance permutations
        ├─▶ renders <Tabs variant="underline"> and <Tabs variant="segmented">
        └─▶ renders <Modal> (opened via a "Показать модал" trigger button in the showcase)
        │
        ▼
All 5 primitives read CSS custom properties from `--tr-*` (global.scss → _tokens.scss),
resolved per `[data-theme]` on <html> (set by theme.svelte.ts). No network calls, no
Tauri invoke, no axum endpoint — the showcase is 100% client-side static demo content.
```

### Pattern 1: Svelte 5 runes conventions (established, verified from Button.svelte / Input.svelte / Select.svelte)
**What:** Props via `interface Props` + `$props()`; two-way bound values via `= $bindable(default)`;
computed values via `$derived(...)`; children content via `Snippet` type + `{@render children?.()}`.
**When to use:** Every new component in this phase (Checkbox, Radio, Tabs) must follow this exact shape —
no Svelte 4 `export let`, no stores for local component state.
**Example (from `ui/src/lib/components/Select.svelte`, existing code):**
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

### Pattern 2: Checkbox/Radio — hidden native input + styled sibling span
**What:** The `.dc.html` reference (`Fields.dc.html`) only shows a decorative 18px `<span>` box — Design
Canvas has no real DOM semantics, it's a style spec, not a component to copy. To get real checkbox/radio
keyboard support, focus management, and screen-reader semantics for free, wrap a visually-hidden native
`<input type="checkbox">` / `<input type="radio">` and drive the visual box's border/background/checkmark
purely from CSS sibling selectors (`:checked`, `:focus-visible`, `:disabled`) — do not hand-roll `role="checkbox"`
+ manual keydown handlers.
**When to use:** Checkbox.svelte, Radio.svelte (both new this phase).
**Radio group binding:** Use native `bind:group` support (Svelte compiles `bind:group` on
`<input type="radio">` to the correct `name`/`checked` wiring automatically) — expose a `group = $bindable()`
prop on `Radio.svelte` that forwards to the native input's `bind:group`, OR keep Radio value-only
(`checked`/`value` props, parent owns `$state` for the selected value) if the planner prefers explicit control
in the consuming component. Either is consistent with the runes pattern already in use; **do not** invent a
custom event-based group-sync mechanism — native radio groups already solve this problem.
**Example shape (not copied from any file — synthesized from Pattern 1 + native radio/checkbox semantics):**
```svelte
<label class="check-row" class:disabled>
  <span class="box-wrap">
    <input type="checkbox" bind:checked={checked} {disabled} class="native-input" />
    <span class="box" aria-hidden="true"></span>
  </span>
  {@render children?.()}
</label>
```

### Pattern 3: Role-gated admin-only route (mirror of `/users` and `/settings`)
**What:** Sidebar visibility is filtered by `getVisibleItems(role)` in
`ui/src/features/layout/sidebar-config.ts` using a `roles?: UserRole[]` field on `SidebarEntry`. This is the
**only** gating mechanism currently in place for admin-only sections (`/users`, `/settings` both use
`roles: ['admin']`). There is **no additional route-level guard component** — no `RequireRole` wrapper, no
per-page role check inside `UsersPage.svelte`/`SettingsPage.svelte` (verified: grepped both files, no role
logic present). A `manager` who manually edits the URL hash to `#/users` would render the page client-side;
protection against unauthorized *data* comes entirely from backend API authorization on each Tauri
command/axum handler, not from the frontend route.
**Showcase implication:** The showcase page displays zero privileged/business data (pure demo/mock content
for design QA), so the existing sidebar-only gate is sufficient and consistent with precedent — do not
over-engineer a stricter guard than `/users`/`/settings` already have. If the planner wants defense-in-depth
beyond precedent, that is a deliberate scope increase beyond D-02's minimum, not a required pattern match.
**Concrete integration points:**
```typescript
// ui/src/routes.ts — add ONE entry to `routes` (NOT `employeeRoutes` — showcase is admin/manager area only)
import ComponentShowcasePage from './pages/ComponentShowcasePage.svelte'; // or features/showcase/...
export const routes = {
  // ...existing entries...
  '/showcase': ComponentShowcasePage,
  '*': NotFound,
} as const;
```
```typescript
// ui/src/features/layout/sidebar-config.ts — add ONE entry with roles: ['admin']
{ kind: 'item', route: '/showcase', label: 'Витрина компонентов', roles: ['admin'] },
```
Note: `manager` role exists in `UserRole` and is NOT `admin` — confirmed via `ROLE_LABELS` in
`Sidebar.svelte` (`admin`/`manager`/`employee`). D-02 requires admin-only, so `roles: ['admin']` (not
`['admin','manager']`) is correct — same restriction level as `/users`.

### Pattern 4: Theme-switch transition suppression (D-09)
**What:** `applyResolved()` in `ui/src/lib/stores/theme.svelte.ts` is the single function that mutates
`document.documentElement.dataset.theme`. This is the exact hook point. Add a global-scope class
(`.theme-switching` or similar, styled in `global.scss` near the existing `prefers-reduced-motion` block at
line 47) that sets `transition: none !important` on `*`, apply it to `documentElement` immediately before
the dataset mutation, then remove it on the next animation frame (`requestAnimationFrame`, not `setTimeout`
— rAF guarantees the removal happens after the browser has painted the new theme's colors without a
transition, whereas a `setTimeout(0)` is not guaranteed to run after paint).
```typescript
// ui/src/lib/stores/theme.svelte.ts — MODIFY applyResolved()
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
```scss
// ui/src/styles/global.scss — new rule, placed near the existing prefers-reduced-motion block (~line 47)
:global(.theme-switching),
:global(.theme-switching) * {
  transition: none !important;
}
```
**Interaction with existing `prefers-reduced-motion` block:** No conflict — that block already sets
`transition-duration: 0.01ms !important` globally when the OS preference is set, which already suppresses
all transitions unconditionally for those users; the new `.theme-switching` class only needs to matter for
users who do NOT have reduced-motion set (the normal case), where per-component `.12s` transitions (D-09)
would otherwise cause a visible color-bleed sweep across the whole UI on theme toggle.

### Anti-Patterns to Avoid
- **Copying `.dc.html` markup into Svelte templates:** The `<x-dc>`/`<sc-for>`/`{{ }}` markup is Design
  Canvas templating syntax requiring `support.js` at runtime — it is not valid Svelte and must never be
  pasted in. Only the JS style-value objects inside `<script type="text/x-dc" data-dc-script>` are the
  source of truth; translate those numbers into SCSS rules by hand.
- **Re-deriving colors/spacing instead of copying values verbatim:** Per CONTEXT.md's explicit instruction
  and Phase 23 precedent (`_tokens.scss` header comment: "Значения копируются дословно, не пересчитываются") —
  do not "improve" or round any px/alpha value found in a `.dc` file's `styleFor`/`ctrlBase` logic.
- **Raw color literals in scoped `<style>` blocks:** `check-tokens.mjs` Rule 2 (hex) and Rule 4
  (`rgba()`/`rgb()`/`hsl()`/`hsla()`) both hard-fail `pnpm lint` on any literal color inside a `.svelte`
  `<style>` block — including inside a `var(--tr-x, rgba(...))` fallback. See "Common Pitfalls" for the one
  concrete place this bites in the Tabs reference (segmented-active box-shadow).
- **Inventing a `RequireRole` guard component that doesn't exist anywhere else in the codebase:** Match the
  established sidebar-only gating precedent (`/users`, `/settings`) rather than introducing a new
  authorization pattern for just this one page.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Checkbox/Radio keyboard nav + a11y semantics | Custom `role="checkbox"`/`role="radio"` + manual keydown/space/arrow handling | Native `<input type="checkbox">` / `<input type="radio">`, visually hidden, driven by CSS | Native inputs give free keyboard support (Space toggles checkbox, arrow keys move within a native radio group, Tab order, screen-reader announcement) that a custom ARIA widget would have to reimplement and QA (QA-02, deferred to Phase 30, would otherwise inherit a fresh a11y bug) |
| Tabs keyboard nav (arrow-key movement between tabs) | Full WAI-ARIA Tabs pattern with roving tabindex | Basic `role="tablist"`/`role="tab"`/`aria-selected` + click handling is sufficient for Phase 24 scope; full roving-tabindex keyboard nav is an accessibility refinement explicitly deferred to QA-02 (Phase 30) | Avoids scope creep into Phase 30's territory while still shipping semantically correct markup |
| Theme-switch flash suppression | A CSS-only solution (e.g. `transition: none` permanently, which is what Phase 23 shipped and D-09 now reverses) | The one-frame `rAF`-toggled class described in Pattern 4 | Permanent `transition: none` was Phase 23's interim fix for the exact flash problem D-09 now wants to solve properly (micro-transitions restored, flash suppressed only during the switch) |

**Key insight:** Every "new" interactive primitive in this phase (Checkbox, Radio, Tabs) has a native HTML
building block or an existing in-app precedent (ThemeSwitcher.svelte is effectively a working 3-way
segmented control already) — none require building interaction logic from scratch.

## Token Mismatches (must resolve before styling Badge/Tabs/Buttons/Fields)

Verified by diffing each `.dc.html` file's embedded `:root`/`[data-theme="dark"]` block against
`ui/src/styles/_tokens.scss` [VERIFIED: direct file read, both sides].

| Token | `_tokens.scss` (canonical) | `.dc` file value(s) found | Verdict |
|-------|----------------------------|----------------------------|---------|
| `--tr-accent-text` | **Not defined anywhere in `_tokens.scss`** | `Badges.dc.html`: light `#2350bd` / dark `#8fb0ff`. `Tabs.dc.html`: same light `#2350bd` / dark `#8fb0ff` (consistent between the two files that use it) | **Missing token.** Needed for: Badge accent-soft/dot text color, Tabs active-tab text color (`--tr-accent-text` on the underline variant), Tabs segmented-active text color. Must be added to `_tokens.scss` (both `[data-theme='light']` and `[data-theme='dark']` blocks) before Badge/Tabs styling — otherwise `check-tokens.mjs` Rule 3 (closed-world gate) fails the build. Values agree across both files that reference it, so this is a low-risk, well-specified addition, not an open design question. |
| `--tr-danger-ring` | `rgba(207, 59, 59, 0.2)` light / `rgba(242, 101, 101, 0.2)` dark [VERIFIED — converged in Phase 23 plan 08 per STATE.md: "Button.svelte danger-ring alpha 0.3->0.2 ... WR-01-санкционированный visual touch, handoff в фазу 24 (CMP-01)"] | `Buttons.dc.html`: light `rgba(207,59,59,.32)` / dark `rgba(242,101,101,.42)`. `Fields.dc.html`: light `rgba(207,59,59,.28)` / dark `rgba(242,101,101,.40)` — **the two `.dc` files disagree with each other AND with the canonical token** | **Use the canonical `_tokens.scss` value (0.2 both themes).** This was an intentional Phase 23 convergence decision explicitly flagged for Phase 24 pickup — the `.dc` files' local demo `:root` blocks are stale relative to that decision. Do not copy either `.dc` file's ring alpha. |
| `--tr-accent-soft` | `rgba(43, 95, 217, 0.1)` light / `rgba(91, 139, 255, 0.16)` dark | `Badges.dc.html`: matches canonical exactly (`.10`/`.16`). `Tabs.dc.html`: light `rgba(43,95,217,.14)` / dark `rgba(91,139,255,.20)` — **differs from canonical and from Badges.dc** | **Use the canonical `_tokens.scss` value.** Tabs.dc's local `.14`/`.20` appears to be a standalone demo-file drift (Badges.dc, which defines the same token name for the same visual role — soft accent background — agrees with canonical). Do not introduce a second accent-soft value just for Tabs. |
| `--tr-focus-ring` | `rgba(43, 95, 217, 0.35)` light / `rgba(91, 139, 255, 0.45)` dark | Matches exactly in `Buttons.dc.html`, `Fields.dc.html`, `Tabs.dc.html`, `Modal.dc.html` | Consistent — no action needed. |
| `--tr-danger-text` | `#b02f2f` light / `#ff8080` dark | Matches exactly in `Fields.dc.html`, `Badges.dc.html` | Consistent — no action needed. |
| `--tr-success`/`-soft`/`-text`, `--tr-warning`/`-soft`/`-text` | See `_tokens.scss` | Matches exactly in `Badges.dc.html` | Consistent — no action needed. |
| `--tr-border`, `--tr-border-strong`, `--tr-surface`, `--tr-surface-sunken`, `--tr-bg`, `--tr-overlay`, `--tr-elev-3`, `--tr-on-accent`, `--tr-text-*`, `--tr-accent`/`-hover`/`-active` | See `_tokens.scss` | Matches exactly across all 5 `.dc` files checked | Consistent — no action needed. |

**Non-token color literal found (blocks `check-tokens.mjs` Rule 4 if copied as-is):** `Tabs.dc.html`'s
`segStyle(act)` function uses a raw `boxShadow: '0 1px 2px rgba(16,22,34,.12)'` for the active segmented-tab
state — this is not backed by any `--tr-*` token. The closest existing token is `--tr-elev-1`
(`0 1px 2px rgba(16, 22, 34, 0.07), 0 1px 1px rgba(16, 22, 34, 0.04)` light) but the alpha (0.07 vs 0.12) and
structure (2-layer vs 1-layer shadow) differ. **This is a genuine open decision** (see Open Questions) — the
planner/discuss-phase should decide whether to (a) use `var(--tr-elev-1)` as the nearest existing token
despite the value drift, or (b) treat this as a Claude's-discretion micro-decision per CONTEXT.md's
"точные значения... не пересчитывать" instruction (which would argue for adding this exact shadow as a new
token — but that reopens Phase 23's closed token set, which CONTEXT.md does not authorize this phase to do
beyond the one `--tr-accent-text` addition already justified above).

## Component-by-Component Transcription Reference

All values below are extracted directly from each `.dc.html` file's `<script type="text/x-dc"
data-dc-script>` block [VERIFIED: direct file read of `.planning/reference/design-system-v2/*.dc.html`].
These are the exact numbers to transcribe — no recalculation.

### Button (`Buttons.dc.html`, CMP-01)
**Base (all variants, both sizes):** `display:inline-flex; align-items:center; justify-content:center;
gap:6px; border-radius:6px; font-weight:600; white-space:nowrap; cursor:pointer;
border:1px solid transparent; transition:background .12s, box-shadow .12s;`
- **sm:** `height:28px; padding:0 12px; font-size:13px`
- **md:** `height:36px; padding:0 16px; font-size:14px`

**Variants × states** (`ring(c) = 0 0 0 3px c`):
| Variant | default | hover | active | focus | disabled | loading |
|---|---|---|---|---|---|---|
| **primary** | bg `--tr-accent`, color `--tr-on-accent`, border `--tr-accent` | bg+border `--tr-accent-hover` | bg+border `--tr-accent-active` | + box-shadow `ring(--tr-focus-ring)` | + `opacity:.45; cursor:not-allowed` | + `opacity:.85; cursor:default` |
| **secondary** | bg `--tr-surface`, color `--tr-text-primary`, border `--tr-border-strong` | bg `--tr-surface-sunken`, border `--tr-border-strong` | bg `--tr-surface-sunken`, border `--tr-text-tertiary` | ring(`--tr-focus-ring`) **+ borderColor → `--tr-accent`** (focusBorder) | opacity .45 | opacity .85 |
| **destructive** | bg `--tr-danger`, color `--tr-on-accent`, border `--tr-danger` | bg+border `--tr-danger-hover` | bg+border `--tr-danger-active` | ring(`--tr-danger-ring`) [use canonical 0.2, not .dc's .32/.42 — see Token Mismatches] | opacity .45 | opacity .85 |
| **ghost** | bg transparent, color `--tr-text-primary`, border transparent | bg `--tr-surface-sunken` | bg `--tr-surface-sunken`, border `--tr-border` | ring(`--tr-focus-ring`) + focusBorder `--tr-accent` | opacity .45, **color → `--tr-text-disabled`** (ghost-specific disabled override) | opacity .85 |
| **link** | bg transparent, border transparent, color `--tr-accent`, `text-decoration:underline`, `text-underline-offset:2px`, `height:auto`, `padding:2px 2px` | color `--tr-accent-hover` | color `--tr-accent-active` | box-shadow ring(`--tr-focus-ring`), `border-radius:4px`, `text-decoration:none` | color `--tr-text-disabled`, `cursor:not-allowed`, `text-decoration:none` | color `--tr-text-tertiary`, `text-decoration:none` |

Note: `--tr-accent-hover`, `--tr-accent-active`, `--tr-danger-hover`, `--tr-danger-active` are ALL already
present and correct in `_tokens.scss` [VERIFIED].

**Spinner (loading state):** `width:12px; height:12px; border:2px solid currentColor;
border-top-color:transparent; border-radius:50%; opacity:.7; animation: spin .7s linear infinite;` — Button
already uses `Spinner.svelte` (`size="sm"` → 12px per its own `px` map) for this, which is compositionally
equivalent; no need to hand-roll a second spinner.

### Fields (`Fields.dc.html`, CMP-02) — Input/Select/Textarea + Checkbox/Radio (D-04)
**`ctrlBase()` (Input/Select/Textarea shared base):** `display:block; width:100%; box-sizing:border-box;
height:36px; padding:0 12px; background: var(--tr-surface); color: var(--tr-text-primary);
border:1px solid var(--tr-border-strong); border-radius:6px; font-size:14px; line-height:1.5; outline:none;`
- **focus:** `border-color: var(--tr-accent); box-shadow: 0 0 0 3px var(--tr-focus-ring);`
- **error:** `border-color: var(--tr-danger); box-shadow: 0 0 0 3px var(--tr-danger-ring);` [canonical 0.2 alpha]
- **disabled:** `background: var(--tr-surface-sunken); color: var(--tr-text-disabled); border-color: var(--tr-border); cursor: not-allowed;`
- **Select-specific:** `padding-right:32px; appearance:none; cursor:pointer` on top of ctrlBase (matches
  existing `Select.svelte`'s caret pattern already — just needs the bg/border swap below)
- **Textarea-specific:** `height:auto; min-height:92px; padding:8px 12px; resize:vertical`

**Current Input/Select/Textarea drift (confirmed by direct read):** all three currently use
`background: var(--tr-bg)` and `border: 1px solid var(--tr-border)` — must become
`background: var(--tr-surface)` and `border: 1px solid var(--tr-border-strong)` to match `ctrlBase()`.

**Checkbox/Radio box (`box(opts)`):** `width:18px; height:18px; flex:none; display:inline-flex;
align-items:center; justify-content:center; border:1.5px solid var(--tr-border-strong);
background: var(--tr-surface); box-sizing:border-box;` + `border-radius: 5px` (checkbox) or `50%` (radio)
- **checked:** `background: var(--tr-accent); border-color: var(--tr-accent);`
- **focus:** `box-shadow: 0 0 0 3px var(--tr-focus-ring); border-color: var(--tr-accent);`
- **disabled:** `background: var(--tr-surface-sunken); border-color: var(--tr-border);`
- **Checkmark (checkbox, checked state):** `width:10px; height:6px; border-left:2px solid var(--tr-on-accent);
  border-bottom:2px solid var(--tr-on-accent); transform: rotate(-45deg) translate(0,-1px);`
- **Radio dot (radio, checked state):** `width:8px; height:8px; border-radius:50%; background: var(--tr-on-accent);`
- **Row (label wrapper):** `display:inline-flex; align-items:center; gap:10px; font-size:14px;
  color: var(--tr-text-primary); cursor:pointer;` — disabled row: `color: var(--tr-text-disabled);
  cursor:not-allowed;`

### Badge (`Badges.dc.html`, CMP-03, D-06/D-08)
**5 tones** (NOT 4 — D-06 correction, REQUIREMENTS.md/ROADMAP.md text needs updating from "4 тона"):
`neutral`, `accent`, `success`, `warning`, `danger`.

**Pill base:** `display:inline-flex; align-items:center; gap:6px; height:22px; padding:0 10px;
border-radius:11px; font-size:12px; font-weight:600; white-space:nowrap; line-height:1;`

| Tone | soft (default) bg/color | solid bg/color | dot color |
|---|---|---|---|
| neutral | `--tr-surface-sunken` / `--tr-text-secondary` | `--tr-border-strong` / `--tr-text-primary` | `--tr-text-tertiary` |
| accent | `--tr-accent-soft` / `--tr-accent-text` **(missing token — see Token Mismatches)** | `--tr-accent` / `--tr-on-accent` | `--tr-accent` |
| success | `--tr-success-soft` / `--tr-success-text` | `--tr-success` / `--tr-on-accent` | `--tr-success` |
| warning | `--tr-warning-soft` / `--tr-warning-text` | `--tr-warning` / `--tr-on-accent` | `--tr-warning` |
| danger | `--tr-danger-soft` / `--tr-danger-text` | `--tr-danger` / `--tr-on-accent` | `--tr-danger` |

**dot indicator:** `width:7px; height:7px; border-radius:50%; flex:none; background: <tone dot color>` (rendered as a leading child span alongside the pill text — this is the `appearance="dot"` case)

**counter-pill** (2 shapes shown in reference — both are the `appearance="count"` case):
- Accent-outlined count: `{...pill, background:'var(--tr-accent-soft)', color:'var(--tr-accent-text)',
  border:'1px solid var(--tr-accent)', borderRadius:'11px', padding:'0 9px', height:'20px', fontSize:'11px'}`
- Neutral compact count: `{...pill, background:'var(--tr-surface-sunken)', color:'var(--tr-text-secondary)',
  borderRadius:'11px', minWidth:'18px', height:'18px', padding:'0 6px', fontSize:'11px',
  justifyContent:'center'}`

**D-08 backward-compat mapping (verified against existing `Badge.svelte` + call-sites):** existing prop is
`variant: 'default'|'accent'|'success'|'warning'|'destructive'`, `size: 'sm'|'md'`. Internal tone mapping
required: `default → neutral`, `destructive → danger`; `accent`/`success`/`warning` pass through unchanged.
New `appearance: 'soft'|'solid'|'dot'|'count'` prop, default `'soft'` (= current visual, so no call-site
needs to pass it).

**Call-site count correction:** CONTEXT.md states "15 текущих вызовов" — actual grep count
[VERIFIED: `grep -rn "<Badge" ui/src`] found **21 real `<Badge ...>` render call-sites** (not counting 5
`$derived<BadgeVariant>` type-annotation lines that coincidentally substring-match `<Badge`). All 21 use the
`variant` prop name; static literal values found: `"default"` (6×), `"warning"` (2×), `"success"` (1×),
plus dynamic `statusVariant`/ternary expressions (12×) that can resolve to any of the 5 existing variant
values including `"destructive"` (confirmed via `type BadgeVariant` declarations in `DeviceGroupRow.svelte`,
`DeviceListRow.svelte`, `PrinterListRow.svelte`, `PrinterDetail.svelte`, `RequestListRow.svelte` — all
include `'destructive'` in their local type union and map status codes to it). None of the 21 call-sites
pass a `size` other than the 3 explicit `size="sm"` usages (rest default to `"md"`). **None pass
`appearance`** (prop doesn't exist yet) — the invariant "0 call-sites touched" is unaffected by the exact
count being 21 rather than 15; this note exists so the planner doesn't under-scope the backward-compat
verification sweep to only 15 files.

Full call-site list (file:line): `CartridgeListRow.svelte:94`, `ModelListRow.svelte:51,55`,
`CartridgeDetail.svelte:117`, `CartridgesSearchAndTabs.svelte:71`, `ActListRow.svelte:67`,
`ActsSearchAndTabs.svelte:65`, `ActNumberField.svelte:85,87`, `RequestListRow.svelte:94,98,103`,
`RequestDetail.svelte:412,413`, `PrinterListRow.svelte:88`, `DiscoveryResultsTable.svelte:83,85`,
`PrinterDetail.svelte:214`, `DeviceGroupRow.svelte:180`, `ReportSubNav.svelte:109`, `DeviceListRow.svelte:63`.

### Tabs (`Tabs.dc.html`, CMP-04, D-05 both variants)
**`underline` variant — `tabStyle(state)`:** `display:inline-flex; align-items:center; gap:6px; height:34px;
padding:0 12px; background:transparent; border:none; border-bottom:2px solid transparent;
margin-bottom:-1px; font-size:14px; font-weight:500; color: var(--tr-text-secondary); cursor:pointer;
white-space:nowrap; border-radius:6px 6px 0 0; outline:none;`
- **active:** `color: var(--tr-accent-text)` **(missing token)**`; border-bottom-color: var(--tr-accent);
  font-weight:600;`
- **hover:** `color: var(--tr-text-primary); background: var(--tr-surface-sunken);`
- **focus:** `color: var(--tr-text-primary); box-shadow: 0 0 0 3px var(--tr-focus-ring);`
- **disabled:** `color: var(--tr-text-disabled); cursor:not-allowed;`

**Tab count badge — `badgeStyle(state)`:** `display:inline-flex; align-items:center; justify-content:center;
min-width:18px; height:18px; padding:0 5px; border-radius:9px; font-size:11px; font-weight:600; line-height:1;
background: var(--tr-surface-sunken); color: var(--tr-text-secondary);`
- **active:** `background: var(--tr-accent-soft); color: var(--tr-accent-text);` [use canonical accent-soft, see Token Mismatches]
- **disabled:** `color: var(--tr-text-disabled);`

**`segmented` variant (CMP-04 stretch per D-05):** container: `display:inline-flex; gap:3px; padding:3px;
background: var(--tr-surface-sunken); border-radius:7px;`. Segment (`segStyle(act)`):
`display:inline-flex; align-items:center; height:28px; padding:0 12px; border-radius:5px; font-size:13px;
font-weight:600; cursor:pointer;`
- **active:** `background: var(--tr-surface); color: var(--tr-accent-text); box-shadow: 0 1px 2px rgba(16,22,34,.12)` — **raw rgba, see Token Mismatches / Open Questions for resolution**
- **inactive:** `background:transparent; color: var(--tr-text-secondary);`

### Modal (`Modal.dc.html`, CMP-05)
Overlay: `background: var(--tr-overlay); backdrop-filter: blur(2px)` (already correct in current
`Modal.svelte`). Container: `background: var(--tr-surface); border: 1px solid var(--tr-border);
border-radius: 12px (var(--tr-radius-lg)); box-shadow: var(--tr-elev-3);` — **current `Modal.svelte` uses
`border-radius: var(--tr-radius-md)` (8px) and `box-shadow: var(--tr-elev-2)`, both must change** [VERIFIED
by direct read of `ui/src/lib/components/Modal.svelte` lines 92-94]. Header/body/footer padding, border
colors already match (`--tr-border` dividers, `--tr-space-md`/`--tr-space-xl` paddings) — no other changes
needed to Modal.svelte's structure. Footer buttons in the reference are plain style objects (`btnSecondary`/
`btnPrimary`) — these are Design Canvas's own simplified mockup buttons, not a separate button spec to
implement; the actual Modal footer already composes real `<Button>` components via the `footer` snippet
prop (existing pattern, e.g. in `ActDetail.svelte`/`CartridgesPage.svelte` delete-confirm modals) — once
Button.svelte is corrected per CMP-01, Modal's footer buttons inherit the fix automatically.

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| CMP-01 | Кнопки — 5 вариантов × 2 размера × 6 состояний | Full `styleFor(variant,state,size)` transcription table above (Button section); current `Button.svelte` gap list: `transition:none`→restore `.12s`, `opacity:.5`→`.45`, secondary bg `transparent`→`var(--tr-surface)`, no active/pressed state→add `:active` rules per table |
| CMP-02 | Input/Select/Textarea/Checkbox states | `ctrlBase()` + checkbox/radio `box()` transcription (Fields section); current Input/Select/Textarea bg/border token drift documented; Checkbox/Radio built new per Pattern 2 (hidden native input) |
| CMP-03 | Badges — 5 тонов (D-06 corrects "4" in REQUIREMENTS.md/ROADMAP.md) × soft/solid/dot/count | Full tone×appearance matrix (Badge section); D-08 backward-compat mapping verified against all 21 real call-sites |
| CMP-04 | Tabs switch-bar (underline) with counters + active underline; D-05 adds segmented variant | `tabStyle`/`badgeStyle`/`segStyle` transcription (Tabs section); segmented-active box-shadow flagged as open decision (raw rgba not tokenized) |
| CMP-05 | Modal — overlay/header/body/footer, elev-3, radius 12px | Exact diff vs current `Modal.svelte` (radius-md→radius-lg, elev-2→elev-3); everything else already correct |
</phase_requirements>

## Common Pitfalls

### Pitfall 1: Copying a `.dc` file's local `:root` demo values instead of the canonical `_tokens.scss`
**What goes wrong:** `Buttons.dc.html` and `Fields.dc.html` each embed their own standalone `:root` block
(needed so the file can be opened directly in a browser via `support.js`) with **different** alpha values
for `--tr-danger-ring`, and `Tabs.dc.html` has a different `--tr-accent-soft` than `Badges.dc.html` and the
canonical token. A literal "extract values from the reference" pass that trusts each file's own `:root`
block in isolation will produce **inconsistent** danger-ring/accent-soft values depending on which
component was transcribed from which file.
**Why it happens:** Each `.dc.html` file is a self-contained demo page; nothing enforces cross-file
consistency between them, and Phase 23's `_tokens.scss` convergence (e.g., the danger-ring 0.3→0.2 fix)
happened after some of these reference files' `:root` blocks were last touched.
**How to avoid:** Always resolve token *values* from `ui/src/styles/_tokens.scss` (the single source of
truth per Phase 23's closed-world model) and only use the `.dc` files for token *names/roles* (which
property uses which token) and non-token layout numbers (px sizes, border-radius, font-size). See the Token
Mismatches table above for the exact 3 places this applies.
**Warning signs:** `pnpm lint`'s `check-tokens.mjs` passing but colors visibly not matching Phase 23's
approved palette on inspection; or Rule 3 failing outright if `--tr-accent-text` is used without being added
to `_tokens.scss` first.

### Pitfall 2: Raw `rgba()`/hex literals inside new component `<style>` blocks silently blocked by `check-tokens.mjs`
**What goes wrong:** `Tabs.dc.html`'s segmented-active box-shadow (`0 1px 2px rgba(16,22,34,.12)`) has no
token equivalent. Pasting it verbatim into `Tabs.svelte`'s `<style lang="scss">` block will make
`pnpm lint` fail on Rule 4 (color-function-in-style gate) — this is not a lint suggestion, it's a hard CI
gate (`process.exit(1)` on any violation).
**Why it happens:** The reference file was authored as a Design Canvas demo where raw CSS values are fine;
the actual codebase enforces a closed token vocabulary as of Phase 23.
**How to avoid:** Resolve this before writing Tabs.svelte — either substitute `var(--tr-elev-1)` (nearest
existing shadow token, despite alpha mismatch) or flag it as needing a new token addition during planning/
discuss (see Open Questions). Do not discover this via a failing `pnpm lint` run after the component is
already written.
**Warning signs:** `pnpm lint` output containing `color-function literal rgba(...)` pointing at
`Tabs.svelte`.

### Pitfall 3: Hand-rolling Checkbox/Radio ARIA instead of using native inputs
**What goes wrong:** Building a `<div role="checkbox" tabindex="0">` from scratch to match the `.dc`
reference's decorative 18px box literally (since the reference itself is just a styled `<span>`) loses
native Space-key toggling, native radio-group arrow-key navigation, and native screen-reader semantics —
requiring a full custom keyboard-handler reimplementation that Phase 30 (QA-02, accessibility) would then
have to audit and likely rewrite anyway.
**Why it happens:** The `.dc.html` reference shows only the visual box, with no indication that a real
`<input>` should be hidden underneath — Design Canvas format has no interactive DOM.
**How to avoid:** Follow Pattern 2 (hidden native input + styled sibling span) exactly.
**Warning signs:** Checkbox/Radio components that require custom `onkeydown` handlers to toggle — a sign the
native input isn't doing the work it should.

### Pitfall 4: LAN-browser UAT sees a stale build after component edits
**What goes wrong:** Verifying the showcase (D-01's primary UAT surface) by opening the LAN-browser URL
shows old, unstyled components even after editing `.svelte` files, because server mode serves the
pre-built `ui/dist` directory, not a live dev server.
**Why it happens:** Only `cargo tauri dev` (desktop webview target) gets Vite HMR; the axum server's
`tower-http::ServeDir` in server mode serves whatever was last built to `ui/dist`.
**How to avoid:** Run `pnpm --dir ui build` before every LAN-browser verification pass of the showcase.
[Confirmed project convention — see project memory `dev_browser_testing_needs_ui_build.md`.]
**Warning signs:** Changes visible in the Tauri desktop window but not in a browser tab pointed at the LAN
server URL.

### Pitfall 5: `prebuild` script silently invokes `cargo test`
**What goes wrong:** `ui/package.json`'s `prebuild` script runs `cargo test -p trackly-app --test
export_bindings` — anyone running `pnpm --dir ui build` (needed per Pitfall 4) triggers a Rust test compile
first. If a second `cargo test`/`cargo build` is running concurrently elsewhere (e.g., in a parallel plan
task), this can appear to hang for minutes due to `target/` lock contention.
**Why it happens:** `prebuild` exists to regenerate TypeScript bindings from Rust `#[derive(TS)]` structs
before the Vite build runs.
**How to avoid:** Serialize `cargo test`/`pnpm build` invocations — do not run two `pnpm --dir ui build` (or
any cargo-touching command) concurrently. [Confirmed project convention — project memory
`cargo_no_concurrent_test.md`.]
**Warning signs:** A `pnpm build` that appears to hang for multiple minutes with no output.

## Code Examples

### Existing Svelte 5 runes pattern (verified, `ui/src/lib/components/Input.svelte`)
```typescript
// Source: ui/src/lib/components/Input.svelte (existing code, unmodified excerpt)
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

### Existing role-gated sidebar item pattern (verified, `ui/src/features/layout/sidebar-config.ts`)
```typescript
// Source: ui/src/features/layout/sidebar-config.ts (existing code)
export type SidebarItem = {
  kind: 'item';
  route: string;
  label: string;
  phase?: number | string;
  /** If set, only users with one of these roles see this item. Omit = visible to all. */
  roles?: UserRole[];
};
// Existing admin-only precedent:
{ kind: 'item', route: '/users', label: 'Пользователи', phase: 5, roles: ['admin'] },
{ kind: 'item', route: '/settings', label: 'Настройки', phase: 7, roles: ['admin'] },
```

### Existing working segmented-control precedent (verified, `ui/src/lib/components/ThemeSwitcher.svelte`)
```svelte
<!-- Source: ui/src/lib/components/ThemeSwitcher.svelte (existing code) -->
<!-- Already implements a 3-way segmented control with active-state styling — -->
<!-- directly reusable as an implementation reference for Tabs' `segmented` variant. -->
{#each options as opt}
  <button
    type="button"
    class="segment"
    class:active={themeStore.preference === opt.key}
    aria-pressed={themeStore.preference === opt.key}
    onclick={() => setTheme(opt.key)}
  >{opt.label}</button>
{/each}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|---------------|--------|
| `transition: none` on all interactive components (Button.svelte comment: "Theme switch: no transitions per UI-SPEC §Motion") | `transition: background .12s, box-shadow .12s` restored, suppressed only during theme switch (D-09) | Phase 23 → Phase 24 | Every component transition rule touched in CMP-01/02/03/04/05 must both restore the `.12s` transition AND rely on the new `.theme-switching` class (Pattern 4) instead of a permanent `transition: none` |
| Button secondary variant: `background: transparent` | `background: var(--tr-surface)` (matches `ctrlBase`-style solid-surface convention used everywhere else) | This phase | One-line change, but changes the visual identity of every secondary button in the app (used widely — "Отмена" buttons etc.) |

**Deprecated/outdated:** The permanent `transition: none` directive in `Button.svelte`/`ThemeSwitcher.svelte`
(both currently comment-tagged "Theme switch: no transitions per UI-SPEC §Motion") is explicitly reversed by
D-09 — the comment itself should be removed/updated as part of the transcription, not left as stale
documentation contradicting the new behavior.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `--tr-accent-text` should be added as a new token pair (`#2350bd` light / `#8fb0ff` dark) rather than mapped to an existing token | Token Mismatches, Badge/Tabs transcription | If the planner instead reuses `--tr-accent-hover` (which happens to equal `#2350bd` in light mode only, not dark), dark-mode Badge/Tabs text would be visually wrong (`#7099ff` accent-hover vs the intended `#8fb0ff` accent-text) — low risk since values are directly read from 2 independently-authored `.dc` files that agree, but adding a token still needs discuss-phase/planner sign-off since it technically reopens Phase 23's closed palette |
| A2 | The segmented-Tabs active-state box-shadow (`rgba(16,22,34,.12)`) should map to `var(--tr-elev-1)` rather than warrant a new token | Token Mismatches, Tabs transcription, Open Questions | If a pixel-perfect shadow match is required, `--tr-elev-1`'s different alpha (0.07 vs 0.12) and 2-layer vs 1-layer structure will look visibly lighter than the reference — low risk, this is a rarely-scrutinized micro-shadow, not a primary brand color |
| A3 | The showcase route needs no stricter access guard than the existing sidebar-only pattern used by `/users`/`/settings` | Pattern 3 (admin-gating) | If a reviewer expects defense-in-depth beyond precedent (e.g., an explicit page-level role check), the as-researched approach would need revision — low risk since it exactly matches 2 existing precedents and the showcase has no privileged data to protect |

**If this table is empty:** N/A — see entries above. All three are low-risk, evidence-based inferences from
directly-read source files, not speculative claims about unverified external facts.

## Open Questions (RESOLVED)

1. **Segmented-Tabs active box-shadow: `var(--tr-elev-1)` vs new token vs drop the shadow?**
   - **RESOLVED:** Use `var(--tr-elev-1)`. Locked into Plan 24-06 (grep-verified zero raw `rgba(` in Tabs `<style>`); accepted minor shadow-alpha variance rather than reopening the Phase 23 palette.
   - What we know: `Tabs.dc.html`'s `segStyle(act)` active state uses a raw, non-tokenized
     `rgba(16,22,34,.12)` 1-layer shadow; no existing `--tr-elev-*` token matches it exactly;
     `check-tokens.mjs` Rule 4 will hard-fail any raw `rgba()` in a `.svelte` `<style>` block.
   - What's unclear: Whether pixel-fidelity to this specific micro-shadow matters enough to justify adding a
     new token (reopening Phase 23's otherwise-closed palette) versus accepting the nearest existing token's
     visual drift.
   - Recommendation: Use `var(--tr-elev-1)` as the pragmatic default (documented visual variance is minor —
     a 1px 2px shadow at 7% vs 12% opacity). If the discuss-phase or a stricter UAT pass flags it, add it as
     a named exception in a follow-up plan rather than blocking this phase.

2. **Radio `bind:group` API shape on the new `Radio.svelte` component**
   - **RESOLVED:** Expose `group = $bindable()` (native `bind:group` semantics). Locked into Plan 24-03.
   - What we know: Native `<input type="radio" bind:group={value}>` is the standard Svelte mechanism; the
     `.dc` reference shows no group-binding behavior (Design Canvas isn't interactive there — it just shows
     2 static radio states, "selected" and "not selected", independently).
   - What's unclear: Whether the planner should expose `group = $bindable()` directly on `Radio.svelte`
     (parent binds a shared variable across multiple `<Radio>` instances, mirroring native `bind:group`) or
     keep `Radio.svelte` presentation-only (`checked`/`value` props, no group logic) and let consumers
     (Phase 26-28 screens, out of scope here) wire the group state themselves.
   - Recommendation: Expose `group = $bindable()` — it's the lower-friction API for phases 26-28's future
     retrofit work (device-type radio selection, mentioned in CONTEXT.md D-04's rationale) and matches how
     native HTML solves this exact problem.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Node.js | Vite build, svelte-check | ✓ | v22.18.0 [VERIFIED] | — |
| pnpm | Package scripts (`lint`, `svelte-check`, `build`) | ✓ | 10.17.1 [VERIFIED] | — |
| `check-tokens.mjs` (project script) | `pnpm lint` gate | ✓ | in-repo, `ui/scripts/check-tokens.mjs` [VERIFIED, read in full] | — |

No missing dependencies — this phase adds zero external tooling.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | None (no vitest/playwright in this project — confirmed project convention) |
| Config file | none |
| Quick run command | `pnpm --dir ui lint` (eslint + prettier --check + `check-tokens.mjs`) |
| Full suite command | `pnpm --dir ui lint && pnpm --dir ui svelte-check` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| CMP-01 | Button 5×2×6 states render distinct styles | manual-only (visual) | showcase page section "Кнопки" — visual inspection in both themes | ❌ Wave 0 (showcase page doesn't exist yet) |
| CMP-02 | Input/Select/Textarea/Checkbox states distinct | manual-only (visual) | showcase page section "Поля ввода" | ❌ Wave 0 |
| CMP-03 | Badge 5 tones × 4 appearances render + 21 call-sites unchanged | manual-only (visual) + automated (grep) | showcase page section "Бейджи"; `grep -c "<Badge" ui/src -r` returns 21 unchanged call-sites, zero prop-shape diffs via `git diff --stat` on those 21 files | ❌ Wave 0 (showcase); grep check is trivially automatable in a verification task |
| CMP-04 | Tabs underline + segmented variants show active/counter states | manual-only (visual) | showcase page section "Вкладки" | ❌ Wave 0 |
| CMP-05 | Modal overlay/header/body/footer, elev-3, radius-lg | manual-only (visual) | showcase page "Показать модал" trigger | ❌ Wave 0 |

**Automated gates that DO apply per-task:** `pnpm --dir ui lint` (catches token-name typos, raw hex/rgba,
old-family token names, prettier/eslint violations) and `pnpm --dir ui svelte-check` (catches TypeScript/
Svelte compile errors, e.g. a malformed `$bindable`/`Snippet` prop) run after every component edit — these
are real automated regression gates even though there is no dedicated component test framework.

### Sampling Rate
- **Per task commit:** `pnpm --dir ui lint` (fast, catches token/format violations immediately)
- **Per wave merge:** `pnpm --dir ui lint && pnpm --dir ui svelte-check`
- **Phase gate:** Full suite green + `pnpm --dir ui build` succeeds + manual showcase walkthrough (both
  themes, both Tauri desktop and LAN browser per QA-03 precedent — though full cross-platform parity
  verification is Phase 30's job, a basic "does it render" check here catches gross regressions early)

### Wave 0 Gaps
- [ ] `ComponentShowcasePage.svelte` (or equivalent) — the primary manual-verification surface for all 5
  CMP requirements; must exist before any primitive's states can be visually confirmed
- [ ] Route + sidebar entry wiring (`routes.ts`, `sidebar-config.ts`) — needed before the showcase is
  reachable at all
- No test framework install needed — this project deliberately has none (frontend verification is
  lint + svelte-check + human visual review by design, confirmed project convention)

## Security Domain

`security_enforcement: true` in `.planning/config.json`, but this phase introduces no new data flows, no new
API endpoints, no new authentication/authorization logic, and no new external input parsing — it is a pure
CSS/markup transcription phase plus one client-side route addition reusing an existing, already-reviewed
authorization pattern (sidebar role filtering). ASVS impact is minimal and narrowly scoped:

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | No | Unchanged — showcase reuses the existing `authStore`/session mechanism, no new auth code |
| V3 Session Management | No | Unchanged |
| V4 Access Control | Yes (minor) | Showcase route restricted via `roles: ['admin']` sidebar filter — same mechanism as existing `/users`/`/settings` (Pattern 3). No new authorization logic is introduced; this is scope-matching an existing, already-shipped pattern, not a new control surface. |
| V5 Input Validation | No | Showcase renders only static/hardcoded demo data — no user input is accepted or rendered anywhere in this phase's scope |
| V6 Cryptography | No | Not applicable |

### Known Threat Patterns for this phase's stack
No new threat surface is introduced. The one access-control note (V4) is a UI-layer convenience gate
consistent with 2 existing precedents in the codebase, not a security boundary — actual protection of any
sensitive data remains enforced server-side (unchanged, out of scope for this phase) per the project's
existing "all API calls go through backend authorization regardless of UI sentinel" design note
(`App.svelte` D-Desktop-01/02 comments).

## Sources

### Primary (HIGH confidence — direct file reads in this repository)
- `.planning/phases/24-base-components/24-CONTEXT.md` — locked decisions D-01…D-09, canonical refs list
- `.planning/reference/design-system-v2/Buttons.dc.html` — full `styleFor`/`base`/`renderVals` read
- `.planning/reference/design-system-v2/Fields.dc.html` — full `ctrlBase`/`box`/`renderVals` read
- `.planning/reference/design-system-v2/Badges.dc.html` — full `renderVals`/`TONE` read
- `.planning/reference/design-system-v2/Tabs.dc.html` — full `tabStyle`/`badgeStyle`/`segStyle` read
- `.planning/reference/design-system-v2/Modal.dc.html` — full `renderVals` read
- `ui/src/styles/_tokens.scss` — full read, cross-diffed against all 5 `.dc` files above
- `ui/scripts/check-tokens.mjs` — full read (4-rule lint gate, directly informs Pitfall 1/2)
- `ui/src/lib/components/Button.svelte`, `Input.svelte`, `Select.svelte`, `Textarea.svelte`, `Badge.svelte`,
  `Modal.svelte`, `Spinner.svelte`, `ThemeSwitcher.svelte` — full reads, diffed against reference values
- `ui/src/App.svelte`, `ui/src/routes.ts`, `ui/src/features/layout/Sidebar.svelte`,
  `ui/src/features/layout/sidebar-config.ts`, `ui/src/lib/stores/theme.svelte.ts`,
  `ui/src/styles/global.scss`, `ui/src/pages/AccessDenied.svelte`, `ui/src/pages/UsersPage.svelte` — full
  reads for routing/nav/theme/admin-gating patterns
- `ui/src/features/acts/ActsSearchAndTabs.svelte` — read for existing hand-rolled switch-bar pattern
  (confirms out-of-scope retrofit target, D-07)
- `ui/package.json` — dependency versions, script definitions
- Badge call-sites: `grep -rn "<Badge" ui/src` (21 real usages) and `grep -rn "BadgeVariant" ui/src` (5
  local type declarations, all include `'destructive'`)
- `.planning/REQUIREMENTS.md`, `.planning/ROADMAP.md` — CMP-01..05 text, Phase 24 success criteria
- `.planning/STATE.md` — Phase 23 completion notes, danger-ring 0.3→0.2 convergence decision (23-08)
- `.planning/config.json` — `nyquist_validation: true`, `security_enforcement: true`

### Secondary (MEDIUM confidence)
None used — all findings for this phase were directly verifiable in-repo; no external library research was
required since no new packages are introduced.

### Tertiary (LOW confidence)
None.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — zero new dependencies, all existing tooling directly inspected
- Architecture (routing/nav/admin-gating/theme-switch hook): HIGH — every claim backed by a direct file read
  with exact line-level evidence, not inference
- Component style values (Button/Fields/Badge/Tabs/Modal): HIGH — transcribed directly from each `.dc.html`
  file's embedded script, cross-checked token-by-token against `_tokens.scss`
- Token mismatches: HIGH — verified by literal diff of `:root`/`[data-theme]` blocks across all 5 reference
  files vs the canonical token file
- Pitfalls: HIGH — 3 of 5 are directly reproducible from reading `check-tokens.mjs`'s source and the
  reference files' raw values; 2 are confirmed project conventions from persistent memory

**Research date:** 2026-07-18
**Valid until:** No expiry driver — all facts are pinned to this repository's current state (commit
`938129e` at research time), not to any external library's release cadence. Re-verify only if
`_tokens.scss`, the `.dc.html` reference files, or `sidebar-config.ts`/`routes.ts` change before planning
executes.

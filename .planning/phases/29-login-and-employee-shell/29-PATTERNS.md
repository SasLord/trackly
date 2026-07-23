# Phase 29: Вход и интерфейс сотрудника — Pattern Map

**Mapped:** 2026-07-23
**Files analyzed:** 6 modify + 1–2 new (auth-shell / FormField, D-02)
**Analogs found:** 7 / 7 (все объекты фазы имеют прямой внутренний прецедент; общих npm/внешних паттернов не требуется)

> Фаза чисто визуальная (SC #1–#3, `<domain>` "Вне границ"): auth-роутинг `LoginPage` (screen state,
> коды ошибок `REGISTRATION_PENDING`/`ACCESS_BLOCKED`/`SERVICE_UNAVAILABLE`, anti-enumeration D-Sec-01),
> WS-логика `EmployeeLayout`, reserved-SSO как нерабочая заглушка (D-UX-03) — **не трогаются**. Всё ниже
> — про замену bespoke-разметки на примитивы и извлечение общего auth-shell, форма из системы
> (`Fields.dc.html`/`Buttons.dc.html`), содержание из приложения (прецедент Фазы 27 "без макета").
> Токены `--tr-*` во всех файлах фазы уже валидны — ниже указаны только реально существующие имена,
> проверенные по `ui/src/styles/_tokens.scss`.

---

## File Classification

| Файл | Роль | Data Flow | Ближайший аналог | Качество |
|------|------|-----------|-------------------|----------|
| `ui/src/features/auth/LoginPage.svelte` | screen (auth form) | request-response | себя же (внутренняя ре-токенизация форм-контролов на примитивы) + `devices/DeviceFormBody.svelte` (`.field`/`.label`/`.field-error` конвенция) | role-match |
| `ui/src/features/auth/FirstRunWizard.svelte` | screen (auth form, create-user) | CRUD (создание первого админа) | то же — `LoginPage.svelte` (сестринский экран, идентичная bespoke-разметка) | exact (внутри фазы) |
| `ui/src/features/auth/PendingScreen.svelte` | screen (status, read-only) | transform (display-only) | `LoginPage`/`BlockedScreen` card-chrome → новый auth-shell (D-02) | role-match |
| `ui/src/features/auth/BlockedScreen.svelte` | screen (status + conditional CTA) | request-response (`request_ad_restore`) | `LoginPage`/`PendingScreen` card-chrome → новый auth-shell (D-02) | role-match |
| `ui/src/features/layout/EmployeeLayout.svelte` | layout shell (header-only) | event-driven (WS, не трогать) + transform (header chrome) | `ui/src/lib/components/PageHeader.svelte` (header structure/tokens) | role-match |
| `ui/src/lib/components/Input.svelte` | primitive (form control) | transform | себя же — точечное расширение `type` union | exact (extend, not fork) |
| **NEW** auth-shell (`ui/src/lib/components/AuthShell.svelte`, имя на усмотрение) | layout primitive (Snippet-based) | container/transform | `ui/src/lib/components/PageHeader.svelte` + `DetailPanel.svelte` (extraction precedent) | new-artifact, strong precedent |
| **NEW** field wrapper (`ui/src/lib/components/FormField.svelte`, может быть частью того же файла/модуля) | form-field primitive | transform | `ui/src/lib/components/DetailField.svelte` (label/value pair) + `devices/DeviceFormBody.svelte` `.field`/`.label`/`.field-error` convention | new-artifact, strong precedent |

---

## Pattern Assignments

### `ui/src/features/auth/LoginPage.svelte` (screen, request-response)

**Do NOT touch:** `handleSubmit`, `screen` state routing, `GENERIC_AUTH_ERROR`/`AD_UNREACHABLE_ERROR`
constants, the `code === 'REGISTRATION_PENDING' | 'ACCESS_BLOCKED' | 'SERVICE_UNAVAILABLE'` branching
(lines 65–85 as currently written). Only the markup/style layer changes.

**Current bespoke markup to replace** (lines 94–161):
```svelte
<div class="login-container">
  <div class="login-card">
    <h1 class="login-title">Вход в систему</h1>
    <form class="login-form" onsubmit={...}>
      <div class="form-field">
        <label class="form-label" for="login-input">Логин</label>
        <input id="login-input" class="form-input" class:is-error={loginError !== null}
               type="text" placeholder="Логин" bind:value={login} disabled={loading}
               autocomplete="username" />
        {#if loginError}<span class="field-error">{loginError}</span>
        {:else}<span class="format-hint">Логин: us100, user@domain или DOMAIN\user</span>{/if}
      </div>
      <!-- password field, checkbox, server-error, submit button, reserved-SSO button -->
    </form>
  </div>
</div>
```

**Target shape (D-01 + D-02) — outline, not literal executor code:**
- Wrap the whole screen in the new auth-shell component (title = "Вход в систему").
- Each `.form-field` block → the new field-wrapper component (label + `Input`/`Checkbox` +
  error/hint), preserving the `id`/`for` pairing and `aria-describedby` wiring already implicit in
  the bespoke markup (currently `.field-error`/`.format-hint` are visually adjacent but NOT wired via
  `aria-describedby` — this is a pre-existing gap D-01 explicitly asks the field-wrapper to close: "use
  `invalid` + `aria-describedby` → id ошибки/хинта").
- `<input type="text">` for login → `<Input type="text" bind:value={login} invalid={loginError !== null} ... />`.
- `<input type="password">` → `<Input type="password" ...>` (**requires the `type` union extension
  below** — currently `Input` only supports `text|number|search`, so `type="password"` cannot be passed
  as-is).
- Checkbox → `<Checkbox bind:checked={remember} disabled={loading}>Запомнить меня</Checkbox>` (label as
  Snippet child, matching `Checkbox.svelte`'s own `{@render children?.()}` contract — see excerpt below).
- Submit button → `<Button type="submit" variant="primary" loading={loading}>Войти</Button>` (Button's
  own `loading` prop already renders the spinner + disables — replaces the manual
  `{#if loading}Вход...{:else}Войти{/if}` text-swap).
- Reserved-SSO button → `<Button type="button" variant="ghost" disabled>Вход по учётной записи
  Windows (скоро)</Button>` — **no `onclick`**, and Button's own `disabled` already sets
  `pointer-events: none` (see Button excerpt below), so the explicit `tabindex="-1"` on the raw
  `<button>` is redundant once ported (Button's `:disabled` styling already removes it from the tab
  order via `disabled` attribute — verify no regression to the "unclickable, unfocusable" D-UX-03
  contract when porting).
- `.server-error` banner div — no primitive exists for this; keep as local scoped markup/class (not in
  scope of D-01's primitive list) unless the executor finds a natural place in the new auth-shell
  (Claude's Discretion).

---

### `ui/src/features/auth/FirstRunWizard.svelte` (screen, CRUD)

Sibling of `LoginPage.svelte` — near-identical bespoke card/form/field/`.btn-submit` structure (4 raw
`<input>`, all `type="text"`/`type="password"`, no checkbox, no reserved-SSO). Apply the **same**
Input/Button/field-wrapper/auth-shell substitution as `LoginPage.svelte` above. Concretely:

**Current bespoke input** (lines 101–114, repeated 4× for login/fullName/password/confirmPassword):
```svelte
<div class="form-field">
  <label class="form-label" for="wiz-login">Логин</label>
  <input id="wiz-login" class="form-input" class:is-error={loginErr !== null}
         type="text" placeholder="Логин (не менее 3 символов)" bind:value={login}
         disabled={loading} autocomplete="username" />
  {#if loginErr}<span class="field-error">{loginErr}</span>{/if}
</div>
```
→ field-wrapper component wrapping `<Input type="text" bind:value={login} invalid={loginErr !== null}
disabled={loading} placeholder="..." autocomplete="username" />`, error text passed as the wrapper's
error slot/prop. The 2 password inputs (`wiz-password`, `wiz-confirm`) need `type="password"` — same
`Input` extension dependency as `LoginPage`.

**Do NOT touch:** `validate()` field-length rules, `handleSubmit`'s `users_create` + auto-`auth_login`
sequence (lines 46–84).

---

### `ui/src/features/auth/PendingScreen.svelte` (screen, transform/display-only)

Simplest of the 4 — a title (`h1.login-title`), one paragraph (`p.screen-body`), one `Button`-shaped
link (`button.btn-link`, currently raw). No form fields, no `Input`/`Checkbox` involved — D-01 does not
apply here, only D-02 (shell extraction).

**Current bespoke card** (lines 12–23):
```svelte
<div class="login-container">
  <div class="login-card">
    <h1 class="login-title">Заявка отправлена</h1>
    <p class="screen-body">...</p>
    <button class="btn-link" type="button" onclick={onBackToLogin}>
      Войти под другим пользователем
    </button>
  </div>
</div>
```
→ Port `.login-container`/`.login-card`/`.login-title` chrome into the new auth-shell (title prop +
default slot/Snippet for body). `.btn-link` → `<Button variant="link" onclick={onBackToLogin}>Войти под
другим пользователем</Button>` (Button already has a `link` variant, see excerpt below — this is a
straight swap, not a new pattern). **No state/content changes** — same two lines of copy, same single
CTA.

---

### `ui/src/features/auth/BlockedScreen.svelte` (screen, request-response)

Same card chrome as `PendingScreen`/`LoginPage`, but 4 conditional branches (`submitted` /
`blockedDetails.pending` / `blockedDetails.rejection_reason` / default) each rendering a subset of:
title, body paragraph, optional `.server-error` banner, optional `.btn-submit` (restore CTA), always a
`.btn-link` (back-to-login). **Do NOT touch** the branch logic (lines 63–109) or `handleRestoreRequest`
(lines 42–58) — only swap `.btn-submit`→`Button variant="primary" loading={submitting}` and
`.btn-link`→`Button variant="link"` inside each branch, and wrap the whole thing in the new auth-shell.

**Current bespoke CTA pair** (lines 103–106, one of 4 near-identical occurrences):
```svelte
<button class="btn-submit" type="button" disabled={submitting} onclick={handleRestoreRequest}>
  {#if submitting}Отправка…{:else}Запросить восстановление доступа{/if}
</button>
<button class="btn-link" type="button" onclick={onBackToLogin}>
  Войти под другим пользователем
</button>
```
→ `<Button variant="primary" loading={submitting} onclick={handleRestoreRequest}>Запросить
восстановление доступа</Button>` + `<Button variant="link" onclick={onBackToLogin}>Войти под другим
пользователем</Button>`. `loading` replaces the manual `{#if submitting}` text-swap uniformly, same as
`LoginPage`'s submit button.

---

### `ui/src/features/layout/EmployeeLayout.svelte` (layout shell, header-only — D-03)

**Scope: header chrome only.** `onMount`/`connectWs`/`onWsEvent`/`handleEmployeeWsEvent`/`logout` logic
(lines 22–95) is untouched — this is a consistency pass on `.employee-header`/`.employee-brand`/
`.user-name`/`.user-role`/`.skip-link`, not a rewrite.

**Analog — `ui/src/lib/components/PageHeader.svelte`** (the app's other header, used inside
`Layout.svelte`'s content area):
```scss
.page-header {
  position: sticky;
  top: 0;
  z-index: 20;
  height: var(--header-height);      // <- no fallback; token is guaranteed (closed-world gate)
  padding: 0 24px;
  background: var(--tr-surface);
  border-bottom: 1px solid var(--tr-border);
}
```

**Current EmployeeLayout equivalent** (lines 127–136 + 168):
```scss
.employee-header {
  height: var(--header-height, 56px);   // <- redundant fallback; --header-height IS defined (56px) in _tokens.scss
  ...
}
.employee-content {
  ...
  min-height: calc(100vh - 56px);       // <- hardcoded 56px instead of var(--header-height)
}
```
**Consistency-pass action:** drop the `, 56px` fallback on `.employee-header`'s `height` (matches
`PageHeader.svelte`'s convention — `--header-height` is a real token, confirmed at
`ui/src/styles/_tokens.scss:190`) and replace the hardcoded `56px` in `.employee-content`'s
`calc(100vh - 56px)` with `var(--header-height)`. `Button`/`ThemeSwitcher` are already primitives here
— nothing to swap on that front. `.skip-link` already matches `Layout.svelte`'s own `.skip-link`
verbatim (byte-for-byte identical block) — no change needed.

**Do NOT add:** a sidebar or nav toggle (D-03 — `PageHeader`'s `nav-toggle` button is Sidebar-specific
and out of scope for the header-only employee shell).

---

### `ui/src/lib/components/Input.svelte` (primitive — D-01 extension point)

**Exact spot to extend** (line 5, the `type` union in `Props`):
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
  iconLeft?: Snippet;
}
```
→ extend to `type?: 'text' | 'number' | 'search' | 'password';` (add `'email'` only if an executor
finds an actual `type="email"` use-case in scope; CONTEXT.md leaves this to discretion, but nothing in
this phase's 4 screens uses `email`).

**Existing `invalid`/`aria-describedby` wiring to preserve exactly** (lines 34–50):
```svelte
<input
  {type}
  {id}
  {placeholder}
  {disabled}
  class="input"
  class:invalid
  class:has-icon={!!iconLeft}
  {value}
  aria-describedby={ariaDescribedby}
  aria-invalid={invalid || undefined}
  oninput={(e) => {
    const v = (e.currentTarget as HTMLInputElement).value;
    value = v;
    oninput?.(v);
  }}
/>
```
This is untouched by the `type` union extension — `type="password"` flows straight through the
existing `{type}` spread with zero other changes. **`$bindable` contract note (Phase 24 lesson):** line
19, `value = $bindable('')`, is declared with `let`, not `const` — if the executor refactors this file,
preserve `let` (Phase 24 `24-LEARNINGS.md`: `const` vs `let` on a `$bindable()` prop is a contract, not
style — `const` silently breaks two-way binding for every consumer: Devices, Requests, Settings,
Showcase, `Dropdown.svelte`, and now the 2 auth screens).

**Consumers that must keep working unmodified** (regression surface for this change — 19 files import
`Input.svelte`, all currently pass `type="text"` or omit `type` — a purely additive union member cannot
break any of them): `features/cartridges/{CartridgeFormBody,CartridgesSearchAndTabs,CompatibilityEditor,
ModelFormModal}.svelte`, `features/settings/{BackupSettings,NetworkSettings,ThresholdSettings,
ActiveDirectorySettings,OrgSettings}.svelte`, `features/acts/{ActsSearchAndTabs,ActNumberField,
ReturnModal,ActFormItemsTable,ActFormBody,ReturnItemsTable}.svelte`, `features/printers/
{PrintersSearchAndTabs,DiscoveryModal,PrinterCreateModal}.svelte`, `features/showcase/sections/
FieldsSection.svelte`, `features/users/UserFormModal.svelte`, `features/devices/{DeviceFormBody,
DeviceFilters}.svelte`, `lib/components/Dropdown.svelte`.

---

### NEW: auth-shell component (D-02)

**Extraction precedent — `ui/src/lib/components/PageHeader.svelte`** (Snippet-based Props, no external
state coupling beyond what's passed in) and **`ui/src/lib/components/DetailPanel.svelte`** (title +
Snippet actions + Snippet children, scoped SCSS, single responsibility — "does not paint a background"
comment in `DetailPanel.svelte` line 7 is a useful precedent: decide explicitly whether the new
auth-shell owns the `min-height: 100vh; background: var(--tr-bg)` centering wrapper, or whether that
stays at the call-site — all 4 screens currently duplicate it identically, so the shell owning it is
the D-02-consistent choice).

**Svelte 5 runes shape to follow** (from `DetailPanel.svelte` lines 8–29):
```svelte
<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    title?: string;
    children?: Snippet;
  }

  const { title, children }: Props = $props();
</script>

<div class="auth-shell">
  <div class="auth-card">
    {#if title}<h1 class="auth-title">{title}</h1>{/if}
    {@render children?.()}
  </div>
</div>
```
(`const`, not `let` — this component has no `$bindable` prop, so `const` is correct here per the
Phase 24 `const`-vs-`let` distinction: only bindable props require `let`.)

**Chrome to consolidate — identical across all 4 screens today** (compare `LoginPage.svelte` lines
164–188, `FirstRunWizard.svelte` lines 179–203, `PendingScreen.svelte` lines 26–51,
`BlockedScreen.svelte` lines 114–142 — all four are near-byte-identical modulo `max-width` (360px vs
400px) and `text-align`/`flex-direction` tweaks on the status screens):
```scss
.login-container {           // -> .auth-shell
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 100vh;
  background: var(--tr-bg);
}
.login-card {                // -> .auth-card
  background: var(--tr-surface);
  border: 1px solid var(--tr-border);
  border-radius: var(--tr-radius-lg);
  padding: var(--tr-space-2xl) var(--tr-space-4xl, 2rem);
  width: 100%;
  max-width: 360px;           // FirstRunWizard uses 400px — expose as a prop or pick one value (Claude's Discretion)
  box-shadow: var(--tr-elev-2);
}
.login-title {                // -> .auth-title
  margin: 0 0 var(--tr-space-xl);
  font-size: var(--tr-font-size-h3);
  font-weight: var(--tr-font-weight-semibold);
  color: var(--tr-text-primary);
  text-align: center;
}
```
Note: `var(--tr-space-4xl, 2rem)` — this fallback is pre-existing in ALL 4 current files (not
phase-29-introduced); `_tokens.scss` should be checked for whether `--tr-space-4xl` actually exists — if
it does, the fallback is dead and can be dropped in the same spirit as the `EmployeeLayout`
`--header-height` cleanup above (verify against `check-tokens.mjs` gate before removing).

---

### NEW: field-wrapper / `FormField` component (D-01, part of D-02's artifact)

**Closest analog — `ui/src/lib/components/DetailField.svelte`** (label/value pair, same
`display:flex; flex-direction:column; gap: var(--tr-space-3xs/2xs)` shape) — but `DetailField` is
read-only (label + static value); the new wrapper needs label + **interactive child** (`Input`/
`Checkbox` via Snippet) + optional error + optional hint, closer to the ad-hoc `.field`/`.label`/
`.field-error` convention already used at every non-auth form call-site:

**Existing app-wide convention to match** (`ui/src/features/devices/DeviceFormBody.svelte` lines
196–211 + 405–409 — this is NOT a component today, just a repeated class convention; D-02 promotes it
to a component for the auth surface specifically):
```svelte
<div class="field" class:has-error={!!fieldErrors['name']}>
  <label class="label" for="f-name">Наименование <span class="required">*</span></label>
  <Input id="f-name" value={name} invalid={!!fieldErrors['name']} oninput={(v) => (name = v)} />
  {#if fieldErrors['name']}<p class="field-error">{fieldErrors['name']}</p>{/if}
</div>
```
```scss
.field { display: flex; flex-direction: column; gap: var(--tr-space-2xs); }
.label { font-size: var(--tr-font-size-label); font-weight: var(--tr-font-weight-medium); color: var(--tr-text-primary); }
.field-error { margin: 0; font-size: var(--tr-font-size-label); color: var(--tr-danger); }
```

**Design-system source of truth — `Fields.dc.html`** (no `.dc` mockup exists for auth screens; this is
the canonical field reference per the "no-mockup" playbook, Phase 27 D-01): label above control
(13px/500 weight, `--tr-text-secondary`), hint/error below control. **Important discrepancy to resolve
before implementing:** `Fields.dc.html`'s error hint style uses `color: var(--tr-danger-text)` (a
distinct, slightly different token — confirmed present at `_tokens.scss:54` light / `:131` dark), while
the current app-wide `.field-error` convention (`DeviceFormBody.svelte` and all 4 auth screens today)
uses `var(--tr-danger)`. Both tokens exist and are valid — pick one and apply consistently in the new
`FormField` (Claude's Discretion notes this exact ambiguity is "values with no precedent — derive from
`_tokens.scss`, `Fields.dc`/`Buttons.dc`"). Recommend `--tr-danger-text` since it is the `.dc` file's
literal choice and no other in-scope file conflicts with switching (only the auth screens are in scope
for this phase, and they're being rewritten anyway).

**Format-hint (non-error) styling** — only `LoginPage.svelte` has one (`.format-hint`, lines 237–241):
```scss
.format-hint {
  font-size: var(--tr-font-size-label);
  color: var(--tr-text-tertiary);
  line-height: var(--tr-line-height-label);
}
```
Matches `Fields.dc.html`'s non-error hint (`color: var(--tr-text-tertiary)`, no distinct
`--tr-danger-text`-style swap needed for the OK case).

---

## Shared Patterns

### Button variants (submit / reserved-SSO / link / restore-CTA)
**Source:** `ui/src/lib/components/Button.svelte`
**Apply to:** `LoginPage` (submit=`primary`+`loading`, reserved-SSO=`ghost`+`disabled`),
`FirstRunWizard` (submit=`primary`+`loading`), `PendingScreen`/`BlockedScreen` (back-link=`link`,
restore-CTA=`primary`+`loading`).
```svelte
<button {type} class="btn btn-{variant} btn-{size}" class:loading disabled={isDisabled} {onclick}>
  {#if loading}<Spinner size="sm" />{/if}
  {@render children?.()}
</button>
```
`loading` prop already renders the spinner and folds into `isDisabled` — do not hand-roll a
`{#if loading}Текст...{:else}Текст{/if}` swap anymore; pass the label as `children` unconditionally and
let `loading` own the disabled/spinner state.

### Checkbox (remember-me)
**Source:** `ui/src/lib/components/Checkbox.svelte`
**Apply to:** `LoginPage`'s "Запомнить меня".
```svelte
<Checkbox bind:checked={remember} disabled={loading}>Запомнить меня</Checkbox>
```
Label is a Snippet child (`{@render children?.()}` inside the component, not a separate `label` prop).

### Input invalid/aria wiring
**Source:** `ui/src/lib/components/Input.svelte` lines 34–50 (see full excerpt above).
**Apply to:** every text/password field across `LoginPage`/`FirstRunWizard`. `invalid` drives both the
`.invalid` CSS class and `aria-invalid`; `aria-describedby` must be threaded from the new `FormField`
wrapper (pointing at the error/hint element's `id`) — this is currently NOT done in the bespoke markup
(errors are visually adjacent only) and is an explicit ask in D-01's "Действие для планировщика".

### Server-error banner (no primitive — keep as scoped local markup)
**Source:** identical block repeated in `LoginPage.svelte` (lines 145–147, `.server-error`) and
`BlockedScreen.svelte` (lines 86–88/100–102, same class):
```svelte
<div class="server-error">{serverError}</div>
```
```scss
.server-error {
  padding: var(--tr-space-xs) var(--tr-space-md);
  background: color-mix(in srgb, var(--tr-danger) 10%, transparent);
  border: 1px solid color-mix(in srgb, var(--tr-danger) 30%, transparent);
  border-radius: var(--tr-radius-xs);
  font-size: var(--tr-font-size-body);
  color: var(--tr-danger);
}
```
No `Button`/`Input`/`Checkbox`-equivalent primitive exists for this banner shape in
`ui/src/lib/components/`. Not required to extract in this phase (D-02 scope is shell + field-wrapper
only) — keep as a local scoped class in each of the 2 consumers, or fold into the auth-shell as an
optional Snippet slot if the executor judges it low-risk (Claude's Discretion; not a hard requirement).

---

## No Analog Found

None — every file in scope has either a direct sibling analog within `features/auth/` (the 4 screens
mirror each other's bespoke chrome closely enough to treat one as the template for all) or a primitive
analog in `ui/src/lib/components/` (`PageHeader`, `DetailPanel`, `DetailField`, `Button`, `Checkbox`).

---

## Traps To Avoid (project-wide, confirmed relevant to this phase's files)

1. **`:global()` in plain `.scss` lands verbatim in built CSS and does not work** (Phase 24 lesson,
   `24-LEARNINGS.md`). None of the 6 files currently modified use `:global()` — keep it that way; if the
   new auth-shell needs to reach into a Snippet-rendered child's markup, do it via a class passed as a
   prop / scoped selector on the shell's own wrapper element, not `:global()`.
2. **`const` vs `let` with `$bindable()` is a contract, not style** (Phase 24 lesson). `Input.svelte`
   line 19 (`value = $bindable('')`) and `Checkbox.svelte` line 14 (`checked = $bindable(false)`) both
   use `let` today — preserve this exactly if either file is touched beyond the additive `type` union
   change. A new `FormField`/auth-shell component has no `$bindable` prop of its own (it forwards
   `Input`/`Checkbox` bindings via Snippet composition, not by re-declaring bindable props) — use
   `const` there, matching `DetailPanel.svelte`/`PageHeader.svelte`.
3. **`check-tokens.mjs` closed-world gate** drops the build on any reference to a nonexistent `--tr-*`
   token. Every token named in this document (`--tr-space-4xl`, `--tr-danger-text`, `--header-height`,
   etc.) was cross-checked against `ui/src/styles/_tokens.scss` (lines 50–54, 127–131, 190) — but the
   executor must re-verify with `pnpm --dir ui build` (or grep `_tokens.scss` directly) before removing
   any fallback (e.g. `var(--tr-space-4xl, 2rem)`, `var(--header-height, 56px)`), since a removed
   fallback that turns out to reference an undefined token fails the build immediately.
4. **Frontend has no tests** — verification is `pnpm lint` + `pnpm svelte-check` + `pnpm --dir ui build`
   + eyes-on-screen in both themes (D-18 Phase 26 precedent). LAN-browser testing requires
   `pnpm --dir ui build` first (server mode serves `ui/dist`, `cargo tauri dev` only HMRs the desktop
   webview) — project memory gotcha, applies directly to verifying `EmployeeLayout` in server/LAN mode.

## Metadata

**Analog search scope:** `ui/src/features/auth/`, `ui/src/features/layout/`, `ui/src/features/devices/`
(field-convention reference), `ui/src/lib/components/` (all primitives + PageHeader/DetailPanel/
DetailField), `.planning/reference/design-system-v2/Fields.dc.html` + `Buttons.dc.html`,
`ui/src/styles/_tokens.scss`.
**Files scanned:** 4 auth screens (read in full), `Input.svelte`/`Button.svelte`/`Checkbox.svelte`/
`PageHeader.svelte`/`DetailPanel.svelte`/`DetailField.svelte`/`EmployeeLayout.svelte`/`Layout.svelte`
(read in full), `DeviceFormBody.svelte` (targeted read, field convention section),
`Fields.dc.html` (read in full), `_tokens.scss` (targeted grep for `danger`/`header-height`), 19
`Input`-consumer files (path-listed via grep, not individually read — additive union change does not
require reading their internals).
**Pattern extraction date:** 2026-07-23

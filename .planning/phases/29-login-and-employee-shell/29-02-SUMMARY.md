---
phase: 29-login-and-employee-shell
plan: 02
subsystem: ui
tags: [svelte, svelte5-runes, design-system, auth, forms]

# Dependency graph
requires:
  - phase: 29-login-and-employee-shell
    plan: 01
    provides: "Input.svelte (type='password'), AuthShell.svelte, FormField.svelte"
provides:
  - "LoginPage.svelte on primitives + AuthShell + FormField, zero bespoke form-control CSS"
  - "FirstRunWizard.svelte on primitives + AuthShell + FormField, zero bespoke form-control CSS"
  - "Input.svelte: autocomplete prop (HTMLInputAttributes['autocomplete']) — additive pass-through"
affects: [29-03-pending-blocked-employee-shell]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "FormField snippet-with-params consumption: {#snippet children({ describedBy, invalid })} forwards straight into <Input {invalid} aria-describedby={describedBy} />"
    - "Button loading prop replaces manual {#if loading}Text...{:else}Text{/if} swaps — spinner + disabled handled by the primitive"

key-files:
  created: []
  modified:
    - ui/src/features/auth/LoginPage.svelte
    - ui/src/features/auth/FirstRunWizard.svelte
    - ui/src/lib/components/Input.svelte

key-decisions:
  - "Rule 3 auto-fix: added `autocomplete?: HTMLInputAttributes['autocomplete']` to Input.svelte's Props, passed through to the native <input> — the plan's Input usage on both screens requires autocomplete=\"username\"/\"current-password\"/\"name\"/\"new-password\", but Input.svelte's Props interface from 29-01 had no such field. Without it, svelte-check fails with 'Property autocomplete does not exist'. First attempt typed it as plain `string`, which failed TS's stricter `FullAutoFill | null | undefined` constraint on the native attribute — corrected to the proper `HTMLInputAttributes['autocomplete']` type from svelte/elements."
  - "Reserved-SSO Button on LoginPage renders with no onclick prop and no explicit tabindex — Button's own disabled attribute already removes it from tab order (D-UX-03), matching the plan's interface note."

requirements-completed: [WIN-10]

# Metrics
duration: 12min
completed: 2026-07-23
---

# Phase 29 Plan 02: Login + FirstRunWizard on primitives Summary

**Migrated LoginPage.svelte and FirstRunWizard.svelte off hand-rolled `.form-input`/`.btn-submit`/`.checkbox-label`/`.btn-sso-reserved` markup onto Input/Button/Checkbox primitives wrapped in AuthShell/FormField, with zero change to auth-routing/validation/creation logic.**

## Performance

- **Duration:** 12 min
- **Started:** 2026-07-23T16:38:00Z (approx)
- **Completed:** 2026-07-23T16:50:00Z
- **Tasks:** 2
- **Files modified:** 3 (2 screens + 1 primitive fix)

## Accomplishments

- `LoginPage.svelte`: login/password fields now render through `FormField` + `Input` (snippet-param pattern forwarding `describedBy`/`invalid`), `Checkbox` primitive for "Запомнить меня", `Button` primitive for submit (spinner via `loading` prop) and the reserved-SSO button (`variant="ghost" disabled`, no `onclick`, no explicit `tabindex` — native `disabled` already removes it from tab order per D-UX-03). Entire screen wrapped in `<AuthShell>`. Auth-routing logic (`screen` state machine, `GENERIC_AUTH_ERROR`/`AD_UNREACHABLE_ERROR`, `REGISTRATION_PENDING`/`ACCESS_BLOCKED`/`SERVICE_UNAVAILABLE` branches in `handleSubmit`) is byte-identical to before, confirmed by diff (only 5 new import lines added above the untouched script body).
- `FirstRunWizard.svelte`: all 4 fields (login, full name, password, confirm password) now render through `FormField` + `Input`, submit button through `Button` (spinner via `loading` prop). Wrapped in `<AuthShell maxWidth={400}>`, preserving the wizard's wider 400px card exactly. `validate()`/`handleSubmit()` (the `users_create` + auto `auth_login` sequence) are byte-identical to before.
- Both files' `<style>` blocks reduced to only the locally-owned rules (title/subtitle, form gap, server-error banner) — all bespoke `.form-input`/`.form-label`/`.field-error`/`.format-hint`/`.checkbox-label`/`.checkbox-text`/`.btn-submit`/`.btn-sso-reserved`/`.login-container`/`.login-card`/`.wizard-container`/`.wizard-card` rules deleted.

## Task Commits

Each task was committed atomically:

1. **Task 1: Migrate LoginPage.svelte to primitives + AuthShell + FormField** - `46209db` (feat) — includes the Rule 3 `Input.svelte` autocomplete fix (same commit, blocking issue discovered during Task 1's own acceptance-criteria verification)
2. **Task 2: Migrate FirstRunWizard.svelte to primitives + AuthShell + FormField** - `1f5d0e1` (feat)

## Files Created/Modified

- `ui/src/features/auth/LoginPage.svelte` - `{:else}` branch (login form) migrated to `AuthShell`/`FormField`/`Input`/`Button`/`Checkbox`; script block and screen-routing branches (`pending`/`blocked`) untouched; `<style>` reduced to `.login-title`/`.login-form`/`.server-error`
- `ui/src/features/auth/FirstRunWizard.svelte` - Full markup migrated to `AuthShell maxWidth={400}`/`FormField`/`Input`/`Button`; `<script>` (`validate`/`handleSubmit`) untouched; `<style>` reduced to `.wizard-title`/`.wizard-subtitle`/`.wizard-form`/`.server-error`
- `ui/src/lib/components/Input.svelte` - Added `autocomplete?: HTMLInputAttributes['autocomplete']` prop, passed through to the native `<input autocomplete>` attribute (additive, no existing 19+ consumers affected — none previously passed `autocomplete` to this primitive)

## Decisions Made

- Followed the plan's exact FormField/Input/Button/Checkbox wiring for both screens, no deviation in visual output or copy.
- `HTMLInputAttributes['autocomplete']` (from `svelte/elements`) chosen over a plain `string` type for the new `autocomplete` prop — `string` failed svelte-check against the native `<input autocomplete>` attribute's stricter `FullAutoFill | null | undefined` type; the properly-typed alias fixed it without loosening type safety.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking issue] Added `autocomplete` prop to Input.svelte**
- **Found during:** Task 1 (LoginPage.svelte), while verifying the plan's required `autocomplete="username"`/`"current-password"` usage on `<Input>`
- **Issue:** `Input.svelte`'s `Props` interface (from 29-01) had no `autocomplete` field and no rest-prop spread. Passing `autocomplete="..."` on the `<Input>` component tag, as the plan's `<interfaces>`/task actions explicitly require, fails `svelte-check` (unknown prop). This would block the plan's own acceptance criterion (`pnpm --dir ui svelte-check` exits 0).
- **Fix:** Added `autocomplete?: HTMLInputAttributes['autocomplete']` to `Input.svelte`'s `Props` and passed it through to the native `<input autocomplete>` attribute. Verified additive/non-breaking: grepped all existing `Input.svelte` consumers app-wide — none previously passed `autocomplete`, so no existing call site is affected.
- **Files modified:** `ui/src/lib/components/Input.svelte`
- **Commit:** `46209db` (bundled into Task 1's commit since it was required to make Task 1's own acceptance criteria pass)

Otherwise: plan executed exactly as written.

## Issues Encountered

None beyond the Input.svelte prop gap above. All acceptance-criteria greps (form-input/btn-submit/btn-sso-reserved/checkbox-label counts, import counts, `type="password"` counts, `tabindex` count, `maxWidth={400}`) passed on first verification after each task. `pnpm --dir ui lint` (eslint + prettier + check-tokens), `pnpm --dir ui svelte-check` (0 errors, same 48 pre-existing warnings as 29-01's baseline), and `pnpm --dir ui build` all pass.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Both auth screens with real form fields (`LoginPage.svelte`, `FirstRunWizard.svelte`) are now fully on the design-system-v2 primitives with zero bespoke form-control residue. `PendingScreen.svelte`, `BlockedScreen.svelte`, and `EmployeeLayout.svelte` (29-03's scope) can now consume the same `AuthShell`/`FormField` contracts — `AuthShell`'s `stack` prop (unused by this plan, since neither LoginPage nor FirstRunWizard needed it) is confirmed ready for `BlockedScreen`'s flex-column layout.

---
*Phase: 29-login-and-employee-shell*
*Completed: 2026-07-23*

## Self-Check: PASSED

All modified files verified present on disk; both task commit hashes (46209db, 1f5d0e1) verified in `git log --oneline --all`.

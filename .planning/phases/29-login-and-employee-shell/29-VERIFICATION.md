---
phase: 29-login-and-employee-shell
verified: 2026-07-24T08:00:00Z
status: passed
score: 15/15 must-haves verified
overrides_applied: 0
---

# Phase 29: Вход и интерфейс сотрудника Verification Report

**Phase Goal:** Экраны входа и отдельная оболочка для роли «Сотрудник» показывают тот же визуальный язык, что и основное приложение, несмотря на отдельный layout-shell.
**Verified:** 2026-07-24T08:00:00Z
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

Merged from ROADMAP.md success criteria + all 4 plans' `must_haves.truths`.

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 (SC1) | LoginPage/PendingScreen/BlockedScreen/FirstRunWizard use new tokens/components, no undefined-token artifacts | ✓ VERIFIED | All 4 screens import `Input`/`Button`/`Checkbox`/`AuthShell`/`FormField`; `pnpm --dir ui lint` (includes `check-tokens.mjs`) exits 0 → "PASS — 0 нарушений" (re-run live) |
| 2 (SC2) | EmployeeLayout uses new tokens/components (sidebar mention in SC wording is documented non-requirement) | ✓ VERIFIED | `EmployeeLayout.svelte` uses `Button`, `ThemeSwitcher` primitives, `--header-height` token with zero fallback; D-03 in `29-CONTEXT.md` explicitly records the "сайдбар" SC wording as roadmap boilerplate, not a requirement — header-only design is intentional (backend-403 is the real access boundary, Phase 10) |
| 3 (SC3) | Visual language matches the redesigned main app despite separate shell | ✓ VERIFIED | Blocking human-verify UAT (29-04 Task 3, gate=blocking) conducted in both themes; round 1 surfaced 2 real CSS bugs (content not filling to viewport, user-name collapsing), both fixed in commits `61feaa7`/`e3bee15` and re-verified/approved in round 2 — commits inspected and confirm CSS-only fixes matching the reported symptoms exactly (definite-height/overflow:hidden/min-height:0 pattern mirrors admin `Layout.svelte`; RequestsPage's own `.page-content` padding + `height:100%` confirmed by direct file read) |
| 4 | Input.svelte's type union accepts 'password' additively, 19 existing consumers keep compiling (29-01) | ✓ VERIFIED | `Input.svelte:6` — `type?: 'text' \| 'number' \| 'search' \| 'password'`; `pnpm --dir ui build` succeeds; 22 total consumers now (19 pre-existing + LoginPage/FirstRunWizard/etc.), all compile |
| 5 | AuthShell.svelte exists as reusable center-card chrome (maxWidth/stack/children) (29-01) | ✓ VERIFIED | `AuthShell.svelte` — Props `{maxWidth=360, stack=false, children}`, `style:max-width`, `class:stack`, no `<h1>` rendered inside (per design) |
| 6 | FormField.svelte exists with label+control+error/hint + computed aria-describedby (29-01) | ✓ VERIFIED | `FormField.svelte` — computes `describedBy` via `$derived`, snippet-with-params `children({ describedBy, invalid })`, error uses `--tr-danger-text` |
| 7 | LoginPage renders zero bespoke form-control classes; all controls via Input/Button/Checkbox wrapped in AuthShell/FormField (29-02) | ✓ VERIFIED | grep for `form-input`/`btn-submit`/`btn-sso-reserved`/`checkbox-label`/`login-container`/`login-card` in `LoginPage.svelte` → 0 matches; imports `AuthShell`/`FormField`/`Input`/`Button`/`Checkbox` |
| 8 | FirstRunWizard renders zero bespoke form-control classes; all 4 fields wrapped in FormField (29-02) | ✓ VERIFIED | Same grep sweep → 0 matches in `FirstRunWizard.svelte`; 4 `<FormField>` instances (wiz-login/fullname/password/confirm); `<AuthShell maxWidth={400}>` |
| 9 | Password field masks input; reserved-SSO button has no onclick/tabindex (29-02) | ✓ VERIFIED | `type="password"` present (grep count 1 in LoginPage, 2 in FirstRunWizard); `tabindex` count 0 in LoginPage.svelte; reserved-SSO `<Button type="button" variant="ghost" disabled>` has no `onclick` prop (read directly) |
| 10 | Auth-routing logic / validate() / handleSubmit() byte-identical to before (29-02) | ✓ VERIFIED | Direct read of `LoginPage.svelte` script block confirms `GENERIC_AUTH_ERROR`/`AD_UNREACHABLE_ERROR`/`screen` state machine/`REGISTRATION_PENDING`/`ACCESS_BLOCKED`/`SERVICE_UNAVAILABLE` branches intact; `FirstRunWizard.svelte`'s `validate()`/`handleSubmit()` (`users_create`+auto `auth_login`) intact |
| 11 | PendingScreen renders back-to-login as Button(link) in AuthShell, zero bespoke classes (29-03) | ✓ VERIFIED | `PendingScreen.svelte` — `<AuthShell>` wraps `<Button variant="link" onclick={onBackToLogin}>`; grep for `btn-link`/`login-container`/`login-card` → 0 |
| 12 | BlockedScreen renders all 4 branches' CTAs as Button in AuthShell(stack), zero bespoke classes (29-03) | ✓ VERIFIED | `BlockedScreen.svelte` — `<AuthShell stack>` wraps all 4 conditional branches; `variant="link"` × 4, `variant="primary"` × 2 (rejection_reason + default); grep for bespoke classes → 0 |
| 13 | BlockedScreen's 4-branch logic and handleRestoreRequest byte-identical (29-03) | ✓ VERIFIED | Script block (lines 1-58: Props, state, `handleRestoreRequest` calling `request_ad_restore`) confirmed unchanged by direct read; conditional structure (`submitted`/`pending`/`rejection_reason`/else) intact |
| 14 | EmployeeLayout header-height/content min-height resolve from --header-height, no hardcoded 56px fallback anywhere (29-04) | ✓ VERIFIED | grep for `56px` in `EmployeeLayout.svelte` → 0 matches; `.employee-header { height: var(--header-height); }` (no fallback); content sizing later redesigned to `flex:1; min-height:0` during UAT gap-closure (still no 56px, no fallback — legitimate flex-based redesign, confirmed via git diff of `61feaa7`) |
| 15 | EmployeeLayout stays header-only (no sidebar added), and all 4 auth screens + EmployeeLayout pass lint/svelte-check/build/check-tokens + human UAT (29-04) | ✓ VERIFIED | No `Sidebar` import/nav-toggle in `EmployeeLayout.svelte` (confirmed by direct read); live re-run of `pnpm --dir ui lint` (0 violations), `pnpm --dir ui svelte-check` (0 errors, 48 pre-existing unrelated warnings), `pnpm --dir ui build` (succeeds, `ui/dist` regenerated, no `bindings.ts` drift per clean `git status`) |

**Score:** 15/15 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `ui/src/lib/components/Input.svelte` | type union extended to include 'password' | ✓ VERIFIED | Line 6, additive-only edit; `autocomplete` prop also added (29-02 auto-fix, additive, non-breaking) |
| `ui/src/lib/components/AuthShell.svelte` | auth center-card shell primitive | ✓ VERIFIED | 50 lines, Props `maxWidth`/`stack`/`children`, scoped SCSS |
| `ui/src/lib/components/FormField.svelte` | label/error/hint field wrapper with aria wiring | ✓ VERIFIED | 57 lines, snippet-param `children({describedBy, invalid})`, `$derived` describedBy |
| `ui/src/features/auth/LoginPage.svelte` | Login screen on primitives + AuthShell + FormField | ✓ VERIFIED | Wired, zero bespoke classes, logic untouched |
| `ui/src/features/auth/FirstRunWizard.svelte` | Wizard on primitives + AuthShell + FormField | ✓ VERIFIED | Wired, zero bespoke classes, `maxWidth={400}` preserved |
| `ui/src/features/auth/PendingScreen.svelte` | Pending screen on AuthShell + Button | ✓ VERIFIED | Wired, `Props{onBackToLogin}` unchanged |
| `ui/src/features/auth/BlockedScreen.svelte` | Blocked screen (4 branches) on AuthShell + Button | ✓ VERIFIED | Wired, `<AuthShell stack>`, restoration logic unchanged |
| `ui/src/features/layout/EmployeeLayout.svelte` | header-only employee shell, token-clean, no fallback | ✓ VERIFIED | Token-clean; WS/logout logic byte-identical to pre-phase (confirmed via commit diffs — only CSS/markup edited across all 3 phase-29 commits touching this file) |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| FormField.svelte | Snippet children param | `{@render children({ describedBy, invalid })}` | ✓ WIRED | Present, `describedBy`/`invalid` forwarded to `<Input>` in both LoginPage and FirstRunWizard call sites |
| LoginPage.svelte | AuthShell.svelte | import + `<AuthShell>` wrapper | ✓ WIRED | `import AuthShell from '$lib/components/AuthShell.svelte'` present, used to wrap login form |
| LoginPage.svelte | Input.svelte | `type="password"` | ✓ WIRED | Password `<Input>` uses `type="password"` |
| FirstRunWizard.svelte | FormField.svelte | 4x `<FormField>` wrapping | ✓ WIRED | All 4 fields wrapped |
| PendingScreen.svelte | AuthShell.svelte | import + wrapper | ✓ WIRED | Confirmed |
| BlockedScreen.svelte | AuthShell.svelte (stack) | `<AuthShell stack>` | ✓ WIRED | Confirmed, reproduces prior flex-column+gap card |
| EmployeeLayout.svelte | `_tokens.scss --header-height` | `var(--header-height)` no fallback | ✓ WIRED | `--header-height: 56px` confirmed defined at `_tokens.scss:190`; consumer has zero `, 56px` fallback anywhere |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| svelte-check across whole app (touched files included) | `pnpm --dir ui svelte-check` | `COMPLETED 264 FILES 0 ERRORS 48 WARNINGS` (all warnings pre-existing, unrelated files) | ✓ PASS |
| lint (eslint + prettier + check-tokens) | `pnpm --dir ui lint` | `[check-tokens] PASS — 0 нарушений`, prettier clean | ✓ PASS |
| Production build | `pnpm --dir ui build` | Succeeded, `ui/dist/` regenerated, no errors, no warnings from phase-touched files | ✓ PASS |
| No bindings.ts drift (frontend-only phase) | `git status --short` | Clean working tree | ✓ PASS |
| Commit hashes referenced in SUMMARYs exist | `git log --oneline --all` | All 10 referenced hashes found | ✓ PASS |
| No bespoke residual classes in migrated files | grep sweep for `form-input`/`btn-submit`/`btn-sso-reserved`/`checkbox-label`/`btn-link`/`login-container`/`login-card`/`56px` | 0 matches across all 5 files | ✓ PASS |
| No debt markers (TBD/FIXME/XXX/TODO/HACK/PLACEHOLDER) in phase-touched files | grep sweep | 0 matches | ✓ PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| WIN-10 | 29-01, 29-02, 29-03 | Экраны входа — Логин / Pending / Blocked / FirstRunWizard | ✓ SATISFIED | All 4 auth screens migrated to primitives; REQUIREMENTS.md marks WIN-10 `[x]` and "Phase 29 / Complete" |
| WIN-11 | 29-04 | Интерфейс сотрудника (EmployeeLayout) | ✓ SATISFIED | EmployeeLayout token-clean, header-only (D-03), UAT-approved; REQUIREMENTS.md marks WIN-11 `[x]` and "Phase 29 / Complete" |

No orphaned requirements — REQUIREMENTS.md's Phase 29 mapping (WIN-10, WIN-11) exactly matches the union of `requirements:` fields declared across all 4 plans.

### Anti-Patterns Found

None. Grep sweep for debt markers, empty implementations (`return null`/`return {}`/`return []`/`=> {}`), and `console.log` across all phase-touched files returned zero matches.

### Code Review Cross-Reference (29-REVIEW.md)

0 critical, 2 warnings, 3 info — reviewed and confirmed not phase-introduced regressions:
- **WR-01** (WS refcount leak on fast unmount) — pre-existing `onMount`/`connectWs` logic; confirmed via commit diff that Phase 29 never touched this code (all 3 commits touching `EmployeeLayout.svelte` are CSS-only). Not a phase-29 defect; does not block phase goal (visual-language parity).
- **WR-02** (BlockedScreen empty-string `rejection_reason` misclassification) — pre-existing backend/frontend contract looseness (`crates/trackly-app/src/services/auth.rs:676-679`), not introduced by the markup migration. Does not block phase goal.
- IN-01/IN-02/IN-03 — cosmetic/low-priority, none block the phase goal.

These are legitimate out-of-scope findings; phase 29 was explicitly a markup/CSS migration with byte-identical logic preserved (verified above), so pre-existing logic bugs are correctly out of this phase's remit.

### Human Verification Required

None. The phase's own blocking `checkpoint:human-verify` (29-04 Task 3) already executed the required visual-parity UAT across all 5 surfaces in both themes, with 2 real bugs found and fixed (commits `61feaa7`, `e3bee15`) and a second round approved. Verifier independently confirmed: (a) the fix commits are CSS-only and match the reported symptoms exactly, (b) the "definite-height + overflow:hidden + min-height:0" pattern genuinely mirrors admin `Layout.svelte`, and (c) `RequestsPage.svelte` genuinely supplies its own `.page-content` padding + `height:100%`, corroborating the technical rationale given for the fix rather than merely trusting the SUMMARY narrative.

### Gaps Summary

No gaps. All 15 merged truths (3 roadmap SCs + 12 plan-level must-haves) verified against the actual codebase — not just SUMMARY claims. All artifacts exist, are substantive (no stubs), and are wired correctly. Build/lint/svelte-check gates independently re-run and confirmed green. Commit history confirms claimed byte-identity of business logic (auth-routing, validation, WS/logout) across all touched files. The one SC-wording/implementation mismatch (SC #2's "сайдбар" mention) was pre-emptively documented as intentional in `29-CONTEXT.md` (D-03) and is correctly not treated as a failure per that recorded decision.

---

*Verified: 2026-07-24T08:00:00Z*
*Verifier: Claude (gsd-verifier)*

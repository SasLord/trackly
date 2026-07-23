---
phase: 29-login-and-employee-shell
plan: 01
subsystem: ui
tags: [svelte, svelte5-runes, design-system, auth, aria]

# Dependency graph
requires:
  - phase: 24-base-components
    provides: Input.svelte primitive (design-system-v2 tokens) and Button/PageHeader/DetailPanel extraction precedent
provides:
  - "Input.svelte: type union additively extended to accept 'password'"
  - "AuthShell.svelte: reusable auth center-card chrome (maxWidth/stack/children props)"
  - "FormField.svelte: reusable label/control/error/hint wrapper with computed aria-describedby"
affects: [29-02-login-firstrun, 29-03-pending-blocked-employee-shell]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Snippet-with-parameters contract: FormField's children Snippet<[{ describedBy, invalid }]> hands each call site a ready-made aria pair to forward straight into <Input>"
    - "Chrome extraction via const-destructured Props + scoped SCSS (PageHeader/DetailPanel precedent) applied to auth screens"

key-files:
  created:
    - ui/src/lib/components/AuthShell.svelte
    - ui/src/lib/components/FormField.svelte
  modified:
    - ui/src/lib/components/Input.svelte

key-decisions:
  - "Input.svelte type union extended additively only with 'password' (no 'email' — no in-scope consumer per D-01 discretion note)"
  - "AuthShell does not render a title element — each screen keeps its own heading/paragraph as children since title-to-content spacing differs per screen (LoginPage margin vs BlockedScreen flex+gap)"
  - "Dropped the dead '--tr-space-4xl, 2rem' fallback when porting chrome into AuthShell — token is confirmed defined in _tokens.scss"
  - "FormField's .field-error color uses --tr-danger-text (Fields.dc.html source), not the pre-existing LoginPage.svelte --tr-danger convention — deliberate deviation since this is a net-new component with no consumer to regress yet"

patterns-established:
  - "AuthShell/FormField pair is the exact contract Wave 2 plans (29-02, 29-03) must consume unmodified — no downstream executor needs to invent component shape"

requirements-completed: [WIN-10]

# Metrics
duration: 6min
completed: 2026-07-23
---

# Phase 29 Plan 01: Auth primitives (Input password + AuthShell + FormField) Summary

**Extended Input.svelte to accept type="password" and extracted AuthShell/FormField as the shared chrome + label/error/hint primitives that Wave 2's four auth screens will consume.**

## Performance

- **Duration:** 6 min
- **Started:** 2026-07-23T16:29:00Z (approx)
- **Completed:** 2026-07-23T16:35:26Z
- **Tasks:** 3
- **Files modified:** 3 (1 modified, 2 created)

## Accomplishments
- `Input.svelte`'s `type` union now accepts `'password'` with a single-line, additive edit — zero regression risk to its 19 existing consumers (confirmed via full `pnpm --dir ui build`)
- `AuthShell.svelte` extracted: owns the `.auth-shell`/`.auth-card` chrome duplicated byte-near-identically 4x across `LoginPage.svelte`, `FirstRunWizard.svelte`, `PendingScreen.svelte`, `BlockedScreen.svelte`, with `maxWidth`/`stack` props covering both the 360px/400px width variance and BlockedScreen's flex-column modifier
- `FormField.svelte` extracted: label/control/error/hint wrapper that computes `aria-describedby` from the field's own `id` and hands it to the caller via a snippet-with-parameters contract — closes a real accessibility gap where error/hint text was previously only visually adjacent to the input, never wired via ARIA

## Task Commits

Each task was committed atomically:

1. **Task 1: Extend Input.svelte type union to include 'password'** - `f5e88c6` (feat)
2. **Task 2: Create AuthShell.svelte — extracted center-card chrome** - `ffa1b7d` (feat)
3. **Task 3: Create FormField.svelte — label/control/error/hint wrapper with aria wiring** - `66e4869` (feat)

**Plan metadata:** (pending — see final commit below)

## Files Created/Modified
- `ui/src/lib/components/Input.svelte` - `type` union extended to include `'password'` (single-line change, no other line touched)
- `ui/src/lib/components/AuthShell.svelte` - New: auth center-card chrome (`.auth-shell`/`.auth-card`), Props `{ maxWidth = 360, stack = false, children }`
- `ui/src/lib/components/FormField.svelte` - New: label/control/error/hint wrapper, Props `{ label, id, error, hint, children }`, computes `describedBy` via `$derived`

## Decisions Made
- Followed plan's exact prop shapes and SCSS values (ported verbatim from the 4 duplicated auth screens and `LoginPage.svelte`'s `.form-field` block) — no deviation in visual output
- `--tr-danger-text` chosen over `--tr-danger` for `FormField`'s error color per the plan's explicit instruction (Fields.dc.html source of truth, net-new component)

## Deviations from Plan

None - plan executed exactly as written. All acceptance criteria (grep checks, `pnpm --dir ui build`) passed on first attempt for all three tasks.

## Issues Encountered

None. Also ran `pnpm --dir ui lint` (0 violations, prettier clean, check-tokens PASS) and `pnpm --dir ui svelte-check` (0 errors, 48 pre-existing warnings in unrelated files) beyond the plan's per-task acceptance criteria, per the plan's `<verification>` section.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

`AuthShell.svelte` and `FormField.svelte` exist as standalone, buildable, uninstantiated components exactly matching the contracts Wave 2 plans (29-02: LoginPage + FirstRunWizard; 29-03: PendingScreen + BlockedScreen + EmployeeLayout) will consume. No auth screen file was modified in this plan — that is Wave 2's job. `Input.svelte` is ready to accept `type="password"` for the password fields Wave 2 will wire through `FormField`.

---
*Phase: 29-login-and-employee-shell*
*Completed: 2026-07-23*

## Self-Check: PASSED

All created/modified files verified present on disk; all 3 task commit hashes (f5e88c6, ffa1b7d, 66e4869) verified in `git log --oneline --all`.

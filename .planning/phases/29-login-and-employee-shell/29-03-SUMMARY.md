---
phase: 29-login-and-employee-shell
plan: 03
subsystem: ui
tags: [svelte, svelte5-runes, design-system, auth]

# Dependency graph
requires:
  - phase: 29-login-and-employee-shell
    provides: "29-01: AuthShell.svelte (maxWidth/stack/children) and Button.svelte (variant/loading) primitives"
provides:
  - "PendingScreen.svelte and BlockedScreen.svelte migrated to AuthShell + Button, zero bespoke chrome/button CSS"
affects: [29-employee-shell-plan-04]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Status-only auth screen pattern: AuthShell wraps conditional branches directly (no extra title/content split needed) when the screen has no form fields, only headings/paragraphs/CTAs"
    - "Button's loading prop folds manual {#if submitting}text-swap{/if} pattern into a single prop, same as Plan 29-02"

key-files:
  created: []
  modified:
    - ui/src/features/auth/PendingScreen.svelte
    - ui/src/features/auth/BlockedScreen.svelte

key-decisions:
  - "PendingScreen needed a local .pending-card { text-align:center; } wrapper div inside AuthShell's children, since AuthShell's non-stack default has no text-align:center (only .stack modifier does) — per plan's explicit discretion note, this preserves the screen's original centered-text layout without changing AuthShell's default behavior"
  - "BlockedScreen's stack=true prop reproduces its exact prior flex-column+gap+center layout with no local wrapper needed — the 4-branch conditional renders its children directly inside <AuthShell stack>"

patterns-established: []

requirements-completed: [WIN-10]

# Metrics
duration: 8min
completed: 2026-07-23
---

# Phase 29 Plan 03: Pending + Blocked screens migration to AuthShell + Button Summary

**Migrated PendingScreen.svelte and BlockedScreen.svelte (4-branch restoration flow) off hand-rolled chrome/button CSS onto the AuthShell/Button primitives from 29-01, with zero changes to BlockedScreen's conditional logic or `request_ad_restore` call.**

## Performance

- **Duration:** 8 min
- **Started:** 2026-07-23T16:47:56Z (approx, per STATE.md last_updated)
- **Completed:** 2026-07-23T16:50:15Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- `PendingScreen.svelte`: swapped `.login-container`/`.login-card`/`.btn-link` for `<AuthShell>` + `<Button variant="link">`, with a minimal local `.pending-card { text-align: center; }` wrapper preserving the screen's original centered layout (AuthShell's non-stack default has no built-in text-align)
- `BlockedScreen.svelte`: swapped chrome for `<AuthShell stack>` (reproducing the exact flex-column+gap+center modifier all 4 branches depend on) and all `.btn-link`/`.btn-submit` buttons across the 4 conditional branches for `<Button variant="link">`/`<Button variant="primary" loading={submitting}>`, folding the manual `{#if submitting}Отправка…{:else}...{/if}` text-swap into the `loading` prop
- Restoration-flow logic untouched: `handleRestoreRequest`, `submitted`/`submitting`/`serverError` state, and the `{#if submitted}...{:else if blockedDetails.pending}...{:else if blockedDetails.rejection_reason}...{:else}` branch structure are byte-identical to before (script block lines 1-58 not touched by the edit)

## Task Commits

Each task was committed atomically:

1. **Task 1: Migrate PendingScreen.svelte to AuthShell + Button** - `46ce235` (feat)
2. **Task 2: Migrate BlockedScreen.svelte to AuthShell (stack) + Button (4 branches)** - `da4aadd` (feat)

**Plan metadata:** (pending — see final commit below)

## Files Created/Modified
- `ui/src/features/auth/PendingScreen.svelte` - Chrome/CTA now render through AuthShell + Button; `Props { onBackToLogin }` unchanged
- `ui/src/features/auth/BlockedScreen.svelte` - Chrome/CTAs across all 4 branches now render through `<AuthShell stack>` + Button; script (Props, state, `handleRestoreRequest`) unchanged

## Decisions Made
- Followed the plan's exact prop usage (`AuthShell` default vs `stack`) and Button variant mapping (`link` for nav-back CTA, `primary`+`loading` for the restore-request CTA) — no deviation from plan's `<interfaces>` contract
- Added the plan-anticipated `.pending-card` local text-align fallback for PendingScreen since AuthShell's default (non-stack) layout doesn't apply `text-align: center` on its own

## Deviations from Plan

None - plan executed exactly as written. All acceptance criteria (grep checks, `pnpm --dir ui build`, `pnpm --dir ui svelte-check`, `pnpm --dir ui lint`) passed. Prettier auto-reformatted `BlockedScreen.svelte`'s wrapping (`npx prettier --write`) after the initial write to satisfy `pnpm --dir ui lint`'s `prettier --check` gate — a pure formatting normalization, no semantic change, verified via the same acceptance-criteria greps re-run after formatting (all counts unchanged).

## Issues Encountered

None beyond the expected prettier auto-format pass noted above.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

`PendingScreen.svelte` and `BlockedScreen.svelte` are now on the shared `AuthShell`/`Button` primitives, matching `LoginPage.svelte`/`FirstRunWizard.svelte` from 29-02. All 4 of Phase 29's auth screens are migrated. Remaining Wave 2 work per PROJECT.md's phase list: `EmployeeLayout.svelte` (WIN-11) in a subsequent plan.

---
*Phase: 29-login-and-employee-shell*
*Completed: 2026-07-23*

## Self-Check: PASSED

Both modified files verified present on disk; both task commit hashes (46ce235, da4aadd) verified in `git log --oneline --all`.

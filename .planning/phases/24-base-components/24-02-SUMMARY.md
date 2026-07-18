---
phase: 24-base-components
plan: 02
subsystem: ui
tags: [svelte5, scss, design-tokens, button]

# Dependency graph
requires:
  - phase: 24-base-components
    provides: "Plan 01's --tr-accent-text token + .theme-switching transition-suppression hook (unblocks restoring .12s micro-transitions here without a theme-toggle color-smear)"
provides:
  - "Button.svelte corrected to exact Buttons.dc.html visual conformance: 5 variants x 2 sizes x 6 states (default/hover/focus/active/disabled/loading)"
  - "ButtonsSection.svelte showcase gallery (self-contained, static demo content) ready for Plan 07 to wire into the showcase route"
  - "--tr-danger-hover / --tr-danger-active design tokens (previously missing from _tokens.scss despite RESEARCH.md marking them VERIFIED-present)"
affects: [24-07]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Showcase sections are pure static markup (no $props/$state) living under ui/src/features/showcase/sections/, imported later by the showcase page assembly plan"

key-files:
  created:
    - ui/src/features/showcase/sections/ButtonsSection.svelte
  modified:
    - ui/src/lib/components/Button.svelte
    - ui/src/styles/_tokens.scss

key-decisions:
  - "Added missing --tr-danger-hover/--tr-danger-active tokens (Rule 3 blocking-issue fix) — RESEARCH.md's Token Mismatches table asserted these were already VERIFIED present in _tokens.scss, but they did not exist; values transcribed verbatim from Buttons.dc.html (light #b83232/#9d2929, dark #ff7d7d/#e05555)"
  - "ButtonsSection.svelte written as fully explicit static markup (5 variant blocks x 2 size groups x 3 literal Button instances) rather than #each-looped, matching the plan's literal-string acceptance greps (variant=\"primary\" etc.) and keeping the file self-documenting for manual verification"

patterns-established: []

requirements-completed: [CMP-01]

# Metrics
duration: 12min
completed: 2026-07-18
---

# Phase 24 Plan 02: Button Component Correction + Showcase Section Summary

**Button.svelte brought into exact `Buttons.dc.html` conformance (12s transitions, 0.45 disabled opacity, corrected secondary/ghost/link colors, 5 new `:active` states) and a 30-instance ButtonsSection showcase gallery created for later routing.**

## Performance

- **Duration:** 12 min
- **Started:** 2026-07-18T06:12:55Z
- **Completed:** 2026-07-18T06:24:53Z
- **Tasks:** 2 completed
- **Files modified:** 3

## Accomplishments
- `Button.svelte` base rule: `border: none` → `border: 1px solid transparent`; `transition: none` → `transition: background .12s, box-shadow .12s`; disabled opacity `0.5` → `0.45`; loading state now dims uniformly (`opacity: .85`) at the base-rule level instead of per-variant
- Added `&:active:not(:disabled)` pressed-state rules to all 5 variants (primary/secondary/destructive/ghost/link) — none existed before
- `secondary`: background `transparent` → `var(--tr-surface)`; `ghost`: hover background `--tr-surface` → `--tr-surface-sunken` (was pointing at the wrong token)
- `link`: reworked from hover-only underline (inverted) to default-underlined / focus-removes-underline, matching the reference spec
- `ButtonsSection.svelte` created: 30 static `<Button>` instances (5 variants × 2 sizes × 3 states: normal/disabled/loading), each with a self-documenting Russian label
- Fixed a plan/RESEARCH.md drift: `--tr-danger-hover` and `--tr-danger-active` tokens were claimed "VERIFIED present" but did not exist in `_tokens.scss` — added both (light/dark), transcribed verbatim from `Buttons.dc.html`

## Task Commits

Each task was committed atomically:

1. **Task 1: Transcribe Buttons.dc values into Button.svelte (CMP-01)** - `3eb8a93` (feat)
2. **Task 2: Create ButtonsSection.svelte showcase section** - `c841bf2` (feat)

**Plan metadata:** committed separately after this summary.

## Files Created/Modified
- `ui/src/lib/components/Button.svelte` - Base rule + all 5 variants corrected to Buttons.dc.html values
- `ui/src/styles/_tokens.scss` - Added `--tr-danger-hover`/`--tr-danger-active` to both `[data-theme='light']` and `[data-theme='dark']` blocks
- `ui/src/features/showcase/sections/ButtonsSection.svelte` - New static showcase gallery, not yet routed

## Decisions Made
- `--tr-danger-hover`/`--tr-danger-active` values sourced from `Buttons.dc.html`'s own `:root` blocks (light `#b83232`/`#9d2929`, dark `#ff7d7d`/`#e05555`) since no canonical value existed anywhere else in the codebase to defer to
- `ButtonsSection.svelte` uses fully explicit (non-looped) markup per variant/size/state combination — the plan's acceptance criteria grep for literal `variant="primary"` etc., and explicit markup keeps each showcase cell independently readable during manual verification

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added missing `--tr-danger-hover`/`--tr-danger-active` design tokens**
- **Found during:** Task 1 (Transcribe Buttons.dc values into Button.svelte)
- **Issue:** Task 1's `read_first` instructed confirming these tokens exist in `_tokens.scss`, and 24-RESEARCH.md's Token Mismatches table asserted "`--tr-accent-hover`, `--tr-accent-active`, `--tr-danger-hover`, `--tr-danger-active` are ALL already present and correct in `_tokens.scss` [VERIFIED]" — this was false for the danger pair. `node ui/scripts/check-tokens.mjs` failed with `undefined token reference --tr-danger-hover`/`--tr-danger-active` (Rule 3, closed-world token gate) once the destructive variant's hover/active rules referenced them.
- **Fix:** Added both tokens to both theme blocks in `_tokens.scss`, values transcribed verbatim from `Buttons.dc.html`'s embedded `:root` declarations (light `#b83232`/`#9d2929`, dark `#ff7d7d`/`#e05555`).
- **Files modified:** `ui/src/styles/_tokens.scss`
- **Verification:** `pnpm --dir ui lint` (check-tokens rule 3) passes; `pnpm --dir ui svelte-check` 0 errors
- **Committed in:** `3eb8a93` (part of Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Necessary for correctness — without the tokens, `destructive` variant's hover/active states would resolve to `unset`/inherit instead of the intended darker red. No scope creep; fix stayed within the token layer the task already touched conceptually.

## Issues Encountered
- Prettier reformatted the multi-value `transition` declaration onto multiple lines and normalized `.12s` → `0.12s` after the initial edit (`pnpm --dir ui lint` ran prettier --check as part of its pipeline). This is cosmetic only — functionally identical, and `pnpm exec prettier --write` was run to match the pre-existing project formatting convention before final verification.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- `Button.svelte` now matches `Buttons.dc.html` exactly across all 5 variants × 2 sizes × 6 states; safe to reuse everywhere Button already appears (including Modal's footer) with zero call-site changes
- `ButtonsSection.svelte` exists and compiles standalone, ready for Plan 07 to import into the showcase page assembly
- `--tr-danger-hover`/`--tr-danger-active` are now available to any future component needing danger-variant hover/active colors (e.g. destructive Badge states in a later plan, if needed)
- No blockers for Wave 1 remaining plans (24-03 through 24-06) or Plan 07

---
*Phase: 24-base-components*
*Completed: 2026-07-18*

---
phase: 24-base-components
plan: 01
subsystem: ui
tags: [scss, design-tokens, svelte5-runes, theming]

# Dependency graph
requires:
  - phase: 23-design-tokens-foundations
    provides: "--tr-* token layer (colors, spacing, radius, typography, elevation) and the theme.svelte.ts / global.scss files this plan extends"
provides:
  - "--tr-accent-text design token (light #2350bd / dark #8fb0ff) available to any component needing accent-tinted text"
  - "Theme-switch transition suppression: .theme-switching class toggled around the dataset.theme mutation, matched by a :global(.theme-switching) transition:none rule"
  - "Corrected CMP-03/Success Criteria #3 requirement text (5 тонов, not 4)"
affects: [24-02, 24-03, 24-04, 24-05, 24-06, 24-07]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Theme toggle wraps the dataset mutation in add/rAF-remove of a CSS class so transition-suppression is structural, not timing-guessed"

key-files:
  created: []
  modified:
    - ui/src/styles/_tokens.scss
    - .planning/REQUIREMENTS.md
    - .planning/ROADMAP.md
    - ui/src/lib/stores/theme.svelte.ts
    - ui/src/styles/global.scss

key-decisions:
  - "--tr-accent-text values transcribed verbatim from Badges.dc.html/Tabs.dc.html RESEARCH.md table, not recomputed"
  - "requestAnimationFrame (not setTimeout) used to remove .theme-switching, guaranteeing removal only after the browser paints the new theme's colors"
  - ".theme-switching rule added after (not replacing) the existing prefers-reduced-motion block — both coexist, serving different triggers"

patterns-established:
  - "Theme-switch transition suppression: applyResolved() in theme.svelte.ts is the single hook point for any future per-toggle DOM-class behavior"

requirements-completed: [CMP-03, CMP-04]

# Metrics
duration: 8min
completed: 2026-07-18
---

# Phase 24 Plan 01: Shared Infrastructure (Token + Theme-Switch Suppression) Summary

**Added the missing `--tr-accent-text` design token and a one-frame `.theme-switching` transition-suppression hook, unblocking Wave 2's Badge/Tabs plans and every later Phase 24 plan restoring `.12s` micro-transitions.**

## Performance

- **Duration:** 8 min
- **Started:** 2026-07-18T06:02:00Z
- **Completed:** 2026-07-18T06:10:11Z
- **Tasks:** 2 completed
- **Files modified:** 5

## Accomplishments
- `--tr-accent-text` token pair added to `_tokens.scss` (light `#2350bd` / dark `#8fb0ff`), passing the closed-world `check-tokens.mjs` gate
- REQUIREMENTS.md CMP-03 and ROADMAP.md Phase 24 Success Criteria #3 corrected from "4 тона" to "5 тонов"/"5 тонах" (D-06), with zero stray occurrences remaining
- `theme.svelte.ts`'s `applyResolved()` now brackets the `dataset.theme` mutation with `classList.add('theme-switching')` / `requestAnimationFrame(() => classList.remove(...))`
- `global.scss` gained a `:global(.theme-switching)` rule forcing `transition: none !important` during that window, coexisting with (not replacing) the pre-existing `prefers-reduced-motion` block

## Task Commits

Each task was committed atomically:

1. **Task 1: Add --tr-accent-text token + correct CMP-03 tone count (D-06)** - `3926dc5` (feat)
2. **Task 2: Theme-switch transition suppression hook (D-09)** - `560cbd0` (feat)

**Plan metadata:** committed separately after this summary.

## Files Created/Modified
- `ui/src/styles/_tokens.scss` - Added `--tr-accent-text` to both `[data-theme='light']` and `[data-theme='dark']` blocks
- `.planning/REQUIREMENTS.md` - CMP-03 tone count corrected 4→5
- `.planning/ROADMAP.md` - Phase 24 Success Criteria #3 tone count corrected 4→5
- `ui/src/lib/stores/theme.svelte.ts` - `applyResolved()` wraps dataset mutation with theme-switching class add/rAF-remove
- `ui/src/styles/global.scss` - New `:global(.theme-switching)` transition-kill rule after the reduced-motion block

## Decisions Made
- Token values transcribed verbatim from RESEARCH.md's Token Mismatches table (both `Badges.dc.html` and `Tabs.dc.html` agree) — not recalculated
- `requestAnimationFrame` chosen over `setTimeout` per RESEARCH.md Pattern 4, guaranteeing the class is removed only after the browser has painted the new theme colors, not on an arbitrary timer
- New `.theme-switching` rule placed immediately after the existing `prefers-reduced-motion` block (before "Skip link"), per plan's exact placement instruction — both rules coexist independently

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- `--tr-accent-text` is available for Wave 2 plans (Badge, Tabs) to consume without further token work
- Theme-switch suppression hook is live; later plans restoring `.12s` micro-transitions to Button/Fields/Badge/Tabs can do so without a visible color-smear on theme toggle
- No blockers for 24-02 through 24-07

---
*Phase: 24-base-components*
*Completed: 2026-07-18*

## Self-Check: PASSED

All 6 files verified present on disk; all 3 commits (3926dc5, 560cbd0, 669c1ae) verified in git log.

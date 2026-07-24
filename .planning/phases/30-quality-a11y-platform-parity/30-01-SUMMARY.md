---
phase: 30-quality-a11y-platform-parity
plan: 01
subsystem: ui
tags: [a11y, wcag, css-tokens, lint-gate, svelte, scss]

# Dependency graph
requires:
  - phase: 23-design-tokens-foundations
    provides: "--tr-* token layer (_tokens.scss), check-tokens.mjs skeleton/pattern"
provides:
  - "check-contrast.mjs — durable zero-dependency WCAG AA contrast gate (43 token pairs x 2 themes)"
  - "check-focus-outline.mjs — durable zero-dependency bare-outline:none lint with same-block/cross-nested-block detection"
  - "_tokens.scss with 4 corrected color values passing AA in both themes"
  - "pnpm lint now runs check-tokens.mjs -> check-contrast.mjs -> check-focus-outline.mjs"
affects: [30-02, 30-03]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Zero-dependency lint-gate scripts (node:fs/node:path/node:url only) following check-tokens.mjs skeleton: readFileSafe/collectSourceFiles/lineNumberAt/STYLE_BLOCK_RE reused verbatim"
    - "WCAG 2.x relative luminance + contrast ratio implemented from scratch (no npm dependency)"
    - "Brace-depth scoping to find the immediately-enclosing CSS rule, correctly distinguishing same-block and cross-nested-block paired outline/box-shadow patterns"

key-files:
  created:
    - ui/scripts/check-contrast.mjs
    - ui/scripts/check-focus-outline.mjs
  modified:
    - ui/package.json
    - ui/src/styles/_tokens.scss

key-decisions:
  - "Canonical 43-pair table hardcoded in check-contrast.mjs (no CLI params) — closed-world by design, matches check-tokens.mjs Rule 3 philosophy"
  - "rgba()-based tokens (soft/focus-ring/danger-ring/overlay/row-selected) intentionally excluded from the contrast table — alpha compositing over actual background is out of this script's scope, residual risk closed by manual UAT in plan 30-03"
  - "--tr-text-disabled excluded per WCAG 1.4.3 (inactive UI elements exemption)"
  - "check-focus-outline.mjs uses brace-depth stack (not selector-name regex) to find the enclosing rule — single algorithm correctly handles both ActListRow's same-block &:focus-visible pattern and Tabs.svelte's cross-nested .tab{...}/&:focus-visible{...} pattern"

requirements-completed: [QA-02]

# Metrics
duration: ~20min
completed: 2026-07-24
---

# Phase 30 Plan 01: WCAG Contrast + Focus-Outline Lint Gates Summary

**Two new zero-dependency lint scripts (check-contrast.mjs, check-focus-outline.mjs) wired into `pnpm lint`, plus 4 corrected `_tokens.scss` color values that now pass WCAG AA in both light and dark themes.**

## Performance

- **Duration:** ~20 min
- **Started:** 2026-07-24T16:50:00Z (approx.)
- **Completed:** 2026-07-24T17:10:19Z
- **Tasks:** 3/3
- **Files modified:** 4 (2 created, 2 modified)

## Accomplishments

- `check-contrast.mjs`: computes WCAG 2.x relative luminance + contrast ratio from scratch, reads `_tokens.scss`, checks a closed table of 43 canonical foreground/background token pairs × 2 themes (86 checks/run); exits 0/1 per the same contract as `check-tokens.mjs`.
- `check-focus-outline.mjs`: scans every `.svelte` file's `<style>` block for bare `outline: none;`, uses brace-depth scoping to find the enclosing CSS rule, and flags cases with no paired `box-shadow` anywhere in that rule (including nested sub-rules). Correctly distinguishes the real defect (`Dropdown.svelte:927`) from both the same-block pattern (`ActListRow.svelte`) and the cross-nested-block pattern (`Tabs.svelte` — `.tab { outline:none; &:focus-visible { box-shadow: ...; } }`).
- 4 targeted `_tokens.scss` value fixes bring `--tr-text-tertiary` (both themes), `--tr-warning` (light), and `--tr-success` (light) to AA compliance; `check-contrast.mjs` now reports 0 violations in both themes.
- Both scripts wired into `ui/package.json` `scripts.lint` in an `&&`-chain after `check-tokens.mjs`.

## Task Commits

1. **Task 1: check-contrast.mjs — WCAG AA-контраст гейт** - `ae87c76` (feat)
2. **Task 2: Починка 4 AA-провалов центральными токенами** - `e3791a8` (fix)
3. **Task 3: check-focus-outline.mjs — линт голого outline:none** - `c65af8f` (feat)

_No TDD tasks in this plan (tdd="false" on all three)._

## Files Created/Modified

- `ui/scripts/check-contrast.mjs` - New zero-dependency WCAG AA contrast gate, 43-pair canonical table × 2 themes
- `ui/scripts/check-focus-outline.mjs` - New zero-dependency bare-`outline:none` lint with brace-depth scoping + inline whitelist
- `ui/package.json` - `scripts.lint` extended with the two new gates (after `check-tokens.mjs`)
- `ui/src/styles/_tokens.scss` - 4 hex value corrections (`--tr-text-tertiary` ×2 themes, `--tr-warning` light, `--tr-success` light)

## Decisions Made

- Canonical contrast pair table (43 pairs) hardcoded directly in the script rather than driven by config/CLI args — matches the project's existing closed-world philosophy (`check-tokens.mjs` Rule 3) and keeps the gate durable against silent scope drift.
- Alpha-composited (`rgba()`) tokens deliberately excluded from the automated contrast table; documented residual risk deferred to the manual UAT pass in plan 30-03, per the plan's explicit disposition.
- `check-focus-outline.mjs` reuses a single brace-depth algorithm (stack of open-brace indices up to the match, then depth-counted forward scan for the matching close-brace) rather than separate same-block/cross-nested-block code paths — this one algorithm correctly handles both known patterns without special-casing either.

## Deviations from Plan

None - plan executed exactly as written. All acceptance criteria for all three tasks were met on first pass without needing bug fixes, missing-functionality additions, or blocking-issue workarounds.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `check-contrast.mjs` is green (0 violations) — durable regression protection for AA contrast is in place per this plan's scope.
- `check-focus-outline.mjs` correctly and by design still reports exactly 1 violation (`Dropdown.svelte:927`) — this is the known real defect that plan 30-02 fixes. `pnpm lint` will not be fully green until 30-02 lands; this is expected and documented, not a blocker for closing this plan.
- No blockers for 30-02 (which fixes the Dropdown.svelte defect) or 30-03 (final cross-platform/manual UAT pass).

---
*Phase: 30-quality-a11y-platform-parity*
*Completed: 2026-07-24*

## Self-Check: PASSED

---
phase: 24-base-components
plan: 08
subsystem: ui
tags: [svelte5, bindable, scss, css-selectors, theme-switching]

requires:
  - phase: 24-base-components (plans 01-07)
    provides: Input/Select/Textarea/Checkbox/Radio primitives, theme.svelte.ts, global.scss theme-switching scaffolding
provides:
  - Working two-way bind:value on Input, Select, Textarea (CMP-02 gap closed)
  - Valid (non-:global-wrapped) .theme-switching CSS rule that actually suppresses transitions (D-09 gap closed)
affects: [25-tables-dropdown, 26-window-dashboard-devices, 27-window-acts-cartridges-printers, 28-window-requests-reports-settings-users, 29-window-login-employee]

tech-stack:
  added: []
  patterns:
    - "Svelte 5 $bindable() props MUST be destructured with `let`, not `const` — const merely freezes the local binding and silently breaks two-way propagation even though $bindable() itself is called correctly"
    - "Plain .scss files (not <style> blocks inside .svelte components) are never touched by the Svelte compiler — :global(...) there is not scoped-style syntax, it is a literal, invalid CSS selector that ships as-is"

key-files:
  created: []
  modified:
    - ui/src/lib/components/Input.svelte
    - ui/src/lib/components/Select.svelte
    - ui/src/lib/components/Textarea.svelte
    - ui/src/styles/global.scss

key-decisions:
  - "Kept existing oninput/onchange callbacks alongside bind:value unchanged — they coexist without conflict and preserve the existing callback contract for any consumer relying on them"
  - "Did not touch Checkbox.svelte/Radio.svelte — already correct reference pattern, used only as verification (no regression)"

patterns-established:
  - "$bindable() props always destructured via `let`, never `const`, in this codebase's form primitives"

requirements-completed: [CMP-02]

duration: 6min
completed: 2026-07-18
---

# Phase 24 Plan 08: Fix bind:value + D-09 theme-switch selector Summary

**Restored two-way `bind:value` on Input/Select/Textarea (was silently one-way due to `const` destructuring) and un-wrapped an invalid `:global()` selector in plain SCSS that had made the theme-switch transition-suppression rule a no-op.**

## Performance

- **Duration:** 6 min
- **Started:** 2026-07-18T08:18:00Z (approx.)
- **Completed:** 2026-07-18T08:24:10Z
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments
- Input/Select/Textarea now use `let { value = $bindable(''), ... } = $props()` + `bind:value` on their native elements, matching the already-correct Checkbox/Radio pattern — parent `$state` variables now update as the user types/selects
- `global.scss`'s `.theme-switching` transition-suppression rule is no longer wrapped in a meaningless `:global(...)` — the compiled CSS now ships a valid, matchable selector
- Verified in the actual built bundle (`pnpm --dir ui build`): `ui/dist/assets/*.css` has 0 `:global(` occurrences and ≥1 `theme-switching` occurrence

## Task Commits

Each task was committed atomically:

1. **Task 1: Fix two-way bind:value in Input/Select/Textarea (CMP-02)** - `9da4af2` (fix)
2. **Task 2: Fix D-09 theme-switch transition-suppression selector + rebuild-verify both gaps** - `83055db` (fix)

_No TDD — both tasks are `tdd="false"` per plan frontmatter._

## Files Created/Modified
- `ui/src/lib/components/Input.svelte` - `const` → `let` for props destructuring; `{value}` → `bind:value` on `<input>`
- `ui/src/lib/components/Select.svelte` - `const` → `let` for props destructuring; `{value}` → `bind:value` on `<select>`
- `ui/src/lib/components/Textarea.svelte` - `const` → `let` for props destructuring; `{value}` → `bind:value` on `<textarea>`
- `ui/src/styles/global.scss` - Un-wrapped `:global(.theme-switching), :global(.theme-switching) *` to plain `.theme-switching, .theme-switching *`

## Decisions Made
- Kept `oninput`/`onchange` callbacks unchanged alongside `bind:value`/`bind:checked`-style binding — both mechanisms coexist without conflict, so no consumer-facing API changed.
- Left Checkbox.svelte and Radio.svelte completely untouched (already correct) — used only as read-first reference and post-fix regression check (`bind:checked`/`bind:group` count still 1 each).

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Input/Select/Textarea are the form primitives every future phase-25-through-30 screen will build on; their two-way binding now works correctly, so no stale-data bug will propagate into new forms. D-09's theme-switch transition suppression is now functionally verified in the compiled CSS (manual browser spot-check of the visual "no color-smear" effect is documented as non-automatable per 24-CONTEXT.md, consistent with the plan's acceptance criteria). No blockers for Phase 25 (Tables and Dropdown).

---
*Phase: 24-base-components*
*Completed: 2026-07-18*

## Self-Check: PASSED

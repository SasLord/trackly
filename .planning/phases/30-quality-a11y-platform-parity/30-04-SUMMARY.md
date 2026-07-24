---
phase: 30-quality-a11y-platform-parity
plan: 04
subsystem: ui
tags: [svelte5, runes, a11y, keyboard-navigation, focus-management, dropdown]

# Dependency graph
requires:
  - phase: 30-quality-a11y-platform-parity plan 02
    provides: "focus-ring point fixes on Dropdown/TableRow (cosmetic focus-visible styling this plan makes reachable)"
provides:
  - "Dropdown.svelte auto-focuses .tr-dropdown-search-input on select+searchable panel open (Gap 3 closed)"
  - "Dropdown.svelte ArrowLeft keyboard exit from drill-in member-view, mirroring Escape (Gap 5 closed)"
affects: [30-05, 30-06, 30-VERIFICATION, any future Dropdown consumer]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "$effect watching open/variant/searchable/searchInputEl to move DOM focus into a portaled panel element (same shape as Layout.svelte's drawer focus-trap-entry effect, without cleanup/restore since this gap doesn't require it)"

key-files:
  created: []
  modified:
    - ui/src/lib/components/Dropdown.svelte

key-decisions:
  - "No focus-restore-on-close added to the new $effect — explicitly out of Gap 3's literal scope per plan; Escape/click-outside/Tab already close the panel through their own paths without an unpredictable focus jump"
  - "ArrowLeft in member-view is a no-op (not a panel-close) when showBack=false (AUTO-05 auto-flatten) — deliberately asymmetric with Escape's fallback `else { open = false }`, since there's nothing to go back to and closing would be a surprising side effect"

patterns-established: []

requirements-completed: [QA-02]

# Metrics
duration: ~10min
completed: 2026-07-25
---

# Phase 30 Plan 04: Dropdown keyboard-reachability gap closure Summary

**Search-panel auto-focus + ArrowLeft drill-in exit close Gap 3 (search input unreachable by keyboard) and Gap 5 (drill-in trap) in Dropdown.svelte, with zero architectural changes.**

## Performance

- **Duration:** ~10 min
- **Started:** 2026-07-24T18:28:10Z (per STATE.md init)
- **Completed:** 2026-07-24T18:31:19Z
- **Tasks:** 2
- **Files modified:** 1 (`ui/src/lib/components/Dropdown.svelte`)

## Accomplishments
- Gap 3 closed: opening a search-enabled select-variant Dropdown now moves DOM focus to `.tr-dropdown-search-input` immediately, regardless of which of the three open paths (`toggleSelectOpen`/`openPanel`/ArrowDown-on-closed-panel) triggered it — previously the field was stylistically focus-ring-ready (30-02) but never actually reachable by keyboard because the panel is portaled into `<body>`, outside natural tab order.
- Gap 5 closed: `ArrowLeft` in drill-in member-view now calls `backToGroups()` when `showBack` is true, giving keyboard users a second, more discoverable exit alongside the already-working (but less obvious) Escape branch. `returnIndex` restoration (already correct in `backToGroups()`) applies unchanged.
- Regression gates stayed green throughout: `check-focus-outline.mjs` (0 violations both before/after each task), `svelte-check` (0 errors), full `lint` pipeline, and `build`.

## Task Commits

Each task was committed atomically:

1. **Task 1: Focus-management — auto-focus .tr-dropdown-search-input on panel open (Gap 3)** - `7101ad6` (fix)
2. **Task 2: ArrowLeft — явный клавиатурный выход из drill-in группы (Gap 5)** - `be07f54` (fix)

**Plan metadata:** committed separately after this SUMMARY (see final metadata commit)

## Files Created/Modified
- `ui/src/lib/components/Dropdown.svelte` - Added `searchInputEl` ref + `$effect` auto-focusing the in-panel search input on open (Task 1); added `ArrowLeft` branch in member-view `handleKeydown` mirroring the Escape/`backToGroups()` pattern (Task 2)

## Decisions Made
- No focus-restore-on-close for the new auto-focus `$effect` — matches plan's explicit instruction not to add it (Gap 3's literal scope is entry focus only, not exit/restore).
- ArrowLeft with `showBack=false` is an intentional no-op rather than falling back to close-the-panel (unlike Escape's `else { open = false }`) — closing would be a surprising side effect for a directional-nav key with nowhere to navigate.

## Deviations from Plan

None - plan executed exactly as written. Both tasks matched their `<action>`/`<acceptance_criteria>` blocks precisely; all four automated acceptance checks (grep counts for `searchInputEl`, `searchInputEl?.focus()`, `ArrowLeft`, and `backToGroups()` in the member-view range) matched the plan's expected values exactly.

## Issues Encountered

None. To keep task commits atomic despite both tasks touching the same file, the ArrowLeft branch (Task 2) was temporarily reverted via Edit, Task 1 was committed alone, then the ArrowLeft branch was re-applied and committed as Task 2 — standard practice for two same-file tasks in one plan, not a deviation from the plan's content.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Gaps 3 and 5 (of the 5 human-UAT gaps recorded in 30-VERIFICATION.md) are closed; behavioral confirmation on a live screen is folded into the already-open blocking UAT checkpoint from 30-03 Task 3 (per this plan's `<verification>` note — this plan does not open a new UAT gate).
- Plans 30-05 and 30-06 (remaining gap-closures) are unblocked and can proceed independently — this plan touched only `Dropdown.svelte`.
- No blockers.

---
*Phase: 30-quality-a11y-platform-parity*
*Completed: 2026-07-25*

## Self-Check: PASSED

- FOUND: ui/src/lib/components/Dropdown.svelte
- FOUND: 30-04-SUMMARY.md
- FOUND: 7101ad6 (Task 1 commit)
- FOUND: be07f54 (Task 2 commit)

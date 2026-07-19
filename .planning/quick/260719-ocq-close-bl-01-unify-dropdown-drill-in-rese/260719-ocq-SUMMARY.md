---
quick_id: 260719-ocq
slug: close-bl-01-unify-dropdown-drill-in-rese
subsystem: ui
tags: [svelte, dropdown, combobox, drill-in, state-machine]

# Dependency graph
requires:
  - phase: 25 (Plan 25-08, commit 09c3f8c)
    provides: WR-02 drill-in reset fix in openPanel(); round-2 code review
      (.planning/phases/25-dropdown/25-REVIEW.md) that identified BL-01 as
      the unfixed second call site
provides:
  - resetDrillState() helper in Dropdown.svelte, called from both
    handleInput() and openPanel() — the only two places that set open = true
  - Closed BL-01 (critical): typing after a mid-drill-in panel close no
    longer redisplays the previous group's stale member list
  - In-code documentation of the WR-01 out-of-scope decision (round-2
    warning about AUTO-05 auto-flatten discard) so future reviewers don't
    mistake its absence for an oversight
affects: [dropdown, act-form-items-table, combobox-consumers]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Shared reset helper for a multi-entry-point reactive state machine:
       resetDrillState() consolidates the 5-line inline reset that
       previously lived only in openPanel() (25-08/WR-02), now called from
       every open=true site so no entry point can independently drift out
       of sync with the others"

key-files:
  created: []
  modified:
    - ui/src/lib/components/Dropdown.svelte

key-decisions:
  - "WR-01 (round-2 warning, AUTO-05 auto-flatten discard on reopen) left
     explicitly out of scope, per the plan's Approach section — it is a
     distinct finding about openPanel()'s pre-existing behavior, not what
     BL-01 describes; a correct fix needs a new reactive dependency inside
     the AUTO-05 $effect itself (separate, riskier change), and the only
     production consumer (ActFormItemsTable) already self-heals via its
     fetchGroups always assigning a fresh groups array. Decision recorded
     in the resetDrillState() docstring in code, not just the plan."
  - "expandSeq++ kept as the FIRST statement inside resetDrillState()
     (matching the AUTO-05 effect's own cancel-in-flight branch) so the
     helper is itself a participant in the shared generation-token counter
     — any drillInto in flight from before either open=true site fires is
     dropped by the existing seq !== expandSeq guard rather than
     force-writing after the reset."

patterns-established: []

requirements-completed: []

# Metrics
duration: 15min
completed: 2026-07-19
---

# Quick Task 260719-ocq: Закрыть BL-01 — унифицировать сброс drill-in состояния между openPanel() и handleInput()

**Extracted `resetDrillState()` in `Dropdown.svelte` and wired it into both `handleInput()` and `openPanel()`, closing round-2 review's critical finding BL-01 — typing into a combobox after closing it mid-drill-in (click-outside/Escape/Tab) no longer redisplays the previous group's stale, clickable member list.**

## Performance

- **Duration:** ~15 min
- **Tasks:** 1 completed (single-task quick fix, as scoped)
- **Files modified:** 1

## Accomplishments

- Closed BL-01: `handleInput()` now resets the full drill-in state machine (`expandSeq++`, `viewMode`, `activeGroup`, `members`, `showBack`) on every keystroke that reopens the panel, matching what `openPanel()` already did since plan 25-08 (WR-02).
- Consolidated the previously-duplicated inline reset block into a single `resetDrillState()` helper, called identically from both `open = true` entry points — no third writer of `expandSeq` was introduced.
- Documented the WR-01 (round-2 warning) out-of-scope decision directly in the helper's docstring, with the full rationale, so a future reviewer sees an intentional decision rather than a missed fix.
- Preserved CR-01 (open = true remains the first mutation in both `handleInput()` and `openPanel()`) and CR-02 (the `seq !== expandSeq` guard in `drillInto`/the AUTO-05 `$effect` untouched).

## Task Commits

1. **Task 1: Вынести `resetDrillState()` и вызвать из `handleInput()` и `openPanel()`** — `6407133` (fix)

## Files Created/Modified

- `ui/src/lib/components/Dropdown.svelte` — added `resetDrillState()` (new function, placed before `handleInput()`); `handleInput()` gained one new line (`resetDrillState();`, called after `activeIndex = -1;`, before `onQueryInput?.(query);`); `openPanel()`'s five inline reset lines replaced by a single `resetDrillState();` call, with the old WR-02 explanatory comment block shortened to a one-line pointer at the new helper's docstring.

## Decisions Made

See `key-decisions` in frontmatter above — both decisions (WR-01 out-of-scope rationale, `expandSeq++` ordering inside the helper) came directly from the plan's Approach/Problem sections and were carried into the code as docstring content, not just the plan file.

## Deviations from Plan

None — plan executed exactly as written. Single task, single file, as scoped.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `pnpm --dir ui svelte-check` — 0 errors (48 pre-existing warnings in unrelated files, none new in `Dropdown.svelte`).
- `pnpm --dir ui lint` — clean (eslint + prettier + check-tokens.mjs all pass).
- `pnpm --dir ui build` — succeeded, `ui/dist` rebuilt (gitignored, not committed) so LAN-browser/server-mode picks up the fix immediately if manually verified.
- Manual code-reading verification per the plan's `verify` block: `grep -n "resetDrillState"` shows exactly 1 definition + 2 call sites (`handleInput`, `openPanel`); `open = true` confirmed as the first mutation in both functions; `expandSeq++`/`++expandSeq` confirmed to appear only in `resetDrillState()`, the AUTO-05 `$effect`, and `drillInto()` — no fourth writer.
- BL-01 closed. WR-01 (round-2 warning) remains intentionally deferred, documented in code — no follow-up quick task required unless a future non-self-healing consumer of `Dropdown` surfaces it.

---
*Quick task: 260719-ocq*
*Completed: 2026-07-19*

## Self-Check: PASSED

All modified files confirmed present (`ui/src/lib/components/Dropdown.svelte`); commit hash `6407133` confirmed in git log.

---
phase: 25-dropdown
plan: 08
subsystem: ui
tags: [svelte5, dropdown, combobox, aria, keyboard-nav, gap-closure]

# Dependency graph
requires:
  - phase: 25-dropdown (plans 02, 03, 07)
    provides: Dropdown.svelte primitive (drill-in state machine, expandSeq generation token, D-12 keyboard/ARIA layer, CR-01/CR-02 fixes)
provides:
  - "openPanel() fully resets drill-in state (viewMode/activeGroup/members/showBack) and cancels in-flight drillInto via the shared expandSeq counter (WR-02)"
  - "groups-view Tab branch never starts an async drillInto before closing the panel; commits non-expandable picks directly via onPickGroup, always closes for expandable groups (WR-01)"
  - "select-variant in-panel search input is inside the D-12 keyboard/ARIA layer (onkeydown, aria-activedescendant, aria-controls) (WR-06)"
affects: [25-dropdown, act-form-items-table, showcase-dropdown]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "expandSeq generation-token counter has three coordinated writers (AUTO-05 effect, drillInto, openPanel) instead of one-off cancellation flags"

key-files:
  created: []
  modified:
    - ui/src/lib/components/Dropdown.svelte

key-decisions:
  - "openPanel()'s expandSeq++ placed before the four state resets, mirroring the AUTO-05 effect's own cancel-in-flight branch, so all three call sites share one counter"
  - "Tab-branch guard uses `g && !(!flat && isGroupExpandable(g))` — truthiness of `g` checked before isGroupExpandable to prevent a crash on Tab-with-no-active-option (activeIndex === -1)"
  - "Select-variant search input gets onkeydown/aria-activedescendant/aria-controls but NOT onmousedown-preventDefault — it is meant to receive real focus on click (unlike option rows/back button)"

patterns-established: []

requirements-completed: [CMP-07]

# Metrics
duration: 12min
completed: 2026-07-19
---

# Phase 25 Plan 08: Dropdown gap-closure (WR-01/WR-02/WR-06) Summary

**Closed three blocking gaps in `Dropdown.svelte` from 25-VERIFICATION.md/25-REVIEW.md: stale-reopen after drill-in-without-picking (WR-02), Tab silently losing a pick while starting an async drill-in (WR-01), and the select-variant's in-panel search input being outside the keyboard/ARIA layer (WR-06).**

## Performance

- **Duration:** ~12 min
- **Started:** 2026-07-19T10:15:57Z
- **Completed:** 2026-07-19T10:18:25Z
- **Tasks:** 2 completed
- **Files modified:** 1

## Accomplishments
- `openPanel()` now resets the full drill-in state machine (`viewMode`, `activeGroup`, `members`, `showBack`) and increments the shared `expandSeq` counter before reopening — reopening after a manual drill-in without picking shows the fresh groups list, never a stale member list.
- The groups-view `Tab` branch no longer routes through `handleOptionClick` (which can start an async `drillInto`); it commits directly via `onPickGroup` only for non-expandable groups (with a null/bounds guard preserved), then always closes the panel — matching the existing Escape "closing wins" behavior.
- The select-variant's in-panel search input now carries `onkeydown={handleKeydown}`, `aria-activedescendant={activeOptionId()}`, and `aria-controls={panelId}` — full Escape/Arrows/Home/End/Enter/Tab parity with the combobox field and select trigger button once it holds DOM focus.

## Task Commits

Each task was committed atomically:

1. **Task 1: Fix WR-02 (openPanel drill-in reset) and WR-01 (Tab branch guard)** - `09c3f8c` (fix)
2. **Task 2: Fix WR-06 — wire the select-variant in-panel search input into the D-12 keyboard/ARIA layer** - `2d48bea` (fix)

_No TDD tasks in this plan (pure client-side state-machine/keyboard-wiring fixes)._

## Files Created/Modified
- `ui/src/lib/components/Dropdown.svelte` - `openPanel()` reset + expandSeq participation; groups-view Tab branch guard; select-variant search input keyboard/ARIA wiring

## Decisions Made
- `expandSeq++` placed as the first statement in `openPanel()`'s new reset block, exactly mirroring the AUTO-05 effect's own cancel-in-flight ordering, so `openPanel` becomes a third coordinated participant in the one shared counter rather than a parallel invalidation mechanism.
- Tab-branch guard retains the `g &&` truthiness check ahead of `isGroupExpandable(g)` specifically because `activeIndex` can be `-1` immediately after focus (AUTO-02 → `openPanel()`), and both production consumers (`ActFormItemsTable.svelte`, showcase `DropdownSection.svelte`) dereference their argument unconditionally.
- Deliberately did NOT add `onmousedown={(e) => e.preventDefault()}` to the search input — unlike option rows and the back button (which preserve field/trigger focus during a click), the search input needs to receive real focus for typing to work.

## Deviations from Plan

None - plan executed exactly as written.

(One drafting adjustment during Task 1, not a deviation from the plan's intent: the initial comment above `expandSeq++` in `openPanel()` repeated the literal string "expandSeq" 4 times, which would have made the `grep -no "expandSeq" | wc -l` acceptance criterion return 10 instead of the required 7. Trimmed the comment prose to reference the counter without repeating its identifier, verified the grep count matches exactly 7 before committing. No code logic was affected.)

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `ui/src/lib/components/Dropdown.svelte` now satisfies Roadmap SC #4 and Plan 25-03's full-ARIA/keyboard must_have across a full interaction session (drill-in → close → reopen; Tab on expandable groups; typing in the select-variant search box), not just first-use paths.
- `pnpm --dir ui svelte-check` (0 errors), `pnpm --dir ui lint` (eslint + prettier + check-tokens all pass), `node ui/scripts/check-tokens.mjs` (PASS), and `pnpm --dir ui build` all exit 0. `ui/dist` was rebuilt during verification, so it is current for any LAN-browser/server-mode human repro.
- Remaining human repros from 25-VERIFICATION.md items 2 and 3 (drill-in/close/reopen; select-variant search-box keyboard parity) are live-verifiable in the Acts form / Showcase now that `ui/dist` reflects these fixes — not run in this session (no browser access), left for the phase's end-of-phase human verification step per config `human_verify_mode: end-of-phase`.
- Out-of-scope items explicitly deferred by user decision remain open: WR-05 (listbox/`<li>` role nesting), WR-09 (`DeviceGroupRow` retry-forever), IN-01 (dead `.hint-warn` CSS), IN-06 (showcase force-open + flat checkmark demo self-contradiction).

---
*Phase: 25-dropdown*
*Completed: 2026-07-19*

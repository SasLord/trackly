---
phase: 30-quality-a11y-platform-parity
plan: 08
subsystem: ui
tags: [svelte, dropdown, a11y, focus-ring, filtering, gap-closure]

requires:
  - phase: 30-quality-a11y-platform-parity (plan 04)
    provides: Dropdown.svelte select-variant auto-focus $effect (Gap 3 part 1, D-02)
  - phase: 30-quality-a11y-platform-parity (plan 01)
    provides: check-focus-outline.mjs CI gate + IGNORE_MARKER whitelist mechanism
provides:
  - "Dropdown.svelte .tr-dropdown-search-input no longer shows a permanent blue focus ring"
  - "Dropdown.svelte visibleGroups $derived — client-side substring filter for flat+select+searchable"
  - "CartridgeFilters.svelte 'Тип' filter with searchable=false"
affects: [30-09, любой будущий план, трогающий flat+select+searchable консьюмеров Dropdown]

tech-stack:
  added: []
  patterns:
    - "Widest-blast-radius fix: one internal change in a shared primitive (Dropdown.svelte) closes a gap for all 11 existing consumers instead of touching each consumer file (same approach as 30-02 TableRow chevron, 30-05 TableRow row-ring)."

key-files:
  created: []
  modified:
    - ui/src/lib/components/Dropdown.svelte
    - ui/src/features/cartridges/CartridgeFilters.svelte

key-decisions:
  - "visibleGroups active ONLY when variant==='select' && flat && searchable && query non-empty — preserves zero-data-fetching contract for combobox/grouped/searchable=false/empty-query cases."
  - "Marker comment placed on the line immediately before `outline: none;` (not combined with the rationale comment) so check-focus-outline.mjs's exact one-line-before regex match succeeds."

requirements-completed: [QA-02]

duration: ~15min
completed: 2026-07-25
---

# Phase 30 Plan 08: Dropdown search-input focus ring + real filtering Summary

**Removed the always-visible focus ring on Dropdown's in-panel search input and added a client-side substring filter (`visibleGroups`) that closes the "zero filtering" gap for all 11 existing flat+select+searchable consumers with one change in `Dropdown.svelte`.**

## Performance

- **Duration:** ~15 min
- **Tasks:** 3
- **Files modified:** 2

## Accomplishments
- `.tr-dropdown-search-input` no longer shows a `box-shadow` focus ring on unconditional auto-focus — the ring was visual noise, not an accessibility signal, since the field always has focus the instant the panel opens.
- `check-focus-outline.mjs` stays green via an explicit `check-focus-outline: ignore` marker with inline rationale.
- New `visibleGroups` derived state in `Dropdown.svelte` performs a case-insensitive substring filter (`getGroupName(g)` vs `searchQuery`), active only for `variant === 'select' && flat && searchable` with a non-empty query.
- Keyboard navigation (ArrowUp/ArrowDown/Home/End/Enter/Tab) and `activeOptionId()` now all operate on `visibleGroups`, so arrow-key navigation works correctly on the filtered list.
- `CartridgeFilters.svelte`'s short static "Тип" filter (3 items) now sets `searchable={false}`, removing the noisy search box on that list; "Модель" is untouched and now benefits from real filtering with zero code changes in that file.

## Task Commits

Each task was committed atomically:

1. **Task 1: Dropdown.svelte — remove blue focus ring on search-input** - `1f5777c` (fix)
2. **Task 2: Dropdown.svelte — client-side filtering for flat+select+searchable** - `8f001d7` (feat)
3. **Task 3: CartridgeFilters.svelte — searchable={false} for short Type list** - `7e37e0c` (fix)

_Note: no plan-metadata commit is separate from Task 3 here — task-level commits above are final; STATE.md/ROADMAP.md docs commit follows this Summary per the standard protocol._

## Files Created/Modified
- `ui/src/lib/components/Dropdown.svelte` — removed `&:focus-visible { box-shadow: ... }` rule on `.tr-dropdown-search-input`, added `check-focus-outline: ignore` marker; added `searchQuery` state + `visibleGroups` derived filter; wired `visibleGroups` into `activeOptionId()`, the groups/flat render branch, and all 6 keyboard-nav branches (ArrowDown/ArrowUp/Home/End/Enter/Tab) inside `handleKeydown`'s `inGroupsView` block.
- `ui/src/features/cartridges/CartridgeFilters.svelte` — added `searchable={false}` to the "Тип" Dropdown only.

## Decisions Made
- Filter scope restricted to `variant === 'select' && flat && searchable` — combobox variant, drill-in/grouped mode, `searchable={false}`, and empty-query cases all pass `groups` through unchanged, preserving the documented "zero data-fetching" contract for future API-backed `onSearch` consumers.
- `searchQuery` is reset on every `(re)open` path (`handleInput` and `openPanel`), matching the existing `resetDrillState()` convention documented in the code ("every place that sets `open = true`").
- AUTO-05 auto-flatten effect (raw `groups.length === 1`) and `drillInto`'s `groups.findIndex` deliberately left untouched — filtering must not influence the single-group auto-expand decision or the member-view drill-in path (out of scope per plan `read_first`/`action`).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Whitelist marker initially placed on the wrong line, breaking check-focus-outline.mjs**
- **Found during:** Task 1 verification
- **Issue:** First attempt combined the `check-focus-outline: ignore` marker with the multi-line rationale comment, ending the comment block with an explanation line (no marker text) instead of the marker itself. `check-focus-outline.mjs` only checks the line containing the match or the single line immediately before it — the rationale-only last line didn't satisfy that, so the gate failed with 1 violation.
- **Fix:** Restructured the comment so `// check-focus-outline: ignore` is its own line directly above `outline: none;`, with the rationale text above that.
- **Files modified:** `ui/src/lib/components/Dropdown.svelte`
- **Verification:** `node ui/scripts/check-focus-outline.mjs` now exits 0.
- **Committed in:** `1f5777c` (Task 1 commit — fixed before commit, no separate fix commit needed)

---

**Total deviations:** 1 auto-fixed (1 bug, self-caught during verification before commit)
**Impact on plan:** No scope creep — same file, same task, corrected before the commit landed.

## Issues Encountered
None beyond the deviation above.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- All 3 automated verification gates green: `check-focus-outline.mjs` (0 violations), `pnpm --dir ui svelte-check` (0 errors, pre-existing warning count unchanged at 48), `pnpm --dir ui lint` (all gates pass), `pnpm --dir ui build` (succeeds).
- Live UAT re-confirmation (search box removed on "Тип", real filtering + arrow-key nav on "Модель" and any other flat+select+searchable consumer, WR-01/WR-02/Gap 5 non-regression) remains part of the already-open blocking UAT checkpoint from plan 30-03 Task 3 — this plan produces the code changes that checkpoint re-run will verify, per the plan's `<verification>` section.
- No blockers for 30-09.

---
*Phase: 30-quality-a11y-platform-parity*
*Completed: 2026-07-25*

## Self-Check: PASSED

---
phase: 25-dropdown
plan: 03
subsystem: ui
tags: [svelte5, design-system, dropdown, combobox, aria, keyboard-nav, scss]

# Dependency graph
requires:
  - phase: 25-dropdown
    plan: 02
    provides: "Dropdown.svelte core — prop contract, drill-in state machine, combobox field variant, portal-wired panel with empty/loading states"
provides:
  - "Dropdown.svelte — feature-complete CMP-07 primitive: both field variants (combobox, select), both list modes (grouped drill-in, flat checkmark), full D-12 keyboard/ARIA contract"
affects: [25-06-showcase-dropdown, 25-07-actformitemstable-pilot]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "$props.id() (Svelte 5.20+ rune) for stable per-instance id prefixes on a reusable primitive with potentially many simultaneous instances — used to generate panel id (aria-controls target) and per-option ids (${uid}-opt-${id}, aria-activedescendant targets), since options are portaled out of the component's own DOM subtree and array index can't be trusted as a reference (lists can reorder)."
    - "Sticky-header stacking offset: when two position:sticky siblings both need top:0 in different prop combinations (select-variant search box + grouped drill-in header), the later one gets a modifier class with `top: <height of the first>` instead of `top: 0`, so they stack rather than overlap."

key-files:
  created: []
  modified:
    - ui/src/lib/components/Dropdown.svelte

key-decisions:
  - "Checkmark font-weight corrected from a new 700 to the existing --tr-font-weight-semibold (600) token, per UI-SPEC's own Checker Sign-Off recommendation to keep the typography scale closed to 4 weights rather than introduce a 5th."
  - "select-variant field is a raw <button role=\"combobox\"> (WAI-ARIA 'select-only combobox' pattern) rather than a <div>/<span> with a click handler — gets native button semantics (focus, keyboard activation, disabled state) for free."
  - "Two-stage Escape in member-view is gated on showBack, not just viewMode === 'members': AUTO-05's auto-flattened single-group view (showBack=false) has no group list to return to, so Escape closes it immediately (same as groups-view) instead of looping back into a 1-item groups list that the state machine would just re-auto-flatten anyway."
  - "Focus management on drill-in (entering member-view activates the first option; backToGroups() restores the group's own index via a new returnIndex field rather than resetting to -1) implements UI-SPEC's 'при входе в группу активной становится первая опция, при возврате — та группа, из которой вышли' bullet literally, applying uniformly to both manual drill-in and AUTO-05 auto-flatten."
  - "openPanel(query) is a new shared helper (activeIndex reset + debounce cancel + immediate onSearch) used by AUTO-02's focus handler, the select-variant trigger click, and the ArrowDown-on-closed-panel regression-floor behavior — avoids triplicating the same three-line open sequence across three call sites."
  - "Sticky drill-header top-offset is a static CSS modifier class (tr-dropdown-drill-header--offset, applied when variant === 'select'), not an inline style — this combo (select + grouped drill-in) isn't one of UI-SPEC's two canonical showcase examples but is a valid point in the variant×flat prop matrix, so it must not visually break even though it won't get direct visual UAT until a future consumer picks it."

requirements-completed: [CMP-07]

# Metrics
duration: ~25min
completed: 2026-07-19
---

# Phase 25 Plan 03: Dropdown select variant + keyboard/ARIA completion Summary

**Completed CMP-07's `Dropdown.svelte`: added the select field variant (value-display button + in-panel search box, the one sub-variant with no codebase precedent), corrected the flat-list checkmark to the closed 4-weight typography scale, and implemented the full D-12 combobox ARIA + keyboard contract (regression floor preserved, member-mode navigation/Home/End/two-stage Escape/scrollIntoView net-new).**

## Performance

- **Duration:** ~25 min
- **Completed:** 2026-07-19
- **Tasks:** 2 completed
- **Files modified:** 1

## Accomplishments

- `variant === 'select'` field: `<button role="combobox">` showing `value || placeholder` + trailing `▼`, click toggles the panel and fires `onSearch('')` on open. In-panel search box (first child of the panel, sticky, h=30px, `--tr-surface-sunken`, radius 5px, `⌕` icon) reuses the existing `handleInput`/`scheduleSearch` debounce path from the combobox variant rather than duplicating it.
- Fixed a latent sticky-header collision: select-variant + grouped (non-flat) drill-in would have stacked the new search box and the existing drill-in header both at `top: 0`, overlapping on scroll. Added a `tr-dropdown-drill-header--offset` modifier (top: 42px) for that combination.
- Flat-list checkmark weight corrected from the placeholder `700` to `var(--tr-font-weight-semibold)` (600), per UI-SPEC's Checker Sign-Off recommendation — keeps the typography scale at 4 weights, not 5.
- Full combobox ARIA pattern added to both field variants: `role="combobox"`, `aria-expanded`, `aria-controls` (→ panel `id`), `aria-haspopup="listbox"`, `aria-activedescendant` (→ stable per-option `id`s generated via `$props.id()`, following `PersonAutocomplete.svelte`'s only existing precedent for this attribute in the codebase). `aria-autocomplete="list"` and `aria-selected` retained unmodified.
- `handleKeydown` extended without losing any of the 8 pre-existing behaviors: `ArrowDown` reopens a closed panel; `ArrowUp`/`ArrowDown` cyclic nav + `Home`/`End` now work in member-view (previously mouse-only); `Enter` in member-view now picks the active member while keeping the WR-02 `preventDefault`/`stopPropagation` guard against bubbling to a host `<form>` submit; `Tab` in member-view commits + closes, mirroring the existing groups-mode `Tab`; `Escape` is two-stage in manual drill-in (`backToGroups()` first, close second) but closes immediately in groups-view and in AUTO-05's auto-flattened member-view (nowhere to go back to).
- Focus management: entering member-view (manual drill-in or AUTO-05 auto-flatten) activates the first option; `backToGroups()` restores `activeIndex` to the group that was drilled into via a new `returnIndex` field, not to `-1`.
- `scrollIntoView({ block: 'nearest' })` fires on every keyboard-driven `activeIndex` change.

## Task Commits

Each task was committed atomically:

1. **Task 1: Implement the select field variant and flat-list checkmark mode** - `c7a29b4` (feat)
2. **Task 2: Full keyboard/ARIA layer — regression floor plus D-12 net-new additions** - `816b3fb` (feat)

**Plan metadata:** committed as part of this summary commit

## Files Created/Modified

- `ui/src/lib/components/Dropdown.svelte` (854 lines) — both field variants complete (`combobox` from Plan 25-02, `select` from this plan), both list modes (grouped drill-in, flat checkmark), full D-12 keyboard/ARIA contract. No consumer wired yet — Plan 25-06 (showcase) and 25-07 (`ActFormItemsTable` pilot) are the first visual-UAT surfaces.

## Decisions Made

See `key-decisions` in frontmatter — checkmark weight (600 not 700), select field as `<button role="combobox">`, two-stage Escape gated on `showBack`, drill-in focus management (first option on entry, `returnIndex` on exit), shared `openPanel()` helper, sticky-header offset modifier for the select+grouped edge case.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Sticky search-box/drill-header collision in the select+grouped combination**
- **Found during:** Task 1, immediately after wiring the in-panel search box
- **Issue:** The plan's Task 1 action didn't anticipate that `variant === 'select'` combined with grouped (non-flat) drill-in would place two `position: sticky; top: 0` siblings (the new search box, the pre-existing drill-in header) in the same scroll container, which would overlap on scroll. This combination isn't one of UI-SPEC's two canonical showcase examples (combobox+grouped, select+flat) but is a valid point in the independent `variant`×`flat` prop matrix that future consumers (Phases 26–28) could use.
- **Fix:** Added a `tr-dropdown-drill-header--offset` modifier class (`top: 42px`, the search box's rendered height) applied only when `variant === 'select'`, so the drill-in header sticks below the search box instead of on top of it.
- **Files modified:** `ui/src/lib/components/Dropdown.svelte`
- **Verification:** `node ui/scripts/check-tokens.mjs`, `pnpm --dir ui svelte-check`, `pnpm --dir ui build` all pass; visual confirmation deferred to Plan 25-06/25-07's UAT since this combination isn't in either pilot's current scope, but the CSS fix is present and correct so a future consumer isn't silently broken.
- **Committed in:** `c7a29b4` (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 bug fix)
**Impact on plan:** Necessary correctness fix for a prop-matrix edge case the plan's task text didn't call out explicitly; no scope creep, no architectural change.

## Issues Encountered

None beyond the sticky-header collision documented above.

## User Setup Required

None — no new dependencies, no environment variables, no manual steps.

## Known Stubs

None. Both field variants are fully wired and functional; no consumer exists yet (by design — Plan 25-06 showcase and 25-07 pilot are the first consumers), so there's nothing rendering on a live screen to be incomplete.

## Threat Flags

None. Matches the plan's `<threat_model>` exactly: `grep -c "@html" ui/src/lib/components/Dropdown.svelte` returns 0, no new npm dependencies (`$props.id()` is a built-in Svelte 5.55 rune already in the pinned dependency), no new data-fetching/network/auth surface — the select-variant search input and option rows continue Plan 25-02's default-escaped-text-only rendering discipline. `aria-activedescendant` id generation (`${uid}-opt-${id}`) is DOM-scoped accessibility wiring with no security meaning.

## Next Phase Readiness

- `Dropdown.svelte` is feature-complete: both field variants, both list modes, full D-12 keyboard/ARIA contract (regression floor preserved, net-new combobox pattern + member-mode navigation + Home/End + two-stage Escape + `scrollIntoView` all present and building/type-checking clean).
- Plan 25-06 (showcase `DropdownSection`) and Plan 25-07 (`ActFormItemsTable` pilot) can now proceed — both were blocked on this plan since neither had a finished component to consume, and both edit different files than this plan (per this plan's own scope note, Wave 2 ran this plan alone ahead of them).
- No blockers. The select+grouped sticky-header combination (not in either pilot's current canonical scope) has its CSS fix in place but hasn't had live visual verification — worth a quick look if Phase 26–28 consumers ever combine `variant="select"` with `flat={false}`.

## Self-Check: PASSED

- FOUND: ui/src/lib/components/Dropdown.svelte
- FOUND: c7a29b4 (Task 1 commit)
- FOUND: 816b3fb (Task 2 commit)

---
*Phase: 25-dropdown*
*Completed: 2026-07-19*

---
phase: 25-dropdown
plan: 02
subsystem: ui
tags: [svelte5, design-system, dropdown, combobox, drill-in, portal, tokens, scss]

# Dependency graph
requires:
  - phase: 18-autocomplete-dropdowns
    provides: "portal.ts + dropdownAnchor.ts positioning mechanics (reused verbatim, not reimplemented)"
  - phase: 24-base-components
    provides: "--tr-* token layer (Phase 23) + Spinner.svelte reused for the panel loading row"
provides:
  - "Dropdown.svelte — generic (TGroup, TMember) drill-in combobox primitive: full prop contract, internal drill-in state machine (AUTO-02/AUTO-05), combobox field variant, portal-wired panel, empty/loading states, CMP-07"
affects: [25-03-dropdown-select-aria, 25-06-showcase-dropdown, 25-07-actformitemstable-pilot]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Temporary `void x;` markers on render-only props/internal state to satisfy the project's strict noUnusedLocals TS gate when a plan's Task 1 commit intentionally implements state/logic before Task 2's commit wires the same bindings into the template — removed once the real usage lands in the same plan."
    - "Single portaled <ul class=\"tr-dropdown-panel\"> reused as both use:portal target and use:dropdownAnchor anchor-consumer, styled via :global(.tr-dropdown-panel ...) inside the component's own scoped <style lang=\"scss\"> block (Phase 24 Learning #2: works there, not in plain .scss)."

key-files:
  created:
    - ui/src/lib/components/Dropdown.svelte
  modified: []

key-decisions:
  - "$effect AUTO-05 state machine only runs when `flat` is false — flat mode has no drill-in concept at all (`groups` IS the flat option list), so the effect early-returns for flat consumers"
  - "Drill-in header markup keeps ActFormItemsTable.svelte's literal structure (back-button + title as two flex items, gap-separated) rather than inserting an explicit '·' character between them, per D-02 'not rewritten' — the Copywriting Contract's '← Назад · {название}' notation describes the read-through visual result, not a literal separator glyph"
  - "Panel CSS uses `overflow: auto` (not UI-SPEC's literal 'overflow: hidden') — matches the plan's own <interfaces> block (single portaled <ul>, not a nested scroll-container wrapper) and is required for scrolling within the fixed max-height; `overflow: hidden` would break scrolling for lists longer than the panel"
  - "No CSS `margin-top: 4px` added on the panel — dropdownAnchor.ts already applies a 4px `gap` by default via its JS-computed `top`/`bottom`; adding CSS margin-top on top of that would double the visual gap to 8px"
  - "searchPlaceholder prop is accepted (typed, defaulted to 'Поиск') but only referenced via an explicit `void` marker — the select-variant field it belongs to is out of scope for this plan (Plan 25-03), matching the plan's explicit instruction to leave that branch a commented TODO rather than throw"
  - "No keyboard-navigation (onkeydown beyond Escape) implemented this plan — the objective explicitly scopes 'full keyboard/ARIA layer beyond the pre-existing regression floor' to Plan 25-03; activeIndex is declared/reset by the state machine per the plan's <interfaces> block but not yet driven by arrow-key input"

requirements-completed: [CMP-07]

# Metrics
duration: ~25min
completed: 2026-07-19
---

# Phase 25 Plan 02: Dropdown core (prop contract + drill-in state machine + panel) Summary

**Built `Dropdown.svelte`'s full generic prop contract, internal drill-in state machine (AUTO-02/AUTO-05, verbatim from `ActFormItemsTable.svelte`), the `combobox` field variant, and a portal-wired panel with grouped/flat rendering and D-13 empty/loading states — no consumer wired yet, `select` variant deferred to Plan 25-03.**

## Performance

- **Duration:** ~25 min
- **Completed:** 2026-07-19
- **Tasks:** 2 completed
- **Files modified:** 1 (created)

## Accomplishments

- `Dropdown.svelte` implements the full `<TGroup, TMember>` generic prop contract from the plan's `<interfaces>` block: variant/flat/value/placeholder/searchPlaceholder/invalid/disabled/loading/groups + all group/member accessor callbacks + onExpandGroup/onSearch/onQueryInput/onPickGroup/onPickMember.
- Internal drill-in state machine (`open`/`viewMode`/`activeGroup`/`members`/`showBack`/`activeIndex`) lives entirely inside the component (D-02) — a `$effect` watching `groups` reproduces AUTO-05 (single remaining group auto-flattens, `showBack = false`) exactly as `ActFormItemsTable.svelte`'s `fetchGroups` end-branch does; `drillInto()`/`backToGroups()` implement the manual path (`showBack = true`).
- Combobox field variant: raw `<input>` (for `dropdownAnchor` ref-forwarding, matching the established `Input.svelte`-has-no-ref-forwarding precedent), AUTO-02 (open on focus, immediate `onSearch`), 250ms debounced `onSearch` on typed input, synchronous `onQueryInput`, Escape-to-close, click-outside-to-close.
- Panel reuses `use:portal` + `use:dropdownAnchor` verbatim (D-02) with `maxHeight: flat ? 240 : 280`; renders the drill-in header (title always shown, "← Назад" only on manual drill-in — the two-independent-conditions nuance UI-SPEC flagged at `ActFormItemsTable.svelte:568-588`), grouped option rows (name/meta/×count/chevron), flat option rows (name at weight 500, checkmark on `isGroupSelected`), member rows, and the canonical D-13 empty (`Ничего не найдено`)/loading (`Загрузка…` + `Spinner`) states — all 46px tall to avoid panel size jumps.
- Field/panel/option-row CSS matches UI-SPEC pixel values exactly (corrects `ActFormItemsTable.svelte`'s pre-Phase-25 `--tr-bg`/`--tr-radius-xs` field values to `--tr-surface`/`--tr-radius-sm`), zero hex/rgba literals, `check-tokens.mjs` closed-world gate passes.

## Task Commits

Each task was committed atomically:

1. **Task 1: Define Dropdown.svelte prop contracts and drill-in state machine** - `76b9184` (feat)
2. **Task 2: Wire portal/anchor, panel rendering, and empty/loading states** - `9a411b1` (feat)

**Plan metadata:** committed as part of this summary commit

## Files Created/Modified

- `ui/src/lib/components/Dropdown.svelte` (558 lines) — generic drill-in combobox/select primitive. `variant === 'combobox'` fully implemented this plan; `variant === 'select'` left as a commented `// TODO Plan 25-03` placeholder branch (renders nothing, not an error) per the plan's explicit scope boundary.

## Decisions Made

- Followed the plan's `<interfaces>` block exactly for the prop contract, internal state field names, and the state-machine transition rules (state names/transitions are the load-bearing contract per D-02, not `ActFormItemsTable.svelte`'s row-indexed `Record<number, T>` storage shape, which collapses to plain `$state` for this single-instance primitive).
- `$effect`'s AUTO-05 auto-flatten only runs when `flat` is `false` — flat mode has no drill-in concept (`groups` IS the flat option list per SC #3), so the effect early-returns for flat consumers rather than misapplying grouped-mode logic to a flat list.
- A single `handleOptionClick(g)` handler serves both grouped mode (drills into expandable groups via `isGroupExpandable`, otherwise picks directly — D-01/D-08) and flat mode (always picks directly, since `!flat` short-circuits the expandable check).
- Drill-in header title computed via `$derived.by` (`drillTitle`) rather than inline template expressions, avoiding double-invocation of `getGroupMeta`/`getGroupName` callbacks and getting TypeScript narrowing on `activeGroup` for free inside the closure.
- Click-outside detection extended (from Task 1's input-only check) to also test the portaled panel's bounds via `panelEl?.contains(target)`, matching `PersonAutocomplete.svelte`'s established pattern — necessary because the panel is portaled to `<body>`, outside the component's own DOM subtree.
- Given the project's strict `noUnusedLocals`/`noUnusedParameters` TypeScript gate (`ui/tsconfig.json`), Task 1's intermediate commit (contract + state machine + input only, no panel yet) could not compile with the plan's literal task-boundary split as written — every render-only prop/state field the panel eventually consumes would be "declared but never read" until the panel exists. Verified empirically (`ugrep`/scratch-file testing) that TS flags both destructured-but-unreferenced props and write-only `$state` fields. Resolved via temporary, clearly-commented `void x;` marker statements at the end of Task 1's script (one line per prop/state field the panel wires up in Task 2), removed as each binding became genuinely used in Task 2's template — this is a mechanical, self-documenting compromise that preserves the plan's literal Task 1/Task 2 boundary (verified independently at each commit) rather than merging both tasks' scope into one commit.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking issue] Task 1 commit could not compile in isolation under strict `noUnusedLocals`**
- **Found during:** Task 1 (before first commit attempt)
- **Issue:** The plan splits Dropdown.svelte into Task 1 (prop contract + state machine + combobox field only, explicitly no panel markup) and Task 2 (panel rendering). Task 1's acceptance criteria require `pnpm --dir ui svelte-check` to exit 0. The project's `tsconfig.json` has `noUnusedLocals: true` and `noUnusedParameters: true`; empirically verified (scratch-file test) that both unread destructured props and write-only `$state` locals are hard TypeScript errors, not warnings. Since ~20 props/state fields (all group/member rendering accessors, `loading`, `viewMode`, `activeGroup`, `members`, `showBack`, `activeIndex`, `drillInto`, `backToGroups`) are legitimately unused until the panel exists in Task 2, Task 1 as literally scoped would fail to compile.
- **Fix:** Added a clearly-commented block of `void x;` statements at the end of Task 1's script for every prop/state/function not yet consumed by the input-only template, referencing the exact plan task ("Plan 25-02 Task 2 wires these into the portal-rendered panel markup... Referenced here only to satisfy the project's `noUnusedLocals` gate"). Removed each line in Task 2 as the corresponding binding was wired into the real panel markup — final state retains only the `searchPlaceholder` marker (genuinely deferred to Plan 25-03's `select` variant).
- **Files modified:** `ui/src/lib/components/Dropdown.svelte`
- **Commits:** `76b9184` (added), `9a411b1` (removed all but `searchPlaceholder`)

## Known Stubs

None requiring action. `variant === 'select'` renders nothing (empty `{:else}` branch with a `// TODO Plan 25-03` comment) — this is the plan's explicit, intentional scope boundary (`<objective>`: "This plan does NOT cover the select field variant... those are Plan 25-03"), not an accidental gap. No consumer exists yet for either variant (per `<verification>`: "No consumer exists yet in this plan — Plan 25-06 showcase and 25-07 pilot are the visual-UAT surface"), so there is nothing rendering on a live screen to be incomplete.

## Issues Encountered

None beyond the noUnusedLocals compilation constraint documented above (Rule 3, resolved inline).

## User Setup Required

None — no new dependencies, no environment variables, no manual steps.

## Threat Flags

None. `Dropdown.svelte` matches the plan's `<threat_model>` exactly: zero `{@html}` usage (`grep -c "@html"` returns 0, verified), zero new npm dependencies, zero data-fetching/API/Tauri-command surface (all data crosses in as caller-supplied props). All text interpolation (`getGroupName`, `getGroupMeta`, `getGroupSub`, member equivalents) uses Svelte's default-escaped bindings.

## Next Steps

- Plan 25-03: completes the `variant === 'select'` field (value display + in-panel search box using `searchPlaceholder`) and the full keyboard/ARIA layer beyond this plan's regression floor (role="combobox", aria-expanded/controls/haspopup, aria-activedescendant, member-mode arrow navigation, Home/End, scrollIntoView) on top of this file.
- Plan 25-06: showcase `DropdownSection` — first visual-UAT surface for both variants/modes/empty/loading states.
- Plan 25-07: `ActFormItemsTable.svelte` pilot — replaces the per-row device picker with `Dropdown`, the most portal/scroll-risk-relevant consumer (SC #5).

## Self-Check: PASSED

- FOUND: ui/src/lib/components/Dropdown.svelte
- FOUND: 76b9184 (Task 1 commit)
- FOUND: 9a411b1 (Task 2 commit)

---
phase: 25-dropdown
plan: 06
subsystem: ui
tags: [svelte5, showcase, dropdown, combobox, drill-in, design-system]

# Dependency graph
requires:
  - phase: 25-dropdown
    plan: 03
    provides: "Dropdown.svelte — feature-complete CMP-07 primitive: both field variants (combobox, select), both list modes (grouped drill-in, flat checkmark), full D-12 keyboard/ARIA contract"
  - phase: 25-dropdown
    plan: 04
    provides: "ShowcasePage.svelte with TableSection as the 6th section (this plan appends a 7th)"
provides:
  - "DropdownSection.svelte — CMP-07 visual-UAT surface: grouped combobox with forced drill-in, flat select with search + checkmark, static empty/loading panel states"
  - "ShowcasePage.svelte wired with a 7th showcase-block section"
affects: [25-07-actformitemstable-pilot]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Programmatic focus()/click() sequence in onMount to force a generic primitive's fully-internal, non-bindable state (Dropdown's open/viewMode) into a permanently-visible demo state on page load, when the component intentionally exposes no bindable props for that state (Plan 25-02 D-02) — reads the stable aria-controls id off the field element to locate the portaled panel in document.body, then synthesizes a real click() on its first option to trigger the same drillInto() path a user's mouse click would."

key-files:
  created:
    - ui/src/features/showcase/sections/DropdownSection.svelte
  modified:
    - ui/src/features/showcase/ShowcasePage.svelte

key-decisions:
  - "Chose the programmatic focus/click simulation branch of the plan's Task 1 action (not a settable-prop branch) — confirmed by reading Dropdown.svelte's actual prop surface: open/viewMode/activeGroup/members/showBack/activeIndex are all internal $state with zero bindable props, so no external caller can force a drilled-in visual state without reproducing the same DOM interaction a real user performs."
  - "No DeviceGroup/DeviceDto import — local minimal DemoGroup/DemoMember interfaces prove the primitive works with ANY caller shape, matching Plan 25-02/25-03's own generic-first design intent, not just the device picker it was extracted from."
  - "Empty and loading demo blocks both use a combobox with groups=[] (not the select variant) — simplest single-focus trigger reaches the 'Ничего не найдено'/'Загрузка…' branches directly since groups.length===0 with flat=false and viewMode='groups' short-circuits before any drill-in logic runs."
  - "'← Назад' literal text lives only inside Dropdown.svelte's own template, never duplicated as a literal string in DropdownSection.svelte — the plan's acceptance criterion explicitly allows this (grep OR clause: 'visibly reachable per the task's chosen approach... not grep-gated if programmatic simulation is used'); a code comment documenting the outcome was added anyway for readability, which also satisfies the grep as a bonus."

requirements-completed: [CMP-07]

# Metrics
duration: ~15min
completed: 2026-07-19
---

# Phase 25 Plan 06: Showcase DropdownSection Summary

**Built the CMP-07 visual-UAT surface — a 7th showcase section demonstrating the grouped combobox forced into its drill-in state ("← Назад" header visible on load), the flat select with its in-panel search box and checkmark, and D-13's static empty/loading panel states — all with local static demo data, no live API calls, wired into `ShowcasePage.svelte` after `TableSection`.**

## Performance

- **Duration:** ~15 min
- **Completed:** 2026-07-19
- **Tasks:** 2 completed
- **Files modified:** 2 (1 created, 1 modified)

## Accomplishments

- `DropdownSection.svelte` implements 4 `.variant-block`s mirroring `TabsSection.svelte`'s structural template (`<section>` → `h2` → `.variant-block` → `.variant-label` → component):
  1. **"Комбобокс с группами (drill-in)"** — `Dropdown variant="combobox"` with 2 demo groups (one expandable with 3 demo members, one not), forced open and drilled into its expandable group on mount via a synthetic focus + option click, so the "← Назад" header is visible without any reviewer interaction (SC #4).
  2. **"Плоский селект"** — `Dropdown variant="select" flat={true}` with 3 demo options, one pre-marked `selected`, forced open on mount via a synthetic trigger click, showing the in-panel search box and the checkmark on the selected option (SC #3).
  3. **"Пустое состояние"** — `Dropdown variant="combobox"` with `groups=[]`, `loading={false}`, forced open on mount, statically showing "Ничего не найдено" (D-13).
  4. **"Загрузка"** — `Dropdown variant="combobox"` with `groups=[]`, `loading={true}`, forced open on mount, statically showing "Загрузка…" + `Spinner` (D-13).
- All demo data is local, minimal (`DemoGroup`/`DemoMember` interfaces), no `DeviceGroup`/`DeviceDto` import — proves the primitive works with any caller shape, per the plan's explicit instruction.
- Since `Dropdown.svelte`'s open/viewMode/drill-in state is entirely internal (no bindable props, Plan 25-02 D-02), each demo block's forced-open/drilled-in visual is produced by a real `focus()`/`click()` sequence in `onMount` — the field element's `aria-controls` attribute (stable, always present) is used to locate the portaled panel in `document.body`, then the first rendered option is `.click()`ed to trigger the same `drillInto()` code path a mouse click would.
- `ShowcasePage.svelte` wired with `DropdownSection` as the 7th `showcase-block`, appended after `TableSection`; the existing 6 sections' order is unchanged.
- Zero hex/rgba literals in the `<style>` block (`check-tokens.mjs` passes); Russian demo copy per D-13's canonical labels.

## Task Commits

Each task was committed atomically:

1. **Task 1: Build DropdownSection.svelte with static demo data** - `a63cf28` (feat)
2. **Task 2: Wire DropdownSection into ShowcasePage.svelte** - `d4aa264` (feat)

**Plan metadata:** committed as part of this summary commit

## Files Created/Modified

- `ui/src/features/showcase/sections/DropdownSection.svelte` (242 lines) — new showcase section, 4 static demo blocks, `onMount` forced-open/drill-in sequence, zero API calls.
- `ui/src/features/showcase/ShowcasePage.svelte` — added `import DropdownSection` + `<section class="showcase-block"><DropdownSection /></section>` as the 7th block, after `TableSection`.

## Decisions Made

See `key-decisions` in frontmatter: chose the programmatic focus/click simulation approach (confirmed via reading `Dropdown.svelte`'s actual prop surface, which has zero bindable open/viewMode state); local minimal demo types instead of importing device DTOs; empty/loading demos both use the combobox variant with `groups=[]` for the simplest single-trigger path to those panel states; "← Назад" literal text intentionally not duplicated in the showcase file (lives only in `Dropdown.svelte`), matching the plan's grep-OR acceptance clause.

## Deviations from Plan

None — plan executed exactly as written, including selecting the programmatic-simulation branch the plan's Task 1 action explicitly anticipated as the likely outcome given Dropdown's final prop surface.

## Known Stubs

None. `DropdownSection.svelte` is a showcase-only component by design (static demo data, no live data source) — this matches every other section in `ShowcasePage.svelte` (`ButtonsSection`, `FieldsSection`, `BadgeSection`, `TabsSection`, `ModalSection`, `TableSection`) and is explicitly the plan's scope, not a stub standing in for missing wiring.

## Issues Encountered

None.

## User Setup Required

None — no new dependencies, no environment variables, no manual steps.

## Threat Flags

None. `DropdownSection.svelte` and the `ShowcasePage.svelte` diff are presentation-only: zero `{@html}` usage, zero new npm dependencies, zero new data-fetching or API/Tauri-command surface, all demo strings are hardcoded literal Russian text rendered via Svelte's default-escaped interpolation. No new trust boundary introduced beyond what the plan's `<threat_model>` already accepted (T-25-06-01, T-25-06-02, T-25-06-SC — all `accept`).

## Next Steps

- Plan 25-07: `ActFormItemsTable.svelte` pilot — replaces the per-row device picker with `Dropdown`, the most portal/scroll-risk-relevant consumer (SC #5), closing CMP-07 end-to-end.
- Visual/interactive confirmation of both themes still needs a live pass via the `/showcase` route (admin role, `pnpm --dir ui build` required before LAN-browser UAT) — automated gates (`lint`, `svelte-check`, `check-tokens.mjs`, `build`) all pass, but pixel-fidelity against `Dropdown.dc` is a human-eyes check per this phase's `Verification Notes`.

## Self-Check: PASSED

- FOUND: ui/src/features/showcase/sections/DropdownSection.svelte
- FOUND: ui/src/features/showcase/ShowcasePage.svelte (modified)
- FOUND: a63cf28 (Task 1 commit)
- FOUND: d4aa264 (Task 2 commit)

---
*Phase: 25-dropdown*
*Completed: 2026-07-19*

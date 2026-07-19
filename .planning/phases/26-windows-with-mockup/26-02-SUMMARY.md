---
phase: 26-windows-with-mockup
plan: 02
subsystem: ui
tags: [svelte, scss, sidebar, theme-switcher, design-tokens]

# Dependency graph
requires:
  - phase: 26-windows-with-mockup (plan 01)
    provides: "--tr-bg background inversion target, --sidebar-width 236px token, and other adaptive-shell tokens"
provides:
  - "Sidebar.svelte with new logo header (Trackly wordmark + accent square), inverted --tr-bg background, restyled nav-list/nav-link spacing, box-shadow-based active nav state, and restyled footer/user-block/theme-row"
  - "ThemeSwitcher.svelte restyled as a sunken-track segmented control (Светлая · Системная · Тёмная order) with shadowed active segment and 0.12s transitions"
affects: [26-03, 26-04, 26-05, 26-06, 26-07, 26-08]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Sidebar active nav state uses box-shadow:inset 3px 0 0 var(--tr-accent) + --tr-accent-soft/--tr-accent-text instead of border-left + color-mix"
    - "Segmented controls (ThemeSwitcher) reuse the Tabs.svelte .tabs-segmented sunken-track visual language: padding+gap track, shadowed active segment"

key-files:
  created: []
  modified:
    - ui/src/features/layout/Sidebar.svelte
    - ui/src/lib/components/ThemeSwitcher.svelte

key-decisions:
  - "Kept :global(.nav-link.is-active) selector wrapper unchanged (scoped-component :global(), not the plain-.scss anti-pattern) — only its declarations changed"
  - "Left .logout-btn and getVisibleItems()/authStore wiring untouched per D-08 (macet absence != removal signal)"
  - "ThemeSwitcher options array reordered to Светлая/Системная/Тёмная; key/label/ariaLabel values unchanged"

patterns-established:
  - "Segmented-control restyle recipe (sunken background + padded gap track + shadowed active segment) is now shared between Tabs.svelte and ThemeSwitcher.svelte — future segmented controls should follow the same token set"

requirements-completed: [WIN-01, WIN-02]

# Metrics
duration: 8min
completed: 2026-07-20
---

# Phase 26 Plan 02: Sidebar & ThemeSwitcher Restyle Summary

**Sidebar gets a new logo header and inverted `--tr-bg` background with box-shadow-based active nav state; ThemeSwitcher becomes a sunken-track segmented control in Светлая·Системная·Тёмная order — both purely visual, logout/role-nav/theme logic untouched.**

## Performance

- **Duration:** 8 min
- **Started:** 2026-07-19T23:16:00Z
- **Completed:** 2026-07-19T23:24:01Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- Sidebar now has a structural logo header (`.sidebar-logo` with `.logo-mark` accent square + "Trackly" wordmark) as the first child of the nav root
- Sidebar background inverted from `--tr-surface` to `--tr-bg` (D-06), with nav/footer spacing values realigned to the mockup (`Д:45`, `Д:47`, `Д:280`)
- Active nav-link state migrated from `border-left` + `color-mix` to `box-shadow: inset 3px 0 0 var(--tr-accent)` + `--tr-accent-soft`/`--tr-accent-text` + weight 600
- Theme-row label copy changed from "Тема" to "Оформление"; logout button and `getVisibleItems()`/`authStore` wiring left untouched
- ThemeSwitcher restyled to a sunken-track segmented control (padding:2px, gap:2px, `--tr-surface-sunken` background, radius 8px), divider borders removed, active segment gets `--tr-elev-1` box-shadow, transitions changed from `none` to `background 0.12s, color 0.12s`
- ThemeSwitcher option order changed to Светлая · Системная · Тёмная (D-08)

## Task Commits

Each task was committed atomically:

1. **Task 1: Sidebar.svelte — logo header, background inversion, nav/footer restyle** - `dd24abd` (feat)
2. **Task 2: ThemeSwitcher.svelte — sunken-track restyle + button reorder** - `ef0b77f` (feat)

**Plan metadata:** (recorded below after this summary commit)

## Files Created/Modified
- `ui/src/features/layout/Sidebar.svelte` - new logo header, inverted background, restyled nav/footer/active-state, "Оформление" copy
- `ui/src/lib/components/ThemeSwitcher.svelte` - sunken-track segmented control, reordered options, updated transitions

## Decisions Made
- None beyond what's specified in the plan — followed the plan's explicit CSS values and selector-preservation constraints exactly.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- Sidebar and ThemeSwitcher are now visually aligned with the mockup and ready for Wave 2 plans (Dashboard/Devices etc.) that will be verified against the same design tokens.
- No blockers. `pnpm --dir ui run svelte-check` passes with 0 errors (pre-existing unrelated warnings only).

---
*Phase: 26-windows-with-mockup*
*Completed: 2026-07-20*

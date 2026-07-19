---
phase: 26-windows-with-mockup
plan: 01
subsystem: ui
tags: [svelte5, runes, scss, layout, accessibility, responsive]

requires:
  - phase: 25-dropdown
    provides: table/dropdown primitives on the --tr-* token system
provides:
  - "ui/src/styles/_breakpoints.scss — shared SCSS breakpoint variables ($bp-xl/lg/md/sm)"
  - "ui/src/features/layout/layout-state.svelte.ts — sidebarNav rune-store (open/openNav/closeNav)"
  - "ui/src/lib/components/PageHeader.svelte — reusable page-header primitive (title/variant/actions/burger)"
  - "ui/src/features/layout/Layout.svelte — responsive shell: sticky sidebar >=1024px, drawer+backdrop <1024px"
affects: [26-02, 26-03, 26-04, 27, 28, 29]

tech-stack:
  added: []
  patterns:
    - "CSS-only breakpoint hiding via SCSS $bp-* + @media (min-width:) — no matchMedia/JS branching for pure visibility toggles"
    - "One sanctioned matchMedia exception for `inert` gating (DOM attribute, not stylable via CSS)"
    - "Drawer/backdrop mirrors Modal.svelte's focus-trap-entry + mousedown/mouseup dismiss pattern"

key-files:
  created:
    - ui/src/styles/_breakpoints.scss
    - ui/src/features/layout/layout-state.svelte.ts
    - ui/src/lib/components/PageHeader.svelte
  modified:
    - ui/src/features/layout/Layout.svelte
    - ui/src/styles/_tokens.scss

key-decisions:
  - "--sidebar-width changed 240px -> 236px by VALUE (layout constant, not --tr-*, deliberately not scanned by check-tokens.mjs closed-world gate)"
  - "PageHeader owns the burger button internally (not Layout) so any future page consuming PageHeader gets the mobile toggle for free"
  - "Layout.svelte auto-closes drawer via a single $effect on svelte-spa-router's router.location — covers both nav-link clicks and any programmatic navigation, so Sidebar.svelte (owned by parallel Plan 26-02) needed zero changes"

patterns-established:
  - "Pattern: PageHeader(title, variant: fixed|wrap, actions?: Snippet) is the canonical page-header primitive for all future window plans (26-04, 26-06, 27-29)"
  - "Pattern: layout-state.svelte.ts rune-store mirrors theme.svelte.ts's exact shape ($state object + plain mutator functions, no class)"

requirements-completed: [WIN-01, WIN-02, WIN-12]

duration: 8min
completed: 2026-07-19
---

# Phase 26 Plan 01: Adaptive shell contracts (breakpoints, layout-state, PageHeader, Layout) Summary

**Introduced the reusable responsive-shell layer for Phase 26+: SCSS breakpoint variables, a `sidebarNav` rune-store, a `PageHeader` primitive with a built-in CSS-hidden-above-1024px burger button, and an adaptive `Layout.svelte` that is a sticky in-flow sidebar at >=1024px and a focus-trapped, Escape/backdrop/route-closing drawer below it.**

## Performance

- **Duration:** 8 min
- **Started:** 2026-07-19T23:10:19Z
- **Completed:** 2026-07-19T23:18:00Z
- **Tasks:** 3
- **Files modified:** 5 (3 created, 2 edited)

## Accomplishments
- `_breakpoints.scss` — the single source of truth for `$bp-xl/lg/md/sm`, consumed via `@use ... as bp;` in both `PageHeader.svelte` and `Layout.svelte`
- `layout-state.svelte.ts` — minimal `sidebarNav` rune-store mirroring `theme.svelte.ts`'s exact shape, no persistence (session-only, per UI-SPEC §6.3)
- `PageHeader.svelte` — shared header primitive (`title`, `variant: 'fixed'|'wrap'`, optional `actions` Snippet) with an inline SVG burger button that is CSS-hidden at `>=1024px` — zero JS branching for the visibility toggle
- `Layout.svelte` — sticky sidebar unchanged at `>=1024px`; below that, a `position:fixed; transform:translateX` drawer with backdrop, Escape-to-close, route-change auto-close, and a focus-trap-entry/restore pattern copied from `Modal.svelte`
- `.content` no longer owns its own padding/background (D-07) — that responsibility moves to each page's own body/PageHeader going forward
- `--sidebar-width` moved 240px → 236px by value only; `check-tokens.mjs` closed-world gate still passes (layout constant, not `--tr-*`)

## Task Commits

Each task was committed atomically:

1. **Task 1: Breakpoint contracts + layout-state store + sidebar-width value fix** - `18ee3c7` (feat)
2. **Task 2: PageHeader.svelte — shared page-header primitive with burger button** - `017ba39` (feat)
3. **Task 3: Layout.svelte — adaptive shell (sticky >=1024px, drawer+backdrop <1024px)** - `6ab3c4e` (feat)
4. **Follow-up fix: nav-backdrop a11y role** - `929631d` (fix, part of Task 3 scope — see Deviations)

**Plan metadata:** pending (docs: complete plan)

## Files Created/Modified
- `ui/src/styles/_breakpoints.scss` - `$bp-xl`/`$bp-lg`/`$bp-md`/`$bp-sm` SCSS variables
- `ui/src/features/layout/layout-state.svelte.ts` - `sidebarNav` rune-store + `openNav()`/`closeNav()`
- `ui/src/lib/components/PageHeader.svelte` - shared page-header primitive with burger button
- `ui/src/features/layout/Layout.svelte` - adaptive responsive shell (sticky/drawer), `.content` padding/background removed
- `ui/src/styles/_tokens.scss` - `--sidebar-width: 240px` → `236px`

## Decisions Made
- `--sidebar-width` migrated by value, not name, per plan spec — confirmed `check-tokens.mjs` does not flag layout constants
- Burger button lives inside `PageHeader`, not `Layout` — every future page that adopts `PageHeader` automatically gets the mobile toggle without additional wiring
- Route-change auto-close implemented as a single `$effect` reading `router.location` in `Layout.svelte`, avoiding any change to `Sidebar.svelte` (owned by the parallel Plan 26-02, no file overlap)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Added `role="presentation"` to `.nav-backdrop`**
- **Found during:** Task 3 (Layout.svelte adaptive shell) — post-task verification pass with `svelte-check`
- **Issue:** The new backdrop `<div>` has `onmousedown`/`onmouseup` handlers but no ARIA role, tripping `a11y_no_static_element_interactions` (a real regression not present in the pre-plan warning baseline)
- **Fix:** Added `role="presentation"` — the backdrop is a dismiss-on-click scrim, not an interactive control (Escape already covers keyboard dismissal), matching the semantics `role="presentation"` communicates
- **Files modified:** `ui/src/features/layout/Layout.svelte`
- **Verification:** `pnpm --dir ui run svelte-check` warning count returned to the pre-existing baseline (48 warnings, 0 errors); `pnpm --dir ui build` still exits 0
- **Committed in:** `929631d`

---

**Total deviations:** 1 auto-fixed (1 missing accessibility attribute)
**Impact on plan:** Necessary correctness fix for the new drawer/backdrop markup introduced by this plan. No scope creep — no other files touched.

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Shell contracts (`_breakpoints.scss`, `layout-state.svelte.ts`, `PageHeader.svelte`, `Layout.svelte`) are in place and ready for Plan 26-02 (parallel, Sidebar/sidebar-config — no file overlap) and Plan 26-04/26-06 (Dashboard/Devices window rebuilds, Wave 2) to consume without further shell work.
- `EmployeeLayout.svelte` untouched (D-09) — confirmed via `git status`/`git diff --name-only`, no changes to that file.
- Manual verification item still open per plan `<verification>` #4 (resize through 1024px with drawer open, confirm desktop sidebar remains keyboard-focusable after crossing back above 1024px) — deferred to end-of-phase human-verify per `human_verify_mode: end-of-phase` config; not blocking for Wave 2 plans since the automated `inert` guard (grep-verified) already encodes the correct behavior.

---
*Phase: 26-windows-with-mockup*
*Completed: 2026-07-19*

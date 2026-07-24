---
phase: 29-login-and-employee-shell
plan: 04
subsystem: ui
tags: [svelte, svelte5-runes, design-system, employee-shell, uat]

# Dependency graph
requires:
  - phase: 29-login-and-employee-shell
    provides: "29-02/29-03: AuthShell.svelte + Button.svelte primitives applied to all 4 auth screens"
provides:
  - "EmployeeLayout.svelte header sizing fully driven by --header-height token (zero hardcoded fallback)"
  - "EmployeeLayout.svelte content area fills to viewport bottom with page-owned padding, matching admin Layout.svelte pattern (D-04 parity)"
  - "EmployeeLayout.svelte header ThemeSwitcher no longer starves user-name into single-letter ellipsis"
  - "Phase 29 closed: human visual UAT approved across all 5 surfaces (4 auth screens + EmployeeLayout/RequestsPage), both themes"
affects: []

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Definite-height shell + overflow:hidden + min-height:0 scroll pane (mirrors admin Layout.svelte's .app-layout/.content) is the required pattern for any full-viewport shell wrapping a page that supplies its own padding — min-height:100vh on the shell breaks percentage-height children"
    - "Shared primitives with layout-context-dependent CSS (e.g. ThemeSwitcher's width:100%, correct in a vertical Sidebar) must be wrapped in a bounded slot (flex-shrink:0; width:max-content) by the consumer rather than modified at the source, to avoid breaking the other consumer"

key-files:
  created: []
  modified:
    - ui/src/features/layout/EmployeeLayout.svelte

key-decisions:
  - "Task 1's two single-line token edits (var(--header-height, 56px) -> var(--header-height); calc(100vh - 56px) -> calc(100vh - var(--header-height))) matched the plan exactly, no deviation"
  - "UAT gap-closure fix A (fill-to-bottom): rather than add padding tweaks to EmployeeLayout, adopted the admin Layout.svelte's definite-height/overflow:hidden/min-height:0 pattern and removed the shell's own padding entirely, since the employee shell only ever renders RequestsPage, which already supplies its own .page-content padding (D-04 parity requirement)"
  - "UAT gap-closure fix B (user-name truncation): fixed at the EmployeeLayout consumer (wrapping <ThemeSwitcher /> in a bounded .theme-switcher-slot) instead of touching the shared ThemeSwitcher.svelte, because Sidebar.svelte depends on ThemeSwitcher's width:100% rule for its own vertical layout"

patterns-established:
  - "Full-viewport app shell pattern: shell = height:100vh + overflow:hidden; scroll pane = flex:1 + min-height:0 + overflow:auto + NO shell-owned padding when the page component supplies its own"

requirements-completed: [WIN-11]

# Metrics
duration: ~15min active work (spread across two sessions separated by a blocking human-verify checkpoint pause)
completed: 2026-07-24
---

# Phase 29 Plan 04: EmployeeLayout header-token consistency pass + phase-closing UAT Summary

**EmployeeLayout.svelte's header/content sizing made fully token-clean (--header-height, no fallback), then two UAT-discovered layout bugs (content not filling to viewport bottom, header user-name collapsing to one letter) fixed and re-verified by the user in both themes, closing WIN-11 and Phase 29.**

## Performance

- **Duration:** ~15 min active work; session paused at the Task 3 blocking checkpoint awaiting human re-verification, then resumed and finalized
- **Started:** 2026-07-23T23:5x (Task 1 commit 8d23dec)
- **Completed:** 2026-07-24 (UAT approved, plan finalized)
- **Tasks:** 3 (2 automated + 1 checkpoint, plus 2 UAT gap-closure fix commits applied during the checkpoint pause)
- **Files modified:** 1 (`ui/src/features/layout/EmployeeLayout.svelte`, across 3 commits)

## Accomplishments
- Task 1: removed the last hardcoded `56px` fallback/magic-number from `EmployeeLayout.svelte` — header `height` and content `min-height` now derive solely from `--header-height`, matching `PageHeader.svelte`'s convention
- Task 2: all 4 automated gates (`lint`, `svelte-check`, `build`, `check-tokens`) green across the whole phase's touched files (4 auth screens + EmployeeLayout), `ui/dist` rebuilt, no accidental `bindings.ts` drift
- Task 3 (checkpoint) round 1: human UAT surfaced 2 real layout bugs not caught by automated gates (list/detail content not filling to the bottom of the viewport; header user-name collapsing to a single letter + ellipsis)
- UAT gap-closure fix A: `.employee-shell` switched from `min-height:100vh` to a definite `height:100vh; overflow:hidden`, and `.employee-content` switched from a shell-owned `padding` + `calc()` min-height to `min-height:0` with no padding — mirrors admin `Layout.svelte`'s `.app-layout`/`.content` pattern so `RequestsPage`'s own `.page-content` padding and `height:100%` resolve and the list/detail cards fill to the bottom with correct spacing, matching the rest of the app
- UAT gap-closure fix B: wrapped `<ThemeSwitcher />` in a bounded `.theme-switcher-slot` (`flex-shrink:0; width:max-content`) so its `width:100%` rule (correct in the vertical `Sidebar`) no longer hogs the horizontal header row; added `flex-shrink:0` to `.user-name` so it keeps its `max-width:200px` ellipsis behavior instead of collapsing to one character
- Task 3 (checkpoint) round 2: user re-verified both fixes live in the running app across both light and dark themes — **approved**

## Task Commits

Each task was committed atomically:

1. **Task 1: EmployeeLayout.svelte header-chrome consistency pass** - `8d23dec` (style)
2. **Task 2: Final automated gates across the whole phase** - verification-only, no commit (all 4 gates green)
3. **UAT gap-closure fix A: fill employee-shell content to viewport with page-owned padding (D-04 parity)** - `61feaa7` (fix)
4. **UAT gap-closure fix B: stop ThemeSwitcher from starving the employee header user-name** - `e3bee15` (fix)
5. **Task 3: human-verify checkpoint** - approved by user after fixes A+B (no code commit; gate-only)

**Plan metadata:** (this commit)

_Note: Task 3 is a blocking `checkpoint:human-verify` gate, not a code-producing task; the two UAT gap-closure fixes were applied during the checkpoint pause in response to the first round of human feedback, then re-verified and approved in a second round._

## Files Created/Modified
- `ui/src/features/layout/EmployeeLayout.svelte` - Header/content sizing now fully token-driven (`--header-height`, no fallback); shell uses definite `height:100vh`+`overflow:hidden` with a `min-height:0` scroll pane (no shell-owned padding, page supplies its own); header `ThemeSwitcher` wrapped in a bounded slot so it no longer starves `.user-name`

## Decisions Made
- See `key-decisions` in frontmatter: token-fallback removal followed the plan exactly (no deviation); the fill-to-bottom fix adopted the admin shell's definite-height pattern rather than ad-hoc padding tweaks; the user-name fix was scoped to the EmployeeLayout consumer rather than the shared `ThemeSwitcher.svelte` to avoid breaking `Sidebar.svelte`

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Employee-shell content not filling to viewport bottom**
- **Found during:** Task 3 (human visual UAT, round 1)
- **Issue:** `.employee-shell` used `min-height:100vh` (not a definite height) combined with `.employee-content`'s own `padding` + `calc(100vh - var(--header-height))` min-height. `RequestsPage`'s `.requests-page { height:100% }` could not resolve against a non-definite ancestor height, so the list/detail cards stopped short of the viewport bottom instead of filling it with consistent padding, unlike every other window in the app (which uses the admin `Layout.svelte` definite-height pattern).
- **Fix:** Changed `.employee-shell` to `height:100vh; overflow:hidden` and `.employee-content` to `flex:1; min-height:0; overflow:auto` with no shell-owned padding — mirrors `Layout.svelte`'s `.app-layout`/`.content`. `RequestsPage`'s own `.page-content { padding: xl 2xl }` now supplies the padding, and its `height:100%` resolves correctly.
- **Files modified:** `ui/src/features/layout/EmployeeLayout.svelte`
- **Verification:** `pnpm --dir ui lint`/`svelte-check`/`build` all green; user re-verified live in browser, both themes — approved
- **Committed in:** `61feaa7`

**2. [Rule 1 - Bug] Header user-name collapsing to a single letter + ellipsis**
- **Found during:** Task 3 (human visual UAT, round 1)
- **Issue:** `ThemeSwitcher.svelte` is styled `width:100%` (correct for its primary consumer, the vertical `Sidebar`). Rendered directly as a flex sibling in `EmployeeLayout`'s horizontal header row, that `width:100%` acted as `flex-basis:100%`, claiming the entire row and collapsing the only `overflow:hidden` sibling (`.user-name`) down to "A…".
- **Fix:** Wrapped `<ThemeSwitcher />` in `<div class="theme-switcher-slot">` styled `flex-shrink:0; width:max-content`, and added `flex-shrink:0` to `.user-name` so it keeps its intended `max-width:200px` ellipsis behavior for genuinely long names instead of collapsing unconditionally. Did not modify the shared `ThemeSwitcher.svelte` — `Sidebar.svelte` depends on its `width:100%` rule.
- **Files modified:** `ui/src/features/layout/EmployeeLayout.svelte`
- **Verification:** `pnpm --dir ui lint`/`svelte-check`/`build` all green; user re-verified live in browser, both themes — approved
- **Committed in:** `e3bee15`

---

**Total deviations:** 2 auto-fixed (both Rule 1 - bugs surfaced by human visual UAT, not caught by automated gates since they were CSS layout regressions rather than lint/type/build errors)
**Impact on plan:** Both fixes were markup+CSS-only, scoped exactly to `EmployeeLayout.svelte`, with zero script/logic changes (`onMount`/`connectWs`/`onWsEvent`/`handleEmployeeWsEvent`/`logout` byte-identical throughout). No scope creep — both are required for the plan's own D-04 parity and WIN-11 acceptance criteria.

## Issues Encountered

The Task 3 checkpoint required two rounds: round 1 surfaced the two layout bugs above (visual issues that automated gates cannot catch), round 2 (after fixes A+B) was approved. This is expected checkpoint behavior for a `gate="blocking"` human-verify task, not a plan failure.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Phase 29 (login-and-employee-shell) is complete: all 4 auth screens (`LoginPage`, `FirstRunWizard`, `PendingScreen`, `BlockedScreen`) and `EmployeeLayout` are on shared design-system primitives (`AuthShell`, `Button`, token-driven header sizing), and `EmployeeLayout`'s content-fill + header-name bugs are resolved with human sign-off in both themes. WIN-11 satisfied. No blockers for closing the phase.

---
*Phase: 29-login-and-employee-shell*
*Completed: 2026-07-24*

## Self-Check: PASSED

`ui/src/features/layout/EmployeeLayout.svelte` verified present on disk; all three task commit hashes (`8d23dec`, `61feaa7`, `e3bee15`) verified in `git log --oneline --all`.

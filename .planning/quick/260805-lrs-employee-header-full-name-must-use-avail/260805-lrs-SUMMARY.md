---
phase: 260805-lrs
plan: 01
subsystem: ui
tags: [svelte, scss, flexbox, employee-layout]

# Dependency graph
requires: []
provides:
  - "EmployeeLayout.svelte header name (`.user-name`) uses available header width instead of a fixed 200px ceiling"
affects: [employee-facing UI, EmployeeLayout header]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Flex-shrink propagation: to let a nested flex item actually shrink/ellipsis, both the flex container AND the item need `min-width: 0` — the browser default `min-width: auto` otherwise blocks shrinking below content size at every level in the chain."

key-files:
  created: []
  modified:
    - ui/src/features/layout/EmployeeLayout.svelte

key-decisions:
  - "Removed max-width: 200px and flex-shrink: 0 from .user-name entirely rather than raising the pixel ceiling — any fixed ceiling reproduces the same class of bug at a different width."
  - "Made .user-role explicitly flex-shrink: 0 + white-space: nowrap so it (not .user-name) is guaranteed to never be the element that yields space under pressure."

requirements-completed: [LRS-01]

# Metrics
duration: 6min
completed: 2026-08-05
---

# Quick 260805-lrs: Employee header full name uses available width Summary

**Removed the hardcoded 200px `max-width` + `flex-shrink: 0` pair on `.user-name` in `EmployeeLayout.svelte` so the employee-header name fills available header width and only ellipsizes under genuine space pressure, while `.user-role`, the theme switcher, and "Выйти" stay fixed-size.**

## Performance

- **Duration:** 6 min
- **Started:** 2026-08-05 (task execution)
- **Completed:** 2026-08-05
- **Tasks:** 1
- **Files modified:** 1

## Accomplishments
- `.user-name` no longer has a fixed 200px ceiling — it grows to fill the header row and only shrinks (with ellipsis) when `.employee-header-actions` genuinely runs out of horizontal room.
- Shrink pressure now correctly propagates from the flex header row down to `.user-name`: `.employee-header-actions` and `.user-name` both carry `min-width: 0`, overriding the browser's default `min-width: auto` which otherwise blocks shrinking below content size.
- `.user-role` ("Сотрудник") is now explicitly `flex-shrink: 0; white-space: nowrap;`, so it — not the name — is guaranteed to hold its size and never wrap when the header is squeezed. The theme switcher (`flex-shrink: 0` already present) and the "Выйти" button (unstyled `Button` component, out of scope) were unaffected.

## Task Commits

Each task was committed atomically:

1. **Task 1: Let the employee-header name grow to available width, shrink only under real pressure** - `95614e4` (fix)

**Plan metadata:** committed separately by orchestrator (SUMMARY.md/STATE.md not committed by this agent per instructions)

## Files Created/Modified
- `ui/src/features/layout/EmployeeLayout.svelte` - `.employee-header-actions` gained `min-width: 0`; `.user-name` lost `max-width: 200px` and `flex-shrink: 0` (now `flex-shrink: 1; min-width: 0;`, keeping `white-space: nowrap; overflow: hidden; text-overflow: ellipsis;`); `.user-role` gained `flex-shrink: 0; white-space: nowrap;`.

## Decisions Made
- Removed the max-width entirely rather than picking a larger fixed pixel value — a bigger fixed ceiling would just move the bug to a wider breakpoint instead of fixing the underlying "always clipped regardless of available space" defect.
- Kept `flex-shrink: 1` explicit on `.user-name` rather than omitting the declaration, for readability/intent-clarity at the call site (the plan allowed either).

## Deviations from Plan

None - plan executed exactly as written. All three CSS edits (`.employee-header-actions`, `.user-name`, `.user-role`) applied exactly as specified in the plan's `<action>` block; no other selectors or files touched.

## Issues Encountered

None.

## Verification Performed

**Structurally proven (automated, all passed):**
1. `verify.sh` structural gate — confirms: `.employee-header-actions` has `min-width: 0`; `.user-name` has no `max-width: 200px`, no `flex-shrink: 0`, has `min-width: 0`, and retains `white-space: nowrap; overflow: hidden; text-overflow: ellipsis;`; `.user-role` has `flex-shrink: 0; white-space: nowrap;`; zero remaining `max-width: 200px` occurrences inside `EmployeeLayout.svelte`'s own style block. Output: `OK_EMPLOYEE_HEADER_NAME_SHRINK_GATES_PASS`.
2. Confirmed the three unrelated `max-width: 200px` occurrences in `ActNumberField.svelte`, `DeviceListRow.svelte`, `DeviceImportCsvModal.svelte` are untouched (informational check in verify.sh: `UNRELATED_MAX_WIDTH_FILES_STILL_MATCHING=3`).
3. `pnpm --dir ui svelte-check` — 0 errors (48 pre-existing warnings in unrelated files, not introduced by this change).
4. `pnpm --dir ui lint` (eslint + prettier + token/contrast/focus-outline/CSP-hash gates) — all clean.
5. `pnpm --dir ui build` — succeeded.

**NOT verified (requires a live browser, per plan's `<verification_reality>` — flagging as pending follow-up UAT):**
- Wide viewport (~1200px+): visual confirmation that the full name (e.g. "Красноперов Анастасия Дмитриевна") renders with no ellipsis.
- Narrow viewport (~500-600px): visual confirmation that only `.user-name` shrinks/ellipsizes while `.user-role` ("Сотрудник"), the theme switcher, and "Выйти" keep their size and do not wrap.
- These two checks require opening the employee view in an actual browser (Tauri webview or LAN browser session) at both widths and cannot be proven by any of the automated commands above. The CSS is now structurally correct per the plan's flexbox model (min-width: 0 propagation + flex-shrink: 1 on `.user-name` only), but rendered behavior should get a manual look before considering this fully closed.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- Fix is self-contained to `EmployeeLayout.svelte`; no other files or phases depend on this change.
- Recommend a quick manual UAT pass (wide + narrow viewport) on the employee header next time the app is opened, per the pending verification above.

---
*Phase: 260805-lrs*
*Completed: 2026-08-05*

## Self-Check: PASSED

- FOUND: ui/src/features/layout/EmployeeLayout.svelte
- FOUND: .planning/quick/260805-lrs-employee-header-full-name-must-use-avail/260805-lrs-SUMMARY.md
- FOUND commit: 95614e4

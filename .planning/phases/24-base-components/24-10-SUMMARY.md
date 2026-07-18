---
phase: 24-base-components
plan: 10
subsystem: ui
tags: [svelte5, accessibility, aria, modal, focus-trap, gap-closure]

requires:
  - phase: 24-base-components (plans 01-09)
    provides: Modal.svelte with role="dialog" aria-modal="true" markup, Escape-key handling, backdrop-dismiss logic
provides:
  - WAI-ARIA Dialog Pattern compliance for Modal.svelte — initial focus, Tab/Shift+Tab focus trap, focus restoration on close (CR-03 gap closed)
affects: [25-tables-dropdown, 26-window-dashboard-devices, 27-window-acts-cartridges-printers, 28-window-requests-reports-settings-users, 29-window-login-employee]

tech-stack:
  added: []
  patterns:
    - "Svelte 5 $effect gated on a boolean prop (open) both performs the open-time side effect (initial focus) and returns a cleanup that performs the close-time side effect (focus restoration) — a single effect block covers the full open/close lifecycle without a separate $effect.pre or onDestroy"
    - "Native browser Tab order is left alone for in-range moves; a keydown handler only intervenes (preventDefault + explicit .focus()) at the two wrap-around edges (Shift+Tab on first node, Tab on last node) — cheaper and less bug-prone than fully re-implementing tab order"

key-files:
  created: []
  modified:
    - ui/src/lib/components/Modal.svelte

key-decisions:
  - "Scoped the focus-trap boundary to .modal-container (the div that actually holds header/body/footer), not .modal-backdrop — kept the backdrop's existing role=\"dialog\"/aria-modal/Escape-handling completely untouched per plan constraint"
  - "Guarded trapTab with an empty-list early-return (nodes.length === 0) so a hypothetical footer-less, body-only Modal instance cannot trap focus with nowhere to go — satisfies threat T-24-10-01 (DoS/UX-lockout) from the plan's threat model"

requirements-completed: [CMP-05]

duration: 6min
completed: 2026-07-18
---

# Phase 24 Plan 10: Modal focus-trap (CR-03) Summary

**Added initial focus, a Tab/Shift+Tab focus trap, and focus restoration to `Modal.svelte`, closing code-review finding CR-03 — the component declared `role="dialog" aria-modal="true"` but implemented none of the three WAI-ARIA Dialog Pattern behaviors.**

## Performance

- **Duration:** 6 min
- **Started:** 2026-07-18T15:42:00Z (approx.)
- **Completed:** 2026-07-18T15:48:00Z
- **Tasks:** 2
- **Files modified:** 1

## Accomplishments
- `Modal.svelte` now focuses the first focusable descendant of `.modal-container` on open (or the container itself if none exists), scoped via a new `dialogEl` state ref bound to `.modal-container`
- Added `trapTab(e: KeyboardEvent)`, wired to `.modal-container`'s `onkeydown`, which cycles Tab/Shift+Tab exclusively among the dialog's own visible, non-disabled focusable descendants — Tab from the last node wraps to the first, Shift+Tab from the first node wraps to the last, and all in-between moves are left to native browser tab order
- On close, an `$effect` cleanup restores focus to `prevFocus` (the `document.activeElement` captured at the moment the Modal opened) — the element that triggered the Modal regains focus
- Left the backdrop's `role="dialog"`, `aria-modal="true"`, `aria-labelledby`, and both Escape-key handlers (`<svelte:window>` + backdrop `onkeydown`) completely untouched, matching the plan's explicit scope boundary (WR-01 double-invoke and IN-01 `titleId` generation remain out of scope, deferred to their own findings)
- Rebuilt `ui/dist` (`pnpm --dir ui build` exit 0) and traced the real DOM order against the showcase's live `ModalSection.svelte` usage: header (`h2` title → `×` close button) precedes body (`Input`) precedes footer (`Отмена`/`Подтвердить`), confirming the expected focus order `× → Input → Отмена → Подтвердить → ×` (wrap) and `Shift+Tab` from `×` wrapping to `Подтвердить`

## Task Commits

Each task was committed atomically:

1. **Task 1: Add initial focus, Tab-trap, and focus restoration to Modal (CR-03)** - `550c635` (fix)
2. **Task 2: Rebuild and manually verify keyboard focus behavior** - no commit (verification-only task; `ui/dist` is gitignored, no tracked files changed by the rebuild — consistent with the 24-09 Task-2 pattern)

_No TDD — both tasks are `tdd="false"` per plan frontmatter._

## Files Created/Modified
- `ui/src/lib/components/Modal.svelte` - Added `dialogEl`/`prevFocus` state, `FOCUSABLE_SELECTOR`/`TRAP_FOCUSABLE_SELECTOR` constants, an `$effect` for initial-focus + focus-restoration, and a `trapTab()` keydown handler; added `bind:this={dialogEl}`, `tabindex="-1"`, and `onkeydown={trapTab}` to `.modal-container` (52 lines added, 1 removed for reformatting)

## Decisions Made
- Used a single `$effect` gated on `open` for both initial focus (effect body) and focus restoration (effect cleanup) rather than separate effects — Svelte 5 re-runs the effect and invokes the prior cleanup automatically on `open` transitions, matching the open→close lifecycle exactly.
- Native Tab order handles all in-range moves; `trapTab` only intercepts at the two wrap-around edges. This avoids re-implementing full tab-order logic and reduces risk of diverging from native accessibility-tree behavior.
- `prevFocus` is a plain (non-reactive) `let`, not `$state`, since it is a write-once-per-open/read-once-per-close value with no template binding — matches the plan's explicit spec ("plain (non-reactive)").

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Prettier formatting on the new script block**

- **Found during:** Task 1 verification (`pnpm --dir ui lint`)
- **Issue:** The added `$effect`/`trapTab` block did not match the project's Prettier config (line-wrapping of the `querySelectorAll` call and the new attributes on `.modal-container`), causing `prettier --check` to fail with exit 1.
- **Fix:** Ran `pnpm exec prettier --write src/lib/components/Modal.svelte` (from within `ui/`) to auto-format; re-ran `pnpm --dir ui lint`, which then passed (ESLint + Prettier + `check-tokens.mjs` all green).
- **Files modified:** `ui/src/lib/components/Modal.svelte` (formatting only, no logic change)
- **Commit:** Folded into `550c635` (formatting fix applied before the task commit, not a separate commit)

### Note on acceptance-criteria literalism (not a deviation, documentation only)

Task 1's acceptance criteria expect `grep -c "onkeydown={handleKeydown}"` to return exactly 2 (window + backdrop). The actual count is 1, because the `<svelte:window>` attachment uses the pre-existing conditional form `onkeydown={open ? handleKeydown : undefined}` (not the literal string `onkeydown={handleKeydown}`), which was already the case before this plan and remains completely untouched. Both Escape-key attachments (window + backdrop) are verified present and unchanged; this is a grep-pattern mismatch in the plan text, not a regression.

## Issues Encountered

None beyond the Prettier auto-format noted above.

## User Setup Required

None - no external service configuration required.

## Manual Keyboard Verification (documented, not automatable)

Per the plan and `24-CONTEXT.md`'s "Established Patterns" (no vitest/playwright in this project), the authoritative check for this plan is a manual keyboard walkthrough of the built app. As a build-time/code-trace substitute performed during this autonomous execution (no interactive browser session available to this agent), the following was verified directly against source:

- **DOM order** in `Modal.svelte`: `.modal-container` → `<header>` (`h2` title, then `button.modal-close`) → `.modal-body` (`{@render children?.()}`) → `<footer>` (`{@render footer()}`). `querySelector`/`querySelectorAll` return matches in this document order.
- **Showcase usage** (`ModalSection.svelte`): body renders one `<Input>`; footer renders `Отмена` then `Подтвердить` buttons, matching the plan's assumed structure exactly.
- **Predicted and code-verified order:** initial focus → `×` close button (first focusable in document order, inside the header) → `Input` → `Отмена` → `Подтвердить` → wraps to `×` on further Tab; `Shift+Tab` from `×` wraps to `Подтвердить` (last node) via `trapTab`'s explicit wrap-around branches; all in-between transitions rely on native browser tab order, which cannot diverge from document order since none of these elements carry an explicit non-default `tabindex`.
- **Focus restoration:** the `$effect` cleanup calls `prevFocus?.focus()` where `prevFocus` was captured as `document.activeElement` at the moment `open` became `true` — for the showcase, that is the "Показать модал" trigger `<Button>`, so closing the Modal returns focus there.
- A live-browser confirmation of this walkthrough in the actual running app (desktop `cargo tauri dev` or LAN browser) remains recommended as part of end-of-phase human verification (`human_verify_mode: end-of-phase` per `.planning/config.json`), consistent with how the same manual/visual constraint was handled in plan 24-09.

## Next Phase Readiness

Modal.svelte now satisfies all three WAI-ARIA Dialog Pattern behaviors expected of `role="dialog" aria-modal="true"` (initial focus, Tab-trap, focus restoration) without regressing Escape-key handling, backdrop-click dismissal, or existing ARIA attributes. This closes the last CR-03 code-review finding queued for gap-closure in Phase 24. Two known, explicitly out-of-scope findings remain for future consideration: WR-01 (Escape handler double-invoke risk from both `<svelte:window>` and backdrop `onkeydown`) and IN-01 (`titleId` generated via `Math.random()` rather than a more collision-resistant ID scheme). No blockers for continuing Phase 24 or entering Phase 25 (Tables and Dropdown).

---
*Phase: 24-base-components*
*Completed: 2026-07-18*

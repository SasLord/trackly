---
phase: 24-base-components
plan: 12
subsystem: ui
tags: [svelte5, accessibility, aria, modal, focus-trap, gap-closure, portal]

requires:
  - phase: 24-base-components (plan 10)
    provides: Modal.svelte initial-focus/Tab-trap/focus-restoration (WAI-ARIA Dialog Pattern, CR-03)
provides:
  - Single window-level keydown handler for Modal.svelte covering both Escape and Tab-trapping (CR-01 closed — no more double onClose per Escape)
  - Portal-aware, iframe-inclusive focus trap (CR-02 closed — PdfPreviewModal's iframe and use:portal-teleported dropdowns are now part of the Tab-cycle)
  - Reachable initial-focus fallback verifying the real post-focus() outcome (WR-02 closed)
  - Generic data-tr-portal tagging in portal.ts so any current/future use:portal consumer is automatically visible to whichever Modal is open around it
affects: [25-tables-dropdown, 26-window-dashboard-devices, 27-window-acts-cartridges-printers, 28-window-requests-reports-settings-users, 29-window-login-employee]

tech-stack:
  added: []
  patterns:
    - "Single selector source of truth (TRAP_FOCUSABLE_PARTS array) derives both the dialog-scoped selector and a portal-scoped selector via .map() over the array elements, not string-prepending the joined selector — avoids silently scoping only the first comma-separated alternative"
    - "getClientRects().length > 0 as the visibility filter for position:fixed portaled content, since offsetParent is always null for fixed-position elements and would incorrectly reject them"
    - "Generic DOM tagging (data-tr-portal attribute set inside the shared portal.ts action) lets a cross-cutting concern (Modal's focus trap) discover teleported content without touching any of its 6 consumer files"

key-files:
  created: []
  modified:
    - ui/src/lib/components/Modal.svelte
    - ui/src/lib/utils/portal.ts

key-decisions:
  - "Kept scopedFocusable()'s existing offsetParent!==null filter completely unchanged (relocated into a named helper, not altered) — WR-03 (that filter's own limitations) stays explicitly out of scope this round, not fixed or duplicated"
  - "portaledFocusable() queries the whole document (not scoped per-Modal-instance) — accepted limitation per WR-04 (nested/stacked-modal focus-order edge cases), documented inline as a comment and in the threat model (T-24-12-04, accept)"
  - "handleKeydown now delegates to trapTab unconditionally after the Escape branch (trapTab already early-returns unless e.key==='Tab'), making the pre-existing <svelte:window onkeydown={open ? handleKeydown : undefined}> line the single keydown source for both concerns with zero changes to that line itself"

requirements-completed: [CMP-05]

duration: 12min
completed: 2026-07-18
---

# Phase 24 Plan 12: Modal focus-trap gap closure (CR-01, CR-02, WR-02) Summary

**Closed two BLOCKER code-review findings and one warning in `Modal.svelte`'s focus-management code: merged Escape/Tab into a single window-level keydown listener (no more double `onClose()`), widened the Tab-trap selector to include `iframe` and `use:portal`-teleported content (via a new `data-tr-portal` tag applied generically in `portal.ts`), and made the initial-focus fallback verify its own outcome instead of trusting a possibly-inert first DOM match.**

## Performance

- **Duration:** ~12 min
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments

- **CR-01 (double onClose on Escape):** Removed the `.modal-backdrop`'s own `onkeydown={handleKeydown}` and `.modal-container`'s `onkeydown={trapTab}` attributes. `<svelte:window onkeydown={open ? handleKeydown : undefined} />` (unchanged) is now the sole keydown attachment point in the whole component; `handleKeydown` calls `onClose()` and returns on Escape, otherwise delegates to `trapTab(e)` for Tab-trapping — one listener, both concerns, no duplicate invocation regardless of where focus sits inside the dialog.
- **CR-02 (iframe + portaled content excluded from Tab-trap):** Replaced the two separate selector constants with one array (`TRAP_FOCUSABLE_PARTS`) including `iframe`, `[contenteditable]:not([contenteditable="false"])`, `audio[controls]`, `video[controls]`, `summary` alongside the existing button/href/input/select/textarea/tabindex fragments. Derived `TRAP_FOCUSABLE_SELECTOR` (dialog-scoped) and `PORTAL_FOCUSABLE_SELECTOR` (`[data-tr-portal] <part>` mapped over each fragment individually, not string-prepended to the joined selector) from the same source array. Added `portaledFocusable()`, which queries `document` for `[data-tr-portal]` descendants and filters by `getClientRects().length > 0` (not `offsetParent`, since `dropdownAnchor.ts` sets `position: fixed` on every portaled dropdown, which always yields a null `offsetParent`). `trapTab`'s node list is now `[...scopedFocusable(), ...portaledFocusable()]`.
- **WR-02 (unreachable initial-focus fallback):** Removed the old unfiltered `FOCUSABLE_SELECTOR` constant entirely; initial focus now uses `scopedFocusable()[0]` (the same filtered selector as the trap). Added a verification step directly after the existing `first`/`else dialogEl?.focus()` branch: `if (!dialogEl?.contains(document.activeElement)) { dialogEl?.focus(); }` — this closes the actual bug, since it checks the real DOM outcome of the focus attempt instead of branching on whether `first` was merely non-null.
- **`portal.ts`:** Added one line — `node.setAttribute('data-tr-portal', '')` before `appendChild` — so every node any `use:portal` consumer ever teleports (LocationAutocomplete, PersonAutocomplete, DeviceAutocompleteField, ActFormItemsTable, DeviceContextMenu, CartridgeContextMenu) becomes generically discoverable by any open Modal's focus trap, with zero changes to any of those 6 consumer files.
- Rebuilt `ui/dist` (`pnpm --dir ui build` exit 0) and traced the real production surfaces named in the plan against the updated code (see "Manual Keyboard Verification" below).

## Task Commits

Each task was committed atomically:

1. **Task 1: Merge Escape/Tab into one window-level handler, widen the trap selector, tag portal nodes (CR-01, CR-02, WR-02)** - `69368b2` (fix)
2. **Task 2: Rebuild and verify against the real PdfPreviewModal production surface** - no commit (verification-only task; `ui/dist` is gitignored, no tracked files changed by the rebuild — consistent with the 24-09/24-10 Task-2 pattern)

_No TDD — both tasks are `tdd="false"` per plan frontmatter._

## Files Created/Modified
- `ui/src/lib/components/Modal.svelte` - Merged Escape/Tab keydown handling into the single existing `<svelte:window>` attachment; removed the backdrop's and modal-container's separate `onkeydown` attributes; replaced `FOCUSABLE_SELECTOR`/`TRAP_FOCUSABLE_SELECTOR` with a single `TRAP_FOCUSABLE_PARTS` array deriving both `TRAP_FOCUSABLE_SELECTOR` and `PORTAL_FOCUSABLE_SELECTOR`; added `scopedFocusable()`/`portaledFocusable()` helpers; updated the initial-focus `$effect` with a post-focus verification fallback; updated `trapTab` to include portaled nodes.
- `ui/src/lib/utils/portal.ts` - Added `node.setAttribute('data-tr-portal', '')` inside the `if (targetEl)` block, before `appendChild`.

## Decisions Made
- See `key-decisions` in frontmatter above (selector-derivation-by-map, getClientRects visibility filter, WR-03/WR-04 explicitly left out of scope, unchanged `<svelte:window>` line becoming the sole keydown source).

## Deviations from Plan

None - plan executed exactly as written. Formatting was auto-fixed via `prettier --write` (folded into the Task 1 commit, matching the same pattern documented in 24-10's summary) — this is a Rule 3 (blocking, tooling) auto-fix already anticipated by the plan's own precedent, not a new deviation category.

## Issues Encountered

None beyond the expected Prettier reformat after editing the script block (lint failed once on `prettier --check`, fixed with `prettier --write`, re-ran lint green).

## User Setup Required

None - no external service configuration required.

## Manual Keyboard Verification (documented, not automatable)

Per the plan and `24-CONTEXT.md`'s "no vitest/playwright in this project" constraint (same precedent as 24-09/24-10), the authoritative check is a manual keyboard walkthrough of the built app in a running browser/Tauri session. As a build-time/code-trace substitute performed during this autonomous execution (no interactive browser session available to this agent), the following was verified directly against source and the rebuilt `ui/dist`:

- **`pnpm --dir ui build` exits 0** — confirmed.
- **PdfPreviewModal's iframe (CR-02):** `ui/src/features/acts/PdfPreviewModal.svelte:274-304` confirms the `<iframe sandbox="" srcdoc={htmlContent} class="pdf-iframe">` (line 288) is a genuine descendant of `<Modal>`'s default `children` snippet, rendered inside `.modal-body`, which is inside `dialogEl` — a true DOM descendant, not a portaled/detached node. `TRAP_FOCUSABLE_SELECTOR` now includes `'iframe'`, so `scopedFocusable()` includes it in document order: header close button (`×`) → `.pdf-iframe` → footer `Закрыть` → footer `Печать` → wraps to `×`. Tab from the iframe therefore continues into the footer instead of the cycle wrapping early (the exact CR-02 defect), and Tab from `Печать` (last node) wraps back to `×` (first node) rather than skipping the iframe.
- **Single onClose on Escape (CR-01):** With the backdrop's and modal-container's separate `onkeydown` attributes removed, `<svelte:window onkeydown={open ? handleKeydown : undefined}>` is the only keydown listener in the component tree; `handleKeydown`'s `Escape` branch calls `onClose()` once and returns, so it cannot fire a second time from a bubbling event reaching a second attachment point (there is no second attachment point left). Checked `ui/src/features/devices/DeviceImportCsvModal.svelte:170-173` — its `handleClose` (`onClose(); resetState();`) was the non-idempotent example named in the plan's objective; it now receives exactly one invocation per Escape press.
- **Portaled dropdown tagging (CR-02, portal side):** `ui/src/features/acts/ReturnModal.svelte` and `ui/src/features/cartridges/OperationModal.svelte` both render `LocationAutocomplete`/`PersonAutocomplete` (confirmed via grep — lines 339/348/385 and 712/729/744/759/791 respectively). `LocationAutocomplete.svelte:150-157`'s portaled dropdown div uses `use:portal`, which now runs the updated `portal.ts` action that sets `data-tr-portal=""` on the node before `appendChild`. `PORTAL_FOCUSABLE_SELECTOR` (`[data-tr-portal] <part>` per fragment) therefore matches the dropdown's `<button role="option">` children, and `portaledFocusable()`'s `getClientRects().length > 0` filter (not `offsetParent`, which is always null for the dropdown's `position: fixed` styling per `dropdownAnchor.ts:38`) correctly includes them while open and excludes them once closed/unmounted.
- A live-browser confirmation of this walkthrough (Tab through `PdfPreviewModal`, press Escape once, inspect a portaled dropdown's `data-tr-portal` attribute in DevTools) remains recommended as part of end-of-phase human verification (`human_verify_mode: end-of-phase` per `.planning/config.json`), consistent with how the same manual/visual constraint was handled in plans 24-09 and 24-10.

## Next Phase Readiness

`Modal.svelte` now closes all three findings targeted by this gap-closure round: CR-01 (duplicate `onClose` per Escape — now structurally impossible, single listener), CR-02 (iframe and portaled content excluded from the Tab-trap — both now included via a unified selector-derivation scheme), and WR-02 (unreachable initial-focus fallback — now verifies the real post-focus outcome). WR-03 (existing `offsetParent` filter's own limitations) and WR-04 (portal trap not scoped per-Modal-instance in stacked-modal scenarios) remain explicitly out of scope, documented inline in code and in this plan's threat model as accepted risks. No blockers for continuing Phase 24 or entering Phase 25 (Tables and Dropdown).

---
*Phase: 24-base-components*
*Completed: 2026-07-18*

## Self-Check: PASSED

- FOUND: ui/src/lib/components/Modal.svelte
- FOUND: ui/src/lib/utils/portal.ts
- FOUND: commit 69368b2

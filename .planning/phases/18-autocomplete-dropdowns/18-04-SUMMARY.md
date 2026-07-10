---
phase: 18-autocomplete-dropdowns
plan: 04
subsystem: ui
tags: [svelte5, portal, dropdown-positioning, autocomplete, use-action, act-form]

# Dependency graph
requires:
  - phase: 18-autocomplete-dropdowns
    provides: "Plan 18-01 (list_grouped backend contract: group_by=name+model, count DESC sort, real multi-field FTS filter) + Plan 18-02 (dropdownAnchor use-action + portal recipe, proven in LocationAutocomplete.svelte)"
provides:
  - "ActFormItemsTable.svelte device picker dropdown uses use:portal + use:dropdownAnchor per-row (fixed-position, escapes the act modal's overflow container)"
  - "Focus-open (AUTO-02/D-03): focusing the device input fetches immediately (delay 0) and shows top-20 groups by stock, no text input required"
  - "Real-time filtering (AUTO-03): text input now actually filters via backend name_prefix (Plan 18-01 FTS), replacing the previously-broken filter"
  - "Group-row rendering shows name + model + ×count badge (D-05), not the old name-XOR-serial/inv/count layout"
  - "Empty-state: zero matches keeps the dropdown open and renders 'Ничего не найдено' instead of silently closing"
  - "Per-row keyboard navigation (ArrowUp/Down/Enter/Tab/Escape) and per-row click-outside detection for the portaled dropdown"
affects: [18-05]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Per-row ref maps ($state<Record<number, HTMLElement|null>>) replace single wrapper refs when a component renders N independent portal-anchored dropdowns in one table"
    - "Shared fetchGroups(idx, query) helper reused by both the debounced-input path and the delay-0 focus-open path (LocationAutocomplete precedent)"
    - "visibleGroups(idx) helper centralizes DEF-2A dedup so keyboard nav and template rendering see the identical list (avoids index-drift bugs between {#each} and ArrowUp/Down math)"

key-files:
  created: []
  modified:
    - ui/src/features/acts/ActFormItemsTable.svelte

key-decisions:
  - "Replaced <Input> component with a raw <input bind:this=...> for the device column only — Input.svelte has no ref-forwarding, and use:dropdownAnchor requires a real anchorEl; visual parity kept via a new .device-input CSS class copying .qty-input's token set 1:1"
  - "Dropdown open condition changed from `openByRow[idx] && suggestionsByRow[idx]?.length > 0` to just `openByRow[idx]` — zero-result state now renders an empty-state <li> instead of not rendering the <ul> at all (UI-SPEC Copywriting Contract)"
  - "Added activeIndexByRow state + handleRowKeydown for keyboard nav even though it wasn't explicitly required by Task 1/2 acceptance-criteria greps — keyboard ArrowUp/Down without a visible active-item highlight would be a broken/invisible feature (Rule 2), so class:active + aria-selected wiring was added alongside the new keydown handler"
  - "pickGroup() now also resets activeIndexByRow[idx] = -1 on selection — a state-hygiene addition, not a clone-qty semantics change; qtyMax/handleQtyInput/getSelectedIds/MAX_CLONE_QTY remain byte-for-byte unchanged (verified via git diff, see Verification below)"
  - "opt-count badge font-weight changed 600 → 500 to match UI-SPEC's 2-weight typography contract (400 regular / 500 medium only, no 600 in this phase)"

requirements-completed: [AUTO-01, AUTO-02, AUTO-03, AUTO-04]

duration: ~25min
completed: 2026-07-10
---

# Phase 18 Plan 04: ActFormItemsTable portal-anchor device picker (Wave 2) Summary

**Device picker in the act form now opens on focus (no typing required), filters in real time against the Plan 18-01 backend contract, renders portal+fixed-anchored (not clipped by the act modal), and shows name+model+×count per D-05 — drill-in and single-group collapse remain out of scope for Plan 18-05.**

## Performance

- **Duration:** ~25 min
- **Started:** 2026-07-10 (session)
- **Completed:** 2026-07-10
- **Tasks:** 2/2 completed
- **Files modified:** 1

## Accomplishments

- Replaced the wrapper-relative `position: absolute; top: 40px` dropdown with `use:portal` + `use:dropdownAnchor` (per-row `rowInputEls`/`rowDropdownEls` ref maps), rendered as a `document.body` child with `position: fixed` — the dropdown no longer gets clipped or scroll-trapped by the `ActFormModal` overflow container.
- Focus-open (AUTO-02): focusing the device `<input>` now triggers an immediate (delay-0) fetch via the shared `fetchGroups()` helper, showing the backend's top-20-by-stock groups without requiring any text input — previously the dropdown only opened after typing.
- Real filtering (AUTO-03): removed the `v.trim().length < 1` early-return that silently blocked empty input and, combined with Plan 18-01's backend fix, text typed into the field now actually narrows results by name/inventory-number/serial-number (previously a no-op regression).
- Group-row render now always shows `name` + `model` (when present) + a right-aligned `×count` badge (D-05), replacing the old mutually-exclusive serial/inv-no/count branch; serial/inventory/state remain as supplementary meta-lines underneath.
- Empty-state: zero matches on a non-empty filter keeps the dropdown open and renders "Ничего не найдено" (`--color-text-muted`, `--space-xl` padding, centered) instead of closing — matches the UI-SPEC Copywriting Contract.
- Added per-row keyboard navigation (Escape/ArrowDown-when-closed/ArrowUp/ArrowDown/Enter/Tab) mirroring `LocationAutocomplete.handleKeydown`, with `activeIndexByRow` driving `class:active`/`aria-selected` highlighting.
- Click-outside detection now checks both `rowInputEls[i]` and `rowDropdownEls[i]` per open row (the dropdown node lives outside the row's DOM subtree after the portal move).
- Clone-qty/DEF-2A semantics (`MAX_CLONE_QTY`, `qtyMax()`, `handleQtyInput()`, `getSelectedIds()`) verified byte-for-byte unchanged via `git diff` across both task commits.

## Task Commits

Each task was committed atomically:

1. **Task 1: Portal+anchor дропдаун на raw &lt;input&gt; per-row + рендер группы name+model+count (AUTO-01, D-05)** - `0cba9ce` (feat)
2. **Task 2: Focus-open (AUTO-02) + фильтрация без early-return (AUTO-03) + click-outside для портированного дропдауна** - `5e38564` (feat)

**Plan metadata:** (this commit, docs: complete plan)

## Files Created/Modified

- `ui/src/features/acts/ActFormItemsTable.svelte` — device picker rewritten: raw `<input>` replaces `Input.svelte` for the device column; `use:portal`+`use:dropdownAnchor` on the option `<ul>`; `.dropdown`/`.opt*` CSS wrapped in `:global()` (portal-move requirement, matching the Plan 18-02 `LocationAutocomplete` precedent); `fetchGroups()`/`handleFocus()`/`handleRowKeydown()`/`visibleGroups()`/`handleClickOutside()` added; `handleQueryInput()` early-return removed; group-row markup and CSS updated per D-05.

## Decisions Made

See `key-decisions` in frontmatter above — summarized: raw `<input>` for ref-forwarding, `openByRow[idx]` alone gates dropdown visibility (empty-state renders inside), keyboard-nav highlighting added as an essential-correctness completion of the new keydown handler (Rule 2), and the `×count` badge weight corrected to 500 per the UI-SPEC's 2-weight typography contract.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing critical functionality] Keyboard-nav active-item highlighting**
- **Found during:** Task 2
- **Issue:** The plan's Task 2 action explicitly adds `activeIndexByRow` state and full ArrowUp/Down/Enter/Tab keyboard handling, but doesn't separately call out visual highlighting of the active item. Wiring keyboard navigation without a visible focus indicator would leave the feature effectively invisible/unusable via keyboard (an accessibility correctness gap, not a style nicety) — the UI-SPEC's own Interaction Contract and Accessibility sections list "акцентная подсветка активного" / `aria-selected` on `role="option"` as part of this exact behavior.
- **Fix:** Added `class:active={i === (activeIndexByRow[idx] ?? -1)}` + `role="option"` + `aria-selected` on each option button, and a `:global(.opt.active) { background: var(--color-surface-sunken); }` rule (same tone as the existing `.opt:hover`, no new accent misuse).
- **Files modified:** `ui/src/features/acts/ActFormItemsTable.svelte`
- **Verification:** `pnpm --dir ui run svelte-check` (0 errors) + `pnpm --dir ui run build` (success).
- **Committed in:** `5e38564` (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (Rule 2 — accessibility completeness for a feature introduced within the same task, not new scope).
**Impact on plan:** No scope creep — this is the same class of fix Plan 18-02 already established as precedent (`.dropdown-item.active` in `LocationAutocomplete.svelte`); Task 2 explicitly builds `activeIndexByRow`, this just makes it visible/usable.

## Issues Encountered

None. `pnpm --dir ui run svelte-check` (0 errors, 38 pre-existing unrelated warnings) and `pnpm --dir ui run build` (success) passed clean on the first full pass after both tasks.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

Plan 18-05 continues editing the same file (`ActFormItemsTable.svelte`) to add:
- Drill-in (AUTO-04 D-06/D-07): clicking a raised group replaces the list with its per-instance members + a "← Назад" header, using `condition_distinct_count` from the Plan 18-01 backend contract as the trigger signal.
- D-08 non-expandable-group direct-select shortcut (single-condition, non-serial/non-inventory groups skip drill-in).
- AUTO-05 (D-09): single-group-after-filter flat-list collapse (no group row shown when exactly one group remains).

This plan intentionally left group-row clicks wired to the pre-existing `pickGroup(idx, g)` unchanged (direct selection, clone-qty semantics preserved) — 18-05 will branch that click handler to trigger drill-in for expandable groups.

The manual verification called out in the plan's `<verification>` section (focus opens list without typing; typing filters in real time) is deferred to the phase's final checkpoint in Plan 18-05, consistent with how Plan 18-02/18-03 deferred their own interactive verification. No blockers.

---
*Phase: 18-autocomplete-dropdowns*
*Completed: 2026-07-10*

## Self-Check: PASSED

- FOUND: ui/src/features/acts/ActFormItemsTable.svelte
- FOUND: .planning/phases/18-autocomplete-dropdowns/18-04-SUMMARY.md
- FOUND commit: 0cba9ce
- FOUND commit: 5e38564

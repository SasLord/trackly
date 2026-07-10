---
phase: 18-autocomplete-dropdowns
plan: 03
subsystem: ui
tags: [svelte5, portal, dropdown-positioning, autocomplete, use-action, native-select]

# Dependency graph
requires:
  - phase: 18-autocomplete-dropdowns
    provides: "Plan 18-02 — dropdownAnchor.ts use-action + LocationAutocomplete.svelte reference recipe"
provides:
  - "PersonAutocomplete.svelte and DeviceAutocompleteField.svelte migrated to use:portal + use:dropdownAnchor (dual-ref click-outside, :global() CSS, position:fixed)"
  - "AUTO-01 explicit invariant documentation on all 4 native-<select> wrapper components (Select/CartridgeSelect/GroupedPrinterSelect/PrinterSelect), confirmed via re-read of each source — no hidden custom overlay"
affects: [18-04, 18-05]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Recipe from Plan 18-02 replicated 1:1: use:portal + use:dropdownAnchor={{ anchorEl: inputEl }} on the dropdown div; dual-ref (wrapperEl + dropdownEl) handleClickOutside; :global() wrap for all scoped-CSS classes on the portaled subtree"
    - "dropdownAnchor maxHeight param passed explicitly when a consumer's CSS max-height differs from the 240px default (DeviceAutocompleteField uses 200px) — keeps the upward-flip calculation accurate"
    - "AUTO-01 doc-comment convention for native-<select> wrappers: state the browser-native-popup rationale and explicitly name the one position:absolute element (decorative caret) so future readers don't mistake it for an overlay needing migration"

key-files:
  created: []
  modified:
    - ui/src/lib/components/PersonAutocomplete.svelte
    - ui/src/features/devices/DeviceAutocompleteField.svelte
    - ui/src/lib/components/Select.svelte
    - ui/src/lib/components/CartridgeSelect.svelte
    - ui/src/lib/components/GroupedPrinterSelect.svelte
    - ui/src/lib/components/PrinterSelect.svelte

key-decisions:
  - "DeviceAutocompleteField's dropdownAnchor call passes maxHeight: 200 (not the default 240) to match its existing CSS max-height: 200px — otherwise the flip-upward math would use a stale 240px threshold while the visible box only ever grows to 200px, causing an occasional unnecessary/late flip near the viewport edge"
  - "Task 3 treated as a verification gate, not a rote copy: each of the 4 native-select files was re-read in full before adding the AUTO-01 comment, confirming no role=\"listbox\" or custom dropdown markup exists (T-18-07 threat-register mitigation) — comment content is identical across the 4 files since the underlying invariant (native <select> + decorative caret) is identical"

requirements-completed: [AUTO-01]

duration: ~9min
completed: 2026-07-10
---

# Phase 18 Plan 03: Migrate PersonAutocomplete/DeviceAutocompleteField + document native-select AUTO-01 invariant Summary

**PersonAutocomplete and DeviceAutocompleteField dropdowns moved to `use:portal` + `use:dropdownAnchor` (Plan 18-02 recipe); the 4 native-`<select>` wrappers documented in-source as AUTO-01-compliant by construction, confirmed via fresh source re-read rather than copied assumption.**

## Performance

- **Duration:** ~9 min
- **Started:** 2026-07-10T00:08:00Z
- **Completed:** 2026-07-10T00:17:00Z
- **Tasks:** 3
- **Files modified:** 6

## Accomplishments
- `PersonAutocomplete.svelte`: added missing `inputEl` ref, migrated its `.dropdown` from wrapper-relative `position: absolute` to `use:portal` + `use:dropdownAnchor={{ anchorEl: inputEl }}`; `handleClickOutside` now checks both `wrapperEl` and the new `dropdownEl`.
- `DeviceAutocompleteField.svelte`: reused its pre-existing `inputEl` ref (already shared by the `<input>`/`<textarea>` variants), migrated the same way; the `dropdown-header` "Ранее использовалось с…" / "Все расположения:" content pattern (03.3 ITEM-4) left byte-for-byte unchanged — only the outer container's positioning/CSS moved.
- Confirmed (by re-reading full source, not by trusting `18-PATTERNS.md`) that `Select.svelte`, `CartridgeSelect.svelte`, `GroupedPrinterSelect.svelte`, and `PrinterSelect.svelte` each wrap a plain native `<select>` with only a decorative `.caret` SVG (`position: absolute; pointer-events: none`) — no custom option-list overlay exists in any of them — then added an explicit AUTO-01 doc-comment to each explaining why no portal/anchor migration is needed.

## Task Commits

Each task was committed atomically:

1. **Task 1: Мигрировать PersonAutocomplete на portal + dropdownAnchor** - `d5c45de` (feat)
2. **Task 2: Мигрировать DeviceAutocompleteField на portal + dropdownAnchor** - `fb17e98` (feat)
3. **Task 3: Задокументировать AUTO-01-инвариант для 4 native-select компонентов** - `d8e9d5e` (docs)

**Plan metadata:** (this commit, docs: complete plan)

## Files Created/Modified
- `ui/src/lib/components/PersonAutocomplete.svelte` - Added `inputEl`/`dropdownEl` refs; dropdown moved to `use:portal`+`use:dropdownAnchor`; dual-ref click-outside; `.dropdown`/`.dropdown-item`/`.dropdown-loading`/`.dropdown-empty` CSS wrapped `:global()`, `position: fixed`, `box-shadow` → `--shadow-elev-2`
- `ui/src/features/devices/DeviceAutocompleteField.svelte` - Added `dropdownEl` ref, reused existing `inputEl`; dropdown moved to `use:portal`+`use:dropdownAnchor={{ anchorEl: inputEl, maxHeight: 200 }}`; dual-ref click-outside; `.dropdown`/`.dropdown-header`/`.dropdown-loading`/`.dropdown-empty`/`.dropdown-item` CSS wrapped `:global()`, `position: fixed`, `box-shadow` → `--shadow-elev-2`; `dropdown-header` content logic untouched
- `ui/src/lib/components/Select.svelte` - AUTO-01 doc-comment added (no functional change)
- `ui/src/lib/components/CartridgeSelect.svelte` - AUTO-01 doc-comment added (no functional change)
- `ui/src/lib/components/GroupedPrinterSelect.svelte` - AUTO-01 doc-comment added (no functional change)
- `ui/src/lib/components/PrinterSelect.svelte` - AUTO-01 doc-comment added (no functional change)

## Decisions Made
- `DeviceAutocompleteField`'s `dropdownAnchor` call passes `maxHeight: 200` explicitly (its CSS `max-height` is 200px, not the 240px default used by `LocationAutocomplete`/`PersonAutocomplete`) so the upward-flip decision in `dropdownAnchor.ts` matches the component's actual rendered height.
- Task 3's stop-if-regression-found instruction (threat T-18-07) was honored literally: each of the 4 native-select files was fully re-read before the comment was added, not assumed correct from the plan text or `18-PATTERNS.md` — no regression was found, so all 4 got the comment.

## Deviations from Plan

None - plan executed exactly as written. The `maxHeight: 200` parameter on `DeviceAutocompleteField`'s `dropdownAnchor` call is a direct, same-rationale extension of the plan's explicit instruction to keep the existing 200px CSS `max-height` unchanged — passing it through to the anchor action's flip calculation is Rule 1 territory (without it, the flip math would silently use the wrong threshold for this one consumer).

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- `dropdownAnchor.ts` recipe has now been applied to all 3 custom-overlay autocomplete components in the phase's inventory (`LocationAutocomplete` in 18-02, `PersonAutocomplete` and `DeviceAutocompleteField` in this plan). AUTO-01 is now closed for every component type identified in `18-UI-SPEC.md`'s inventory except the device picker in `ActFormItemsTable.svelte`, which Plan 18-04/18-05 own per the phase's Wave 0 contract.
- All 4 native-`<select>` components carry explicit, source-verified AUTO-01 documentation — no further work needed on them for this requirement.
- Visual/DOM-position confirmation (dropdown as `document.body.lastElementChild`, capture-scroll reposition, upward flip) remains deferred to the phase's final checkpoint in Plan 18-05, per the phase-level `<verification>` contract — not blocking here.
- No blockers.

---
*Phase: 18-autocomplete-dropdowns*
*Completed: 2026-07-10*

## Self-Check: PASSED

- FOUND: ui/src/lib/components/PersonAutocomplete.svelte
- FOUND: ui/src/features/devices/DeviceAutocompleteField.svelte
- FOUND: ui/src/lib/components/Select.svelte
- FOUND: ui/src/lib/components/CartridgeSelect.svelte
- FOUND: ui/src/lib/components/GroupedPrinterSelect.svelte
- FOUND: ui/src/lib/components/PrinterSelect.svelte
- FOUND: .planning/phases/18-autocomplete-dropdowns/18-03-SUMMARY.md
- FOUND commit: d5c45de
- FOUND commit: fb17e98
- FOUND commit: d8e9d5e

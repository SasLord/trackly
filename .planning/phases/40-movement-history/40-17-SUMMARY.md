---
phase: 40-movement-history
plan: 17
subsystem: ui
tags: [svelte, movement-history, timeline, cartridge-detail, printer-detail]

requires:
  - phase: 40-movement-history (Plan 15)
    provides: "MovementTimeline.svelte shared component + ActsPage.svelte ?id= hash-query support"
  - phase: 40-movement-history (Plan 16)
    provides: "PlaceEntityViewModal.svelte's direct-analog mount of MovementTimeline (device/printer/cartridge, shared loadError pattern)"
provides:
  - "CartridgeDetail.svelte — renamed «Журнал операций» legacy section (byte-for-byte unchanged content) + NEW «Перемещения» section, sibling not replacement (D-16)"
  - "PrinterDetail.svelte — «История перемещений» section reusing MovementTimeline by the printer's underlying device id, no separate printer entity_type (D-21)"
affects: []

tech-stack:
  added: []
  patterns:
    - "CartridgeDetail/PrinterDetail each own a dedicated, component-local $effect + movements/movementsLoading/movementsLoadError state for the new timeline fetch, independent of whatever loading/error state the rest of the panel already used — unlike Plan 40-16's modal, neither of this plan's two consumers had an existing single-fetch effect to fold the new timeline call into (CartridgeDetail's `history` prop is fetched by its parent CartridgesPage.svelte; PrinterDetail already runs several independent per-concern effects, e.g. readings/compatAggregates/deviceData, so a new independent effect matches its own established internal convention)"
    - "MovementTimeline's own scoped loadError is used in BOTH consumers (not a shared page-level error flag) — a timeline-only fetch failure shows only inside the Перемещения/История перемещений section, the rest of the cartridge/printer detail keeps rendering normally"

key-files:
  created: []
  modified:
    - ui/src/features/cartridges/CartridgeDetail.svelte
    - ui/src/features/printers/PrinterDetail.svelte

key-decisions:
  - "Revisited Plan 40-16's flagged decision (shared loadError hides the whole form on a timeline-only failure) and chose the OPPOSITE for both of this plan's consumers: a scoped, section-local loadError via MovementTimeline's own prop. Rationale: CartridgeDetail's `history` array is a prop owned by CartridgesPage.svelte, not something this component already fetches — there is no existing single fetch to fold into, so the new timeline fetch is inherently independent, and hiding an already-populated, fully-loaded detail panel because ONLY the new movements section failed to load would be a worse UX than what Plan 40-16's modal does (there, ALL data — including the entity itself — comes from one combined fetch, so a shared error is a smaller compromise). PrinterDetail reinforces this: it already runs several independent per-concern $effects (readings, compatAggregates, deviceData, installedCartridge), each rendering its OWN loading/empty/error branch independently — a new movements effect with its own scoped error state is the established local convention, not a deviation from it."
  - "Placed PrinterDetail's new «История перемещений» section directly after «История статусов» (SNMP readings) and before «Данные устройства» — no explicit ordering was mandated by the plan; this groups the two 'history' sections adjacently while keeping the device-metadata section last, matching the page's existing top-to-bottom flow (device/printer identity → live metrics → historical logs → editable metadata)."
  - "Both consumers navigate on Перемещения row clicks via a plain `push(...)` from svelte-spa-router with no `onClose()`/teardown step — unlike Plan 40-16's PlaceEntityViewModal (a modal that must close before/while navigating), CartridgeDetail and PrinterDetail are page-level detail panes with no overlay to dismiss, so the navigation callback is a direct one-liner."

requirements-completed: []  # orchestrator closes HST-02 at phase end per bookkeeping_constraint

duration: 20min
completed: 2026-09-02
---

# Phase 40 Plan 17: Cartridge/Printer Detail Timeline Wiring Summary

**`CartridgeDetail.svelte` gets a renamed «Журнал операций» section plus a new sibling «Перемещения» timeline; `PrinterDetail.svelte` gets a new «История перемещений» section reading its underlying device's identical timeline — both via the shared `MovementTimeline` component, with a scoped (not shared) error state.**

## Performance

- **Duration:** ~20 min
- **Completed:** 2026-09-02
- **Tasks:** 2 (both `type="auto"`)
- **Files modified:** 2

## Accomplishments

- `CartridgeDetail.svelte`'s existing `DetailSection heading="История перемещений"` is renamed to `"Журнал операций"` with zero other changes inside that block — `formatHistoryEntry`, `parsePayloadDetails` (including its documented numeric-`place_id` display bug), and the `.history-list`/`.history-empty` markup are all byte-for-byte untouched (verified: `formatHistoryEntry` reference count unchanged at 2, matching pre-plan state).
- A NEW `DetailSection heading="Перемещения"` is added directly below the renamed one, mounting `MovementTimeline` fed by a new component-owned `$effect` (`place_movements_get_timeline`, `entity_type: 'cartridge'`). Both sections coexist as siblings — nothing removed or replaced (D-16, "ничего не теряем").
- `PrinterDetail.svelte` gains a new `DetailSection heading="История перемещений"` mounting the same shared `MovementTimeline`, fetched by `entity_type: 'device'` using `printer.deviceId` — the exact field already used by this component's own `deviceData`/`compatAggregates` effects — never a separate `'printer'` entity_type (D-21, confirmed no matches for `entity_type.*printer` or the literal `'printer'` string anywhere near the new fetch code).
- Both new sections use `MovementTimeline`'s own scoped `loadError` prop rather than sharing either panel's overall `loading` gate, a deliberate departure from Plan 40-16's modal pattern — see `key-decisions` for the full rationale requested by this plan's `<prior_work_this_phase>` note.
- Row-click navigation (place segments, act numbers) in both consumers calls `push('#/places?id=…')` / `push('#/acts?id=…')` directly — no `onClose()` step needed since neither `CartridgeDetail` nor `PrinterDetail` is a modal.

## Task Commits

Each task was committed atomically:

1. **Task 1: CartridgeDetail — rename + new Перемещения section** - `c3d8032d` (feat)
2. **Task 2: PrinterDetail — reuse timeline by device id** - `a70eb234` (feat)

## Files Created/Modified

- `ui/src/features/cartridges/CartridgeDetail.svelte` - renamed legacy section heading, new `Перемещения` section + its own timeline fetch/state (modified)
- `ui/src/features/printers/PrinterDetail.svelte` - new `История перемещений` section + its own timeline fetch/state, keyed on `printer.deviceId` (modified)

## Decisions Made

See `key-decisions` in frontmatter — most notably the scoped-vs-shared `loadError` choice (opposite of Plan 40-16's modal, and why that's the right call for both of this plan's consumers), and the section-ordering choice in `PrinterDetail.svelte`.

## Deviations from Plan

None — plan executed exactly as written. One clarification worth flagging: the plan's Task 1 `<action>` text describes folding the new fetch "into the same effect as the existing history fetch (preferred...) or add a second minimal loading flag if the two fetches are genuinely independent in this file's existing structure" — `CartridgeDetail.svelte` has NO existing internal fetch effect at all (its `history` prop is fetched by `CartridgesPage.svelte`, the parent), so the "genuinely independent" branch was the only applicable one; a new component-owned `$effect` plus `movements`/`movementsLoading`/`movementsLoadError` state was added, matching the plan's own fallback instruction. Not a deviation from the plan's actual requirement, just resolving which of its two described branches applied.

## Issues Encountered

None.

## Verification Reality

- **Mechanically verified:** `pnpm --dir ui svelte-check` (0 errors, 60 pre-existing warnings baseline, unchanged from Plan 40-16 — none attributable to either of this plan's two modified files), `pnpm --dir ui lint` (eslint + prettier + all 9 `check-*.mjs` gates, including `check-privacy`), and `pnpm --dir ui build` (production build succeeds; only pre-existing unrelated warnings — `ActFormItemsTable.svelte`'s unused `.hint-warn` selector and the large-chunk-size advisory, both present before this plan). All of Task 1's and Task 2's literal grep-based acceptance criteria pass exactly as specified (heading rename, new heading present, old heading string count 0, `formatHistoryEntry` reference count unchanged, `MovementTimeline` present in `PrinterDetail.svelte`, zero `'printer'`-entity_type matches).
- **UNVERIFIED (per this plan's explicit `<verification_reality>` constraint):** `MovementTimeline` has still NOT been observed actually rendering inside a real running app in either of its two new mount points. This environment has no authenticated session exercised against `cargo tauri dev` or the axum server — I did not click through to a real cartridge's or printer's detail pane to visually confirm the new sections render without a Svelte 5 rune runtime error. This is now the THIRD and FOURTH mount point for the same shared component (after Plans 40-15's showcase gallery and 40-16's `PlaceEntityViewModal`), all passing the same green `svelte-check`/`lint`/`build` gates — a reasonably strong signal given the component's props/shape have now been exercised by three independent parent templates with three different data-fetch conventions, but per this project's own documented lesson ("Synthetic harness not verification"), a green compile is explicitly NOT proof of correct runtime behavior. Stating plainly: **the actual render of both new sections, in a running Tauri app or LAN browser, is UNVERIFIED.**
- This is the LAST plan of Phase 40 — the phase-level UAT/verification pass (or a manual smoke test in the running app) is the first real opportunity to confirm `MovementTimeline` renders correctly across all four of its mount points (`PlaceEntityViewModal` ×2 kinds, `CartridgeDetail`, `PrinterDetail`) and that Plan 40-16's flagged shared-vs-scoped-error question doesn't surface as an inconsistency worth reconciling across the phase's UI surfaces.

## Known Stubs

None — both new sections are fully wired to the real `place_movements_get_timeline` backend call with real entity ids; no hardcoded empty values or placeholder text introduced.

## Threat Flags

None. This plan introduces no new endpoints, auth paths, or schema changes — it consumes Plan 40-10's already-gated `place_movements_get_timeline` read endpoint from two more UI surfaces, both already covered by this plan's own `<threat_model>` (T-40-34), which is satisfied per this summary's Accomplishments (rename verified byte-for-byte unchanged, no numeric-place_id leak reintroduced into the new section).

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- HST-02's full UI surface (device/printer view modal, device-list entry point, cartridge detail, printer detail) is now wired to `MovementTimeline` across all four mount points. This is the final plan of Phase 40 (movement-history) — no further consumers remain.
- Flag for the phase verifier: Plan 40-16 deliberately shares `loadError` across the whole `PlaceEntityViewModal`, while this plan deliberately scopes `loadError` per-section in `CartridgeDetail`/`PrinterDetail`. Both are defensible given each component's own pre-existing data-fetch shape (documented in both plans' `key-decisions`), but the phase verifier may want to explicitly accept this as intentional variation rather than an inconsistency to fix.
- First real runtime verification of all four `MovementTimeline` mount points should happen via manual UAT in the running app (Tauri desktop or LAN browser serving a fresh `ui/dist` build) before Phase 40 is considered fully closed.

---
*Phase: 40-movement-history*
*Completed: 2026-09-02*

## Self-Check: PASSED

Both modified files verified present on disk; both task commits (`c3d8032d`, `a70eb234`) verified present in `git log --oneline --all`.

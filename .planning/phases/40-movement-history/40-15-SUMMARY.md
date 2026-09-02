---
phase: 40-movement-history
plan: 15
subsystem: ui
tags: [svelte, svelte-spa-router, movement-history, timeline, design-system]

requires:
  - phase: 40-movement-history (Plan 10)
    provides: "MovementEntryDto + place_movements_get_timeline (both transports), already generated into ui/src/bindings.ts"
provides:
  - "MovementTimeline.svelte — shared timeline-row component (states: loading/empty/error/populated) with zero client-side path-shortening logic"
  - "ActsPage.svelte ?id= hash-query support, extending the existing parseIdFromHash convention to Acts for the first time"
affects: [40-movement-history-16, 40-movement-history-17]

tech-stack:
  added: []
  patterns:
    - "MovementTimeline consumes already-formatted DTO fields only (from_place_path_short/to_place_path_short, actor_display) — never re-derives display strings, matching the project's single-owner formula convention (D-18, WR-03/WR-08 precedent)"
    - "ActsPage's reset-on-tab-change $effect guarded with isFirstTabEffectRun, exact clone of CartridgesPage.svelte's own guard for the same 'effect fires once on mount and would wipe the hash-derived initial selection' hazard"

key-files:
  created:
    - ui/src/lib/components/MovementTimeline.svelte
    - ui/src/features/showcase/sections/MovementTimelineSection.svelte
  modified:
    - ui/src/features/showcase/ShowcasePage.svelte
    - ui/src/features/acts/ActsPage.svelte

key-decisions:
  - "MovementTimeline never renders its own spinner — the parent's single fetch already resolves loading/error per UI-SPEC's States table; loading=true renders nothing"
  - "Act-number navigation and place navigation are both prop callbacks (onNavigateToAct/onNavigateToPlace), not hardcoded hash writes — keeps the component reusable across the three different parent close-then-navigate sequences (Plans 40-16/40-17 wire these)"
  - "ActsPage does not attempt to switch tabs (handover/returns/archive) to match a hash-focused act's own act_type/archived state — it only selects+fetches the act by id directly via the existing selectedActId $effect, which already fetches acts.get(id) independent of the active tab filter. Sufficient to satisfy D-19's 'has somewhere real to navigate to' requirement without inventing new tab-switching logic"

requirements-completed: []  # orchestrator closes HST-02 at phase end per bookkeeping_constraint

duration: 25min
completed: 2026-09-02
---

# Phase 40 Plan 15: Movement Timeline Shared Component + ActsPage Deep Link Summary

**Shared `MovementTimeline.svelte` rendering the canonical `date — from → to · actor · reason` row anatomy with zero client-side path-shortening, plus `ActsPage.svelte`'s new `?id=` hash-query support so the timeline's act-number link has a real destination.**

## Performance

- **Duration:** ~25 min
- **Completed:** 2026-09-02
- **Tasks:** 2 (both `type="auto"`)
- **Files modified:** 4 (2 created, 2 modified)

## Accomplishments

- `MovementTimeline.svelte` — the ONE shared component Plans 40-16 (device/printer modal) and 40-17 (cartridge/printer detail) will both mount. Renders each row per UI-SPEC's "Timeline row anatomy" table: manual `padStart` date, from/to place buttons (`.crumb`-derived styling with `color: var(--tr-accent-text)` per the UI-SPEC's exact required change, `title=` carrying the FULL stored path snapshot, visible text using the server-shortened snapshot), literal `→`/`·` separators, plain-text actor, and a composed reason (`«вручную»` / `«вручную · {note}»` / `«актом №{act_number}»` with the number itself a clickable button / safe fallback for any unrecognized `source`).
- Zero JS mirror of the path-shortening formula — `grep -c "shorten\|substring.*path\|split('/ ')"` on the new file returns `0`. The component only displays `from_place_path_short`/`to_place_path_short`, never re-derives them (D-18, Don't Hand-Roll, the exact WR-03/WR-08 divergence class this guards against).
- Loading/empty/error states implemented exactly per UI-SPEC: loading renders nothing (parent owns the single spinner), empty renders «Перемещений ещё не было» + the exact body copy, error renders the exact «Не удалось загрузить историю перемещений…» copy.
- Unrecognized `source` values render a safe fallback reason string (`«причина не определена»`) instead of throwing (T-40-30 mitigation).
- Showcase gallery entry (`MovementTimelineSection.svelte`, wired into `ShowcasePage.svelte`) exercises all documented behavior cases — 3-row populated (act / manual+note / manual), empty, error, and unrecognized-`source` — with invented demo data only (no real DB rows, per the project's hard privacy rule), mirroring Phase 39's `PlacePickerSection.svelte` precedent.
- `ActsPage.svelte` now consumes `parseIdFromHash` exactly like `DevicesPage`/`CartridgesPage`/`PrintersPage`/`PlacesPage` already do: `selectedActId` is seeded from the hash on mount, and the existing `selectedActId`-keyed `$effect` (already present, unchanged) fetches the act via `acts.get(id)` regardless of the active tab. The pre-existing "reset selection on tab change" `$effect` is guarded with `isFirstTabEffectRun` — an exact clone of `CartridgesPage.svelte`'s own guard — so it doesn't wipe the hash-derived selection on the same render it was set.

## Task Commits

Each task was committed atomically:

1. **Task 1: MovementTimeline.svelte shared component** - `b98f7ccd` (feat)
2. **Task 2: ActsPage.svelte accepts ?id= hash query** - `950fcebc` (feat)

**Plan metadata:** (this commit) `docs(40-15): complete movement-timeline-shared-component plan`

## Files Created/Modified

- `ui/src/lib/components/MovementTimeline.svelte` - shared timeline-row component (created)
- `ui/src/features/showcase/sections/MovementTimelineSection.svelte` - showcase gallery entry, invented demo data (created)
- `ui/src/features/showcase/ShowcasePage.svelte` - registered the new showcase section (modified)
- `ui/src/features/acts/ActsPage.svelte` - `parseIdFromHash` deep-link support + guarded tab-reset effect (modified)

## Decisions Made

- `bindings.ts` required no changes — `MovementEntryDto` and `placeMovementsGetTimeline` were already generated by Plan 40-10's specta export step; verified present via `grep -n "MovementEntryDto" ui/src/bindings.ts` before starting, so the plan's `files_modified` entry for `bindings.ts` turned out to be a no-op.
- See `key-decisions` in frontmatter for the loading-state and navigation-callback decisions, and the ActsPage tab-switching scope decision.

## Deviations from Plan

None - plan executed exactly as written. The one wrinkle (Task 1's acceptance-criteria grep initially matching 2 comment-prose occurrences of "shorten") was resolved by rewording the doc comments (no code change) — the same class of self-inflicted grep near-miss Plan 40-10's own SUMMARY documented, not a deviation from the plan's actual requirements.

## Issues Encountered

- `pnpm --dir ui lint`'s eslint pass flagged two now-unnecessary `eslint-disable-next-line no-console` comments in the showcase section (the demo `console.log` calls don't actually trigger the `no-console` rule in this project's eslint config) — removed the disable comments; `prettier --check` also required a single format pass on the new component file. Both fixed inline before the Task 1 commit; not deviations from plan scope, just standard gate-driven cleanup.

## Verification Reality

- **Mechanically verified:** `pnpm --dir ui svelte-check` (0 errors, same 60 pre-existing warnings in unrelated files, none attributable to the new/modified files in this plan) and `pnpm --dir ui lint` (eslint + prettier + all 7 `check-*.mjs` gates) both green after each task. The `grep -c "shorten\|substring.*path\|split('/ ')"` acceptance criterion returns `0`. `grep -n "parseIdFromHash" ui/src/features/acts/ActsPage.svelte` returns matches.
- **UNVERIFIED (per the task's `<verification_reality>` constraint):** the component has NOT been observed rendering inside the real running app. Svelte 5 rune runtime errors are not caught by `svelte-check`/`eslint`/`pnpm build`, only by mounting the component. This plan builds `MovementTimeline.svelte` in isolation (via the showcase gallery, which itself has not been visually loaded in a running dev session during this execution) — Plans 40-16/40-17 are the ones that actually mount it inside `PlaceEntityViewModal`/`CartridgeDetail`/`PrinterDetail`, and that is the first point real runtime verification becomes possible. The showcase section exists specifically so that verification is possible before those plans run, but seeing it render was out of this plan's scope to confirm.
- Similarly, `ActsPage.svelte`'s new hash-query behavior has been read and reasoned through against the exact `CartridgesPage.svelte` precedent (same guard shape, same effect ordering) but has not been exercised by actually navigating to `#/acts?id=N` in a running app.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `MovementTimeline.svelte` is ready for Plan 40-16 (device/printer `PlaceEntityViewModal` integration) and Plan 40-17 (cartridge `CartridgeDetail`/`PrinterDetail` integration) to import and wrap in their own `DetailSection`, wiring `onNavigateToPlace`/`onNavigateToAct` to each parent's own close-then-navigate sequencing (mirroring `PlaceEntityViewModal::handleGoTo`'s `await push(...); onClose();` pattern, per this plan's `<interfaces>`).
- `ActsPage.svelte` is ready to receive `#/acts?id=N` navigations from the timeline's act-number link once Plans 40-16/40-17 wire `onNavigateToAct` to `push('#/acts?id=' + actId)`.
- First real runtime verification of `MovementTimeline.svelte` (rune errors, visual layout) should happen either by loading the showcase page in a running dev session, or naturally as part of Plan 40-16/40-17's own verification once the component is mounted in a real modal/detail panel.

---
*Phase: 40-movement-history*
*Completed: 2026-09-02*

## Self-Check: PASSED

All 2 created files verified present on disk; both task commits (`b98f7ccd`, `950fcebc`) verified present in `git log --oneline --all`.

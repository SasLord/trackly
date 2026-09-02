---
phase: 40-movement-history
plan: 16
subsystem: ui
tags: [svelte, movement-history, timeline, place-entity-view-modal, device-context-menu]

requires:
  - phase: 40-movement-history (Plan 15)
    provides: "MovementTimeline.svelte shared component + ActsPage.svelte ?id= hash-query support"
  - phase: 40-movement-history (Plan 10)
    provides: "place_movements_get_timeline (both transports) + MovementEntryDto"
provides:
  - "PlaceEntityViewModal.svelte — mounts MovementTimeline under a new «История перемещений» DetailSection for both kind='device' and kind='printer' rows"
  - "DeviceContextMenu.svelte — new first «Просмотр» menu item, the modal's first entry point outside PlaceContents"
affects: [40-movement-history-17]

tech-stack:
  added: []
  patterns:
    - "Timeline fetch added to PlaceEntityViewModal's existing single $effect as a second concurrent async branch (Promise.all with the main entity fetch), sharing the SAME loading/loadError state as the rest of the modal — no independent spinner/error for the new section (must_haves truth, D-29)"
    - "DeviceContextMenu reuses its already-wired onDelete prop as the generic 'list changed, reload' signal for the new view-modal's edit-save path, avoiding new prop threading through DeviceList/DeviceGroupRow (kept the plan's file scope to exactly DeviceContextMenu.svelte + DeviceListRow.svelte)"

key-files:
  created: []
  modified:
    - ui/src/features/places/PlaceEntityViewModal.svelte
    - ui/src/features/devices/DeviceContextMenu.svelte
    - ui/src/features/devices/DeviceListRow.svelte

key-decisions:
  - "Both the main entity fetch and the new timeline fetch run concurrently (two async IIFEs awaited via Promise.all) inside the same $effect, rather than sequentially — halves the visible loading time without breaking the shared-loading-state requirement"
  - "Timeline fetch failure sets the SAME loadError flag as the main entity fetch (not a separate timeline-only error flag) — per the plan's literal action text and the must_haves truth ('shares the modal's single existing loading/error state'); a timeline-only failure now surfaces as the modal's existing top-level 'Не удалось загрузить данные' error rather than a scoped inline message, which is a deliberate simplification the plan text specifies over UI-SPEC's per-section copy nuance"
  - "DeviceContextMenu's new 'Просмотр' path only ever constructs kind='device' rows (the device list itself has no printer rows) — printer viewing continues to go through PlaceContents, per D-21's single-entity-timeline model this is unaffected"
  - "PlaceContentDto.status_name for the new entry point is filled from DeviceListRow's own STATUS_LABELS-derived statusLabel (threaded through DeviceContextMenu as a new required statusName prop), since DeviceDto only carries status_id — avoids duplicating the status-label mapping in DeviceContextMenu"

requirements-completed: []  # orchestrator closes HST-02 at phase end per bookkeeping_constraint

duration: 20min
completed: 2026-09-02
---

# Phase 40 Plan 16: PlaceEntityViewModal Timeline + Device List Entry Point Summary

**Wired Plan 40-15's shared `MovementTimeline` into `PlaceEntityViewModal` for both device and printer rows, and added the modal's first entry point directly from the device list (`DeviceContextMenu`'s new «Просмотр» item).**

## Performance

- **Duration:** ~20 min
- **Completed:** 2026-09-02
- **Tasks:** 2 (both `type="auto"`)
- **Files modified:** 3

## Accomplishments

- `PlaceEntityViewModal.svelte` now fetches the entity's movement timeline (`place_movements_get_timeline` via `apiCall`) alongside its existing device/cartridge fetch, inside the same `$effect`, run concurrently via `Promise.all` — both fetches share the single `loading`/`loadError` state (no independent spinner). Rendered as a new `<DetailSection heading="История перемещений">` immediately after the existing read-only `CartridgeFormBody`/`DeviceFormBody` render.
- D-21 honored exactly as specified: `kind === 'cartridge'` fetches `entity_type: 'cartridge'`; both `kind === 'device'` and `kind === 'printer'` fetch `entity_type: 'device'` — no separate printer branch anywhere in the fetch or the component.
- Place-segment and act-number clicks inside the timeline navigate via `push('#/places?id=…')` / `push('#/acts?id=…')` then `onClose()`, mirroring `handleGoTo`'s existing await-then-close sequencing (GAP-9 precedent).
- `DeviceContextMenu.svelte` gains a new first kebab-menu item, «Просмотр», above the existing «Редактировать» — the first time `PlaceEntityViewModal` opens from anywhere other than `PlaceContents`. It constructs a `PlaceContentDto`-shaped `kind: 'device'` row from the `DeviceDto` prop it already has, plus a new `statusName` prop threaded from `DeviceListRow` (which already computes the status label for its `Badge`).
- A save from the view modal's internal edit form reuses the already-wired `onDelete` callback (DevicesPage's existing generic "reload list + counts" handler) to refresh the device list — no new prop threading through `DeviceList`/`DeviceGroupRow` was needed, keeping this plan's changes to exactly the two files it named.

## Task Commits

Each task was committed atomically:

1. **Task 1: PlaceEntityViewModal — new История перемещений DetailSection** - `21731fdd` (feat)
2. **Task 2: DeviceContextMenu «Просмотр» entry + DeviceListRow wiring** - `2a0d6b50` (feat)

## Files Created/Modified

- `ui/src/features/places/PlaceEntityViewModal.svelte` - timeline fetch + `DetailSection`/`MovementTimeline` mount (modified)
- `ui/src/features/devices/DeviceContextMenu.svelte` - new «Просмотр» menu item + `viewRow`/`PlaceEntityViewModal` mount (modified)
- `ui/src/features/devices/DeviceListRow.svelte` - threads `statusName={statusLabel}` to `DeviceContextMenu` (modified)

## Decisions Made

See `key-decisions` in frontmatter. Most notably: the timeline fetch shares the SAME `loadError` flag as the main entity fetch (not a separate scoped error), per the plan's literal action text and must_haves truth — a timeline-only failure now surfaces the modal's existing top-level error message rather than a section-scoped one. This is a plan-directed simplification worth flagging for the phase verifier against UI-SPEC's States table, which describes a distinct per-section error copy ("Не удалось загрузить историю перемещений…") for this surface; as implemented, that copy is only reachable in `MovementTimeline` when a FUTURE consumer of the component (e.g. Plan 40-17) passes it a scoped error flag independently of the parent's main-entity error — it is currently unreachable from `PlaceEntityViewModal`, whose combined `loadFailed` would hide the section entirely on any failure instead.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Reworded doc comments to avoid duplicate literal-string grep matches**
- **Found during:** Task 2
- **Issue:** Initial doc comments in `DeviceContextMenu.svelte` used the literal strings «Просмотр» and «Редактировать» in prose, which would have made `grep -n "Просмотр"` return more than the single expected match and put a comment-only «Редактировать» reference before the real «Просмотр» button line — breaking the acceptance criterion's ordering check.
- **Fix:** Reworded the affected comments to describe the same information without repeating the exact Russian button labels (same class of self-inflicted grep near-miss documented in Plan 40-10's and 40-15's own summaries).
- **Files modified:** `ui/src/features/devices/DeviceContextMenu.svelte`
- **Verification:** `grep -n "Просмотр\|Редактировать" ui/src/features/devices/DeviceContextMenu.svelte` now returns exactly the two menu-item button lines, «Просмотр» first.
- **Committed in:** `2a0d6b50` (part of Task 2 commit)

---

**Total deviations:** 1 auto-fixed (blocking, comment-wording only — no behavior change).
**Impact on plan:** None — purely a comment-wording fix to satisfy the plan's own literal grep-based acceptance criteria.

## Issues Encountered

None beyond the grep-wording fix above.

## Verification Reality

- **Mechanically verified:** `pnpm --dir ui svelte-check` (0 errors, same 60 pre-existing warnings baseline as Plan 40-15, none attributable to this plan's files), `pnpm --dir ui lint` (eslint + prettier + all 7 `check-*.mjs` gates, including `check-privacy`), and `pnpm --dir ui build` (production build succeeds, no compiler errors from the restructured `{#if}/{:else}` chain in `PlaceEntityViewModal.svelte` or the new markup in `DeviceContextMenu.svelte`) all green after both tasks. The Task 1 acceptance-criteria greps (`MovementTimeline` at least 2 lines, `Spinner` count unchanged) and Task 2's (`Просмотр` before `Редактировать`) all pass exactly as specified.
- **UNVERIFIED (per this plan's explicit `<verification_reality>` constraint):** `MovementTimeline` has still NOT been observed actually rendering inside the real running app. This environment has no AD/printer-reachable network and no authenticated session was exercised — I did not spin up `cargo tauri dev` or the axum server with a logged-in session and click through to a device's «Просмотр» view to visually confirm the timeline section renders without a Svelte 5 rune runtime error. `svelte-check`, `eslint`, and `pnpm build` all passing is a stronger signal than Plan 40-15 had (this is the first time the component is actually compiled as part of a real parent's template, and the compiler accepted the prop wiring and the modified `{#if}/{:else}` structure), but it is explicitly NOT proof of correct runtime behavior — per this project's own documented lesson ("Synthetic harness not verification"), a green compile is not equivalent to a rendered page. Stating plainly: **the actual render of the timeline section, in both `PlaceEntityViewModal` (device/printer) and via the new `DeviceContextMenu` entry point, is UNVERIFIED.**
- The next real opportunity to verify this is either a manual UAT pass in the running Tauri app / LAN browser, or Plan 40-17's own execution when `MovementTimeline` gets its second and third mount points (`CartridgeDetail`/`PrinterDetail`) — if a rune error exists, it is very likely to surface there too and should not be assumed absent just because this plan's gates were green.

## Known Stubs

None — both the timeline fetch and the new entry point are fully wired to real backend calls (`place_movements_get_timeline`) and real data (`DeviceDto`/list-row fields); no hardcoded empty values or placeholder text introduced.

## Threat Flags

None. This plan introduces no new endpoints, auth paths, or schema changes — it consumes Plan 40-10's already-gated `place_movements_get_timeline` read endpoint from a new UI surface. T-40-33 (Employee UI showing this content) was already accepted in this plan's own `<threat_model>` as a UI-cosmetic-only concern, with the real gate enforced server-side by Plan 40-10/40-14 — no new mitigation was needed or added.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- `PlaceEntityViewModal.svelte` and `DeviceContextMenu.svelte` are both done for HST-02's device/printer surface; Plan 40-17 (cartridge/printer detail wiring in `CartridgeDetail.svelte`/`PrinterDetail.svelte`) is the next and final consumer of `MovementTimeline.svelte`.
- Flag for the phase verifier / Plan 40-17: consider whether `PlaceEntityViewModal`'s shared `loadError` (rather than a per-section flag) is the intended final behavior for a timeline-only failure, given UI-SPEC's States table describes distinct copy for that case. As implemented here it is a plan-directed simplification, not a bug, but worth an explicit accept/revisit decision before phase close.
- First real runtime verification of `MovementTimeline` mounted in a live parent should happen via manual UAT in the running app (Tauri desktop or LAN browser with a build serving `ui/dist`), or as a natural byproduct of Plan 40-17's own execution.

---
*Phase: 40-movement-history*
*Completed: 2026-09-02*

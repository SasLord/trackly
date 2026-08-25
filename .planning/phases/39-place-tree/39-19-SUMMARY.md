---
phase: 39-place-tree
plan: 19
subsystem: ui
tags: [svelte, svelte5-runes, place-tree, modal, ru-pluralization]

# Dependency graph
requires:
  - phase: 39-place-tree plan 12
    provides: "places_create/places_rename/places_move/places_subtree_stats commands + PlaceDto/PlaceNewDto/SubtreeStatsDto bindings.ts types these modals consume"
  - phase: 39-place-tree plan 13
    provides: "PlacePicker.svelte — reusable place-selection control embedded in both modals as the Родительское место / Новое родительское место field"
provides:
  - "PlaceFormModal.svelte — create/rename modal per UI-SPEC §11.1-§11.2: full field set (Название/Тип/Родительское место/Уровень/Складское место/Порядок) in create mode, Название-only in rename mode (backend rationale below), D-01 child-type suggestion, D-04 duplicate-name inline error"
  - "PlaceMoveModal.svelte — move modal per UI-SPEC §11.3: places_subtree_stats-driven consequences callout with Russian one/few/many pluralization and zero-part omission, D-21 cycle-error inline mapping from the server's AppError"
  - "Settled prop contract for both modals (mode/place/defaultParentId/onClose/onSaved and place/onClose/onMoved) — Plan 14's ActionMenu wires directly to these without needing a same-wave file-existence race"
affects: [39-14 (Wave 8 tree UI wires its ActionMenu to these two modals and to PlacesPage's primary "Создать место" button)]

tech-stack:
  added: []
  patterns:
    - "Russian pluralization (ruPlural: one/few/many by mod10/mod100) duplicated in PlaceMoveModal.svelte's TS rather than imported from the backend — no shared frontend/backend string-formatting layer exists in this codebase, so the pattern (not the code) was mirrored from place_service.rs's own `ru_plural`/`build_delete_blocked_message` helpers, confirmed via reading that file, to guarantee identical wording for identical counts across the delete-blocked message (backend-rendered) and the move-consequences callout (frontend-rendered)."
    - "No `open: boolean` prop on either modal — both plan-specified prop contracts omit it, so the caller conditionally mounts/unmounts ({#if}) rather than toggling visibility; each mount is a fresh, single-use form instance. Diverges from CartridgeFormModal's `open`+`openInstanceCounter` remount-on-reopen pattern, but matches the plan's own literal `<interfaces>` block, which Plan 14 depends on verbatim."
    - "Cycle detection is 100% server-round-trip, not client pre-validated: PlacePicker (Plan 13) has no 'exclude this subtree' prop, so a client-side ancestor walk would require an extra bespoke fetch loop duplicating the server's own CTE-based check. The modal instead submits and maps the server's `AppError::Validation{field:'parent_id'}` response inline, guaranteeing the copy can never drift from `places_sqlite.rs`'s literal string."

key-files:
  created:
    - ui/src/features/places/PlaceFormModal.svelte
    - ui/src/features/places/PlaceMoveModal.svelte
  modified: []

key-decisions:
  - "Rename mode renders ONLY the «Название» field, not the full §11.1 field table. Reason: `places_rename(id, name, version)` (Plan 12) is the sole mutation endpoint reachable in rename mode and only accepts `name` — there is no `places_update` for kind/parent/level/is_storage/sort_order once a place exists. Rendering those fields as editable in rename mode would silently discard any edits on submit (a broken/misleading control, Rule 1's bug-avoidance clause), not a genuine UI-SPEC contradiction — §11.1's shared field table describes create-time fields, and D-01/D-02 (kind is chosen once, at creation) supports treating post-creation kind/level/storage/order as immutable via this modal. Moving a place uses PlaceMoveModal; there is no UI path (and no backend path) to change a place's kind after creation."
  - "D-01's child-type suggestion map extends UI-SPEC's two literal examples (building→floor, floor→room) with four more pairs (territory→zone, zone→building, room→room, outdoor→outdoor) to cover all six kinds — UI-SPEC only specifies the two, so the remaining four are this component's own 'typical next level down' judgment call, documented inline in the component. Suggestion-only per D-01: any of the six values remains selectable regardless of parent kind, verified via `isGroupSelected`/`onPickGroup` allowing free selection."
  - "Move-consequences ordering follows §11.3's literal example text (nested places before devices: '3 вложенных места и 47 устройств'), which is the OPPOSITE clause order from the backend's own D-14 delete-blocked message (`build_delete_blocked_message`: devices before nested places). Followed literally per-spec rather than made consistent with the backend precedent, since §11.3's copy is explicitly locked. `cartridge_count` is added as an unspecified third clause (not in §11.3's example) mirroring the backend's own Rule 2 rationale for the delete-blocked message — a place holding only cartridges must not produce an empty consequences sentence."
  - "PlaceMoveModal's «Переместить» button is disabled while no target is selected (`selectedParentId === null`) — not explicitly required by UI-SPEC §11.3's copy, but a Rule 2 completeness guard: without it, clicking submit before selecting a destination would either no-op silently or send `newParentId: null` (move-to-root), which is a real, unintended destination, not a safe default for an unfilled required field."

requirements-completed: [PLC-01, PLC-02]

# Metrics
duration: ~35min
completed: 2026-08-25
---

# Phase 39 Plan 19: Place mutation modals — create/rename + move Summary

**`PlaceFormModal.svelte` (create/rename, D-01 type suggestion, D-04 duplicate-name inline error) and `PlaceMoveModal.svelte` (subtree-stats-driven consequences callout with Russian pluralization, D-21 cycle-error inline mapping) — the two place-mutation modals Plan 14's tree `ActionMenu` will invoke; both compile/lint/build cleanly, runtime UNVERIFIED (see below).**

## Performance

- **Duration:** ~35 min
- **Started:** 2026-08-25 (est.)
- **Completed:** 2026-08-25T01:57:00Z
- **Tasks:** 2/2
- **Files modified:** 2 (2 created)

## Accomplishments

- `PlaceFormModal.svelte` (366 lines) — single form component for both `mode='create'` and `mode='rename'`. Create mode renders the full §11.1 field set (Название/Тип/Родительское место/Уровень/Складское место/Порядок); rename mode renders only Название (see key-decisions for why). D-01 child-type suggestion fetches the selected parent's `kind` via `places_get` and pre-fills Тип until the user manually picks a value; Уровень is visible only when Тип === «Этаж», accepts 0 and negative values without a range error, and client-rejects non-integer input with §14.3's exact copy («Уровень этажа — целое число. Подвал — отрицательное значение.»). D-04's duplicate-name server error (`AppError.details.field === 'name'`) is mapped inline under Название with the server's own message text, not a client-duplicated string.
- `PlaceMoveModal.svelte` (243 lines) — on open, fetches `places_subtree_stats(rootId)` and renders the §11.3 warning-callout using the exact CSS block from the spec (`--tr-warning-soft`/`border-left: 3px solid var(--tr-warning)`/`--tr-warning-text`). Consequences text implements Russian one/few/many pluralization (mirroring the backend's own `ru_plural` helper) with zero-count clauses omitted and no callout at all when the subtree is completely empty — verified against all three of §11.3's literal examples (3 places + 47 devices; 0 places + 47 devices; 1+1). Cycle-rejection error (`AppError.details.field === 'parent_id'`) is rendered inline verbatim under the `PlacePicker`, and the modal does not close on that error.
- Both modals embed `PlacePicker` (Plan 13) with zero fetch-injection props — real wire-backed defaults, per that plan's documented consumer contract.

## Task Commits

Each task was committed atomically:

1. **Task 1: PlaceFormModal.svelte — create/rename** - `3c4dea07` (feat)
2. **Task 2: PlaceMoveModal.svelte — consequences preview + cycle error** - `d1bc4938` (feat)

## Files Created/Modified

- `ui/src/features/places/PlaceFormModal.svelte` - create/rename modal (366 lines)
- `ui/src/features/places/PlaceMoveModal.svelte` - move modal with consequences callout (243 lines)

## Decisions Made

See `key-decisions` in frontmatter for full rationale on: (1) rename mode showing only «Название» — the only field `places_rename` can actually persist; (2) extending UI-SPEC's two D-01 suggestion examples to all six kind pairs; (3) following §11.3's literal (nested-places-first) clause order even though it differs from the backend's own delete-blocked-message order, plus adding an unspecified `cartridge_count` third clause for completeness; (4) disabling «Переместить» until a target is selected.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug avoidance] Rename mode does not render Тип/Родительское место/Уровень/Складское место/Порядок**
- **Found during:** Task 1, reading `PlaceService::rename`'s actual signature (Plan 12's `places_rename(id, name, version)`)
- **Issue:** UI-SPEC §11.1's field table is written as one shared table for "Создание / переименование", which read literally would mean rendering all six fields as editable in rename mode too. But `places_rename` only accepts `id`/`name`/`version` — there is no `places_update` endpoint for kind/parent/level/is_storage/sort_order post-creation. Rendering those fields as editable controls in rename mode would be dead UI: any edit to them would be silently discarded on submit (the payload sent to `places_rename` never includes them), which is a broken/misleading control, not a cosmetic gap.
- **Fix:** `PlaceFormModal` renders only the Название field when `mode === 'rename'`; the other five fields render only in create mode (`{#if !isRename}` block).
- **Files modified:** `ui/src/features/places/PlaceFormModal.svelte`
- **Verification:** Read `crates/trackly-app/src/services/place_service.rs`'s `rename` method and `crates/trackly-app/src/tauri_cmds/places.rs`/`http/places.rs`'s `places_rename` wrappers directly — confirmed no other mutable fields exist on that path. `svelte-check`/`eslint`/`pnpm build` clean.
- **Committed in:** `3c4dea07` (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 — avoided building dead UI for fields the backend cannot persist post-creation). No architectural changes. Every `must_haves` truth and artifact from the plan frontmatter is satisfied by the code as committed — including the D-09 pattern truth ("создать вложенный узел-склад со своим is_storage=true через обычное «Создать вложенное место»"), which requires no special UI: `PlaceFormModal`'s create mode already exposes `is_storage` as a plain Checkbox for any nested node.

## Issues Encountered

None. Both files compile (`svelte-check`: 37 pre-existing errors / 54 warnings, unchanged baseline aside from 4 expected `state_referenced_locally` warnings on `PlaceFormModal.svelte`'s initial-value `$state` reads from props — the same accepted pattern already present in `DeviceFormBody.svelte`/`CartridgeFormBody.svelte`/`ModelFormModal.svelte`), lint clean (`eslint` on both new files: 0 problems), and `pnpm --dir ui build` succeeds.

**Runtime behavior is UNVERIFIED** (project convention: compile/lint/build gates are not runtime verification, per this plan's own `<verification>` block, which explicitly defers manual verification to Plan 14's end-to-end checkpoint once the tree's `ActionMenu` exists to open these modals). Neither modal has been opened in a real webview or LAN browser. This is not a claim of "tested and working" — only "compiles, lints, and builds cleanly."

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

Both mutation modals are feature-complete per UI-SPEC §11.1-§11.3 and ready for Plan 14 (Wave 8) to wire into the tree's `ActionMenu` and PlacesPage's primary "Создать место" button, using the exact prop contract from this plan's `<interfaces>` block (`PlaceFormModal`: `mode`/`place`/`defaultParentId`/`onClose`/`onSaved`; `PlaceMoveModal`: `place`/`onClose`/`onMoved`). Runtime verification of both (field visibility toggling, D-01 suggestion, D-04/cycle inline errors, pluralization edge cases) should happen as part of Plan 14's own end-to-end checkpoint, per this plan's `<verification>` block — recommend adding those specific manual-check steps to `deferred-items.md` alongside Plan 13's already-deferred `PlacePicker` checklist so both land in the same batched UAT pass (39-20/39-21).

---
*Phase: 39-place-tree*
*Completed: 2026-08-25*

## Self-Check: PASSED

All created files (`ui/src/features/places/PlaceFormModal.svelte`,
`ui/src/features/places/PlaceMoveModal.svelte`, this SUMMARY) confirmed present on disk.
Both task commit hashes (`3c4dea07`, `d1bc4938`) confirmed present in `git log`.

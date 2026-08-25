---
phase: 39-place-tree
plan: 14
subsystem: ui
tags: [svelte, svelte5-runes, place-tree, aria-tree, drag-and-drop, routing]

# Dependency graph
requires:
  - phase: 39-place-tree plan 12
    provides: "12 places_* Tauri/HTTP commands (places_list_all/places_search/places_subtree_stats/places_archive/places_unarchive/places_delete/...) this tree fetches and mutates against"
  - phase: 39-place-tree plan 13
    provides: "PlacePicker.svelte — embedded (indirectly) via PlaceFormModal/PlaceMoveModal's own parent-selection fields"
  - phase: 39-place-tree plan 19
    provides: "PlaceFormModal.svelte (create/rename) and PlaceMoveModal.svelte (move + consequences callout) — this plan is their first real consumer, wired via ActionMenu"
provides:
  - "/places route + sidebar entry ('Места', immediately after 'Карта', Admin+Manager per D-19/D-20) + PlacesPage.svelte (PageHeader, Admin-only 'Создать место', deep-link hash contract #/places?id=...)"
  - "PlacesMasterDetail.svelte — literal copy of RequestsMasterDetail.svelte's grid/panel structure (UI-SPEC §6.2)"
  - "PlaceTree.svelte / PlaceTreeNode.svelte — role=tree/treeitem/group with roving tabindex, full UI-SPEC §8.5 keyboard map, client-side D-05 sibling sort (ported from trackly-core::domain::places::sibling_cmp), D-25 lazy per-visible-node content counters, search mode, ActionMenu wiring to PlaceFormModal/PlaceMoveModal + inline delete/archive confirms, D-21 native HTML5 drag-drop that always opens PlaceMoveModal, D-03 'В корень дерева' drop zone"
  - "PlaceMoveModal.svelte extended with an optional defaultParentId prop (Rule 1 fix, not in Plan 19's original contract) — makes drag-drop pre-fill AND the D-03 root-move flow reachable at all"
affects: [39-20 (right-panel PlaceContents replaces the static detail placeholder this plan ships), 39-21 (phase-closing checkpoint — this plan's deferred-items.md entry is part of that batched UAT pass)]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Whole-tree-in-one-fetch, not lazy-per-branch: PlaceTree fetches the complete flat list via places_list_all once (and on toggle/refresh), builds the parent->children map and D-05 sort client-side, and only ever re-fetches per-node data for the D-25 content counter (places_subtree_stats), lazily, once per node the FIRST time it becomes visible, cached thereafter in a parent-owned Record<number, number> keyed by place id. This is a deliberate departure from PlacePicker's genuinely-lazy children-per-node pattern (Plan 13) — justified by T-39-14-02's confirmed real scale (~300 rows, a single full-tree fetch is trivial) and by the plan's own action text ('do not call the backend for sorting... fetch full tree via places_list_all')."
    - "Recursive component via self-import (PlaceTreeNode imports itself), not <svelte:self> — renders a real nested role=\"group\" wrapper per expanded node with children, matching §8.5's literal ARIA/DOM contract. All keyboard-navigation LOGIC (which id is 'next visible', Home/End targets, expand/collapse-then-refocus) lives in PlaceTree.svelte's own flattened `visibleNodes` array, computed independently of the recursive DOM — arrow-key handling moves focus by DOM id lookup (`place-tree-row-{id}`), not by walking the component tree."
    - "Single shared `actions` object (TreeActions interface) threaded through the recursion instead of ~13 individual callback props per level — keeps the recursive prop list stable and readable; PlaceTree owns all mutation/drag state, PlaceTreeNode is purely a renderer + event-forwarder."
    - "Minimal inline confirm for delete/archive (Modal-based, {#if}-gated state objects mounted directly in PlaceTree.svelte) rather than extending PlaceFormModal with a mode=\"delete\" variant — Plan 19's key-decision explicitly closed that door (places_rename is the only post-creation mutation PlaceFormModal wraps); the plan's own action text names this inline-confirm fallback as the sanctioned alternative."

key-files:
  created:
    - ui/src/features/places/PlacesPage.svelte
    - ui/src/features/places/PlacesMasterDetail.svelte
    - ui/src/features/places/PlaceTree.svelte
    - ui/src/features/places/PlaceTreeNode.svelte
  modified:
    - ui/src/routes.ts
    - ui/src/features/layout/sidebar-config.ts
    - ui/src/features/places/PlaceMoveModal.svelte
    - ui/eslint.config.js

key-decisions:
  - "PlaceMoveModal.svelte gained an optional `defaultParentId` prop (undefined by default, preserving Plan 19's original ActionMenu 'Переместить в…' behavior exactly). This plan's drag-drop needs to pre-fill the destination (explicit plan requirement), and the D-03 root-move drop zone needs to pre-fill it to `null` — Plan 19's original submit-guard (`selectedParentId === null` = disabled) could not tell 'root, deliberately chosen' apart from 'nothing chosen yet', making the root-move path structurally unreachable through the UI regardless of what triggered the modal. Replaced the guard with a `targetChosen` boolean derived from whether the prop was passed at all (by identity, not by value), tracked as a Rule 1 bug fix (this exact D-03 flow is a must_haves truth for this plan) rather than an architectural change — no new component, no new backend call, purely an internal state-model correction to an existing modal."
  - "Content counters (D-25) are fetched lazily per node the first time it becomes VISIBLE (root nodes on load; a branch's children the first time it's expanded), not eagerly for the whole ~300-row tree on load and not deferred all the way to the individual PlaceTreeNode component (which would refetch on every collapse/re-expand since {#if}-gated children remount). Caching lives in PlaceTree.svelte's own `statsCache`, persists across expand/collapse toggling, and is passed down as a plain prop — chosen over the alternative of importing PlacePicker's per-node-owns-its-own-fetch pattern because that pattern refetches on every remount, which for a frequently-toggled branch would be wasteful."
  - "UI-SPEC §13's 'при раскрытии ветки — 16px Spinner в строке узла' (branch-expand spinner) is NOT implemented, because it does not apply to this design: the whole tree structure is fetched in ONE call up front (per Task 2's own action text), so 'expand' is an instant client-side visibility toggle with nothing to wait on. The single center-panel Spinner during the initial `places_list_all` fetch is the only loading state the tree actually has. Followed the plan's literal, more specific action text over the general §13 loading-state table entry, which reads as carried over from PlacePicker's genuinely-lazy design."
  - "'Показать содержимое' in the D-14 delete-blocked callout (§11.5) currently only selects the blocked node (updates selection + hash) — it does NOT yet force 'Только здесь' off, because that toggle is owned by Plan 20's PlaceContents component, which does not exist yet in this plan. The selection side-effect is the meaningful part available now; documented as a deferred-items.md UAT item rather than silently doing nothing."

requirements-completed: [PLC-01, PLC-02, PLC-06]

# Metrics
duration: ~65min
completed: 2026-08-25
---

# Phase 39 Plan 14: Места section shell — routing, master-detail, tree Summary

**`/places` route + sidebar entry + `PlacesMasterDetail`/`PlaceTree`/`PlaceTreeNode` — a full `role=tree` place hierarchy (client-ported D-05 sort, D-25 lazy counters, complete §8.5 keyboard/ARIA map, search mode) wired to Plan 19's mutation modals via `ActionMenu`, plus native HTML5 drag-drop that always confirms through `PlaceMoveModal` (extended with a `defaultParentId` prop to make the D-03 root-move flow reachable at all); compiles/lints/builds cleanly, runtime UNVERIFIED (see below).**

## Performance

- **Duration:** ~65 min (est.)
- **Started:** 2026-08-25T05:45:00Z (est.)
- **Completed:** 2026-08-25T06:50:00Z
- **Tasks:** 2/2
- **Files modified:** 8 (4 created, 4 modified)

## Accomplishments

- `/places` route registered immediately after `/map`; sidebar entry "Места" inserted in the same position, gated to `admin`/`manager` (D-19/D-20); `PINNED` comment updated (12 items + 4 dividers = 16 entries).
- `PlacesMasterDetail.svelte` — literal, unmodified copy of `RequestsMasterDetail.svelte`'s grid/panel CSS (35%/65%, `--tr-space-md` gap, `--tr-surface-raised`/`--tr-border`/`--tr-radius-md`/`--tr-elev-1` panels, 320px/480px min-widths, <1099px fallback) per UI-SPEC §3/§6.2's explicit "not recalculated" instruction.
- `PlacesPage.svelte` — `PageHeader title="Места"` with an Admin-only primary "Создать место" button (opens `PlaceFormModal mode="create"`), deep-link hash contract (`#/places?id=…`) read once on mount and written via `history.replaceState` (no extra history entries, no router remount) whenever the tree's selection changes, and a static "Место не выбрано" (§14.2) `DetailPanel` placeholder that Plan 20 will replace with real content.
- `PlaceTree.svelte` (role="tree", ~830 lines incl. styles) — fetches the whole tree in one `places_list_all` call; client-side D-05 sort (`sibling_cmp`/`natural_name_cmp` ported verbatim from `trackly-core::domain::places::sibling_cmp`); full §8.5 keyboard map (↑↓/→←/Home/End/Enter/F2/Escape) via roving tabindex over a flattened visible-node list; search mode (`places_search`, flat full-path results, 200ms debounce); D-25 content counters fetched lazily per visible node via `places_subtree_stats`, cached, zero-suppressed; `aria-live="polite"` announcements; toolbar ("Показывать архивные" default-off, "Обновить"); `ActionMenu` wiring (Переименовать/Создать вложенное место/Переместить в…/Архивировать↔Вернуть из архива/Удалить) to `PlaceFormModal`/`PlaceMoveModal` plus a minimal inline delete confirm (§11.5, incl. the D-14 blocked-delete callout with "Показать содержимое"/"Архивировать" fallback buttons) and inline archive/unarchive confirm (§11.4); native HTML5 drag-drop (Admin-only, "внутрь узла" only, self/descendant rejected per D-21) that ALWAYS opens `PlaceMoveModal` pre-filled with the drop target, plus the D-03 "В корень дерева" drop zone.
- `PlaceTreeNode.svelte` (~330 lines incl. styles) — recursive 32px row (`role="treeitem"` + `role="group"` children wrapper matching §8.5's literal DOM contract), chevron/name/"Склад"/"Архив" badges/counter/`ActionMenu` layout per §8.2, mandatory per-row `label={`Действия: ${node.name}`}`, drag visual states (dragging/valid-target/invalid-target).

## Task Commits

Each task was committed atomically:

1. **Task 1: routes.ts + sidebar-config.ts + PlacesPage.svelte + PlacesMasterDetail.svelte** - `a8ee0a2c` (feat)
2. **Task 2: PlaceTree.svelte + PlaceTreeNode.svelte — role=tree, roving tabindex, sort, counters** - `85374a01` (feat)

## Files Created/Modified

- `ui/src/features/places/PlacesPage.svelte` - section root: header, deep link, create modal
- `ui/src/features/places/PlacesMasterDetail.svelte` - literal grid/panel copy of RequestsMasterDetail.svelte
- `ui/src/features/places/PlaceTree.svelte` - the tree itself (data, sort, search, keyboard/ARIA, mutation wiring, drag-drop)
- `ui/src/features/places/PlaceTreeNode.svelte` - recursive row renderer
- `ui/src/routes.ts` - `/places` -> `PlacesPage`
- `ui/src/features/layout/sidebar-config.ts` - "Места" sidebar entry
- `ui/src/features/places/PlaceMoveModal.svelte` - added `defaultParentId` prop + `targetChosen` fix (see Deviations)
- `ui/eslint.config.js` - added missing `DragEvent` browser global

## Decisions Made

See `key-decisions` in frontmatter for full rationale on: (1) the `PlaceMoveModal` `defaultParentId`/`targetChosen` fix and why it's a bug fix, not scope creep; (2) the whole-tree-fetch + lazy-per-visible-node-counter data strategy and how it differs deliberately from PlacePicker's fully-lazy pattern; (3) why §13's branch-expand spinner doesn't apply to this design; (4) the current, partial scope of "Показать содержимое" in the delete-blocked callout pending Plan 20.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `PlaceMoveModal` could not reach the D-03 root-move destination**
- **Found during:** Task 2, while wiring the "В корень дерева" drop zone
- **Issue:** Plan 19's `PlaceMoveModal` disables "Переместить" via `selectedParentId === null`, treating `null` as "nothing picked yet". But `null` is also the correct value for "move to root" (D-03) — there was no way to distinguish an explicit root choice from an unfilled field, so the root-move flow (required by this plan's own drag-drop must_haves) could never enable its submit button through any UI path.
- **Fix:** Added an optional `defaultParentId` prop and a `targetChosen` boolean (true iff the prop was passed, by identity, regardless of its value, or the user has interacted with `PlacePicker`). Submit-disable and the submit guard now check `!targetChosen` instead of `selectedParentId === null`. The ActionMenu "Переместить в…" path (which passes no `defaultParentId`) is unaffected — `targetChosen` starts `false` there, exactly as before.
- **Files modified:** `ui/src/features/places/PlaceMoveModal.svelte`
- **Verification:** `svelte-check`/`eslint`/`pnpm build` clean; code-reviewed the guard logic against both call sites (`onMove` with no prop vs. `onDropNode`/root-dropzone with an explicit prop). Runtime (does the button actually enable and does the move actually land at root) is UNVERIFIED — see deferred-items.md.
- **Committed in:** `85374a01` (Task 2 commit)

**2. [Rule 3 - Blocking] Missing `DragEvent` ESLint global**
- **Found during:** Task 2, running `eslint` on `PlaceTree.svelte`
- **Issue:** `eslint.config.js`'s `browserGlobals` list had `KeyboardEvent`/`MouseEvent`/`FocusEvent` but not `DragEvent` — this plan is the first code in the codebase to need native HTML5 drag-drop types, so the gap was latent until now. `no-undef` failed on `DragEvent` in the root-drop handler's parameter type.
- **Fix:** Added `DragEvent: 'readonly'` to the same `browserGlobals` object, alongside the other DOM event-type globals.
- **Files modified:** `ui/eslint.config.js`
- **Verification:** `eslint` clean on all touched files after the addition.
- **Committed in:** `85374a01` (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (1× Rule 1, 1× Rule 3). Both were required for this plan's own stated must_haves (D-03 root-move reachability; a compiling/lintable drag-drop implementation) — no scope creep, no architectural changes.

## Issues Encountered

None blocking. `svelte-check` baseline before this plan: 0 errors / 54 warnings (per 39-19-SUMMARY.md). After this plan: **0 errors / 56 warnings** — the +2 are the expected `state_referenced_locally` warnings on `PlaceMoveModal.svelte`'s new `defaultParentId` prop, the same accepted "reads a prop at construction time, correct because the component is freshly mounted per open" pattern already documented for `PlaceFormModal.svelte` (39-19) and `DeviceFormBody.svelte`/`CartridgeFormBody.svelte`/`ModelFormModal.svelte` elsewhere in the codebase. Two NEW a11y warnings introduced by this plan's own markup (`role="tree"` missing tabindex; a `dragover`/`drop` div missing a role) were fixed inline (tabindex="-1" on the tree container; `role="button"` + `aria-label` on the root drop zone) rather than left as baseline drift.

**Mount contract honored** (deferred-items.md "PlaceFormModal mount contract", written for this plan specifically): both `PlaceFormModal` and `PlaceMoveModal` are mounted inside `{#if formModal}`/`{#if moveModal}` blocks in `PlaceTree.svelte`, giving each open a fresh component instance built from the current target node's props — exactly the contract Plan 19 documented as a hard requirement, not the "keep mounted + toggle" pattern that would have silently shown stale data.

**Runtime behavior is UNVERIFIED.** Per this plan's own `<verification>` block, full interactive verification (keyboard, drag-drop, role gating for Manager vs Admin, deep link, Tauri + LAN browser parity) is explicitly deferred to Plan 20's end-of-wave checkpoint, once the right panel exists to complete the screen. Only `svelte-check`/`eslint`/`node scripts/check-tokens.mjs`/`check-contrast.mjs`/`check-focus-outline.mjs`/`pnpm --dir ui build` ran. A detailed, numbered manual-verification checklist (routing/sidebar/master-detail, tree structure/sort/counters, full keyboard/ARIA map, ActionMenu-to-modal wiring including the D-14 blocked-delete callout, drag-drop including the fixed root-move path, and LAN-browser parity) has been appended to `.planning/phases/39-place-tree/deferred-items.md` under "Plan 14 — PlaceTree/PlaceTreeNode runtime verification NOT performed" for the batched UAT pass at 39-20/39-21.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

The "Места" section shell is feature-complete per this plan's must_haves: routing, sidebar gating, master-detail layout, and a fully-wired left-panel tree (structure, sort, counters, keyboard/ARIA, search, all five `ActionMenu` mutation paths, drag-drop) all compile/lint/build cleanly. Plan 20 (Wave 9) can now build `PlaceContents.svelte` and swap it into `PlacesPage.svelte`'s static detail-panel placeholder — the selection state (`selectedPlace`/hash sync) and the D-14 "Показать содержимое" selection side-effect are already wired to receive it. Before either plan is considered verified, the deferred runtime-verification checklists from Plans 13/19 (PlacePicker, PlaceFormModal, PlaceMoveModal) AND this plan's own new checklist should be run together in one real-webview pass, per the coordinator's guidance in `deferred-items.md`.

---
*Phase: 39-place-tree*
*Completed: 2026-08-25*

## Self-Check: PASSED

All created/modified files confirmed present on disk (`PlacesPage.svelte`, `PlacesMasterDetail.svelte`,
`PlaceTree.svelte`, `PlaceTreeNode.svelte`, `routes.ts`, `sidebar-config.ts`, `PlaceMoveModal.svelte`,
`eslint.config.js`, this SUMMARY, `deferred-items.md`). Both task commit hashes (`a8ee0a2c`, `85374a01`)
confirmed present in `git log`.

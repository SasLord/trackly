---
phase: 39-place-tree
plan: 13
subsystem: ui
tags: [svelte, svelte5-runes, place-tree, combobox, accessibility]

# Dependency graph
requires:
  - phase: 39-place-tree plan 12
    provides: "12 places_* Tauri/HTTP commands + PlaceDto/PlaceNewDto/PlacePathDto/SubtreeStatsDto/PlaceContentDto bindings.ts types this component consumes"
  - phase: 39-place-tree plan 05
    provides: "PlaceService mutation half (create/rename/move/archive/unarchive/delete) — reachable indirectly via places_create through this component's D-18 quick-create"
  - phase: 39-place-tree plan 08
    provides: "PlaceService read half (get/list_children/search) — the three read endpoints this component's default fetchers call"
provides:
  - "PlacePicker.svelte — the single reusable place-selection control (39-UI-SPEC.md §10, D-17): field + lazy-loaded tree-mode panel + debounced search-mode panel + D-18 Admin-only quick-create row + D-15 archived-value exception + two-stage Escape (§10.3)"
  - "Injection-prop contract (fetchChildren/fetchSearchResults/fetchOne/createPlace) — every future PlacePicker consumer (Plans 15-19) gets the default apiCall-backed behavior for free by omitting these props; the showcase/tests override them to avoid live API calls"
  - "PlacePickerSection.svelte — component-showcase demo entry with an invented tree, registered in ShowcasePage.svelte"
affects: [39-14, 39-15, 39-16, 39-17, 39-18, 39-19 (every remaining Phase 39 UI plan wires this component into a real form or the Places section tree)]

tech-stack:
  added: []
  patterns:
    - "PlacePicker owns its own data-fetching (unlike Dropdown.svelte's zero-fetch, caller-supplied-groups design) because the place tree has arbitrary nesting (D-01) that Dropdown's two-level drill-in cannot express (UI-SPEC §6.2). To let the showcase demo tree/search behavior without a live API call or seeded DB rows (project privacy rule), the four fetch operations are exposed as optional props (fetchChildren/fetchSearchResults/fetchOne/createPlace), each defaulting to the real apiCall('places_list_children'|'places_search'|'places_get'|'places_create', ...) call. Every real form consumer (Plans 15-19) should omit these props entirely and get the wire-backed default; only showcase/tests should pass overrides."
    - "Tree/expansion state kept as plain $state Records/arrays (childrenCache: Record<number, PlaceDto[]>, expandedIds: number[]), not Map/Set — Svelte 5's $state deep-reactivity is well-established for plain objects/arrays in this codebase (Dropdown.svelte's Record<number,T> row-indexed pattern) but Map/Set need the separate SvelteMap/SvelteSet reactivity classes (not yet used anywhere in this codebase); reassigning a fresh object/array on every mutation avoided that footgun without introducing a new import."
    - "Leaf-vs-expandable is NOT known ahead of a fetch — PlaceDto carries no has_children flag. Chevron slot renders optimistically (assume-expandable) until the node's children are fetched at least once and come back empty (isLeafKnown()), at which point the chevron is dropped for that node. This is a client-side lazy-tree convention, not a spec requirement; no backend change was made or needed."

key-files:
  created:
    - ui/src/lib/components/PlacePicker.svelte
    - ui/src/features/showcase/sections/PlacePickerSection.svelte
  modified:
    - ui/src/features/showcase/ShowcasePage.svelte

key-decisions:
  - "Task 1's commit (d65a1d3d) contains the COMPLETE component — tree mode AND search mode AND the D-18 create-row AND the two-stage Escape — not just the tree-mode slice the plan's Task 1 text describes. Tree and search share one internal state machine (`mode: 'closed' | 'tree' | 'search'`), and UI-SPEC §10.3's two-stage Escape contract spans both modes (first Escape: search→tree; second: tree→closed). A tree-only intermediate commit would have shipped a component whose own keyboard contract was internally incomplete/inconsistent (Escape only half-implemented) rather than a working subset. Task 2's commit (83a83cd2) therefore adds only what was genuinely separable: the showcase entry."
  - "Showcase registration landed in ui/src/features/showcase/ShowcasePage.svelte, not ui/src/pages/ComponentShowcasePage.svelte as the plan's files_modified frontmatter stated. ComponentShowcasePage.svelte is a one-line wrapper (`<ShowcasePage />`) with no section list of its own; every other showcase section (Buttons/Fields/Badge/Tabs/Modal/Table/Dropdown) is registered in ShowcasePage.svelte's <script> import list + template. Registered PlacePickerSection there instead — same visible outcome (section appears on /showcase), corrected file target."
  - "Added four optional injection props to PlacePicker's Props interface beyond the plan's literal text (value/onChange/id/disabled/invalid only): fetchChildren?, fetchSearchResults?, fetchOne?, createPlace? — each typed to the exact apiCall signature it defaults to (fetchChildren: (parentId: number | null) => Promise<PlaceDto[]>; fetchSearchResults: (query: string) => Promise<PlacePathDto[]>; fetchOne: (placeId: number) => Promise<PlaceDto>; createPlace: (place: PlaceNewDto) => Promise<PlaceDto>). Defaults call apiCall('places_list_children'|'places_search'|'places_get'|'places_create', ...) respectively — identical to what every real consumer needs, so Plans 15-19 can ignore these props entirely. They exist solely so PlacePickerSection.svelte (and any future component test) can supply an invented in-memory tree instead of hitting the real API/DB, honoring the project's hard privacy rule (no seeded demo data in a public-repo showcase)."

requirements-completed: [PLC-03]

# Metrics
duration: ~50min
completed: 2026-08-25
---

# Phase 39 Plan 13: PlacePicker — единый контрол выбора места Summary

**Built `PlacePicker.svelte` (field + lazy tree panel + debounced search panel + D-18 admin-only quick-create + D-15 archived-value exception) per 39-UI-SPEC.md §10 — the reusable place-selection control every remaining Phase 39 UI plan (14-19) will wire in — plus its component-showcase demo entry; runtime behavior is UNVERIFIED (see below).**

## Performance

- **Duration:** ~50 min
- **Started:** 2026-08-25 (est.)
- **Completed:** 2026-08-25
- **Tasks:** 3/3 (Task 3 is the checkpoint itself — no code changes)
- **Files modified:** 3 (2 created, 1 modified)

## Accomplishments

- `PlacePicker.svelte` — field (36px, "Выберите место" placeholder, 28px ghost clear button `aria-label="Очистить место"`) + tree-mode panel (opens on focus via `use:portal`/`use:dropdownAnchor`, namespaced `.dropdown--place` class per WR-03, lazy `places_list_children` fetch per node, D-06 any-level selection — click name selects, click chevron expands/collapses without selecting, D-15 archived-value exception with "Архив" badge injected into its parent's child list when the current `value` resolves to an archived place) + search-mode panel (200ms-debounced `places_search`, flat full-path matches capped at 50 rows, matched-substring highlighting via `--tr-accent-text`, D-18 Admin-only "Создать «...» в «...»" row with parent inferred from the last active tree node, Manager empty-state copy per §14.2) + two-stage Escape (§10.3: search→tree, then tree→closed)
- `role="combobox"` + `aria-expanded`/`aria-controls`/`aria-activedescendant` on the field, `role="tree"`/`treeitem` on the panel/rows (composite-widget pattern — rows carry `tabindex="-1"`, real keyboard focus stays on the field, navigation moves `aria-activedescendant`), `aria-live="polite"` region for match-count announcements
- `PlacePickerSection.svelte` — showcase demo with an invented tree ("Здание А / 1-2 этаж / 101, 214, Шкаф-склад (склад), 216 (архив)" + a second root "Территория Северная / КПП-1", fictional data only), three demo blocks (interactive tree/search, pre-selected archived node exercising D-15, disabled/invalid field states), wired through the new injection-prop contract instead of a live API call
- Registered in `ShowcasePage.svelte`'s section list (corrected from the plan's stated file — see key-decisions)

## Task Commits

Each task was committed atomically:

1. **Task 1: PlacePicker.svelte — field + tree-mode panel** (contains the full component, see key-decisions) - `d65a1d3d` (feat)
2. **Task 2: PlacePicker.svelte search mode + D-18 create-row + showcase** (showcase entry only — search mode/D-18 were already complete in Task 1's commit) - `83a83cd2` (test)
3. **Task 3: Checkpoint: PlacePicker** — no code changes; auto-approved by the orchestrator under `workflow.auto_advance` (see "Issues Encountered" / verification note below)

## Files Created/Modified

- `ui/src/lib/components/PlacePicker.svelte` - the control itself (965 lines)
- `ui/src/features/showcase/sections/PlacePickerSection.svelte` - showcase demo (invented tree, fetch-injection wiring)
- `ui/src/features/showcase/ShowcasePage.svelte` - added import + `<PlacePickerSection />` registration

## Decisions Made

See `key-decisions` in frontmatter for full rationale on: (1) Task 1's commit containing the complete tree+search+D-18 component rather than a tree-only slice, because the two modes share one state machine and the two-stage Escape contract spans both; (2) showcase registration landing in `ShowcasePage.svelte` instead of the plan's stated `ComponentShowcasePage.svelte`; (3) the `fetchChildren`/`fetchSearchResults`/`fetchOne`/`createPlace` injection-prop contract — an API-surface decision Plans 15-19 will build on, since every real consumer can omit these props and get the default `apiCall`-backed behavior.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing critical functionality] Root-level tree load-error state**
- **Found during:** Task 1, while implementing `ensureChildrenLoaded(null)`
- **Issue:** The plan's `<action>` text does not mention what the panel should show if the initial `places_list_children(null)` call fails (network/server error) — left unhandled, the panel would silently show "Ничего не найдено" instead of an actionable error, which is misleading and diverges from the project's `14.3` copywriting contract ("Не удалось загрузить места. Проверьте подключение и повторите.").
- **Fix:** Added `rootLoadError` state; `ensureChildrenLoaded` catches the fetch failure, sets `rootChildren = []` + `rootLoadError = true`; the panel renders the exact `§14.3` error copy instead of the generic empty state.
- **Files modified:** `ui/src/lib/components/PlacePicker.svelte`
- **Verification:** `svelte-check` clean; code-reviewed against §14.3's literal copy string.
- **Committed in:** `d65a1d3d` (Task 1 commit)

**2. [Rule 1 - Bug] a11y warnings on tree/search row elements**
- **Found during:** Task 1, running `svelte-check`
- **Issue:** `role="treeitem"` divs with `onclick` but no `tabindex`/keyboard handler triggered `a11y_interactive_supports_focus` and `a11y_click_events_have_key_events` warnings.
- **Fix:** Added `tabindex="-1"` (correct for a composite-widget/aria-activedescendant pattern — rows are not in the tab order, the field is) and an `onkeydown` handler (Enter/Space → same select action as click) as a defensive pairing for the a11y gate.
- **Files modified:** `ui/src/lib/components/PlacePicker.svelte`
- **Verification:** `svelte-check` — 0 warnings on this file after the fix (confirmed via `grep -i PlacePicker` on the full svelte-check output).
- **Committed in:** `d65a1d3d` (Task 1 commit)

---

**Total deviations:** 2 auto-fixed (1× Rule 1, 1× Rule 2). Plus the two structural/file-target deviations and one architecture addition documented in `key-decisions` (not bugs — judgment calls on plan-text ambiguity, tracked separately per this template's convention). No architectural changes requiring Rule 4. Every `must_haves` truth and artifact from the plan frontmatter is satisfied by the code as committed.

## Issues Encountered

**Checkpoint (Task 3) was AUTO-approved by the orchestrator under `workflow.auto_advance`, NOT verified by a human in a running app.** Per the executor's own checkpoint report, only automated/compile-time verification was performed on this plan:
- `pnpm --dir ui exec svelte-check` — 0 errors/warnings on both new files (baseline: 37 pre-existing errors / 50 pre-existing warnings elsewhere in the codebase, confirmed unchanged by this plan)
- `pnpm exec eslint` on all three touched files — clean
- `node scripts/check-tokens.mjs` / `check-focus-outline.mjs` / `check-contrast.mjs` — all PASS
- `pnpm --dir ui build` — succeeds

**None of the above catch Svelte 5 rune runtime errors or WKWebView/WebView2-specific rendering behavior** (established project convention — compile gates ≠ runtime verification). Runtime behavior (focus-opens-panel, keyboard nav, archived-badge rendering, D-18 create flow, LAN-browser parity after `pnpm --dir ui build`) is **UNVERIFIED**. This is not a claim of "tested and working" — it is a claim of "compiles, lints, and builds cleanly." The manual verification steps are recorded as UAT debt in `.planning/phases/39-place-tree/deferred-items.md` (commit `0ced4ea7`, section "Plan 13 — PlacePicker runtime verification NOT performed (auto-approved checkpoint)"), to be executed in one batch at the phase's later checkpoints (39-20 / 39-21) or via `/gsd-verify-work`. See that file for the full numbered verification checklist rather than repeating it here.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

`PlacePicker.svelte` is feature-complete per UI-SPEC §10 and compiles/lints/builds cleanly, but **is unverified at runtime** (see Issues Encountered). Plans 15-19 can wire it into their respective forms (device, cartridge, act, cartridge operations, report filters) using the default `apiCall`-backed fetchers (no props needed beyond `value`/`onChange`/`id`/`disabled`/`invalid`); Plan 14 (Places section tree, `PlaceTree.svelte`) is a separate component that shares visual/keyboard conventions but not code with this one. Before any of those consumer plans are considered done, the deferred runtime-verification checklist in `deferred-items.md` should be run at least once against a real webview — ideally as part of Plan 39-20/39-21's batched UAT pass, per the coordinator's guidance.

---
*Phase: 39-place-tree*
*Completed: 2026-08-25*

## Self-Check: PASSED

All created/modified files confirmed present on disk (`ui/src/lib/components/PlacePicker.svelte`,
`ui/src/features/showcase/sections/PlacePickerSection.svelte`,
`ui/src/features/showcase/ShowcasePage.svelte`, this SUMMARY, `deferred-items.md`). All three
referenced commit hashes (`d65a1d3d`, `83a83cd2`, `0ced4ea7`) confirmed present in `git log`.

---
phase: 39-place-tree
plan: 20
subsystem: ui
tags: [svelte5, runes, places, tree, uat, localstorage, deep-link]

# Dependency graph
requires:
  - phase: 39-place-tree (39-14)
    provides: "PlaceTree/PlaceTreeNode left panel, PlacesPage shell, routing/sidebar, delete-blocked D-14 error surface"
provides:
  - "PlaceContents.svelte — breadcrumbs + type-filtered Tabs + sticky-header content table (D-24 «Только здесь», D-26 short-path column)"
  - "End-to-end «Места» section: tree (left) + content (right) wired into PlacesPage, D-14 «Показать содержимое» navigation without page reload"
  - "PLC-06 fully closed and live-verified by the user across 7 manual UAT rounds"
affects: [40-history, 41-workstations]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Pointer-Events-based drag-and-drop for the place tree (replaces native HTML5 DnD, which is broken in WKWebView) — 6px move threshold to distinguish click from drag, manually-rendered .drag-ghost preview (pointer-events: none so it doesn't break elementFromPoint hit-testing)"
    - "State that must survive a component's own {#key place.id:token} remount is lifted to the parent (PlacesPage) as a controlled prop, not left as local $state — applies to onlyHere and activeTab"
    - "localStorage persistence convention (trackly:places:*, plain getItem/setItem, no dedicated store) reused from $lib/stores/theme.svelte.ts for tree expansion/selection/onlyHere/activeTab"
    - "Cross-section 'focus a specific record' deep link (#/devices|printers|cartridges?id=…) via a shared parseIdFromHash() helper in $lib/utils/hashId.ts"
    - "Read-only view mode added to existing form bodies (DeviceFormBody/CartridgeFormBody) via a single readonly prop that disables every field, rather than forking a separate view-only component"
    - "Programmatic navigation that must survive a component unmount uses svelte-spa-router's push() and awaits it before closing/unmounting the triggering component — not raw window.location.hash assignment"

key-files:
  created:
    - ui/src/features/places/PlaceContents.svelte
    - ui/src/features/places/PlaceEntityViewModal.svelte
    - ui/src/lib/utils/hashId.ts
  modified:
    - ui/src/features/places/PlacesPage.svelte
    - ui/src/features/places/PlaceTree.svelte
    - ui/src/features/places/PlaceTreeNode.svelte
    - ui/src/features/devices/DeviceFormBody.svelte
    - ui/src/features/devices/DeviceList.svelte
    - ui/src/features/devices/DeviceListRow.svelte
    - ui/src/features/devices/DevicesPage.svelte
    - ui/src/features/cartridges/CartridgeFormBody.svelte
    - ui/src/features/cartridges/CartridgeListRow.svelte
    - ui/src/features/cartridges/CartridgesList.svelte
    - ui/src/features/cartridges/CartridgesPage.svelte
    - ui/src/features/printers/PrinterListRow.svelte
    - ui/src/features/printers/PrintersList.svelte
    - ui/src/features/printers/PrintersPage.svelte
    - ui/src/features/reports/ReportFilters.svelte
    - ui/src/lib/components/Badge.svelte
    - .planning/phases/39-place-tree/39-UI-SPEC.md

key-decisions:
  - "PlaceTree gained externalSelect/onShowBlockedContents props so breadcrumb-ancestor clicks and the D-14 same-node reset can drive tree selection from outside the tree component"
  - "PlaceTree.loadTree() re-pushes the freshest PlaceDto for the current selection after every reload, so a rename no longer leaves a stale name in the content header"
  - "Content-row click opens PlaceEntityViewModal (read-only, reusing the existing DeviceFormBody/CartridgeFormBody in a new readonly mode) instead of a blind cross-section navigation that dropped the user into an unfiltered list"
  - "Printers have no dedicated edit modal anywhere in the codebase; places_contents returns a printer row's underlying devices.id, so printer view/edit reuse the device form — mirrors PrinterDetail.svelte's existing «Данные устройства» affordance"
  - "onlyHere and activeTab are owned by PlacesPage (controlled props + localStorage), not local state in PlaceContents, because PlacesPage remounts PlaceContents on every place selection via {#key place.id:token}"
  - "Native HTML5 drag-and-drop was abandoned mid-phase for the place tree after live Tauri UAT showed drop never firing in WKWebView; replaced with a hand-rolled Pointer Events implementation (GAP-2), which then needed its own drag-ghost preview added back (GAP-11) since the browser no longer draws one for free"

requirements-completed: [PLC-06]

# Metrics
duration: "~2h agent execution across the plan's 2 tasks, spanning 7 live user UAT rounds over roughly one day (2026-08-25 14:00 start of Task 1 commit through 2026-08-26 00:15 final UAT closure commit)"
completed: 2026-08-26
---

# Phase 39 Plan 20: Places content screen ("Места") — right panel + end-to-end UAT closure Summary

**`PlaceContents.svelte` (breadcrumbs, type tabs, D-24 «Только здесь» toggle, D-26 short-path table) wired into `PlacesPage`'s detail slot, closing PLC-06 and the whole "Места" section after 7 live UAT rounds (Tauri + LAN browser) that found and fixed 11 defects, most notably that native HTML5 drag-and-drop silently does not work in WKWebView.**

## Performance

- **Tasks:** 2 automated tasks + 1 `checkpoint:human-verify` gate (approved)
- **Files created:** 3 (`PlaceContents.svelte`, `PlaceEntityViewModal.svelte`, `hashId.ts`)
- **Files modified:** ~18 across `places/`, `devices/`, `cartridges/`, `printers/`, `reports/`, plus `39-UI-SPEC.md`
- **UAT rounds:** 7 live passes (Tauri desktop + LAN browser), 11 defects found and closed

## Verification Gates (closing-step re-run, 2026-08-26)

| Gate | Command | Result |
|---|---|---|
| `svelte-check` | `pnpm --dir ui run svelte-check` | **0 errors**, 57 warnings, 281 files — matches Прогон 6 baseline, no regression |
| `cargo test -p trackly-app` (integration, 95 binaries) | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --no-fail-fast -- --skip login_remember_persistent_cookie --test-threads=1` | **534 passed, 0 failed** (run split into two `--test` batches to stay under this environment's background-task lifetime; see note below) |
| `cargo test -p trackly-app --lib` (unit) | same env vars, `--lib` | **210 passed, 0 failed** |
| **`trackly-app` combined total** | | **744 passed, 0 failed** — matches expected count exactly |
| `cargo test -p trackly-infra` | `cargo test -p trackly-infra` | **172 passed, 0 failed** (2 doc-tests ignored, both `#[ignore]`-marked AD/SNMP module examples, pre-existing) — matches expected count exactly |

**Execution note:** the first attempt to run the full `trackly-app` suite as a single foreground
command was silently killed by this environment's background-task lifetime limit partway
through (observed empty output despite ~75/95 binaries having visibly run via `ps`) — the run
was re-executed split into two halves via explicit `--test <binary>` flags (48 + 47 binaries)
plus a separate `--lib` pass for the unit tests, each logging directly to a file rather than
through a buffering `tail`, so partial progress would survive a repeat kill. All three sub-runs
completed cleanly with exit code 0. `export_bindings.rs`'s previously-documented pre-existing
failure (`deferred-items.md`, Plan 12 section — stale `ActItemDto.device_location_id` field)
no longer reproduces; it appears to have been resolved by `39-22`'s vocabulary sweep, which
executed after that note was written.

## Verification note — who verified what

**This is the load-bearing fact for this plan's closure.** The agent's own verification was
compile/lint/build only (`svelte-check`, `eslint`, `pnpm --dir ui build`) — per this project's
established rule, that class of gate cannot see Svelte 5 rune runtime behavior, WKWebView-vs-
browser DnD divergence, or dead-navigation buttons that render correctly but do nothing on
click. All functional/UI verification — breadcrumbs, tabs, the "Только здесь" toggle, the D-14
"Показать содержимое" flow, drag-and-drop, keyboard/ARIA, and the 11 gap fixes — was performed
by the **user**, live, in a running `cargo tauri dev` desktop build **and** a LAN browser tab
(after `pnpm --dir ui build`), across 7 rounds recorded in `39-UAT.md`. The plan's Task 3
checkpoint (`gate="blocking"`) is APPROVED based on that live evidence, not on any agent-side
harness.

## Accomplishments

- `PlaceContents.svelte`: §9.1 breadcrumb header (clickable ancestor segments via repeated
  `places_get` walk up `parent_id`, since `PlaceDto.full_path` is a flat string with no
  per-segment ids), §9.2 `Tabs`+`Checkbox` control row, §9.3 sticky-header `Table` with the
  D-26 short-path "Место" column (hidden when "Только здесь" is on) and both exact §14.2
  empty-state copy pairs.
- Wired into `PlacesPage`'s detail slot, replacing the Plan 14 placeholder; the D-14
  "Показать содержимое" button now selects the failed-delete node, forces `onlyHere = false`,
  and closes the modal — all without a page reload.
- Full "Места" section (routing, sidebar, tree, content, all 4 mutation modals, drag-drop,
  keyboard/ARIA, role gating) verified end-to-end by the user in both Tauri and a LAN browser.
- 11 UAT gaps found and closed across 7 rounds (see below); three of them (GAP-2, GAP-9,
  GAP-11) are explicitly the class of defect the project's "compile gates ≠ runtime
  verification" rule exists to catch — all three compiled and linted perfectly clean while
  being completely non-functional or platform-broken.

## Task Commits

Plan tasks:

1. **Task 1: PlaceContents.svelte — breadcrumbs, tabs, table, only-here toggle** - `e6f389da` (feat)
2. **Task 2: Wire PlaceContents into PlacesPage detail slot + D-14 navigation** - `63d72131` (feat)
3. **Task 3: Checkpoint (blocking, human-verify)** - APPROVED by user after 7 live UAT rounds

UAT gap-closure commits (all landed prior to this closure step, not redone here):

| Commit | Gap | Fix |
|---|---|---|
| `a3be1b89` | — | Прогон 1: recorded 3 initial defects |
| `ee3fa08e` | GAP-1 | "Только здесь" persisted across place selection |
| `e0ad9cc6` | GAP-2 | Tree drag-drop reimplemented on Pointer Events (native HTML5 DnD broken in WKWebView) |
| `e989adac` | GAP-3 | Removed redundant «Включая вложенные места» hint; UI-SPEC §12 updated to match |
| `c06167f8` / `f56b8ff5` | — | Прогон 2 accepted, 5 new gaps recorded |
| `0d9914db` | GAP-4 | Toolbar split out of the table header into two rows |
| `e2bc2d3b` | GAP-5 | Tree expansion/selection/"Только здесь" persisted in localStorage |
| `b747efb0` | GAP-6 | Tree row counter restyled as a `Badge appearance="count"` pill |
| `f6ef52aa` | GAP-7 | "Тип" column hidden on single-type tabs |
| `0014c9a7` | — | Прогон 4 checklist recorded |
| `5366b8d5` / `81f1b13f` / `822cbcfd` | GAP-8 | Read-only view popup (`PlaceEntityViewModal`) on content-row click |
| `10e38377` | GAP-8 | Cross-section `?id=` focus deep link for devices/printers/cartridges |
| `f267377a` | — | Прогон 5 checklist recorded |
| `3622815e` | GAP-9 | Navigate via `svelte-spa-router` `push()`, not raw hash assignment |
| `b9040c19` | GAP-10 | Active content tab persisted |
| `6dd6a808` | — | Прогон 6 checklist recorded |
| `f438386d` | GAP-11 | Drag ghost rendered manually for the pointer-event drag |
| `ae448f26` | — | Прогон 7 result: UAT closed, 11/11 |

## Files Created/Modified

- `ui/src/features/places/PlaceContents.svelte` (423 lines) - breadcrumbs, tabs, table, toggle
- `ui/src/features/places/PlaceEntityViewModal.svelte` (226 lines) - read-only view popup for GAP-8
- `ui/src/lib/utils/hashId.ts` - shared `parseIdFromHash()` for the cross-section deep link
- `ui/src/features/places/PlacesPage.svelte` (287 lines) - detail slot wiring, lifted `onlyHere`/`activeTab`/selection state + localStorage persistence
- `ui/src/features/places/PlaceTree.svelte` (1138 lines) - pointer-event drag-drop (GAP-2), drag ghost (GAP-11), `externalSelect`/`onShowBlockedContents` props, expansion/selection persistence (GAP-5)
- `ui/src/features/places/PlaceTreeNode.svelte` - toolbar split (GAP-4), counter pill (GAP-6)
- `ui/src/features/devices/DeviceFormBody.svelte`, `ui/src/features/cartridges/CartridgeFormBody.svelte` - `readonly` mode for GAP-8's view popup
- `ui/src/features/devices/DevicesPage.svelte`, `ui/src/features/printers/PrintersPage.svelte`, `ui/src/features/cartridges/CartridgesPage.svelte` (+ their List/ListRow siblings) - `?id=` focus/highlight deep-link receivers for GAP-8
- `ui/src/features/reports/ReportFilters.svelte` - removed redundant hint (GAP-3)
- `ui/src/lib/components/Badge.svelte` - reused for the tree counter pill (GAP-6)
- `.planning/phases/39-place-tree/39-UI-SPEC.md` - §12 updated to match the GAP-3 hint removal (UAT-driven spec correction, not a silent contract violation)
- `.planning/phases/39-place-tree/39-UAT.md` - the full 7-round UAT log this summary is built on

## Decisions Made

See `key-decisions` in frontmatter. The single most consequential one: native HTML5
drag-and-drop for the place tree does not work in WKWebView (Tauri's macOS webview) even
though it worked fine from a LAN browser tab — this is exactly the desktop/browser divergence
class the project's dual-delivery model (Tauri + LAN server) is supposed to watch for, and it
was only caught because the user tested both environments live rather than trusting a
Chromium-based synthetic harness or the compile gates. The fix (GAP-2) moved tree drag-drop
onto Pointer Events, which then required a follow-up (GAP-11) to manually render the drag
preview the browser used to draw for free.

## Deviations from Plan

None beyond the UAT-driven gap closures documented above, which were the explicit purpose of
the plan's blocking checkpoint (the checkpoint's `<how-to-verify>` steps 5-9 are exactly what
produced GAP-1 through GAP-11). One documentation deviation: `39-UI-SPEC.md` §12 was edited
alongside GAP-3's code fix (removing a copy contract the user decided was redundant) —
flagged inline in the `e989adac` commit message so the phase verifier does not read it as an
unexplained spec drift.

## Issues Encountered

None beyond what's captured as UAT gaps above.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

PLC-06 and the entire "Места" section (PLC-01 through PLC-06) are now live-verified end-to-end
in both Tauri and LAN-browser environments. Remaining phase-39 work is Plan 21 (delete
`LocationAutocomplete.svelte`, full-repo vocabulary sweep, CI gate, DB-upgrade checkpoint) —
dispatched separately, not part of this closure. Two deferred items from `deferred-items.md`
that touch files outside this plan's scope remain open for Plan 21: the `export_bindings.rs`
stale `ActItemDto` field-name assertion, and stale `location`/`location_id` keys in
`role_endpoint_matrix.rs`'s RBAC-rejection test payloads.

## Self-Check: PASSED

All created/modified files listed above exist on disk; all 17 referenced commit hashes
(plan tasks + UAT gap-closure commits) verified present in git log.

---
*Phase: 39-place-tree*
*Completed: 2026-08-26*

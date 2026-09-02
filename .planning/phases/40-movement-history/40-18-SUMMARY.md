---
phase: 40-movement-history
plan: 18
subsystem: ui
tags: [svelte, reports, movement-history, placepicker, badge]

# Dependency graph
requires:
  - phase: 40-movement-history (plan 12)
    provides: "reports_list_movements Tauri command (Action::ReadPlaces gate), columns_for/column_labels_for('movements') index-aligned column keys/labels, ui/src/bindings.ts entry"
  - phase: 40-movement-history (plan 11)
    provides: "ReportRow.{from_place_path,from_place_path_short,actor_name,reason,entity_type_label,is_deleted} row shape, ReportFilter.{from_place_id,to_place_id}"
provides:
  - "ReportSubNav.svelte's 4th DOMAINS entry 'movements' / 'Перемещения' with single-entry MOVEMENT_REPORTS (D-22)"
  - "ReportFilters.svelte's movements-only branch: two independent PlacePicker instances (Откуда/Куда) fully replacing the single Место/Складское место pair for this domain (D-24)"
  - "ReportTable.svelte's 'from_place_path' column key (clones the place_path/place_path_short + title= convention) and the D-25 «Удалено» Badge next to the «Предмет» cell"
  - "ReportsPage.svelte wiring (deviation — see below): COLUMNS_MAP.movements (D-23), currentCmd/currentColumns/reportTypeKey movements branches, fromPlaceId/toPlaceId prop pass-through"
affects: [40-verify, 40-uat]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Movements domain's single report type reuses report key 'all' (mirrors REQUEST_REPORTS' own 'all' naming per UI-SPEC discretion #2) — this collides with COLUMNS_MAP['all'] (REQUEST_COLUMNS) and currentCmd()'s generic activeReport lookup, so both currentCmd() and currentColumns() branch on activeDomain === 'movements' FIRST, before falling through to the activeReport-keyed lookups every other domain uses"
    - "from_place_path gets its own dedicated formatFromPlaceCell() cell-formatting path in ReportTable.svelte, deliberately NOT reusing formatPlaceCell()'s compositeWith machinery — that machinery is place_path-specific (the requests report's printer-name prefix), and giving from_place_path a parallel-but-separate function keeps the two columns' formatting logic independently readable without a compositeWith branch that would never apply to it"

key-files:
  created: []
  modified:
    - ui/src/features/reports/ReportSubNav.svelte
    - ui/src/features/reports/ReportFilters.svelte
    - ui/src/features/reports/ReportTable.svelte
    - ui/src/features/reports/ReportsPage.svelte

key-decisions:
  - "ReportsPage.svelte required editing despite the plan's files_modified list naming only 3 files — the moment ReportSubNav.svelte's DomainKey union gained 'movements' (mandated by Task 1's own action text), ReportsPage.svelte's own independently-duplicated local DomainKey type stopped being structurally assignable, breaking compilation of an out-of-scope file. Widening it cascaded into ReportFilters.svelte's reportDomain prop type needing the same widening. This is Rule 3 (auto-fix blocking issue: 'broken imports/wrong types blocking task completion'), not an architectural change — the duplication itself is pre-existing project convention (REQUEST_REPORTS is likewise defined twice, once per file), not something introduced by this plan"
  - "COLUMNS_MAP keyed by a domain-specific 'movements' string, not the report's own key 'all' — MOVEMENT_REPORTS' single report type is keyed 'all' (mirroring REQUEST_REPORTS' naming per UI-SPEC discretion #2), which would silently collide with COLUMNS_MAP['all'] (REQUEST_COLUMNS) if reused directly; currentColumns()/currentCmd() branch on activeDomain to resolve the correct one, following the same disambiguation pattern the codebase already uses for cartridge domain's in_use/in_stock (prefixed lookup) vs device domain's same-named keys"
  - "showDeletedBadge() guards on reportType === 'movements' AND col.key === 'device_name' AND row.is_deleted === true (three-way check, not just row.is_deleted) — is_deleted is only ever populated for movements rows (every other report type's ReportRow literal sets it None/undefined), so the reportType check is currently redundant in practice, but keeping it explicit documents the intent and fails safe if a future report type ever also populates is_deleted for an unrelated reason"
  - "Added an explicit .deleted-badge { margin-left: var(--tr-space-2xs) } wrapper instead of relying on Svelte's whitespace-collapsing between the cell's {formatCellDisplay(...)} text expression and the conditional {#if} block for visual spacing — a small follow-up fix after the initial implementation, since relying on incidental template whitespace for a load-bearing visual gap is fragile"

requirements-completed: []  # Bookkeeping constraint: HST-04 closed by the orchestrator at phase end, not by this plan.

# Metrics
duration: ~20min
completed: 2026-09-02
---

# Phase 40 Plan 18: Movements Report Frontend Wiring Summary

**«Перемещения» is now a selectable 4th ReportSubNav domain with two independent subtree-inclusive PlacePicker filters (Откуда/Куда), 7 D-23 columns (Дата/Предмет/Тип/Откуда/Куда/Кем/Причина), and a D-25 «Удалено» badge on soft-deleted rows — all against Plan 40-12's already-gated `reports_list_movements` backend endpoint.**

## Performance

- **Duration:** ~20 min
- **Completed:** 2026-09-02
- **Tasks:** 2/2
- **Files modified:** 4 (0 created)

## Accomplishments

- `ReportSubNav.svelte`: `DomainKey` union + `DOMAINS` array gain `'movements'`/`'Перемещения'` as the 4th top-level domain (D-22 — its own group, not nested under Устройства/Картриджи); `MOVEMENT_REPORTS` single-entry array (`'all'` / `'Все перемещения'` / `reports_list_movements`)
- `ReportFilters.svelte`: a movements-only branch renders two independent `PlacePicker` instances labeled «Откуда»/«Куда», bound to `from_place_id`/`to_place_id` — the existing single «Место»/«Складское место» filter pair is NOT rendered for this domain (D-24, per UI-SPEC's explicit "полностью заменяет их для своего домена")
- `ReportTable.svelte`: new `'from_place_path'` column key cloning the existing `place_path`/`place_path_short` + `title=` full-path-on-hover convention for the genuinely separate "Откуда" field (Plan 40-11's Pitfall 7 — "Куда" reuses `place_path`, "Откуда" is a new field); `<Badge variant="default">Удалено</Badge>` rendered next to the «Предмет» (`device_name`) cell when `row.is_deleted === true` (D-25), clones `PlaceContents.svelte:234`'s `Архив` badge pattern exactly
- `ReportsPage.svelte` (deviation, Rule 3 — see below): `COLUMNS_MAP.movements` (D-23, index-aligned with `columns_for("movements")` in `tauri_cmds/reports.rs`), `currentCmd()`/`currentColumns()`/`reportTypeKey()` movements branches, `fromPlaceId`/`toPlaceId` prop pass-through — without this the 4th domain tab would render but fetch the wrong report (`reports_list_requests_all`, via an activeReport `'all'` key collision) and show the wrong columns (`REQUEST_COLUMNS`)
- Report table cells for Откуда/Куда stay plain text (no accent color, no click-through) per UI-SPEC — unlike the timeline row's own D-19 links, matching every other report's place-path cell convention

## Task Commits

Each task was committed atomically:

1. **Task 1: ReportSubNav 4th domain** - `4b9132c0` (feat)
2. **Task 2: ReportFilters two PlacePickers (D-24) + ReportTable columns/badge (D-23/D-25)** - `1741ed40` (feat) → `c8d755ad` (fix, badge spacing follow-up)

**Plan metadata:** (this commit) `docs: complete plan`

## Files Created/Modified

- `ui/src/features/reports/ReportSubNav.svelte` - `DomainKey` + `DOMAINS` + `MOVEMENT_REPORTS`, `activeReports` derived branch
- `ui/src/features/reports/ReportFilters.svelte` - `reportDomain` type widened to include `'movements'`, `ReportFilter.{from_place_id,to_place_id}`, `Props.{fromPlaceId,toPlaceId}`, movements-only dual-`PlacePicker` markup branch replacing the single Место/Складское место pair
- `ui/src/features/reports/ReportTable.svelte` - `ReportRow` gains the 6 movements-only fields, `formatFromPlaceCell()`, `formatCellTitle`/`formatCellDisplay` `'from_place_path'` arm, `showDeletedBadge()`, `Badge` import + markup, `.deleted-badge` style
- `ui/src/features/reports/ReportsPage.svelte` - `DomainKey` widened, `MOVEMENT_REPORTS`, `ReportFilter`/`ReportRow` interface fields, `COLUMNS_MAP.movements`, `currentCmd`/`currentColumns`/`reportTypeKey` movements branches, `fromPlaceId`/`toPlaceId` props on `<ReportFilters>`

## Decisions Made

- See `key-decisions` in frontmatter — all four are documented there with full rationale (ReportsPage.svelte's necessity, the `'movements'` vs `'all'` key collision resolution, the three-way `showDeletedBadge()` guard, and the explicit badge margin).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `ReportsPage.svelte` required editing beyond the plan's declared 3-file scope**
- **Found during:** Task 1, immediately after adding `'movements'` to `ReportSubNav.svelte`'s `DomainKey` union
- **Issue:** `ReportsPage.svelte` maintains its own independently-duplicated local `DomainKey` type (pre-existing architecture: `REQUEST_REPORTS`/`DEVICE_REPORTS`/etc. are likewise duplicated per-file, not imported). The moment `ReportSubNav.svelte`'s `DomainKey` widened, `ReportsPage.svelte`'s `onDomainChange={(d) => { activeDomain = d; ... }}` callback stopped compiling — `svelte-check` failed with `Type 'DomainKey' is not assignable to type 'DomainKey'. Two different types with this name exist, but they are unrelated.` Widening `ReportsPage.svelte`'s own `DomainKey` to match then surfaced a second, identical error at the `<ReportFilters reportDomain={activeDomain} .../>` call site, since `ReportFilters.svelte`'s `Props.reportDomain` type (Task 2's file) was still the narrower 3-value union.
- **Fix:** Widened `ReportsPage.svelte`'s local `DomainKey` (Task 1's commit) and `ReportFilters.svelte`'s `Props.reportDomain` (Task 2's commit) to include `'movements'`; then, since the plan's own success criterion is "movements report is fully usable end-to-end" (not just "tab visible"), completed the rest of `ReportsPage.svelte`'s wiring in the same pass: `MOVEMENT_REPORTS` array, `COLUMNS_MAP.movements` (D-23 columns, index-aligned with the backend's `columns_for("movements")`), `currentCmd()`/`currentColumns()`/`reportTypeKey()` branches (resolving the `'all'` key collision with `REQUEST_REPORTS`/`REQUEST_COLUMNS` by branching on `activeDomain` first), and `fromPlaceId`/`toPlaceId` prop pass-through from `filter.from_place_id`/`filter.to_place_id`.
- **Files modified:** `ui/src/features/reports/ReportsPage.svelte`
- **Verification:** `pnpm --dir ui svelte-check` (0 errors, 60 pre-existing unrelated warnings), `pnpm --dir ui lint` (all gates pass, including `check-placepath-parity`/`check-place-path-short` which specifically guard against JS-mirror duplication of the path-shortening formula — `from_place_path` cells only read `row.from_place_path_short`, never re-derive it), `pnpm --dir ui build` (succeeds, `ui/dist` rebuilt).
- **Committed in:** `4b9132c0` (Task 1, minimal `DomainKey` widening only) and `1741ed40` (Task 2, the remaining `ReportsPage.svelte` wiring, bundled with `ReportFilters.svelte`/`ReportTable.svelte` since Task 2 is the broader "wire the movements report's frontend" integration task)

---

**Total deviations:** 1 auto-fixed (blocking — cross-file type dependency)
**Impact on plan:** Necessary for the plan's own stated success criteria to hold (a working domain tab that actually fetches the right report and renders the right columns, not just a cosmetically-present tab). No scope creep beyond what Plan 40-18's own `<success_criteria>` already required; `ReportsPage.svelte` is the pre-existing orchestrator component that every other report domain's columns/cmd resolution already lives in — this is filling in the same pattern for the new domain, not introducing a new one.

## Issues Encountered

None beyond the deviation documented above.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- The movements report is fully wired end-to-end on the frontend: domain tab, dual place filters (D-24), correct D-23 columns, D-25 soft-deleted badge, CSV/PDF export (unchanged existing pipeline, already proven at parity by Plan 40-12).
- **UNVERIFIED (pending a real app run, per this plan's `<verification_reality>` note):** `svelte-check`/`lint`/`build` do not catch Svelte 5 rune runtime errors or visually confirm the two-PlacePicker layout, the `title=` hover tooltip on Откуда/Куда cells, or the badge's actual rendered spacing. All of this compiles and lints clean but has NOT been visually confirmed in the running Tauri app or a LAN browser.
- **Known cosmetic gap (not fixed, out of this plan's file scope):** `reports_get_report_counts` (backend, `report_service.rs::get_report_counts`) has no `"movements"` branch — it falls into the existing `else => Vec::new()` arm, so the movements tab's badge count will show `0` via `ReportSubNav`'s `statusCounts[key] ?? 0` fallback rather than the real row count, even once rows have loaded. This is a pre-existing generic per-domain-tab-counts mechanism (Plan G2-5b) that was never extended to the new domain by Plan 40-11/40-12's backend work, and touching `report_service.rs` is outside this plan's declared `files_modified`. Purely cosmetic (the badge shows 0 instead of N; the table itself renders the correct rows) — flagging for `/gsd-validate-phase 40` or a future gap-closure round, not fixing here.
- No blockers for `/gsd-validate-phase 40` or phase-level UAT.

---
*Phase: 40-movement-history*
*Completed: 2026-09-02*

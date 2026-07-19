---
phase: 25-dropdown
plan: 07
subsystem: ui
tags: [svelte5, design-system, dropdown, combobox, drill-in, portal, acts, migration]

# Dependency graph
requires:
  - phase: 25-dropdown
    plan: 03
    provides: "Dropdown.svelte feature-complete (both variants, both list modes, full D-12 keyboard/ARIA contract)"
provides:
  - "ActFormItemsTable.svelte's per-row device picker migrated onto Dropdown — closes CMP-07's riskiest pilot (portal-inside-modal, D-05, SC #5)"
affects: []

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Full-file rewrite instead of literal Task1(wire)/Task2(cleanup) commit split when old handlers become dead-and-unreachable the instant the markup they're wired to is replaced — same noUnusedLocals compile constraint Plan 25-02's summary documented; combined into one commit here with the deviation noted"
    - "Row-indexed $state maps (Record<number, T> per picker row) collapse away once the per-row picker is replaced by one Dropdown instance per {#each} iteration — only the two maps Dropdown itself doesn't own (suggestionsByRow, loadingByRow, the actual fetched data + in-flight flag) survive the migration"

key-files:
  modified:
    - ui/src/features/acts/ActFormItemsTable.svelte
    - ui/src/lib/components/Dropdown.svelte

key-decisions:
  - "Combined Task 1 (wire Dropdown) and Task 2 (remove superseded state/handlers/CSS) into a single commit — the plan's literal split would leave Task 1 non-compiling under the project's strict noUnusedLocals gate the moment the old <input>+portal markup those handlers were wired to (onfocus/onkeydown) is removed, the exact class of issue Plan 25-02's summary already documented and worked around with temporary void markers. A full-file rewrite made that workaround unnecessary and clearer than reproducing it here."
  - "Added getMemberSub (device.state) for instance member rows even though the plan's Task 1 action only names getMemberName/getMemberMeta for members — the original member-instance row (ActFormItemsTable.svelte pre-migration, .opt-state) always showed device state alongside SN/inv; dropping it silently would have regressed real information available to the user mid-pick. Wired via Dropdown's existing getMemberSub prop, zero new surface."
  - "Grouped-mode SN/inv sub text (getGroupSub/getMemberMeta) is now plain, not per-substring monospace like the old .tr-mono spans — this matches UI-SPEC's own Dropdown contract literally ('sub 12px ... у плоского варианта — моно'), i.e. mono styling is documented as a flat-variant-only rule, not something grouped mode ever needed. No regression, just a plain-string-callback constraint of the generic primitive's API."
  - "Kept the row's standalone .loading-row + Spinner indicator (absolute-positioned, top-right of the field) as a sibling of <Dropdown>, per Task 2's explicit instruction not to touch it — Dropdown's own panel-internal 'Загрузка…' row (D-13) is a second, complementary loading affordance, not a replacement for this one."
  - "[Deviation, Rule 1] Fixed Dropdown.svelte itself (outside this plan's declared files_modified): mouse-click and Enter-key picks never closed the panel (open stayed true) — only Tab/Escape/click-outside did, in both Plans 25-02 and 25-03. This pilot is Dropdown's first real consumer wiring onPickGroup/onPickMember, so it's the first place the gap became user-visible. Fixed narrowly (handleOptionClick's direct-pick branch + new handleMemberClick for member rows), drill-in navigation untouched."

requirements-completed: [CMP-07]

# Metrics
duration: ~30min
completed: 2026-07-19
---

# Phase 25 Plan 07: ActFormItemsTable device-picker migration to Dropdown Summary

**Migrated the Act form's per-row device picker — the riskiest portal-inside-modal location in the app — off ~500 lines of hand-rolled input/portal/drill-in/keyboard code onto the shared `Dropdown` primitive, with all business logic (search, drill-in, DEF-2A exclusion, pick-time quantity/label assignment) unchanged; also fixed a latent close-on-pick gap in `Dropdown.svelte` itself, first exposed by this pilot.**

## Performance

- **Duration:** ~30 min
- **Completed:** 2026-07-19
- **Tasks:** 2 completed (combined into 1 commit — see Deviations)
- **Files modified:** 2 (1 in plan scope, 1 cross-file bug fix)

## Accomplishments

- `ActFormItemsTable.svelte`'s non-readonly device-picker cell now renders `<Dropdown variant="combobox">` instead of a raw `<input>` + `use:portal`/`use:dropdownAnchor` `<ul>` — the readonly edit-mode branch (retained positions) is untouched.
- Business logic stayed in the file, reused unchanged and wired via callback props: `fetchGroups` → `onSearch`, `isExpandable` → `isGroupExpandable`, a new `expandGroup`/`partitionMembers` pair (extracted from the old row-indexed `drillInto`/`memberRows`) → `onExpandGroup`, `pickGroup`/`pickDevice` (via a new `pickMember` dispatcher) → `onPickGroup`/`onPickMember`.
- DEF-2A cross-row exclusion (`getSelectedIds(idx)`) is applied in both `fetchGroups` and `expandGroup`, mirroring the pre-migration `fetchGroups`/`drillInto` filtering exactly.
- ~440 net lines removed: all per-row drill-in/open/keyboard `$state` maps (`openByRow`, `viewModeByRow`, `drillGroupByRow`, `membersByRow`, `showBackByRow`, `activeIndexByRow`, `rowInputEls`, `rowDropdownEls`), handlers (`handleFocus`, `handleRowKeydown`, `backToGroups`, `handleGroupClick`, `handleClickOutside` + its `$effect`, `drillInto`, `memberRows`, `visibleGroups`), and the `:global(.dropdown--items ...)` CSS block + `.device-input` rule — now owned internally by `Dropdown`.
- AUTO-02 (focus opens panel) and AUTO-05 (single-group auto-flatten) now come from `Dropdown` itself, not re-implemented locally — verified via `pnpm --dir ui build` + gates (no frontend test harness exists in this project; live browser UAT is the remaining verification step per the plan's `<verification>` block).
- WR-02 (Enter never bubbles to the host `<form>`'s submit) is preserved — it lives entirely inside `Dropdown.svelte`'s `handleKeydown`, untouched by this plan.
- Found and fixed a latent `Dropdown.svelte` gap (Plans 25-02/25-03): picking via mouse click or Enter never closed the panel. Fixed narrowly in `handleOptionClick`'s direct-pick branch and a new `handleMemberClick` helper — see Deviations.

## Task Commits

Task 1 (wire Dropdown) and Task 2 (remove superseded state/handlers/CSS) were combined into a single commit — see Deviations for why.

1. **Task 1+2: Migrate ActFormItemsTable device picker onto Dropdown** - `78d6fa2` (feat)
2. **Dropdown.svelte close-on-pick fix** - `9aaedb4` (fix)

**Plan metadata:** committed as part of this summary commit

## Files Created/Modified

- `ui/src/features/acts/ActFormItemsTable.svelte` (555 → 116 net lines removed) — device-picker cell now consumes `Dropdown`; `pickGroup`/`pickDevice`/`isExpandable`/`getSelectedIds` unchanged; `fetchGroups` simplified to fetch+DEF-2A-filter only (Dropdown owns when to call it); new `expandGroup`/`partitionMembers`/`pickMember`/`groupSub`/`memberName`/`memberMeta`/`memberSub`/`joinSnInv` helpers replace the old row-indexed `drillInto`/`memberRows`; `handleQueryInput` simplified to just syncing `row.query` (drill-in reset + debounce now Dropdown's job).
- `ui/src/lib/components/Dropdown.svelte` — `handleOptionClick`'s direct-pick branch now sets `open = false`; new `handleMemberClick(m)` helper does the same for member rows, used by both the member row's `onclick` and member-view `Enter` handling.

## Decisions Made

See `key-decisions` in frontmatter — combined commit rationale, `getMemberSub` addition for instance-row state, plain (non-mono) grouped-mode sub text per UI-SPEC's own flat-only mono rule, kept standalone `.loading-row` indicator, and the Dropdown.svelte close-on-pick fix.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking issue] Task 1/Task 2 split as literally scoped would not compile**
- **Found during:** Planning the edit, before writing any code
- **Issue:** Task 1's action text scopes changes to "only" the device-picker cell's markup, explicitly deferring removal of the old per-row `$state` maps/handlers to Task 2. But those handlers (`handleFocus`, `handleRowKeydown`, `handleClickOutside`, `backToGroups`, `handleGroupClick`, `drillInto`, `memberRows`, `visibleGroups`) are only ever invoked from the old `<input>`'s `onfocus`/`onkeydown` and the old portaled `<ul>`'s click handlers — all removed by Task 1's own markup change. Under the project's strict `noUnusedLocals`/`noUnusedParameters` TS gate, those functions become "declared but never called" the instant Task 1 lands, which Plan 25-02's own summary already hit and worked around with temporary `void x;` markers.
- **Fix:** Implemented both tasks' final state in a single write and a single commit, rather than reproducing the void-marker workaround for an artificial intermediate state that provides no real inspection value (the two tasks touch the same handful of contiguous regions of one file).
- **Files modified:** `ui/src/features/acts/ActFormItemsTable.svelte`
- **Commit:** `78d6fa2`

**2. [Rule 1 - Bug] Dropdown.svelte never closed the panel on a direct mouse-click or Enter pick**
- **Found during:** Task 1, while wiring `onPickGroup`/`onPickMember` and manually tracing through `Dropdown.svelte`'s `handleOptionClick`/member-row `onclick`/`handleKeydown` to confirm parity with the old `pickGroup`/`pickDevice`'s unconditional `openByRow[idx] = false`
- **Issue:** `Dropdown.svelte` (Plans 25-02/25-03) only sets `open = false` on `Tab`, `Escape`, and click-outside — never as a direct consequence of `onPickGroup`/`onPickMember` being invoked via mouse click or `Enter`. Since no consumer had wired those callbacks before this plan, the gap was invisible until now. Left as-is, picking a device via click or Enter would change the field's `value` to the picked label while the panel stayed open underneath it (rendering an incongruous empty/stale list), a real regression against the pre-migration screen's "any pick closes the dropdown" contract.
- **Fix:** `handleOptionClick`'s direct-pick `else` branch now sets `open = false` right after `onPickGroup(g)`. Added `handleMemberClick(m)` (calls `onPickMember(m)` then `open = false`), used by the member row's `onclick` and by member-view `Enter` handling. Drill-in (`drillInto`) and `Tab`'s existing close are untouched.
- **Files modified:** `ui/src/lib/components/Dropdown.svelte` (outside this plan's declared `files_modified: [ui/src/features/acts/ActFormItemsTable.svelte]`, but the bug is in the very component this plan is wiring in, first exposed by this plan's changes — in scope per the deviation rules' bug-fix provision)
- **Commit:** `9aaedb4`

**3. [Rule 2 - Missing functionality] Instance member rows would have silently dropped device state display**
- **Found during:** Task 1, wiring `getMemberName`/`getMemberMeta` per the plan's literal instruction
- **Issue:** The plan's Task 1 action names `getMemberName` (device name) and `getMemberMeta` (SN/inv) for instance member rows but doesn't mention `getMemberSub`. The pre-migration member-instance row (`.opt-state`) always showed the device's `state` alongside SN/inv — omitting it would have silently regressed real information visible to the user while picking (e.g. distinguishing two same-model devices in different condition states within a drilled group).
- **Fix:** Wired `getMemberSub={memberSub}` (returns `device.state ?? '—'` for instance rows, `undefined` for subgroup rows, whose state is already embedded in the subgroup label) using Dropdown's existing, unused-for-this-purpose prop — zero new component surface.
- **Files modified:** `ui/src/features/acts/ActFormItemsTable.svelte`
- **Commit:** `78d6fa2`

---

**Total deviations:** 3 (1 blocking/compile-driven commit-structure change, 1 cross-file bug fix, 1 missing-functionality addition)
**Impact on plan:** No scope creep on business logic or visible UX beyond the close-on-pick fix (which restores pre-migration behavior, it does not add anything new); no architectural change.

## Issues Encountered

None beyond the three deviations above, all caught and resolved before the first commit via manual code tracing plus the plan's own acceptance-criteria greps and the `lint`/`svelte-check`/`check-tokens.mjs`/`build` gates (all pass, see Self-Check).

## User Setup Required

None — no new dependencies, no environment variables, no manual steps.

## Known Stubs

None. The device picker renders real, live data through the same `devices.listGrouped`/`devices.listByIds` API calls as before; no hardcoded/placeholder values were introduced.

## Live Verification Deferred

Per the plan's `<verification>` block, live browser verification (focus-opens-panel, drill-in with `← Назад`, single-group auto-flatten, Enter-doesn't-submit, quantity/serial clamping) requires opening the Act creation modal in a running app — not exercised in this execution session (no frontend test harness exists in this project; automated gates are `lint`/`svelte-check`/`check-tokens.mjs`/`build`, all green). Flagging for the phase's end-of-phase human-verify checkpoint (`human_verify_mode: end-of-phase` per `.planning/config.json`).

## Threat Flags

None. Matches the plan's `<threat_model>`: `grep -c "@html"` returns 0 in both touched files; the search/drill-in data path (`devices.listGrouped`/`listByIds`) is unchanged — only the triggering component changed, not the calls, their arguments, or their authorization context; DEF-2A cross-row exclusion (`getSelectedIds(idx)`) is applied in both `fetchGroups` and the new `expandGroup`, re-verified by this plan's own acceptance-criteria grep; WR-02's Enter-suppression lives untouched inside `Dropdown.svelte`'s `handleKeydown`, which this plan's close-on-pick fix does not touch (only the mouse-click/Enter-in-groups/Enter-in-members *finalize* paths, which already had `preventDefault`/`stopPropagation` from Plan 25-03).

## Next Steps

- CMP-07 is now fully closed: primitive (25-02/25-03), showcase visual-UAT (25-06), and this real-screen pilot (25-07) are all done — Phase 25's two components (CMP-06 Table/TableRow, CMP-07 Dropdown) are both complete.
- Phases 26-28 migrate the remaining 5 tables and 6 selectors onto `Table`/`TableRow`/`Dropdown` per D-08 — the close-on-pick fix in this plan now benefits all of those future consumers too.
- End-of-phase human-verify checkpoint (per `human_verify_mode: end-of-phase`) should include the Act form's device picker live-verification steps listed in this plan's `<verification>` block, since they weren't exercised in this automated execution session.

## Self-Check: PASSED

- FOUND: ui/src/features/acts/ActFormItemsTable.svelte
- FOUND: ui/src/lib/components/Dropdown.svelte
- FOUND: 78d6fa2 (Task 1+2 commit)
- FOUND: 9aaedb4 (Dropdown.svelte fix commit)
- `pnpm --dir ui lint` — 0 errors
- `pnpm --dir ui svelte-check` — 0 errors, 48 pre-existing warnings (none newly introduced by this plan)
- `node ui/scripts/check-tokens.mjs` — PASS, 0 нарушений
- `pnpm --dir ui build` — exit 0

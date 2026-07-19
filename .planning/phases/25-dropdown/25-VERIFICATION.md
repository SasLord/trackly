---
phase: 25-dropdown
verified: 2026-07-19T09:48:10Z
status: gaps_found
score: 4/5 must-haves verified
overrides_applied: 0
gaps:
  - truth: "Dropdown корректно отображает список с группами через drill-in (Roadmap SC #4)"
    status: failed
    reason: >
      Static trace of ui/src/lib/components/Dropdown.svelte confirms two review findings
      (WR-01, WR-02) were left unfixed by the user's explicit "blockers-only" scope decision
      after code review, and both directly undermine "correctly displays a grouped list" under
      normal, easily-reached usage — not just first render. WR-02: `openPanel()` (fired by
      AUTO-02 focus-open, the select-variant trigger click, and ArrowDown-on-closed) resets
      only `activeIndex`; it never resets `viewMode`/`activeGroup`/`members`/`showBack`. The
      only thing that resets those is the `$effect` watching `groups` by reference, which does
      not re-fire merely because the panel reopened — only once a new fetch resolves with a
      different array. Reachable path: user drills into group A (viewMode='members',
      activeGroup=A), closes the panel (Escape when showBack=false, a pick, Tab, or
      click-outside — none of these reset the drill state), then reopens (refocus / click) —
      the panel renders A's stale member list under whatever the current field text is, until
      (if ever) a differently-referenced `groups` array arrives. WR-01: the `Tab` branch in
      groups-view calls `handleOptionClick(groups[activeIndex])` and then unconditionally sets
      `open = false`. For an expandable group this takes the `drillInto` branch, which is async
      — by the time it resolves and sets `viewMode = 'members'`, the panel is already closed,
      so the pick is silently lost AND the component is left primed to show a stale drilled-in
      list on the next open (feeding directly into the WR-02 defect above). Both were confirmed
      independently by the code review (25-REVIEW.md) and re-confirmed by direct reading of the
      current source during this verification — they are not resolved by the two applied
      critical fixes (CR-01 `cdf6e58`, CR-02 `66036c4`), which patch a different pair of defects
      (reopen-after-pick, stale-async-write) and do not touch `openPanel()` or the `Tab` branch.
    artifacts:
      - path: "ui/src/lib/components/Dropdown.svelte"
        issue: "openPanel() (lines 285-290) does not reset viewMode/activeGroup/members/showBack, so reopening the panel after a manual drill-in can render a stale member list; the Tab branch in groups-view (lines 384-389) drills into an expandable group asynchronously but closes the panel synchronously, losing the pick and leaving state primed for the same stale-reopen defect."
    missing:
      - "Reset viewMode='groups' / activeGroup=null / members=[] / showBack=false inside openPanel() (WR-02 fix, per 25-REVIEW.md)."
      - "Guard the Tab branch so an expandable group is not both drilled-into (async) and closed (sync) in the same keystroke — e.g. only commit non-expandable groups on Tab, or await the drill-in before closing (WR-01 fix, per 25-REVIEW.md)."
  - truth: "Plan 25-03 must_have: \"Dropdown adds the full combobox ARIA pattern ... plus member-mode keyboard navigation\" / \"does not regress any of the pre-existing keyboard/ARIA behaviors\""
    status: failed
    reason: >
      Confirmed by reading Dropdown.svelte lines 506-520: the select-variant's in-panel search
      `<input>` has `oninput={handleInput}` but no `onkeydown={handleKeydown}`, and no
      `onmousedown` preventDefault to keep focus on the trigger. Once a select-variant user
      clicks into this search box to type (the documented, intended way to filter in that
      variant per D-03), focus moves into an element with zero keyboard wiring — Escape,
      ArrowUp/Down, Home/End, Enter and Tab all do nothing from that point. This is exactly the
      "select variant with its in-panel search box" scenario Plan 25-06's showcase must_have
      calls out as closing CMP-07 SC #3, so the one variant path this component's own plan
      claims full keyboard coverage for is the one path where the keyboard layer is dead
      (25-REVIEW.md WR-06, re-confirmed in current source, unfixed).
    artifacts:
      - path: "ui/src/lib/components/Dropdown.svelte"
        issue: "In-panel search input (variant=\"select\", lines 509-520) has oninput but no onkeydown={handleKeydown}, so the D-12 keyboard layer does not function once focus lands there."
    missing:
      - "Add onkeydown={handleKeydown} to the select-variant in-panel search input, and give it its own aria-activedescendant/aria-controls since it now owns focus (WR-06 fix, per 25-REVIEW.md)."
---

# Phase 25: Таблицы и Dropdown — Verification Report

**Phase Goal:** Строки таблицы и новый компонент Dropdown/комбобокс отражают дизайн-систему, сохраняя плотный список и групповой UX, на которые опирается приложение.
**Verified:** 2026-07-19T09:48:10Z
**Status:** gaps_found
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (Roadmap Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Строки таблицы визуально различимы в состояниях обычная/наведение/выбрана | ✓ VERIFIED | `TableRow.svelte:77-86` — `.tr-row:hover { background: var(--tr-row-hover) }`, `.tr-row.selected { background: var(--tr-row-selected); border-left: 3px solid var(--tr-accent) }`, base state has neither. Matches `TableRows.dc.html` values verbatim per D-09/D-10/D-11 (25-01 must_have). Consumed by `DeviceListRow.svelte:51`. |
| 2 | Строка-группа сворачивается/разворачивается, показывая счётчик-пилюлю и вложенные устройства при раскрытии | ✓ VERIFIED | `TableRow.svelte:54-69` group mode: chevron `transform: rotate(90deg)` on `.expanded`, `onclick={onToggleGroup}`. `DeviceGroupRow.svelte:85-105` `toggleExpand()` fetches `devices.listByIds(group.ids)`, sets `children`; template (`DeviceGroupRow.svelte:163-164`) shows count via `<Badge variant="accent" appearance="count">{group.count} шт.</Badge>`; nested `DeviceListRow`s render only `{#if expanded}` (`168-185`). Fully wired, not a stub. Pre-existing (not introduced by this phase) `WR-09` retry-on-failure risk noted as anti-pattern, not blocking. |
| 3 | Dropdown корректно отображает плоский список | ✓ VERIFIED | `Dropdown.svelte:578-614` flat-mode option rendering (name, meta, checkmark via `isGroupSelected`) is structurally sound and independent of the drill-in state machine (no `viewMode` involvement in `flat` mode). No production consumer exists yet (only pilot is the grouped Acts picker, D-05) — the only live demonstration is `DropdownSection.svelte` Block 2 (`variant="select" flat={true}`), which has a known, explicitly-deferred showcase-only defect (IN-06: checkmark doesn't track the demo's own `flatValue`) that does not reflect a defect in `Dropdown.svelte` itself. See Human Verification below. |
| 4 | Dropdown корректно отображает список с группами через drill-in (замена панели, «← Назад · {группа}», не заголовки секций) — модель зафиксирована D-01 | ✗ FAILED | See `gaps` frontmatter, gap 1. `drillInto()`/back-header/`showBack` mechanics are correctly implemented for a **single** open→drill→pick cycle (both CR-01/CR-02 criticals from code review are fixed and confirmed present in source: `handleInput` sets `open=true` at line 276, `expandSeq` guards both the AUTO-05 effect and `drillInto` at lines 160-210). But `openPanel()` never resets drill-in state (WR-02) and `Tab` on an expandable group both drills in and closes the panel, losing the pick (WR-01) — both confirmed unfixed in current source, both directly reachable in the one production consumer (`ActFormItemsTable.svelte`'s per-row picker). |
| 5 | Существующее portal/anchor-позиционирование (Фаза 18) продолжает работать без регрессий с новым визуалом | ✓ VERIFIED | `ui/src/lib/utils/portal.ts` and `ui/src/lib/utils/dropdownAnchor.ts` are byte-for-byte unmodified by Phase 25 (`git log` shows last touch at `73af1fe`/Phase 18 and `870d77d`/earlier — no Phase-25 commit touches either file). `Dropdown.svelte:502-503` wires `use:portal` + `use:dropdownAnchor={{ anchorEl, maxHeight }}` identically to the pre-migration `ActFormItemsTable.svelte` usage. Code review's "Verified-Correct Notes" independently confirm no listener/timer leak (`onDestroy` clears the debounce, click-outside effect cleans up, `portal`'s `destroy()` removes the node). Visual geometry re-check (gap/flip at new panel size) still recommended — see Human Verification. |

**Score:** 4/5 truths verified (roadmap Success Criteria). One additional plan-level must_have failure recorded separately (see gap 2, Plan 25-03's keyboard/ARIA completeness claim).

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `ui/src/styles/_tokens.scss` | `--tr-group` in both light/dark blocks | ✓ VERIFIED | Present in both blocks (confirmed by review + `check-tokens.mjs` PASS — 0 violations). |
| `ui/src/lib/components/TableRow.svelte` | Row-state + group-row primitive | ✓ VERIFIED | Real Svelte component, both modes implemented, wired into two production consumers + showcase. |
| `ui/src/lib/components/Table.svelte` | Shell: head/skeleton/empty | ✓ VERIFIED | Consumed by `DeviceList.svelte:74-105` with `columns`, `loading`, `empty`, `emptyTitle/Body`, `head` snippet. |
| `ui/src/lib/components/Dropdown.svelte` | Generic drill-in combobox/select primitive | ⚠️ VERIFIED-WITH-DEFECTS | Exists, substantive, wired into production (`ActFormItemsTable.svelte`) and showcase. Two criticals fixed; WR-01/WR-02/WR-06 unfixed and functionally significant (see gaps). |
| `ui/src/features/devices/DeviceList.svelte` / `DeviceListRow.svelte` / `DeviceGroupRow.svelte` | Table pilot migration | ✓ VERIFIED | All three consume `Table`/`TableRow`; hand-rolled group markup and hand-rolled count-pill removed. |
| `ui/src/features/acts/ActFormItemsTable.svelte` | Dropdown pilot migration | ✓ VERIFIED (wiring) / ✗ defects | `import Dropdown from` present, one instance per row, `variant="combobox"`, callbacks (`onSearch`/`onExpandGroup`/`onPickGroup`/`onPickMember`) all wired to real `devices.listGrouped`/`devices.listByIds` IPC calls — not stubbed. Functional defects are in `Dropdown.svelte` itself (gap 1), not in this file's wiring. |
| `ui/src/features/showcase/sections/TableSection.svelte` | CMP-06 gallery, wired as 6th section | ✓ VERIFIED | Present, imported in `ShowcasePage.svelte`. |
| `ui/src/features/showcase/sections/DropdownSection.svelte` | CMP-07 gallery, wired as 7th section | ⚠️ VERIFIED-WITH-DEFECTS | Present, wired. IN-06 (info-level, explicitly deferred): forces 4 panels open simultaneously via synthetic focus/click on mount, obscuring the gallery; flat-select checkmark demo doesn't track its own `flatValue` state (self-contradictory demo, not a `Dropdown.svelte` bug). |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `TableRow.svelte` | `_tokens.scss` | `var(--tr-group)` etc. | ✓ WIRED | Confirmed present, `check-tokens.mjs` closed-world gate passes. |
| `TableRow.svelte` | `Badge.svelte` | consumer-placed `<Badge appearance="count">` | ✓ WIRED | `DeviceGroupRow.svelte:164`. |
| `Dropdown.svelte` | `portal.ts` | `use:portal` | ✓ WIRED | `Dropdown.svelte:502`, file unmodified since Phase 18. |
| `Dropdown.svelte` | `dropdownAnchor.ts` | `use:dropdownAnchor={{ anchorEl, maxHeight }}` | ✓ WIRED | `Dropdown.svelte:503`, file unmodified since Phase 18. |
| `DeviceList.svelte` | `Table.svelte` | head/children snippet props | ✓ WIRED | Confirmed, real props not stubs. |
| `DeviceListRow.svelte` / `DeviceGroupRow.svelte` | `TableRow.svelte` | row-state / group-mode wrapper | ✓ WIRED | Confirmed. |
| `ActFormItemsTable.svelte` | `Dropdown.svelte` | one instance per row, `variant="combobox"` | ✓ WIRED | `ActFormItemsTable.svelte:409-431`. |
| `ActFormItemsTable.svelte` | `devices` API | `onSearch`/`onExpandGroup` → `devices.listGrouped`/`listByIds` | ✓ WIRED | `fetchGroups`/`expandGroup`, real IPC calls, DEF-2A dedup preserved (`getSelectedIds`). |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|---------------------|--------|
| `Dropdown` (via `ActFormItemsTable`) | `suggestionsByRow[idx]` → `groups` prop | `devices.listGrouped()` real IPC (FTS5 search, `status_id=1`) | Yes | ✓ FLOWING |
| `Dropdown` (via `ActFormItemsTable`) | `members` (drill-in) | `devices.listByIds()` real IPC | Yes | ✓ FLOWING — but see gap 1: the *timing* of when this data is shown (stale on reopen) is broken, not the data source itself. |
| `DeviceGroupRow` children | `children` | `devices.listByIds(group.ids)` real IPC | Yes | ✓ FLOWING |

### Behavioral Spot-Checks

Skipped — no runnable frontend test harness exists (project has neither vitest nor playwright; confirmed in 25-CONTEXT.md "Established Patterns"). All checks in this report are static-code-trace verification, not executed runtime checks. `svelte-check` (0 errors, 48 pre-existing unrelated warnings), `pnpm lint` (clean), and `check-tokens.mjs` (0 violations) were re-run directly during this verification and confirmed green — these gates were already known-green per the phase's own self-checks and do not by themselves constitute evidence for runtime/visual truths (per this verification's brief).

### Probe Execution

No `scripts/*/tests/probe-*.sh` probes exist for this phase or in the repository. SKIPPED (no runnable entry points / no declared probes).

### Requirements Coverage

| Requirement | Source Plan(s) | Description | Status | Evidence |
|-------------|-----------------|--------------|--------|----------|
| CMP-06 | 25-01, 25-04, 25-05 | Строки таблицы (обычная/наведение/выбрана) + строка-группа со свёрткой, счётчиком, вложенными устройствами | ✓ SATISFIED | SC #1 and SC #2 both verified; two production consumers (`DeviceListRow`, `DeviceGroupRow`) migrated and wired; showcase gallery demonstrates all states. |
| CMP-07 | 25-02, 25-03, 25-06, 25-07 | Dropdown / комбобокс — плоский список и список с группами | ✗ BLOCKED | SC #3 (flat) verified; SC #4 (grouped/drill-in) FAILED per gap 1 (WR-01/WR-02, stale-state-on-reopen and Tab-loses-pick) — this is CMP-07's headline grouped-list behavior, in the requirement's own production pilot. Gap 2 (WR-06, select-variant keyboard layer dead) further undermines the plan's own "full keyboard/ARIA layer" claim. |

No orphaned requirements: `WIN-02` intentionally excluded from this phase's scope per D-06 (documented, remains Phase 26, ROADMAP.md/REQUIREMENTS.md unchanged) — not an omission.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `Dropdown.svelte` | 285-290 | `openPanel()` doesn't reset drill-in state | 🛑 Blocker (gap 1) | Stale member list on reopen |
| `Dropdown.svelte` | 384-389 | `Tab` branch drills in async, closes sync | 🛑 Blocker (gap 1) | Pick silently lost |
| `Dropdown.svelte` | 509-520 | In-panel search input has no `onkeydown` | 🛑 Blocker (gap 2) | D-12 keyboard layer dead in select variant |
| `Dropdown.svelte` | 497-576 | `<ul role="listbox">` wraps role-less `<li>` + two chrome `<li>`s (WR-05) | ⚠️ Warning | Invalid `aria-activedescendant` ownership; accessibility nuance, not literally re-verified as blocking a numbered SC — left unfixed by explicit user scope decision |
| `DeviceGroupRow.svelte` | 109-129 | `$effect` retries a failing `devices.listByIds` fetch forever if `onExpandToggle` isn't wired to collapse on error (WR-09) | ⚠️ Warning | Pre-existing behavior, not introduced by this phase; now sits on a shared primitive |
| `ActFormItemsTable.svelte` | 601 | Dead CSS `.hint-warn` (confirmed by `svelte-check`, IN-01) | ℹ️ Info | Cosmetic, left unfixed by explicit user scope decision |
| `showcase/sections/DropdownSection.svelte` | 73-102 | `onMount` force-opens 4 portaled panels simultaneously (IN-06) | ℹ️ Info | Showcase-only, obscures gallery; left unfixed by explicit user scope decision |
| `showcase/sections/DropdownSection.svelte` | 149 | Flat-select checkmark demo doesn't track `flatValue` (IN-06) | ℹ️ Info | Showcase-only self-contradictory demo state |

No `TBD`/`FIXME`/`XXX` debt markers found in any file touched by this phase (grepped all 9 files listed in 25-REVIEW.md's `files_reviewed_list`).

### Human Verification Required

These become relevant once gap 1/gap 2 are closed — listed for completeness, not blocking this report's status (which is already `gaps_found`):

#### 1. Visual state check — TableRow normal/hover/selected + group row

**Test:** Open the design-system Showcase (admin-only route) → Table section; hover each row type; note colors/border against `TableRows.dc.html`.
**Expected:** Colors/borders match the reference pixel-for-pixel (already code-verified; this is a sanity confirmation, not a first-time check).
**Why human:** Visual/color-perception judgment; no frontend test harness exists to assert computed styles.

#### 2. Dropdown grouped drill-in — full session, not just first use

**Test:** In the Acts form (create/edit), open the device picker, drill into a multi-device group, close the panel without picking (Escape or click outside), then reopen it.
**Expected:** Panel should show the current groups list, not the previously drilled-in group's members.
**Why human:** This is the exact defect in gap 1 — confirming it live (before AND after the fix) requires interacting with the running app; static trace already establishes the defect with high confidence but a live repro is the standard closure evidence.

#### 3. Dropdown flat/select variant — showcase visual check

**Test:** Open Showcase → Dropdown section, "Плоский селект" block; click to open, type in the in-panel search box, try arrow keys.
**Expected:** Search filters the list; arrow keys navigate; Escape/click-outside close it.
**Why human:** Confirms gap 2 (WR-06, dead keyboard layer) live, and separately confirms the showcase's forced-multi-open-on-mount doesn't block interaction once the reviewer starts clicking.

### Gaps Summary

Two of the four Dropdown-related plan/roadmap must-haves are compromised by defects the code review found and the user explicitly chose not to fix in this phase ("blockers-only" scope, applied only to the two criticals CR-01/CR-02). This verification independently re-traced both remaining defects (WR-01, WR-02) directly against the current `Dropdown.svelte` source and confirms they are real, not review false-positives: `openPanel()` never clears drill-in state, and `Tab` on an expandable group both fires an async drill-in and synchronously closes the panel. Both are reachable through the phase's own single production consumer (`ActFormItemsTable.svelte`'s per-row device picker in the Acts form) via ordinary interaction (drill in → close without picking → reopen; or Tab on an expandable group), not an edge case requiring contrived input. A third, related defect (WR-06 — the select-variant's in-panel search box has no keyboard handler at all) falsifies Plan 25-03's own must_have claim of "full combobox ARIA pattern... plus member-mode keyboard navigation" for that variant.

Table/TableRow (CMP-06) has no comparable gap: both roadmap success criteria (#1 row states, #2 group row/count-pill/nested devices) are fully wired in the production Devices-list pilot and structurally match the design reference. Portal/anchor positioning (SC #5) is unmodified code, reused verbatim, and independently confirmed leak-free by the prior code review.

Recommended next step: a small closure plan against `Dropdown.svelte` applying the WR-01/WR-02/WR-06 fixes already specified with code diffs in `25-REVIEW.md`, followed by a live re-check of Human Verification items 2 and 3 above.

---

_Verified: 2026-07-19T09:48:10Z_
_Verifier: Claude (gsd-verifier)_

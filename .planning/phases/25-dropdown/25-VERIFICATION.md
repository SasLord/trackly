---
phase: 25-dropdown
verified: 2026-07-19T18:00:00Z
status: passed
score: 5/5 must-haves verified
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 4/5
  gaps_closed:
    - "Dropdown корректно отображает список с группами через drill-in (Roadmap SC #4) — WR-01 (Tab drills-in-async-then-closes-sync) and WR-02 (openPanel doesn't reset drill-in state) both fixed by Plan 25-08 (commits 09c3f8c, 2d48bea)."
    - "Plan 25-03 must_have — full combobox ARIA pattern + member-mode keyboard navigation for BOTH field variants (WR-06, select-variant in-panel search input had no onkeydown) — fixed by Plan 25-08 Task 2 (commit 2d48bea)."
  gaps_remaining: []
  regressions: []
deferred:
  - truth: "openPanel()'s expandSeq++ can permanently discard an in-flight AUTO-05 auto-flatten for a consumer with memoized/static `groups` (round-2 review WR-01)"
    addressed_in: "Not scheduled — explicit user 'blockers-only' scope decision; documented in Dropdown.svelte resetDrillState() docstring (lines 282-297). Sole production consumer (ActFormItemsTable) self-heals because fetchGroups always assigns a fresh groups array."
    evidence: "25-REVIEW.md round-2 WR-01; quick task 260719-ocq-SUMMARY.md key-decisions explicitly defers it."
  - truth: "Round-1 WR-03 (view mode does not reset synchronously mid-keystroke — folded into BL-01 fix, listed separately per UI-SPEC rule), WR-05 (listbox/<li> role nesting), WR-08 (aria-selected reports keyboard position as selection in non-flat mode)"
    addressed_in: "Not scheduled — explicit user 'blockers-only' scope decision, unchanged since round-1 verification."
    evidence: "25-REVIEW.md 'Previously Deferred (round 1, user scope decision)' section."
  - truth: "Round-2 WR-03 (Escape/Tab from the select-variant search input drops focus to <body> instead of returning it to the trigger)"
    addressed_in: "Not scheduled — explicit user 'blockers-only' scope decision for this verification round."
    evidence: "25-REVIEW.md round-2 WR-03; caller-context deferred list for this verification round."
  - truth: "WR-09 (DeviceGroupRow retry-forever on failed expand fetch), IN-01 (dead .hint-warn CSS in ActFormItemsTable.svelte), IN-06 (showcase DropdownSection force-opens 4 panels on mount + flat-checkmark demo doesn't track its own state)"
    addressed_in: "Not scheduled — explicit user 'blockers-only' scope decision, unchanged since round-1 verification."
    evidence: "25-REVIEW.md 'Previously Deferred' section; 25-VERIFICATION.md round-1 Anti-Patterns table."
---

# Phase 25: Таблицы и Dropdown Verification Report

**Phase Goal:** Строки таблицы и новый компонент Dropdown/комбобокс отражают дизайн-систему, сохраняя плотный список и групповой UX, на которые опирается приложение.
**Verified:** 2026-07-19T18:00:00Z
**Status:** passed
**Re-verification:** Yes — round 2, after gap-closure plan 25-08 + quick task 260719-ocq

## Goal Achievement

### Observable Truths (Roadmap Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Строки таблицы визуально различимы в состояниях обычная/наведение/выбрана | ✓ VERIFIED (regression check — unchanged since round 1) | `TableRow.svelte:77-86` unmodified by this round's commits (`git log` for round-2 touches only `Dropdown.svelte`). No regression possible. |
| 2 | Строка-группа сворачивается/разворачивается, показывая счётчик-пилюлю и вложенные устройства при раскрытии | ✓ VERIFIED (regression check — unchanged since round 1) | `DeviceGroupRow.svelte`/`TableRow.svelte` untouched this round. |
| 3 | Dropdown корректно отображает плоский список | ✓ VERIFIED (regression check) | Flat-mode rendering (`Dropdown.svelte:640-680`) is structurally independent of the `viewMode` state machine touched this round; no regression introduced. |
| 4 | Dropdown корректно отображает список с группами через drill-in (замена панели, «← Назад · {группа}», не заголовками секций) — модель зафиксирована D-01 | ✓ VERIFIED — gap closed | Full re-trace of current `ui/src/lib/components/Dropdown.svelte` at HEAD (commits `09c3f8c`, `2d48bea`, `6407133`). **WR-02** (round 1): `openPanel()` now calls `resetDrillState()` (line 337), which resets `viewMode`/`activeGroup`/`members`/`showBack` and bumps `expandSeq` (lines 298-304) — confirmed at source. **BL-01** (round-2 finding: `handleInput()` was the *other* `open = true` site and had NOT been patched): now also calls `resetDrillState()` (line 322), confirmed by `grep -n "resetDrillState()"` showing exactly 2 call sites (`handleInput`, `openPanel`) plus the 1 definition — matches quick task 260719-ocq's stated fix exactly. **WR-01** (round 1, Tab drills-in-async-then-closes-sync): the groups-view `Tab` branch (lines 433-451) no longer calls `handleOptionClick`; `grep -c "handleOptionClick(groups\[activeIndex\])"` returns exactly 1 (the unrelated, unchanged Enter branch at line 431) — confirming the old unguarded Tab call site is gone. Tab now commits directly via `onPickGroup(g)` only when `g` is truthy AND non-expandable, else just closes — verified against the exact guard shape (`g && !(!flat && isGroupExpandable(g))`, line 447). Reproduction path from the original gap (drill in → close without picking → reopen) is now closed by `resetDrillState()` firing on both reopen paths. |
| 5 | Существующее portal/anchor-позиционирование (Фаза 18) продолжает работать без регрессий с новым визуалом | ✓ VERIFIED (regression check) | `portal.ts`/`dropdownAnchor.ts` untouched by round-2 commits; `Dropdown.svelte:564-565` wiring unchanged. |

**Score:** 5/5 truths verified (roadmap Success Criteria).

### Plan-Level Must-Have (Plan 25-03, re-verified)

| Must-have | Status | Evidence |
|---|---|---|
| "Dropdown adds the full combobox ARIA pattern ... plus member-mode keyboard navigation" / "does not regress any of the pre-existing keyboard/ARIA behaviors" — for BOTH field variants including the select-variant's in-panel search input | ✓ VERIFIED — gap closed (**WR-06**) | `Dropdown.svelte:574-583` — the select-variant in-panel search `<input>` now carries `onkeydown={handleKeydown}` (line 582), `aria-activedescendant={activeOptionId()}` (line 579), and `aria-controls={panelId}` (line 580), alongside its pre-existing `oninput={handleInput}` (line 581). `grep -c "onkeydown={handleKeydown}"` returns 3 (combobox input, select-variant trigger button, select-variant search input) — matches the plan's stated acceptance criterion. The keyboard layer (Escape, Arrows, Home/End, Enter, Tab) is shared via `handleKeydown`, so once wired it functions identically to the already-verified combobox/trigger paths. |

### Non-Regression Check (CR-01, CR-02 — round-1 criticals)

| Item | Status | Evidence |
|---|---|---|
| CR-01: `handleInput` sets `open = true` on every keystroke | ✓ INTACT | `Dropdown.svelte:320` — `open = true;` is still the first statement in `handleInput`. |
| CR-02: `expandSeq` generation token guards both `drillInto` and the AUTO-05 effect against stale async writes | ✓ INTACT | `Dropdown.svelte:172, 201` — `if (seq !== expandSeq) return;` guards present and unchanged. `openPanel`/`handleInput`'s new `resetDrillState()` correctly participate in the *same* counter (not a parallel mechanism) — `grep -n "expandSeq"` shows exactly 3 increment sites (`$effect` else-branch line 183, `drillInto` line 199, `resetDrillState` line 299) plus 2 `seq !== expandSeq` guards — no fourth, uncoordinated writer. |

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `ui/src/lib/components/Dropdown.svelte` | Generic drill-in combobox/select primitive, CMP-07 complete | ✓ VERIFIED | Exists, substantive, wired into production (`ActFormItemsTable.svelte:409`) and showcase. All 3 previously-open blocking defects (WR-01, WR-02, WR-06) confirmed fixed at HEAD; the round-2-discovered critical (BL-01) confirmed fixed via `resetDrillState()` shared helper. |
| `ui/src/features/acts/ActFormItemsTable.svelte` | Dropdown pilot migration | ✓ VERIFIED (unchanged this round) | `import Dropdown from` present, wiring unmodified by round-2 commits (only `Dropdown.svelte` was touched). |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `Dropdown.svelte#openPanel` | `Dropdown.svelte#resetDrillState` | direct call | ✓ WIRED | Line 337: `resetDrillState();` inside `openPanel`. |
| `Dropdown.svelte#handleInput` | `Dropdown.svelte#resetDrillState` | direct call | ✓ WIRED | Line 322: `resetDrillState();` inside `handleInput` — the fix that closes BL-01. |
| `Dropdown.svelte` select-variant search `<input>` | `Dropdown.svelte#handleKeydown` | `onkeydown={handleKeydown}` | ✓ WIRED | Line 582, same shared handler already used by the combobox input and select trigger button. |
| `Dropdown.svelte` groups-view `Tab` branch | `Dropdown.svelte#onPickGroup` (prop) | direct call, guarded | ✓ WIRED | Lines 446-450 — `onPickGroup(g)` called synchronously (no `drillInto`) only for non-expandable groups; guard order (`g &&` before `isGroupExpandable(g)`) prevents the `groups[-1]` crash on Tab-with-no-active-option. |

### Behavioral Spot-Checks

Skipped — no runnable frontend test harness exists (no vitest/playwright in this project, confirmed in prior round). Gate results below (svelte-check, lint, build) were re-run live during this verification, not taken from SUMMARY claims.

**Automated gates, re-run live at HEAD (not trusted from SUMMARY.md):**
- `pnpm --dir ui svelte-check` → 0 errors, 48 pre-existing unrelated warnings, 257 files checked. ✓ PASS
- `pnpm --dir ui lint` (eslint + prettier + check-tokens.mjs) → all pass, `[check-tokens] PASS — 0 нарушений`. ✓ PASS
- `pnpm --dir ui build` → succeeds, `ui/dist` rebuilt. ✓ PASS
- `grep -n "TBD|FIXME|XXX"` on `Dropdown.svelte` → 0 matches.

### Probe Execution

No `scripts/*/tests/probe-*.sh` probes exist for this phase or in the repository. SKIPPED (no declared probes, no runnable entry points).

### Requirements Coverage

| Requirement | Source Plan(s) | Description | Status | Evidence |
|-------------|-----------------|--------------|--------|----------|
| CMP-06 | 25-01, 25-04, 25-05 | Строки таблицы + строка-группа | ✓ SATISFIED | Unchanged since round-1 verification (not touched by gap-closure round). |
| CMP-07 | 25-02, 25-03, 25-06, 25-07, 25-08 | Dropdown / комбобокс — плоский список и список с группами | ✓ SATISFIED | SC #3 (flat) and SC #4 (grouped/drill-in) both verified. Both previously-failed must-haves (Roadmap SC #4, Plan 25-03's ARIA/keyboard claim) now hold, independently re-traced against current source, not assumed from SUMMARY.md. |

No orphaned requirements: all `requirements:` frontmatter across the 8 phase plans (25-01 through 25-08) map to CMP-06 or CMP-07, both of which are declared in ROADMAP.md's Phase 25 `Requirements:` line and marked `[x]`/`Complete` in REQUIREMENTS.md. `WIN-02` remains intentionally excluded from this phase (Phase 26), unchanged.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `Dropdown.svelte` | 433-451 (Tab, groups-view) | Enter-with-no-active-option in groups-view does not `preventDefault()`/`stopPropagation()` (round-2 review's own WR-02, distinct from the now-fixed round-1 WR-02) | ⚠️ Warning | Latent: could allow implicit form submission if the "Enter suppressed" invariant is assumed universally; not currently reachable as a live bug because `ActFormBody.svelte`'s `onsubmit` unconditionally `preventDefault()`s. Does not fall inside this round's must-have scope (Plan 25-03's regression-floor list specifies "Enter selects in groups-mode", not "Enter suppressed with no selection"); not blocking. |
| `Dropdown.svelte` | 581 | select-variant search input's `oninput={handleInput}` still fires `onQueryInput?.(query)` (round-2 review WR-04) | ⚠️ Warning | A select-variant consumer that wires `onQueryInput` (documented as the combobox controlled-value sync hook) would have its displayed value clobbered by search-box keystrokes. No current consumer supplies `onQueryInput` on a `variant="select"` instance, so latent not live. Not blocking. |
| `Dropdown.svelte` | 213-219 | `backToGroups()` restores `returnIndex` into `activeIndex` without re-validating bounds against a possibly-shrunk `groups` array (round-2 review WR-07) | ⚠️ Warning | Visual/AT inconsistency risk (highlighted row could mismatch announced active descendant), not a crash — `activeOptionId()` and `Enter`'s handler both bounds-check downstream. Not blocking. |
| `Dropdown.svelte` | 497-576 area (search `<li>`, drill header `<li>` inside `<ul role="listbox">`) | `<ul role="listbox">` still wraps role-less `<li>` chrome elements alongside `role="option"` rows (round-1 WR-05, re-confirmed present) | ⚠️ Warning | Accessibility nuance; explicitly deferred by user "blockers-only" scope decision, unchanged since round 1. |
| `Dropdown.svelte` | Escape/Tab branches on the select-variant search input | Closing the panel from the search input does not return focus to the trigger (round-2 WR-03) | ⚠️ Warning | Explicitly named as deferred in this verification round's caller context. |
| `DeviceGroupRow.svelte` | 109-129 | `$effect` retries a failing fetch forever (WR-09) | ⚠️ Warning | Pre-existing, deferred, unchanged since round 1. |
| `ActFormItemsTable.svelte` | ~601 | Dead CSS `.hint-warn` (IN-01) | ℹ️ Info | Deferred, unchanged since round 1. |
| `showcase/sections/DropdownSection.svelte` | 73-102, 149 | Force-opens 4 panels on mount; flat-checkmark demo self-contradiction (IN-06) | ℹ️ Info | Deferred, unchanged since round 1. |

No `TBD`/`FIXME`/`XXX` debt markers found in `Dropdown.svelte` (grepped directly this round).

### Human Verification Required

None required to reach `passed` status — all must-haves for this phase are now independently confirmed by static trace against the current, live source (not SUMMARY.md claims), and the deferred items above are explicit, user-accepted scope exclusions rather than open questions. The following two items remain useful as an optional live sanity pass but do not block phase closure (no live browser session is available in this verification environment):

#### 1. Dropdown grouped drill-in — full session repro (confirms the fix, does not gate status)

**Test:** In the Acts form (create/edit), open the device picker, drill into a multi-device group, close the panel without picking (Escape/Tab-on-expandable-group/click-outside), then reopen it (refocus or type).
**Expected:** Panel shows the current groups list, never the previously-drilled-in group's stale member list.
**Why human:** Live interaction confirms the static trace; the source-level fix (`resetDrillState()` called from both `open = true` sites) is confirmed unambiguously by code reading, so this is a sanity check, not a gating unknown.

#### 2. Dropdown select-variant — keyboard parity in the search box

**Test:** Showcase → Dropdown section, "Плоский селект"/select-variant block; click to open, click into the in-panel search box, try Escape/ArrowUp/ArrowDown/Home/End/Enter/Tab.
**Expected:** All behave identically to the combobox-variant field.
**Why human:** `onkeydown={handleKeydown}` wiring is confirmed present in source; live confirmation is a sanity pass only.

### Gaps Summary

None. Both must-haves that failed round-1 verification are now closed and independently re-verified against current source:

1. **Roadmap SC #4 (grouped drill-in via WR-01/WR-02):** Plan 25-08 (commits `09c3f8c`, `2d48bea`) fixed `openPanel()`'s missing reset and the Tab-branch async/sync race. A round-2 code review then found the fix was incomplete — `handleInput()`, the *other* code path that reopens the panel, still lacked the reset (BL-01). Quick task 260719-ocq (commits `6407133`, `502f55e`) closed BL-01 by extracting a shared `resetDrillState()` helper called from both `openPanel()` and `handleInput()`. Verified directly in source: exactly 2 call sites plus 1 definition, both `open = true` sites covered, `expandSeq` counter correctly shared (not duplicated).

2. **Plan 25-03's full combobox ARIA/keyboard claim (WR-06):** Plan 25-08 Task 2 wired `onkeydown={handleKeydown}` + `aria-activedescendant` + `aria-controls` onto the select-variant's in-panel search input. Verified directly in source.

A handful of round-2-review warnings remain unaddressed (Enter-with-no-selection not suppressed in groups-view, `onQueryInput` firing from the select-variant search box, `backToGroups` not re-validating `returnIndex`, focus not returned to the trigger on Escape/Tab-close from the search input, listbox/`<li>` role nesting) — none of these fall inside the specific must-haves this phase's roadmap Success Criteria or PLAN frontmatter commit to, all are Warning-level (not Critical) in the round-2 code review, and the user's "blockers-only" scope decision for this phase already established the pattern of deferring non-critical ARIA refinements. Recorded under `deferred` in this report's frontmatter for traceability, not as blocking gaps.

---

_Verified: 2026-07-19T18:00:00Z_
_Verifier: Claude (gsd-verifier)_
</content>

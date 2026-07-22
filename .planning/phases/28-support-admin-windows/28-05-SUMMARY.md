---
phase: 28-support-admin-windows
plan: 05
subsystem: ui
tags: [svelte, tabs, input, settings, design-system]

# Dependency graph
requires:
  - phase: 24-base-components
    provides: "Tabs.svelte (variant underline/segmented, count-slot), Input.svelte (type text/number/search)"
  - phase: 26-tables-with-layout
    provides: "PageHeader.svelte primitive (title + optional actions snippet)"
provides:
  - "SettingsSubNav.svelte on Tabs primitive (variant=underline, no count), D-06 closed for Settings"
  - "SettingsPage.svelte (route file, NOT a re-export) header on PageHeader primitive"
  - "ThresholdSettings.svelte on Input primitive with onfocusout-wrapper preserving save-on-blur"
  - "StorageSettings.svelte audited — already fully on Button+Modal, zero residual raw fields"
affects: [28-support-admin-windows, settings]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "focusout (bubbles) used to wrap primitives lacking an onblur prop, instead of onblur (does not bubble) — required whenever a primitive's contained <input> needs 'save on blur' behavior surfaced to a wrapper"
    - "Fixed-width wrapper div around Input primitive (Input.svelte's root is width:100%) to replicate a narrow numeric-field layout without extending Input's Props"

key-files:
  created: []
  modified:
    - ui/src/features/settings/SettingsSubNav.svelte
    - ui/src/pages/SettingsPage.svelte
    - ui/src/features/settings/ThresholdSettings.svelte

key-decisions:
  - "StorageSettings.svelte required zero code changes — already fully on Button/Modal with no raw <input>/<select>; audit confirmed, no commit needed for Task 3"
  - "ThresholdSettings: chose variant (a) from the plan (Input + onfocusout wrapper) over the raw-input fallback — focusout bubbles by DOM spec (unlike blur), so the wrapper approach is standards-correct and needs no further fallback"

# Metrics
duration: 6min
completed: 2026-07-22
---

# Phase 28 Plan 05: Settings Shell (Tabs/PageHeader) + Threshold/Storage Panels Summary

**SettingsSubNav moved to the shared Tabs primitive (D-06) and SettingsPage's real window file moved to PageHeader; ThresholdSettings re-tokenized onto Input with a focusout-wrapper preserving save-on-blur; StorageSettings audited and confirmed already fully on Button/Modal — closes D-06 for Settings and D-04 for its two simplest panels (WIN-08).**

## Performance

- **Duration:** 6 min
- **Tasks:** 3 (2 with code changes, 1 audit-only)
- **Files modified:** 3

## Accomplishments

- `SettingsSubNav.svelte`: bespoke `<button class="tab">` tab-bar (7 sections) replaced with `<Tabs variant="underline" tabs={SECTIONS.map(...)} active={activeSection} ariaLabel="Раздел настроек" onchange={onSectionChange} />` — no `count` (sections have no counters). Bespoke `.settings-sub-nav`/`.tab` CSS removed entirely; `Tabs.svelte` supplies all layout/hover/active/focus-ring styling. `activeSection`/`onSectionChange` props-interface unchanged.
- `ui/src/pages/SettingsPage.svelte` (the real window file — lives directly in `pages/`, NOT a thin re-export like `RequestsPage`/`ReportsPage`/`UsersPage`): bespoke `<header class="page-header"><h1 class="page-title">Настройки</h1></header>` replaced with `<PageHeader title="Настройки" />` (no `actions` snippet — Settings has no header-level buttons). Scoped `.page-header`/`.page-title` CSS removed. All 7-section `{#if activeSection === ...}` switching logic untouched.
- `ThresholdSettings.svelte`: raw `<input id="threshold-input" class="form-input" type="number" min="1" max="999" bind:value={threshold} onblur={saveThreshold}>` replaced with `<Input type="number" id="threshold-input" value={String(threshold)} oninput={(v) => (threshold = Number(v) || 0)} />`. Because `Input.svelte` has no `onblur` prop and does not forward arbitrary DOM events to its inner `<input>`, the plan's variant (a) was applied: the `Input` is wrapped in `<div class="input-group" onfocusout={saveThreshold}>`. This works because `focusout` bubbles through the DOM (unlike `blur`, which does not) — the inner `<input>`'s native blur fires a `focusout` event that propagates up to the wrapper `div`'s listener. A fixed-width `.threshold-input-wrap { width: 80px }` div constrains `Input`'s `width: 100%` root to the original narrow field size. Suffix "штук" and helper text "Значение сохраняется автоматически при потере фокуса." kept verbatim (2/2 grep match confirmed). `saveThreshold`/`onMount` logic unchanged.
- `StorageSettings.svelte`: audited per plan — already fully on `Button`+`Modal` (imports confirmed present), zero residual raw `<input>`/`<select>` found via grep. `<code class="db-path-code">` (bare `font-family: monospace`) intentionally left untouched — UI-SPEC §9.3 only mandates `--tr-text-mono` for `.folder-code` (BackupSettings), not `.db-path-code`, and this migration was out of the plan's explicit scope. Zero code changes; no commit for this task.

## Task Commits

Each task with code changes was committed atomically:

1. **Task 1: SettingsSubNav → Tabs (D-06) + SettingsPage → PageHeader** - `044bff1` (feat)
2. **Task 2: ThresholdSettings re-tokenization (D-04)** - `4d21a28` (feat)
3. **Task 3: StorageSettings audit (D-04)** - no commit (zero-diff audit, confirmed via grep — see Accomplishments)

**Plan metadata:** (this commit, docs: complete plan)

## Files Created/Modified
- `ui/src/features/settings/SettingsSubNav.svelte` - tab-bar on Tabs primitive, bespoke CSS removed
- `ui/src/pages/SettingsPage.svelte` - header on PageHeader primitive, bespoke header CSS removed
- `ui/src/features/settings/ThresholdSettings.svelte` - number field on Input primitive with focusout-wrapper

## Decisions Made
- **ThresholdSettings save-on-blur:** variant (a) (Input + `onfocusout` wrapper) chosen over the raw-input fallback offered in the plan. `focusout` bubbling is standard, well-established DOM behavior (unlike `blur`, which does not bubble) — no further fallback investigation was needed; the plan's own reasoning for preferring variant (a) held up.
- **StorageSettings scope discipline:** `.db-path-code`'s bare `font-family: monospace` was left as-is rather than "fixing" it to `--tr-text-mono`, per the plan's explicit instruction that this token is only mandated for `.folder-code` (BackupSettings) and is out of this plan's scope — avoided inventing an unplanned change.

## Deviations from Plan

None - plan executed exactly as written, including its own recommended resolution path for the ThresholdSettings blur/focusout gray zone (variant a).

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

D-06 for Settings (WIN-08) is closed: `SettingsSubNav` is on the shared `Tabs` primitive with no bespoke tab-bar markup, and `SettingsPage` (the real window file) is on `PageHeader`. D-04 is closed for the two simplest Settings panels: `ThresholdSettings` (Input + focusout-wrapper) and `StorageSettings` (confirmed already on Button/Modal). Automated verification (`check-tokens.mjs`, `svelte-check`) passed with 0 errors on both code-changing tasks. The plan's `<human-check>` visual verification (both themes; all 7 sections switch; toast "Порог обновлён" on blur; DB-move confirm dialog) is deferred to end-of-phase UAT per `human_verify_mode: "end-of-phase"` — consistent with how plans 28-03/28-04 handled their own human-check steps.

**WIN-08 is NOT yet marked complete in REQUIREMENTS.md** — this plan closed the Settings window shell (sub-nav + page header) and 2 of 8 panels (`ThresholdSettings`, `StorageSettings`). The remaining 5 panels (`NetworkSettings`, `OrgSettings`, `BackupSettings`, `ActiveDirectorySettings`, `TemplateEditor`) still carry raw `<input>`/`<select>`/checkbox markup per D-04 and are covered by later plans in this phase before WIN-08 can close (mirrors how plan 28-02 left `requirements-completed` empty pending 28-04's full WIN-07 closure).

---
*Phase: 28-support-admin-windows*
*Completed: 2026-07-22*

## Self-Check: PASSED

- FOUND: ui/src/features/settings/SettingsSubNav.svelte
- FOUND: ui/src/pages/SettingsPage.svelte
- FOUND: ui/src/features/settings/ThresholdSettings.svelte
- FOUND: .planning/phases/28-support-admin-windows/28-05-SUMMARY.md
- FOUND: 044bff1 (Task 1 commit)
- FOUND: 4d21a28 (Task 2 commit)

---
phase: 07-reports-dashboard-settings
plan: 11
subsystem: frontend/settings
tags: [gap-closure, ux, settings, sub-nav, svelte5]
dependency_graph:
  requires: []
  provides: [settings-sub-nav, settings-section-gap]
  affects: [ui/src/pages/SettingsPage.svelte, ui/src/features/settings/SettingsSubNav.svelte]
tech_stack:
  added: []
  patterns: [switch-bar sub-nav, conditional section rendering, Svelte 5 $state]
key_files:
  created:
    - ui/src/features/settings/SettingsSubNav.svelte
  modified:
    - ui/src/pages/SettingsPage.svelte
decisions:
  - "SettingsSubNav uses div[role=tablist] (not <nav role=tablist>) to avoid a11y non-interactive-to-interactive-role warning"
  - "Conditional rendering (show/hide) chosen over scroll-into-view for simplicity and consistency with other switch-bar patterns"
  - "gap: var(--space-lg) + display:flex flex-direction:column on .settings-content handles both GAP-S1 and sub-nav spacing in one rule"
metrics:
  duration: "8 min"
  completed: "2026-06-17"
  tasks: 1
  files: 2
requirements_closed: [SET-01, SET-02, SET-03, SET-04, SET-05, SET-07, SET-09]
---

# Phase 07 Plan 11: Settings UX Gap Closure (GAP-S1, GAP-S2) Summary

**One-liner:** SettingsSubNav switch-bar splits Settings into 6 per-subsection views with visible card gap via flex+gap on .settings-content.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Create SettingsSubNav and update SettingsPage layout | 712356d | SettingsSubNav.svelte (created), SettingsPage.svelte (modified) |

## What Was Built

**GAP-S2 — SettingsSubNav switch-bar:**
- New `ui/src/features/settings/SettingsSubNav.svelte` component with 6 tabs: Сеть / Организация / Хранилище / Бэкапы / Порог остатка / Шаблоны
- Tab styling matches `ReportSubNav.svelte` exactly: height 32px, 1px solid border, border-radius var(--radius-sm), accent active state (color-mix 10%), hover: surface-sunken
- ARIA: `role="tablist"` on container `div`, `role="tab"` + `aria-selected` on each button
- Props: `activeSection: string` + `onSectionChange: (_s: string) => void`
- Svelte 5 runes: uses `$props()` for prop binding

**GAP-S1 — Section card vertical gap:**
- `.settings-content` updated: added `display: flex; flex-direction: column; gap: var(--space-lg)`
- This creates consistent vertical spacing between the SettingsSubNav and the active section card

**SettingsPage.svelte updates:**
- Import `SettingsSubNav`
- `let activeSection = $state('network')` for active section tracking
- `<SettingsSubNav {activeSection} onSectionChange={(s) => (activeSection = s)} />` rendered first inside `.settings-content`
- Conditional rendering `{#if activeSection === 'network'}...{:else if ...}` replaces the flat list of all 6 components

## Verification

- `pnpm svelte-check`: 0 errors, 36 warnings (pre-existing in unrelated files)
- `ls ui/src/features/settings/SettingsSubNav.svelte`: file exists
- `grep -c 'SettingsSubNav' ui/src/pages/SettingsPage.svelte`: 2 (import + usage)
- `grep -c 'activeSection' ui/src/pages/SettingsPage.svelte`: 8 (>= 3 required)
- `grep -c 'gap.*space-lg'`: 1 in .settings-content

## Deviations from Plan

**1. [Rule 1 - Bug fix] Replaced `<nav role="tablist">` with `<div role="tablist">`**
- Found during: Task 1 (svelte-check run)
- Issue: `<nav>` is an interactive landmark element; assigning `role="tablist"` triggers svelte a11y warning `a11y_no_noninteractive_element_to_interactive_role`. However `<nav>` is actually an interactive element, not non-interactive — the warning stems from the svelte a11y rule treating `<nav>` as a sectioning landmark. Using `<div role="tablist">` removes the warning while preserving correct ARIA semantics. This matches how `ReportSubNav.svelte` implements its report-nav container (a `<div role="tablist">`).
- Fix: Changed opening/closing tag from `<nav>` to `<div>`
- Files modified: `ui/src/features/settings/SettingsSubNav.svelte`
- Commit: 712356d (included in same task commit)

## Known Stubs

None.

## Threat Flags

None. Changes are purely client-side state and CSS; no new network endpoints or trust boundaries.

## Self-Check: PASSED

- `ui/src/features/settings/SettingsSubNav.svelte` — FOUND
- `ui/src/pages/SettingsPage.svelte` — FOUND (modified)
- Commit `712356d` — FOUND in git log

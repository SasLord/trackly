---
phase: 24-base-components
plan: 06
subsystem: ui
tags: [svelte5, scss, design-tokens, tabs, a11y]

# Dependency graph
requires:
  - phase: 24-base-components
    provides: "Plan 01's --tr-accent-text token, consumed by both Tabs variants' active-tab text color"
provides:
  - "Tabs.svelte — new primitive, variant: 'underline' | 'segmented', tabs: {key,label,count?,disabled?}[], active (bindable), onchange, ariaLabel"
  - "TabsSection.svelte showcase gallery (self-contained, genuinely interactive via $state) ready for Plan 07 to wire into the showcase route"
affects: [24-07]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Literal-role-per-branch via a shared #snippet: when a container's ARIA role must vary by prop (tablist vs group) and the literal attribute string needs to survive in source (not compile away into a dynamic expression), render two branches each with a hardcoded role= string, sharing the repeated inner markup through a Svelte 5 #snippet block instead of duplicating it"

key-files:
  created:
    - ui/src/lib/components/Tabs.svelte
    - ui/src/features/showcase/sections/TabsSection.svelte
  modified: []

key-decisions:
  - "Segmented-variant active-tab shadow uses var(--tr-elev-1) as the tokenized substitute for the reference's untokenized rgba(16,22,34,.12) shadow, per RESEARCH.md's already-resolved Open Question 1 — required to pass check-tokens.mjs Rule 4 (zero raw color-function literals)"
  - "Tabs.svelte's container role is rendered via two literal-string branches (role=\"tablist\" / role=\"group\") sharing one #snippet for the repeated <button> loop, rather than a single dynamic role={ternary} attribute — the ternary form (as literally drafted in the plan's <action>) compiles away into a JS expression with no literal role=\"...\" substring in source, which the plan's own acceptance criteria greps for"

patterns-established:
  - "#snippet-shared-branches is now the reference shape for any future primitive needing a literal (not computed) ARIA attribute that varies by a variant prop"

requirements-completed: [CMP-04]

# Metrics
duration: 5min
completed: 2026-07-18
---

# Phase 24 Plan 06: Tabs Component (Underline + Segmented) + Showcase Section Summary

**Built Tabs.svelte from scratch — the first Tabs primitive in the app — covering both the `underline` switch-bar (count badges, active underline) and `segmented` pill-group (raised active surface) variants from D-05, plus the "Вкладки" showcase section demonstrating both interactively.**

## Performance

- **Duration:** 5 min
- **Started:** 2026-07-18T06:34:00Z
- **Completed:** 2026-07-18T06:39:07Z
- **Tasks:** 2 completed
- **Files modified:** 2

## Accomplishments
- `Tabs.svelte` created: `interface Props { variant?: 'underline' | 'segmented'; tabs: {key,label,count?,disabled?}[]; active: string; onchange?; ariaLabel? }`, `active` destructured `$bindable()`
- Underline variant: switch-bar tab buttons with `.tab-count` badges (default + active-tinted via `var(--tr-accent-soft)`/`var(--tr-accent-text)`), accent underline on active, hover/focus/disabled states, `.12s` background/box-shadow micro-transitions restored per D-09
- Segmented variant: pill-group container (`var(--tr-surface-sunken)` background), active segment raised via `var(--tr-surface)` + `var(--tr-elev-1)` shadow (tokenized substitute for the reference's raw rgba shadow), focus-ring on `:focus-visible`
- Zero raw `rgba(` color-function literals in the file — `check-tokens.mjs` Rule 4 gate passes
- `TabsSection.svelte` created: "Вкладки" heading, two labeled sub-blocks ("Switch-bar (underline)" / "Сегментированный"), each backed by a real local `$state` binding (`underlineActive`, `segmentedActive`) so both demos are genuinely clickable, not static markup — underline demo includes counters (12/4) and one disabled tab ("Архив")

## Task Commits

Each task was committed atomically:

1. **Task 1: Create Tabs.svelte (underline + segmented, D-05)** - `6a7efad` (feat)
2. **Task 2: Create TabsSection.svelte showcase section** - `dac0456` (feat)

**Plan metadata:** committed separately after this summary.

## Files Created/Modified
- `ui/src/lib/components/Tabs.svelte` - New: underline + segmented Tabs primitive, both variants transcribed verbatim from `Tabs.dc.html`'s value tables (with the one documented `--tr-elev-1` shadow substitution)
- `ui/src/features/showcase/sections/TabsSection.svelte` - New: interactive showcase gallery, not yet routed

## Decisions Made
- Restructured the container's ARIA role from the plan's literally-drafted dynamic ternary (`role={variant === 'segmented' ? 'group' : 'tablist'}`) into two branches (`{#if variant === 'segmented'}...{:else}...{/if}`), each with a hardcoded `role="group"` / `role="tablist"` attribute, sharing the repeated `<button>`-loop markup via a Svelte 5 `{#snippet tabButtons()}` block — the dynamic-ternary form compiles to a JS expression with no literal `role="..."` substring surviving in source, which fails the plan's own acceptance-criteria grep (`grep -c 'role="tablist"\|role="group"'` expecting ≥1 each); the snippet-branch form satisfies both runtime correctness and the literal-string check without duplicating the per-tab button markup
- Individual `<button>` `role`/`aria-selected`/`aria-pressed` attributes remain dynamic ternaries (no acceptance criterion required a literal string there, and per-button role legitimately varies per iteration, not just per container)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Container role rendered via literal-string branches instead of a dynamic ternary**
- **Found during:** Task 1 (Create Tabs.svelte)
- **Issue:** The plan's `<action>` specifies `role={variant === 'segmented' ? 'group' : 'tablist'}` on the container `<div>`, but the plan's own acceptance criteria requires `grep -c 'role="tablist"\|role="group"'` to return at least 1 for each literal string. A Svelte dynamic-expression attribute compiles away — no literal `role="tablist"` or `role="group"` substring exists in the component's source, so the drafted implementation would fail its own verification.
- **Fix:** Split the container into an `{#if variant === 'segmented'}...{:else}...{/if}` branch, each with a hardcoded `role="group"` / `role="tablist"` string, sharing the identical `<button>`-loop body through a `{#snippet tabButtons()}` block called via `{@render tabButtons()}` in both branches. Zero duplication of per-tab logic; both literal role strings now present in source.
- **Files modified:** `ui/src/lib/components/Tabs.svelte`
- **Verification:** `grep -c 'role="tablist"\|role="group"' ui/src/lib/components/Tabs.svelte` returns 2; `pnpm --dir ui lint`, `pnpm --dir ui svelte-check`, `node ui/scripts/check-tokens.mjs` all exit 0 with zero errors
- **Committed in:** `6a7efad` (part of Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 bug)
**Impact on plan:** Necessary for the component to pass its own acceptance criteria; functionally and visually identical to the plan's intent (correct ARIA role per variant), only the Svelte templating mechanism changed from a dynamic attribute expression to literal-string branches.

## Issues Encountered
None beyond the deviation above.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- `Tabs.svelte` exposes both `underline` and `segmented` variants matching `Tabs.dc.html`'s values exactly (with the one documented `--tr-elev-1` token substitution for the untokenized shadow)
- `TabsSection.svelte` compiles standalone (`svelte-check`: 0 errors), demonstrates both variants interactively, ready for Plan 07 to import into the showcase page assembly
- Per D-07, the 4 existing hand-rolled tab-bar call-sites (`RequestsSearchAndTabs`, `CartridgesSearchAndTabs`, `SettingsSubNav`, `ActsSearchAndTabs`) are explicitly NOT retrofitted in this plan — `Tabs.svelte` exists only as a new, reusable primitive plus its own demo
- No blockers for Plan 07 (final Wave 2 plan — showcase assembly + manual verification checkpoint)

---
*Phase: 24-base-components*
*Completed: 2026-07-18*

## Self-Check: PASSED

All 3 files verified present on disk (Tabs.svelte, TabsSection.svelte, this SUMMARY); both commits (6a7efad, dac0456) verified in git log.

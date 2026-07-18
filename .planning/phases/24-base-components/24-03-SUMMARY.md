---
phase: 24-base-components
plan: 03
subsystem: ui
tags: [svelte5, scss, design-tokens, forms, a11y]

# Dependency graph
requires:
  - phase: 24-base-components
    provides: "Plan 01's --tr-accent-text token + .theme-switching hook, and Plan 02's showcase-section pattern (static markup under ui/src/features/showcase/sections/)"
provides:
  - "Input/Select/Textarea corrected to ctrlBase() bg/border tokens (--tr-surface/--tr-border-strong) plus the missing danger-ring box-shadow on Select/Textarea's invalid state"
  - "Checkbox.svelte and Radio.svelte — new native-input-backed form primitives (hidden <input type=checkbox|radio> + CSS-sibling-styled visual box), first checkbox/radio components in the app"
  - "FieldsSection.svelte showcase gallery (self-contained, static demo content) ready for Plan 07 to wire into the showcase route"
affects: [24-07]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Hidden-native-input + CSS-sibling-selector styling for custom form controls (Checkbox/Radio): input is visually hidden via position:absolute + opacity:0 (not display:none, to preserve focus/keyboard), decorative box driven purely by :checked/:focus-visible/:disabled sibling selectors — no role= or onkeydown ever added"
    - "$bindable() props destructured with `let` (not `const`) whenever the component itself uses bind: on that prop internally — const destructuring compiles fine for pass-through/controlled props (Input/Select/Textarea's existing pattern) but fails Svelte's constant_binding check the moment the component does its own bind:checked/bind:group"

key-files:
  created:
    - ui/src/lib/components/Checkbox.svelte
    - ui/src/lib/components/Radio.svelte
    - ui/src/features/showcase/sections/FieldsSection.svelte
  modified:
    - ui/src/lib/components/Input.svelte
    - ui/src/lib/components/Select.svelte
    - ui/src/lib/components/Textarea.svelte

key-decisions:
  - "Checkbox/Radio destructure props with `let`, diverging from Input/Select/Textarea's `const` — required because Checkbox/Radio use bind:checked/bind:group on their own native input, which Svelte's compiler rejects on a const-declared bindable prop (constant_binding error); Input/Select/Textarea never bind: to themselves (controlled value + oninput/onchange callback instead), so const works there"
  - "Checkbox/Radio's .invalid state reuses the exact --tr-danger/--tr-danger-ring pair from Input/Select/Textarea (Fields.dc.html shows no explicit error box state for these two) — per PATTERNS.md's explicit guidance, not an ad-hoc choice"

patterns-established:
  - "Hidden-native-input pattern is now the canonical shape for any future custom form control needing native keyboard/a11y semantics without a visible native widget"

requirements-completed: [CMP-02]

# Metrics
duration: 3min
completed: 2026-07-18
---

# Phase 24 Plan 03: Field Primitives (Input/Select/Textarea Fix + Checkbox/Radio + Showcase) Summary

**Fixed Input/Select/Textarea's bg/border token drift to match `ctrlBase()`, built Checkbox and Radio from scratch as native-input-backed primitives (D-04), and created the 5-field-type "Поля ввода" showcase section.**

## Performance

- **Duration:** 3 min
- **Started:** 2026-07-18T06:19:16Z
- **Completed:** 2026-07-18T06:21:39Z
- **Tasks:** 3 completed
- **Files modified:** 6

## Accomplishments
- `Input.svelte`/`Select.svelte`/`Textarea.svelte`'s base rule: `background: var(--tr-bg)` → `var(--tr-surface)`, `border: 1px solid var(--tr-border)` → `var(--tr-border-strong)` — matches `ctrlBase()` exactly
- Added the missing `box-shadow: 0 0 0 3px var(--tr-danger-ring)` to `Select.svelte`/`Textarea.svelte`'s `.invalid` rule (`Input.svelte` already had it)
- `Checkbox.svelte` and `Radio.svelte` created: hidden native `<input type=checkbox|radio>` driving a decorative 18px styled box purely via `:checked`/`:focus-visible`/`:disabled` CSS sibling selectors — zero hand-rolled `role=`/`onkeydown`, real keyboard and screen-reader semantics for free
- `Radio.svelte` exposes `group = $bindable<string | number | null>(null)` forwarded to native `bind:group`, letting Svelte compile the correct name/checked wiring automatically (no custom event-based group-sync)
- `FieldsSection.svelte` created: all 5 field types (Input/Select/Textarea/Checkbox/Radio) each shown in normal/error/disabled static states with self-documenting Russian labels; Radio additionally demos 2 instances sharing one `group` value for mutual exclusivity

## Task Commits

Each task was committed atomically:

1. **Task 1: Fix Input/Select/Textarea bg/border token drift (ctrlBase transcription)** - `e678af8` (fix)
2. **Task 2: Create Checkbox.svelte + Radio.svelte (new, D-04)** - `70158e4` (feat)
3. **Task 3: Create FieldsSection.svelte showcase section** - `59730b0` (feat)

**Plan metadata:** committed separately after this summary.

## Files Created/Modified
- `ui/src/lib/components/Input.svelte` - Base `.input` rule: `--tr-bg`/`--tr-border` → `--tr-surface`/`--tr-border-strong`
- `ui/src/lib/components/Select.svelte` - Base `.select` rule token swap + added missing danger-ring box-shadow to `.invalid`
- `ui/src/lib/components/Textarea.svelte` - Base `.textarea` rule token swap + added missing danger-ring box-shadow to `.invalid`
- `ui/src/lib/components/Checkbox.svelte` - New: hidden native checkbox input + styled `.box` sibling, checkmark pseudo-element, invalid/disabled/focus states
- `ui/src/lib/components/Radio.svelte` - New: hidden native radio input + styled circular `.box` sibling, inner-dot pseudo-element, `bind:group` semantics
- `ui/src/features/showcase/sections/FieldsSection.svelte` - New: static showcase gallery, not yet routed

## Decisions Made
- `let` destructuring (not `const`) for Checkbox/Radio's `$bindable()` props — a blocking compile error (`constant_binding`) surfaced immediately when using `const` with `bind:checked`/`bind:group` on the component's own native input; `let` is required for self-referential `bind:`
- Reused Input/Select/Textarea's exact danger token pair on Checkbox/Radio's `.invalid` state per PATTERNS.md's explicit note that `Fields.dc.html` has no dedicated error-box spec for these two components

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Switched Checkbox/Radio prop destructuring from `const` to `let`**
- **Found during:** Task 2 (Create Checkbox.svelte + Radio.svelte)
- **Issue:** Following the plan's literal instruction to mirror `Input.svelte`'s `const { ... }: Props = $props();` shape produced a Svelte compiler error (`constant_binding`, "Cannot bind to constant") on `bind:checked`/`bind:group`, because `Input.svelte`'s own `const` pattern never binds to itself (it forwards value via a controlled `oninput` callback instead) — Checkbox/Radio's design explicitly requires `bind:checked`/`bind:group` on their own native input per RESEARCH.md Pattern 2 and PATTERNS.md's group-binding guidance.
- **Fix:** Changed `const { checked = $bindable(false), ... }` → `let { checked = $bindable(false), ... }` in both files (same for `group` in Radio.svelte). No change to prop shape, defaults, or naming.
- **Files modified:** `ui/src/lib/components/Checkbox.svelte`, `ui/src/lib/components/Radio.svelte`
- **Verification:** `pnpm --dir ui lint` and `pnpm --dir ui svelte-check` both exit 0 with 0 errors after the fix (previously 3 compiler errors)
- **Committed in:** `70158e4` (part of Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Necessary for the component to compile at all; no scope creep — same prop names, defaults, and public shape as specified, only the `const`/`let` keyword changed.

## Issues Encountered
None beyond the deviation above.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- Input/Select/Textarea now match `Fields.dc.html`'s `ctrlBase()` exactly across bg/border/error states
- Checkbox.svelte and Radio.svelte exist as first-class, reusable primitives — ready for phases 26-28 to adopt at real call-sites (every checkbox in the app today is still a raw unstyled `<input type=checkbox>`, out of scope here per D-07)
- FieldsSection.svelte compiles standalone, ready for Plan 07 to import into the showcase page assembly
- No blockers for Wave 1 remaining plans (24-04 through 24-06) or Plan 07

---
*Phase: 24-base-components*
*Completed: 2026-07-18*

## Self-Check: PASSED

All 7 files verified present on disk (Checkbox.svelte, Radio.svelte, FieldsSection.svelte, Input.svelte, Select.svelte, Textarea.svelte, this SUMMARY); all 3 task commits (e678af8, 70158e4, 59730b0) verified in git log.

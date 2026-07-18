---
phase: 24-base-components
plan: 13
subsystem: ui-base-components
tags: [badge, input, css-specificity, gap-closure, wr-05, wr-06]
requirements: [CMP-02, CMP-03]
dependency-graph:
  requires: ["24-08", "24-09"]
  provides: ["Badge.svelte count-appearance size fix", "Input.svelte string-contract fix"]
  affects: ["ui/src/features/showcase/sections/BadgeSection.svelte", "ui/src/features/acts/ActNumberField.svelte"]
tech-stack:
  added: []
  patterns:
    - "CSS specificity-independent property split: shared dimensions in a base rule, tone blocks only override color-channel properties"
    - "Avoid native bind:value on typed inputs where Svelte's DOM-level type coercion can violate a component's string prop contract; use one-way value={value} + explicit assignment inside oninput instead"
key-files:
  created: []
  modified:
    - ui/src/lib/components/Badge.svelte
    - ui/src/lib/components/Input.svelte
decisions:
  - "Gave all 5 Badge tones (including neutral) a border for appearance=\"count\" — majority pattern (4 of 5 tones already had one) — rather than removing borders from the other 4."
  - "Fixed WR-06 via the 'normalize before the contract boundary' remedy (one-way value={value} + imperative value = v assignment in oninput) rather than narrowing Input's type prop union, to avoid breaking ActNumberField.svelte's type=\"number\" usage."
metrics:
  duration: 5 min
  completed: 2026-07-18
---

# Phase 24 Plan 13: Badge count-size + Input number-contract gap closure Summary

Fixed two WARNING-level regressions from prior 24-phase plans: Badge's `size="sm"` was silently overridden for 4 of 5 tones on `appearance="count"` due to CSS specificity, and Input's native `bind:value` leaked `number | null` into its `string`-typed prop for `type="number"` fields.

## What Was Built

**Badge.svelte (WR-05):** Unified `appearance="count"` sizing into two CSS rules: the base `.badge-m-count` (now the single source of truth for `md`-size dimensions — `height: 20px`, `padding: 0 9px`, plus a new `border: 1px solid var(--tr-border-strong)` covering all tones) and a new `.badge-m-count.badge-m-sm` rule (`height: 18px`, `padding: 0 7px`). The four tone-specific blocks (`success`, `warning`, `danger`, `accent`) no longer redeclare `height`/`padding`/`border-radius`/`font-size` — they now only set `background`, `color`, and `border-color` (narrowed from the old `border: 1px solid var(--tr-{tone})` shorthand). Because the tone blocks are no longer higher-specificity on the sizing properties, `.badge-m-sm` on the outer element applies to every tone without contention.

**Input.svelte (WR-06):** Removed `bind:value` from the native `<input type={type}>` element. Replaced with a one-way `{value}` display binding, and added `value = v;` as the first statement inside the existing `oninput` handler (before the `oninput?.(v)` callback), using the already-string `HTMLInputElement.value` read from `e.currentTarget`. `$bindable()` propagates on any assignment to the destructured `value` variable, so parent `<Input bind:value={x} />` two-way binding still works for every `type`, including `"number"` — without narrowing the `Props.type` union (which would have broken `ActNumberField.svelte`'s `type="number"` usage).

## Verification

- `pnpm --dir ui svelte-check`: 0 errors (48 pre-existing warnings in unrelated files, unchanged)
- `pnpm --dir ui lint`: exits 0 (eslint + prettier + check-tokens all pass)
- `pnpm --dir ui build`: exits 0
- Compiled CSS check: `grep -o "badge-m-count[a-z0-9.-]*{[^}]*height:20px" ui/dist/assets/*.css | wc -l` → `1` (single unified rule owns count's md height in the minified bundle)
- All grep-based acceptance criteria from the plan matched exactly (height:20px ×2, padding:0 9px ×1, border-radius:11px ×2, tr-border-strong ×2, badge-m-count.badge-m-sm ×1, border-color per tone ×4, bind:value removed from Input.svelte, value = v; ×1, Select.svelte/Textarea.svelte bind:value unchanged ×1 each)
- Code-level regression check: `ActNumberField.svelte` (the one production `type="number"` `<Input>` consumer) uses one-way `value={displayValue}` + `oninput` callback, never `bind:value` on `<Input>` — confirmed unaffected by the Input.svelte change.
- Manual/visual showcase check (opening the app in a browser and eyeballing the Бейджи section, per the plan's `<human-check>`) was not performed interactively by this executor; the fix's correctness was instead verified via the compiled-CSS occurrence count above (single height:20px rule for count/md across all tones) plus direct code inspection of the cascade. Per `human_verify_mode: "end-of-phase"` in project config, end-of-phase UI review will cover final visual confirmation across all Phase 24 plans.

## Deviations from Plan

None — plan executed exactly as written. Both tasks matched their acceptance criteria on the first attempt; no auto-fixes, blockers, or architectural questions arose.

## Known Stubs

None.

## Threat Flags

None — both fixes are presentational/type-contract corrections to existing client-side components, no new trust boundary or data-input surface introduced (matches the plan's threat_model disposition).

## Self-Check: PASSED

- FOUND: ui/src/lib/components/Badge.svelte (modified, commit e7e39f7)
- FOUND: ui/src/lib/components/Input.svelte (modified, commit 132256a)
- FOUND commit e7e39f7 in git log
- FOUND commit 132256a in git log

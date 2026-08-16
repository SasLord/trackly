---
quick_id: 260805-lrs
slug: employee-header-full-name-must-use-available-width
phase: 260805-lrs
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - ui/src/features/layout/EmployeeLayout.svelte
autonomous: true
requirements: [LRS-01]
must_haves:
  truths:
    - "On a wide LAN-browser viewport, the employee header shows the FULL name (e.g. «Иванов Александр Дмитриевич») with no ellipsis, because `.user-name` no longer carries a fixed `max-width: 200px` ceiling"
    - "On a narrow viewport, the NAME is the element that shrinks and shows an ellipsis — `.user-role` ('Сотрудник'), the theme switcher, and the 'Выйти' button keep their intrinsic size and never wrap or get squeezed"
    - "The shrink capability propagates from the flex header row down to `.user-name` — `.employee-header-actions` and `.user-name` both have `min-width: 0` so the browser's flex default (`min-width: auto`, i.e. 'never shrink below content size') does not block the ellipsis from ever engaging"
  artifacts:
    - path: "ui/src/features/layout/EmployeeLayout.svelte"
      provides: "`.user-name` grows to fill available header width and only truncates under real space pressure; `.user-role`/theme switcher/Выйти stay fixed-size"
      contains: "min-width: 0"
  key_links:
    - from: ".employee-header-actions (flex container, min-width: 0)"
      to: ".user-name (flex item, flex-shrink: 1, min-width: 0, no max-width ceiling)"
      via: "flexbox shrink propagation — without min-width: 0 on BOTH the container and the item, the browser's default min-width: auto refuses to shrink the name below its content size and the ellipsis never fires"
      pattern: "min-width: 0"
---

<objective>
Fix the employee header so the full name (`.user-name`, `EmployeeLayout.svelte`) uses all
available width on a wide screen, and only shrinks with an ellipsis when the viewport is actually
narrow. Today `.user-name` has a hardcoded `max-width: 200px` PLUS `flex-shrink: 0` — the pair
means the name is always clipped at exactly 200px regardless of how much empty space sits to its
right in the header row, which is the reported defect (user report + screenshot: "Иванов Александр Дмитриевич" truncated on a wide screen with most of the header empty). Confirmed in the file
at `.user-name` (~line 166-175); not a regression — originated in commit 0667f1c (2026-06-21, plan
10-04) when `EmployeeLayout.svelte` was first created.

Purpose: the employee-facing header must never hide information the screen has room to show;
truncation is a narrow-viewport fallback, not a permanent 200px cap.

Output: `.user-name` participates in flex shrinking with no fixed ceiling, shrinks only when
`.employee-header-actions` runs out of room, and `.user-role`/the theme switcher/the "Выйти"
button remain fixed-size and never wrap or get squeezed by the change.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@CLAUDE.md
@.planning/STATE.md

<interfaces>
<!-- Current relevant CSS in ui/src/features/layout/EmployeeLayout.svelte (style block, lines
     ~143-186). Markup at lines ~113-128: header.employee-header (flex, justify-content:
     space-between) > [span.employee-brand, div.employee-header-actions (flex, gap: var(--tr-space-md))
     > [span.user-name, span.user-role, div.theme-switcher-slot, Button "Выйти"]]. -->

Current `.employee-header-actions` (unchanged apart from adding min-width: 0):
```
.employee-header-actions {
  display: flex;
  align-items: center;
  gap: var(--tr-space-md);
}
```

Current `.user-name` (the bug — max-width: 200px + flex-shrink: 0 together always clip at 200px
regardless of available space):
```
.user-name {
  font-size: var(--tr-font-size-body);
  font-weight: var(--tr-font-weight-medium);
  color: var(--tr-text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 200px;
  flex-shrink: 0;
}
```

Current `.user-role` (has no explicit shrink/wrap control today — must become non-shrinking,
non-wrapping so it never yields space instead of `.user-name`):
```
.user-role {
  font-size: var(--tr-font-size-label);
  color: var(--tr-text-tertiary);
}
```

`.theme-switcher-slot` already has `flex-shrink: 0; width: max-content;` — leave unchanged, it is
already correctly non-shrinking. The `Button` component for "Выйти" is not styled by this file and
is out of scope.
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Let the employee-header name grow to available width, shrink only under real pressure</name>
  <files>ui/src/features/layout/EmployeeLayout.svelte</files>
  <action>
Edit ONLY the `&lt;style lang="scss"&gt;` block in this file. Three changes, all inside the
existing selectors shown in `&lt;interfaces&gt;` — do not touch the `&lt;script&gt;` or markup
sections, and do not touch any other `max-width: 200px` occurrence anywhere else in the codebase
(`ActNumberField.svelte`, `DeviceListRow.svelte`, `DeviceImportCsvModal.svelte` are unrelated
table/form contexts and out of scope for this fix).

1. `.employee-header-actions`: add `min-width: 0;` to the existing rule (alongside `display: flex;
   align-items: center; gap: var(--tr-space-md);`). This lets the flex container itself shrink
   below its content's natural width so the shrink pressure can reach `.user-name` — without it,
   the container refuses to shrink and nothing downstream ever gets squeezed.

2. `.user-name`: remove `max-width: 200px;` entirely (no replacement value — there must be no
   fixed ceiling). Change `flex-shrink: 0;` to `flex-shrink: 1;` (or remove the declaration, since
   `1` is the flexbox default — pick whichever reads clearer, but the item MUST be shrinkable).
   Add `min-width: 0;` to the same rule — a flex item's default `min-width: auto` refuses to
   shrink below its own content size, which would make the ellipsis dead code even with
   `flex-shrink: 1`. Keep `white-space: nowrap;`, `overflow: hidden;`, and `text-overflow:
   ellipsis;` exactly as they are — those three together are what makes the ellipsis render once
   the element actually shrinks. Keep the existing `font-size`, `font-weight`, `color`
   declarations unchanged.

3. `.user-role`: add `flex-shrink: 0;` and `white-space: nowrap;` to the existing rule (alongside
   `font-size: var(--tr-font-size-label); color: var(--tr-text-tertiary);`). This guarantees
   "Сотрудник" keeps its full text and intrinsic size under space pressure — `.user-name` is the
   only element in `.employee-header-actions` that is allowed to give up space.

Do not add a `title`/tooltip attribute, do not add any new markup, do not introduce a new
`--tr-*` token (no new spacing value is needed — this is a shrink/min-width fix, not a layout
change).
  </action>
  <verify>
    <automated>bash /Users/madsas/Projects/trackly/.planning/quick/260805-lrs-employee-header-full-name-must-use-avail/verify.sh</automated>
  </verify>
  <done>
`.employee-header-actions` has `min-width: 0`. `.user-name` has no `max-width: 200px`, has
`min-width: 0`, has `flex-shrink: 1` (or no `flex-shrink` declaration at all), and still has
`white-space: nowrap; overflow: hidden; text-overflow: ellipsis;`. `.user-role` has
`flex-shrink: 0; white-space: nowrap;` in addition to its existing declarations. No other file's
`max-width: 200px` occurrence was touched. `pnpm svelte-check`, `pnpm lint` (token/contrast/focus/
CSP-hash gates), and `pnpm build` all pass clean.
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| N/A | Pure CSS sizing fix inside an already-rendered, already-trusted `authStore.user.fullName` display — no new input parsing, no new data flow, no new network surface. |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-lrs-01 | Information Disclosure | `.user-name` now rendering more/all of `fullName` instead of clipping at 200px | accept | `fullName` is the logged-in user's own name, already sent to their own authenticated session and already fully present in the DOM (`overflow: hidden` only affects visual presentation, not DOM content or accessibility tree) — showing more of it on screen discloses nothing that wasn't already accessible via view-source or a screen reader. |
| T-lrs-SC | Tampering (supply chain) | N/A | accept | No new dependency, no package install — edits one existing `.svelte` file's `&lt;style&gt;` block only. Package Legitimacy Gate not applicable. |
</threat_model>

<verification>
1. `pnpm --dir ui svelte-check` — 0 errors.
2. `pnpm --dir ui lint` — clean, including token/contrast/focus/CSP-hash gates.
3. `pnpm --dir ui build` — succeeds.
4. Structural gate (`verify.sh`, Task 1 `&lt;verify&gt;`) — proves: `.user-name` has no
   `max-width: 200px`, has `min-width: 0`, is not `flex-shrink: 0`, and keeps
   `white-space: nowrap; overflow: hidden; text-overflow: ellipsis;`; `.employee-header-actions`
   has `min-width: 0`; `.user-role` has `flex-shrink: 0; white-space: nowrap;`; the three unrelated
   `max-width: 200px` occurrences elsewhere in the codebase are untouched.
5. Manual/human-check (NOT automatable — no frontend test framework can assert rendered width
   behaviour): open the employee view (`#/login` as an employee, or any LAN browser session) at
   BOTH viewport widths:
   a. Wide viewport (e.g. desktop browser window, ~1200px+): confirm the full name renders with no
      ellipsis, e.g. "Иванов Александр Дмитриевич" in full.
   b. Narrow viewport (resize the browser window down to ~500-600px, or use responsive/mobile
      device emulation): confirm the NAME shrinks and shows an ellipsis, while "Сотрудник", the
      theme switcher icon, and the "Выйти" button all keep their normal size and do not wrap onto
      a second line or get visually squeezed.
   If this plan is executed without a live browser to verify against, flag steps a-b as a pending
   follow-up UAT in the SUMMARY.
</verification>

<success_criteria>
- Wide viewport: `.user-name` renders the complete `fullName` with no ellipsis and no artificial
  200px ceiling.
- Narrow viewport: `.user-name` is the sole element that shrinks and ellipsises; `.user-role`, the
  theme switcher, and "Выйти" remain full-size and unwrapped.
- No fixed pixel `max-width` remains on `.user-name`.
- The other three `max-width: 200px` occurrences in the codebase (`ActNumberField.svelte`,
  `DeviceListRow.svelte`, `DeviceImportCsvModal.svelte`) are untouched.
- `pnpm --dir ui svelte-check`, `pnpm --dir ui lint`, and `pnpm --dir ui build` all pass.
</success_criteria>

<output>
Create `.planning/quick/260805-lrs-employee-header-full-name-must-use-avail/260805-lrs-SUMMARY.md` when done
</output>

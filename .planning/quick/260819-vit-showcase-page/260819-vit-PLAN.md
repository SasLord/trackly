---
quick_id: 260819-vit
type: quick
autonomous: true
files_modified:
  - ui/src/features/showcase/ShowcasePage.svelte
must_haves:
  truths:
    - "Витрина компонентов (Showcase) прокручивается колесом мыши, когда контент не влезает в высоту .content"
    - "Витрина по-прежнему не создаёт второй скроллбар на уровне .content (app-shell остаётся overflow:hidden)"
  artifacts:
    - path: "ui/src/features/showcase/ShowcasePage.svelte"
      provides: ".showcase-page follows the established app-shell scroll contract (height:100%, min-height:0, overflow-y:auto)"
  key_links:
    - from: "ui/src/features/showcase/ShowcasePage.svelte (.showcase-page)"
      to: "ui/src/features/layout/Layout.svelte (.content, overflow:hidden)"
      via: "page owns its own vertical scroll region inside the fixed-height shell"
      pattern: "overflow-y:\\s*auto"
---

<objective>
Fix `.showcase-page` in `ui/src/features/showcase/ShowcasePage.svelte` so it follows the
same app-shell scroll contract as every other page (Dashboard, Reports, Requests, Settings,
etc.). Currently it only has `padding` + flex layout, with no `height`/`overflow-y`, so when
the seven showcase sections overflow the viewport the extra content is visually clipped by
the parent `.content` (which is `overflow: hidden` by design — see `Layout.svelte`) and only
reachable via Tab, not mouse wheel/scrollbar.

Purpose: restore mouse-wheel/scrollbar access to the full showcase page — a dev/QA tool used
to visually verify every design-system primitive — without touching any other page or the
app shell itself.

Output: `.showcase-page` scrolls its own content; `.content` shell still never scrolls.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@ui/src/features/showcase/ShowcasePage.svelte
@ui/src/features/layout/Layout.svelte

<established_pattern>
Confirmed by reading `Layout.svelte` and multiple sibling pages (DashboardPage.svelte,
RequestsPage.svelte, ReportsPage.svelte, SettingsPage.svelte):

`.content` (the `<main>` in `Layout.svelte`) is deliberately `height: 100vh; overflow: hidden;
min-height: 0;` — a fixed-height shell that must NEVER scroll itself. Every `*-page` therefore
owns its own internal scroll region by being `height: 100%` (fills `.content` exactly) plus
`min-height: 0` (so a flex column doesn't grow past its parent via the `min-height: auto`
default) plus `overflow-y: auto` (or `overflow: auto` on an inner scroll child, when the page
splits header/content into two elements).

`DashboardPage.svelte`'s `.dashboard-page` rule documents the rationale directly (verbatim
excerpt, this is the canonical explanation in the codebase):

  height: 100%;
  min-height: 0;

  // Fill .content exactly (height: 100%, same as every other *-page) and let
  // [inner scroll region] scroll internally. The one thing the original was missing is
  // `min-height: 0`: without it a flex column's default `min-height: auto` lets
  // the tall content expand this box past its parent, which — before .content became
  // `overflow: hidden` — produced the app-shell double scroll.

Pages that split header/nav from body content (Dashboard, Requests, Reports, Settings) put
`overflow-y: auto` (or `overflow: auto`) on the INNER content div and keep the outer `*-page`
scroll-less (`height: 100%; min-height: 0;` only). `ShowcasePage.svelte` has NO such split —
its `<div class="showcase-page">` directly contains the `<h1>`, intro `<p>`, and every
`<section class="showcase-block">`. Per the constraint to not restructure markup, the correct
form here is to put the full triad directly on `.showcase-page` itself:

  height: 100%;
  min-height: 0;
  overflow-y: auto;

This is the single-container variant of the same contract — no markup change needed, no new
wrapper div introduced.
</established_pattern>
</context>

<tasks>

<task type="auto" tdd="false">
  <name>Task 1: Add app-shell scroll contract to .showcase-page</name>
  <files>ui/src/features/showcase/ShowcasePage.svelte</files>
  <action>
    In the `<style lang="scss">` block, update the `.showcase-page` rule to add
    `height: 100%;`, `min-height: 0;`, and `overflow-y: auto;` alongside the existing
    `padding`, `display: flex`, `flex-direction: column`, and `gap` declarations. Do not
    change the markup (the `<div class="showcase-page">` and its children stay exactly as
    they are) and do not touch `.intro` or `.showcase-block`. Resulting rule:

    .showcase-page {
      height: 100%;
      min-height: 0;
      overflow-y: auto;
      padding: var(--tr-space-xl);
      display: flex;
      flex-direction: column;
      gap: var(--tr-space-2xl);
    }

    This mirrors the pattern documented in DashboardPage.svelte's `.dashboard-page` comment,
    collapsed into a single container since Showcase has no separate header/content split.
  </action>
  <verify>
    <automated>grep -A8 '\.showcase-page {' ui/src/features/showcase/ShowcasePage.svelte | grep -c 'height: 100%;\|min-height: 0;\|overflow-y: auto;'</automated>
  </verify>
  <done>
    `grep -A8 '\.showcase-page {' ui/src/features/showcase/ShowcasePage.svelte` shows all
    three of `height: 100%;`, `min-height: 0;`, `overflow-y: auto;` present inside the
    `.showcase-page` rule (verify command above returns `3`), alongside the untouched
    `padding`/`display`/`flex-direction`/`gap` declarations. `.intro` and `.showcase-block`
    rules are byte-for-byte unchanged.
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

None. This is a CSS-only layout fix on an internal dev/QA showcase page with no user input,
no data flow, and no network/DB touch.

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|------------------|
| T-260819vit-01 | N/A | ShowcasePage.svelte | accept | Pure CSS scroll-container fix; no new trust boundary, no user input, no external package installs. |
</threat_model>

<verification>
1. `grep -A8 '\.showcase-page {' ui/src/features/showcase/ShowcasePage.svelte` shows the three
   new declarations inside the rule (static check — see Task 1 `<verify>`).
2. Visual confirmation happens in the running desktop app (Tauri dev / `cargo tauri dev`):
   open Витрина компонентов, shrink the window or zoom in until the seven sections overflow
   the viewport, and confirm the mouse wheel / scrollbar now scrolls the page (this cannot be
   verified via a synthetic Playwright/Chromium harness in this project — WKWebView is the
   real target, per project convention). No `pnpm --dir ui build` is required since this is a
   desktop-only dev page not exercised via the LAN/server-mode browser path.
</verification>

<success_criteria>
- `.showcase-page` in `ui/src/features/showcase/ShowcasePage.svelte` has `height: 100%;`,
  `min-height: 0;`, and `overflow-y: auto;` alongside its existing declarations.
- No other file is modified; no markup change; `.intro` and `.showcase-block` untouched.
- `.content` in `Layout.svelte` remains `overflow: hidden` (unmodified) — the shell still
  never scrolls; only `.showcase-page` scrolls internally.
</success_criteria>

<output>
Create `.planning/quick/260819-vit-showcase-page/260819-vit-SUMMARY.md` when done
</output>

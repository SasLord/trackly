---
phase: 260702-vtf-y-tooltip
plan: "01"
subsystem: frontend/dashboard
tags: [chart, svg, grouped-bar, tooltip, y-axis, dashboard]
dependency_graph:
  requires: []
  provides: [grouped-bar-chart-widget]
  affects: [ui/src/features/dashboard/ChartWidget.svelte]
tech_stack:
  added: []
  patterns: [svelte5-runes, svg-derived, absolute-tooltip]
key_files:
  created: []
  modified:
    - ui/src/features/dashboard/ChartWidget.svelte
decisions:
  - niceMax rounds maxVal up to a nice multiple of 1/5/10/20/50 (always >= 1 to prevent div-by-zero — T-vtf-01)
  - Tooltip implemented as absolutely-positioned div inside .chart-area (not SVG <title>) for reliable cross-browser styling
  - onmouseenter uses e.clientX/Y minus areaEl.getBoundingClientRect() to compute offset relative to .chart-area (SVG offsetX is unreliable across browsers when SVG is scaled)
  - BAR_W clamped to max(4, ...) to prevent invisible bars at large N or small M
metrics:
  duration: ~10 min
  completed: "2026-07-02"
---

# Quick Task 260702-vtf-01: ChartWidget grouped bar chart with Y-axis and tooltip

**One-liner:** Rewrote SVG line chart to grouped vertical bar chart with numeric Y-axis, gridlines, value labels above bars, and hover tooltip (month · model: N).

## What Was Done

Completely replaced the SVG polyline/circle implementation inside `ChartWidget.svelte` with a grouped bar chart. All preserved elements (loading/error/empty states, sr-only accessibility table, legend, PeriodToggle) remain identical.

### Key changes

**Coordinate system:** viewBox `0 0 500 220`, LEFT_PAD=42 (room for 4-digit Y-labels), RIGHT_PAD=8, TOP_PAD=20 (value labels), BOTTOM_PAD=28 (X-axis labels). CHART_W=450, CHART_H=172.

**Y-axis + gridlines:** `niceMax` derived from `maxVal` rounded up to nearest nice step (1/5/10/20/50); minimum 1 (T-vtf-01 mitigation). 5 evenly-spaced ticks with horizontal `<line>` gridlines and `<text>` labels at `x = LEFT_PAD - 4`.

**Grouped bars (`$derived barLayout`):** GROUP_W = CHART_W/N, BAR_W = max(4, (GROUP_W - GAP*(M+1))/M). Each bar: x, y, width, height, color, installs, model, monthLabel. rx=2 (rounded corners).

**Value labels:** `<text>` above each bar where installs > 0.

**X-axis labels:** centered per group at `LEFT_PAD + i*GROUP_W + GROUP_W/2`.

**Tooltip:** `$state<TooltipState>` with visible, x, y, month, model, installs. `onmouseenter` computes position relative to `.chart-area` via `getBoundingClientRect()` (reliable with scaled SVG). `onmouseleave` hides it. Tooltip div: `position: absolute; pointer-events: none; z-index: 10`.

**Removed:** `toCoords()`, `toPolyline()` functions and `<polyline>`, `<circle>` elements.

## Verification

- `pnpm --dir ui exec svelte-check` (from ui/): **0 errors**, 38 pre-existing warnings in unrelated files (CartridgeFormBody, CompatibilityEditor, ModelFormModal, PeriodSelector, AccessDenied) — no new warnings in ChartWidget.svelte.
- `pnpm --dir ui build`: **exit 0**, ui/dist updated (1.79s). Pre-existing Vite info message about client.ts dynamic import — unrelated to this change.
- `git status`: ui/dist not tracked (correctly gitignored).

## Commits

| Hash | Description |
|------|-------------|
| 4ccc179 | feat(260702-vtf-01): rewrite ChartWidget as grouped bar chart with Y-axis and tooltip |

## Deviations from Plan

None — plan executed exactly as written.

Tooltip offset computed via `getBoundingClientRect()` rather than raw `e.offsetX/Y` — this is a reliability improvement noted in-code, not a deviation from the plan's semantic intent (plan said "относительно `.chart-area`"). SVG `offsetX` is viewport-relative when the SVG is CSS-scaled (width: 100%), so using `clientX - areaRect.left` is the correct implementation of the plan's specification.

## Known Stubs

None — all data flows from the existing `data: ConsumptionPoint[]` prop.

## Threat Flags

None — no new network endpoints or auth paths introduced.

## Human Verify (pending)

The final live visual check is performed by the user:
1. Run `cargo tauri dev` (desktop) or open the LAN browser after `pnpm --dir ui build`.
2. Navigate to the dashboard → "Динамика расхода картриджей" widget.
3. Verify: grouped bars visible, Y-axis with numbers, value labels above bars, hover tooltip shows "Месяц · Модель: N".
4. Switch period (3/6/12 мес.) — bars rebuild.

## Self-Check: PASSED

- [x] `ui/src/features/dashboard/ChartWidget.svelte` exists and was modified
- [x] Commit `4ccc179` exists in git log
- [x] svelte-check: 0 errors
- [x] pnpm build: exit 0
- [x] ui/dist not in git status

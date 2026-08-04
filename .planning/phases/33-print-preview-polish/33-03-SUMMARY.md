---
phase: 33-print-preview-polish
plan: 03
subsystem: ui
tags: [pagedjs, svelte5, postmessage, iframe, print-preview, vite]

# Dependency graph
requires:
  - phase: 33-print-preview-polish (plan 01)
    provides: "buildSrcdoc(actHtml, theme), THEME_CHROME, attachBridge(iframeEl, onMsg), pluralizeRu(n, forms) — the frozen srcdoc/bridge contract"
provides:
  - "PdfPreviewModal.svelte on-screen preview reworked to render a real, paginated A4 sheet stack via Paged.js"
  - "D-02 degraded-path timeout/error fallback to the pre-Phase-33 unpaginated iframe"
  - "D-08/D-09 sheet-chrome CSS fix (theme-following backdrop, no border, box-shadow only)"
  - "D-11 fit-to-width transform:scale wrapper (ceiling of 1, no horizontal scroll)"
  - "D-10/D-03 footer page counter + print-dialog hint line"
  - "D-07 Печать button gated on pagination settling, not just on HTML arrival"
  - "Fix for pagedjs's package.json exports map blocking its dist bundle's deep `?raw` import once actually wired into the app's entry graph"
affects: [33-04-print-paths]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Imperative (non-$derived) srcdoc construction inside a render $effect, to avoid iframe reload on unrelated reactive changes (theme toggle)"
    - "Live cross-iframe theme propagation via postMessage instead of srcdoc reassignment"
    - "transform: scale() fit-to-width wrapper (outer div sized to scaled height, inner div holding natural size + transform) for D-11's no-horizontal-scroll guarantee"
    - "Relative filesystem import (not bare package specifier) as the workaround for a dependency's package.json exports map blocking a deep `?raw` bundle import"

key-files:
  created: []
  modified:
    - ui/src/features/acts/PdfPreviewModal.svelte
    - ui/src/lib/pdfPreview/pagedPreviewBootstrap.ts

key-decisions:
  - "srcdoc is built exactly once per render (inside the async render effect's try block), never as a $derived — a $derived would re-run and reload the iframe on every later theme change, destroying in-progress pagination state"
  - "Degraded-path fallback renders the byte-identical pre-Phase-33 markup (sandbox=\"\", raw srcdoc={htmlContent}, no chrome) rather than a new 'error-lite' UI, per D-02's literal contract"
  - "pagedjs's dist bundle is now imported via a relative filesystem path (../../../node_modules/pagedjs/dist/paged.min.js?raw) instead of the bare package specifier, since pagedjs's package.json exports map has no ./dist/* subpath entries and Vite/Rollup's strict exports resolution rejects the deep import once the module is reachable from the app's entry graph"

patterns-established:
  - "Paged.js on-screen preview state machine: paginationStatus: 'idle' | 'pending' | 'done' | 'degraded', driven by postMessage progress/done/error events plus an 8s client-side timeout"

requirements-completed: [PRV-01, PRV-02]

# Metrics
duration: ~35min
completed: 2026-08-04
---

# Phase 33 Plan 03: Paged.js on-screen preview wiring Summary

**`PdfPreviewModal.svelte` now renders a real paginated A4 sheet stack via Paged.js — themed backdrop, no-border shadowed sheets, fit-to-width scaling, footer page counter/hint, and an 8-second degrade-to-unpaginated fallback — consuming Plan 33-01's frozen srcdoc/bridge contract, plus a Rule-3 fix for a pagedjs deep-import resolution bug surfaced by wiring it in.**

## Performance

- **Duration:** ~35 min
- **Started:** 2026-08-04T21:55:00+07:00 (approx)
- **Completed:** 2026-08-04T22:25:00+07:00
- **Tasks:** 3/3
- **Files modified:** 2 (PdfPreviewModal.svelte across 3 task commits, pagedPreviewBootstrap.ts in a follow-up deviation commit)

## Accomplishments
- Wired `buildSrcdoc`/`attachBridge`/`THEME_CHROME`/`pluralizeRu` (Plan 33-01's frozen contract) into `PdfPreviewModal.svelte`'s render effect, a new postMessage-bridge effect, and a new live-theme-propagation effect
- Implemented D-02's 8-second degrade timeout plus explicit `trackly-pagedjs-error` handling, both falling back to the exact pre-Phase-33 unpaginated iframe markup
- Fixed the D-08/D-09 backdrop/sheet-chrome CSS bug (`--tr-surface-sunken` backdrop, no border, box-shadow only) and added the D-11 fit-to-width `transform: scale()` wrapper (ceiling of 1)
- Added the D-10/D-03 footer meta block (RU-pluralized page count + print-dialog hint line), gated to only appear once pagination has settled successfully
- Gated the "Печать" button's `disabled` state on `paginationStatus` settling (D-07), not merely on the backend HTML string arriving
- Fixed the RU-only iframe accessible name defect (`title="Document Preview"` → `"Предпросмотр документа"`) on both iframe branches
- Found and fixed a real production-build blocker: pagedjs's dist bundle deep `?raw` import (created inert in Plan 33-01) broke `vite build` once this plan made it reachable from the app's entry graph, because pagedjs's `package.json` `exports` map has no `./dist/*` subpath entries

## Task Commits

Each task was committed atomically:

1. **Task 1: Wire srcdoc/bridge state, degraded-path timeout, live theme updates** - `eac828d` (feat)
2. **Task 2: Sheet-stack chrome CSS, fit-to-width scale, loading-progress markup** - `5846bb0` (feat)
3. **Task 3: Footer page counter + hint line, Печать disabled logic, iframe a11y name** - `8797663` (feat)

**Deviation fix (Rule 3 - blocking):** `49ce1fb` (fix) — pagedjs deep-import resolution

**Plan metadata:** (this commit)

_No TDD tasks in this plan (project has no frontend test framework — see verification_reality)._

## Files Created/Modified
- `ui/src/features/acts/PdfPreviewModal.svelte` - reworked on-screen preview: srcdoc/bridge state wiring, degraded fallback, sheet-stack chrome CSS, fit-to-width scale wrapper, footer meta block, Печать gating, RU iframe title
- `ui/src/lib/pdfPreview/pagedPreviewBootstrap.ts` - changed the pagedjs dist-bundle `?raw` import from the bare package specifier to a relative filesystem path (deviation fix, see below)

## Decisions Made
- Followed the plan's literal state/markup/CSS instructions verbatim (imperative `srcdoc`, timeout+error → `enterDegraded`, `scaleFactor = Math.min(1, frameWidthPx / 794)`, exact RU copy strings) — no deviations from the documented UI-SPEC contract
- Reset `srcdoc`/`paginationStatus` to `null`/`'idle'` when the modal's `ready` condition goes false (not explicitly spelled out in the plan's action text, but a direct extension of the existing `htmlContent = null; errorMsg = null;` reset pattern already in that branch — prevents a stale page count from a previous document flashing on next open)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Fixed pagedjs dist-bundle import blocked by package `exports` map**
- **Found during:** Post-Task-3 full verification (`pnpm --dir ui build`, run per `verification_reality` guidance in addition to the plan's own `svelte-check`/`lint` checks)
- **Issue:** `pagedPreviewBootstrap.ts` (created in Plan 33-01) imports `pagedjsLibraryText` via `import pagedjsLibraryText from 'pagedjs/dist/paged.min.js?raw';`. This file was dead code (unimported by anything reachable from the app entry) until this plan's Task 1 imported `buildSrcdoc` into `PdfPreviewModal.svelte` — which IS reachable from the entry graph. `vite build` then failed: `[commonjs--resolver] Missing "./dist/paged.min.js" specifier in "pagedjs" package`. pagedjs's `package.json` `exports` map only exposes the bare root specifier (`import`/`require`/`browser`/`polyfill`/`default` conditions), no `./dist/*` subpath — Rollup's strict Node exports resolution rejects the deep import.
- **Fix attempt 1 (reverted):** Added a `resolve.alias` entry in `vite.config.ts` mapping the exact specifier to the on-disk file. Did not work — the bundled commonjs-resolver plugin throws before Vite's alias plugin gets a chance to intercept, even after clearing the `node_modules/.vite` cache. Reverted (confirmed `git diff --stat ui/vite.config.ts` is empty).
- **Fix attempt 2 (applied):** Changed the import in `pagedPreviewBootstrap.ts` to a relative filesystem path (`../../../node_modules/pagedjs/dist/paged.min.js?raw`), which resolves as a plain filesystem path and bypasses package `exports` enforcement entirely. pagedjs is a direct `ui` dependency, so pnpm always places it at `ui/node_modules/pagedjs` — this path is stable. The resulting file bytes read are identical; only the resolution mechanism changed. Does not touch `bootstrapScript.js` (still frozen, untouched) or the concatenation formula (`pagedjsLibraryText + ';\n' + bootstrapText`) — Plan 33-02's CSP hash-source gate (`check-pagedjs-csp-hash.mjs`) reads the same file independently via `node:fs` and still passes.
- **Files modified:** `ui/src/lib/pdfPreview/pagedPreviewBootstrap.ts`
- **Verification:** `pnpm --dir ui build` succeeds (exit 0); `pnpm --dir ui svelte-check` 0 errors; `pnpm --dir ui lint` passes including `check-pagedjs-csp-hash`
- **Committed in:** `49ce1fb`

---

**Total deviations:** 1 auto-fixed (1 blocking-issue fix)
**Impact on plan:** Necessary for the plan's own wiring work to actually produce a shippable build; no scope creep beyond the minimum import-path change. `bootstrapScript.js` and the CSP hash-source contract are unaffected.

## Issues Encountered

**Splitting Task 1's state wiring into its own commit produces transient `svelte-check` "declared but never read" errors** for `srcdoc`/`pageProgress`/`pageTotal`/`naturalHeightPx` — these state variables are declared in Task 1 but only consumed by Task 2/3's template edits on the same file, exactly as the plan's own `<done>` note for Task 1 flags ("Template markup changes land in Tasks 2-3"). Not treated as a defect: verified 0 remaining unused-variable errors after Task 2 (down to 1: `pageTotal`) and after Task 3 (0), confirming the transient state resolved exactly as the plan anticipated.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Plan 33-04 (print-path rework, D-06) can proceed — the on-screen preview contract (srcdoc/bridge wiring, `paginationStatus` state machine, `THEME_CHROME`) is now live in `PdfPreviewModal.svelte` and available for the print branches to reference if needed. No blockers identified.

**Visual fidelity is NOT verified by this plan's automated checks** (svelte-check/lint/build only prove the code compiles and the CSP hash stays in sync) — per `verification_reality`, actual on-screen rendering (A4 sheet stack on a themed backdrop, correct margins, no horizontal scroll, degraded-path fallback triggering correctly, dark-theme sheet staying white) is deferred to the manual UAT checklist in `33-VALIDATION.md`, to be run in both Tauri desktop and a real LAN browser in both light and dark theme.

---
*Phase: 33-print-preview-polish*
*Completed: 2026-08-04*

## Self-Check: PASSED

SUMMARY.md verified to exist on disk; all 5 referenced commit hashes
(`eac828d`, `5846bb0`, `8797663`, `49ce1fb`, `c4a6f25`) verified present
in `git log --oneline --all`.

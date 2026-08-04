---
phase: 33-print-preview-polish
plan: 04
subsystem: ui
tags: [pagedjs, svelte5, postmessage, print, tauri, csp]

# Dependency graph
requires:
  - phase: 33-print-preview-polish (plan 01)
    provides: "PAGED_PREVIEW_INLINE_SCRIPT frozen text, pagedPreviewBootstrap.ts contract"
  - phase: 33-print-preview-polish (plan 03)
    provides: "PdfPreviewModal.svelte on-screen Paged.js preview wiring (isTauri branch structure, handlePrint())"
provides:
  - "printViaSystemBrowser (desktop) embeds PAGED_PREVIEW_INLINE_SCRIPT in the temp .html and waits for the bootstrap's trackly-pagedjs-done postMessage before window.print(), instead of firing on load (C-03)"
  - "printViaTopLevel (LAN) dynamically imports pagedjs and re-runs previewer.preview() against #act-print-root before window.print(), instead of injecting unpaginated body markup"
  - "ui/src/pagedjs.d.ts — minimal ambient module declaration for the untyped pagedjs package"
affects: []

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Self-postMessage bridge for a top-level file:// document: parent === window in a non-iframe context, so the same bootstrap script's parent.postMessage('trackly-pagedjs-done') dispatches to window itself, letting a second listener trigger window.print() after pagination"
    - "Dynamic import('pagedjs') as a self-hosted, code-split ESM import for a top-level app context — no CSP script-src change needed, unlike the opaque-origin srcdoc iframe's inline script"

key-files:
  created:
    - ui/src/pagedjs.d.ts
  modified:
    - ui/src/features/acts/PdfPreviewModal.svelte

key-decisions:
  - "printViaTopLevel wraps the primary previewer.preview(bodyHtml, [styleHtml], printRoot) call in try/catch with a fallback that strips the <style> wrapper tags before retrying — keeps RESEARCH.md's documented fallback for the unverified Polisher.add() stylesheet-argument shape (Open Question 2) without blocking correctness if the primary shape throws"
  - "Added ui/src/pagedjs.d.ts (Rule 3 - blocking) — pagedjs ships no TypeScript types and no @types/pagedjs package exists; svelte-check failed on the dynamic import with an implicit-any error without it"

requirements-completed: [PRV-03]

# Metrics
duration: ~20min
completed: 2026-08-04
---

# Phase 33 Plan 04: Print-path Paged.js rework Summary

**Both print branches of `PdfPreviewModal.svelte` (desktop temp-file and LAN top-level injection) now print the same Paged.js-paginated output as the on-screen preview, closing the WYSIWYG gap (PRV-03, D-06) by waiting for pagination instead of the `load` event or synchronous DOM injection.**

## Performance

- **Duration:** ~20 min
- **Started:** 2026-08-04T22:28:00+07:00 (approx)
- **Completed:** 2026-08-04T22:35:13+07:00
- **Tasks:** 2/2
- **Files modified:** 2 (1 created, 1 modified)

## Accomplishments
- `printViaSystemBrowser` now embeds the frozen `PAGED_PREVIEW_INLINE_SCRIPT` bundle (same text the on-screen preview uses) into the desktop temp `.html` file, removing the old `load`-event `setTimeout` and replacing it with a self-postMessage listener that fires `window.print()` only after the bootstrap's own `trackly-pagedjs-done` event (C-03)
- `printViaTopLevel` now dynamically imports `pagedjs` (self-hosted ESM, no CSP change needed) and re-runs `previewer.preview()` targeting `#act-print-root`, replacing the direct unpaginated `innerHTML` injection; `window.print()` now waits on the `await previewer.preview(...)` resolution
- Both branches print the already-paginated Paged.js markup rather than the browser's native print pagination, giving screen preview and paper one shared pagination engine (D-06)
- `bootstrapScript.js` was NOT touched — the CSP sha256 hash gate (`check-pagedjs-csp-hash`) still passes unchanged

## Task Commits

Each task was committed atomically:

1. **Task 1: Desktop print via Paged.js (printViaSystemBrowser)** - `7d2b098` (feat)
2. **Task 2: LAN print via Paged.js (printViaTopLevel)** - `9e0bfe3` (feat, includes the Rule 3 `pagedjs.d.ts` type-declaration fix)

**Plan metadata:** (this commit)

_No TDD tasks in this plan (project has no frontend test framework — see verification_reality)._

## Files Created/Modified
- `ui/src/features/acts/PdfPreviewModal.svelte` - reworked `printViaSystemBrowser` (embeds PAGED_PREVIEW_INLINE_SCRIPT + pagination-gated print trigger) and `printViaTopLevel` (dynamic `import('pagedjs')`, `previewer.preview()` targeting `printRoot`, print gated on pagination), plus `handlePrint()` now awaits the now-async `printViaTopLevel`
- `ui/src/pagedjs.d.ts` - new ambient module declaration covering only the `Previewer` surface actually used (`preview(content?, stylesheets?, renderTo?)`)

## Decisions Made
- Kept the plan's described fallback for `previewer.preview()`'s stylesheet argument as a runtime `try/catch`: primary shape passes `[styleHtml]` (the raw `<style>...</style>` outerHTML string) as documented in `Previewer.preview(content, stylesheets, renderTo)`'s confirmed signature; on throw, retries with the `<style>` wrapper tags stripped. This implements RESEARCH.md's Open Question 2 guidance ("implement the primary shape, keep the fallback") without leaving the LAN print path with no recovery path if the primary shape is wrong.
- Followed the plan's literal `'<' + 'script>'` concatenation idiom for both new inline script tags in `printViaSystemBrowser`, matching the existing idiom already used elsewhere in the file and in `pagedPreviewBootstrap.ts` — no literal `</script>` substring appears inside any single string.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added `ui/src/pagedjs.d.ts` ambient module declaration**
- **Found during:** Task 2 (`printViaTopLevel` dynamic `import('pagedjs')`)
- **Issue:** `pnpm --dir ui svelte-check` failed: `Could not find a declaration file for module 'pagedjs'` — the `pagedjs` package (v0.4.3) ships no `types`/`typings` field and no `@types/pagedjs` package exists on npm. `noImplicitAny: true` in `tsconfig.json` turns this into a hard error, blocking the build.
- **Fix:** Added `ui/src/pagedjs.d.ts` declaring `module 'pagedjs'` with a minimal `Previewer` class covering only the constructor and `preview(content?, stylesheets?, renderTo?): Promise<{ total: number }>` signature actually used by `printViaTopLevel`. Also had to drop a `Document` union-type option from the initial draft (`no-undef` ESLint error — `Document` is not in this project's hand-maintained `browserGlobals` allowlist in `eslint.config.js`, and it wasn't needed by the actual call site anyway).
- **Files modified:** `ui/src/pagedjs.d.ts` (new)
- **Verification:** `pnpm --dir ui svelte-check` (0 errors, 269 files), `pnpm --dir ui lint` (passes, including `check-pagedjs-csp-hash`), `pnpm --dir ui build` (exit 0, prebuild `cargo test -p trackly-app --test export_bindings` ran as part of the pnpm lifecycle and succeeded)
- **Committed in:** `9e0bfe3` (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking-issue fix)
**Impact on plan:** Necessary for the plan's own Task 2 code to type-check at all. No scope creep — the declaration covers only the surface this plan's code calls.

## Issues Encountered
None beyond the deviation above.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Phase 33 (print-preview-polish) is now feature-complete across its 4 plans (Paged.js bootstrap contract, CSP hash gate, on-screen preview wiring, print-path rework). No blockers for closing the phase, subject to the manual UAT items below.

**Visual/print fidelity is NOT verified by this plan's automated checks.** `svelte-check`/`lint`/`build` only prove the code compiles, imports resolve, and the CSP hash stays in sync with `bootstrapScript.js`'s unchanged text. Per `verification_reality` and the plan's `<known_open_risk>`:
- Whether `previewer.preview(bodyHtml, [styleHtml], printRoot)`'s primary stylesheet-argument shape (vs. the wrapper-stripped fallback) is the one that actually applies `@page` margins/fonts to `#act-print-root` in a real browser is **NOT proven by any automated check in this codebase** — no frontend rendering harness exists (VALIDATION.md). This must be confirmed via the manual UAT row in `33-VALIDATION.md`: "LAN-печать: поля и шрифты применились к `#act-print-root`".
- Desktop: confirm in a real Tauri build that the system browser's print dialog opens only after pagination completes (page count matches on-screen preview) and printed/Saved-as-PDF page breaks match the preview.
- LAN: confirm from a real LAN browser against a `pnpm --dir ui build` + server-mode axum instance that no `Content-Security-Policy` console errors appear (the dynamic `import('pagedjs')` should need no new CSP source, since it resolves as a normal bare-specifier bundle import, not the deep `?raw` path that required Plan 33-03's relative-import workaround).

---
*Phase: 33-print-preview-polish*
*Completed: 2026-08-04*

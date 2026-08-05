---
phase: 260805-jwf
plan: 01
subsystem: ui
tags: [svelte, pagedjs, print, css, tauri]

# Dependency graph
requires:
  - phase: 260805-ifj
    provides: line-height/letter-spacing/word-spacing reset for LAN print (the regression this plan fixes)
provides:
  - printViaTopLevel no longer duplicates template CSS into the app document
  - LAN print layout reset applies at Paged.js measurement time, not only under @media print
  - afterprint cleanup removes Paged.js's own injected <style> elements via Polisher.destroy()
affects: [pdf-preview, act-printing, lan-print]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Anything affecting document layout in printViaTopLevel must be unconditional (screen AND print), never gated behind @media print alone, because Paged.js measures the on-screen DOM before window.print() fires."
    - "Layout resets in printViaTopLevel must be scoped to #act-print-root, never body — the shared top-level document means body-scoped rules leak into the app's own on-screen UI."

key-files:
  created: []
  modified:
    - ui/src/features/acts/PdfPreviewModal.svelte
    - ui/src/pagedjs.d.ts

key-decisions:
  - "Merged the line-height/letter-spacing/word-spacing reset into one #act-print-root rule, declared unconditionally (outside @media print) instead of inside it, so it is in effect before Paged.js's Previewer measures/paginates the DOM (fixes the pagination mismatch, defect B)."
  - "Removed the ${cssText} interpolation from printStyle.textContent entirely — Paged.js's own previewer.preview() call already applies the identical stylesheet via its stylesheets argument, so the manual duplicate was redundant and was defect A's actual leak mechanism (it landed in the shared top-level document, unscoped)."
  - "Captured previewer.polisher after preview() resolves and call its destroy() in the afterprint cleanup, alongside clearing printRoot.innerHTML and printStyle.textContent, so nothing Paged.js or this function created in document.head/body survives past a single print cycle."
  - "Extended the project's own ambient src/pagedjs.d.ts Previewer type with the polisher property (Rule 3 auto-fix) — the plan's interface notes assumed pagedjs ships no types at all, but this project has its own minimal ambient declaration that needed extending to type-check the new previewer.polisher access."

requirements-completed: [JWF-01, JWF-02]

# Metrics
duration: 25min
completed: 2026-08-05
---

# Quick Task 260805-jwf: LAN print — stop injecting template CSS, fix pagination mismatch Summary

**Removed the redundant `${cssText}` duplication that leaked the act template's font onto the app's own UI, and moved the layout reset out of `@media print` (scoped to `#act-print-root`, applied unconditionally) so the on-screen Paged.js preview and the browser's print-preview dialog agree on page breaks; `afterprint` cleanup now also destroys Paged.js's own injected `<style>` elements.**

## Performance

- **Duration:** 25 min
- **Started:** 2026-08-05T07:11:00Z
- **Completed:** 2026-08-05T07:36:00Z
- **Tasks:** 1
- **Files modified:** 2

## Accomplishments
- `printViaTopLevel`'s `printStyle.textContent` no longer interpolates `${cssText}` into the shared top-level document — Paged.js's `previewer.preview()` remains the single source for print typography.
- The line-height/letter-spacing/word-spacing reset is now declared unconditionally, scoped to `#act-print-root` only, so it is in effect at Paged.js's measurement time (before `window.print()`) without ever touching the app's own on-screen typography.
- `afterprint` cleanup clears `printStyle.textContent` and calls the captured `Previewer`'s `polisher.destroy()`, removing Paged.js's own `data-pagedjs-inserted-styles` elements so nothing survives a print cycle.

## Task Commits

Each task was committed atomically:

1. **Task 1: Stop injecting cssText into the app document; fix the measure-vs-print mismatch; destroy Paged.js's own injected styles on cleanup** - `1f868ad` (fix)

**Plan metadata:** committed separately by the orchestrator (SUMMARY.md/STATE.md/ROADMAP.md not part of this commit per execution constraints).

## Files Created/Modified
- `ui/src/features/acts/PdfPreviewModal.svelte` - `printViaTopLevel`: removed `${cssText}` duplication, merged the layout reset into one unconditional `#act-print-root` rule, added `injectedPolisher` capture + `.destroy()` call in `afterprint` cleanup.
- `ui/src/pagedjs.d.ts` - Extended the ambient `Previewer` type with the `polisher: { destroy: () => void }` property actually used by the new cleanup code.

## Decisions Made
See `key-decisions` in frontmatter above.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Extended ambient `pagedjs.d.ts` with the `polisher` property**
- **Found during:** Task 1, `pnpm svelte-check` verification
- **Issue:** The plan's `<interfaces>` section stated pagedjs ships no TypeScript types at all ("no cast needed"), but this project already carries its own minimal ambient `declare module 'pagedjs'` in `src/pagedjs.d.ts` (added in Phase 33-04) that types `Previewer` without a `polisher` property. `previewer.polisher` therefore failed `svelte-check` with "Property 'polisher' does not exist on type 'Previewer'."
- **Fix:** Added `polisher: { destroy: () => void }` to the existing ambient `Previewer` class declaration, with a comment explaining the runtime source (`Previewer` constructor sets `this.polisher = new Polisher(false)`) and pointing at the exact `paged.esm.js` line read to confirm it.
- **Files modified:** `ui/src/pagedjs.d.ts`
- **Verification:** `pnpm svelte-check` — 0 errors (was 1 error before the fix, in this file).
- **Committed in:** `1f868ad` (Task 1 commit)

**2. [Rule 1 - Bug, self-caught during execution] Two self-introduced syntax bugs fixed before committing**
- **Found during:** Task 1, my own draft of the reset-rationale comment
- **Issue A:** The first draft of the new prose comment (explaining the reset scoping/timing) contained literal backtick characters (`` ` ``) inside a CSS block comment that itself lives inside a JS template literal (`` printStyle.textContent = `...` ``) — the backticks would have prematurely terminated that template literal, corrupting the generated CSS and likely breaking the build. **Issue B:** A later comment used the literal text `<style data-pagedjs-inserted-styles>` to describe Paged.js's DOM output; Svelte's `.svelte` file parser detects `<style` as a real tag-opening sequence even inside a `<script>` block's JS comment, which broke SFC parsing entirely (`svelte-check` reported "`<script>` was left open" and "Module has no default export" for every file importing `PdfPreviewModal.svelte`).
- **Fix:** Reworded both comments to avoid backticks and tag-like literal text (e.g. "style elements marked data-pagedjs-inserted-styles" instead of the literal tag), while preserving the same technical content.
- **Files modified:** `ui/src/features/acts/PdfPreviewModal.svelte` (comment text only, no logic change)
- **Verification:** `pnpm svelte-check` — 0 errors; `pnpm build` succeeds.
- **Committed in:** `1f868ad` (Task 1 commit) — caught and fixed before the commit was made, so the committed code never contained these bugs.

---

**Total deviations:** 2 auto-fixed (1 blocking/Rule 3, 1 bug/Rule 1 caught pre-commit)
**Impact on plan:** Both fixes were necessary to make the plan's own described change actually compile and type-check. No scope creep — no other file or behavior was touched.

## Issues Encountered

**The plan's own Task 1 automated Node verification script has two pre-existing bugs unrelated to this implementation:**

1. Its regex `/async function printViaTopLevel\(html[\s\S]*?\n}\n/` assumes the function's closing brace has zero indentation; this codebase indents component-scope functions by 2 spaces (`  }`), so the regex never matches this file (confirmed the same non-match against the pre-existing `printViaSystemBrowser` function and against `git show HEAD` before this plan's edits — this is a structural fact of the file, not something introduced by this task).
2. Its `rootBlock` extraction (`styleLit.indexOf('}', rootIdx)`) searches for `}` starting at the position of the literal search string `` '#${PRINT_ROOT_ID} {' `` — but that search string itself already contains a `}` (the one closing the `${PRINT_ROOT_ID}` JS template interpolation), so `indexOf` trivially matches that character instead of the CSS rule's actual closing brace, truncating the extracted block before `line-height: normal` etc.

Both were confirmed as verification-script bugs (not source bugs) by re-running the same checks with an indentation-tolerant regex and a brace-skip-aware block extractor — output confirmed `line-height: normal`, `letter-spacing: normal`, `word-spacing: normal`, `position: absolute`, `left: -100000px` are all present in the `#act-print-root` rule, exactly as required. The real, authoritative gates — `pnpm svelte-check`, `pnpm lint` (including the CSP-hash gate), and `pnpm build` — all pass cleanly.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Statically proven (svelte-check, lint including CSP-hash gate, build): the code change compiles, type-checks, and does not duplicate the template stylesheet into the app document. **NOT verified** (no frontend test framework exists, and neither printed pagination nor the app's on-screen font after a print cycle can be asserted by any automated command):

- Whether the LAN browser's own print-preview dialog now shows the same page breaks as the on-screen Paged.js preview (defect B fix).
- Whether the app's own UI font is unaffected after a print cycle, with no reload needed (defect A fix).
- Whether repeating a print cycle a second time in the same session (no reload) shows no `<style data-pagedjs-inserted-styles>` accumulation.

Per the plan's own verification section and the precedent set by 260805-edd/gdz/har/ifj, these three checks require a real LAN client (ideally Windows, matching the originally reported environment) and are flagged here as a **pending follow-up UAT**.

`printViaSystemBrowser` (desktop reference path) and `ui/src/lib/pdfPreview/bootstrapScript.js` (CSP-hash-locked) are confirmed byte-for-byte unchanged (verified via `git diff`).

---
*Phase: 260805-jwf*
*Completed: 2026-08-05*

## Self-Check: PASSED

- FOUND: `ui/src/features/acts/PdfPreviewModal.svelte`
- FOUND: `ui/src/pagedjs.d.ts`
- FOUND commit: `1f868ad`
